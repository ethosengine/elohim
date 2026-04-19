//! Gate Client — Wisdom-as-System-Auth for the Elohim Protocol.
//!
//! This crate is the cross-cutting library every relational-impact write path
//! in the protocol calls so that wisdom wraps every creation-event.
//!
//! # Why every write path invokes the gate
//!
//! Three principles from the spec drive the architecture of this crate:
//!
//! - **P0 — Capability-Wisdom Coupling.** Capability and wisdom scale together
//!   by construction: no code path gains reach without passing through wisdom.
//! - **P1 — Wisdom-as-System-Auth.** Every creation-event with potential for
//!   relational impact on others passes through the wisdom layer before
//!   execution — like auth middleware, you cannot forget it by accident.
//! - **P1.5 — Privacy, Drafting, Play as Architectural Primitives.** The gate
//!   fires at boundary-crossing, not at every interior action; offline and
//!   private-drafting interiors are architecturally exempt.
//!
//! # Architecture
//!
//! The gate is **not a service endpoint**. It is a **protocol invariant**
//! implemented as a library that every network-impacting call site depends on.
//! The elohim-agent-service hosts the wisdom engine; the gate callers are
//! distributed across:
//!
//! - Zome coordinator functions (before DHT commit)
//! - Doorway HTTP POST handlers (via `tower::Layer`)
//! - libp2p custom-protocol senders
//! - Sync triggers projecting private state to peers
//! - Elohim-agent capability invocations
//! - Advice-seeking flows from users to elohim
//!
//! # Phase
//!
//! This crate ships in two phases (see [`phase::Phase`]):
//!
//! - **DevContext (rehearsal):** `wisdom-invoke` steps are mocked to return
//!   `Allow { phase: DevContext }`. Mechanical gates execute real logic. Every
//!   call-site integration is real. This lets the architecture breathe in its
//!   intended shape before real elohim are live.
//! - **ElohimActive:** `wisdom-invoke` calls a running elohim-agent-service.
//!   Activating is a configuration flip, not a rewrite.
//!
//! # Call-site patterns
//!
//! The same [`check`] / [`check_blocking`] / [`tower_layer`] surface serves
//! three integration shapes. Each is demonstrated below — all three pattern
//! examples are compiled by `cargo test --doc`.
//!
//! - [**Pattern A**](#pattern-a--zome-coordinator-pre-commit) — Zome coordinator
//!   pre-commit check, uses [`check_blocking`] because Holochain zomes run
//!   synchronously in WASM.
//! - [**Pattern B**](#pattern-b--doorway-http-post-handler-towerlayer) — Doorway
//!   HTTP POST handler wraps an Axum router with [`tower_layer`].
//! - [**Pattern C**](#pattern-c--direct-in-process-call) — Direct in-process
//!   call from a libp2p sender, sync trigger, or elohim-agent invocation.
//!
//! ## Pattern A — Zome coordinator (pre-commit)
//!
//! In a Holochain zome coordinator function, the gate fires **before** the
//! entry is committed. [`check_blocking`] is required because zome
//! coordinators run synchronously inside WASM.
//!
//! Real zome code depends on the HDK (not available in doctests), so this
//! example is marked `no_run` but shows the exact shape of the call site.
//! See spec §1.4 Pattern A.
//!
//! ```rust,no_run
//! use gate_client::{check_blocking, GateDecision, GateStatus, RelationalImpactEvent};
//!
//! // In real code this is `fn create_content(input) -> ExternResult<ActionHash>`
//! // and uses HDK functions like `agent_info()?.agent_latest_pubkey` and
//! // `create_entry(&input.content)?`. We inline plain strings here so the
//! // example stays self-contained.
//! fn create_content_zome(cid: String, reach: String, author: String)
//!     -> Result<String, String>
//! {
//!     let event = RelationalImpactEvent::ContentPublish {
//!         content_cid: cid,
//!         declared_reach: reach,
//!         author,
//!     };
//!
//!     let decision: GateDecision =
//!         check_blocking(event).map_err(|e| format!("gate error: {e}"))?;
//!
//!     match decision.status {
//!         GateStatus::Allow { .. } => {
//!             // Proceed with `create_entry(&input.content)?` in real zome code.
//!             Ok("action-hash".to_string())
//!         }
//!         GateStatus::Decline { grounds } => {
//!             // In real zome code: `Err(wasm_error!(format!(...)))`.
//!             Err(format!("gate declined: {}", grounds.summary))
//!         }
//!         GateStatus::Escalate { target, .. } => {
//!             // Queue for review, return a stub to the upstream caller.
//!             Err(format!("queued for review by {:?}", target))
//!         }
//!         GateStatus::Verdict(_tag) => {
//!             Err("verdict shape unexpected for ContentPublish in a zome".into())
//!         }
//!     }
//! }
//! ```
//!
//! ## Pattern B — Doorway HTTP POST handler (tower::Layer)
//!
//! Every state-changing HTTP route on doorway opts in with a single
//! `.layer(gate_client::tower_layer())` call. The layer infers a
//! [`RelationalImpactEvent`] from the request path, calls [`check`], and
//! short-circuits with 403 / 202 for Decline / Escalate. See
//! [`tower`] for the full path-inference table and spec §1.4 Pattern B.
//!
//! Marked `no_run` to avoid binding a port, but the router builds and
//! type-checks.
//!
//! ```rust,no_run
//! use axum::{routing::post, Router};
//!
//! async fn create_content() -> &'static str { "ok" }
//! async fn create_attestation() -> &'static str { "ok" }
//! async fn emit_economic_event() -> &'static str { "ok" }
//!
//! # async fn run() {
//! let app: Router = Router::new()
//!     .route("/content", post(create_content))
//!     .route("/attestation", post(create_attestation))
//!     .route("/economic-event", post(emit_economic_event))
//!     .layer(gate_client::tower_layer());
//!
//! // In real code doorway binds the port:
//! let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
//! axum::serve(listener, app).await.unwrap();
//! # }
//! ```
//!
//! ## Pattern C — Direct in-process call
//!
//! Callers outside zomes and tower (libp2p senders, sync triggers, direct
//! elohim-agent invocations) await [`check`] directly and branch on the
//! status. This example actually runs under `cargo test --doc` using the
//! DevContext mock. See spec §1.4 Pattern C.
//!
//! ```rust
//! use gate_client::{check, GateStatus, RelationalImpactEvent};
//!
//! async fn send_peer_message(recipient: String, payload_kind: String) -> Result<(), String> {
//!     let event = RelationalImpactEvent::PeerMessage { recipient, payload_kind };
//!
//!     let decision = check(event).await.map_err(|e| format!("gate error: {e}"))?;
//!
//!     match decision.status {
//!         GateStatus::Allow { .. } => {
//!             // proceed with libp2p send, sync projection, etc.
//!             Ok(())
//!         }
//!         GateStatus::Decline { grounds } => Err(format!("declined: {}", grounds.summary)),
//!         GateStatus::Escalate { .. } => Err("escalated to review".into()),
//!         GateStatus::Verdict(_) => Ok(()), // evaluator-shape gate, proceed
//!     }
//! }
//!
//! // DevContext mock returns Allow — this example runs in `cargo test --doc`.
//! tokio_test::block_on(async {
//!     send_peer_message("agent-peer".to_string(), "handshake".to_string()).await.unwrap();
//! });
//! ```
//!
//! # Further reading
//!
//! See `elohim/elohim-agent/spec/2026-04-18-gate-interface.md` for the full
//! architectural specification.

pub mod dag;
pub mod dag_runner;
pub mod error;
pub mod events;
pub mod phase;
pub mod space;
pub mod tower;
pub mod transport;
pub mod types;

// Re-export attestation builder and constants for downstream use.
pub use dag::attestation::{
    DecisionAttestationBuilder, DecisionAttestationPayload, DEV_CONTEXT_ELOHIM_ID,
    DEV_CONTEXT_SUBSTANCE_CID,
};

#[cfg(any(test, feature = "testing"))]
pub mod testing;

// ─── Test-only decision override ──────────────────────────────────────────────
//
// Allows integration tests to inject a specific GateDecision for one `check()`
// call without building a full mock infrastructure. Scoped to the current thread
// so parallel tests cannot interfere.
//
// Usage:
//   gate_client::__test_set_decision_override(Some(gate_client::testing::mock_decline(...)));
//   // ... drive the request ...
//   gate_client::__test_set_decision_override(None); // restore
#[cfg(any(test, feature = "testing"))]
thread_local! {
    static DECISION_OVERRIDE: std::cell::RefCell<Option<GateDecision>> =
        const { std::cell::RefCell::new(None) };
}

/// Set a one-shot decision override for the next `check()` call on this thread.
/// Only available in `#[cfg(test)]` or `feature = "testing"` builds.
#[cfg(any(test, feature = "testing"))]
pub fn __test_set_decision_override(decision: Option<GateDecision>) {
    DECISION_OVERRIDE.with(|cell| {
        *cell.borrow_mut() = decision;
    });
}

// ─── Test-only per-gate context injection ─────────────────────────────────────
//
// Allows integration tests to pre-populate context fields for a specific gate
// by name.  Consumed once by `check()` → `run_unified()`.
//
// Usage:
//   gate_client::__test_set_gate_ctx("discernment-gate-v1-mechanical", Some(ctx_ext));
//   // ... call check(AttestationWrite) ...
//   // ctx is auto-consumed; call again with None to clear if needed.
//
//   gate_client::__test_set_gate_ctx("reach-gate-v1", Some(reach_ctx));
//   // ... call check(CapabilityInvoke{capability:"reach-negotiation"}) ...
#[cfg(any(test, feature = "testing"))]
thread_local! {
    static GATE_CTXS: std::cell::RefCell<std::collections::HashMap<String, GateContext>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Set a one-shot context extension for the named gate.
///
/// The extension is consumed by the next `check()` call on this thread that
/// routes to that gate.  Pass `None` to clear any pending extension.
///
/// Only available in `#[cfg(test)]` or `feature = "testing"` builds.
#[cfg(any(test, feature = "testing"))]
pub fn __test_set_gate_ctx(gate_name: &str, ctx: Option<GateContext>) {
    GATE_CTXS.with(|cell| {
        let mut map = cell.borrow_mut();
        match ctx {
            Some(c) => {
                map.insert(gate_name.to_string(), c);
            }
            None => {
                map.remove(gate_name);
            }
        }
    });
}

// ─── Test-only per-gate attestation resolver injection ────────────────────────
//
// Allows integration tests to inject a concrete AttestationResolver keyed by
// gate name.  Consumed once by `check()` → `run_unified()` → `run_app_domain_gate`.
//
// Usage:
//   gate_client::__test_set_gate_attestation_resolver(
//       "reach-gate-v1",
//       Some(Arc::new(StubAttestationResolver::single("subj", records))),
//   );
//   // ... call check(CapabilityInvoke{capability:"reach-negotiation"}) ...
//   // resolver is auto-consumed; call with None to clear if needed.
//
// The gate name is one of the keys returned by `domain_gates_for_event`.
#[cfg(any(test, feature = "testing"))]
thread_local! {
    static GATE_ATTESTATION_RESOLVERS: std::cell::RefCell<
        std::collections::HashMap<String, std::sync::Arc<dyn AttestationResolver>>
    > = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Set a one-shot [`AttestationResolver`] for the named gate.
///
/// The resolver is consumed by `run_app_domain_gate` the first time it is
/// called from this thread for that gate.  Pass `None` to remove any pending
/// resolver for that gate.
///
/// The `gate_name` must match one of the names returned by `domain_gates_for_event`
/// (currently only `"reach-gate-v1"` has an `aggregate-attestations` step).
///
/// Only available in `#[cfg(test)]` or `feature = "testing"` builds.
#[cfg(any(test, feature = "testing"))]
pub fn __test_set_gate_attestation_resolver(
    gate_name: &str,
    resolver: Option<std::sync::Arc<dyn AttestationResolver>>,
) {
    GATE_ATTESTATION_RESOLVERS.with(|cell| {
        let mut map = cell.borrow_mut();
        match resolver {
            Some(r) => {
                map.insert(gate_name.to_string(), r);
            }
            None => {
                map.remove(gate_name);
            }
        }
    });
}

/// Set a one-shot [`AttestationResolver`] for the next `check()` call on this
/// thread that invokes the `aggregate-attestations` step.
///
/// # Deprecation
///
/// Prefer [`__test_set_gate_attestation_resolver`]`("reach-gate-v1", resolver)`.
/// This shim delegates to that API and is retained for backward compatibility
/// with existing Phase 5 tests until all callers are migrated.
///
/// Only available in `#[cfg(test)]` or `feature = "testing"` builds.
#[cfg(any(test, feature = "testing"))]
#[deprecated(
    since = "0.6.0",
    note = "Use __test_set_gate_attestation_resolver(\"reach-gate-v1\", resolver) instead"
)]
pub fn __test_set_attestation_resolver(resolver: Option<std::sync::Arc<dyn AttestationResolver>>) {
    __test_set_gate_attestation_resolver("reach-gate-v1", resolver);
}

// ─── Test-only wisdom output injection ────────────────────────────────────────
//
// Allows integration tests to force `WisdomInvokeExecutor` to write a specific
// value for a given (constitution_cid, output_key) pair.  Consumed once by the
// executor after it writes its default mock, so the injected value overwrites
// the mock.  Scoped to the current thread so parallel tests cannot bleed.
//
// Usage:
//   gate_client::__test_set_wisdom_output(
//       "epr:constitutions:content-safety-v1",
//       "safetyOutput",
//       json!({"decision": "decline", ...}),
//   );
//   // ... call check(ContentPublish / PeerMessage / AttestationWrite) ...
//   // Override is consumed once; subsequent calls see the default mock again.
#[cfg(any(test, feature = "testing"))]
thread_local! {
    static WISDOM_OUTPUT_OVERRIDES: std::cell::RefCell<
        std::collections::HashMap<(String, String), serde_json::Value>
    > = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Inject a wisdom output override for a specific `(constitution_cid, output_key)` pair.
///
/// The value is consumed once by `WisdomInvokeExecutor` after it writes its
/// default mock, replacing it.  Pass a new value to reset for the next call.
///
/// Only available in `#[cfg(test)]` or `feature = "testing"` builds.
#[cfg(any(test, feature = "testing"))]
pub fn __test_set_wisdom_output(
    constitution_cid: &str,
    output_key: &str,
    value: serde_json::Value,
) {
    WISDOM_OUTPUT_OVERRIDES.with(|m| {
        m.borrow_mut().insert(
            (constitution_cid.to_string(), output_key.to_string()),
            value,
        );
    });
}

/// Take (consume) a pending wisdom output override for `(constitution_cid, output_key)`.
///
/// Called by `WisdomInvokeExecutor` after writing its default mock.  Returns
/// `Some(value)` if an override was set; `None` otherwise.
///
/// Only available in `#[cfg(test)]` or `feature = "testing"` builds.
#[cfg(any(test, feature = "testing"))]
#[doc(hidden)]
pub fn __test_take_wisdom_output_override(
    constitution_cid: &str,
    output_key: &str,
) -> Option<serde_json::Value> {
    WISDOM_OUTPUT_OVERRIDES.with(|m| {
        m.borrow_mut()
            .remove(&(constitution_cid.to_string(), output_key.to_string()))
    })
}

// Public re-exports for ergonomic use.
pub use dag::aggregation_spec::{AggregationSpec, LadderTier, Reduction, SubjectScope};
pub use dag::content_safety_gate::{
    default_content_safety_gate_declaration, default_content_safety_gate_yaml,
    CONTENT_SAFETY_GATE_V1_CID,
};
pub use dag::context::GateContext;
pub use dag::discernment_gate::{
    default_discernment_gate_declaration, default_discernment_gate_yaml, DISCERNMENT_GATE_V1_CID,
};
pub use dag::executors::{
    AggregateAttestationsExecutor, AttestationRecord, AttestationResolver, CapabilityDispatcher,
    ContentNodeResolver, ContextAssembleExecutor, EmbeddedContentNodeResolver,
    EscalateToReviewExecutor, MechanicalRulesetExecutor, MockCapabilityDispatcher,
    NullAttestationResolver, SkillInvokeExecutor, StubAttestationResolver, SynthesizeExecutor,
    WisdomInvokeExecutor,
};
pub use dag::seven_valence_rules::{
    default_seven_valence_rules, SEVEN_VALENCE_RULES_V1_BODY, SEVEN_VALENCE_RULES_V1_CID,
    SEVEN_VALENCE_RULES_V1_VERSION,
};
pub use dag::universal_band::{
    active_universal_band_pointer, default_universal_band_declaration, default_universal_band_yaml,
    UniversalBandPointer, ACTIVE_UNIVERSAL_BAND_NAME, ACTIVE_UNIVERSAL_BAND_VERSION,
};
pub use dag_runner::{configure_runner, global_runner, AlreadyConfigured, DagRunner};

/// Run the discernment gate directly with a pre-populated context extension.
///
/// Available only in `#[cfg(test)]` or `feature = "testing"` builds.
///
/// This is a testing helper that exposes the `pub(crate)` `dag_runner::run_discernment_gate`
/// to integration tests in `tests/`.  It exists so that Phase 3 per-rule integration
/// tests can continue injecting pre-populated `moment` and `priorAttestations` context
/// fields — which would otherwise require real Phase 5+ `assemble` resolvers.
///
/// **Production callers MUST use `check()`** — the unified public entry point.
/// Phase 5+ will remove this helper once real `assemble` resolvers are wired and
/// context injection can happen transparently inside `check()`.
#[cfg(any(test, feature = "testing"))]
pub async fn run_discernment_gate(
    event: &RelationalImpactEvent,
    gate_context_extension: GateContext,
) -> GateResult<GateDecision> {
    dag_runner::run_discernment_gate(event, gate_context_extension).await
}
pub use error::{GateError, GateResult};
pub use events::RelationalImpactEvent;
pub use phase::Phase;
pub use space::{SpaceContext, SpaceType};
pub use tower::{infer_event_from_path, tower_layer, GateLayer};
pub use transport::{GateClientConfig, Transport};
pub use types::{
    DeclineGrounds, EscalationTarget, GateDecision, GateStatus, GateTag, Severity, SideEffect,
};

/// Primary entry point for gate checks.
///
/// Called from every relational-impact write path before the side effect is
/// realized. In DevContext phase, returns an Allow driven by the universal-band
/// DAG for boundary-crossing events and `Allow { exempt: true }` for interior
/// events.
///
/// # Execution order
///
/// 1. If a `__test_set_decision_override` is set (test builds only), consume and
///    return it — neither DAG runs.
/// 2. If the event's space-type is exempt (offline / private-drafting-interior /
///    play-interior / roleplay-interior), return `Allow { exempt: true }` without
///    running any DAG.
/// 3. Run the universal-band DAG. If it declines or escalates, short-circuit.
/// 4. Run all applicable app-domain gates in declaration order (spec §3.3).
///    Any Decline/Escalate short-circuits the remaining gates.
///    Phase 6 gates: `ContentPublish` / `AttestationWrite` / `PeerMessage` →
///    `content-safety-gate-v1` first; `AttestationWrite` also runs
///    `discernment-gate-v1-mechanical`; `CapabilityInvoke{reach-negotiation}` →
///    `reach-gate-v1`. Events with no domain gates return the universal-band decision.
///
/// See [`dag_runner::domain_gates_for_event`] for the full dispatch rules.
pub async fn check(event: RelationalImpactEvent) -> GateResult<GateDecision> {
    // Test-only: consume a one-shot decision override if one is set.
    #[cfg(any(test, feature = "testing"))]
    {
        let override_decision = DECISION_OVERRIDE.with(|cell| cell.borrow_mut().take());
        if let Some(decision) = override_decision {
            return Ok(decision);
        }
    }

    let space = space::detect_from_event(&event);

    if space.is_exempt() {
        return Ok(GateDecision::allow_exempt(Phase::DevContext));
    }

    // Phase 5: unified dispatch — universal band first, then app-domain gate.
    // Consume all pending per-gate context extensions (test builds only).
    #[cfg(any(test, feature = "testing"))]
    let ctx_exts = GATE_CTXS.with(|cell| {
        let mut map = cell.borrow_mut();
        std::mem::take(&mut *map)
    });
    #[cfg(not(any(test, feature = "testing")))]
    let ctx_exts: std::collections::HashMap<String, GateContext> = std::collections::HashMap::new();

    // Consume all pending per-gate attestation resolver injections (test builds only).
    #[cfg(any(test, feature = "testing"))]
    let attestation_resolver_overrides = GATE_ATTESTATION_RESOLVERS.with(|cell| {
        let mut map = cell.borrow_mut();
        std::mem::take(&mut *map)
    });
    #[cfg(not(any(test, feature = "testing")))]
    let attestation_resolver_overrides: std::collections::HashMap<
        String,
        std::sync::Arc<dyn AttestationResolver>,
    > = std::collections::HashMap::new();

    dag_runner::run_unified(&event, ctx_exts, attestation_resolver_overrides).await
}

/// Synchronous variant for zome coordinator contexts (Holochain WASM).
///
/// Mirrors [`check`] but runs the async DAG on a freshly-created single-use
/// Tokio runtime so it can be called from synchronous WASM contexts.
///
/// Target use case: Holochain zome coordinator functions that run in WASM
/// without an ambient Tokio runtime. This path always spins a fresh
/// current-thread runtime and runs the gate check on it.
///
/// **Do NOT call this from inside an existing Tokio runtime.** It will panic
/// with "Cannot start a runtime from within a runtime." Async callers should
/// use [`check`] with `.await` instead. Phase 3+ may add a `block_in_place`
/// bridge path if a legitimate nested-runtime use case emerges.
///
/// Includes the same Phase 6 unified dispatch as [`check`]: `ContentPublish`,
/// `AttestationWrite`, and `PeerMessage` route through `content-safety-gate-v1`
/// first; `AttestationWrite` also routes through `discernment-gate-v1-mechanical`;
/// `CapabilityInvoke{capability:"reach-negotiation"}` routes through
/// `reach-gate-v1`; all after the universal band allows.
pub fn check_blocking(event: RelationalImpactEvent) -> GateResult<GateDecision> {
    let space = space::detect_from_event(&event);
    if space.is_exempt() {
        return Ok(GateDecision::allow_exempt(Phase::DevContext));
    }

    // Consume all pending per-gate context extensions (test builds only).
    #[cfg(any(test, feature = "testing"))]
    let ctx_exts = GATE_CTXS.with(|cell| {
        let mut map = cell.borrow_mut();
        std::mem::take(&mut *map)
    });
    #[cfg(not(any(test, feature = "testing")))]
    let ctx_exts: std::collections::HashMap<String, GateContext> = std::collections::HashMap::new();

    // Consume all pending per-gate attestation resolver injections (test builds only).
    #[cfg(any(test, feature = "testing"))]
    let attestation_resolver_overrides = GATE_ATTESTATION_RESOLVERS.with(|cell| {
        let mut map = cell.borrow_mut();
        std::mem::take(&mut *map)
    });
    #[cfg(not(any(test, feature = "testing")))]
    let attestation_resolver_overrides: std::collections::HashMap<
        String,
        std::sync::Arc<dyn AttestationResolver>,
    > = std::collections::HashMap::new();

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| GateError::DagExecution(format!("runtime build error: {e}")))?
        .block_on(dag_runner::run_unified(
            &event,
            ctx_exts,
            attestation_resolver_overrides,
        ))
}

/// Configure the gate client — transport, phase override, trust assessor.
pub fn configure(_config: GateClientConfig) {
    // Phase 0 placeholder — configuration storage wired in Phase 1.
}

/// Queue an escalation for steward / qahal / existential review.
///
/// Phase 0 stub. Implementation in Phase 2 alongside escalate-to-review executor.
pub async fn queue_for_review(
    _target: EscalationTarget,
    _context: serde_json::Value,
) -> GateResult<String> {
    Err(GateError::NotYetImplemented(
        "queue_for_review arrives in Phase 2".to_string(),
    ))
}

// ─── Unit tests for check() and check_blocking() ──────────────────────────────

#[cfg(test)]
mod check_tests {
    use super::*;
    use crate::events::RelationalImpactEvent;

    // ─── Helper constructors for the 8 event variants ─────────────────────────

    fn all_events() -> Vec<RelationalImpactEvent> {
        vec![
            RelationalImpactEvent::ContentPublish {
                content_cid: "cid-1".to_string(),
                declared_reach: "public".to_string(),
                author: "agent-a".to_string(),
            },
            RelationalImpactEvent::AttestationWrite {
                subject_hash: "hash-1".to_string(),
                claim_kind: "brit".to_string(),
                issuer: "agent-b".to_string(),
            },
            RelationalImpactEvent::EconomicEventEmit {
                event_kind: "transfer".to_string(),
                provider: "agent-c".to_string(),
                receiver: "agent-d".to_string(),
                quantity: "5".to_string(),
            },
            RelationalImpactEvent::PeerMessage {
                recipient: "agent-e".to_string(),
                payload_kind: "handshake".to_string(),
            },
            RelationalImpactEvent::SyncToPeers {
                manifest_cid: "manifest-cid".to_string(),
                item_count: 3,
            },
            RelationalImpactEvent::AdviceSought {
                requester: "agent-f".to_string(),
                summary_cid: "sum-cid".to_string(),
                topic: "conflict".to_string(),
            },
            RelationalImpactEvent::CapabilityInvoke {
                capability: "translate".to_string(),
                requester: "agent-g".to_string(),
                request_id: "req-1".to_string(),
            },
            RelationalImpactEvent::PrivateToPublicCrossing {
                source_space: "private-draft".to_string(),
                artifact_ref: "artifact-cid".to_string(),
            },
        ]
    }

    // ─── check(): all 8 events return Allow in DevContext ─────────────────────
    //
    // In Phase 0 / DevContext, none of the current enum variants map to an
    // exempt interior space — all 8 go through the non-exempt `allow_mocked`
    // path. This documents the expected DevContext behaviour at the protocol
    // boundary: every relational-impact event is gated (not short-circuited),
    // and the Phase 0 rehearsal always yields Allow.

    #[tokio::test]
    async fn check_all_events_return_allow_in_dev_context() {
        for event in all_events() {
            let kind = event.kind();
            let result = check(event).await;
            assert!(result.is_ok(), "check() returned Err for {kind}");
            let decision = result.unwrap();
            assert!(
                decision.is_allowed(),
                "check() must return Allow for {kind} in DevContext"
            );
        }
    }

    #[tokio::test]
    async fn check_all_events_return_non_exempt_allow_in_dev_context() {
        // Phase 4 note: AttestationWrite now routes through the discernment-gate
        // after the universal band.  Without pre-populated context, the gate falls
        // to rule-7 (steady-state no-mint) and returns Allow{exempt:true}.
        // This is architecturally correct: the discernment gate's terminal-no-mint
        // path deliberately uses Allow{exempt:true} to signal "no story-point minted".
        // All other 7 events are still non-exempt (universal-band-only path).
        for event in all_events() {
            let kind = event.kind();
            let decision = check(event).await.unwrap();
            if kind == "attestation-write" {
                // AttestationWrite → discernment-gate rule-7 → Allow{exempt:true}
                // (no context extension provided — steady-state silence).
                assert!(
                    decision.is_allowed(),
                    "check() must return Allow for {kind} in DevContext; got: {:?}",
                    decision.status
                );
                // is_exempt is true here because discernment-gate terminal-no-mint
                // returns Allow{exempt:true} — this is the expected Phase 4 shape.
            } else {
                assert!(
                    !decision.is_exempt(),
                    "check() must NOT return exempt Allow for {kind}; \
                     non-AttestationWrite variants run universal-band only"
                );
            }
        }
    }

    #[tokio::test]
    async fn check_all_events_return_dev_context_phase() {
        for event in all_events() {
            let kind = event.kind();
            let decision = check(event).await.unwrap();
            assert!(
                decision.phase.is_dev_context(),
                "check() must return DevContext phase for {kind}"
            );
        }
    }

    #[tokio::test]
    async fn check_mocked_reasoning_has_zero_confidence() {
        // ConstitutionalReasoningSummary::mocked() has confidence 0.0, flagging
        // that this decision carries no reputation weight.
        let event = RelationalImpactEvent::ContentPublish {
            content_cid: "cid".to_string(),
            declared_reach: "public".to_string(),
            author: "agent".to_string(),
        };
        let decision = check(event).await.unwrap();
        assert!(
            (decision.reasoning.confidence - 0.0).abs() < f32::EPSILON,
            "mocked reasoning must have confidence 0.0"
        );
        assert_eq!(decision.reasoning.primary_principle, "dev-context-mock");
    }

    // ─── check_blocking(): mirrors check() for all 8 variants ─────────────────

    #[test]
    fn check_blocking_all_events_return_allow_in_dev_context() {
        // Phase 4 note: AttestationWrite routes through discernment-gate rule-7
        // (no context → steady-state no-mint → Allow{exempt:true}).
        // All other 7 events remain non-exempt via the universal-band-only path.
        for event in all_events() {
            let kind = event.kind();
            let result = check_blocking(event);
            assert!(result.is_ok(), "check_blocking() returned Err for {kind}");
            let decision = result.unwrap();
            assert!(
                decision.is_allowed(),
                "check_blocking() must return Allow for {kind} in DevContext"
            );
            if kind != "attestation-write" {
                assert!(
                    !decision.is_exempt(),
                    "check_blocking() must return non-exempt Allow for {kind} \
                     (universal-band-only path)"
                );
            }
        }
    }

    #[test]
    fn check_blocking_returns_dev_context_phase() {
        let event = RelationalImpactEvent::AttestationWrite {
            subject_hash: "h".to_string(),
            claim_kind: "brit".to_string(),
            issuer: "agent".to_string(),
        };
        let decision = check_blocking(event).unwrap();
        assert!(decision.phase.is_dev_context());
    }

    // ─── check(): decision override mechanism ────────────────────────────────

    #[tokio::test]
    async fn check_override_is_consumed_and_returns_injected_decision() {
        // Inject a Decline decision — check() must consume it and return it,
        // not the default Allow.
        __test_set_decision_override(Some(crate::testing::mock_decline(
            "override-category",
            "injected for test",
        )));

        let event = RelationalImpactEvent::ContentPublish {
            content_cid: "cid".to_string(),
            declared_reach: "public".to_string(),
            author: "agent".to_string(),
        };
        let decision = check(event).await.unwrap();
        assert!(
            !decision.is_allowed(),
            "injected Decline must be returned by check()"
        );
        // Confirm it is specifically a Decline with the injected category.
        let json = serde_json::to_string(&decision.status).unwrap();
        assert!(
            json.contains("override-category"),
            "Decline body must carry the injected category, got: {json}"
        );
    }

    #[tokio::test]
    async fn check_override_is_one_shot_and_cleared_after_first_call() {
        // Inject a Decline.
        __test_set_decision_override(Some(crate::testing::mock_decline(
            "one-shot",
            "should only fire once",
        )));

        let event = RelationalImpactEvent::ContentPublish {
            content_cid: "cid".to_string(),
            declared_reach: "public".to_string(),
            author: "agent".to_string(),
        };
        // First call consumes the override.
        let first = check(event.clone()).await.unwrap();
        assert!(
            !first.is_allowed(),
            "first call must return the injected Decline"
        );

        // Second call on the same thread must fall back to the DevContext default Allow.
        let second = check(event).await.unwrap();
        assert!(
            second.is_allowed(),
            "second call must return DevContext Allow (override cleared)"
        );
    }
}

// ─── Unit tests for tower header assertions (need override mechanism) ─────────

#[cfg(test)]
mod tower_header_tests {
    use axum::{body::Body, routing::post, Router};
    use http::{Method, Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn ok_handler() -> axum::response::Response {
        axum::response::Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(r#"{"handled":true}"#))
            .unwrap()
    }

    fn gated_content_router() -> Router {
        Router::new()
            .route("/content", post(ok_handler))
            .layer(crate::tower::GateLayer::new())
    }

    fn gated_attestation_router() -> Router {
        Router::new()
            .route("/attestation", post(ok_handler))
            .layer(crate::tower::GateLayer::new())
    }

    fn gated_crossing_router() -> Router {
        Router::new()
            .route("/crossing", post(ok_handler))
            .layer(crate::tower::GateLayer::new())
    }

    async fn post_to(router: Router, path: &str) -> (StatusCode, http::HeaderMap, String) {
        let req = Request::builder()
            .method(Method::POST)
            .uri(path)
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        let status = resp.status();
        let headers = resp.headers().clone();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            headers,
            String::from_utf8_lossy(&bytes).into_owned(),
        )
    }

    // ─── M-5: Decline produces 403, x-gate-verdict: decline, grounds in body ──

    #[tokio::test]
    async fn decline_produces_403_with_correct_header_and_grounds_in_body() {
        crate::__test_set_decision_override(Some(crate::testing::mock_decline(
            "harmful-content",
            "content violates existential safety principle",
        )));

        let router = gated_content_router();
        let (status, headers, body) = post_to(router, "/content").await;

        assert_eq!(status, StatusCode::FORBIDDEN, "Decline must produce 403");

        let verdict = headers
            .get("x-gate-verdict")
            .expect("x-gate-verdict header must be present");
        assert_eq!(verdict, "decline", "x-gate-verdict must be 'decline'");

        assert!(
            body.contains("harmful-content"),
            "body must contain the grounds category, got: {body}"
        );
        assert!(
            body.contains("\"gate\":\"declined\"") || body.contains("\"gate\": \"declined\""),
            "body must carry gate:declined marker, got: {body}"
        );
    }

    // ─── M-6: Escalate produces 202, x-gate-verdict: escalate, target in body ─

    #[tokio::test]
    async fn escalate_produces_202_with_correct_header_and_target_in_body() {
        use crate::types::{EscalationTarget, Severity};

        crate::__test_set_decision_override(Some(crate::testing::mock_escalate(
            EscalationTarget::AppSteward {
                steward_id: "steward-001".to_string(),
            },
            Severity::High,
        )));

        let router = gated_attestation_router();
        let (status, headers, body) = post_to(router, "/attestation").await;

        assert_eq!(status, StatusCode::ACCEPTED, "Escalate must produce 202");

        let verdict = headers
            .get("x-gate-verdict")
            .expect("x-gate-verdict header must be present");
        assert_eq!(verdict, "escalate", "x-gate-verdict must be 'escalate'");

        // The body JSON must include the target and severity fields.
        assert!(
            body.contains("\"gate\":\"escalated\"") || body.contains("\"gate\": \"escalated\""),
            "body must carry gate:escalated marker, got: {body}"
        );
        assert!(
            body.contains("high") || body.contains("High"),
            "body must include severity, got: {body}"
        );
    }

    // ─── Verdict: passes to inner handler, x-gate-verdict: verdict ───────────

    #[tokio::test]
    async fn verdict_passes_to_inner_handler_with_verdict_header() {
        use crate::types::GateTag;

        crate::__test_set_decision_override(Some(crate::testing::mock_verdict(
            GateTag::StoryPoint {
                valence: "constructive".to_string(),
                magnitude: "medium".to_string(),
                evidence_type: "direct".to_string(),
            },
        )));

        let router = gated_crossing_router();
        let (status, headers, body) = post_to(router, "/crossing").await;

        // Verdict passes through to inner handler — should return 200 from ok_handler.
        assert_eq!(
            status,
            StatusCode::OK,
            "Verdict must pass through to inner handler"
        );

        let verdict = headers
            .get("x-gate-verdict")
            .expect("x-gate-verdict header must be present");
        assert_eq!(verdict, "verdict", "x-gate-verdict must be 'verdict'");

        assert!(
            body.contains("\"handled\":true") || body.contains("\"handled\": true"),
            "inner handler body must be present, got: {body}"
        );
    }

    // ─── Unmapped path with REGISTERED handler: no x-gate-verdict header ─────
    //
    // When a registered handler is on an unmapped path, the gate falls through
    // (Phase 1 limitation). The x-gate-verdict header should NOT be present
    // because the gate never ran.

    #[tokio::test]
    async fn registered_unmapped_path_has_no_gate_verdict_header() {
        // Register a handler on a path the gate does NOT know about.
        let router = Router::new()
            .route("/unmapped-known-handler", post(ok_handler))
            .layer(crate::tower::GateLayer::new());

        let req = Request::builder()
            .method(Method::POST)
            .uri("/unmapped-known-handler")
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK, "handler must respond 200");
        assert!(
            resp.headers().get("x-gate-verdict").is_none(),
            "x-gate-verdict must NOT be present when gate bypassed the path"
        );

        // Confirm the inner handler was actually reached (not some other 200-producing path).
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8_lossy(&bytes).into_owned();
        assert!(
            body.contains("\"handled\":true"),
            "inner handler body must be present, got: {body}"
        );
    }
}
