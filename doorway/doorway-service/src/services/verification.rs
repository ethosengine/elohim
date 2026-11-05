//! Blob Verification Service - server-side check of bytes against their CID
//!
//! Provides authoritative blob integrity verification for defense-in-depth:
//! - Primary verification point for client-downloaded content
//! - Fallback when client-side WASM/SubtleCrypto is unavailable
//!
//! ## Endpoint
//!
//! `POST /api/blob/verify` with `{"expected_cid", "data_base64" | "fetch_url"}`.
//!
//! ## What is checked
//!
//! The claim is a CID, and the CID names its own profile: the bytes are hashed under
//! the `(codec, multihash, digest length)` the claimed CID declares, through
//! [`elohim_address::verify_cid_any`] — raw + BLAKE3-256 (bytes), raw + sha2-256
//! (citations), dag-cbor + sha2-256 (records). Any other profile is answered as a typed
//! `unsupported-profile` outcome: neither verified nor refuted, and never `is_valid`.
//! Text that is not a CID at all (`sha256-<hex>`, bare hex) is an error, not a
//! mismatch.

use bytes::Bytes;
use cid::Cid;
use elohim_address::{compute_bytes_cid, verify_cid_any, Verify};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Instant;
use tracing::{debug, warn};

// ============================================================================
// Types
// ============================================================================

/// Request for blob verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyBlobRequest {
    /// The CID the bytes are claimed to have (normally a byte CID, `bafkr4i…`)
    pub expected_cid: String,

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

/// What a verification established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum VerifyOutcome {
    /// The bytes hash, under the claimed CID's own profile, to its digest.
    Verified,
    /// The profile is known and the digest differs.
    Mismatch,
    /// The claimed CID's profile is not one this doorway checks; nothing was decided.
    UnsupportedProfile {
        codec: u64,
        multihash: u64,
        len: usize,
    },
    /// No verification ran (bad request, unreadable data, fetch failure, …).
    Error,
}

impl From<Verify> for VerifyOutcome {
    fn from(value: Verify) -> Self {
        match value {
            Verify::Verified => VerifyOutcome::Verified,
            Verify::Mismatch => VerifyOutcome::Mismatch,
            Verify::UnsupportedProfile {
                codec,
                multihash,
                len,
            } => VerifyOutcome::UnsupportedProfile {
                codec,
                multihash,
                len,
            },
        }
    }
}

/// Response from blob verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyBlobResponse {
    /// `true` only when the outcome is [`VerifyOutcome::Verified`]
    pub is_valid: bool,

    /// What the check established
    pub outcome: VerifyOutcome,

    /// The byte CID of the data received (byte profile), when data was read
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computed_cid: Option<String>,

    /// Expected CID (echoed back)
    pub expected_cid: String,

    /// Size of data in bytes
    pub size_bytes: u64,

    /// Time taken to verify in milliseconds
    pub duration_ms: u64,

    /// Error message if verification failed to run
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl VerifyBlobResponse {
    /// The response for a check that ran over `data`.
    pub fn checked(expected: &Cid, data: &[u8], duration_ms: u64) -> Self {
        let outcome = VerifyOutcome::from(verify_cid_any(expected, data));
        Self {
            is_valid: outcome == VerifyOutcome::Verified,
            outcome,
            computed_cid: Some(compute_bytes_cid(data).to_string()),
            expected_cid: expected.to_string(),
            size_bytes: data.len() as u64,
            duration_ms,
            error: None,
        }
    }

    /// Create an error response
    pub fn error(expected_cid: String, error: String) -> Self {
        Self {
            is_valid: false,
            outcome: VerifyOutcome::Error,
            computed_cid: None,
            expected_cid,
            size_bytes: 0,
            duration_ms: 0,
            error: Some(error),
        }
    }
}

/// Parse the claimed CID, or answer the error response. A legacy `sha256-<hex>` or
/// bare hex is not a CID and is refused here — never compared.
fn parse_expected(expected_cid: &str) -> Result<Cid, VerifyBlobResponse> {
    Cid::from_str(expected_cid).map_err(|e| {
        VerifyBlobResponse::error(
            expected_cid.to_string(),
            format!("expected_cid is not a CID: {e}"),
        )
    })
}

// ============================================================================
// Verification Service
// ============================================================================

/// Service configuration
#[derive(Debug, Clone)]
pub struct VerificationConfig {
    /// Maximum inline data size (default: 50 MB)
    pub max_inline_bytes: u64,
    /// Maximum blob size to fetch from URL (default: 500 MB)
    pub max_fetch_bytes: u64,
    /// Timeout for URL fetching (seconds)
    pub fetch_timeout_secs: u64,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            max_inline_bytes: 50 * 1024 * 1024, // 50 MB
            max_fetch_bytes: 500 * 1024 * 1024, // 500 MB
            fetch_timeout_secs: 30,
        }
    }
}

/// Blob verification service
pub struct VerificationService {
    config: VerificationConfig,
    /// HTTP client for fetching blobs from URLs
    http_client: reqwest::Client,
}

impl VerificationService {
    /// Create a new verification service
    pub fn new(config: VerificationConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            http_client,
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(VerificationConfig::default())
    }

    /// Verify a blob from inline base64 data
    pub fn verify_base64(&self, data_base64: &str, expected_cid: &str) -> VerifyBlobResponse {
        let start = Instant::now();
        let expected = match parse_expected(expected_cid) {
            Ok(cid) => cid,
            Err(response) => return response,
        };

        // Decode base64
        let data = match base64_decode(data_base64) {
            Ok(d) => d,
            Err(e) => {
                return VerifyBlobResponse::error(
                    expected_cid.to_string(),
                    format!("Base64 decode failed: {e}"),
                );
            }
        };

        // Check size limit
        if data.len() as u64 > self.config.max_inline_bytes {
            return VerifyBlobResponse::error(
                expected_cid.to_string(),
                format!(
                    "Data too large: {} bytes exceeds {} byte limit",
                    data.len(),
                    self.config.max_inline_bytes
                ),
            );
        }

        let response =
            VerifyBlobResponse::checked(&expected, &data, start.elapsed().as_millis() as u64);
        debug!(
            expected = %expected_cid,
            outcome = ?response.outcome,
            size = data.len(),
            duration_ms = response.duration_ms,
            "Blob verification completed"
        );
        response
    }

    /// Verify a blob from raw bytes
    pub fn verify_bytes(&self, data: &[u8], expected_cid: &str) -> VerifyBlobResponse {
        let start = Instant::now();
        let expected = match parse_expected(expected_cid) {
            Ok(cid) => cid,
            Err(response) => return response,
        };
        let response =
            VerifyBlobResponse::checked(&expected, data, start.elapsed().as_millis() as u64);
        debug!(
            expected = %expected_cid,
            outcome = ?response.outcome,
            size = data.len(),
            duration_ms = response.duration_ms,
            "Blob verification completed"
        );
        response
    }

    /// Verify a blob from Bytes
    pub fn verify(&self, data: Bytes, expected_cid: &str) -> VerifyBlobResponse {
        self.verify_bytes(&data, expected_cid)
    }

    /// Verify a blob by fetching it from a URL
    ///
    /// Fetches the blob via HTTP GET and checks it under the expected CID's profile.
    /// Respects max_blob_size_bytes configuration to prevent memory exhaustion.
    pub async fn verify_from_url(&self, url: &str, expected_cid: &str) -> VerifyBlobResponse {
        let start = Instant::now();
        let expected = match parse_expected(expected_cid) {
            Ok(cid) => cid,
            Err(response) => return response,
        };

        // Fetch the blob
        let response = match self.http_client.get(url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                warn!("Failed to fetch blob from {}: {}", url, e);
                return VerifyBlobResponse::error(
                    expected_cid.to_string(),
                    format!("Failed to fetch blob: {e}"),
                );
            }
        };

        // Check response status
        if !response.status().is_success() {
            return VerifyBlobResponse::error(
                expected_cid.to_string(),
                format!("HTTP {} fetching blob", response.status()),
            );
        }

        // Check content length if available
        if let Some(content_length) = response.content_length() {
            if content_length > self.config.max_fetch_bytes {
                return VerifyBlobResponse::error(
                    expected_cid.to_string(),
                    format!(
                        "Blob too large: {} bytes (max: {})",
                        content_length, self.config.max_fetch_bytes
                    ),
                );
            }
        }

        // Read the body
        let data = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!("Failed to read blob body from {}: {}", url, e);
                return VerifyBlobResponse::error(
                    expected_cid.to_string(),
                    format!("Failed to read blob body: {e}"),
                );
            }
        };

        // Check size after reading (in case content-length was missing)
        if data.len() as u64 > self.config.max_fetch_bytes {
            return VerifyBlobResponse::error(
                expected_cid.to_string(),
                format!(
                    "Blob too large: {} bytes (max: {})",
                    data.len(),
                    self.config.max_fetch_bytes
                ),
            );
        }

        let response =
            VerifyBlobResponse::checked(&expected, &data, start.elapsed().as_millis() as u64);
        debug!(
            url = %url,
            expected = %expected_cid,
            outcome = ?response.outcome,
            size = data.len(),
            duration_ms = response.duration_ms,
            "URL blob verification completed"
        );
        response
    }

    /// Handle a verification request
    pub async fn handle_request(&self, request: VerifyBlobRequest) -> VerifyBlobResponse {
        // Check if we have inline data
        if let Some(ref data_base64) = request.data_base64 {
            return self.verify_base64(data_base64, &request.expected_cid);
        }

        // Check if we should fetch from URL
        if let Some(ref fetch_url) = request.fetch_url {
            return self.verify_from_url(fetch_url, &request.expected_cid).await;
        }

        VerifyBlobResponse::error(
            request.expected_cid,
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

/// The byte CID (raw + BLAKE3-256) of a stream, computed incrementally.
pub struct StreamingHasher {
    hasher: blake3::Hasher,
    bytes_processed: u64,
}

impl StreamingHasher {
    /// Create a new streaming hasher
    pub fn new() -> Self {
        Self {
            hasher: blake3::Hasher::new(),
            bytes_processed: 0,
        }
    }

    /// Update with a chunk of data
    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
        self.bytes_processed += data.len() as u64;
    }

    /// Finalize and return the stream's byte CID and length
    pub fn finalize(self) -> (elohim_address::ByteCid, u64) {
        let cid = elohim_address::ByteCid::from_digest(*self.hasher.finalize().as_bytes());
        (cid, self.bytes_processed)
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
    fn verify_bytes_admits_the_kernel_vector() {
        let service = VerificationService::with_defaults();
        let result = service.verify_bytes(
            b"hello world",
            "bafkr4igxjga67jykbseaxdmmdgc5a5o3zp3htom2l6mrjznk7fvyggu6eq",
        );
        assert!(result.is_valid);
        assert_eq!(result.outcome, VerifyOutcome::Verified);
        assert_eq!(
            result.computed_cid.as_deref(),
            Some("bafkr4igxjga67jykbseaxdmmdgc5a5o3zp3htom2l6mrjznk7fvyggu6eq")
        );
        assert_eq!(result.size_bytes, 11);
    }

    #[test]
    fn verify_bytes_mismatch() {
        let service = VerificationService::with_defaults();
        let other = compute_bytes_cid(b"other bytes").to_string();
        let result = service.verify_bytes(b"Test data", &other);
        assert!(!result.is_valid);
        assert_eq!(result.outcome, VerifyOutcome::Mismatch);
    }

    /// A CID names its own profile: a citation CID (raw + sha2-256) over the same
    /// bytes verifies as itself, never as a byte address.
    #[test]
    fn verify_dispatches_on_the_claimed_profile() {
        let service = VerificationService::with_defaults();
        let cite = elohim_address::compute_raw_sha2_cid(b"hello world").to_string();
        assert_eq!(
            service.verify_bytes(b"hello world", &cite).outcome,
            VerifyOutcome::Verified
        );
    }

    /// An unknown profile is typed `unsupported-profile`, not a mismatch and never valid.
    #[test]
    fn unsupported_profile_is_typed_not_valid() {
        let service = VerificationService::with_defaults();
        // raw + sha3-256 (0x16): a well-formed CID of a profile the kernel does not check.
        let mh = cid::multihash::Multihash::<64>::wrap(0x16, &[7u8; 32]).unwrap();
        let sha3 = Cid::new_v1(0x55, mh).to_string();
        let result = service.verify_bytes(b"hello world", &sha3);
        assert!(!result.is_valid);
        assert!(
            matches!(
                result.outcome,
                VerifyOutcome::UnsupportedProfile { codec: 0x55, .. }
            ),
            "{:?}",
            result.outcome
        );
        let wire = serde_json::to_value(&result).unwrap();
        assert_eq!(wire["outcome"]["kind"], "unsupported-profile");
    }

    /// `sha256-<hex>` and bare hex are not CIDs: an error, never compared.
    #[test]
    fn legacy_spellings_are_errors_not_mismatches() {
        let service = VerificationService::with_defaults();
        for legacy in [format!("sha256-{}", "00".repeat(32)), "00".repeat(32)] {
            let result = service.verify_bytes(b"Test data", &legacy);
            assert!(!result.is_valid);
            assert_eq!(result.outcome, VerifyOutcome::Error, "{legacy}");
        }
    }

    #[test]
    fn test_verify_base64() {
        use base64::{engine::general_purpose, Engine as _};

        let service = VerificationService::with_defaults();
        let data = b"Base64 test data";
        let data_base64 = general_purpose::STANDARD.encode(data);
        let expected = compute_bytes_cid(data).to_string();

        let result = service.verify_base64(&data_base64, &expected);
        assert!(result.is_valid);
        assert_eq!(result.computed_cid.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn streaming_hasher_matches_the_one_shot_byte_cid() {
        let mut hasher = StreamingHasher::new();
        hasher.update(b"hello ");
        hasher.update(b"world");
        let (cid, size) = hasher.finalize();
        assert_eq!(
            cid.to_string(),
            "bafkr4igxjga67jykbseaxdmmdgc5a5o3zp3htom2l6mrjznk7fvyggu6eq"
        );
        assert_eq!(size, 11);
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
