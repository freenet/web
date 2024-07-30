#!/usr/bin/fish

# Note: should use --release in production
cd rust/gkwasm
wasm-pack build --target web --out-dir ../../hugo-site/static/wasm
cd ../..

# Rename the generated wasm file to match the expected name in our JavaScript
mv hugo-site/static/wasm/gkwasm_bg.wasm hugo-site/static/wasm/gkwasm.wasm
