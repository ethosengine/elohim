//! `RulesArtifact` — the in-memory representation of a `gate-rules-declaration`
//! ContentNode body.
//!
//! # Wire format
//!
//! `gate-rules-declaration` ContentNodes store their body as JSON (consistent
//! with all other ContentNode bodies). This module deserializes that JSON into
//! the [`RulesArtifact`] struct.
//!
//! # Body shape (authoritative for Task 3.2)
//!
//! Task 3.2 MUST produce JSON that matches this struct exactly. The shape:
//!
//! ```json
//! {
//!   "ruleSetName": "seven-valence-v1",
//!   "version": "1.0.0",
//!   "description": "Seven-valence mechanical discernment of experience-moments",
//!   "appliesToEventType": "attestation-write",
//!   "inputKeysExpected": ["moment", "priorAttestations"],
//!   "outputShape": "StoryPointTag",
//!   "rules": [
//!     {
//!       "id": "rule-1-first-pass-green",
//!       "when": "moment.status == \"passed\" AND priorAttestations.count == 0",
//!       "outcome": {
//!         "valence": "progress",
//!         "magnitude": "meaningful",
//!         "evidenceType": "first-pass-green"
//!       },
//!       "rationale": "First time the scenario goes green on any compute context."
//!     }
//!   ]
//! }
//! ```
//!
//! All fields except `description`, `rationale`, and `outputShape` are required.
//! The `rules` array is evaluated in declaration order; first-match-wins.
//!
//! # Rule condition language
//!
//! The `when` string is parsed by [`crate::dag::rule_expr::parse_rule_condition`].
//! Supported: `==`, `!=`, `AND` conjunction, dotted paths of any depth, literals
//! (`null`, `true`, `false`, number, single- or double-quoted string).
//! Unsupported: `OR`, `NOT`, arithmetic, function calls. Hard parse failure on
//! unsupported syntax.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::GateError;

// ─── Outcome ─────────────────────────────────────────────────────────────────

/// The outcome emitted when a rule matches.
///
/// This is written to the `output_key` in `GateContext` as a JSON object.
/// The shape is intentionally open (`serde_json::Value` fields on Outcome
/// are the raw JSON) to allow any app-domain tag shape. The `valence`,
/// `magnitude`, and `evidence_type` fields correspond to `StoryPointTag`
/// for the seven-valence discernment gate but the struct is not restricted
/// to that domain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuleOutcome {
    /// Semantic valence of the matched moment.
    ///
    /// For the seven-valence gate: `"progress"`, `"regression"`, `"discovery"`,
    /// `"validation"`, `"witness"`, `"refinement"`, `"confirmation"`.
    pub valence: String,

    /// Scale indicator. For the seven-valence gate: `"meaningful"` or `"small"`.
    pub magnitude: String,

    /// Evidence type discriminator. Examples: `"first-pass-green"`,
    /// `"known-cause-recurrence"`, `"novel-failure-class"`, `"recovery"`,
    /// `"cross-fingerprint-attestation"`, `"evidence-enriched"`.
    pub evidence_type: String,
}

// ─── Rule ─────────────────────────────────────────────────────────────────────

/// A single declarative rule inside a `gate-rules-declaration`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    /// Stable identifier for this rule (used in tracing / audit). Must be
    /// unique within the rule set.
    pub id: String,

    /// Condition expression parsed by [`crate::dag::rule_expr::parse_rule_condition`].
    ///
    /// Example: `"moment.status == \"passed\" AND priorAttestations.count == 0"`
    pub when: String,

    /// The outcome emitted when `when` is satisfied.
    pub outcome: RuleOutcome,

    /// Human-readable rationale (optional). Never evaluated; for inspection only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

// ─── RulesArtifact ───────────────────────────────────────────────────────────

/// In-memory representation of a `gate-rules-declaration` ContentNode body.
///
/// Deserialize from `serde_json::from_value(content_node.body)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesArtifact {
    /// Stable name for the rule set. Used in tracing and logging.
    ///
    /// Example: `"seven-valence-v1"`
    pub rule_set_name: String,

    /// Semantic version of the rule set.
    pub version: String,

    /// Human-readable description (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The `eventType` string this rule set is designed for. Used for
    /// inspection / validation; not enforced at runtime.
    ///
    /// Example: `"attestation-write"`
    pub applies_to_event_type: String,

    /// The `GateContext` keys the rule evaluator expects to be populated before
    /// the ruleset runs. Used for inspection / validation.
    pub input_keys_expected: Vec<String>,

    /// Describes the shape of values written to `output_key`.
    ///
    /// Example: `"StoryPointTag"`. For inspection only; not a runtime type
    /// constraint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_shape: Option<String>,

    /// The rules, evaluated in declaration order (first-match-wins).
    pub rules: Vec<Rule>,
}

impl RulesArtifact {
    /// Deserialize from a `serde_json::Value` (the ContentNode's body field).
    ///
    /// Returns `GateError::DagExecution` if the JSON does not conform to the
    /// expected shape.
    pub fn from_json(body: &Value) -> Result<Self, GateError> {
        serde_json::from_value(body.clone()).map_err(|e| {
            GateError::DagExecution(format!(
                "failed to parse gate-rules-declaration body as RulesArtifact: {e}"
            ))
        })
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn minimal_artifact_json() -> Value {
        json!({
            "ruleSetName": "test-rules-v1",
            "version": "1.0.0",
            "appliesToEventType": "attestation-write",
            "inputKeysExpected": ["moment", "priorAttestations"],
            "rules": [
                {
                    "id": "rule-1",
                    "when": "moment.status == \"passed\" AND priorAttestations.count == 0",
                    "outcome": {
                        "valence": "progress",
                        "magnitude": "meaningful",
                        "evidenceType": "first-pass-green"
                    }
                }
            ]
        })
    }

    fn full_artifact_json() -> Value {
        json!({
            "ruleSetName": "seven-valence-v1",
            "version": "1.0.0",
            "description": "Seven-valence mechanical discernment",
            "appliesToEventType": "attestation-write",
            "inputKeysExpected": ["moment", "priorAttestations"],
            "outputShape": "StoryPointTag",
            "rules": [
                {
                    "id": "rule-1-first-pass-green",
                    "when": "moment.status == \"passed\" AND priorAttestations.count == 0",
                    "outcome": {
                        "valence": "progress",
                        "magnitude": "meaningful",
                        "evidenceType": "first-pass-green"
                    },
                    "rationale": "First time the scenario goes green."
                },
                {
                    "id": "rule-2-regression",
                    "when": "moment.status == \"failed\" AND priorAttestations.mostRecentStatus == \"passed\"",
                    "outcome": {
                        "valence": "regression",
                        "magnitude": "meaningful",
                        "evidenceType": "known-cause-recurrence"
                    }
                }
            ]
        })
    }

    #[test]
    fn deserialize_minimal_artifact() {
        let artifact = RulesArtifact::from_json(&minimal_artifact_json()).unwrap();
        assert_eq!(artifact.rule_set_name, "test-rules-v1");
        assert_eq!(artifact.version, "1.0.0");
        assert_eq!(artifact.applies_to_event_type, "attestation-write");
        assert_eq!(
            artifact.input_keys_expected,
            vec!["moment", "priorAttestations"]
        );
        assert_eq!(artifact.rules.len(), 1);
        assert!(artifact.description.is_none());
        assert!(artifact.output_shape.is_none());
    }

    #[test]
    fn deserialize_full_artifact() {
        let artifact = RulesArtifact::from_json(&full_artifact_json()).unwrap();
        assert_eq!(artifact.rule_set_name, "seven-valence-v1");
        assert_eq!(
            artifact.description.as_deref(),
            Some("Seven-valence mechanical discernment")
        );
        assert_eq!(artifact.output_shape.as_deref(), Some("StoryPointTag"));
        assert_eq!(artifact.rules.len(), 2);

        let r1 = &artifact.rules[0];
        assert_eq!(r1.id, "rule-1-first-pass-green");
        assert_eq!(r1.outcome.valence, "progress");
        assert_eq!(r1.outcome.magnitude, "meaningful");
        assert_eq!(r1.outcome.evidence_type, "first-pass-green");
        assert_eq!(
            r1.rationale.as_deref(),
            Some("First time the scenario goes green.")
        );
    }

    #[test]
    fn deserialize_rule_without_rationale() {
        let artifact = RulesArtifact::from_json(&full_artifact_json()).unwrap();
        let r2 = &artifact.rules[1];
        assert!(r2.rationale.is_none());
    }

    #[test]
    fn from_json_rejects_invalid_body() {
        let bad = json!({ "notARulesArtifact": true });
        let err = RulesArtifact::from_json(&bad).unwrap_err();
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(msg.contains("gate-rules-declaration"), "got: {msg}");
    }

    #[test]
    fn from_json_rejects_null() {
        let err = RulesArtifact::from_json(&Value::Null).unwrap_err();
        assert!(matches!(err, GateError::DagExecution(_)));
    }

    #[test]
    fn roundtrip_serialize_deserialize() {
        let artifact = RulesArtifact::from_json(&full_artifact_json()).unwrap();
        let serialized = serde_json::to_value(&artifact).unwrap();
        let roundtrip = RulesArtifact::from_json(&serialized).unwrap();
        assert_eq!(roundtrip.rule_set_name, artifact.rule_set_name);
        assert_eq!(roundtrip.rules.len(), artifact.rules.len());
        assert_eq!(roundtrip.rules[0].id, artifact.rules[0].id);
    }

    #[test]
    fn outcome_fields_are_camel_case_in_json() {
        let outcome = RuleOutcome {
            valence: "progress".into(),
            magnitude: "meaningful".into(),
            evidence_type: "first-pass-green".into(),
        };
        let json = serde_json::to_value(&outcome).unwrap();
        assert!(
            json.get("evidenceType").is_some(),
            "evidenceType must be camelCase"
        );
        assert!(
            json.get("evidence_type").is_none(),
            "snake_case must not leak"
        );
    }
}
