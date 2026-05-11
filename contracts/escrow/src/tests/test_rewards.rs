#![cfg(test)]

extern crate alloc;

use crate::constants::SCALAR_7;
use crate::contract::ClaimParams;
use crate::errors::ContractErrors;
use crate::rewards::compute_leaf_hash;
use crate::storage::{add_committed_rewards, get_committed_rewards, get_reward_epoch, is_claimed, set_latest_epoch};
use crate::tests::test_utils::{create_test_data, TestData};
use alloc::vec::Vec as StdVec;
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::{symbol_short, Address, BytesN, Env, Map, Symbol, TryFromVal, Vec};

fn seed_escrow(test_data: &TestData, amount: u128) {
    test_data
        .token_a_sac
        .mock_all_auths()
        .mint(&test_data.contract.address, &(amount as i128));
}

#[test]
pub fn test_set_root_and_claim_rewards_success() {
    let e = Env::default();
    e.mock_all_auths();
    let test_data = create_test_data(&e);

    let user = Address::generate(&e);
    let vault = test_data.vault_a.clone();
    let deposit_id: u64 = 1;
    let epoch: u32 = 1;
    let reward_amount: u128 = 50 * SCALAR_7;
    let root = compute_leaf_hash(&e, &vault, epoch, deposit_id, &user, reward_amount);
    let proof: Vec<BytesN<32>> = Vec::new(&e);
    let program_end_ts = e.ledger().timestamp();

    seed_escrow(&test_data, reward_amount);

    test_data
        .contract
        .mock_all_auths()
        .set_rewards_root(&test_data.token_a, &vault, &epoch, &root, &reward_amount, &1u32, &program_end_ts);

    let stored = e.as_contract(&test_data.contract.address, || {
        get_reward_epoch(&e, &vault, epoch).expect("epoch stored")
    });
    assert_eq!(stored.root, root);
    assert_eq!(stored.asset, test_data.token_a);
    assert_eq!(stored.total_rewards, reward_amount);
    assert_eq!(stored.leaf_count, 1);

    assert_eq!(test_data.token_a_tc.balance(&user), 0);

    let claim = ClaimParams {
        deposit_id,
        amount: reward_amount,
        leaf_index: 0,
        proof: proof.clone(),
    };

    test_data.contract.mock_all_auths().claim(&user, &vault, &epoch, &claim);

    assert_eq!(test_data.token_a_tc.balance(&user), reward_amount as i128);
    assert!(e.as_contract(&test_data.contract.address, || is_claimed(&e, &vault, epoch, deposit_id)));

    let duplicate_claim = ClaimParams {
        deposit_id,
        amount: reward_amount,
        leaf_index: 0,
        proof: proof.clone(),
    };

    let second = test_data
        .contract
        .mock_all_auths()
        .try_claim(&user, &vault, &epoch, &duplicate_claim);
    assert_eq!(second.unwrap_err().unwrap(), ContractErrors::RewardLeafAlreadyClaimed);
}

#[test]
pub fn test_set_rewards_root_requires_mature_program() {
    let e = Env::default();
    e.mock_all_auths();
    let test_data = create_test_data(&e);

    let user = Address::generate(&e);
    let deposit_id: u64 = 2;

    let epoch: u32 = 2;
    let reward_amount: u128 = 10 * SCALAR_7;
    let vault = test_data.vault_a.clone();
    let root = compute_leaf_hash(&e, &vault, epoch, deposit_id, &user, reward_amount);
    let proof: Vec<BytesN<32>> = Vec::new(&e);

    seed_escrow(&test_data, reward_amount);

    let future_program_end = e.ledger().timestamp() + 10;
    let result = test_data.contract.mock_all_auths().try_set_rewards_root(
        &test_data.token_a,
        &vault,
        &epoch,
        &root,
        &reward_amount,
        &1u32,
        &future_program_end,
    );

    assert_eq!(result.unwrap_err().unwrap(), ContractErrors::RewardEpochNotMatured);

    assert!(e.as_contract(&test_data.contract.address, || get_reward_epoch(&e, &vault, epoch).is_none()));

    let claim_attempt = ClaimParams {
        deposit_id,
        amount: reward_amount,
        leaf_index: 0,
        proof: proof.clone(),
    };

    let claim_err = test_data.contract.mock_all_auths().try_claim(&user, &vault, &epoch, &claim_attempt);
    assert_eq!(claim_err.unwrap_err().unwrap(), ContractErrors::RewardEpochNotFound);
}

#[test]
pub fn test_claim_with_invalid_amount_rejected() {
    let e = Env::default();
    e.mock_all_auths();
    let test_data = create_test_data(&e);

    let user = Address::generate(&e);
    let deposit_id: u64 = 3;

    let epoch: u32 = 3;
    let reward_amount: u128 = 25 * SCALAR_7;
    let vault = test_data.vault_a.clone();
    let root = compute_leaf_hash(&e, &vault, epoch, deposit_id, &user, reward_amount);
    let proof: Vec<BytesN<32>> = Vec::new(&e);
    let program_end_ts = e.ledger().timestamp();

    seed_escrow(&test_data, reward_amount);

    test_data
        .contract
        .mock_all_auths()
        .set_rewards_root(&test_data.token_a, &vault, &epoch, &root, &reward_amount, &1u32, &program_end_ts);

    let wrong_amount = reward_amount + SCALAR_7;
    let wrong_claim = ClaimParams {
        deposit_id,
        amount: wrong_amount,
        leaf_index: 0,
        proof: proof.clone(),
    };

    let err = test_data.contract.mock_all_auths().try_claim(&user, &vault, &epoch, &wrong_claim);

    assert_eq!(err.unwrap_err().unwrap(), ContractErrors::RewardInvalidProof);
    assert_eq!(test_data.token_a_tc.balance(&user), 0);
}

#[test]
fn test_sweep_expired_epoch_fails_when_amount_exceeds_committed_rewards() {
    let e = Env::default();
    e.mock_all_auths();
    let test_data = create_test_data(&e);

    let vault = test_data.vault_a.clone();
    let epoch: u32 = 1;
    let committed: u128 = 100 * SCALAR_7;
    let sweep_amount: u128 = committed + 1;

    // Seed enough balance so the balance check passes; failure must come from the committed-rewards check.
    seed_escrow(&test_data, committed * 2);

    // Simulate an expired persistent epoch entry: latest_epoch and committed_rewards exist,
    // but the persistent RewardEpoch entry does not.
    e.as_contract(&test_data.contract.address, || {
        set_latest_epoch(&e, &vault, epoch);
        add_committed_rewards(&e, &test_data.token_a, committed);
    });

    let result = test_data
        .contract
        .mock_all_auths()
        .try_sweep_expired_epoch(&test_data.token_a, &vault, &epoch, &sweep_amount);

    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractErrors::SweepAmountExceedsCommittedRewards
    );
    assert_eq!(test_data.token_a_tc.balance(&test_data.admin), 0);
}

#[test]
fn test_sweep_expired_epoch_succeeds_when_amount_within_committed_rewards() {
    let e = Env::default();
    e.mock_all_auths();
    let test_data = create_test_data(&e);

    let vault = test_data.vault_a.clone();
    let epoch: u32 = 1;
    let committed: u128 = 100 * SCALAR_7;
    let sweep_amount: u128 = committed;

    seed_escrow(&test_data, committed);

    e.as_contract(&test_data.contract.address, || {
        set_latest_epoch(&e, &vault, epoch);
        add_committed_rewards(&e, &test_data.token_a, committed);
    });

    test_data
        .contract
        .mock_all_auths()
        .sweep_expired_epoch(&test_data.token_a, &vault, &epoch, &sweep_amount);

    assert_eq!(test_data.token_a_tc.balance(&test_data.contract.address), 0);
    assert_eq!(test_data.token_a_tc.balance(&test_data.admin), sweep_amount as i128);

    let remaining = e.as_contract(&test_data.contract.address, || {
        get_committed_rewards(&e, &test_data.token_a)
    });
    assert_eq!(remaining, 0);
}

#[test]
fn test_sweep_expired_epoch_emits_event() {
    let e = Env::default();
    e.mock_all_auths();
    let test_data = create_test_data(&e);

    let vault = test_data.vault_a.clone();
    let epoch: u32 = 7;
    let committed: u128 = 100 * SCALAR_7;
    let sweep_amount: u128 = committed;

    seed_escrow(&test_data, committed);

    e.as_contract(&test_data.contract.address, || {
        set_latest_epoch(&e, &vault, epoch);
        add_committed_rewards(&e, &test_data.token_a, committed);
    });

    // Drain prior events (e.g. mints) so we only inspect the sweep.
    let _ = e.events().all();

    test_data
        .contract
        .mock_all_auths()
        .sweep_expired_epoch(&test_data.token_a, &vault, &epoch, &sweep_amount);

    let escrow_events: StdVec<_> = e
        .events()
        .all()
        .into_iter()
        .filter(|(addr, _, _)| *addr == test_data.contract.address)
        .collect();

    assert_eq!(escrow_events.len(), 1);
    let (_, topics_val, data_val) = &escrow_events[0];

    let topics: Vec<soroban_sdk::Val> = Vec::try_from_val(&e, topics_val).unwrap();
    assert_eq!(topics.len(), 2);
    let t0: Symbol = Symbol::try_from_val(&e, &topics.get_unchecked(0)).unwrap();
    let t1: Symbol = Symbol::try_from_val(&e, &topics.get_unchecked(1)).unwrap();
    assert_eq!(t0, symbol_short!("SWEEP"));
    assert_eq!(t1, symbol_short!("epoch"));

    let data_map: Map<Symbol, soroban_sdk::Val> = Map::try_from_val(&e, data_val).unwrap();
    let ev_asset: Address =
        Address::try_from_val(&e, &data_map.get_unchecked(Symbol::new(&e, "asset"))).unwrap();
    let ev_vault: Address =
        Address::try_from_val(&e, &data_map.get_unchecked(Symbol::new(&e, "vault"))).unwrap();
    let ev_epoch: u32 =
        u32::try_from_val(&e, &data_map.get_unchecked(Symbol::new(&e, "epoch"))).unwrap();
    let ev_admin: Address =
        Address::try_from_val(&e, &data_map.get_unchecked(Symbol::new(&e, "admin"))).unwrap();
    let ev_amount: u128 =
        u128::try_from_val(&e, &data_map.get_unchecked(Symbol::new(&e, "amount"))).unwrap();

    assert_eq!(ev_asset, test_data.token_a);
    assert_eq!(ev_vault, vault);
    assert_eq!(ev_epoch, epoch);
    assert_eq!(ev_admin, test_data.admin);
    assert_eq!(ev_amount, sweep_amount);
}
