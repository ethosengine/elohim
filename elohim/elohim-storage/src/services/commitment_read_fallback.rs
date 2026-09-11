//! Notary read-back for `GET /api/v1/commitments/{cid}` — the projection is a
//! cache, the DHT is the truth (story 07 scenario 3, `@concern:hosted-compute-contracted`).
//!
//! ## The gap this closes
//!
//! Story 07's vocabulary is explicit: the notary is "the network's own shared
//! witness: what it records, **any peer holding the network can read back**,
//! without asking the doorway". Measured on the household mesh 2026-09-11, that
//! was false in two distinct ways for a `hosted-cell` `delegates-compute` grant
//! issued by the doorway's pool peer:
//!
//! 1. **Even the authoring peer 404s.** `GET /api/v1/commitments/{id}` served
//!    ONLY the `rea_commitments` projection (the elohim/lamad REA ledger). A
//!    notarized Mishpat `Commitment` lands in `mishpat_commitments`, and the
//!    mishpat→REA mirror deliberately bridges only the `replicates-*` actions
//!    (`mishpat_projection::replication_mirror_for`) — so a `delegates-compute`
//!    grant has no `rea_commitments` row anywhere, by design. matthew (`:8090`)
//!    listed the cid under `GET /api/v1/commitments/facing/rea` and 404'd on
//!    `GET /api/v1/commitments/{cid}` in the same breath.
//! 2. **A non-authoring peer has no projection at all.** `post_commit` signals
//!    are cell-local (they fire only on the AUTHORING cell), so a peer that did
//!    not issue the grant never projects it — even though its own conductor can
//!    read the entry straight off the DHT. Verified live: jessica's conductor
//!    answered `mishpat::get_commitment(cid)` with the full grant while her
//!    storage 404'd on the same cid.
//!
//! ## The shape of the fix (p2p-design-gate, Notarized / Path A)
//!
//! The commitment is Category A: the DHT is truth, SQL is a read-optimised
//! projection. A projection miss is therefore **not** evidence of absence — it
//! is a cache miss, and the cure for a cache miss is to ask the authority once.
//! The read cascade is:
//!
//! 1. `rea_commitments` (unchanged fast path, owned by the caller).
//! 2. `mishpat_commitments` — the notarized ledger this peer already holds.
//! 3. ONE bounded `mishpat::get_commitment(cid)` through **this peer's own
//!    conductor** — never through the doorway, never through another peer's
//!    HTTP surface. On success the row is projected, so the second read is
//!    local (P1: storage as reconciliation controller).
//! 4. 404 only when the peer's own DHT view also answers none.
//!
//! ## Bounds (C6a) and absence-vs-outage (C4)
//!
//! All work that can refuse the call happens BEFORE it: [`classify_cid`] is
//! pure, and an id that cannot be a notarized cid never spends a conductor
//! call. The conductor call is uncancellable, so there is exactly ONE of them
//! per request — no retry loop, no polling, no fan-out to other peers.
//!
//! An unreachable conductor is an OUTAGE (`StorageError::Conductor` → 503), not
//! an answer. Only `Ok(None)` from the conductor — the peer's own DHT view
//! genuinely carrying nothing under that cid — becomes a 404.
//!
//! ## Known gap (named, not silent)
//!
//! A commitment whose payload projects to something other than
//! `CommitmentProjection::Upsert` (`revokes-commitment`, `author-lens`,
//! identity-head) has no `mishpat_commitments` row shape, so this fallback
//! logs and answers `None`. The lifecycle a freshly-fetched row carries is the
//! entry-derived one (`parse_commitment_payload`'s default); the authoritative
//! lifecycle lives in the `CommitmentByState` links off the commitment's own
//! anchor, which this single-call budget does not read. Both are recorded as
//! `gapNote`s on the `commitment_read_fallback` seam-registry row.

use std::sync::Arc;

use elohim_views::MishpatCommitmentView;
use holochain_types::prelude::EntryHash;

use crate::db::DbPool;
use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::mishpat_projection::{parse_commitment_payload, CommitmentProjection};
use crate::services::conductor_writes;
use crate::services::mishpat_commitment_facing::to_view;

/// Upper bound on a commitment id accepted from the path. A base64 `uhCEk…`
/// `EntryHash` is 53 characters; anything materially longer is a malformed
/// request, refused before any work is spent on it.
const MAX_COMMITMENT_ID_LEN: usize = 128;

/// The `uhC` family prefix every base64 HoloHash carries. An id wearing it is
/// CLAIMING to be a notarized hash, so failing to decode is the caller's error
/// (400), not "the notary has nothing" (404).
const HOLO_HASH_PREFIX: &str = "uhC";

/// What an id on `GET /api/v1/commitments/{id}` can be — decided PURELY, before
/// any database or conductor work.
///
/// The route's id space is genuinely mixed: `rea_commitments` ids are slugs
/// (`custody-blob-7cfe3fa4b46a1b05`) AND sometimes entry hashes, while a
/// notarized Mishpat commitment id is always an `EntryHash`. Classifying up
/// front is what keeps a slug lookup from ever spending an uncancellable
/// conductor call, and what keeps a mistyped cid from being reported as a
/// truthful absence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CidVerdict {
    /// A well-formed base64 `EntryHash` — eligible for ONE bounded notary read.
    Notarized,
    /// Claims to be a HoloHash (`uhC…`) but does not decode, or is empty /
    /// over-long. Refused with 400 rather than spending a conductor call.
    Malformed,
    /// An opaque local projection id (a slug). Never a notarized commitment
    /// cid, so no conductor call is spent on it — a miss stays a plain 404.
    LocalId,
}

/// Classify a `{id}` path segment. Pure — no DB, no conductor, no clock.
pub fn classify_cid(id: &str) -> CidVerdict {
    if id.is_empty() || id.len() > MAX_COMMITMENT_ID_LEN {
        return CidVerdict::Malformed;
    }
    if !id.starts_with(HOLO_HASH_PREFIX) {
        return CidVerdict::LocalId;
    }
    match EntryHash::try_from(id) {
        Ok(_) => CidVerdict::Notarized,
        Err(_) => CidVerdict::Malformed,
    }
}

/// Serve a notarized commitment by cid, falling back from this peer's own
/// projection to this peer's own conductor.
///
/// Returns `Ok(None)` ONLY when the peer's own DHT view carries nothing under
/// the cid (or the id could never be one). An unreachable conductor is an
/// `Err`, never a `None` — absence and outage must not collapse into the same
/// answer (C4).
pub async fn read_notarized(
    hc: Option<&Arc<HcClient>>,
    pool: &DbPool,
    cid: &str,
) -> Result<Option<MishpatCommitmentView>, StorageError> {
    let owned = cid.to_string();
    read_notarized_with(pool, cid, || async move {
        let hc = hc.ok_or_else(|| {
            StorageError::Conductor(
                "notary read-back unavailable: this peer has no conductor bridge".to_string(),
            )
        })?;
        conductor_writes::get_commitment(hc, &owned).await
    })
    .await
}

/// The testable core of [`read_notarized`]: everything except WHICH conductor
/// answers. `read` is invoked AT MOST ONCE, and only after both projection legs
/// have missed — the single-call budget is a property of this function, so a
/// test can prove it without a conductor.
pub async fn read_notarized_with<F, Fut>(
    pool: &DbPool,
    cid: &str,
    read: F,
) -> Result<Option<MishpatCommitmentView>, StorageError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<
        Output = Result<Option<conductor_writes::GetCommitmentOutput>, StorageError>,
    >,
{
    let verdict = classify_cid(cid);
    if verdict == CidVerdict::Malformed {
        return Err(StorageError::InvalidInput(format!(
            "commitment id is not a readable identifier: {cid}"
        )));
    }

    // Leg 2 — the notarized ledger this peer already holds. On the authoring
    // peer this is the whole fix; it costs one indexed SQL read.
    {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Database(format!("commitment read pool: {e}")))?;
        if let Some(row) = crate::db::mishpat_commitments::get_by_cid(&mut conn, cid)
            .map_err(|e| StorageError::Database(e.to_string()))?
        {
            return Ok(Some(to_view(row)));
        }
    }

    if verdict == CidVerdict::LocalId {
        // A slug that is in neither projection is simply absent. Spending a
        // conductor call on it would be work with a known-empty answer.
        return Ok(None);
    }

    // Leg 3 — ONE bounded read through THIS peer's own conductor.
    let Some(held) = read().await? else {
        return Ok(None);
    };

    // The conductor must have answered about the cid we asked for. A mismatch
    // is a bug or a hostile answer, never something to cache.
    let entry_hash = held.entry_hash.to_string();
    if entry_hash != cid {
        return Err(StorageError::Internal(format!(
            "notary answered for {entry_hash} when asked for {cid}"
        )));
    }

    let action_hash = held.action_hash.to_string();
    let projected =
        parse_commitment_payload(&held.action, &held.payload_json, &entry_hash, &action_hash)
            .map_err(|e| {
                StorageError::Internal(format!("notarized commitment {cid} does not project: {e}"))
            })?;

    let CommitmentProjection::Upsert(row) = projected else {
        // Named gap, not a silent drop — see the module doc.
        tracing::warn!(
            cid = %cid,
            action = %held.action,
            "notary read-back: commitment action has no mishpat_commitments row shape; \
             answering absent"
        );
        return Ok(None);
    };

    // Leg 3b — project, so the second read is local (P1 reconciliation).
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Database(format!("commitment read pool: {e}")))?;
    let stored = crate::db::mishpat_commitments::upsert_with_anchor(&mut conn, row)
        .map_err(|e| StorageError::Database(e.to_string()))?;
    tracing::info!(
        cid = %cid,
        action = %stored.action,
        "notary read-back: projected a commitment this peer did not author"
    );
    Ok(Some(to_view(stored)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A real `uhCEk…` entry hash, and the matching `uhCkk…` action hash, taken
    /// from the household mesh's `hosted-cell` grant (matthew's pool conductor,
    /// measured 2026-09-11).
    const LIVE_CID: &str = "uhCEk64woTR-RLRs6-rs6qpmqHNP_tD-O7ZQNxMbUBjUOt50NKmst";
    const OTHER_CID: &str = "uhCEkCBTB6oPDEMN4oDlC4m-M79Ngr2TrSzggxlI8J7TR_yt8QNAd";

    /// The exact `payload_json` jessica's own conductor returned for `LIVE_CID`.
    const LIVE_PAYLOAD: &str = concat!(
        r#"{"action":"delegates-compute","scope":"hosted-cell","#,
        r#""provider":"uhCAkwZmnsxA_FMajziYQDbvwWpGX49rxBh-CZyEmMI5Q7_3iB5Fu","#,
        r#""recipient":"uhCAkRB5x3YIURVWFhObwjYWW6zuf6n_iMfhyp3AykeQJCMNyUkl9","#,
        r#""bounds":{"epr_scope":["*"],"reach_ceiling":"commons","rate_per_hour":60,"#,
        r#""rotation_ttl_days":30},"valid_from":"2026-09-11T04:15:24+00:00","#,
        r#""valid_until":"2026-10-11T04:15:24+00:00"}"#
    );

    fn live_grant() -> conductor_writes::GetCommitmentOutput {
        conductor_writes::GetCommitmentOutput {
            action_hash: holochain_types::prelude::ActionHash::from_raw_39(vec![
                132, 41, 36, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]),
            entry_hash: EntryHash::try_from(LIVE_CID).expect("fixture cid decodes"),
            action: "delegates-compute".to_string(),
            payload_json: LIVE_PAYLOAD.to_string(),
            signed_at: "2026-09-11T04:15:24+00:00".to_string(),
        }
    }

    #[test]
    fn a_real_entry_hash_is_eligible_for_the_notary_read() {
        assert_eq!(classify_cid(LIVE_CID), CidVerdict::Notarized);
    }

    #[test]
    fn a_projection_slug_never_spends_a_conductor_call() {
        assert_eq!(
            classify_cid("custody-blob-7cfe3fa4b46a1b05"),
            CidVerdict::LocalId
        );
        assert_eq!(
            classify_cid("replicates-commons-1bdb3d8ab7734df2"),
            CidVerdict::LocalId
        );
    }

    #[test]
    fn a_truncated_cid_is_refused_rather_than_reported_absent() {
        // Wears the HoloHash prefix, so it is a mistyped cid (400), never a
        // truthful "the notary has nothing" (404).
        assert_eq!(classify_cid("uhCEk64woTR-RLRs6"), CidVerdict::Malformed);
    }

    #[test]
    fn empty_and_over_long_ids_are_refused_before_any_work() {
        assert_eq!(classify_cid(""), CidVerdict::Malformed);
        assert_eq!(
            classify_cid(&"u".repeat(MAX_COMMITMENT_ID_LEN + 1)),
            CidVerdict::Malformed
        );
    }

    /// The story-07 case: absent from the projection, present on the DHT — it
    /// is SERVED, and it is PROJECTED so the second read never asks again.
    #[tokio::test]
    async fn a_commitment_only_the_dht_holds_is_served_and_projected() {
        let pool = crate::test_util::test_pool();
        let calls = AtomicUsize::new(0);

        let view = read_notarized_with(&pool, LIVE_CID, || {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Ok(Some(live_grant())) }
        })
        .await
        .expect("notary read succeeds")
        .expect("the DHT holds this commitment");

        assert_eq!(view.cid, LIVE_CID);
        assert_eq!(view.scope, "hosted-cell");
        assert_eq!(view.action, "delegates-compute");
        assert_eq!(
            view.recipient,
            "uhCAkRB5x3YIURVWFhObwjYWW6zuf6n_iMfhyp3AykeQJCMNyUkl9"
        );
        assert_eq!(view.valid_until, "2026-10-11T04:15:24+00:00");
        assert_eq!(calls.load(Ordering::SeqCst), 1, "exactly one notary read");

        // Second read is local: the reader must not be invoked again.
        let again = read_notarized_with(&pool, LIVE_CID, || {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Ok(Some(live_grant())) }
        })
        .await
        .expect("projected read succeeds")
        .expect("the projection now holds it");
        assert_eq!(again.cid, LIVE_CID);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "the projected row is served without a second conductor call"
        );
    }

    /// A cid absent from BOTH the projection and this peer's DHT view is a
    /// truthful 404 — `Ok(None)`, never an error.
    #[tokio::test]
    async fn a_cid_absent_from_both_answers_absent() {
        let pool = crate::test_util::test_pool();
        let answer = read_notarized_with(&pool, OTHER_CID, || async { Ok(None) })
            .await
            .expect("an absent commitment is an answer, not a failure");
        assert!(answer.is_none());
    }

    /// A malformed cid is refused BEFORE the reader is reachable — the
    /// uncancellable conductor call is never spent on it.
    #[tokio::test]
    async fn a_malformed_cid_is_refused_before_the_conductor_call() {
        let pool = crate::test_util::test_pool();
        let calls = AtomicUsize::new(0);
        let err = read_notarized_with(&pool, "uhCEk-not-a-hash", || {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Ok(Some(live_grant())) }
        })
        .await
        .expect_err("a malformed cid is a caller error");
        assert!(matches!(err, StorageError::InvalidInput(_)), "{err:?}");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    /// An outage is not an absence: a conductor that cannot answer must not be
    /// reported as "the notary has nothing" (C4).
    #[tokio::test]
    async fn an_unreachable_conductor_is_an_outage_not_an_absence() {
        let pool = crate::test_util::test_pool();
        let err = read_notarized_with(&pool, LIVE_CID, || async {
            Err(StorageError::Conductor("bridge down".into()))
        })
        .await
        .expect_err("an outage propagates");
        assert!(matches!(err, StorageError::Conductor(_)), "{err:?}");
    }

    /// A conductor answering about a different entry is refused, never cached.
    #[tokio::test]
    async fn an_answer_about_another_cid_is_refused() {
        let pool = crate::test_util::test_pool();
        let err = read_notarized_with(&pool, OTHER_CID, || async { Ok(Some(live_grant())) })
            .await
            .expect_err("a mismatched answer is refused");
        assert!(matches!(err, StorageError::Internal(_)), "{err:?}");
    }
}
