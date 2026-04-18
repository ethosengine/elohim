//! Gate-process DAG interpreter — reads a GateProcessDeclaration and executes
//! its step graph against an assembled GateContext.
//!
//! Phase 0: types only. The interpreter executor and step dispatch arrive in
//! Phase 2 alongside the universal-band-declaration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
