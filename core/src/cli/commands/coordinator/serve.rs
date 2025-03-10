use crate::{compute::handle_request, DriaOracle};
use alloy::{eips::BlockNumberOrTag, primitives::U256};
use dria_oracle_contracts::{
    bytes32_to_string, bytes_to_string, OracleCoordinator::StatusUpdate, TaskStatus,
};
use eyre::Result;

impl DriaOracle {
    pub(in crate::cli) async fn process_task_by_id(&self, task_id: U256) -> Result<()> {
        log::info!("Processing task {}.", task_id);
        let request = self.coordinator.requests(task_id).call().await?;

        log::info!(
            "Request Information:\nRequester: {}\nStatus:    {}\nInput:     {}\nModels:    {}\nProtocol:  {}",
            request.requester,
            TaskStatus::try_from(request.status)?,
            bytes_to_string(&request.input)?,
            bytes_to_string(&request.models)?,
            bytes32_to_string(&request.protocol)?
        );

        let status = TaskStatus::try_from(request.status)?;
        match handle_request(self, status, task_id, request.protocol).await {
            Ok(Some(_receipt)) => {}
            Ok(None) => {
                log::info!("Task {} ignored.", task_id)
            }
            // using `{:#}` here to get a single-line error message
            Err(e) => log::error!("Could not process task {}: {:#}", task_id, e),
        }

        Ok(())
    }

    pub(in crate::cli) async fn process_task_by_event(&self, event: StatusUpdate) {
        let Ok(status) = TaskStatus::try_from(event.statusAfter) else {
            log::error!("Could not parse task status: {}", event.statusAfter);
            return;
        };

        if let Err(err) = handle_request(self, status, event.taskId, event.protocol).await {
            log::error!("Could not process task {}: {:?}", event.taskId, err);
        }
    }

    pub(in crate::cli) async fn process_tasks_within_range(
        &self,
        from_block: BlockNumberOrTag,
        to_block: BlockNumberOrTag,
    ) -> Result<()> {
        log::info!(
            "Processing tasks between blocks: {} - {}",
            from_block
                .as_number()
                .map(|n| n.to_string())
                .unwrap_or(from_block.to_string()),
            to_block
                .as_number()
                .map(|n| n.to_string())
                .unwrap_or(to_block.to_string())
        );

        for (event, _) in self.get_tasks_in_range(from_block, to_block).await? {
            self.process_task_by_event(event).await;
        }

        Ok(())
    }
}
