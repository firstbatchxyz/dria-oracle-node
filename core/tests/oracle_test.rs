//! Test the oracle with a simple 2 + 2 question.
//!
//! ```sh
//! cargo test --package dria-oracle --test oracle_test --all-features -- test_oracle_two_plus_two --exact --show-output
//! ```
#![cfg(feature = "anvil")]

use alloy::{eips::BlockNumberOrTag, primitives::utils::parse_ether};
use dkn_workflows::Model;
use dria_oracle::{handle_request, DriaOracle, DriaOracleConfig};
use dria_oracle_contracts::{bytes_to_string, string_to_bytes, OracleKind, TaskStatus, WETH};
use eyre::Result;

#[tokio::test]
async fn test_oracle_two_plus_two() -> Result<()> {
    dotenvy::dotenv().unwrap();
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Off)
        .filter_module("dria_oracle", log::LevelFilter::Debug)
        .filter_module("oracle_test", log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    // task setup
    let model = Model::GPT4o;
    let difficulty = 1;
    let models = string_to_bytes(model.to_string());
    let protocol = format!("test/{}", env!("CARGO_PKG_VERSION"));
    let input = string_to_bytes("What is the result of 2 + 2?".to_string());

    // node setup
    let config = DriaOracleConfig::new_from_env()?;
    let node = DriaOracle::new(config).await?;

    // setup accounts
    let requester = node.connect(node.anvil_new_funded_wallet(None).await?);
    let mut generator = node.connect(node.anvil_new_funded_wallet(None).await?);
    let mut validator = node.connect(node.anvil_new_funded_wallet(None).await?);

    // buy some WETH for all people
    let amount = parse_ether("100")?;
    for node in [&requester, &generator, &validator] {
        let token = WETH::new(*node.token.address(), &node.provider);
        let balance_before = node.get_token_balance(node.address()).await?;

        let call = token.deposit().value(amount);
        let _ = call.send().await?.get_receipt().await?;

        let balance_after = node.get_token_balance(node.address()).await?;
        assert!(balance_after.amount > balance_before.amount);
    }

    // whitelist validator with impersonation
    node.anvil_whitelist_registry(validator.address()).await?;
    assert!(node.is_whitelisted(validator.address()).await?);

    // register & prepare generator oracle
    generator.register(OracleKind::Generator).await?;
    generator
        .prepare_oracle(vec![OracleKind::Generator], vec![model.clone()])
        .await?;
    assert!(generator.is_registered(OracleKind::Generator).await?);

    // register & prepare validator oracle
    validator.register(OracleKind::Validator).await?;
    validator
        .prepare_oracle(vec![OracleKind::Validator], vec![model])
        .await?;
    assert!(validator.is_registered(OracleKind::Validator).await?);

    // approve some tokens for the coordinator from requester
    requester
        .approve(*node.coordinator.address(), amount)
        .await?;

    // make a request with just one generation and validation request
    let request_receipt = requester
        .request(input, models, difficulty, 1, 1, protocol)
        .await?;

    // handle generation by reading the latest event
    let tasks = node
        .get_tasks_in_range(
            request_receipt.block_number.unwrap(),
            BlockNumberOrTag::Latest,
        )
        .await?;
    assert!(tasks.len() == 1);
    let event = tasks[0].0.clone();
    let task_id = event.taskId;
    assert_eq!(event.statusBefore, TaskStatus::None as u8);
    assert_eq!(event.statusAfter, TaskStatus::PendingGeneration as u8);
    let generation_receipt = handle_request(
        &generator,
        TaskStatus::PendingGeneration,
        event.taskId,
        event.protocol,
    )
    .await?
    .unwrap();

    // handle validation by reading the latest event
    let tasks = node
        .get_tasks_in_range(
            generation_receipt.block_number.unwrap(),
            BlockNumberOrTag::Latest,
        )
        .await?;
    assert!(tasks.len() == 1);
    let (event, _) = tasks.into_iter().next().unwrap();
    assert_eq!(event.taskId, task_id);
    assert_eq!(event.statusBefore, TaskStatus::PendingGeneration as u8);
    assert_eq!(event.statusAfter, TaskStatus::PendingValidation as u8);
    let validation_receipt = handle_request(
        &validator,
        TaskStatus::PendingValidation,
        event.taskId,
        event.protocol,
    )
    .await?
    .unwrap();

    let tasks = node
        .get_tasks_in_range(
            validation_receipt.block_number.unwrap(),
            BlockNumberOrTag::Latest,
        )
        .await?;
    assert!(tasks.len() == 1);
    let (event, _) = tasks.into_iter().next().unwrap();
    assert_eq!(event.taskId, task_id);
    assert_eq!(event.statusBefore, TaskStatus::PendingValidation as u8);
    assert_eq!(event.statusAfter, TaskStatus::Completed as u8);

    // get responses
    let responses = node.coordinator.getResponses(task_id).call().await?._0;
    assert_eq!(responses.len(), 1);
    let response = responses.into_iter().next().unwrap();
    let output_string = bytes_to_string(&response.output)?;
    assert!(output_string.contains("4"), "output must contain 4");
    assert!(!response.score.is_zero(), "score must be non-zero");
    log::debug!("Output: {}", output_string);

    Ok(())
}
