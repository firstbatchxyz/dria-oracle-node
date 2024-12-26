use dria_oracle::{DriaOracle, DriaOracleConfig};
use eyre::Result;

#[tokio::test]
async fn test_whitelist() -> Result<()> {
    dotenvy::dotenv().unwrap();

    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Off)
        .filter_module("dria_oracle", log::LevelFilter::Debug)
        .filter_module("whitelist_test", log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    // node setup
    let config = DriaOracleConfig::new_from_env()?;
    let node = DriaOracle::new(config).await?;

    // setup random account
    let account = node.connect(node.anvil_funded_wallet(None).await?);

    // whitelist validator with impersonation
    log::info!("Whitelisting validator");
    node.anvil_whitelist_registry(account.address()).await?;
    assert!(node.is_whitelisted(account.address()).await?);

    Ok(())
}
