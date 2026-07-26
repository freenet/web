//! Stateless proof-of-work challenges for River invitation issuance.
//!
//! A challenge is authenticated with a process-local HMAC key, so clients
//! cannot lower its difficulty or extend its lifetime. Successfully used
//! challenge ids are retained until expiry to make each proof single-use.

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::Utc;
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

const CHALLENGE_BYTES: usize = 16;
const SIGNATURE_BYTES: usize = 32;
const CHALLENGE_TTL_SECONDS: i64 = 5 * 60;
const DOMAIN: &[u8] = b"freenet-river-invite-pow-v1";

/// Difficulty increases as successful invitation volume approaches the global
/// emergency ceiling. Each additional bit doubles expected work.
pub const MEDIUM_TRAFFIC_THRESHOLD: usize = 90;
pub const HIGH_TRAFFIC_THRESHOLD: usize = 150;
pub const DEFAULT_POW_DIFFICULTY: u8 = 16;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PowChallenge {
    pub challenge: String,
    pub issued_at: i64,
    pub difficulty: u8,
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct PowChallengeResponse {
    #[serde(flatten)]
    pub challenge: PowChallenge,
    pub algorithm: &'static str,
    pub expires_in_seconds: i64,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PowError {
    #[error("malformed challenge")]
    Malformed,
    #[error("challenge signature is invalid")]
    InvalidSignature,
    #[error("challenge has expired")]
    Expired,
    #[error("proof of work is invalid")]
    InvalidProof,
    #[error("challenge has already been used")]
    Reused,
    #[error("proof-of-work state is unavailable")]
    Lock,
}

pub struct PowManager {
    secret: [u8; 32],
    base_difficulty: u8,
    /// challenge id -> wall-clock expiry. Entries exist only after a valid
    /// proof is consumed, so challenge-request floods do not grow this map.
    used: Mutex<HashMap<[u8; CHALLENGE_BYTES], i64>>,
}

impl PowManager {
    pub fn new(base_difficulty: u8) -> Self {
        let mut secret = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret);
        Self {
            secret,
            base_difficulty,
            used: Mutex::new(HashMap::new()),
        }
    }

    #[cfg(test)]
    fn with_secret(base_difficulty: u8, secret: [u8; 32]) -> Self {
        Self {
            secret,
            base_difficulty,
            used: Mutex::new(HashMap::new()),
        }
    }

    pub fn difficulty(&self, recent_invites: usize) -> u8 {
        let extra = if recent_invites >= HIGH_TRAFFIC_THRESHOLD {
            4
        } else if recent_invites >= MEDIUM_TRAFFIC_THRESHOLD {
            2
        } else {
            0
        };
        self.base_difficulty.saturating_add(extra).min(30)
    }

    pub fn issue(&self, recent_invites: usize) -> PowChallengeResponse {
        let mut id = [0u8; CHALLENGE_BYTES];
        rand::thread_rng().fill_bytes(&mut id);
        let issued_at = Utc::now().timestamp();
        let difficulty = self.difficulty(recent_invites);
        let signature = self.sign(&id, issued_at, difficulty);
        PowChallengeResponse {
            challenge: PowChallenge {
                challenge: hex::encode(id),
                issued_at,
                difficulty,
                signature: hex::encode(signature),
            },
            algorithm: "sha256-leading-zero-bits-v1",
            expires_in_seconds: CHALLENGE_TTL_SECONDS,
        }
    }

    /// Validate and atomically consume a proof. The returned challenge id can
    /// be passed to [`Self::release`] if a downstream admission check fails.
    pub fn verify_and_consume(
        &self,
        challenge: &PowChallenge,
        nonce: u64,
    ) -> Result<[u8; CHALLENGE_BYTES], PowError> {
        let id: [u8; CHALLENGE_BYTES] = hex::decode(&challenge.challenge)
            .ok()
            .and_then(|v| v.try_into().ok())
            .ok_or(PowError::Malformed)?;
        let signature: [u8; SIGNATURE_BYTES] = hex::decode(&challenge.signature)
            .ok()
            .and_then(|v| v.try_into().ok())
            .ok_or(PowError::Malformed)?;

        let now = Utc::now().timestamp();
        let age = now.saturating_sub(challenge.issued_at);
        if !(0..=CHALLENGE_TTL_SECONDS).contains(&age) {
            return Err(PowError::Expired);
        }

        let mut mac = HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts 32-byte keys");
        mac.update(DOMAIN);
        mac.update(&id);
        mac.update(&challenge.issued_at.to_be_bytes());
        mac.update(&[challenge.difficulty]);
        mac.verify_slice(&signature)
            .map_err(|_| PowError::InvalidSignature)?;

        if !valid_proof(&id, nonce, challenge.difficulty) {
            return Err(PowError::InvalidProof);
        }

        let mut used = self.used.lock().map_err(|_| PowError::Lock)?;
        used.retain(|_, expiry| *expiry > now);
        if used.contains_key(&id) {
            return Err(PowError::Reused);
        }
        used.insert(id, challenge.issued_at + CHALLENGE_TTL_SECONDS);
        Ok(id)
    }

    /// Make a consumed challenge reusable after a downstream refusal. This
    /// prevents a race at the global ceiling or a transient storage error from
    /// forcing a legitimate browser to repeat the expensive work.
    pub fn release(&self, id: &[u8; CHALLENGE_BYTES]) {
        if let Ok(mut used) = self.used.lock() {
            used.remove(id);
        }
    }

    fn sign(
        &self,
        id: &[u8; CHALLENGE_BYTES],
        issued_at: i64,
        difficulty: u8,
    ) -> [u8; SIGNATURE_BYTES] {
        let mut mac = HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts 32-byte keys");
        mac.update(DOMAIN);
        mac.update(id);
        mac.update(&issued_at.to_be_bytes());
        mac.update(&[difficulty]);
        mac.finalize().into_bytes().into()
    }
}

pub(crate) fn valid_proof(id: &[u8; CHALLENGE_BYTES], nonce: u64, difficulty: u8) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update(id);
    hasher.update(nonce.to_be_bytes());
    has_leading_zero_bits(&hasher.finalize(), difficulty)
}

fn has_leading_zero_bits(hash: &[u8], difficulty: u8) -> bool {
    let full_bytes = usize::from(difficulty / 8);
    let remaining_bits = difficulty % 8;
    if full_bytes > hash.len() || (full_bytes == hash.len() && remaining_bits > 0) {
        return false;
    }
    if hash[..full_bytes].iter().any(|byte| *byte != 0) {
        return false;
    }
    remaining_bits == 0 || hash[full_bytes] >> (8 - remaining_bits) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solve(challenge: &PowChallenge) -> u64 {
        (0..u64::MAX)
            .find(|nonce| {
                let id: [u8; CHALLENGE_BYTES] = hex::decode(&challenge.challenge)
                    .unwrap()
                    .try_into()
                    .unwrap();
                valid_proof(&id, *nonce, challenge.difficulty)
            })
            .unwrap()
    }

    #[test]
    fn difficulty_is_adaptive() {
        let manager = PowManager::with_secret(8, [7; 32]);
        assert_eq!(manager.difficulty(0), 8);
        assert_eq!(manager.difficulty(MEDIUM_TRAFFIC_THRESHOLD), 10);
        assert_eq!(manager.difficulty(HIGH_TRAFFIC_THRESHOLD), 12);
    }

    #[test]
    fn valid_proof_is_single_use() {
        let manager = PowManager::with_secret(8, [7; 32]);
        let response = manager.issue(0);
        let nonce = solve(&response.challenge);
        let id = manager
            .verify_and_consume(&response.challenge, nonce)
            .unwrap();
        assert_eq!(
            manager.verify_and_consume(&response.challenge, nonce),
            Err(PowError::Reused)
        );
        manager.release(&id);
        assert!(manager
            .verify_and_consume(&response.challenge, nonce)
            .is_ok());
    }

    #[test]
    fn signed_fields_cannot_be_changed() {
        let manager = PowManager::with_secret(8, [7; 32]);
        let mut challenge = manager.issue(0).challenge;
        challenge.difficulty = 1;
        assert_eq!(
            manager.verify_and_consume(&challenge, 0),
            Err(PowError::InvalidSignature)
        );
    }

    #[test]
    fn incorrect_nonce_is_rejected() {
        let manager = PowManager::with_secret(8, [7; 32]);
        let challenge = manager.issue(0).challenge;
        let nonce = solve(&challenge);
        let wrong = nonce.wrapping_add(1);
        if wrong != nonce {
            assert_eq!(
                manager.verify_and_consume(&challenge, wrong),
                Err(PowError::InvalidProof)
            );
        }
    }
}
