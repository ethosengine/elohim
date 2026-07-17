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

use std::sync::atomic::{AtomicU64, Ordering};
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
///   "provider":"<provider>", "valid_from":"<ISO>", "valid_until":"<ISO>" }
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
    provider: &str,
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
        "provider": provider,
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
    provider: &str,
    head_ref: &str,
    commitment_cid: &str,
    has_point_in_time: &str,
) -> EmitEconomicEventInput {
    EmitEconomicEventInput {
        // `id` stays keyed by `self_cid` — the dedup/idempotency logical key the
        // provide-reconcile loop relies on (changing it would re-author every
        // existing runtime provide row once). Only `provider` (the join column)
        // becomes the agent_cid.
        id: format!("provide:{self_cid}:{head_ref}"),
        action: "replicates-commons".to_string(),
        provider: provider.to_string(),
        receiver: head_ref.to_string(),
        has_point_in_time: has_point_in_time.to_string(),
        commitment_cid: commitment_cid.to_string(),
        content_store_commitment_cid: None,
        target_epr_id: head_ref.to_string(),
        // STAYS "commons" — see fn doc. NOT the content's reach.
        reach: "commons".to_string(),
    }
}

/// Process-wide count of provide-loop author calls skipped because no candidate
/// yielded an `agent_cid` provider. Monotonic for process lifetime (mirrors the
/// `identity_namespace` violation counter idiom). Read by
/// [`provider_unresolved_skip_count`] (unit tests + introspection).
static PROVIDER_UNRESOLVED_SKIPS: AtomicU64 = AtomicU64::new(0);

/// Read the running total of provide authors skipped for an unresolvable
/// `agent_cid` provider. Monotonic; reset only by restart.
pub fn provider_unresolved_skip_count() -> u64 {
    PROVIDER_UNRESOLVED_SKIPS.load(Ordering::Relaxed)
}

/// Record one skipped provide author (no `agent_cid` provider resolvable) and
/// emit a RATE-SANE signal: the process-wide counter + the scrapeable metric
/// bump on every skip, but the WARN fires only on the FIRST occurrence per
/// process (the provide loop retries every tick, so an unresolved-provider node
/// would otherwise WARN-storm). Subsequent skips log at DEBUG. Returns the new
/// running total (for tests).
fn record_provider_unresolved_skip(head_ref: &str) -> u64 {
    let prior = PROVIDER_UNRESOLVED_SKIPS.fetch_add(1, Ordering::Relaxed);
    crate::metrics::inc_provide_provider_unresolved();
    if prior == 0 {
        tracing::warn!(
            target: "identity_coherence",
            counter = "elohim_provide_provider_unresolved_total",
            head_ref = %head_ref,
            "provide author: no agent_cid provider resolvable (session + own cell key both \
             non-agent_cid) — SKIPPING author this tick rather than writing a transport-id \
             provider that could never join the resilience card (rate-sane: first occurrence)"
        );
    } else {
        tracing::debug!(
            target: "identity_coherence",
            counter = "elohim_provide_provider_unresolved_total",
            head_ref = %head_ref,
            skips = prior + 1,
            "provide author: agent_cid provider still unresolvable — skip (throttled)"
        );
    }
    prior + 1
}

/// Resolve the `provider` `agent_cid` (`uhCAk…`) to write for a content provide
/// from an ordered list of candidates, returning the FIRST that is a valid
/// `agent_cid`.
///
/// The resilience-card join is `humans.agent_pub_key = rea_commitments.provider`,
/// which expects an `agent_cid` — a libp2p `self_cid` (`12D3Koo…`) or any other
/// transport id silently never joins (a junk row that can never join is worse
/// than absence). So this NEVER falls back to a non-`agent_cid`. Candidate order
/// (resolved by the caller): (a) the active local session's `agent_pub_key`,
/// (b) the pod's own conductor cell key (`HcClient::agent_key_uhcak`, the same
/// truthful `uhCAk…` source `genesis_self_heal` fills the humans row from).
/// Returns `None` when NO candidate is a valid `agent_cid` — the author then
/// SKIPS this tick. PURE.
fn resolve_provider(candidates: &[Option<&str>]) -> Option<String> {
    candidates
        .iter()
        .flatten()
        .map(|s| s.trim())
        .find(|s| crate::identity_namespace::is_agent_cid(s))
        .map(|s| s.to_string())
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

        // CID enforcement (rung 3): resolve the node's holochain agent_cid
        // (`uhCAk…`) for the `provider` field. The resilience-card join is
        // `humans.agent_pub_key = rea_commitments.provider`, which expects the
        // agent_cid — NOT the libp2p `self_cid` (`12D3Koo…`). Candidate order:
        // (a) the active local session's key (the same one `GET /auth/me`
        // returns), then (b) the pod's OWN conductor cell key
        // (`HcClient::agent_key_uhcak`) — the truthful `uhCAk…` the
        // `genesis_self_heal` bootstrap fills the humans row from. We NEVER fall
        // back to `self_cid`: writing a transport id here mints a `provider` row
        // that can never join the card (worse than absence). If neither candidate
        // is an agent_cid, SKIP authoring this tick (rate-sane signal) so the
        // reconciler retries next tick once a truthful key is available.
        let session_cid = {
            let mut sconn = self.pool.get().map_err(|e| {
                StorageError::Internal(format!("pool checkout for provider resolution: {e}"))
            })?;
            crate::db::local_sessions::get_active_session(&mut sconn)
                .ok()
                .flatten()
                .map(|s| s.agent_pub_key)
        };
        let cell_key = self.hc.agent_key_uhcak();
        let provider = match resolve_provider(&[session_cid.as_deref(), Some(cell_key.as_str())]) {
            Some(p) => p,
            None => {
                // No truthful agent_cid available — skip rather than write a
                // transport-id provider. Returns Err so the reconciler rolls the
                // per-key latch back to NeedsCommitment and retries next tick; the
                // commitment is NOT authored (no conductor write below runs).
                record_provider_unresolved_skip(&req.head_ref);
                return Err(StorageError::Internal(format!(
                    "provide author: no agent_cid provider resolvable for {} \
                     (session + own cell key both non-agent_cid); skipping author this tick",
                    req.head_ref
                )));
            }
        };

        // Regression tripwire: `provider` is now guaranteed an agent_cid, so this
        // stays silent on the correct path — but it would flag (WARN +
        // elohim_identity_namespace_violation_total) any future edit that
        // reintroduced a non-agent_cid provider. NEVER rejects.
        crate::identity_namespace::observe_agent_cid_write(
            "rea_commitments.provider",
            Some(&provider),
        );

        // Step 1 — notarise the replicates-content Commitment. The content's own
        // reach (Stage B) rides the request; `reach_ceiling` stays commons.
        let payload_json = build_content_payload(
            &provider,
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
            &provider,
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
            "12D3KooSelfTransportId",
            "uhCAkProviderAgentCid",
            "epr:household-record",
            "uhCkk-commit-1",
            "2026-06-13T00:00:00Z",
        );
        assert_eq!(
            input.reach, "commons",
            "the ProvideAnnounce event reach must stay commons (schema-8 / bounds gate)"
        );
        assert_eq!(input.action, "replicates-commons");
        assert_eq!(
            input.provider, "uhCAkProviderAgentCid",
            "provider is the resolved agent_cid, NOT the self_cid (the join key)"
        );
        assert!(
            input.id.starts_with("provide:12D3KooSelfTransportId:"),
            "id stays keyed by self_cid for dedup stability"
        );
        assert_eq!(input.receiver, "epr:household-record");
        assert_eq!(input.target_epr_id, "epr:household-record");
        assert_eq!(input.commitment_cid, "uhCkk-commit-1");
        assert!(
            input.content_store_commitment_cid.is_none(),
            "a pure provide carries no content-store commitment (fulfills == [])"
        );
    }

    #[test]
    fn resolve_provider_prefers_session_agent_cid() {
        // (i) A valid uhCAk session key wins — the join-correct provider. The
        // pod's own cell key (also a valid agent_cid) is the second candidate but
        // is not needed when the session already resolves.
        assert_eq!(
            resolve_provider(&[Some("uhCAkSessionAgentCid"), Some("uhCAkCellKey")]).as_deref(),
            Some("uhCAkSessionAgentCid")
        );
    }

    #[test]
    fn resolve_provider_falls_back_to_cell_key_when_no_session() {
        // (ii) No active session (None), but the pod's own conductor cell key is
        // a valid agent_cid — the genesis_self_heal source (b). Resolves to it.
        assert_eq!(
            resolve_provider(&[None, Some("uhCAkCellKey")]).as_deref(),
            Some("uhCAkCellKey")
        );
    }

    #[test]
    fn resolve_provider_skips_non_agent_cid_session_but_takes_cell_key() {
        // A stray transport-id session value is NEVER written; the valid cell key
        // still resolves (candidate order, first agent_cid wins).
        assert_eq!(
            resolve_provider(&[Some("12D3KooStraySession"), Some("uhCAkCellKey")]).as_deref(),
            Some("uhCAkCellKey")
        );
    }

    #[test]
    fn resolve_provider_returns_none_when_nothing_is_agent_cid() {
        // (iii) Neither the session nor the cell key is an agent_cid (only
        // transport ids / empties) → None. The author SKIPS this tick rather than
        // writing a 12D3Koo provider that could never join the resilience card.
        assert_eq!(
            resolve_provider(&[Some("12D3KooSession"), Some("12D3KooCell")]),
            None
        );
        assert_eq!(resolve_provider(&[None, None]), None);
        assert_eq!(resolve_provider(&[Some(""), Some("   ")]), None);
    }

    #[test]
    fn provider_unresolved_skip_records_signal() {
        // (iii cont.) The skip path bumps the process-wide visibility counter (and
        // the scrapeable metric) so an unresolvable-provider node is not silent.
        let before = provider_unresolved_skip_count();
        record_provider_unresolved_skip("epr:skip-1");
        record_provider_unresolved_skip("epr:skip-2");
        assert_eq!(provider_unresolved_skip_count(), before + 2);
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
