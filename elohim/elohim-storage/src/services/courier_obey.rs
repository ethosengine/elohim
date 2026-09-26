//! Courier obey — the adoption trigger's fast path to a head its author just
//! declared, verified read-only and stamped only once its bytes are held here.
//!
//! # The gap this closes
//!
//! A content doc naming a new head crosses the sync plane in under a second. The
//! canonical-head LINK that elects it gossips on its own schedule, and until it
//! lands this node's conductor keeps answering the previous election. The
//! adoption trigger could only re-probe its own conductor, so a doorway took
//! 47–59 s on a loaded household to offer a version its sibling had already
//! published (story 1.4b; `doorway-apex-transition.feature`).
//!
//! The author's publish mints its own election for the version it published
//! (`ContentService::update_via_conductor`, the `Declare` arm). The peer whose
//! doc apply raised the trigger — the COURIER — serves that election link and
//! the version's record over the existing `ContentHeadRecord` view, and this
//! module hands both to the OWN conductor's `verify_carried_head_evidence`.
//!
//! # What makes it safe (the reverted 1.4b's blockers)
//!
//! - **Nothing is declared here.** The verifier commits nothing; no link is
//!   minted on this node, no writer lock is taken.
//! - **Authority is authoring standing.** The wasm admits a carried election only
//!   when the agent that declared it authored the version and the root the
//!   version descends from, resolved from records this node already holds. A
//!   valid signature by anyone else is refused.
//! - **The author's own clock.** The carried link keeps its original timestamp
//!   and is merged with every candidate this node holds; older evidence elects
//!   the newer head and yields nothing to adopt, and `canonical_move_verdict`
//!   refuses any equal-or-older same-tier move on top of that.
//! - **The bytes before the move.** A row is not repointed onto a blob this node
//!   does not hold; it keeps serving the version it has while the bytes are
//!   requested, and the trigger's ladder re-checks. A verified version that names
//!   no blob never leaves a previous version's pointer standing under it.
//! - **Bounded, memoised work.** Refused evidence is remembered per
//!   `(courier, id, hint)`, and a courier whose evidence keeps being refused is
//!   skipped entirely for [`REFUSAL_TTL`], so rotating hints buys nothing.
//!
//! The doc hint only routes attention: the courier must serve that head, and the
//! verified election must elect it, or nothing moves.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use seam_contracts::Answer;

use crate::conductor_admission::AdmissionClass;
use crate::db::content_diesel::{self, ContentProjectionPatch, MinTrust, StampMode, StampOutcome};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::rea_projection::ContentEntry;
use crate::services::conductor_writes::{self, CarriedHeadEvidenceWire};
use crate::services::head_adoption::HeadRecordFetcher;

/// How long refused evidence is skipped.
pub const REFUSAL_TTL: Duration = Duration::from_secs(15 * 60);

/// Most refused `(courier, id, hint)` triples remembered; the oldest is evicted.
pub const REFUSAL_CAP: usize = 4096;

/// Refusals within [`REFUSAL_TTL`] after which a courier is skipped outright.
pub const COURIER_REFUSAL_LIMIT: u32 = 8;

/// The own conductor's read-only verifier, behind a trait so the composition is
/// testable without a conductor.
#[async_trait::async_trait]
pub trait EvidenceVerifier: Send + Sync {
    /// `content_store::verify_carried_head_evidence`. `Err` = refused.
    async fn verify(
        &self,
        id: &str,
        link_record: Vec<u8>,
        head_record: Vec<u8>,
    ) -> Result<Option<CarriedHeadEvidenceWire>, StorageError>;
}

/// [`EvidenceVerifier`] over the lamad conductor, on the `Background` lane: a
/// peer's doc traffic decides how often this runs, so it must never queue ahead
/// of a person's read.
pub struct ConductorVerifier(pub Arc<HcClient>);

#[async_trait::async_trait]
impl EvidenceVerifier for ConductorVerifier {
    async fn verify(
        &self,
        id: &str,
        link_record: Vec<u8>,
        head_record: Vec<u8>,
    ) -> Result<Option<CarriedHeadEvidenceWire>, StorageError> {
        conductor_writes::call_verify_carried_head_evidence(
            &self.0,
            id,
            link_record,
            head_record,
            AdmissionClass::Background,
        )
        .await
    }
}

/// Whether this node holds a blob, and a way to ask for it.
#[async_trait::async_trait]
pub trait BytePresence: Send + Sync {
    /// Held locally, whole or as enough shards to reassemble.
    async fn held(&self, address: &str) -> bool;
    /// Ask peers for it, `holder` first. Fire-and-forget; the caller re-checks
    /// [`Self::held`].
    fn request(&self, origin: &str, address: &str, holder: &str);
}

/// What one courier attempt did. Closed metric-label vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourierOutcome {
    /// Refused recently, for this head or too often for this courier; skipped.
    Memoised,
    /// The courier could not be asked or did not answer, or the own conductor
    /// shed the verification (backpressure, not a verdict).
    Unreachable,
    /// The courier did not serve an election link and a record for the head the
    /// doc names (it may hold a different head, or be forwarding the doc).
    NoEvidence,
    /// The own conductor refused the carried evidence as not proving what it
    /// claims.
    Refused,
    /// The own conductor could not give a verdict now (timeout, restart, a
    /// lineage not fully held yet). Not remembered: nothing was learned about
    /// the evidence.
    VerifyUnavailable,
    /// The slug is bound to a release channel, which elects its head; the
    /// courier path never moves it.
    ChannelHeld,
    /// The evidence is genuine but its declarer holds no authoring standing
    /// over the head (typically another peer's own declaration of it).
    NoStanding,
    /// The evidence verified, but the merged candidates elected nothing.
    NoWinner,
    /// The merged election elects a head other than the one the doc names.
    Disagrees,
    /// Verified, but the version names no blob while this row serves one; moving
    /// the head would leave the old pointer under the new version.
    PointerAbsent,
    /// Verified, but the new version's bytes are not held here yet; a fetch was
    /// requested and the row still serves what it had.
    AwaitingBytes,
    /// The row now declares the courier-carried head under its election.
    Stamped,
    /// The row already declared exactly this head; the stamp refreshed it.
    Current,
    /// The stamp guard refused: the row already obeys an equal-or-newer election.
    StampRefused,
    /// A local fault (DB) stopped the stamp.
    Failed,
}

impl CourierOutcome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Memoised => "courier_memoised",
            Self::Unreachable => "courier_unreachable",
            Self::NoEvidence => "courier_no_evidence",
            Self::Refused => "courier_refused",
            Self::VerifyUnavailable => "courier_verify_unavailable",
            Self::ChannelHeld => "courier_channel_held",
            Self::NoStanding => "courier_no_standing",
            Self::NoWinner => "courier_no_winner",
            Self::Disagrees => "courier_disagrees",
            Self::PointerAbsent => "courier_pointer_absent",
            Self::AwaitingBytes => "courier_awaiting_bytes",
            Self::Stamped => "courier_obeyed",
            Self::Current => "courier_current",
            Self::StampRefused => "courier_stamp_refused",
            Self::Failed => "courier_failed",
        }
    }

    /// Worth another rung of the trigger's ladder? Not once the answer is
    /// settled: adopted, already current, already newer, or a remembered
    /// refusal.
    pub fn retry_warranted(self) -> bool {
        !matches!(
            self,
            Self::Stamped | Self::Current | Self::StampRefused | Self::Memoised | Self::ChannelHeld
        )
    }
}

/// Refused evidence, bounded by count and age, per `(courier, id, hint)` and per
/// courier.
pub struct RefusalMemo {
    cap: usize,
    ttl: Duration,
    courier_limit: u32,
    inner: Mutex<MemoInner>,
}

#[derive(Default)]
struct MemoInner {
    at: HashMap<String, Instant>,
    order: VecDeque<String>,
    /// Refusal times per courier, oldest first.
    by_courier: HashMap<String, VecDeque<Instant>>,
}

impl RefusalMemo {
    pub fn new(cap: usize, ttl: Duration, courier_limit: u32) -> Self {
        Self {
            cap: cap.max(1),
            ttl,
            courier_limit: courier_limit.max(1),
            inner: Mutex::new(MemoInner::default()),
        }
    }

    fn key(courier: &str, id: &str, hint: &str) -> String {
        format!("{courier}\u{1}{id}\u{1}{hint}")
    }

    /// Skip this attempt? True when this exact triple was refused within the
    /// TTL, or the courier has reached its refusal limit within it.
    pub fn is_refused(&self, courier: &str, id: &str, hint: &str, now: Instant) -> bool {
        let Ok(mut inner) = self.inner.lock() else {
            return false;
        };
        let ttl = self.ttl;
        if let Some(times) = inner.by_courier.get_mut(courier) {
            while times
                .front()
                .is_some_and(|t| now.saturating_duration_since(*t) >= ttl)
            {
                times.pop_front();
            }
            if times.len() >= self.courier_limit as usize {
                return true;
            }
        }
        inner
            .at
            .get(&Self::key(courier, id, hint))
            .is_some_and(|t| now.saturating_duration_since(*t) < ttl)
    }

    /// Remember a refusal. Evicts the oldest triples past the cap; never clears
    /// everything at once, so an overflow cannot re-admit the whole set.
    /// `against_courier` counts it toward the courier's limit: true for
    /// evidence that does not prove what it claims, false for honest evidence
    /// that merely lacks authoring standing (another peer's own declaration of
    /// the head), which says nothing bad about the peer that carried it.
    pub fn remember(
        &self,
        courier: &str,
        id: &str,
        hint: &str,
        now: Instant,
        against_courier: bool,
    ) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        let key = Self::key(courier, id, hint);
        if inner.at.insert(key.clone(), now).is_none() {
            inner.order.push_back(key);
        }
        while inner.at.len() > self.cap {
            match inner.order.pop_front() {
                Some(old) => {
                    inner.at.remove(&old);
                }
                None => break,
            }
        }
        if !against_courier {
            return;
        }
        let limit = self.courier_limit as usize;
        let times = inner.by_courier.entry(courier.to_string()).or_default();
        times.push_back(now);
        while times.len() > limit {
            times.pop_front();
        }
        // Couriers are peers this node is connected to; drop the ones whose
        // refusals have all aged out so the map cannot grow without bound.
        if inner.by_courier.len() > self.cap {
            let ttl = self.ttl;
            inner.by_courier.retain(|_, t| {
                t.back()
                    .is_some_and(|last| now.saturating_duration_since(*last) < ttl)
            });
        }
        while inner.by_courier.len() > self.cap {
            let oldest = inner
                .by_courier
                .iter()
                .min_by_key(|(_, t)| t.back().copied())
                .map(|(k, _)| k.clone());
            match oldest {
                Some(k) => {
                    inner.by_courier.remove(&k);
                }
                None => break,
            }
        }
    }

    pub fn len(&self) -> usize {
        self.inner.lock().map(|i| i.at.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// How the own conductor answered a verification that did not succeed. Only a
/// refusal the zome itself authored is a verdict on the evidence; anything else
/// (a timeout, a restart, a decode fault, a lineage not held yet) is not, and
/// must never be remembered against the head or the courier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VerifyFailure {
    Refused,
    NoStanding,
    Unavailable,
}

fn classify_verify_failure(e: &StorageError) -> VerifyFailure {
    let msg = e.to_string();
    if crate::conductor_admission::is_admission_shed(e)
        || !msg.contains("verify_carried_head_evidence:")
        || msg.contains("lineage-not-held")
    {
        VerifyFailure::Unavailable
    } else if msg.contains("holds no authoring standing") {
        VerifyFailure::NoStanding
    } else {
        VerifyFailure::Refused
    }
}

/// The patch a proven head record projects: only fields the conductor proved.
pub(crate) fn patch_from_proven(c: &ContentEntry) -> ContentProjectionPatch {
    ContentProjectionPatch {
        blob_cid: c.blob_cid.clone(),
        content_size_bytes: c
            .content_size_bytes
            .map(|n| i32::try_from(n).unwrap_or(i32::MAX)),
        title: Some(c.title.clone()),
        description: Some(c.description.clone()),
        content_type: Some(c.content_type.clone()),
        content_format: Some(c.content_format.clone()),
        reach: Some(c.reach.clone()),
        metadata_json: Some(c.metadata_json.clone()),
    }
}

/// The patch a head MOVE carries from a proven record: the version's pointer,
/// size and metadata (which names its server bundle) — and nothing that could
/// narrow `reach` or rewrite identity fields, exactly as the adopt path's
/// T-1 move does (`head_adoption::adopt_local`). The RC-4 non-narrowing guard
/// lives on the projection, not here.
pub(crate) fn move_patch_from_proven(c: &ContentEntry) -> ContentProjectionPatch {
    ContentProjectionPatch {
        blob_cid: c.blob_cid.clone(),
        content_size_bytes: c
            .content_size_bytes
            .map(|n| i32::try_from(n).unwrap_or(i32::MAX)),
        metadata_json: Some(c.metadata_json.clone()),
        ..Default::default()
    }
}

/// Would adopting `content` leave this row's previous blob pointer standing
/// under a version that names none? (`ContentProjectionPatch.blob_cid: None`
/// preserves the column.)
pub fn pointer_would_be_left_behind(
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    content: &ContentEntry,
) -> bool {
    if content
        .blob_cid
        .as_deref()
        .is_some_and(|b| !b.trim().is_empty())
    {
        return false;
    }
    pool.get()
        .ok()
        .and_then(|mut c| content_diesel::get_content(&mut c, ctx, id, MinTrust::Invisible).ok())
        .flatten()
        .is_some_and(|row| row.blob_hash.as_deref().is_some_and(|b| !b.is_empty()))
}

/// The blobs a version needs to be served: its browser blob and, when the
/// metadata names one, its server bundle.
pub fn bytes_needed(c: &ContentEntry) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(b) = c.blob_cid.as_deref().filter(|b| !b.trim().is_empty()) {
        out.push(b.to_string());
    }
    if let Some(s) = content_diesel::server_bundle_from_metadata(&c.metadata_json) {
        if !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

/// Are all of a version's bytes held here? Requests the missing ones from
/// `holder` (and any advertiser) as a side effect.
pub async fn bytes_ready(
    bytes: &dyn BytePresence,
    content: &ContentEntry,
    id: &str,
    holder: &str,
) -> bool {
    let mut ready = true;
    for address in bytes_needed(content) {
        if !bytes.held(&address).await {
            bytes.request(&format!("head-adoption:{id}"), &address, holder);
            ready = false;
        }
    }
    ready
}

/// Ask `courier` for the evidence behind the head `doc_hint` names, verify it in
/// the own conductor without committing anything, and — once the version's
/// bytes are held here — stamp it under its election.
#[allow(clippy::too_many_arguments)]
pub async fn courier_obey(
    verifier: &dyn EvidenceVerifier,
    fetcher: &dyn HeadRecordFetcher,
    bytes: &dyn BytePresence,
    memo: &RefusalMemo,
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    courier: &str,
    doc_hint: &str,
) -> CourierOutcome {
    if memo.is_refused(courier, id, doc_hint, Instant::now()) {
        return CourierOutcome::Memoised;
    }
    if crate::services::head_adoption::held_by_release_channel(pool, ctx, id) {
        return CourierOutcome::ChannelHeld;
    }

    let carried = match fetcher.fetch(courier, id).await {
        Answer::Present(c) => c,
        Answer::Absent => return CourierOutcome::NoEvidence,
        Answer::Unreachable => return CourierOutcome::Unreachable,
    };
    // The courier must itself serve the head the doc names; a peer that is only
    // forwarding the doc, or already holds another head, is not evidence.
    if carried.head_action_hash.trim() != doc_hint.trim() {
        return CourierOutcome::NoEvidence;
    }
    let (Some(link_record), Some(head_record)) =
        (carried.election_link_record.clone(), carried.record.clone())
    else {
        return CourierOutcome::NoEvidence;
    };

    let verified = match verifier.verify(id, link_record, head_record).await {
        Ok(Some(v)) => v,
        Ok(None) => return CourierOutcome::NoWinner,
        Err(e) => {
            let failure = classify_verify_failure(&e);
            if failure == VerifyFailure::Unavailable {
                return CourierOutcome::VerifyUnavailable;
            }
            let no_standing = failure == VerifyFailure::NoStanding;
            memo.remember(courier, id, doc_hint, Instant::now(), !no_standing);
            if no_standing {
                return CourierOutcome::NoStanding;
            }
            tracing::warn!(
                target: "elohim_storage::head_adoption_trigger",
                content_id = %id, courier = %courier, head = %doc_hint, error = %e,
                "courier-obey: the own conductor REFUSED the courier's head evidence — \
                 remembered, not re-verified"
            );
            return CourierOutcome::Refused;
        }
    };
    let Some(proven) = verified.head else {
        return CourierOutcome::Disagrees;
    };
    if proven.head_action_hash.to_string().trim() != doc_hint.trim() {
        return CourierOutcome::Disagrees;
    }
    let Some(ordering) = proven.canonical_ordering() else {
        return CourierOutcome::NoWinner;
    };

    if pointer_would_be_left_behind(pool, ctx, id, &proven.content) {
        return CourierOutcome::PointerAbsent;
    }
    if !bytes_ready(bytes, &proven.content, id, courier).await {
        return CourierOutcome::AwaitingBytes;
    }

    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(content_id = %id, error = %e, "courier-obey: db conn for stamp");
            return CourierOutcome::Failed;
        }
    };
    match content_diesel::stamp_declared_head_mode(
        &mut conn,
        ctx,
        id,
        doc_hint.trim(),
        Some(proven.declared_at),
        Some(move_patch_from_proven(&proven.content)),
        StampMode::HealCanonical,
        Some(ordering),
    ) {
        Ok(StampOutcome::Stamped) => CourierOutcome::Stamped,
        Ok(StampOutcome::Refreshed) => CourierOutcome::Current,
        Ok(_) => CourierOutcome::StampRefused,
        Err(e) => {
            tracing::warn!(content_id = %id, error = %e, "courier-obey: stamp failed");
            CourierOutcome::Failed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::head_adoption::CarriedHeadRecord;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const ID: &str = "landing";
    const COURIER: &str = "12D3KooWcourier";
    const HEAD_A: &str = "uhCkkA";
    const HEAD_B: &str = "uhCkkB";
    /// Election clocks: A's is older than B's.
    const AT_A: i64 = 1_000;
    const AT_B: i64 = 2_000;

    fn pool_with_row_at_a() -> DbPool {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::default_lamad();
        content_diesel::create_content(
            &mut conn,
            &ctx,
            content_diesel::CreateContentInput {
                id: ID.to_string(),
                title: ID.to_string(),
                description: None,
                content_type: "concept".to_string(),
                content_format: "html5-app".to_string(),
                blob_hash: Some("sha256-aaaa".to_string()),
                blob_cid: Some("sha256-aaaa".to_string()),
                content_size_bytes: Some(10),
                metadata_json: Some("{}".to_string()),
                reach: "commons".to_string(),
                created_by: None,
                tags: Vec::new(),
                content_body: None,
                dht_anchor_hash: None,
            },
        )
        .unwrap();
        content_diesel::stamp_declared_head_mode(
            &mut conn,
            &ctx,
            ID,
            HEAD_A,
            Some(AT_A),
            None,
            StampMode::HealCanonical,
            Some((AT_A, false)),
        )
        .unwrap();
        drop(conn);
        pool
    }

    fn row(pool: &DbPool) -> (Option<String>, Option<String>, Option<i64>) {
        let mut conn = pool.get().unwrap();
        let c = content_diesel::get_content(
            &mut conn,
            &AppContext::default_lamad(),
            ID,
            MinTrust::Invisible,
        )
        .unwrap()
        .unwrap();
        (
            c.declared_head_action_hash,
            c.blob_hash,
            c.canonical_declared_at,
        )
    }

    fn evidence(head: Option<&str>, blob: Option<&str>, at: i64) -> CarriedHeadEvidenceWire {
        let head_json = head.map(|h| {
            serde_json::json!({
                "content_id": ID,
                "head_action_hash": h,
                "declared_at": at,
                "canonical": true,
                "canonical_declared_at": at,
                "canonical_earned": false,
                "content": {
                    "id": ID,
                    "content_type": "concept",
                    "content_format": "html5-app",
                    "title": "B",
                    "description": "",
                    "reach": "commons",
                    "blob_cid": blob,
                },
            })
        });
        serde_json::from_value(serde_json::json!({
            "election": {
                "winner_target": head.unwrap_or("uhCkkOther"),
                "canonical_declared_at": at,
                "canonical_earned": false,
            },
            "head": head_json,
        }))
        .expect("evidence fixture")
    }

    struct Courier {
        serves: &'static str,
        asked: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl HeadRecordFetcher for Courier {
        async fn fetch(&self, _peer: &str, _id: &str) -> Answer<CarriedHeadRecord> {
            self.asked.fetch_add(1, Ordering::SeqCst);
            Answer::Present(CarriedHeadRecord {
                head_action_hash: self.serves.to_string(),
                record: Some(b"record".to_vec()),
                record_absent_reason: None,
                election_link_record: Some(b"link".to_vec()),
            })
        }
    }

    fn courier(serves: &'static str) -> Courier {
        Courier {
            serves,
            asked: AtomicUsize::new(0),
        }
    }

    enum Verdict {
        Proves(CarriedHeadEvidenceWire),
        Refuses(&'static str),
    }

    struct Verifier {
        verdict: Verdict,
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl EvidenceVerifier for Verifier {
        async fn verify(
            &self,
            _id: &str,
            _link: Vec<u8>,
            _record: Vec<u8>,
        ) -> Result<Option<CarriedHeadEvidenceWire>, StorageError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            match &self.verdict {
                Verdict::Proves(e) => Ok(Some(e.clone())),
                Verdict::Refuses(msg) => Err(StorageError::Internal((*msg).to_string())),
            }
        }
    }

    fn verifier(verdict: Verdict) -> Verifier {
        Verifier {
            verdict,
            calls: AtomicUsize::new(0),
        }
    }

    struct Bytes {
        held: bool,
        requested: Mutex<Vec<(String, String)>>,
    }

    #[async_trait::async_trait]
    impl BytePresence for Bytes {
        async fn held(&self, _address: &str) -> bool {
            self.held
        }
        fn request(&self, _origin: &str, address: &str, holder: &str) {
            self.requested
                .lock()
                .unwrap()
                .push((address.to_string(), holder.to_string()));
        }
    }

    fn bytes(held: bool) -> Bytes {
        Bytes {
            held,
            requested: Mutex::new(Vec::new()),
        }
    }

    fn memo() -> RefusalMemo {
        RefusalMemo::new(REFUSAL_CAP, REFUSAL_TTL, COURIER_REFUSAL_LIMIT)
    }

    async fn run(
        v: &Verifier,
        f: &Courier,
        b: &Bytes,
        m: &RefusalMemo,
        pool: &DbPool,
        hint: &str,
    ) -> CourierOutcome {
        courier_obey(
            v,
            f,
            b,
            m,
            pool,
            &AppContext::default_lamad(),
            ID,
            COURIER,
            hint,
        )
        .await
    }

    #[tokio::test]
    async fn a_verified_head_whose_bytes_are_held_moves_head_pointer_and_ordering_together() {
        let pool = pool_with_row_at_a();
        let v = verifier(Verdict::Proves(evidence(
            Some(HEAD_B),
            Some("sha256-bbbb"),
            AT_B,
        )));
        let outcome = run(&v, &courier(HEAD_B), &bytes(true), &memo(), &pool, HEAD_B).await;
        assert_eq!(outcome, CourierOutcome::Stamped);
        assert_eq!(
            row(&pool),
            (Some(HEAD_B.into()), Some("sha256-bbbb".into()), Some(AT_B)),
            "head, pointer and the author's election clock move as one"
        );
    }

    #[tokio::test]
    async fn a_verified_head_whose_bytes_are_absent_leaves_the_served_version_and_asks_the_courier()
    {
        let pool = pool_with_row_at_a();
        let v = verifier(Verdict::Proves(evidence(
            Some(HEAD_B),
            Some("sha256-bbbb"),
            AT_B,
        )));
        let b = bytes(false);
        let outcome = run(&v, &courier(HEAD_B), &b, &memo(), &pool, HEAD_B).await;
        assert_eq!(outcome, CourierOutcome::AwaitingBytes);
        assert!(outcome.retry_warranted());
        assert_eq!(
            row(&pool),
            (Some(HEAD_A.into()), Some("sha256-aaaa".into()), Some(AT_A))
        );
        assert_eq!(
            *b.requested.lock().unwrap(),
            vec![("sha256-bbbb".to_string(), COURIER.to_string())]
        );
    }

    #[tokio::test]
    async fn a_courier_that_does_not_serve_the_doc_head_is_not_evidence() {
        let pool = pool_with_row_at_a();
        let v = verifier(Verdict::Proves(evidence(
            Some(HEAD_B),
            Some("sha256-bbbb"),
            AT_B,
        )));
        let outcome = run(&v, &courier(HEAD_A), &bytes(true), &memo(), &pool, HEAD_B).await;
        assert_eq!(outcome, CourierOutcome::NoEvidence);
        assert_eq!(v.calls.load(Ordering::SeqCst), 0, "no conductor call spent");
        assert_eq!(row(&pool).0.as_deref(), Some(HEAD_A));
    }

    #[tokio::test]
    async fn an_election_that_elects_another_head_moves_nothing() {
        let pool = pool_with_row_at_a();
        let v = verifier(Verdict::Proves(evidence(None, None, AT_B)));
        let outcome = run(&v, &courier(HEAD_B), &bytes(true), &memo(), &pool, HEAD_B).await;
        assert_eq!(outcome, CourierOutcome::Disagrees);
        assert_eq!(row(&pool).0.as_deref(), Some(HEAD_A));
    }

    #[tokio::test]
    async fn an_older_election_than_the_row_holds_is_refused_by_the_stamp_guard() {
        let pool = pool_with_row_at_a();
        let v = verifier(Verdict::Proves(evidence(
            Some(HEAD_B),
            Some("sha256-bbbb"),
            AT_A - 1,
        )));
        let outcome = run(&v, &courier(HEAD_B), &bytes(true), &memo(), &pool, HEAD_B).await;
        assert_eq!(outcome, CourierOutcome::StampRefused);
        assert_eq!(
            row(&pool),
            (Some(HEAD_A.into()), Some("sha256-aaaa".into()), Some(AT_A))
        );
    }

    #[tokio::test]
    async fn a_verified_version_naming_no_blob_never_leaves_the_old_pointer_under_it() {
        let pool = pool_with_row_at_a();
        let v = verifier(Verdict::Proves(evidence(Some(HEAD_B), None, AT_B)));
        let outcome = run(&v, &courier(HEAD_B), &bytes(true), &memo(), &pool, HEAD_B).await;
        assert_eq!(outcome, CourierOutcome::PointerAbsent);
        assert_eq!(row(&pool).0.as_deref(), Some(HEAD_A));
    }

    #[tokio::test]
    async fn refused_evidence_is_not_re_fetched_or_re_verified() {
        let pool = pool_with_row_at_a();
        let v = verifier(Verdict::Refuses(
            "verify_carried_head_evidence: carried link record carries an invalid author signature",
        ));
        let f = courier(HEAD_B);
        let m = memo();
        assert_eq!(
            run(&v, &f, &bytes(true), &m, &pool, HEAD_B).await,
            CourierOutcome::Refused
        );
        assert_eq!(
            run(&v, &f, &bytes(true), &m, &pool, HEAD_B).await,
            CourierOutcome::Memoised
        );
        assert_eq!(f.asked.load(Ordering::SeqCst), 1);
        assert_eq!(v.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn evidence_without_standing_is_remembered_for_the_head_but_does_not_quarantine_the_courier(
    ) {
        let pool = pool_with_row_at_a();
        let v = verifier(Verdict::Refuses(
            "verify_carried_head_evidence: declarer X holds no authoring standing over Y",
        ));
        let m = memo();
        for i in 0..(COURIER_REFUSAL_LIMIT + 2) {
            let hint: &'static str = Box::leak(format!("uhCkkHint{i}").into_boxed_str());
            let outcome = run(&v, &courier(hint), &bytes(true), &m, &pool, hint).await;
            assert_eq!(outcome, CourierOutcome::NoStanding);
        }
        assert!(!m.is_refused(COURIER, ID, "uhCkkFresh", Instant::now()));
    }

    #[tokio::test]
    async fn a_conductor_that_cannot_answer_now_is_not_a_verdict_on_the_evidence() {
        let pool = pool_with_row_at_a();
        let m = memo();
        for msg in [
            "Conductor error: websocket closed",
            "verify_carried_head_evidence: lineage-not-held: the lineage of X is not fully held here yet",
        ] {
            let v = verifier(Verdict::Refuses(msg));
            let outcome = run(&v, &courier(HEAD_B), &bytes(true), &m, &pool, HEAD_B).await;
            assert_eq!(outcome, CourierOutcome::VerifyUnavailable, "{msg}");
            assert!(outcome.retry_warranted());
        }
        assert!(m.is_empty(), "nothing remembered");
    }

    #[tokio::test]
    async fn a_slug_bound_to_a_release_channel_is_never_moved_by_a_courier() {
        let pool = pool_with_row_at_a();
        {
            let mut conn = pool.get().unwrap();
            content_diesel::update_content(
                &mut conn,
                &AppContext::default_lamad(),
                content_diesel::UpdateContentInput {
                    id: ID.to_string(),
                    metadata_json: Some(
                        r#"{"releaseChannel":"runtime:elohim:app:stable"}"#.to_string(),
                    ),
                    ..Default::default()
                },
            )
            .unwrap();
        }
        assert!(
            crate::services::head_adoption::held_by_release_channel(
                &pool,
                &AppContext::default_lamad(),
                ID,
            ),
            "fixture: the row is bound to a release channel"
        );
        let v = verifier(Verdict::Proves(evidence(
            Some(HEAD_B),
            Some("sha256-bbbb"),
            AT_B,
        )));
        let f = courier(HEAD_B);
        let outcome = run(&v, &f, &bytes(true), &memo(), &pool, HEAD_B).await;
        assert_eq!(outcome, CourierOutcome::ChannelHeld);
        assert_eq!(f.asked.load(Ordering::SeqCst), 0);
        assert_eq!(row(&pool).0.as_deref(), Some(HEAD_A));
    }

    #[test]
    fn a_courier_rotating_hints_is_skipped_once_it_reaches_its_refusal_limit() {
        let m = memo();
        let now = Instant::now();
        for i in 0..COURIER_REFUSAL_LIMIT {
            assert!(!m.is_refused(COURIER, ID, &format!("h{i}"), now));
            m.remember(COURIER, ID, &format!("h{i}"), now, true);
        }
        assert!(m.is_refused(COURIER, ID, "a-hint-never-seen", now));
        assert!(!m.is_refused("another-courier", ID, "a-hint-never-seen", now));
        assert!(
            !m.is_refused(COURIER, ID, "a-hint-never-seen", now + REFUSAL_TTL),
            "refusals age out"
        );
    }

    #[test]
    fn the_refusal_memo_evicts_the_oldest_and_never_clears_everything() {
        let m = RefusalMemo::new(2, REFUSAL_TTL, u32::MAX);
        let now = Instant::now();
        m.remember("c", ID, "h1", now, false);
        m.remember("c", ID, "h2", now, false);
        m.remember("c", ID, "h3", now, false);
        assert_eq!(m.len(), 2);
        assert!(!m.is_refused("c", ID, "h1", now), "oldest evicted");
        assert!(m.is_refused("c", ID, "h2", now));
        assert!(m.is_refused("c", ID, "h3", now));
    }

    #[test]
    fn a_version_needs_its_browser_blob_and_the_server_bundle_its_metadata_names() {
        let entry: ContentEntry = serde_json::from_value(serde_json::json!({
            "id": ID, "content_type": "concept", "title": "t", "description": "",
            "content_format": "html5-app", "reach": "commons",
            "blob_cid": "sha256-browser",
            "metadata_json": r#"{"serverBlobHash":"sha256-server"}"#,
        }))
        .unwrap();
        assert_eq!(
            bytes_needed(&entry),
            vec!["sha256-browser", "sha256-server"]
        );
    }
}
