use crate::constants::{LEDGER_MONTH, LEDGER_WEEK};
use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
pub enum CoreStorageKeys {
    Admin,        // --> Address
    DepositAsset, // --> Address
    Paused,       // --> bool
}

pub fn admin(e: &Env, value: Option<Address>) -> Option<Address> {
    if let Some(v) = value {
        e.storage().instance().set(&CoreStorageKeys::Admin, &v);
    }

    e.storage().instance().get(&CoreStorageKeys::Admin)
}

pub fn deposit_asset(e: &Env, value: Option<Address>) -> Option<Address> {
    if let Some(v) = value {
        e.storage().instance().set(&CoreStorageKeys::DepositAsset, &v);
    }

    e.storage().instance().get(&CoreStorageKeys::DepositAsset)
}

pub fn paused(e: &Env, value: Option<bool>) -> Option<bool> {
    if let Some(v) = value {
        e.storage().instance().set(&CoreStorageKeys::Paused, &v);
    }

    e.storage().instance().get(&CoreStorageKeys::Paused)
}

pub fn bump_instance(e: &Env) {
    e.storage().instance().extend_ttl(LEDGER_WEEK, LEDGER_MONTH);
}
