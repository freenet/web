#!/usr/bin/fish

# Note: should use --release in production
cd rust/gkwasm
wasm-pack build --target web --out-dir ../../hugo-site/static/wasm
cd ../..

# Rename the generated files to match the expected names in our JavaScript
mv hugo-site/static/wasm/gkwasm_bg.wasm hugo-site/static/wasm/gkwasm.wasm
mv hugo-site/static/wasm/gkwasm.js hugo-site/static/wasm/gkwasm.js
