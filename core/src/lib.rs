#![doc = include_str!("../../README.md")]

mod cli;
pub use cli::cli;

mod node;
pub use node::DriaOracle;

/// Node configurations.
mod configurations;
pub use configurations::DriaOracleConfig;

mod compute;
pub use compute::{handle_request, mine_nonce};

mod contracts;
pub use contracts::{bytes32_to_string, bytes_to_string, string_to_bytes, string_to_bytes32};
pub use contracts::{OracleCoordinator, OracleRegistry, ERC20, WETH};
pub use contracts::{OracleKind, TaskStatus};
