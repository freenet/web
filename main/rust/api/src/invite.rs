//! River room invite generation
//!
//! This module replicates the necessary types from river-core to generate
//! room invitations without depending on the full river-core crate.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Simple hash function matching freenet_scaffold::util::fast_hash
/// Uses Java-style string hashing (multiply by 31, add byte)
pub fn fast_hash(bytes: &[u8]) -> FastHash {
    let mut hash: i64 = 0;
    for &byte in bytes {
        hash = hash.wrapping_mul(31).wrapping_add(byte as i64);
    }
    FastHash(hash)
}

/// Hash type matching freenet_scaffold::util::FastHash
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FastHash(pub i64);

/// Member ID derived from a VerifyingKey hash
/// Matches river_core::room_state::member::MemberId
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MemberId(pub FastHash);

impl From<&VerifyingKey> for MemberId {
    fn from(vk: &VerifyingKey) -> Self {
        MemberId(fast_hash(&vk.to_bytes()))
    }
}

impl From<VerifyingKey> for MemberId {
    fn from(vk: VerifyingKey) -> Self {
        MemberId(fast_hash(&vk.to_bytes()))
    }
}

/// Room member entry
/// Matches river_core::room_state::member::Member
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Member {
    pub owner_member_id: MemberId,
    pub invited_by: MemberId,
    pub member_vk: VerifyingKey,
}

/// Authorized member with signature from inviter
/// Matches river_core::room_state::member::AuthorizedMember
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthorizedMember {
    pub member: Member,
    pub signature: Signature,
}

/// Room invitation containing credentials for joining
/// Matches river cli Invitation struct
#[derive(Serialize, Deserialize)]
pub struct Invitation {
    pub room: VerifyingKey,
    pub invitee_signing_key: SigningKey,
    pub invitee: AuthorizedMember,
}

#[derive(Error, Debug)]
pub enum InviteError {
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Sign a serializable struct using CBOR encoding
/// Matches river_core::util::sign_struct
fn sign_struct<T: Serialize>(message: &T, signing_key: &SigningKey) -> Signature {
    let mut data = Vec::new();
    ciborium::ser::into_writer(message, &mut data).expect("Serialization should not fail");
    signing_key.sign(&data)
}

/// Create a room invitation
///
/// # Arguments
/// * `room_owner_vk` - The room owner's verifying key (identifies the room)
/// * `inviter_signing_key` - The signing key of the member creating the invite
///
/// # Returns
/// A base58-encoded invitation string that can be shared with the invitee
pub fn create_invitation(
    room_owner_vk: &VerifyingKey,
    inviter_signing_key: &SigningKey,
) -> Result<String, InviteError> {
    // Generate a new signing key for the invitee
    let invitee_signing_key = SigningKey::generate(&mut rand::thread_rng());
    let invitee_vk = invitee_signing_key.verifying_key();

    // Create the member entry
    let member = Member {
        owner_member_id: room_owner_vk.into(),
        invited_by: inviter_signing_key.verifying_key().into(),
        member_vk: invitee_vk,
    };

    // Sign the member entry with the inviter's key
    let signature = sign_struct(&member, inviter_signing_key);
    let authorized_member = AuthorizedMember { member, signature };

    // Create the invitation
    let invitation = Invitation {
        room: *room_owner_vk,
        invitee_signing_key,
        invitee: authorized_member,
    };

    // Serialize with CBOR and encode as base58
    let mut data = Vec::new();
    ciborium::ser::into_writer(&invitation, &mut data)
        .map_err(|e| InviteError::Serialization(e.to_string()))?;

    Ok(bs58::encode(data).into_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_hash() {
        let data = b"hello world";
        let hash = fast_hash(data);
        // Should produce a consistent hash
        assert_eq!(hash, fast_hash(data));
    }

    #[test]
    fn test_member_id_from_vk() {
        let signing_key = SigningKey::generate(&mut rand::thread_rng());
        let vk = signing_key.verifying_key();
        let id1: MemberId = (&vk).into();
        let id2: MemberId = (&vk).into();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_create_invitation() {
        // Create room owner
        let owner_signing_key = SigningKey::generate(&mut rand::thread_rng());
        let owner_vk = owner_signing_key.verifying_key();

        // Create invitation
        let invite_code = create_invitation(&owner_vk, &owner_signing_key).unwrap();

        // Verify it's valid base58
        assert!(!invite_code.is_empty());
        let decoded = bs58::decode(&invite_code).into_vec().unwrap();
        assert!(!decoded.is_empty());

        // Verify we can deserialize it
        let invitation: Invitation = ciborium::de::from_reader(&decoded[..]).unwrap();
        assert_eq!(invitation.room, owner_vk);
    }

    #[test]
    fn test_real_freenet_chat_invite() {
        // Room owner VK from freenet-chat: 93XNNwmRLQ6nwUwi4dDmp3kpjMb5ekMRc2e22x5TAnUY
        let owner_vk_bytes = bs58::decode("93XNNwmRLQ6nwUwi4dDmp3kpjMb5ekMRc2e22x5TAnUY")
            .into_vec()
            .expect("Failed to decode owner VK");
        let owner_vk = VerifyingKey::from_bytes(&owner_vk_bytes.try_into().expect("Invalid VK length"))
            .expect("Invalid VK");

        // Signing key from rooms.json for freenet-chat
        let signing_key_bytes: [u8; 32] = [
            1, 3, 163, 211, 4, 113, 25, 236, 171, 57, 117, 76, 11, 233, 182, 31,
            111, 137, 94, 202, 149, 4, 41, 59, 145, 54, 18, 82, 243, 194, 71, 224
        ];
        let signing_key = SigningKey::from_bytes(&signing_key_bytes);

        let invite = create_invitation(&owner_vk, &signing_key)
            .expect("Failed to create invitation");

        println!("\n=== Generated Invite for freenet-chat ===");
        println!("{}", invite);
        println!("Length: {} chars\n", invite.len());

        // Verify format
        assert!(!invite.is_empty());
        let decoded = bs58::decode(&invite).into_vec().unwrap();
        let invitation: Invitation = ciborium::de::from_reader(&decoded[..]).unwrap();
        assert_eq!(invitation.room, owner_vk);
    }
}
