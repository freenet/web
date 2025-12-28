# Ghost Key Development Guide

## Overview
Ghost Keys provide anonymous, verifiable identities backed by Freenet donations. This crate implements the cryptographic primitives shared across the website, CLI, and API.

## Core Flow
1. User donates via Stripe and generates an Ed25519 key pair locally.
2. Public key is blinded (RSA blind signatures, RFC 9474) before being sent to the server.
3. Server signs the blinded key with its delegate RSA key and returns the signature.
4. Browser unblinds the signature and combines it with metadata to form the Ghost Key certificate.
5. Trust chain: Freenet master key → delegate certificate → Ghost Key certificate.

## Key Modules
- `armorable.rs`: Base64 serialization trait for cryptographic objects.
- `delegate_certificate.rs`: Delegate key management and verification.
- `ghost_key_certificate.rs`: Certificate creation and validation logic.
- `util.rs`: Blind signature helpers and cryptographic utilities.
- `errors.rs`: Error types used across the library.

## Building & Testing
```bash
cd rust/gklib
cargo build
cargo test
```

Workspace commands:
```bash
cargo make build-wasm-dev       # Build browser WASM module (uses this crate)
cargo make build-wasm-release
cargo make integration-test
./test_cli_commands.sh          # CLI integration tests exercising certificates
```

## Browser Integration
- `rust/gkwasm/` exposes Ghost Key functionality to the browser (wasm-bindgen).
- WebAssembly output is copied to `hugo-site/static/wasm/`.
- `hugo-site/content/ghostkey/create/index.html` demonstrates the end-to-end flow.

## Security Notes
- Blind signatures ensure server never sees the real public key.
- Certificates verify back to the master key (`FREENET_MASTER_VERIFYING_KEY_BASE64`).
- Keep an eye on revocation, delegation chains, and threshold signatures when extending.

## Useful CLI Commands
```bash
ghostkey generate-master-key
ghostkey generate-delegate --master-signing-key master.pem --info "test"
ghostkey generate-ghost-key --delegate-cert delegate.pem --delegate-key delegate-key.pem
ghostkey verify-ghost-key --ghost-certificate ghost.pem
ghostkey sign-message --ghost-certificate ghost.pem --ghost-signing-key key.pem --message "test"
```
