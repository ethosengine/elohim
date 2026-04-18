//! MechanicalRulesetExecutor — applies a CID-addressed declarative rule set.
//!
//! Matches `StepType::MechanicalRuleset { params }`. Implements the 5th
//! concrete [`StepExecutor`] (Phase 2 shipped four; Phase 3 ships this one).
//!
//! # Execution model
//!
//! 1. Fetch the rules artifact body via the injected [`ContentNodeResolver`],
//!    using `params.rules_cid` as the lookup key.
//! 2. Deserialize the body as [`RulesArtifact`].
//! 3. Build a sub-context from `params.input_keys` (keys present in `GateContext`).
//! 4. Evaluate rules in declaration order using the rule condition DSL
//!    ([`crate::dag::rule_expr`]). First match wins.
//! 5. Write the winning rule's `outcome` as a `serde_json::Value` to
//!    `params.output_key` in `GateContext`. Write `Value::Null` if no rule
//!    matches (the DAG edge `"ruleDecision == null"` then routes to
//!    `terminal-no-mint`).
//! 6. Return `StepOutcome::Continue` always — the outcome disposition is
//!    expressed via the written context key and DAG edges, not by short-circuiting.
//!
//! # Wrong step kind
//!
//! Receiving a step whose `kind()` is not `MechanicalRuleset` is a
//! `GateError::DagExecution` — same contract as all other executors.
//!
//! # ContentNodeResolver
//!
//! The executor is injected with a `Box<dyn ContentNodeResolver>` at
//! construction time. For unit tests, use [`EmbeddedContentNodeResolver`]
//! pre-loaded with in-memory artifacts. For Phase 3 dev-context, use
//! [`EmbeddedContentNodeResolver`] bundled with the seven-valence rules JSON.
//! For production, a DHT-backed resolver will be injected here.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::{debug, info};

use crate::dag::context::GateContext;
use crate::dag::executor::{StepExecutor, StepOutcome};
use crate::dag::rule_expr::{evaluate_condition, parse_rule_condition};
use crate::dag::rules_artifact::RulesArtifact;
use crate::dag::{MechanicalRulesetParams, StepType};
use crate::error::GateError;
use crate::events::RelationalImpactEvent;

// ─── ContentNodeResolver trait ────────────────────────────────────────────────

/// Resolves a CID to a ContentNode's body as `serde_json::Value`.
///
/// This is a narrower contract than [`super::context_assemble::PullResolver`]:
/// it takes a CID (content-addressed identity) and returns the body — not an
/// open query string. The executor does not need to know how the CID is
/// resolved (DHT fetch, embedded map, HTTP call); that is the resolver's job.
///
/// # Relation to PullResolver
///
/// `PullResolver` is the generic pull-source abstraction used by
/// `ContextAssembleExecutor` (`from`, `query`, `outputKey` triples). It handles
/// sources like `elohim-storage`, `dht`, `source-chain`, `manifest` with
/// arbitrary query strings.
///
/// `ContentNodeResolver` is a narrower contract for CID-to-body resolution
/// used by `MechanicalRulesetExecutor` and future executors that fetch
/// parameter artifacts by CID. The two traits serve different purposes and
/// are kept separate so neither bleeds into the other's concern.
#[async_trait]
pub trait ContentNodeResolver: Send + Sync {
    /// Fetch the body of the ContentNode identified by `cid`.
    ///
    /// Returns `Err(GateError::DagExecution(_))` if the CID cannot be resolved
    /// (not found, network error, etc.).
    async fn resolve_body(&self, cid: &str) -> Result<Value, GateError>;
}

// ─── EmbeddedContentNodeResolver ─────────────────────────────────────────────

/// A [`ContentNodeResolver`] backed by an in-memory CID-to-body map.
///
/// Used for:
/// - Unit tests (inject a hand-crafted rules artifact without network/DHT).
/// - Phase 3 dev-context (bundle the seven-valence rules JSON at compile time).
///
/// Construction: build the map with known CIDs → body JSON values, then call
/// [`DagRunner::with_content_resolver`] (or [`MechanicalRulesetExecutor::new`]
/// directly for unit tests).
pub struct EmbeddedContentNodeResolver {
    /// Maps CID strings to their ContentNode body values.
    pub bodies: HashMap<String, Value>,
}

impl EmbeddedContentNodeResolver {
    /// Construct from a pre-built map.
    pub fn new(bodies: HashMap<String, Value>) -> Self {
        Self { bodies }
    }

    /// Convenience: a single-entry resolver for use in unit tests.
    pub fn single(cid: impl Into<String>, body: Value) -> Self {
        let mut map = HashMap::new();
        map.insert(cid.into(), body);
        Self { bodies: map }
    }
}

#[async_trait]
impl ContentNodeResolver for EmbeddedContentNodeResolver {
    async fn resolve_body(&self, cid: &str) -> Result<Value, GateError> {
        self.bodies.get(cid).cloned().ok_or_else(|| {
            GateError::DagExecution(format!(
                "ContentNodeResolver: no embedded body for CID '{cid}'"
            ))
        })
    }
}

// ─── MechanicalRulesetExecutor ────────────────────────────────────────────────

/// Executor for `StepType::MechanicalRuleset`.
///
/// Injected with a [`ContentNodeResolver`] at construction; stateless per-call
/// so it is `Send + Sync` and can be wrapped in `Arc<dyn StepExecutor>`.
pub struct MechanicalRulesetExecutor {
    resolver: Box<dyn ContentNodeResolver>,
}

impl MechanicalRulesetExecutor {
    /// Construct with a custom resolver.
    pub fn new(resolver: Box<dyn ContentNodeResolver>) -> Self {
        Self { resolver }
    }

    async fn execute_params(
        &self,
        params: &MechanicalRulesetParams,
        ctx: &mut GateContext,
    ) -> Result<(), GateError> {
        // Step 1: fetch the rules artifact body.
        let body = self.resolver.resolve_body(&params.rules_cid).await?;

        // Step 2: deserialize.
        let artifact = RulesArtifact::from_json(&body)?;

        debug!(
            rules_cid = params.rules_cid,
            rule_set_name = artifact.rule_set_name,
            rule_count = artifact.rules.len(),
            "MechanicalRulesetExecutor: evaluating rules"
        );

        // Step 3: build sub-context (only keys declared in input_keys are
        // visible to the rule evaluator — prevents accidental leakage).
        let mut rule_ctx = GateContext::new();
        for key in &params.input_keys {
            if let Some(val) = ctx.get(key) {
                rule_ctx.insert(key.clone(), val.clone());
            }
            // Missing keys are not errors — they resolve to null in the DSL.
        }

        // Step 4: evaluate rules in order, first-match-wins.
        let mut matched_outcome: Option<Value> = None;
        let mut matched_rule_id: Option<String> = None;

        for rule in &artifact.rules {
            // Parse the condition — hard failure if the DSL is malformed.
            let condition = parse_rule_condition(&rule.when).map_err(|e| {
                GateError::DagExecution(format!(
                    "rule '{}' in artifact '{}': failed to parse condition '{}': {e}",
                    rule.id, artifact.rule_set_name, rule.when
                ))
            })?;

            if evaluate_condition(&condition, &rule_ctx) {
                matched_outcome = Some(json!({
                    "valence": rule.outcome.valence,
                    "magnitude": rule.outcome.magnitude,
                    "evidenceType": rule.outcome.evidence_type,
                    "ruleId": rule.id,
                    "ruleSetName": artifact.rule_set_name
                }));
                matched_rule_id = Some(rule.id.clone());
                break;
            }
        }

        // Step 5: write result to output_key.
        let outcome_val = matched_outcome.unwrap_or(Value::Null);

        info!(
            rules_cid = params.rules_cid,
            output_key = params.output_key,
            matched_rule = matched_rule_id.as_deref().unwrap_or("(none)"),
            "MechanicalRulesetExecutor: wrote outcome"
        );

        ctx.insert(params.output_key.clone(), outcome_val);

        Ok(())
    }
}

#[async_trait]
impl StepExecutor for MechanicalRulesetExecutor {
    async fn execute(
        &self,
        step: &StepType,
        ctx: &mut GateContext,
        _event: &RelationalImpactEvent,
    ) -> Result<StepOutcome, GateError> {
        let params = match step {
            StepType::MechanicalRuleset { params } => params,
            _ => {
                return Err(GateError::DagExecution(
                    "MechanicalRulesetExecutor received wrong step kind".to_string(),
                ))
            }
        };
        self.execute_params(params, ctx).await?;
        Ok(StepOutcome::Continue)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::{MechanicalRulesetParams, StepType, WisdomInvokeParams};
    use crate::events::RelationalImpactEvent;
    use serde_json::json;

    fn event() -> RelationalImpactEvent {
        RelationalImpactEvent::AttestationWrite {
            subject_hash: "hash-1".into(),
            claim_kind: "brit".into(),
            issuer: "agent-a".into(),
        }
    }

    fn minimal_rules_body() -> Value {
        json!({
            "ruleSetName": "test-v1",
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
                    },
                    "rationale": "First pass."
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

    fn step(rules_cid: &str, input_keys: Vec<String>, output_key: &str) -> StepType {
        StepType::MechanicalRuleset {
            params: MechanicalRulesetParams {
                rules_cid: rules_cid.into(),
                input_keys,
                output_key: output_key.into(),
            },
        }
    }

    fn executor_with(cid: &str, body: Value) -> MechanicalRulesetExecutor {
        MechanicalRulesetExecutor::new(Box::new(EmbeddedContentNodeResolver::single(cid, body)))
    }

    // ─── Happy path: rule 1 (first-pass-green) ────────────────────────────────

    #[tokio::test]
    async fn rule1_first_pass_green_writes_outcome() {
        let exec = executor_with("cid-rules", minimal_rules_body());
        let mut ctx = GateContext::new();
        ctx.insert("moment", json!({"status": "passed"}));
        ctx.insert(
            "priorAttestations",
            json!({"count": 0, "mostRecentStatus": null}),
        );

        let s = step(
            "cid-rules",
            vec!["moment".into(), "priorAttestations".into()],
            "ruleDecision",
        );
        let outcome = exec.execute(&s, &mut ctx, &event()).await.unwrap();

        assert!(matches!(outcome, StepOutcome::Continue));
        let decision = ctx.get("ruleDecision").unwrap();
        assert_eq!(
            decision.get("valence").and_then(|v| v.as_str()),
            Some("progress")
        );
        assert_eq!(
            decision.get("evidenceType").and_then(|v| v.as_str()),
            Some("first-pass-green")
        );
        assert_eq!(
            decision.get("ruleId").and_then(|v| v.as_str()),
            Some("rule-1")
        );
    }

    // ─── Happy path: rule 2 (regression) ─────────────────────────────────────

    #[tokio::test]
    async fn rule2_regression_writes_outcome() {
        let exec = executor_with("cid-rules", minimal_rules_body());
        let mut ctx = GateContext::new();
        ctx.insert("moment", json!({"status": "failed"}));
        ctx.insert(
            "priorAttestations",
            json!({"count": 1, "mostRecentStatus": "passed"}),
        );

        let s = step(
            "cid-rules",
            vec!["moment".into(), "priorAttestations".into()],
            "ruleDecision",
        );
        exec.execute(&s, &mut ctx, &event()).await.unwrap();

        let decision = ctx.get("ruleDecision").unwrap();
        assert_eq!(
            decision.get("valence").and_then(|v| v.as_str()),
            Some("regression")
        );
        assert_eq!(
            decision.get("evidenceType").and_then(|v| v.as_str()),
            Some("known-cause-recurrence")
        );
    }

    // ─── No-match path: steady-state writes null ──────────────────────────────

    #[tokio::test]
    async fn no_rule_match_writes_null() {
        let exec = executor_with("cid-rules", minimal_rules_body());
        let mut ctx = GateContext::new();
        // Scenario: passed but prior attestation exists — rule 1 won't match,
        // rule 2 won't match (not failed), so no rule fires → null.
        ctx.insert("moment", json!({"status": "passed"}));
        ctx.insert(
            "priorAttestations",
            json!({"count": 3, "mostRecentStatus": "passed"}),
        );

        let s = step(
            "cid-rules",
            vec!["moment".into(), "priorAttestations".into()],
            "ruleDecision",
        );
        exec.execute(&s, &mut ctx, &event()).await.unwrap();

        assert_eq!(ctx.get("ruleDecision"), Some(&Value::Null));
    }

    // ─── CID not found → error ────────────────────────────────────────────────

    #[tokio::test]
    async fn unknown_cid_returns_error() {
        let exec = executor_with("known-cid", minimal_rules_body());
        let mut ctx = GateContext::new();

        let s = step("unknown-cid", vec![], "ruleDecision");
        let result = exec.execute(&s, &mut ctx, &event()).await;
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err for unknown CID but got Ok"),
        };
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("unknown-cid"),
            "error must name the CID, got: {msg}"
        );
    }

    // ─── Malformed rules artifact body → error ────────────────────────────────

    #[tokio::test]
    async fn malformed_artifact_body_returns_error() {
        let exec = executor_with("cid-bad", json!({"notAnArtifact": true}));
        let mut ctx = GateContext::new();

        let s = step("cid-bad", vec![], "ruleDecision");
        let result = exec.execute(&s, &mut ctx, &event()).await;
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err for malformed artifact but got Ok"),
        };
        assert!(matches!(err, GateError::DagExecution(_)));
    }

    // ─── Malformed rule condition → error ─────────────────────────────────────

    #[tokio::test]
    async fn malformed_rule_condition_returns_error() {
        let bad_body = json!({
            "ruleSetName": "bad-rules",
            "version": "1.0.0",
            "appliesToEventType": "attestation-write",
            "inputKeysExpected": [],
            "rules": [
                {
                    "id": "bad-rule",
                    "when": "x = null",  // single = is invalid
                    "outcome": {
                        "valence": "progress",
                        "magnitude": "meaningful",
                        "evidenceType": "test"
                    }
                }
            ]
        });
        let exec = executor_with("cid-bad-rule", bad_body);
        let mut ctx = GateContext::new();

        let s = step("cid-bad-rule", vec![], "ruleDecision");
        let result = exec.execute(&s, &mut ctx, &event()).await;
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err for malformed rule condition but got Ok"),
        };
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("bad-rule"),
            "error must name the offending rule, got: {msg}"
        );
    }

    // ─── Wrong step kind → error ──────────────────────────────────────────────

    #[tokio::test]
    async fn wrong_step_kind_returns_error() {
        let exec = executor_with("cid-rules", minimal_rules_body());
        let wrong_step = StepType::WisdomInvoke {
            params: WisdomInvokeParams {
                constitution_cid: "c".into(),
                framing_cid: "f".into(),
                context_keys: vec![],
                output_key: "o".into(),
            },
        };
        let mut ctx = GateContext::new();
        let result = exec.execute(&wrong_step, &mut ctx, &event()).await;
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err for wrong step kind but got Ok"),
        };
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("wrong step kind"),
            "error must mention wrong step kind, got: {msg}"
        );
    }

    // ─── Input keys filtering: keys not in input_keys are not visible ─────────

    #[tokio::test]
    async fn input_keys_filtering_isolates_rule_context() {
        // Only "moment" is in input_keys; "priorAttestations" is not.
        // Rule 1 references both → priorAttestations.count resolves to null,
        // so count == 0 becomes null == 0 → false → rule 1 does not match.
        let exec = executor_with("cid-rules", minimal_rules_body());
        let mut ctx = GateContext::new();
        ctx.insert("moment", json!({"status": "passed"}));
        ctx.insert("priorAttestations", json!({"count": 0})); // present in ctx but not input_keys

        let s = step("cid-rules", vec!["moment".into()], "ruleDecision"); // priorAttestations excluded
        exec.execute(&s, &mut ctx, &event()).await.unwrap();

        // Rule 1 should NOT match because priorAttestations is not visible.
        assert_eq!(
            ctx.get("ruleDecision"),
            Some(&Value::Null),
            "rule 1 must not fire when priorAttestations is excluded from input_keys"
        );
    }

    // ─── Output key is written even when no rules match ───────────────────────

    #[tokio::test]
    async fn output_key_always_written() {
        let exec = executor_with("cid-rules", minimal_rules_body());
        let mut ctx = GateContext::new();
        // Provide no matching context.

        let s = step("cid-rules", vec![], "theOutput");
        exec.execute(&s, &mut ctx, &event()).await.unwrap();

        // Key must be present (null) even with no match.
        assert!(
            ctx.get("theOutput").is_some(),
            "output_key must be written even when no rules match"
        );
    }

    // ─── EmbeddedContentNodeResolver: exact CID match ─────────────────────────

    #[tokio::test]
    async fn embedded_resolver_exact_match() {
        let resolver = EmbeddedContentNodeResolver::single("my-cid", json!({"hello": "world"}));
        let body = resolver.resolve_body("my-cid").await.unwrap();
        assert_eq!(body, json!({"hello": "world"}));
    }

    #[tokio::test]
    async fn embedded_resolver_unknown_cid_errors() {
        let resolver = EmbeddedContentNodeResolver::single("known", json!({}));
        let err = resolver.resolve_body("unknown").await.unwrap_err();
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(msg.contains("unknown"), "got: {msg}");
    }

    // ─── First-match-wins ordering ────────────────────────────────────────────

    #[tokio::test]
    async fn first_match_wins_not_last() {
        // Both rules can match — only the first in declaration order should fire.
        let two_matching_rules = json!({
            "ruleSetName": "ordering-test",
            "version": "1.0.0",
            "appliesToEventType": "test",
            "inputKeysExpected": ["x"],
            "rules": [
                {
                    "id": "first",
                    "when": "x == null",
                    "outcome": {"valence": "first-match", "magnitude": "small", "evidenceType": "a"}
                },
                {
                    "id": "second",
                    "when": "x == null",
                    "outcome": {"valence": "second-match", "magnitude": "small", "evidenceType": "b"}
                }
            ]
        });
        let exec = executor_with("cid-order", two_matching_rules);
        let mut ctx = GateContext::new();
        // x is missing → null

        let s = step("cid-order", vec!["x".into()], "out");
        exec.execute(&s, &mut ctx, &event()).await.unwrap();

        let out = ctx.get("out").unwrap();
        assert_eq!(
            out.get("valence").and_then(|v| v.as_str()),
            Some("first-match"),
            "first matching rule must win"
        );
        assert_eq!(
            out.get("ruleId").and_then(|v| v.as_str()),
            Some("first"),
            "ruleId must identify the winning rule"
        );
    }

    // ─── ruleSetName is propagated into outcome ───────────────────────────────

    #[tokio::test]
    async fn outcome_carries_rule_set_name() {
        let exec = executor_with("cid-rules", minimal_rules_body());
        let mut ctx = GateContext::new();
        ctx.insert("moment", json!({"status": "passed"}));
        ctx.insert("priorAttestations", json!({"count": 0}));

        let s = step(
            "cid-rules",
            vec!["moment".into(), "priorAttestations".into()],
            "ruleDecision",
        );
        exec.execute(&s, &mut ctx, &event()).await.unwrap();

        let decision = ctx.get("ruleDecision").unwrap();
        assert_eq!(
            decision.get("ruleSetName").and_then(|v| v.as_str()),
            Some("test-v1"),
            "ruleSetName must be included in outcome for auditability"
        );
    }
}
