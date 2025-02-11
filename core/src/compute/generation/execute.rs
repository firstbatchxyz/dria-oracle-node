use std::time::Duration;

use alloy::primitives::U256;
use dkn_workflows::{MessageInput, Model};
use eyre::{eyre, Context, Result};

use super::request::GenerationRequest;
use super::workflow::*;

use crate::compute::execute_workflow_with_timedout_retries;
use crate::compute::parse_downloadable;
use crate::DriaOracle;

/// Executes a request using the given model, and optionally a node.
/// Returns the raw string output.
pub async fn execute_generation(
    request: &GenerationRequest,
    model: Model,
    node: Option<&DriaOracle>,
) -> Result<String> {
    log::debug!(
        "Executing {} generation request with: {}",
        request.request_type(),
        model
    );

    match request {
        // workflows are executed directly without any prompts
        // as we expect their memory to be pre-filled
        GenerationRequest::Workflow(workflow) => {
            let duration = Duration::from_secs(workflow.get_config().max_time);
            execute_workflow_with_timedout_retries(&workflow, model, duration).await
        }

        // string requests are used with the generation workflow with a given prompt
        GenerationRequest::String(input) => {
            let (workflow, duration) = make_generation_workflow(input.clone())?;
            execute_workflow_with_timedout_retries(&workflow, model, duration).await
        }

        // chat history requests are used with the chat workflow
        // and the existing history is fetched & parsed from previous requests
        GenerationRequest::ChatHistory(chat_request) => {
            let mut history = if chat_request.history_id == 0 {
                // if task id is zero, there is no prior history
                Vec::new()
            } else if let Some(node) = node {
                let history_id = U256::from(chat_request.history_id);
                // if task id is non-zero, we need the node to get the history
                // first make sure that next-task-id is larger than the history
                if history_id >= node.coordinator.nextTaskId().call().await?._0 {
                    return Err(eyre!(
                        "chat history cant exist as its larger than the latest task id"
                    ));
                }

                let history_task = node
                    .coordinator
                    .getBestResponse(history_id)
                    .call()
                    .await
                    .wrap_err("could not get chat history task from contract")?
                    ._0;

                // parse it as chat history output
                let history_str = parse_downloadable(&history_task.output).await?;

                // if its a previous message array, we can parse it directly
                if let Ok(messages) = serde_json::from_str::<Vec<MessageInput>>(&history_str) {
                    messages
                } else {
                    // otherwise, we can fallback to fetching input manually and creating a new history on-the-fly
                    let request = node.coordinator.requests(history_id).call().await?;
                    let input = parse_downloadable(&request.input).await?;

                    // create a new history with the input
                    vec![
                        MessageInput::new_user_message(input),
                        MessageInput::new_assistant_message(history_str),
                    ]
                }
            } else {
                return Err(eyre!("node is required for chat history"));
            };

            // prepare the workflow with chat history
            let (workflow, duration) =
                make_chat_workflow(history.clone(), chat_request.content.clone())?;
            let output = execute_workflow_with_timedout_retries(&workflow, model, duration).await?;

            // append user input to chat history
            history.push(MessageInput::new_assistant_message(output));

            // return the stringified output
            serde_json::to_string(&history).wrap_err("could not serialize chat history")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute::generation::request::{ChatHistoryRequest, GenerationRequest};
    #[tokio::test]
    #[ignore = "run this manually"]
    async fn test_ollama_generation() {
        dotenvy::dotenv().unwrap();
        let request = GenerationRequest::String("What is the result of 2 + 2?".to_string());
        let output = execute_generation(&request, Model::Llama3_1_8B, None)
            .await
            .unwrap();

        println!("Output:\n{}", output);
        assert!(output.contains('4'));
    }

    #[tokio::test]
    #[ignore = "run this manually"]
    async fn test_openai_generation() {
        dotenvy::dotenv().unwrap();
        let request = GenerationRequest::String("What is the result of 2 + 2?".to_string());
        let output = execute_generation(&request, Model::GPT4Turbo, None)
            .await
            .unwrap();

        println!("Output:\n{}", output);
        assert!(output.contains('4'));
    }

    #[tokio::test]
    #[ignore = "run this manually"]
    async fn test_openai_chat() {
        dotenvy::dotenv().unwrap();
        let request = ChatHistoryRequest {
            history_id: 0,
            content: "What is 2+2?".to_string(),
        };
        let request_bytes = serde_json::to_vec(&request).unwrap();
        let request = GenerationRequest::try_parse_bytes(&request_bytes.into())
            .await
            .unwrap();
        let output = execute_generation(&request, Model::GPT4Turbo, None)
            .await
            .unwrap();

        println!("Output:\n{}", output);
        assert!(output.contains('4'));
    }

    #[tokio::test]
    #[ignore = "run this manually"]
    async fn test_workflow_on_arweave() {
        // cargo test --package dria-oracle --lib --all-features -- compute::generation::execute::tests::test_raw_workflow --exact --show-output --ignored
        dotenvy::dotenv().unwrap();

        let contract_result = hex_literal::hex!("7b2261727765617665223a223658797a572d71666e7670756b787344535a444b2d4f514a6e715a686b62703044624e4e6649696c7a706f227d");
        let request = GenerationRequest::try_parse_bytes(&contract_result.into())
            .await
            .unwrap();
        let output = execute_generation(&request, Model::GPT4o, None)
            .await
            .unwrap();

        println!("{}", output);
    }
}
