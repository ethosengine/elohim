//! AttentionTending wire type — Phase 3.5 Trust-Compute Gradient tending subsystem.
//!
//! Wire contract for the `AttentionTending` EPR kind. This is a **peer-private**
//! discernment signal that stays on the agent's source chain and is never
//! gossiped to other peers. T5 adds the `Visibility::Private` HDI integrity
//! entry; this module is the libp2p data-ops wire shape used within the local
//! elohim and for k-anonymous aggregation at T16.
//!
//! ## Category classification
//!
//! Category **B** — agent-scoped (private). The signal is authored by the local
//! agent about their own attention pattern. No attestation to external peers.
//! Privacy is structural: `Visibility::Private` at the HDI layer means the
//! entry never leaves the source chain. The tending decision belongs to the human
//! and their elohim only.
//!
//! ## Wire format
//!
//! MessagePack-encoded via `rmp_serde`. `camelCase` field names via
//! `#[serde(rename_all = "camelCase")]`. Enum variants use `kebab-case`
//! on the wire so `"values-forward"` round-trips correctly.
//!
//! Schema: `elohim/sdk/schemas/v1/p2p/attention-tending.schema.json`
//!
//! ## Four classifications (graduated)
//!
//! - **values-forward**: human values-aligned discernment. 30-day default TTL.
//! - **fatigue**: attention/bandwidth exhaustion. 7-day default TTL.
//! - **scope-mismatch**: content outside current context. 90-day default TTL.
//! - **safety**: constitutional-floor concern. No expiry (lifecycle service
//!   at T15 enforces this; the TTL floor of 3600s applies on-wire only).
//!
//! ## TTL semantics
//!
//! `ttl_seconds` is an on-wire floor (minimum 3600 = 1 hour) to prevent
//! permanent moats. Classification-specific defaults and the "safety has no
//! expiry" rule are enforced by the tending lifecycle service at T15.
//!
//! ## Stub fields
//!
//! `filter_subject` and `context` are open `serde_json::Value` stubs at T2.
//! T15 (tending lifecycle service) may enrich the inner shape. Using `Value`
//! here is intentional and safe: this module is in `elohim-storage`, NOT in
//! a Holochain zome, so `serde_json::Value` does not break the zome boundary.
//!
//! Source: genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md §6.1

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Graduated tending classification for an [`AttentionTending`].
///
/// Wire encoding: kebab-case via `#[serde(rename_all = "kebab-case")]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum Classification {
    /// Human values-aligned discernment. Default TTL: 30 days.
    ValuesForward,
    /// Attention or bandwidth exhaustion. Default TTL: 7 days.
    Fatigue,
    /// Content is outside the current context or scope. Default TTL: 90 days.
    ScopeMismatch,
    /// Constitutional-floor concern. No expiry by convention (enforced at T15).
    Safety,
}

// ---------------------------------------------------------------------------
// Wire struct
// ---------------------------------------------------------------------------

/// AttentionTending EPR kind — Phase 3.5 tending subsystem peer-private primitive.
///
/// A TTL-bounded classification of a filter pattern (content kind, topic,
/// author scope, etc.) that feeds the elohim's tender conversation and the
/// collective's anonymous wisdom layer. Peer-private by default — stays on
/// the agent's source chain, never gossiped.
///
/// Invariants enforced by [`AttentionTending::validate`]:
/// - `tended_at` MUST be non-empty.
/// - `ttl_seconds` MUST be ≥ 3600 (1 hour minimum).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AttentionTending {
    /// Pattern to tend: content kind, topic, author scope, etc.
    /// Open stub — T15 may enrich the inner shape.
    #[ts(type = "Record<string, unknown>")]
    pub filter_subject: serde_json::Value,

    /// Graduated tending classification.
    pub classification: Classification,

    /// Optional human-readable reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reason: Option<String>,

    /// TTL in seconds. Minimum 1 hour (3600) to prevent permanent moats.
    pub ttl_seconds: u64,

    /// Unix timestamps of tending events. The earliest is creation; later
    /// entries are re-tending events. Must be non-empty.
    pub tended_at: Vec<u64>,

    /// Scope where this tending applies: collective, mode, time-of-day, etc.
    /// Open stub — T15 may enrich the inner shape.
    #[ts(type = "Record<string, unknown>")]
    pub context: serde_json::Value,

    /// Base64-encoded ed25519 public key of the tending agent.
    pub signed_by: String,

    /// Base64-encoded ed25519 signature over canonical bytes.
    pub signature: String,
}

impl AttentionTending {
    /// Validate structural invariants.
    ///
    /// Returns `Ok(())` if the struct satisfies all on-wire constraints.
    /// Returns `Err` with a description for each violated constraint:
    /// - `tended_at` must be non-empty (a tending with no events is meaningless).
    /// - `ttl_seconds` must be ≥ 3600 (1-hour floor prevents permanent moats).
    pub fn validate(&self) -> Result<(), String> {
        if self.tended_at.is_empty() {
            return Err("tended_at must be non-empty".to_string());
        }
        if self.ttl_seconds < 3600 {
            return Err(format!(
                "ttl_seconds must be >= 3600 (got {})",
                self.ttl_seconds
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tending(classification: Classification, reason: Option<&str>) -> AttentionTending {
        AttentionTending {
            filter_subject: serde_json::json!({"contentKind": "concept"}),
            classification,
            reason: reason.map(|s| s.to_string()),
            ttl_seconds: 3600,
            tended_at: vec![1_746_000_000],
            context: serde_json::json!({"collective": "household"}),
            signed_by: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string(),
            signature: "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // JSON round-trips for all 4 classifications
    // -----------------------------------------------------------------------

    #[test]
    fn json_roundtrip_values_forward() {
        let t = make_tending(Classification::ValuesForward, None);
        let json = serde_json::to_string(&t).unwrap();
        assert!(
            json.contains("\"values-forward\""),
            "wire name should be values-forward, got: {json}"
        );
        let back: AttentionTending = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn json_roundtrip_fatigue() {
        let t = make_tending(Classification::Fatigue, None);
        let json = serde_json::to_string(&t).unwrap();
        assert!(
            json.contains("\"fatigue\""),
            "wire name should be fatigue, got: {json}"
        );
        let back: AttentionTending = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn json_roundtrip_scope_mismatch() {
        let t = make_tending(Classification::ScopeMismatch, None);
        let json = serde_json::to_string(&t).unwrap();
        assert!(
            json.contains("\"scope-mismatch\""),
            "wire name should be scope-mismatch, got: {json}"
        );
        let back: AttentionTending = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn json_roundtrip_safety() {
        let t = make_tending(Classification::Safety, None);
        let json = serde_json::to_string(&t).unwrap();
        assert!(
            json.contains("\"safety\""),
            "wire name should be safety, got: {json}"
        );
        let back: AttentionTending = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    // -----------------------------------------------------------------------
    // MessagePack round-trip (rmp_serde) — at least one classification
    // -----------------------------------------------------------------------

    #[test]
    fn msgpack_roundtrip_all_classifications() {
        let cases = vec![
            make_tending(Classification::ValuesForward, None),
            make_tending(Classification::Fatigue, Some("too much on this topic")),
            make_tending(Classification::ScopeMismatch, None),
            make_tending(Classification::Safety, None),
        ];

        for t in cases {
            let bytes = rmp_serde::to_vec_named(&t)
                .unwrap_or_else(|e| panic!("msgpack encode failed for {t:?}: {e}"));
            let back: AttentionTending = rmp_serde::from_slice(&bytes)
                .unwrap_or_else(|e| panic!("msgpack decode failed for {t:?}: {e}"));
            assert_eq!(
                back, t,
                "MessagePack round-trip failed for classification={:?}",
                t.classification
            );
        }
    }

    // -----------------------------------------------------------------------
    // validate() rejects empty tended_at
    // -----------------------------------------------------------------------

    #[test]
    fn validate_rejects_empty_tended_at() {
        let t = AttentionTending {
            filter_subject: serde_json::json!({}),
            classification: Classification::Fatigue,
            reason: None,
            ttl_seconds: 3600,
            tended_at: vec![],
            context: serde_json::json!({}),
            signed_by: "AAAAAAA=".to_string(),
            signature: "BBBBBBB=".to_string(),
        };
        let result = t.validate();
        assert!(result.is_err(), "empty tended_at should fail validate()");
        assert!(
            result.unwrap_err().contains("tended_at"),
            "error message should mention tended_at"
        );
    }

    // -----------------------------------------------------------------------
    // validate() rejects ttl_seconds < 3600
    // -----------------------------------------------------------------------

    #[test]
    fn validate_rejects_ttl_below_minimum() {
        let t = AttentionTending {
            filter_subject: serde_json::json!({}),
            classification: Classification::ValuesForward,
            reason: None,
            ttl_seconds: 100,
            tended_at: vec![1_746_000_000],
            context: serde_json::json!({}),
            signed_by: "AAAAAAA=".to_string(),
            signature: "BBBBBBB=".to_string(),
        };
        let result = t.validate();
        assert!(result.is_err(), "ttl_seconds=100 should fail validate()");
        assert!(
            result.unwrap_err().contains("ttl_seconds"),
            "error message should mention ttl_seconds"
        );
    }

    // -----------------------------------------------------------------------
    // validate() accepts a well-formed instance
    // -----------------------------------------------------------------------

    #[test]
    fn validate_accepts_well_formed_instance() {
        let t = make_tending(Classification::ScopeMismatch, None);
        assert!(
            t.validate().is_ok(),
            "well-formed instance should pass validate()"
        );
    }

    // -----------------------------------------------------------------------
    // Required field rejection on deserialize
    // -----------------------------------------------------------------------

    #[test]
    fn deserialize_rejects_missing_classification() {
        let json = r#"{
            "filterSubject": {"contentKind": "concept"},
            "ttlSeconds": 3600,
            "tendedAt": [1746000000],
            "context": {},
            "signedBy": "AAAAAAA=",
            "signature": "BBBBBBB="
        }"#;
        let result: Result<AttentionTending, _> = serde_json::from_str(json);
        assert!(result.is_err(), "should reject missing classification");
    }

    #[test]
    fn deserialize_rejects_missing_filter_subject() {
        let json = r#"{
            "classification": "fatigue",
            "ttlSeconds": 3600,
            "tendedAt": [1746000000],
            "context": {},
            "signedBy": "AAAAAAA=",
            "signature": "BBBBBBB="
        }"#;
        let result: Result<AttentionTending, _> = serde_json::from_str(json);
        assert!(result.is_err(), "should reject missing filterSubject");
    }

    // -----------------------------------------------------------------------
    // Optional `reason` is omitted from serialization when None
    // -----------------------------------------------------------------------

    #[test]
    fn optional_reason_absent_when_none() {
        let t = make_tending(Classification::ValuesForward, None);
        let json = serde_json::to_string(&t).unwrap();
        assert!(
            !json.contains("\"reason\""),
            "reason key should not appear in JSON when None, got: {json}"
        );
    }

    #[test]
    fn optional_reason_present_when_some() {
        let t = make_tending(Classification::Fatigue, Some("testing reasons"));
        let json = serde_json::to_string(&t).unwrap();
        assert!(
            json.contains("\"reason\""),
            "reason key should appear in JSON when Some, got: {json}"
        );
        assert!(
            json.contains("\"testing reasons\""),
            "reason value should appear in JSON, got: {json}"
        );
    }
}
