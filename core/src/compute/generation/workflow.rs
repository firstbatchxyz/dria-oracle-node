use std::time::Duration;

use dkn_workflows::{MessageInput, Workflow};
use serde_json::json;

const DEFAULT_MAX_TIME: u64 = 50;
const DEFAULT_MAX_STEPS: u64 = 10;

/// Creates a generation workflow with the given input.
///
/// It is an alias for `make_chat_workflow` with a single message alone.
pub fn make_generation_workflow(input: String) -> Result<(Workflow, Duration), serde_json::Error> {
    make_chat_workflow(Vec::new(), input, None, None)
}

/// Creates a chat workflow with the given input.
///
/// `messages` is the existing message history, which will be used as context for the `input` message.
pub fn make_chat_workflow(
    mut messages: Vec<MessageInput>,
    input: String,
    max_time_sec: Option<u64>,
    max_steps: Option<u64>,
) -> Result<(Workflow, Duration), serde_json::Error> {
    // add the new input to the message history as a user message
    messages.push(MessageInput::new_user_message(input));

    // we do like this in-case a dynamic assign is needed
    let max_time_sec = max_time_sec.unwrap_or(DEFAULT_MAX_TIME);
    let max_steps = max_steps.unwrap_or(DEFAULT_MAX_STEPS);

    let workflow = json!({
        "config": {
            "max_steps": max_steps,
            "max_time": max_time_sec,
            "tools": [""]
        },
        "tasks": [
            {
                "id": "A",
                "name": "Generate with history",
                "description": "Expects an array of messages for generation",
                "operator": "generation",
                "messages": messages,
                "outputs": [
                    {
                        "type": "write",
                        "key": "result",
                        "value": "__result"
                    }
                ]
            },
            {
                "id": "__end",
                "operator": "end",
                "messages": [{ "role": "user", "content": "End of the task" }],
            }
        ],
        "steps": [ { "source": "A", "target": "__end" } ],
        "return_value": {
            "input": {
                "type": "read",
                "key": "result"
            }
        }
    });

    let workflow = serde_json::from_value(workflow)?;

    Ok((workflow, Duration::from_secs(max_time_sec)))
}
