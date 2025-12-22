default: build

test: build
	cargo test

build:
	stellar contract build --optimize

fmt:
	cargo fmt --all

clean:
	cargo clean
