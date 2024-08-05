#!/usr/bin/fish

# Note: should use --release in production
cd rust/gkwasm
wasm-pack build --target web --out-dir ../../hugo-site/static/wasm
cd ../..
