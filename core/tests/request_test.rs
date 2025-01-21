#![cfg(feature = "anvil")]

//! Tests the request command, resulting in a task being created in the coordinator contract.
//!
//! 1. Requester buys some WETH, and it is approved within the request command.
//! 2. Requester requests a task with a given input, models, difficulty, num_gens, and num_vals.
//! 3. The task is created in the coordinator contract.

use alloy::primitives::{aliases::U40, utils::parse_ether};
use dkn_workflows::Model;
use dria_oracle::{DriaOracle, DriaOracleConfig};
use dria_oracle_contracts::{bytes_to_string, WETH};
use eyre::Result;

#[tokio::test]
async fn test_request() -> Result<()> {
    dotenvy::dotenv().unwrap();
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Off)
        .filter_module("dria_oracle", log::LevelFilter::Debug)
        .filter_module("request_test", log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    // task setup
    let (difficulty, num_gens, num_vals) = (1, 1, 1);
    let protocol = format!("test/{}", env!("CARGO_PKG_VERSION"));
    let models = vec![Model::GPT4Turbo];
    let input = "What is the result of 2 + 2?".to_string();

    // node setup
    let config = DriaOracleConfig::new_from_env()?;
    let node = DriaOracle::new(config).await?;

    // setup account & buy some WETH
    let requester = node.connect(node.anvil_new_funded_wallet(None).await?);
    let token = WETH::new(requester.addresses.token, &requester.provider);
    let _ = token.deposit().value(parse_ether("100")?).send().await?;

    // request a task, and see it in the coordinator
    let task_id = node.get_next_task_id().await?;
    requester
        .request_task(&input, models, difficulty, num_gens, num_vals, protocol)
        .await?;

    // get the task info
    let (request, _, _) = node.get_task(task_id).await?;
    assert_eq!(input, bytes_to_string(&request.input).unwrap());
    assert_eq!(difficulty, request.parameters.difficulty);
    assert_eq!(U40::from(num_gens), request.parameters.numGenerations);
    assert_eq!(U40::from(num_vals), request.parameters.numValidations);

    Ok(())
}
