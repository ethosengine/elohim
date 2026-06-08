//! Bounds validator — the substrate keystone for compute-commitment enforcement.
//!
//! # What `validate` does
//!
//! [`validate`] is a pure async function that accepts an [`EventForValidation`]
//! projection plus two trait objects (a [`CommitmentFetcher`] and a
//! [`RateHistory`]) and walks seven sequential checks:
//!
//! 1. **Commitment found** — fetches the `Mishpat::Commitment` named by
//!    `event.bounded_by`; fails immediately with
//!    [`ViolationKind::CommitmentNotFound`] if the CID resolves to nothing or
//!    if the conductor is unreachable.
//! 2. **Not revoked** — the commitment's `revoked_at` must be `None`.
//! 3. **Active window** — `event.signed_at` must fall within
//!    `[valid_from, valid_until]`.
//! 4. **Scope check** — `event.action` must equal `commitment.scope` AND
//!    `event.target_epr_id` must appear in `bounds.epr_scope` (or `"*"` is
//!    present as a wildcard).
//! 5. **Reach ceiling** — `event.reach` must be ≤ `bounds.reach_ceiling` in
//!    the protocol's 8-level total-order (see [`reach_rank`]).
//! 6. **Rate limit** — the sliding-window count for this commitment in the
//!    past 60 minutes must be strictly less than `bounds.rate_per_hour`.
//! 7. **Key rotation** — the commitment must not be older than
//!    `bounds.rotation_ttl_days`.
//!
//! Checks run in the order above because later checks are more expensive
//! (conductor round-trip, sliding-window query) or logically depend on earlier
//! ones (reach rank is meaningless if the commitment is revoked).
//!
//! # Return type
//!
//! Returns `Result<BoundsChecksView, BoundsViolation>`.  The success path
//! returns the full check trail (all 7 booleans true), enabling diagnostic
//! callers (e.g. `POST /api/v1/diagnostics/validate-bounds`) to report what
//! passed before a violation occurred.
//!
//! # Relationship to per-instance validators
//!
//! Per-instance validators (`republish_epr_validator` from Sprint 1; the
//! `serve_url_projection_validator` and Phase-C instances from Sprints 3 and
//! 5a-e) each handle the schema-specific mapping from their event view to
//! [`EventForValidation`], then delegate the seven checks here.  This means
//! the core enforcement logic lives in exactly one place regardless of which
//! action type is being validated.
//!
//! # Reach-rank total order
//!
//! The [`reach_rank`] helper encodes the 8-level reach vocabulary as a `u8`
//! ordinal: `private(0) < self(1) < intimate(2) < trusted(3) < familiar(4) <
//! community(5) < public(6) < commons(7)`.  A higher rank means wider
//! propagation.  The reach-ceiling check passes when the event's rank is ≤
//! the commitment's ceiling rank.  Unknown reach strings return `None`, which
//! the validator surfaces as a [`ViolationKind::ReachCeilingExceeded`] so
//! unrecognised values are always rejected rather than silently promoted.

use crate::services::commitment_fetcher::{CommitmentFetcher, FetchError};
use crate::services::rate_history::RateHistory;
use elohim_views::bounds::{BoundsChecksView, ViolationKind};

/// Subset of `EconomicEventView` that `bounds_validator` needs.
///
/// Per-instance validators (e.g. `republish_epr_validator`) convert from their
/// schema-specific event view into this projection before calling [`validate`].
/// Keeping the projection narrow means the validator does not grow a dependency
/// on the full event view (which carries many optional fields irrelevant to
/// bounds enforcement).
#[derive(Debug, Clone)]
pub struct EventForValidation {
    /// REA action string, e.g. `"republish-epr"`.
    pub action: String,
    /// Agent CID of the performer.
    pub performer: String,
    /// CID of the `Mishpat::Commitment` that bounds this event.
    pub bounded_by: String,
    /// EPR identifier targeted by this event.
    pub target_epr_id: String,
    /// Reach level claimed by the event (must be ≤ `bounds.reach_ceiling`).
    pub reach: String,
    /// ISO-8601 timestamp at which the event was signed.
    pub signed_at: String,
}

/// Emitted when any of the seven bounds checks fails.
///
/// `checks` records which checks had passed before the failure, giving the
/// caller a partial trail useful for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundsViolation {
    /// Which check failed.
    pub kind: ViolationKind,
    /// CID of the commitment under inspection (populated from `event.bounded_by`
    /// for `CommitmentNotFound` since there is no `CommitmentRecord` to read).
    pub commitment_cid: String,
    /// Human-readable description of the failure.
    pub summary: String,
    /// Snapshot of check flags at the point of failure.
    pub checks: BoundsChecksView,
}

/// Validate `event` against the Commitment it names, using the provided
/// fetcher and rate-history trait objects.
///
/// Returns `Ok(BoundsChecksView)` (all 7 booleans true) when all checks pass,
/// enabling the diagnostic route to report the full check trail.
/// Returns `Err(BoundsViolation)` at the first check that fails, carrying a
/// partial `BoundsChecksView` trail.
pub async fn validate<F: CommitmentFetcher, R: RateHistory>(
    event: &EventForValidation,
    fetcher: &F,
    rate_history: &R,
) -> Result<BoundsChecksView, BoundsViolation> {
    let mut checks = BoundsChecksView::default();

    // 1. Fetch the Commitment -----------------------------------------------
    let commitment = match fetcher.fetch(&event.bounded_by).await {
        Ok(Some(c)) => {
            checks.commitment_found = true;
            c
        }
        Ok(None) => {
            return Err(BoundsViolation {
                kind: ViolationKind::CommitmentNotFound,
                commitment_cid: event.bounded_by.clone(),
                summary: format!("no Commitment found for cid {}", event.bounded_by),
                checks,
            });
        }
        Err(FetchError::ConductorUnreachable(msg)) => {
            return Err(BoundsViolation {
                kind: ViolationKind::CommitmentNotFound,
                commitment_cid: event.bounded_by.clone(),
                summary: format!("conductor unreachable: {msg}"),
                checks,
            });
        }
        Err(FetchError::MalformedRecord(msg)) => {
            return Err(BoundsViolation {
                kind: ViolationKind::CommitmentNotFound,
                commitment_cid: event.bounded_by.clone(),
                summary: format!("malformed record: {msg}"),
                checks,
            });
        }
        Err(FetchError::NotarizedRequired(msg)) => {
            // Un-notarized row: fail-closed — same as not found from the
            // validator's perspective (spec §6.5: a bounds-gate is NEVER
            // cleared on un-notarized provenance).
            return Err(BoundsViolation {
                kind: ViolationKind::CommitmentNotFound,
                commitment_cid: event.bounded_by.clone(),
                summary: format!("commitment not notarized: {msg}"),
                checks,
            });
        }
    };

    // 2. Revocation check (cheap; most likely to short-circuit) --------------
    if commitment.revoked_at.is_some() {
        return Err(BoundsViolation {
            kind: ViolationKind::CommitmentRevoked,
            commitment_cid: commitment.cid.clone(),
            summary: format!(
                "commitment revoked at {}",
                commitment.revoked_at.as_deref().unwrap_or("unknown")
            ),
            checks,
        });
    }
    checks.not_revoked = true;

    // 3. Active-window check -------------------------------------------------
    let now = &event.signed_at;
    if now < &commitment.valid_from || now > &commitment.valid_until {
        return Err(BoundsViolation {
            kind: ViolationKind::CommitmentInactive,
            commitment_cid: commitment.cid.clone(),
            summary: format!(
                "event signed at {now} outside commitment window [{}, {}]",
                commitment.valid_from, commitment.valid_until
            ),
            checks,
        });
    }
    checks.active = true;

    // 4. Scope check ---------------------------------------------------------
    // 4a. event.action must equal commitment.scope
    if event.action != commitment.scope {
        return Err(BoundsViolation {
            kind: ViolationKind::ScopeNotIncluded,
            commitment_cid: commitment.cid.clone(),
            summary: format!(
                "event.action='{}' does not match commitment.scope='{}'",
                event.action, commitment.scope
            ),
            checks,
        });
    }
    // 4b. target_epr_id must appear in bounds.epr_scope, or "*" is present
    let epr_scope = commitment
        .bounds
        .get("epr_scope")
        .and_then(|v| v.as_array())
        .ok_or_else(|| BoundsViolation {
            kind: ViolationKind::ScopeNotIncluded,
            commitment_cid: commitment.cid.clone(),
            summary: "commitment.bounds.epr_scope missing or not an array".into(),
            checks: checks.clone(),
        })?;
    let scope_matches = epr_scope.iter().any(|v| {
        v.as_str()
            .map(|s| s == "*" || s == event.target_epr_id)
            .unwrap_or(false)
    });
    if !scope_matches {
        return Err(BoundsViolation {
            kind: ViolationKind::ScopeNotIncluded,
            commitment_cid: commitment.cid.clone(),
            summary: format!(
                "event.target_epr_id='{}' not in commitment.bounds.epr_scope",
                event.target_epr_id
            ),
            checks,
        });
    }
    checks.scope_includes_event = true;

    // 5. Reach-ceiling check -------------------------------------------------
    let reach_rank_val = reach_rank(&event.reach).ok_or_else(|| BoundsViolation {
        kind: ViolationKind::ReachCeilingExceeded,
        commitment_cid: commitment.cid.clone(),
        summary: format!("unknown reach value '{}'", event.reach),
        checks: checks.clone(),
    })?;
    let ceiling = commitment
        .bounds
        .get("reach_ceiling")
        .and_then(|v| v.as_str())
        .ok_or_else(|| BoundsViolation {
            kind: ViolationKind::ReachCeilingExceeded,
            commitment_cid: commitment.cid.clone(),
            summary: "commitment.bounds.reach_ceiling missing".into(),
            checks: checks.clone(),
        })?;
    let ceiling_rank = reach_rank(ceiling).ok_or_else(|| BoundsViolation {
        kind: ViolationKind::ReachCeilingExceeded,
        commitment_cid: commitment.cid.clone(),
        summary: format!("unknown ceiling value '{ceiling}'"),
        checks: checks.clone(),
    })?;
    if reach_rank_val > ceiling_rank {
        return Err(BoundsViolation {
            kind: ViolationKind::ReachCeilingExceeded,
            commitment_cid: commitment.cid.clone(),
            summary: format!(
                "event.reach='{}' exceeds commitment.reach_ceiling='{ceiling}'",
                event.reach
            ),
            checks,
        });
    }
    checks.reach_ceiling_ok = true;

    // 6. Rate-limit check (sliding window) -----------------------------------
    let rate_per_hour = commitment
        .bounds
        .get("rate_per_hour")
        .and_then(|v| v.as_u64())
        .unwrap_or(u64::MAX);
    let recent = rate_history
        .count_in_window(&commitment.cid, &event.signed_at, 60)
        .await
        .map_err(|e| BoundsViolation {
            kind: ViolationKind::RateLimitExceeded,
            commitment_cid: commitment.cid.clone(),
            summary: format!("rate-history query failed: {e}"),
            checks: checks.clone(),
        })?;
    if (recent as u64) >= rate_per_hour {
        return Err(BoundsViolation {
            kind: ViolationKind::RateLimitExceeded,
            commitment_cid: commitment.cid.clone(),
            summary: format!(
                "recent count {recent} >= rate_per_hour {rate_per_hour} in 60min window"
            ),
            checks,
        });
    }
    checks.rate_within_limit = true;

    // 7. Key-rotation check --------------------------------------------------
    let rotation_ttl_days = commitment
        .bounds
        .get("rotation_ttl_days")
        .and_then(|v| v.as_u64())
        .unwrap_or(u64::MAX);
    let valid_from = chrono::DateTime::parse_from_rfc3339(&commitment.valid_from).map_err(|e| {
        BoundsViolation {
            kind: ViolationKind::KeyRotationStale,
            commitment_cid: commitment.cid.clone(),
            summary: format!("bad valid_from: {e}"),
            checks: checks.clone(),
        }
    })?;
    let signed_at =
        chrono::DateTime::parse_from_rfc3339(&event.signed_at).map_err(|e| BoundsViolation {
            kind: ViolationKind::KeyRotationStale,
            commitment_cid: commitment.cid.clone(),
            summary: format!("bad signed_at: {e}"),
            checks: checks.clone(),
        })?;
    let age_days = (signed_at - valid_from).num_days() as u64;
    if age_days > rotation_ttl_days {
        return Err(BoundsViolation {
            kind: ViolationKind::KeyRotationStale,
            commitment_cid: commitment.cid.clone(),
            summary: format!(
                "commitment age {age_days}d exceeds rotation_ttl_days {rotation_ttl_days}"
            ),
            checks,
        });
    }
    checks.key_rotation_current = true;

    Ok(checks)
}

/// Map a reach vocabulary string to a numeric rank for ceiling comparison.
///
/// The 8 levels form a strict total order from narrowest (`private = 0`) to
/// widest (`commons = 7`).  Returns `None` for any unrecognised string so
/// unknown values are rejected rather than silently accepted or promoted.
fn reach_rank(s: &str) -> Option<u8> {
    match s {
        "private" => Some(0),
        "self" => Some(1),
        "intimate" => Some(2),
        "trusted" => Some(3),
        "familiar" => Some(4),
        "community" => Some(5),
        "public" => Some(6),
        "commons" => Some(7),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::commitment_fetcher::{CommitmentRecord, MockCommitmentFetcher};
    use crate::services::rate_history::MockRateHistory;

    fn sample_active_commitment() -> CommitmentRecord {
        CommitmentRecord {
            cid: "commitment-cid-abc".into(),
            action: "delegates-compute".into(),
            scope: "republish-epr".into(),
            provider: "agent:matthew-steward".into(),
            recipient: "agent:deploy-svc-matthew".into(),
            bounds: serde_json::json!({
                "epr_scope": ["epr:lamad-spa"],
                "reach_ceiling": "commons",
                "rate_per_hour": 30,
                "rotation_ttl_days": 90
            }),
            valid_from: "2026-05-01T00:00:00Z".into(),
            valid_until: "2026-08-01T00:00:00Z".into(),
            revoked_at: None,
        }
    }

    fn sample_event() -> EventForValidation {
        EventForValidation {
            action: "republish-epr".into(),
            performer: "agent:deploy-svc-matthew".into(),
            bounded_by: "commitment-cid-abc".into(),
            target_epr_id: "epr:lamad-spa".into(),
            reach: "commons".into(),
            signed_at: "2026-05-28T12:00:00Z".into(),
        }
    }

    #[tokio::test]
    async fn validate_passes_when_all_checks_satisfied() {
        let fetcher = MockCommitmentFetcher::new();
        fetcher.seed("commitment-cid-abc", sample_active_commitment());
        let rate = MockRateHistory::new(); // empty — count == 0

        let result = validate(&sample_event(), &fetcher, &rate).await;
        assert!(result.is_ok());
        let checks = result.unwrap();
        assert!(checks.commitment_found);
        assert!(checks.not_revoked);
        assert!(checks.active);
        assert!(checks.scope_includes_event);
        assert!(checks.reach_ceiling_ok);
        assert!(checks.rate_within_limit);
        assert!(checks.key_rotation_current);
    }

    #[tokio::test]
    async fn validate_rejects_commitment_not_found() {
        let fetcher = MockCommitmentFetcher::new(); // empty
        let rate = MockRateHistory::new();
        let result = validate(&sample_event(), &fetcher, &rate).await;
        assert!(matches!(
            result,
            Err(BoundsViolation {
                kind: ViolationKind::CommitmentNotFound,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn validate_rejects_revoked_commitment() {
        let mut c = sample_active_commitment();
        c.revoked_at = Some("2026-05-15T00:00:00Z".into());
        let fetcher = MockCommitmentFetcher::new();
        fetcher.seed(&c.cid.clone(), c);
        let rate = MockRateHistory::new();
        let result = validate(&sample_event(), &fetcher, &rate).await;
        assert!(matches!(
            result,
            Err(BoundsViolation {
                kind: ViolationKind::CommitmentRevoked,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn validate_rejects_inactive_window() {
        let mut c = sample_active_commitment();
        c.valid_until = "2026-05-15T00:00:00Z".into(); // event is 2026-05-28
        let fetcher = MockCommitmentFetcher::new();
        fetcher.seed(&c.cid.clone(), c);
        let rate = MockRateHistory::new();
        let result = validate(&sample_event(), &fetcher, &rate).await;
        assert!(matches!(
            result,
            Err(BoundsViolation {
                kind: ViolationKind::CommitmentInactive,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn validate_rejects_out_of_scope_action() {
        let mut c = sample_active_commitment();
        c.scope = "serve-url-projection".into(); // event.action is republish-epr
        let fetcher = MockCommitmentFetcher::new();
        fetcher.seed(&c.cid.clone(), c);
        let rate = MockRateHistory::new();
        let result = validate(&sample_event(), &fetcher, &rate).await;
        assert!(matches!(
            result,
            Err(BoundsViolation {
                kind: ViolationKind::ScopeNotIncluded,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn validate_rejects_reach_ceiling_exceeded() {
        // Plan spec note: the plan used reach="public" with ceiling="commons" which
        // would PASS (public rank 6 <= commons rank 7). Corrected: use ceiling="community"
        // (rank 5) so that event reach="public" (rank 6) genuinely exceeds it.
        let mut e = sample_event();
        e.reach = "public".into(); // rank 6
        let mut c = sample_active_commitment();
        c.bounds = serde_json::json!({
            "epr_scope": ["epr:lamad-spa"],
            "reach_ceiling": "community", // rank 5 — public(6) > community(5) → violation
            "rate_per_hour": 30,
            "rotation_ttl_days": 90
        });
        let fetcher = MockCommitmentFetcher::new();
        fetcher.seed(&c.cid.clone(), c);
        let rate = MockRateHistory::new();
        let result = validate(&e, &fetcher, &rate).await;
        assert!(matches!(
            result,
            Err(BoundsViolation {
                kind: ViolationKind::ReachCeilingExceeded,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn validate_rejects_rate_limit_exceeded() {
        let fetcher = MockCommitmentFetcher::new();
        fetcher.seed("commitment-cid-abc", sample_active_commitment());
        let rate = MockRateHistory::new();
        rate.seed("commitment-cid-abc", "2026-05-28T12:00:00Z", 30); // rate_per_hour limit
        let result = validate(&sample_event(), &fetcher, &rate).await;
        assert!(matches!(
            result,
            Err(BoundsViolation {
                kind: ViolationKind::RateLimitExceeded,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn validate_rejects_key_rotation_stale() {
        let mut c = sample_active_commitment();
        c.valid_from = "2026-01-01T00:00:00Z".into(); // 148 days before signed_at
        c.bounds = serde_json::json!({
            "epr_scope": ["epr:lamad-spa"],
            "reach_ceiling": "commons",
            "rate_per_hour": 30,
            "rotation_ttl_days": 90 // exceeded: 148 > 90
        });
        let fetcher = MockCommitmentFetcher::new();
        fetcher.seed(&c.cid.clone(), c);
        let rate = MockRateHistory::new();
        let result = validate(&sample_event(), &fetcher, &rate).await;
        assert!(matches!(
            result,
            Err(BoundsViolation {
                kind: ViolationKind::KeyRotationStale,
                ..
            })
        ));
    }
}
