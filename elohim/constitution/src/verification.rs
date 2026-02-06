//! DHT verification for constitutional documents.
//!
//! This module provides read-only verification of constitutional document
//! hashes against the Holochain DHT. Documents are created and managed
//! through the Holochain zome; this module only verifies integrity.

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::VerificationResult;

/// Error types for DHT verification operations.
#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    /// DHT is not reachable
    #[error("DHT unavailable: {0}")]
    DhtUnavailable(String),

    /// Network error during verification
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Invalid response from DHT
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Document not found in DHT
    #[error("Document not found: {0}")]
    NotFound(String),
}

/// Trait for verifying constitutional documents against DHT.
///
/// This is a clean abstraction over the DHT verification mechanism,
/// allowing for different implementations (live, offline, mock).
#[async_trait]
pub trait DhtVerifier: Send + Sync {
    /// Verify a document hash against the DHT.
    ///
    /// Returns verification result including whether the hash matches.
    async fn verify(
        &self,
        document_id: &str,
        expected_hash: &str,
    ) -> Result<VerificationResult, VerificationError>;

    /// Get the current version of a document from DHT.
    async fn get_current_version(
        &self,
        document_id: &str,
    ) -> Result<Option<String>, VerificationError>;

    /// Check if DHT is reachable.
    async fn is_available(&self) -> bool;

    /// Get the hash of document content stored in DHT.
    async fn get_hash(&self, document_id: &str) -> Result<Option<String>, VerificationError>;
}

/// Read-only Holochain DHT verifier.
///
/// Connects to a Holochain conductor to verify constitutional document hashes.
/// This implementation only reads; it does not write to the DHT.
pub struct HolochainDhtVerifier {
    /// Conductor WebSocket URL
    conductor_url: String,
    /// Cell ID for the constitution DNA
    cell_id: String,
    /// Connection state
    connected: Arc<RwLock<bool>>,
}

impl HolochainDhtVerifier {
    /// Create a new DHT verifier.
    pub fn new(conductor_url: String, cell_id: String) -> Self {
        Self {
            conductor_url,
            cell_id,
            connected: Arc::new(RwLock::new(false)),
        }
    }

    /// Connect to the conductor.
    pub async fn connect(&self) -> Result<(), VerificationError> {
        // TODO: Implement actual WebSocket connection to conductor
        // using holochain_client crate
        tracing::info!(
            url = %self.conductor_url,
            cell = %self.cell_id,
            "Connecting to Holochain conductor"
        );

        let mut connected = self.connected.write().await;
        *connected = true;

        Ok(())
    }

    /// Call a zome function (internal helper).
    async fn call_zome<T: serde::de::DeserializeOwned>(
        &self,
        fn_name: &str,
        payload: impl serde::Serialize,
    ) -> Result<T, VerificationError> {
        // TODO: Implement actual zome call
        // This would use holochain_client::AppWebsocket
        let _payload_json =
            serde_json::to_value(payload).map_err(|e| VerificationError::InvalidResponse(e.to_string()))?;

        tracing::debug!(fn_name = %fn_name, "Calling zome function");

        Err(VerificationError::DhtUnavailable(
            "Zome calls not yet implemented".to_string(),
        ))
    }
}

#[async_trait]
impl DhtVerifier for HolochainDhtVerifier {
    async fn verify(
        &self,
        document_id: &str,
        expected_hash: &str,
    ) -> Result<VerificationResult, VerificationError> {
        let connected = self.connected.read().await;
        if !*connected {
            return Err(VerificationError::DhtUnavailable(
                "Not connected to conductor".to_string(),
            ));
        }
        drop(connected);

        // Call zome function to get document hash
        let actual_hash: Option<String> = self
            .call_zome("get_constitution_hash", document_id)
            .await
            .ok();

        let verified = actual_hash.as_ref() == Some(&expected_hash.to_string());

        Ok(VerificationResult {
            document_id: document_id.to_string(),
            expected_hash: expected_hash.to_string(),
            actual_hash,
            verified,
            verified_at: chrono::Utc::now(),
            dht_source: Some(self.cell_id.clone()),
        })
    }

    async fn get_current_version(
        &self,
        document_id: &str,
    ) -> Result<Option<String>, VerificationError> {
        self.call_zome("get_constitution_version", document_id)
            .await
    }

    async fn is_available(&self) -> bool {
        let connected = self.connected.read().await;
        *connected
    }

    async fn get_hash(&self, document_id: &str) -> Result<Option<String>, VerificationError> {
        self.call_zome("get_constitution_hash", document_id).await
    }
}

/// Offline verifier using cached hashes.
///
/// Used when DHT is unavailable. Verifies against locally cached hashes
/// that were previously retrieved from the DHT.
pub struct OfflineDhtVerifier {
    /// Cached document hashes
    cached_hashes: Arc<RwLock<HashMap<String, CachedHash>>>,
}

/// A cached hash with metadata.
#[derive(Debug, Clone)]
struct CachedHash {
    hash: String,
    version: String,
    cached_at: chrono::DateTime<chrono::Utc>,
}

impl OfflineDhtVerifier {
    /// Create a new offline verifier.
    pub fn new() -> Self {
        Self {
            cached_hashes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load cached hashes from a file or previous session.
    pub async fn load_cache(&self, hashes: HashMap<String, (String, String)>) {
        let mut cache = self.cached_hashes.write().await;
        for (doc_id, (hash, version)) in hashes {
            cache.insert(
                doc_id,
                CachedHash {
                    hash,
                    version,
                    cached_at: chrono::Utc::now(),
                },
            );
        }
    }

    /// Cache a hash for later offline verification.
    pub async fn cache_hash(&self, document_id: &str, hash: &str, version: &str) {
        let mut cache = self.cached_hashes.write().await;
        cache.insert(
            document_id.to_string(),
            CachedHash {
                hash: hash.to_string(),
                version: version.to_string(),
                cached_at: chrono::Utc::now(),
            },
        );
    }
}

impl Default for OfflineDhtVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DhtVerifier for OfflineDhtVerifier {
    async fn verify(
        &self,
        document_id: &str,
        expected_hash: &str,
    ) -> Result<VerificationResult, VerificationError> {
        let cache = self.cached_hashes.read().await;

        let actual_hash = cache.get(document_id).map(|c| c.hash.clone());
        let verified = actual_hash.as_ref() == Some(&expected_hash.to_string());

        Ok(VerificationResult {
            document_id: document_id.to_string(),
            expected_hash: expected_hash.to_string(),
            actual_hash,
            verified,
            verified_at: chrono::Utc::now(),
            dht_source: Some("offline-cache".to_string()),
        })
    }

    async fn get_current_version(
        &self,
        document_id: &str,
    ) -> Result<Option<String>, VerificationError> {
        let cache = self.cached_hashes.read().await;
        Ok(cache.get(document_id).map(|c| c.version.clone()))
    }

    async fn is_available(&self) -> bool {
        true // Offline cache is always "available"
    }

    async fn get_hash(&self, document_id: &str) -> Result<Option<String>, VerificationError> {
        let cache = self.cached_hashes.read().await;
        Ok(cache.get(document_id).map(|c| c.hash.clone()))
    }
}

/// Compute SHA256 hash of content.
pub fn compute_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    hex::encode(hasher.finalize())
}

/// Compute hash of a constitutional document's content.
pub fn hash_document_content(content: &crate::types::ConstitutionalContent) -> String {
    let json = serde_json::to_string(content).unwrap_or_default();
    compute_hash(json.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_offline_verifier() {
        let verifier = OfflineDhtVerifier::new();

        // Cache a hash
        verifier.cache_hash("doc-1", "abc123", "1.0.0").await;

        // Verify correct hash
        let result = verifier.verify("doc-1", "abc123").await.unwrap();
        assert!(result.verified);

        // Verify incorrect hash
        let result = verifier.verify("doc-1", "wrong").await.unwrap();
        assert!(!result.verified);

        // Verify missing document
        let result = verifier.verify("doc-2", "any").await.unwrap();
        assert!(!result.verified);
        assert!(result.actual_hash.is_none());
    }

    #[test]
    fn test_compute_hash() {
        let hash1 = compute_hash(b"hello");
        let hash2 = compute_hash(b"hello");
        let hash3 = compute_hash(b"world");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA256 = 32 bytes = 64 hex chars
    }
}
