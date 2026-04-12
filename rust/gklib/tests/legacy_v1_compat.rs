//! Regression tests against pre-0.2.0 ghost-key certificate bytes.
//!
//! Fixtures in `tests/fixtures/legacy_v1/` were generated from an unmodified
//! checkout of main before the delegate → notary rename landed (issue
//! freenet/web#24). These tests pin the wire-format compatibility decisions:
//!
//! - `#[serde(rename = "delegate_verifying_key")]` on `NotaryPayload`
//! - `#[serde(rename = "delegate")]` on `GhostkeyCertificateV1`
//! - `legacy_armor_aliases()` in `armorable.rs` accepting old PEM headers
//!
//! If any of these are removed or changed, these tests MUST fail — they are
//! the line of defense against silently breaking existing ghost keys in the
//! wild.

use std::path::PathBuf;

use ed25519_dalek::{Verifier, VerifyingKey};
use ghostkey_lib::armorable::Armorable;
use ghostkey_lib::ghost_key_certificate::GhostkeyCertificateV1;
use ghostkey_lib::notary_certificate::NotaryCertificateV1;
use ghostkey_lib::signed_message::SignedMessage;

fn fixture_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push("legacy_v1");
    p.push(name);
    p
}

fn load_master_verifying_key() -> VerifyingKey {
    VerifyingKey::from_file(&fixture_path("master_verifying_key.pem"))
        .expect("legacy master verifying key must still parse")
}

#[test]
fn legacy_notary_cert_parses_with_new_types() {
    // Fixture file header is `BEGIN DELEGATE_CERTIFICATE_V1`. New type name
    // is `NotaryCertificateV1`. The armor alias must bridge them.
    let cert = NotaryCertificateV1::from_file(&fixture_path("delegate_certificate.pem"))
        .expect("legacy delegate_certificate.pem must parse as NotaryCertificateV1");

    let master = load_master_verifying_key();
    let info = cert
        .verify(&Some(master))
        .expect("legacy notary cert must verify against fixture master key");
    assert_eq!(info, "donation_amount:20");
}

#[test]
fn legacy_ghost_key_cert_parses_and_verifies_chain() {
    let cert = GhostkeyCertificateV1::from_file(&fixture_path("ghost_key_certificate.pem"))
        .expect("legacy ghost_key_certificate.pem must parse");

    let master = load_master_verifying_key();
    let info = cert
        .verify(&Some(master))
        .expect("legacy ghost key cert must chain back to fixture master key");
    assert_eq!(info, "donation_amount:20");
}

#[test]
fn legacy_signed_message_still_verifies() {
    let signed: SignedMessage = SignedMessage::from_file(&fixture_path("signed_message.bin"))
        .expect("legacy signed_message.bin must deserialize under new types");

    let master = load_master_verifying_key();
    signed
        .certificate
        .verify(&Some(master))
        .expect("cert chain inside legacy signed message must verify");

    // The ghost key signature over the message must also still verify.
    signed
        .certificate
        .verifying_key
        .verify(&signed.message, &signed.signature)
        .expect("ed25519 signature over legacy message must verify under new types");
}

#[test]
fn reserializing_legacy_notary_cert_is_byte_identical() {
    // Load an old cert, re-serialize with new code, and assert the bytes
    // are identical. This is the strongest possible check that the
    // #[serde(rename = "delegate_verifying_key")] freeze is correct: any
    // drift in the CBOR key names would change the payload bytes, which
    // would invalidate the master signature that's verified above.
    let cert = NotaryCertificateV1::from_file(&fixture_path("delegate_certificate.pem"))
        .expect("legacy notary cert must parse");

    let round_tripped_bytes = cert.to_bytes().expect("notary cert must serialize");

    // Independently parse the legacy PEM → base64 → bytes path to compare.
    let legacy_pem = std::fs::read_to_string(fixture_path("delegate_certificate.pem"))
        .expect("fixture readable");
    let legacy_b64: String = legacy_pem
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect();
    use base64::Engine;
    let legacy_bytes = base64::engine::general_purpose::STANDARD
        .decode(&legacy_b64)
        .expect("legacy base64 decodes");

    assert_eq!(
        round_tripped_bytes, legacy_bytes,
        "re-serialized notary cert must be byte-identical to the legacy fixture; \
         any drift means the #[serde(rename)] wire-format freeze is broken"
    );
}

#[test]
fn reserializing_legacy_ghost_key_cert_is_byte_identical() {
    let cert = GhostkeyCertificateV1::from_file(&fixture_path("ghost_key_certificate.pem"))
        .expect("legacy ghost key cert must parse");

    let round_tripped_bytes = cert.to_bytes().expect("ghost key cert must serialize");

    let legacy_pem = std::fs::read_to_string(fixture_path("ghost_key_certificate.pem"))
        .expect("fixture readable");
    let legacy_b64: String = legacy_pem
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect();
    use base64::Engine;
    let legacy_bytes = base64::engine::general_purpose::STANDARD
        .decode(&legacy_b64)
        .expect("legacy base64 decodes");

    assert_eq!(
        round_tripped_bytes, legacy_bytes,
        "re-serialized ghost key cert must be byte-identical to the legacy fixture"
    );
}

#[test]
fn new_code_writes_canonical_armor_header() {
    // New code should write `BEGIN NOTARY_CERTIFICATE_V1`, not the legacy
    // `BEGIN DELEGATE_CERTIFICATE_V1`. Reading back the new header must
    // also work (tested via round-trip).
    let cert = NotaryCertificateV1::from_file(&fixture_path("delegate_certificate.pem"))
        .expect("legacy cert must parse");

    let armored = cert.to_armored_string().expect("notary cert must armor");

    assert!(
        armored.contains("-----BEGIN NOTARY_CERTIFICATE_V1-----"),
        "new code must write canonical NOTARY_CERTIFICATE_V1 header, got: {}",
        armored.lines().next().unwrap_or("(empty)")
    );
    assert!(
        !armored.contains("DELEGATE_CERTIFICATE_V1"),
        "new code must NOT write the legacy DELEGATE_CERTIFICATE_V1 header"
    );

    // Round-trip the new-style armor back through the same parser.
    let round_tripped =
        NotaryCertificateV1::from_armored_string(&armored).expect("new-style armor must parse");
    assert_eq!(round_tripped.payload.info, cert.payload.info);
}
