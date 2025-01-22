use alloy::primitives::utils::format_ether;
use dria_oracle_contracts::OracleKind;
use eyre::Result;

use crate::DriaOracle;

impl DriaOracle {
    /// Registers the oracle node as an oracle for the given `kind`.
    ///
    /// - If the node is already registered, it will do nothing.
    /// - If the node is not registered, it will approve the required amount of tokens
    ///   to the registry and then register the node.
    pub async fn register(&self, kind: OracleKind) -> Result<()> {
        log::info!("Registering as a {}.", kind);

        // check if registered already
        if self.is_registered(kind).await? {
            log::warn!("You are already registered as a {}.", kind);
            return Ok(());
        }

        // calculate the required approval for registration
        let stake = self.get_registry_stake_amount(kind).await?;
        let allowance = self
            .allowance(self.address(), *self.registry.address())
            .await?;

        // approve if necessary
        if allowance.amount < stake.amount {
            let difference = stake.amount - allowance.amount;
            log::info!(
                "Approving {} tokens for {} registration.",
                format_ether(difference),
                kind
            );

            // check balance
            let balance = self.get_token_balance(self.address()).await?;
            if balance.amount < difference {
                return Err(eyre::eyre!(
                    "Not enough balance to approve. (have: {}, required: {})",
                    balance,
                    difference
                ));
            }

            // approve the difference
            self.approve(*self.registry.address(), difference).await?;
        } else {
            log::info!("Already approved enough tokens.");
        }

        // register
        log::info!("Registering.");
        self.register_kind(kind).await?;

        Ok(())
    }

    /// Unregisters the oracle node as an oracle for the given `kind`.
    ///
    /// - If the node is not registered, it will do nothing.
    /// - If the node is registered, it will unregister the node and transfer all allowance
    ///   from the registry back to the oracle.
    pub async fn unregister(&self, kind: OracleKind) -> Result<()> {
        log::info!("Unregistering as {}.", kind);

        // check if not registered anyways
        if !self.is_registered(kind).await? {
            log::warn!("You are already not registered as a {}.", kind);
            return Ok(());
        }

        self.unregister_kind(kind).await?;

        // transfer all allowance from registry back to oracle
        // to get back the registrations fee
        let allowance = self
            .allowance(*self.registry.address(), self.address())
            .await?;
        log::info!(
            "Transferring all allowance ({}) back from registry.",
            allowance
        );
        self.transfer_from(*self.registry.address(), self.address(), allowance.amount)
            .await?;

        Ok(())
    }

    /// Displays the registration status of the oracle node for all oracle kinds.
    pub(in crate::cli) async fn display_registrations(&self) -> Result<()> {
        for kind in [OracleKind::Generator, OracleKind::Validator] {
            let is_registered = self.is_registered(kind).await?;
            log::info!("{}: {}", kind, is_registered);
        }

        Ok(())
    }
}
