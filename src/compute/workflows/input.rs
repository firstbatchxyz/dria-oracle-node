use alloy::primitives::{Bytes, U256};
use dkn_workflows::{Entry, Executor, Model, ProgramMemory, Workflow};
use eyre::{eyre, Context, Result};
use lazy_static::lazy_static;

use super::{chat::*, postprocess::*};
use crate::{
    bytes_to_string,
    data::{Arweave, OracleExternalData},
    DriaOracle,
};

lazy_static! {
    static ref GENERATION_WORKFLOW: Workflow = {
        serde_json::from_str(include_str!("presets/generation.json"))
            .expect("could not parse generation workflow")
    };
}

#[derive(Debug)]
pub enum RequestType {
    ChatHistory(ChatHistoryRequest),
    Workflow(Workflow),
    String(String),
}

impl RequestType {
    /// Given an input of byte-slice, parses it into a valid request type.
    pub async fn try_parse_bytes(input_bytes: &Bytes) -> Result<Self> {
        let input_string = Self::parse_downloadable(input_bytes).await?;
        if let Ok(chat_input) = serde_json::from_str::<ChatHistoryRequest>(&input_string) {
            Ok(RequestType::ChatHistory(chat_input))
        } else if let Ok(workflow) = serde_json::from_str::<Workflow>(&input_string) {
            Ok(RequestType::Workflow(workflow))
        } else {
            Ok(RequestType::String(input_string))
        }
    }

    /// Executes a request using the given model, and optionally a node.
    /// Returns the raw string output.
    pub async fn execute(&self, model: Model, node: Option<&DriaOracle>) -> Result<String> {
        log::debug!("Executing workflow with: {}", model);
        let mut memory = ProgramMemory::new();
        let executor = Executor::new(model);

        match self {
            Self::Workflow(workflow) => executor
                .execute(None, workflow, &mut memory)
                .await
                .wrap_err("could not execute worfklow input"),
            Self::String(input) => {
                let entry = Entry::String(input.clone());
                executor
                    .execute(Some(&entry), &GENERATION_WORKFLOW, &mut memory)
                    .await
                    .wrap_err("could not execute worfklow for string input")
            }
            Self::ChatHistory(chat_history) => {
                if let Some(node) = node {
                    // get history from blockchain
                    let history_task = node
                        .get_task_best_response(U256::from(chat_history.history_id))
                        .await?;

                    // parse it as chat history output
                    let history_str = Self::parse_downloadable(&history_task.output).await?;
                    let mut history =
                        serde_json::from_str::<Vec<ChatHistoryResponse>>(&history_str)?;

                    // execute the workflow
                    // TODO: add chat history to memory
                    let entry = Entry::String(chat_history.content.clone());
                    let output = executor
                        .execute(Some(&entry), &GENERATION_WORKFLOW, &mut memory)
                        .await
                        .wrap_err("could not execute chat worfklow")?;

                    // append user input & workflow output to chat history
                    history.push(ChatHistoryResponse::user(chat_history.content.clone()));
                    history.push(ChatHistoryResponse::assistant(output));

                    // return the stringified output
                    let out = serde_json::to_string(&history)
                        .wrap_err("could not serialize chat history")?;

                    Ok(out)
                } else {
                    Err(eyre!("node is required for chat history"))
                }
            }
        }
    }

    pub async fn post_process(output: String, protocol: &str) -> Result<(Bytes, Bytes, bool)> {
        match protocol.split('/').next().unwrap_or_default() {
            SwanPurchasePostProcessor::PROTOCOL => {
                SwanPurchasePostProcessor::new("<shop_list>", "</shop_list>").post_process(output)
            }
            _ => IdentityPostProcessor.post_process(output),
        }
    }

    /// Parses a given bytes input to a string, and if it is a storage key identifier it automatically
    /// downloads the data from Arweave.
    pub async fn parse_downloadable(input_bytes: &Bytes) -> Result<String> {
        // first, convert to string
        let mut input_string = bytes_to_string(input_bytes)?;

        // then, check storage
        if Arweave::is_key(input_string.clone()) {
            // if its a txid, we download the data and parse it again
            let input_bytes_from_arweave = Arweave::default()
                .get(input_string)
                .await
                .wrap_err("could not download from Arweave")?;

            // convert the input to string
            input_string = bytes_to_string(&input_bytes_from_arweave)?;
        }

        Ok(input_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // only implemented for testing purposes
    // because Workflow and ChatHistory do not implement PartialEq
    impl PartialEq for RequestType {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (Self::ChatHistory(_), Self::ChatHistory(_)) => true,
                (Self::Workflow(_), Self::Workflow(_)) => true,
                (Self::String(a), Self::String(b)) => a == b,
                _ => false,
            }
        }
    }

    #[tokio::test]
    async fn test_parse_input_string() {
        let input_str = "foobar";
        let entry = RequestType::try_parse_bytes(&input_str.as_bytes().into()).await;
        assert_eq!(entry.unwrap(), RequestType::String(input_str.into()));
    }

    #[tokio::test]
    async fn test_parse_input_arweave() {
        // contains the string: "\"Hello, Arweave!\""
        // hex for: Zg6CZYfxXCWYnCuKEpnZCYfy7ghit1_v4-BCe53iWuA
        let arweave_key = "660e826587f15c25989c2b8a1299d90987f2ee0862b75fefe3e0427b9de25ae0";
        let expected_str = "\"Hello, Arweave!\"";

        let entry = RequestType::try_parse_bytes(&arweave_key.as_bytes().into()).await;
        assert_eq!(entry.unwrap(), RequestType::String(expected_str.into()));
    }

    #[tokio::test]
    async fn test_parse_input_workflow() {
        let workflow_str = include_str!("presets/generation.json");
        let expected_workflow = serde_json::from_str::<Workflow>(&workflow_str).unwrap();

        let entry = RequestType::try_parse_bytes(&workflow_str.as_bytes().into()).await;
        assert_eq!(entry.unwrap(), RequestType::Workflow(expected_workflow));
    }

    #[tokio::test]
    #[ignore = "run this manually"]
    async fn test_ollama_generation() {
        dotenvy::dotenv().unwrap();
        let input = RequestType::String("What is the result of 2 + 2?".to_string());
        let output = input.execute(Model::Llama3_1_8B, None).await.unwrap();

        println!("Output:\n{}", output);
        assert!(output.contains('4'));
    }

    #[tokio::test]
    #[ignore = "run this manually"]
    async fn test_openai_generation() {
        dotenvy::dotenv().unwrap();
        let input = RequestType::String("What is the result of 2 + 2?".to_string());
        let output = input.execute(Model::GPT4Turbo, None).await.unwrap();

        println!("Output:\n{}", output);
        assert!(output.contains('4'));
    }
}
