//! The notary's RSA keys, and the blind-signature types the issuing protocol
//! exchanges.
//!
//! The scheme is RSABSSA-SHA384-PSS-Deterministic (RFC 9474): SHA-384, PSS
//! with a 48-byte salt, and no message randomizer. That is what
//! `blind-rsa-signatures` 0.15 produced with `Options::default()` and
//! `randomize_msg = false`, which every certificate issued so far used.
//!
//! [`PublicKey`] and [`SecretKey`] wrap `blind-rsa-signatures`' key types
//! for one reason: their serialized form. Until 0.3.0 these keys were
//! `blind-rsa-signatures` 0.15 keys, which serialize through `rsa` 0.8's
//! `serde` support as `{n, e}` (and `{pubkey_components, d, primes}` for the
//! secret key), each number a `num-bigint-dig` `BigUint`. Later `rsa`
//! releases serialize keys differently. The notary public key sits inside
//! the payload the master key signs, so its bytes must not change, and the
//! notary signing keys already on disk must still load. These wrappers read
//! and write exactly the old form.
//!
//! The wrapper names are frozen too: `Armorable` derives the PEM label from
//! the type name, and notary signing key files are labelled
//! `SECRET_KEY_V1`.

use std::ops::Deref;

use blind_rsa_signatures::reexports::rsa;
use rsa::traits::{PrivateKeyParts, PublicKeyParts};
use rsa::BoxedUint;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// `blind-rsa-signatures`' own re-exports (`rand`, `rsa`, ...), for callers
/// that need the exact versions it uses.
pub use blind_rsa_signatures::reexports;
pub use blind_rsa_signatures::{
    BlindSignature, BlindingResult, DefaultRng, Deterministic, Error, MessageRandomizer, Secret,
    Sha384, Signature, PSS,
};

/// The blinded message a client sends to be signed (`BlindedMessage` in
/// `blind-rsa-signatures` 0.15; same serialized form).
pub use blind_rsa_signatures::BlindMessage;

/// `blind-rsa-signatures`' public key, with the scheme parameters fixed.
pub type RawPublicKey = blind_rsa_signatures::PublicKey<Sha384, PSS, Deterministic>;
/// `blind-rsa-signatures`' secret key, with the scheme parameters fixed.
pub type RawSecretKey = blind_rsa_signatures::SecretKey<Sha384, PSS, Deterministic>;

/// Modulus size of the notary keys this crate generates.
pub const NOTARY_KEY_BITS: usize = 2048;

/// A notary verifying key. Derefs to [`RawPublicKey`] for `blind`,
/// `finalize`, `verify` and `to_der`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicKey(pub RawPublicKey);

/// A notary signing key. Derefs to [`RawSecretKey`] for `blind_sign`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretKey(pub RawSecretKey);

impl Deref for PublicKey {
    type Target = RawPublicKey;
    fn deref(&self) -> &RawPublicKey {
        &self.0
    }
}

impl Deref for SecretKey {
    type Target = RawSecretKey;
    fn deref(&self) -> &RawSecretKey {
        &self.0
    }
}

impl SecretKey {
    /// Generate a new notary key pair of [`NOTARY_KEY_BITS`] bits.
    pub fn generate() -> Result<(PublicKey, SecretKey), Error> {
        let kp = blind_rsa_signatures::KeyPair::<Sha384, PSS, Deterministic>::generate(
            &mut DefaultRng,
            NOTARY_KEY_BITS,
        )?;
        Ok((PublicKey(kp.pk), SecretKey(kp.sk)))
    }

    /// The matching verifying key.
    pub fn public_key(&self) -> Result<PublicKey, Error> {
        self.0.public_key().map(PublicKey)
    }
}

impl PublicKey {
    /// Unblind `blind_sig` with the `secret` from [`RawPublicKey::blind`]
    /// and check the result, for callers that kept only the secret (the
    /// browser does: the blinded message went to the server).
    pub fn finalize_with_secret(
        &self,
        blind_sig: &BlindSignature,
        secret: Secret,
        msg: impl AsRef<[u8]>,
    ) -> Result<Signature, Error> {
        let result = BlindingResult {
            // Not read by `finalize`.
            blind_message: BlindMessage(Vec::new()),
            secret,
            msg_randomizer: None,
        };
        self.0.finalize(blind_sig, &result, msg)
    }

    /// Verify a finalized signature over `msg`: RSASSA-PSS with SHA-384.
    ///
    /// The salt length is read from the signature rather than fixed at 48,
    /// as `blind-rsa-signatures` 0.15 (and so ghostkey_lib 0.2 and
    /// `ghostkey-verify`) verified. 0.19's own `verify` insists on 48, which
    /// every issued certificate uses; keeping the old rule means this
    /// migration changes no verifier's verdict.
    pub fn verify_signature(&self, sig: &Signature, msg: impl AsRef<[u8]>) -> Result<(), Error> {
        use rsa::signature::Verifier as _;
        let key: &rsa::PublicKey = self.0.as_ref();
        if sig.len() != key.size() {
            return Err(Error::UnsupportedParameters);
        }
        let sig =
            rsa::pss::Signature::try_from(sig.as_ref()).map_err(|_| Error::VerificationFailed)?;
        rsa::pss::VerifyingKey::<rsa::sha2::Sha384>::new_with_auto_salt_len(key.clone())
            .verify(msg.as_ref(), &sig)
            .map_err(|_| Error::VerificationFailed)
    }
}

// --- serialized form -------------------------------------------------------

/// `rsa` 0.8's `RsaPublicKey`.
#[derive(Serialize, Deserialize)]
struct PublicKeyWire {
    n: BigUintWire,
    e: BigUintWire,
}

/// `rsa` 0.8's `RsaPrivateKey` (its precomputed values are not serialized).
#[derive(Serialize, Deserialize)]
struct SecretKeyWire {
    pubkey_components: PublicKeyWire,
    d: BigUintWire,
    primes: Vec<BigUintWire>,
}

impl Serialize for PublicKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let k: &rsa::PublicKey = self.0.as_ref();
        PublicKeyWire {
            n: BigUintWire::from_boxed(k.n().as_ref()),
            e: BigUintWire::from_boxed(k.e()),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let w = PublicKeyWire::deserialize(deserializer)?;
        // Up to rsa's largest modulus and down to any size: ghostkey_lib 0.2
        // decoded keys with no checks at all, and a verdict on an odd key is
        // the verifier's to give, not the decoder's. rsa still refuses an
        // even modulus or an exponent outside 2..2^33.
        let key = rsa::PublicKey::new_with_size_limits(
            w.n.to_boxed(),
            w.e.to_boxed(),
            1..=rsa::PublicKey::MAX_SIZE,
        )
        .map_err(serde::de::Error::custom)?;
        Ok(PublicKey(RawPublicKey::new(key)))
    }
}

impl Serialize for SecretKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let k: &rsa::PrivateKey = self.0.as_ref();
        SecretKeyWire {
            pubkey_components: PublicKeyWire {
                n: BigUintWire::from_boxed(k.n().as_ref()),
                e: BigUintWire::from_boxed(k.e()),
            },
            d: BigUintWire::from_boxed(k.d()),
            primes: k
                .primes()
                .iter()
                .map(|p| BigUintWire::from_boxed(p))
                .collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SecretKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let w = SecretKeyWire::deserialize(deserializer)?;
        // Validates the key (de = 1 mod p-1 for every prime, and that the
        // primes multiply to n) and precomputes the CRT values.
        let key = rsa::PrivateKey::from_components(
            w.pubkey_components.n.to_boxed(),
            w.pubkey_components.e.to_boxed(),
            w.d.to_boxed(),
            w.primes.iter().map(BigUintWire::to_boxed).collect(),
        )
        .map_err(serde::de::Error::custom)?;
        Ok(SecretKey(RawSecretKey::new(key)))
    }
}

/// A `num-bigint-dig` 0.8 `BigUint` as `rsa` 0.8 serializes it (it enables
/// the `u64_digit` feature).
///
/// Decodes from little-endian `u32` digits, packed pairwise into `u64`
/// digits with high zero digits stripped. Encodes every `u64` digit as two
/// `u32`s, except the most significant, which is one `u32` when it is below
/// `u32::MAX` (a top digit of exactly `u32::MAX` still takes two). The same
/// rule as `ghostkey-verify`'s `BigUintWire`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct BigUintWire {
    /// Little-endian `u64` digits, no high zero digit.
    digits: Vec<u64>,
}

impl BigUintWire {
    fn from_be_bytes(bytes: &[u8]) -> Self {
        let mut digits: Vec<u64> = bytes
            .rchunks(8)
            .map(|c| {
                let mut buf = [0u8; 8];
                buf[8 - c.len()..].copy_from_slice(c);
                u64::from_be_bytes(buf)
            })
            .collect();
        while digits.last() == Some(&0) {
            digits.pop();
        }
        Self { digits }
    }

    fn from_boxed(n: &BoxedUint) -> Self {
        Self::from_be_bytes(&n.to_be_bytes())
    }

    /// Big-endian bytes with no leading zero byte (empty for zero).
    fn to_be_bytes(&self) -> Vec<u8> {
        let mut out: Vec<u8> = self
            .digits
            .iter()
            .rev()
            .flat_map(|d| d.to_be_bytes())
            .collect();
        let first = out.iter().position(|&b| b != 0).unwrap_or(out.len());
        out.drain(..first);
        out
    }

    fn to_boxed(&self) -> BoxedUint {
        BoxedUint::from_be_slice_vartime(&self.to_be_bytes())
    }
}

impl Serialize for BigUintWire {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let last = self.digits.len().saturating_sub(1);
        let mut out: Vec<u32> = Vec::with_capacity(self.digits.len() * 2);
        for (i, &d) in self.digits.iter().enumerate() {
            out.push(d as u32);
            if !(i == last && d < u64::from(u32::MAX)) {
                out.push((d >> 32) as u32);
            }
        }
        out.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BigUintWire {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw: Vec<u32> = Vec::deserialize(deserializer)?;
        let mut digits: Vec<u64> = raw
            .chunks(2)
            .map(|c| match *c {
                [lo, hi] => u64::from(lo) | (u64::from(hi) << 32),
                [lo] => u64::from(lo),
                _ => unreachable!("chunks(2) yields one or two items"),
            })
            .collect();
        while digits.last() == Some(&0) {
            digits.pop();
        }
        Ok(Self { digits })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cbor<T: Serialize>(v: &T) -> Vec<u8> {
        let mut buf = Vec::new();
        ciborium::ser::into_writer(v, &mut buf).unwrap();
        buf
    }

    fn round_trip(raw: &[u32]) -> Vec<u32> {
        let w: BigUintWire = ciborium::de::from_reader(cbor(&raw.to_vec()).as_slice()).unwrap();
        ciborium::de::from_reader(cbor(&w).as_slice()).unwrap()
    }

    #[test]
    fn biguint_encoding_matches_num_bigint_dig_u64_digit() {
        assert_eq!(round_trip(&[]), Vec::<u32>::new());
        assert_eq!(round_trip(&[0]), Vec::<u32>::new());
        assert_eq!(round_trip(&[65537]), vec![65537]);
        assert_eq!(round_trip(&[65537, 0]), vec![65537]);
        assert_eq!(round_trip(&[u32::MAX - 1]), vec![u32::MAX - 1]);
        assert_eq!(round_trip(&[u32::MAX]), vec![u32::MAX, 0]);
        assert_eq!(round_trip(&[1, 1]), vec![1, 1]);
        assert_eq!(round_trip(&[1, 2, 3]), vec![1, 2, 3]);
        assert_eq!(round_trip(&[1, 0, 3, 0, 0, 0]), vec![1, 0, 3]);
        assert_eq!(round_trip(&[1, 2, 3, 4]), vec![1, 2, 3, 4]);
    }

    #[test]
    fn biguint_bytes_round_trip() {
        for bytes in [
            vec![],
            vec![1],
            vec![1, 0, 1],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
            vec![0xff; 256],
        ] {
            assert_eq!(BigUintWire::from_be_bytes(&bytes).to_be_bytes(), bytes);
        }
        assert_eq!(
            BigUintWire::from_be_bytes(&[0, 0, 1, 0, 1]).to_be_bytes(),
            vec![1, 0, 1]
        );
    }

    #[test]
    fn keys_round_trip_through_the_old_serialized_form() {
        let (pk, sk) = SecretKey::generate().unwrap();
        let pk2: PublicKey = ciborium::de::from_reader(cbor(&pk).as_slice()).unwrap();
        assert_eq!(pk, pk2);
        assert_eq!(cbor(&pk), cbor(&pk2));
        let sk2: SecretKey = ciborium::de::from_reader(cbor(&sk).as_slice()).unwrap();
        assert_eq!(sk2.public_key().unwrap(), pk);
        assert_eq!(cbor(&sk), cbor(&sk2));
    }
}
