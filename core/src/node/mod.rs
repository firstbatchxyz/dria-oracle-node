mod coordinator;
mod core;
mod registry;
mod token;

mod types;
use types::*;

mod utils;

#[cfg(feature = "anvil")]
mod anvil;

use super::DriaOracleConfig;

use dkn_workflows::DriaWorkflowsConfig;
use dria_oracle_contracts::{ContractAddresses, OracleKind};

pub struct DriaOracle {
    pub config: DriaOracleConfig,
    /// Contract addresses for the oracle, respects the connected chain.
    pub addresses: ContractAddresses,
    /// Underlying provider type.
    pub provider: DriaOracleProvider,
    /// Kinds of this oracle, i.e. `generator`, `validator`.
    pub kinds: Vec<OracleKind>,
    /// Workflows config, defines the available models & services.
    pub workflows: DriaWorkflowsConfig,
}
