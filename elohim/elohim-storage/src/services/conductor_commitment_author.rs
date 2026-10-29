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
//! [`ConductorRevoker`] authors a `revokes-commitment` whose projection
//! (`mishpat_projection::parse_revokes_commitment`) sets `revoked_at` on the
//! target row. Both submitters use it through
//! `provide_reconcile::submit_revocation`: the un-pin route
//! (`http::handle_remove_pin`, targeting the pin's `commitment_cid`) and the
//! reconciler's removed-pin arm. A re-pin after a revocation authors a renewal
//! (`build_content_payload_renewing`), never the revoked entry again.
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
    CommitmentAuthor, CommitmentRevoker, ProvideAuthorRequest, ProvideRenewal,
    ProvideRevokeRequest, RevokeError, RevokePrecondition,
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
/// Carries `bounds.epr_scope: [head_ref]` — the bounds validator's scope check
/// (4b) reads `epr_scope` as an array and refuses an event whose target isn't
/// covered, so a payload without it can notarise but never announce (defect
/// surfaced live 2026-07-26 on the first real authoring). Same bridge
/// convention as `mishpat_projection`'s bounds_json synthesis. Still NO
/// `ratio_attestation` (that belongs to `delegates-compute` /
/// `replicates-dwelling`, not a content re-publish); `closure_rule` optional
/// and omitted here.
pub fn build_content_payload(
    provider: &str,
    head_ref: &str,
    reach: &str,
    valid_from: &str,
    valid_until: &str,
    rate_per_minute: i64,
) -> String {
    build_content_payload_renewing(
        provider,
        head_ref,
        reach,
        valid_from,
        valid_until,
        rate_per_minute,
        None,
    )
}

/// [`build_content_payload`] for a provide that may renew a revoked
/// commitment. With `renewal: None` the bytes are exactly
/// [`build_content_payload`]'s, so an unrenewed commitment keeps its cid.
pub fn build_content_payload_renewing(
    provider: &str,
    head_ref: &str,
    reach: &str,
    valid_from: &str,
    valid_until: &str,
    rate_per_minute: i64,
    renewal: Option<&ProvideRenewal>,
) -> String {
    let mut payload = serde_json::json!({
        "action": "replicates-commons",
        "variant": "content",
        "head_ref": head_ref,
        "reach": reach,
        "bounds": {
            "rate_per_minute": rate_per_minute,
            // reach_ceiling stays commons — the offer bound + hash-neutrality
            // invariant; the content's own `reach` (above) is what generalizes.
            "reach_ceiling": "commons",
            // Scope the offer to the one EPR it re-publishes; the bounds
            // validator's scope check (4b) requires this array.
            "epr_scope": [head_ref],
        },
        "provider": provider,
        "valid_from": valid_from,
        "valid_until": valid_until,
    });
    // The lineage is added only to a renewal: an unrenewed payload keeps the
    // exact keys (and so the cid) it has always had. The mishpat integrity zome
    // checks only `variant` and `bounds.reach_ceiling` on this action and the
    // coordinator reads named fields without refusing others, so these two keys
    // move no DNA hash.
    if let (Some(renewal), serde_json::Value::Object(fields)) = (renewal, &mut payload) {
        fields.insert("renewal".to_string(), serde_json::json!(renewal.sequence));
        fields.insert(
            "supersedes".to_string(),
            serde_json::json!(renewal.supersedes),
        );
    }
    payload.to_string()
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
/// SKIPS this tick. PURE. Production reads the head of
/// [`agent_provider_candidates`] directly; this is its first-element view, kept
/// for the tests that pin the order.
#[cfg(test)]
fn resolve_provider(candidates: &[Option<&str>]) -> Option<String> {
    agent_provider_candidates(candidates).into_iter().next()
}

/// Every candidate that passes `is_agent_cid`, trimmed, de-duplicated, caller
/// order preserved. [`resolve_provider`] is its first element, so the key the
/// author writes under and the keys the reconciler dedups against come from one
/// rule. PURE.
fn agent_provider_candidates(candidates: &[Option<&str>]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in candidates.iter().flatten().map(|s| s.trim()) {
        if crate::identity_namespace::is_agent_cid(s) && !out.iter().any(|o| o == s) {
            out.push(s.to_string());
        }
    }
    out
}

/// May a client whose mishpat writes are authored by `signer` revoke the row
/// `req` names? Only when the row's provider IS that signer: a revocation is
/// the provider withdrawing its own offer, and this node signs as no one else.
fn authorize_revoke(signer: Option<&str>, req: &ProvideRevokeRequest) -> Result<(), String> {
    match signer {
        Some(signer)
            if crate::identity_namespace::is_agent_cid(signer) && signer == req.provider =>
        {
            Ok(())
        }
        _ => Err(format!(
            "commitment {} is provided by {}, not by this conductor's mishpat cell agent; \
             refusing to revoke it",
            req.target_cid, req.provider
        )),
    }
}

/// Production [`CommitmentRevoker`]: a `revokes-commitment` through the local
/// conductor's mishpat cell, signed by that cell's agent key. The HTTP un-pin
/// route uses it directly; [`ConductorCommitmentAuthor`] delegates to it.
pub struct ConductorRevoker {
    hc: Arc<HcClient>,
}

impl ConductorRevoker {
    pub fn new(hc: Arc<HcClient>) -> Self {
        Self { hc }
    }
}

#[async_trait]
impl CommitmentRevoker for ConductorRevoker {
    async fn revoke_authority(&self) -> Option<String> {
        self.hc
            .mishpat_agent_key_uhcak()
            .filter(|key| crate::identity_namespace::is_agent_cid(key))
    }

    async fn revoke_commons(
        &self,
        req: &ProvideRevokeRequest,
        precondition: &RevokePrecondition,
    ) -> Result<(), RevokeError> {
        // One client for the check and the call: the key compared here is the
        // key of the cell the call writes on.
        let hc = &self.hc;
        authorize_revoke(hc.mishpat_agent_key_uhcak().as_deref(), req)
            .map_err(RevokeError::Refused)?;
        // The refusal reason, if the precondition refuses: then no call was
        // offered, which the caller must be able to tell from a failed call.
        let refusal: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
        let guarded = || {
            precondition().inspect_err(|reason| {
                if let Ok(mut slot) = refusal.lock() {
                    *slot = Some(reason.clone());
                }
            })
        };
        // `signed_at` comes with the request (fixed by the first claim of this
        // target), so every attempt carries the same entry.
        let payload_json = build_revoke_payload(&req.target_cid, &req.signed_at);
        let outcome = conductor_writes::call_create_commitment_guarded(
            hc,
            CreateMishpatCommitmentInput {
                action: "revokes-commitment".to_string(),
                payload_json,
                signed_at: req.signed_at.clone(),
            },
            &guarded,
        )
        .await;
        match outcome {
            Ok(_) => Ok(()),
            Err(e) => match refusal.lock().ok().and_then(|mut slot| slot.take()) {
                Some(reason) => Err(RevokeError::Refused(reason)),
                None => Err(RevokeError::Failed(e)),
            },
        }
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

impl ConductorCommitmentAuthor {
    /// The agent_cid keys this author may write `provider` under, preferred
    /// first: the active local session's key, then the pod's own cell key.
    /// `author_commons` writes under the first; the reconciler dedups against
    /// all of them.
    fn own_provider_candidates(&self) -> Result<Vec<String>, StorageError> {
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
        Ok(agent_provider_candidates(&[
            session_cid.as_deref(),
            Some(cell_key.as_str()),
        ]))
    }
}

#[async_trait]
impl CommitmentAuthor for ConductorCommitmentAuthor {
    async fn own_providers(&self) -> Vec<String> {
        self.own_provider_candidates().unwrap_or_default()
    }

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
        let provider = match self.own_provider_candidates()?.into_iter().next() {
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
        let payload_json = build_content_payload_renewing(
            &provider,
            &req.head_ref,
            &req.reach,
            &valid_from,
            &valid_until,
            DEFAULT_RATE_PER_MINUTE,
            req.renewal.as_ref(),
        );
        let new_cid = crate::chain_write_gate::as_writer(
            crate::chain_write_gate::WriterKind::Provide,
            conductor_writes::create_commitment_returning_cid(
                &self.hc,
                CreateMishpatCommitmentInput {
                    action: "replicates-commons".to_string(),
                    payload_json,
                    signed_at: valid_from.clone(),
                },
            ),
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
        crate::chain_write_gate::as_writer(
            crate::chain_write_gate::WriterKind::Provide,
            economic_event_emit_service::emit(&emit_input, &self.hc, &fetcher, &rate),
        )
        .await
        .map_err(|e| {
            StorageError::Internal(format!(
                "ProvideAnnounce emit failed for {} (commitment {} notarised): {e}",
                req.head_ref, new_cid
            ))
        })?;

        Ok(new_cid)
    }
}

#[async_trait]
impl CommitmentRevoker for ConductorCommitmentAuthor {
    async fn revoke_authority(&self) -> Option<String> {
        ConductorRevoker::new(self.hc.clone())
            .revoke_authority()
            .await
    }

    async fn revoke_commons(
        &self,
        req: &ProvideRevokeRequest,
        precondition: &RevokePrecondition,
    ) -> Result<(), RevokeError> {
        ConductorRevoker::new(self.hc.clone())
            .revoke_commons(req, precondition)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn revoke_req(provider: &str) -> ProvideRevokeRequest {
        ProvideRevokeRequest {
            target_cid: "uhCEkTarget".to_string(),
            head_ref: "epr:X".to_string(),
            provider: provider.to_string(),
            signed_at: "2026-10-13T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn only_the_signing_cell_agent_may_revoke_its_own_row() {
        assert!(authorize_revoke(Some("uhCAkCell"), &revoke_req("uhCAkCell")).is_ok());
        // A row another agent provides (a session key handed off by a doorway).
        assert!(authorize_revoke(Some("uhCAkCell"), &revoke_req("uhCAkSession")).is_err());
        // No mishpat cell, or a signer that is not an agent key.
        assert!(authorize_revoke(None, &revoke_req("uhCAkCell")).is_err());
        assert!(authorize_revoke(Some("12D3Koo"), &revoke_req("12D3Koo")).is_err());
    }

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

    /// The payload `build_content_payload` produced before renewals existed,
    /// kept verbatim so a change to the live builder cannot silently re-key
    /// every unrenewed commitment.
    fn legacy_content_payload(
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
                "reach_ceiling": "commons",
                "epr_scope": [head_ref],
            },
            "provider": provider,
            "valid_from": valid_from,
            "valid_until": valid_until,
        })
        .to_string()
    }

    #[test]
    fn an_unrenewed_commitment_keeps_the_bytes_it_had_before_renewals() {
        let args = (
            "uhCAkSteward",
            "epr:album-1",
            "household",
            "2026-10-29T00:00:00+00:00",
            "2027-10-29T00:00:00+00:00",
            60,
        );
        let legacy = legacy_content_payload(args.0, args.1, args.2, args.3, args.4, args.5);
        assert_eq!(
            build_content_payload(args.0, args.1, args.2, args.3, args.4, args.5),
            legacy
        );
        assert_eq!(
            build_content_payload_renewing(args.0, args.1, args.2, args.3, args.4, args.5, None),
            legacy,
            "renewal 0 must reproduce today's entry, so nothing re-keys"
        );
    }

    #[test]
    fn a_renewal_is_a_new_entry_that_names_its_predecessor() {
        let same_day = |renewal: Option<&ProvideRenewal>| {
            build_content_payload_renewing(
                "uhCAkSteward",
                "epr:album-1",
                "commons",
                "2026-10-29T00:00:00+00:00",
                "2027-10-29T00:00:00+00:00",
                60,
                renewal,
            )
        };
        let first = ProvideRenewal {
            sequence: 1,
            supersedes: "uhCEkRevokedToday".to_string(),
        };
        let original = same_day(None);
        let renewed = same_day(Some(&first));
        assert_ne!(
            renewed, original,
            "re-pinning on the day of the revocation must not re-author the revoked entry"
        );
        let v: Value = serde_json::from_str(&renewed).expect("json");
        assert_eq!(v["renewal"], 1);
        assert_eq!(v["supersedes"], "uhCEkRevokedToday");
        // Everything the contract already carried is unchanged.
        let o: Value = serde_json::from_str(&original).expect("json");
        for key in [
            "action",
            "variant",
            "head_ref",
            "reach",
            "bounds",
            "provider",
            "valid_from",
            "valid_until",
        ] {
            assert_eq!(v[key], o[key], "{key} unchanged by a renewal");
        }
        // A second renewal of the same scope is a third distinct entry.
        let second = ProvideRenewal {
            sequence: 2,
            supersedes: "uhCEkRenewedThenRevoked".to_string(),
        };
        assert_ne!(same_day(Some(&second)), renewed);
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
    fn agent_provider_candidates_filters_dedups_and_keeps_order() {
        let got = agent_provider_candidates(&[
            Some("12D3KooTransport"),
            Some(" uhCAkSession "),
            None,
            Some("uhCAkCell"),
            Some("uhCAkSession"),
        ]);
        assert_eq!(
            got,
            vec!["uhCAkSession".to_string(), "uhCAkCell".to_string()]
        );
        // author_commons writes under the first candidate (resolve_provider).
        assert_eq!(
            resolve_provider(&[
                Some("12D3KooTransport"),
                Some(" uhCAkSession "),
                Some("uhCAkCell")
            ]),
            got.first().cloned()
        );
        assert!(agent_provider_candidates(&[None, Some("12D3KooX")]).is_empty());
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
