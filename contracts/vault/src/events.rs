use soroban_sdk::{contractevent, Address};

#[contractevent(topics = ["ADMIN", "update"], data_format = "single-value")]
pub struct ContractAdminEvent {
    pub address: Address,
}

#[contractevent(topics = ["STATUS", "update"], data_format = "single-value")]
pub struct ContractStatusEvent {
    pub paused: bool,
}

#[contractevent(topics = ["DEPOSIT", "create"])]
pub struct DepositEvent {
    pub deposit_id: u64,
    pub owner: Address,
    pub amount: u128,
    pub started_at: u64,
}

#[contractevent(topics = ["DEPOSIT", "withdraw"])]
pub struct WithdrawDepositEvent {
    pub deposit_id: u64,
    pub owner: Address,
    pub amount: u128,
}

#[contractevent(topics = ["WITHDRAW", "total"])]
pub struct WithdrawEvent {
    pub amount: u128,
    pub owner: Address,
}
