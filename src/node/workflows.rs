use dkn_workflows::DriaWorkflowsConfig;
use tokio_util::sync::CancellationToken;

use super::OracleKind;

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
