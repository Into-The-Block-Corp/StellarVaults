use crate::errors::ContractErrors;
use crate::events::DepositEvent;
use crate::storage::core::{bump_instance, deposit_asset, paused};
use crate::storage::deposit_state::{add_total_principal, consume_next_deposit_id, deposit, DepositRecord};
use soroban_sdk::token::TokenClient;
use soroban_sdk::{panic_with_error, token, Address, Env};

pub fn handle_deposit(e: &Env, from: Address, amount: u128) -> u64 {
    if paused(e, None).unwrap_or(false) {
        panic_with_error!(e, ContractErrors::VaultPaused);
    }

    let deposit_asset_address: Address = deposit_asset(e, None).unwrap();

    from.require_auth();

    let token_client: TokenClient = token::Client::new(e, &deposit_asset_address);
    let contract_address: Address = e.current_contract_address();
    token_client.transfer(&from, &contract_address, &(amount as i128));

    let started_at: u64 = e.ledger().timestamp();
    let deposit_id: u64 = consume_next_deposit_id(e);

    let mut record: DepositRecord = deposit(&e, &from, None, false).unwrap_or(DepositRecord {
        owner: from.clone(),
        amount: 0,
    });

    record.amount += amount;

    deposit(&e, &from, Some(record), false);

    add_total_principal(e, amount);
    bump_instance(e);

    DepositEvent {
        deposit_id,
        owner: from,
        amount,
        started_at,
    }
    .publish(&e);

    deposit_id
}
