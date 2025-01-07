mod coordinator;
mod registry;
mod token;

use super::parsers::*;
use crate::contracts::OracleKind;
use alloy::{eips::BlockNumberOrTag, primitives::U256};
use clap::Subcommand;
use dkn_workflows::Model;

// https://docs.rs/clap/latest/clap/_derive/index.html#arg-attributes
#[derive(Subcommand)]
pub enum Commands {
    /// Register oracle as a specific oracle kind.
    Register {
        #[arg(help = "The oracle kinds to register as.", required = true)]
        kinds: Vec<OracleKind>,
    },
    /// Unregister oracle as a specific oracle kind.
    Unregister {
        #[arg(help = "The oracle kinds to unregister as.", required = true)]
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
    /// Start the oracle node.
    Start {
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
        #[arg(help = "The oracle kinds to handle tasks as.", required = false)]
        kinds: Vec<OracleKind>,
        #[arg(short, long = "model", help = "The models to serve.", required = true, value_parser = parse_model)]
        models: Vec<Model>,
    },
    /// View status of a given task.
    View {
        #[arg(help = "Task id.", required = true)]
        task_id: U256,
    },
    /// Process a single task.
    Process {
        #[arg(help = "Task id.", required = true)]
        task_id: U256,
        #[arg(help = "The oracle kinds to handle the task as.", required = false)]
        kinds: Vec<OracleKind>,
        #[arg(short, long = "model", help = "The models to use for this task.", required = true, value_parser = parse_model)]
        models: Vec<Model>,
    },
    /// View tasks between specific blocks.
    Tasks {
        #[arg(long, help = "Starting block number, defaults to 'earliest'.", value_parser = parse_block_number_or_tag)]
        from: Option<BlockNumberOrTag>,
        #[arg(long, help = "Ending block number, defaults to 'latest'.", value_parser = parse_block_number_or_tag)]
        to: Option<BlockNumberOrTag>,
    },
    /// Request a task.
    Request {
        #[arg(help = "The input to request a task with.", required = true)]
        input: String,
        #[arg(help = "The models to accept.", required = true, value_parser=parse_model)]
        models: Vec<Model>,
        #[arg(long, help = "The difficulty of the task.", default_value_t = 1)]
        difficulty: u8,
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
