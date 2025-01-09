use alloy::{eips::BlockNumberOrTag, primitives::U256};
use clap::Subcommand;
use dkn_workflows::Model;
use dria_oracle_contracts::OracleKind;

use super::parsers::*;

mod coordinator;
mod registry;
mod token;

// https://docs.rs/clap/latest/clap/_derive/index.html#arg-attributes
#[derive(Subcommand)]
pub enum Commands {
    /// Register oracle as a specific oracle kind.
    Register {
        #[arg(help = "The oracle kinds to register as.", required = true, value_parser = parse_oracle_kind)]
        kinds: Vec<OracleKind>,
    },
    /// Unregister oracle as a specific oracle kind.
    Unregister {
        #[arg(help = "The oracle kinds to unregister as.", required = true, value_parser = parse_oracle_kind)]
        kinds: Vec<OracleKind>,
    },
    /// See all registrations.
    Registrations,
    /// See the current balance of the oracle node.
    Balance,
    /// See claimable rewards from the coordinator.
    Rewards,
    /// Claim rewards from the coordinator.
    Claim,
    /// Serve the oracle node.
    Serve {
        #[arg(help = "The oracle kinds to handle tasks as, if omitted will default to all registered kinds.", value_parser = parse_oracle_kind)]
        kinds: Vec<OracleKind>,
        #[arg(short, long = "model", help = "The models to serve.", required = true, value_parser = parse_model)]
        models: Vec<Model>,
        #[arg(
            long,
            help = "Block number to starting listening from, omit to start from latest block.",
            value_parser = parse_block_number_or_tag
        )]
        from: Option<BlockNumberOrTag>,
        #[arg(
            long,
            help = "Block number to stop listening at, omit to keep running the node indefinitely.",
            value_parser = parse_block_number_or_tag
        )]
        to: Option<BlockNumberOrTag>,
        #[arg(
            long,
            help = "Optional task id to serve specifically.",
            required = false
        )]
        task_id: Option<U256>,
    },
    /// View tasks. fsdkhfk fsdkjfdks
    View {
        #[arg(long, help = "Starting block number, defaults to 'earliest'.", value_parser = parse_block_number_or_tag)]
        from: Option<BlockNumberOrTag>,
        #[arg(long, help = "Ending block number, defaults to 'latest'.", value_parser = parse_block_number_or_tag)]
        to: Option<BlockNumberOrTag>,
        #[arg(long, help = "Task id to view.")]
        task_id: Option<U256>,
    },
    /// Request a task.
    Request {
        #[arg(help = "The input to request a task with.", required = true)]
        input: String,
        #[arg(help = "The models to accept.", required = true, value_parser=parse_model)]
        models: Vec<Model>,
        #[arg(long, help = "The difficulty of the task.", default_value_t = 2)]
        difficulty: u8,
        #[arg(long, help = "Protocol name for the request", default_value = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))]
        protocol: String,
        #[arg(
            long,
            help = "The number of generations to request.",
            default_value_t = 1
        )]
        num_gens: u64,
        #[arg(
            long,
            help = "The number of validations to request.",
            default_value_t = 1
        )]
        num_vals: u64,
    },
}
