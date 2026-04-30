use crate::errors::ContractErrors;
use crate::events::DepositEvent;
use crate::storage::core::{bump_instance, deposit_asset, paused};
use crate::storage::deposit_state::{add_total_principal, consume_next_deposit_id, deposit, DepositRecord};
use soroban_sdk::token::TokenClient;
use core::convert::TryFrom;
use soroban_sdk::{panic_with_error, token, Address, Env, String};

pub fn handle_deposit(e: &Env, from: Address, amount: u128, referral_id: Option<String>) -> u64 {
    if paused(e, None).unwrap_or(false) {
        panic_with_error!(e, ContractErrors::VaultPaused);
    }

    if amount == 0 {
        panic_with_error!(e, ContractErrors::ZeroAmountDeposit);
    }

    let deposit_asset_address: Address = deposit_asset(e, None).unwrap();

    from.require_auth();

    let token_client: TokenClient = token::Client::new(e, &deposit_asset_address);
    let contract_address: Address = e.current_contract_address();
    let amount_i128 = i128::try_from(amount).unwrap_or_else(|_| panic_with_error!(e, ContractErrors::AmountOverflow));
    token_client.transfer(&from, &contract_address, &amount_i128);

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
        referral_id,
    }
    .publish(&e);

    deposit_id
}
