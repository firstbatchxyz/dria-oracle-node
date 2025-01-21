mod coordinator;
mod core;
mod registry;
mod token;

mod types;
use alloy::network::Ethereum;
use types::*;

mod utils;

#[cfg(feature = "anvil")]
mod anvil;

use super::DriaOracleConfig;

use dkn_workflows::DriaWorkflowsConfig;
use dria_oracle_contracts::OracleKind;
use dria_oracle_contracts::{OracleCoordinator, OracleRegistry, ERC20};

pub struct DriaOracle {
    pub config: DriaOracleConfig,
    /// Contract addresses for the oracle, respects the connected chain.
    // pub addresses: ContractAddresses,
    pub token: ERC20::ERC20Instance<DriaOracleTransport, DriaOracleProvider, Ethereum>,
    pub coordinator: OracleCoordinator::OracleCoordinatorInstance<
        DriaOracleTransport,
        DriaOracleProvider,
        Ethereum,
    >,
    pub registry:
        OracleRegistry::OracleRegistryInstance<DriaOracleTransport, DriaOracleProvider, Ethereum>,
    /// Underlying provider type.
    pub provider: DriaOracleProvider,
    /// Kinds of this oracle, i.e. `generator`, `validator`.
    pub kinds: Vec<OracleKind>,
    /// Workflows config, defines the available models & services.
    pub workflows: DriaWorkflowsConfig,
}
