use std::str::FromStr;

use alloy::{
    network::EthereumWallet,
    node_bindings::Anvil,
    primitives::{address, keccak256, Address, Bytes, U256},
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
    sol,
    sol_types::SolValue,
};
use eyre::Result;

sol! {
    #[allow(missing_docs)]
    // solc v0.8.26; solc tests/contracts/TestSolidityPacked.sol --via-ir --optimize --bin
    #[sol(rpc, bytecode="608080604052346015576102bd908161001a8239f35b5f80fdfe60806040526004361015610011575f80fd5b5f3560e01c8063026131d7146101115780639dac80891461007f5763aa1e84de1461003a575f80fd5b3461007b57602036600319011261007b5760043567ffffffffffffffff811161007b5761006d60209136906004016101a9565b818151910120604051908152f35b5f80fd5b3461007b5761010d60206100f96059610097366101ff565b916040979397519788956bffffffffffffffffffffffff199060601b1685870152603486015263ffffffff60e01b9060e01b166054850152151560f81b60588401528051918291018484015e81015f838201520301601f198101835282610173565b604051918291602083526020830190610263565b0390f35b3461007b5761010d6101696100f963ffffffff61012d366101ff565b604080516001600160a01b03909616602087015285019390935293166060830152911515608082015260a08082015292839160c0830190610263565b03601f1981018352825b90601f8019910116810190811067ffffffffffffffff82111761019557604052565b634e487b7160e01b5f52604160045260245ffd5b81601f8201121561007b5780359067ffffffffffffffff821161019557604051926101de601f8401601f191660200185610173565b8284526020838301011161007b57815f926020809301838601378301015290565b60a060031982011261007b576004356001600160a01b038116810361007b57916024359160443563ffffffff8116810361007b5791606435801515810361007b57916084359067ffffffffffffffff821161007b57610260916004016101a9565b90565b805180835260209291819084018484015e5f828201840152601f01601f191601019056fea2646970667358221220e3f817afaaaef1121ac298a855627f97dac246dd3de24d8585bef6c9fce3ab5664736f6c634300081a0033")]
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
}

#[tokio::test]
async fn test_encode_packed() -> Result<()> {
    let anvil = Anvil::new().try_spawn()?;
    let signer: PrivateKeySigner = anvil.keys()[0].clone().into();
    let wallet = EthereumWallet::from(signer);
    let rpc_url = anvil.endpoint().parse()?;
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url);
    let contract = TestSolidityPacked::deploy(&provider).await?;

    // prepare parameters
    let some_address = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
    let some_number = U256::from(0x12345678);
    let some_short = 0x69u32;
    let some_bool = true;
    let some_bytes = Bytes::from_static(&[0x42u8; 42]);

    // call contract
    let contract_bytes = contract
        .encodePacked(
            some_address,
            some_number,
            some_short,
            some_bool,
            some_bytes.clone(),
        )
        .call()
        .await?
        ._0;

    // encode locally
    let mut local_bytes_vec = Vec::new();
    some_address.abi_encode_packed_to(&mut local_bytes_vec);
    some_number.abi_encode_packed_to(&mut local_bytes_vec);
    some_short.abi_encode_packed_to(&mut local_bytes_vec);
    some_bool.abi_encode_packed_to(&mut local_bytes_vec);
    some_bytes.abi_encode_packed_to(&mut local_bytes_vec);
    let local_bytes = Bytes::from(local_bytes_vec);

    assert_eq!(contract_bytes, local_bytes);

    Ok(())
}

#[tokio::test]
#[ignore = "bug with bytes encoding"]
async fn test_encode() -> Result<()> {
    let anvil = Anvil::new().try_spawn()?;
    let signer: PrivateKeySigner = anvil.keys()[0].clone().into();
    let wallet = EthereumWallet::from(signer);
    let rpc_url = anvil.endpoint().parse()?;
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url);
    let contract = TestSolidityPacked::deploy(&provider).await?;

    // prepare parameters
    let some_address = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
    let some_number = U256::from(0x12345678);
    let some_short = 0x69u32;
    let some_bool = true;
    let some_bytes = Bytes::from_static(&[0x42u8; 42]);

    // call contract
    let contract_bytes = contract
        .encode(
            some_address,
            some_number,
            some_short,
            some_bool,
            some_bytes.clone(),
        )
        .with_cloned_provider()
        .call()
        .await?
        ._0;

    // encode with alloy
    // let mut encoder = alloy::en ::Encoder::new();
    let mut local_bytes_vec = Vec::new();
    local_bytes_vec.extend(some_address.abi_encode());
    local_bytes_vec.extend(some_number.abi_encode());
    local_bytes_vec.extend(some_short.abi_encode());
    local_bytes_vec.extend(some_bool.abi_encode());
    local_bytes_vec.extend(some_bytes.abi_encode());
    let local_bytes = Bytes::from(local_bytes_vec);

    assert_eq!(contract_bytes, local_bytes);

    Ok(())
}

#[tokio::test]
async fn test_decode() -> Result<()> {
    let b = Bytes::from_str("000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000060000000000000000000000007befbfbd18777d12efbfbdefbfbdefbfbd79466b6fefbfbd6cefbfbd01efbfbd380000000000000000000000007c467217056444efbfbdd99ac6ab0231efbfbd55efbfbdefbfbd240f0000000000000000000000005defbfbd0b2b3d4eefbfbd3b45efbfbd42efbfbd0457efbfbdefbfbd1b64664f000000000000000000000000efbfbdefbfbd66efbfbdefbfbdefbfbd13efbfbd56efbfbd4d11efbfbdefbfbdefbfbd5befbfbd7869130000000000000000000000001b0cefbfbd01efbfbdefbfbdefbfbd2862efbfbd5130efbfbdefbfbdefbfbd03efbfbd2defbfbd6800000000000000000000000052efbfbd7a5b2f152133efbfbd773d2969efbfbd15efbfbdefbfbdefbfbdefbfbd")?;
    let addr = Vec::<Address>::abi_decode(&b, false)?;
    println!("{:?}", addr);

    Ok(())
}
#[tokio::test]
async fn test_hash() -> Result<()> {
    let anvil = Anvil::new().try_spawn()?;
    let signer: PrivateKeySigner = anvil.keys()[0].clone().into();
    let wallet = EthereumWallet::from(signer);
    let rpc_url = anvil.endpoint().parse()?;
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url);
    let contract = TestSolidityPacked::deploy(&provider).await?;

    let some_bytes = Bytes::from_static(&hex_literal::hex!("deadbeef123deadbeef123deadbeef"));

    let contract_bytes = contract.hash(some_bytes.clone()).call().await?._0;
    let local_bytes = keccak256(some_bytes);

    assert_eq!(contract_bytes, local_bytes);

    Ok(())
}
