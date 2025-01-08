#![doc = include_str!("../../README.md")]

mod cli;
pub use cli::{handle_command, Cli};

mod node;
pub use node::DriaOracle;

/// Node configurations.
mod configurations;
pub use configurations::DriaOracleConfig;

mod compute;
pub use compute::{handle_generation, handle_request, handle_validation, mine_nonce};
