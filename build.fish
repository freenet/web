#!/usr/bin/fish

# Note: should use --release in production
cargo build --target wasm32-unknown-unknown --manifest-path rust/gkwasm/Cargo.toml
cp rust/target/wasm32-unknown-unknown/debug/gkwasm.wasm hugo-site/static/wasm/
