use alloy::{
    eips::BlockNumberOrTag,
    primitives::{utils::parse_ether, Bytes},
};
use dkn_oracle::{
    bytes_to_string, commands, handle_request, DriaOracle, DriaOracleConfig, ModelConfig,
    OracleKind, TaskStatus, WETH,
};
use eyre::Result;
use ollama_workflows::Model;

#[tokio::test]
#[ignore = "run manually"]
async fn test_oracle_request() -> Result<()> {
    // task setup
    let difficulty = 1;
    let models = Bytes::from_static(b"gpt-3.5-turbo");
    let input = Bytes::from_static(b"What is the result of 2 + 2?");
    println!("Input: {}", bytes_to_string(&input)?);

    // node setup
    let model_config = ModelConfig::new(vec![Model::GPT3_5Turbo]);
    let config = DriaOracleConfig::new_from_env()?;
    let (node, _anvil) = DriaOracle::anvil_new(config).await?;

    // setup accounts
    let requester = node.connect(node.anvil_funded_wallet(None).await?);
    let generator = node.connect(node.anvil_funded_wallet(None).await?);
    let validator = node.connect(node.anvil_funded_wallet(None).await?);

    // buy some WETH for all people
    let amount = parse_ether("100").unwrap();
    for node in [&requester, &generator, &validator] {
        let token = WETH::new(node.contract_addresses.token, &node.provider);
        let balance_before = node.get_token_balance(node.address()).await?;
        let _ = token.deposit().value(amount).send().await?;
        let balance_after = node.get_token_balance(node.address()).await?;
        assert!(balance_after.amount > balance_before.amount);
    }

    // register generator oracle
    commands::register(&generator, OracleKind::Generator).await?;
    assert!(generator.is_registered(OracleKind::Generator).await?);

    // register validator oracle
    commands::register(&validator, OracleKind::Validator).await?;
    assert!(validator.is_registered(OracleKind::Validator).await?);

    // approve some tokens for the coordinator from requester
    requester
        .approve(node.contract_addresses.coordinator, amount)
        .await?;

    // make a request with just one generation and validation request
    let request_receipt = requester.request(input, models, difficulty, 1, 1).await?;

    // handle generation by reading the latest event
    let tasks = node
        .get_tasks(
            request_receipt.block_number.unwrap(),
            BlockNumberOrTag::Latest,
        )
        .await?;
    assert!(tasks.len() == 1);
    let (event, _) = tasks.into_iter().next().unwrap();
    let task_id = event.taskId;
    assert_eq!(event.statusBefore, TaskStatus::None as u8);
    assert_eq!(event.statusAfter, TaskStatus::PendingGeneration as u8);
    let generation_receipt =
        handle_request(&generator, &[OracleKind::Generator], &model_config, event)
            .await?
            .unwrap();

    // handle validation by reading the latest event
    let tasks = node
        .get_tasks(
            generation_receipt.block_number.unwrap(),
            BlockNumberOrTag::Latest,
        )
        .await?;
    assert!(tasks.len() == 1);
    let (event, _) = tasks.into_iter().next().unwrap();
    assert_eq!(event.taskId, task_id);
    assert_eq!(event.statusBefore, TaskStatus::PendingGeneration as u8);
    assert_eq!(event.statusAfter, TaskStatus::PendingValidation as u8);
    let validation_receipt =
        handle_request(&validator, &[OracleKind::Validator], &model_config, event)
            .await?
            .unwrap();

    let tasks = node
        .get_tasks(
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
    let responses = node.get_task_responses(task_id).await?;
    assert_eq!(responses.len(), 1);
    let response = responses.into_iter().next().unwrap();
    println!("Output: {}", bytes_to_string(&response.output)?);
    println!("Score: {}", response.score);

    Ok(())
}
