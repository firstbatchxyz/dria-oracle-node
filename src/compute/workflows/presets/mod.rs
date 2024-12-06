use dkn_workflows::Workflow;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref GENERATION_WORKFLOW: Workflow =
        serde_json::from_str(include_str!("generation.json"))
            .expect("could not parse generation workflow");
}

pub fn get_chat_workflow() -> Workflow {
    serde_json::from_str(include_str!("chat.json")).expect("could not parse chat workflow")
}

#[allow(unused)]
pub fn get_validation_workflow() -> Workflow {
    serde_json::from_str(include_str!("validation.json"))
        .expect("could not parse validation workflow")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_workflow() {
        // should not give error if parsing succeeds
        let _ = get_chat_workflow();
    }

    #[test]
    fn test_validation_workflow() {
        // should not give error if parsing succeeds
        let _ = get_validation_workflow();
    }
}
