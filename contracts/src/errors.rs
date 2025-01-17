use alloy::contract::Error;
use alloy::primitives::utils::format_ether;
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
        Error::PendingTransactionError(tx) => {
            eyre!("Transaction is pending: {:?}", tx)
        }
        Error::NotADeploymentTransaction => {
            eyre!("Transaction is not a deployment transaction")
        }
        Error::ContractNotDeployed => eyre!("Contract is not deployed"),
        Error::AbiError(e) => eyre!("An error occurred ABI encoding or decoding: {}", e),
        Error::TransportError(error) => {
            const VALIDATE: bool = false;

            // here we try to parse the error w.r.t provided contract interfaces
            // or return a default one in the end if it was not parsed successfully
            if let Some(payload) = error.as_error_resp() {
                // an ERC20 error
                if let Some(erc_20_error) = payload.as_decoded_error::<ERC20Errors>(VALIDATE) {
                    erc_20_error.into()
                } else
                // an OracleRegistry error
                if let Some(registry_error) =
                    payload.as_decoded_error::<OracleRegistryErrors>(VALIDATE)
                {
                    registry_error.into()
                } else
                // an OracleCoordinator error
                if let Some(coordinator_error) =
                    payload.as_decoded_error::<OracleCoordinatorErrors>(VALIDATE)
                {
                    coordinator_error.into()
                } else {
                    eyre!("Unhandled error response: {:#?}", error)
                }
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
            } // _ => eyre!("Unhandled Oracle registry error"),
            OracleRegistryErrors::TooEarlyToUnregister(e) => {
                eyre!(
                    "Too early to unregister: {} secs remaining",
                    e.minTimeToWait
                )
            }
            OracleRegistryErrors::NotWhitelisted(e) => {
                eyre!("Validator {} is not whitelisted", e.validator)
            }
            // generic
            OracleRegistryErrors::FailedCall(_) => {
                eyre!("Failed call")
            }
            OracleRegistryErrors::ERC1967InvalidImplementation(e) => {
                eyre!("Invalid implementation: {}", e.implementation)
            }
            OracleRegistryErrors::UUPSUnauthorizedCallContext(_) => {
                eyre!("Unauthorized UUPS call context")
            }
            OracleRegistryErrors::UUPSUnsupportedProxiableUUID(e) => {
                eyre!("Unsupported UUPS proxiable UUID: {}", e.slot)
            }
            OracleRegistryErrors::ERC1967NonPayable(_) => {
                eyre!("ERC1967 Non-payable")
            }
            OracleRegistryErrors::InvalidInitialization(_) => {
                eyre!("Invalid initialization")
            }
            OracleRegistryErrors::AddressEmptyCode(e) => {
                eyre!("Address {} is empty", e.target)
            }
            OracleRegistryErrors::NotInitializing(_) => {
                eyre!("Not initializing")
            }
        }
    }
}

impl From<OracleCoordinatorErrors> for ErrReport {
    fn from(value: OracleCoordinatorErrors) -> Self {
        match value {
            OracleCoordinatorErrors::AlreadyResponded(e) => {
                eyre!("Already responded to task {}", e.taskId)
            }
            OracleCoordinatorErrors::InsufficientFees(e) => {
                eyre!("Insufficient fees (have: {}, want: {})", e.have, e.want)
            }
            OracleCoordinatorErrors::InvalidParameterRange(e) => {
                eyre!(
                    "Invalid parameter range: {} <= {}* <= {}",
                    e.min,
                    e.have,
                    e.max
                )
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
            // generic
            OracleCoordinatorErrors::FailedInnerCall(_) => {
                eyre!("Failed inner call")
            }
            OracleCoordinatorErrors::ERC1967InvalidImplementation(e) => {
                eyre!("Invalid implementation: {}", e.implementation)
            }
            OracleCoordinatorErrors::UUPSUnauthorizedCallContext(_) => {
                eyre!("Unauthorized UUPS call context")
            }
            OracleCoordinatorErrors::UUPSUnsupportedProxiableUUID(e) => {
                eyre!("Unsupported UUPS proxiable UUID: {}", e.slot)
            }
            OracleCoordinatorErrors::ERC1967NonPayable(_) => {
                eyre!("ERC1967 Non-payable")
            }
            OracleCoordinatorErrors::InvalidInitialization(_) => {
                eyre!("Invalid initialization")
            }
            OracleCoordinatorErrors::AddressEmptyCode(e) => {
                eyre!("Address {} is empty", e.target)
            }
            OracleCoordinatorErrors::NotInitializing(_) => {
                eyre!("Not initializing")
            }
        }
    }
}
