use super::DriaOracle;
use alloy::eips::BlockNumberOrTag;
use alloy::primitives::aliases::U40;
use alloy::primitives::{Bytes, U256};
use alloy::rpc::types::{Log, TransactionReceipt};
use dria_oracle_contracts::string_to_bytes32;
use eyre::{eyre, Result};

use dria_oracle_contracts::OracleCoordinator::{
    getFeeReturn, getResponsesReturn, getValidationsReturn, requestsReturn,
    LLMOracleTaskParameters, StatusUpdate,
};

impl DriaOracle {
    /// Request an oracle task. This is not done by the oracle normally, but we have it added for testing purposes.
    pub async fn request(
        &self,
        input: Bytes,
        models: Bytes,
        difficulty: u8,
        num_gens: u64,
        num_vals: u64,
        protocol: String,
    ) -> Result<TransactionReceipt> {
        let parameters = LLMOracleTaskParameters {
            difficulty,
            numGenerations: U40::from(num_gens),
            numValidations: U40::from(num_vals),
        };

        let req = self
            .coordinator
            .request(string_to_bytes32(protocol)?, input, models, parameters);
        let tx = self.send_with_gas_hikes(req).await?;
        self.wait_for_tx(tx).await
    }

    /// Responds to a generation request with the response, metadata, and a valid nonce.
    pub async fn respond_generation(
        &self,
        task_id: U256,
        response: Bytes,
        metadata: Bytes,
        nonce: U256,
    ) -> Result<TransactionReceipt> {
        let req = self.coordinator.respond(task_id, nonce, response, metadata);
        let tx = self.send_with_gas_hikes(req).await?;
        self.wait_for_tx(tx).await
    }

    /// Responds to a validation request with the score, metadata, and a valid nonce.
    #[inline]
    pub async fn respond_validation(
        &self,
        task_id: U256,
        scores: Vec<U256>,
        metadata: Bytes,
        nonce: U256,
    ) -> Result<TransactionReceipt> {
        let req = self.coordinator.validate(task_id, nonce, scores, metadata);
        let tx = self.send_with_gas_hikes(req).await?;
        self.wait_for_tx(tx).await
    }

    /// Get previous tasks within the range of blocks.
    pub async fn get_tasks_in_range(
        &self,
        from_block: impl Into<BlockNumberOrTag>,
        to_block: impl Into<BlockNumberOrTag>,
    ) -> Result<Vec<(StatusUpdate, Log)>> {
        let tasks = self
            .coordinator
            .StatusUpdate_filter()
            .from_block(from_block)
            .to_block(to_block)
            .query()
            .await?;

        Ok(tasks)
    }

    /// Get task info for a given task id.
    pub async fn get_task(
        &self,
        task_id: U256,
    ) -> Result<(requestsReturn, getResponsesReturn, getValidationsReturn)> {
        // check if task id is valid
        if task_id.is_zero() {
            return Err(eyre!("Task ID must be non-zero."));
        } else if task_id >= self.coordinator.nextTaskId().call().await?._0 {
            return Err(eyre!("Task with id {} has not been created yet.", task_id));
        }

        // get task info
        let request = self.coordinator.requests(task_id).call().await?;
        let responses = self.coordinator.getResponses(task_id).call().await?;
        let validations = self.coordinator.getValidations(task_id).call().await?;

        Ok((request, responses, validations))
    }

    /// Get fee details for a given request setting.
    pub async fn get_request_fee(
        &self,
        difficulty: u8,
        num_gens: u64,
        num_vals: u64,
    ) -> Result<getFeeReturn> {
        let parameters = LLMOracleTaskParameters {
            difficulty,
            numGenerations: U40::from(num_gens),
            numValidations: U40::from(num_vals),
        };

        let fees = self.coordinator.getFee(parameters).call().await?;

        Ok(fees)
    }
}
