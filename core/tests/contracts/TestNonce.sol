// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

/// @dev solc v0.8.26; solc tests/contracts/TestNonce.sol --via-ir --optimize --bin
/// @dev you may have to do: `solc-select install 0.8.26`
contract TestNonce {
    function assertValidNonce(uint256 taskId, bytes calldata input, address requester, address responder, uint256 nonce, uint8 difficulty)
        external
        pure
        returns (bytes memory message, bool result, bytes32 candidate, uint256 target)
    {
        message = abi.encodePacked(taskId, input, requester, responder, nonce);
        target = type(uint256).max >> uint256(difficulty);
        candidate = keccak256(message);
        result = uint256(candidate) <= target;
    }
}
