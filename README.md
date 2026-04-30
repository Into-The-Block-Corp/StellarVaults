# Sentora Smart Contracts

Soroban workspace that contains the on-chain components of the Sentora vault system.

## Contracts

| Package | Description |
| --- | --- |
| `vault` | Manages user deposits, accrues rewards, and exposes deposit/withdraw events for the indexer. |
| `escrow` | Holds the rewards token supply and releases allocations when a Merkle proof is verified. |

Both contracts are built with the Soroban SDK and rely on deterministic addresses so they can be referenced by the backend and frontend services.

## Requirements

- Rust toolchain with `wasm32v1-none` target (`rustup target add wasm32v1-none`).
- Soroban CLI (`soroban`) for building and deploying WASM artifacts.
- Stellar CLI (`stellar`) if you plan to interact with the local Quickstart network.

## Building

```bash
cd smart-contracts
make build         # builds + optimizes vault and escrow
```

Artifacts are written to `target/wasm32v1-none/release/*.wasm` and optimized with `stellar contract optimize`. Use `make clean` to remove build output.

## Testing

```bash
make test          # runs `cargo test` for the workspace
make fmt           # optional: rustfmt
```

## Deployment

The recommended path is to run `./scripts/deploy_contracts.sh` from the repository root. That script:

1. Builds both contracts using the configuration in this directory.
2. Ensures Soroban identities and network aliases exist.
3. Deploys (and optionally funds) the contracts, writing their IDs to `scripts/contract-ids.env`.

You can also deploy manually with Soroban CLI:

```bash
soroban contract deploy \
  --wasm target/wasm32v1-none/release/vault.wasm \
  --source dev --network local --salt <optional>
```

See the script header comments for supported environment overrides (admin address, deterministic salts, token IDs, etc.).
