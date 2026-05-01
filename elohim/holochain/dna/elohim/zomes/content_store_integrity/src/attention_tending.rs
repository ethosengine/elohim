//! AttentionTending integrity entry type — Phase 3.5 P3.5.5 (DHT layer).
//!
//! **Peer-private by design.** This entry stays on the agent's source chain,
//! signed and immutable per action, but never gossiped to the DHT.
//! The load-bearing flag is `#[entry_type(visibility = "private")]` on the
//! `EntryTypes` variant in `lib.rs`.
//!
//! ## Why Visibility::Private?
//!
//! AttentionTending records a human's discernment decision about a filter
//! pattern (brainstorm §6.1). The decision belongs to the human and their
//! elohim only:
//!
//! - **Non-repudiation**: signed-atom provenance on the source chain.
//! - **Confidentiality**: never gossipped, never visible to peers.
//! - **Aggregation without exposure**: T16 reads private chains post-k-anonymity
//!   and emits a `CollectiveFilterPattern` (T6) — the individual record stays
//!   private, only the anonymized aggregate is shared.
//!
//! ## Wire shape vs HDI shape
//!
//! The T2 wire type (`elohim-storage/src/p2p/attention_tending.rs`) uses
//! `serde_json::Value` for `filter_subject` and `context` (safe because it's
//! in `elohim-storage`, not in a Holochain zome).  This HDI version uses
//! `*_json: String` per `feedback_serde_json_value_breaks_zome_boundary`.
//!
//! ## Updates ARE supported
//!
//! Re-tending events append to `tended_at` and extend the TTL. Unlike
//! `FeedbackSignal` (which is event-shaped and immutable), `AttentionTending`
//! is a **state entry** — the coordinator updates it on re-tending.
//! `validate_update_entry` in `lib.rs` falls through to
//! `validate_create_entry`, which validates the updated state.
//! **Do not add update-rejection here.**
//!
//! ## Deferred rules
//!
//! - T15 enforces classification-specific TTL defaults and the
//!   "safety classification has no expiry" convention.
//! - T9 provides the cross-agent privacy sweettest (coordinator level).
//! - T16 reads private chains and aggregates post-k-anonymity.

use hdi::prelude::*;

/// Bootstrap classification whitelist.
///
/// Wire values are kebab-case, matching the T2 `Classification` enum wire names.
const CLASSIFICATIONS: &[&str] = &["values-forward", "fatigue", "scope-mismatch", "safety"];

/// AttentionTending integrity entry — Phase 3.5 tending subsystem.
///
/// A TTL-bounded classification of a filter pattern stored **privately**
/// on the author's source chain. Never gossiped to the DHT.
///
/// Invariants enforced by [`AttentionTending::validate`]:
/// - `classification` must be one of the four bootstrap kinds.
/// - `ttl_seconds` must be >= 3600 (1 hour minimum) to prevent permanent moats.
/// - `tended_at` must be non-empty (at minimum the creation timestamp).
///
/// `filter_subject_json` and `context_json` hold pre-stringified JSON stubs;
/// T15 will enrich the inner shape once the tending lifecycle service ships.
#[hdk_entry_helper]
#[derive(Clone)]
pub struct AttentionTending {
    /// JSON-encoded FilterSubject — schema will be defined by T15.
    pub filter_subject_json: String,

    /// values-forward / fatigue / scope-mismatch / safety
    pub classification: String,

    /// Optional human-readable reason.
    pub reason: Option<String>,

    /// TTL in seconds; minimum 3600 to avoid permanent moats.
    pub ttl_seconds: u64,

    /// Unix timestamps; non-empty (creation + re-tendings).
    pub tended_at: Vec<u64>,

    /// JSON-encoded ContextScope — schema will be defined by T15.
    pub context_json: String,

    /// Raw bytes of signer's pubkey (T9 coordinator decodes from wire).
    pub signer_pubkey: Vec<u8>,
}

impl AttentionTending {
    /// Phase 3.5 create-time floor checks. Deterministic, no DHT lookups.
    /// Returns `Ok(())` when valid; `Err(reason)` when a floor is violated.
    pub fn validate(&self) -> Result<(), String> {
        // Floor 1: classification whitelist.
        if !CLASSIFICATIONS.contains(&self.classification.as_str()) {
            return Err(format!(
                "unknown classification: {} (allowed: {:?})",
                self.classification, CLASSIFICATIONS
            ));
        }

        // Floor 2: ttl_seconds minimum 3600 (1 hour) to avoid permanent moats.
        //
        // Classification-specific defaults (e.g. 30 days for values-forward)
        // and the "safety has no expiry" convention are deferred to the T15
        // tending lifecycle service; this floor only ensures nothing slips
        // through as an ephemeral sub-hour tending that can't be cleaned up.
        if self.ttl_seconds < 3600 {
            return Err(format!(
                "ttl_seconds must be >= 3600 (1 hour minimum), got {}",
                self.ttl_seconds
            ));
        }

        // Floor 3: tended_at non-empty (must include creation event).
        //
        // A tending with no timestamps is semantically meaningless; the
        // creation event is always included in the initial `tended_at`.
        if self.tended_at.is_empty() {
            return Err("tended_at must contain at least the creation timestamp".to_string());
        }

        // filter_subject_json and context_json structural validation deferred
        // to T15 (tending lifecycle service) which knows the inner schemas.

        Ok(())
    }
}

// =============================================================================
// Inline tests (host target; no HDK context required)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Test helper
    // -----------------------------------------------------------------------

    fn make_tending(
        classification: &str,
        ttl_seconds: u64,
        tended_at: Vec<u64>,
        reason: Option<&str>,
    ) -> AttentionTending {
        AttentionTending {
            filter_subject_json: r#"{"contentKind":"concept"}"#.to_string(),
            classification: classification.to_string(),
            reason: reason.map(|s| s.to_string()),
            ttl_seconds,
            tended_at,
            context_json: r#"{"collective":"household"}"#.to_string(),
            signer_pubkey: vec![0u8; 32],
        }
    }

    fn valid() -> AttentionTending {
        make_tending("values-forward", 3600, vec![1_746_000_000], None)
    }

    // -----------------------------------------------------------------------
    // Floor 1: classification whitelist — each of the 4 accepted
    // -----------------------------------------------------------------------

    #[test]
    fn accepts_values_forward() {
        let t = make_tending("values-forward", 3600, vec![1_746_000_000], None);
        assert!(t.validate().is_ok());
    }

    #[test]
    fn accepts_fatigue() {
        let t = make_tending("fatigue", 3600, vec![1_746_000_000], None);
        assert!(t.validate().is_ok());
    }

    #[test]
    fn accepts_scope_mismatch() {
        let t = make_tending("scope-mismatch", 3600, vec![1_746_000_000], None);
        assert!(t.validate().is_ok());
    }

    #[test]
    fn accepts_safety() {
        let t = make_tending("safety", 3600, vec![1_746_000_000], None);
        assert!(t.validate().is_ok());
    }

    #[test]
    fn rejects_unknown_classification() {
        let t = make_tending("ban", 3600, vec![1_746_000_000], None);
        let result = t.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("unknown classification"), "expected 'unknown classification', got: {msg}");
        assert!(msg.contains("ban"), "expected offending value in error, got: {msg}");
    }

    // -----------------------------------------------------------------------
    // Floor 2: ttl_seconds boundary (3599 rejected, 3600 accepted)
    // -----------------------------------------------------------------------

    #[test]
    fn rejects_ttl_3599() {
        let t = make_tending("values-forward", 3599, vec![1_746_000_000], None);
        let result = t.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("ttl_seconds"), "expected 'ttl_seconds' in error, got: {msg}");
        assert!(msg.contains("3599"), "expected offending value in error, got: {msg}");
    }

    #[test]
    fn accepts_ttl_3600() {
        let t = make_tending("values-forward", 3600, vec![1_746_000_000], None);
        assert!(t.validate().is_ok());
    }

    // -----------------------------------------------------------------------
    // Floor 3: tended_at non-empty
    // -----------------------------------------------------------------------

    #[test]
    fn rejects_empty_tended_at() {
        let t = make_tending("values-forward", 3600, vec![], None);
        let result = t.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("tended_at"), "expected 'tended_at' in error, got: {msg}");
    }

    #[test]
    fn accepts_single_tended_at() {
        let t = make_tending("fatigue", 3600, vec![1_746_000_000], None);
        assert!(t.validate().is_ok());
    }

    #[test]
    fn accepts_multiple_tended_at_re_tending() {
        // Re-tending events: creation + two re-tends
        let t = make_tending(
            "scope-mismatch",
            3600,
            vec![1_746_000_000, 1_746_100_000, 1_746_200_000],
            None,
        );
        assert!(t.validate().is_ok());
    }

    // -----------------------------------------------------------------------
    // Optional reason field — Some and None both accepted
    // -----------------------------------------------------------------------

    #[test]
    fn accepts_reason_none() {
        let t = make_tending("safety", 3600, vec![1_746_000_000], None);
        assert!(t.validate().is_ok());
    }

    #[test]
    fn accepts_reason_some() {
        let t = make_tending("fatigue", 3600, vec![1_746_000_000], Some("bandwidth exhaustion on ephemera"));
        assert!(t.validate().is_ok());
    }

    // -----------------------------------------------------------------------
    // Floor ordering — classification checked before ttl/tended_at
    // -----------------------------------------------------------------------

    #[test]
    fn rejects_unknown_classification_before_ttl_check() {
        // Invalid classification AND invalid ttl — classification error takes priority
        let t = make_tending("invalid-kind", 100, vec![], None);
        let result = t.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("unknown classification"),
            "classification floor should fire first, got: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Update-rejection coverage note
    // -----------------------------------------------------------------------
    //
    // AttentionTending IS updatable — re-tending events append to `tended_at`
    // and extend the TTL per brainstorm §6.1.  `validate_update_entry` in
    // `lib.rs` falls through to `validate_create_entry`, which applies these
    // same floor checks against the updated state.  No update-rejection logic
    // is present here or in `lib.rs` for this entry type.
    //
    // The cross-agent privacy sweettest (verifying the entry never gossips to
    // the DHT) is deferred to T9 (coordinator + sweettest in conductor).

    // Verify the "all valid" baseline used throughout this test module passes.
    #[test]
    fn baseline_valid_passes() {
        assert!(valid().validate().is_ok());
    }
}
