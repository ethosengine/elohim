//! DID Resolution Service
//!
//! Resolves W3C DIDs to their DID Documents and extracts service endpoints.
//! Supports did:web and did:key methods for doorway federation.
//!
//! See holochain/doorway/DID-FEDERATION.md for architecture details.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Configuration for the DID resolver
#[derive(Debug, Clone)]
pub struct DIDResolverConfig {
    /// Cache TTL for resolved DID documents (default: 5 minutes)
    pub cache_ttl: Duration,
    /// Timeout for HTTP requests (default: 5 seconds)
    pub request_timeout: Duration,
    /// Maximum cache entries (default: 1000)
    pub max_cache_entries: usize,
}

impl Default for DIDResolverConfig {
    fn default() -> Self {
        Self {
            cache_ttl: Duration::from_secs(300),
            request_timeout: Duration::from_secs(5),
            max_cache_entries: 1000,
        }
    }
}

/// Cached DID document with expiration
struct CachedDocument {
    document: DIDDocument,
    expires_at: Instant,
}

/// DID Resolution Service
///
/// Resolves DIDs to their documents and caches results.
pub struct DIDResolver {
    config: DIDResolverConfig,
    cache: RwLock<HashMap<String, CachedDocument>>,
    http_client: reqwest::Client,
}

impl DIDResolver {
    /// Create a new DID resolver with default configuration
    pub fn new() -> Self {
        Self::with_config(DIDResolverConfig::default())
    }

    /// Create a new DID resolver with custom configuration
    pub fn with_config(config: DIDResolverConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(config.request_timeout)
            .user_agent("elohim-doorway/1.0")
            .build()
            .unwrap_or_default();

        Self {
            config,
            cache: RwLock::new(HashMap::new()),
            http_client,
        }
    }

    /// Resolve a DID to its DID Document
    pub async fn resolve(&self, did: &str) -> Result<DIDDocument, DIDResolverError> {
        // Check cache first
        if let Some(doc) = self.get_cached(did).await {
            debug!(did = %did, "DID resolved from cache");
            return Ok(doc);
        }

        // Resolve based on DID method
        let document = if did.starts_with("did:web:") {
            self.resolve_did_web(did).await?
        } else if did.starts_with("did:key:") {
            self.resolve_did_key(did)?
        } else {
            return Err(DIDResolverError::UnsupportedMethod(did.to_string()));
        };

        // Cache the result
        self.cache_document(did, document.clone()).await;

        Ok(document)
    }

    /// Get a cached DID document if still valid
    async fn get_cached(&self, did: &str) -> Option<DIDDocument> {
        let cache = self.cache.read().await;
        cache.get(did).and_then(|cached| {
            if cached.expires_at > Instant::now() {
                Some(cached.document.clone())
            } else {
                None
            }
        })
    }

    /// Cache a DID document
    async fn cache_document(&self, did: &str, document: DIDDocument) {
        let mut cache = self.cache.write().await;

        // Evict oldest entries if cache is full
        if cache.len() >= self.config.max_cache_entries {
            // Simple eviction: remove expired entries first
            cache.retain(|_, v| v.expires_at > Instant::now());

            // If still too full, just clear half (simple LRU approximation)
            if cache.len() >= self.config.max_cache_entries {
                let to_remove: Vec<_> = cache.keys().take(cache.len() / 2).cloned().collect();
                for key in to_remove {
                    cache.remove(&key);
                }
            }
        }

        cache.insert(
            did.to_string(),
            CachedDocument {
                document,
                expires_at: Instant::now() + self.config.cache_ttl,
            },
        );
    }

    /// Resolve a did:web DID
    ///
    /// did:web:example.com → https://example.com/.well-known/did.json
    /// did:web:example.com:path:to → https://example.com/path/to/did.json
    async fn resolve_did_web(&self, did: &str) -> Result<DIDDocument, DIDResolverError> {
        let url = did_web_to_url(did)?;
        debug!(did = %did, url = %url, "Resolving did:web");

        let response = self
            .http_client
            .get(&url)
            .header("Accept", "application/did+ld+json, application/json")
            .send()
            .await
            .map_err(|e| DIDResolverError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DIDResolverError::ResolutionFailed(format!(
                "HTTP {} from {}",
                response.status(),
                url
            )));
        }

        let document: DIDDocument = response
            .json()
            .await
            .map_err(|e| DIDResolverError::InvalidDocument(e.to_string()))?;

        // Validate that the document ID matches the requested DID
        if document.id != did {
            warn!(
                expected = %did,
                actual = %document.id,
                "DID document ID mismatch"
            );
            // Allow mismatch but log it - some implementations may have minor variations
        }

        Ok(document)
    }

    /// Resolve a did:key DID
    ///
    /// did:key DIDs are self-describing - the public key is encoded in the DID itself.
    /// This creates a minimal DID document with the embedded key.
    fn resolve_did_key(&self, did: &str) -> Result<DIDDocument, DIDResolverError> {
        // did:key:z6Mk... → extract the multibase-encoded public key
        let key_part = did
            .strip_prefix("did:key:")
            .ok_or_else(|| DIDResolverError::InvalidDID("Missing did:key: prefix".to_string()))?;

        // Validate it looks like a valid multibase key (starts with 'z' for base58btc)
        if !key_part.starts_with('z') {
            return Err(DIDResolverError::InvalidDID(
                "did:key must use base58btc encoding (z prefix)".to_string(),
            ));
        }

        // Build a minimal DID document
        // Note: Full did:key resolution would decode the key and determine its type
        // For now, we assume Ed25519 which is most common in Holochain context
        Ok(DIDDocument {
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
            ],
            id: did.to_string(),
            verification_method: vec![VerificationMethod {
                id: format!("{did}#{key_part}"),
                method_type: "Ed25519VerificationKey2020".to_string(),
                controller: did.to_string(),
                public_key_multibase: Some(key_part.to_string()),
            }],
            authentication: vec![format!("{}#{}", did, key_part)],
            assertion_method: vec![format!("{}#{}", did, key_part)],
            service: vec![],
            elohim_capabilities: None,
            elohim_region: None,
        })
    }

    /// Extract a specific service endpoint from a DID document
    pub fn extract_service_endpoint(document: &DIDDocument, service_type: &str) -> Option<String> {
        document
            .service
            .iter()
            .find(|s| s.service_type == service_type)
            .map(|s| s.service_endpoint.clone())
    }

    /// Extract the blob storage endpoint from a DID document
    pub fn extract_blob_endpoint(document: &DIDDocument) -> Option<String> {
        Self::extract_service_endpoint(document, "ElohimBlobStore")
    }

    /// Extract the Holochain gateway endpoint from a DID document
    pub fn extract_holochain_endpoint(document: &DIDDocument) -> Option<String> {
        Self::extract_service_endpoint(document, "HolochainGateway")
    }

    /// Clear the cache (useful for testing)
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> DIDResolverStats {
        let cache = self.cache.read().await;
        let now = Instant::now();
        let valid_entries = cache.values().filter(|v| v.expires_at > now).count();

        DIDResolverStats {
            total_entries: cache.len(),
            valid_entries,
            expired_entries: cache.len() - valid_entries,
        }
    }
}

impl Default for DIDResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a did:web DID to its resolution URL
fn did_web_to_url(did: &str) -> Result<String, DIDResolverError> {
    let without_prefix = did
        .strip_prefix("did:web:")
        .ok_or_else(|| DIDResolverError::InvalidDID("Missing did:web: prefix".to_string()))?;

    let parts: Vec<&str> = without_prefix.split(':').collect();
    if parts.is_empty() {
        return Err(DIDResolverError::InvalidDID(
            "Empty domain in did:web".to_string(),
        ));
    }

    // First part is the domain (with percent-encoding for special chars)
    let domain = parts[0].replace("%3A", ":");

    // Remaining parts form the path
    let path = if parts.len() > 1 {
        format!("/{}/did.json", parts[1..].join("/"))
    } else {
        "/.well-known/did.json".to_string()
    };

    Ok(format!("https://{domain}{path}"))
}

/// DID Document structure (simplified for resolution)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDDocument {
    /// JSON-LD context
    #[serde(rename = "@context", default)]
    pub context: Vec<String>,

    /// The DID this document describes
    pub id: String,

    /// Verification methods (public keys)
    #[serde(rename = "verificationMethod", default)]
    pub verification_method: Vec<VerificationMethod>,

    /// Authentication verification method references
    #[serde(default)]
    pub authentication: Vec<String>,

    /// Assertion method references
    #[serde(rename = "assertionMethod", default)]
    pub assertion_method: Vec<String>,

    /// Service endpoints
    #[serde(default)]
    pub service: Vec<Service>,

    /// Elohim-specific capabilities
    #[serde(
        rename = "elohim:capabilities",
        skip_serializing_if = "Option::is_none"
    )]
    pub elohim_capabilities: Option<Vec<String>>,

    /// Elohim-specific region
    #[serde(rename = "elohim:region", skip_serializing_if = "Option::is_none")]
    pub elohim_region: Option<String>,
}

/// Verification method in DID Document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    #[serde(rename = "type")]
    pub method_type: String,
    pub controller: String,
    #[serde(rename = "publicKeyMultibase", skip_serializing_if = "Option::is_none")]
    pub public_key_multibase: Option<String>,
}

/// Service endpoint in DID Document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    #[serde(rename = "type")]
    pub service_type: String,
    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: String,
}

/// DID resolver statistics
#[derive(Debug, Clone)]
pub struct DIDResolverStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub expired_entries: usize,
}

/// Errors from DID resolution
#[derive(Debug, thiserror::Error)]
pub enum DIDResolverError {
    #[error("Invalid DID: {0}")]
    InvalidDID(String),

    #[error("Unsupported DID method: {0}")]
    UnsupportedMethod(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Resolution failed: {0}")]
    ResolutionFailed(String),

    #[error("Invalid DID document: {0}")]
    InvalidDocument(String),
}

/// Create a shared DID resolver instance
pub fn create_resolver() -> Arc<DIDResolver> {
    Arc::new(DIDResolver::new())
}

/// Create a shared DID resolver with custom configuration
pub fn create_resolver_with_config(config: DIDResolverConfig) -> Arc<DIDResolver> {
    Arc::new(DIDResolver::with_config(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_did_web_to_url_simple() {
        assert_eq!(
            did_web_to_url("did:web:example.com").unwrap(),
            "https://example.com/.well-known/did.json"
        );
    }

    #[test]
    fn test_did_web_to_url_with_path() {
        assert_eq!(
            did_web_to_url("did:web:example.com:users:alice").unwrap(),
            "https://example.com/users/alice/did.json"
        );
    }

    #[test]
    fn test_did_web_to_url_doorway() {
        assert_eq!(
            did_web_to_url("did:web:doorway-a.elohim.host").unwrap(),
            "https://doorway-a.elohim.host/.well-known/did.json"
        );
    }

    #[test]
    fn test_resolve_did_key_creates_document() {
        let resolver = DIDResolver::new();
        let did = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
        let doc = resolver.resolve_did_key(did).unwrap();

        assert_eq!(doc.id, did);
        assert_eq!(doc.verification_method.len(), 1);
        assert!(doc.verification_method[0]
            .public_key_multibase
            .as_ref()
            .unwrap()
            .starts_with('z'));
    }

    #[test]
    fn test_extract_blob_endpoint() {
        let doc = DIDDocument {
            context: vec![],
            id: "did:web:test.example.com".to_string(),
            verification_method: vec![],
            authentication: vec![],
            assertion_method: vec![],
            service: vec![
                Service {
                    id: "did:web:test.example.com#blobs".to_string(),
                    service_type: "ElohimBlobStore".to_string(),
                    service_endpoint: "https://test.example.com/api/v1/blobs".to_string(),
                },
                Service {
                    id: "did:web:test.example.com#holochain".to_string(),
                    service_type: "HolochainGateway".to_string(),
                    service_endpoint: "wss://test.example.com/app/4445".to_string(),
                },
            ],
            elohim_capabilities: None,
            elohim_region: None,
        };

        assert_eq!(
            DIDResolver::extract_blob_endpoint(&doc),
            Some("https://test.example.com/api/v1/blobs".to_string())
        );
        assert_eq!(
            DIDResolver::extract_holochain_endpoint(&doc),
            Some("wss://test.example.com/app/4445".to_string())
        );
    }
}
