use crate::DriaOracle;
use dria_oracle_contracts::{OracleKind, TaskStatus};

use alloy::{
    primitives::{FixedBytes, U256},
    rpc::types::TransactionReceipt,
};
use eyre::Result;

use super::{handle_generation, handle_validation};

/// Handles a task request.
///
/// - Generation tasks are forwarded to `handle_generation`
/// - Validation tasks are forwarded to `handle_validation`
pub async fn handle_request(
    node: &DriaOracle,
    status: TaskStatus,
    task_id: U256,
    protocol: FixedBytes<32>,
) -> Result<Option<TransactionReceipt>> {
    log::debug!("Received event for task {} ()", task_id);

    // we check the `statusAfter` field of the event, which indicates the final status of the listened task
    let response_receipt = match status {
        TaskStatus::PendingGeneration => {
            if node.kinds.contains(&OracleKind::Generator) {
                handle_generation(node, task_id, protocol).await?
            } else {
                log::debug!(
                    "Ignoring generation task {} as you are not generator.",
                    task_id
                );
                return Ok(None);
            }
        }
        TaskStatus::PendingValidation => {
            if node.kinds.contains(&OracleKind::Validator) {
                handle_validation(node, task_id).await?
            } else {
                log::debug!(
                    "Ignoring generation task {} as you are not validator.",
                    task_id
                );
                return Ok(None);
            }
        }
        TaskStatus::Completed => {
            return Ok(None);
        }
        // this is unexpected to happen unless the contract has an eror
        // but we dont have to return an error just for this
        TaskStatus::None => {
            log::error!("None status received in an event: {}", task_id);
            return Ok(None);
        }
    };

    if let Some(receipt) = &response_receipt {
        log::info!(
            "Task {} processed successfully. (tx: {})",
            task_id,
            receipt.transaction_hash
        );
    } else {
        log::debug!("Task {} ignored.", task_id)
    }

    Ok(response_receipt)
}
