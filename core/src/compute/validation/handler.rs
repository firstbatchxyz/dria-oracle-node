use crate::{compute::parse_downloadable, mine_nonce, DriaOracle};
use alloy::{primitives::U256, rpc::types::TransactionReceipt};
use dkn_workflows::Model;
use dria_oracle_storage::ArweaveStorage;
use eyre::{eyre, Context, Result};

use super::execute::execute_validations;

/// Handles a validation request.
pub async fn handle_validation(
    node: &DriaOracle,
    task_id: U256,
) -> Result<Option<TransactionReceipt>> {
    log::info!("Handling validation task {}", task_id);

    // check if already responded as generator, because we cant validate our own answer
    log::debug!("Checking if we are a generator for this task");
    let responses = node.coordinator.getResponses(task_id).call().await?._0;
    if responses.iter().any(|r| r.responder == node.address()) {
        log::debug!(
            "Cant validate {} with your own generation response",
            task_id
        );
        return Ok(None);
    }

    // check if we have validated anyways
    log::debug!("Checking if we have validated already");
    let validations = node.coordinator.getValidations(task_id).call().await?._0;
    if validations.iter().any(|v| v.validator == node.address()) {
        return Err(eyre!("Already validated {}", task_id));
    }

    // fetch the request from contract
    log::debug!("Fetching the task request");
    let request = node.coordinator.requests(task_id).call().await?;

    // fetch each generation response & download its metadata
    log::debug!("Fetching response messages");
    let responses = node.coordinator.getResponses(task_id).call().await?._0;
    let mut generations = Vec::new();
    for response in responses {
        let metadata_str = parse_downloadable(&response.metadata).await?;
        generations.push(metadata_str);
    }
    let input = parse_downloadable(&request.input).await?;

    // validate each response
    log::debug!("Computing validation scores");
    let model = Model::GPT4o; // all validations use Gpt 4o
    let validations = execute_validations(input, generations, model).await?;
    let scores = validations
        .iter()
        .map(|v| v.final_score_as_solidity_type())
        .collect::<Vec<_>>();
    let metadata =
        serde_json::to_string(&validations).wrap_err("could not serialize validations")?;
    log::debug!("Validation metadata:\n{}", metadata);

    // uploading to storage
    log::debug!("Uploading metadata to storage");
    let arweave = ArweaveStorage::new_from_env()?;
    let metadata = arweave.put_if_large(metadata.into()).await?;

    // mine nonce
    log::debug!("Mining nonce for task");
    let nonce = mine_nonce(
        request.parameters.difficulty,
        &request.requester,
        &node.address(),
        &request.input,
        &task_id,
    )
    .nonce;

    // respond
    log::debug!("Responding with validation");
    let tx_receipt = node
        .respond_validation(task_id, scores, metadata, nonce)
        .await?;
    Ok(Some(tx_receipt))
}
