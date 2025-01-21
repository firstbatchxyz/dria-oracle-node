use crate::node::utils::get_connected_chain;

use super::{DriaOracle, DriaOracleConfig};
use alloy::contract::CallBuilder;
use alloy::hex::FromHex;

#[cfg(feature = "anvil")]
use alloy::providers::ext::AnvilApi;

use alloy::providers::{PendingTransactionBuilder, WalletProvider};
use alloy::transports::RpcError;
use alloy::{
    network::EthereumWallet,
    primitives::Address,
    providers::{Provider, ProviderBuilder},
};
use dkn_workflows::{DriaWorkflowsConfig, Model, ModelProvider};
use dria_oracle_contracts::{
    contract_error_report, get_coordinator_address, ContractAddresses, OracleCoordinator,
    OracleKind, TokenBalance,
};
use eyre::{eyre, Context, Result};
use std::env;

impl DriaOracle {
    /// Creates a new oracle node with the given private key and connected to the chain at the given RPC URL.
    ///
    /// The contract addresses are chosen based on the chain id returned from the provider.
    pub async fn new(config: DriaOracleConfig) -> Result<Self> {
        #[cfg(not(feature = "anvil"))]
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(config.wallet.clone())
            .on_http(config.rpc_url.clone());

        #[cfg(feature = "anvil")]
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(config.wallet.clone())
            .on_anvil_with_config(|anvil| {
                anvil.fork(config.rpc_url.clone()).port(Self::ANVIL_PORT)
            });

        // also set some balance for the chosen wallet
        #[cfg(feature = "anvil")]
        provider
            .anvil_set_balance(
                config.wallet.default_signer().address(),
                alloy::primitives::utils::parse_ether(Self::ANVIL_FUND_ETHER).unwrap(),
            )
            .await?;

        // fetch the chain id so that we can use the correct addresses
        let chain = get_connected_chain(&provider).await?;

        #[cfg(not(feature = "anvil"))]
        log::info!("Connected to chain: {}", chain);
        #[cfg(feature = "anvil")]
        log::info!("Connected to Anvil with forked chain: {}", chain);

        // get coordinator address from static list or the environment
        // address within env can have 0x at the start, or not, does not matter
        let coordinator_address = if let Ok(addr) = env::var("COORDINATOR_ADDRESS") {
            Address::from_hex(addr).wrap_err("could not parse coordinator address in env")?
        } else {
            get_coordinator_address(chain)?
        };

        // create a coordinator instance and get token & registry addresses
        let coordinator = OracleCoordinator::new(coordinator_address, &provider);
        let token_address = coordinator
            .feeToken()
            .call()
            .await
            .wrap_err("could not get token address from the coordinator")?
            ._0;
        let registry_address = coordinator
            .registry()
            .call()
            .await
            .wrap_err("could not get registry address from the coordinator")?
            ._0;

        let node = Self {
            config,
            addresses: ContractAddresses {
                coordinator: coordinator_address,
                registry: registry_address,
                token: token_address,
            },
            provider,
            kinds: Vec::default(), // TODO: take this from main config
            workflows: DriaWorkflowsConfig::default(), // TODO: take this from main config
        };

        Ok(node)
    }

    /// Creates a new node with the given wallet.
    ///
    /// - Provider is cloned and its wallet is mutated.
    /// - Config is cloned and its wallet & address are updated.
    pub fn connect(&self, wallet: EthereumWallet) -> Self {
        let mut provider = self.provider.clone();
        *provider.wallet_mut() = wallet.clone();

        Self {
            provider,
            config: self.config.clone().with_wallet(wallet),
            addresses: self.addresses.clone(),
            kinds: self.kinds.clone(),
            workflows: self.workflows.clone(),
        }
    }

    pub async fn prepare_oracle(
        &mut self,
        mut kinds: Vec<OracleKind>,
        models: Vec<Model>,
    ) -> Result<()> {
        if kinds.is_empty() {
            // if kinds are not provided, use the registrations as kinds
            log::debug!("No kinds provided. Checking registrations.");
            for kind in [OracleKind::Generator, OracleKind::Validator] {
                if self.is_registered(kind).await? {
                    kinds.push(kind);
                }
            }

            if kinds.is_empty() {
                return Err(eyre!("You are not registered as any type of oracle."))?;
            }
        } else {
            // otherwise, make sure we are registered to required kinds
            for kind in &kinds {
                if !self.is_registered(*kind).await? {
                    return Err(eyre!("You need to register as {} first.", kind))?;
                }
            }
        }

        // prepare model config & check services
        let mut model_config = DriaWorkflowsConfig::new(models);
        if model_config.models.is_empty() {
            return Err(eyre!("No models provided."))?;
        }
        let ollama_config = model_config.ollama.clone();
        model_config = model_config.with_ollama_config(
            ollama_config
                .with_min_tps(5.0)
                .with_timeout(std::time::Duration::from_secs(150)),
        );
        model_config.check_services().await?;

        // validator-specific checks here
        if kinds.contains(&OracleKind::Validator) {
            // make sure we have GPT4o model
            if !model_config
                .models
                .contains(&(ModelProvider::OpenAI, Model::GPT4o))
            {
                return Err(eyre!("Validator must have GPT4o model."))?;
            }

            // make sure node is whitelisted
            if !self.is_whitelisted(self.address()).await? {
                return Err(eyre!("You are not whitelisted in the registry."))?;
            }
        }

        self.workflows = model_config;
        self.kinds = kinds;

        Ok(())
    }

    /// Returns the native token balance of a given address.
    pub async fn get_native_balance(&self, address: Address) -> Result<TokenBalance> {
        let balance = self.provider.get_balance(address).await?;
        Ok(TokenBalance::new(balance, "ETH", None))
    }

    /// Returns the address of the configured wallet.
    #[inline(always)]
    pub fn address(&self) -> Address {
        self.config.wallet.default_signer().address()
    }

    /// Waits for a transaction to be mined, returning the receipt.
    #[inline]
    pub async fn wait_for_tx<T, N>(
        &self,
        tx: PendingTransactionBuilder<T, N>,
    ) -> Result<N::ReceiptResponse>
    where
        T: alloy::transports::Transport + Clone,
        N: alloy::network::Network,
    {
        log::info!("Waiting for tx: {:?}", tx.tx_hash());
        let receipt = tx
            .with_timeout(self.config.tx_timeout)
            .get_receipt()
            .await?;
        Ok(receipt)
    }

    /// Given a request, retries sending it with increasing gas prices to avoid
    /// the "tx underpriced" errors.
    #[inline]
    pub async fn send_with_gas_hikes<T, P, D, N>(
        &self,
        req: CallBuilder<T, P, D, N>,
    ) -> Result<PendingTransactionBuilder<T, N>>
    where
        T: alloy::transports::Transport + Clone,
        P: alloy::providers::Provider<T, N> + Clone,
        D: alloy::contract::CallDecoder + Clone,
        N: alloy::network::Network,
    {
        // gas price hikes to try in increasing order, first is 0 to simply use the
        // initial gas fee for the first attempt
        const GAS_PRICE_HIKES: [u128; 4] = [0, 12, 24, 36];

        // try and send tx, with increasing gas prices for few attempts
        let initial_gas_price = self.provider.get_gas_price().await?;
        for (attempt_no, increase_percentage) in GAS_PRICE_HIKES.iter().enumerate() {
            // set gas price
            let gas_price = initial_gas_price + (initial_gas_price / 100) * increase_percentage;

            // try to send tx with gas price
            match req.clone().gas_price(gas_price).send().await {
                // if all is well, we can return the tx
                Ok(tx) => {
                    return Ok(tx);
                }
                // if we get an RPC error; specifically, if the tx is underpriced, we try again with higher gas
                Err(alloy::contract::Error::TransportError(RpcError::ErrorResp(err))) => {
                    // TODO: kind of a code-smell, can we do better check here?
                    if err.message.contains("underpriced") {
                        log::warn!(
                            "{} with gas {} in attempt {}",
                            err.message,
                            gas_price,
                            attempt_no + 1,
                        );

                        // wait just a little bit
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

                        continue;
                    } else {
                        // otherwise let it be handled by the error report
                        return Err(contract_error_report(
                            alloy::contract::Error::TransportError(RpcError::ErrorResp(err)),
                        ));
                    }
                }
                // if we get any other error, we report it
                Err(err) => return Err(contract_error_report(err)),
            };
        }

        // all attempts failed
        Err(eyre!("Failed all attempts send tx due to underpriced gas."))
    }
}

impl core::fmt::Display for DriaOracle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Dria Oracle Node v{}\nOracle Address: {}\nRPC URL: {}\nCoordinator: {}\nTx timeout: {}s",
            env!("CARGO_PKG_VERSION"),
            self.address(),
            self.config.rpc_url,
            self.addresses.coordinator,
            self.config.tx_timeout.map(|t| t.as_secs()).unwrap_or_default()
        )
    }
}
