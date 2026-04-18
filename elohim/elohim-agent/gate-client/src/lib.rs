//! Gate Client — Wisdom-as-System-Auth for the Elohim Protocol.
//!
//! This crate is the cross-cutting library every relational-impact write path
//! in the protocol calls to ensure wisdom wraps every creation-event.
//!
//! # Architecture
//!
//! The gate is **not a service endpoint**. It is a **protocol invariant**
//! implemented as a library that every network-impacting call site depends on.
//! The elohim-agent-service hosts the wisdom engine; the gate callers are
//! distributed across:
//!
//! - Zome coordinator functions (before DHT commit)
//! - Doorway HTTP POST handlers (via tower::Layer)
//! - libp2p custom-protocol senders
//! - Sync triggers projecting private state to peers
//! - Elohim-agent capability invocations
//! - Advice-seeking flows from users to elohim
//!
//! # Phase
//!
//! This crate ships in two phases:
//!
//! - **DevContext (rehearsal):** `wisdom-invoke` steps are mocked to return
//!   `Allow { phase: DevContext }`. Mechanical gates execute real logic. Every
//!   call-site integration is real. This lets the architecture breathe in its
//!   intended shape before real elohim are live.
//! - **ElohimActive:** `wisdom-invoke` calls a running elohim-agent-service.
//!   Activating is a configuration flip, not a rewrite.
//!
//! See `elohim/elohim-agent/spec/2026-04-18-gate-interface.md` for the full
//! architectural specification.

pub mod dag;
pub mod error;
pub mod events;
pub mod phase;
pub mod space;
pub mod transport;
pub mod types;

#[cfg(test)]
pub mod testing;

// Public re-exports for ergonomic use.
pub use error::{GateError, GateResult};
pub use events::RelationalImpactEvent;
pub use phase::Phase;
pub use space::{SpaceContext, SpaceType};
pub use transport::{GateClientConfig, Transport};
pub use types::{
    DeclineGrounds, EscalationTarget, GateDecision, GateStatus, GateTag, Severity, SideEffect,
};

/// Primary entry point for gate checks.
///
/// Called from every relational-impact write path before the side effect is
/// realized. In DevContext phase, returns a mocked `Allow` for boundary-crossing
/// events and `Allow { exempt: true }` for interior events.
pub async fn check(event: RelationalImpactEvent) -> GateResult<GateDecision> {
    let space = space::detect_from_event(&event);

    if space.is_exempt() {
        return Ok(GateDecision::allow_exempt(Phase::DevContext));
    }

    // Phase 0 skeleton: returns a mocked Allow.
    // Phase 1 will wire in context-assembly + DAG interpreter.
    // Phase 2 will wire in the universal-band DAG.
    Ok(GateDecision::allow_mocked(Phase::DevContext))
}

/// Synchronous variant for zome coordinator contexts (Holochain WASM).
///
/// Phase 0 stub. Will be implemented when the Rust HDK/WASM integration lands.
pub fn check_blocking(event: RelationalImpactEvent) -> GateResult<GateDecision> {
    // Bridge to async via a minimal executor once we're in WASM context.
    // Phase 0: return the same mock without actually blocking.
    let space = space::detect_from_event(&event);
    if space.is_exempt() {
        return Ok(GateDecision::allow_exempt(Phase::DevContext));
    }
    Ok(GateDecision::allow_mocked(Phase::DevContext))
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
