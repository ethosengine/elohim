//! Identity import — decrypt custodial key bundle from doorway
//!
//! Replicates the exact crypto parameters from `doorway/src/custodial_keys/crypto.rs`:
//! - Argon2id: 64 MB memory, 3 iterations, 4 parallelism
//! - ChaCha20-Poly1305: authenticated encryption
//!
//! The key bundle comes from doorway's NativeHandoffResponse. The user provides
//! their password to decrypt the Ed25519 signing key locally.

use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, Key, KeyInit, Nonce};

use crate::doorway::KeyExportFormat;

// Must match doorway/src/custodial_keys/crypto.rs exactly
const ARGON2_MEMORY_KB: u32 = 65536; // 64 MB
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_PARALLELISM: u32 = 4;

/// Decrypt a key bundle from doorway, returning the 32-byte Ed25519 signing key.
///
/// The user's password is used to derive the encryption key via Argon2id,
/// then ChaCha20-Poly1305 decrypts the private key.
///
/// # Arguments
///
/// - `bundle`: Encrypted key bundle from doorway's native handoff
/// - `password`: User's password (same one used to register at doorway)
///
/// # Returns
///
/// The 32-byte Ed25519 signing key bytes.
pub fn decrypt_key_bundle(bundle: &KeyExportFormat, password: &str) -> Result<[u8; 32], String> {
    // Decode base64 fields
    let salt = BASE64
        .decode(&bundle.key_derivation_salt)
        .map_err(|e| format!("Invalid salt: {}", e))?;

    let nonce_bytes = BASE64
        .decode(&bundle.encryption_nonce)
        .map_err(|e| format!("Invalid nonce: {}", e))?;

    let ciphertext = BASE64
        .decode(&bundle.encrypted_private_key)
        .map_err(|e| format!("Invalid encrypted key: {}", e))?;

    if nonce_bytes.len() != 12 {
        return Err(format!(
            "Invalid nonce length: expected 12, got {}",
            nonce_bytes.len()
        ));
    }

    // Derive encryption key from password + salt (Argon2id)
    let params = Params::new(
        ARGON2_MEMORY_KB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(32),
    )
    .map_err(|e| format!("Invalid Argon2 params: {}", e))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut encryption_key = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), &salt, &mut encryption_key)
        .map_err(|e| format!("Key derivation failed: {}", e))?;

    // Decrypt private key (ChaCha20-Poly1305)
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&encryption_key));
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_slice())
        .map_err(|_| "Failed to decrypt key — wrong password?".to_string())?;

    if plaintext.len() != 32 {
        return Err(format!(
            "Invalid decrypted key length: expected 32, got {}",
            plaintext.len()
        ));
    }

    let mut signing_key = [0u8; 32];
    signing_key.copy_from_slice(&plaintext);
    Ok(signing_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chacha20poly1305::KeyInit as _;

    /// End-to-end test: encrypt a key with doorway's parameters, decrypt with ours
    #[test]
    fn test_decrypt_roundtrip() {
        let password = "test-password-123";

        // Generate a fake 32-byte signing key
        let original_key: [u8; 32] = [42u8; 32];

        // Generate random salt and nonce
        let salt = [1u8; 16];
        let nonce_bytes = [2u8; 12];

        // Derive encryption key (same as doorway would)
        let params = Params::new(
            ARGON2_MEMORY_KB,
            ARGON2_ITERATIONS,
            ARGON2_PARALLELISM,
            Some(32),
        )
        .unwrap();
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut encryption_key = [0u8; 32];
        argon2
            .hash_password_into(password.as_bytes(), &salt, &mut encryption_key)
            .unwrap();

        // Encrypt (same as doorway would)
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&encryption_key));
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), original_key.as_slice())
            .unwrap();

        // Build a KeyExportFormat
        let bundle = KeyExportFormat {
            version: 1,
            identifier: "test@example.com".to_string(),
            human_id: "uhCAk_test".to_string(),
            public_key: BASE64.encode([0u8; 32]),
            encrypted_private_key: BASE64.encode(&ciphertext),
            key_derivation_salt: BASE64.encode(salt),
            encryption_nonce: BASE64.encode(nonce_bytes),
            exported_at: "2025-01-01T00:00:00Z".to_string(),
            doorway_id: "test-doorway".to_string(),
        };

        // Decrypt with our function
        let decrypted = decrypt_key_bundle(&bundle, password).unwrap();
        assert_eq!(decrypted, original_key);
    }

    #[test]
    fn test_decrypt_wrong_password() {
        let original_key: [u8; 32] = [42u8; 32];
        let salt = [1u8; 16];
        let nonce_bytes = [2u8; 12];

        let params = Params::new(
            ARGON2_MEMORY_KB,
            ARGON2_ITERATIONS,
            ARGON2_PARALLELISM,
            Some(32),
        )
        .unwrap();
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut encryption_key = [0u8; 32];
        argon2
            .hash_password_into(b"correct-password", &salt, &mut encryption_key)
            .unwrap();

        let cipher = ChaCha20Poly1305::new(Key::from_slice(&encryption_key));
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), original_key.as_slice())
            .unwrap();

        let bundle = KeyExportFormat {
            version: 1,
            identifier: "test@example.com".to_string(),
            human_id: "uhCAk_test".to_string(),
            public_key: BASE64.encode([0u8; 32]),
            encrypted_private_key: BASE64.encode(&ciphertext),
            key_derivation_salt: BASE64.encode(salt),
            encryption_nonce: BASE64.encode(nonce_bytes),
            exported_at: "2025-01-01T00:00:00Z".to_string(),
            doorway_id: "test-doorway".to_string(),
        };

        let result = decrypt_key_bundle(&bundle, "wrong-password");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("wrong password"));
    }
}
