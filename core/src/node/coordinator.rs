use self::OracleCoordinator::getFeeReturn;
use super::{DriaOracle, DriaOracleProviderTransport};
use crate::contracts::*;
use alloy::contract::EventPoller;
use alloy::eips::BlockNumberOrTag;
use alloy::primitives::aliases::U40;
use alloy::primitives::{Bytes, U256};
use alloy::rpc::types::{Log, TransactionReceipt};
use eyre::{eyre, Context, Result};
use LLMOracleTask::{TaskResponse, TaskValidation};
use OracleCoordinator::LLMOracleTaskParameters;
use OracleCoordinator::{getResponsesReturn, getValidationsReturn, requestsReturn, StatusUpdate};

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
        let coordinator = OracleCoordinator::new(self.addresses.coordinator, &self.provider);

        let parameters = LLMOracleTaskParameters {
            difficulty,
            numGenerations: U40::from(num_gens),
            numValidations: U40::from(num_vals),
        };
        let req = coordinator.request(string_to_bytes32(protocol)?, input, models, parameters);
        let tx = req
            .send()
            .await
            .map_err(contract_error_report)
            .wrap_err("could not request task")?;

        log::info!("Hash: {:?}", tx.tx_hash());
        let receipt = tx
            .with_timeout(self.config.tx_timeout)
            .get_receipt()
            .await?;
        Ok(receipt)
    }

    /// Returns the best response to a given task request, as per their scores.
    /// Note that this logic takes place at contract level, and will revert if the task is not completed.
    pub async fn get_task_best_response(&self, task_id: U256) -> Result<TaskResponse> {
        let coordinator = OracleCoordinator::new(self.addresses.coordinator, &self.provider);

        let request = coordinator.getBestResponse(task_id).call().await?;
        Ok(request._0)
    }

    /// Returns the task request with the given id.
    pub async fn get_task_request(
        &self,
        task_id: U256,
    ) -> Result<OracleCoordinator::requestsReturn> {
        let coordinator = OracleCoordinator::new(self.addresses.coordinator, &self.provider);

        let request = coordinator.requests(task_id).call().await?;
        Ok(request)
    }

    /// Returns the generation responses to a given task request.
    pub async fn get_task_responses(&self, task_id: U256) -> Result<Vec<TaskResponse>> {
        let coordinator = OracleCoordinator::new(self.addresses.coordinator, &self.provider);

        let responses = coordinator.getResponses(task_id).call().await?;
        Ok(responses._0)
    }

    /// Returns the validation responses to a given task request.
    pub async fn get_task_validations(&self, task_id: U256) -> Result<Vec<TaskValidation>> {
        let coordinator = OracleCoordinator::new(self.addresses.coordinator, &self.provider);

        let responses = coordinator.getValidations(task_id).call().await?;
        Ok(responses._0)
    }

    pub async fn respond_generation(
        &self,
        task_id: U256,
        response: Bytes,
        metadata: Bytes,
        nonce: U256,
    ) -> Result<TransactionReceipt> {
        let coordinator = OracleCoordinator::new(self.addresses.coordinator, &self.provider);

        let req = coordinator.respond(task_id, nonce, response, metadata);
        let tx = req.send().await.map_err(contract_error_report)?;

        log::info!("Hash: {:?}", tx.tx_hash());
        let receipt = tx
            .with_timeout(self.config.tx_timeout)
            .get_receipt()
            .await?;
        Ok(receipt)
    }

    pub async fn respond_validation(
        &self,
        task_id: U256,
        scores: Vec<U256>,
        metadata: Bytes,
        nonce: U256,
    ) -> Result<TransactionReceipt> {
        let coordinator = OracleCoordinator::new(self.addresses.coordinator, &self.provider);

        let req = coordinator.validate(task_id, nonce, scores, metadata);
        let tx = req.send().await.map_err(contract_error_report)?;

        log::info!("Hash: {:?}", tx.tx_hash());
        let receipt = tx
            .with_timeout(self.config.tx_timeout)
            .get_receipt()
            .await?;
        Ok(receipt)
    }

    /// Subscribes to events & processes tasks.
    pub async fn subscribe_to_tasks(
        &self,
    ) -> Result<EventPoller<DriaOracleProviderTransport, StatusUpdate>> {
        let coordinator = OracleCoordinator::new(self.addresses.coordinator, &self.provider);

        Ok(coordinator.StatusUpdate_filter().watch().await?)
    }

    /// Get previous tasks within the range of blocks.
    pub async fn get_tasks_in_range(
        &self,
        from_block: impl Into<BlockNumberOrTag>,
        to_block: impl Into<BlockNumberOrTag>,
    ) -> Result<Vec<(StatusUpdate, Log)>> {
        let coordinator = OracleCoordinator::new(self.addresses.coordinator, &self.provider);

        let tasks = coordinator
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
        let coordinator = OracleCoordinator::new(self.addresses.coordinator, &self.provider);

        // check if task id is valid
        if task_id.is_zero() {
            return Err(eyre!("Task ID must be non-zero."));
        } else if task_id >= coordinator.nextTaskId().call().await?._0 {
            return Err(eyre!("Task with id {} has not been created yet.", task_id));
        }

        // get task info
        let request = coordinator.requests(task_id).call().await?;
        let responses = coordinator.getResponses(task_id).call().await?;
        let validations = coordinator.getValidations(task_id).call().await?;

        Ok((request, responses, validations))
    }

    /// Returns the next task id.
    pub async fn get_next_task_id(&self) -> Result<U256> {
        let coordinator = OracleCoordinator::new(self.addresses.coordinator, &self.provider);

        let task_id = coordinator.nextTaskId().call().await?;
        Ok(task_id._0)
    }

    /// Get fee details for a given request setting.
    pub async fn get_request_fee(
        &self,
        difficulty: u8,
        num_gens: u64,
        num_vals: u64,
    ) -> Result<getFeeReturn> {
        let coordinator = OracleCoordinator::new(self.addresses.coordinator, &self.provider);

        let parameters = LLMOracleTaskParameters {
            difficulty,
            numGenerations: U40::from(num_gens),
            numValidations: U40::from(num_vals),
        };

        let fees = coordinator.getFee(parameters).call().await?;

        Ok(fees)
    }
}
