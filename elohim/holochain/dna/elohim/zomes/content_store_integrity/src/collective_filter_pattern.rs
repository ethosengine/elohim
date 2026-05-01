//! CollectiveFilterPattern integrity entry type — Phase 3.5 P3.5.6 (DHT layer).
//!
//! This is the **k-anonymous aggregate** that the tending lifecycle (T16) emits
//! when a tending pattern crosses the participation threshold. The entry is
//! **PUBLIC** — it gossips to the DHT so peers can discover collective filter
//! patterns from a collective they belong to.
//!
//! ## K-Anonymity is the load-bearing property
//!
//! **NO peer identities are embedded in this entry.** This is a deliberate
//! departure from T4 (`FeedbackSignal`) and T5 (`AttentionTending`), both of
//! which carry a `signer_pubkey` field.
//!
//! The rationale (brainstorm §6.4 "NO peer identities — pure aggregate"):
//!
//! - `signer_pubkey` in a *collective* entry would suggest "the agent whose
//!   tending this represents" — directly violating k-anonymity.
//! - The Holochain **action header** already carries the publishing peer's pubkey
//!   as audit metadata (unforgeable, non-removable). That is the right place for
//!   publisher identity — it is NOT an embedded identity claim in the entry
//!   payload.
//! - T16's aggregator enforces the k-anonymity threshold (default k=5) before
//!   calling `create_entry`. Only when ≥ k peers have tended the same pattern
//!   is a `CollectiveFilterPattern` published.
//!
//! ## PUBLIC vs PRIVATE
//!
//! Unlike `AttentionTending` (Visibility::Private, source-chain only), this
//! entry has **no `#[entry_type(visibility = "private")]` attribute** on the
//! `EntryTypes` variant. The default visibility is Public: the entry gossips
//! to the DHT so other peers can benefit from the collective signal.
//!
//! ## Updates
//!
//! Updates re-run create-time validation. T16 aggregator service may convention
//! to always-create-new-entries (snapshot-per-window) rather than update an
//! existing entry — the integrity layer does not constrain this.
//!
//! ## Deferred rules
//!
//! - T16 aggregator enforces k-anonymity threshold before publication.
//! - T15 tending lifecycle provides `filter_subject_json` inner schema.
//! - T9/T16 cross-agent privacy sweettests live in the coordinator layer.

use hdi::prelude::*;

/// Bootstrap classification whitelist.
///
/// Mirrors T5 (`AttentionTending`) CLASSIFICATIONS — both entry types use the
/// same four values, which ensures the individual tending record (T5) and the
/// aggregate (T6) speak the same vocabulary.
const CLASSIFICATIONS: &[&str] = &["values-forward", "fatigue", "scope-mismatch", "safety"];

/// Bootstrap trend whitelist.
///
/// Three states for the direction of participation over the context window.
const TRENDS: &[&str] = &["rising", "stable", "falling"];

/// CollectiveFilterPattern integrity entry — Phase 3.5 k-anonymous aggregate.
///
/// Published to the DHT by T16 once ≥ k collective members have tended the
/// same filter pattern. No individual peer is identified here — publisher
/// identity lives in the Holochain action header, not the entry payload.
///
/// Invariants enforced by [`CollectiveFilterPattern::validate`]:
/// - `collective` must be non-empty.
/// - `classification` must be one of the four bootstrap kinds.
/// - `participating_pct` must be 0–100 (u8 prevents >255; validator caps ≤ 100).
/// - `trend` must be one of the three bootstrap kinds.
/// - `context_window_seconds` must be ≥ 3600 (1 hour minimum — prevents
///   tiny-window noise being published as a community signal).
///
/// `filter_subject_json` holds a pre-stringified JSON stub; T15/T16 know the
/// inner schema. The integrity zome enforces only the `String` type boundary.
#[hdk_entry_helper]
#[derive(Clone)]
pub struct CollectiveFilterPattern {
    /// Collective ID (qahal identifier). Non-empty.
    ///
    /// Identifies the collective (qahal, patron circle, household, DAO, etc.)
    /// whose members produced this aggregate. Not a peer identity.
    pub collective: String,

    /// JSON-encoded FilterSubject — inner schema deferred to T15/T16.
    ///
    /// HDI accepts any String here; T16 aggregator gates JSON syntax via
    /// `serde_json::from_str` before calling `create_entry`. Malformed JSON
    /// committed via the integrity zome is silently accepted at this layer.
    pub filter_subject_json: String,

    /// Whitelisted: values-forward / fatigue / scope-mismatch / safety.
    ///
    /// Mirrors the same whitelist as T5 `AttentionTending`.
    pub classification: String,

    /// Participation percentage; 0–100.
    ///
    /// `u8` prevents values > 255; the validator caps at 100. Represents
    /// the fraction of collective members whose source-chain tending records
    /// contributed to this aggregate (exact semantics defined by T16).
    pub participating_pct: u8,

    /// Whitelisted: rising / stable / falling.
    ///
    /// Direction of participation over the `context_window_seconds` window.
    pub trend: String,

    /// Window (seconds) over which the aggregate was computed.
    ///
    /// Minimum 3600 (1 hour) to prevent tiny-window noise being published as
    /// a community signal. Classification-specific window defaults are deferred
    /// to the T16 aggregator service.
    pub context_window_seconds: u64,
}

impl CollectiveFilterPattern {
    /// Phase 3.5 create-time floor checks. Deterministic, no DHT lookups.
    /// Returns `Ok(())` when valid; `Err(reason)` when a floor is violated.
    ///
    /// Floor ordering is deliberate: collective → classification → pct → trend →
    /// context_window. A caller with multiple violations sees the highest-priority
    /// failure first.
    pub fn validate(&self) -> Result<(), String> {
        // Floor 1: collective non-empty.
        //
        // A collective aggregate with no collective identifier is meaningless;
        // the receiving peer cannot correlate it with a qahal membership.
        if self.collective.is_empty() {
            return Err("collective must not be empty".to_string());
        }

        // Floor 2: classification whitelist.
        //
        // Same four bootstrap values as T5 AttentionTending — vocabulary parity
        // is intentional so individual tendings and aggregates speak the same
        // classification language.
        if !CLASSIFICATIONS.contains(&self.classification.as_str()) {
            return Err(format!(
                "unknown classification: {} (allowed: {:?})",
                self.classification, CLASSIFICATIONS
            ));
        }

        // Floor 3: participating_pct in [0, 100].
        //
        // u8 already prevents values > 255; this floor prevents logically
        // impossible percentages (101–255). 0% is technically valid — a
        // collective with zero participation could still be recorded, though
        // T16 would not normally publish at 0%.
        if self.participating_pct > 100 {
            return Err(format!(
                "participating_pct must be 0-100, got {}",
                self.participating_pct
            ));
        }

        // Floor 4: trend whitelist.
        if !TRENDS.contains(&self.trend.as_str()) {
            return Err(format!(
                "unknown trend: {} (allowed: {:?})",
                self.trend, TRENDS
            ));
        }

        // Floor 5: context_window_seconds >= 3600 (1 hour minimum).
        //
        // Tiny windows produce noise signals that have no community value and
        // could be used to deanonymize individual tendings through timing
        // correlation. One hour is the minimum useful window for collective
        // pattern emergence.
        if self.context_window_seconds < 3600 {
            return Err(format!(
                "context_window_seconds must be >= 3600 (1 hour minimum), got {}",
                self.context_window_seconds
            ));
        }

        // No peer-identity field to validate — by design (brainstorm §6.4).
        // T16 aggregator's k-anonymity threshold (default k=5) gates publication.

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

    fn make_pattern(
        collective: &str,
        classification: &str,
        participating_pct: u8,
        trend: &str,
        context_window_seconds: u64,
    ) -> CollectiveFilterPattern {
        CollectiveFilterPattern {
            collective: collective.to_string(),
            filter_subject_json: r#"{"contentKind":"concept"}"#.to_string(),
            classification: classification.to_string(),
            participating_pct,
            trend: trend.to_string(),
            context_window_seconds,
        }
    }

    fn valid() -> CollectiveFilterPattern {
        make_pattern("household-alpha", "values-forward", 42, "rising", 3600)
    }

    // -----------------------------------------------------------------------
    // Baseline: all-valid passes
    // -----------------------------------------------------------------------

    #[test]
    fn baseline_valid_passes() {
        assert!(valid().validate().is_ok());
    }

    // -----------------------------------------------------------------------
    // Floor 1: collective non-empty
    // -----------------------------------------------------------------------

    #[test]
    fn rejects_empty_collective() {
        let p = make_pattern("", "values-forward", 42, "rising", 3600);
        let result = p.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("collective must not be empty"),
            "expected collective message, got: {msg}"
        );
    }

    #[test]
    fn accepts_nonempty_collective() {
        let p = make_pattern("qahal-genesis", "fatigue", 30, "stable", 7200);
        assert!(p.validate().is_ok());
    }

    // -----------------------------------------------------------------------
    // Floor 2: classification whitelist — all 4 accepted
    // -----------------------------------------------------------------------

    #[test]
    fn accepts_classification_values_forward() {
        let p = make_pattern("coll-1", "values-forward", 50, "stable", 3600);
        assert!(p.validate().is_ok());
    }

    #[test]
    fn accepts_classification_fatigue() {
        let p = make_pattern("coll-1", "fatigue", 50, "stable", 3600);
        assert!(p.validate().is_ok());
    }

    #[test]
    fn accepts_classification_scope_mismatch() {
        let p = make_pattern("coll-1", "scope-mismatch", 50, "stable", 3600);
        assert!(p.validate().is_ok());
    }

    #[test]
    fn accepts_classification_safety() {
        let p = make_pattern("coll-1", "safety", 50, "stable", 3600);
        assert!(p.validate().is_ok());
    }

    #[test]
    fn rejects_unknown_classification() {
        let p = make_pattern("coll-1", "ban", 50, "stable", 3600);
        let result = p.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("unknown classification"),
            "expected 'unknown classification', got: {msg}"
        );
        assert!(msg.contains("ban"), "expected offending value in error, got: {msg}");
    }

    // -----------------------------------------------------------------------
    // Floor 3: participating_pct boundary (0, 100 accepted; 101 rejected)
    // -----------------------------------------------------------------------

    #[test]
    fn accepts_participating_pct_zero() {
        let p = make_pattern("coll-1", "fatigue", 0, "falling", 3600);
        assert!(p.validate().is_ok());
    }

    #[test]
    fn accepts_participating_pct_100() {
        let p = make_pattern("coll-1", "fatigue", 100, "rising", 3600);
        assert!(p.validate().is_ok());
    }

    #[test]
    fn rejects_participating_pct_101() {
        // u8 wraps at 255; 101 is still representable and logically invalid.
        let p = make_pattern("coll-1", "fatigue", 101, "rising", 3600);
        let result = p.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("participating_pct must be 0-100"),
            "expected pct message, got: {msg}"
        );
        assert!(msg.contains("101"), "expected offending value in error, got: {msg}");
    }

    // -----------------------------------------------------------------------
    // Floor 4: trend whitelist — all 3 accepted
    // -----------------------------------------------------------------------

    #[test]
    fn accepts_trend_rising() {
        let p = make_pattern("coll-1", "values-forward", 20, "rising", 3600);
        assert!(p.validate().is_ok());
    }

    #[test]
    fn accepts_trend_stable() {
        let p = make_pattern("coll-1", "values-forward", 20, "stable", 3600);
        assert!(p.validate().is_ok());
    }

    #[test]
    fn accepts_trend_falling() {
        let p = make_pattern("coll-1", "values-forward", 20, "falling", 3600);
        assert!(p.validate().is_ok());
    }

    #[test]
    fn rejects_unknown_trend() {
        let p = make_pattern("coll-1", "values-forward", 20, "spiking", 3600);
        let result = p.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("unknown trend"),
            "expected 'unknown trend', got: {msg}"
        );
        assert!(msg.contains("spiking"), "expected offending value in error, got: {msg}");
    }

    // -----------------------------------------------------------------------
    // Floor 5: context_window_seconds boundary (3599 rejected, 3600 accepted)
    // -----------------------------------------------------------------------

    #[test]
    fn rejects_context_window_3599() {
        let p = make_pattern("coll-1", "fatigue", 25, "stable", 3599);
        let result = p.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("context_window_seconds"),
            "expected 'context_window_seconds' in error, got: {msg}"
        );
        assert!(msg.contains("3599"), "expected offending value in error, got: {msg}");
    }

    #[test]
    fn accepts_context_window_3600() {
        let p = make_pattern("coll-1", "fatigue", 25, "stable", 3600);
        assert!(p.validate().is_ok());
    }

    #[test]
    fn accepts_context_window_large() {
        // 7-day window: valid
        let p = make_pattern("coll-1", "scope-mismatch", 60, "falling", 604_800);
        assert!(p.validate().is_ok());
    }

    // -----------------------------------------------------------------------
    // Floor ordering: collective fires before classification
    // -----------------------------------------------------------------------

    #[test]
    fn floor_order_collective_before_classification() {
        // Empty collective AND invalid classification — collective fires first.
        let p = make_pattern("", "invalid-class", 101, "spiking", 100);
        let result = p.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("collective must not be empty"),
            "collective floor should fire first, got: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Floor ordering: classification fires before trend
    // -----------------------------------------------------------------------

    #[test]
    fn floor_order_classification_before_trend() {
        // Valid collective, invalid classification AND invalid trend.
        let p = make_pattern("coll-1", "invalid-class", 50, "spiking", 3600);
        let result = p.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("unknown classification"),
            "classification floor should fire before trend, got: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // K-anonymity property: NO signer_pubkey field
    // -----------------------------------------------------------------------
    //
    // The struct definition enforces this at compile time — `CollectiveFilterPattern`
    // has no `signer_pubkey` field. This test documents the property in the test
    // suite so reviewers notice immediately if it is accidentally added.
    //
    // We verify by attempting to construct the struct with all known fields and
    // confirming the set matches the brainstorm §6.4 spec exactly.
    #[test]
    fn struct_has_no_signer_pubkey() {
        // If this compiles, the struct has exactly these fields and no signer_pubkey.
        // Adding signer_pubkey would break this exhaustive construction.
        let _p = CollectiveFilterPattern {
            collective: "coll-1".to_string(),
            filter_subject_json: r#"{}"#.to_string(),
            classification: "fatigue".to_string(),
            participating_pct: 30,
            trend: "stable".to_string(),
            context_window_seconds: 7200,
        };
        // If this line is reached, no signer_pubkey field exists.
        assert!(true);
    }

    // -----------------------------------------------------------------------
    // Update-behavior coverage note
    // -----------------------------------------------------------------------
    //
    // `validate_update_entry` in `lib.rs` falls through to `validate_create_entry`
    // for CollectiveFilterPattern — the same floor checks apply to the updated
    // state.  T16 aggregator service may convention to always-create-new-entries
    // (one snapshot per context window) rather than updating existing entries.
    // This is a service-level convention, not enforced at the integrity layer.
    //
    // A sweettest verifying DHT gossip behavior is deferred to T16.
}
