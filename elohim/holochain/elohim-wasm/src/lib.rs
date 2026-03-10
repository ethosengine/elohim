//! Elohim WASM - Client-Side Verification
//!
//! Provides SHA256 blob verification in WebAssembly for:
//! - Offline verification capability
//! - Faster local verification vs server round-trip
//! - Defense-in-depth (client + server verification)
//!
//! ## Usage in JavaScript
//!
//! ```javascript
//! import init, { verify_blob, compute_hash, StreamingHasher } from 'elohim-wasm';
//!
//! await init();
//!
//! // One-shot verification
//! const result = verify_blob(blobData, "expected-sha256-hash");
//! console.log(result.is_valid, result.computed_hash);
//!
//! // Streaming verification for large files
//! const hasher = new StreamingHasher();
//! hasher.update(chunk1);
//! hasher.update(chunk2);
//! const result = hasher.finalize("expected-hash");
//! ```
//!
//! ## Build
//!
//! ```bash
//! wasm-pack build --target web --out-dir pkg
//! ```

use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;

// Initialize panic hook for better error messages in browser console
#[cfg(feature = "console_error_panic_hook")]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

// ============================================================================
// Types
// ============================================================================

/// Result of blob verification
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct VerificationResult {
    is_valid: bool,
    computed_hash: String,
    expected_hash: String,
    size_bytes: u64,
    error: Option<String>,
}

#[wasm_bindgen]
impl VerificationResult {
    /// Whether the blob matches the expected hash
    #[wasm_bindgen(getter)]
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }

    /// The computed SHA256 hash (hex string)
    #[wasm_bindgen(getter)]
    pub fn computed_hash(&self) -> String {
        self.computed_hash.clone()
    }

    /// The expected hash that was checked against
    #[wasm_bindgen(getter)]
    pub fn expected_hash(&self) -> String {
        self.expected_hash.clone()
    }

    /// Size of the data in bytes
    #[wasm_bindgen(getter)]
    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    /// Error message if verification failed due to an error
    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }

    /// Create a success result (internal use)
    fn new_success(computed_hash: String, expected_hash: String, size_bytes: u64) -> Self {
        let is_valid = computed_hash.to_lowercase() == expected_hash.to_lowercase();
        Self {
            is_valid,
            computed_hash,
            expected_hash,
            size_bytes,
            error: None,
        }
    }

    /// Create an error result (internal use)
    fn new_error(expected_hash: String, error_msg: String) -> Self {
        Self {
            is_valid: false,
            computed_hash: String::new(),
            expected_hash,
            size_bytes: 0,
            error: Some(error_msg),
        }
    }
}

// ============================================================================
// One-Shot Verification
// ============================================================================

/// Verify a blob against an expected SHA256 hash
///
/// # Arguments
/// * `data` - The blob data as a Uint8Array
/// * `expected_hash` - Expected SHA256 hash (hex string, 64 characters)
///
/// # Returns
/// A VerificationResult with is_valid, computed_hash, and size_bytes
#[wasm_bindgen]
pub fn verify_blob(data: &[u8], expected_hash: &str) -> VerificationResult {
    #[cfg(feature = "console_error_panic_hook")]
    set_panic_hook();

    // Validate expected hash format
    if expected_hash.len() != 64 {
        return VerificationResult::new_error(
            expected_hash.to_string(),
            format!(
                "Invalid hash length: expected 64 characters, got {}",
                expected_hash.len()
            ),
        );
    }

    if !expected_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return VerificationResult::new_error(
            expected_hash.to_string(),
            "Invalid hash: must be hexadecimal".to_string(),
        );
    }

    let computed = compute_hash(data);
    VerificationResult::new_success(computed, expected_hash.to_string(), data.len() as u64)
}

/// Compute SHA256 hash of data
///
/// # Arguments
/// * `data` - The data to hash as a Uint8Array
///
/// # Returns
/// SHA256 hash as lowercase hex string (64 characters)
#[wasm_bindgen]
pub fn compute_hash(data: &[u8]) -> String {
    #[cfg(feature = "console_error_panic_hook")]
    set_panic_hook();

    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

// ============================================================================
// Streaming Verification
// ============================================================================

/// Incremental SHA256 hasher for large files
///
/// Use this when processing large blobs in chunks to avoid
/// loading the entire file into memory at once.
///
/// # Example (JavaScript)
/// ```javascript
/// const hasher = new StreamingHasher();
/// for await (const chunk of stream) {
///     hasher.update(chunk);
/// }
/// const result = hasher.finalize("expected-hash");
/// ```
#[wasm_bindgen]
pub struct StreamingHasher {
    hasher: Sha256,
    bytes_processed: u64,
}

#[wasm_bindgen]
impl StreamingHasher {
    /// Create a new streaming hasher
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        #[cfg(feature = "console_error_panic_hook")]
        set_panic_hook();

        Self {
            hasher: Sha256::new(),
            bytes_processed: 0,
        }
    }

    /// Update the hasher with a chunk of data
    ///
    /// # Arguments
    /// * `chunk` - A chunk of data as a Uint8Array
    pub fn update(&mut self, chunk: &[u8]) {
        self.hasher.update(chunk);
        self.bytes_processed += chunk.len() as u64;
    }

    /// Get the number of bytes processed so far
    #[wasm_bindgen(getter)]
    pub fn bytes_processed(&self) -> u64 {
        self.bytes_processed
    }

    /// Finalize the hash and verify against expected
    ///
    /// Note: This consumes the hasher. Create a new one for subsequent operations.
    ///
    /// # Arguments
    /// * `expected_hash` - Expected SHA256 hash (hex string)
    ///
    /// # Returns
    /// VerificationResult with the computed hash and validity
    pub fn finalize(self, expected_hash: &str) -> VerificationResult {
        let computed = hex::encode(self.hasher.finalize());

        // Validate expected hash format
        if expected_hash.len() != 64 {
            return VerificationResult::new_error(
                expected_hash.to_string(),
                format!(
                    "Invalid hash length: expected 64 characters, got {}",
                    expected_hash.len()
                ),
            );
        }

        VerificationResult::new_success(computed, expected_hash.to_string(), self.bytes_processed)
    }

    /// Finalize and return just the hash (without verification)
    ///
    /// Use this when you just need the hash, not verification.
    pub fn finalize_hash(self) -> String {
        hex::encode(self.hasher.finalize())
    }
}

impl Default for StreamingHasher {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Check if a string is a valid SHA256 hash format
#[wasm_bindgen]
pub fn is_valid_hash_format(hash: &str) -> bool {
    hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit())
}

/// Get the WASM module version
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let data = b"Hello, World!";
        let hash = compute_hash(data);
        assert_eq!(
            hash,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );
    }

    #[test]
    fn test_verify_blob_valid() {
        let data = b"Test data for verification";
        let expected = compute_hash(data);
        let result = verify_blob(data, &expected);

        assert!(result.is_valid);
        assert_eq!(result.computed_hash, expected);
        assert_eq!(result.size_bytes, data.len() as u64);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_verify_blob_invalid() {
        let data = b"Test data";
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let result = verify_blob(data, wrong_hash);

        assert!(!result.is_valid);
        assert_ne!(result.computed_hash, wrong_hash);
    }

    #[test]
    fn test_verify_blob_bad_hash_length() {
        let data = b"Test data";
        let result = verify_blob(data, "abc123");

        assert!(!result.is_valid);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Invalid hash length"));
    }

    #[test]
    fn test_streaming_hasher() {
        let mut hasher = StreamingHasher::new();
        hasher.update(b"Hello, ");
        hasher.update(b"World!");

        let expected = "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f";
        let result = hasher.finalize(expected);

        assert!(result.is_valid);
        assert_eq!(result.computed_hash, expected);
        assert_eq!(result.size_bytes, 13);
    }

    #[test]
    fn test_streaming_hasher_bytes_processed() {
        let mut hasher = StreamingHasher::new();
        assert_eq!(hasher.bytes_processed(), 0);

        hasher.update(b"12345");
        assert_eq!(hasher.bytes_processed(), 5);

        hasher.update(b"67890");
        assert_eq!(hasher.bytes_processed(), 10);
    }

    #[test]
    fn test_is_valid_hash_format() {
        assert!(is_valid_hash_format(
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        ));
        assert!(!is_valid_hash_format("abc123")); // Too short
        assert!(!is_valid_hash_format(
            "zzzz6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        )); // Invalid hex
    }
}

// ============================================================================
// WASM-specific Tests
// ============================================================================

#[cfg(test)]
mod wasm_tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    use super::*;

    #[wasm_bindgen_test]
    fn test_wasm_verify_blob() {
        let data = b"WASM test data";
        let expected = compute_hash(data);
        let result = verify_blob(data, &expected);

        assert!(result.is_valid());
        assert_eq!(result.computed_hash(), expected);
    }

    #[wasm_bindgen_test]
    fn test_wasm_streaming() {
        let mut hasher = StreamingHasher::new();
        hasher.update(b"Streaming ");
        hasher.update(b"test");

        let hash = hasher.finalize_hash();
        assert_eq!(hash.len(), 64);
    }
}
