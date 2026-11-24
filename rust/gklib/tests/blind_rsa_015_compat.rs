//! Compatibility of this crate (blind-rsa-signatures 0.19, rsa 0.10) with the
//! code every Ghost Key so far was issued with: ghostkey_lib 0.2.0 and
//! blind-rsa-signatures 0.15.1 (rsa 0.8).
//!
//! Two kinds of evidence:
//!
//! - Frozen fixtures in `tests/fixtures/blind_rsa_015/`, written by the old
//!   code (generator: `regenerate_fixtures_with_the_old_code`, ignored). They
//!   include a notary signing key file in the form gkapi loads from disk and
//!   a full issuing transcript: the blinded message a browser sent, the blind
//!   signature the server returned, and the result.
//! - Live round trips against ghostkey_lib 0.2.0 from crates.io (byte-identical
//!   to the source of main before this change), in both directions: old
//!   browser WASM against a new gkapi, a new browser WASM against an old
//!   gkapi, new certificates checked by old verifiers, and key files passed
//!   between old and new binaries (rollback).
//!
//! RSA signing is deterministic (`s = m^d mod n`; the RSA blinding inside
//! `blind_sign` cancels out), so a blind signature from either version must
//! be the same bytes for the same key and blinded message.
//!
//! The production certificate is a real Ghost Key, issued by gkapi and
//! signed by the real master key.

use std::path::PathBuf;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::VerifyingKey;
use ghostkey_lib::armorable::Armorable;
use ghostkey_lib::blind_rsa::{
    BlindMessage, BlindSignature, DefaultRng, PublicKey, Secret, SecretKey, Signature,
};
use ghostkey_lib::ghost_key_certificate::GhostkeyCertificateV1;
use ghostkey_lib::notary_certificate::NotaryCertificateV1;
use ghostkey_lib::util::create_keypair;
use rand_core::OsRng;

// The old Armorable is used by path only: with both blanket traits in scope
// every method call would be ambiguous.
use ghostkey_lib_02::armorable as old_armor;
use ghostkey_lib_02::ghost_key_certificate::GhostkeyCertificateV1 as OldGhostkeyCertificate;
use ghostkey_lib_02::notary_certificate::NotaryCertificateV1 as OldNotaryCertificate;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/blind_rsa_015")
        .join(name)
}

struct Transcript {
    message: Vec<u8>,
    blinded_ghost_key_base64: String,
    blinding_secret: Vec<u8>,
    blind_signature_base64: String,
    signature: Vec<u8>,
    notary_certificate_base64: String,
    ghost_key_certificate_base64: String,
    info: String,
}

fn transcript() -> Transcript {
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fixture("transcript.json")).unwrap())
            .unwrap();
    let s = |k: &str| v[k].as_str().unwrap().to_string();
    let b = |k: &str| B64.decode(s(k)).unwrap();
    Transcript {
        message: b("message_base64"),
        blinded_ghost_key_base64: s("blinded_ghost_key_base64"),
        blinding_secret: b("blinding_secret_base64"),
        blind_signature_base64: s("blind_signature_base64"),
        signature: b("signature_base64"),
        notary_certificate_base64: s("notary_certificate_base64"),
        ghost_key_certificate_base64: s("ghost_key_certificate_base64"),
        info: s("info"),
    }
}

fn master() -> VerifyingKey {
    VerifyingKey::from_file(&fixture("master_verifying_key.pem")).unwrap()
}

fn cbor<T: serde::Serialize>(v: &T) -> Vec<u8> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(v, &mut buf).unwrap();
    buf
}

// --- frozen fixtures from the old code -------------------------------------

#[test]
fn old_notary_certificate_verifies_and_reencodes_identically() {
    let t = transcript();
    let cert = NotaryCertificateV1::from_file(&fixture("notary_certificate.pem")).unwrap();
    assert_eq!(cert.verify(&Some(master())).unwrap(), t.info);
    assert_eq!(cert.to_base64().unwrap(), t.notary_certificate_base64);
    assert_eq!(
        cert.to_armored_string().unwrap(),
        std::fs::read_to_string(fixture("notary_certificate.pem")).unwrap()
    );
}

#[test]
fn old_ghost_key_certificate_verifies_and_reencodes_identically() {
    let t = transcript();
    let cert = GhostkeyCertificateV1::from_file(&fixture("ghost_key_certificate.pem")).unwrap();
    assert_eq!(cert.verify(&Some(master())).unwrap(), t.info);
    assert_eq!(cert.to_base64().unwrap(), t.ghost_key_certificate_base64);
    assert_eq!(cert.signature.0, t.signature);
}

#[test]
fn production_certificate_verifies_against_the_real_master_key() {
    let pem = std::fs::read_to_string(fixture("production_ghostkey_certificate.pem")).unwrap();
    let cert = GhostkeyCertificateV1::from_armored_string(&pem).unwrap();
    // `None` means the built-in production master key.
    let info = cert.verify(&None).unwrap();
    let old = <OldGhostkeyCertificate as old_armor::Armorable>::from_armored_string(&pem).unwrap();
    assert_eq!(old.verify(&None).unwrap(), info);
    assert_eq!(cert.to_armored_string().unwrap().trim(), pem.trim());
    // A tampered copy must still be refused.
    let mut bad = cert.clone();
    bad.signature.0[100] ^= 1;
    assert!(bad.verify(&None).is_err());
}

#[test]
fn old_notary_signing_key_file_loads_and_rewrites_identically() {
    let file = std::fs::read_to_string(fixture("notary_signing_key.pem")).unwrap();
    assert!(file.starts_with("-----BEGIN SECRET_KEY_V1-----"));
    let sk = SecretKey::from_armored_string(&file).unwrap();
    let cert = NotaryCertificateV1::from_file(&fixture("notary_certificate.pem")).unwrap();
    assert_eq!(sk.public_key().unwrap(), cert.payload.notary_verifying_key);
    assert_eq!(sk.to_armored_string().unwrap(), file);
}

/// An old browser's blinded message, signed by the new server, gives the
/// very bytes the old server gave.
#[test]
fn new_server_signs_an_old_clients_blinded_message_identically() {
    let t = transcript();
    let sk = SecretKey::from_file(&fixture("notary_signing_key.pem")).unwrap();
    let blinded = BlindMessage::from_base64(&t.blinded_ghost_key_base64).unwrap();
    assert_eq!(blinded.to_base64().unwrap(), t.blinded_ghost_key_base64);
    let blind_sig = sk.blind_sign(&blinded).unwrap();
    assert_eq!(blind_sig.to_base64().unwrap(), t.blind_signature_base64);
}

/// The new browser code unblinds what the old server returned to the same
/// signature the old browser code produced.
#[test]
fn new_client_finalizes_an_old_servers_blind_signature_identically() {
    let t = transcript();
    let cert = NotaryCertificateV1::from_base64(&t.notary_certificate_base64).unwrap();
    let blind_sig = BlindSignature::from_base64(&t.blind_signature_base64).unwrap();
    let sig = cert
        .payload
        .notary_verifying_key
        .finalize_with_secret(&blind_sig, Secret(t.blinding_secret.clone()), &t.message)
        .unwrap();
    assert_eq!(sig.0, t.signature);
}

// --- live round trips against ghostkey_lib 0.2.0 ---------------------------

/// The old crate's view of a new key or certificate: decode the new bytes
/// with the old types.
fn to_old<T: old_armor::Armorable>(bytes: &[u8]) -> T {
    T::from_bytes(bytes).expect("old code must decode what new code writes")
}

fn old_options() -> blind_rsa_015::Options {
    blind_rsa_015::Options::default()
}

/// Old browser (0.15 blind/finalize) against the new gkapi signer.
#[test]
fn old_client_against_new_server() {
    let master = ed25519_dalek::SigningKey::generate(&mut OsRng);
    let (cert, sk) = NotaryCertificateV1::new(&master, &"x".to_string()).unwrap();
    let old_cert: OldNotaryCertificate = to_old(&cert.to_bytes().unwrap());
    let old_pk = &old_cert.payload.notary_verifying_key;

    let (_, ghost_vk) = create_keypair(&mut OsRng).unwrap();
    let msg = Armorable::to_bytes(&ghost_vk).unwrap();
    let blinding = old_pk
        .blind(&mut OsRng, &msg, false, &old_options())
        .unwrap();

    // Over the wire as base64 of the CBOR newtype, both ways.
    let blinded =
        BlindMessage::from_base64(&old_armor::Armorable::to_base64(&blinding.blind_msg).unwrap())
            .unwrap();
    let blind_sig = sk.blind_sign(&blinded).unwrap();
    let old_blind_sig =
        blind_rsa_015::BlindSignature::from_base64(&blind_sig.to_base64().unwrap()).unwrap();

    let sig = old_pk
        .finalize(&old_blind_sig, &blinding.secret, None, &msg, &old_options())
        .expect("old client must unblind the new server's signature");

    // The certificate the old browser assembles verifies with both versions.
    let old_ghost = OldGhostkeyCertificate {
        notary: old_cert,
        verifying_key: ghost_vk,
        signature: sig,
    };
    let bytes = old_armor::Armorable::to_bytes(&old_ghost).unwrap();
    assert_eq!(
        old_ghost.verify(&Some(master.verifying_key())).unwrap(),
        "x"
    );
    let new_ghost = GhostkeyCertificateV1::from_bytes(&bytes).unwrap();
    assert_eq!(
        new_ghost.verify(&Some(master.verifying_key())).unwrap(),
        "x"
    );
    assert_eq!(new_ghost.to_bytes().unwrap(), bytes);
}

/// New browser against an old gkapi signing with an old key file. This is
/// the window between the site deploy (immediate on merge) and the manual
/// gkapi deploy.
#[test]
fn new_client_against_old_server() {
    let master = ghostkey_lib_02::util::create_keypair(&mut OsRng).unwrap().0;
    let (old_cert, old_sk) = OldNotaryCertificate::new(&master, &"y".to_string()).unwrap();
    let cert = NotaryCertificateV1::from_bytes(&old_armor::Armorable::to_bytes(&old_cert).unwrap())
        .expect("new code must decode an old notary certificate");
    let pk = &cert.payload.notary_verifying_key;

    let (_, ghost_vk) = create_keypair(&mut OsRng).unwrap();
    let msg = Armorable::to_bytes(&ghost_vk).unwrap();
    let blinding = pk.blind(&mut DefaultRng, &msg).unwrap();

    let old_blinded =
        blind_rsa_015::BlindedMessage::from_base64(&blinding.blind_message.to_base64().unwrap())
            .unwrap();
    let old_blind_sig = old_sk
        .blind_sign(&mut OsRng, &old_blinded, &old_options())
        .expect("old server must sign the new client's blinded message");
    let blind_sig =
        BlindSignature::from_base64(&old_armor::Armorable::to_base64(&old_blind_sig).unwrap())
            .unwrap();

    let sig = pk
        .finalize_with_secret(&blind_sig, blinding.secret.clone(), &msg)
        .expect("new client must unblind the old server's signature");

    let ghost = GhostkeyCertificateV1 {
        notary: cert,
        verifying_key: ghost_vk,
        signature: sig,
    };
    let bytes = ghost.to_bytes().unwrap();
    assert_eq!(ghost.verify(&Some(master.verifying_key())).unwrap(), "y");
    let old_ghost: OldGhostkeyCertificate = to_old(&bytes);
    assert_eq!(
        old_ghost.verify(&Some(master.verifying_key())).unwrap(),
        "y"
    );
}

/// Certificates the new code issues (new notary key, new signing) are
/// accepted by the old verifier and are the bytes it would re-encode.
#[test]
fn new_certificates_verify_under_the_old_code() {
    let master = ed25519_dalek::SigningKey::generate(&mut OsRng);
    let (cert, sk) = NotaryCertificateV1::new(&master, &"z".to_string()).unwrap();
    let (ghost, _) = GhostkeyCertificateV1::new(&cert, &sk);
    let bytes = ghost.to_bytes().unwrap();
    let old: OldGhostkeyCertificate = to_old(&bytes);
    assert_eq!(old.verify(&Some(master.verifying_key())).unwrap(), "z");
    assert_eq!(old_armor::Armorable::to_bytes(&old).unwrap(), bytes);
    assert_eq!(
        old_armor::Armorable::to_armored_string(&old).unwrap(),
        ghost.to_armored_string().unwrap()
    );
}

/// Key files move between versions in both directions (a rollback must
/// load keys a new CLI generated), and both versions sign identically.
#[test]
fn signing_key_files_are_interchangeable_and_sign_identically() {
    // New -> old.
    let (pk, sk) = SecretKey::generate().unwrap();
    let armored = sk.to_armored_string().unwrap();
    assert!(armored.starts_with("-----BEGIN SECRET_KEY_V1-----"));
    let old_sk = <blind_rsa_015::SecretKey as old_armor::Armorable>::from_armored_string(&armored)
        .expect("old code must load a key file the new code wrote");
    let old_pk = old_sk.public_key().unwrap();
    assert_eq!(old_armor::Armorable::to_bytes(&old_pk).unwrap(), cbor(&pk));
    assert_eq!(old_pk.to_der().unwrap(), pk.to_der().unwrap());

    let (_, ghost_vk) = create_keypair(&mut OsRng).unwrap();
    let msg = Armorable::to_bytes(&ghost_vk).unwrap();
    let blinding = pk.blind(&mut DefaultRng, &msg).unwrap();
    let new_sig = sk.blind_sign(&blinding.blind_message).unwrap();
    let old_sig = old_sk
        .blind_sign(
            &mut OsRng,
            blind_rsa_015::BlindedMessage(blinding.blind_message.0.clone()),
            &old_options(),
        )
        .unwrap();
    assert_eq!(new_sig.0, old_sig.0);

    // Old -> new.
    let old_kp = blind_rsa_015::KeyPair::generate(&mut OsRng, 2048).unwrap();
    let old_armored = old_armor::Armorable::to_armored_string(&old_kp.sk).unwrap();
    let sk = SecretKey::from_armored_string(&old_armored).unwrap();
    assert_eq!(sk.to_armored_string().unwrap(), old_armored);
    let pk: PublicKey = sk.public_key().unwrap();
    assert_eq!(
        cbor(&pk),
        old_armor::Armorable::to_bytes(&old_kp.pk).unwrap()
    );
}

/// A signature from one version verifies under the other.
#[test]
fn signatures_verify_across_versions() {
    let old_kp = blind_rsa_015::KeyPair::generate(&mut OsRng, 2048).unwrap();
    let pk: PublicKey = ciborium::de::from_reader(
        old_armor::Armorable::to_bytes(&old_kp.pk)
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    let old_sig = ghostkey_lib_02::util::unblinded_rsa_sign(&old_kp, b"message").unwrap();
    pk.verify_signature(&Signature(old_sig.0.clone()), b"message")
        .unwrap();

    let sk: SecretKey = ciborium::de::from_reader(
        old_armor::Armorable::to_bytes(&old_kp.sk)
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    let new_sig = ghostkey_lib::util::unblinded_rsa_sign(&sk, b"message").unwrap();
    old_kp
        .pk
        .verify(
            &blind_rsa_015::Signature(new_sig.0),
            None,
            b"message",
            &old_options(),
        )
        .unwrap();
}

/// Verdicts on signatures the issuing code never makes, old against new:
/// other PSS salt lengths and wrong lengths agree; a signature not reduced
/// modulo n is the one documented change (old accepted, new refuses).
#[test]
fn odd_signature_verdicts_old_against_new() {
    use ghostkey_lib::blind_rsa::reexports::rsa;
    use rsa::signature::{RandomizedSigner, SignatureEncoding};
    use rsa::traits::PublicKeyParts;

    let old_kp = blind_rsa_015::KeyPair::generate(&mut OsRng, 2048).unwrap();
    let old_bytes = old_armor::Armorable::to_bytes(&old_kp.sk).unwrap();
    let sk: SecretKey = ciborium::de::from_reader(old_bytes.as_slice()).unwrap();
    let pk = sk.public_key().unwrap();
    let raw: &rsa::PrivateKey = sk.0.as_ref();
    let size = raw.size();
    let msg = b"a ghost key";

    let old_ok = |sig: &[u8]| {
        old_kp
            .pk
            .verify(
                &blind_rsa_015::Signature(sig.to_vec()),
                None,
                msg,
                &old_options(),
            )
            .is_ok()
    };
    let new_ok = |sig: &[u8]| pk.verify_signature(&Signature(sig.to_vec()), msg).is_ok();
    let pad = |sig: Vec<u8>| {
        let mut out = vec![0u8; size - sig.len()];
        out.extend_from_slice(&sig);
        out
    };

    for salt in [0, 1, 20, 32, 48, 64, 100] {
        let signer =
            rsa::pss::SigningKey::<rsa::sha2::Sha384>::new_with_salt_len(raw.clone(), salt);
        let sig = pad(signer.sign_with_rng(&mut DefaultRng, msg).to_vec());
        assert!(old_ok(&sig), "old refused salt {salt}");
        assert!(new_ok(&sig), "new refused salt {salt}");

        for bad in [sig[1..].to_vec(), [&[0u8][..], &sig].concat(), Vec::new()] {
            assert!(!old_ok(&bad) && !new_ok(&bad), "length {}", bad.len());
        }
        let mut flipped = sig.clone();
        flipped[size / 2] ^= 1;
        assert!(!old_ok(&flipped) && !new_ok(&flipped));
    }

    // s + n, when it still fits the modulus length. Whether it fits depends
    // on the key and the signature; with n at least 3/4 of 2^2048 a key whose
    // 128 tries all miss is rare, and sixteen such keys in a row are not seen.
    for attempt in 0..16 {
        let old_kp = if attempt == 0 {
            old_kp.clone()
        } else {
            blind_rsa_015::KeyPair::generate(&mut OsRng, 2048).unwrap()
        };
        let old_bytes = old_armor::Armorable::to_bytes(&old_kp.sk).unwrap();
        let sk: SecretKey = ciborium::de::from_reader(old_bytes.as_slice()).unwrap();
        let pk = sk.public_key().unwrap();
        let raw: &rsa::PrivateKey = sk.0.as_ref();
        let n = raw.n().as_ref().to_be_bytes().to_vec();
        let old_ok = |sig: &[u8]| {
            old_kp
                .pk
                .verify(
                    &blind_rsa_015::Signature(sig.to_vec()),
                    None,
                    msg,
                    &old_options(),
                )
                .is_ok()
        };
        let new_ok = |sig: &[u8]| pk.verify_signature(&Signature(sig.to_vec()), msg).is_ok();
        for _ in 0..128 {
            let sig = ghostkey_lib::util::unblinded_rsa_sign(&sk, msg).unwrap().0;
            let bigger = add_be(&sig, &n);
            if bigger.len() != size {
                continue;
            }
            assert!(old_ok(&sig) && new_ok(&sig));
            assert!(old_ok(&bigger), "ghostkey_lib 0.2 accepted s + n");
            assert!(!new_ok(&bigger), "0.3 refuses s + n (RFC 8017)");
            return;
        }
    }
    panic!("no signature had room above its modulus");
}

/// `a + b` for big-endian unsigned integers, with no leading zero byte.
fn add_be(a: &[u8], b: &[u8]) -> Vec<u8> {
    let len = a.len().max(b.len()) + 1;
    let at = |x: &[u8], i: usize| if i < x.len() { x[x.len() - 1 - i] } else { 0 };
    let mut out = vec![0u8; len];
    let mut carry = 0u16;
    for i in 0..len {
        let s = at(a, i) as u16 + at(b, i) as u16 + carry;
        out[len - 1 - i] = s as u8;
        carry = s >> 8;
    }
    let first = out.iter().position(|&x| x != 0).unwrap_or(out.len());
    out.split_off(first)
}

/// Writes `tests/fixtures/blind_rsa_015/` with the OLD code. Kept for the
/// record; regenerating replaces the frozen evidence, so do not do it unless
/// the fixtures themselves are wrong.
#[test]
#[ignore]
fn regenerate_fixtures_with_the_old_code() {
    use blind_rsa_015::{BlindSignature as OldBlindSig, BlindedMessage as OldBlinded};
    let out = fixture("");
    let (master_sk, master_vk) = ghostkey_lib_02::util::create_keypair(&mut OsRng).unwrap();
    let info =
        r#"{"action":"freenet-donation","amount":20,"delegate-key-created":"2026-11-01 00:00:00"}"#
            .to_string();
    let (notary_cert, notary_sk) = OldNotaryCertificate::new(&master_sk, &info).unwrap();
    old_armor::Armorable::to_file(&master_vk, &out.join("master_verifying_key.pem")).unwrap();
    old_armor::Armorable::to_file(&notary_cert, &out.join("notary_certificate.pem")).unwrap();
    old_armor::Armorable::to_file(&notary_sk, &out.join("notary_signing_key.pem")).unwrap();

    let (ghost_sk, ghost_vk) = ghostkey_lib_02::util::create_keypair(&mut OsRng).unwrap();
    let msg = old_armor::Armorable::to_bytes(&ghost_vk).unwrap();
    let pk = &notary_cert.payload.notary_verifying_key;
    let blinding = pk.blind(&mut OsRng, &msg, false, &old_options()).unwrap();
    let blinded_b64 = old_armor::Armorable::to_base64(&blinding.blind_msg).unwrap();
    let blinded = <OldBlinded as old_armor::Armorable>::from_base64(&blinded_b64).unwrap();
    let blind_sig = notary_sk
        .blind_sign(&mut OsRng, &blinded, &old_options())
        .unwrap();
    let blind_sig_b64 = old_armor::Armorable::to_base64(&blind_sig).unwrap();
    let blind_sig = <OldBlindSig as old_armor::Armorable>::from_base64(&blind_sig_b64).unwrap();
    let sig = pk
        .finalize(&blind_sig, &blinding.secret, None, &msg, &old_options())
        .unwrap();
    let cert = OldGhostkeyCertificate {
        notary: notary_cert.clone(),
        verifying_key: ghost_vk,
        signature: sig.clone(),
    };
    assert_eq!(cert.verify(&Some(master_vk)).unwrap(), info);
    old_armor::Armorable::to_file(&cert, &out.join("ghost_key_certificate.pem")).unwrap();
    old_armor::Armorable::to_file(&ghost_sk, &out.join("ghost_signing_key.pem")).unwrap();

    let transcript = serde_json::json!({
        "generated_by": "ghostkey_lib 0.2.0 + blind-rsa-signatures 0.15.1 (rsa 0.8.2)",
        "message_base64": B64.encode(&msg),
        "blinded_ghost_key_base64": blinded_b64,
        "blinding_secret_base64": B64.encode(&blinding.secret.0),
        "blind_signature_base64": blind_sig_b64,
        "signature_base64": B64.encode(&sig.0),
        "notary_certificate_base64": old_armor::Armorable::to_base64(&notary_cert).unwrap(),
        "ghost_key_certificate_base64": old_armor::Armorable::to_base64(&cert).unwrap(),
        "info": info,
    });
    std::fs::write(
        out.join("transcript.json"),
        serde_json::to_string_pretty(&transcript).unwrap() + "\n",
    )
    .unwrap();
}
