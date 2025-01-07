use alloy::{
    primitives::{Bytes, FixedBytes},
    sol,
};
use clap::ValueEnum;
use eyre::{Context, Result};

use self::OracleCoordinator::StatusUpdate;

// OpenZepeplin ERC20
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    ERC20,
    "./src/contracts/abi/ERC20.json"
);

// Base WETH
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    WETH,
    "./src/contracts/abi/IWETH9.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    OracleRegistry,
    "./src/contracts/abi/LLMOracleRegistry.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    OracleCoordinator,
    "./src/contracts/abi/LLMOracleCoordinator.json"
);

/// `TaskStatus` as it appears within the coordinator.
#[derive(Debug, Clone, Copy, Default)]
pub enum TaskStatus {
    #[default]
    None,
    PendingGeneration,
    PendingValidation,
    Completed,
}

impl From<TaskStatus> for u8 {
    fn from(status: TaskStatus) -> u8 {
        status as u8
    }
}

impl TryFrom<u8> for TaskStatus {
    type Error = eyre::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TaskStatus::None),
            1 => Ok(TaskStatus::PendingGeneration),
            2 => Ok(TaskStatus::PendingValidation),
            3 => Ok(TaskStatus::Completed),
            _ => Err(eyre::eyre!("Invalid TaskStatus: {}", value)),
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::PendingGeneration => write!(f, "Pending Generation"),
            Self::PendingValidation => write!(f, "Pending Validation"),
            Self::Completed => write!(f, "Completed"),
        }
    }
}

impl std::fmt::Display for StatusUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Task {}: {} -> {}",
            self.taskId, self.statusBefore, self.statusAfter
        )
    }
}

/// `OracleKind` as it appears within the registry.
#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum OracleKind {
    Generator,
    Validator,
}

impl From<OracleKind> for u8 {
    fn from(kind: OracleKind) -> u8 {
        kind as u8
    }
}

impl TryFrom<u8> for OracleKind {
    type Error = eyre::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(OracleKind::Generator),
            1 => Ok(OracleKind::Validator),
            _ => Err(eyre::eyre!("Invalid OracleKind: {}", value)),
        }
    }
}

impl std::fmt::Display for OracleKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OracleKind::Generator => write!(f, "Generator"),
            OracleKind::Validator => write!(f, "Validator"),
        }
    }
}

/// Small utility to convert bytes to string.
pub fn bytes_to_string(bytes: &Bytes) -> Result<String> {
    String::from_utf8(bytes.to_vec()).wrap_err("could not convert bytes to string")
}

/// Small utility to convert string to bytes.
pub fn string_to_bytes(input: String) -> Bytes {
    input.into()
}

/// Small utility to convert bytes32 to string.
/// Simply reads bytes until the first null byte.
pub fn bytes32_to_string(bytes: &FixedBytes<32>) -> Result<String> {
    let mut bytes = bytes.to_vec();
    if let Some(pos) = bytes.iter().position(|&b| b == 0) {
        bytes.truncate(pos);
    } else {
        return Err(eyre::eyre!("Could not find null terminator in bytes32"));
    }
    String::from_utf8(bytes).wrap_err("could not convert bytes to string")
}

/// Small utility to convert string to bytes32.
/// Ensures that the string is at most 31 bytes long, and right-bads zeros for the null-terminator.
pub fn string_to_bytes32(input: String) -> Result<FixedBytes<32>> {
    let mut bytes = input.into_bytes();
    if bytes.len() > 31 {
        return Err(eyre::eyre!("String is too long for bytes32"));
    }

    // append bytes via resizing
    bytes.resize(32, 0);
    Ok(FixedBytes::from_slice(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_to_bytes() {
        let input = "hello".to_string();
        let bytes = string_to_bytes(input.clone());
        let string = bytes_to_string(&bytes).unwrap();
        assert_eq!(input, string);
    }

    #[test]
    fn test_string_to_bytes32() {
        let input = "hello".to_string();
        let bytes32 = string_to_bytes32(input.clone()).unwrap();
        let string = bytes32_to_string(&bytes32).unwrap();
        assert_eq!(input, string);
    }
}
