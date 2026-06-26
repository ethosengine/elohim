//! Operation-authorization gate — the CORE op-gate logic for Che keyless
//! peer-client governance (Che op-gate Slice 1, §14).
//!
//! # What `authorize_operation` does
//!
//! 1. Looks up the active `delegates-compute` grant for (performer, capability)
//!    in `mishpat_commitments` via [`find_active_delegates_compute`] — scope
//!    filter is pushed into SQL (Review I1: a newer differently-scoped grant
//!    cannot shadow a valid one).
//! 2. Enforces `performer == recipient` with an explicit guard (Review C3 —
//!    this guard lives HERE ONLY; the shared `bounds_validator` is NOT touched,
//!    which would break the live provide loop).  The lookup already filters by
//!    recipient, so this guard is belt-and-suspenders.
//! 3. Constructs an [`EventForValidation`] adapter and delegates to the shared
//!    7-check [`bounds_validator::validate`].
//! 4. Any [`BoundsViolation`] → fail-closed deny.  The result carries its own
//!    `AuthorizeOperationResult { allowed, commitment_cid, reason }` — no variant
//!    is added to `BoundsViolation` (Review C2).
//!
//! # Rate-limit note (Review I6)
//!
//! [`bounds_validator`] check 6 counts `economic_events` with a matching
//! `bounded_by` CID.  This path never emits such events, so the rate window
//! always reads 0 → the rate check is INERT here.  This is correct and expected
//! for Slice 1; Slice 4 will wire the actual event emission.

use crate::db::DbPool;
use crate::services::bounds_validator::{validate, EventForValidation};
use crate::services::commitment_fetcher::ProjectionCommitmentFetcher;
use crate::services::rate_history::DieselRateHistory;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Input for the operation-authorization gate.
///
/// `performer` is the agent CID of the requester (set by the doorway from its
/// verified JWT — never from an `X-Agent-Cid` header).
pub struct AuthorizeOperationRequest {
    /// Agent CID of the node/agent requesting the operation.
    pub performer: String,
    /// The REA action / capability being requested (e.g. `"orchestrate-node"`).
    pub capability: String,
    /// Target EPR identifier, if scoped.  `None` maps to `"*"` (any EPR).
    pub target_epr_id: Option<String>,
    /// Reach level claimed for this operation (must be ≤ `bounds.reach_ceiling`).
    pub reach: String,
}

/// Verdict returned by the operation-authorization gate.
#[derive(Debug, Clone)]
pub struct AuthorizeOperationResult {
    /// `true` = all seven bounds checks passed; `false` = deny.
    pub allowed: bool,
    /// CID of the `delegates-compute` commitment that bounded the verdict
    /// (populated even on deny, so callers can log which commitment was checked).
    pub commitment_cid: Option<String>,
    /// Human-readable reason (diagnostic only; `"ok"` on success).
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Authorize an operation against the active `delegates-compute` grant.
///
/// Returns a verdict (never errors out — all infra failures map to `allowed:false`
/// with a descriptive `reason`, so callers always get a safe deny on breakdown).
///
/// `signed_at` is the ISO-8601 wall-clock timestamp at which the operation is
/// being authorized (supplied by the HTTP handler via `now_iso()`; passed here
/// for testability).
pub async fn authorize_operation(
    pool: &DbPool,
    req: AuthorizeOperationRequest,
    signed_at: String,
) -> AuthorizeOperationResult {
    // 1. Look up the active delegates-compute grant for (performer, capability).
    //    Scope filter pushed into SQL (Review I1).
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => return deny(None, format!("db pool: {e}")),
    };

    let commitment = match crate::db::mishpat_commitments::find_active_delegates_compute(
        &mut conn,
        &req.performer,
        &req.capability,
    ) {
        Ok(Some(c)) => c,
        Ok(None) => {
            return deny(
                None,
                "no active delegates-compute grant for (performer, capability)".into(),
            )
        }
        Err(e) => return deny(None, format!("lookup: {e}")),
    };

    // Release connection before the async bounds-validate (which will acquire its
    // own pool connection via ProjectionCommitmentFetcher and DieselRateHistory).
    drop(conn);

    // 2. performer == recipient guard (Review C3).
    //    Structural: the lookup already filtered `recipient.eq(performer)`.
    //    Explicit guard for defence-in-depth clarity — a future multi-party
    //    query change must not silently widen this.
    if commitment.recipient != req.performer {
        return deny(
            Some(commitment.cid),
            "performer is not the grant recipient".into(),
        );
    }

    // 3. Adapter: map the request + commitment into the EventForValidation
    //    projection that bounds_validator expects.
    let event = EventForValidation {
        action: req.capability.clone(), // check 4 compares event.action == commitment.scope
        performer: req.performer.clone(),
        bounded_by: commitment.cid.clone(),
        target_epr_id: req.target_epr_id.unwrap_or_else(|| "*".into()),
        reach: req.reach.clone(),
        signed_at,
    };

    let fetcher = ProjectionCommitmentFetcher::new(pool.clone());
    // [I6] rate-limit check (check 6) counts economic_events with bounded_by == commitment.cid.
    // This path never emits such events, so the window always reads 0 → always passes.
    // Inert in Slice 1; wire event emission in Slice 4.
    let rate = DieselRateHistory { pool: pool.clone() };

    // 4. Delegate to the shared 7-check validator.  Any BoundsViolation → deny.
    match validate(&event, &fetcher, &rate).await {
        Ok(_) => AuthorizeOperationResult {
            allowed: true,
            commitment_cid: Some(commitment.cid),
            reason: "ok".into(),
        },
        Err(v) => deny(Some(commitment.cid), format!("{v:?}")),
    }
}

// ---------------------------------------------------------------------------
// Internal helper
// ---------------------------------------------------------------------------

fn deny(cid: Option<String>, reason: String) -> AuthorizeOperationResult {
    AuthorizeOperationResult {
        allowed: false,
        commitment_cid: cid,
        reason,
    }
}
