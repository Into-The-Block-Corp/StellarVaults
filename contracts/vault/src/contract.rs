use crate::deposit::handle_deposit;
use crate::errors::ContractErrors;
use crate::events::{ContractAdminEvent, ContractStatusEvent, WithdrawEvent};
use crate::storage::core::{admin, bump_instance, deposit_asset, escrow, paused};
use crate::storage::deposit_state::{deposit, reduce_total_principal, DepositRecord};
use crate::utils::withdraw_deposit_asset;
use soroban_sdk::{contract, contractimpl, panic_with_error, Address, BytesN, Env, String};

pub trait VaultContractTrait {
    /// arguments:
    /// * new_admin: The admin of the contract
    /// * new_deposit_asset: The asset users will deposit into the vault
    fn __constructor(e: Env, new_admin: Address, new_deposit_asset: Address, new_escrow: Address);

    // ---------------------------------------
    // ------------ Admin methods ------------
    // ---------------------------------------

    /// This method allows upgrading the contract to a different WASM
    fn upgrade(e: Env, hash: BytesN<32>);

    /// This method sets the admin of the contract, if is called for the first time it won't require authorization.
    /// The new admin doesn't need to sign the transaction, only the current admin.
    fn update_admin(e: Env, new_admin: Address);

    /// This pauses/unpauses the deposits and withdrawals.
    fn set_status(e: Env, new_status: bool);

    /// Deposits assets into the vault on behalf of `from`, returning the new deposit identifier.
    /// * referral_id: Optional referral identifier emitted with the deposit event.
    fn deposit(e: Env, from: Address, amount: u128, referral_id: Option<String>) -> u64;

    /// Withdraws the funds from the user's deposits
    ///
    /// # Returns
    /// A vector with tuples with the structure:
    /// * Deposit ID
    /// * Deposit asset withdrew
    fn withdraw(e: Env, from: Address, amount: u128) -> Result<(), ContractErrors>;
}

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContractTrait for VaultContract {
    fn __constructor(e: Env, new_admin: Address, new_deposit_asset: Address, new_escrow: Address) {
        admin(&e, Some(new_admin));
        deposit_asset(&e, Some(new_deposit_asset));
        escrow(&e, Some(new_escrow));
        paused(&e, Some(false));
    }

    fn upgrade(e: Env, hash: BytesN<32>) {
        admin(&e, None).unwrap().require_auth();
        e.deployer().update_current_contract_wasm(hash);
        bump_instance(&e);
    }

    fn update_admin(e: Env, new_admin: Address) {
        if let Some(v) = admin(&e, None) {
            v.require_auth();
        }
        admin(&e, Some(new_admin.clone()));
        bump_instance(&e);
        ContractAdminEvent { address: new_admin }.publish(&e);
    }

    fn set_status(e: Env, new_status: bool) {
        admin(&e, None).unwrap().require_auth();
        paused(&e, Some(new_status.clone()));
        bump_instance(&e);
        ContractStatusEvent { paused: new_status }.publish(&e);
    }

    fn deposit(e: Env, from: Address, amount: u128, referral_id: Option<String>) -> u64 {
        handle_deposit(&e, from, amount, referral_id)
    }

    fn withdraw(e: Env, from: Address, amount: u128) -> Result<(), ContractErrors> {
        if paused(&e, None).unwrap_or(false) {
            panic_with_error!(e, ContractErrors::VaultPaused);
        }

        from.require_auth();

        let mut saved_deposit: DepositRecord = deposit(&e, &from, None, false).unwrap_or(DepositRecord {
            owner: from.clone(),
            amount: 0,
        });

        if saved_deposit.amount < amount {
            return Err(ContractErrors::NotEnoughDeposit);
        }

        saved_deposit.amount -= amount;

        if saved_deposit.amount == 0 {
            deposit(&e, &from, None, true);
        } else {
            deposit(&e, &from, Some(saved_deposit), false);
        }

        withdraw_deposit_asset(&e, &from, &amount)?;
        reduce_total_principal(&e, &amount);
        bump_instance(&e);

        WithdrawEvent {
            owner: from.clone(),
            amount,
        }
        .publish(&e);

        Ok(())
    }
}
