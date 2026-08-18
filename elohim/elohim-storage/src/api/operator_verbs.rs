//! Commitment-gated operator runtime verbs (habit `operator-runtime-surface`, slice 1).
//!
//! `POST /api/v1/operator/reconcile` — an operator holding an active
//! `delegates-compute` grant with `scope="operator-reconcile"` asks THIS peer to
//! run a reconciliation sweep now. The peer authorizes the request itself
//! (fail-closed, via the shared [`authorize_operation`] gate), kicks the
//! existing projection-reconcile loop (never a second reconcile
//! implementation), and answers with an attestation naming the commitment CID
//! it acted under — the auditable half of the verb.
//!
//! ## Why this route IS in `build_manifest()`
//!
//! Unlike `POST /api/v1/authorize-operation` (a verdict oracle, deliberately
//! off-manifest), this verb is the public face of the operator surface: the
//! whole point is that an operator reaches a peer *through the doorway* with a
//! commitment instead of kubectl. It is safe to expose because it is
//! fail-closed (no verified performer → 403; no active grant → 403) and
//! bounded (the kick coalesces into the single-flight sweep — a flood of
//! accepted kicks schedules at most one extra sweep).
//!
//! ## Performer trust boundary
//!
//! `performer` is read from the `x-elohim-verified-performer` header, which the
//! doorway STRIPS from inbound requests and re-injects from its verified JWT
//! before forwarding (same trust class as the doorway's op-gate performer —
//! never the spoofable `X-Agent-Cid`). A direct in-cluster caller can set the
//! header itself; that is the existing node-local admin posture ("operator-only
//! is an ingress property") and is exactly the Tauri/localhost self-operator
//! path. The commitment gate still runs for every caller.
//!
//! ## Attestation (slice 1 = response attestation)
//!
//! The 200 body names the commitment CID the peer acted under. Slice 2 emits
//! the corresponding REA `EconomicEvent` (`bounded_by` = the grant CID), which
//! also makes `bounds_validator`'s rate check live for operator verbs.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Request, Response};
use serde::Serialize;

use crate::db::DbPool;
use crate::error::StorageError;
use crate::services::operation_authorization::{authorize_operation, AuthorizeOperationRequest};
use crate::services::response;

/// Header carrying the doorway-verified performer agent CID on the internal hop.
pub const VERIFIED_PERFORMER_HEADER: &str = "x-elohim-verified-performer";

/// The `delegates-compute` scope string that authorizes this verb.
pub const OPERATOR_RECONCILE_CAPABILITY: &str = "operator-reconcile";

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

/// Response body for an accepted operator verb — the peer's attestation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorVerbAttestation {
    pub verb: &'static str,
    pub accepted: bool,
    /// CID of the `delegates-compute` commitment the peer acted under.
    pub commitment_cid: String,
    /// `reconcile-scheduled` | `reconcile-already-pending`.
    pub outcome: &'static str,
    /// This peer's self CID (who is attesting).
    pub attested_by: String,
    /// ISO-8601 wall-clock of the authorization decision.
    pub requested_at: String,
}

/// Response body for a refused operator verb.
///
/// `reason` is the internals-free violation vocabulary from
/// `operation_authorization` (Review C5: commitment internals stay
/// server-side); the commitment CID is deliberately NOT echoed on deny.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorVerbRefusal {
    pub verb: &'static str,
    pub accepted: bool,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Kick plumbing (pure, unit-tested)
// ---------------------------------------------------------------------------

/// Outcome of attempting to wake the projection-reconcile loop.
#[derive(Debug, PartialEq, Eq)]
pub enum KickOutcome {
    /// The kick was queued — the loop runs a sweep on its next wake.
    Scheduled,
    /// A kick is already queued (capacity-1 channel) — idempotent accept:
    /// the pending sweep covers this request too.
    AlreadyPending,
    /// The reconcile loop is not running (PROJECTION_RECONCILE_SECS=0, no P2P
    /// handle, or no DB pool) — honest 503, never a silent no-op.
    Unavailable,
}

/// Map a `try_send` on the capacity-1 kick channel to a [`KickOutcome`].
///
/// Bounded by construction (C6a): at most one kick is ever queued, so an
/// authorized flood coalesces into a single extra sweep. Idempotent (C6b):
/// kicking a loop that already has a pending kick is an accept, not an error.
pub fn kick_reconcile(kick: Option<&tokio::sync::mpsc::Sender<()>>) -> KickOutcome {
    use tokio::sync::mpsc::error::TrySendError;
    match kick {
        None => KickOutcome::Unavailable,
        Some(tx) => match tx.try_send(()) {
            Ok(()) => KickOutcome::Scheduled,
            Err(TrySendError::Full(())) => KickOutcome::AlreadyPending,
            // Receiver dropped ⇒ the reconcile loop never started or exited.
            Err(TrySendError::Closed(())) => KickOutcome::Unavailable,
        },
    }
}

// ---------------------------------------------------------------------------
// Logic layer (directly testable without hyper plumbing)
// ---------------------------------------------------------------------------

/// The three verdict shapes the verb can produce; the HTTP handler maps each
/// to a status + body. Extracted so integration tests exercise the full
/// authorize→kick chain against a seeded pool without a `Request<Incoming>`.
#[derive(Debug)]
pub enum ReconcileVerbOutcome {
    Accepted(OperatorVerbAttestation),
    Refused(OperatorVerbRefusal),
    LoopUnavailable,
}

/// Authorize and execute the reconcile verb for `performer`.
///
/// `performer: None` (no verified-performer header) is an immediate refusal —
/// there is nothing to authorize (fail-closed, C12). The commitment gate runs
/// for every named performer; only an allowed verdict reaches the kick.
pub async fn perform_reconcile_verb(
    pool: &DbPool,
    performer: Option<&str>,
    kick: Option<&tokio::sync::mpsc::Sender<()>>,
    self_cid: &str,
    now_iso: String,
) -> ReconcileVerbOutcome {
    let performer = match performer {
        Some(p) if !p.is_empty() => p,
        _ => {
            return ReconcileVerbOutcome::Refused(OperatorVerbRefusal {
                verb: "reconcile",
                accepted: false,
                reason: "no-verified-performer".into(),
            })
        }
    };

    let auth = authorize_operation(
        pool,
        AuthorizeOperationRequest {
            performer: performer.to_string(),
            capability: OPERATOR_RECONCILE_CAPABILITY.to_string(),
            target_epr_id: None,
            // Same conservative claim as the doorway op-gate: `commons` is the
            // highest rank, hence the strictest ceiling check (deny-safe).
            reach: "commons".to_string(),
        },
        now_iso.clone(),
    )
    .await;

    if !auth.allowed {
        // C8: every deny is observable with its reason; C5: internals-free
        // vocabulary, and the checked commitment CID is not echoed.
        tracing::warn!(
            performer,
            reason = %auth.reason,
            "operator-verb reconcile DENIED"
        );
        return ReconcileVerbOutcome::Refused(OperatorVerbRefusal {
            verb: "reconcile",
            accepted: false,
            reason: auth.reason,
        });
    }

    let commitment_cid = auth
        .commitment_cid
        .unwrap_or_else(|| "unknown-commitment".into());

    let outcome = match kick_reconcile(kick) {
        KickOutcome::Scheduled => "reconcile-scheduled",
        KickOutcome::AlreadyPending => "reconcile-already-pending",
        KickOutcome::Unavailable => {
            // Authorized but the loop is off — honest absence (C4), never a
            // 200 that did nothing.
            tracing::warn!(
                performer,
                %commitment_cid,
                "operator-verb reconcile authorized but reconcile loop unavailable"
            );
            return ReconcileVerbOutcome::LoopUnavailable;
        }
    };

    tracing::info!(
        performer,
        %commitment_cid,
        outcome,
        "operator-verb reconcile ACCEPTED (commitment-gated)"
    );

    ReconcileVerbOutcome::Accepted(OperatorVerbAttestation {
        verb: "reconcile",
        accepted: true,
        commitment_cid,
        outcome,
        attested_by: self_cid.to_string(),
        requested_at: now_iso,
    })
}

// ---------------------------------------------------------------------------
// HTTP handler
// ---------------------------------------------------------------------------

/// `POST /api/v1/operator/reconcile`
pub async fn handle_reconcile(
    req: Request<Incoming>,
    pool: &DbPool,
    kick: Option<&tokio::sync::mpsc::Sender<()>>,
    self_cid: &str,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let performer = req
        .headers()
        .get(VERIFIED_PERFORMER_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let now_iso = chrono::Utc::now().to_rfc3339();

    match perform_reconcile_verb(pool, performer.as_deref(), kick, self_cid, now_iso).await {
        ReconcileVerbOutcome::Accepted(att) => Ok(response::ok(&att)),
        ReconcileVerbOutcome::Refused(refusal) => Ok(response::forbidden(&refusal)),
        ReconcileVerbOutcome::LoopUnavailable => Ok(response::service_unavailable(
            "reconcile loop unavailable on this peer (projection-reconcile disabled)",
        )),
    }
}

// ---------------------------------------------------------------------------
// Unit tests (kick coalescing; the authorize chain is covered in
// tests/operator_verbs.rs against a seeded pool)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn kick_is_bounded_and_idempotent() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
        // First kick schedules.
        assert_eq!(kick_reconcile(Some(&tx)), KickOutcome::Scheduled);
        // Second kick while one is queued coalesces (accept, no growth).
        assert_eq!(kick_reconcile(Some(&tx)), KickOutcome::AlreadyPending);
        // The loop drains one kick → next kick schedules again.
        rx.recv().await.expect("queued kick");
        assert_eq!(kick_reconcile(Some(&tx)), KickOutcome::Scheduled);
    }

    #[tokio::test]
    async fn kick_reports_unavailable_when_loop_absent() {
        // No sender wired at all (no p2p / no pool arm).
        assert_eq!(kick_reconcile(None), KickOutcome::Unavailable);
        // Receiver dropped (reconcile disabled arm dropped it).
        let (tx, rx) = tokio::sync::mpsc::channel::<()>(1);
        drop(rx);
        assert_eq!(kick_reconcile(Some(&tx)), KickOutcome::Unavailable);
    }
}
