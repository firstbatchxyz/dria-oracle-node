use alloy::primitives::{keccak256, Address, Bytes, U256};
use alloy::sol_types::SolValue;

pub struct NonceResult {
    pub nonce: U256,
    pub candidate: U256,
    pub target: U256,
}

/// Mines a nonce for the oracle proof-of-work.
///
/// Returns (nonce, candidate, target).
pub fn mine_nonce(
    difficulty: u8,
    requester: &Address,
    responder: &Address,
    input: &Bytes,
    task_id: &U256,
) -> NonceResult {
    let big_one = U256::from(1);
    let mut nonce = U256::ZERO;

    // target is (2^256-1) / 2^difficulty
    let target = U256::MAX >> difficulty;

    loop {
        // encode packed
        let mut message = Vec::new();
        task_id.abi_encode_packed_to(&mut message);
        input.abi_encode_packed_to(&mut message);
        requester.abi_encode_packed_to(&mut message);
        responder.abi_encode_packed_to(&mut message);
        nonce.abi_encode_packed_to(&mut message);

        // check hash
        let digest = keccak256(message.clone());
        let candidate = U256::from_be_bytes(*digest); // big-endian!
        if candidate < target {
            return NonceResult {
                nonce,
                candidate,
                target,
            };
        }

        nonce += big_one;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DriaOracle, DriaOracleConfig};
    use alloy::{
        primitives::{address, Bytes, U256},
        sol,
    };
    use eyre::Result;

    sol! {
        #[allow(missing_docs)]
        // solc v0.8.26; solc tests/contracts/TestNonce.sol --via-ir --optimize --bin
        #[sol(rpc, bytecode="608080604052346015576101bc908161001a8239f35b5f80fdfe6080806040526004361015610012575f80fd5b5f3560e01c6359327f7c14610025575f80fd5b346101825760c0366003190112610182576024359067ffffffffffffffff8211610182573660238301121561018257816004013567ffffffffffffffff811161018257366024828501011161018257604435906001600160a01b038216820361018257606435906001600160a01b03821682036101825760a4359260ff841680940361018257604092828693602460208601996004358b5201868601378301916bffffffffffffffffffffffff199060601b16848301526bffffffffffffffffffffffff199060601b166054820152608435606882015203016028810183526067601f1991011682019282841067ffffffffffffffff85111761016e5760a0928492836040525f19901c908051832090608085525180938160808701528686015e5f84840186015281811115602085015260408401526060830152601f01601f19168101030190f35b634e487b7160e01b5f52604160045260245ffd5b5f80fdfea26469706673582212200bd4b20c7c71d8e44c60f255bf0a5e937f501f0e718e64687e36e0ddbc0d491864736f6c634300081a0033")]
        contract TestNonce {
            function assertValidNonce(uint256 taskId, bytes calldata input, address requester, address responder, uint256 nonce, uint8 difficulty) external
            view
            returns (bytes memory message, bool result, bytes32 candidate, uint256 target)
            {
                message = abi.encodePacked(taskId, input, requester, responder, nonce);
                target = type(uint256).max >> uint256(difficulty);
                candidate = keccak256(message);
                result = uint256(candidate) <= target;
            }
        }
    }

    #[tokio::test]
    async fn test_nonce_contract() -> Result<()> {
        let config = DriaOracleConfig::new_from_env()?;
        let (node, _anvil) = DriaOracle::anvil_new(config).await?;
        let contract = TestNonce::deploy(&node.provider).await?;

        // prepare parameters
        let difficulty = 2u8;
        let task_id = U256::from(1);
        let requester = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let responder = node.address();
        let input = Bytes::from_iter("im some bytes yallllll".bytes());

        // call contract
        let NonceResult {
            nonce,
            candidate,
            target,
        } = mine_nonce(difficulty, &requester, &responder, &input, &task_id);
        // println!("Nonce:     {}", nonce);
        // println!("Target:    {:x}", target);
        // println!("Candidate: {:x}", candidate);
        let contract_bytes = contract
            .assertValidNonce(task_id, input, requester, responder, nonce, difficulty)
            .call()
            .await?;

        // println!("\nResult:    {}", contract_bytes.result);
        // println!("Target:    {:x}", contract_bytes.target);
        // println!("Candidate: {:x}", contract_bytes.candidate);
        // println!("Message:\n{:x}", contract_bytes.message);
        assert_eq!(contract_bytes.target, target);
        assert_eq!(U256::from_be_bytes(contract_bytes.candidate.0), candidate);
        assert_eq!(contract_bytes.result, true);
        Ok(())
    }

    #[test]
    fn test_nonce_local() {
        let requester = address!("0877022A137b8E8CE1C3020B9f047651dD02E37B");
        let responder = address!("0877022A137b8E8CE1C3020B9f047651dD02E37B");
        let input = vec![0x01, 0x02, 0x03].into();
        let task_id = U256::from(0x1234);
        let difficulty = 2;

        let NonceResult {
            nonce,
            candidate,
            target,
        } = mine_nonce(difficulty, &requester, &responder, &input, &task_id);
        assert!(!nonce.is_zero());

        println!("Nonce: {}", nonce);
        println!("Target: {:x}", target);
        println!("Candidate: {:x}", candidate);
    }
}
