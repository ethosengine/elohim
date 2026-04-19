//! Concrete step executors for the v1 universal-band DAG.
//!
//! Phase 2 ships four executors:
//! - [`ContextAssembleExecutor`] — gathers signals into GateContext via pull sources.
//! - [`WisdomInvokeExecutor`] — dev-context stub for elohim LLM wisdom calls.
//! - [`SynthesizeExecutor`] — composes prior step outputs into a final GateDecision.
//! - [`EscalateToReviewExecutor`] — routes to steward/qahal/existential review.
//!
//! Phase 3 adds:
//! - [`MechanicalRulesetExecutor`] — applies a CID-addressed declarative rule set.
//!
//! Phase 5 adds:
//! - [`AggregateAttestationsExecutor`] — queries attestations, applies a reduction,
//!   writes a structured scalar outcome to GateContext.
//!
//! Phase 5+ will add `SkillInvokeExecutor`.
//!
//! ## Side-effect conversion
//!
//! The shared helper [`convert_side_effect_specs`] converts `Vec<SideEffectSpec>`
//! from a DAG node (both terminal nodes and `Synthesize` steps) into concrete
//! `Vec<SideEffect>` values. Used by both `SynthesizeExecutor` and
//! `terminal_to_decision` in the interpreter.

pub mod aggregate_attestations;
pub mod context_assemble;
pub mod escalate_to_review;
pub mod mechanical_ruleset;
/// Phase 3+ pull resolver stubs — return `Value::Null` with a warn log until
/// real elohim-storage, DHT, source-chain, and manifest wiring lands.
pub mod phase3_stubs;
pub mod synthesize;
pub mod wisdom_invoke;

pub use aggregate_attestations::{
    AggregateAttestationsExecutor, AttestationRecord, AttestationResolver, NullAttestationResolver,
    StubAttestationResolver,
};
pub use context_assemble::ContextAssembleExecutor;
pub use escalate_to_review::EscalateToReviewExecutor;
pub use mechanical_ruleset::{
    ContentNodeResolver, EmbeddedContentNodeResolver, MechanicalRulesetExecutor,
};
pub use synthesize::SynthesizeExecutor;
pub use wisdom_invoke::WisdomInvokeExecutor;

use serde_json::Value;

use crate::dag::SideEffectSpec;
use crate::error::GateError;
use crate::types::SideEffect;

use super::context::GateContext;

// ─── convert_side_effect_specs ────────────────────────────────────────────────

/// Convert a slice of `SideEffectSpec` declarations into concrete `SideEffect`
/// values by resolving the named context keys.
///
/// Used by both `SynthesizeExecutor` and `terminal_to_decision` in the
/// interpreter.
///
/// ## Phase 2 behaviour
///
/// Conversions are best-effort but not silent: if a required context key is
/// missing, returns `GateError::DagExecution` naming the key. Unknown effect
/// types also return `GateError::DagExecution`.
///
/// Real side-effect execution (DHT writes, conductor calls) is Phase 7+ when
/// doorway wiring lands.
pub fn convert_side_effect_specs(
    specs: &[SideEffectSpec],
    ctx: &GateContext,
) -> Result<Vec<SideEffect>, GateError> {
    let mut out = Vec::with_capacity(specs.len());
    for spec in specs {
        let effect = convert_one(spec, ctx)?;
        out.push(effect);
    }
    Ok(out)
}

/// Convert a single `SideEffectSpec` to a concrete `SideEffect`.
fn convert_one(spec: &SideEffectSpec, ctx: &GateContext) -> Result<SideEffect, GateError> {
    match spec.effect_type.as_str() {
        "MintAttestation" => {
            // Expected keys: [momentEntryHash, ruleDecision]
            // Shape defaults to "Unknown" if no `shape` field in spec.
            let target_hash_key = spec.params_from_keys.first().ok_or_else(|| {
                GateError::DagExecution(
                    "MintAttestation side effect requires at least 1 key in params_from_keys; \
                     got 0 in spec for effect_type 'MintAttestation'"
                        .to_string(),
                )
            })?;
            let tag_key = spec.params_from_keys.get(1).ok_or_else(|| {
                GateError::DagExecution(
                    "MintAttestation side effect requires at least 2 keys in params_from_keys \
                     (target_hash, rule_decision); got 1"
                        .to_string(),
                )
            })?;

            let target_hash = require_string_key(ctx, target_hash_key)?;
            let tag_val = require_key(ctx, tag_key)?;
            let tag_json = serde_json::to_string(tag_val)?;

            // Use the `shape` field from the spec declaration if present,
            // otherwise default to "Unknown".  For the discernment-gate the
            // YAML declares `shape: "StoryPointLink"`, which flows through here
            // so callers can verify the correct attestation shape was requested.
            let shape = spec.shape.clone().unwrap_or_else(|| "Unknown".to_string());

            Ok(SideEffect::MintAttestation { shape, target_hash, tag_json })
        }

        "EmitEconomicEvent" => {
            let event_key = spec.params_from_keys.first().ok_or_else(|| {
                GateError::DagExecution(
                    "EmitEconomicEvent side effect requires at least 1 key in params_from_keys"
                        .to_string(),
                )
            })?;
            let event_val = require_key(ctx, event_key)?;
            let event_json = serde_json::to_string(event_val)?;
            Ok(SideEffect::EmitEconomicEvent { event_json })
        }

        "OpenStewardReview" => {
            // Expects keys: [grounds, context]
            let grounds_key = spec.params_from_keys.first().ok_or_else(|| {
                GateError::DagExecution(
                    "OpenStewardReview requires at least 1 key (grounds) in params_from_keys"
                        .to_string(),
                )
            })?;
            let context_key = spec.params_from_keys.get(1).ok_or_else(|| {
                GateError::DagExecution(
                    "OpenStewardReview requires at least 2 keys (grounds, context) in params_from_keys"
                        .to_string(),
                )
            })?;

            let grounds_val = require_key(ctx, grounds_key)?;
            let context_val = require_key(ctx, context_key)?;

            Ok(SideEffect::OpenStewardReview {
                grounds_json: serde_json::to_string(grounds_val)?,
                context_json: serde_json::to_string(context_val)?,
            })
        }

        "UpdateReachAggregation" => {
            // Expects keys: [subject, delta]
            let subject_key = spec.params_from_keys.first().ok_or_else(|| {
                GateError::DagExecution(
                    "UpdateReachAggregation requires at least 1 key (subject) in params_from_keys"
                        .to_string(),
                )
            })?;
            let delta_key = spec.params_from_keys.get(1).ok_or_else(|| {
                GateError::DagExecution(
                    "UpdateReachAggregation requires at least 2 keys (subject, delta) in params_from_keys"
                        .to_string(),
                )
            })?;

            let subject_hash = require_string_key(ctx, subject_key)?;
            let delta_val = require_key(ctx, delta_key)?;
            let delta_json = serde_json::to_string(delta_val)?;

            Ok(SideEffect::UpdateReachAggregation { subject_hash, delta_json })
        }

        unknown => Err(GateError::DagExecution(format!(
            "unknown side effect type '{unknown}'; \
             supported: MintAttestation, EmitEconomicEvent, OpenStewardReview, UpdateReachAggregation"
        ))),
    }
}

/// Require a context key to be present; return an error naming it if absent.
fn require_key<'a>(ctx: &'a GateContext, key: &str) -> Result<&'a Value, GateError> {
    ctx.get(key).ok_or_else(|| {
        GateError::DagExecution(format!(
            "side-effect conversion: missing required context key '{key}'"
        ))
    })
}

/// Require a context key that must be a string; returns the owned `String`.
fn require_string_key(ctx: &GateContext, key: &str) -> Result<String, GateError> {
    let val = require_key(ctx, key)?;
    match val {
        Value::String(s) => Ok(s.clone()),
        other => {
            // Non-string values are serialized to string as a best-effort fallback.
            Ok(serde_json::to_string(other)?)
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::SideEffectSpec;
    use crate::types::SideEffect;
    use serde_json::json;

    fn ctx_with(pairs: &[(&str, serde_json::Value)]) -> GateContext {
        let mut ctx = GateContext::new();
        for (k, v) in pairs {
            ctx.insert(*k, v.clone());
        }
        ctx
    }

    // ─── MintAttestation ──────────────────────────────────────────────────────

    #[test]
    fn mint_attestation_happy_path() {
        let specs = vec![SideEffectSpec {
            effect_type: "MintAttestation".into(),
            params_from_keys: vec!["momentEntryHash".into(), "ruleDecision".into()],
            shape: None,
        }];
        let ctx = ctx_with(&[
            ("momentEntryHash", json!("hash-abc")),
            ("ruleDecision", json!({"valence": "constructive"})),
        ]);
        let result = convert_side_effect_specs(&specs, &ctx).unwrap();
        assert_eq!(result.len(), 1);
        assert!(
            matches!(&result[0], SideEffect::MintAttestation { target_hash, .. } if target_hash == "hash-abc"),
            "got: {:?}",
            result[0]
        );
    }

    #[test]
    fn mint_attestation_missing_target_key_returns_error() {
        let specs = vec![SideEffectSpec {
            effect_type: "MintAttestation".into(),
            params_from_keys: vec!["missingHash".into(), "ruleDecision".into()],
            shape: None,
        }];
        let ctx = ctx_with(&[("ruleDecision", json!({}))]);
        let err = convert_side_effect_specs(&specs, &ctx).unwrap_err();
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(msg.contains("missingHash"), "got: {msg}");
    }

    // ─── EmitEconomicEvent ────────────────────────────────────────────────────

    #[test]
    fn emit_economic_event_happy_path() {
        let specs = vec![SideEffectSpec {
            effect_type: "EmitEconomicEvent".into(),
            params_from_keys: vec!["eventPayload".into()],
            shape: None,
        }];
        let ctx = ctx_with(&[("eventPayload", json!({"kind": "transfer", "amount": 5}))]);
        let result = convert_side_effect_specs(&specs, &ctx).unwrap();
        assert_eq!(result.len(), 1);
        assert!(
            matches!(&result[0], SideEffect::EmitEconomicEvent { event_json } if event_json.contains("transfer")),
            "got: {:?}",
            result[0]
        );
    }

    #[test]
    fn emit_economic_event_missing_key_returns_error() {
        let specs = vec![SideEffectSpec {
            effect_type: "EmitEconomicEvent".into(),
            params_from_keys: vec!["noSuchKey".into()],
            shape: None,
        }];
        let ctx = GateContext::new();
        let err = convert_side_effect_specs(&specs, &ctx).unwrap_err();
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(msg.contains("noSuchKey"), "got: {msg}");
    }

    // ─── OpenStewardReview ────────────────────────────────────────────────────

    #[test]
    fn open_steward_review_happy_path() {
        let specs = vec![SideEffectSpec {
            effect_type: "OpenStewardReview".into(),
            params_from_keys: vec!["grounds".into(), "context".into()],
            shape: None,
        }];
        let ctx = ctx_with(&[
            (
                "grounds",
                json!({"category": "safety", "summary": "potential harm"}),
            ),
            ("context", json!({"eventId": "evt-1"})),
        ]);
        let result = convert_side_effect_specs(&specs, &ctx).unwrap();
        assert_eq!(result.len(), 1);
        assert!(
            matches!(&result[0], SideEffect::OpenStewardReview { grounds_json, .. }
                if grounds_json.contains("safety")),
            "got: {:?}",
            result[0]
        );
    }

    #[test]
    fn open_steward_review_missing_context_key_returns_error() {
        let specs = vec![SideEffectSpec {
            effect_type: "OpenStewardReview".into(),
            params_from_keys: vec!["grounds".into(), "missingContext".into()],
            shape: None,
        }];
        let ctx = ctx_with(&[("grounds", json!({}))]);
        let err = convert_side_effect_specs(&specs, &ctx).unwrap_err();
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(msg.contains("missingContext"), "got: {msg}");
    }

    // ─── UpdateReachAggregation ───────────────────────────────────────────────

    #[test]
    fn update_reach_aggregation_happy_path() {
        let specs = vec![SideEffectSpec {
            effect_type: "UpdateReachAggregation".into(),
            params_from_keys: vec!["subject".into(), "delta".into()],
            shape: None,
        }];
        let ctx = ctx_with(&[
            ("subject", json!("agent-hash-xyz")),
            ("delta", json!({"reach": 1})),
        ]);
        let result = convert_side_effect_specs(&specs, &ctx).unwrap();
        assert_eq!(result.len(), 1);
        assert!(
            matches!(&result[0], SideEffect::UpdateReachAggregation { subject_hash, .. }
                if subject_hash.contains("agent-hash-xyz")),
            "got: {:?}",
            result[0]
        );
    }

    #[test]
    fn update_reach_aggregation_missing_delta_returns_error() {
        let specs = vec![SideEffectSpec {
            effect_type: "UpdateReachAggregation".into(),
            params_from_keys: vec!["subject".into(), "missingDelta".into()],
            shape: None,
        }];
        let ctx = ctx_with(&[("subject", json!("agent-hash"))]);
        let err = convert_side_effect_specs(&specs, &ctx).unwrap_err();
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(msg.contains("missingDelta"), "got: {msg}");
    }

    // ─── Unknown effect type → error ─────────────────────────────────────────

    #[test]
    fn unknown_effect_type_returns_error() {
        let specs = vec![SideEffectSpec {
            effect_type: "TeleportEntity".into(),
            params_from_keys: vec![],
            shape: None,
        }];
        let ctx = GateContext::new();
        let err = convert_side_effect_specs(&specs, &ctx).unwrap_err();
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(msg.contains("TeleportEntity"), "got: {msg}");
    }

    // ─── Empty specs → empty vec ──────────────────────────────────────────────

    #[test]
    fn empty_specs_returns_empty_vec() {
        let ctx = GateContext::new();
        let result = convert_side_effect_specs(&[], &ctx).unwrap();
        assert!(result.is_empty());
    }

    // ─── Multiple specs converted in order ───────────────────────────────────

    #[test]
    fn multiple_specs_converted_in_order() {
        let specs = vec![
            SideEffectSpec {
                effect_type: "EmitEconomicEvent".into(),
                params_from_keys: vec!["ev".into()],
                shape: None,
            },
            SideEffectSpec {
                effect_type: "MintAttestation".into(),
                params_from_keys: vec!["hash".into(), "tag".into()],
                shape: None,
            },
        ];
        let ctx = ctx_with(&[
            ("ev", json!({"kind": "transfer"})),
            ("hash", json!("h")),
            ("tag", json!({"valence": "constructive"})),
        ]);
        let result = convert_side_effect_specs(&specs, &ctx).unwrap();
        assert_eq!(result.len(), 2);
        assert!(matches!(&result[0], SideEffect::EmitEconomicEvent { .. }));
        assert!(matches!(&result[1], SideEffect::MintAttestation { .. }));
    }
}
