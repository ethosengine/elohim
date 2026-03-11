//! Custodial Key Service
//!
//! Main service for managing per-user keypairs for hosted humans.
//!
//! # Responsibilities
//!
//! - Generate and encrypt keypairs at registration
//! - Decrypt and cache keys at login
//! - Provide signing keys for zome calls
//! - Export keys for migration to stewardship

use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::db::schemas::{CustodialKeyMaterial, UserDoc};
use crate::types::{DoorwayError, Result};

use super::cache::{SigningKeyCache, SigningKeyCacheConfig};
use super::crypto::{
    decrypt_private_key, derive_key_encryption_key, encrypt_private_key, generate_keypair,
    generate_random_bytes, NONCE_LEN, SALT_LEN,
};

// =============================================================================
// Key Export Format
// =============================================================================

/// Export format for key migration to Tauri (stewardship).
///
/// This bundle contains everything needed to decrypt and use the key
/// in a Tauri app. The private key is still encrypted - the user must
/// provide their password to the Tauri app to decrypt it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyExportFormat {
    /// Version of the export format (for forward compatibility)
    pub version: u32,

    /// User identifier (email)
    pub identifier: String,

    /// Holochain human ID
    pub human_id: String,

    /// Ed25519 public key (base64)
    pub public_key: String,

    /// Encrypted private key (base64) - still password-protected
    pub encrypted_private_key: String,

    /// Key derivation salt (base64)
    pub key_derivation_salt: String,

    /// Encryption nonce (base64)
    pub encryption_nonce: String,

    /// When this export was created
    pub exported_at: String,

    /// Doorway that held custody
    pub doorway_id: String,
}

// =============================================================================
// Custodial Key Service
// =============================================================================

/// Service for managing custodial keys.
///
/// Provides key generation, encryption, caching, and export functionality.
pub struct CustodialKeyService {
    /// In-memory cache for decrypted signing keys
    cache: Arc<SigningKeyCache>,
}

impl CustodialKeyService {
    /// Create a new custodial key service with default cache configuration.
    pub fn new() -> Self {
        Self {
            cache: Arc::new(SigningKeyCache::new(SigningKeyCacheConfig::default())),
        }
    }

    /// Create a new custodial key service with custom cache configuration.
    pub fn with_cache_config(config: SigningKeyCacheConfig) -> Self {
        Self {
            cache: Arc::new(SigningKeyCache::new(config)),
        }
    }

    /// Generate and encrypt a new keypair for a user during registration.
    ///
    /// # Arguments
    ///
    /// - `password`: User's password (used to encrypt the private key)
    ///
    /// # Returns
    ///
    /// The `CustodialKeyMaterial` struct ready for MongoDB storage.
    /// The `public_key` field should be used as the user's `agent_pub_key`.
    pub fn generate_key_material(&self, password: &str) -> Result<CustodialKeyMaterial> {
        // 1. Generate Ed25519 keypair
        let (signing_key, verifying_key) = generate_keypair();

        // 2. Generate random salt and nonce
        let salt: [u8; SALT_LEN] = generate_random_bytes();
        let nonce: [u8; NONCE_LEN] = generate_random_bytes();

        // 3. Derive encryption key from password
        let encryption_key = derive_key_encryption_key(password.as_bytes(), &salt)?;

        // 4. Encrypt private key
        let private_key_bytes = signing_key.to_bytes();
        let encrypted = encrypt_private_key(&private_key_bytes, &encryption_key, &nonce)?;

        // 5. Build key material struct
        let key_material = CustodialKeyMaterial {
            public_key: BASE64.encode(verifying_key.to_bytes()),
            encrypted_private_key: BASE64.encode(&encrypted),
            key_derivation_salt: BASE64.encode(salt),
            encryption_nonce: BASE64.encode(nonce),
            created_at: bson::DateTime::now(),
            key_version: 1,
            exported: false,
            exported_at: None,
        };

        debug!(
            public_key = %key_material.public_key,
            "Generated new custodial keypair"
        );

        Ok(key_material)
    }

    /// Decrypt and cache a user's signing key (called at login).
    ///
    /// # Arguments
    ///
    /// - `session_id`: Unique session identifier (for cache key)
    /// - `user`: The user document containing encrypted key material
    /// - `password`: User's password (to decrypt the key)
    ///
    /// # Returns
    ///
    /// The user's public key (verifying key) on success.
    pub fn activate_key(
        &self,
        session_id: &str,
        user: &UserDoc,
        password: &str,
    ) -> Result<VerifyingKey> {
        let key_material = user
            .custodial_key
            .as_ref()
            .ok_or_else(|| DoorwayError::Auth("User has no custodial key".into()))?;

        // Block activation for steward users in normal login flow.
        // For disaster recovery, the recovery endpoint first:
        // 1. Validates recovery approval (M-of-N votes, Elohim check)
        // 2. Generates a NEW custodial key
        // 3. Updates user: custodial_key = new_key, is_steward = false
        // 4. Then calls activate_key (which passes because is_steward is now false)
        if user.is_steward {
            return Err(DoorwayError::Auth(
                "User has migrated to stewardship - use recovery flow to regain custody".into(),
            ));
        }

        // 1. Decode stored values
        let salt = BASE64
            .decode(&key_material.key_derivation_salt)
            .map_err(|e| DoorwayError::Internal(format!("Invalid salt encoding: {e}")))?;

        let nonce = BASE64
            .decode(&key_material.encryption_nonce)
            .map_err(|e| DoorwayError::Internal(format!("Invalid nonce encoding: {e}")))?;

        let encrypted = BASE64
            .decode(&key_material.encrypted_private_key)
            .map_err(|e| DoorwayError::Internal(format!("Invalid ciphertext encoding: {e}")))?;

        // 2. Derive encryption key from password
        let encryption_key = derive_key_encryption_key(password.as_bytes(), &salt)?;

        // 3. Decrypt private key
        let nonce_arr: [u8; NONCE_LEN] = nonce
            .try_into()
            .map_err(|_| DoorwayError::Internal("Invalid nonce length".into()))?;

        let private_key_bytes = decrypt_private_key(&encrypted, &encryption_key, &nonce_arr)?;

        // 4. Reconstruct signing key
        let signing_key = SigningKey::from_bytes(&private_key_bytes);
        let verifying_key = signing_key.verifying_key();

        // 5. Cache the signing key
        self.cache
            .insert(session_id.to_string(), signing_key, user.human_id.clone());

        debug!(
            session_id = %session_id,
            human_id = %user.human_id,
            "Activated custodial signing key"
        );

        Ok(verifying_key)
    }

    /// Get a signing key from the cache.
    ///
    /// Returns None if the session doesn't have a cached key or it expired.
    pub fn get_signing_key(&self, session_id: &str) -> Option<SigningKey> {
        self.cache.get(session_id)
    }

    /// Check if a session has a cached signing key.
    pub fn has_signing_key(&self, session_id: &str) -> bool {
        self.cache.contains(session_id)
    }

    /// Sign data with the user's cached key.
    ///
    /// # Arguments
    ///
    /// - `session_id`: Session identifier to look up the cached key
    /// - `data`: Data to sign
    ///
    /// # Returns
    ///
    /// 64-byte Ed25519 signature.
    pub fn sign(&self, session_id: &str, data: &[u8]) -> Result<Vec<u8>> {
        let signing_key = self
            .cache
            .get(session_id)
            .ok_or_else(|| DoorwayError::Auth("Session key not found in cache".into()))?;

        let signature = super::crypto::sign_payload(&signing_key, data);
        Ok(signature.to_bytes().to_vec())
    }

    /// Deactivate a key (called at logout).
    ///
    /// Removes the key from the cache (zeroized on drop).
    pub fn deactivate_key(&self, session_id: &str) {
        if self.cache.remove(session_id) {
            debug!(session_id = %session_id, "Deactivated custodial signing key");
        }
    }

    /// Deactivate all keys for a user (logout all sessions).
    pub fn deactivate_all(&self, human_id: &str) {
        let removed = self.cache.remove_human(human_id);
        if removed > 0 {
            info!(
                human_id = %human_id,
                sessions_removed = removed,
                "Deactivated all custodial signing keys for user"
            );
        }
    }

    /// Export key material for migration to Tauri (stewardship).
    ///
    /// The exported bundle still has the private key encrypted - the user
    /// must provide their password to the Tauri app to decrypt it.
    ///
    /// # Arguments
    ///
    /// - `user`: User document with custodial key
    /// - `doorway_id`: ID of this doorway (for audit trail)
    pub fn export_key(&self, user: &UserDoc, doorway_id: &str) -> Result<KeyExportFormat> {
        let key_material = user
            .custodial_key
            .as_ref()
            .ok_or_else(|| DoorwayError::Auth("User has no custodial key to export".into()))?;

        if user.is_steward {
            return Err(DoorwayError::Auth(
                "User has already migrated to stewardship".into(),
            ));
        }

        let export = KeyExportFormat {
            version: 1,
            identifier: user.identifier.clone(),
            human_id: user.human_id.clone(),
            public_key: key_material.public_key.clone(),
            encrypted_private_key: key_material.encrypted_private_key.clone(),
            key_derivation_salt: key_material.key_derivation_salt.clone(),
            encryption_nonce: key_material.encryption_nonce.clone(),
            exported_at: chrono::Utc::now().to_rfc3339(),
            doorway_id: doorway_id.to_string(),
        };

        info!(
            human_id = %user.human_id,
            identifier = %user.identifier,
            "Exported custodial key for migration to stewardship"
        );

        Ok(export)
    }

    /// Run periodic cleanup of expired cache entries.
    ///
    /// Should be called periodically (e.g., every minute) to remove
    /// expired sessions from the cache.
    pub fn cleanup(&self) -> usize {
        let removed = self.cache.cleanup();
        if removed > 0 {
            debug!(entries_removed = removed, "Cleaned up expired signing keys");
        }
        removed
    }

    /// Get current cache size.
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Get cache statistics.
    pub fn cache_stats(&self) -> super::cache::CacheStatsSnapshot {
        self.cache.stats()
    }
}

impl Default for CustodialKeyService {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_user(service: &CustodialKeyService, password: &str) -> UserDoc {
        let key_material = service.generate_key_material(password).unwrap();
        UserDoc::new_with_custodial_key(
            "test@example.com".to_string(),
            "email".to_string(),
            "password_hash".to_string(),
            "human-123".to_string(),
            key_material,
        )
    }

    #[test]
    fn test_generate_key_material() {
        let service = CustodialKeyService::new();
        let password = "test-password-123";

        let key_material = service.generate_key_material(password).unwrap();

        // Check all fields are populated
        assert!(!key_material.public_key.is_empty());
        assert!(!key_material.encrypted_private_key.is_empty());
        assert!(!key_material.key_derivation_salt.is_empty());
        assert!(!key_material.encryption_nonce.is_empty());
        assert_eq!(key_material.key_version, 1);
        assert!(!key_material.exported);

        // Public key should be valid base64 (32 bytes = ~44 chars)
        let pub_bytes = BASE64.decode(&key_material.public_key).unwrap();
        assert_eq!(pub_bytes.len(), 32);
    }

    #[test]
    fn test_activate_and_sign() {
        let service = CustodialKeyService::new();
        let password = "my-secure-password";
        let user = create_test_user(&service, password);

        // Activate key
        let session_id = "session-123";
        let verifying_key = service.activate_key(session_id, &user, password).unwrap();

        // Key should be in cache
        assert!(service.has_signing_key(session_id));

        // Sign some data
        let message = b"Hello, Holochain!";
        let signature = service.sign(session_id, message).unwrap();

        // Signature should be 64 bytes
        assert_eq!(signature.len(), 64);

        // Verify signature
        use ed25519_dalek::Verifier;
        let sig = ed25519_dalek::Signature::from_bytes(signature.as_slice().try_into().unwrap());
        assert!(verifying_key.verify(message, &sig).is_ok());
    }

    #[test]
    fn test_activate_wrong_password_fails() {
        let service = CustodialKeyService::new();
        let correct_password = "correct-password";
        let wrong_password = "wrong-password";
        let user = create_test_user(&service, correct_password);

        // Try to activate with wrong password
        let result = service.activate_key("session-123", &user, wrong_password);
        assert!(result.is_err());
    }

    #[test]
    fn test_deactivate_key() {
        let service = CustodialKeyService::new();
        let password = "test-password";
        let user = create_test_user(&service, password);

        let session_id = "session-123";
        service.activate_key(session_id, &user, password).unwrap();

        assert!(service.has_signing_key(session_id));

        service.deactivate_key(session_id);

        assert!(!service.has_signing_key(session_id));
    }

    #[test]
    fn test_deactivate_all_sessions() {
        let service = CustodialKeyService::new();
        let password = "test-password";
        let user = create_test_user(&service, password);

        // Activate multiple sessions for same user
        service.activate_key("session-1", &user, password).unwrap();
        service.activate_key("session-2", &user, password).unwrap();
        service.activate_key("session-3", &user, password).unwrap();

        assert_eq!(service.cache_size(), 3);

        // Deactivate all
        service.deactivate_all(&user.human_id);

        assert_eq!(service.cache_size(), 0);
    }

    #[test]
    fn test_export_key() {
        let service = CustodialKeyService::new();
        let password = "test-password";
        let user = create_test_user(&service, password);

        let export = service.export_key(&user, "doorway-1").unwrap();

        assert_eq!(export.version, 1);
        assert_eq!(export.identifier, user.identifier);
        assert_eq!(export.human_id, user.human_id);
        assert_eq!(
            export.public_key,
            user.custodial_key.as_ref().unwrap().public_key
        );
        assert_eq!(export.doorway_id, "doorway-1");
        assert!(!export.exported_at.is_empty());
    }

    #[test]
    fn test_steward_user_cannot_activate() {
        let service = CustodialKeyService::new();
        let password = "test-password";
        let mut user = create_test_user(&service, password);

        // Mark as steward
        user.mark_steward();

        // Try to activate - should fail
        let result = service.activate_key("session-123", &user, password);
        assert!(result.is_err());
    }

    #[test]
    fn test_steward_user_cannot_export() {
        let service = CustodialKeyService::new();
        let password = "test-password";
        let mut user = create_test_user(&service, password);

        // Mark as steward
        user.mark_steward();

        // Try to export - should fail
        let result = service.export_key(&user, "doorway-1");
        assert!(result.is_err());
    }
}
