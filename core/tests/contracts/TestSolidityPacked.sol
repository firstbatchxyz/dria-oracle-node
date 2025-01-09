// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

/// @dev solc v0.8.26; solc tests/contracts/TestSolidityPacked.sol --via-ir --optimize --bin
/// @dev you may have to do: `solc-select install 0.8.26`
contract TestSolidityPacked {
    function encodePacked(address someAddress, uint256 someNumber, uint32 someShort, bool someBool, bytes memory someBytes) public pure returns (bytes memory) {
        return abi.encodePacked(someAddress, someNumber, someShort, someBool, someBytes);
    }

    function encode(address someAddress, uint256 someNumber, uint32 someShort, bool someBool, bytes memory someBytes) public pure returns (bytes memory) {
        return abi.encode(someAddress, someNumber, someShort, someBool, someBytes);
    }

    function hash(bytes memory data) public pure returns (bytes32) {
        return keccak256(data);
    }
}
