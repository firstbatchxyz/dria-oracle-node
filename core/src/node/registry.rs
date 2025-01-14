use alloy::{primitives::Address, rpc::types::TransactionReceipt};
use dria_oracle_contracts::{OracleKind, OracleRegistry, TokenBalance, ERC20};
use eyre::Result;

impl crate::DriaOracle {
    /// Register the oracle with the registry.
    #[inline]
    pub async fn register_kind(&self, kind: OracleKind) -> Result<TransactionReceipt> {
        let registry = OracleRegistry::new(self.addresses.registry, &self.provider);

        let req = registry.register(kind.into());
        let tx = self.send_with_gas_hikes(req).await?;

        self.wait_for_tx(tx).await
    }

    /// Unregister from the oracle registry.
    #[inline]
    pub async fn unregister_kind(&self, kind: OracleKind) -> Result<TransactionReceipt> {
        let registry = OracleRegistry::new(self.addresses.registry, &self.provider);

        let req = registry.unregister(kind.into());
        let tx = self.send_with_gas_hikes(req).await?;

        self.wait_for_tx(tx).await
    }

    /// Returns the amount of tokens to be staked to registry.
    pub async fn get_registry_stake_amount(&self, kind: OracleKind) -> Result<TokenBalance> {
        let registry = OracleRegistry::new(self.addresses.registry, &self.provider);

        let stake_amount = registry.getStakeAmount(kind.into()).call().await?._0;

        // return the symbol as well
        let token = ERC20::new(self.addresses.token, &self.provider);
        let token_symbol = token.symbol().call().await?._0;

        Ok(TokenBalance::new(
            stake_amount,
            token_symbol,
            Some(self.addresses.token),
        ))
    }

    /// Returns whether the oracle is registered as a given kind.
    #[inline]
    pub async fn is_registered(&self, kind: OracleKind) -> Result<bool> {
        let registry = OracleRegistry::new(self.addresses.registry, &self.provider);

        let is_registered = registry
            .isRegistered(self.address(), kind.into())
            .call()
            .await?;
        Ok(is_registered._0)
    }

    /// Returns whether a given address is whitelisted or not.
    #[inline]
    pub async fn is_whitelisted(&self, address: Address) -> Result<bool> {
        let registry = OracleRegistry::new(self.addresses.registry, &self.provider);

        let is_whitelisted = registry.isWhitelisted(address).call().await?;
        Ok(is_whitelisted._0)
    }
}
