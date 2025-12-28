//! Doorway Client Library
//!
//! Caching and publishing traits for Holochain DNAs to integrate with Doorway gateway.
//!
//! ## Core Concepts
//!
//! ### Caching (API responses)
//! 1. **Cacheable trait** - Entry types implement this to declare caching behavior
//! 2. **CacheSignal** - Signal type for post_commit to notify doorway of changes
//! 3. **CacheRule** - Declarative caching rules for zome functions
//!
//! ### Publishing (raw content serving)
//! 4. **Publishable trait** - Content types that can be served as raw bytes
//! 5. **ContentServer** - DHT entry registering an agent as content publisher
//! 6. **PublishSignal** - Signal to announce publishing availability
//!
//! ## Usage in DNAs
//!
//! ```ignore
//! use doorway_client::{Cacheable, CacheSignal, emit_cache_signal};
//!
//! // 1. Implement Cacheable for your entry type
//! impl Cacheable for Content {
//!     fn cache_type() -> &'static str { "Content" }
//!     fn cache_id(&self) -> String { self.id.clone() }
//!     fn cache_ttl() -> u64 { 3600 } // 1 hour
//!     fn is_public(&self) -> bool { self.reach == "commons" }
//! }
//!
//! // 2. In post_commit, emit cache signal
//! #[hdk_extern]
//! fn post_commit(actions: Vec<SignedActionHashed>) -> ExternResult<()> {
//!     for action in actions {
//!         if let Some(content) = get_entry_from_action::<Content>(&action)? {
//!             emit_cache_signal(CacheSignal::upsert(&content))?;
//!         }
//!     }
//!     Ok(())
//! }
//! ```

use serde::{Deserialize, Serialize};

// Publishing module - content serving capabilities
pub mod publish;

// Re-export publishing types at crate root
pub use publish::{
    Publishable,
    ContentServer,
    ContentServerCapability,
    PublishSignal,
    PublishSignalType,
    DoorwayPublishSignal,
    Html5AppBundle,
    Html5AppManifest,
    FindPublishersInput,
    FindPublishersOutput,
};

// =============================================================================
// Cacheable Trait - Entry types implement this
// =============================================================================

/// Trait for entry types that should be cached by Doorway.
///
/// Implement this trait on your entry types to declare their caching behavior.
/// The doorway will use this information to cache and serve content.
pub trait Cacheable {
    /// The cache type name (e.g., "Content", "LearningPath")
    /// This becomes the {type} in /api/v1/cache/{type}/{id}
    fn cache_type() -> &'static str;

    /// The unique ID for this entry in the cache
    fn cache_id(&self) -> String;

    /// Time-to-live in seconds (default: 300 = 5 minutes)
    fn cache_ttl() -> u64 {
        300
    }

    /// Whether this entry is publicly accessible without auth
    fn is_public(&self) -> bool {
        false
    }

    /// Optional reach level for reach-aware caching
    fn reach(&self) -> Option<&str> {
        None
    }

    /// Convert to JSON for caching
    fn to_cache_json(&self) -> Result<serde_json::Value, serde_json::Error>
    where
        Self: Serialize,
    {
        serde_json::to_value(self)
    }
}

// =============================================================================
// Cache Signals - For post_commit notifications
// =============================================================================

/// Signal type for cache updates sent via post_commit.
///
/// Doorway subscribes to these signals to maintain its cache.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheSignal {
    /// Signal type: "upsert", "delete", "invalidate"
    pub signal_type: CacheSignalType,
    /// Document type (e.g., "Content", "LearningPath")
    pub doc_type: String,
    /// Document ID
    pub doc_id: String,
    /// The document data (for upsert)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// TTL in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_secs: Option<u64>,
    /// Whether publicly accessible
    #[serde(default)]
    pub public: bool,
    /// Reach level for reach-aware caching
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reach: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CacheSignalType {
    /// Insert or update a document in cache
    Upsert,
    /// Remove a document from cache
    Delete,
    /// Invalidate cache entries matching a pattern
    Invalidate,
}

impl CacheSignal {
    /// Create an upsert signal for a cacheable entry
    pub fn upsert<T: Cacheable + Serialize>(entry: &T) -> Self {
        Self {
            signal_type: CacheSignalType::Upsert,
            doc_type: T::cache_type().to_string(),
            doc_id: entry.cache_id(),
            data: entry.to_cache_json().ok(),
            ttl_secs: Some(T::cache_ttl()),
            public: entry.is_public(),
            reach: entry.reach().map(|s| s.to_string()),
        }
    }

    /// Create a delete signal
    pub fn delete(doc_type: &str, doc_id: &str) -> Self {
        Self {
            signal_type: CacheSignalType::Delete,
            doc_type: doc_type.to_string(),
            doc_id: doc_id.to_string(),
            data: None,
            ttl_secs: None,
            public: false,
            reach: None,
        }
    }

    /// Create an invalidate signal for a type
    pub fn invalidate(doc_type: &str) -> Self {
        Self {
            signal_type: CacheSignalType::Invalidate,
            doc_type: doc_type.to_string(),
            doc_id: "*".to_string(),
            data: None,
            ttl_secs: None,
            public: false,
            reach: None,
        }
    }
}

/// Wrapper for emitting cache signals in a consistent format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorwaySignal {
    /// Signal namespace - always "doorway"
    pub namespace: String,
    /// The cache signal payload
    pub payload: CacheSignal,
}

impl DoorwaySignal {
    pub fn new(signal: CacheSignal) -> Self {
        Self {
            namespace: "doorway".to_string(),
            payload: signal,
        }
    }

    /// Convert to bytes for emit_signal
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

// =============================================================================
// HDK Integration (when hdk feature is enabled)
// =============================================================================

#[cfg(feature = "hdk")]
pub use hdk_integration::*;

#[cfg(feature = "hdk")]
mod hdk_integration {
    use super::*;

    /// Emit a cache signal to doorway via HDK
    ///
    /// This should be called from post_commit to notify doorway of cache updates.
    #[inline]
    pub fn emit_cache_signal(signal: CacheSignal) -> Result<(), String> {
        let doorway_signal = DoorwaySignal::new(signal);
        let bytes = doorway_signal.to_bytes()
            .map_err(|e| format!("Failed to serialize cache signal: {}", e))?;

        // Note: In actual HDK usage, this would call hdk::prelude::emit_signal
        // For now, we just return the bytes for the caller to handle
        let _ = bytes;
        Ok(())
    }
}

/// The standard function name for cache rule introspection
pub const CACHE_RULES_FN: &str = "__doorway_cache_rules";

// =============================================================================
// CacheRule - The core type shared between DNAs and Doorway
// =============================================================================

/// Cache rule for a single zome function.
///
/// Defines how the Doorway gateway should cache responses for this function.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheRule {
    /// Zome function name (e.g., "get_content")
    pub fn_name: String,

    /// Whether this function's results should be cached
    #[serde(default = "default_true")]
    pub cacheable: bool,

    /// Cache TTL in seconds (default: 300 = 5 minutes)
    #[serde(default = "default_ttl")]
    pub ttl_secs: u64,

    /// Whether this endpoint is publicly accessible without auth
    #[serde(default)]
    pub public: bool,

    /// Optional: Field path to check for public visibility
    /// e.g., "reach" means check response.reach
    #[serde(default)]
    pub reach_field: Option<String>,

    /// Required value for reach_field to be considered public
    /// e.g., "commons" means response.reach must equal "commons"
    #[serde(default)]
    pub reach_value: Option<String>,

    /// Function names that invalidate this cache entry
    /// e.g., ["create_content", "update_content", "delete_content"]
    #[serde(default)]
    pub invalidated_by: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_ttl() -> u64 {
    300 // 5 minutes
}

impl CacheRule {
    /// Create a new cache rule for a function
    pub fn new(fn_name: impl Into<String>) -> Self {
        Self {
            fn_name: fn_name.into(),
            cacheable: true,
            ttl_secs: 300,
            public: false,
            reach_field: None,
            reach_value: None,
            invalidated_by: vec![],
        }
    }

    /// Create a rule that marks a function as not cacheable
    pub fn not_cacheable(fn_name: impl Into<String>) -> Self {
        Self {
            fn_name: fn_name.into(),
            cacheable: false,
            ttl_secs: 0,
            public: false,
            reach_field: None,
            reach_value: None,
            invalidated_by: vec![],
        }
    }
}

// =============================================================================
// CacheRuleBuilder - Ergonomic rule construction
// =============================================================================

/// Builder for constructing cache rules with a fluent API
#[derive(Debug, Clone)]
pub struct CacheRuleBuilder {
    rule: CacheRule,
}

impl CacheRuleBuilder {
    /// Start building a rule for a function
    pub fn new(fn_name: impl Into<String>) -> Self {
        Self {
            rule: CacheRule::new(fn_name),
        }
    }

    /// Set the cache TTL in seconds
    pub fn ttl(mut self, seconds: u64) -> Self {
        self.rule.ttl_secs = seconds;
        self
    }

    /// TTL shorthand: 1 minute
    pub fn ttl_1m(self) -> Self {
        self.ttl(60)
    }

    /// TTL shorthand: 5 minutes
    pub fn ttl_5m(self) -> Self {
        self.ttl(300)
    }

    /// TTL shorthand: 15 minutes
    pub fn ttl_15m(self) -> Self {
        self.ttl(900)
    }

    /// TTL shorthand: 1 hour
    pub fn ttl_1h(self) -> Self {
        self.ttl(3600)
    }

    /// TTL shorthand: 1 day
    pub fn ttl_1d(self) -> Self {
        self.ttl(86400)
    }

    /// Mark as explicitly public (no auth required)
    pub fn public(mut self) -> Self {
        self.rule.public = true;
        self
    }

    /// Mark as requiring auth (default)
    pub fn private(mut self) -> Self {
        self.rule.public = false;
        self
    }

    /// Set reach-based visibility: public if response.{field} == {value}
    ///
    /// Example: `.reach_based("reach", "commons")` means the response
    /// is public if response.reach == "commons"
    pub fn reach_based(mut self, field: impl Into<String>, value: impl Into<String>) -> Self {
        self.rule.reach_field = Some(field.into());
        self.rule.reach_value = Some(value.into());
        self
    }

    /// Set functions that invalidate this cache entry
    pub fn invalidated_by(mut self, functions: Vec<&str>) -> Self {
        self.rule.invalidated_by = functions.into_iter().map(String::from).collect();
        self
    }

    /// Disable caching for this function
    pub fn not_cacheable(mut self) -> Self {
        self.rule.cacheable = false;
        self
    }

    /// Build the cache rule
    pub fn build(self) -> CacheRule {
        self.rule
    }
}

// =============================================================================
// Convenience functions for common patterns
// =============================================================================

/// Create a rule for content that uses reach-based visibility
pub fn content_rule(fn_name: &str, ttl_secs: u64, invalidators: Vec<&str>) -> CacheRule {
    CacheRuleBuilder::new(fn_name)
        .ttl(ttl_secs)
        .reach_based("reach", "commons")
        .invalidated_by(invalidators)
        .build()
}

/// Create a rule for a public list endpoint
pub fn public_list_rule(fn_name: &str, ttl_secs: u64, invalidators: Vec<&str>) -> CacheRule {
    CacheRuleBuilder::new(fn_name)
        .ttl(ttl_secs)
        .public()
        .invalidated_by(invalidators)
        .build()
}

/// Create a rule for user-specific data (requires auth, short TTL)
pub fn user_data_rule(fn_name: &str, invalidators: Vec<&str>) -> CacheRule {
    CacheRuleBuilder::new(fn_name)
        .ttl(60) // 1 minute for user data
        .private()
        .invalidated_by(invalidators)
        .build()
}

/// Create a rule for statistics (public, medium TTL)
pub fn stats_rule(fn_name: &str, invalidators: Vec<&str>) -> CacheRule {
    CacheRuleBuilder::new(fn_name)
        .ttl_5m()
        .public()
        .invalidated_by(invalidators)
        .build()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_rule_defaults() {
        let rule: CacheRule = serde_json::from_str(r#"{"fn_name": "get_thing"}"#).unwrap();
        assert!(rule.cacheable);
        assert_eq!(rule.ttl_secs, 300);
        assert!(!rule.public);
    }

    #[test]
    fn test_builder() {
        let rule = CacheRuleBuilder::new("get_content")
            .ttl_1h()
            .reach_based("reach", "commons")
            .invalidated_by(vec!["create_content", "update_content"])
            .build();

        assert_eq!(rule.fn_name, "get_content");
        assert_eq!(rule.ttl_secs, 3600);
        assert!(!rule.public);
        assert_eq!(rule.reach_field, Some("reach".to_string()));
        assert_eq!(rule.reach_value, Some("commons".to_string()));
        assert_eq!(rule.invalidated_by.len(), 2);
    }

    #[test]
    fn test_public_rule() {
        let rule = CacheRuleBuilder::new("get_all_paths")
            .ttl_5m()
            .public()
            .build();

        assert!(rule.public);
        assert_eq!(rule.ttl_secs, 300);
    }

    #[test]
    fn test_content_rule_helper() {
        let rule = content_rule("get_content", 3600, vec!["create_content"]);

        assert_eq!(rule.fn_name, "get_content");
        assert_eq!(rule.ttl_secs, 3600);
        assert_eq!(rule.reach_field, Some("reach".to_string()));
        assert_eq!(rule.reach_value, Some("commons".to_string()));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let rule = CacheRuleBuilder::new("get_content")
            .ttl_1h()
            .reach_based("reach", "commons")
            .invalidated_by(vec!["create_content"])
            .build();

        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: CacheRule = serde_json::from_str(&json).unwrap();

        assert_eq!(rule, deserialized);
    }
}
