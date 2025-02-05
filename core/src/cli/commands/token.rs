use crate::DriaOracle;
use eyre::Result;

impl DriaOracle {
    /// Display token balances.
    pub(in crate::cli) async fn display_balance(&self) -> Result<()> {
        let eth_balance = self.get_native_balance(self.address()).await?;
        let token_balance = self.get_token_balance(self.address()).await?;

        log::info!(
            "Your balances:\n{}",
            [eth_balance, token_balance]
                .map(|b| b.to_string())
                .join("\n")
        );

        Ok(())
    }

    /// Show the amount of claimable rewards.
    pub(in crate::cli) async fn display_rewards(&self) -> Result<()> {
        let allowance = self
            .allowance(*self.coordinator.address(), self.address())
            .await?;

        log::info!("Claimable rewards:");
        log::info!("{} ", allowance);
        if allowance.amount.is_zero() {
            log::warn!("You have no claimable rewards!");
        }

        Ok(())
    }

    /// Claim rewards
    pub(in crate::cli) async fn claim_rewards(&self) -> Result<()> {
        // get allowance
        let allowance = self
            .allowance(*self.coordinator.address(), self.address())
            .await?;

        // check if there are rewards to claim
        if allowance.amount.is_zero() {
            log::warn!("No rewards to claim.");
        } else {
            // transfer rewards
            self.transfer_from(
                *self.coordinator.address(),
                self.address(),
                allowance.amount,
            )
            .await?;
            log::info!("Rewards claimed: {}.", allowance);
        }

        Ok(())
    }
}
