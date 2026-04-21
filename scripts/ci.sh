#!/bin/sh
set -eu

cargo test --workspace
cargo +nightly fmt --all --check
cargo +nightly clippy --workspace --all-targets
cargo build --release
