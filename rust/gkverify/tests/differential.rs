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
    cert.verify(master).map(|v| v.info().to_string())
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
    // Whatever this crate parses, its canonical forms parse again, to
    // themselves.
    if let Ok(cert) = GhostkeyCertificate::from_cbor(bytes) {
        let pem = cert.to_armored();
        let again = GhostkeyCertificate::from_armored(&pem).expect("canonical armour parses");
        assert_eq!(again.to_armored(), pem);
        let cbor = GhostkeyCertificate::from_cbor(&cert.to_cbor()).expect("canonical CBOR parses");
        assert_eq!(cbor.to_cbor(), cert.to_cbor());
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
    assert_eq!(verified.ghost_key(), &c.ghost.verifying_key());
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
    // Fresh RSA keys each time, so the moduli and signatures vary. A
    // signature with a leading zero byte turns up in a few percent of runs;
    // `wrong_rsa_signature_is_refused_by_both` covers the length rule
    // deterministically.
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

    // The RSA signature, and the master's Ed25519 signature, as CBOR byte
    // strings rather than arrays: ciborium hands both decoders the bytes one
    // by one, and both re-encode them as arrays.
    let mut v = genuine.clone();
    let sig = field(&mut v, "signature");
    let Value::Array(items) = sig.clone() else {
        panic!("not an array")
    };
    *sig = Value::Bytes(
        items
            .iter()
            .map(|i| u8::try_from(i.as_integer().unwrap()).unwrap())
            .collect(),
    );
    assert_eq!(assert_agree(&encode(&v), &master), Ok(INFO.to_string()));
    let master_sig = field(field(&mut v, "delegate"), "signature");
    let Value::Array(items) = master_sig.clone() else {
        panic!("not an array")
    };
    *master_sig = Value::Bytes(
        items
            .iter()
            .map(|i| u8::try_from(i.as_integer().unwrap()).unwrap())
            .collect(),
    );
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

/// A certificate whose notary key is `sk`, with the ghost key signed under
/// RSA-PSS SHA-384 with salt length `salt`, as no ghostkey_lib code path will
/// make it: for keys and salts the issuing code never uses.
fn chain_with_notary_key(sk: &rsa_08::RsaPrivateKey, salt: usize) -> (Vec<u8>, VerifyingKey) {
    use ghostkey_lib::notary_certificate::NotaryPayload;
    use rsa_08::signature::{RandomizedSigner, SignatureEncoding};
    use rsa_08::PublicKeyParts;
    let master = SigningKey::generate(&mut OsRng);
    let payload = NotaryPayload {
        notary_verifying_key: blind_rsa_signatures::PublicKey(sk.to_public_key()),
        info: INFO.to_string(),
    };
    let notary = NotaryCertificateV1 {
        signature: master.sign(&payload.to_bytes().unwrap()),
        payload,
    };
    let ghost = SigningKey::generate(&mut OsRng).verifying_key();
    let msg = Armorable::to_bytes(&ghost).unwrap();
    let signer =
        rsa_08::pss::SigningKey::<rsa_08::sha2::Sha384>::new_with_salt_len(sk.clone(), salt);
    let sig = signer.sign_with_rng(&mut OsRng, &msg).to_vec();
    // Left-pad to the modulus length, as blind-rsa-signatures does.
    let mut padded = vec![0u8; sk.size() - sig.len()];
    padded.extend_from_slice(&sig);
    let cert = GhostkeyCertificateV1 {
        notary,
        verifying_key: ghost,
        signature: RsaSignature(padded),
    };
    (bytes_of(&cert), master.verifying_key())
}

#[test]
fn any_pss_salt_length_agrees() {
    // blind-rsa-signatures verifies with the salt length detected from the
    // signature, whatever its options say; so does this crate. Every
    // certificate issued so far uses 48.
    let c = chain(INFO);
    let sk = c.notary_keypair.sk.0.clone();
    for salt in [0, 1, 20, 32, 48, 64, 206] {
        let (bytes, master) = chain_with_notary_key(&sk, salt);
        assert_eq!(
            assert_agree(&bytes, &master),
            Ok(INFO.to_string()),
            "salt {salt}"
        );
    }
}

#[test]
fn notary_key_shapes_within_the_limits_agree() {
    // Modulus lengths that are not a multiple of 8 bits exercise the
    // encoded-message length arithmetic; e = 3 is the other exponent
    // blind-rsa-signatures signs with.
    use rsa_08::BigUint;
    let keys = [
        rsa_08::RsaPrivateKey::new(&mut OsRng, 2049).unwrap(),
        rsa_08::RsaPrivateKey::new(&mut OsRng, 2050).unwrap(),
        rsa_08::RsaPrivateKey::new(&mut OsRng, 3072).unwrap(),
        rsa_08::RsaPrivateKey::new_with_exp(&mut OsRng, 2048, &BigUint::from(3u32)).unwrap(),
    ];
    for (i, sk) in keys.iter().enumerate() {
        let (bytes, master) = chain_with_notary_key(sk, 48);
        assert_eq!(
            assert_agree(&bytes, &master),
            Ok(INFO.to_string()),
            "key {i}"
        );
    }
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

    let padded = body.contains('=');
    // (text, whether ghostkey_lib accepts it), each pinned so a change in
    // either parser shows up as the variant that moved.
    let variants: Vec<(String, bool)> = vec![
        (pem.clone(), true),
        (format!("Here is my key:\n\n{pem}\nthanks"), true),
        (pem.replace('\n', "\r\n"), true),
        (pem.replace("GHOSTKEY_CERTIFICATE_V1", "GHOSTKEY_CERTIFICATE"), true),
        (pem.replace("-----END GHOSTKEY_CERTIFICATE_V1-----\n", ""), true),
        (
            format!("-----BEGIN GHOSTKEY_CERTIFICATE_V1-----{body}-----END GHOSTKEY_CERTIFICATE_V1-----"),
            true,
        ),
        (format!("-----BEGIN GHOSTKEY_CERTIFICATE_V1-----\n{body}"), true),
        // Only the first block after the first BEGIN is read.
        (format!("{pem}{pem}"), true),
        // A broken versioned block, then a good unversioned one: the second
        // label is tried.
        (
            format!(
                "-----BEGIN GHOSTKEY_CERTIFICATE_V1-----\nAAAA\n-----END GHOSTKEY_CERTIFICATE_V1-----\n{}",
                pem.replace("GHOSTKEY_CERTIFICATE_V1", "GHOSTKEY_CERTIFICATE")
            ),
            true,
        ),
        // Indented lines keep their spaces, which base64 refuses.
        (pem.replace('\n', "\n  "), false),
        (pem.replace("GHOSTKEY_CERTIFICATE_V1", "DELEGATE_CERTIFICATE_V1"), false),
        (pem.replace("GHOSTKEY_CERTIFICATE_V1", "GHOSTKEY_CERTIFICATE_V2"), false),
        // One base64 character dropped.
        (pem.replacen(&body[..10], &body[..9], 1), false),
        // Padding is required; a body that needs none is unchanged by this.
        (pem.replace('=', ""), !padded),
        (body.clone(), false),
        (String::new(), false),
    ];
    for (i, (text, accepted)) in variants.iter().enumerate() {
        let r = GhostkeyCertificateV1::from_armored_string(text)
            .ok()
            .and_then(|c| c.verify(&Some(master)).ok());
        let o = GhostkeyCertificate::from_armored(text)
            .ok()
            .and_then(|c| c.verify(&master).ok())
            .map(|v| v.info().to_string());
        assert_eq!(r, o, "variant {i}: {text:?}");
        assert_eq!(o.is_some(), *accepted, "variant {i}: {text:?}");
        if o.is_some() {
            let canon = GhostkeyCertificate::from_armored(text)
                .unwrap()
                .to_armored();
            assert_eq!(canon, pem, "variant {i} canonicalizes");
        }
    }
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
    assert_eq!(verified.info(), reference);
    assert_eq!(verified.info(), "donation_amount:20");
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
    assert_eq!(verified.info(), reference);
    let info = NotaryInfo::parse(verified.info()).unwrap();
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
    let key = *verified.ghost_key();
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

#[test]
fn difference_oversized_input() {
    // ghostkey_lib parses any amount of text around the armour, and ignores
    // any bytes after the CBOR item; this crate stops at MAX_CERTIFICATE_BYTES.
    use ghostkey_verify::MAX_CERTIFICATE_BYTES;
    let c = chain(INFO);
    let master = c.master.verifying_key();
    let pem = c.cert.to_armored_string().unwrap();
    let cbor = bytes_of(&c.cert);

    let fits = format!("{pem}{}", " ".repeat(MAX_CERTIFICATE_BYTES - pem.len()));
    assert_eq!(fits.len(), MAX_CERTIFICATE_BYTES);
    assert!(GhostkeyCertificate::from_armored(&fits).is_ok());
    let over = format!("{fits} ");
    assert!(GhostkeyCertificateV1::from_armored_string(&over)
        .unwrap()
        .verify(&Some(master))
        .is_ok());
    assert_eq!(
        GhostkeyCertificate::from_armored(&over).err(),
        Some(Error::TooLarge)
    );

    let mut padded = cbor.clone();
    padded.resize(MAX_CERTIFICATE_BYTES, 0);
    assert_eq!(ours(&padded, &master), Ok(INFO.to_string()));
    padded.push(0);
    assert_eq!(reference(&padded, &master), Ok(INFO.to_string()));
    assert_eq!(ours(&padded, &master), Err(Error::TooLarge));
}

#[test]
fn difference_weak_master_key() {
    // Under a small-order master key, the non-strict Ed25519 check that
    // ghostkey_lib (and this crate, for parity) applies to the notary
    // certificate accepts a forged signature: R = identity, S = 0. A notary
    // certificate anyone can make chains to it.
    let mut identity = [0u8; 32];
    identity[0] = 1;
    let weak = VerifyingKey::from_bytes(&identity).unwrap();
    let c = chain(INFO);
    let mut cert = c.cert.clone();
    let mut forged = [0u8; 64];
    forged[0] = 1;
    cert.notary.signature = ed25519_dalek::Signature::from_bytes(&forged);
    let bytes = bytes_of(&cert);
    assert_eq!(reference(&bytes, &weak), Ok(INFO.to_string()));
    assert_eq!(ours(&bytes, &weak), Err(Error::WeakMasterKey));
}

#[test]
fn difference_notary_key_outside_the_limits() {
    // ghostkey_lib verifies under any key the master signed. This crate
    // takes only what blind-rsa-signatures would issue under: 2048 to 4096
    // bits, e = 3 or 65537.
    use rsa_08::BigUint;
    let keys = [
        rsa_08::RsaPrivateKey::new(&mut OsRng, 1024).unwrap(),
        rsa_08::RsaPrivateKey::new(&mut OsRng, 2047).unwrap(),
        rsa_08::RsaPrivateKey::new_with_exp(&mut OsRng, 2048, &BigUint::from(5u32)).unwrap(),
    ];
    for (i, sk) in keys.iter().enumerate() {
        let (bytes, master) = chain_with_notary_key(sk, 48);
        assert_eq!(reference(&bytes, &master), Ok(INFO.to_string()), "key {i}");
        assert_eq!(ours(&bytes, &master), Err(Error::NotaryKey), "key {i}");
    }
}

#[test]
fn cheap_refusals_come_before_the_chain() {
    // A weak ghost key is refused before any signature is looked at, and a
    // weak master before that: with every signature broken, those are still
    // the errors. Moving either check after the chain fails this test.
    let mut identity = [0u8; 32];
    identity[0] = 1;
    let weak = VerifyingKey::from_bytes(&identity).unwrap();
    let c = chain(INFO);
    let master = c.master.verifying_key();
    let mut cert = c.cert.clone();
    cert.verifying_key = weak;
    cert.notary.payload.info = "tampered".into();
    cert.signature = RsaSignature(vec![1; 256]);
    let bytes = bytes_of(&cert);
    assert_eq!(ours(&bytes, &master), Err(Error::WeakGhostKey));
    assert_eq!(ours(&bytes, &weak), Err(Error::WeakMasterKey));
    assert!(reference(&bytes, &master).is_err());
}

#[test]
fn largest_notary_key_fits_the_size_limit() {
    // A 4096-bit notary key is the largest accepted; its certificate, with a
    // genuine-length info, must fit
    // MAX_CERTIFICATE_BYTES in both forms, or the two limits contradict each
    // other. The RSA signature is the variable part: CBOR spends two bytes on
    // each signature byte of 24 or more, so the bound is checked against the
    // worst case too, not only this signature.
    use ghostkey_verify::MAX_CERTIFICATE_BYTES;
    let sk = rsa_08::RsaPrivateKey::new(&mut OsRng, 4096).unwrap();
    let (bytes, master) = chain_with_notary_key(&sk, 48);
    assert_eq!(assert_agree(&bytes, &master), Ok(INFO.to_string()));
    let cert = GhostkeyCertificate::from_cbor(&bytes).unwrap();
    let pem = cert.to_armored();
    assert!(pem.len() <= MAX_CERTIFICATE_BYTES, "{} bytes", pem.len());
    assert_eq!(
        GhostkeyCertificate::from_armored(&pem).unwrap().to_cbor(),
        bytes
    );

    let mut worst = GhostkeyCertificateV1::from_bytes(&bytes).unwrap();
    worst.signature = RsaSignature(vec![0xff; 512]);
    let worst = GhostkeyCertificate::from_cbor(&bytes_of(&worst)).unwrap();
    let worst_pem = worst.to_armored();
    assert!(
        worst_pem.len() <= MAX_CERTIFICATE_BYTES,
        "worst case {} bytes",
        worst_pem.len()
    );
}

/// A certificate whose RSA signature is a CBOR byte string: half the size of
/// its canonical array, so it fits MAX_CERTIFICATE_BYTES as sent and not once
/// re-encoded. Lean otherwise (a one-digit modulus: it only has to parse), so
/// a genuine certificate fits beside it in one text.
fn oversized_decoy(c: &Chain) -> Vec<u8> {
    use ciborium::Value;
    let mut v: Value = ciborium::de::from_reader(bytes_of(&c.cert).as_slice()).unwrap();
    *field(
        field(
            field(field(&mut v, "delegate"), "payload"),
            "delegate_verifying_key",
        ),
        "n",
    ) = Value::Array(vec![Value::Integer(1.into())]);
    *field(field(field(&mut v, "delegate"), "payload"), "info") = Value::Text("x".into());
    *field(&mut v, "signature") = Value::Bytes(vec![0xff; 1500]);
    let mut cbor = Vec::new();
    ciborium::ser::into_writer(&v, &mut cbor).unwrap();
    cbor
}

fn armour(cbor: &[u8]) -> String {
    use base64::Engine;
    format!(
        "-----BEGIN GHOSTKEY_CERTIFICATE_V1-----\n{}\n-----END GHOSTKEY_CERTIFICATE_V1-----\n",
        base64::engine::general_purpose::STANDARD.encode(cbor)
    )
}

#[test]
fn difference_canonical_form_over_the_size_limit() {
    // ghostkey_lib parses the decoy (and refuses the chain); this crate
    // refuses it at parse, so every certificate it accepts has a canonical
    // form it can read back.
    use ghostkey_verify::MAX_CERTIFICATE_BYTES;
    let c = chain(INFO);
    let cbor = oversized_decoy(&c);
    assert!(cbor.len() <= MAX_CERTIFICATE_BYTES);
    assert!(GhostkeyCertificateV1::from_bytes(&cbor).is_ok());
    assert_eq!(
        GhostkeyCertificate::from_cbor(&cbor).err(),
        Some(Error::TooLarge)
    );
    let pem = armour(&cbor);
    assert!(pem.len() <= MAX_CERTIFICATE_BYTES);
    assert!(GhostkeyCertificateV1::from_armored_string(&pem).is_ok());
    assert_eq!(
        GhostkeyCertificate::from_armored(&pem).err(),
        Some(Error::TooLarge)
    );
}

#[test]
fn difference_oversized_block_stops_the_armour_search() {
    // The decoy followed by a genuine certificate under the unversioned
    // label: the search stops at the decoy, where ghostkey_lib also stops,
    // rather than going on to accept the second.
    use ghostkey_verify::MAX_CERTIFICATE_BYTES;
    let c = chain(INFO);
    let genuine = c
        .cert
        .to_armored_string()
        .unwrap()
        .replace("GHOSTKEY_CERTIFICATE_V1", "GHOSTKEY_CERTIFICATE");
    assert!(GhostkeyCertificate::from_armored(&genuine).is_ok());
    let both = format!("{}{genuine}", armour(&oversized_decoy(&c)));
    assert!(both.len() <= MAX_CERTIFICATE_BYTES, "{} bytes", both.len());
    assert!(GhostkeyCertificateV1::from_armored_string(&both)
        .unwrap()
        .verify(&Some(c.master.verifying_key()))
        .is_err());
    assert_eq!(
        GhostkeyCertificate::from_armored(&both).err(),
        Some(Error::TooLarge)
    );
}

#[test]
fn canonical_bound_sits_exactly_at_the_limit() {
    // Sweep a byte-string RSA signature across the lengths where the input
    // fits and its canonical armour crosses MAX_CERTIFICATE_BYTES: the
    // certificate parses exactly when ghostkey_lib's own canonical armour of
    // it fits. Every parse that succeeds also goes through assert_agree, whose
    // canonical re-parse check would fail without the bound.
    use ciborium::Value;
    use ghostkey_verify::MAX_CERTIFICATE_BYTES;
    let c = chain(INFO);
    let master = c.master.verifying_key();
    let genuine: Value = ciborium::de::from_reader(bytes_of(&c.cert).as_slice()).unwrap();
    let (mut parsed, mut refused, mut at_limit) = (0, 0, false);
    // Bytes below 24 take one CBOR byte in an array, the rest two, so mixing
    // them steps the canonical length one byte at a time and lands on the
    // limit itself.
    for len in 900..1500 {
        for small in [0, len / 2, len] {
            let mut v = genuine.clone();
            let mut sig = vec![0x01; small];
            sig.resize(len, 0xff);
            *field(&mut v, "signature") = Value::Bytes(sig);
            let mut cbor = Vec::new();
            ciborium::ser::into_writer(&v, &mut cbor).unwrap();
            if cbor.len() > MAX_CERTIFICATE_BYTES {
                continue;
            }
            let canonical = GhostkeyCertificateV1::from_bytes(&cbor)
                .unwrap()
                .to_armored_string()
                .unwrap();
            let fits = canonical.len() <= MAX_CERTIFICATE_BYTES;
            at_limit |= canonical.len() == MAX_CERTIFICATE_BYTES;
            let ours = GhostkeyCertificate::from_cbor(&cbor);
            assert_eq!(ours.is_ok(), fits, "{len} bytes, {small} small");
            if fits {
                parsed += 1;
                let _ = assert_agree(&cbor, &master);
            } else {
                refused += 1;
                assert_eq!(ours.err(), Some(Error::TooLarge));
            }
        }
    }
    assert!(at_limit, "no case landed exactly on the limit");
    assert!(
        parsed > 10 && refused > 10,
        "{parsed} parsed, {refused} refused"
    );
}
