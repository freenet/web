# Ghostkey Library (ghostkey_lib)

A Rust library for creating and managing [ghost keys and certificates](https://freenet.org/ghostkey/)
in the [Freenet](https://freenet.org/) ecosystem.

## Features

- Creation and verification of delegate certificates
- Creation and verification of ghost key certificates
- RSA and Ed25519 cryptographic operations
- Serialization and deserialization of certificates

## Main Components

- `DelegateCertificateV1`: Represents a delegate certificate signed by a master key
- `GhostkeyCertificateV1`: Represents a ghost key certificate signed by a delegate key
- `Armorable`: Trait for serializing and deserializing objects to/from bytes and armored strings

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
ghostkey_lib = "0.1.2" # Replace with the latest version
```

Example usage:

```rust
use ghostkey_lib::{DelegateCertificateV1, GhostkeyCertificateV1, util::create_keypair};
use rand_core::OsRng;

// Create a master key pair
let (master_signing_key, master_verifying_key) = create_keypair(&mut OsRng).unwrap();

// Create a delegate certificate
let info = "Test Delegate".to_string();
let (delegate_certificate, delegate_signing_key) =
    DelegateCertificateV1::new(&master_signing_key, &info).unwrap();

// Create a ghost key certificate
let (ghost_key_certificate, ghost_key_signing_key) =
    GhostkeyCertificateV1::new(&delegate_certificate, &delegate_signing_key);

// Verify the ghost key certificate
let verified_info = ghost_key_certificate
    .verify(&Some(master_verifying_key))
    .unwrap();
assert_eq!(verified_info, info);
```

## License

`ghostkey_lib` is released under the [GNU Lesser General Public License v3.0](https://www.gnu.org/licenses/lgpl-3.0.html).
