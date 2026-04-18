//! Seven-valence rules artifact — embedded loader and stable reference constants.
//!
//! This module bundles the `seven-valence-v1` rules JSON at compile time and
//! exposes it as a [`RulesArtifact`] via [`default_seven_valence_rules`].
//!
//! # CID reference
//!
//! The rules artifact is addressed at runtime by `SEVEN_VALENCE_RULES_V1_CID`.
//! Phase 3 uses an `EmbeddedContentNodeResolver` pre-loaded with this CID →
//! body pair (see Task 3.3 for the DAG that wires this). When a DHT-backed
//! resolver is available, the same CID will resolve via EPR lookup.
//!
//! # Context pre-computation contract
//!
//! Rules 5 and 6 consume two boolean fields that the **context-assemble step
//! (Task 3.4) must precompute** before this ruleset executes:
//!
//! - `priorAttestations.witnessEligible` — `true` when `moment.status` equals
//!   `priorAttestations.mostRecentStatus` AND
//!   `priorAttestations.computeFingerprintIsNew == true`.
//!
//! - `priorAttestations.refinementEligible` — `true` when `moment.status`
//!   equals `priorAttestations.mostRecentStatus` AND
//!   `priorAttestations.hasRicherEvidence == true` AND
//!   `priorAttestations.computeFingerprintIsNew == false`.
//!
//! These booleans are required because the rule condition DSL supports `==`
//! with a literal RHS only (no path-on-both-sides comparisons). The context-
//! assemble step injects them before the `mechanical-ruleset` step runs.
//!
//! # Why the steady-state rule 7 is not in this file
//!
//! Rule 7 ("else → do not mint") is implicit: when no rule matches the
//! executor writes `Value::Null` to the output key. The DAG edge
//! `ruleDecision == null` routes to `terminal-no-mint`. There is no explicit
//! rule needed for silence.

use crate::dag::rules_artifact::RulesArtifact;
use crate::error::GateError;

// ─── Stable CID reference ─────────────────────────────────────────────────────

/// Stable EPR-style CID used to address the seven-valence rules artifact.
///
/// Phase 3 resolves this via `EmbeddedContentNodeResolver`. Phase 5+ will
/// resolve it via DHT EPR lookup once the gate-rules ContentNode is seeded.
pub const SEVEN_VALENCE_RULES_V1_CID: &str = "epr:rules:seven-valence-v1";

/// The version string embedded in the artifact body.
pub const SEVEN_VALENCE_RULES_V1_VERSION: &str = "1.0.0";

// ─── Embedded JSON ────────────────────────────────────────────────────────────

/// The raw JSON body of the seven-valence rules artifact, embedded at compile
/// time from `src/dag/rules/seven_valence_v1.json`.
pub const SEVEN_VALENCE_RULES_V1_BODY: &str = include_str!("rules/seven_valence_v1.json");

// ─── Loader ───────────────────────────────────────────────────────────────────

/// Parse and return the embedded seven-valence rules as a [`RulesArtifact`].
///
/// # Errors
///
/// Returns `GateError::DagExecution` if the embedded JSON cannot be parsed.
/// In practice this should never fail because the JSON is validated by the
/// compile-time unit test below.
pub fn default_seven_valence_rules() -> Result<RulesArtifact, GateError> {
    let body: serde_json::Value =
        serde_json::from_str(SEVEN_VALENCE_RULES_V1_BODY).map_err(|e| {
            GateError::DagExecution(format!(
                "failed to parse embedded seven_valence_v1.json: {e}"
            ))
        })?;
    RulesArtifact::from_json(&body)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── T1: embedded JSON parses without error ───────────────────────────────

    #[test]
    fn embedded_json_parses_successfully() {
        let artifact = default_seven_valence_rules()
            .expect("embedded seven_valence_v1.json must parse without error");
        assert_eq!(artifact.rule_set_name, "seven-valence-v1");
        assert_eq!(artifact.version, "1.0.0");
    }

    // ─── T2: artifact has 6+ rules ────────────────────────────────────────────
    //
    // Rule 7 (steady-state) is implicit (no-match → null), so the artifact
    // contains exactly 6 explicit rules: 1, 2a, 2b, 3, 4, 5, 6.

    #[test]
    fn artifact_has_at_least_six_rules() {
        let artifact = default_seven_valence_rules().unwrap();
        assert!(
            artifact.rules.len() >= 6,
            "seven-valence rules must have at least 6 explicit rules, got {}",
            artifact.rules.len()
        );
    }

    // ─── T3: outputShape is StoryPointTag ────────────────────────────────────

    #[test]
    fn output_shape_is_story_point_tag() {
        let artifact = default_seven_valence_rules().unwrap();
        assert_eq!(
            artifact.output_shape.as_deref(),
            Some("StoryPointTag"),
            "outputShape must be 'StoryPointTag'"
        );
    }

    // ─── T4: appliesToEventType is attestation-write ─────────────────────────

    #[test]
    fn applies_to_event_type_is_attestation_write() {
        let artifact = default_seven_valence_rules().unwrap();
        assert_eq!(artifact.applies_to_event_type, "attestation-write");
    }

    // ─── T5: inputKeysExpected contains moment and priorAttestations ─────────

    #[test]
    fn input_keys_contain_moment_and_prior_attestations() {
        let artifact = default_seven_valence_rules().unwrap();
        assert!(
            artifact.input_keys_expected.contains(&"moment".to_string()),
            "inputKeysExpected must contain 'moment'"
        );
        assert!(
            artifact
                .input_keys_expected
                .contains(&"priorAttestations".to_string()),
            "inputKeysExpected must contain 'priorAttestations'"
        );
    }

    // ─── T6: rule IDs are unique ──────────────────────────────────────────────

    #[test]
    fn rule_ids_are_unique() {
        let artifact = default_seven_valence_rules().unwrap();
        let mut ids: Vec<&str> = artifact.rules.iter().map(|r| r.id.as_str()).collect();
        ids.sort_unstable();
        let original_len = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "all rule IDs must be unique");
    }

    // ─── T7: rule ordering — rule-1 first, rules 2a/2b before 3 ─────────────

    #[test]
    fn rule_ordering_1_before_2_before_3() {
        let artifact = default_seven_valence_rules().unwrap();
        let ids: Vec<&str> = artifact.rules.iter().map(|r| r.id.as_str()).collect();

        let pos_1 = ids.iter().position(|&id| id == "rule-1-first-pass-green");
        let pos_2a = ids
            .iter()
            .position(|&id| id == "rule-2a-novel-failure-class");
        let pos_2b = ids
            .iter()
            .position(|&id| id == "rule-2b-known-cause-recurrence");
        let pos_3 = ids
            .iter()
            .position(|&id| id == "rule-3-validates-failure-mode");

        assert!(pos_1.is_some(), "rule-1-first-pass-green must be present");
        assert!(
            pos_2a.is_some(),
            "rule-2a-novel-failure-class must be present"
        );
        assert!(
            pos_2b.is_some(),
            "rule-2b-known-cause-recurrence must be present"
        );
        assert!(
            pos_3.is_some(),
            "rule-3-validates-failure-mode must be present"
        );

        let pos_1 = pos_1.unwrap();
        let pos_2a = pos_2a.unwrap();
        let pos_2b = pos_2b.unwrap();
        let pos_3 = pos_3.unwrap();

        assert!(pos_1 < pos_2a, "rule-1 must come before rule-2a");
        assert!(
            pos_2a < pos_3,
            "rule-2a must come before rule-3 (deliberate ordering per spec §7.3)"
        );
        assert!(
            pos_2b < pos_3,
            "rule-2b must come before rule-3 (deliberate ordering per spec §7.3)"
        );
    }

    // ─── T8: all six explicit rules carry required valences ───────────────────

    #[test]
    fn all_expected_valences_are_present() {
        let artifact = default_seven_valence_rules().unwrap();
        let valences: Vec<&str> = artifact
            .rules
            .iter()
            .map(|r| r.outcome.valence.as_str())
            .collect();

        for expected in &[
            "progress",
            "discovery",
            "regression",
            "validation",
            "witness",
            "refinement",
        ] {
            assert!(
                valences.contains(expected),
                "valence '{expected}' must be present in the rules"
            );
        }
    }

    // ─── T9: all rules carry rationale ───────────────────────────────────────

    #[test]
    fn all_rules_have_rationale() {
        let artifact = default_seven_valence_rules().unwrap();
        for rule in &artifact.rules {
            assert!(
                rule.rationale.is_some() && !rule.rationale.as_deref().unwrap_or("").is_empty(),
                "rule '{}' must have a non-empty rationale",
                rule.id
            );
        }
    }

    // ─── T10: CID constant is stable, version matches embedded body ──────────

    #[test]
    fn cid_constant_format_and_version_match() {
        assert_eq!(SEVEN_VALENCE_RULES_V1_CID, "epr:rules:seven-valence-v1");
        let artifact = default_seven_valence_rules().unwrap();
        assert_eq!(artifact.version, SEVEN_VALENCE_RULES_V1_VERSION);
    }

    // ─── T11: rule conditions parse without error ─────────────────────────────

    #[test]
    fn all_rule_conditions_parse_cleanly() {
        use crate::dag::rule_expr::parse_rule_condition;

        let artifact = default_seven_valence_rules().unwrap();
        for rule in &artifact.rules {
            parse_rule_condition(&rule.when).unwrap_or_else(|e| {
                panic!(
                    "rule '{}' has an unparseable condition '{}': {e}",
                    rule.id, rule.when
                )
            });
        }
    }

    // ─── T12: rule-5 and rule-6 use precomputed booleans ─────────────────────

    #[test]
    fn witness_and_refinement_rules_use_precomputed_booleans() {
        let artifact = default_seven_valence_rules().unwrap();

        let witness_rule = artifact
            .rules
            .iter()
            .find(|r| r.id == "rule-5-witness")
            .expect("rule-5-witness must be present");
        assert!(
            witness_rule.when.contains("witnessEligible"),
            "rule-5 must reference priorAttestations.witnessEligible (Option A precomputed bool)"
        );

        let refinement_rule = artifact
            .rules
            .iter()
            .find(|r| r.id == "rule-6-refinement")
            .expect("rule-6-refinement must be present");
        assert!(
            refinement_rule.when.contains("refinementEligible"),
            "rule-6 must reference priorAttestations.refinementEligible (Option A precomputed bool)"
        );
    }

    // ─── T13: BODY constant is valid JSON ────────────────────────────────────

    #[test]
    fn body_constant_is_valid_json() {
        serde_json::from_str::<serde_json::Value>(SEVEN_VALENCE_RULES_V1_BODY)
            .expect("SEVEN_VALENCE_RULES_V1_BODY must be valid JSON");
    }
}
