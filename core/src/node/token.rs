use super::DriaOracle;
use alloy::primitives::{Address, U256};
use alloy::rpc::types::TransactionReceipt;
use dria_oracle_contracts::{TokenBalance, ERC20};
use eyre::Result;

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
        let tx = self.send_with_gas_hikes(req).await?;
        self.wait_for_tx(tx).await
    }

    /// Approves the `spender` to spend `amount` tokens on behalf of the caller.
    pub async fn approve(&self, spender: Address, amount: U256) -> Result<TransactionReceipt> {
        let token = ERC20::new(self.addresses.token, &self.provider);

        let req = token.approve(spender, amount);
        let tx = self.send_with_gas_hikes(req).await?;
        self.wait_for_tx(tx).await
    }

    /// Returns the allowance of a given `spender` address to spend tokens on behalf of `owner` address.
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
