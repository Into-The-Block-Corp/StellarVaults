use soroban_sdk::contracterror;

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractErrors {
    VaultPaused = 100,
    NotEnoughDeposit = 101,
    WithdrawDepositAssetFailed = 102,
    RewardAmountTooLarge = 103,
}
