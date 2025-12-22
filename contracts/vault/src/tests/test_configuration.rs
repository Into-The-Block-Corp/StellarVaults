#![cfg(test)]

use crate::contract::VaultContract;
use crate::storage::core::{admin, deposit_asset, paused};
use crate::tests::test_utils::{create_test_data, TestData};
use soroban_sdk::testutils::{Address as _, Events, MockAuth, MockAuthInvoke};
use soroban_sdk::{symbol_short, vec, Address, Bytes, BytesN, Env, IntoVal, Symbol};

#[test]
pub fn test_constructor() {
    let e: Env = Env::default();
    let new_admin: Address = Address::generate(&e);
    let new_deposit_asset: Address = Address::generate(&e);

    let contract_id: Address = e.register(VaultContract, (new_admin.clone(), new_deposit_asset.clone()));

    e.as_contract(&contract_id, || {
        assert_eq!(new_admin, admin(&e, None).unwrap());
        assert_eq!(new_deposit_asset, deposit_asset(&e, None).unwrap());
        assert_eq!(false, paused(&e, None).unwrap());
    });
}

#[test]
pub fn test_updating_admin() {
    let e: Env = Env::default();
    let test_data: TestData = create_test_data(&e);
    let new_admin: Address = Address::generate(&e);

    // If the admin hasn't signed the transaction it should fail
    assert!(test_data.contract.try_update_admin(&new_admin).is_err());

    test_data
        .contract
        .mock_auths(&[MockAuth {
            address: &test_data.admin,
            invoke: &MockAuthInvoke {
                contract: &test_data.contract.address,
                fn_name: "update_admin",
                args: (new_admin.clone(),).into_val(&e),
                sub_invokes: &[],
            },
        }])
        .update_admin(&new_admin);

    assert_eq!(
        e.events().all(),
        vec![
            &e,
            (
                test_data.contract.address.clone(),
                (symbol_short!("ADMIN"), symbol_short!("update")).into_val(&e),
                new_admin.into_val(&e)
            ),
        ]
    );

    e.as_contract(&test_data.contract.address, || {
        assert_ne!(admin(&e, None).unwrap(), test_data.admin);
        assert_eq!(admin(&e, None).unwrap(), new_admin);
    });
}

#[test]
pub fn test_pausing() {
    let e: Env = Env::default();
    let test_data: TestData = create_test_data(&e);

    // If the admin hasn't signed the transaction it should fail
    assert!(test_data.contract.try_set_status(&true).is_err());

    test_data
        .contract
        .mock_auths(&[MockAuth {
            address: &test_data.admin,
            invoke: &MockAuthInvoke {
                contract: &test_data.contract.address,
                fn_name: "set_status",
                args: (true,).into_val(&e),
                sub_invokes: &[],
            },
        }])
        .set_status(&true);

    assert_eq!(
        e.events().all(),
        vec![
            &e,
            (
                test_data.contract.address.clone(),
                (symbol_short!("STATUS"), symbol_short!("update")).into_val(&e),
                true.into_val(&e)
            ),
        ]
    );

    e.as_contract(&test_data.contract.address, || {
        assert_eq!(paused(&e, None).unwrap(), true);
    });

    test_data.contract.mock_all_auths().set_status(&false);

    assert_eq!(
        e.events().all(),
        vec![
            &e,
            (
                test_data.contract.address.clone(),
                (symbol_short!("STATUS"), symbol_short!("update")).into_val(&e),
                false.into_val(&e)
            ),
        ]
    );

    e.as_contract(&test_data.contract.address, || {
        assert_eq!(paused(&e, None).unwrap(), false);
    });
}

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
