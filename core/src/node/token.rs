use super::DriaOracle;
use crate::contracts::*;
use alloy::primitives::{Address, U256};
use alloy::rpc::types::TransactionReceipt;
use eyre::{Context, Result};

impl DriaOracle {
    /// Returns the token balance of a given address.
    pub async fn get_token_balance(&self, address: Address) -> Result<TokenBalance> {
        let token = ERC20::new(self.addresses.token, &self.provider);
        let token_balance = token.balanceOf(address).call().await?._0;
        let token_symbol = token.symbol().call().await?._0;

        Ok(TokenBalance::new(
            token_balance,
            token_symbol,
            Some(self.addresses.token),
        ))
    }

    /// Transfer tokens from one address to another, calls `transferFrom` of the ERC20 contract.
    ///
    /// Assumes that approvals are made priorly.
    pub async fn transfer_from(
        &self,
        from: Address,
        to: Address,
        amount: U256,
    ) -> Result<TransactionReceipt> {
        let token = ERC20::new(self.addresses.token, &self.provider);

        let req = token.transferFrom(from, to, amount);
        let tx = req.send().await.map_err(contract_error_report)?;

        log::info!("Hash: {:?}", tx.tx_hash());
        let receipt = tx
            .with_timeout(self.config.tx_timeout)
            .get_receipt()
            .await?;
        Ok(receipt)
    }

    pub async fn approve(&self, spender: Address, amount: U256) -> Result<TransactionReceipt> {
        let token = ERC20::new(self.addresses.token, &self.provider);

        let req = token.approve(spender, amount);
        let tx = req
            .send()
            .await
            .map_err(contract_error_report)
            .wrap_err("could not approve tokens")?;

        log::info!("Hash: {:?}", tx.tx_hash());
        let receipt = tx
            .with_timeout(self.config.tx_timeout)
            .get_receipt()
            .await?;
        Ok(receipt)
    }

    pub async fn allowance(&self, owner: Address, spender: Address) -> Result<TokenBalance> {
        let token = ERC20::new(self.addresses.token, &self.provider);
        let token_symbol = token.symbol().call().await?._0;

        let allowance = token.allowance(owner, spender).call().await?._0;
        Ok(TokenBalance::new(
            allowance,
            token_symbol,
            Some(self.addresses.token),
        ))
    }
}
