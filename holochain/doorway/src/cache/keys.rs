//! Cache key definitions
//!
//! Generic cache keys for Holochain zome calls.

use std::fmt;

/// Cache key for a zome function call
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// DNA hash (base64 or hex encoded)
    pub dna_hash: String,
    /// Zome name
    pub zome: String,
    /// Function name
    pub fn_name: String,
    /// Serialized input arguments (for cache key uniqueness)
    pub args_hash: String,
}

impl CacheKey {
    /// Create a new cache key
    pub fn new(dna_hash: &str, zome: &str, fn_name: &str, args: &str) -> Self {
        // Hash the args for a shorter key
        let args_hash = if args.is_empty() {
            "empty".to_string()
        } else {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(args.as_bytes());
            let hash = hasher.finalize();
            hex::encode(&hash[..8]) // First 8 bytes = 16 hex chars
        };

        Self {
            dna_hash: dna_hash.to_string(),
            zome: zome.to_string(),
            fn_name: fn_name.to_string(),
            args_hash,
        }
    }

    /// Create from components with pre-computed args hash
    pub fn with_args_hash(dna_hash: &str, zome: &str, fn_name: &str, args_hash: &str) -> Self {
        Self {
            dna_hash: dna_hash.to_string(),
            zome: zome.to_string(),
            fn_name: fn_name.to_string(),
            args_hash: args_hash.to_string(),
        }
    }

    /// Convert to storage key string
    pub fn to_storage_key(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.dna_hash, self.zome, self.fn_name, self.args_hash
        )
    }

    /// Create a pattern for invalidating all calls to a function
    pub fn invalidation_pattern(dna_hash: &str, zome: &str, fn_name: &str) -> String {
        format!("{}:{}:{}:", dna_hash, zome, fn_name)
    }
}

impl fmt::Display for CacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}:{}({})",
            &self.dna_hash[..8.min(self.dna_hash.len())],
            self.zome,
            self.fn_name,
            self.args_hash
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_creation() {
        let key = CacheKey::new("dna123", "content_store", "get_content", r#"{"id":"abc"}"#);
        assert_eq!(key.dna_hash, "dna123");
        assert_eq!(key.zome, "content_store");
        assert_eq!(key.fn_name, "get_content");
        assert!(!key.args_hash.is_empty());
    }

    #[test]
    fn test_cache_key_empty_args() {
        let key = CacheKey::new("dna123", "zome", "fn", "");
        assert_eq!(key.args_hash, "empty");
    }

    #[test]
    fn test_cache_key_deterministic() {
        let key1 = CacheKey::new("dna", "zome", "fn", r#"{"id":"123"}"#);
        let key2 = CacheKey::new("dna", "zome", "fn", r#"{"id":"123"}"#);
        assert_eq!(key1.args_hash, key2.args_hash);
        assert_eq!(key1.to_storage_key(), key2.to_storage_key());
    }

    #[test]
    fn test_different_args_different_keys() {
        let key1 = CacheKey::new("dna", "zome", "fn", r#"{"id":"123"}"#);
        let key2 = CacheKey::new("dna", "zome", "fn", r#"{"id":"456"}"#);
        assert_ne!(key1.args_hash, key2.args_hash);
    }

    #[test]
    fn test_invalidation_pattern() {
        let pattern = CacheKey::invalidation_pattern("dna123", "content_store", "get_content");
        assert_eq!(pattern, "dna123:content_store:get_content:");

        // A matching key should contain this pattern
        let key = CacheKey::new("dna123", "content_store", "get_content", r#"{"id":"x"}"#);
        assert!(key.to_storage_key().starts_with(&pattern));
    }

    #[test]
    fn test_display() {
        let key = CacheKey::new("dna_hash_very_long", "zome", "get_thing", "{}");
        let display = format!("{}", key);
        assert!(display.contains("zome"));
        assert!(display.contains("get_thing"));
    }
}
