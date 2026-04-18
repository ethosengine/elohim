//! Discernment gate v1 (mechanical) — embedded loader and stable reference constants.
//!
//! This module bundles the `discernment-gate-v1-mechanical` gate-process YAML at
//! compile time and exposes it as a [`GateProcessDeclaration`] via
//! [`default_discernment_gate_declaration`].
//!
//! # CID reference
//!
//! The gate declaration is addressed at runtime by [`DISCERNMENT_GATE_V1_CID`].
//! Phase 3 wires this gate into `check()` for `AttestationWrite` events scoped
//! to `experience-moment` content (Task 3.4).  When a DHT-backed resolver is
//! available the same CID will resolve via EPR lookup.
//!
//! # DAG topology
//!
//! ```text
//! assemble (context-assemble)
//!     └─ next ──► rules (mechanical-ruleset)
//!                     ├─ ruleDecision == null  ──► terminal: terminal-no-mint
//!                     └─ ruleDecision != null  ──► synthesize (synthesize, Terminate)
//! ```
//!
//! # Seven-valence rules CID
//!
//! The `rules` step references [`SEVEN_VALENCE_RULES_V1_CID`] via its
//! `rules_cid` field.  Phase 3 resolves this via `EmbeddedContentNodeResolver`
//! pre-loaded with the CID → body pair from `seven_valence_rules.rs`.

use super::GateProcessDeclaration;

// Re-export so callers can refer to it without importing the sub-module.
pub use crate::dag::seven_valence_rules::SEVEN_VALENCE_RULES_V1_CID as RULES_CID;

// ─── Stable CID reference ─────────────────────────────────────────────────────

/// Stable EPR-style CID used to address the discernment-gate-v1-mechanical
/// gate-process declaration.
///
/// Phase 3 resolves this via `EmbeddedContentNodeResolver`.  Phase 5+ will
/// resolve it via DHT EPR lookup once the gate ContentNode is seeded.
pub const DISCERNMENT_GATE_V1_CID: &str = "epr:gates:discernment-gate-v1-mechanical";

// ─── Embedded YAML ────────────────────────────────────────────────────────────

/// The raw YAML source of the discernment-gate v1 declaration.
///
/// Embedded at compile time via `include_str!`.  The file lives alongside this
/// module at `src/dag/discernment_gate_v1.yaml`.
pub fn default_discernment_gate_yaml() -> &'static str {
    include_str!("discernment_gate_v1.yaml")
}

/// Parse the embedded discernment-gate v1 YAML into a [`GateProcessDeclaration`].
///
/// # Panics
///
/// Panics if the embedded YAML fails to parse — this would be a compile-time
/// authoring error caught by the unit tests below.
pub fn default_discernment_gate_declaration() -> GateProcessDeclaration {
    serde_yaml::from_str(default_discernment_gate_yaml()).expect(
        "discernment_gate_v1.yaml failed to parse into GateProcessDeclaration — \
         this is a compile-time authoring error; fix the YAML",
    )
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::{GateProcessDeclaration, StepType};

    /// Helper: parse the embedded YAML and return the declaration (or panic with
    /// a clear message on parse failure, to distinguish authoring errors from
    /// test logic errors).
    fn parsed() -> GateProcessDeclaration {
        let yaml = default_discernment_gate_yaml();
        serde_yaml::from_str(yaml).unwrap_or_else(|e| {
            panic!("discernment_gate_v1.yaml failed to parse: {e}\n\nYAML:\n{yaml}")
        })
    }

    // ── Parse correctness ──────────────────────────────────────────────────────

    #[test]
    fn yaml_parses_without_error() {
        let _ = parsed();
    }

    #[test]
    fn name_is_discernment_gate_v1_mechanical() {
        let d = parsed();
        assert_eq!(d.name, "discernment-gate-v1-mechanical");
    }

    #[test]
    fn version_is_1_0_0() {
        let d = parsed();
        assert_eq!(d.version, "1.0.0");
    }

    #[test]
    fn event_type_is_attestation_write() {
        let d = parsed();
        assert_eq!(d.event_type, "attestation-write");
    }

    #[test]
    fn entrypoint_is_assemble() {
        let d = parsed();
        assert_eq!(d.dag.entrypoint, "assemble");
    }

    // ── Step presence ──────────────────────────────────────────────────────────

    #[test]
    fn contains_all_expected_step_ids() {
        let d = parsed();
        let step_ids: std::collections::HashSet<&str> =
            d.dag.steps.keys().map(String::as_str).collect();
        let expected = ["assemble", "rules", "synthesize"];
        for id in &expected {
            assert!(
                step_ids.contains(id),
                "expected step '{id}' not found in DAG; present steps: {step_ids:?}"
            );
        }
        assert_eq!(
            step_ids.len(),
            expected.len(),
            "unexpected extra steps in DAG: {step_ids:?}"
        );
    }

    // ── Step type correctness ──────────────────────────────────────────────────

    #[test]
    fn assemble_is_context_assemble() {
        let d = parsed();
        let step = &d.dag.steps["assemble"].step;
        assert!(
            matches!(step, StepType::ContextAssemble { .. }),
            "assemble must be ContextAssemble, got: {step:?}"
        );
    }

    #[test]
    fn rules_is_mechanical_ruleset() {
        let d = parsed();
        let step = &d.dag.steps["rules"].step;
        assert!(
            matches!(step, StepType::MechanicalRuleset { .. }),
            "rules must be MechanicalRuleset, got: {step:?}"
        );
    }

    #[test]
    fn synthesize_is_synthesize() {
        let d = parsed();
        let step = &d.dag.steps["synthesize"].step;
        assert!(
            matches!(step, StepType::Synthesize { .. }),
            "synthesize must be Synthesize, got: {step:?}"
        );
    }

    // ── rules_cid references SEVEN_VALENCE_RULES_V1_CID ──────────────────────

    #[test]
    fn rules_step_references_seven_valence_rules_v1_cid() {
        let d = parsed();
        let step = &d.dag.steps["rules"].step;
        if let StepType::MechanicalRuleset { params } = step {
            assert_eq!(
                params.rules_cid,
                RULES_CID,
                "rules step rules_cid must equal SEVEN_VALENCE_RULES_V1_CID (via RULES_CID re-export)"
            );
        } else {
            panic!("rules is not MechanicalRuleset");
        }
    }

    // ── Structural correctness ─────────────────────────────────────────────────

    #[test]
    fn assemble_next_is_rules() {
        let d = parsed();
        let next = d.dag.steps["assemble"].next.as_deref();
        assert_eq!(next, Some("rules"), "assemble.next must be rules");
    }

    #[test]
    fn rules_has_two_conditional_edges() {
        let d = parsed();
        let edges = &d.dag.steps["rules"].edges;
        assert_eq!(
            edges.len(),
            2,
            "rules must have 2 conditional edges (null / not-null)"
        );
    }

    #[test]
    fn rules_null_edge_targets_terminal_no_mint() {
        let d = parsed();
        let edges = &d.dag.steps["rules"].edges;
        let null_edge = edges.iter().find(|e| e.when.contains("null"));
        assert!(null_edge.is_some(), "no null edge found on rules step");
        assert_eq!(
            null_edge.unwrap().target,
            "terminal-no-mint",
            "null edge must target terminal-no-mint"
        );
    }

    #[test]
    fn rules_non_null_edge_targets_synthesize() {
        let d = parsed();
        let edges = &d.dag.steps["rules"].edges;
        let non_null_edge = edges
            .iter()
            .find(|e| e.when.contains("!= null") || e.when.contains("!= null"));
        assert!(
            non_null_edge.is_some(),
            "no non-null edge found on rules step"
        );
        assert_eq!(
            non_null_edge.unwrap().target,
            "synthesize",
            "non-null edge must target synthesize"
        );
    }

    // ── synthesize side_effects ────────────────────────────────────────────────

    #[test]
    fn synthesize_uses_verdict_from_rule_builder() {
        let d = parsed();
        let step = &d.dag.steps["synthesize"].step;
        if let StepType::Synthesize { params } = step {
            assert_eq!(
                params.decision_builder, "verdict-from-rule",
                "synthesize must use verdict-from-rule builder"
            );
        } else {
            panic!("synthesize is not Synthesize");
        }
    }

    #[test]
    fn synthesize_has_two_side_effects() {
        let d = parsed();
        let step = &d.dag.steps["synthesize"].step;
        if let StepType::Synthesize { params } = step {
            assert_eq!(
                params.side_effects.len(),
                2,
                "synthesize must have 2 side effects (MintAttestation + EmitEconomicEvent), \
                 got: {:?}",
                params
                    .side_effects
                    .iter()
                    .map(|s| &s.effect_type)
                    .collect::<Vec<_>>()
            );
        } else {
            panic!("synthesize is not Synthesize");
        }
    }

    #[test]
    fn synthesize_first_side_effect_is_mint_attestation() {
        let d = parsed();
        let step = &d.dag.steps["synthesize"].step;
        if let StepType::Synthesize { params } = step {
            let first = &params.side_effects[0];
            assert_eq!(
                first.effect_type, "MintAttestation",
                "first side effect must be MintAttestation"
            );
            assert!(
                first
                    .params_from_keys
                    .contains(&"momentEntryHash".to_string()),
                "MintAttestation must reference momentEntryHash key"
            );
            assert!(
                first.params_from_keys.contains(&"ruleDecision".to_string()),
                "MintAttestation must reference ruleDecision key"
            );
        } else {
            panic!("synthesize is not Synthesize");
        }
    }

    #[test]
    fn synthesize_second_side_effect_is_emit_economic_event() {
        let d = parsed();
        let step = &d.dag.steps["synthesize"].step;
        if let StepType::Synthesize { params } = step {
            let second = &params.side_effects[1];
            assert_eq!(
                second.effect_type, "EmitEconomicEvent",
                "second side effect must be EmitEconomicEvent"
            );
            assert!(
                second
                    .params_from_keys
                    .contains(&"ruleDecision".to_string()),
                "EmitEconomicEvent must reference ruleDecision key"
            );
        } else {
            panic!("synthesize is not Synthesize");
        }
    }

    // ── terminals map ──────────────────────────────────────────────────────────

    #[test]
    fn terminal_no_mint_is_present() {
        let d = parsed();
        assert!(
            d.dag.terminals.contains_key("terminal-no-mint"),
            "terminals map must contain 'terminal-no-mint'; keys: {:?}",
            d.dag.terminals.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn terminal_no_mint_has_exempt_allow_decision() {
        let d = parsed();
        let terminal = &d.dag.terminals["terminal-no-mint"];
        let status = terminal.decision.get("status").and_then(|v| v.as_str());
        assert_eq!(
            status,
            Some("allow"),
            "terminal-no-mint decision.status must be 'allow'"
        );
        let exempt = terminal.decision.get("exempt").and_then(|v| v.as_bool());
        assert_eq!(
            exempt,
            Some(true),
            "terminal-no-mint decision.exempt must be true"
        );
        let rationale = terminal.decision.get("rationale").and_then(|v| v.as_str());
        assert_eq!(
            rationale,
            Some("steady-state-silence"),
            "terminal-no-mint decision.rationale must be 'steady-state-silence'"
        );
    }

    #[test]
    fn terminal_no_mint_has_empty_side_effects() {
        let d = parsed();
        let terminal = &d.dag.terminals["terminal-no-mint"];
        assert!(
            terminal.side_effects.is_empty(),
            "terminal-no-mint must have no side effects; got {} side effect(s)",
            terminal.side_effects.len()
        );
    }

    #[test]
    fn only_terminal_is_terminal_no_mint() {
        let d = parsed();
        assert_eq!(
            d.dag.terminals.len(),
            1,
            "DAG must have exactly 1 terminal (terminal-no-mint); got: {:?}",
            d.dag.terminals.keys().collect::<Vec<_>>()
        );
    }

    // ── CID constant ──────────────────────────────────────────────────────────

    #[test]
    fn cid_constant_has_expected_value() {
        assert_eq!(
            DISCERNMENT_GATE_V1_CID,
            "epr:gates:discernment-gate-v1-mechanical"
        );
    }

    // ── Round-trip ────────────────────────────────────────────────────────────

    #[test]
    fn default_discernment_gate_declaration_round_trips() {
        let from_fn = default_discernment_gate_declaration();
        let from_yaml = parsed();

        let val_fn: serde_json::Value = serde_json::to_value(&from_fn).unwrap();
        let val_yaml: serde_json::Value = serde_json::to_value(&from_yaml).unwrap();

        assert_eq!(val_fn["name"], val_yaml["name"], "name mismatch");
        assert_eq!(val_fn["version"], val_yaml["version"], "version mismatch");
        assert_eq!(
            val_fn["event_type"], val_yaml["event_type"],
            "event_type mismatch"
        );
        assert_eq!(
            val_fn["dag"]["entrypoint"], val_yaml["dag"]["entrypoint"],
            "dag.entrypoint mismatch"
        );

        let keys_fn: std::collections::BTreeSet<String> = val_fn["dag"]["steps"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        let keys_yaml: std::collections::BTreeSet<String> = val_yaml["dag"]["steps"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        assert_eq!(keys_fn, keys_yaml, "dag step id sets must match");
    }
}
