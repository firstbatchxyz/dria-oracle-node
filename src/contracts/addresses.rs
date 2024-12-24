use alloy::primitives::{address, Address};
use alloy_chains::{
    Chain,
    NamedChain::{AnvilHardhat, Base, BaseSepolia},
};
use lazy_static::lazy_static;
use std::collections::HashMap;

/// Contract addresses.
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

lazy_static! {
    /// Contract addresses per chain-id.
    pub static ref ADDRESSES: HashMap<Chain, ContractAddresses> = {
        let mut contracts = HashMap::new();

        // localhost
        contracts.insert(
            AnvilHardhat.into(),
            ContractAddresses {
                token: address!("5FbDB2315678afecb367f032d93F642f64180aa3"),
                registry: address!("e7f1725E7734CE288F8367e1Bb143E90bb3F0512"),
                coordinator: address!("9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"),
            },
        );

        // base-sepolia
        contracts.insert(
            BaseSepolia.into(),
            ContractAddresses {
                token: address!("4200000000000000000000000000000000000006"),
                registry: address!("408d245a853137e44a2465d5c66061f97582eae9"),
                coordinator: address!("13f977bde221b470d3ae055cde7e1f84debfe202"),
            },
        );

        // base mainnet
        contracts.insert(
          Base.into(),
          ContractAddresses {
              token: address!("4200000000000000000000000000000000000006"),
              registry: address!("7645eef691ad9dc0f29b6abfc73cca4c8be44051"),
              coordinator: address!("17b6d1eddcd5f9ca19bb2ffed2f3deb6bd74bd20"),
          },
        );

        contracts
    };
}
