//! Cache rule protocol for DNA introspection
//!
//! DNAs can implement `__doorway_cache_rules` to declare their caching needs.
//! Doorway discovers these rules and applies them to REST API requests.
//!
//! ## Protocol
//!
//! Any zome can export a function with this signature:
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
//!     ])
//! }
//! ```

use dashmap::DashMap;
use std::collections::HashMap;
use std::time::Duration;

// Re-export the shared CacheRule type from doorway-client
pub use doorway_client::{CacheRule, CacheRuleBuilder, CACHE_RULES_FN};

/// Extension trait for CacheRule to add doorway-specific methods
pub trait CacheRuleExt {
    /// Get TTL as Duration
    fn ttl(&self) -> Duration;

    /// Check if this rule allows public access based on response data
    fn is_public_response(&self, response: &serde_json::Value) -> bool;
}

impl CacheRuleExt for CacheRule {
    fn ttl(&self) -> Duration {
        Duration::from_secs(self.ttl_secs)
    }

    fn is_public_response(&self, response: &serde_json::Value) -> bool {
        if self.public {
            // Explicitly public, no field check needed
            return true;
        }

        // Check reach field if specified
        if let (Some(field), Some(required_value)) = (&self.reach_field, &self.reach_value) {
            if let Some(actual_value) = response.get(field).and_then(|v| v.as_str()) {
                return actual_value == required_value;
            }
        }

        false
    }
}

/// Default rules applied when a DNA doesn't implement __doorway_cache_rules
#[derive(Debug, Clone)]
pub struct DefaultRules;

impl DefaultRules {
    /// Convention-based rules for common patterns
    pub fn for_function(fn_name: &str) -> Option<CacheRule> {
        // get_* functions are cacheable by default
        if fn_name.starts_with("get_") || fn_name.starts_with("list_") {
            Some(CacheRule {
                fn_name: fn_name.to_string(),
                cacheable: true,
                ttl_secs: 300, // 5 minutes for lists/gets
                public: false, // Require auth by default
                reach_field: None,
                reach_value: None,
                invalidated_by: vec![],
            })
        } else {
            // create_*, update_*, delete_* are not cacheable
            None
        }
    }
}

/// Stored rules for a DNA
#[derive(Debug, Clone)]
pub struct DnaRules {
    /// DNA hash (base64 or hex encoded)
    pub dna_hash: String,

    /// Rules indexed by function name
    pub rules: HashMap<String, CacheRule>,

    /// Whether we've attempted discovery for this DNA
    pub discovered: bool,

    /// Reverse index: function -> functions it invalidates
    pub invalidation_map: HashMap<String, Vec<String>>,
}

impl DnaRules {
    /// Create empty rules (not yet discovered)
    pub fn empty(dna_hash: &str) -> Self {
        Self {
            dna_hash: dna_hash.to_string(),
            rules: HashMap::new(),
            discovered: false,
            invalidation_map: HashMap::new(),
        }
    }

    /// Create from discovered rules
    pub fn from_rules(dna_hash: &str, rules: Vec<CacheRule>) -> Self {
        let mut rule_map = HashMap::new();
        let mut invalidation_map: HashMap<String, Vec<String>> = HashMap::new();

        for rule in rules {
            // Build reverse invalidation map
            for invalidator in &rule.invalidated_by {
                invalidation_map
                    .entry(invalidator.clone())
                    .or_default()
                    .push(rule.fn_name.clone());
            }

            rule_map.insert(rule.fn_name.clone(), rule);
        }

        Self {
            dna_hash: dna_hash.to_string(),
            rules: rule_map,
            discovered: true,
            invalidation_map,
        }
    }

    /// Get rule for a function (with fallback to convention)
    pub fn get_rule(&self, fn_name: &str) -> Option<CacheRule> {
        self.rules
            .get(fn_name)
            .cloned()
            .or_else(|| DefaultRules::for_function(fn_name))
    }

    /// Get functions that should be invalidated when this function is called
    pub fn get_invalidations(&self, fn_name: &str) -> Vec<String> {
        self.invalidation_map.get(fn_name).cloned().unwrap_or_default()
    }
}

/// Store for cache rules across all DNAs
pub struct CacheRuleStore {
    /// Rules indexed by DNA hash
    rules: DashMap<String, DnaRules>,
}

impl CacheRuleStore {
    /// Create a new rule store
    pub fn new() -> Self {
        Self {
            rules: DashMap::new(),
        }
    }

    /// Get rules for a DNA (creates empty entry if not exists)
    pub fn get_dna_rules(&self, dna_hash: &str) -> DnaRules {
        self.rules
            .entry(dna_hash.to_string())
            .or_insert_with(|| DnaRules::empty(dna_hash))
            .clone()
    }

    /// Store discovered rules for a DNA
    pub fn set_dna_rules(&self, dna_hash: &str, rules: Vec<CacheRule>) {
        let dna_rules = DnaRules::from_rules(dna_hash, rules);
        self.rules.insert(dna_hash.to_string(), dna_rules);
    }

    /// Mark a DNA as discovered (even if no rules found)
    pub fn mark_discovered(&self, dna_hash: &str) {
        self.rules
            .entry(dna_hash.to_string())
            .or_insert_with(|| DnaRules::empty(dna_hash))
            .discovered = true;
    }

    /// Check if we've attempted discovery for a DNA
    pub fn is_discovered(&self, dna_hash: &str) -> bool {
        self.rules
            .get(dna_hash)
            .map(|r| r.discovered)
            .unwrap_or(false)
    }

    /// Get rule for a specific function
    pub fn get_rule(&self, dna_hash: &str, fn_name: &str) -> Option<CacheRule> {
        self.get_dna_rules(dna_hash).get_rule(fn_name)
    }
}

impl Default for CacheRuleStore {
    fn default() -> Self {
        Self::new()
    }
}

// Note: CACHE_RULES_FN is re-exported from doorway_client above

/// The zome name hint (any zome can implement this, but this is conventional)
pub const CACHE_RULES_ZOME_HINT: &str = "__doorway__";

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
    fn test_is_public_response() {
        let rule = CacheRule {
            fn_name: "get_content".into(),
            cacheable: true,
            ttl_secs: 3600,
            public: false,
            reach_field: Some("reach".into()),
            reach_value: Some("commons".into()),
            invalidated_by: vec![],
        };

        let public_response = serde_json::json!({"reach": "commons", "title": "Test"});
        let private_response = serde_json::json!({"reach": "private", "title": "Test"});

        assert!(rule.is_public_response(&public_response));
        assert!(!rule.is_public_response(&private_response));
    }

    #[test]
    fn test_explicit_public() {
        let rule = CacheRule {
            fn_name: "get_public_thing".into(),
            cacheable: true,
            ttl_secs: 3600,
            public: true, // Explicitly public
            reach_field: None,
            reach_value: None,
            invalidated_by: vec![],
        };

        // Any response is public when public=true
        let response = serde_json::json!({"anything": "here"});
        assert!(rule.is_public_response(&response));
    }

    #[test]
    fn test_default_rules() {
        assert!(DefaultRules::for_function("get_content").is_some());
        assert!(DefaultRules::for_function("list_paths").is_some());
        assert!(DefaultRules::for_function("create_content").is_none());
        assert!(DefaultRules::for_function("update_content").is_none());
        assert!(DefaultRules::for_function("random_function").is_none());
    }

    #[test]
    fn test_dna_rules_invalidation_map() {
        let rules = vec![
            CacheRule {
                fn_name: "get_content".into(),
                cacheable: true,
                ttl_secs: 3600,
                public: true,
                reach_field: None,
                reach_value: None,
                invalidated_by: vec!["create_content".into(), "update_content".into()],
            },
            CacheRule {
                fn_name: "list_content".into(),
                cacheable: true,
                ttl_secs: 300,
                public: true,
                reach_field: None,
                reach_value: None,
                invalidated_by: vec!["create_content".into(), "delete_content".into()],
            },
        ];

        let dna_rules = DnaRules::from_rules("test_dna", rules);

        // create_content invalidates both get_content and list_content
        let invalidated = dna_rules.get_invalidations("create_content");
        assert!(invalidated.contains(&"get_content".to_string()));
        assert!(invalidated.contains(&"list_content".to_string()));

        // update_content only invalidates get_content
        let invalidated = dna_rules.get_invalidations("update_content");
        assert!(invalidated.contains(&"get_content".to_string()));
        assert!(!invalidated.contains(&"list_content".to_string()));
    }

    #[test]
    fn test_rule_store() {
        let store = CacheRuleStore::new();

        // Not discovered initially
        assert!(!store.is_discovered("some_dna"));

        // Set rules
        store.set_dna_rules(
            "some_dna",
            vec![CacheRule {
                fn_name: "get_thing".into(),
                cacheable: true,
                ttl_secs: 600,
                public: true,
                reach_field: None,
                reach_value: None,
                invalidated_by: vec![],
            }],
        );

        // Now discovered
        assert!(store.is_discovered("some_dna"));

        // Can get rule
        let rule = store.get_rule("some_dna", "get_thing").unwrap();
        assert_eq!(rule.ttl_secs, 600);
    }
}
