## Project Structure

This repository uses the recommended structure for a Soroban project:

```text
.
├── contracts
│   └── hello_world
│       ├── src
│       │   ├── lib.rs
│       │   └── test.rs
│       └── Cargo.toml
├── Cargo.toml
└── README.md
```

# Vault Contract

The Vault contract is a simple contract that does nothing but hold assets deposited by the users, the contract has the
next trait:

```rust
pub trait VaultContractTrait {
    /// arguments:
    /// * new_admin: The admin of the contract
    /// * new_deposit_asset: The asset users will deposit into the vault
    fn __constructor(e: Env, new_admin: Address, new_deposit_asset: Address);

    /// This method allows upgrading the contract to a different WASM
    fn upgrade(e: Env, hash: BytesN<32>);

    /// This method sets the admin of the contract, if is called for the first time it won't require authorization.
    /// The new admin doesn't need to sign the transaction, only the current admin.
    fn update_admin(e: Env, new_admin: Address);

    /// This pauses/unpauses the deposits.
    fn set_status(e: Env, new_status: bool);

    /// Deposit assets into the vault on behalf of `from`, returning the new deposit identifier.
    fn deposit(e: Env, from: Address, amount: u128) -> u64;

    /// Withdraws the funds from the user's deposit
    fn withdraw(e: Env, from: Address, amount: u128) -> Result<(), ContractErrors>;
}
```

The contract can be paused by the admin after the process has ended, this will prevent new deposits but will still allow
users to withdraw their funds

# Escrow Contract

The Escrow contract is the contract that takes care of holding the balance that will be claimed by users according to
the Merkle trees generated off-chain, the contract has the next trait:

```rust
pub trait EscrowContractTrait {
    /// arguments:
    /// * new_admin: The admin of the contract
    fn __constructor(e: Env, new_admin: Address);

    /// This method allows upgrading the contract to a different WASM
    fn upgrade(e: Env, hash: BytesN<32>);

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
}
```

# How to build the contracts

To build both contracts you can:

```shell
make build
```

This will automatically optimize the contracts builds after they were created.

# How to test the contract

To test the contract you can:

```shell
make test
```