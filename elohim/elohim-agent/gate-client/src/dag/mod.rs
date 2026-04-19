//! Gate-process DAG interpreter — reads a GateProcessDeclaration and executes
//! its step graph against an assembled GateContext.
//!
//! # Module structure
//!
//! - `mod.rs` (this file) — public API re-exports + Phase-0 type definitions.
//! - [`context`] — [`GateContext`]: the typed key-value accumulator.
//! - [`executor`] — [`StepExecutor`] trait, [`StepKind`], [`StepOutcome`].
//! - [`expr`] — minimal conditional-edge expression parser + evaluator.
//! - [`interpreter`] — [`DagInterpreter`]: the DAG walk engine.
//!
//! # Phase note
//!
//! Phase 0 landed the type stubs below. Phase 2 Task 2.1 adds the interpreter
//! engine (context, executor trait, expression language, and the interpreter
//! itself). Phase 2 Task 2.2 implements the seven concrete step executors.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Sub-modules ─────────────────────────────────────────────────────────────

pub mod attestation;
pub mod context;
pub mod discernment_gate;
pub mod executor;
pub mod executors;
pub mod expr;
pub mod interpreter;
pub mod rule_expr;
pub mod rules_artifact;
pub mod seven_valence_rules;
pub mod universal_band;

// ─── Public re-exports ────────────────────────────────────────────────────────

pub use context::GateContext;
pub use executor::{ArcExecutor, StepExecutor, StepKind, StepOutcome};
pub use executors::{
    ContentNodeResolver, ContextAssembleExecutor, EmbeddedContentNodeResolver,
    EscalateToReviewExecutor, MechanicalRulesetExecutor, SynthesizeExecutor, WisdomInvokeExecutor,
};
pub use interpreter::DagInterpreter;

// ─── Phase-0 type definitions (path-stable) ──────────────────────────────────
//
// These types were originally in `src/dag.rs`. They remain public with the
// same paths as before: `gate_client::dag::StepType`, etc.

/// The seven v1 step types, per spec §2.1.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum StepType {
    /// Gather signals into GateContext via memory/DHT/source-chain pulls.
    ContextAssemble { params: ContextAssembleParams },

    /// Core-constitution-primed elohim LLM call.
    WisdomInvoke { params: WisdomInvokeParams },

    /// Apply a CID-addressed declarative rule set.
    MechanicalRuleset { params: MechanicalRulesetParams },

    /// Query + reduce over DHT attestation graph.
    AggregateAttestations { params: AggregateAttestationsParams },

    /// Invoke a named ElohimCapability as a sub-step.
    SkillInvoke { params: SkillInvokeParams },

    /// Compose prior step outputs into a GateDecision + side effects.
    Synthesize { params: SynthesizeParams },

    /// Terminal: route to app-steward / qahal / existential review.
    EscalateToReview { params: EscalateToReviewParams },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAssembleParams {
    pub pulls: Vec<Pull>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pull {
    pub from: String,
    pub query: String,
    pub output_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WisdomInvokeParams {
    pub constitution_cid: String,
    pub framing_cid: String,
    pub context_keys: Vec<String>,
    pub output_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MechanicalRulesetParams {
    pub rules_cid: String,
    pub input_keys: Vec<String>,
    pub output_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateAttestationsParams {
    pub aggregation_spec_cid: String,
    pub subject_key: String,
    pub output_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInvokeParams {
    pub capability: String,
    pub request_from_keys: Vec<String>,
    pub output_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizeParams {
    pub input_keys: Vec<String>,
    pub decision_builder: String,
    pub side_effects: Vec<SideEffectSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideEffectSpec {
    #[serde(rename = "type")]
    pub effect_type: String,
    pub params_from_keys: Vec<String>,
    /// Optional shape hint for the side effect (e.g. "StoryPointLink").
    ///
    /// When present, this is passed through to the concrete `SideEffect`
    /// constructor.  When absent, the conversion falls back to "Unknown".
    /// Declared in the gate-process YAML alongside the side-effect type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalateToReviewParams {
    pub target_spec_cid: String,
    pub severity: String,
}

/// A full gate-process DAG — nodes + edges + terminals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateProcessDag {
    pub entrypoint: String,
    pub steps: HashMap<String, StepNode>,
    pub terminals: HashMap<String, TerminalNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepNode {
    #[serde(flatten)]
    pub step: StepType,
    #[serde(default)]
    pub next: Option<String>,
    #[serde(default)]
    pub edges: Vec<ConditionalEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalEdge {
    pub when: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalNode {
    pub decision: serde_json::Value,
    #[serde(default)]
    pub side_effects: Vec<SideEffectSpec>,
}

/// A gate-process declaration as stored in a ContentNode with
/// `contentType: gate-process-declaration`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateProcessDeclaration {
    pub name: String,
    pub version: String,
    pub event_type: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub dag: GateProcessDag,
}
