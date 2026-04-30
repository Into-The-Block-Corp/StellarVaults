use crate::constants::{LEDGER_MONTH, LEDGER_WEEK};
use soroban_sdk::{contracttype, Address, BytesN, Env};

const EPOCH_MIN_TTL: u32 = LEDGER_WEEK;
const EPOCH_MAX_TTL: u32 = LEDGER_MONTH * 6;
const CLAIM_MIN_TTL: u32 = LEDGER_WEEK;
const CLAIM_MAX_TTL: u32 = LEDGER_MONTH * 6;
const COMMITTED_MIN_TTL: u32 = LEDGER_WEEK;
const COMMITTED_MAX_TTL: u32 = LEDGER_MONTH * 6;

#[contracttype]
pub enum StorageKeys {
    Admin,                              // --> Address
    Allowance((Address, Address)),      // -> Allowance
    LatestRewardEpoch(Address),         // -> u32
    RewardEpoch((Address, u32)),        // -> RewardEpoch
    RewardClaimed((Address, u32, u64)), // -> u64 bitset word
    CommittedRewards(Address),           // asset -> u128 total committed rewards
}

#[contracttype]
#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub struct Allowance {
    pub target: Address,
    pub asset: Address,
    pub amount: u128,
    pub current: u128,
    pub deadline: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardEpoch {
    pub root: BytesN<32>,
    pub asset: Address,
    pub total_rewards: u128,
    pub leaf_count: u32,
    pub program_end_ts: u64,
}

pub fn admin(e: &Env, value: Option<Address>) -> Option<Address> {
    if let Some(v) = value {
        e.storage().instance().set(&StorageKeys::Admin, &v);
    }

    e.storage().instance().get(&StorageKeys::Admin)
}

pub fn allowance(e: &Env, target: &Address, asset: &Address, value: Option<Allowance>) -> Option<Allowance> {
    let key: StorageKeys = StorageKeys::Allowance((target.clone(), asset.clone()));

    if let Some(v) = value {
        e.storage().persistent().set(&key, &v);
    }

    e.storage().persistent().get(&key)
}

pub fn bump_instance(e: &Env) {
    e.storage().instance().extend_ttl(LEDGER_WEEK, LEDGER_MONTH);
}

pub fn bump_allowance(e: &Env, target: &Address, asset: &Address) {
    e.storage().persistent().extend_ttl(
        &StorageKeys::Allowance((target.clone(), asset.clone())),
        LEDGER_WEEK,
        LEDGER_MONTH * 3,
    );
}

pub fn put_reward_epoch(e: &Env, vault: &Address, epoch: u32, value: &RewardEpoch) {
    let key = StorageKeys::RewardEpoch((vault.clone(), epoch));
    let storage = e.storage().persistent();
    storage.set(&key, value);
    storage.extend_ttl(&key, EPOCH_MIN_TTL, EPOCH_MAX_TTL);
}

pub fn get_reward_epoch(e: &Env, vault: &Address, epoch: u32) -> Option<RewardEpoch> {
    let key = StorageKeys::RewardEpoch((vault.clone(), epoch));
    e.storage().persistent().get(&key)
}

pub fn bump_reward_epoch(e: &Env, vault: &Address, epoch: u32) {
    let key = StorageKeys::RewardEpoch((vault.clone(), epoch));
    e.storage().persistent().extend_ttl(&key, EPOCH_MIN_TTL, EPOCH_MAX_TTL);
}

pub fn get_latest_epoch(e: &Env, vault: &Address) -> Option<u32> {
    e.storage().instance().get(&StorageKeys::LatestRewardEpoch(vault.clone()))
}

pub fn set_latest_epoch(e: &Env, vault: &Address, epoch: u32) {
    let storage = e.storage().instance();
    let key = StorageKeys::LatestRewardEpoch(vault.clone());
    storage.set(&key, &epoch);
    storage.extend_ttl(LEDGER_WEEK, LEDGER_MONTH);
}

pub fn is_claimed(e: &Env, vault: &Address, epoch: u32, deposit_id: u64) -> bool {
    let (word_index, mask) = claim_position(deposit_id);
    let key = StorageKeys::RewardClaimed((vault.clone(), epoch, word_index));
    let word: u64 = e.storage().persistent().get(&key).unwrap_or(0);
    (word & mask) != 0
}

pub fn set_claimed(e: &Env, vault: &Address, epoch: u32, deposit_id: u64) {
    let (word_index, mask) = claim_position(deposit_id);
    let key = StorageKeys::RewardClaimed((vault.clone(), epoch, word_index));
    let storage = e.storage().persistent();
    let mut word: u64 = storage.get(&key).unwrap_or(0);
    word |= mask;
    storage.set(&key, &word);
    storage.extend_ttl(&key, CLAIM_MIN_TTL, CLAIM_MAX_TTL);
}

pub fn get_committed_rewards(e: &Env, asset: &Address) -> u128 {
    e.storage()
        .persistent()
        .get(&StorageKeys::CommittedRewards(asset.clone()))
        .unwrap_or(0)
}

pub fn add_committed_rewards(e: &Env, asset: &Address, amount: u128) {
    let key = StorageKeys::CommittedRewards(asset.clone());
    let storage = e.storage().persistent();
    let current = get_committed_rewards(e, asset);
    storage.set(&key, &(current + amount));
    storage.extend_ttl(&key, COMMITTED_MIN_TTL, COMMITTED_MAX_TTL);
}

pub fn reduce_committed_rewards(e: &Env, asset: &Address, amount: u128) {
    let key = StorageKeys::CommittedRewards(asset.clone());
    let storage = e.storage().persistent();
    let current = get_committed_rewards(e, asset);
    let new_amount = current.saturating_sub(amount);
    storage.set(&key, &new_amount);
    storage.extend_ttl(&key, COMMITTED_MIN_TTL, COMMITTED_MAX_TTL);
}

fn claim_position(deposit_id: u64) -> (u64, u64) {
    let word_index = deposit_id / 64;
    let offset = (deposit_id % 64) as u32;
    let mask = 1u64 << offset;
    (word_index, mask)
}
