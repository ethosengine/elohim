//! Cryptographic primitives for custodial key management.
//!
//! # Algorithms
//!
//! - **Key Generation**: Ed25519 (same as Holochain agents)
//! - **Key Derivation**: Argon2id (memory-hard, brute-force resistant)
//! - **Encryption**: ChaCha20-Poly1305 (authenticated encryption)
//!
//! # Security Parameters
//!
//! Argon2id parameters are tuned for password-based key encryption:
//! - 64 MB memory (prevents GPU attacks)
//! - 3 iterations
//! - 4 parallelism threads

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, Key, KeyInit, Nonce};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;

use crate::types::{DoorwayError, Result};

// =============================================================================
// Constants
// =============================================================================

/// Argon2id memory cost in KiB (64 MB)
pub const ARGON2_MEMORY_KB: u32 = 65536;

/// Argon2id iteration count
pub const ARGON2_ITERATIONS: u32 = 3;

/// Argon2id parallelism (threads)
pub const ARGON2_PARALLELISM: u32 = 4;

/// Salt length for key derivation (16 bytes)
pub const SALT_LEN: usize = 16;

/// Nonce length for ChaCha20-Poly1305 (12 bytes)
pub const NONCE_LEN: usize = 12;

/// Ed25519 private key length (32 bytes)
pub const PRIVATE_KEY_LEN: usize = 32;

/// ChaCha20-Poly1305 auth tag length (16 bytes)
pub const AUTH_TAG_LEN: usize = 16;

// =============================================================================
// Key Generation
// =============================================================================

/// Generate a new Ed25519 signing keypair.
///
/// Uses the OS cryptographically secure random number generator.
///
/// # Returns
///
/// A tuple of (signing_key, verifying_key) where:
/// - `signing_key` is the private key (32 bytes)
/// - `verifying_key` is the public key (32 bytes)
pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

/// Generate cryptographically secure random bytes.
///
/// # Type Parameters
///
/// - `N`: The number of bytes to generate
pub fn generate_random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    OsRng.fill_bytes(&mut bytes);
    bytes
}

// =============================================================================
// Key Derivation
// =============================================================================

/// Derive a 256-bit encryption key from a password using Argon2id.
///
/// This key is used to encrypt the user's private key before storage.
/// The memory-hard parameters make brute-force attacks expensive.
///
/// # Arguments
///
/// - `password`: User's password (UTF-8 bytes)
/// - `salt`: Random 16-byte salt (unique per user)
///
/// # Returns
///
/// A 32-byte key encryption key (KEK).
///
/// # Security
///
/// - Uses Argon2id (hybrid of Argon2i and Argon2d)
/// - 64 MB memory makes GPU/ASIC attacks costly
/// - Salt prevents rainbow table attacks
pub fn derive_key_encryption_key(password: &[u8], salt: &[u8]) -> Result<[u8; 32]> {
    let params = Params::new(
        ARGON2_MEMORY_KB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(32),
    )
    .map_err(|e| DoorwayError::Internal(format!("Invalid Argon2 params: {e}")))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password, salt, &mut key)
        .map_err(|e| DoorwayError::Internal(format!("Key derivation failed: {e}")))?;

    Ok(key)
}

// =============================================================================
// Encryption / Decryption
// =============================================================================

/// Encrypt a private key using ChaCha20-Poly1305.
///
/// # Arguments
///
/// - `private_key`: The 32-byte Ed25519 private key to encrypt
/// - `encryption_key`: The 32-byte key encryption key (from Argon2)
/// - `nonce`: A 12-byte random nonce (unique per encryption)
///
/// # Returns
///
/// Ciphertext (48 bytes = 32 bytes encrypted key + 16 bytes auth tag).
///
/// # Security
///
/// - ChaCha20-Poly1305 provides authenticated encryption
/// - The auth tag prevents tampering
/// - Nonce must never be reused with the same key
pub fn encrypt_private_key(
    private_key: &[u8; PRIVATE_KEY_LEN],
    encryption_key: &[u8; 32],
    nonce: &[u8; NONCE_LEN],
) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(encryption_key));
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(nonce), private_key.as_slice())
        .map_err(|e| DoorwayError::Internal(format!("Encryption failed: {e}")))?;

    Ok(ciphertext)
}

/// Decrypt a private key using ChaCha20-Poly1305.
///
/// # Arguments
///
/// - `ciphertext`: The encrypted key (48 bytes)
/// - `encryption_key`: The 32-byte key encryption key (from Argon2)
/// - `nonce`: The 12-byte nonce used during encryption
///
/// # Returns
///
/// The decrypted 32-byte Ed25519 private key.
///
/// # Errors
///
/// Returns an error if:
/// - The ciphertext is tampered (auth tag verification fails)
/// - The wrong password was used (wrong encryption key)
pub fn decrypt_private_key(
    ciphertext: &[u8],
    encryption_key: &[u8; 32],
    nonce: &[u8; NONCE_LEN],
) -> Result<[u8; PRIVATE_KEY_LEN]> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(encryption_key));
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| DoorwayError::Auth("Failed to decrypt key (wrong password?)".into()))?;

    if plaintext.len() != PRIVATE_KEY_LEN {
        return Err(DoorwayError::Internal(format!(
            "Invalid decrypted key length: expected {}, got {}",
            PRIVATE_KEY_LEN,
            plaintext.len()
        )));
    }

    let mut key = [0u8; PRIVATE_KEY_LEN];
    key.copy_from_slice(&plaintext);
    Ok(key)
}

// =============================================================================
// Signing
// =============================================================================

/// Sign a payload with an Ed25519 private key.
///
/// # Arguments
///
/// - `signing_key`: The Ed25519 signing key
/// - `payload`: The data to sign
///
/// # Returns
///
/// A 64-byte Ed25519 signature.
pub fn sign_payload(signing_key: &SigningKey, payload: &[u8]) -> Signature {
    signing_key.sign(payload)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let (signing_key, verifying_key) = generate_keypair();

        // Keys should be 32 bytes
        assert_eq!(signing_key.to_bytes().len(), 32);
        assert_eq!(verifying_key.to_bytes().len(), 32);

        // Verifying key should match signing key
        assert_eq!(signing_key.verifying_key(), verifying_key);
    }

    #[test]
    fn test_random_bytes() {
        let bytes1: [u8; 16] = generate_random_bytes();
        let bytes2: [u8; 16] = generate_random_bytes();

        // Should generate different values
        assert_ne!(bytes1, bytes2);
    }

    #[test]
    fn test_key_derivation() {
        let password = b"test-password-123";
        let salt: [u8; SALT_LEN] = generate_random_bytes();

        let key1 = derive_key_encryption_key(password, &salt).unwrap();
        let key2 = derive_key_encryption_key(password, &salt).unwrap();

        // Same password + salt = same key
        assert_eq!(key1, key2);

        // Different salt = different key
        let salt2: [u8; SALT_LEN] = generate_random_bytes();
        let key3 = derive_key_encryption_key(password, &salt2).unwrap();
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let password = b"my-secure-password";
        let salt: [u8; SALT_LEN] = generate_random_bytes();
        let nonce: [u8; NONCE_LEN] = generate_random_bytes();

        // Generate keypair
        let (signing_key, _) = generate_keypair();
        let private_key = signing_key.to_bytes();

        // Derive encryption key
        let encryption_key = derive_key_encryption_key(password, &salt).unwrap();

        // Encrypt
        let ciphertext = encrypt_private_key(&private_key, &encryption_key, &nonce).unwrap();

        // Ciphertext should be 48 bytes (32 + 16 auth tag)
        assert_eq!(ciphertext.len(), PRIVATE_KEY_LEN + AUTH_TAG_LEN);

        // Decrypt
        let decrypted = decrypt_private_key(&ciphertext, &encryption_key, &nonce).unwrap();

        // Should match original
        assert_eq!(decrypted, private_key);
    }

    #[test]
    fn test_decrypt_wrong_password_fails() {
        let password = b"correct-password";
        let wrong_password = b"wrong-password";
        let salt: [u8; SALT_LEN] = generate_random_bytes();
        let nonce: [u8; NONCE_LEN] = generate_random_bytes();

        let (signing_key, _) = generate_keypair();
        let private_key = signing_key.to_bytes();

        let encryption_key = derive_key_encryption_key(password, &salt).unwrap();
        let ciphertext = encrypt_private_key(&private_key, &encryption_key, &nonce).unwrap();

        // Try to decrypt with wrong password
        let wrong_key = derive_key_encryption_key(wrong_password, &salt).unwrap();
        let result = decrypt_private_key(&ciphertext, &wrong_key, &nonce);

        // Should fail
        assert!(result.is_err());
    }

    #[test]
    fn test_signing() {
        let (signing_key, verifying_key) = generate_keypair();
        let message = b"Hello, Holochain!";

        let signature = sign_payload(&signing_key, message);

        // Verify signature
        use ed25519_dalek::Verifier;
        assert!(verifying_key.verify(message, &signature).is_ok());
    }
}
