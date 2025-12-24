//! Blob Verification Service - Server-Side SHA256 Verification
//!
//! Provides authoritative blob integrity verification for defense-in-depth:
//! - Primary verification point for client-downloaded content
//! - Fallback when client-side WASM/SubtleCrypto is unavailable
//! - Streaming verification for large files
//!
//! ## Endpoint
//!
//! `POST /api/blob/verify`
//!
//! ## Verification Flow
//!
//! 1. Client downloads blob from custodian/CDN
//! 2. Client computes hash locally (WASM or SubtleCrypto)
//! 3. If local verification unavailable, client sends to server
//! 4. Server verifies and returns authoritative result

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Instant;
use tracing::{debug, warn};

// ============================================================================
// Types
// ============================================================================

/// Request for blob verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyBlobRequest {
    /// Expected SHA256 hash (hex string, 64 chars)
    pub expected_hash: String,

    /// Blob data as base64 (for small blobs)
    #[serde(default)]
    pub data_base64: Option<String>,

    /// URL to fetch blob from (alternative to inline data)
    #[serde(default)]
    pub fetch_url: Option<String>,

    /// Content ID for logging/tracing
    #[serde(default)]
    pub content_id: Option<String>,
}

/// Response from blob verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyBlobResponse {
    /// Whether the hash matches
    pub is_valid: bool,

    /// Computed SHA256 hash (hex string)
    pub computed_hash: String,

    /// Expected hash (echoed back)
    pub expected_hash: String,

    /// Size of data in bytes
    pub size_bytes: u64,

    /// Time taken to verify in milliseconds
    pub duration_ms: u64,

    /// Error message if verification failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl VerifyBlobResponse {
    /// Create a success response
    pub fn success(computed_hash: String, expected_hash: String, size_bytes: u64, duration_ms: u64) -> Self {
        let is_valid = computed_hash.to_lowercase() == expected_hash.to_lowercase();
        Self {
            is_valid,
            computed_hash,
            expected_hash,
            size_bytes,
            duration_ms,
            error: None,
        }
    }

    /// Create an error response
    pub fn error(expected_hash: String, error: String) -> Self {
        Self {
            is_valid: false,
            computed_hash: String::new(),
            expected_hash,
            size_bytes: 0,
            duration_ms: 0,
            error: Some(error),
        }
    }
}

// ============================================================================
// Verification Service
// ============================================================================

/// Service configuration
#[derive(Debug, Clone)]
pub struct VerificationConfig {
    /// Maximum inline data size (default: 50 MB)
    pub max_inline_bytes: u64,
    /// Timeout for URL fetching (seconds)
    pub fetch_timeout_secs: u64,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            max_inline_bytes: 50 * 1024 * 1024, // 50 MB
            fetch_timeout_secs: 30,
        }
    }
}

/// Blob verification service
pub struct VerificationService {
    config: VerificationConfig,
}

impl VerificationService {
    /// Create a new verification service
    pub fn new(config: VerificationConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(VerificationConfig::default())
    }

    /// Verify a blob from inline base64 data
    pub fn verify_base64(&self, data_base64: &str, expected_hash: &str) -> VerifyBlobResponse {
        let start = Instant::now();

        // Decode base64
        let data = match base64_decode(data_base64) {
            Ok(d) => d,
            Err(e) => {
                return VerifyBlobResponse::error(
                    expected_hash.to_string(),
                    format!("Base64 decode failed: {}", e),
                );
            }
        };

        // Check size limit
        if data.len() as u64 > self.config.max_inline_bytes {
            return VerifyBlobResponse::error(
                expected_hash.to_string(),
                format!(
                    "Data too large: {} bytes exceeds {} byte limit",
                    data.len(),
                    self.config.max_inline_bytes
                ),
            );
        }

        // Compute hash
        let computed_hash = compute_sha256(&data);
        let duration_ms = start.elapsed().as_millis() as u64;

        debug!(
            expected = %expected_hash,
            computed = %computed_hash,
            size = data.len(),
            duration_ms = duration_ms,
            "Blob verification completed"
        );

        VerifyBlobResponse::success(
            computed_hash,
            expected_hash.to_string(),
            data.len() as u64,
            duration_ms,
        )
    }

    /// Verify a blob from raw bytes
    pub fn verify_bytes(&self, data: &[u8], expected_hash: &str) -> VerifyBlobResponse {
        let start = Instant::now();

        let computed_hash = compute_sha256(data);
        let duration_ms = start.elapsed().as_millis() as u64;

        debug!(
            expected = %expected_hash,
            computed = %computed_hash,
            size = data.len(),
            duration_ms = duration_ms,
            "Blob verification completed"
        );

        VerifyBlobResponse::success(
            computed_hash,
            expected_hash.to_string(),
            data.len() as u64,
            duration_ms,
        )
    }

    /// Verify a blob from Bytes
    pub fn verify(&self, data: Bytes, expected_hash: &str) -> VerifyBlobResponse {
        self.verify_bytes(&data, expected_hash)
    }

    /// Handle a verification request
    pub async fn handle_request(&self, request: VerifyBlobRequest) -> VerifyBlobResponse {
        // Check if we have inline data
        if let Some(ref data_base64) = request.data_base64 {
            return self.verify_base64(data_base64, &request.expected_hash);
        }

        // Check if we should fetch from URL
        if let Some(ref _fetch_url) = request.fetch_url {
            // TODO: Implement URL fetching when reqwest is added
            return VerifyBlobResponse::error(
                request.expected_hash,
                "URL fetching not yet implemented".to_string(),
            );
        }

        VerifyBlobResponse::error(
            request.expected_hash,
            "Either data_base64 or fetch_url must be provided".to_string(),
        )
    }

    /// Get configuration
    pub fn config(&self) -> &VerificationConfig {
        &self.config
    }
}

impl Default for VerificationService {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Compute SHA256 hash of data
pub fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Decode base64 data (supports both standard and URL-safe)
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    use base64::{engine::general_purpose, Engine as _};

    // Try standard base64 first
    if let Ok(data) = general_purpose::STANDARD.decode(input) {
        return Ok(data);
    }

    // Try URL-safe base64
    if let Ok(data) = general_purpose::URL_SAFE.decode(input) {
        return Ok(data);
    }

    // Try with padding stripped
    if let Ok(data) = general_purpose::STANDARD_NO_PAD.decode(input) {
        return Ok(data);
    }

    if let Ok(data) = general_purpose::URL_SAFE_NO_PAD.decode(input) {
        return Ok(data);
    }

    Err("Invalid base64 encoding".to_string())
}

/// Compute SHA256 hash incrementally for streaming
pub struct StreamingHasher {
    hasher: Sha256,
    bytes_processed: u64,
}

impl StreamingHasher {
    /// Create a new streaming hasher
    pub fn new() -> Self {
        Self {
            hasher: Sha256::new(),
            bytes_processed: 0,
        }
    }

    /// Update with a chunk of data
    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
        self.bytes_processed += data.len() as u64;
    }

    /// Finalize and return the hash
    pub fn finalize(self) -> (String, u64) {
        let hash = hex::encode(self.hasher.finalize());
        (hash, self.bytes_processed)
    }

    /// Get bytes processed so far
    pub fn bytes_processed(&self) -> u64 {
        self.bytes_processed
    }
}

impl Default for StreamingHasher {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sha256() {
        let data = b"Hello, World!";
        let hash = compute_sha256(data);

        // Known SHA256 hash of "Hello, World!"
        assert_eq!(
            hash,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );
    }

    #[test]
    fn test_verify_bytes() {
        let service = VerificationService::with_defaults();
        let data = b"Test data for verification";
        let expected_hash = compute_sha256(data);

        let result = service.verify_bytes(data, &expected_hash);
        assert!(result.is_valid);
        assert_eq!(result.computed_hash, expected_hash);
        assert_eq!(result.size_bytes, data.len() as u64);
    }

    #[test]
    fn test_verify_bytes_mismatch() {
        let service = VerificationService::with_defaults();
        let data = b"Test data";
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";

        let result = service.verify_bytes(data, wrong_hash);
        assert!(!result.is_valid);
        assert_ne!(result.computed_hash, wrong_hash);
    }

    #[test]
    fn test_verify_base64() {
        use base64::{engine::general_purpose, Engine as _};

        let service = VerificationService::with_defaults();
        let data = b"Base64 test data";
        let data_base64 = general_purpose::STANDARD.encode(data);
        let expected_hash = compute_sha256(data);

        let result = service.verify_base64(&data_base64, &expected_hash);
        assert!(result.is_valid);
        assert_eq!(result.computed_hash, expected_hash);
    }

    #[test]
    fn test_streaming_hasher() {
        let mut hasher = StreamingHasher::new();

        hasher.update(b"Hello, ");
        hasher.update(b"World!");

        let (hash, size) = hasher.finalize();

        assert_eq!(
            hash,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );
        assert_eq!(size, 13);
    }

    #[test]
    fn test_base64_decode_variants() {
        use base64::{engine::general_purpose, Engine as _};

        let data = b"Test";

        // Standard
        let encoded = general_purpose::STANDARD.encode(data);
        assert_eq!(base64_decode(&encoded).unwrap(), data);

        // URL-safe
        let encoded = general_purpose::URL_SAFE.encode(data);
        assert_eq!(base64_decode(&encoded).unwrap(), data);

        // No padding
        let encoded = general_purpose::STANDARD_NO_PAD.encode(data);
        assert_eq!(base64_decode(&encoded).unwrap(), data);
    }
}
