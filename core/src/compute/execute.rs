use core::time::Duration;
use dkn_workflows::{Executor, Model, ProgramMemory, Workflow};
use eyre::Context;

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
    const NUM_RETRIES: usize = 3;

    let mut retries = 0;
    let executor = Executor::new(model);
    while retries < NUM_RETRIES {
        let mut memory = ProgramMemory::new();
        tokio::select! {
            result = executor.execute(None, &workflow, &mut memory) => {
              return result.wrap_err("could not execute workflow");
            },
            // normally the workflow has a timeout logic as well, but it doesnt work that well, and may get stuck
            _ = tokio::time::sleep(duration) => {
                // if we have retries left, log a warning and continue
                // note that other errors will be returned as is
                if retries < NUM_RETRIES {
                  retries += 1;
                  log::warn!("Execution timed out, retrying {}/{}", retries + 1, NUM_RETRIES);
                  continue;
                }
            }
        };
    }

    // all retries failed
    return Err(eyre::eyre!(
        "Execution timed out after {} retries",
        NUM_RETRIES
    ));
}
