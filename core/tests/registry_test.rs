use eyre::Result;

#[tokio::test]
async fn test_registry() -> Result<()> {
    dotenvy::dotenv().unwrap();
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Off)
        .filter_module("dria_oracle", log::LevelFilter::Debug)
        .filter_module("request_test", log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    // TODO: !!!
    //     let config = DriaOracleConfig::new_from_env()?;
    //     let (node, _anvil) = DriaOracle::anvil_new(config).await?;
    //     assert!(node.provider.get_block_number().await? > 1);

    //     // tries to register if registered, or opposite, to trigger an error
    //     const KIND: OracleKind = OracleKind::Generator;
    //     let result = if node.is_registered(KIND).await? {
    //         node.register_kind(KIND).await
    //     } else {
    //         node.unregister_kind(KIND).await
    //     };
    //     assert!(result.is_err());

    //     // both errors include the node address in their message, which we look for here:
    //     let err = result.unwrap_err();
    //     err.to_string().contains(&node.address().to_string());

    Ok(())
}
