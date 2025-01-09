use alloy::primitives::utils::format_ether;
use dkn_workflows::Model;
use dria_oracle_contracts::string_to_bytes;
use eyre::Result;

impl crate::DriaOracle {
    /// Requests a task with the given parameters.
    ///
    /// Oracle does not usually do this, but we still provide the capability for testing & playing around.
    pub async fn request_task(
        &self,
        input: &str,
        models: Vec<Model>,
        difficulty: u8,
        num_gens: u64,
        num_vals: u64,
        protocol: String,
    ) -> Result<()> {
        log::info!("Requesting a new task.");
        let input = string_to_bytes(input.to_string());
        let models_str = models
            .iter()
            .map(|m| m.to_string())
            .collect::<Vec<String>>()
            .join(",");
        let models = string_to_bytes(models_str);

        // get total fee for the request
        log::debug!("Checking fee & allowance.");
        let total_fee = self
            .get_request_fee(difficulty, num_gens, num_vals)
            .await?
            .totalFee;
        // check balance
        let balance = self.get_token_balance(self.address()).await?.amount;
        if balance < total_fee {
            return Err(eyre::eyre!(
                "Insufficient balance. Please fund your wallet."
            ));
        }

        // check current allowance
        let allowance = self
            .allowance(self.address(), self.addresses.coordinator)
            .await?
            .amount;
        // make sure we have enough allowance
        if allowance < total_fee {
            let approval_amount = total_fee - allowance;
            log::info!(
                "Insufficient allowance. Approving the required amount: {}.",
                format_ether(approval_amount)
            );

            self.approve(self.addresses.coordinator, approval_amount)
                .await?;
            log::info!("Token approval successful.");
        }

        // make the request
        let receipt = self
            .request(input, models, difficulty, num_gens, num_vals, protocol)
            .await?;
        log::info!(
            "Task requested successfully. tx: {}",
            receipt.transaction_hash
        );

        Ok(())
    }
}
