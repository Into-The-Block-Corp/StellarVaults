#![cfg(test)]

use crate::constants::SCALAR_7;
use crate::tests::test_utils::{create_test_data, TestData};
use soroban_sdk::testutils::{MockAuth, MockAuthInvoke};
use soroban_sdk::{vec, Env, IntoVal};

#[test]
fn test_withdraw() {
    let e: Env = Env::default();
    let test_data: TestData = create_test_data(&e);
    let amount_to_allow: u128 = 500_000_000 * SCALAR_7;

    test_data
        .token_a_sac
        .mock_all_auths()
        .mint(&test_data.contract.address, &(amount_to_allow as i128));

    test_data
        .token_b_sac
        .mock_all_auths()
        .mint(&test_data.contract.address, &(amount_to_allow as i128));

    assert!(test_data
        .contract
        .try_withdraw(&vec![
            &e,
            (test_data.token_a.clone(), amount_to_allow / 2),
            (test_data.token_b.clone(), amount_to_allow / 2)
        ])
        .is_err());

    test_data
        .contract
        .mock_auths(&[MockAuth {
            address: &test_data.admin,
            invoke: &MockAuthInvoke {
                contract: &test_data.contract.address,
                fn_name: "withdraw",
                args: (vec![
                    &e,
                    (test_data.token_a.clone(), amount_to_allow / 2),
                    (test_data.token_b.clone(), amount_to_allow / 2),
                ],)
                    .into_val(&e),
                sub_invokes: &[],
            },
        }])
        .withdraw(&vec![
            &e,
            (test_data.token_a.clone(), amount_to_allow / 2),
            (test_data.token_b.clone(), amount_to_allow / 2),
        ]);

    assert_eq!(
        test_data.token_a_tc.balance(&test_data.contract.address),
        (amount_to_allow / 2) as i128
    );

    assert_eq!(
        test_data.token_b_tc.balance(&test_data.contract.address),
        (amount_to_allow / 2) as i128
    );

    assert_eq!(test_data.token_a_tc.balance(&test_data.admin), (amount_to_allow / 2) as i128);

    assert_eq!(test_data.token_b_tc.balance(&test_data.admin), (amount_to_allow / 2) as i128);
}

#[test]
fn test_unrealistic_withdraw_overflow() {
    let e: Env = Env::default();
    e.mock_all_auths();
    let test_data: TestData = create_test_data(&e);

    test_data.token_a_sac.mock_all_auths().mint(&test_data.contract.address, &i128::MAX);
    test_data.token_b_sac.mock_all_auths().mint(&test_data.contract.address, &i128::MAX);

    // This will fail, not because an i128 overflow but because we are trying to subtract more than assets can even hold
    assert!(test_data
        .contract
        .try_withdraw(&vec![
            &e,
            (test_data.token_a.clone(), u128::MAX),
            (test_data.token_b.clone(), u128::MAX),
        ])
        .is_err());
}
