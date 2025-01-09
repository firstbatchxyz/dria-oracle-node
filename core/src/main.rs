use std::time::Duration;

use clap::Parser;
use dria_oracle::{Cli, DriaOracle, DriaOracleConfig};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // default commands such as version and help exit at this point
    let cli = Cli::parse();

    // read env w.r.t cli argument, defaults to `.env`
    let dotenv_result = dotenvy::from_path(&cli.env);

    // init env logger
    let log_level = match cli.debug {
        true => log::LevelFilter::Debug,
        false => log::LevelFilter::Info,
    };
    env_logger::builder()
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .filter(None, log::LevelFilter::Off)
        .filter_module("dria_oracle", log_level)
        .filter_module("dkn_workflows", log_level)
        .filter_module("dria_oracle_contracts", log_level)
        .filter_module("dria_oracle_storage", log_level)
        .parse_default_env()
        .init();

    // log about env usage after env logger init is executed
    match dotenv_result {
        Ok(_) => log::info!("Loaded .env file at: {}", cli.env.display()),
        Err(e) => log::warn!("Could not load .env file: {}", e),
    }

    // read required env variables
    let secret_key = Cli::read_secret_key()?;
    let rpc_url = Cli::read_rpc_url()?;
    let tx_timeout = Cli::read_tx_timeout()?;

    // create config
    let config = DriaOracleConfig::new(&secret_key, rpc_url)?
        .with_tx_timeout(Duration::from_secs(tx_timeout));

    // create node
    let node = DriaOracle::new(config).await?;
    log::info!("{}", node);
    log::info!("{}", node.addresses);

    // handle cli command
    dria_oracle::handle_command(cli.command, node).await?;

    log::info!("Bye!");
    Ok(())
}
