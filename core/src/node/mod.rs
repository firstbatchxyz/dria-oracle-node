use alloy::network::Ethereum;
use dkn_workflows::DriaWorkflowsConfig;
use dria_oracle_contracts::OracleKind;
use dria_oracle_contracts::{OracleCoordinator, OracleRegistry, ERC20};

mod coordinator;
mod core;
mod registry;
mod token;

mod types;
use types::*;

#[cfg(feature = "anvil")]
mod anvil;

use super::DriaOracleConfig;
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

impl std::fmt::Display for DriaOracle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
          f,
          "Dria Oracle Node v{}\nOracle Address: {}\nRPC URL: {}\nCoordinator: {}\nTx timeout: {}s",
          env!("CARGO_PKG_VERSION"),
          self.address(),
          self.config.rpc_url,
          self.coordinator.address(),
          self.config.tx_timeout.map(|t| t.as_secs()).unwrap_or_default()
      )
    }
}
