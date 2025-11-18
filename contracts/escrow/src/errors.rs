use soroban_sdk::contracterror;

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractErrors {
    AllowanceExpired = 201,
    AllowanceMaxedOut = 202,
    AllowancePaymentFailed = 203,
    RewardEpochNotFound = 301,
    RewardEpochOutOfOrder = 302,
    RewardEpochNotMatured = 303,
    RewardLeafAlreadyClaimed = 304,
    RewardInvalidProof = 305,
    RewardLeafIndexOutOfBounds = 306,
    RewardAmountTooLarge = 307,
    InsufficientEscrowBalance = 308,
}
