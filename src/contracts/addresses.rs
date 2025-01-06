use alloy::primitives::{address, Address};
use alloy_chains::NamedChain;

/// Contract addresses.
///
/// All contracts can be derived from the `coordinator` contract.
#[derive(Debug, Clone)]
pub struct ContractAddresses {
    /// Token used within the registry and coordinator.
    pub token: Address,
    /// Oracle registry.
    pub registry: Address,
    /// Oracle coordinator.
    pub coordinator: Address,
}

impl std::fmt::Display for ContractAddresses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Contract Addresses:\n  Token: {}\n  Registry: {}\n  Coordinator: {}",
            self.token, self.registry, self.coordinator
        )
    }
}

/// Returns the coordinator contract address for a given chain.
///
/// Will return an error if the chain is not supported, i.e. a coordinator address
/// is not deployed there.
pub fn get_coordinator_address(chain: NamedChain) -> eyre::Result<Address> {
    
    let addresses = match chain {
        NamedChain::AnvilHardhat => address!("9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"),
        NamedChain::BaseSepolia => address!("13f977bde221b470d3ae055cde7e1f84debfe202"),
        NamedChain::Base => address!("17b6d1eddcd5f9ca19bb2ffed2f3deb6bd74bd20"),
        _ => return Err(eyre::eyre!("Chain {} is not supported", chain)),
    };

    Ok(addresses)
}
