use std::time::Duration;

use dkn_workflows::Workflow;
use serde_json::json;

pub fn make_validation_workflow(
    instruction: String,
    mut generations: Vec<String>,
) -> Result<(Workflow, Duration), serde_json::Error> {
    // workflow processes the array in reverse order, so we reverse the input outside
    // to get the correct order in results
    generations.reverse();

    let max_time_sec = (generations.len() as u64) * 5 + 10; // we need at most few seconds per generation, plus some leeway here

    let workflow = json!({
        "config": {
            "max_steps": generations.len() + 5, // we need one step per generation, plus some leeway here
            "max_time": max_time_sec,
            "tools": ["ALL"],
            "max_tokens": 1000 // max tokens do not need to change
        },
        "external_memory": {
            "instruction": instruction,
            "generations": generations
        },
        "tasks": [
            {
                "id": "evaluate",
                "name": "Task",
                "description": "Task Description",
                "messages": [
                {
                    "role": "user",
                    "content": "\n# Instruction Following Assessment\n\nEvaluate alignment between output and intent. Assess understanding of task goal and restrictions.\n\n**Instruction Components**: Task Goal (intended outcome), Restrictions (text styles, formats, or designated methods, etc).\n\n**Scoring**: Rate outputs 1 to 5:\n\n1. **Irrelevant**: No alignment.\n\n2. **Partial Focus**: Addresses one aspect poorly.\n\n3. **Partial Compliance**:\n\n- (1) Meets goal or restrictions, neglecting other.\n\n- (2) Acknowledges both but slight deviations.\n\n4. **Almost There**: Near alignment, minor deviations.\n\n5. **Comprehensive Compliance**: Fully aligns, meets all requirements.\n\n## Format:\n\n### Input\n\nInstruction: Write a compelling product description  \n\nTexts:\n\n<text 1> The sleek smartphone features a 6.1\" display\n\n<text 2> Buy this phone it has good screen\n\n<text 3> Experience crystal-clear visuals on the stunning 6.1\" OLED display\n\n### Output\n\n### Output for Text 1\n\n{\n  \"score\": 3,\n  \"rationale\": \"Basic description with specs but lacks compelling language.\"\n}\n\n### Output for Text 2\n\n{\n  \"score\": 1,\n  \"rationale\": \"Too informal and vague. Missing specific details.\"\n}\n\n### Output for Text 3\n\n{\n  \"score\": 5,\n  \"rationale\": \"Uses engaging language. Includes specific features. Creates desire.\"\n}\n\n---\n  \n## Annotation\n\n### Input\n\nInstruction: {{instruction}}\n\nTexts:\n\n{{generations}}\n\n### Output"
                }
                ],
                "schema": "{\"properties\": {\"instruction_following\": {\"description\": \"The score for the instruction following ability.\", \"gte\": 1, \"lte\": 5, \"title\": \"Instruction Following\", \"type\": \"integer\"}, \"truthfulness\": {\"description\": \"The score for the truthfulness of the response.\", \"gte\": 1, \"lte\": 5, \"title\": \"Truthfulness\", \"type\": \"integer\"}, \"helpfulness\": {\"description\": \"The score for the helpfulness of the response.\", \"gte\": 1, \"lte\": 5, \"title\": \"Helpfulness\", \"type\": \"integer\"}, \"final_score\": {\"description\": \"The overall score of the evaluation.\", \"gte\": 1, \"lte\": 5, \"title\": \"Final Score\", \"type\": \"integer\"}, \"rationale\": {\"description\": \"The rationale of the evaluation.\", \"title\": \"Rationale\", \"type\": \"string\"}}, \"required\": [\"instruction_following\", \"truthfulness\", \"helpfulness\", \"final_score\", \"rationale\"], \"title\": \"Evaluation\", \"type\": \"object\", \"additionalProperties\": false}",
                "inputs": [
                {
                    "name": "instruction",
                    "value": {
                    "type": "read",
                    "key": "instruction"
                    },
                    "required": true
                },
                {
                    "name": "generations",
                    "value": {
                    "type": "pop",
                    "key": "generations"
                    },
                    "required": true
                }
                ],
                "operator": "generation",
                "outputs": [
                {
                    "type": "push",
                    "key": "search_result",
                    "value": "__result"
                }
                ]
            },
            {
                "id": "_end",
                "name": "Task",
                "description": "Task Description",
                "messages": [
                {
                    "role": "user",
                    "content": ""
                }
                ],
                "operator": "end"
            }
        ],
        "steps": [
            {
                "source": "evaluate",
                "target": "_end",
                "condition": {
                "input": {
                    "type": "size",
                    "key": "generations"
                },
                "expected": "0",
                "expression": "Equal",
                "target_if_not": "evaluate"
                }
            }
        ],
        "return_value": {
            "input": {
                "type": "get_all",
                "key": "search_result"
            },
            "to_json": true
        }
    });

    let workflow = serde_json::from_value(workflow)?;

    Ok((workflow, Duration::from_secs(max_time_sec)))
}
