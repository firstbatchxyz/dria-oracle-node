use alloy::{eips::BlockNumberOrTag, primitives::U256};
use dria_oracle_contracts::{bytes32_to_string, bytes_to_string, TaskStatus};
use eyre::Result;

impl crate::DriaOracle {
    /// Views the task events between two blocks, logs everything on screen.
    pub(in crate::cli) async fn view_task_events(
        &self,
        from_block: BlockNumberOrTag,
        to_block: BlockNumberOrTag,
    ) -> Result<()> {
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
                "Task {} changed from {} to {} at block {}, tx: {}",
                event.taskId,
                TaskStatus::try_from(event.statusBefore).unwrap_or_default(),
                TaskStatus::try_from(event.statusAfter).unwrap_or_default(),
                log.block_number.unwrap_or_default(),
                log.transaction_hash.unwrap_or_default()
            );
        }

        Ok(())
    }

    /// Views the request, responses and validations of a single task, logs everything on screen.
    pub(in crate::cli) async fn view_task(&self, task_id: U256) -> Result<()> {
        log::info!("Viewing task {}.", task_id);
        let (request, responses, validations) = self.get_task(task_id).await?;

        log::info!(
          "Request Information:\nRequester: {}\nStatus:    {}\nInput:     {}\nModels:    {}\nProtocol:   {}",
          request.requester,
          TaskStatus::try_from(request.status)?,
          bytes_to_string(&request.input)?,
          bytes_to_string(&request.models)?,
          bytes32_to_string(&request.protocol)?
      );

        log::info!("Responses:");
        if responses._0.is_empty() {
            log::info!("There are no responses yet.");
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
            log::info!("There are no validations yet.");
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
}
