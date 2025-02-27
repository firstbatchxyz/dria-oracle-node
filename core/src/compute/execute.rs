use core::time::Duration;
use dkn_workflows::{ExecutionError, Executor, Model, ProgramMemory, Workflow};
use eyre::Context;

const NUM_RETRIES: usize = 4;

/// A wrapper for executing a workflow with retries.
///
/// - Creates an `Executor` with the given model.
/// - Executes the given workflow with the executor over an empty memory.
/// - If the execution fails due to timeout, retries up to 3 times with slightly increasing timeout durations.
pub async fn execute_workflow_with_timedout_retries(
    workflow: &Workflow,
    model: Model,
    duration: Duration,
) -> eyre::Result<String> {
    let executor = Executor::new(model);

    let mut retries = 0;
    while retries < NUM_RETRIES {
        let mut memory = ProgramMemory::new();
        tokio::select! {
            result = executor.execute(None, workflow, &mut memory) => {
              if let Err(ExecutionError::WorkflowFailed(reason)) = result {
                // handle Workflow failed errors with retries
                log::warn!("Execution gave WorkflowFailed error with: {}", reason);
                if retries < NUM_RETRIES {
                  retries += 1;
                  log::warn!("Retrying {}/{}", retries, NUM_RETRIES);
                  continue;
                }
              } else {
                return result.wrap_err("could not execute workflow");
              }
            },
            // normally the workflow has a timeout logic as well, but it doesnt work that well, and may get stuck
            _ = tokio::time::sleep(duration) => {
                // if we have retries left, log a warning and continue
                // note that other errors will be returned as is
                log::warn!("Execution timed out");
                if retries < NUM_RETRIES {
                  retries += 1;
                  log::warn!("Retrying {}/{}", retries, NUM_RETRIES);
                  continue;
                }
            }
        };
    }

    // all retries failed
    Err(eyre::eyre!(
        "Execution failed after {} retries",
        NUM_RETRIES
    ))
}
