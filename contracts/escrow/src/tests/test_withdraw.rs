#![cfg(test)]

extern crate alloc;

use crate::constants::SCALAR_7;
use crate::tests::test_utils::{create_test_data, TestData};
use alloc::vec::Vec as StdVec;
use soroban_sdk::testutils::{Events, MockAuth, MockAuthInvoke};
use soroban_sdk::{symbol_short, vec, Address, Env, IntoVal, Map, Symbol, TryFromVal, Vec as SorobanVec};

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
fn test_withdraw_emits_admin_withdraw_event() {
    let e: Env = Env::default();
    let test_data: TestData = create_test_data(&e);
    let initial_balance: u128 = 1_000 * SCALAR_7;
    let keep_amount: u128 = 400 * SCALAR_7;
    let expected_transfer: u128 = initial_balance - keep_amount;

    test_data
        .token_a_sac
        .mock_all_auths()
        .mint(&test_data.contract.address, &(initial_balance as i128));
    test_data
        .token_b_sac
        .mock_all_auths()
        .mint(&test_data.contract.address, &(initial_balance as i128));

    // Drain any prior events (e.g. mints) so we only inspect events from withdraw.
    let _ = e.events().all();

    test_data
        .contract
        .mock_auths(&[MockAuth {
            address: &test_data.admin,
            invoke: &MockAuthInvoke {
                contract: &test_data.contract.address,
                fn_name: "withdraw",
                args: (vec![
                    &e,
                    (test_data.token_a.clone(), keep_amount),
                    (test_data.token_b.clone(), keep_amount),
                ],)
                    .into_val(&e),
                sub_invokes: &[],
            },
        }])
        .withdraw(&vec![
            &e,
            (test_data.token_a.clone(), keep_amount),
            (test_data.token_b.clone(), keep_amount),
        ]);

    let escrow_events: StdVec<_> = e
        .events()
        .all()
        .into_iter()
        .filter(|(addr, _, _)| *addr == test_data.contract.address)
        .collect();

    assert_eq!(escrow_events.len(), 2);

    for (idx, expected_asset) in [&test_data.token_a, &test_data.token_b].iter().enumerate() {
        let (_, topics_val, data_val) = &escrow_events[idx];

        let topics: SorobanVec<soroban_sdk::Val> = SorobanVec::try_from_val(&e, topics_val).unwrap();
        assert_eq!(topics.len(), 2);
        let t0: Symbol = Symbol::try_from_val(&e, &topics.get_unchecked(0)).unwrap();
        let t1: Symbol = Symbol::try_from_val(&e, &topics.get_unchecked(1)).unwrap();
        assert_eq!(t0, symbol_short!("ADMIN"));
        assert_eq!(t1, symbol_short!("withdraw"));

        let data_map: Map<Symbol, soroban_sdk::Val> = Map::try_from_val(&e, data_val).unwrap();
        let ev_asset: Address =
            Address::try_from_val(&e, &data_map.get_unchecked(Symbol::new(&e, "asset"))).unwrap();
        let ev_admin: Address =
            Address::try_from_val(&e, &data_map.get_unchecked(Symbol::new(&e, "admin"))).unwrap();
        let ev_amount: u128 =
            u128::try_from_val(&e, &data_map.get_unchecked(Symbol::new(&e, "amount"))).unwrap();

        assert_eq!(ev_asset, **expected_asset);
        assert_eq!(ev_admin, test_data.admin);
        assert_eq!(ev_amount, expected_transfer);
    }
}

#[test]
fn test_withdraw_does_not_emit_event_when_nothing_to_transfer() {
    let e: Env = Env::default();
    let test_data: TestData = create_test_data(&e);
    let balance: u128 = 500 * SCALAR_7;

    test_data
        .token_a_sac
        .mock_all_auths()
        .mint(&test_data.contract.address, &(balance as i128));

    let _ = e.events().all();

    // Keep amount equals contract balance => transfer_amount = 0 => no event.
    test_data
        .contract
        .mock_auths(&[MockAuth {
            address: &test_data.admin,
            invoke: &MockAuthInvoke {
                contract: &test_data.contract.address,
                fn_name: "withdraw",
                args: (vec![&e, (test_data.token_a.clone(), balance)],).into_val(&e),
                sub_invokes: &[],
            },
        }])
        .withdraw(&vec![&e, (test_data.token_a.clone(), balance)]);

    let escrow_events: StdVec<_> = e
        .events()
        .all()
        .into_iter()
        .filter(|(addr, _, _)| *addr == test_data.contract.address)
        .collect();

    assert!(escrow_events.is_empty());
}
