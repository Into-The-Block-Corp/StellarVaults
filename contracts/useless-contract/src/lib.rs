#![no_std]

// This is a dumb contract to test other contracts upgrades and comply with the suggestion from the auditors

use soroban_sdk::{Symbol, contract, contractimpl, symbol_short};

trait UselessContractTrait {
    fn hello() -> Symbol;
}

#[contract]
struct UselessContract;

#[contractimpl]
impl UselessContractTrait for UselessContract {
    fn hello() -> Symbol {
        symbol_short!("world")
    }
}
