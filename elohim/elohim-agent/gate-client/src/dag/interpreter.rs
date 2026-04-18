//! DagInterpreter — walks a GateProcessDeclaration, dispatches steps,
//! evaluates conditional edges, and returns a GateDecision.
//!
//! ## Execution model (spec §2.3)
//!
//! 1. Start at `dag.entrypoint`.
//! 2. Look up the step in `dag.steps`.
//! 3. Dispatch to the registered executor for that step's kind.
//! 4. If the executor returns `Terminate`, short-circuit and return.
//! 5. Otherwise evaluate `edges` in order — the first matching `when` wins.
//!    If no edge matches but `next` is set, follow it.
//!    If the target resolves to a terminal id, go to step 7.
//! 6. Repeat from step 2 with the new step id.
//! 7. When a terminal is reached, parse its `decision` JSON into a
//!    `GateStatus`, construct and return `GateDecision`.
//!
//! ## Cycle detection
//!
//! If any step id is visited more than `MAX_STEP_VISITS` times, the interpreter
//! returns `GateError::DagExecution("cycle detected in DAG")`. A valid DAG has
//! no cycles; this protects against malformed manifests.

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::GateError;
use crate::events::RelationalImpactEvent;
use crate::phase::Phase;
use crate::types::{ConstitutionalReasoningSummary, GateDecision, GateStatus};

use super::context::GateContext;
use super::executor::{ArcExecutor, StepKind, StepOutcome};
use super::expr::eval_edge;
use super::{GateProcessDeclaration, StepType, TerminalNode};

/// Maximum number of times a single step id may be visited before the
/// interpreter declares a cycle.
const MAX_STEP_VISITS: usize = 64;

pub struct DagInterpreter {
    executors: HashMap<StepKind, ArcExecutor>,
}

impl Default for DagInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl DagInterpreter {
    /// Create a new interpreter with an empty executor registry.
    pub fn new() -> Self {
        Self { executors: HashMap::new() }
    }

    /// Register an executor for a specific step kind.
    ///
    /// Calling this twice for the same kind replaces the previous executor.
    pub fn register(&mut self, kind: StepKind, executor: Arc<dyn super::executor::StepExecutor>) {
        self.executors.insert(kind, executor);
    }

    /// Walk the DAG declared in `dag` and return the final gate decision.
    ///
    /// `initial_context` is typically assembled by the caller before running
    /// (e.g., the event payload fields pre-populated as context keys). Steps
    /// may extend it further as they execute.
    pub async fn run(
        &self,
        declaration: &GateProcessDeclaration,
        event: &RelationalImpactEvent,
        initial_context: GateContext,
    ) -> Result<GateDecision, GateError> {
        let dag = &declaration.dag;
        let mut ctx = initial_context;
        let mut current_id = dag.entrypoint.clone();

        // Visit counter for cycle detection.
        let mut visit_counts: HashMap<String, usize> = HashMap::new();

        loop {
            // ── Cycle detection ──────────────────────────────────────────────
            let visits = visit_counts.entry(current_id.clone()).or_insert(0);
            *visits += 1;
            if *visits > MAX_STEP_VISITS {
                return Err(GateError::DagExecution(format!(
                    "cycle detected in DAG: step '{current_id}' visited more than {MAX_STEP_VISITS} times"
                )));
            }

            // ── Terminal check ───────────────────────────────────────────────
            if let Some(terminal) = dag.terminals.get(&current_id) {
                return terminal_to_decision(terminal, &ctx);
            }

            // ── Step lookup ──────────────────────────────────────────────────
            let step_node = dag.steps.get(&current_id).ok_or_else(|| {
                GateError::DagExecution(format!(
                    "unknown step id '{current_id}' in DAG"
                ))
            })?;

            let step: &StepType = &step_node.step;
            let kind = step.kind();

            // ── Executor dispatch ────────────────────────────────────────────
            let executor = self.executors.get(&kind).ok_or_else(|| {
                GateError::DagExecution(format!(
                    "no executor registered for step kind {kind:?} (step '{current_id}')"
                ))
            })?;

            let outcome = executor.execute(step, &mut ctx, event).await?;

            // ── Early termination ────────────────────────────────────────────
            if let StepOutcome::Terminate(decision, _side_effects) = outcome {
                return Ok(decision);
            }

            // ── Edge resolution ──────────────────────────────────────────────
            //
            // Evaluate edges in declaration order; the first matching `when`
            // wins. If no edge matches, follow `next`. If neither is set,
            // the DAG is malformed.

            let mut next: Option<String> = None;

            for edge in &step_node.edges {
                match eval_edge(&edge.when, &ctx) {
                    Ok(true) => {
                        next = Some(edge.target.clone());
                        break;
                    }
                    Ok(false) => continue,
                    Err(e) => return Err(e),
                }
            }

            if next.is_none() {
                next = step_node.next.clone();
            }

            current_id = next.ok_or_else(|| {
                GateError::DagExecution(format!(
                    "step '{current_id}' has no matching edge and no 'next' fallback"
                ))
            })?;
        }
    }
}

/// Parse a [`TerminalNode`]'s `decision` JSON value into a [`GateDecision`].
///
/// The terminal's `decision` field must be a JSON object with a `status` tag
/// that deserializes into [`GateStatus`]. If it doesn't parse, that is a DAG
/// authoring error and we return `DagExecution`.
fn terminal_to_decision(terminal: &TerminalNode, _ctx: &GateContext) -> Result<GateDecision, GateError> {
    let status: GateStatus = serde_json::from_value(terminal.decision.clone())
        .map_err(|e| {
            GateError::DagExecution(format!(
                "terminal node 'decision' did not deserialize into GateStatus: {e}"
            ))
        })?;

    // Side effects from the terminal node are collected and included.
    let side_effects = terminal
        .side_effects
        .iter()
        .map(|spec| {
            // In Phase 2.1, side-effect conversion is not yet wired — the
            // SideEffectSpec carries a `type` string + `params_from_keys`.
            // Task 2.2 will map these to concrete SideEffect variants.
            // For now we return no side effects from terminal nodes.
            let _ = spec;
        })
        .collect::<Vec<_>>();
    let _ = side_effects;

    Ok(GateDecision {
        status,
        reasoning: ConstitutionalReasoningSummary {
            primary_principle: "dag-terminal".to_string(),
            summary: "Decision produced by DAG terminal node.".to_string(),
            confidence: 1.0,
            phase_note: "Phase 2 DAG interpreter.".to_string(),
        },
        side_effects: vec![],
        decision_attestation_cid: None,
        phase: Phase::DevContext,
    })
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::{
        executor::{TerminateExecutor, TestExecutor},
        ConditionalEdge, ContextAssembleParams, GateProcessDag, StepNode,
        WisdomInvokeParams,
    };
    use crate::phase::Phase;
    use crate::types::{GateStatus, GateTag};
    use serde_json::json;
    use std::collections::HashMap;

    // ─── DAG builder helpers ──────────────────────────────────────────────────

    fn content_publish_event() -> RelationalImpactEvent {
        RelationalImpactEvent::ContentPublish {
            content_cid: "cid".into(),
            declared_reach: "public".into(),
            author: "agent".into(),
        }
    }

    fn allow_terminal() -> TerminalNode {
        TerminalNode {
            decision: json!({"status": "allow", "exempt": false}),
            side_effects: vec![],
        }
    }

    fn decline_terminal() -> TerminalNode {
        TerminalNode {
            decision: json!({
                "status": "decline",
                "grounds": {
                    "category": "test-decline",
                    "summary": "declined by DAG",
                    "principleRefs": []
                }
            }),
            side_effects: vec![],
        }
    }

    fn verdict_terminal(valence: &str) -> TerminalNode {
        TerminalNode {
            decision: json!({
                "status": "verdict",
                "tag_kind": "story-point",
                "valence": valence,
                "magnitude": "medium",
                "evidenceType": "direct"
            }),
            side_effects: vec![],
        }
    }

    fn build_interpreter_with_continue() -> DagInterpreter {
        let mut interp = DagInterpreter::new();
        // Register TestExecutor (writes nothing, always Continue) for all 7 kinds.
        for kind in [
            StepKind::ContextAssemble,
            StepKind::WisdomInvoke,
            StepKind::MechanicalRuleset,
            StepKind::AggregateAttestations,
            StepKind::SkillInvoke,
            StepKind::Synthesize,
            StepKind::EscalateToReview,
        ] {
            interp.register(
                kind,
                Arc::new(TestExecutor {
                    output_key: "noop".into(),
                    output_value: json!(true),
                }),
            );
        }
        interp
    }

    // ─── Linear DAG (A → B → terminal) ───────────────────────────────────────

    #[tokio::test]
    async fn linear_dag_reaches_terminal() {
        // A → B → "allow-terminal"
        let mut steps = HashMap::new();
        steps.insert(
            "step-a".to_string(),
            StepNode {
                step: StepType::ContextAssemble {
                    params: ContextAssembleParams { pulls: vec![] },
                },
                next: Some("step-b".to_string()),
                edges: vec![],
            },
        );
        steps.insert(
            "step-b".to_string(),
            StepNode {
                step: StepType::ContextAssemble {
                    params: ContextAssembleParams { pulls: vec![] },
                },
                next: Some("allow-terminal".to_string()),
                edges: vec![],
            },
        );
        let mut terminals = HashMap::new();
        terminals.insert("allow-terminal".to_string(), allow_terminal());

        let dag = GateProcessDeclaration {
            name: "test".into(),
            version: "1".into(),
            event_type: "content-publish".into(),
            input_schema: json!({}),
            output_schema: json!({}),
            dag: GateProcessDag { entrypoint: "step-a".to_string(), steps, terminals },
        };

        let interp = build_interpreter_with_continue();
        let decision = interp
            .run(&dag, &content_publish_event(), GateContext::new())
            .await
            .unwrap();

        assert!(decision.is_allowed());
        assert!(!decision.is_exempt());
    }

    // ─── Conditional branching ────────────────────────────────────────────────

    #[tokio::test]
    async fn conditional_edge_routes_to_correct_terminal() {
        // step-a writes "result" = "positive"
        // edge: if result == 'positive' → allow-terminal
        //       else (next) → decline-terminal
        let mut steps = HashMap::new();
        steps.insert(
            "step-a".to_string(),
            StepNode {
                step: StepType::ContextAssemble {
                    params: ContextAssembleParams { pulls: vec![] },
                },
                next: Some("decline-terminal".to_string()),
                edges: vec![ConditionalEdge {
                    when: "result == 'positive'".to_string(),
                    target: "allow-terminal".to_string(),
                }],
            },
        );
        let mut terminals = HashMap::new();
        terminals.insert("allow-terminal".to_string(), allow_terminal());
        terminals.insert("decline-terminal".to_string(), decline_terminal());

        let dag = GateProcessDeclaration {
            name: "test".into(),
            version: "1".into(),
            event_type: "content-publish".into(),
            input_schema: json!({}),
            output_schema: json!({}),
            dag: GateProcessDag { entrypoint: "step-a".to_string(), steps, terminals },
        };

        // Executor writes "result" = "positive" to context.
        let mut interp = DagInterpreter::new();
        interp.register(
            StepKind::ContextAssemble,
            Arc::new(TestExecutor {
                output_key: "result".into(),
                output_value: json!("positive"),
            }),
        );

        let decision = interp
            .run(&dag, &content_publish_event(), GateContext::new())
            .await
            .unwrap();

        assert!(decision.is_allowed(), "should have routed to allow-terminal");
    }

    #[tokio::test]
    async fn conditional_edge_falls_through_to_next_when_no_match() {
        // Executor writes "result" = "negative"
        // edge: if result == 'positive' → allow-terminal (no match)
        // next: decline-terminal
        let mut steps = HashMap::new();
        steps.insert(
            "step-a".to_string(),
            StepNode {
                step: StepType::ContextAssemble {
                    params: ContextAssembleParams { pulls: vec![] },
                },
                next: Some("decline-terminal".to_string()),
                edges: vec![ConditionalEdge {
                    when: "result == 'positive'".to_string(),
                    target: "allow-terminal".to_string(),
                }],
            },
        );
        let mut terminals = HashMap::new();
        terminals.insert("allow-terminal".to_string(), allow_terminal());
        terminals.insert("decline-terminal".to_string(), decline_terminal());

        let dag = GateProcessDeclaration {
            name: "test".into(),
            version: "1".into(),
            event_type: "content-publish".into(),
            input_schema: json!({}),
            output_schema: json!({}),
            dag: GateProcessDag { entrypoint: "step-a".to_string(), steps, terminals },
        };

        let mut interp = DagInterpreter::new();
        interp.register(
            StepKind::ContextAssemble,
            Arc::new(TestExecutor {
                output_key: "result".into(),
                output_value: json!("negative"),
            }),
        );

        let decision = interp
            .run(&dag, &content_publish_event(), GateContext::new())
            .await
            .unwrap();

        assert!(!decision.is_allowed(), "should have fallen through to decline-terminal");
    }

    // ─── Early terminal via StepOutcome::Terminate ────────────────────────────

    #[tokio::test]
    async fn executor_terminate_short_circuits() {
        // step-a has Terminate executor — DAG should not proceed to step-b or terminals.
        let mut steps = HashMap::new();
        steps.insert(
            "step-a".to_string(),
            StepNode {
                step: StepType::WisdomInvoke {
                    params: WisdomInvokeParams {
                        constitution_cid: "c".into(),
                        framing_cid: "f".into(),
                        context_keys: vec![],
                        output_key: "out".into(),
                    },
                },
                next: Some("step-b".to_string()),
                edges: vec![],
            },
        );
        steps.insert(
            "step-b".to_string(),
            StepNode {
                step: StepType::ContextAssemble {
                    params: ContextAssembleParams { pulls: vec![] },
                },
                next: Some("allow-terminal".to_string()),
                edges: vec![],
            },
        );
        let mut terminals = HashMap::new();
        terminals.insert("allow-terminal".to_string(), allow_terminal());

        let dag = GateProcessDeclaration {
            name: "test".into(),
            version: "1".into(),
            event_type: "content-publish".into(),
            input_schema: json!({}),
            output_schema: json!({}),
            dag: GateProcessDag { entrypoint: "step-a".to_string(), steps, terminals },
        };

        // WisdomInvoke executor terminates immediately with Decline.
        let terminating_decision = GateDecision {
            status: crate::types::GateStatus::Decline {
                grounds: crate::types::DeclineGrounds {
                    category: "early-terminate".into(),
                    summary: "executor terminated early".into(),
                    principle_refs: vec![],
                },
            },
            reasoning: ConstitutionalReasoningSummary::mocked(),
            side_effects: vec![],
            decision_attestation_cid: None,
            phase: Phase::DevContext,
        };

        let mut interp = DagInterpreter::new();
        interp.register(
            StepKind::WisdomInvoke,
            Arc::new(TerminateExecutor { decision: terminating_decision }),
        );

        let decision = interp
            .run(&dag, &content_publish_event(), GateContext::new())
            .await
            .unwrap();

        assert!(!decision.is_allowed(), "executor Terminate must short-circuit to Decline");
        let json = serde_json::to_string(&decision.status).unwrap();
        assert!(json.contains("early-terminate"), "got: {json}");
    }

    // ─── Unknown step-id error ────────────────────────────────────────────────

    #[tokio::test]
    async fn unknown_step_id_returns_dag_execution_error() {
        let steps = HashMap::new(); // empty — entrypoint "step-x" does not exist
        let terminals = HashMap::new();

        let dag = GateProcessDeclaration {
            name: "test".into(),
            version: "1".into(),
            event_type: "content-publish".into(),
            input_schema: json!({}),
            output_schema: json!({}),
            dag: GateProcessDag {
                entrypoint: "step-x".to_string(),
                steps,
                terminals,
            },
        };

        let interp = DagInterpreter::new();
        let err = interp
            .run(&dag, &content_publish_event(), GateContext::new())
            .await
            .unwrap_err();

        assert!(
            matches!(err, GateError::DagExecution(_)),
            "expected DagExecution error, got: {err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("step-x"), "error message should name the bad id, got: {msg}");
    }

    // ─── Cycle detection ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn cycle_detection_triggers_after_max_visits() {
        // step-a → step-a (self-loop)
        let mut steps = HashMap::new();
        steps.insert(
            "step-a".to_string(),
            StepNode {
                step: StepType::ContextAssemble {
                    params: ContextAssembleParams { pulls: vec![] },
                },
                next: Some("step-a".to_string()), // self-loop
                edges: vec![],
            },
        );
        let terminals = HashMap::new();

        let dag = GateProcessDeclaration {
            name: "test".into(),
            version: "1".into(),
            event_type: "content-publish".into(),
            input_schema: json!({}),
            output_schema: json!({}),
            dag: GateProcessDag { entrypoint: "step-a".to_string(), steps, terminals },
        };

        let interp = build_interpreter_with_continue();
        let err = interp
            .run(&dag, &content_publish_event(), GateContext::new())
            .await
            .unwrap_err();

        assert!(
            matches!(err, GateError::DagExecution(_)),
            "expected DagExecution cycle error"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("cycle"),
            "error message should mention cycle, got: {msg}"
        );
    }

    // ─── Multi-terminal DAG ───────────────────────────────────────────────────

    #[tokio::test]
    async fn multi_terminal_dag_selects_correct_terminal_based_on_context() {
        // step-a writes "verdict_val" based on context key "input_valence"
        // edge: verdict_val == 'constructive' → constructive-terminal
        // edge: verdict_val == 'conflict'     → conflict-terminal
        // next: default-terminal
        //
        // We pre-populate "input_valence" = "conflict" in initial_context.
        // The TestExecutor copies input_valence into verdict_val.
        // Then edges route to conflict-terminal.

        let mut steps = HashMap::new();
        steps.insert(
            "step-classify".to_string(),
            StepNode {
                step: StepType::MechanicalRuleset {
                    params: crate::dag::MechanicalRulesetParams {
                        rules_cid: "r".into(),
                        input_keys: vec!["input_valence".into()],
                        output_key: "verdict_val".into(),
                    },
                },
                next: Some("default-terminal".to_string()),
                edges: vec![
                    ConditionalEdge {
                        when: "verdict_val == 'constructive'".to_string(),
                        target: "constructive-terminal".to_string(),
                    },
                    ConditionalEdge {
                        when: "verdict_val == 'conflict'".to_string(),
                        target: "conflict-terminal".to_string(),
                    },
                ],
            },
        );

        let mut terminals = HashMap::new();
        terminals.insert(
            "constructive-terminal".to_string(),
            verdict_terminal("constructive"),
        );
        terminals.insert("conflict-terminal".to_string(), verdict_terminal("conflict"));
        terminals.insert("default-terminal".to_string(), allow_terminal());

        let dag = GateProcessDeclaration {
            name: "test".into(),
            version: "1".into(),
            event_type: "content-publish".into(),
            input_schema: json!({}),
            output_schema: json!({}),
            dag: GateProcessDag {
                entrypoint: "step-classify".to_string(),
                steps,
                terminals,
            },
        };

        // Executor writes verdict_val = "conflict" (copying from context).
        let mut interp = DagInterpreter::new();
        interp.register(
            StepKind::MechanicalRuleset,
            Arc::new(TestExecutor {
                output_key: "verdict_val".into(),
                output_value: json!("conflict"),
            }),
        );

        let decision = interp
            .run(&dag, &content_publish_event(), GateContext::new())
            .await
            .unwrap();

        // Should have routed to conflict-terminal which emits Verdict(StoryPoint{valence:"conflict"})
        assert!(
            matches!(
                &decision.status,
                GateStatus::Verdict(GateTag::StoryPoint { valence, .. }) if valence == "conflict"
            ),
            "expected conflict StoryPoint verdict, got: {:?}",
            decision.status
        );
    }

    // ─── DAG with initial context pre-populated ───────────────────────────────

    #[tokio::test]
    async fn initial_context_is_available_to_first_step() {
        // The edge condition checks a key from the initial context directly.
        let mut steps = HashMap::new();
        steps.insert(
            "step-a".to_string(),
            StepNode {
                step: StepType::ContextAssemble {
                    params: ContextAssembleParams { pulls: vec![] },
                },
                next: Some("decline-terminal".to_string()),
                edges: vec![ConditionalEdge {
                    when: "pre_key == 'pre_val'".to_string(),
                    target: "allow-terminal".to_string(),
                }],
            },
        );
        let mut terminals = HashMap::new();
        terminals.insert("allow-terminal".to_string(), allow_terminal());
        terminals.insert("decline-terminal".to_string(), decline_terminal());

        let dag = GateProcessDeclaration {
            name: "test".into(),
            version: "1".into(),
            event_type: "content-publish".into(),
            input_schema: json!({}),
            output_schema: json!({}),
            dag: GateProcessDag { entrypoint: "step-a".to_string(), steps, terminals },
        };

        let mut initial = GateContext::new();
        initial.insert("pre_key", json!("pre_val")); // pre-populated

        // Noop executor for ContextAssemble.
        let mut interp = DagInterpreter::new();
        interp.register(
            StepKind::ContextAssemble,
            Arc::new(TestExecutor {
                output_key: "noop".into(),
                output_value: json!(null),
            }),
        );

        let decision = interp.run(&dag, &content_publish_event(), initial).await.unwrap();
        assert!(decision.is_allowed(), "initial context key should have matched the edge");
    }

    // ─── No executor registered ───────────────────────────────────────────────

    #[tokio::test]
    async fn missing_executor_returns_dag_execution_error() {
        let mut steps = HashMap::new();
        steps.insert(
            "step-a".to_string(),
            StepNode {
                step: StepType::WisdomInvoke {
                    params: WisdomInvokeParams {
                        constitution_cid: "c".into(),
                        framing_cid: "f".into(),
                        context_keys: vec![],
                        output_key: "out".into(),
                    },
                },
                next: Some("allow-terminal".to_string()),
                edges: vec![],
            },
        );
        let mut terminals = HashMap::new();
        terminals.insert("allow-terminal".to_string(), allow_terminal());

        let dag = GateProcessDeclaration {
            name: "test".into(),
            version: "1".into(),
            event_type: "content-publish".into(),
            input_schema: json!({}),
            output_schema: json!({}),
            dag: GateProcessDag { entrypoint: "step-a".to_string(), steps, terminals },
        };

        // No executor registered — DagInterpreter::new() only.
        let interp = DagInterpreter::new();
        let err = interp
            .run(&dag, &content_publish_event(), GateContext::new())
            .await
            .unwrap_err();

        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("WisdomInvoke") || msg.contains("wisdom") || msg.contains("executor"),
            "error should name the missing executor kind, got: {msg}"
        );
    }

    // ─── Step with no next and no matching edge ───────────────────────────────

    #[tokio::test]
    async fn step_with_no_routing_returns_dag_execution_error() {
        let mut steps = HashMap::new();
        steps.insert(
            "step-a".to_string(),
            StepNode {
                step: StepType::ContextAssemble {
                    params: ContextAssembleParams { pulls: vec![] },
                },
                next: None,  // no fallback
                edges: vec![], // no conditional edges
            },
        );
        let terminals = HashMap::new();

        let dag = GateProcessDeclaration {
            name: "test".into(),
            version: "1".into(),
            event_type: "content-publish".into(),
            input_schema: json!({}),
            output_schema: json!({}),
            dag: GateProcessDag { entrypoint: "step-a".to_string(), steps, terminals },
        };

        let interp = build_interpreter_with_continue();
        let err = interp
            .run(&dag, &content_publish_event(), GateContext::new())
            .await
            .unwrap_err();

        assert!(matches!(err, GateError::DagExecution(_)));
    }
}
