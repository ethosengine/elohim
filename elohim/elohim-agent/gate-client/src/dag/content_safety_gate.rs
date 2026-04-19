//! Content safety gate v1 — embedded loader and stable reference constants.
//!
//! This module bundles the `content-safety-gate-v1` gate-process YAML at compile
//! time and exposes it as a [`GateProcessDeclaration`] via
//! [`default_content_safety_gate_declaration`].
//!
//! # CID reference
//!
//! The gate declaration is addressed at runtime by [`CONTENT_SAFETY_GATE_V1_CID`].
//! Phase 6 wires this gate into `check()` for `ContentPublish`, `AttestationWrite`,
//! and `PeerMessage` events (Task 6.2).  When a DHT-backed resolver is available
//! the same CID will resolve via EPR lookup.
//!
//! # DAG topology
//!
//! ```text
//! assemble (context-assemble)
//!     └─ next ──► wisdom-safety (wisdom-invoke)
//!                     └─ next ──► synthesize (synthesize, Terminate)
//! ```
//!
//! # Phase 6 DevContext
//!
//! The `wisdom-safety` step is backed by [`WisdomInvokeExecutor`] which returns a
//! mocked `{"decision":"allow"}` output.  No content is blocked in DevContext.
//! Phase 7+ activation calls `ContentSafetyReview` capability via skill-invoke.
//!
//! # Decision builder
//!
//! The `synthesize` step uses `"allow-or-decline-from-wisdom"` which:
//! - Reads `safetyOutput` from GateContext.
//! - Emits `GateStatus::Decline { grounds: DeclineGrounds { category: "content-safety" } }`
//!   when `safetyOutput.decision == "decline"`.
//! - Emits `GateStatus::Allow { exempt: false }` otherwise (DevContext always Allow).

use super::GateProcessDeclaration;

// ─── Stable CID reference ─────────────────────────────────────────────────────

/// Stable EPR-style CID used to address the content-safety-gate-v1 gate-process
/// declaration.
///
/// Phase 6 resolves this via `EmbeddedContentNodeResolver`.  Phase 7+ will
/// resolve it via DHT EPR lookup once the gate ContentNode is seeded.
pub const CONTENT_SAFETY_GATE_V1_CID: &str = "epr:gates:content-safety-gate-v1";

// ─── Embedded YAML ────────────────────────────────────────────────────────────

/// The raw YAML source of the content-safety-gate v1 declaration.
///
/// Embedded at compile time via `include_str!`.  The file lives alongside this
/// module at `src/dag/content_safety_gate_v1.yaml`.
pub fn default_content_safety_gate_yaml() -> &'static str {
    include_str!("content_safety_gate_v1.yaml")
}

/// Parse the embedded content-safety-gate v1 YAML into a [`GateProcessDeclaration`].
///
/// # Panics
///
/// Panics if the embedded YAML fails to parse — this would be a compile-time
/// authoring error caught by the unit tests below.
pub fn default_content_safety_gate_declaration() -> GateProcessDeclaration {
    serde_yaml::from_str(default_content_safety_gate_yaml()).expect(
        "content_safety_gate_v1.yaml failed to parse into GateProcessDeclaration — \
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
        let yaml = default_content_safety_gate_yaml();
        serde_yaml::from_str(yaml).unwrap_or_else(|e| {
            panic!("content_safety_gate_v1.yaml failed to parse: {e}\n\nYAML:\n{yaml}")
        })
    }

    // ── Parse correctness ──────────────────────────────────────────────────────

    #[test]
    fn yaml_parses_without_error() {
        let _ = parsed();
    }

    #[test]
    fn name_is_content_safety_gate_v1() {
        let d = parsed();
        assert_eq!(d.name, "content-safety-gate-v1");
    }

    #[test]
    fn version_is_1_0_0() {
        let d = parsed();
        assert_eq!(d.version, "1.0.0");
    }

    #[test]
    fn event_type_is_wildcard() {
        let d = parsed();
        assert_eq!(
            d.event_type, "*",
            "content-safety-gate fires on all events (wide gate); event_type must be '*'"
        );
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
        let expected = ["assemble", "wisdom-safety", "synthesize"];
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
    fn wisdom_safety_is_wisdom_invoke() {
        let d = parsed();
        let step = &d.dag.steps["wisdom-safety"].step;
        assert!(
            matches!(step, StepType::WisdomInvoke { .. }),
            "wisdom-safety must be WisdomInvoke, got: {step:?}"
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

    // ── wisdom-safety constitution / framing CIDs ─────────────────────────────

    #[test]
    fn wisdom_safety_references_content_safety_constitution() {
        let d = parsed();
        let step = &d.dag.steps["wisdom-safety"].step;
        if let StepType::WisdomInvoke { params } = step {
            assert_eq!(
                params.constitution_cid, "epr:constitutions:content-safety-v1",
                "wisdom-safety must reference the content-safety constitution"
            );
            assert_eq!(
                params.framing_cid, "epr:framings:content-safety-v1",
                "wisdom-safety must reference the content-safety framing"
            );
        } else {
            panic!("wisdom-safety is not WisdomInvoke");
        }
    }

    #[test]
    fn wisdom_safety_context_keys_include_subject_and_body() {
        let d = parsed();
        let step = &d.dag.steps["wisdom-safety"].step;
        if let StepType::WisdomInvoke { params } = step {
            assert!(
                params.context_keys.contains(&"contentSubject".to_string()),
                "wisdom-safety context_keys must include contentSubject; got: {:?}",
                params.context_keys
            );
            assert!(
                params.context_keys.contains(&"contentBody".to_string()),
                "wisdom-safety context_keys must include contentBody; got: {:?}",
                params.context_keys
            );
        } else {
            panic!("wisdom-safety is not WisdomInvoke");
        }
    }

    #[test]
    fn wisdom_safety_output_key_is_safety_output() {
        let d = parsed();
        let step = &d.dag.steps["wisdom-safety"].step;
        if let StepType::WisdomInvoke { params } = step {
            assert_eq!(
                params.output_key, "safetyOutput",
                "wisdom-safety output_key must be 'safetyOutput'"
            );
        } else {
            panic!("wisdom-safety is not WisdomInvoke");
        }
    }

    // ── synthesize uses allow-or-decline-from-wisdom builder ─────────────────

    #[test]
    fn synthesize_uses_allow_or_decline_from_wisdom_builder() {
        let d = parsed();
        let step = &d.dag.steps["synthesize"].step;
        if let StepType::Synthesize { params } = step {
            assert_eq!(
                params.decision_builder, "allow-or-decline-from-wisdom",
                "synthesize must use allow-or-decline-from-wisdom builder"
            );
        } else {
            panic!("synthesize is not Synthesize");
        }
    }

    #[test]
    fn synthesize_input_keys_include_safety_output() {
        let d = parsed();
        let step = &d.dag.steps["synthesize"].step;
        if let StepType::Synthesize { params } = step {
            assert!(
                params.input_keys.contains(&"safetyOutput".to_string()),
                "synthesize input_keys must include 'safetyOutput'; got: {:?}",
                params.input_keys
            );
        } else {
            panic!("synthesize is not Synthesize");
        }
    }

    #[test]
    fn synthesize_has_no_side_effects() {
        let d = parsed();
        let step = &d.dag.steps["synthesize"].step;
        if let StepType::Synthesize { params } = step {
            assert!(
                params.side_effects.is_empty(),
                "content-safety-gate synthesize must have no side effects (read-only gate); \
                 got: {:?}",
                params.side_effects
            );
        } else {
            panic!("synthesize is not Synthesize");
        }
    }

    // ── Structural correctness ─────────────────────────────────────────────────

    #[test]
    fn assemble_next_is_wisdom_safety() {
        let d = parsed();
        let next = d.dag.steps["assemble"].next.as_deref();
        assert_eq!(
            next,
            Some("wisdom-safety"),
            "assemble.next must be 'wisdom-safety'"
        );
    }

    #[test]
    fn wisdom_safety_next_is_synthesize() {
        let d = parsed();
        let next = d.dag.steps["wisdom-safety"].next.as_deref();
        assert_eq!(
            next,
            Some("synthesize"),
            "wisdom-safety.next must be 'synthesize'"
        );
    }

    #[test]
    fn assemble_has_no_conditional_edges() {
        let d = parsed();
        let edges = &d.dag.steps["assemble"].edges;
        assert!(
            edges.is_empty(),
            "assemble must have no conditional edges (linear next link only); got: {edges:?}"
        );
    }

    #[test]
    fn wisdom_safety_has_no_conditional_edges() {
        let d = parsed();
        let edges = &d.dag.steps["wisdom-safety"].edges;
        assert!(
            edges.is_empty(),
            "wisdom-safety must have no conditional edges (linear next link only); got: {edges:?}"
        );
    }

    // ── terminals map ──────────────────────────────────────────────────────────

    #[test]
    fn terminals_map_is_empty() {
        let d = parsed();
        assert!(
            d.dag.terminals.is_empty(),
            "content-safety-gate has no static-decision terminals; all outcomes produced \
             by synthesize; got: {:?}",
            d.dag.terminals.keys().collect::<Vec<_>>()
        );
    }

    // ── CID constant ──────────────────────────────────────────────────────────

    #[test]
    fn cid_constant_has_expected_value() {
        assert_eq!(
            CONTENT_SAFETY_GATE_V1_CID,
            "epr:gates:content-safety-gate-v1"
        );
    }

    // ── Round-trip ────────────────────────────────────────────────────────────

    #[test]
    fn default_content_safety_gate_declaration_round_trips() {
        let from_fn = default_content_safety_gate_declaration();
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
