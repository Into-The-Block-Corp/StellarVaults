use crate::constants::{LEDGER_MONTH, LEDGER_WEEK};
use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DepositRecord {
    pub owner: Address,
    pub amount: u128,
}

#[contracttype]
pub enum DepositStorageKey {
    NextDepositId,
    Deposit(Address),
    TotalPrincipal,
}

/// Atomically increments and returns the next available deposit identifier.
/// Ensures each deposit gets a unique, deterministic ID that off-chain jobs can reference.
pub fn consume_next_deposit_id(e: &Env) -> u64 {
    let next: u64 = e.storage().instance().get(&DepositStorageKey::NextDepositId).unwrap_or(1);

    e.storage().instance().set(&DepositStorageKey::NextDepositId, &(next + 1));

    next
}

pub fn deposit(e: &Env, owner: &Address, value: Option<DepositRecord>, remove: bool) -> Option<DepositRecord> {
    let key = DepositStorageKey::Deposit(owner.clone());
    if let Some(v) = value {
        e.storage().persistent().set(&key, &v);
        e.storage().persistent().extend_ttl(&key, LEDGER_WEEK, LEDGER_MONTH);
    } else if remove {
        e.storage().persistent().remove(&key);
    }

    let result = e.storage().persistent().get(&key);
    if result.is_some() {
        e.storage().persistent().extend_ttl(&key, LEDGER_WEEK, LEDGER_MONTH);
    }
    result
}

/// Adds `amount` to the running total principal locked in the vault and returns the updated sum.
pub fn add_total_principal(e: &Env, amount: u128) -> u128 {
    let current: u128 = e.storage().instance().get(&DepositStorageKey::TotalPrincipal).unwrap_or(0);
    let updated: u128 = current + amount;
    e.storage().instance().set(&DepositStorageKey::TotalPrincipal, &updated);
    updated
}

/// Reduces the `amount` from the total principal locked in the vault and returns the updated sum.
pub fn reduce_total_principal(e: &Env, amount: &u128) -> u128 {
    let current: u128 = e.storage().instance().get(&DepositStorageKey::TotalPrincipal).unwrap_or(0);
    if *amount > current {
        panic!("reduce_total_principal: underflow — withdrawing {} but only {} recorded", amount, current);
    }
    let updated: u128 = current - amount;
    e.storage().instance().set(&DepositStorageKey::TotalPrincipal, &updated);
    updated
}

/// Reads the aggregate principal currently tracked for the vault.
pub fn total_principal(e: &Env) -> u128 {
    e.storage().instance().get(&DepositStorageKey::TotalPrincipal).unwrap_or(0)
}
