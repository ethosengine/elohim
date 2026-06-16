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

/// Build the EXACT `replicates-content` content payload (snake_case JSON).
///
/// This is a PURE function so the wire contract is unit-testable without a
/// conductor. The shape is BOTH schema-valid and projection-valid
/// (`mishpat_projection::parse_replicates_commons`):
///
/// ```json
/// { "action":"replicates-commons", "variant":"content", "head_ref":"<cid>",
///   "reach":"<content-reach>",
///   "bounds":{ "rate_per_minute":<int>=1>, "reach_ceiling":"commons" },
///   "provider":"<self_cid>", "valid_from":"<ISO>", "valid_until":"<ISO>" }
/// ```
///
/// Stage B: `reach` is the CONTENT's own reach (threaded from the pin/content
/// row), not pinned to "commons". `reach_ceiling` STAYS "commons" — it bounds
/// the offer and keeps the DNA hash-neutral (the mishpat integrity zome gates
/// only `reach_ceiling`). The action string stays `replicates-commons` (the
/// one-window alias the coordinator + projection both honor alongside the
/// renamed `replicates-content`).
///
/// Deliberately carries NO `epr_scope` and NO `ratio_attestation` (those belong
/// to `delegates-compute` / `replicates-dwelling`, not a content re-publish).
/// `closure_rule` is optional and omitted here.
pub fn build_content_payload(
    self_cid: &str,
    head_ref: &str,
    reach: &str,
    valid_from: &str,
    valid_until: &str,
    rate_per_minute: i64,
) -> String {
    serde_json::json!({
        "action": "replicates-commons",
        "variant": "content",
        "head_ref": head_ref,
        "reach": reach,
        "bounds": {
            "rate_per_minute": rate_per_minute,
            // reach_ceiling stays commons — the offer bound + hash-neutrality
            // invariant; the content's own `reach` (above) is what generalizes.
            "reach_ceiling": "commons",
        },
        "provider": self_cid,
        "valid_from": valid_from,
        "valid_until": valid_until,
    })
    .to_string()
}

/// Build the ProvideAnnounce [`EmitEconomicEventInput`] for a content provide.
///
/// PURE so the load-bearing `reach` invariant is unit-testable without a
/// conductor: the event `reach` is HARD-PINNED to "commons" and does NOT take the
/// content's reach. The EconomicEvent `reach` is bounds-validated by
/// `bounds_validator` check 5 (`reach_rank()`), which REJECTS any value outside
/// the schema-8 vocabulary ("unknown reach value"). A content reach of
/// `household`/`local` (the Stage B case, ~11.5k local rows) would fail that gate
/// and roll back the author. The CONTENT's own reach lives on the COMMITMENT
/// payload (`build_content_payload.reach`, NOT bounds-validated) — that is what
/// the projection reads to scope the `content:<reach>` provide row the snapshot
/// counts. The event only needs to clear `<= reach_ceiling(=commons)`, which
/// "commons" does for every content reach. A future edit that threads the content
/// reach here would silently reintroduce the roll-back; the unit test below is the
/// guard.
///
/// A ProvideAnnounce has no specific counterparty receiver (the offer is to the
/// commons); `receiver`/`target_epr_id` mirror the head_ref, consistent with the
/// projection's logical key. `content_store_commitment_cid = None` → a pure
/// provide (`fulfills == []`); `bounded_by` is the new Mishpat commitment CID.
pub fn build_provide_announce_input(
    self_cid: &str,
    head_ref: &str,
    commitment_cid: &str,
    has_point_in_time: &str,
) -> EmitEconomicEventInput {
    EmitEconomicEventInput {
        id: format!("provide:{self_cid}:{head_ref}"),
        action: "replicates-commons".to_string(),
        provider: self_cid.to_string(),
        receiver: head_ref.to_string(),
        has_point_in_time: has_point_in_time.to_string(),
        commitment_cid: commitment_cid.to_string(),
        content_store_commitment_cid: None,
        target_epr_id: head_ref.to_string(),
        // STAYS "commons" — see fn doc. NOT the content's reach.
        reach: "commons".to_string(),
    }
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

        // CID enforcement (rung 2b, observation-only): the `provider` we are about
        // to write (below + in the ProvideAnnounce event) is this node's `self_cid`,
        // which on the libp2p transport is a `12D3Koo…` transport id — NOT the
        // `uhCAk…` agent_cid the resilience-card joins on `rea_commitments.provider`
        // expect. Surface the namespace mismatch (WARN + the
        // `elohim_identity_namespace_violation_total` counter) so the drift shakes
        // out in metrics; NEVER rejects. See `identity_namespace` module docs +
        // backlog `cid-enforcement-rollout.md`.
        crate::identity_namespace::observe_agent_cid_write(
            "rea_commitments.provider",
            Some(&self.self_cid),
        );

        // Step 1 — notarise the replicates-content Commitment. The content's own
        // reach (Stage B) rides the request; `reach_ceiling` stays commons.
        let payload_json = build_content_payload(
            &self.self_cid,
            &req.head_ref,
            &req.reach,
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
        let emit_input = build_provide_announce_input(
            &self.self_cid,
            &req.head_ref,
            &new_cid,
            &now.to_rfc3339(),
        );
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
            "commons",
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
        let json = build_content_payload(
            "p",
            "h",
            "commons",
            "2026-06-08T00:00:00Z",
            "2027-06-08T00:00:00Z",
            1,
        );
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["bounds"]["rate_per_minute"], 1);
    }

    #[test]
    fn content_payload_threads_non_commons_reach_keeps_commons_ceiling() {
        // Stage B: the content's own reach rides the top-level `reach`; the
        // `reach_ceiling` bound STAYS commons (hash-neutrality + bounds invariant).
        for reach in ["household", "local", "community"] {
            let json = build_content_payload(
                "agent:self",
                "epr:hh-1",
                reach,
                "2026-06-08T00:00:00Z",
                "2027-06-08T00:00:00Z",
                60,
            );
            let v: Value = serde_json::from_str(&json).unwrap();
            assert_eq!(
                v["reach"], reach,
                "top-level reach is the content's own reach"
            );
            assert_eq!(
                v["bounds"]["reach_ceiling"], "commons",
                "reach_ceiling stays commons regardless of content reach"
            );
        }
    }

    #[test]
    fn provide_announce_event_reach_is_always_commons() {
        // THE load-bearing Stage B invariant: the ProvideAnnounce EconomicEvent
        // `reach` is hard-pinned to "commons" regardless of the content's reach.
        // The author's `req.reach` (household/local/…) rides the COMMITMENT
        // payload, NOT the event — because bounds_validator check 5 (reach_rank)
        // rejects any non-schema-8 reach on the event and would roll back the
        // author. This test is the guard against a future edit re-threading the
        // content reach here.
        let input = build_provide_announce_input(
            "agent:self",
            "epr:household-record",
            "uhCkk-commit-1",
            "2026-06-13T00:00:00Z",
        );
        assert_eq!(
            input.reach, "commons",
            "the ProvideAnnounce event reach must stay commons (schema-8 / bounds gate)"
        );
        assert_eq!(input.action, "replicates-commons");
        assert_eq!(input.provider, "agent:self");
        assert_eq!(input.receiver, "epr:household-record");
        assert_eq!(input.target_epr_id, "epr:household-record");
        assert_eq!(input.commitment_cid, "uhCkk-commit-1");
        assert!(
            input.content_store_commitment_cid.is_none(),
            "a pure provide carries no content-store commitment (fulfills == [])"
        );
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
