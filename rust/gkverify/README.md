# ghostkey-verify

Verify-only Ghost Key certificate chains, for linking into a Freenet contract.

A Ghost Key certificate chains Freenet's Ed25519 master key to a notary's RSA
key (the notary certificate), and the notary's blind RSA signature to the
donor's Ed25519 ghost key. This crate checks that chain, checks Ed25519
signatures by the ghost key over raw payload bytes (such as the `ScopedPayload`
the ghostkey vault returns), and parses the notary's `info` (donation amount
and the month the notary key was created).

It does nothing else: no key generation, no blinding, no randomness. Neither
`rand` nor `getrandom` is in its dependency tree, so it builds for
`wasm32-unknown-unknown` without a custom `getrandom` backend, and a module
linking it imports nothing from the host on its account.

```rust
use ghostkey_verify::{production_master, verify_payload_signature, GhostkeyCertificate};

fn check(cert_pem: &str, scoped_payload: &[u8], signature: &[u8]) -> Result<u64, ghostkey_verify::Error> {
    let cert = GhostkeyCertificate::from_armored(cert_pem)?;
    // The cheap Ed25519 check first, so a forged entry never costs an RSA check.
    verify_payload_signature(cert.claimed_ghost_key(), scoped_payload, signature)?;
    let verified = cert.verify(&production_master())?;
    let info = verified.notary_info()?;
    // Every notary key so far is a donation notary; don't take a future kind for one.
    if info.action != "freenet-donation" {
        return Err(ghostkey_verify::Error::Info);
    }
    Ok(info.amount)
}
```

## Agreement with ghostkey_lib

Parsing and the chain check reproduce
[`ghostkey_lib`](https://crates.io/crates/ghostkey_lib) 0.2 exactly, including
the bytes the master key signs, and differential tests against it cover valid
chains, tampered info, wrong master keys, wrong RSA signatures, swapped keys,
re-encoded and mutated certificates, armour variants, and recorded certificates
(including one under the production master key). The differences are
deliberate, and each refuses something no genuine certificate contains:

- input over `MAX_CERTIFICATE_BYTES` (4 KiB), as given or in canonical form, is
  refused;
- a weak master key, and a certificate for a weak (small-order) ghost key, are
  refused;
- the notary RSA key must have an odd 2048 to 4096 bit modulus and exponent 3
  or 65537;
- an RSA signature must be below the modulus.

Payload signatures use Ed25519 `verify_strict`.

## Dependencies

- `rsa` is the 0.10 release candidate, pinned exactly (`=0.10.0-rc.19`): the
  stable 0.9 line pulls in `rand` through `num-bigint-dig`'s `prime` feature,
  and release candidates change API between themselves. Cargo allows one
  `rsa` 0.10 per build, so a crate that depends on this one cannot also use a
  different `rsa` 0.10 release. This crate moves to `rsa` 0.10.0 when it is
  released ([freenet/web#213](https://github.com/freenet/web/issues/213)).
- Do not enable `rsa`'s `getrandom` feature in a contract: it brings in
  `getrandom` through feature unification, which this crate cannot prevent.
- `ed25519-dalek` is 2.x, the version ghostkey-common, freenet-bitcoin and
  freenet/mail use, and `VerifyingKey` is re-exported from it. Moving to 3.x
  would be a breaking release.

## License

MIT OR Apache-2.0
