mod coordinator;
mod core;
mod registry;
mod token;

#[cfg(feature = "anvil")]
mod anvil;

use super::DriaOracleConfig;
use alloy::providers::fillers::{
    BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
};
use alloy::{
    network::{Ethereum, EthereumWallet},
    providers::{Identity, RootProvider},
};

use dkn_workflows::DriaWorkflowsConfig;
use dria_oracle_contracts::{ContractAddresses, OracleKind};

#[cfg(not(feature = "anvil"))]
type DriaOracleProviderTransport = alloy::transports::http::Http<alloy::transports::http::Client>;
#[cfg(feature = "anvil")]
type DriaOracleProviderTransport = alloy::transports::BoxTransport;

#[cfg(not(feature = "anvil"))]
type DriaOracleProvider = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider<DriaOracleProviderTransport>,
    DriaOracleProviderTransport,
    Ethereum,
>;

#[cfg(feature = "anvil")]
type DriaOracleProvider = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    alloy::providers::layers::AnvilProvider<
        RootProvider<DriaOracleProviderTransport>,
        DriaOracleProviderTransport,
    >,
    DriaOracleProviderTransport,
    Ethereum,
>;

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
