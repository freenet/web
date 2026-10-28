//! Writes proof vectors for the wasm probe (`wasm-probe/`) to run under
//! wasm32, with the result each must give. CI runs this test, builds the
//! probe for wasm32-unknown-unknown and runs every vector through it with
//! `scripts/run_wasm_vectors.mjs`.
//!
//! The native verdict is asserted here too, so a vector whose expectation is
//! wrong fails before it ever reaches wasm.

use std::path::PathBuf;

use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use ghostkey_lib::armorable::Armorable;
use ghostkey_lib::ghost_key_certificate::GhostkeyCertificateV1;
use ghostkey_lib::notary_certificate::NotaryCertificateV1;
use ghostkey_lib::signed_message::SignedMessage;
use ghostkey_lib::util::unblinded_rsa_sign;
use ghostkey_verify::{production_master, verify_payload_signature, GhostkeyCertificate};
use rand::rngs::OsRng;

const INFO: &str =
    r#"{"action":"freenet-donation","amount":5,"delegate-key-created":"2026-10-01 00:00:00"}"#;

/// The probe's result: the month, shifted, or the code of the failing step.
const OK_OCTOBER: i64 = 10 << 32;
const CHAIN: i64 = -5;
const PAYLOAD: i64 = -6;
const INFO_FORMAT: i64 = -7;

struct Vector {
    name: &'static str,
    master: VerifyingKey,
    cert: String,
    sig: [u8; 64],
    payload: Vec<u8>,
    expect: i64,
}

/// What the probe computes, natively.
fn native(v: &Vector) -> i64 {
    let Ok(cert) = GhostkeyCertificate::from_armored(&v.cert) else {
        return -4;
    };
    let Ok(verified) = cert.verify(&v.master) else {
        return CHAIN;
    };
    if verify_payload_signature(&verified.ghost_key, &v.payload, &v.sig).is_err() {
        return PAYLOAD;
    }
    match verified.notary_info() {
        Ok(info) => i64::from(info.created_month.month) << 32,
        Err(_) => INFO_FORMAT,
    }
}

#[test]
fn write_wasm_vectors() {
    let master = SigningKey::generate(&mut OsRng);
    let (notary, notary_sk) = NotaryCertificateV1::new(&master, &INFO.to_string()).unwrap();
    let (cert, ghost) = GhostkeyCertificateV1::new(&notary, &notary_sk);
    let keypair = blind_rsa_signatures::KeyPair::new(notary_sk.public_key().unwrap(), notary_sk);
    let pem = |c: &GhostkeyCertificateV1| c.to_armored_string().unwrap();
    let payload = b"a ScopedPayload, as the vault returned it".to_vec();
    let sig = ghost.sign(&payload).to_bytes();

    let mut tampered = cert.clone();
    tampered.notary.payload.info = INFO.replace("\"amount\":5", "\"amount\":5000");

    let mut wrong_rsa = cert.clone();
    let other = SigningKey::generate(&mut OsRng).verifying_key();
    wrong_rsa.signature =
        unblinded_rsa_sign(&keypair, &Armorable::to_bytes(&other).unwrap()).unwrap();

    let mut weak = cert.clone();
    let mut identity = [0u8; 32];
    identity[0] = 1;
    weak.verifying_key = VerifyingKey::from_bytes(&identity).unwrap();
    weak.signature =
        unblinded_rsa_sign(&keypair, &Armorable::to_bytes(&weak.verifying_key).unwrap()).unwrap();

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let production =
        std::fs::read_to_string(dir.join("tests/fixtures/production_ghostkey_certificate.pem"))
            .unwrap();
    let legacy = dir.join("../gklib/tests/fixtures/legacy_v1");
    let legacy_master = VerifyingKey::from_file(&legacy.join("master_verifying_key.pem")).unwrap();
    let signed = SignedMessage::from_file(&legacy.join("signed_message.bin")).unwrap();

    let m = master.verifying_key();
    let vectors = [
        Vector {
            name: "valid",
            master: m,
            cert: pem(&cert),
            sig,
            payload: payload.clone(),
            expect: OK_OCTOBER,
        },
        Vector {
            // The real chain verifies under wasm; the signature is not the
            // ghost key's, whose signing key nobody here has.
            name: "production-chain",
            master: production_master(),
            cert: production,
            sig,
            payload: payload.clone(),
            expect: PAYLOAD,
        },
        Vector {
            name: "legacy-signed-message",
            master: legacy_master,
            cert: signed.certificate.to_armored_string().unwrap(),
            sig: signed.signature.to_bytes(),
            payload: signed.message.clone(),
            expect: INFO_FORMAT,
        },
        Vector {
            name: "tampered-info",
            master: m,
            cert: pem(&tampered),
            sig,
            payload: payload.clone(),
            expect: CHAIN,
        },
        Vector {
            name: "wrong-master",
            master: SigningKey::generate(&mut OsRng).verifying_key(),
            cert: pem(&cert),
            sig,
            payload: payload.clone(),
            expect: CHAIN,
        },
        Vector {
            name: "wrong-rsa-signature",
            master: m,
            cert: pem(&wrong_rsa),
            sig,
            payload: payload.clone(),
            expect: CHAIN,
        },
        Vector {
            name: "weak-ghost-key",
            master: m,
            cert: pem(&weak),
            sig,
            payload: payload.clone(),
            expect: CHAIN,
        },
        Vector {
            name: "wrong-payload",
            master: m,
            cert: pem(&cert),
            sig,
            payload: b"something else".to_vec(),
            expect: PAYLOAD,
        },
    ];

    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("wasm-vectors");
    std::fs::create_dir_all(&out).unwrap();
    let mut manifest = Vec::new();
    for v in &vectors {
        assert_eq!(native(v), v.expect, "{}: native verdict", v.name);
        let mut bytes = v.master.to_bytes().to_vec();
        bytes.extend_from_slice(&(v.cert.len() as u32).to_le_bytes());
        bytes.extend_from_slice(v.cert.as_bytes());
        bytes.extend_from_slice(&v.sig);
        bytes.extend_from_slice(&v.payload);
        std::fs::write(out.join(format!("{}.bin", v.name)), bytes).unwrap();
        manifest.push(format!(
            "{{\"name\":\"{}\",\"expect\":\"{}\"}}",
            v.name, v.expect
        ));
    }
    std::fs::write(
        out.join("manifest.json"),
        format!("[{}]\n", manifest.join(",")),
    )
    .unwrap();
}
