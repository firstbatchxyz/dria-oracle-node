// use alloy::eips::BlockNumberOrTag;
use alloy::eips::BlockNumberOrTag;
use eyre::Result;
use std::{env, path::PathBuf};
use tokio_util::sync::CancellationToken;

mod commands;
use commands::Commands;

mod parsers;
use parsers::*;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to the .env file
    #[arg(short, long, default_value = "./.env")]
    pub env: PathBuf,

    /// Enable debug-level logs
    #[arg(short, long)]
    pub debug: bool,
}

impl Cli {
    pub fn read_secret_key() -> Result<alloy::primitives::B256> {
        let key = env::var("SECRET_KEY")?;
        parse_secret_key(&key)
    }

    pub fn read_rpc_url() -> Result<reqwest::Url> {
        let url = env::var("RPC_URL")?;
        parse_url(&url)
    }

    pub fn read_tx_timeout() -> Result<u64> {
        let timeout = env::var("TX_TIMEOUT_SECS").unwrap_or("30".to_string());
        timeout.parse().map_err(Into::into)
    }
}

/// Handles a given CLI command, using the provided node.
pub async fn handle_command(command: Commands, mut node: crate::DriaOracle) -> Result<()> {
    match command {
        Commands::Balance => node.display_balance().await?,
        Commands::Claim => node.claim_rewards().await?,
        Commands::Rewards => node.display_rewards().await?,
        Commands::Serve {
            task_id,
            kinds,
            models,
            from,
            to,
        } => {
            let token = CancellationToken::new();
            node.prepare_oracle(kinds, models).await?;

            // create a signal handler
            let termination_token = token.clone();
            let termination_handle = tokio::spawn(async move {
                wait_for_termination(termination_token).await.unwrap();
            });

            // launch node
            node.start_oracle(from, to, token).await?;

            // wait for handle
            if let Err(e) = termination_handle.await {
                log::error!("Error in termination handler: {}", e);
            }

            node.process_task_by_id(task_id).await?
        }
        Commands::View { task_id, from, to } => {
            if let Some(task_id) = task_id {
                node.view_task(task_id).await?
            } else {
                node.view_task_events(
                    from.unwrap_or(BlockNumberOrTag::Earliest),
                    to.unwrap_or(BlockNumberOrTag::Latest),
                )
                .await?
            }
        }
        Commands::Register { kinds } => {
            for kind in kinds {
                node.register(kind).await?
            }
        }
        Commands::Unregister { kinds } => {
            for kind in kinds {
                node.unregister(kind).await?;
            }
        }
        Commands::Registrations => node.display_registrations().await?,
        Commands::Request {
            input,
            models,
            difficulty,
            num_gens,
            num_vals,
            protocol,
        } => {
            node.request_task(&input, models, difficulty, num_gens, num_vals, protocol)
                .await?
        }
    };

    Ok(())
}

/// Waits for various termination signals, and cancels the given token when the signal is received.
async fn wait_for_termination(cancellation: CancellationToken) -> Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;
        tokio::select! {
            _ = sigterm.recv() => log::warn!("Recieved SIGTERM"),
            _ = sigint.recv() => log::warn!("Recieved SIGINT"),
            _ = cancellation.cancelled() => {
                // no need to wait if cancelled anyways
                // although this is not likely to happen
                return Ok(());
            }
        };

        cancellation.cancel();
    }

    #[cfg(not(unix))]
    {
        log::error!("No signal handling for this platform: {}", env::consts::OS);
        cancellation.cancel();
    }

    log::info!("Terminating the application...");

    Ok(())
}
