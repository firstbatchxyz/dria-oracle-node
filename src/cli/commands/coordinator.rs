use std::time::Duration;

use crate::{
    compute::handle_request,
    contracts::{bytes_to_string, string_to_bytes, OracleKind, TaskStatus},
    DriaOracle,
    OracleCoordinator::StatusUpdate,
};
use alloy::{
    eips::BlockNumberOrTag,
    primitives::{utils::format_ether, U256},
};
use dkn_workflows::{DriaWorkflowsConfig, Model, ModelProvider};
use eyre::{eyre, Context, Result};
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

impl DriaOracle {
    /// Runs the main loop of the oracle node.
    pub(in crate::cli) async fn run_oracle(
        &self,
        mut kinds: Vec<OracleKind>,
        models: Vec<Model>,
        from_block: BlockNumberOrTag,
        cancellation: CancellationToken,
    ) -> Result<()> {
        // if kinds are not provided, use the registrations as kinds
        if kinds.is_empty() {
            log::debug!("No kinds provided. Checking registrations.");
            for kind in [OracleKind::Generator, OracleKind::Validator] {
                if self.is_registered(kind).await? {
                    kinds.push(kind);
                }
            }

            if kinds.is_empty() {
                return Err(eyre!("You are not registered as any type of oracle."))?;
            }
        } else {
            // otherwise, make sure we are registered to required kinds
            for kind in &kinds {
                if !self.is_registered(*kind).await? {
                    return Err(eyre!("You need to register as {} first.", kind))?;
                }
            }
        }

        log::info!(
            "Running as: {}",
            kinds
                .iter()
                .map(|kind| kind.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        );

        // prepare model config & check services
        let mut model_config = DriaWorkflowsConfig::new(models);
        if model_config.models.is_empty() {
            return Err(eyre!("No models provided."))?;
        }
        let ollama_config = model_config.ollama.clone();
        model_config = model_config.with_ollama_config(
            ollama_config
                .with_min_tps(5.0)
                .with_timeout(Duration::from_secs(150)),
        );
        model_config.check_services().await?;

        // validator-specific checks here
        if kinds.contains(&OracleKind::Validator) {
            // make sure we have GPT4o model
            if !model_config
                .models
                .contains(&(ModelProvider::OpenAI, Model::GPT4o))
            {
                return Err(eyre!("Validator must have GPT4o model."))?;
            }

            // make sure node is whitelisted
            if !self.is_whitelisted(self.address()).await? {
                return Err(eyre!("You are not whitelisted in the registry."))?;
            }
        }

        // check previous tasks if `from_block` is not `Latest`
        if from_block != BlockNumberOrTag::Latest {
            tokio::select! {
                _ = cancellation.cancelled() => {
                    log::debug!("Cancellation signal received. Stopping...");
                    return Ok(());
                }
                result = self.handle_previous_tasks(from_block, &model_config, &kinds) => {
                    if let Err(e) = result {
                        log::error!("Could not handle previous tasks: {:?}", e);
                        log::warn!("Continuing anyways...");
                    }
                }
            }
        }

        loop {
            // subscribe to new tasks
            log::info!(
                "Subscribing to LLMOracleCoordinator ({})",
                self.addresses.coordinator,
            );
            let mut event_stream = self
                .subscribe_to_tasks()
                .await
                .wrap_err("could not subscribe to tasks")?
                .into_stream();

            // start the event loop
            log::info!("Listening for events...");
            loop {
                tokio::select! {
                    _ = cancellation.cancelled() => {
                        log::debug!("Cancellation signal received. Stopping...");
                        return Ok(());
                    }
                    next = event_stream.next() => {
                        match next {
                            Some(Ok((event, log))) => {
                                log::debug!(
                                    "Handling task {} (tx: {})",
                                    event.taskId,
                                    log.transaction_hash.unwrap_or_default()
                                );
                                self.handle_event_log(event, &kinds, &model_config).await
                            }
                            Some(Err(e)) => log::error!("Could not handle event: {}", e),
                            None => {
                                log::warn!("Stream ended, waiting a bit before restarting.");
                                tokio::time::sleep(Duration::from_secs(5)).await;
                                break
                            },
                        }
                    }
                }
            }
        }
    }

    async fn handle_event_log(
        &self,
        event: StatusUpdate,
        kinds: &[OracleKind],
        workflows: &DriaWorkflowsConfig,
    ) {
        let task_id = event.taskId;
        let Ok(status) = TaskStatus::try_from(event.statusAfter) else {
            log::error!("Could not parse task status: {}", event.statusAfter);
            return;
        };

        match handle_request(self, kinds, workflows, status, event.taskId, event.protocol).await {
            Ok(Some(receipt)) => {
                log::info!(
                    "Task {} processed successfully. (tx: {})",
                    task_id,
                    receipt.transaction_hash
                )
            }
            Ok(None) => {
                log::debug!("Task {} ignored.", task_id)
            }
            Err(e) => log::error!("Could not process task: {:?}", e),
        }
    }

    async fn handle_previous_tasks(
        &self,
        from_block: BlockNumberOrTag,
        workflows: &DriaWorkflowsConfig,
        kinds: &[OracleKind],
    ) -> Result<()> {
        log::info!(
            "Checking previous tasks from block {} until now.",
            from_block
        );
        let prev_tasks = self
            .get_tasks_in_range(from_block, BlockNumberOrTag::Latest)
            .await?;

        for (event, log) in prev_tasks {
            let status_before = TaskStatus::try_from(event.statusBefore)?;
            let status_after = TaskStatus::try_from(event.statusAfter)?;
            let task_id = event.taskId;
            log::info!(
                "Previous task: {} ({} -> {})",
                task_id,
                status_before,
                status_after
            );
            log::debug!(
                "Handling task {} (tx: {})",
                task_id,
                log.transaction_hash.unwrap_or_default()
            );
            match handle_request(
                self,
                kinds,
                workflows,
                status_after,
                event.taskId,
                event.protocol,
            )
            .await
            {
                Ok(Some(receipt)) => {
                    log::info!(
                        "Task {} processed successfully. (tx: {})",
                        task_id,
                        receipt.transaction_hash
                    )
                }
                Ok(None) => {
                    log::info!("Task {} ignored.", task_id)
                }
                Err(e) => log::error!("Could not process task: {:?}", e),
            }
        }

        Ok(())
    }

    pub(in crate::cli) async fn view_task_events(
        &self,
        from_block: impl Into<BlockNumberOrTag> + Clone,
        to_block: impl Into<BlockNumberOrTag> + Clone,
    ) -> Result<()> {
        let from_block: BlockNumberOrTag = from_block.clone().into();
        let to_block: BlockNumberOrTag = to_block.clone().into();
        log::info!(
            "Viewing task ids & statuses between blocks: {} - {}",
            from_block
                .as_number()
                .map(|n| n.to_string())
                .unwrap_or(from_block.to_string()),
            to_block
                .as_number()
                .map(|n| n.to_string())
                .unwrap_or(to_block.to_string())
        );

        let task_events = self.get_tasks_in_range(from_block, to_block).await?;

        for (event, log) in task_events {
            log::info!(
                "Task {} changed from {} to {} at block {}",
                event.taskId,
                TaskStatus::try_from(event.statusBefore).unwrap_or_default(),
                TaskStatus::try_from(event.statusAfter).unwrap_or_default(),
                log.block_number.unwrap_or_default()
            );
        }

        Ok(())
    }

    pub(in crate::cli) async fn view_task(&self, task_id: U256) -> Result<()> {
        log::info!("Viewing task {}.", task_id);
        let (request, responses, validations) = self.get_task(task_id).await?;

        log::info!(
            "Request Information:\nRequester: {}\nStatus:    {}\nInput:     {}\nModels:    {}",
            request.requester,
            TaskStatus::try_from(request.status)?,
            bytes_to_string(&request.input)?,
            bytes_to_string(&request.models)?
        );

        log::info!("Responses:");
        if responses._0.is_empty() {
            log::warn!("There are no responses yet.");
        } else {
            for (idx, response) in responses._0.iter().enumerate() {
                log::info!(
                    "Response  #{}\nOutput:    {}\nMetadata:  {}\nGenerator: {}",
                    idx,
                    bytes_to_string(&response.output)?,
                    bytes_to_string(&response.metadata)?,
                    response.responder
                );
            }
        }

        log::info!("Validations:");
        if validations._0.is_empty() {
            log::warn!("There are no validations yet.");
        } else {
            for (idx, validation) in validations._0.iter().enumerate() {
                log::info!(
                    "Validation #{}\nScores:     {:?}\nMetadata:   {}\nValidator:  {}",
                    idx,
                    validation.scores,
                    bytes_to_string(&validation.metadata)?,
                    validation.validator
                );
            }
        }

        Ok(())
    }

    pub(in crate::cli) async fn process_task(
        &self,
        workflows: &DriaWorkflowsConfig,
        kinds: &[OracleKind],
        task_id: U256,
    ) -> Result<()> {
        log::info!("Processing task {}.", task_id);
        let request = self.get_task_request(task_id).await?;

        log::info!(
            "Request Information:\nRequester: {}\nStatus:    {}\nInput:     {}\nModels:    {}",
            request.requester,
            TaskStatus::try_from(request.status)?,
            bytes_to_string(&request.input)?,
            bytes_to_string(&request.models)?
        );

        // TODO: !!!
        let status = TaskStatus::try_from(request.status)?;
        match handle_request(self, kinds, workflows, status, task_id, request.protocol).await {
            Ok(Some(receipt)) => {
                log::info!(
                    "Task {} processed successfully. (tx: {})",
                    task_id,
                    receipt.transaction_hash
                )
            }
            Ok(None) => {
                log::info!("Task {} ignored.", task_id)
            }
            Err(e) => log::error!("Could not process task: {:?}", e),
        }

        Ok(())
    }

    pub async fn request_task(
        &self,
        input: &str,
        models: Vec<Model>,
        difficulty: u8,
        num_gens: u64,
        num_vals: u64,
        protocol: String,
    ) -> Result<()> {
        let input = string_to_bytes(input.to_string());
        let models_str = models
            .iter()
            .map(|m| m.to_string())
            .collect::<Vec<String>>()
            .join(",");
        let models = string_to_bytes(models_str);
        log::info!("Requesting a new task.");

        // get total fee for the request
        let total_fee = self
            .get_request_fee(difficulty, num_gens, num_vals)
            .await?
            .totalFee;

        // check balance
        let balance = self.get_token_balance(self.address()).await?.amount;
        if balance < total_fee {
            return Err(eyre!("Insufficient balance. Please fund your wallet."));
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
