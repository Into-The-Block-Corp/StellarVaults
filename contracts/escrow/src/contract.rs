use crate::errors::ContractErrors;
use crate::events::{ContractAdminEvent, RewardClaimedEvent, RewardRootUpdatedEvent};
use crate::rewards::{compute_leaf_hash, compute_root_from_proof};
use crate::storage::{
    add_committed_rewards, admin, bump_instance, bump_reward_epoch, get_committed_rewards,
    get_latest_epoch, get_reward_epoch, is_claimed, put_reward_epoch, reduce_committed_rewards,
    set_claimed, set_latest_epoch, RewardEpoch,
};
use core::convert::TryFrom;
use soroban_sdk::{contract, contractimpl, contracttype, panic_with_error, token, Address, BytesN, Env, Vec};

#[derive(Clone)]
#[contracttype]
pub struct ClaimParams {
    pub deposit_id: u64,
    pub amount: u128,
    pub leaf_index: u32,
    pub proof: Vec<BytesN<32>>,
}

pub trait EscrowContractTrait {
    /// arguments:
    /// * new_admin: The admin of the contract
    fn __constructor(e: Env, new_admin: Address);

    /// This method allows upgrading the contract to a different WASM
    fn upgrade(e: Env, hash: BytesN<32>);

    /// This method sets the admin of the escrow contract.
    fn update_admin(e: Env, new_admin: Address);

    /// This method can be called by the contract admin to withdraw funds from the contract
    ///
    /// arguments:
    /// * actions: A vector containing a tuple with the asset to withdraw and the amount to keep in the contract
    fn withdraw(e: Env, actions: Vec<(Address, u128)>);

    /// Stores a new rewards root for the given epoch and vault.
    fn set_rewards_root(
        e: Env,
        asset: Address,
        vault: Address,
        epoch: u32,
        root: BytesN<32>,
        total_rewards: u128,
        leaf_count: u32,
        program_end_ts: u64,
    ) -> Result<(), ContractErrors>;

    /// Claims matured rewards for a single deposit using its Merkle proof.
    fn claim(e: Env, from: Address, vault: Address, epoch: u32, claim: ClaimParams) -> Result<(), ContractErrors>;

    /// Reads the stored reward epoch metadata (root, totals, program end, asset) for a vault.
    fn get_reward_epoch(e: Env, vault: Address, epoch: u32) -> Option<RewardEpoch>;

    /// Recover funds from an expired epoch whose storage has been reclaimed.
    fn sweep_expired_epoch(e: Env, asset: Address, vault: Address, epoch: u32, amount: u128) -> Result<(), ContractErrors>;
}

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContractTrait for EscrowContract {
    fn __constructor(e: Env, new_admin: Address) {
        admin(&e, Some(new_admin));
        bump_instance(&e);
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

    fn withdraw(e: Env, actions: Vec<(Address, u128)>) {
        let admin: Address = admin(&e, None).unwrap();
        admin.require_auth();
        for (asset, amount) in actions {
            let client: token::TokenClient = token::TokenClient::new(&e, &asset);
            let balance: i128 = client.balance(&e.current_contract_address());
            let amount_i128 = i128::try_from(amount)
                .unwrap_or_else(|_| panic_with_error!(e, ContractErrors::WithdrawAmountTooLarge));
            let transfer_amount = balance - amount_i128;
            if transfer_amount <= 0 {
                continue;
            }
            client.transfer(&e.current_contract_address(), &admin, &transfer_amount);
        }
        bump_instance(&e);
    }

    fn set_rewards_root(
        e: Env,
        asset: Address,
        vault: Address,
        epoch: u32,
        root: BytesN<32>,
        total_rewards: u128,
        leaf_count: u32,
        program_end_ts: u64,
    ) -> Result<(), ContractErrors> {
        admin(&e, None).unwrap().require_auth();

        if e.ledger().timestamp() < program_end_ts {
            return Err(ContractErrors::RewardEpochNotMatured);
        }

        if let Some(latest) = get_latest_epoch(&e, &vault) {
            if epoch <= latest {
                return Err(ContractErrors::RewardEpochOutOfOrder);
            }
        }

        if get_reward_epoch(&e, &vault, epoch).is_some() {
            return Err(ContractErrors::RewardEpochOutOfOrder);
        }

        let amount_i128 = i128::try_from(total_rewards).map_err(|_| ContractErrors::RewardAmountTooLarge)?;
        let token_client = token::TokenClient::new(&e, &asset);
        let contract_balance = token_client.balance(&e.current_contract_address());

        let committed = get_committed_rewards(&e, &asset);
        let committed_i128 = i128::try_from(committed).map_err(|_| ContractErrors::RewardAmountTooLarge)?;
        if contract_balance < committed_i128 + amount_i128 {
            return Err(ContractErrors::InsufficientEscrowBalance);
        }

        add_committed_rewards(&e, &asset, total_rewards);

        let epoch_data = RewardEpoch {
            root: root.clone(),
            asset: asset.clone(),
            total_rewards,
            leaf_count,
            program_end_ts,
        };

        put_reward_epoch(&e, &vault, epoch, &epoch_data);
        set_latest_epoch(&e, &vault, epoch);
        bump_instance(&e);

        RewardRootUpdatedEvent {
            epoch,
            vault: vault.clone(),
            root,
            asset,
            total_rewards,
            leaf_count,
            program_end_ts,
        }
        .publish(&e);

        Ok(())
    }

    fn claim(e: Env, from: Address, vault: Address, epoch: u32, claim: ClaimParams) -> Result<(), ContractErrors> {
        from.require_auth();

        let epoch_data = get_reward_epoch(&e, &vault, epoch).ok_or(ContractErrors::RewardEpochNotFound)?;

        if e.ledger().timestamp() < epoch_data.program_end_ts {
            return Err(ContractErrors::RewardEpochNotMatured);
        }

        let token_client = token::TokenClient::new(&e, &epoch_data.asset);
        let contract_address = e.current_contract_address();
        let asset = epoch_data.asset.clone();

        let ClaimParams {
            deposit_id,
            amount,
            leaf_index,
            proof,
        } = claim;

        if amount == 0 {
            return Err(ContractErrors::RewardInvalidProof);
        }

        if leaf_index >= epoch_data.leaf_count {
            return Err(ContractErrors::RewardLeafIndexOutOfBounds);
        }

        if is_claimed(&e, &vault, epoch, deposit_id) {
            return Err(ContractErrors::RewardLeafAlreadyClaimed);
        }

        let leaf = compute_leaf_hash(&e, &vault, epoch, deposit_id, &from, amount);
        let computed_root = compute_root_from_proof(&e, &leaf, &proof, leaf_index);

        if computed_root != epoch_data.root {
            return Err(ContractErrors::RewardInvalidProof);
        }

        let amount_i128 = i128::try_from(amount).map_err(|_| ContractErrors::RewardAmountTooLarge)?;

        if token_client.balance(&contract_address) < amount_i128 {
            return Err(ContractErrors::InsufficientEscrowBalance);
        }

        token_client.transfer(&contract_address, &from, &amount_i128);
        reduce_committed_rewards(&e, &epoch_data.asset, amount);
        set_claimed(&e, &vault, epoch, deposit_id);

        RewardClaimedEvent {
            epoch,
            vault: vault.clone(),
            deposit_id,
            owner: from.clone(),
            asset: asset.clone(),
            amount,
        }
        .publish(&e);

        bump_reward_epoch(&e, &vault, epoch);
        bump_instance(&e);

        Ok(())
    }

    fn get_reward_epoch(e: Env, vault: Address, epoch: u32) -> Option<RewardEpoch> {
        get_reward_epoch(&e, &vault, epoch)
    }

    fn sweep_expired_epoch(e: Env, asset: Address, vault: Address, epoch: u32, amount: u128) -> Result<(), ContractErrors> {
        let admin_addr = admin(&e, None).unwrap();
        admin_addr.require_auth();

        // Only allow sweep if the epoch data has expired (no longer in storage)
        if get_reward_epoch(&e, &vault, epoch).is_some() {
            return Err(ContractErrors::RewardEpochNotExpired);
        }

        // Prove the epoch actually existed at some point: it must be at or
        // before the latest published epoch for this vault. Without this,
        // any fabricated or future (vault, epoch) pair would pass the
        // is_none() check above.
        let latest = get_latest_epoch(&e, &vault).ok_or(ContractErrors::RewardEpochNotFound)?;
        if epoch > latest {
            return Err(ContractErrors::RewardEpochNotFound);
        }

        let amount_i128 = i128::try_from(amount).map_err(|_| ContractErrors::RewardAmountTooLarge)?;
        let token_client = token::TokenClient::new(&e, &asset);
        let balance = token_client.balance(&e.current_contract_address());

        if balance < amount_i128 {
            return Err(ContractErrors::InsufficientEscrowBalance);
        }

        if get_committed_rewards(&e, &asset) < amount {
            return Err(ContractErrors::SweepAmountExceedsCommittedRewards);
        }

        token_client.transfer(&e.current_contract_address(), &admin_addr, &amount_i128);
        reduce_committed_rewards(&e, &asset, amount);
        bump_instance(&e);

        Ok(())
    }
}
