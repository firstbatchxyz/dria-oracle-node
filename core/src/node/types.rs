//! This module exports the transport and provider type based on the `anvil` feature.
//!
//! - If `anvil` is enabled, then Anvil-compatible provider type is created.
//! - Otherwise, default provider is created for Ethereum-like networks.

use alloy::providers::fillers::{
    BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
};
use alloy::{
    network::{Ethereum, EthereumWallet},
    providers::{Identity, RootProvider},
};

#[cfg(not(feature = "anvil"))]
pub type DriaOracleProviderTransport =
    alloy::transports::http::Http<alloy::transports::http::Client>;

#[cfg(feature = "anvil")]
pub type DriaOracleTransport = alloy::transports::BoxTransport;

#[cfg(not(feature = "anvil"))]
pub type DriaOracleProvider = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider<DriaOracleTransport>,
    DriaOracleTransport,
    Ethereum,
>;

#[cfg(feature = "anvil")]
pub type DriaOracleProvider = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    alloy::providers::layers::AnvilProvider<RootProvider<DriaOracleTransport>, DriaOracleTransport>,
    DriaOracleTransport,
    Ethereum,
>;
