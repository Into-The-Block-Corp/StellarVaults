#![cfg(test)]

use crate::constants::SCALAR_7;
use crate::contract::ClaimParams;
use crate::errors::ContractErrors;
use crate::rewards::{compute_leaf_hash, compute_root_from_proof};
use crate::storage::{get_reward_epoch, is_claimed, put_reward_epoch, set_latest_epoch, RewardEpoch};
use crate::tests::test_utils::{create_test_data, TestData};
use hex;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{testutils::Address as _, vec, Address, BytesN, Env, Vec};

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
pub fn test_reward_epoch_out_of_order() {
    let e = Env::default();
    e.mock_all_auths();
    let test_data: TestData = create_test_data(&e);
    e.ledger().set_timestamp(2);

    let user: Address = Address::generate(&e);
    let vault: Address = test_data.vault_a.clone();
    let deposit_id: u64 = 1;
    let epoch: u32 = 1;
    let reward_amount: u128 = 50 * SCALAR_7;
    let root: BytesN<32> = compute_leaf_hash(&e, &vault, epoch, deposit_id, &user, reward_amount);
    let program_end_ts: u64 = 1;

    seed_escrow(&test_data, reward_amount);

    test_data
        .contract
        .set_rewards_root(&test_data.token_a, &vault, &epoch, &root, &reward_amount, &1u32, &program_end_ts);

    let error = test_data
        .contract
        .try_set_rewards_root(&test_data.token_a, &vault, &0, &root, &reward_amount, &1u32, &program_end_ts)
        .unwrap_err()
        .unwrap();

    assert_eq!(error, ContractErrors::RewardEpochOutOfOrder);
}

#[test]
pub fn test_insufficient_escrow_balance() {
    let e = Env::default();
    e.mock_all_auths();
    let test_data: TestData = create_test_data(&e);
    e.ledger().set_timestamp(2);

    let user: Address = Address::generate(&e);
    let vault: Address = test_data.vault_a.clone();
    let deposit_id: u64 = 1;
    let epoch: u32 = 1;
    let reward_amount: u128 = 50 * SCALAR_7;
    let root: BytesN<32> = compute_leaf_hash(&e, &vault, epoch, deposit_id, &user, reward_amount);
    let program_end_ts: u64 = 1;

    let error = test_data
        .contract
        .try_set_rewards_root(&test_data.token_a, &vault, &epoch, &root, &reward_amount, &1u32, &program_end_ts)
        .unwrap_err()
        .unwrap();

    assert_eq!(error, ContractErrors::InsufficientEscrowBalance);
}

#[test]
pub fn test_unrealistic_reward_amount_too_large() {
    let e = Env::default();
    e.mock_all_auths();
    let test_data: TestData = create_test_data(&e);
    e.ledger().set_timestamp(2);

    let user: Address = Address::generate(&e);
    let vault: Address = test_data.vault_a.clone();
    let deposit_id: u64 = 1;
    let epoch: u32 = 1;
    let reward_amount: u128 = u128::MAX;
    let root: BytesN<32> = compute_leaf_hash(&e, &vault, epoch, deposit_id, &user, reward_amount);
    let program_end_ts: u64 = 1;

    seed_escrow(&test_data, 1);

    let error = test_data
        .contract
        .try_set_rewards_root(&test_data.token_a, &vault, &epoch, &root, &reward_amount, &1u32, &program_end_ts)
        .unwrap_err()
        .unwrap();

    assert_eq!(error, ContractErrors::RewardAmountTooLarge);
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
pub fn test_compute_leaf_hash() {
    let e: Env = Env::default();
    let vault: Address = Address::from_str(&e, "CDYMK6HVGZROJECHBDXADSDCJQHBILIZOGPJGRLZQI67PVQH2NZYQSVE");
    let epoch: u32 = 1764211517u32;
    let deposit_id: u64 = 3;
    let owner: Address = Address::from_str(&e, "GATIIRG2KOA37ZMHTO4JSK6CVWPDKZZTYNSD6L4S3TBZ6VOGGS2T74AE");
    let amount: u128 = 100;
    let mut leaf_bytes: [u8; 32] = [0u8; 32];
    hex::decode_to_slice(
        "654df1c78d895e18d03161731cc52ab2ae6c4cac0f89a39c40ebd1c9a6668c57",
        &mut leaf_bytes as &mut [u8],
    )
    .ok();
    let expected_leaf: BytesN<32> = BytesN::from_array(&e, &leaf_bytes);
    let computed_leaf: BytesN<32> = compute_leaf_hash(&e, &vault, epoch, deposit_id, &owner, amount);

    assert_eq!(expected_leaf, computed_leaf);
}

#[test]
pub fn test_compute_root_from_proof() {
    let e: Env = Env::default();
    let vault: Address = Address::from_str(&e, "CDYMK6HVGZROJECHBDXADSDCJQHBILIZOGPJGRLZQI67PVQH2NZYQSVE");
    let epoch: u32 = 1764211517u32;
    let deposit_id: u64 = 3;
    let owner: Address = Address::from_str(&e, "GATIIRG2KOA37ZMHTO4JSK6CVWPDKZZTYNSD6L4S3TBZ6VOGGS2T74AE");
    let amount: u128 = 100;
    let computed_leaf: BytesN<32> = compute_leaf_hash(&e, &vault, epoch, deposit_id, &owner, amount);
    let leaf_index: u32 = 2;

    let mut proof_item_1: [u8; 32] = [0u8; 32];
    hex::decode_to_slice(
        "a8059ffaa7791ba745b60abfcb528684eadb86e99616dc8a1820d7d7aa3eafa3",
        &mut proof_item_1 as &mut [u8],
    )
    .ok();
    let mut proof_item_2: [u8; 32] = [0u8; 32];
    hex::decode_to_slice(
        "1192e3aa4ec37cda826f382d2f808c741f53fbb2c957ef71f81c9f696199b633",
        &mut proof_item_2 as &mut [u8],
    )
    .ok();
    let mut proof_item_3: [u8; 32] = [0u8; 32];
    hex::decode_to_slice(
        "fbd492e08508c20a32a896604ea3cd9055d4207fc7779c4ddabe1ee112a1bf89",
        &mut proof_item_3 as &mut [u8],
    )
    .ok();

    let proof: Vec<BytesN<32>> = vec![
        &e,
        BytesN::from_array(&e, &proof_item_1),
        BytesN::from_array(&e, &proof_item_2),
        BytesN::from_array(&e, &proof_item_3),
    ];

    let mut root: [u8; 32] = [0u8; 32];
    hex::decode_to_slice(
        "dab9c763519418c6adcefeb8faa4d917da3c3e957f1125173fa8261649650e82",
        &mut root as &mut [u8],
    )
    .ok();
    let expected_root: BytesN<32> = BytesN::from_array(&e, &root);

    let computed_root: BytesN<32> = compute_root_from_proof(&e, &computed_leaf, &proof, leaf_index);

    assert_eq!(expected_root, computed_root);
}

#[test]
pub fn test_compute_root_from_proof_2() {
    let e: Env = Env::default();
    let vault: Address = Address::from_str(&e, "CDYMK6HVGZROJECHBDXADSDCJQHBILIZOGPJGRLZQI67PVQH2NZYQSVE");
    let epoch: u32 = 1764211517u32;
    let deposit_id: u64 = 38;
    let owner: Address = Address::from_str(&e, "GBTR22ZYP5LT6M66W7OCZDBWREHVWN4CAFINU4S22O7EEYIKEXUMECZC");
    let amount: u128 = 100;
    let mut leaf_bytes: [u8; 32] = [0u8; 32];
    hex::decode_to_slice(
        "3b96655d588376a970b51cc0d1ab77a4914a4c9383b6c2b77ab59e6a095b011b",
        &mut leaf_bytes as &mut [u8],
    )
    .ok();
    let expected_leaf: BytesN<32> = BytesN::from_array(&e, &leaf_bytes);
    let computed_leaf: BytesN<32> = compute_leaf_hash(&e, &vault, epoch, deposit_id, &owner, amount);

    assert_eq!(expected_leaf, computed_leaf);

    let leaf_index: u32 = 37;

    let mut proof_item_1: [u8; 32] = [0u8; 32];
    hex::decode_to_slice(
        "4844be7eda06e752cadb16110e8d1a0c03904b405368e60a99f7d0439534ed68",
        &mut proof_item_1 as &mut [u8],
    )
    .ok();
    let mut proof_item_2: [u8; 32] = [0u8; 32];
    hex::decode_to_slice(
        "b0fe3a46733e052ff0db904638a804e87eff8bf4a9ab2497221fd07ab1186a9c",
        &mut proof_item_2 as &mut [u8],
    )
    .ok();
    let mut proof_item_3: [u8; 32] = [0u8; 32];
    hex::decode_to_slice(
        "e8fe73bbec24eea9421a80ad35f02d5d83130bb979b61f90090ca1b09c6f94ef",
        &mut proof_item_3 as &mut [u8],
    )
    .ok();
    let mut proof_item_4: [u8; 32] = [0u8; 32];
    hex::decode_to_slice(
        "08308ab4feda66da331c79d82469c1f1806fd1a7d0c818fa45c10ed9fe691c5a",
        &mut proof_item_4 as &mut [u8],
    )
    .ok();
    let mut proof_item_5: [u8; 32] = [0u8; 32];
    hex::decode_to_slice(
        "dd95256c99ff85b0b29c911070c4303051479a99a9dc3a38ed67079100d02804",
        &mut proof_item_5 as &mut [u8],
    )
    .ok();
    let mut proof_item_6: [u8; 32] = [0u8; 32];
    hex::decode_to_slice(
        "2780319b88e7b994b6ace77b1a4ebcb19c5d4d1197de9b32e25dd8ea497de996",
        &mut proof_item_6 as &mut [u8],
    )
    .ok();
    let mut proof_item_7: [u8; 32] = [0u8; 32];
    hex::decode_to_slice(
        "e6d89b1c4bb10be728f7aacbce9f268a5d1dbac90de18f6da59c211887d43498",
        &mut proof_item_7 as &mut [u8],
    )
    .ok();

    let proof: Vec<BytesN<32>> = vec![
        &e,
        BytesN::from_array(&e, &proof_item_1),
        BytesN::from_array(&e, &proof_item_2),
        BytesN::from_array(&e, &proof_item_3),
        BytesN::from_array(&e, &proof_item_4),
        BytesN::from_array(&e, &proof_item_5),
        BytesN::from_array(&e, &proof_item_6),
        BytesN::from_array(&e, &proof_item_7),
    ];

    let mut root: [u8; 32] = [0u8; 32];
    hex::decode_to_slice(
        "a65daa51ef674d537d2e9db271e83dc5d0ac26d23ef2a0f12ff72353ab40a14e",
        &mut root as &mut [u8],
    )
    .ok();
    let expected_root: BytesN<32> = BytesN::from_array(&e, &root);

    let computed_root: BytesN<32> = compute_root_from_proof(&e, &computed_leaf, &proof, leaf_index);

    assert_eq!(expected_root, computed_root);
}

#[test]
pub fn test_claim_more_errors() {
    let e: Env = Env::default();
    let test_data: TestData = create_test_data(&e);
    e.ledger().set_timestamp(100);

    let user: Address = Address::generate(&e);
    let vault: Address = test_data.vault_a.clone();
    let deposit_id: u64 = 1;
    let epoch: u32 = 1;
    let reward_amount: u128 = 50 * SCALAR_7;
    let root: BytesN<32> = compute_leaf_hash(&e, &vault, epoch, deposit_id, &user, reward_amount);
    let proof: Vec<BytesN<32>> = Vec::new(&e);
    let program_end_ts: u64 = e.ledger().timestamp();

    seed_escrow(&test_data, reward_amount);

    test_data
        .contract
        .mock_all_auths()
        .set_rewards_root(&test_data.token_a, &vault, &epoch, &root, &reward_amount, &1u32, &program_end_ts);

    let mut claim = ClaimParams {
        deposit_id,
        amount: reward_amount,
        leaf_index: 0,
        proof: proof.clone(),
    };

    // Should fail if the user hasn't signed the transaction
    assert!(test_data.contract.try_claim(&user, &vault, &epoch, &claim).is_err());

    e.ledger().set_timestamp(1);

    // Should fail because for some reason the timestamp is less than the program_end_ts
    assert_eq!(
        test_data
            .contract
            .mock_all_auths()
            .try_claim(&user, &vault, &epoch, &claim)
            .unwrap_err()
            .unwrap(),
        ContractErrors::RewardEpochNotMatured,
    );

    e.ledger().set_timestamp(100);

    claim.leaf_index = claim.leaf_index + 1;

    assert_eq!(
        test_data
            .contract
            .mock_all_auths()
            .try_claim(&user, &vault, &epoch, &claim)
            .unwrap_err()
            .unwrap(),
        ContractErrors::RewardLeafIndexOutOfBounds,
    );

    claim.leaf_index = claim.leaf_index - 1;
    claim.amount = 0;

    assert_eq!(
        test_data
            .contract
            .mock_all_auths()
            .try_claim(&user, &vault, &epoch, &claim)
            .unwrap_err()
            .unwrap(),
        ContractErrors::RewardInvalidProof,
    );

    claim.amount = reward_amount;

    test_data
        .token_a_tc
        .mock_all_auths()
        .transfer(&test_data.contract.address, &Address::generate(&e), &(reward_amount as i128));

    assert_eq!(
        test_data
            .contract
            .mock_all_auths()
            .try_claim(&user, &vault, &epoch, &claim)
            .unwrap_err()
            .unwrap(),
        ContractErrors::InsufficientEscrowBalance,
    );
}

#[test]
pub fn test_impossible_reward_amount_too_large() {
    let e: Env = Env::default();
    let test_data: TestData = create_test_data(&e);
    e.ledger().set_timestamp(100);

    let user: Address = Address::generate(&e);
    let vault: Address = test_data.vault_a.clone();
    let deposit_id: u64 = 1;
    let epoch: u32 = 1;
    let reward_amount: u128 = u128::MAX;
    let root: BytesN<32> = compute_leaf_hash(&e, &vault, epoch, deposit_id, &user, reward_amount);
    let proof: Vec<BytesN<32>> = Vec::new(&e);
    let program_end_ts: u64 = e.ledger().timestamp();

    e.as_contract(&test_data.contract.address, || {
        let epoch_data = RewardEpoch {
            root: root.clone(),
            asset: test_data.token_a.clone(),
            total_rewards: reward_amount,
            leaf_count: 1,
            program_end_ts,
        };
        put_reward_epoch(&e, &vault, epoch, &epoch_data);
        set_latest_epoch(&e, &vault, epoch);
    });

    let claim = ClaimParams {
        deposit_id,
        amount: reward_amount,
        leaf_index: 0,
        proof: proof.clone(),
    };

    assert_eq!(
        test_data
            .contract
            .mock_all_auths()
            .try_claim(&user, &vault, &epoch, &claim)
            .unwrap_err()
            .unwrap(),
        ContractErrors::RewardAmountTooLarge,
    );
}
