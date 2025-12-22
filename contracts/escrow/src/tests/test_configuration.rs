#![cfg(test)]

use crate::tests::test_utils::{create_test_data, TestData};
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{symbol_short, Address, Bytes, BytesN, Env, IntoVal, Symbol};

mod useless_contract {
    use soroban_sdk::contractimport;

    contractimport!(file = "../../target/wasm32v1-none/release/useless_contract.wasm");
}

#[test]
fn test_upgrade() {
    let e: Env = Env::default();
    let test_data: TestData = create_test_data(&e);

    e.register(useless_contract::WASM, ());
    let hash: BytesN<32> = e.crypto().sha256(&Bytes::from_slice(&e, useless_contract::WASM)).to_bytes();

    // If there is no authorization it fails
    assert!(test_data.contract.try_upgrade(&hash).is_err());

    // If someone but the admin is signing it should fail
    assert!(test_data
        .contract
        .mock_auths(&[MockAuth {
            address: &Address::generate(&e),
            invoke: &MockAuthInvoke {
                contract: &test_data.contract.address,
                fn_name: "upgrade",
                args: (hash.clone(),).into_val(&e),
                sub_invokes: &[],
            },
        }])
        .try_upgrade(&hash)
        .is_err());

    test_data
        .contract
        .mock_auths(&[MockAuth {
            address: &test_data.admin,
            invoke: &MockAuthInvoke {
                contract: &test_data.contract.address,
                fn_name: "upgrade",
                args: (hash.clone(),).into_val(&e),
                sub_invokes: &[],
            },
        }])
        .upgrade(&hash);

    assert_eq!(
        e.invoke_contract::<Symbol>(&test_data.contract.address, &symbol_short!("hello"), ().into_val(&e)),
        symbol_short!("world")
    )
}
