#![cfg(test)]

extern crate alloc;

use crate::constants::SCALAR_7;
use crate::contract::VaultContractClient;
use crate::errors::ContractErrors;
use crate::storage::deposit_state::{deposit, total_principal, DepositRecord};
use crate::tests::test_utils::{create_test_data, TestData};
use alloc::vec::Vec as StdVec;
use soroban_sdk::testutils::{Address as _, Events, MockAuth, MockAuthInvoke};
use soroban_sdk::{symbol_short, Address, Env, Map, String, Symbol, TryFromVal, Vec as SorobanVec};
use soroban_sdk::{token, IntoVal};

#[test]
fn deposit_successful_flow() {
    let e: Env = Env::default();
    e.mock_all_auths();

    let test_data: TestData = create_test_data(&e);
    let contract: VaultContractClient = test_data.contract;
    let deposit_asset: Address = test_data.deposit_asset;

    let user = Address::generate(&e);
    let amount: u128 = 500 * SCALAR_7;

    let token_client = token::Client::new(&e, &deposit_asset);
    let mint_client = token::StellarAssetClient::new(&e, &deposit_asset);
    mint_client.mint(&user, &(amount as i128));
    let _ = e.events().all();

    let deposit_id = contract.deposit(&user, &amount, &None);

    let events_after_deposit = e.events().all();

    e.as_contract(&contract.address, || {
        let record: DepositRecord = deposit(&e, &user, None, false).unwrap();
        assert_eq!(record.owner, user);
        assert_eq!(record.amount, amount);
        assert_eq!(total_principal(&e), amount);
    });
    let started_at = e.ledger().timestamp();

    let contract_balance = token_client.balance(&contract.address);
    assert_eq!(contract_balance, amount as i128);
    assert_eq!(token_client.balance(&user), 0);

    let native_events: StdVec<_> = events_after_deposit.into_iter().collect();
    assert!(!native_events.is_empty());

    let (event_contract, topics_val, data_val) = native_events.last().unwrap();
    assert_eq!(*event_contract, contract.address);

    let topics_vec: SorobanVec<soroban_sdk::Val> = SorobanVec::try_from_val(&e, topics_val).unwrap();
    assert_eq!(topics_vec.len(), 2);
    let topic0: Symbol = Symbol::try_from_val(&e, &topics_vec.get_unchecked(0)).unwrap();
    let topic1: Symbol = Symbol::try_from_val(&e, &topics_vec.get_unchecked(1)).unwrap();
    assert_eq!(topic0, symbol_short!("DEPOSIT"));
    assert_eq!(topic1, symbol_short!("create"));

    let data_map: Map<Symbol, soroban_sdk::Val> = Map::try_from_val(&e, data_val).unwrap();
    let event_deposit_id: u64 = u64::try_from_val(&e, &data_map.get_unchecked(Symbol::new(&e, "deposit_id"))).unwrap();
    let event_owner: Address = Address::try_from_val(&e, &data_map.get_unchecked(Symbol::new(&e, "owner"))).unwrap();
    let event_amount: u128 = u128::try_from_val(&e, &data_map.get_unchecked(Symbol::new(&e, "amount"))).unwrap();
    let event_started: u64 = u64::try_from_val(&e, &data_map.get_unchecked(Symbol::new(&e, "started_at"))).unwrap();

    assert_eq!(event_deposit_id, deposit_id);
    assert_eq!(event_owner, user);
    assert_eq!(event_amount, amount);
    assert_eq!(event_started, started_at);
    let event_referral_id: Option<String> =
        Option::<String>::try_from_val(&e, &data_map.get_unchecked(Symbol::new(&e, "referral_id"))).unwrap();
    assert_eq!(event_referral_id, Option::<String>::None);
}

#[test]
fn deposit_rejects_when_paused() {
    let e: Env = Env::default();
    e.mock_all_auths();

    let test_data: TestData = create_test_data(&e);
    let contract: VaultContractClient = test_data.contract;
    let deposit_asset: Address = test_data.deposit_asset;

    let user = Address::generate(&e);
    let amount: u128 = 100 * SCALAR_7;

    let mint_client = token::StellarAssetClient::new(&e, &deposit_asset);
    mint_client.mint(&user, &(amount as i128));
    let _ = e.events().all();

    e.as_contract(&contract.address, || {
        crate::storage::core::paused(&e, Some(true));
    });

    let vault_paused_error = contract.try_deposit(&user, &amount, &None).unwrap_err().unwrap();
    assert_eq!(vault_paused_error, ContractErrors::VaultPaused.into());
}

#[test]
fn deposit_emits_referral_id_when_present() {
    let e: Env = Env::default();
    e.mock_all_auths();

    let test_data: TestData = create_test_data(&e);
    let contract: VaultContractClient = test_data.contract;
    let deposit_asset: Address = test_data.deposit_asset;

    let user = Address::generate(&e);
    let amount: u128 = 250 * SCALAR_7;
    let referral_id: Option<String> = Some(String::from_str(&e, "ref-42"));

    let mint_client = token::StellarAssetClient::new(&e, &deposit_asset);
    mint_client.mint(&user, &(amount as i128));
    let _ = e.events().all();

    let deposit_id = contract.deposit(&user, &amount, &referral_id);
    let events_after_deposit = e.events().all();

    let native_events: StdVec<_> = events_after_deposit.into_iter().collect();
    assert!(!native_events.is_empty());

    let (_, _, data_val) = native_events.last().unwrap();
    let data_map: Map<Symbol, soroban_sdk::Val> = Map::try_from_val(&e, data_val).unwrap();

    let event_deposit_id: u64 = u64::try_from_val(&e, &data_map.get_unchecked(Symbol::new(&e, "deposit_id"))).unwrap();
    let event_referral_id: Option<String> =
        Option::<String>::try_from_val(&e, &data_map.get_unchecked(Symbol::new(&e, "referral_id"))).unwrap();

    assert_eq!(event_deposit_id, deposit_id);
    assert_eq!(event_referral_id, referral_id);
}

#[test]
fn test_expected_deposit_errors() {
    let e: Env = Env::default();
    let test_data: TestData = create_test_data(&e);

    let depositor: Address = Address::generate(&e);

    // Should fail if the depositor has no funds
    assert!(test_data
        .contract
        .mock_all_auths()
        .try_deposit(&depositor, &SCALAR_7, &None)
        .is_err());

    // We transfer funds so we are sure next errors aren't because of lack of funds
    test_data.deposit_asset_sac.mock_all_auths().mint(&depositor, &(SCALAR_7 as i128));

    // Should fail if the transaction is not signed by the depositor
    assert!(test_data.contract.try_deposit(&depositor, &SCALAR_7, &None).is_err());

    // If the deposit is zero it will fail
    assert!(test_data.contract.try_deposit(&depositor, &0, &None).is_err());

    // This one will finally pass
    test_data
        .contract
        .mock_auths(&[MockAuth {
            address: &depositor,
            invoke: &MockAuthInvoke {
                contract: &test_data.contract.address,
                fn_name: "deposit",
                args: (depositor.clone(), SCALAR_7.clone(), None::<String>).into_val(&e),
                sub_invokes: &[MockAuthInvoke {
                    contract: &test_data.deposit_asset,
                    fn_name: "transfer",
                    args: (depositor.clone(), test_data.contract.address.clone(), SCALAR_7 as i128).into_val(&e),
                    sub_invokes: &[],
                }],
            },
        }])
        .deposit(&depositor, &SCALAR_7, &None);
}
