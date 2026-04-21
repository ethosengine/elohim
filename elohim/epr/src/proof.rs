//! Ed25519 signing and verification.
//!
//! Conforms to RFC 8032. Uses ed25519-dalek's SigningKey/VerifyingKey.
//! AgentKeypair wraps a SigningKey and exposes the public key bytes
//! for verification paths.

use crate::error::{EprError, Result};
use ed25519_dalek::{Signature as EdSig, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::{CryptoRng, RngCore};

pub struct AgentKeypair(SigningKey);

impl AgentKeypair {
    /// Generate a fresh random keypair using the provided CSPRNG.
    pub fn generate<R: CryptoRng + RngCore>(rng: &mut R) -> Self {
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);
        AgentKeypair(SigningKey::from_bytes(&seed))
    }

    /// Construct a keypair from a 32-byte secret seed.
    pub fn from_secret(secret: &[u8]) -> Result<Self> {
        if secret.len() != 32 {
            return Err(EprError::Signature(format!(
                "ed25519 secret must be 32 bytes, got {}",
                secret.len()
            )));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(secret);
        Ok(AgentKeypair(SigningKey::from_bytes(&arr)))
    }

    /// Return the raw 32-byte public key.
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.0.verifying_key().to_bytes()
    }

    /// Return the raw 32-byte secret seed.
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }
}

/// Sign `message` with the given keypair. Returns 64 raw bytes.
pub fn sign(kp: &AgentKeypair, message: &[u8]) -> Vec<u8> {
    kp.0.sign(message).to_bytes().to_vec()
}

/// Verify a detached signature.
pub fn verify(public_key: &[u8; 32], message: &[u8], signature: &[u8]) -> bool {
    let Ok(vk) = VerifyingKey::from_bytes(public_key) else {
        return false;
    };
    let Ok(sig) = EdSig::from_slice(signature) else {
        return false;
    };
    vk.verify(message, &sig).is_ok()
}
