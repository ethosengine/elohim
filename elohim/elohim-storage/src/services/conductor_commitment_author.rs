//! Production [`CommitmentAuthor`] — the conductor-backed write half of the
//! Slice-2b provide loop.
//!
//! The [`ProvideReconciler`](crate::services::provide_reconcile::ProvideReconciler)
//! diffs desired (caught-up commons pins) against actual (live
//! `replicates-commons` projection rows) and calls this author once per logical
//! key. The unit-tested seam uses a mock; THIS is the real impl that:
//!
//! 1. builds the EXACT `replicates-commons` content payload (schema-valid AND
//!    projection-valid — see [`build_content_payload`]),
//! 2. notarises it via the Mishpat `create_commitment` coordinator
//!    (`conductor_writes::create_commitment_returning_cid` → the new commitment
//!    CID, which becomes the pin's `commitment_cid` back-reference), and
//! 3. emits the bounds-validated ProvideAnnounce EconomicEvent through
//!    `economic_event_emit_service::emit` (a PURE provide — no counterparty
//!    content-store commitment, so `content_store_commitment_cid = None` →
//!    `fulfills == []`; `bounded_by` is the new commitment CID).
//!
//! `revoke_commons` authors a `revokes-commitment` whose projection
//! (`mishpat_projection::parse_revokes_commitment`) sets `revoked_at` on the
//! target row — the stranded-row safety-net arm of the reconciler. The
//! authoritative un-pin revocation lives in `http::handle_remove_pin` (T10),
//! which targets the pin's `commitment_cid` directly.
//!
//! ## Idempotency
//!
//! The reconciler's LOGICAL-KEY dedup `(provider, head_ref)` against the live
//! projection is the author-once guarantee (it survives restart). This author
//! additionally derives `valid_from`/`signed_at` DETERMINISTICALLY from the
//! logical key + a fixed window, so a within-window retry (e.g. the author
//! call succeeded on the conductor but the back-fill of the pin's
//! `commitment_cid` raced) re-sends byte-identical content rather than minting a
//! second commitment with a fresh timestamp.
//!
//! Spec: 2026-06-08-epr-acquisition-slice2b-provide-loop-design.md (provide loop).

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::services::commitment_fetcher::ConductorCommitmentFetcher;
use crate::services::conductor_writes::{self, CreateMishpatCommitmentInput};
use crate::services::economic_event_emit_service::{self, EmitEconomicEventInput};
use crate::services::provide_reconcile::{
    CommitmentAuthor, ProvideAuthorRequest, ProvideRevokeRequest,
};
use crate::services::rate_history::DieselRateHistory;

/// Default rate ceiling for a self-authored commons provide. The provide loop
/// re-publishes commons content the peer already fully holds; a generous
/// per-minute ceiling keeps mutual hosting free/in-kind within reach without
/// inviting abuse (the bounds validator enforces it against the
/// `economic_events` rate history). Must be `>= 1` to satisfy the schema.
pub const DEFAULT_RATE_PER_MINUTE: i64 = 60;

/// Default validity window for a self-authored commons commitment. A generous
/// window (365 days) so the offer outlives ordinary uptime gaps; un-pinning
/// withdraws it early via `revokes-commitment` regardless of `valid_until`.
pub const DEFAULT_VALIDITY_DAYS: i64 = 365;

/// Production [`CommitmentAuthor`]: writes through the local conductor (Mishpat
/// `create_commitment`) and emits the bounds-validated ProvideAnnounce.
pub struct ConductorCommitmentAuthor {
    hc: Arc<HcClient>,
    self_cid: String,
    /// Shared process pool — backs the production `CommitmentFetcher`
    /// (projection-table read) and `RateHistory` the emit step's bounds check
    /// requires.
    pool: crate::db::DbPool,
}

impl ConductorCommitmentAuthor {
    pub fn new(hc: Arc<HcClient>, self_cid: String, pool: crate::db::DbPool) -> Self {
        Self { hc, self_cid, pool }
    }
}

/// Build the EXACT `replicates-commons` content payload (snake_case JSON).
///
/// This is a PURE function so the wire contract is unit-testable without a
/// conductor. The shape is BOTH schema-valid and projection-valid
/// (`mishpat_projection::parse_replicates_commons`):
///
/// ```json
/// { "action":"replicates-commons", "variant":"content", "head_ref":"<cid>",
///   "reach":"commons",
///   "bounds":{ "rate_per_minute":<int>=1>, "reach_ceiling":"commons" },
///   "provider":"<self_cid>", "valid_from":"<ISO>", "valid_until":"<ISO>" }
/// ```
///
/// Deliberately carries NO `epr_scope` and NO `ratio_attestation` (those belong
/// to `delegates-compute` / `replicates-dwelling`, not a commons re-publish).
/// `closure_rule` is optional and omitted here.
pub fn build_content_payload(
    self_cid: &str,
    head_ref: &str,
    valid_from: &str,
    valid_until: &str,
    rate_per_minute: i64,
) -> String {
    serde_json::json!({
        "action": "replicates-commons",
        "variant": "content",
        "head_ref": head_ref,
        "reach": "commons",
        "bounds": {
            "rate_per_minute": rate_per_minute,
            "reach_ceiling": "commons",
        },
        "provider": self_cid,
        "valid_from": valid_from,
        "valid_until": valid_until,
    })
    .to_string()
}

/// Build the `revokes-commitment` payload (snake_case JSON) targeting an
/// existing commitment CID. PURE for unit-testability.
pub fn build_revoke_payload(target_cid: &str, signed_at: &str) -> String {
    serde_json::json!({
        "action": "revokes-commitment",
        "target_cid": target_cid,
        "reason": "provide-withdrawn",
        "signed_at": signed_at,
    })
    .to_string()
}

/// Derive a DETERMINISTIC `valid_from` (== `signed_at`) for a logical provide
/// key. Bucketed to the day so a within-window retry of the same `(provider,
/// head_ref)` re-sends byte-identical content rather than minting a duplicate.
/// The logical-key dedup against the live projection is the primary guard; this
/// only keeps a retry idempotent at the byte level.
fn deterministic_valid_from(now: chrono::DateTime<chrono::Utc>) -> String {
    // Truncate to the start of the UTC day.
    let date = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("valid midnight");
    chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(date, chrono::Utc).to_rfc3339()
}

/// `valid_until` = `valid_from` + [`DEFAULT_VALIDITY_DAYS`].
fn validity_until(valid_from_iso: &str) -> Result<String, StorageError> {
    let start = chrono::DateTime::parse_from_rfc3339(valid_from_iso)
        .map_err(|e| StorageError::Internal(format!("bad valid_from: {e}")))?;
    Ok((start + chrono::Duration::days(DEFAULT_VALIDITY_DAYS)).to_rfc3339())
}

#[async_trait]
impl CommitmentAuthor for ConductorCommitmentAuthor {
    async fn author_commons(&self, req: &ProvideAuthorRequest) -> Result<String, StorageError> {
        let now = chrono::Utc::now();
        let valid_from = deterministic_valid_from(now);
        let valid_until = validity_until(&valid_from)?;

        // Step 1 — notarise the replicates-commons Commitment.
        let payload_json = build_content_payload(
            &self.self_cid,
            &req.head_ref,
            &valid_from,
            &valid_until,
            DEFAULT_RATE_PER_MINUTE,
        );
        let new_cid = conductor_writes::create_commitment_returning_cid(
            &self.hc,
            CreateMishpatCommitmentInput {
                action: "replicates-commons".to_string(),
                payload_json,
                signed_at: valid_from.clone(),
            },
        )
        .await?;

        // Step 2 — emit the bounds-validated ProvideAnnounce EconomicEvent.
        //
        // A PURE provide: no counterparty content-store commitment, so
        // `content_store_commitment_cid = None` → `fulfills == []` (no spurious
        // EventFulfillsCommitment link). `bounded_by` is the new Mishpat
        // commitment CID (the `entry_hash` returned above — see
        // `create_commitment_returning_cid`).
        //
        // CRITICAL: this emit's bounds check fetches the commitment we JUST
        // authored. Its projection into `mishpat_commitments` lands
        // ASYNCHRONOUSLY (post-commit signal → subscriber → upsert), so a
        // `ProjectionCommitmentFetcher` (read-side, projection-backed) would race
        // the signal and return `Ok(None)` → CommitmentNotFound → the emit fails
        // and the whole author call rolls back, even though the commitment IS
        // notarised. We therefore use the `ConductorCommitmentFetcher`, which
        // reads the entry directly from the conductor via `get_commitment`
        // (keyed by the `entry_hash` CID) — available the instant
        // `create_commitment` returns. The projection-backed read path remains
        // correct for third-party (read-path) bounds checks elsewhere; only this
        // self-referential, just-authored case needs the conductor read.
        let fetcher = ConductorCommitmentFetcher {
            hc_client: self.hc.clone(),
        };
        let rate = DieselRateHistory {
            pool: self.pool.clone(),
        };
        let emit_input = EmitEconomicEventInput {
            id: format!("provide:{}:{}", self.self_cid, req.head_ref),
            action: "replicates-commons".to_string(),
            provider: self.self_cid.clone(),
            // A ProvideAnnounce has no specific counterparty receiver; the offer
            // is to the commons. The receiver mirrors the commitment recipient
            // (head_ref), consistent with the projection's logical key.
            receiver: req.head_ref.clone(),
            has_point_in_time: now.to_rfc3339(),
            commitment_cid: new_cid.clone(),
            content_store_commitment_cid: None,
            target_epr_id: req.head_ref.clone(),
            reach: "commons".to_string(),
        };
        // A bounds failure on our OWN freshly-authored, in-window commitment is
        // not expected; surface it (and the conductor error path) as a write
        // failure so the reconciler rolls the latch back and retries next tick.
        // The commitment IS notarised regardless — the next tick finds it live
        // in the projection and does NOT re-author (logical-key dedup).
        economic_event_emit_service::emit(&emit_input, &self.hc, &fetcher, &rate)
            .await
            .map_err(|e| {
                StorageError::Internal(format!(
                    "ProvideAnnounce emit failed for {} (commitment {} notarised): {e}",
                    req.head_ref, new_cid
                ))
            })?;

        Ok(new_cid)
    }

    async fn revoke_commons(&self, req: &ProvideRevokeRequest) -> Result<(), StorageError> {
        let signed_at = chrono::Utc::now().to_rfc3339();
        let payload_json = build_revoke_payload(&req.target_cid, &signed_at);
        conductor_writes::call_create_commitment(
            &self.hc,
            CreateMishpatCommitmentInput {
                action: "revokes-commitment".to_string(),
                payload_json,
                signed_at,
            },
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    // -----------------------------------------------------------------------
    // build_content_payload — the wire contract. The conductor round-trip is
    // thin glue verified by compile + the T1 sweettest; the PAYLOAD SHAPE is
    // the load-bearing thing, so it is unit-tested exactly.
    // -----------------------------------------------------------------------

    #[test]
    fn content_payload_matches_schema_and_projection_contract() {
        let json = build_content_payload(
            "agent:self-steward",
            "epr:album-1",
            "2026-06-08T00:00:00+00:00",
            "2027-06-08T00:00:00+00:00",
            60,
        );
        let v: Value = serde_json::from_str(&json).expect("payload is valid JSON");

        // ── Required keys present with exact values ─────────────────────────
        assert_eq!(v["action"], "replicates-commons");
        assert_eq!(v["variant"], "content");
        assert_eq!(v["head_ref"], "epr:album-1");
        assert_eq!(v["reach"], "commons");
        assert_eq!(v["provider"], "agent:self-steward");
        assert_eq!(v["valid_from"], "2026-06-08T00:00:00+00:00");
        assert_eq!(v["valid_until"], "2027-06-08T00:00:00+00:00");

        // bounds object — rate_per_minute >= 1 and reach_ceiling == commons.
        let bounds = v.get("bounds").expect("bounds object present");
        assert_eq!(bounds["rate_per_minute"], 60);
        assert!(
            bounds["rate_per_minute"].as_i64().unwrap() >= 1,
            "rate_per_minute must be >= 1 (schema)"
        );
        assert_eq!(bounds["reach_ceiling"], "commons");

        // ── Forbidden keys absent ───────────────────────────────────────────
        assert!(
            v.get("epr_scope").is_none(),
            "replicates-commons must NOT carry epr_scope"
        );
        assert!(
            v.get("ratio_attestation").is_none(),
            "replicates-commons must NOT carry ratio_attestation"
        );

        // ── Exactly the expected top-level key set (no stray fields) ─────────
        let obj = v.as_object().expect("payload is an object");
        let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "action",
                "bounds",
                "head_ref",
                "provider",
                "reach",
                "valid_from",
                "valid_until",
                "variant",
            ],
            "top-level keys must be exactly the contract set"
        );
    }

    #[test]
    fn content_payload_honors_supplied_rate() {
        let json =
            build_content_payload("p", "h", "2026-06-08T00:00:00Z", "2027-06-08T00:00:00Z", 1);
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["bounds"]["rate_per_minute"], 1);
    }

    #[test]
    fn revoke_payload_targets_cid() {
        let json = build_revoke_payload("uhCkk-target-1", "2026-06-11T09:00:00Z");
        let v: Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(v["action"], "revokes-commitment");
        assert_eq!(v["target_cid"], "uhCkk-target-1");
        assert_eq!(v["signed_at"], "2026-06-11T09:00:00Z");
        assert!(
            v.get("reason").is_some(),
            "revoke payload carries a reason for the audit trail"
        );
    }

    #[test]
    fn deterministic_valid_from_buckets_to_day() {
        let t1 = chrono::DateTime::parse_from_rfc3339("2026-06-08T09:15:30Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let t2 = chrono::DateTime::parse_from_rfc3339("2026-06-08T23:59:59Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        // Two different instants on the same UTC day → identical valid_from, so
        // a within-day retry re-sends byte-identical content.
        assert_eq!(deterministic_valid_from(t1), deterministic_valid_from(t2));
        // Next day differs.
        let t3 = chrono::DateTime::parse_from_rfc3339("2026-06-09T00:00:01Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert_ne!(deterministic_valid_from(t1), deterministic_valid_from(t3));
    }

    #[test]
    fn validity_until_is_default_window_after_valid_from() {
        let vf = deterministic_valid_from(
            chrono::DateTime::parse_from_rfc3339("2026-06-08T12:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        );
        let vu = validity_until(&vf).expect("validity_until");
        let start = chrono::DateTime::parse_from_rfc3339(&vf).unwrap();
        let end = chrono::DateTime::parse_from_rfc3339(&vu).unwrap();
        assert_eq!((end - start).num_days(), DEFAULT_VALIDITY_DAYS);
    }
}
