//! The certificate's CBOR shape, mirrored type for type from `ghostkey_lib`
//! 0.2 so that `serde` + `ciborium` accept the same inputs and re-encode them
//! to the same bytes.
//!
//! The re-encoding matters: the master key signs the CBOR encoding of the
//! notary payload as `ghostkey_lib` writes it, and `ghostkey_lib` verifies by
//! re-encoding the payload it decoded. Every type below therefore has to
//! serialize exactly as its `ghostkey_lib` counterpart does. The differential
//! tests compare the bytes directly.
//!
//! Field names are frozen wire format (`delegate`, `delegate_verifying_key`):
//! they predate the notary rename, see `ghostkey_lib`'s CHANGELOG.

use ed25519_dalek::{Signature, VerifyingKey};
use rsa::BoxedUint;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{Error, MAX_NOTARY_KEY_BITS, MIN_NOTARY_KEY_BITS};

/// `ghostkey_lib::ghost_key_certificate::GhostkeyCertificateV1`.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct CertificateWire {
    #[serde(rename = "delegate")]
    pub notary: NotaryCertificateWire,
    pub verifying_key: VerifyingKey,
    pub signature: RsaSignatureWire,
}

/// `ghostkey_lib::notary_certificate::NotaryCertificateV1`.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct NotaryCertificateWire {
    pub payload: NotaryPayloadWire,
    pub signature: Signature,
}

/// `ghostkey_lib::notary_certificate::NotaryPayload`.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct NotaryPayloadWire {
    #[serde(rename = "delegate_verifying_key")]
    pub notary_key: RsaPublicKeyWire,
    pub info: String,
}

/// `blind_rsa_signatures::PublicKey`, a newtype around `rsa` 0.8's
/// `RsaPublicKey`.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct RsaPublicKeyWire(pub RsaPublicKeyFields);

/// `rsa` 0.8's `RsaPublicKey` with its `serde` feature: `n` and `e` as
/// `num-bigint-dig` 0.8 `BigUint`s.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct RsaPublicKeyFields {
    pub n: BigUintWire,
    pub e: BigUintWire,
}

/// `blind_rsa_signatures::Signature`, a newtype around `Vec<u8>`.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct RsaSignatureWire(#[serde(deserialize_with = "bounded_vec")] pub Vec<u8>);

/// Most elements reserved up front for a sequence: room for the 128 digits
/// of a 4096-bit modulus and the 512 bytes of its signature. A longer
/// sequence still decodes, growing as it goes.
const MAX_RESERVED: usize = 1024;

/// `Vec<T>`'s `Deserialize`, except that the reservation made from the
/// length a CBOR header claims is capped at [`MAX_RESERVED`] elements, where
/// serde's own cap is 1 MiB. A 50-byte input claiming a 2^32-element array
/// would otherwise reserve 1 MiB before failing, and a contract's linear
/// memory never shrinks. Accepts exactly the inputs `Vec<T>` accepts.
fn bounded_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct V<T>(core::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for V<T> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            f.write_str("a sequence")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<T>, A::Error> {
            let mut out = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(MAX_RESERVED));
            while let Some(item) = seq.next_element()? {
                out.push(item);
            }
            Ok(out)
        }
    }
    deserializer.deserialize_seq(V(core::marker::PhantomData))
}

/// A `num-bigint-dig` 0.8 `BigUint` built with the `u64_digit` feature, which
/// `rsa` 0.8 enables.
///
/// It decodes from little-endian `u32` digits, packed pairwise into `u64`
/// digits and stripped of high zero digits. It encodes every `u64` digit as
/// two `u32`s, except the most significant, which becomes one `u32` when it is
/// below `u32::MAX` (strictly: a top digit of exactly `u32::MAX` still takes
/// two). That asymmetry is `num-bigint-dig`'s, reproduced so the bytes match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BigUintWire {
    /// Little-endian `u64` digits, no high zero digit.
    digits: Vec<u64>,
}

impl<'de> Deserialize<'de> for BigUintWire {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw: Vec<u32> = bounded_vec(deserializer)?;
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

impl BigUintWire {
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

    #[cfg(test)]
    pub(crate) fn from_u32s(raw: &[u32]) -> Self {
        let bytes = to_cbor(&raw.to_vec());
        ciborium::de::from_reader(bytes.as_slice()).unwrap()
    }
}

impl RsaPublicKeyWire {
    /// The key, if it is within the accepted parameters: an odd modulus of
    /// 2048 to 4096 bits and a public exponent of 3 or 65537.
    pub(crate) fn to_rsa(&self) -> Result<rsa::RsaPublicKey, Error> {
        let RsaPublicKeyFields { n, e } = &self.0;
        let e_ok = matches!(e.digits.as_slice(), [3] | [65537]);
        let n = n.to_be_bytes();
        let n_bits = match n.first() {
            Some(&top) => n.len() * 8 - top.leading_zeros() as usize,
            None => 0,
        };
        if !e_ok || !(MIN_NOTARY_KEY_BITS..=MAX_NOTARY_KEY_BITS).contains(&n_bits) {
            return Err(Error::NotaryKey);
        }
        let e = BoxedUint::from(e.digits[0]);
        rsa::RsaPublicKey::new_with_max_size(
            BoxedUint::from_be_slice_vartime(&n),
            e,
            MAX_NOTARY_KEY_BITS,
        )
        .map_err(|_| Error::NotaryKey)
    }
}

/// CBOR-encode as `ghostkey_lib`'s `Armorable::to_bytes` does.
pub(crate) fn to_cbor<T: Serialize>(value: &T) -> Vec<u8> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(value, &mut buf).expect("writing CBOR to a Vec cannot fail");
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(raw: &[u32]) -> Vec<u32> {
        let bytes = to_cbor(&BigUintWire::from_u32s(raw));
        ciborium::de::from_reader(bytes.as_slice()).unwrap()
    }

    #[test]
    fn biguint_encoding_matches_num_bigint_dig_u64_digit() {
        // Values chosen at the boundaries of the top-digit rule.
        assert_eq!(round_trip(&[]), Vec::<u32>::new());
        assert_eq!(round_trip(&[0]), Vec::<u32>::new());
        assert_eq!(round_trip(&[65537]), vec![65537]);
        assert_eq!(round_trip(&[65537, 0]), vec![65537]);
        assert_eq!(round_trip(&[u32::MAX - 1]), vec![u32::MAX - 1]);
        // A top digit of exactly u32::MAX takes two words.
        assert_eq!(round_trip(&[u32::MAX]), vec![u32::MAX, 0]);
        assert_eq!(round_trip(&[1, 1]), vec![1, 1]);
        assert_eq!(round_trip(&[1, 2, 3]), vec![1, 2, 3]);
        assert_eq!(round_trip(&[1, 0, 3, 0, 0, 0]), vec![1, 0, 3]);
        assert_eq!(round_trip(&[1, 2, 3, 4]), vec![1, 2, 3, 4]);
    }

    #[test]
    fn biguint_be_bytes() {
        assert_eq!(BigUintWire::from_u32s(&[]).to_be_bytes(), Vec::<u8>::new());
        assert_eq!(
            BigUintWire::from_u32s(&[65537]).to_be_bytes(),
            vec![1, 0, 1]
        );
        assert_eq!(
            BigUintWire::from_u32s(&[0x0304_0506, 0x0102]).to_be_bytes(),
            vec![1, 2, 3, 4, 5, 6]
        );
    }

    fn key(n_words: usize, e: u32) -> Result<rsa::RsaPublicKey, Error> {
        // An odd number with its top bit set, `n_words * 32` bits long.
        let mut n = vec![0x1357_9bdf_u32; n_words];
        n[0] |= 1;
        n[n_words - 1] |= 0x8000_0000;
        RsaPublicKeyWire(RsaPublicKeyFields {
            n: BigUintWire::from_u32s(&n),
            e: BigUintWire::from_u32s(&[e]),
        })
        .to_rsa()
    }

    #[test]
    fn notary_key_parameters() {
        assert!(key(64, 65537).is_ok());
        assert!(key(64, 3).is_ok());
        assert!(key(96, 65537).is_ok());
        assert!(key(128, 65537).is_ok());
        assert_eq!(key(63, 65537).err(), Some(Error::NotaryKey));
        assert_eq!(key(32, 65537).err(), Some(Error::NotaryKey));
        assert_eq!(key(129, 65537).err(), Some(Error::NotaryKey));
        // 4097 bits: one bit past the largest accepted.
        let mut n = vec![0x1357_9bdf_u32 | 1; 128];
        n.push(1);
        let just_over = RsaPublicKeyWire(RsaPublicKeyFields {
            n: BigUintWire::from_u32s(&n),
            e: BigUintWire::from_u32s(&[65537]),
        });
        assert_eq!(just_over.to_rsa().err(), Some(Error::NotaryKey));
        for e in [0, 1, 2, 5, 17, 65539] {
            assert_eq!(key(64, e).err(), Some(Error::NotaryKey), "e = {e}");
        }
        // An even modulus is refused by rsa itself.
        let mut n = vec![0x1357_9bd0_u32; 64];
        n[63] |= 0x8000_0000;
        let even = RsaPublicKeyWire(RsaPublicKeyFields {
            n: BigUintWire::from_u32s(&n),
            e: BigUintWire::from_u32s(&[65537]),
        });
        assert_eq!(even.to_rsa().err(), Some(Error::NotaryKey));
    }
}
