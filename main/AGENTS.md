# Claude Guide for Freenet.org Website

## Project Overview
This repository contains the Freenet.org website, combining a Hugo static site with Rust-based WebAssembly components for Ghost Key functionality.

## Build Commands

### Development
```bash
cargo make dev                    # Build all components (wasm + hugo) in dev mode
cargo make build-wasm-dev         # WebAssembly only
cargo make build-hugo-dev         # Hugo site only
cd hugo-site && hugo server -D    # Serve site locally
```

Use worktrees to avoid conflicts with other agents, create worktrees in ~/code/freenet/web

### Production
```bash
cargo make release                # Full production build
cargo make build-site             # Prepare deployment artifacts
```

### Testing
```bash
cargo make integration-test
./test_cli_commands.sh
cargo test --all
```

### Linting & Formatting
```bash
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --all-targets --all-features
prettier --write "hugo-site/**/*.{html,css,js,md}"
```

## Architecture Overview
- `rust/gklib/`: Ghost Key cryptographic primitives (library crate)
- `rust/gkwasm/`: WebAssembly bindings exposed to the browser via wasm-bindgen
- `rust/api/`: Axum-based REST API for Ghost Key operations (Stripe integration)
- `rust/cli/`: CLI for Ghost Key management tasks
- `hugo-site/`: Hugo source, theme, and static assets (including WASM output)

### WebAssembly workflow
1. Edit code in `rust/gkwasm/src/`
2. Run `cargo make build-wasm-dev` (or `build-wasm-release`)
3. Generated artifacts land in `hugo-site/static/wasm/`
4. Test via `hugo server` at `http://localhost:1313/ghostkey/create/`

## Environment Variables (API / Stripe)
- `STRIPE_TEST_SECRET_KEY`
- `STRIPE_LIVE_SECRET_KEY`
- `STRIPE_TEST_WEBHOOK_SECRET`
- `STRIPE_LIVE_WEBHOOK_SECRET`

## Deployment
GitHub Actions builds and deploys the site on pushes to `main`:
1. Build Rust components (with WASM caching)
2. Compile WebAssembly modules
3. Build Hugo site
4. Publish to GitHub Pages

## Development Tips
- Use `cargo make` tasks defined in `Makefile.toml` for consistent commands.
- WebAssembly builds are cached in CI; keep build scripts in sync.
- Run CLI tests (`test_cli_commands.sh`) when touching Ghost Key commands.
- Prettier config enforces a 100-character line width and automatic markdown wrapping.

## Related Guides
- Ghost Key library specifics in `rust/gklib/AGENTS.md`
- See `README.md` for onboarding and dependency setup.
