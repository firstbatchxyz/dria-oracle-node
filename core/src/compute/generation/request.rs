use alloy::primitives::Bytes;
use dkn_workflows::Workflow;
use eyre::Result;

use crate::compute::parse_downloadable;

/// A request with chat history.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ChatHistoryRequest {
    /// Task Id of which the output will act like history.
    pub history_id: usize,
    /// Message content.
    pub content: String,
}

/// An oracle request.
#[derive(Debug)]
pub enum GenerationRequest {
    /// A chat-history request, usually indicates a previous response to be continued upon.
    ChatHistory(ChatHistoryRequest),
    /// A Workflow object, can be executed directly.
    Workflow(Workflow),
    /// A plain string request, can be executed with a generation workflow.
    String(String),
}

impl GenerationRequest {
    /// Returns hte name of the request type, mostly for diagnostics.
    pub fn request_type(&self) -> &str {
        match self {
            Self::ChatHistory(_) => "chat",
            Self::Workflow(_) => "workflow",
            Self::String(_) => "string",
        }
    }

    /// Given an input of byte-slice, parses it into a valid request type.
    pub async fn try_parse_bytes(input_bytes: &Bytes) -> Result<Self> {
        let input_string = parse_downloadable(input_bytes).await?;
        log::debug!("Parsing input string: {}", input_string);
        Ok(Self::try_parse_string(input_string).await)
    }

    /// Given an input of string, parses it into a valid request type.
    pub async fn try_parse_string(input_string: String) -> Self {
        if let Ok(chat_input) = serde_json::from_str::<ChatHistoryRequest>(&input_string) {
            GenerationRequest::ChatHistory(chat_input)
        } else if let Ok(workflow) = serde_json::from_str::<Workflow>(&input_string) {
            GenerationRequest::Workflow(workflow)
        } else {
            GenerationRequest::String(input_string)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // only implemented for testing purposes
    // because `Workflow` does not implement `PartialEq`
    impl PartialEq for GenerationRequest {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (Self::ChatHistory(a), Self::ChatHistory(b)) => {
                    a.content == b.content && a.history_id == b.history_id
                }
                (Self::Workflow(_), Self::Workflow(_)) => true, // not implemented
                (Self::String(a), Self::String(b)) => a == b,
                _ => false,
            }
        }
    }

    #[tokio::test]
    async fn test_parse_request_string() {
        let request_str = "foobar";
        let entry = GenerationRequest::try_parse_bytes(&request_str.as_bytes().into()).await;
        assert_eq!(
            entry.unwrap(),
            GenerationRequest::String(request_str.into())
        );
    }

    #[tokio::test]
    async fn test_parse_request_arweave() {
        // contains the string: "\"Hello, Arweave!\""
        let arweave_key = serde_json::json!({
            "arweave": "Zg6CZYfxXCWYnCuKEpnZCYfy7ghit1_v4-BCe53iWuA"
        })
        .to_string();
        let expected_str = "\"Hello, Arweave!\"";

        let entry = GenerationRequest::try_parse_bytes(&arweave_key.into()).await;
        assert_eq!(
            entry.unwrap(),
            GenerationRequest::String(expected_str.into())
        );
    }

    #[tokio::test]
    async fn test_parse_request_chat() {
        let request = ChatHistoryRequest {
            history_id: 0,
            content: "foobar".to_string(),
        };
        let request_bytes = serde_json::to_vec(&request).unwrap();
        let entry = GenerationRequest::try_parse_bytes(&request_bytes.into()).await;
        assert_eq!(entry.unwrap(), GenerationRequest::ChatHistory(request));
    }

    #[tokio::test]
    async fn test_parse_request_workflow() {
        // contains a workflow
        let arweave_key = serde_json::json!({
            "arweave": "ALRD6i-Xm7xSyl5hF-Tc9WRvsc5C71_TzV3fh1PVgkw"
        })
        .to_string();
        let workflow = GenerationRequest::try_parse_bytes(&arweave_key.into())
            .await
            .unwrap();
        if let GenerationRequest::Workflow(_) = workflow {
            /* do nothing */
        } else {
            panic!("Expected workflow, got something else");
        }
    }
}
