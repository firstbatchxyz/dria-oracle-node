use alloy::contract::Error;
use alloy::primitives::utils::format_ether;
use alloy::transports::RpcError;
use eyre::{eyre, ErrReport};

use super::OracleCoordinator::OracleCoordinatorErrors;
use super::OracleRegistry::OracleRegistryErrors;
use super::ERC20::ERC20Errors;

/// Generic contract error reporter, handles custom errors for known contracts such as ERC20, LLMOracleRegistry, and LLMOracleCoordinator.
///
/// The given contract error is matched against known contract errors and a custom error message is returned
pub fn contract_error_report(error: Error) -> ErrReport {
    match error {
        Error::UnknownFunction(function) => {
            eyre!("Unknown function: function {} does not exist", function)
        }
        Error::UnknownSelector(selector) => eyre!(
            "Unknown function: function with selector {} does not exist",
            selector
        ),
        Error::NotADeploymentTransaction => {
            eyre!("Transaction is not a deployment transaction")
        }
        Error::ContractNotDeployed => eyre!("Contract is not deployed"),
        Error::AbiError(e) => eyre!("An error occurred ABI encoding or decoding: {}", e),
        Error::TransportError(error) => {
            // here we try to parse the error w.r.t provided contract interfaces
            // or return a default one in the end if it was not parsed successfully
            if let RpcError::ErrorResp(payload) = error {
                payload
                    .as_decoded_error(false)
                    .map(ERC20Errors::into)
                    .or_else(|| {
                        payload
                            .as_decoded_error(false)
                            .map(OracleRegistryErrors::into)
                    })
                    .or_else(|| {
                        payload
                            .as_decoded_error(false)
                            .map(OracleCoordinatorErrors::into)
                    })
                    .unwrap_or(eyre!("Unhandled contract error: {}", payload))
            } else {
                eyre!("Unknown transport error: {:#?}", error)
            }
        }
    }
}

impl From<ERC20Errors> for ErrReport {
    fn from(value: ERC20Errors) -> Self {
        match value {
            ERC20Errors::ERC20InsufficientAllowance(e) => eyre!(
                "Insufficient allowance for {} (have {}, need {})",
                e.spender,
                format_ether(e.allowance),
                format_ether(e.needed)
            ),
            ERC20Errors::ERC20InsufficientBalance(e) => eyre!(
                "Insufficient balance for {} (have {}, need {})",
                e.sender,
                format_ether(e.balance),
                format_ether(e.needed)
            ),
            ERC20Errors::ERC20InvalidReceiver(e) => {
                eyre!("Invalid receiver: {}", e.receiver)
            }
            ERC20Errors::ERC20InvalidApprover(e) => {
                eyre!("Invalid approver: {}", e.approver)
            }
            ERC20Errors::ERC20InvalidSender(e) => eyre!("Invalid sender: {}", e.sender),
            ERC20Errors::ERC20InvalidSpender(e) => eyre!("Invalid spender: {}", e.spender),
        }
    }
}

impl From<OracleRegistryErrors> for ErrReport {
    fn from(value: OracleRegistryErrors) -> Self {
        match value {
            OracleRegistryErrors::AlreadyRegistered(e) => {
                eyre!("Already registered: {}", e._0)
            }
            OracleRegistryErrors::InsufficientFunds(_) => eyre!("Insufficient funds."),
            OracleRegistryErrors::NotRegistered(e) => eyre!("Not registered: {}", e._0),
            OracleRegistryErrors::OwnableInvalidOwner(e) => {
                eyre!("Invalid owner: {}", e.owner)
            }
            OracleRegistryErrors::OwnableUnauthorizedAccount(e) => {
                eyre!("Unauthorized account: {}", e.account)
            }
            _ => eyre!("Unhandled Oracle registry error"),
        }
    }
}

impl From<OracleCoordinatorErrors> for ErrReport {
    fn from(value: OracleCoordinatorErrors) -> Self {
        match value {
            OracleCoordinatorErrors::AlreadyResponded(e) => {
                eyre!("Already responded to task {}", e.taskId)
            }
            OracleCoordinatorErrors::InsufficientRewards(e) => eyre!(
                "Insufficient rewards (have: {}, want: {})",
                e.given,
                e.required
            ),
            OracleCoordinatorErrors::InvalidDifficulty(e) => {
                eyre!("Invalid difficulty: {}", e.difficulty)
            }
            OracleCoordinatorErrors::InvalidNonce(e) => {
                eyre!("Invalid nonce for task: {} (nonce: {})", e.taskId, e.nonce)
            }
            OracleCoordinatorErrors::InvalidTaskStatus(e) => eyre!(
                "Invalid status for task: {} (have: {}, want: {})",
                e.taskId,
                e.have,
                e.want
            ),
            OracleCoordinatorErrors::InvalidNumGenerations(e) => {
                eyre!("Invalid number of generations: {}", e.numGenerations)
            }
            OracleCoordinatorErrors::InvalidNumValidations(e) => {
                eyre!("Invalid number of validations: {}", e.numValidations)
            }
            OracleCoordinatorErrors::InvalidValidation(e) => {
                eyre!("Invalid validation for task: {}", e.taskId)
            }
            OracleCoordinatorErrors::NotRegistered(e) => {
                eyre!("Not registered: {}", e.oracle)
            }
            OracleCoordinatorErrors::OwnableInvalidOwner(e) => {
                eyre!("Invalid owner: {}", e.owner)
            }
            OracleCoordinatorErrors::OwnableUnauthorizedAccount(e) => {
                eyre!("Unauthorized account: {}", e.account)
            }
            _ => eyre!("Unhandled Oracle coordinator error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{contracts::OracleKind, DriaOracle, DriaOracleConfig};
    use alloy::providers::Provider;

    #[tokio::test]
    async fn test_registry_error() -> eyre::Result<()> {
        let config = DriaOracleConfig::new_from_env()?.enable_logs();
        let (node, _anvil) = DriaOracle::anvil_new(config).await?;
        assert!(node.provider.get_block_number().await? > 1);

        // tries to register if registered, or opposite, to trigger an error
        const KIND: OracleKind = OracleKind::Generator;
        let result = if node.is_registered(KIND).await? {
            node.register(KIND).await
        } else {
            node.unregister(KIND).await
        };
        assert!(result.is_err());

        // both errors include the node address in their message, which we look for here:
        let err = result.unwrap_err();
        err.to_string().contains(&node.address().to_string());

        Ok(())
    }
}
