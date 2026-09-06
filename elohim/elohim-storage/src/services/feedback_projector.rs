//! The single application path for witnessed corrections (contract §§1-3, 5-7).
//!
//! One loop owns the whole journey from "a link exists somewhere" to "the
//! aggregate moved": fair rotating enumeration over a durable subscription set,
//! fetch-and-VERIFY of each unapplied act, group resolution, acceptance
//! verification against the target's exact root author, and a transactional
//! apply. Nothing else writes an application row.
//!
//! Four shapes here are deliberate departures from what the codebase already
//! does, each because the existing shape has a named failure:
//!
//! 1. **Fair rotation, not `take(N)`.** `release_adoption/watch.rs` takes the
//!    first N members of an unsorted set every tick, so the ninth member of a
//!    nine-member set is never visited. The cursor here advances past whatever
//!    was served — on FAILURE as well as success — so N members under a per-tick
//!    budget of B are each visited within ceil(N/B) ticks.
//!
//! 2. **One transaction that PROPAGATES.** `db/economic_events.rs`'s per-item
//!    loop swallows errors so a partial batch reads as success. The apply here
//!    commits the application row, its members and the aggregate together or
//!    not at all.
//!
//! 3. **Pending is not rejected.** An unfetchable dependency holds the group
//!    `pending` and it is SHOWN as pending. Only a positive mismatch — wrong
//!    author, wrong entry type, a request the evidence does not bind — is a
//!    rejection, and rejections are not retried.
//!
//! 4. **A correction alone contributes ZERO.** An allegation debits nobody.
//!    Only an ACCEPTED correction moves the aggregate, and it moves it against
//!    the TARGET's root author, never against the signal's signer.

use std::sync::Arc;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use serde::{Deserialize, Serialize};

use crate::db::feedback_application::{
    self as app_db, ApplicationRow, MemberRow, MEMBER_ROLE_ACCEPTANCE, MEMBER_ROLE_CORRECTION,
    MEMBER_STATUS_MEMBER, MEMBER_STATUS_PENDING, MEMBER_STATUS_REJECTED, STATUS_APPLIED,
    STATUS_PENDING, STATUS_REJECTED,
};
use crate::db::feedback_subscriptions as sub_db;
use crate::db::standing_generations::{
    self as gen_db, GenerationAggregateRow, GEN_PUBLISHED, GEN_REBUILDING,
};
use crate::db::standing_view::{upsert as upsert_standing_view, StandingViewRow};
use crate::db::DbPool;
use crate::error::StorageError;
use crate::services::standing_projector::{score_for_debit_sum, serialize_score};

// ---------------------------------------------------------------------------
// Budgets. Every one is a conductor-round-trip or byte budget, sized BEFORE any
// call is made on a tick — the same discipline the release-adoption sweep uses.
// ---------------------------------------------------------------------------

/// How often the projector sweeps. Discovery is not latency-critical: a
/// correction is a deliberate act and a minute of projection latency costs
/// nothing, while a tighter loop only spends conductor capacity.
pub const SWEEP_INTERVAL_SECS: u64 = 45;

/// Subscription members visited per tick. NOT a `take(N)` over an unsorted set
/// — see the rotation note above.
pub const MAX_MEMBERS_PER_SWEEP: i64 = 8;

/// Records FETCHED per tick, across all members. Full link enumeration still
/// scales with accumulated history per member; this bounds only the fetches.
pub const MAX_RECORDS_PER_SWEEP: usize = 32;

/// Bytes of fetched record payload per tick.
pub const MAX_BYTES_PER_SWEEP: usize = 4 * 1024 * 1024;

/// First retry delay for a PENDING group; doubles per attempt to the ceiling.
pub const RETRY_BASE_SECS: i64 = 60;
pub const RETRY_MAX_SECS: i64 = 3600;

// ---------------------------------------------------------------------------
// Wire mirrors of the coordinator's own types
// ---------------------------------------------------------------------------

/// Mirror of `content_store_integrity::FeedbackSignal` — the DHT ENTRY, which
/// is a different shape from the libp2p wire type in `p2p/feedback_signal.rs`
/// (that one carries `signed_by`/`signature` strings and typed enums). The
/// entry is what a fetched record actually contains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DhtFeedbackSignal {
    pub target_cid: String,
    pub signal_kind: String,
    #[serde(default)]
    pub vouch_kind: Option<String>,
    #[serde(default)]
    pub evidence_cid: Option<String>,
    pub standing_impact: String,
    #[serde(default)]
    pub signer_pubkey: Vec<u8>,
}

/// Mirror of `content_store::correction::LineageCandidate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LineageCandidate {
    pub action_hash: String,
    pub predecessor: Option<String>,
    pub author: Option<String>,
    pub timestamp: Option<i64>,
    pub fetch_outcome: String,
    pub in_root: bool,
}

/// Mirror of `content_store::correction::ContentLineageOutput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ContentLineage {
    pub referenced_action_hash: String,
    pub root_action_hash: String,
    /// `uhCAk…` display form of the root Create's author.
    pub root_author: String,
    pub content_id: String,
    #[serde(default)]
    pub candidates: Vec<LineageCandidate>,
    #[serde(default)]
    pub head_action_hash: Option<String>,
    #[serde(default)]
    pub contested: bool,
    #[serde(default)]
    pub contested_predecessors: Vec<String>,
}

/// One discovered act reference with its explicit outcome (`§3`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredRef {
    pub action_hash: String,
    pub fetch_outcome: String,
}

// ---------------------------------------------------------------------------
// The conductor seam
// ---------------------------------------------------------------------------

/// Everything the projector needs from the DHT, behind ONE trait.
///
/// This is the mocking seam: the whole application path — rotation, grouping,
/// acceptance, apply, backoff — is testable against a fake reader with no
/// conductor, exactly as `CommitmentFetcher`/`RateHistory` make the
/// bounds-validator testable.
#[async_trait::async_trait]
pub trait FeedbackDhtReader: Send + Sync {
    /// The receiving storage's own content-cell DNA hash. §1: an envelope
    /// naming a different DNA hash is rejected, and cross-context evidence is
    /// explicitly NOT this slice.
    fn origin_dna_hash(&self) -> String;

    /// Reference-plus-outcome enumeration for one subscription member.
    async fn refs_for_target(&self, base_action_hash: &str)
        -> Result<Vec<DiscoveredRef>, StorageError>;

    /// The SIGNED record for one act. `Ok(None)` = not retrievable right now,
    /// which is PENDING, never proof of absence.
    async fn signal_record(
        &self,
        action_hash: &str,
    ) -> Result<Option<FetchedRecord>, StorageError>;

    /// Verified exact-root lineage of a content action.
    async fn content_lineage(
        &self,
        action_hash: &str,
    ) -> Result<Option<ContentLineage>, StorageError>;
}

/// What the reader hands back for one act: the fields §1 requires be taken
/// from the SIGNED ACTION, plus the decoded entry.
#[derive(Debug, Clone)]
pub struct FetchedRecord {
    /// The action hash the record actually carries — compared against the one
    /// requested, so a substituted answer is caught.
    pub action_hash: String,
    /// `record.action().author()` — NOT the entry's `signer_pubkey`, which is
    /// coordinator-derived and not integrity-bound to the action author.
    pub author_raw: Vec<u8>,
    pub timestamp_micros: i64,
    /// `true` when the record carried an App entry whose hash matched the
    /// action's `entry_hash`. A signed header alone is never evidence.
    pub entry_hash_bound: bool,
    /// `None` when the record answered `Hidden`/`NotStored` or held a
    /// non-FeedbackSignal entry.
    pub entry: Option<DhtFeedbackSignal>,
    pub entry_bytes_len: usize,
}

// ---------------------------------------------------------------------------
// §1 verification (pure)
// ---------------------------------------------------------------------------

/// A verified act. Identity is `(origin DNA hash, action hash)` — the SIGNED
/// act, deliberately not the entry hash: two authors' identical corrections
/// share an entry hash and are two acts.
#[derive(Debug, Clone)]
pub struct VerifiedAct {
    pub action_hash: String,
    pub origin_dna_hash: String,
    /// Normalised to 32 bytes.
    pub author: Vec<u8>,
    pub timestamp_micros: i64,
    pub entry: DhtFeedbackSignal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActVerification {
    Verified(Box<VerifiedAct>),
    /// A dependency is not retrievable. Retryable; never shown as accepted.
    Pending(String),
    /// A POSITIVE mismatch. Not retried.
    Rejected(String),
}

impl PartialEq for VerifiedAct {
    fn eq(&self, other: &Self) -> bool {
        self.action_hash == other.action_hash
            && self.origin_dna_hash == other.origin_dna_hash
            && self.author == other.author
            && self.entry == other.entry
    }
}
impl Eq for VerifiedAct {}

/// **Pure.** Normalise an agent key to its 32 ed25519 bytes.
///
/// Holochain's `AgentPubKey::get_raw_39()` is `3-byte prefix || 32-byte key ||
/// 4-byte location`; other surfaces carry the bare 32. Comparing the two
/// representations byte-for-byte silently never matches, which is exactly how
/// an author gate turns into a permanent refusal.
pub fn normalize_agent_key(raw: &[u8]) -> Vec<u8> {
    match raw.len() {
        39 => raw[3..35].to_vec(),
        36 => raw[3..].to_vec(),
        _ => raw.to_vec(),
    }
}

/// **Pure.** Verify a fetched record against §1 before it is allowed to mean
/// anything.
pub fn verify_record(
    requested_action_hash: &str,
    expected_origin_dna: &str,
    envelope_origin_dna: &str,
    fetched: Option<FetchedRecord>,
) -> ActVerification {
    if envelope_origin_dna != expected_origin_dna {
        return ActVerification::Rejected(format!(
            "foreign origin DNA hash '{envelope_origin_dna}' (this cell is '{expected_origin_dna}') \
             — cross-context evidence is not slice 1"
        ));
    }
    let Some(fetched) = fetched else {
        return ActVerification::Pending(format!(
            "act {requested_action_hash} not retrievable from this peer's DHT view"
        ));
    };
    if fetched.action_hash != requested_action_hash {
        return ActVerification::Rejected(format!(
            "fetched record names action {} but {requested_action_hash} was requested",
            fetched.action_hash
        ));
    }
    if !fetched.entry_hash_bound {
        return ActVerification::Rejected(format!(
            "act {requested_action_hash}: entry hash does not match the action's entry hash"
        ));
    }
    let Some(entry) = fetched.entry else {
        // Hidden / NotStored / wrong entry type. Bytes absent is PENDING (the
        // authority may serve them later); it is not a claim of absence.
        return ActVerification::Pending(format!(
            "act {requested_action_hash}: entry bytes absent (Hidden/NotStored) — a signed header \
             alone is never evidence"
        ));
    };
    ActVerification::Verified(Box::new(VerifiedAct {
        action_hash: requested_action_hash.to_string(),
        origin_dna_hash: expected_origin_dna.to_string(),
        author: normalize_agent_key(&fetched.author_raw),
        timestamp_micros: fetched.timestamp_micros,
        entry,
    }))
}

// ---------------------------------------------------------------------------
// §7/§8 group resolution (pure)
// ---------------------------------------------------------------------------

/// **Pure.** The operation group an act belongs to.
///
/// A correction's group key is its EVIDENCE ACTION HASH — the thing §8 pins the
/// operation to on chain. An acceptance's group key is the correction group it
/// targets, which the caller resolves by looking up the correction first; a
/// vouch that targets an unknown correction is not yet groupable.
pub fn correction_group_key(act: &VerifiedAct) -> Option<String> {
    if act.entry.signal_kind != "correction" {
        return None;
    }
    act.entry.evidence_cid.clone()
}

/// **Pure.** Is this act an acceptance of a correction (§5.2)?
pub fn is_acceptance(act: &VerifiedAct) -> bool {
    act.entry.signal_kind == "vouch"
        && act.entry.vouch_kind.as_deref() == Some("accept-correction")
}

// ---------------------------------------------------------------------------
// §5.2 acceptance verification (pure over a resolved lineage)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptanceVerdict {
    /// The vouch author IS the target's exact root author. `subject` is that
    /// root author — the subject a contribution is applied against.
    Accepted { subject: Vec<u8> },
    /// A dependency could not be fetched. Shown as pending; never accepted.
    Pending(String),
    /// A positive mismatch. Not retried.
    Refused(String),
}

/// **Pure.** Verify one acceptance against §5.2's full chain:
/// vouch → correction → target content → EXACT root author.
///
/// Every link is checked here because integrity admits any vouch by anyone
/// (`content_store_integrity/src/feedback_signal.rs` validates shape, not
/// authority). Acceptance is therefore a storage-verified projection
/// classification, and this function is where that classification lives.
pub fn verify_acceptance(
    vouch: &VerifiedAct,
    correction: &VerifiedAct,
    target_lineage: Option<&ContentLineage>,
    root_author_bytes: Option<&[u8]>,
) -> AcceptanceVerdict {
    if !is_acceptance(vouch) {
        return AcceptanceVerdict::Refused(format!(
            "act {} is not an accept-correction vouch",
            vouch.action_hash
        ));
    }
    if vouch.entry.target_cid != correction.action_hash {
        return AcceptanceVerdict::Refused(format!(
            "vouch {} targets {} but was paired with correction {}",
            vouch.action_hash, vouch.entry.target_cid, correction.action_hash
        ));
    }
    if correction.entry.signal_kind != "correction" {
        return AcceptanceVerdict::Refused(format!(
            "vouch {} targets act {}, which is a '{}', not a correction",
            vouch.action_hash, correction.action_hash, correction.entry.signal_kind
        ));
    }
    let Some(lineage) = target_lineage else {
        return AcceptanceVerdict::Pending(format!(
            "target lineage for {} not retrievable",
            correction.entry.target_cid
        ));
    };
    if lineage.referenced_action_hash != correction.entry.target_cid {
        return AcceptanceVerdict::Refused(format!(
            "lineage answers for {} but the correction targets {}",
            lineage.referenced_action_hash, correction.entry.target_cid
        ));
    }
    let Some(root_author) = root_author_bytes else {
        return AcceptanceVerdict::Pending(format!(
            "root author of {} not decodable yet",
            correction.entry.target_cid
        ));
    };
    let root_author = normalize_agent_key(root_author);
    if vouch.author != root_author {
        return AcceptanceVerdict::Refused(format!(
            "vouch {} was authored by an agent that is not the target's exact root author — \
             slice 1 admits the root author only (delegate acceptance is a named gap)",
            vouch.action_hash
        ));
    }
    // Same-cell self-acceptance is refused by the coordinator and unsupported
    // here: it stays visibly pending rather than being reported as accepted.
    if correction.author == root_author {
        return AcceptanceVerdict::Pending(
            "same-cell self-acceptance is unsupported in slice 1 — held pending, never reported \
             as accepted"
                .to_string(),
        );
    }
    AcceptanceVerdict::Accepted {
        subject: root_author,
    }
}

// ---------------------------------------------------------------------------
// §7 contribution policy (pure, over PINNED bytes)
// ---------------------------------------------------------------------------

/// The debit-weight table PINNED into a generation. Deserialised from the
/// generation's `policy_bytes`, never re-read from the live registry: reading
/// the registry at replay time makes a replay non-deterministic the moment the
/// manifest moves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedPolicy {
    /// `"<signal_kind>/<vouch_kind|->/<standing_impact>" -> weight`.
    #[serde(default)]
    pub debit_weights: std::collections::BTreeMap<String, i32>,
    /// Weight applied when an ACCEPTED correction lands and no explicit key
    /// matches. Bootstrap default mirrors the pre-cutover correction weights.
    #[serde(default = "default_accepted_correction_weight")]
    pub accepted_correction_default: i32,
}

fn default_accepted_correction_weight() -> i32 {
    2
}

impl Default for PinnedPolicy {
    fn default() -> Self {
        let mut debit_weights = std::collections::BTreeMap::new();
        // Bootstrap weights. Only the ACCEPTED arms exist: an unaccepted
        // correction has no weight because it contributes nothing at all.
        debit_weights.insert("correction/accepted/advisory".to_string(), 0);
        debit_weights.insert("correction/accepted/debit-soft".to_string(), 2);
        debit_weights.insert("correction/accepted/debit-firm".to_string(), 8);
        Self {
            debit_weights,
            accepted_correction_default: default_accepted_correction_weight(),
        }
    }
}

impl PinnedPolicy {
    pub fn canonical_bytes(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// A LOCAL digest of the pinned bytes — deliberately not an EPR CID and not
    /// minted through the canonical addressing codec. Its only job is to say
    /// "these are the same policy bytes as last tick", which is what decides
    /// whether a generation may be resumed or a new one must be opened.
    pub fn policy_digest(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(self.canonical_bytes().as_bytes());
        format!("sha256:{}", hex::encode(h.finalize()))
    }

    /// **Pure.** The contribution one GROUP makes.
    ///
    /// A correction alone contributes ZERO — an allegation debits nobody. That
    /// is a deliberate change from the pre-cutover immediate debit, and it is
    /// what makes "Unknown" mean "no accepted correction" rather than "not yet
    /// projected".
    ///
    /// The policy input carries the vouch SUBTYPE and the acceptance STATUS,
    /// which the pre-cutover policy trait could not see (it took kind and
    /// impact only).
    pub fn contribution(&self, signal_kind: &str, accepted: bool, standing_impact: &str) -> i32 {
        if !accepted {
            return 0;
        }
        let key = format!("{signal_kind}/accepted/{standing_impact}");
        self.debit_weights
            .get(&key)
            .copied()
            .unwrap_or(self.accepted_correction_default)
    }
}

// ---------------------------------------------------------------------------
// Transactional apply (§7)
// ---------------------------------------------------------------------------

/// What one resolved group contributes to one generation.
#[derive(Debug, Clone)]
pub struct GroupOutcome {
    pub group_key: String,
    pub status: String,
    pub contribution: i32,
    pub subject: Option<Vec<u8>>,
    pub accepted: bool,
    /// The MAXIMUM included action timestamp, never the last enumerated.
    pub max_included_at_micros: Option<i64>,
    pub error: Option<String>,
    pub members: Vec<MemberRow>,
}

/// **Pure.** Backoff for the nth attempt, capped.
pub fn backoff_secs(attempts: i32) -> i64 {
    let shifted = RETRY_BASE_SECS.saturating_mul(1i64 << attempts.clamp(0, 16));
    shifted.min(RETRY_MAX_SECS)
}

/// Commit one group's outcome: the application row, its member rows and — when
/// the group is newly applied with a non-zero contribution — the generation
/// aggregate, in ONE transaction that PROPAGATES failure.
///
/// Idempotent by the application row: a group already `applied` in this
/// generation re-commits its members and returns without touching the
/// aggregate, so replay after commit is a no-op (§7).
pub fn commit_group(
    conn: &mut SqliteConnection,
    generation_id: i32,
    evaluator: &[u8],
    outcome: &GroupOutcome,
) -> Result<bool, StorageError> {
    let now = Utc::now().to_rfc3339();
    conn.transaction::<bool, diesel::result::Error, _>(|c| {
        let existing = app_db::fetch_application(c, generation_id, &outcome.group_key)?;
        let already_applied = existing
            .as_ref()
            .map(|r| r.status == STATUS_APPLIED)
            .unwrap_or(false);

        for m in &outcome.members {
            app_db::upsert_member(c, m)?;
        }

        if already_applied {
            // Replay after commit is a no-op on the aggregate. The member rows
            // above still land, so a LATE-ARRIVING member of an already-applied
            // group is recorded (and visible) without re-contributing.
            return Ok(false);
        }

        let attempts = existing.as_ref().map(|r| r.attempts).unwrap_or(0);
        let next_attempts = if outcome.status == STATUS_PENDING {
            attempts.saturating_add(1)
        } else {
            attempts
        };
        let next_retry_at = if outcome.status == STATUS_PENDING {
            Some(
                (Utc::now() + chrono::Duration::seconds(backoff_secs(attempts)))
                    .to_rfc3339(),
            )
        } else {
            None
        };

        app_db::upsert_application(
            c,
            &ApplicationRow {
                generation_id,
                group_key: outcome.group_key.clone(),
                status: outcome.status.clone(),
                contribution: outcome.contribution,
                subject_pubkey: outcome.subject.clone(),
                max_included_at_micros: outcome.max_included_at_micros,
                accepted: i32::from(outcome.accepted),
                attempts: next_attempts,
                next_retry_at,
                applied_at: if outcome.status == STATUS_APPLIED {
                    Some(now.clone())
                } else {
                    None
                },
                last_error: outcome.error.clone(),
                created_at: existing
                    .as_ref()
                    .map(|r| r.created_at.clone())
                    .unwrap_or_else(|| now.clone()),
            },
        )?;

        // Only an APPLIED group with a real contribution and a subject moves an
        // aggregate. A zero-only correction leaves the subject's aggregate
        // ABSENT — readers see Unknown, not Neutral (§7).
        if outcome.status == STATUS_APPLIED && outcome.contribution != 0 {
            if let Some(subject) = outcome.subject.as_ref() {
                let existing_agg = gen_db::fetch_aggregate(c, generation_id, evaluator, subject)?;
                let prior = existing_agg
                    .as_ref()
                    .map(|r| r.debit_weight_sum)
                    .unwrap_or(0);
                // CHECKED arithmetic (§7): the pre-cutover `+` on an i32 column
                // wraps in release builds, which would silently invert a score.
                let new_sum = prior.checked_add(outcome.contribution).ok_or(
                    diesel::result::Error::RollbackTransaction,
                )?;
                let prior_ts = existing_agg.as_ref().and_then(|r| r.last_signal_at_micros);
                let new_ts = match (prior_ts, outcome.max_included_at_micros) {
                    (Some(a), Some(b)) => Some(a.max(b)),
                    (a, b) => a.or(b),
                };
                gen_db::upsert_aggregate(
                    c,
                    &GenerationAggregateRow {
                        generation_id,
                        evaluator_pubkey: evaluator.to_vec(),
                        subject_pubkey: subject.clone(),
                        debit_weight_sum: new_sum,
                        last_signal_at_micros: new_ts,
                    },
                )?;
            }
        }
        Ok(true)
    })
    .map_err(|e| StorageError::Database(format!("commit_group({}): {e}", outcome.group_key)))
}

/// Publish a generation: copy its aggregates into `standing_view` and flip the
/// status, in ONE transaction.
///
/// This is the "built to completion, then atomically published" arm of §7's
/// rebuild. Readers keep seeing the previous generation until this commits;
/// they never see a half-replayed table.
pub fn publish_generation(
    conn: &mut SqliteConnection,
    generation_id: i32,
    evaluator: &[u8],
    policy_manifest_cid: &str,
) -> Result<usize, StorageError> {
    let now = Utc::now().to_rfc3339();
    conn.transaction::<usize, diesel::result::Error, _>(|c| {
        let aggregates = gen_db::list_aggregates(c, generation_id)?;
        for agg in &aggregates {
            let last_signal_at = agg
                .last_signal_at_micros
                .and_then(|micros| chrono::DateTime::<Utc>::from_timestamp_micros(micros))
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| now.clone());
            upsert_standing_view(
                c,
                &StandingViewRow {
                    evaluator_pubkey: agg.evaluator_pubkey.clone(),
                    subject_pubkey: agg.subject_pubkey.clone(),
                    score: serialize_score(score_for_debit_sum(agg.debit_weight_sum)),
                    debit_weight_sum: agg.debit_weight_sum,
                    last_signal_at,
                    manifest_cid: policy_manifest_cid.to_string(),
                },
            )?;
        }
        // Demote any previously-published generation for this evaluator FIRST,
        // so there is never a moment with two published generations.
        {
            use crate::db::diesel_schema::standing_generations::dsl as t;
            diesel::update(
                t::standing_generations
                    .filter(t::evaluator_pubkey.eq(evaluator))
                    .filter(t::status.eq(GEN_PUBLISHED))
                    .filter(t::generation_id.ne(generation_id)),
            )
            .set(t::status.eq("superseded"))
            .execute(c)?;
        }
        gen_db::set_status(c, generation_id, GEN_PUBLISHED, Some(&now))?;
        Ok(aggregates.len())
    })
    .map_err(|e| StorageError::Database(format!("publish_generation({generation_id}): {e}")))
}

/// Open (or resume) the generation this evaluator serves.
///
/// Returns the PUBLISHED generation when its pinned policy still matches; a new
/// `building` generation otherwise. A policy change never re-weights history in
/// place — it opens a new generation, which is the whole reason generations
/// exist.
pub fn resolve_generation(
    conn: &mut SqliteConnection,
    evaluator: &[u8],
    policy: &PinnedPolicy,
) -> Result<i32, StorageError> {
    let now = Utc::now().to_rfc3339();
    let digest = policy.policy_digest();
    if let Some(g) = gen_db::published_generation(conn, evaluator)
        .map_err(|e| StorageError::Database(e.to_string()))?
    {
        if g.policy_manifest_cid == digest {
            return Ok(g.generation_id);
        }
    }
    if let Some(g) = gen_db::in_flight_generation(conn, evaluator)
        .map_err(|e| StorageError::Database(e.to_string()))?
    {
        if g.policy_manifest_cid == digest {
            return Ok(g.generation_id);
        }
    }
    gen_db::create_generation(
        conn,
        evaluator,
        &digest,
        &policy.canonical_bytes(),
        gen_db::GEN_BUILDING,
        &now,
    )
    .map_err(|e| StorageError::Database(e.to_string()))
}

/// Begin a rebuild of a published generation: mark it `rebuilding` (VISIBLE to
/// readers) and clear its application + aggregate rows so the replay starts
/// clean. The subscription set and the outbox live OUTSIDE the cleared state.
pub fn begin_rebuild(conn: &mut SqliteConnection, generation_id: i32) -> Result<(), StorageError> {
    conn.transaction::<(), diesel::result::Error, _>(|c| {
        gen_db::set_status(c, generation_id, GEN_REBUILDING, None)?;
        app_db::clear_generation(c, generation_id)?;
        gen_db::clear_aggregates(c, generation_id)?;
        Ok(())
    })
    .map_err(|e| StorageError::Database(format!("begin_rebuild({generation_id}): {e}")))
}

// ---------------------------------------------------------------------------
// The sweep
// ---------------------------------------------------------------------------

/// What one tick did. Returned so a test can assert the rotation visited what
/// it should have, without reaching into the database.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TickReport {
    pub members_visited: usize,
    pub refs_seen: usize,
    pub records_fetched: usize,
    pub groups_applied: usize,
    pub groups_pending: usize,
    pub groups_rejected: usize,
}

pub struct FeedbackProjector {
    pool: DbPool,
    reader: Arc<dyn FeedbackDhtReader>,
    evaluator: Vec<u8>,
    policy: PinnedPolicy,
}

impl FeedbackProjector {
    pub fn new(
        pool: DbPool,
        reader: Arc<dyn FeedbackDhtReader>,
        evaluator: Vec<u8>,
        policy: PinnedPolicy,
    ) -> Self {
        Self {
            pool,
            reader,
            evaluator: normalize_agent_key(&evaluator),
            policy,
        }
    }

    /// One sweep. Budgets are sized BEFORE any conductor call is made.
    pub async fn tick(&self) -> Result<TickReport, StorageError> {
        let mut report = TickReport::default();
        let mut records_budget = MAX_RECORDS_PER_SWEEP;
        let mut bytes_budget = MAX_BYTES_PER_SWEEP;

        let (generation_id, members) = {
            let mut conn = self
                .pool
                .get()
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let generation_id = resolve_generation(&mut conn, &self.evaluator, &self.policy)?;
            let members = sub_db::next_members(&mut conn, MAX_MEMBERS_PER_SWEEP)
                .map_err(|e| StorageError::Database(e.to_string()))?;
            (generation_id, members)
        };

        let expected_dna = self.reader.origin_dna_hash();
        let mut last_visited: Option<(String, String)> = None;

        for member in &members {
            report.members_visited += 1;
            last_visited = Some((member.member_kind.clone(), member.member_key.clone()));

            // Mark the visit FIRST, and advance the cursor at the end of the
            // tick regardless of outcome: a member that keeps failing must
            // yield its turn, or it starves every member behind it.
            {
                let mut conn = self
                    .pool
                    .get()
                    .map_err(|e| StorageError::Database(e.to_string()))?;
                let now = Utc::now().to_rfc3339();
                sub_db::mark_visited(&mut conn, &member.member_kind, &member.member_key, &now)
                    .map_err(|e| StorageError::Database(e.to_string()))?;
            }

            let refs = match self.reader.refs_for_target(&member.member_key).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(
                        target: "elohim_storage::feedback_projector",
                        member = %member.member_key,
                        error = %e,
                        "discovery enumeration failed — the member yields its turn and is retried"
                    );
                    continue;
                }
            };
            report.refs_seen += refs.len();

            for r in refs {
                if records_budget == 0 || bytes_budget == 0 {
                    break;
                }
                match self
                    .handle_ref(generation_id, &expected_dna, member, &r, &mut bytes_budget)
                    .await
                {
                    Ok(Some(status)) => {
                        records_budget -= 1;
                        report.records_fetched += 1;
                        match status.as_str() {
                            STATUS_APPLIED => report.groups_applied += 1,
                            STATUS_REJECTED => report.groups_rejected += 1,
                            _ => report.groups_pending += 1,
                        }
                    }
                    Ok(None) => {}
                    Err(e) => tracing::warn!(
                        target: "elohim_storage::feedback_projector",
                        action = %r.action_hash,
                        error = %e,
                        "act could not be projected this tick — held, not dropped"
                    ),
                }
            }
        }

        if let Some((kind, key)) = last_visited {
            let mut conn = self
                .pool
                .get()
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let now = Utc::now().to_rfc3339();
            sub_db::write_cursor(&mut conn, Some(&kind), Some(&key), &now)
                .map_err(|e| StorageError::Database(e.to_string()))?;
        }

        Ok(report)
    }

    /// Project ONE discovered act. Returns the group status it produced, or
    /// `None` when the act needed no work this tick.
    async fn handle_ref(
        &self,
        generation_id: i32,
        expected_dna: &str,
        member: &sub_db::SubscriptionRow,
        discovered: &DiscoveredRef,
        bytes_budget: &mut usize,
    ) -> Result<Option<String>, StorageError> {
        // Already recorded as a settled member of a group in this generation?
        // Then this is a replay and there is nothing to fetch.
        {
            let mut conn = self
                .pool
                .get()
                .map_err(|e| StorageError::Database(e.to_string()))?;
            if let Some(existing) = app_db::fetch_member(
                &mut conn,
                generation_id,
                &member.origin_dna_hash,
                &discovered.action_hash,
            )
            .map_err(|e| StorageError::Database(e.to_string()))?
            {
                if existing.member_status != MEMBER_STATUS_PENDING {
                    return Ok(None);
                }
            }
        }

        let fetched = self.reader.signal_record(&discovered.action_hash).await?;
        if let Some(f) = fetched.as_ref() {
            *bytes_budget = bytes_budget.saturating_sub(f.entry_bytes_len);
        }
        let act = match verify_record(
            &discovered.action_hash,
            expected_dna,
            &member.origin_dna_hash,
            fetched,
        ) {
            ActVerification::Verified(a) => *a,
            ActVerification::Pending(reason) => {
                self.record_unsettled(
                    generation_id,
                    &member.origin_dna_hash,
                    &discovered.action_hash,
                    MEMBER_STATUS_PENDING,
                    &reason,
                )?;
                return Ok(Some(STATUS_PENDING.to_string()));
            }
            ActVerification::Rejected(reason) => {
                self.record_unsettled(
                    generation_id,
                    &member.origin_dna_hash,
                    &discovered.action_hash,
                    MEMBER_STATUS_REJECTED,
                    &reason,
                )?;
                return Ok(Some(STATUS_REJECTED.to_string()));
            }
        };

        if act.entry.signal_kind == "correction" {
            return self.project_correction(generation_id, &act).map(Some);
        }
        if is_acceptance(&act) {
            return self.project_acceptance(generation_id, &act).await.map(Some);
        }
        // Not a correction and not an acceptance: outside slice 1's scope.
        Ok(None)
    }

    /// A correction on its own. Contributes ZERO; it opens (or joins) the group
    /// and subscribes to the correction action so its acceptance is discovered.
    fn project_correction(
        &self,
        generation_id: i32,
        act: &VerifiedAct,
    ) -> Result<String, StorageError> {
        let Some(group_key) = correction_group_key(act) else {
            self.record_unsettled(
                generation_id,
                &act.origin_dna_hash,
                &act.action_hash,
                MEMBER_STATUS_REJECTED,
                "correction carries no evidence_cid — it has no operation group",
            )?;
            return Ok(STATUS_REJECTED.to_string());
        };
        let now = Utc::now().to_rfc3339();
        let member = MemberRow {
            generation_id,
            origin_dna_hash: act.origin_dna_hash.clone(),
            action_hash: act.action_hash.clone(),
            group_key: group_key.clone(),
            member_status: MEMBER_STATUS_MEMBER.to_string(),
            member_role: MEMBER_ROLE_CORRECTION.to_string(),
            author_pubkey: Some(act.author.clone()),
            action_timestamp_micros: Some(act.timestamp_micros),
            last_error: None,
            discovered_at: now.clone(),
        };

        let mut conn = self
            .pool
            .get()
            .map_err(|e| StorageError::Database(e.to_string()))?;

        // Subscribe to the CORRECTION ACTION: acceptance vouches link from it,
        // not from the content action. Without this the peer would discover the
        // allegation and never its acceptance.
        sub_db::add_member(
            &mut conn,
            sub_db::KIND_CORRECTION_ACTION,
            &act.action_hash,
            &act.origin_dna_hash,
            sub_db::SOURCE_DISCOVERED,
            &now,
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;

        let existing = app_db::fetch_application(&mut conn, generation_id, &group_key)
            .map_err(|e| StorageError::Database(e.to_string()))?;
        // An already-ACCEPTED group keeps its contribution: a late correction
        // member joins without re-contributing.
        let (status, contribution, subject, accepted) = match existing.as_ref() {
            Some(row) if row.status == STATUS_APPLIED => (
                STATUS_APPLIED.to_string(),
                row.contribution,
                row.subject_pubkey.clone(),
                row.accepted == 1,
            ),
            _ => (STATUS_APPLIED.to_string(), 0, None, false),
        };

        let outcome = GroupOutcome {
            group_key,
            status,
            contribution,
            subject,
            accepted,
            max_included_at_micros: Some(act.timestamp_micros),
            error: None,
            members: vec![member],
        };
        commit_group(&mut conn, generation_id, &self.evaluator, &outcome)?;
        Ok(STATUS_APPLIED.to_string())
    }

    /// An acceptance. This is where a group's contribution actually lands.
    async fn project_acceptance(
        &self,
        generation_id: i32,
        vouch: &VerifiedAct,
    ) -> Result<String, StorageError> {
        let correction_hash = vouch.entry.target_cid.clone();
        let expected_dna = self.reader.origin_dna_hash();

        let correction = match verify_record(
            &correction_hash,
            &expected_dna,
            &vouch.origin_dna_hash,
            self.reader.signal_record(&correction_hash).await?,
        ) {
            ActVerification::Verified(a) => *a,
            ActVerification::Pending(reason) => {
                self.record_unsettled(
                    generation_id,
                    &vouch.origin_dna_hash,
                    &vouch.action_hash,
                    MEMBER_STATUS_PENDING,
                    &reason,
                )?;
                return Ok(STATUS_PENDING.to_string());
            }
            ActVerification::Rejected(reason) => {
                self.record_unsettled(
                    generation_id,
                    &vouch.origin_dna_hash,
                    &vouch.action_hash,
                    MEMBER_STATUS_REJECTED,
                    &reason,
                )?;
                return Ok(STATUS_REJECTED.to_string());
            }
        };

        let lineage = self
            .reader
            .content_lineage(&correction.entry.target_cid)
            .await?;
        let root_author_bytes = lineage
            .as_ref()
            .and_then(|l| decode_agent_display(&l.root_author));

        let verdict = verify_acceptance(
            vouch,
            &correction,
            lineage.as_ref(),
            root_author_bytes.as_deref(),
        );

        let Some(group_key) = correction_group_key(&correction) else {
            self.record_unsettled(
                generation_id,
                &vouch.origin_dna_hash,
                &vouch.action_hash,
                MEMBER_STATUS_REJECTED,
                "the accepted correction carries no evidence_cid — it has no operation group",
            )?;
            return Ok(STATUS_REJECTED.to_string());
        };

        let now = Utc::now().to_rfc3339();
        let mut conn = self
            .pool
            .get()
            .map_err(|e| StorageError::Database(e.to_string()))?;

        match verdict {
            AcceptanceVerdict::Accepted { subject } => {
                let contribution = self.policy.contribution(
                    &correction.entry.signal_kind,
                    true,
                    &correction.entry.standing_impact,
                );
                let members = vec![
                    MemberRow {
                        generation_id,
                        origin_dna_hash: vouch.origin_dna_hash.clone(),
                        action_hash: vouch.action_hash.clone(),
                        group_key: group_key.clone(),
                        member_status: MEMBER_STATUS_MEMBER.to_string(),
                        member_role: MEMBER_ROLE_ACCEPTANCE.to_string(),
                        author_pubkey: Some(vouch.author.clone()),
                        action_timestamp_micros: Some(vouch.timestamp_micros),
                        last_error: None,
                        discovered_at: now.clone(),
                    },
                    MemberRow {
                        generation_id,
                        origin_dna_hash: correction.origin_dna_hash.clone(),
                        action_hash: correction.action_hash.clone(),
                        group_key: group_key.clone(),
                        member_status: MEMBER_STATUS_MEMBER.to_string(),
                        member_role: MEMBER_ROLE_CORRECTION.to_string(),
                        author_pubkey: Some(correction.author.clone()),
                        action_timestamp_micros: Some(correction.timestamp_micros),
                        last_error: None,
                        discovered_at: now.clone(),
                    },
                ];
                let outcome = GroupOutcome {
                    group_key,
                    status: STATUS_APPLIED.to_string(),
                    contribution,
                    subject: Some(subject),
                    accepted: true,
                    // MAXIMUM included timestamp, not the last enumerated.
                    max_included_at_micros: Some(
                        vouch.timestamp_micros.max(correction.timestamp_micros),
                    ),
                    error: None,
                    members,
                };
                commit_group(&mut conn, generation_id, &self.evaluator, &outcome)?;
                Ok(STATUS_APPLIED.to_string())
            }
            AcceptanceVerdict::Pending(reason) => {
                self.record_unsettled(
                    generation_id,
                    &vouch.origin_dna_hash,
                    &vouch.action_hash,
                    MEMBER_STATUS_PENDING,
                    &reason,
                )?;
                Ok(STATUS_PENDING.to_string())
            }
            AcceptanceVerdict::Refused(reason) => {
                self.record_unsettled(
                    generation_id,
                    &vouch.origin_dna_hash,
                    &vouch.action_hash,
                    MEMBER_STATUS_REJECTED,
                    &reason,
                )?;
                Ok(STATUS_REJECTED.to_string())
            }
        }
    }

    /// Record an act that could not settle into a group yet. The act's own
    /// action hash doubles as its group key here: a pending act belongs to no
    /// resolved group, and inventing one would fabricate a contribution.
    fn record_unsettled(
        &self,
        generation_id: i32,
        origin_dna_hash: &str,
        action_hash: &str,
        member_status: &str,
        reason: &str,
    ) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| StorageError::Database(e.to_string()))?;
        app_db::upsert_member(
            &mut conn,
            &MemberRow {
                generation_id,
                origin_dna_hash: origin_dna_hash.to_string(),
                action_hash: action_hash.to_string(),
                group_key: format!("unsettled:{action_hash}"),
                member_status: member_status.to_string(),
                member_role: MEMBER_ROLE_CORRECTION.to_string(),
                author_pubkey: None,
                action_timestamp_micros: None,
                last_error: Some(reason.to_string()),
                discovered_at: Utc::now().to_rfc3339(),
            },
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(())
    }
}

/// **Pure.** Decode a `uhCAk…` HoloHash-display agent key to its 32 raw bytes.
///
/// PANIC GUARD, load-bearing: `holo_hash`'s decoder opens with `&s[..1]` and
/// panics on an empty string, and this value arrives from a DHT answer.
pub fn decode_agent_display(display: &str) -> Option<Vec<u8>> {
    let rest = display.strip_prefix('u')?;
    if rest.is_empty() {
        return None;
    }
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(rest)
        .or_else(|_| BASE64.decode(rest))
        .ok()?;
    Some(normalize_agent_key(&raw))
}

/// One-time cutover from the pre-slice-1 immediate-debit writer (§7).
///
/// Before this contract, `api/epr.rs` debited `standing_view` the moment a
/// FeedbackSignal arrived, against the SIGNAL'S SIGNER, with no acceptance and
/// no verification. Those served values encode superseded semantics: an
/// allegation debited someone, and it debited the wrong someone.
///
/// So the cutover does two things and refuses a third:
///
/// 1. Opens generation 1 in `rebuilding` — a state readers SEE.
/// 2. CLEARS this evaluator's `standing_view` rows, so no superseded debit is
///    served past the cutover. Readers see Unknown ("this peer has not replayed
///    yet"), which is honest, rather than a number computed under rules the
///    peer no longer applies.
/// 3. Refuses to CARRY the old numbers forward as the generation's starting
///    aggregate — importing them and then replaying on top would double-count
///    every subject.
///
/// **Stated exclusion.** Extension-signal contributions
/// (`standing_projector::project_extension_signal`) are EXCLUDED from the
/// generation rather than replayed into it: they are live substrate detections,
/// not DHT-replayable inputs. They are cleared here with everything else and
/// re-accrue as new extension signals arrive. That is a declared exclusion, not
/// a silent loss.
///
/// Idempotent: a second call with a generation already present is a no-op.
pub fn cutover_standing_view(
    conn: &mut SqliteConnection,
    evaluator: &[u8],
    policy: &PinnedPolicy,
) -> Result<Option<i32>, StorageError> {
    let evaluator = normalize_agent_key(evaluator);
    let already = gen_db::published_generation(conn, &evaluator)
        .map_err(|e| StorageError::Database(e.to_string()))?
        .or(gen_db::in_flight_generation(conn, &evaluator)
            .map_err(|e| StorageError::Database(e.to_string()))?);
    if already.is_some() {
        return Ok(None);
    }
    let now = Utc::now().to_rfc3339();
    let digest = policy.policy_digest();
    let canonical = policy.canonical_bytes();
    let evaluator_for_tx = evaluator.clone();
    conn.transaction::<i32, diesel::result::Error, _>(move |c| {
        let generation_id = gen_db::create_generation(
            c,
            &evaluator_for_tx,
            &digest,
            &canonical,
            GEN_REBUILDING,
            &now,
        )?;
        use crate::db::diesel_schema::standing_view::dsl as sv;
        diesel::delete(sv::standing_view.filter(sv::evaluator_pubkey.eq(&evaluator_for_tx)))
            .execute(c)?;
        Ok(generation_id)
    })
    .map(Some)
    .map_err(|e| StorageError::Database(format!("cutover_standing_view: {e}")))
}

/// Enqueue a NOTIFIED act for fetch-and-verify (§4 → §3).
///
/// This replaces the pre-cutover immediate debit at `api/epr.rs`. A semantic
/// payload that arrived over the wire is a CLAIM: `signed_by` is a claim, the
/// signature is over the semantic bytes and not over the action, and the sender
/// is not the authority. Nothing here writes standing. The act joins the
/// durable subscription set, and the projector fetches the SIGNED record and
/// verifies §1 for itself.
///
/// Returns `Ok(false)` when the notification carries no act reference — a
/// pre-slice-1 peer's message, which is not projected and is not an error.
pub fn admit_notified_signal(
    conn: &mut SqliteConnection,
    act_ref: Option<&crate::p2p::feedback_signal::FeedbackActRef>,
    local_dna_hash: Option<&str>,
) -> Result<bool, StorageError> {
    let Some(act_ref) = act_ref else {
        return Ok(false);
    };
    let Some(local) = local_dna_hash else {
        return Ok(false);
    };
    if act_ref.origin_dna_hash != local {
        return Err(StorageError::InvalidInput(format!(
            "feedback act reference names foreign origin DNA '{}' (this cell is '{local}') —              cross-context evidence is not slice 1",
            act_ref.origin_dna_hash
        )));
    }
    let now = Utc::now().to_rfc3339();
    for (kind, key) in [
        (sub_db::KIND_CORRECTION_ACTION, act_ref.action_hash.as_str()),
        (sub_db::KIND_CONTENT_TARGET, act_ref.routing_key.as_str()),
    ] {
        sub_db::add_member(
            conn,
            kind,
            key,
            &act_ref.origin_dna_hash,
            sub_db::SOURCE_NOTIFIED,
            &now,
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;
    }
    Ok(true)
}

/// Spawn the sweep loop.
///
/// bounded-work: one tick per [`SWEEP_INTERVAL_SECS`], `MissedTickBehavior::Skip`
/// so a stalled runtime coalesces missed ticks instead of catching up in a
/// burst, and every per-tick cost capped by the budgets above.
pub fn spawn(projector: FeedbackProjector) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(SWEEP_INTERVAL_SECS));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            match projector.tick().await {
                Ok(report) if report.members_visited > 0 => tracing::debug!(
                    target: "elohim_storage::feedback_projector",
                    ?report,
                    "feedback projector sweep"
                ),
                Ok(_) => {}
                Err(e) => tracing::warn!(
                    target: "elohim_storage::feedback_projector",
                    error = %e,
                    "feedback projector sweep failed — retried on the next tick"
                ),
            }
        }
    });
}

#[cfg(test)]
mod tests;
