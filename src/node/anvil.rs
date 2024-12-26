//! Anvil-related utilities.
//!
//! This module is only available when the `anvil` feature is enabled.

use crate::node::contract_error_report;
use crate::OracleRegistry;

use super::DriaOracle;
use alloy::network::EthereumWallet;
use alloy::primitives::Address;
use alloy::primitives::{utils::parse_ether, U256};
use alloy::providers::ext::AnvilApi;
use alloy::rpc::types::TransactionReceipt;
use alloy::signers::local::PrivateKeySigner;
use eyre::{Context, Result};

impl DriaOracle {
    /// Default ETH funding amount for generated wallets.
    const ANVIL_FUND_ETHER: &'static str = "10000";

    /// Generates a random wallet, funded with the given `fund` amount.
    ///
    /// If `fund` is not provided, 10K ETH is used.
    pub async fn anvil_funded_wallet(&self, fund: Option<U256>) -> Result<EthereumWallet> {
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
        // registry.provider().anvil_impersonate_account(owner).await?;

        let req = registry.addToWhitelist(vec![address]).from(owner);
        let tx = req
            .send()
            .await
            .map_err(contract_error_report)
            .wrap_err("could not add to whitelist")?;

        log::info!("Hash: {:?}", tx.tx_hash());
        let receipt = tx
            .with_timeout(self.config.tx_timeout)
            .get_receipt()
            .await?;

        // registry
        //     .provider()
        //     .anvil_stop_impersonating_account(owner)
        //     .await?;

        Ok(receipt)
    }
}
