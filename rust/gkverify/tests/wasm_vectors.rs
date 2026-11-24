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
use ghostkey_lib::blind_rsa::{
    DefaultRng, PublicKey as NotaryPublicKey, RawPublicKey, Signature as RsaSignature,
};
use ghostkey_lib::ghost_key_certificate::GhostkeyCertificateV1;
use ghostkey_lib::notary_certificate::NotaryCertificateV1;
use ghostkey_lib::signed_message::SignedMessage;
use ghostkey_lib::util::unblinded_rsa_sign;
use ghostkey_verify::{production_master, verify_payload_signature, Error, GhostkeyCertificate};
use rand::rngs::OsRng;

const INFO: &str =
    r#"{"action":"freenet-donation","amount":5,"delegate-key-created":"2026-10-01 00:00:00"}"#;

/// The probe's result for a proof that verifies: the month, shifted.
const OK_OCTOBER: i64 = 10 << 32;

/// The probe's code for each error: the same table as `wasm-probe`'s `code`.
fn code(e: Error) -> i64 {
    match e {
        Error::TooLarge => -10,
        Error::Armor => -11,
        Error::Malformed => -12,
        Error::WeakMasterKey => -13,
        Error::NotarySignature => -14,
        Error::NotaryKey => -15,
        Error::GhostKeySignature => -16,
        Error::WeakGhostKey => -17,
        Error::PayloadSignatureLength => -18,
        Error::PayloadSignature => -19,
        Error::Info => -20,
        _ => -99,
    }
}

struct Vector {
    name: String,
    master: VerifyingKey,
    cert: String,
    sig: [u8; 64],
    payload: Vec<u8>,
    expect: i64,
}

/// What the probe computes, natively.
fn native(v: &Vector) -> i64 {
    let run = || -> Result<u8, Error> {
        let verified = GhostkeyCertificate::from_armored(&v.cert)?.verify(&v.master)?;
        verify_payload_signature(verified.ghost_key(), &v.payload, &v.sig)?;
        Ok(verified.notary_info()?.created_month.month)
    };
    match run() {
        Ok(month) => i64::from(month) << 32,
        Err(e) => code(e),
    }
}

/// `cert` with its RSA signature replaced by a `len`-byte CBOR byte string.
fn with_byte_string_signature(cert: &GhostkeyCertificateV1, len: usize) -> String {
    use ciborium::Value;
    let mut v: Value = ciborium::de::from_reader(cert.to_bytes().unwrap().as_slice()).unwrap();
    if let Value::Map(m) = &mut v {
        for (k, val) in m.iter_mut() {
            if k.as_text() == Some("signature") {
                *val = Value::Bytes(vec![0xff; len]);
            }
        }
    }
    let mut cbor = Vec::new();
    ciborium::ser::into_writer(&v, &mut cbor).unwrap();
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&cbor);
    let pem = format!(
        "-----BEGIN GHOSTKEY_CERTIFICATE_V1-----\n{b64}\n-----END GHOSTKEY_CERTIFICATE_V1-----\n"
    );
    // Under the limit as sent, so the refusal is the canonical-form one.
    assert!(pem.len() <= ghostkey_verify::MAX_CERTIFICATE_BYTES);
    pem
}

/// `cert` with an unknown top-level field holding arrays nested `depth`
/// deep, which both decoders skip, up to ciborium's recursion limit.
fn with_nested_unknown_field(cert: &GhostkeyCertificateV1, depth: usize) -> String {
    use ciborium::Value;
    let mut v: Value = ciborium::de::from_reader(cert.to_bytes().unwrap().as_slice()).unwrap();
    let mut nested = Value::Array(vec![]);
    for _ in 0..depth {
        nested = Value::Array(vec![nested]);
    }
    if let Value::Map(m) = &mut v {
        m.push((Value::Text("x".into()), nested));
    }
    let mut cbor = Vec::new();
    ciborium::ser::into_writer(&v, &mut cbor).unwrap();
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&cbor);
    format!(
        "-----BEGIN GHOSTKEY_CERTIFICATE_V1-----\n{b64}\n-----END GHOSTKEY_CERTIFICATE_V1-----\n"
    )
}

/// An armoured certificate under the notary RSA key `sk`, made without
/// `NotaryCertificateV1::new`, which only makes 2048-bit e = 65537 keys, and
/// signed with rsa's PSS directly, since blind-rsa-signatures' blind signing
/// does not produce a valid signature under a 2049-bit key.
fn cert_under_notary_key(master: &SigningKey, sk: &rsa::PrivateKey) -> (String, SigningKey) {
    use ghostkey_lib::notary_certificate::NotaryPayload;
    let payload = NotaryPayload {
        notary_verifying_key: notary_public(sk),
        info: INFO.to_string(),
    };
    let notary = NotaryCertificateV1 {
        signature: master.sign(&payload.to_bytes().unwrap()),
        payload,
    };
    let ghost = SigningKey::generate(&mut OsRng);
    let msg = Armorable::to_bytes(&ghost.verifying_key()).unwrap();
    let cert = GhostkeyCertificateV1 {
        notary,
        verifying_key: ghost.verifying_key(),
        signature: pss_sign(sk, 48, &msg),
    };
    // The reference accepts it too, or the vector tests nothing.
    assert!(cert.verify(&Some(master.verifying_key())).is_ok());
    (cert.to_armored_string().unwrap(), ghost)
}

/// A notary RSA key of `bits` bits with public exponent `e`, of a shape the
/// issuing code never makes.
fn rsa_key(bits: usize, e: u32) -> rsa::PrivateKey {
    rsa::PrivateKey::new_with_exp(&mut DefaultRng, bits, rsa::BoxedUint::from(e)).unwrap()
}

/// `sk`'s public half as ghostkey_lib holds a notary key.
fn notary_public(sk: &rsa::PrivateKey) -> NotaryPublicKey {
    NotaryPublicKey(RawPublicKey::new(sk.to_public_key()))
}

/// RSA-PSS SHA-384 over `msg` with salt length `salt`, left-padded to the
/// modulus length as blind-rsa-signatures pads.
fn pss_sign(sk: &rsa::PrivateKey, salt: usize, msg: &[u8]) -> RsaSignature {
    use rsa::signature::{RandomizedSigner, SignatureEncoding};
    use rsa::traits::PublicKeyParts;
    let signer = rsa::pss::SigningKey::<rsa::sha2::Sha384>::new_with_salt_len(sk.clone(), salt);
    let sig = signer.sign_with_rng(&mut DefaultRng, msg).to_vec();
    let mut padded = vec![0u8; sk.size() - sig.len()];
    padded.extend_from_slice(&sig);
    RsaSignature(padded)
}

#[test]
fn write_wasm_vectors() {
    let master = SigningKey::generate(&mut OsRng);
    let (notary, notary_sk) = NotaryCertificateV1::new(&master, &INFO.to_string()).unwrap();
    let (cert, ghost) = GhostkeyCertificateV1::new(&notary, &notary_sk);
    let pem = |c: &GhostkeyCertificateV1| c.to_armored_string().unwrap();
    let payload = b"a ScopedPayload, as the vault returned it".to_vec();
    let sig = ghost.sign(&payload).to_bytes();

    let mut tampered = cert.clone();
    tampered.notary.payload.info = INFO.replace("\"amount\":5", "\"amount\":5000");

    let mut wrong_rsa = cert.clone();
    let other = SigningKey::generate(&mut OsRng).verifying_key();
    wrong_rsa.signature =
        unblinded_rsa_sign(&notary_sk, &Armorable::to_bytes(&other).unwrap()).unwrap();

    let mut weak = cert.clone();
    let mut identity = [0u8; 32];
    identity[0] = 1;
    weak.verifying_key = VerifyingKey::from_bytes(&identity).unwrap();
    weak.signature = unblinded_rsa_sign(
        &notary_sk,
        &Armorable::to_bytes(&weak.verifying_key).unwrap(),
    )
    .unwrap();

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let production =
        std::fs::read_to_string(dir.join("tests/fixtures/production_ghostkey_certificate.pem"))
            .unwrap();
    let legacy = dir.join("../gklib/tests/fixtures/legacy_v1");
    let legacy_master = VerifyingKey::from_file(&legacy.join("master_verifying_key.pem")).unwrap();
    let signed = SignedMessage::from_file(&legacy.join("signed_message.bin")).unwrap();

    let m = master.verifying_key();
    let mut vectors = vec![
        Vector {
            name: "valid".into(),
            master: m,
            cert: pem(&cert),
            sig,
            payload: payload.clone(),
            expect: OK_OCTOBER,
        },
        Vector {
            // The real chain verifies under wasm; the signature is not the
            // ghost key's, whose signing key nobody here has.
            name: "production-chain".into(),
            master: production_master(),
            cert: production,
            sig,
            payload: payload.clone(),
            expect: code(Error::PayloadSignature),
        },
        Vector {
            name: "legacy-signed-message".into(),
            master: legacy_master,
            cert: signed.certificate.to_armored_string().unwrap(),
            sig: signed.signature.to_bytes(),
            payload: signed.message.clone(),
            expect: code(Error::Info),
        },
        Vector {
            name: "tampered-info".into(),
            master: m,
            cert: pem(&tampered),
            sig,
            payload: payload.clone(),
            expect: code(Error::NotarySignature),
        },
        Vector {
            name: "wrong-master".into(),
            master: SigningKey::generate(&mut OsRng).verifying_key(),
            cert: pem(&cert),
            sig,
            payload: payload.clone(),
            expect: code(Error::NotarySignature),
        },
        Vector {
            name: "wrong-rsa-signature".into(),
            master: m,
            cert: pem(&wrong_rsa),
            sig,
            payload: payload.clone(),
            expect: code(Error::GhostKeySignature),
        },
        Vector {
            name: "weak-ghost-key".into(),
            master: m,
            cert: pem(&weak),
            sig,
            payload: payload.clone(),
            expect: code(Error::WeakGhostKey),
        },
        Vector {
            name: "wrong-payload".into(),
            master: m,
            cert: pem(&cert),
            sig,
            payload: b"something else".to_vec(),
            expect: code(Error::PayloadSignature),
        },
    ];

    // More fresh notary keys through the 32-bit-limb arithmetic, whose
    // behaviour depends on the operands.
    for i in 2..=5 {
        let (notary, notary_sk) = NotaryCertificateV1::new(&master, &INFO.to_string()).unwrap();
        let (cert, ghost) = GhostkeyCertificateV1::new(&notary, &notary_sk);
        vectors.push(Vector {
            name: format!("valid-key-{i}"),
            master: m,
            cert: pem(&cert),
            sig: ghost.sign(&payload).to_bytes(),
            payload: payload.clone(),
            expect: OK_OCTOBER,
        });
    }
    // Notary keys the issuing code never makes but the crate accepts: e = 3,
    // and a modulus whose length is not a multiple of 8 bits.
    for (name, sk) in [
        ("notary-e3", rsa_key(2048, 3)),
        ("notary-2049-bits", rsa_key(2049, 65537)),
    ] {
        let (cert, ghost) = cert_under_notary_key(&master, &sk);
        vectors.push(Vector {
            name: name.into(),
            master: m,
            cert,
            sig: ghost.sign(&payload).to_bytes(),
            payload: payload.clone(),
            expect: OK_OCTOBER,
        });
    }
    // Deep nesting in a field both decoders skip: within ciborium's limit it
    // must verify on wasm's stack, beyond it the certificate is refused.
    vectors.push(Vector {
        name: "nested-unknown-field-200".into(),
        master: m,
        cert: with_nested_unknown_field(&cert, 200),
        sig,
        payload: payload.clone(),
        expect: OK_OCTOBER,
    });
    vectors.push(Vector {
        name: "nested-unknown-field-300".into(),
        master: m,
        cert: with_nested_unknown_field(&cert, 300),
        sig,
        payload: payload.clone(),
        expect: code(Error::Armor),
    });
    // Fits as sent, not once canonical: the RSA signature as a CBOR byte
    // string, which re-encodes at twice the size. Exercises the computed
    // armour length on 32-bit usize.
    vectors.push(Vector {
        name: "oversized-canonical-form".into(),
        master: m,
        cert: with_byte_string_signature(&cert, 2000),
        sig,
        payload: payload.clone(),
        expect: code(Error::TooLarge),
    });
    vectors.push(Vector {
        name: "oversized".into(),
        master: m,
        cert: format!(
            "{}{}",
            pem(&cert),
            " ".repeat(ghostkey_verify::MAX_CERTIFICATE_BYTES)
        ),
        sig,
        payload: payload.clone(),
        expect: code(Error::TooLarge),
    });
    vectors.push(Vector {
        name: "weak-master".into(),
        master: weak.verifying_key,
        cert: pem(&cert),
        sig,
        payload: payload.clone(),
        expect: code(Error::WeakMasterKey),
    });

    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("wasm-vectors");
    std::fs::create_dir_all(&out).unwrap();
    let mut manifest = Vec::new();
    for v in &vectors {
        assert_eq!(native(v), v.expect, "{}: native verdict", v.name);
        // -99 is the probe's catch-all for an error its table does not name.
        assert_ne!(
            v.expect, -99,
            "{}: an error missing from the code table",
            v.name
        );
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
