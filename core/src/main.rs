use std::env;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let dotenv_result = dotenvy::dotenv();

    let default_log = if env::var("DEBUG").is_ok() {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    env_logger::builder()
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .filter(None, log::LevelFilter::Off)
        .filter_module("dria_oracle", default_log)
        .filter_module("dkn_workflows", default_log)
        .parse_default_env()
        .init();

    // log about env usage
    match dotenv_result {
        Ok(path) => log::info!("Loaded .env file at: {}", path.display()),
        Err(e) => log::warn!("Could not load .env file: {}", e),
    }

    // launch CLI
    dria_oracle::cli().await?;

    log::info!("Bye!");
    Ok(())
}
