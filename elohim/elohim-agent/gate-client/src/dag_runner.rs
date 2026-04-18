//! DagRunner — orchestrates the universal-band DAG for every gate check.
//!
//! This module owns the singleton that `check()` delegates to.  It:
//!
//! 1. Constructs a [`DagInterpreter`] with all four Phase 2 executors registered.
//! 2. Parses the embedded universal-band v1 declaration once at startup.
//! 3. Exposes a `run()` method that accepts a [`RelationalImpactEvent`] plus a
//!    pre-populated initial [`GateContext`] and drives the DAG to completion.
//!
//! # Singleton strategy
//!
//! A `OnceLock<Arc<DagRunner>>` static is used so construction (YAML parse +
//! interpreter setup) happens exactly once per process, with no `Mutex` held on
//! the hot path.  Because `DagInterpreter` and all registered executors are
//! stateless once constructed, `DagRunner` is `Send + Sync`.
//!
//! # ContextAssemble resolvers
//!
//! The universal-band DAG's `authorize` and `assemble-context` steps pull from
//! `source-chain` and `manifest` sources.  In DevContext no real resolver is
//! registered — the `ContextAssembleExecutor` routes unregistered sources to
//! Phase 3+ stubs which emit `Value::Null` with a `tracing::warn!`.  This is
//! intentional: the wisdom step receives null context values but still produces
//! its mock Allow, and the DAG completes successfully.

use std::sync::{Arc, OnceLock};

use serde_json::json;
use tracing::{info_span, Instrument};

use crate::dag::executor::StepKind;
use crate::dag::executors::{
    ContextAssembleExecutor, EscalateToReviewExecutor, SynthesizeExecutor, WisdomInvokeExecutor,
};
use crate::dag::{
    context::GateContext, interpreter::DagInterpreter,
    universal_band::default_universal_band_declaration, GateProcessDeclaration,
};
use crate::error::GateError;
use crate::events::RelationalImpactEvent;
use crate::space;
use crate::types::GateDecision;

// ─── DagRunner ────────────────────────────────────────────────────────────────

/// Orchestrates a single run of the universal-band DAG.
///
/// Constructed once via [`global_runner()`]; reused for every [`run()`] call.
pub struct DagRunner {
    interpreter: DagInterpreter,
    declaration: GateProcessDeclaration,
}

// SAFETY: DagInterpreter contains Arc<dyn StepExecutor> (all Send+Sync) and a
// HashMap.  GateProcessDeclaration is plain-data Serialize/Deserialize.
// Neither holds any interior mutability; both are sound to share across threads.
unsafe impl Send for DagRunner {}
unsafe impl Sync for DagRunner {}

impl DagRunner {
    /// Construct a runner with all four Phase 2 executors registered.
    fn new() -> Self {
        let mut interpreter = DagInterpreter::new();

        // ContextAssembleExecutor — no pre-registered resolvers.
        // Phase 3+ stubs handle elohim-storage / dht / source-chain / manifest
        // sources by returning Value::Null (with tracing::warn).
        interpreter.register(
            StepKind::ContextAssemble,
            Arc::new(ContextAssembleExecutor::new()),
        );
        interpreter.register(
            StepKind::WisdomInvoke,
            Arc::new(WisdomInvokeExecutor::new()),
        );
        interpreter.register(StepKind::Synthesize, Arc::new(SynthesizeExecutor::new()));
        interpreter.register(
            StepKind::EscalateToReview,
            Arc::new(EscalateToReviewExecutor::new()),
        );

        let declaration = default_universal_band_declaration();

        Self {
            interpreter,
            declaration,
        }
    }

    /// Execute the universal-band DAG for the given event and return the
    /// final `GateDecision`.
    ///
    /// `initial_ctx` should already contain event-specific fields (e.g.,
    /// `eventKind`, `spaceType`) pre-populated by the caller.
    pub async fn run(
        &self,
        event: &RelationalImpactEvent,
        initial_ctx: GateContext,
    ) -> Result<GateDecision, GateError> {
        self.interpreter
            .run(&self.declaration, event, initial_ctx)
            .await
    }
}

// ─── Global singleton ─────────────────────────────────────────────────────────

static RUNNER: OnceLock<Arc<DagRunner>> = OnceLock::new();

/// Return the process-global [`DagRunner`], constructing it on first call.
///
/// Construction parses the embedded YAML and registers executors.  All
/// subsequent calls return a clone of the same `Arc` with no blocking.
pub fn global_runner() -> Arc<DagRunner> {
    RUNNER.get_or_init(|| Arc::new(DagRunner::new())).clone()
}

// ─── Initial context builder ──────────────────────────────────────────────────

/// Build the initial [`GateContext`] from a [`RelationalImpactEvent`].
///
/// Pre-populates:
/// - `eventKind` — the kebab-case event discriminator.
/// - `spaceType` — the space type string from [`space::detect_from_event`].
/// - Event-specific primary fields (content_cid, declared_reach, etc.) using
///   consistent camelCase keys matching the YAML's context_keys declarations.
pub fn build_initial_context(event: &RelationalImpactEvent) -> GateContext {
    let mut ctx = GateContext::new();

    // ── Universal fields ──────────────────────────────────────────────────────
    ctx.insert("eventKind", json!(event.kind()));
    let space = space::detect_from_event(event);
    // Serialize SpaceType respecting its serde rename_all = "kebab-case".
    let space_type_json = serde_json::to_value(space.space_type)
        .unwrap_or_else(|_| json!(format!("{:?}", space.space_type)));
    ctx.insert("spaceType", space_type_json);

    // ── Event-specific primary fields ─────────────────────────────────────────
    match event {
        RelationalImpactEvent::ContentPublish {
            content_cid,
            declared_reach,
            author,
        } => {
            ctx.insert("contentCid", json!(content_cid));
            ctx.insert("declaredReach", json!(declared_reach));
            ctx.insert("author", json!(author));
        }
        RelationalImpactEvent::AttestationWrite {
            subject_hash,
            claim_kind,
            issuer,
        } => {
            ctx.insert("subjectHash", json!(subject_hash));
            ctx.insert("claimKind", json!(claim_kind));
            ctx.insert("issuer", json!(issuer));
        }
        RelationalImpactEvent::EconomicEventEmit {
            event_kind,
            provider,
            receiver,
            quantity,
        } => {
            ctx.insert("economicEventKind", json!(event_kind));
            ctx.insert("provider", json!(provider));
            ctx.insert("receiver", json!(receiver));
            ctx.insert("quantity", json!(quantity));
        }
        RelationalImpactEvent::PeerMessage {
            recipient,
            payload_kind,
        } => {
            ctx.insert("recipient", json!(recipient));
            ctx.insert("payloadKind", json!(payload_kind));
        }
        RelationalImpactEvent::SyncToPeers {
            manifest_cid,
            item_count,
        } => {
            ctx.insert("manifestCid", json!(manifest_cid));
            ctx.insert("itemCount", json!(item_count));
        }
        RelationalImpactEvent::AdviceSought {
            requester,
            summary_cid,
            topic,
        } => {
            ctx.insert("requester", json!(requester));
            ctx.insert("summaryCid", json!(summary_cid));
            ctx.insert("topic", json!(topic));
        }
        RelationalImpactEvent::CapabilityInvoke {
            capability,
            requester,
            request_id,
        } => {
            ctx.insert("capability", json!(capability));
            ctx.insert("requester", json!(requester));
            ctx.insert("requestId", json!(request_id));
        }
        RelationalImpactEvent::PrivateToPublicCrossing {
            source_space,
            artifact_ref,
        } => {
            ctx.insert("sourceSpace", json!(source_space));
            ctx.insert("artifactRef", json!(artifact_ref));
        }
    }

    ctx
}

/// Run the universal-band DAG for the given event with tracing spans.
///
/// This is the hot-path entry point called from `check()`.  It:
/// 1. Builds the initial context.
/// 2. Wraps the DAG run in an `info_span` for the gate check as a whole.
/// 3. Each individual DAG step will be traced inside the interpreter.
pub async fn run_with_tracing(event: &RelationalImpactEvent) -> Result<GateDecision, GateError> {
    let runner = global_runner();
    let initial_ctx = build_initial_context(event);

    let span = info_span!(
        "gate_check",
        event_kind = event.kind(),
        band = "universal-band-v1"
    );

    runner.run(event, initial_ctx).instrument(span).await
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::RelationalImpactEvent;
    use crate::types::GateStatus;

    fn content_publish_event() -> RelationalImpactEvent {
        RelationalImpactEvent::ContentPublish {
            content_cid: "bafytest".into(),
            declared_reach: "public".into(),
            author: "agent-test".into(),
        }
    }

    // ─── global_runner() returns the same Arc on repeated calls ───────────────

    #[test]
    fn global_runner_is_a_singleton() {
        let r1 = global_runner();
        let r2 = global_runner();
        // Both Arcs point to the same allocation.
        assert!(
            Arc::ptr_eq(&r1, &r2),
            "global_runner must return the same Arc"
        );
    }

    // ─── build_initial_context populates universal fields ─────────────────────

    #[test]
    fn initial_context_has_event_kind() {
        let event = content_publish_event();
        let ctx = build_initial_context(&event);
        assert_eq!(
            ctx.get("eventKind"),
            Some(&serde_json::json!("content-publish"))
        );
    }

    #[test]
    fn initial_context_has_space_type() {
        let event = content_publish_event();
        let ctx = build_initial_context(&event);
        assert!(
            ctx.get("spaceType").is_some(),
            "spaceType must be in initial context"
        );
    }

    #[test]
    fn initial_context_has_content_cid_for_content_publish() {
        let event = content_publish_event();
        let ctx = build_initial_context(&event);
        assert_eq!(ctx.get("contentCid"), Some(&serde_json::json!("bafytest")));
    }

    // ─── DagRunner::run produces an Allow decision ───────────────────────────

    #[tokio::test]
    async fn dag_runner_run_produces_allow() {
        let runner = DagRunner::new();
        let event = content_publish_event();
        let ctx = build_initial_context(&event);

        let decision = runner.run(&event, ctx).await.unwrap();
        assert!(
            decision.is_allowed(),
            "universal-band must Allow in DevContext"
        );
    }

    #[tokio::test]
    async fn dag_runner_run_decision_is_not_exempt() {
        let runner = DagRunner::new();
        let event = content_publish_event();
        let ctx = build_initial_context(&event);

        let decision = runner.run(&event, ctx).await.unwrap();
        assert!(
            !decision.is_exempt(),
            "DAG decision must not be exempt (only the short-circuit path is exempt)"
        );
    }

    #[tokio::test]
    async fn dag_runner_run_phase_is_dev_context() {
        let runner = DagRunner::new();
        let event = content_publish_event();
        let ctx = build_initial_context(&event);

        let decision = runner.run(&event, ctx).await.unwrap();
        assert!(
            decision.phase.is_dev_context(),
            "DevContext phase must be set on DAG-produced decisions"
        );
    }

    #[tokio::test]
    async fn dag_runner_run_reasoning_phase_note_is_wisdom_sourced() {
        // The allow-with-wisdom builder propagates phase_note = "wisdom-sourced"
        // from the wisdom mock output.  This proves the real DAG ran rather than
        // the Phase 0 stub (which uses ConstitutionalReasoningSummary::mocked()).
        let runner = DagRunner::new();
        let event = content_publish_event();
        let ctx = build_initial_context(&event);

        let decision = runner.run(&event, ctx).await.unwrap();
        assert_eq!(
            decision.reasoning.phase_note, "wisdom-sourced",
            "phase_note must be 'wisdom-sourced' to prove the real DAG ran; \
             got: {:?}",
            decision.reasoning.phase_note
        );
    }

    // ─── run_with_tracing wraps in a span and returns Allow ──────────────────

    #[tokio::test]
    async fn run_with_tracing_returns_allow() {
        let event = content_publish_event();
        let decision = run_with_tracing(&event).await.unwrap();
        assert!(
            matches!(decision.status, GateStatus::Allow { .. }),
            "run_with_tracing must return Allow in DevContext"
        );
    }

    // ─── All 8 event variants produce Allow ──────────────────────────────────

    #[tokio::test]
    async fn all_event_variants_produce_allow_via_dag() {
        let events = vec![
            RelationalImpactEvent::ContentPublish {
                content_cid: "cid-1".into(),
                declared_reach: "public".into(),
                author: "agent-a".into(),
            },
            RelationalImpactEvent::AttestationWrite {
                subject_hash: "hash-1".into(),
                claim_kind: "brit".into(),
                issuer: "agent-b".into(),
            },
            RelationalImpactEvent::EconomicEventEmit {
                event_kind: "transfer".into(),
                provider: "agent-c".into(),
                receiver: "agent-d".into(),
                quantity: "5".into(),
            },
            RelationalImpactEvent::PeerMessage {
                recipient: "agent-e".into(),
                payload_kind: "handshake".into(),
            },
            RelationalImpactEvent::SyncToPeers {
                manifest_cid: "manifest-cid".into(),
                item_count: 3,
            },
            RelationalImpactEvent::AdviceSought {
                requester: "agent-f".into(),
                summary_cid: "sum-cid".into(),
                topic: "conflict".into(),
            },
            RelationalImpactEvent::CapabilityInvoke {
                capability: "translate".into(),
                requester: "agent-g".into(),
                request_id: "req-1".into(),
            },
            RelationalImpactEvent::PrivateToPublicCrossing {
                source_space: "private-draft".into(),
                artifact_ref: "artifact-cid".into(),
            },
        ];

        let runner = DagRunner::new();
        for event in &events {
            let kind = event.kind();
            let ctx = build_initial_context(event);
            let decision = runner
                .run(event, ctx)
                .await
                .unwrap_or_else(|e| panic!("DAG run failed for {kind}: {e}"));
            assert!(decision.is_allowed(), "DAG must Allow {kind} in DevContext");
        }
    }
}
