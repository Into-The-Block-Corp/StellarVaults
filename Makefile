default: build

test: build
	cargo test

build:
	stellar contract build --package vault
	stellar contract build --package escrow
	stellar contract optimize --wasm target/wasm32v1-none/release/vault.wasm
	stellar contract optimize --wasm target/wasm32v1-none/release/escrow.wasm

fmt:
	cargo fmt --all

clean:
	cargo clean
