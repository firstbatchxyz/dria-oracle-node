use alloy::eips::BlockNumberOrTag;
use eyre::{Context, Result};
use futures_util::StreamExt;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::DriaOracle;

mod request;
mod serve;
mod view;

impl DriaOracle {
    /// Starts the oracle node.
    pub(in crate::cli) async fn serve(
        &self,
        from_block: Option<BlockNumberOrTag>,
        to_block: Option<BlockNumberOrTag>,
        cancellation: CancellationToken,
    ) -> Result<()> {
        log::info!(
            "Started oracle as {} using models: {}",
            self.kinds
                .iter()
                .map(|k| k.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            self.workflows
                .models
                .iter()
                .map(|(_, m)| m.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        // check previous tasks if `from_block` is given
        if let Some(from_block) = from_block {
            tokio::select! {
                _ = cancellation.cancelled() => {
                    log::debug!("Cancellation signal received. Stopping...");
                    return Ok(());
                }
                result = self.process_tasks_within_range(from_block, to_block.clone().unwrap_or(BlockNumberOrTag::Latest)) => {
                    if let Err(e) = result {
                        log::error!("Could not handle previous tasks: {:?}", e);
                        log::warn!("Continuing anyways...");
                    }
                }
            }
        }

        if to_block.is_some() {
            // if there was a `to_block` specified, we are done at this point
            return Ok(());
        }

        // otherwise, we can continue with the event loop
        loop {
            // subscribe to new tasks
            log::info!("Subscribing to task events");
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
                                self.process_task_by_event(event).await
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
}
