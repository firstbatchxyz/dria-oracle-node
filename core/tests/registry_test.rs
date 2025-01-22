#![cfg(feature = "anvil")]

use dria_oracle::{DriaOracle, DriaOracleConfig};
use dria_oracle_contracts::OracleKind;
use eyre::Result;

#[tokio::test]
async fn test_registry() -> Result<()> {
    dotenvy::dotenv().unwrap();
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Off)
        .filter_module("dria_oracle", log::LevelFilter::Debug)
        .filter_module("registry_test", log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    let config = DriaOracleConfig::new_from_env()?;
    let node = DriaOracle::new(config).await?;

    // tries to register if registered, or opposite, to trigger an error
    const KIND: OracleKind = OracleKind::Generator;
    let result = if node.is_registered(KIND).await? {
        log::info!("Oracle is registered, we will try to register again to get an error.");
        node.register_kind(KIND).await
    } else {
        log::info!("Oracle is not registered, we will try to unregister again to get an error.");
        node.unregister_kind(KIND).await
    };
    assert!(result.is_err());

    // both errors include the node address in their message, which we look for here:
    let err = result.unwrap_err();
    err.to_string().contains(&node.address().to_string());

    Ok(())
}
