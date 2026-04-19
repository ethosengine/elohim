//! `AggregationSpec` — deserializable form of an aggregation-spec ContentNode body.
//!
//! Mirrors `lamad:schema:metadata:aggregation-spec` exactly. Three reduction
//! shapes are supported: `count`, `weighted-sum`, and `threshold-ladder`. The
//! spec is fetched by CID via a [`ContentNodeResolver`] and parsed here; the
//! `AggregateAttestationsExecutor` does not deal with raw JSON beyond the
//! initial `serde_json::from_value` call.
//!
//! # Source of truth
//!
//! `elohim/sdk/domains/lamad/schemas/aggregation-spec-metadata.schema.json`

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::GateError;

// ─── AggregationSpec ──────────────────────────────────────────────────────────

/// Top-level aggregation spec parsed from a ContentNode body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregationSpec {
    pub spec_name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub attestation_kinds: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_scope: Option<SubjectScope>,
    /// ISO 8601 duration string, or null for all-time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_window: Option<String>,
    pub reduction: Reduction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_shape: Option<String>,
}

impl AggregationSpec {
    /// Parse from a `serde_json::Value` (the ContentNode body).
    pub fn from_json(body: &Value) -> Result<Self, GateError> {
        serde_json::from_value(body.clone()).map_err(|e| {
            GateError::DagExecution(format!(
                "AggregationSpec: failed to deserialize spec body: {e}"
            ))
        })
    }
}

// ─── SubjectScope ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SubjectScope {
    ContentNode,
    Agent,
    GateDecision,
    AppManifest,
}

// ─── Reduction ────────────────────────────────────────────────────────────────

/// The three reduction shapes from the aggregation-spec schema.
///
/// `kind` is the JSON discriminator.  Uses `#[serde(tag = "kind")]` so the
/// on-wire JSON is `{ "kind": "count", "requiredCount": 5 }` — matching the
/// schema's `oneOf` entries exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Reduction {
    Count {
        #[serde(rename = "requiredCount")]
        required_count: u64,
    },
    #[serde(rename = "weighted-sum")]
    WeightedSum {
        /// Map of attestation kind → weight.
        weights: HashMap<String, f64>,
        threshold: f64,
    },
    #[serde(rename = "threshold-ladder")]
    ThresholdLadder { tiers: Vec<LadderTier> },
}

// ─── LadderTier ───────────────────────────────────────────────────────────────

/// A single tier in a threshold-ladder reduction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LadderTier {
    /// Output label for this tier (e.g., "community", "public", "protocol").
    pub level: String,
    /// Minimum qualifying count (attestation total) for this tier.
    pub min_count: u64,
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn count_spec_json() -> Value {
        json!({
            "specName": "reach-level-v1",
            "version": "1.0.0",
            "attestationKinds": ["brit", "reach-endorsement"],
            "subjectScope": "content-node",
            "timeWindow": "P180D",
            "reduction": {
                "kind": "count",
                "requiredCount": 5
            },
            "outputShape": "ReachLevel"
        })
    }

    fn weighted_sum_spec_json() -> Value {
        json!({
            "specName": "weighted-reach-v1",
            "version": "1.0.0",
            "attestationKinds": ["brit", "reach-endorsement", "deep-engagement"],
            "reduction": {
                "kind": "weighted-sum",
                "weights": {
                    "brit": 2.0,
                    "reach-endorsement": 1.0,
                    "deep-engagement": 3.0
                },
                "threshold": 10.0
            }
        })
    }

    fn threshold_ladder_spec_json() -> Value {
        json!({
            "specName": "reach-tier-v1",
            "version": "1.0.0",
            "attestationKinds": ["brit"],
            "reduction": {
                "kind": "threshold-ladder",
                "tiers": [
                    { "level": "protocol", "minCount": 20 },
                    { "level": "public",   "minCount": 10 },
                    { "level": "community","minCount": 3  }
                ]
            }
        })
    }

    #[test]
    fn parse_count_spec() {
        let spec = AggregationSpec::from_json(&count_spec_json()).unwrap();
        assert_eq!(spec.spec_name, "reach-level-v1");
        assert_eq!(spec.attestation_kinds, vec!["brit", "reach-endorsement"]);
        assert_eq!(spec.time_window, Some("P180D".into()));
        assert!(matches!(
            spec.reduction,
            Reduction::Count { required_count: 5 }
        ));
    }

    #[test]
    fn parse_weighted_sum_spec() {
        let spec = AggregationSpec::from_json(&weighted_sum_spec_json()).unwrap();
        match spec.reduction {
            Reduction::WeightedSum { weights, threshold } => {
                assert_eq!(*weights.get("brit").unwrap(), 2.0);
                assert_eq!(*weights.get("deep-engagement").unwrap(), 3.0);
                assert_eq!(threshold, 10.0);
            }
            other => panic!("expected WeightedSum, got {other:?}"),
        }
    }

    #[test]
    fn parse_threshold_ladder_spec() {
        let spec = AggregationSpec::from_json(&threshold_ladder_spec_json()).unwrap();
        match &spec.reduction {
            Reduction::ThresholdLadder { tiers } => {
                assert_eq!(tiers.len(), 3);
                assert_eq!(tiers[0].level, "protocol");
                assert_eq!(tiers[0].min_count, 20);
                assert_eq!(tiers[2].level, "community");
                assert_eq!(tiers[2].min_count, 3);
            }
            other => panic!("expected ThresholdLadder, got {other:?}"),
        }
    }

    #[test]
    fn malformed_spec_returns_error() {
        let bad = json!({ "notASpec": true });
        let err = AggregationSpec::from_json(&bad).unwrap_err();
        assert!(matches!(err, GateError::DagExecution(_)));
    }

    #[test]
    fn optional_fields_can_be_absent() {
        let minimal = json!({
            "specName": "min-spec",
            "version": "1.0.0",
            "attestationKinds": ["brit"],
            "reduction": { "kind": "count", "requiredCount": 1 }
        });
        let spec = AggregationSpec::from_json(&minimal).unwrap();
        assert!(spec.description.is_none());
        assert!(spec.subject_scope.is_none());
        assert!(spec.time_window.is_none());
        assert!(spec.output_shape.is_none());
    }
}
