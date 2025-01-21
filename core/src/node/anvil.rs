//! Anvil-related utilities.
//!
//! This module is only available when the `anvil` feature is enabled,
//! which is only expected to happen in tests.

use super::DriaOracle;

use alloy::network::{Ethereum, EthereumWallet};
use alloy::primitives::Address;
use alloy::primitives::{utils::parse_ether, U256};
use alloy::providers::ext::AnvilApi;
use alloy::providers::{PendingTransactionBuilder, Provider, ProviderBuilder};
use alloy::rpc::types::{TransactionReceipt, TransactionRequest};
use alloy::signers::local::PrivateKeySigner;
use alloy::transports::http::Http;
use dria_oracle_contracts::OracleRegistry;
use eyre::Result;
use reqwest::{Client, Url};

impl DriaOracle {
    /// We dedicate an unused port to Anvil.
    pub const ANVIL_PORT: u16 = 8545;
    /// Default ETH funding amount for generated wallets.
    pub const ANVIL_FUND_ETHER: &'static str = "10000";

    /// Generates a random wallet, funded with the given `fund` amount.
    ///
    /// If `fund` is not provided, 10K ETH is used.
    pub async fn anvil_new_funded_wallet(&self, fund: Option<U256>) -> Result<EthereumWallet> {
        let fund = fund.unwrap_or_else(|| parse_ether(Self::ANVIL_FUND_ETHER).unwrap());
        let signer = PrivateKeySigner::random();
        self.provider
            .anvil_set_balance(signer.address(), fund)
            .await?;
        let wallet = EthereumWallet::from(signer);
        Ok(wallet)
    }

    /// Whitelists a given address, impersonates the owner in doing so.
    pub async fn anvil_whitelist_registry(&self, address: Address) -> Result<TransactionReceipt> {
        let registry = OracleRegistry::new(self.addresses.registry, &self.provider);
        let owner = registry.owner().call().await?._0;

        let tx = self
            .send_impersonated_transaction(
                registry
                    .addToWhitelist(vec![address])
                    .into_transaction_request(),
                owner,
            )
            .await?;
        let receipt = self.wait_for_tx(tx).await?;

        Ok(receipt)
    }

    /// Assumes that an Anvil instance is running already at the given port.
    ///
    /// We use this due to the issue: https://github.com/alloy-rs/alloy/issues/1918
    #[inline]
    pub async fn send_impersonated_transaction(
        &self,
        tx: TransactionRequest,
        from: Address,
    ) -> Result<PendingTransactionBuilder<Http<Client>, Ethereum>> {
        let anvil = ProviderBuilder::new().on_http(Self::anvil_url());

        anvil.anvil_impersonate_account(from).await?;
        let pending_tx = anvil.send_transaction(tx.from(from)).await?;
        anvil.anvil_stop_impersonating_account(from).await?;

        Ok(pending_tx)
    }

    /// Returns the spawned Anvil URL, can be used with `ProviderBuilder::new().on_http(url)`.
    #[inline(always)]
    pub fn anvil_url() -> Url {
        Url::parse(&format!("http://localhost:{}", Self::ANVIL_PORT)).expect("could not parse URL")
    }
}
