mod coordinator;
mod registry;
mod token;

#[cfg(feature = "anvil")]
mod anvil;

use super::DriaOracleConfig;
use crate::contracts::*;
use alloy::providers::fillers::{
    BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
};
use alloy::providers::layers::AnvilProvider;
use alloy::providers::WalletProvider;
use alloy::transports::http::Http;
use alloy::transports::BoxTransport;
use alloy::{
    network::{Ethereum, EthereumWallet},
    primitives::Address,
    providers::{Identity, Provider, ProviderBuilder, RootProvider},
};
use alloy_chains::Chain;
use eyre::{eyre, Context, Result};
use reqwest::Client;
use std::env;

// TODO: use a better type for these
#[cfg(not(feature = "anvil"))]
type DriaOracleProviderTransport = Http<Client>;
#[cfg(not(feature = "anvil"))]
type DriaOracleProvider = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider<DriaOracleProviderTransport>,
    DriaOracleProviderTransport,
    Ethereum,
>;

#[cfg(feature = "anvil")]
type DriaOracleProviderTransport = BoxTransport;
#[cfg(feature = "anvil")]
type DriaOracleProvider = FillProvider<
    JoinFill<
        JoinFill<
            JoinFill<
                Identity,
                JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
            >,
            WalletFiller<EthereumWallet>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    AnvilProvider<RootProvider<BoxTransport>, BoxTransport>,
    BoxTransport,
    Ethereum,
>;

pub struct DriaOracle {
    pub config: DriaOracleConfig,
    /// Contract addresses for the oracle, respects the connected chain.
    pub addresses: ContractAddresses,
    /// Underlying provider type.
    pub provider: DriaOracleProvider,
}

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
            .on_anvil_with_wallet_and_config(|anvil| {
                anvil.fork(config.rpc_url.clone()).block_time(1)
            });

        // fetch the chain id so that we can use the correct addresses
        let chain_id_u64 = provider
            .get_chain_id()
            .await
            .wrap_err("could not get chain id")?;
        let chain = Chain::from_id(chain_id_u64);
        log::info!("Connected to chain: {}", chain);

        let node = Self {
            config,
            addresses: ADDRESSES[&chain].clone(),
            provider,
        };

        node.check_contract_sizes().await?;
        node.check_contract_tokens().await?;

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
        }
    }

    /// Returns the native token balance of a given address.
    pub async fn get_native_balance(&self, address: Address) -> Result<TokenBalance> {
        let balance = self.provider.get_balance(address).await?;
        Ok(TokenBalance::new(balance, "ETH".to_string(), None))
    }

    /// Checks contract sizes to ensure they are deployed.
    ///
    /// Returns an error if any of the contracts are not deployed.
    pub async fn check_contract_sizes(&self) -> Result<()> {
        let coordinator_size = self
            .provider
            .get_code_at(self.addresses.coordinator)
            .await
            .map(|s| s.len())?;
        if coordinator_size == 0 {
            return Err(eyre!("Coordinator contract not deployed."));
        }
        let registry_size = self
            .provider
            .get_code_at(self.addresses.registry)
            .await
            .map(|s| s.len())?;
        if registry_size == 0 {
            return Err(eyre!("Registry contract not deployed."));
        }
        let token_size = self
            .provider
            .get_code_at(self.addresses.token)
            .await
            .map(|s| s.len())?;
        if token_size == 0 {
            return Err(eyre!("Token contract not deployed."));
        }

        Ok(())
    }

    /// Ensures that the registry & coordinator tokens match the expected token.
    pub async fn check_contract_tokens(&self) -> Result<()> {
        let coordinator = OracleCoordinator::new(self.addresses.coordinator, &self.provider);
        let registry = OracleRegistry::new(self.addresses.registry, &self.provider);

        // check registry
        let registry_token = registry.token().call().await?._0;
        if registry_token != self.addresses.token {
            return Err(eyre!("Registry token does not match."));
        }

        // check coordinator
        let coordinator_token = coordinator.feeToken().call().await?._0;
        if coordinator_token != self.addresses.token {
            return Err(eyre!("Registry token does not match."));
        }

        Ok(())
    }

    /// Returns the address of the configured wallet.
    #[inline(always)]
    pub fn address(&self) -> Address {
        self.config.wallet.default_signer().address()
    }
}

impl core::fmt::Display for DriaOracle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Dria Oracle Node v{}\nAddress: {}\nRPC URL: {}",
            env!("CARGO_PKG_VERSION"),
            self.address(),
            self.config.rpc_url,
        )
    }
}
