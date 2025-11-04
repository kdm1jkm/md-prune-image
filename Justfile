default: check

check: format lint clean build

format:
    cargo fmt --all

lint:
    cargo clippy --fix -- -D warnings

build:
    cargo build --all-features

clean:
    cargo clean

