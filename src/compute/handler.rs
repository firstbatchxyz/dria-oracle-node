use crate::{
    contracts::{OracleKind, TaskStatus},
    DriaOracle,
};
use alloy::{
    primitives::{FixedBytes, U256},
    rpc::types::TransactionReceipt,
};
use dkn_workflows::DriaWorkflowsConfig;
use eyre::Result;

use super::{handle_generation, handle_validation};

/// Handles a task request.
///
/// - Generation tasks are forwarded to `handle_generation`
/// - Validation tasks are forwarded to `handle_validation`
pub async fn handle_request(
    node: &DriaOracle,
    kinds: &[OracleKind],
    workflows: &DriaWorkflowsConfig,
    status: TaskStatus,
    task_id: U256,
    protocol: FixedBytes<32>,
) -> Result<Option<TransactionReceipt>> {
    log::debug!("Received event for task {} ()", task_id);

    // we check the `statusAfter` field of the event, which indicates the final status of the listened task
    let response_tx_hash = match status {
        TaskStatus::PendingGeneration => {
            if kinds.contains(&OracleKind::Generator) {
                handle_generation(node, workflows, task_id, protocol).await?
            } else {
                log::debug!(
                    "Ignoring generation task {} as you are not generator.",
                    task_id
                );
                return Ok(None);
            }
        }
        TaskStatus::PendingValidation => {
            if kinds.contains(&OracleKind::Validator) {
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
            log::debug!("Task {} is completed.", task_id);
            return Ok(None);
        }
        // this is kind of unexpected, but we dont have to return an error just for this
        TaskStatus::None => {
            log::error!("None status received in an event: {}", task_id);
            return Ok(None);
        }
    };

    Ok(response_tx_hash)
}
