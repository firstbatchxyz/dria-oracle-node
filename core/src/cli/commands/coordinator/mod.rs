mod request;
mod view;

use std::time::Duration;

use crate::{compute::handle_request, DriaOracle};
use alloy::{eips::BlockNumberOrTag, primitives::U256};
use dria_oracle_contracts::{
    bytes_to_string, OracleCoordinator::StatusUpdate, OracleKind, TaskStatus,
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
}
