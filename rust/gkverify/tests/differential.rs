//! Differential tests: `ghostkey_verify` against `ghostkey_lib` 0.2, the
//! implementation every deployed Ghost Key verifier uses.
//!
//! The reference for the chain is `GhostkeyCertificateV1::from_bytes` /
//! `from_armored_string` followed by `verify`. The reference for payload
//! signatures is Ed25519 `verify_strict`, which is what freenet-bitcoin's
//! inbox contract applies to `ScopedPayload` signatures.
//!
//! Each case asserts both that the two agree and what they agree on, so a
//! case that silently stopped exercising its refusal (both accepting) fails
//! too. The known, intentional differences (see the crate docs) have their own
//! tests at the end, each asserting that `ghostkey_lib` accepts and this crate
//! refuses.

use std::path::PathBuf;

use blind_rsa_signatures::{KeyPair, Signature as RsaSignature};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use ghostkey_lib::armorable::Armorable;
use ghostkey_lib::ghost_key_certificate::GhostkeyCertificateV1;
use ghostkey_lib::notary_certificate::NotaryCertificateV1;
use ghostkey_lib::signed_message::SignedMessage;
use ghostkey_lib::util::unblinded_rsa_sign;
use ghostkey_verify::{
    production_master, verify_payload_signature, Error, GhostkeyCertificate, NotaryInfo, YearMonth,
    FREENET_MASTER_VERIFYING_KEY,
};
use rand::rngs::{OsRng, StdRng};
use rand::{Rng, SeedableRng};

const INFO: &str =
    r#"{"action":"freenet-donation","amount":20,"delegate-key-created":"2026-10-01 00:00:00"}"#;

/// A complete chain made by `ghostkey_lib` itself.
struct Chain {
    master: SigningKey,
    notary_keypair: KeyPair,
    notary: NotaryCertificateV1,
    cert: GhostkeyCertificateV1,
    ghost: SigningKey,
}

fn chain(info: &str) -> Chain {
    let master = SigningKey::generate(&mut OsRng);
    let (notary, notary_sk) = NotaryCertificateV1::new(&master, &info.to_string()).unwrap();
    let (cert, ghost) = GhostkeyCertificateV1::new(&notary, &notary_sk);
    let notary_keypair = KeyPair::new(notary_sk.public_key().unwrap(), notary_sk);
    Chain {
        master,
        notary_keypair,
        notary,
        cert,
        ghost,
    }
}

/// `ghostkey_lib`'s verdict on certificate bytes.
fn reference(bytes: &[u8], master: &VerifyingKey) -> Result<String, ()> {
    let cert = GhostkeyCertificateV1::from_bytes(bytes).map_err(|_| ())?;
    cert.verify(&Some(*master)).map_err(|_| ())
}

/// This crate's verdict on certificate bytes.
fn ours(bytes: &[u8], master: &VerifyingKey) -> Result<String, Error> {
    let cert = GhostkeyCertificate::from_cbor(bytes)?;
    cert.verify(master).map(|v| v.info)
}

/// Both must accept with the same info, or both refuse.
fn assert_agree(bytes: &[u8], master: &VerifyingKey) -> Result<String, Error> {
    let r = reference(bytes, master);
    let o = ours(bytes, master);
    assert_eq!(
        r.as_ref().ok(),
        o.as_ref().ok(),
        "ghostkey_lib said {r:?}, ghostkey_verify said {o:?}"
    );
    // Wherever both parse, both must re-encode to the same bytes: that
    // re-encoding is what the master key's signature is checked against.
    if let (Ok(r), Ok(o)) = (
        GhostkeyCertificateV1::from_bytes(bytes),
        GhostkeyCertificate::from_cbor(bytes),
    ) {
        assert_eq!(r.to_bytes().unwrap(), o.to_cbor(), "re-encodings differ");
    }
    o
}

fn bytes_of(cert: &GhostkeyCertificateV1) -> Vec<u8> {
    cert.to_bytes().unwrap()
}

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
}

// ---------------------------------------------------------------------------
// The chain: generated cases
// ---------------------------------------------------------------------------

#[test]
fn valid_chain_verifies_in_both_and_reencodes_identically() {
    let c = chain(INFO);
    let bytes = bytes_of(&c.cert);
    let master = c.master.verifying_key();
    assert_eq!(assert_agree(&bytes, &master), Ok(INFO.to_string()));

    let ours = GhostkeyCertificate::from_cbor(&bytes).unwrap();
    assert_eq!(ours.to_cbor(), bytes);
    assert_eq!(ours.to_armored(), c.cert.to_armored_string().unwrap());
    assert_eq!(ours.claimed_ghost_key(), &c.ghost.verifying_key());

    let verified = ours.verify(&master).unwrap();
    assert_eq!(verified.ghost_key, c.ghost.verifying_key());
    let info = verified.notary_info().unwrap();
    assert_eq!(info.amount, 20);
    assert_eq!(
        info.created_month,
        YearMonth {
            year: 2026,
            month: 10
        }
    );
}

#[test]
fn many_valid_chains_agree() {
    // Fresh RSA keys each time, so moduli and signatures with leading zero
    // bytes and high digits of every shape turn up.
    for i in 0..8 {
        let info = format!(
            r#"{{"action":"freenet-donation","amount":{i},"delegate-key-created":"2025-0{}-01 00:00:00"}}"#,
            i + 1
        );
        let c = chain(&info);
        assert_eq!(
            assert_agree(&bytes_of(&c.cert), &c.master.verifying_key()),
            Ok(info)
        );
    }
}

#[test]
fn tampered_info_is_refused_by_both() {
    let c = chain(INFO);
    let master = c.master.verifying_key();
    for tampered in [
        INFO.replace("20", "2500"),
        INFO.replace("2026-10", "2020-01"),
        String::new(),
        format!("{INFO} "),
    ] {
        let mut cert = c.cert.clone();
        cert.notary.payload.info = tampered;
        assert_eq!(
            assert_agree(&bytes_of(&cert), &master),
            Err(Error::NotarySignature)
        );
    }
}

#[test]
fn wrong_master_is_refused_by_both() {
    let c = chain(INFO);
    let bytes = bytes_of(&c.cert);
    let other = SigningKey::generate(&mut OsRng).verifying_key();
    assert_eq!(assert_agree(&bytes, &other), Err(Error::NotarySignature));
    assert_eq!(
        assert_agree(&bytes, &production_master()),
        Err(Error::NotarySignature)
    );
}

#[test]
fn notary_key_swapped_is_refused_by_both() {
    // A notary payload naming another RSA key, still carrying the master's
    // signature over the original.
    let c = chain(INFO);
    let other = chain(INFO);
    let mut cert = c.cert.clone();
    cert.notary.payload.notary_verifying_key = other.notary.payload.notary_verifying_key.clone();
    assert_eq!(
        assert_agree(&bytes_of(&cert), &c.master.verifying_key()),
        Err(Error::NotarySignature)
    );
}

#[test]
fn notary_signature_by_another_master_is_refused_by_both() {
    let c = chain(INFO);
    let mut cert = c.cert.clone();
    let impostor = SigningKey::generate(&mut OsRng);
    cert.notary.signature = impostor.sign(&cert.notary.payload.to_bytes().unwrap());
    assert_eq!(
        assert_agree(&bytes_of(&cert), &c.master.verifying_key()),
        Err(Error::NotarySignature)
    );
    // The impostor's chain is then complete under the impostor's key, in
    // both, which shows the refusal above was about the master and nothing
    // else.
    assert_eq!(
        assert_agree(&bytes_of(&cert), &impostor.verifying_key()),
        Ok(INFO.to_string())
    );
}

#[test]
fn wrong_rsa_signature_is_refused_by_both() {
    let c = chain(INFO);
    let master = c.master.verifying_key();
    let ghost_bytes = Armorable::to_bytes(&c.cert.verifying_key).unwrap();

    // Signed by a different notary key.
    let other = chain(INFO);
    let mut cert = c.cert.clone();
    cert.signature = unblinded_rsa_sign(&other.notary_keypair, &ghost_bytes).unwrap();
    assert_eq!(
        assert_agree(&bytes_of(&cert), &master),
        Err(Error::GhostKeySignature)
    );

    // The right notary, the wrong message.
    let mut cert = c.cert.clone();
    let wrong_key = SigningKey::generate(&mut OsRng).verifying_key();
    cert.signature =
        unblinded_rsa_sign(&c.notary_keypair, &Armorable::to_bytes(&wrong_key).unwrap()).unwrap();
    assert_eq!(
        assert_agree(&bytes_of(&cert), &master),
        Err(Error::GhostKeySignature)
    );

    // Every single-bit flip of the genuine signature, at a spread of
    // positions.
    let genuine = c.cert.signature.0.clone();
    for byte in (0..genuine.len()).step_by(17).chain([genuine.len() - 1]) {
        for bit in [0, 7] {
            let mut sig = genuine.clone();
            sig[byte] ^= 1 << bit;
            let mut cert = c.cert.clone();
            cert.signature = RsaSignature(sig);
            assert_eq!(
                assert_agree(&bytes_of(&cert), &master),
                Err(Error::GhostKeySignature),
                "flip at byte {byte} bit {bit}"
            );
        }
    }

    // Wrong lengths: shorter, longer, empty.
    for sig in [
        genuine[1..].to_vec(),
        [&[0u8][..], &genuine].concat(),
        Vec::new(),
    ] {
        let mut cert = c.cert.clone();
        cert.signature = RsaSignature(sig);
        assert_eq!(
            assert_agree(&bytes_of(&cert), &master),
            Err(Error::GhostKeySignature)
        );
    }
}

#[test]
fn ghost_key_swapped_is_refused_by_both() {
    let c = chain(INFO);
    let mut cert = c.cert.clone();
    cert.verifying_key = SigningKey::generate(&mut OsRng).verifying_key();
    assert_eq!(
        assert_agree(&bytes_of(&cert), &c.master.verifying_key()),
        Err(Error::GhostKeySignature)
    );
}

#[test]
fn random_mutations_agree() {
    // Byte flips, truncations, insertions and deletions over the whole
    // encoding: structure, field names, digits, signatures. Seeded, so a
    // failure reproduces; the chains themselves are fresh each run, and the
    // property must hold for any chain.
    let mut rng = StdRng::seed_from_u64(0x6b65_7976);
    let mut accepted = 0;
    let mut refused = 0;
    for _ in 0..3 {
        let c = chain(INFO);
        let master = c.master.verifying_key();
        let genuine = bytes_of(&c.cert);
        for _ in 0..400 {
            let mut bytes = genuine.clone();
            match rng.gen_range(0..4) {
                0 => {
                    let i = rng.gen_range(0..bytes.len());
                    bytes[i] ^= rng.gen_range(1..=255u8);
                }
                1 => bytes.truncate(rng.gen_range(0..bytes.len())),
                2 => {
                    let i = rng.gen_range(0..=bytes.len());
                    bytes.insert(i, rng.gen());
                }
                _ => {
                    bytes.remove(rng.gen_range(0..bytes.len()));
                }
            }
            match assert_agree(&bytes, &master) {
                Ok(_) => accepted += 1,
                Err(_) => refused += 1,
            }
        }
    }
    // Trailing bytes after the CBOR item are ignored by both, so a few
    // mutations are accepted; nearly all must be refused.
    assert!(refused > 1000, "only {refused} mutations refused");
    assert!(accepted > 0, "no mutation exercised the accepting path");
}

#[test]
fn weird_but_equivalent_encodings_agree() {
    // Inputs that are not what ghostkey_lib writes but that its decoder
    // accepts: structs as CBOR arrays, unknown map keys, and a modulus with
    // high zero digits. Each must re-encode to the genuine bytes and verify
    // in both, or neither.
    use ciborium::Value;
    let c = chain(INFO);
    let master = c.master.verifying_key();
    let genuine: Value = ciborium::de::from_reader(bytes_of(&c.cert).as_slice()).unwrap();

    let encode = |v: &Value| {
        let mut out = Vec::new();
        ciborium::ser::into_writer(v, &mut out).unwrap();
        out
    };
    // Unknown key at the top level.
    let mut v = genuine.clone();
    if let Value::Map(m) = &mut v {
        m.push((Value::Text("extra".into()), Value::Integer(7.into())));
    }
    assert_eq!(assert_agree(&encode(&v), &master), Ok(INFO.to_string()));

    // Keys reordered.
    let mut v = genuine.clone();
    if let Value::Map(m) = &mut v {
        m.reverse();
    }
    assert_eq!(assert_agree(&encode(&v), &master), Ok(INFO.to_string()));

    // The modulus with two zero high digits appended: decoders normalize it
    // away, so the master's signature still matches the re-encoding.
    let mut v = genuine.clone();
    let n = field(
        field(
            field(field(&mut v, "delegate"), "payload"),
            "delegate_verifying_key",
        ),
        "n",
    );
    let Value::Array(n) = n else {
        panic!("n is not an array")
    };
    n.push(Value::Integer(0.into()));
    n.push(Value::Integer(0.into()));
    assert_eq!(assert_agree(&encode(&v), &master), Ok(INFO.to_string()));

    // The notary certificate as an array instead of a map: ciborium refuses
    // a struct in that form, in both.
    let mut v = genuine.clone();
    let delegate = field(&mut v, "delegate");
    let Value::Map(m) = delegate.clone() else {
        panic!("not a map")
    };
    *delegate = Value::Array(m.into_iter().map(|(_, v)| v).collect());
    assert_eq!(assert_agree(&encode(&v), &master), Err(Error::Malformed));

    // A duplicate key, which serde refuses.
    let mut v = genuine.clone();
    if let Value::Map(m) = &mut v {
        let dup = m[0].clone();
        m.push(dup);
    }
    assert_eq!(assert_agree(&encode(&v), &master), Err(Error::Malformed));

    // The ghost key as an array of byte values rather than a byte string.
    let mut v = genuine.clone();
    let vk = field(&mut v, "verifying_key");
    let Value::Bytes(b) = vk.clone() else {
        panic!("not bytes")
    };
    *vk = Value::Array(b.into_iter().map(|x| Value::Integer(x.into())).collect());
    // dalek's decoder takes either; both re-encode it as a byte string,
    // which is what the notary signed.
    assert_eq!(assert_agree(&encode(&v), &master), Ok(INFO.to_string()));

    // Trailing garbage after the item: both ignore it.
    let mut bytes = encode(&genuine);
    bytes.extend_from_slice(b"trailing");
    assert_eq!(assert_agree(&bytes, &master), Ok(INFO.to_string()));
}

/// The value under `name` in a CBOR map.
fn field<'a>(v: &'a mut ciborium::Value, name: &str) -> &'a mut ciborium::Value {
    let ciborium::Value::Map(m) = v else {
        panic!("not a map")
    };
    &mut m
        .iter_mut()
        .find(|(k, _)| k.as_text() == Some(name))
        .unwrap_or_else(|| panic!("no field {name}"))
        .1
}

// ---------------------------------------------------------------------------
// The chain: armour
// ---------------------------------------------------------------------------

#[test]
fn armour_variants_agree() {
    let c = chain(INFO);
    let master = c.master.verifying_key();
    let pem = c.cert.to_armored_string().unwrap();
    let body: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();

    let variants = [
        pem.clone(),
        format!("Here is my key:\n\n{pem}\nthanks"),
        pem.replace('\n', "\r\n"),
        pem.replace("GHOSTKEY_CERTIFICATE_V1", "GHOSTKEY_CERTIFICATE"),
        pem.replace("-----END GHOSTKEY_CERTIFICATE_V1-----\n", ""),
        format!("-----BEGIN GHOSTKEY_CERTIFICATE_V1-----{body}-----END GHOSTKEY_CERTIFICATE_V1-----"),
        format!("-----BEGIN GHOSTKEY_CERTIFICATE_V1-----\n{body}"),
        pem.replace('\n', "\n  "),
        pem.replace("GHOSTKEY_CERTIFICATE_V1", "DELEGATE_CERTIFICATE_V1"),
        pem.replace("GHOSTKEY_CERTIFICATE_V1", "GHOSTKEY_CERTIFICATE_V2"),
        format!("{pem}{pem}"),
        pem.replacen(&body[..10], &body[..9], 1),
        pem.replace('=', ""),
        body.clone(),
        String::new(),
        // A broken versioned block followed by a good unversioned one: the
        // second label is tried.
        format!(
            "-----BEGIN GHOSTKEY_CERTIFICATE_V1-----\nAAAA\n-----END GHOSTKEY_CERTIFICATE_V1-----\n{}",
            pem.replace("GHOSTKEY_CERTIFICATE_V1", "GHOSTKEY_CERTIFICATE")
        ),
    ];
    let mut accepted = 0;
    for (i, text) in variants.iter().enumerate() {
        let r = GhostkeyCertificateV1::from_armored_string(text)
            .ok()
            .and_then(|c| c.verify(&Some(master)).ok());
        let o = GhostkeyCertificate::from_armored(text)
            .ok()
            .and_then(|c| c.verify(&master).ok())
            .map(|v| v.info);
        assert_eq!(r, o, "variant {i}: {text:?}");
        if o.is_some() {
            accepted += 1;
            let canon = GhostkeyCertificate::from_armored(text)
                .unwrap()
                .to_armored();
            assert_eq!(canon, pem, "variant {i} canonicalizes");
        }
    }
    assert!(
        (5..variants.len() - 3).contains(&accepted),
        "{accepted} accepted"
    );
}

// ---------------------------------------------------------------------------
// The chain: recorded certificates
// ---------------------------------------------------------------------------

#[test]
fn ghostkey_lib_legacy_fixtures_agree() {
    let dir = fixture("../gklib/tests/fixtures/legacy_v1");
    let master = VerifyingKey::from_file(&dir.join("master_verifying_key.pem")).unwrap();
    let pem = std::fs::read_to_string(dir.join("ghost_key_certificate.pem")).unwrap();

    let reference = GhostkeyCertificateV1::from_armored_string(&pem)
        .unwrap()
        .verify(&Some(master))
        .unwrap();
    let cert = GhostkeyCertificate::from_armored(&pem).unwrap();
    let verified = cert.verify(&master).unwrap();
    assert_eq!(verified.info, reference);
    assert_eq!(verified.info, "donation_amount:20");
    // The legacy format has no creation date.
    assert_eq!(verified.notary_info(), Err(Error::Info));
    assert_eq!(cert.to_armored(), pem);
    assert_eq!(
        cert.verify(&production_master()),
        Err(Error::NotarySignature)
    );

    // The signed message carries a certificate and a signature by its ghost
    // key, made by ghostkey_lib's signing path.
    let signed = SignedMessage::from_file(&dir.join("signed_message.bin")).unwrap();
    let cert =
        GhostkeyCertificate::from_cbor(&Armorable::to_bytes(&signed.certificate).unwrap()).unwrap();
    let verified = cert.verify(&master).unwrap();
    let sig = signed.signature.to_bytes();
    assert!(signed
        .certificate
        .verifying_key
        .verify_strict(&signed.message, &signed.signature)
        .is_ok());
    assert_eq!(
        verified.verify_payload_signature(&signed.message, &sig),
        Ok(())
    );
    let mut other = signed.message.clone();
    other.push(0);
    assert_eq!(
        verified.verify_payload_signature(&other, &sig),
        Err(Error::PayloadSignature)
    );
}

#[test]
fn production_certificate_agrees() {
    // A genuine certificate under Freenet's production master key: the test
    // Ghost Key used in end-to-end walk-throughs. Public, like every
    // certificate; its signing key is not in this repository.
    let pem = std::fs::read_to_string(fixture(
        "tests/fixtures/production_ghostkey_certificate.pem",
    ))
    .unwrap();
    let reference = GhostkeyCertificateV1::from_armored_string(&pem)
        .unwrap()
        .verify(&None)
        .unwrap();
    let cert = GhostkeyCertificate::from_armored(&pem).unwrap();
    let verified = cert.verify(&production_master()).unwrap();
    assert_eq!(verified.info, reference);
    let info = NotaryInfo::parse(&verified.info).unwrap();
    assert_eq!(info.action, "freenet-donation");
    assert_eq!(cert.to_armored(), pem);

    let wrong = SigningKey::generate(&mut OsRng).verifying_key();
    assert_eq!(cert.verify(&wrong), Err(Error::NotarySignature));
}

#[test]
fn master_key_matches_ghostkey_lib() {
    let lib = VerifyingKey::from_base64(ghostkey_lib::FREENET_MASTER_VERIFYING_KEY_BASE64).unwrap();
    assert_eq!(lib.as_bytes(), &FREENET_MASTER_VERIFYING_KEY);
    assert_eq!(production_master(), lib);
}

// ---------------------------------------------------------------------------
// Payload signatures (ScopedPayload)
// ---------------------------------------------------------------------------

/// The reference: what freenet-bitcoin's inbox does with an entry's
/// `scoped_payload` and `signature`.
fn reference_payload(key: &VerifyingKey, payload: &[u8], sig: &[u8]) -> bool {
    let Ok(sig): Result<[u8; 64], _> = sig.try_into() else {
        return false;
    };
    key.verify_strict(payload, &ed25519_dalek::Signature::from_bytes(&sig))
        .is_ok()
}

fn assert_payload_agree(key: &VerifyingKey, payload: &[u8], sig: &[u8]) -> Result<(), Error> {
    let o = verify_payload_signature(key, payload, sig);
    assert_eq!(reference_payload(key, payload, sig), o.is_ok(), "{o:?}");
    o
}

#[test]
fn payload_signatures_agree() {
    let c = chain(INFO);
    let verified = GhostkeyCertificate::from_cbor(&bytes_of(&c.cert))
        .unwrap()
        .verify(&c.master.verifying_key())
        .unwrap();
    let key = verified.ghost_key;
    // Shaped like a ScopedPayload; the bytes are opaque to the check.
    let payload =
        b"\xa2irequestor\xa1fWebApp\x58\x20AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgpayload\x43abc";
    let sig = c.ghost.sign(payload).to_bytes();

    // Valid.
    assert_eq!(assert_payload_agree(&key, payload, &sig), Ok(()));
    assert_eq!(verified.verify_payload_signature(payload, &sig), Ok(()));

    // Wrong signer.
    let other = SigningKey::generate(&mut OsRng).sign(payload).to_bytes();
    assert_eq!(
        assert_payload_agree(&key, payload, &other),
        Err(Error::PayloadSignature)
    );

    // Wrong payload: changed, truncated, extended, empty.
    for p in [
        &b"\xa2irequestor\xa1fWebApp\x58\x20AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABgpayload\x43abc"[..],
        &payload[..payload.len() - 1],
        &[&payload[..], b"x"].concat(),
        b"",
    ] {
        assert_eq!(
            assert_payload_agree(&key, p, &sig),
            Err(Error::PayloadSignature)
        );
    }

    // Every byte of the signature flipped.
    for i in 0..64 {
        let mut s = sig;
        s[i] ^= 0x01;
        assert_eq!(
            assert_payload_agree(&key, payload, &s),
            Err(Error::PayloadSignature),
            "flip at {i}"
        );
    }

    // Malleated: S + l, which non-canonical-S checks exist to refuse.
    let mut s = sig;
    let l: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde,
        0x14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
    ];
    let mut carry = 0u16;
    for i in 0..32 {
        let v = u16::from(s[32 + i]) + u16::from(l[i]) + carry;
        s[32 + i] = v as u8;
        carry = v >> 8;
    }
    assert_eq!(
        assert_payload_agree(&key, payload, &s),
        Err(Error::PayloadSignature)
    );

    // Wrong length.
    for s in [&sig[..63], &[&sig[..], &[0]].concat()[..], &[][..]] {
        assert!(!reference_payload(&key, payload, s));
        assert_eq!(
            verify_payload_signature(&key, payload, s),
            Err(Error::PayloadSignatureLength)
        );
    }
}

#[test]
fn payload_signature_under_a_weak_key_is_refused() {
    // The identity point: a key "anyone can sign for". Strict verification
    // refuses every signature under it.
    let mut identity = [0u8; 32];
    identity[0] = 1;
    let weak = VerifyingKey::from_bytes(&identity).unwrap();
    assert!(weak.is_weak());
    let mut sig = [0u8; 64];
    sig[0] = 1; // R = identity, S = 0: valid under the cofactorless equation.
    assert_eq!(
        assert_payload_agree(&weak, b"anything", &sig),
        Err(Error::PayloadSignature)
    );
}

// ---------------------------------------------------------------------------
// Intentional differences: ghostkey_lib accepts, this crate refuses.
// ---------------------------------------------------------------------------

#[test]
fn difference_weak_ghost_key_certificate() {
    let c = chain(INFO);
    let master = c.master.verifying_key();
    let mut identity = [0u8; 32];
    identity[0] = 1;
    let weak = VerifyingKey::from_bytes(&identity).unwrap();
    let mut cert = c.cert.clone();
    cert.verifying_key = weak;
    cert.signature =
        unblinded_rsa_sign(&c.notary_keypair, &Armorable::to_bytes(&weak).unwrap()).unwrap();
    let bytes = bytes_of(&cert);
    assert_eq!(reference(&bytes, &master), Ok(INFO.to_string()));
    assert_eq!(ours(&bytes, &master), Err(Error::WeakGhostKey));
}

#[test]
fn difference_rsa_signature_not_reduced() {
    // s + n, where it still fits the modulus length, is the same signature to
    // a verifier that reduces modulo n. RFC 8017 says to refuse it. Whether
    // s + n fits depends on the key and the signature, so this searches a few
    // keys and many signatures; for one key it fails only if none of 64
    // signatures fits, and that needs a modulus within a few percent of
    // 2^2048, for all four keys.
    use rsa_08::{BigUint, PublicKeyParts};
    for _ in 0..4 {
        let c = chain(INFO);
        let master = c.master.verifying_key();
        let n = c.notary_keypair.pk.0.n().clone();
        let size = c.notary_keypair.pk.0.size();
        for _ in 0..64 {
            let ghost = SigningKey::generate(&mut OsRng).verifying_key();
            let sig = unblinded_rsa_sign(&c.notary_keypair, &Armorable::to_bytes(&ghost).unwrap())
                .unwrap();
            let bigger = (BigUint::from_bytes_be(&sig.0) + &n).to_bytes_be();
            if bigger.len() != size {
                continue;
            }
            let mut cert = c.cert.clone();
            cert.verifying_key = ghost;
            cert.signature = sig;
            assert_eq!(ours(&bytes_of(&cert), &master), Ok(INFO.to_string()));
            cert.signature = RsaSignature(bigger);
            let bytes = bytes_of(&cert);
            assert_eq!(reference(&bytes, &master), Ok(INFO.to_string()));
            assert_eq!(ours(&bytes, &master), Err(Error::GhostKeySignature));
            return;
        }
    }
    panic!("no signature had room above its modulus");
}
