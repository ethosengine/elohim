//! GateContext — the typed key-value accumulator that flows through DAG steps.
//!
//! As each step executes, it reads inputs from the context and writes its
//! output key. The context is serializable so that a stable summary CID can
//! be derived for attestation (Phase 4).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A typed key-value store that accumulates as DAG steps execute.
///
/// Keys are step output keys declared in `StepType` params (e.g.,
/// `output_key`, `context_keys`). Values are arbitrary JSON. The context
/// is passed `&mut` through the interpreter so each step can augment it.
///
/// # Attestation / CID
///
/// `to_summary_cid()` returns a deterministic hex string derived from the
/// sorted serialised content of the context. Phase 4 will record this on
/// the `GateDecisionAttestation`. For now it is a stable SHA-256 placeholder.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GateContext(HashMap<String, Value>);

impl GateContext {
    /// Create an empty context.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Insert a key-value pair, replacing any existing value.
    pub fn insert(&mut self, key: impl Into<String>, value: Value) {
        self.0.insert(key.into(), value);
    }

    /// Get a reference to the value for a key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    /// Remove and return the value for a key.
    pub fn take(&mut self, key: &str) -> Option<Value> {
        self.0.remove(key)
    }

    /// Return true if the context contains the given key.
    pub fn contains(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Merge all entries from `other` into `self`, overwriting on collision.
    pub fn merge(&mut self, other: GateContext) {
        for (k, v) in other.0 {
            self.0.insert(k, v);
        }
    }

    /// Return a stable hex string representing the content of this context.
    ///
    /// The string is deterministic: keys are sorted before serialization so
    /// insertion order does not affect the output. Phase 4 will store this as
    /// the `context_summary_cid` on `GateDecisionAttestation`.
    ///
    /// Implementation: SHA-256 over the canonical JSON (sorted keys).
    ///
    // PHASE 4 TODO: canonicalize number representations (e.g., integer-valued floats
    // → integers) before summary CID stability can be claimed across heterogeneous
    // executor implementations. Today, Value::Number(1.0) and Value::Number(1)
    // serialize differently, producing different CIDs for what manifest authors
    // may consider "the same" value. Not a bug for Task 2.1 — a contract to resolve
    // before the CID lands in DHT attestations (Phase 4).
    pub fn to_summary_cid(&self) -> String {
        // Build a sorted-key map for deterministic serialization.
        let mut sorted: Vec<(&String, &Value)> = self.0.iter().collect();
        sorted.sort_by_key(|(k, _)| k.as_str());

        let canonical = serde_json::to_string(&sorted)
            .unwrap_or_else(|_| "{}".to_string());

        // Inline SHA-256 using std only — avoids adding a new dep.
        // sha2 is already a workspace dep; use it.
        sha256_hex(canonical.as_bytes())
    }

    /// Iterate over all entries in the context.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.0.iter()
    }
}

/// Compute a lowercase hex SHA-256 digest of the input bytes.
fn sha256_hex(input: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as FmtWrite;

    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();
    result.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn new_context_is_empty() {
        let ctx = GateContext::new();
        assert!(!ctx.contains("anything"));
        assert!(ctx.get("anything").is_none());
    }

    #[test]
    fn insert_and_get_round_trip() {
        let mut ctx = GateContext::new();
        ctx.insert("foo", json!("bar"));
        assert_eq!(ctx.get("foo"), Some(&json!("bar")));
    }

    #[test]
    fn insert_overwrites_existing() {
        let mut ctx = GateContext::new();
        ctx.insert("x", json!(1));
        ctx.insert("x", json!(2));
        assert_eq!(ctx.get("x"), Some(&json!(2)));
    }

    #[test]
    fn contains_returns_false_for_missing_key() {
        let ctx = GateContext::new();
        assert!(!ctx.contains("missing"));
    }

    #[test]
    fn contains_returns_true_after_insert() {
        let mut ctx = GateContext::new();
        ctx.insert("present", json!(true));
        assert!(ctx.contains("present"));
    }

    #[test]
    fn take_removes_and_returns_value() {
        let mut ctx = GateContext::new();
        ctx.insert("ephemeral", json!(42));
        let v = ctx.take("ephemeral");
        assert_eq!(v, Some(json!(42)));
        assert!(!ctx.contains("ephemeral"));
    }

    #[test]
    fn take_returns_none_for_missing_key() {
        let mut ctx = GateContext::new();
        assert!(ctx.take("nope").is_none());
    }

    #[test]
    fn merge_combines_two_contexts() {
        let mut a = GateContext::new();
        a.insert("a_key", json!("a_val"));

        let mut b = GateContext::new();
        b.insert("b_key", json!("b_val"));

        a.merge(b);

        assert_eq!(a.get("a_key"), Some(&json!("a_val")));
        assert_eq!(a.get("b_key"), Some(&json!("b_val")));
    }

    #[test]
    fn merge_overwrites_on_collision() {
        let mut a = GateContext::new();
        a.insert("k", json!("original"));

        let mut b = GateContext::new();
        b.insert("k", json!("override"));

        a.merge(b);
        assert_eq!(a.get("k"), Some(&json!("override")));
    }

    #[test]
    fn to_summary_cid_is_stable_for_same_content() {
        let mut ctx1 = GateContext::new();
        ctx1.insert("k1", json!("v1"));
        ctx1.insert("k2", json!(42));

        let mut ctx2 = GateContext::new();
        // Insert in different order — CID must still match.
        ctx2.insert("k2", json!(42));
        ctx2.insert("k1", json!("v1"));

        assert_eq!(ctx1.to_summary_cid(), ctx2.to_summary_cid());
    }

    #[test]
    fn to_summary_cid_differs_for_different_content() {
        let mut ctx1 = GateContext::new();
        ctx1.insert("k", json!("v1"));

        let mut ctx2 = GateContext::new();
        ctx2.insert("k", json!("v2"));

        assert_ne!(ctx1.to_summary_cid(), ctx2.to_summary_cid());
    }

    #[test]
    fn to_summary_cid_is_64_hex_chars() {
        let ctx = GateContext::new();
        let cid = ctx.to_summary_cid();
        assert_eq!(cid.len(), 64, "SHA-256 hex = 64 chars, got: {cid}");
        assert!(cid.chars().all(|c| c.is_ascii_hexdigit()), "not hex: {cid}");
    }

    #[test]
    fn context_serialises_and_deserialises() {
        let mut ctx = GateContext::new();
        ctx.insert("key", json!({"nested": true}));
        let json = serde_json::to_string(&ctx).unwrap();
        let restored: GateContext = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx.get("key"), restored.get("key"));
    }
}
