//! Doorway Client Library
//!
//! Shared types for the Doorway cache rule protocol. This crate provides:
//!
//! - `CacheRule`: Struct for declaring caching behavior per zome function
//! - `cache_rules!`: Macro for easy cache rule definition (when `hdk` feature enabled)
//! - Builder pattern for constructing rules
//!
//! ## Usage in DNAs
//!
//! ```ignore
//! use doorway_client::{CacheRule, CacheRuleBuilder};
//! use hdk::prelude::*;
//!
//! #[hdk_extern]
//! fn __doorway_cache_rules(_: ()) -> ExternResult<Vec<CacheRule>> {
//!     Ok(vec![
//!         CacheRuleBuilder::new("get_content")
//!             .ttl(3600)
//!             .reach_based("reach", "commons")
//!             .invalidated_by(vec!["create_content", "update_content"])
//!             .build(),
//!         CacheRuleBuilder::new("get_all_paths")
//!             .ttl(300)
//!             .public()
//!             .invalidated_by(vec!["create_path", "delete_path"])
//!             .build(),
//!     ])
//! }
//! ```
//!
//! ## Defaults
//!
//! If a DNA doesn't implement `__doorway_cache_rules`, the gateway applies conventions:
//! - `get_*` and `list_*` functions → cacheable, 5 min TTL, auth required
//! - Other functions → not cacheable via REST API

use serde::{Deserialize, Serialize};

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
