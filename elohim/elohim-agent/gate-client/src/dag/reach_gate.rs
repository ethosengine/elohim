//! Reach gate v1 — embedded loader and stable reference constants.
//!
//! This module bundles the `reach-gate-v1` gate-process YAML at compile time
//! and exposes it as a [`GateProcessDeclaration`] via
//! [`default_reach_gate_declaration`].
//!
//! # CID reference
//!
//! The gate declaration is addressed at runtime by [`REACH_GATE_V1_CID`].
//! Phase 5 wires this gate into `check()` for `CapabilityInvoke { ReachNegotiation }`
//! events (Task 5.3).  When a DHT-backed resolver is available the same CID
//! will resolve via EPR lookup.
//!
//! # DAG topology
//!
//! ```text
//! assemble (context-assemble)
//!     └─ next ──► aggregate (aggregate-attestations)
//!                     ├─ reachData.edge_case == true  ──► wisdom-edge (wisdom-invoke)
//!                     └─ reachData.edge_case == false ──► synthesize (synthesize, Terminate)
//! wisdom-edge (wisdom-invoke) [Phase 5 DevContext: mocked Allow]
//!     └─ next ──► synthesize (synthesize, Terminate)
//! ```
//!
//! # Reach aggregation spec CID
//!
//! The `aggregate` step references [`REACH_AGGREGATION_V1_CID`] via its
//! `aggregation_spec_cid` field.  Phase 5 resolves this via
//! `EmbeddedContentNodeResolver` pre-loaded with the CID → body pair from
//! `reach_aggregation.rs`.
//!
//! # synthesize builder
//!
//! The `synthesize` step uses `"verdict-from-reach"` which reads
//! `reachData.level` from GateContext and emits
//! `GateStatus::Verdict(GateTag::ReachLevel { level })`.  The `self-reach`
//! default fires when `reachData.level` is null (no tier met).

use super::GateProcessDeclaration;

// Re-export so callers can refer to the aggregation CID without importing the
// sub-module directly.
pub use crate::dag::reach_aggregation::REACH_AGGREGATION_V1_CID as AGGREGATION_SPEC_CID;

// ─── Stable CID reference ─────────────────────────────────────────────────────

/// Stable EPR-style CID used to address the reach-gate-v1 gate-process declaration.
///
/// Phase 5 resolves this via `EmbeddedContentNodeResolver`.  Phase 6+ will
/// resolve it via DHT EPR lookup once the gate ContentNode is seeded.
pub const REACH_GATE_V1_CID: &str = "epr:gates:reach-gate-v1";

// ─── Embedded YAML ────────────────────────────────────────────────────────────

/// The raw YAML source of the reach-gate v1 declaration.
///
/// Embedded at compile time via `include_str!`.  The file lives alongside this
/// module at `src/dag/reach_gate_v1.yaml`.
pub fn default_reach_gate_yaml() -> &'static str {
    include_str!("reach_gate_v1.yaml")
}

/// Parse the embedded reach-gate v1 YAML into a [`GateProcessDeclaration`].
///
/// # Panics
///
/// Panics if the embedded YAML fails to parse — this would be a compile-time
/// authoring error caught by the unit tests below.
pub fn default_reach_gate_declaration() -> GateProcessDeclaration {
    serde_yaml::from_str(default_reach_gate_yaml()).expect(
        "reach_gate_v1.yaml failed to parse into GateProcessDeclaration — \
         this is a compile-time authoring error; fix the YAML",
    )
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::{GateProcessDeclaration, StepType};

    /// Helper: parse the embedded YAML and return the declaration (or panic with
    /// a clear message on parse failure).
    fn parsed() -> GateProcessDeclaration {
        let yaml = default_reach_gate_yaml();
        serde_yaml::from_str(yaml)
            .unwrap_or_else(|e| panic!("reach_gate_v1.yaml failed to parse: {e}\n\nYAML:\n{yaml}"))
    }

    // ── Parse correctness ──────────────────────────────────────────────────────

    #[test]
    fn yaml_parses_without_error() {
        let _ = parsed();
    }

    #[test]
    fn name_is_reach_gate_v1() {
        let d = parsed();
        assert_eq!(d.name, "reach-gate-v1");
    }

    #[test]
    fn version_is_1_0_0() {
        let d = parsed();
        assert_eq!(d.version, "1.0.0");
    }

    #[test]
    fn event_type_is_capability_invoke() {
        let d = parsed();
        assert_eq!(d.event_type, "capability-invoke");
    }

    #[test]
    fn entrypoint_is_assemble() {
        let d = parsed();
        assert_eq!(d.dag.entrypoint, "assemble");
    }

    // ── Step presence ──────────────────────────────────────────────────────────

    #[test]
    fn contains_all_four_expected_steps() {
        let d = parsed();
        let step_ids: std::collections::HashSet<&str> =
            d.dag.steps.keys().map(String::as_str).collect();
        let expected = ["assemble", "aggregate", "wisdom-edge", "synthesize"];
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
    fn aggregate_is_aggregate_attestations() {
        let d = parsed();
        let step = &d.dag.steps["aggregate"].step;
        assert!(
            matches!(step, StepType::AggregateAttestations { .. }),
            "aggregate must be AggregateAttestations, got: {step:?}"
        );
    }

    #[test]
    fn wisdom_edge_is_wisdom_invoke() {
        let d = parsed();
        let step = &d.dag.steps["wisdom-edge"].step;
        assert!(
            matches!(step, StepType::WisdomInvoke { .. }),
            "wisdom-edge must be WisdomInvoke, got: {step:?}"
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

    // ── aggregate step references REACH_AGGREGATION_V1_CID ───────────────────

    #[test]
    fn aggregate_step_references_reach_aggregation_v1_cid() {
        let d = parsed();
        let step = &d.dag.steps["aggregate"].step;
        if let StepType::AggregateAttestations { params } = step {
            assert_eq!(
                params.aggregation_spec_cid, AGGREGATION_SPEC_CID,
                "aggregate step aggregation_spec_cid must equal REACH_AGGREGATION_V1_CID \
                 (via AGGREGATION_SPEC_CID re-export)"
            );
        } else {
            panic!("aggregate is not AggregateAttestations");
        }
    }

    // ── Structural correctness ─────────────────────────────────────────────────

    #[test]
    fn assemble_next_is_aggregate() {
        let d = parsed();
        let next = d.dag.steps["assemble"].next.as_deref();
        assert_eq!(next, Some("aggregate"), "assemble.next must be aggregate");
    }

    #[test]
    fn aggregate_has_two_conditional_edges() {
        let d = parsed();
        let edges = &d.dag.steps["aggregate"].edges;
        assert_eq!(
            edges.len(),
            2,
            "aggregate must have 2 conditional edges (edge_case true/false)"
        );
    }

    #[test]
    fn aggregate_edge_case_true_targets_wisdom_edge() {
        let d = parsed();
        let edges = &d.dag.steps["aggregate"].edges;
        let edge = edges.iter().find(|e| e.when.contains("true"));
        assert!(
            edge.is_some(),
            "no edge_case==true edge found on aggregate step"
        );
        assert_eq!(
            edge.unwrap().target,
            "wisdom-edge",
            "edge_case==true must target wisdom-edge"
        );
    }

    #[test]
    fn aggregate_edge_case_false_targets_synthesize() {
        let d = parsed();
        let edges = &d.dag.steps["aggregate"].edges;
        let edge = edges.iter().find(|e| e.when.contains("false"));
        assert!(
            edge.is_some(),
            "no edge_case==false edge found on aggregate step"
        );
        assert_eq!(
            edge.unwrap().target,
            "synthesize",
            "edge_case==false must target synthesize"
        );
    }

    #[test]
    fn wisdom_edge_next_is_synthesize() {
        let d = parsed();
        let next = d.dag.steps["wisdom-edge"].next.as_deref();
        assert_eq!(
            next,
            Some("synthesize"),
            "wisdom-edge.next must be synthesize"
        );
    }

    // ── synthesize uses verdict-from-reach builder ────────────────────────────

    #[test]
    fn synthesize_uses_verdict_from_reach_builder() {
        let d = parsed();
        let step = &d.dag.steps["synthesize"].step;
        if let StepType::Synthesize { params } = step {
            assert_eq!(
                params.decision_builder, "verdict-from-reach",
                "synthesize must use verdict-from-reach builder"
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
                "reach-gate synthesize must have no side effects (read-only verdict)"
            );
        } else {
            panic!("synthesize is not Synthesize");
        }
    }

    // ── terminals map ──────────────────────────────────────────────────────────

    #[test]
    fn terminals_map_is_empty() {
        let d = parsed();
        assert!(
            d.dag.terminals.is_empty(),
            "reach-gate has no static-decision terminals; all outcomes are produced \
             by synthesize; got: {:?}",
            d.dag.terminals.keys().collect::<Vec<_>>()
        );
    }

    // ── CID constant ──────────────────────────────────────────────────────────

    #[test]
    fn cid_constant_has_expected_value() {
        assert_eq!(REACH_GATE_V1_CID, "epr:gates:reach-gate-v1");
    }

    // ── Round-trip ────────────────────────────────────────────────────────────

    #[test]
    fn default_reach_gate_declaration_round_trips() {
        let from_fn = default_reach_gate_declaration();
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
