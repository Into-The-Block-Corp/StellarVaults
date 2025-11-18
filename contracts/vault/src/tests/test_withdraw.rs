#![cfg(test)]

use crate::constants::SCALAR_7;
use crate::errors::ContractErrors;
use crate::storage::deposit_state::{deposit, total_principal, DepositRecord};
use crate::tests::test_utils::{create_test_data, TestData};
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, Env, IntoVal};

#[test]
pub fn test_simple_withdraw() {
    let e: Env = Env::default();
    let test_data: TestData = create_test_data(&e);

    let depositor: Address = Address::generate(&e);
    let amount: u128 = 100 * SCALAR_7;
    test_data.deposit_asset_sac.mock_all_auths().mint(&depositor, &(amount as i128));
    test_data.contract.mock_all_auths().deposit(&depositor, &amount);

    assert!(test_data.contract.try_withdraw(&depositor, &amount).is_err());

    e.as_contract(&test_data.contract.address, || {
        assert!(deposit(&e, &depositor, None, false).is_some());
    });

    assert_eq!(test_data.deposit_asset_tc.balance(&depositor), 0);

    test_data
        .contract
        .mock_auths(&[MockAuth {
            address: &depositor,
            invoke: &MockAuthInvoke {
                contract: &test_data.contract.address,
                fn_name: "withdraw",
                args: (depositor.clone(), amount.clone()).into_val(&e),
                sub_invokes: &[],
            },
        }])
        .withdraw(&depositor, &amount);

    e.as_contract(&test_data.contract.address, || {
        assert!(deposit(&e, &depositor, None, false).is_none());
        assert_eq!(total_principal(&e), 0);
    });

    assert_eq!(test_data.deposit_asset_tc.balance(&depositor), amount as i128);
}

#[test]
pub fn test_multiple_withdraws() {
    let e: Env = Env::default();
    let test_data: TestData = create_test_data(&e);

    let depositor: Address = Address::generate(&e);
    let amount: u128 = 100 * SCALAR_7;

    for _ in 0..4 {
        test_data.deposit_asset_sac.mock_all_auths().mint(&depositor, &(amount as i128));
        test_data.contract.mock_all_auths().deposit(&depositor, &amount);
    }

    assert_eq!(test_data.deposit_asset_tc.balance(&depositor), 0);

    // We will withdraw half of a deposit
    test_data.contract.mock_all_auths().withdraw(&depositor, &(amount / 2));

    assert_eq!(test_data.deposit_asset_tc.balance(&depositor) as u128, amount / 2);

    e.as_contract(&test_data.contract.address, || {
        let first_deposit: DepositRecord = deposit(&e, &depositor, None, false).unwrap();
        assert_eq!(first_deposit.amount, (amount * 4) - (amount / 2));
        assert_eq!(total_principal(&e), (amount * 4) - (amount / 2));
    });

    test_data.contract.mock_all_auths().withdraw(&depositor, &(amount * 3));
    assert_eq!(test_data.deposit_asset_tc.balance(&depositor) as u128, (amount / 2) + (amount * 3));
}

#[test]
pub fn test_withdraw_errors() {
    let e: Env = Env::default();
    let test_data: TestData = create_test_data(&e);

    let depositor: Address = Address::generate(&e);

    assert_eq!(
        test_data
            .contract
            .mock_all_auths()
            .try_withdraw(&depositor, &1)
            .unwrap_err()
            .unwrap(),
        ContractErrors::NotEnoughDeposit,
    );

    let amount: u128 = 100 * SCALAR_7;

    for _ in 0..3 {
        test_data.deposit_asset_sac.mock_all_auths().mint(&depositor, &(amount as i128));
        test_data.contract.mock_all_auths().deposit(&depositor, &amount);
    }

    assert_eq!(
        test_data
            .contract
            .mock_all_auths()
            .try_withdraw(&depositor, &(amount * 5))
            .unwrap_err()
            .unwrap(),
        ContractErrors::NotEnoughDeposit,
    );
}
