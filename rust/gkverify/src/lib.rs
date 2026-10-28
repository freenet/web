//! Verify-only Ghost Key certificate chains, for Freenet contracts.
//!
//! A Ghost Key certificate chains three keys:
//!
//! 1. Freenet's Ed25519 **master** key signs a **notary certificate**, which
//!    names the notary's RSA public key and an `info` string (donation amount
//!    and the month the notary key was created, see [`NotaryInfo`]).
//! 2. The notary blind-signs (RSA-PSS, SHA-384, RFC 9474) the donor's
//!    Ed25519 **ghost key**. The finalized, unblinded signature is in the
//!    certificate.
//! 3. The ghost key signs whatever its owner wants, in practice a
//!    `ScopedPayload` produced by the ghostkey vault. This crate checks that
//!    signature over the raw payload bytes and leaves decoding them to the
//!    caller, so it does not depend on `freenet-stdlib`.
//!
//! This crate is the verification half of
//! [`ghostkey_lib`](https://crates.io/crates/ghostkey_lib) and nothing else:
//! no key generation, no blinding, no randomness. Its dependency tree contains
//! neither `rand` nor `getrandom`, so it builds for `wasm32-unknown-unknown`
//! and the module it links into imports nothing from the host on its account.
//!
//! # Agreement with `ghostkey_lib`
//!
//! Parsing and the chain check reproduce `ghostkey_lib` 0.2's
//! `GhostkeyCertificateV1::from_armored_string`, `from_bytes` and `verify`,
//! including the exact bytes the master key signs (the CBOR re-encoding of the
//! notary payload), and a differential test suite holds the two together. The
//! intentional differences all refuse something `ghostkey_lib` accepts and no
//! genuine certificate contains:
//!
//! * Certificates over [`MAX_CERTIFICATE_BYTES`] are refused unparsed, and
//!   so is a weak master key.
//! * [`GhostkeyCertificate::verify`] refuses a certificate that certifies a
//!   weak (small-order) ghost key, which anyone can sign for. The notary
//!   blind-signs whatever key it is given, so such certificates can exist.
//!   `freenet-bitcoin`'s inbox contract makes the same check after
//!   `ghostkey_lib`'s.
//! * The notary's RSA key must have an odd modulus of 2048 to 4096 bits and
//!   public exponent 3 or 65537: the range `blind-rsa-signatures`' own
//!   parameter check allows, measured in bits rather than whole bytes. Every
//!   notary key issued is exactly 2048 bits with e = 65537. This bounds the
//!   cost of a check.
//! * An RSA signature must be smaller than the modulus (RFC 8017, 5.2.2).
//!   `ghostkey_lib` reduces it instead, so it also accepts the signature plus
//!   a multiple of the modulus when that still fits the modulus length.
//!
//! The payload signature check, [`verify_payload_signature`], is Ed25519
//! `verify_strict`, as `freenet-bitcoin`'s inbox contract uses for
//! `ScopedPayload` signatures.
//!
//! # Cost
//!
//! A chain check is one Ed25519 verification and one RSA-2048 public-key
//! operation. Callers that face untrusted input should bound its size before
//! parsing (a genuine armoured certificate is about 1.6 KB) and check the
//! cheap Ed25519 payload signature, against [`GhostkeyCertificate::claimed_ghost_key`],
//! before the chain.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod armor;
mod error;
mod info;
mod wire;

use ed25519_dalek::{Signature, Verifier};
use rsa::signature::Verifier as _;

pub use ed25519_dalek::VerifyingKey;
pub use error::Error;
pub use info::{NotaryInfo, YearMonth};

/// Compiles the README's example.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

/// Freenet's production Ghost Key master verifying key.
///
/// The same key as `ghostkey_lib::FREENET_MASTER_VERIFYING_KEY_BASE64`, which
/// is the base64 of its CBOR encoding; a test holds the two together.
pub const FREENET_MASTER_VERIFYING_KEY: [u8; 32] = [
    0x62, 0x9d, 0x97, 0xa2, 0xdd, 0x89, 0x22, 0xf5, 0xec, 0xf1, 0x28, 0xf3, 0x4b, 0xa0, 0x26, 0xab,
    0xfe, 0x6e, 0x85, 0xdd, 0x0f, 0x27, 0x9d, 0x2a, 0xd6, 0x0d, 0xa4, 0x52, 0xc4, 0xb2, 0xfa, 0x73,
];

/// [`FREENET_MASTER_VERIFYING_KEY`] as a verifying key.
pub fn production_master() -> VerifyingKey {
    VerifyingKey::from_bytes(&FREENET_MASTER_VERIFYING_KEY)
        .expect("the production master key is a valid point")
}

/// Largest certificate accepted, in bytes, by [`GhostkeyCertificate::from_armored`]
/// (of armoured text) and [`GhostkeyCertificate::from_cbor`] (of CBOR).
///
/// A genuine certificate is about 1.6 KB armoured; one under a 4096-bit
/// notary key, the largest accepted, is under 3 KB. The same bound as
/// freenet-bitcoin's inbox contract, which checks it before parsing.
pub const MAX_CERTIFICATE_BYTES: usize = 4 * 1024;

/// Smallest notary RSA modulus accepted, in bits.
pub const MIN_NOTARY_KEY_BITS: usize = 2048;
/// Largest notary RSA modulus accepted, in bits.
pub const MAX_NOTARY_KEY_BITS: usize = 4096;

/// A parsed, not yet verified, Ghost Key certificate.
///
/// Nothing read from it means anything until [`GhostkeyCertificate::verify`]
/// succeeds.
#[derive(Clone)]
pub struct GhostkeyCertificate {
    wire: wire::CertificateWire,
}

/// What a verified certificate certifies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedGhostkey {
    /// The ghost key the chain certifies.
    pub ghost_key: VerifyingKey,
    /// The notary certificate's `info` string, exactly as the master key
    /// signed it. [`NotaryInfo::parse`] reads the current format.
    pub info: String,
}

impl GhostkeyCertificate {
    /// Parse a PEM-armoured certificate (`-----BEGIN GHOSTKEY_CERTIFICATE_V1-----`),
    /// accepting exactly what `ghostkey_lib`'s `from_armored_string` accepts.
    ///
    /// That includes any text around the armour, so one certificate has many
    /// spellings. A caller that keys anything by certificate text should
    /// store [`GhostkeyCertificate::to_armored`] instead of its input.
    ///
    /// Refuses text longer than [`MAX_CERTIFICATE_BYTES`] before parsing.
    pub fn from_armored(armored: &str) -> Result<Self, Error> {
        if armored.len() > MAX_CERTIFICATE_BYTES {
            return Err(Error::TooLarge);
        }
        armor::decode(armored)
    }

    /// Parse a certificate's CBOR encoding, as `ghostkey_lib`'s `from_bytes`
    /// does. Like it, this ignores bytes after the first CBOR item.
    ///
    /// Refuses more than [`MAX_CERTIFICATE_BYTES`] before parsing.
    pub fn from_cbor(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_CERTIFICATE_BYTES {
            return Err(Error::TooLarge);
        }
        let wire = ciborium::de::from_reader(bytes).map_err(|_| Error::Malformed)?;
        Ok(Self { wire })
    }

    /// The canonical CBOR encoding: what `ghostkey_lib` would write for this
    /// certificate.
    pub fn to_cbor(&self) -> Vec<u8> {
        wire::to_cbor(&self.wire)
    }

    /// The canonical armoured form, byte for byte what `ghostkey_lib`'s
    /// `to_armored_string` writes.
    pub fn to_armored(&self) -> String {
        armor::encode(&self.to_cbor())
    }

    /// The ghost key the certificate claims to certify, unverified.
    ///
    /// For checking an Ed25519 signature before paying for the chain's RSA
    /// check. It certifies nothing until [`GhostkeyCertificate::verify`]
    /// returns it.
    pub fn claimed_ghost_key(&self) -> &VerifyingKey {
        &self.wire.verifying_key
    }

    /// Verify the chain from `master` to the ghost key, returning the ghost
    /// key and the notary's `info`.
    ///
    /// `master` is trusted as given: pass [`production_master`] or a key
    /// from the contract's own parameters, never one read from the input.
    /// A weak master key, under which anyone could sign notary
    /// certificates, is refused.
    ///
    /// Refuses a certificate for a weak ghost key (see the crate docs). That
    /// check comes first, so such a certificate never costs an RSA operation.
    pub fn verify(&self, master: &VerifyingKey) -> Result<VerifiedGhostkey, Error> {
        if master.is_weak() {
            return Err(Error::WeakMasterKey);
        }
        if self.wire.verifying_key.is_weak() {
            return Err(Error::WeakGhostKey);
        }
        let info = self.verify_chain(master)?;
        Ok(VerifiedGhostkey {
            ghost_key: self.wire.verifying_key,
            info,
        })
    }

    /// `ghostkey_lib`'s chain check, with this crate's RSA parameter limits.
    fn verify_chain(&self, master: &VerifyingKey) -> Result<String, Error> {
        let notary = &self.wire.notary;

        // 1. The master key signed the notary payload. `ghostkey_lib` signs
        //    and verifies the payload's CBOR re-encoding, not the bytes it was
        //    read from, and so does this.
        let payload_bytes = wire::to_cbor(&notary.payload);
        master
            .verify(&payload_bytes, &notary.signature)
            .map_err(|_| Error::NotarySignature)?;

        // 2. The notary signed the ghost key: RSA-PSS with SHA-384 over the
        //    ghost key's CBOR encoding, any salt length, no message
        //    randomizer, as `blind_rsa_signatures::PublicKey::verify` with
        //    default options.
        let key = notary.payload.notary_key.to_rsa()?;
        let sig = &self.wire.signature.0;
        if sig.len() != rsa::traits::PublicKeyParts::size(&key) {
            return Err(Error::GhostKeySignature);
        }
        let sig =
            rsa::pss::Signature::try_from(sig.as_slice()).map_err(|_| Error::GhostKeySignature)?;
        let message = wire::to_cbor(&self.wire.verifying_key);
        rsa::pss::VerifyingKey::<sha2::Sha384>::new_with_auto_salt_len(key)
            .verify(&message, &sig)
            .map_err(|_| Error::GhostKeySignature)?;

        Ok(notary.payload.info.clone())
    }
}

/// Check an Ed25519 signature by `ghost_key` over `payload`, for example the
/// CBOR `ScopedPayload` bytes exactly as the ghostkey vault returned them.
///
/// Strict verification (`verify_strict`): refuses weak keys, small-order
/// commitments and non-canonical signatures.
pub fn verify_payload_signature(
    ghost_key: &VerifyingKey,
    payload: &[u8],
    signature: &[u8],
) -> Result<(), Error> {
    let signature: [u8; 64] = signature
        .try_into()
        .map_err(|_| Error::PayloadSignatureLength)?;
    ghost_key
        .verify_strict(payload, &Signature::from_bytes(&signature))
        .map_err(|_| Error::PayloadSignature)
}

impl VerifiedGhostkey {
    /// [`verify_payload_signature`] under this ghost key.
    pub fn verify_payload_signature(&self, payload: &[u8], signature: &[u8]) -> Result<(), Error> {
        verify_payload_signature(&self.ghost_key, payload, signature)
    }

    /// The notary `info`, parsed. See [`NotaryInfo::parse`].
    pub fn notary_info(&self) -> Result<NotaryInfo, Error> {
        NotaryInfo::parse(&self.info)
    }
}
