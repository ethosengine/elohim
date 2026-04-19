//! Reach aggregation spec v1 — embedded loader and stable reference constants.
//!
//! This module bundles the `reach-v1` aggregation-spec JSON at compile time and
//! exposes it as an [`AggregationSpec`] via [`default_reach_aggregation`].
//!
//! # CID reference
//!
//! The spec is addressed at runtime by [`REACH_AGGREGATION_V1_CID`].
//! Phase 5 resolves this via `EmbeddedContentNodeResolver` pre-loaded with
//! this CID → body pair. When a DHT-backed resolver is available, the same
//! CID will resolve via EPR lookup once the aggregation-spec ContentNode is
//! seeded.
//!
//! # Aggregation design
//!
//! The spec uses a `threshold-ladder` reduction with three named tiers:
//!
//! | Level       | minCount | Meaning                                     |
//! |-------------|----------|---------------------------------------------|
//! | `protocol`  | 50       | Existential-boundary peers recognise content |
//! | `public`    | 20       | Open audience endorsement                   |
//! | `community` | 5        | Known community recognition                 |
//! | `self-reach`| (default)| Below community threshold; contributors only |
//!
//! The `self-reach` level is implicit — it is the executor's default when no
//! tier matches (`level: null` from the ladder walks to `"self-reach"` in the
//! `verdict-from-reach` synthesize builder).
//!
//! # Attestation kinds
//!
//! Four kinds feed this aggregation:
//! - `gate-decision` — a gate verdict (any gate) referencing this content
//! - `witness` — a seven-valence witness attestation on an experience-story
//!   associated with this content node
//! - `endorsement` — an explicit peer endorsement attestation
//! - `content-approval` — a steward-consent approval attestation
//!
//! Note: the selection of these four kinds is a v1 draft (see user-review
//! concern in reach_gate.rs).  The `witness` kind feeds reach directly from
//! the discernment-gate's story-point accumulation, creating a live link
//! between learning engagement and audience visibility.

use crate::dag::aggregation_spec::AggregationSpec;
use crate::error::GateError;

// ─── Stable CID reference ─────────────────────────────────────────────────────

/// Stable EPR-style CID used to address the reach-aggregation-v1 spec artifact.
///
/// Phase 5 resolves this via `EmbeddedContentNodeResolver`. Phase 6+ will
/// resolve it via DHT EPR lookup once the aggregation-spec ContentNode is
/// seeded.
pub const REACH_AGGREGATION_V1_CID: &str = "epr:specs:reach-v1";

/// The version string embedded in the spec body.
pub const REACH_AGGREGATION_V1_VERSION: &str = "1.0.0";

// ─── Embedded JSON ────────────────────────────────────────────────────────────

/// The raw JSON body of the reach aggregation spec, embedded at compile time
/// from `src/dag/specs/reach_aggregation_v1.json`.
pub const REACH_AGGREGATION_V1_BODY: &str = include_str!("specs/reach_aggregation_v1.json");

// ─── Loader ───────────────────────────────────────────────────────────────────

/// Parse and return the embedded reach aggregation spec as an [`AggregationSpec`].
///
/// # Errors
///
/// Returns `GateError::DagExecution` if the embedded JSON cannot be parsed.
/// In practice this should never fail because the JSON is validated by the
/// compile-time unit test below.
pub fn default_reach_aggregation() -> Result<AggregationSpec, GateError> {
    let body: serde_json::Value = serde_json::from_str(REACH_AGGREGATION_V1_BODY).map_err(|e| {
        GateError::DagExecution(format!(
            "failed to parse embedded reach_aggregation_v1.json: {e}"
        ))
    })?;
    AggregationSpec::from_json(&body)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::aggregation_spec::{Reduction, SubjectScope};

    // ─── T1: embedded JSON parses without error ───────────────────────────────

    #[test]
    fn embedded_json_parses_successfully() {
        let spec = default_reach_aggregation()
            .expect("embedded reach_aggregation_v1.json must parse without error");
        assert_eq!(spec.spec_name, "reach-v1");
        assert_eq!(spec.version, "1.0.0");
    }

    // ─── T2: BODY constant is valid JSON ─────────────────────────────────────

    #[test]
    fn body_constant_is_valid_json() {
        serde_json::from_str::<serde_json::Value>(REACH_AGGREGATION_V1_BODY)
            .expect("REACH_AGGREGATION_V1_BODY must be valid JSON");
    }

    // ─── T3: CID constant is stable, version matches embedded body ───────────

    #[test]
    fn cid_constant_format_and_version_match() {
        assert_eq!(REACH_AGGREGATION_V1_CID, "epr:specs:reach-v1");
        let spec = default_reach_aggregation().unwrap();
        assert_eq!(spec.version, REACH_AGGREGATION_V1_VERSION);
    }

    // ─── T4: outputShape is ReachLevel ───────────────────────────────────────

    #[test]
    fn output_shape_is_reach_level() {
        let spec = default_reach_aggregation().unwrap();
        assert_eq!(
            spec.output_shape.as_deref(),
            Some("ReachLevel"),
            "outputShape must be 'ReachLevel'"
        );
    }

    // ─── T5: subjectScope is content-node ────────────────────────────────────

    #[test]
    fn subject_scope_is_content_node() {
        let spec = default_reach_aggregation().unwrap();
        assert_eq!(
            spec.subject_scope,
            Some(SubjectScope::ContentNode),
            "subjectScope must be ContentNode"
        );
    }

    // ─── T6: timeWindow is P365D ──────────────────────────────────────────────

    #[test]
    fn time_window_is_p365d() {
        let spec = default_reach_aggregation().unwrap();
        assert_eq!(
            spec.time_window.as_deref(),
            Some("P365D"),
            "timeWindow must be 'P365D'"
        );
    }

    // ─── T7: reduction is threshold-ladder with 3 tiers ──────────────────────

    #[test]
    fn reduction_is_threshold_ladder_with_three_tiers() {
        let spec = default_reach_aggregation().unwrap();
        match &spec.reduction {
            Reduction::ThresholdLadder { tiers } => {
                assert_eq!(
                    tiers.len(),
                    3,
                    "threshold-ladder must have exactly 3 tiers, got {}",
                    tiers.len()
                );
            }
            other => panic!("expected ThresholdLadder reduction, got {other:?}"),
        }
    }

    // ─── T8: tier ordering — protocol > public > community ───────────────────
    //
    // The executor walks tiers in order and picks the first one met; tiers MUST
    // be declared highest-first so `protocol` (50) is checked before `public`
    // (20) and `community` (5).

    #[test]
    fn tier_ordering_protocol_public_community() {
        let spec = default_reach_aggregation().unwrap();
        match &spec.reduction {
            Reduction::ThresholdLadder { tiers } => {
                assert_eq!(tiers[0].level, "protocol", "first tier must be protocol");
                assert_eq!(tiers[0].min_count, 50, "protocol minCount must be 50");

                assert_eq!(tiers[1].level, "public", "second tier must be public");
                assert_eq!(tiers[1].min_count, 20, "public minCount must be 20");

                assert_eq!(tiers[2].level, "community", "third tier must be community");
                assert_eq!(tiers[2].min_count, 5, "community minCount must be 5");
            }
            other => panic!("expected ThresholdLadder, got {other:?}"),
        }
    }

    // ─── T9: attestationKinds has expected four kinds ─────────────────────────

    #[test]
    fn attestation_kinds_has_four_expected_entries() {
        let spec = default_reach_aggregation().unwrap();
        let kinds = &spec.attestation_kinds;
        assert_eq!(
            kinds.len(),
            4,
            "attestationKinds must have 4 entries, got {}: {:?}",
            kinds.len(),
            kinds
        );
        for expected in &[
            "gate-decision",
            "witness",
            "endorsement",
            "content-approval",
        ] {
            assert!(
                kinds.contains(&expected.to_string()),
                "attestationKinds must contain '{expected}'; got: {kinds:?}"
            );
        }
    }

    // ─── T10: description is non-empty ───────────────────────────────────────

    #[test]
    fn description_is_present_and_non_empty() {
        let spec = default_reach_aggregation().unwrap();
        let desc = spec.description.as_deref().unwrap_or("");
        assert!(
            !desc.is_empty(),
            "description must be present and non-empty"
        );
    }
}
