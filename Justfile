default: check

check: format lint clean build

format:
    cargo fmt --all

lint:
    cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged

build:
    cargo build --all-features

clean:
    cargo clean

