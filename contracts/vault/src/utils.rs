use crate::errors::ContractErrors;
use crate::storage::core::deposit_asset;
use soroban_sdk::{token, Address, Env};

pub fn withdraw_deposit_asset(e: &Env, to: &Address, amount: &u128) -> Result<(), ContractErrors> {
    let asset: Address = deposit_asset(&e, None).unwrap();

    let withdraw_result = token::TokenClient::new(&e, &asset).try_transfer(&e.current_contract_address(), to, &(amount.clone() as i128));

    if withdraw_result.is_err() {
        Err(ContractErrors::WithdrawDepositAssetFailed)
    } else {
        Ok(())
    }
}
