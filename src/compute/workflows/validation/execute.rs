use alloy::primitives::U256;
use dkn_workflows::{Executor, Model, ProgramMemory};
use eyre::{Context, Result};

use super::workflow::make_validation_workflow;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ValidationResult {
    /// How helpful the response is.
    helpfulness: u8,
    /// How closely the response follows the instruction.
    instruction_following: u8,
    /// The final score of the response, which we write to the contract.
    final_score: u8,
    /// The truthfulness of the response.
    /// If the response is correct but it doesn't make sense w.r.t instruction,
    /// this may still be correct.
    truthfulness: u8,
    /// The rationale for the scores reported.
    rationale: String,
}

impl ValidationResult {
    /// Clamps the score to the range `[1, 5]` and scales it to the range `[1-255]`.
    pub fn final_score_as_solidity_type(&self) -> U256 {
        U256::from(match self.final_score.clamp(1, 5) {
            1 => 1,
            2 => 64,
            3 => 85,
            4 => 127,
            5 => 255,
            _ => unreachable!(),
        })
    }
}

/// Executes a validation request using the given model.
pub async fn validate_generations(
    instruction: String,
    generations: Vec<String>,
    model: Model,
) -> Result<Vec<ValidationResult>> {
    let workflow = make_validation_workflow(instruction, generations)?;

    log::debug!("Executing validation request with: {}", model);
    let mut memory = ProgramMemory::new();
    let executor = Executor::new(model);
    let result_str = executor.execute(None, &workflow, &mut memory).await?;

    // first parse as vec of string
    // then parse each string as a ValidationResult
    // FIXME: this is a workflows bug, can return a single parseable string instead of array of parsable strings later
    let result: Vec<ValidationResult> = serde_json::from_str::<Vec<String>>(&result_str)
        .wrap_err("could not parse validation results")?
        .into_iter()
        .map(|s| serde_json::from_str::<ValidationResult>(&s))
        .collect::<Result<Vec<_>, _>>()
        .wrap_err("could not parse validation results")?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires OpenAI API key"]
    async fn test_validation() {
        dotenvy::dotenv().unwrap();

        let instruction = "What is 2 + 2".to_string();
        let generations: Vec<String> = [
            "2 + 2 is 4.",                          // correct
            "2 + 2 is 889992.",                     // incorrect
            "Bonito applebum",                      // completely irrelevant
            "2 + 2 is 4 because apples are green.", // correct but irrational
            "2 + 2 is not 5.",                      // correct but irrelevant
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        let model = Model::GPT4oMini;
        let results = validate_generations(instruction, generations.clone(), model)
            .await
            .unwrap();

        assert_eq!(
            results.len(),
            generations.len(),
            "expected same number of results"
        );
        assert!(
            results[0].final_score == 5,
            "expected top score from correct response"
        );
        assert!(
            results[1].final_score == 1,
            "expected minimum score from wrong response"
        );
        assert!(
            results[2].final_score == 1,
            "expected minimum score from irrelevant response"
        );
    }
}
