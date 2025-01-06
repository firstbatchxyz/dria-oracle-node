mod commands;
use std::time::Duration;

use commands::Commands;

mod parsers;
use dkn_workflows::DriaWorkflowsConfig;
use parsers::*;

use crate::{DriaOracle, DriaOracleConfig};
use alloy::{eips::BlockNumberOrTag, primitives::B256};
use clap::Parser;
use eyre::{Context, Result};
use reqwest::Url;
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// RPC URL of the Ethereum node.
    #[arg(short, long, env = "RPC_URL", value_parser = parse_url)]
    rpc_url: Url,

    /// Ethereum wallet's secret (private) key.
    #[arg(short, long, env = "SECRET_KEY", value_parser = parse_secret_key)]
    secret_key: B256,
}

/// Main CLI entry point.
pub async fn cli() -> Result<()> {
    // default commands such as version and help exit at this point,
    // so we can do the node setup after this line
    let cli = Cli::parse();

    // store cli-parsed options
    let rpc_url = cli.rpc_url;
    let secret_key = cli.secret_key;

    // create node
    let config = DriaOracleConfig::new(&secret_key, rpc_url)
        .wrap_err("could not create oracle configuration")?
        .with_tx_timeout(Duration::from_secs(30)); // timeout is 30secs by default
    let node = DriaOracle::new(config)
        .await
        .wrap_err("could not create oracle node")?;
    log::info!("{}", node);
    log::info!("{}", node.addresses);

    match cli.command {
        Commands::Balance => node.display_balance().await?,
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
        Commands::Claim => node.claim_rewards().await?,
        Commands::Rewards => node.display_rewards().await?,
        Commands::Start {
            kinds,
            models,
            from,
        } => {
            let token = CancellationToken::new();

            // create a signal handler
            let termination_token = token.clone();
            let termination_handle = tokio::spawn(async move {
                wait_for_termination(termination_token).await.unwrap();
            });

            // launch node
            node.run_oracle(
                kinds,
                models,
                from.unwrap_or(BlockNumberOrTag::Latest),
                token,
            )
            .await?;

            // wait for handle
            if let Err(e) = termination_handle.await {
                log::error!("Error in termination handler: {}", e);
            }
        }
        Commands::View { task_id } => node.view_task(task_id).await?,
        Commands::Process {
            task_id,
            kinds,
            models,
        } => {
            node.process_task(&DriaWorkflowsConfig::new(models), &kinds, task_id)
                .await?
        }
        Commands::Tasks { from, to } => {
            node.view_task_events(
                from.unwrap_or(BlockNumberOrTag::Earliest),
                to.unwrap_or(BlockNumberOrTag::Latest),
            )
            .await?
        }
        Commands::Request {
            input,
            models,
            difficulty,
            num_gens,
            num_vals,
        } => {
            const PROTOCOL: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

            node.request_task(
                &input,
                models,
                difficulty,
                num_gens,
                num_vals,
                PROTOCOL.to_string(),
            )
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
