use dkn_workflows::DriaWorkflowsConfig;
use dria_oracle_contracts::OracleKind;
use tokio_util::sync::CancellationToken;

/// A utility struct to be used with tasks.
///
/// It manages oracle kinds, the workflow config to be used and a
/// cancellation for graceful exits.
#[derive(Debug, Clone)]
pub struct DriaOracleWorkflows {
    pub kinds: Vec<OracleKind>,
    pub config: DriaWorkflowsConfig,
    pub cancellation: CancellationToken,
}

// TODO: !!!
