//! Universal-band v1 — protocol-root gate DAG embedded as a default.
//!
//! The universal band runs before every gate invocation.  It is a governable
//! artifact: the active version is named by the constitutional pointer
//! (`ACTIVE_UNIVERSAL_BAND_NAME` / `ACTIVE_UNIVERSAL_BAND_VERSION` below).
//!
//! Phase 2: the pointer is a compile-time constant.  Phase 3+ will replace
//! the constant with a DHT lookup so the community can ratify a new version
//! without a binary release.
//!
//! # DAG topology (v1)
//!
//! ```text
//! authorize (context-assemble)
//!     └─ next ──► assemble-context (context-assemble)
//!                     └─ next ──► wisdom-primary (wisdom-invoke)
//!                                     ├─ allow     ──► record-decision  (synthesize, Terminate)
//!                                     ├─ decline   ──► record-decline   (synthesize, Terminate)
//!                                     ├─ escalate  ──► escalate-to-review (EscalateToReview, Terminate)
//!                                     └─ need-deeper ► record-decision  (Phase 3+ TODO: wisdom-deeper)
//! ```

use super::GateProcessDeclaration;

// ─── Constitutional pointer ───────────────────────────────────────────────────
//
// External consumers should call `active_universal_band_pointer()` rather than
// reading these constants directly.  The function is the stable API; the
// constants are an internal implementation detail that Phase 3+ will supersede
// with a DHT-backed lookup.

/// The name of the currently active universal band declaration.
///
/// Internal use and tests only.  External callers: use
/// [`active_universal_band_pointer()`] instead.
pub const ACTIVE_UNIVERSAL_BAND_NAME: &str = "universal-band-v1";

/// The version of the currently active universal band declaration.
///
/// Internal use and tests only.  External callers: use
/// [`active_universal_band_pointer()`] instead.
pub const ACTIVE_UNIVERSAL_BAND_VERSION: &str = "1.0.0";

// ─── Typed pointer ────────────────────────────────────────────────────────────

/// A reference to the active universal-band declaration at a moment in time.
///
/// Phase 2: always resolves to the embedded default (compile-time).
/// Phase 3+: will resolve via DHT lookup against a well-known constitutional
/// pointer, with graceful fallback to the embedded default on lookup failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniversalBandPointer {
    pub name: String,
    pub version: String,
    // Phase 3+: add resolution_source: PointerSource enum
}

/// Return the currently-active universal-band pointer.
///
/// Phase 2: returns the embedded default. Phase 3+: will query DHT, falling
/// back to the embedded default on any resolution failure.
pub fn active_universal_band_pointer() -> UniversalBandPointer {
    UniversalBandPointer {
        name: ACTIVE_UNIVERSAL_BAND_NAME.to_string(),
        version: ACTIVE_UNIVERSAL_BAND_VERSION.to_string(),
    }
}

// ─── Embedded YAML ───────────────────────────────────────────────────────────

/// The raw YAML source of the universal-band v1 declaration.
///
/// Embedded at compile time via `include_str!`.  The file lives alongside this
/// module at `src/dag/universal_band_v1.yaml`.
pub fn default_universal_band_yaml() -> &'static str {
    include_str!("universal_band_v1.yaml")
}

/// Parse the embedded universal-band v1 YAML into a [`GateProcessDeclaration`].
///
/// Panics if the embedded YAML fails to parse — this would be a compile-time
/// authoring error caught by the unit tests below.
pub fn default_universal_band_declaration() -> GateProcessDeclaration {
    serde_yaml::from_str(default_universal_band_yaml()).expect(
        "universal_band_v1.yaml failed to parse into GateProcessDeclaration — \
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
        let yaml = default_universal_band_yaml();
        serde_yaml::from_str(yaml).unwrap_or_else(|e| {
            panic!("universal_band_v1.yaml failed to parse: {e}\n\nYAML:\n{yaml}")
        })
    }

    // ── Parse correctness ──────────────────────────────────────────────────────

    #[test]
    fn yaml_parses_without_error() {
        // The mere act of calling parsed() without panic is the assertion.
        let _ = parsed();
    }

    #[test]
    fn name_is_universal_band_v1() {
        let d = parsed();
        assert_eq!(d.name, "universal-band-v1");
    }

    #[test]
    fn version_is_1_0_0() {
        let d = parsed();
        assert_eq!(d.version, "1.0.0");
    }

    #[test]
    fn event_type_is_universal_sentinel() {
        let d = parsed();
        assert_eq!(d.event_type, "*");
    }

    #[test]
    fn entrypoint_is_authorize() {
        let d = parsed();
        assert_eq!(d.dag.entrypoint, "authorize");
    }

    // ── Step presence ──────────────────────────────────────────────────────────

    #[test]
    fn contains_all_expected_step_ids() {
        let d = parsed();
        let step_ids: std::collections::HashSet<&str> =
            d.dag.steps.keys().map(String::as_str).collect();
        let expected = [
            "authorize",
            "assemble-context",
            "wisdom-primary",
            "record-decision",
            "record-decline",
            "escalate-to-review",
        ];
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
    fn authorize_is_context_assemble() {
        let d = parsed();
        let step = &d.dag.steps["authorize"].step;
        assert!(
            matches!(step, StepType::ContextAssemble { .. }),
            "authorize must be ContextAssemble, got: {step:?}"
        );
    }

    #[test]
    fn assemble_context_is_context_assemble() {
        let d = parsed();
        let step = &d.dag.steps["assemble-context"].step;
        assert!(
            matches!(step, StepType::ContextAssemble { .. }),
            "assemble-context must be ContextAssemble, got: {step:?}"
        );
    }

    #[test]
    fn wisdom_primary_is_wisdom_invoke() {
        let d = parsed();
        let step = &d.dag.steps["wisdom-primary"].step;
        assert!(
            matches!(step, StepType::WisdomInvoke { .. }),
            "wisdom-primary must be WisdomInvoke, got: {step:?}"
        );
    }

    #[test]
    fn record_decision_is_synthesize() {
        let d = parsed();
        let step = &d.dag.steps["record-decision"].step;
        assert!(
            matches!(step, StepType::Synthesize { .. }),
            "record-decision must be Synthesize, got: {step:?}"
        );
    }

    #[test]
    fn record_decline_is_synthesize() {
        let d = parsed();
        let step = &d.dag.steps["record-decline"].step;
        assert!(
            matches!(step, StepType::Synthesize { .. }),
            "record-decline must be Synthesize, got: {step:?}"
        );
    }

    #[test]
    fn escalate_to_review_is_escalate_to_review() {
        let d = parsed();
        let step = &d.dag.steps["escalate-to-review"].step;
        assert!(
            matches!(step, StepType::EscalateToReview { .. }),
            "escalate-to-review must be EscalateToReview, got: {step:?}"
        );
    }

    // ── Structural correctness ─────────────────────────────────────────────────

    #[test]
    fn authorize_next_is_assemble_context() {
        let d = parsed();
        let next = d.dag.steps["authorize"].next.as_deref();
        assert_eq!(
            next,
            Some("assemble-context"),
            "authorize.next must be assemble-context"
        );
    }

    #[test]
    fn assemble_context_next_is_wisdom_primary() {
        let d = parsed();
        let next = d.dag.steps["assemble-context"].next.as_deref();
        assert_eq!(
            next,
            Some("wisdom-primary"),
            "assemble-context.next must be wisdom-primary"
        );
    }

    #[test]
    fn wisdom_primary_has_four_conditional_edges() {
        let d = parsed();
        let edges = &d.dag.steps["wisdom-primary"].edges;
        assert_eq!(
            edges.len(),
            4,
            "wisdom-primary must have 4 conditional edges (allow/decline/escalate/need-deeper)"
        );
    }

    #[test]
    fn wisdom_primary_allow_edge_targets_record_decision() {
        let d = parsed();
        let edges = &d.dag.steps["wisdom-primary"].edges;
        let allow_edge = edges.iter().find(|e| e.when.contains("'allow'"));
        assert!(allow_edge.is_some(), "no allow edge found");
        assert_eq!(
            allow_edge.unwrap().target,
            "record-decision",
            "allow edge must target record-decision"
        );
    }

    #[test]
    fn wisdom_primary_decline_edge_targets_record_decline() {
        let d = parsed();
        let edges = &d.dag.steps["wisdom-primary"].edges;
        let decline_edge = edges.iter().find(|e| e.when.contains("'decline'"));
        assert!(decline_edge.is_some(), "no decline edge found");
        assert_eq!(
            decline_edge.unwrap().target,
            "record-decline",
            "decline edge must target record-decline"
        );
    }

    #[test]
    fn wisdom_primary_escalate_edge_targets_escalate_to_review() {
        let d = parsed();
        let edges = &d.dag.steps["wisdom-primary"].edges;
        let esc_edge = edges.iter().find(|e| e.when.contains("'escalate'"));
        assert!(esc_edge.is_some(), "no escalate edge found");
        assert_eq!(
            esc_edge.unwrap().target,
            "escalate-to-review",
            "escalate edge must target escalate-to-review"
        );
    }

    #[test]
    fn active_universal_band_pointer_returns_expected_name_and_version() {
        let ptr = active_universal_band_pointer();
        assert_eq!(ptr.name, "universal-band-v1");
        assert_eq!(ptr.version, "1.0.0");
    }

    #[test]
    fn wisdom_primary_need_deeper_edge_targets_record_decision() {
        let decl = default_universal_band_declaration();
        let wisdom_primary = decl.dag.steps.get("wisdom-primary").unwrap();
        let need_deeper_edge = wisdom_primary
            .edges
            .iter()
            .find(|e| e.when.contains("need-deeper"))
            .expect("wisdom-primary must have a need-deeper edge");
        assert_eq!(need_deeper_edge.target, "record-decision");
    }

    #[test]
    fn terminals_map_is_empty() {
        // The universal band uses executor-produced Terminate, not terminal nodes.
        // If terminals were non-empty, the interpreter would bypass the executors.
        let d = parsed();
        assert!(
            d.dag.terminals.is_empty(),
            "universal-band terminals map must be empty; executors produce Terminate directly. \
             Found: {:?}",
            d.dag.terminals.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn record_decision_uses_allow_with_wisdom_builder() {
        let d = parsed();
        let step = &d.dag.steps["record-decision"].step;
        if let StepType::Synthesize { params } = step {
            assert_eq!(
                params.decision_builder, "allow-with-wisdom",
                "record-decision must use allow-with-wisdom builder"
            );
        } else {
            panic!("record-decision is not Synthesize");
        }
    }

    #[test]
    fn record_decline_uses_decline_from_wisdom_builder() {
        let d = parsed();
        let step = &d.dag.steps["record-decline"].step;
        if let StepType::Synthesize { params } = step {
            assert_eq!(
                params.decision_builder, "decline-from-wisdom",
                "record-decline must use decline-from-wisdom builder"
            );
        } else {
            panic!("record-decline is not Synthesize");
        }
    }

    #[test]
    fn wisdom_primary_output_key_is_wisdom_output() {
        let d = parsed();
        let step = &d.dag.steps["wisdom-primary"].step;
        if let StepType::WisdomInvoke { params } = step {
            assert_eq!(
                params.output_key, "wisdom_output",
                "wisdom-primary output_key must be wisdom_output"
            );
        } else {
            panic!("wisdom-primary is not WisdomInvoke");
        }
    }

    #[test]
    fn constitutional_pointer_matches_yaml_name_and_version() {
        let d = parsed();
        assert_eq!(
            d.name, ACTIVE_UNIVERSAL_BAND_NAME,
            "ACTIVE_UNIVERSAL_BAND_NAME must match the embedded YAML name"
        );
        assert_eq!(
            d.version, ACTIVE_UNIVERSAL_BAND_VERSION,
            "ACTIVE_UNIVERSAL_BAND_VERSION must match the embedded YAML version"
        );
    }

    #[test]
    fn default_universal_band_declaration_round_trips() {
        // default_universal_band_declaration() must not panic and must return
        // a structurally equivalent declaration to the direct serde_yaml parse.
        // We compare via sorted JSON values (not raw strings) to be insensitive
        // to HashMap key ordering, which is non-deterministic.
        let from_fn = default_universal_band_declaration();
        let from_yaml = parsed();

        // Serialize both to serde_json::Value (sorted object keys) for comparison.
        let val_fn: serde_json::Value = serde_json::to_value(&from_fn).unwrap();
        let val_yaml: serde_json::Value = serde_json::to_value(&from_yaml).unwrap();

        // Compare top-level scalar fields directly to give clear failure messages.
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

        // Compare the step id sets (order-independent).
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
