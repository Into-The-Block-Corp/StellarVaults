use soroban_sdk::{contractevent, Address, BytesN};

#[contractevent(topics = ["REWARD", "root"])]
pub struct RewardRootUpdatedEvent {
    pub epoch: u32,
    pub vault: Address,
    pub root: BytesN<32>,
    pub asset: Address,
    pub total_rewards: u128,
    pub leaf_count: u32,
    pub program_end_ts: u64,
}

#[contractevent(topics = ["REWARD", "claim"])]
pub struct RewardClaimedEvent {
    pub epoch: u32,
    pub vault: Address,
    pub deposit_id: u64,
    pub owner: Address,
    pub asset: Address,
    pub amount: u128,
}
