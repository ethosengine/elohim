//! Provide-loop reconciler (Slice 2b) — the P1 controller that turns
//! caught-up commons pins into notarized `replicates-commons` Commitments.
//!
//! ## Desired vs actual (P1 reconciliation)
//!
//! - **desired** = active `item` pins whose acquisition tracker reports
//!   caught-up AND whose content has `reach == "commons"` (you can only offer
//!   to the commons what you have fully fetched and what the commons may hold).
//! - **actual** = this provider's non-revoked `replicates-commons` Commitments,
//!   keyed by the LOGICAL key `(provider, head_ref)` (head_ref == recipient on
//!   the commitment row).
//!
//! The diff authors a Commitment only when a desired key has NO live actual
//! row. It never revokes: absence from the desired set is not evidence that
//! the offer was withdrawn (content may be missing, a pin may be held back).
//! A scope this node revoked is re-offered as a RENEWAL ([`ProvideRenewal`]):
//! a new commitment whose payload names the one it renews, so its cid is new.
//!
//! ## The revoke arm (removed pins only)
//!
//! A revocation is notarized and irreversible, so
//! [`ProvideReconciler::revoke_withdrawn`] acts only on POSITIVE evidence that
//! the operator withdrew a scope, and is off unless switched on
//! ([`REVOKE_ARM_ENV`]):
//!
//! - a scope is **withdrawn** when it has a pin whose status is `removed` and
//!   no pin in any other status ([`withdrawn_scopes`]). No pin row, a `retired`
//!   or paused pin, missing content and failed eligibility are not evidence;
//! - scopes are compared through [`scope_key`] on both sides (pins are stored
//!   bare, commitments keep the spelling they were authored with);
//! - only rows provided by the agent key the revocation would be signed with
//!   ([`CommitmentRevoker::revoke_authority`]) are considered, and only rows
//!   whose bounds positively read as a content provide for that scope;
//! - a row must be seen withdrawn on [`REVOKE_AFTER_STRANDED_PASSES`] passes
//!   in a row, and any pass that could not observe starts every count over;
//! - at most [`REVOKES_PER_TICK`] submissions a pass;
//! - every submission goes through [`submit_revocation`], the protocol the
//!   HTTP un-pin route uses too: a durable claim in `mishpat_revocations`
//!   before the call (so the route and the arm never both revoke one row), the
//!   evidence re-checked AFTER the call's wait for the chain and the conductor
//!   and immediately before it is offered, and an unknown outcome retried a
//!   bounded number of times and then reported, never silently dropped;
//! - the revocation targets the row's `cid` (the commitment's entry hash),
//!   which is what the projection matches to set `revoked_at`.
//!
//! ## Restart safety (the load-bearing property)
//!
//! The in-memory `latch` is a pure optimisation. On process restart it is
//! empty, but the actual set is re-derived from the DHT projection
//! (`live_commons_provides_for_provider`), so a key already provided before the
//! restart is found in `actual` and is NOT re-authored. LOGICAL-KEY dedup —
//! not the latch — is the author-once guarantee. The latch only suppresses
//! redundant authoring attempts *within* a process lifetime.
//!
//! Authoring rides a [`CommitmentAuthor`] seam so the dedup/idempotency/restart
//! logic is unit-testable without a live conductor (mirrors `CommitmentFetcher`).
//!
//! Spec: 2026-06-08-epr-acquisition-slice2b-provide-loop-design.md.

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::DbPool;
use crate::error::StorageError;

/// Pin `kind` the provide loop authors offers for (single-item pins). Cluster
/// pins are a later slice and never enter the desired set.
const PIN_KIND_ITEM: &str = "item";
/// Pin `status` that is eligible for a commons offer (removed/paused pins are
/// excluded; a removed pin's offer is revoked, not re-authored).
const PIN_STATUS_ACTIVE: &str = "active";

/// Pin `status` the HTTP un-pin writes: the operator's explicit withdrawal, and
/// the only pin state the revoke arm treats as evidence.
pub const PIN_STATUS_REMOVED: &str = "removed";

/// Passes in a row a row must be seen withdrawn before it is revoked. The tick
/// is 60 s, so a pin removed and restored within two minutes costs nothing, and
/// the first passes after boot never revoke. The un-pin route revokes at once
/// and does not wait on this.
pub const REVOKE_AFTER_STRANDED_PASSES: u32 = 3;

/// Most revocations one pass may submit. Each is one `create_commitment` on the
/// node's single mishpat source chain, taken one at a time behind the chain
/// write gate, so four keeps the provide writer's turn at the gate to a few
/// conductor round trips, and a mistake costs at most four irreversible acts a
/// minute while the operator reaches the switch.
pub const REVOKES_PER_TICK: usize = 4;

/// Environment switch for the revoke arm, read once at startup:
/// `off` (the default) | `dry-run` | `apply`.
pub const REVOKE_ARM_ENV: &str = "ELOHIM_PROVIDE_REVOKE_ARM";

/// What the revoke arm does on a pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RevokeArmMode {
    /// Does nothing and reads nothing.
    #[default]
    Off,
    /// Computes the plan `Apply` would act on and logs it. No conductor call.
    DryRun,
    /// Submits the planned revocations.
    Apply,
}

impl RevokeArmMode {
    /// Parse the raw value of [`REVOKE_ARM_ENV`]. Anything that is not exactly
    /// `apply` or `dry-run` (case and surrounding space aside) is `Off`.
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
            Some("apply") => Self::Apply,
            Some("dry-run") => Self::DryRun,
            _ => Self::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::DryRun => "dry-run",
            Self::Apply => "apply",
        }
    }
}

/// The evidence that a scope was withdrawn: the removed pin that says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithdrawnScope {
    pub pin_id: i32,
    /// When the pin was removed (its `updated_at`).
    pub removed_at: String,
}

/// The scopes the operator withdrew, keyed by [`scope_key`]: those with a pin
/// in status `removed` and NO pin (of any kind, under any agent) in any other
/// status. `pins` must be every pin row; an empty table withdraws nothing.
pub fn withdrawn_scopes(
    pins: &[crate::db::models::AcquisitionPin],
) -> HashMap<String, WithdrawnScope> {
    let mut held: HashSet<&str> = HashSet::new();
    let mut removed: HashMap<&str, &crate::db::models::AcquisitionPin> = HashMap::new();
    for pin in pins {
        let key = scope_key(&pin.head_ref);
        if pin.status == PIN_STATUS_REMOVED {
            let latest = removed.entry(key).or_insert(pin);
            if pin.updated_at > latest.updated_at {
                *latest = pin;
            }
        } else {
            held.insert(key);
        }
    }
    removed
        .into_iter()
        .filter(|(key, _)| !held.contains(key))
        .map(|(key, pin)| {
            (
                key.to_string(),
                WithdrawnScope {
                    pin_id: pin.id,
                    removed_at: pin.updated_at.clone(),
                },
            )
        })
        .collect()
}

/// One revocation to submit: a commitment of this node for a withdrawn scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedRevocation {
    /// The commitment's `cid`: its entry hash, the revocation's target.
    pub cid: String,
    /// The commitment's scope as it spells it.
    pub head_ref: String,
    /// The commitment's provider: the agent key the revocation is signed with.
    pub provider: String,
    pub pin_id: i32,
    /// The removed pin's `updated_at`. The first claim fixes it as the
    /// revocation's `signed_at`.
    pub removed_at: String,
}

/// What one pass of the revoke arm saw and did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RevokeSweep {
    /// False when the arm is off or the pass could not observe (and so reset
    /// every count).
    pub observed: bool,
    /// Live rows of this node for withdrawn scopes with no settled submission.
    pub withdrawn_rows: usize,
    /// The rows that have been seen long enough to revoke.
    pub planned: Vec<PlannedRevocation>,
    /// Cids whose revocation the conductor accepted this pass.
    pub issued: Vec<String>,
    /// Cids whose revocation call returned an error this pass. Its outcome is
    /// unknown; it is attempted again after the settle time, up to the bound.
    pub unconfirmed: Vec<String>,
    /// Cids whose evidence no longer held at the last moment before the call
    /// (a re-pin while the call waited for the chain): nothing was submitted.
    pub withdrawn_at_the_last_moment: Vec<String>,
    /// Cids out of attempts with no revocation projected: this node may or may
    /// not have revoked them. Reported on every pass until the operator acts.
    pub stuck: Vec<String>,
}

/// What one submission through [`submit_revocation`] came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Submission {
    /// The conductor accepted the revocation.
    Issued,
    /// The call returned an error; its outcome is unknown and recorded so.
    Unconfirmed(String),
    /// Nothing was submitted: another attempt is in flight, one was accepted,
    /// a revocation is projected, or the target is out of attempts.
    NotDue,
    /// Nothing was submitted: the evidence did not hold (before the claim, or
    /// at the last moment before the call).
    NoLongerHolds(String),
}

/// Does the revocation `plan` describes still hold, read fresh now? The scope
/// must still be withdrawn ([`withdrawn_scopes`]), no revocation of the target
/// may be projected, and a projected target row must be live and provided by
/// `plan.provider`. A target the projection has not seen yet (the un-pin route
/// can run before its commitment's signal lands) is judged on the pins alone.
pub fn revocation_still_holds(
    conn: &mut diesel::sqlite::SqliteConnection,
    plan: &PlannedRevocation,
) -> Result<(), String> {
    let pins =
        crate::db::acquisition_pins::list_all_pins(conn).map_err(|e| format!("pin read: {e}"))?;
    if !withdrawn_scopes(&pins).contains_key(scope_key(&plan.head_ref)) {
        return Err(format!(
            "scope {} is pinned again (or no longer withdrawn)",
            plan.head_ref
        ));
    }
    if crate::db::mishpat_revocations::projected_revoked_at(conn, &plan.cid)
        .map_err(|e| format!("revocation read: {e}"))?
        .is_some()
    {
        return Err("a revocation of the target is already projected".to_string());
    }
    if let Some(row) = crate::db::mishpat_commitments::get_by_cid(conn, &plan.cid)
        .map_err(|e| format!("commitment read: {e}"))?
    {
        if row.revoked_at.is_some() {
            return Err("the target is already revoked".to_string());
        }
        if row.provider != plan.provider {
            return Err(format!(
                "the target is provided by {}, not {}",
                row.provider, plan.provider
            ));
        }
    }
    Ok(())
}

/// Submit the revocation `plan` describes, through the one protocol both
/// submitters use: the removed-pin arm and the HTTP un-pin route.
///
/// 1. The evidence is read fresh ([`revocation_still_holds`]) and the signing
///    key must be the target's provider.
/// 2. The attempt is claimed in `mishpat_revocations`
///    ([`crate::db::mishpat_revocations::claim`]). Only one submitter holds a
///    target's attempt at a time; the first claim fixes `signed_at`, so every
///    attempt is the same entry.
/// 3. The call is made with the evidence check as its precondition, run by the
///    revoker AFTER the call has waited for the chain write gate and the
///    conductor admission permit, immediately before the call is offered. A
///    scope re-pinned during that wait is not revoked, and the claim is given
///    back ([`crate::db::mishpat_revocations::release`]).
/// 4. The answer is recorded: `issued`, or `unconfirmed` (the call may have
///    committed; a later claim retries it after the settle time, bounded).
pub async fn submit_revocation<R: CommitmentRevoker + ?Sized>(
    pool: &DbPool,
    revoker: &R,
    plan: &PlannedRevocation,
    writer: crate::chain_write_gate::WriterKind,
    now: chrono::DateTime<chrono::Utc>,
) -> Submission {
    if revoker.revoke_authority().await.as_deref() != Some(plan.provider.as_str()) {
        return Submission::NoLongerHolds(
            "the signing key is not the target's provider".to_string(),
        );
    }
    let claimed = pool.get().map_err(|e| e.to_string()).and_then(|mut conn| {
        revocation_still_holds(&mut conn, plan)?;
        crate::db::mishpat_revocations::claim(&mut conn, &plan.cid, &plan.removed_at, now)
            .map_err(|e| format!("revocation record write: {e}"))
            .map(Some)
    });
    let (signed_at, attempt) = match claimed {
        Ok(Some(crate::db::mishpat_revocations::Claim::Submit { signed_at, attempt })) => {
            (signed_at, attempt)
        }
        Ok(_) => return Submission::NotDue,
        Err(reason) => return Submission::NoLongerHolds(reason),
    };

    let precondition_pool = pool.clone();
    let precondition_plan = plan.clone();
    let precondition = move || -> Result<(), String> {
        let mut conn = precondition_pool.get().map_err(|e| format!("pool: {e}"))?;
        revocation_still_holds(&mut conn, &precondition_plan)
    };
    let req = ProvideRevokeRequest {
        target_cid: plan.cid.clone(),
        head_ref: plan.head_ref.clone(),
        provider: plan.provider.clone(),
        signed_at,
    };
    let outcome =
        crate::chain_write_gate::as_writer(writer, revoker.revoke_commons(&req, &precondition))
            .await;

    let record = |outcome_label: Option<&str>| {
        let written = pool.get().map_err(|e| e.to_string()).and_then(|mut conn| {
            let recorded = match outcome_label {
                Some(label) => {
                    crate::db::mishpat_revocations::finish(&mut conn, &plan.cid, attempt, label)
                }
                None => crate::db::mishpat_revocations::release(&mut conn, &plan.cid, attempt),
            };
            recorded.map_err(|e| e.to_string())
        });
        if let Err(e) = written {
            // The claim stays `pending`: a later claim takes it as an unknown
            // outcome, so nothing is lost, at the cost of one more attempt.
            tracing::warn!(
                target: "elohim_storage::provide",
                target_cid = %plan.cid,
                error = %e,
                "provide revoke: outcome not recorded; the attempt stays pending and is retried after the settle time"
            );
        }
    };
    match outcome {
        Ok(()) => {
            record(Some(crate::db::mishpat_revocations::ISSUE_ISSUED));
            Submission::Issued
        }
        Err(RevokeError::Refused(reason)) => {
            record(None);
            Submission::NoLongerHolds(reason)
        }
        Err(RevokeError::Failed(e)) => {
            record(Some(crate::db::mishpat_revocations::ISSUE_UNCONFIRMED));
            Submission::Unconfirmed(e.to_string())
        }
    }
}

/// Lifecycle of one logical provide `(provider, head_ref)` as the reconciler
/// observes it. Category C (in-memory; recomputed on restart from the
/// projection + pin tables). `Projected`/`Active` distinguish a freshly
/// authored-but-ungraduated commitment from one a provide event has graduated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvideStage {
    /// Desired but no live commitment yet — the next tick will author.
    NeedsCommitment,
    /// Author call issued this lifetime; awaiting projection.
    Authoring,
    /// A live `replicates-commons` row exists (proposed).
    Projected,
    /// Announce step in flight (reserved for the gossip-announce follow-on).
    Announcing,
    /// A live row exists and has graduated to 'active'.
    Active,
    /// The pin was removed; a revocation has been authored.
    Revoked,
}

/// A single logical provide the reconciler decided to author or revoke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvideAuthorRequest {
    pub provider: String,
    pub head_ref: String,
    /// The CONTENT's own reach (Stage B), threaded from the desired pin's
    /// content row. The author writes this as the commitment's top-level
    /// `reach` (the `reach_ceiling` bound stays "commons"); the projection scopes
    /// the `content:<reach>` provide row by it. Defaults to "commons" when the
    /// caller has no per-content reach (back-compat with the commons loop).
    pub reach: String,
    /// `Some` when this provide renews a commitment of this node for the same
    /// scope that was revoked (see [`ProvideRenewal`]); `None` for a provide
    /// with no revoked predecessor, whose payload is exactly what it was before
    /// renewals existed, so its cid is unchanged.
    pub renewal: Option<ProvideRenewal>,
}

/// The lineage a renewed provide commitment carries.
///
/// A commitment's cid is the entry hash of its content, and the author buckets
/// validity to the UTC day, so without this a scope re-pinned on the day its
/// commitment was revoked would re-author the revoked entry byte for byte: the
/// same cid, which the projection keeps revoked. A renewal is a NEW commitment
/// by an explicit sequence: its payload names the commitment it renews and its
/// place in the lineage, so its cid differs from every predecessor's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvideRenewal {
    /// 1 for the first renewal of an unrenewed commitment, then one more than
    /// the commitment it renews. An unrenewed commitment is sequence 0 and
    /// carries no renewal fields.
    pub sequence: u64,
    /// The cid (entry hash) of the commitment this one renews.
    pub supersedes: String,
}

/// A revocation of one commitment of this node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvideRevokeRequest {
    /// `cid` of the commitment to revoke: its ENTRY hash, the key the revoke
    /// projection matches. Never the action hash (`dht_anchor_hash`).
    pub target_cid: String,
    pub head_ref: String,
    /// The target's provider. The revoker refuses unless this is the agent key
    /// it signs with.
    pub provider: String,
    /// The revocation's `signed_at`, fixed by the first claim of the target
    /// (the time its pin was removed), so every attempt carries the same entry.
    pub signed_at: String,
}

/// Run by a revoker immediately before it offers the conductor call: after the
/// call has waited for the chain write gate and the admission permit. `Err`
/// (with the reason) = the revocation no longer holds; submit nothing.
pub type RevokePrecondition = dyn Fn() -> Result<(), String> + Send + Sync;

/// Why a revocation was not accepted.
#[derive(Debug)]
pub enum RevokeError {
    /// Nothing was submitted: the precondition or the signing-key check
    /// refused before the conductor was called.
    Refused(String),
    /// The call was made, or may have been, and did not answer with success.
    /// It may have committed.
    Failed(StorageError),
}

/// The revoking half of a [`CommitmentAuthor`], alone, so the HTTP un-pin route
/// revokes through the same seam (and the same claim protocol,
/// [`submit_revocation`]) as the reconciler's arm without an authoring context.
#[async_trait]
pub trait CommitmentRevoker: Send + Sync {
    /// Author a revokes-commitment targeting `req.target_cid`, running
    /// `precondition` immediately before the conductor call is offered.
    async fn revoke_commons(
        &self,
        req: &ProvideRevokeRequest,
        precondition: &RevokePrecondition,
    ) -> Result<(), RevokeError>;

    /// The agent key (`uhCAk…`) a revocation from this revoker is signed with:
    /// the conductor cell's own key, never a session key. Only rows this key
    /// provides are revoked. `None` = not established right now.
    async fn revoke_authority(&self) -> Option<String>;
}

/// Author seam — the conductor write of a `replicates-commons` Commitment, and
/// (through [`CommitmentRevoker`]) of a `revokes-commitment`. Production impl
/// wraps `conductor_writes`; tests use [`MockAuthor`] to assert exactly-once
/// authoring across ticks and restarts.
#[async_trait]
pub trait CommitmentAuthor: CommitmentRevoker {
    /// Author a replicates-commons Commitment for the given provide. Returns
    /// the new commitment action_hash (the back-reference stored on the pin).
    async fn author_commons(&self, req: &ProvideAuthorRequest) -> Result<String, StorageError>;

    /// The agent_cid (`uhCAk…`) provider keys this author writes commitments
    /// under, preferred first. These are the values the projection stores in
    /// `mishpat_commitments.provider`, so the reconciler dedups against them,
    /// not against the transport id. Empty = none resolvable right now.
    async fn own_providers(&self) -> Vec<String>;
}

/// In-memory author for unit tests. Records every author/revoke call so a test
/// can assert exactly-once semantics.
#[derive(Debug, Default)]
pub struct MockAuthor {
    pub authored: Mutex<Vec<ProvideAuthorRequest>>,
    pub revoked: Mutex<Vec<ProvideRevokeRequest>>,
    next_cid: std::sync::atomic::AtomicU64,
    /// Agent-namespace providers this mock "writes under". `None` = the default
    /// [`MockAuthor::DEFAULT_PROVIDER`].
    providers: Option<Vec<String>>,
    /// The key this mock "signs revocations with". `None` = the default
    /// [`MockAuthor::DEFAULT_PROVIDER`]; `Some(None)` = not established.
    authority: Option<Option<String>>,
}

impl MockAuthor {
    /// Agent-cid-shaped, deliberately unlike any transport-shaped `self_provider`
    /// the tests pass: the two namespaces must not share a string.
    pub const DEFAULT_PROVIDER: &'static str = "uhCAkMockAgentProvider";

    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_providers(providers: Vec<String>) -> Self {
        Self {
            providers: Some(providers),
            ..Self::default()
        }
    }
    pub fn with_authority(mut self, authority: Option<&str>) -> Self {
        self.authority = Some(authority.map(str::to_string));
        self
    }
    pub async fn authored_keys(&self) -> Vec<(String, String)> {
        self.authored
            .lock()
            .await
            .iter()
            .map(|r| (r.provider.clone(), r.head_ref.clone()))
            .collect()
    }
}

#[async_trait]
impl CommitmentAuthor for MockAuthor {
    async fn own_providers(&self) -> Vec<String> {
        self.providers
            .clone()
            .unwrap_or_else(|| vec![Self::DEFAULT_PROVIDER.to_string()])
    }
    async fn author_commons(&self, req: &ProvideAuthorRequest) -> Result<String, StorageError> {
        self.authored.lock().await.push(req.clone());
        let n = self
            .next_cid
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(format!("uhCkk-mock-commit-{n}"))
    }
}

#[async_trait]
impl CommitmentRevoker for MockAuthor {
    async fn revoke_authority(&self) -> Option<String> {
        self.authority
            .clone()
            .unwrap_or_else(|| Some(Self::DEFAULT_PROVIDER.to_string()))
    }
    async fn revoke_commons(
        &self,
        req: &ProvideRevokeRequest,
        precondition: &RevokePrecondition,
    ) -> Result<(), RevokeError> {
        precondition().map_err(RevokeError::Refused)?;
        self.revoked.lock().await.push(req.clone());
        Ok(())
    }
}

/// Reach-aware provide-eligibility seam.
///
/// Off-commons, "may this node provide content X at reach R?" is a privacy
/// question, not just a presence question: a household-reach record is not
/// openly providable; the node must have **embodied responsibility** for the
/// scope (the same predicate that decides receiver-side pre-authorization — you
/// may provide a scope iff you have standing to hold it). Commons stays openly
/// providable.
///
/// The seam takes `(head_ref, reach)` candidates (caught-up AND locally present)
/// and returns the head_refs the node is eligible to provide. Extracting it
/// behind a trait (mirroring [`CommitmentAuthor`]/[`MockAuthor`]) lets the
/// admit/reject decision be unit-tested without a live DB pool — the production
/// resolver reads `peer_identity_bindings`; the mock injects the eligible set.
pub trait ProvideEligibility: Send + Sync {
    /// Filter `candidates` (`(head_ref, reach)`) to the head_refs this node is
    /// eligible to provide.
    fn eligible_head_refs(&self, candidates: &[(String, String)]) -> HashSet<String>;
}

/// Production eligibility resolver: builds the scope topic per candidate and
/// runs [`classify_pre_authorization`](crate::p2p::classify_pre_authorization).
///
/// `node_has_embodied_responsibility` is resolved ONCE (Stage 1 ignores the
/// topic — it answers "does this node have any standing at all"); Stage 2/3
/// will thread per-scope graph walks through the same call without changing
/// this seam. The pillar is supplied here (no `pillar` column on content);
/// `community` reach additionally carries the collective segment when known
/// (none at Stage 1 — `None`), riding the topic ladder so Stage 2/3 tightening
/// flows through with no call-site rewrite.
///
/// Gated on the `p2p` feature: the classifier + topic machinery live in the
/// `p2p` module. The `ProvideEligibility` trait itself is ungated so a
/// non-p2p build (and the unit tests) can still inject a mock resolver.
#[cfg(feature = "p2p")]
pub struct ClassifierEligibility {
    pool: Arc<DbPool>,
    pillar: String,
}

#[cfg(feature = "p2p")]
impl ClassifierEligibility {
    pub fn new(pool: Arc<DbPool>, pillar: impl Into<String>) -> Self {
        Self {
            pool,
            pillar: pillar.into(),
        }
    }
}

#[cfg(feature = "p2p")]
impl ProvideEligibility for ClassifierEligibility {
    fn eligible_head_refs(&self, candidates: &[(String, String)]) -> HashSet<String> {
        use crate::p2p::{classify_pre_authorization, topics::topic_for, PreAuthorizationDecision};

        if candidates.is_empty() {
            return HashSet::new();
        }
        // Resolve the embodied-responsibility boolean once. NOTE (flag, not
        // fixed here): this resolver fails OPEN on pool/query error (returns
        // true), so under DB stress it can admit a node that lacks standing —
        // existing Stage 1 behavior, tracked with the receiver-side gate, not
        // re-designed in this change.
        let has_responsibility =
            crate::p2p::reach_authorization::node_has_embodied_responsibility(&self.pool, "");

        candidates
            .iter()
            .filter(|(_head_ref, reach)| {
                // Stage B DEGRADE-OPEN: a reach OUTSIDE the schema-8 DNA
                // vocabulary (`local`, `household`, `neighborhood`, …) does not
                // parse to `elohim_epr::Reach` and `topic_for` is `Reach`-typed,
                // so we cannot build its canonical topic. Rather than DROP it
                // (the Stage-A storage projection already reads such reaches
                // through, so dropping here would silently starve household/local
                // content of a provide author — the regression this prevents), we
                // ADMIT it. Consistent with the storage-side degrade-open: the
                // schema-8 vocabulary makes no claim about the others.
                let Some(reach_enum) = parse_reach_for_topic(reach) else {
                    return true;
                };
                let topic = topic_for(&self.pillar, reach_enum, None);
                matches!(
                    classify_pre_authorization(&topic, has_responsibility),
                    PreAuthorizationDecision::Standing
                )
            })
            .map(|(head_ref, _reach)| head_ref.clone())
            .collect()
    }
}

/// Parse a reach string to [`elohim_epr::Reach`] for topic construction.
/// Mirrors the storage-side read-through (`mishpat_projection::parse_reach`):
/// membership in the canonical reach vocabulary is "the parse succeeds".
#[cfg(feature = "p2p")]
fn parse_reach_for_topic(reach: &str) -> Option<elohim_epr::Reach> {
    serde_json::from_value::<elohim_epr::Reach>(serde_json::Value::String(reach.to_string())).ok()
}

/// Does `row` still stand at `now`? A `valid_until` that does not parse counts
/// as standing (never re-author on unreadable data); that is logged once.
fn row_unexpired(
    row: &crate::db::models::MishpatCommitment,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    match chrono::DateTime::parse_from_rfc3339(&row.valid_until) {
        Ok(until) => until.with_timezone(&chrono::Utc) > now,
        Err(_) => {
            static WARNED: std::sync::atomic::AtomicBool =
                std::sync::atomic::AtomicBool::new(false);
            if !WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                tracing::warn!(
                    target: "elohim_storage::provide",
                    cid = %row.cid,
                    valid_until = %row.valid_until,
                    "provide reconcile: unreadable valid_until on a commitment row; counting it as live"
                );
            }
            true
        }
    }
}

/// Does the content reach the projection stored on `row` (`bounds_json.reach`)
/// equal `desired_reach`? A row whose reach cannot be read counts as a match.
fn row_reach_matches(row: &crate::db::models::MishpatCommitment, desired_reach: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(&row.bounds_json)
        .ok()
        .and_then(|b| b.get("reach").and_then(|r| r.as_str()).map(str::to_string))
        .is_none_or(|stored| stored == desired_reach)
}

/// Is `row` POSITIVELY a content provide for its own recipient? Its bounds must
/// parse to an object that scopes the commitment to exactly that recipient
/// (`epr_scope: [recipient]`, as the projection writes a content commitment)
/// and carry no `commons_bytes` (the capacity pledge, projected under the same
/// action). Malformed, null or unrecognised bounds are not content.
fn is_content_provide(row: &crate::db::models::MishpatCommitment) -> bool {
    if row.recipient.is_empty() {
        return false;
    }
    let Ok(serde_json::Value::Object(bounds)) =
        serde_json::from_str::<serde_json::Value>(&row.bounds_json)
    else {
        return false;
    };
    if bounds.contains_key("commons_bytes") {
        return false;
    }
    matches!(
        bounds.get("epr_scope"),
        Some(serde_json::Value::Array(scope))
            if scope.len() == 1 && scope[0].as_str() == Some(row.recipient.as_str())
    )
}

/// May the revoke arm act on `row` at `now`? Only when its `valid_until` reads
/// as a time still ahead: a lapsed offer needs no revocation, and a row whose
/// validity cannot be read is left alone (the opposite default to
/// [`row_unexpired`], because here the doubt must stop a write).
fn row_stands_readably(
    row: &crate::db::models::MishpatCommitment,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    chrono::DateTime::parse_from_rfc3339(&row.valid_until)
        .is_ok_and(|until| until.with_timezone(&chrono::Utc) > now)
}

/// The one spelling a scope is compared in. The pin API stores a head_ref bare
/// (`X`); a commitment keeps the spelling it was authored with (`epr:X` or
/// `X`). Both name the same scope.
pub fn scope_key(head_ref: &str) -> &str {
    head_ref.strip_prefix("epr:").unwrap_or(head_ref)
}

/// A row's place in its scope's lineage: the `renewal` its payload carried,
/// projected into `bounds_json`; 0 for an unrenewed commitment.
fn renewal_sequence(row: &crate::db::models::MishpatCommitment) -> u64 {
    serde_json::from_str::<serde_json::Value>(&row.bounds_json)
        .ok()
        .and_then(|b| b.get("renewal").and_then(|v| v.as_u64()))
        .unwrap_or(0)
}

/// The renewal a new provide for a scope carries, given every commitment this
/// node's providers made for that scope (revoked or not).
///
/// The scope's lineage head is its row with the highest renewal sequence, then
/// the latest `valid_from`, then the latest projection, then the greatest cid.
/// A new provide renews the head when the head was revoked, or when the head is
/// itself a renewal (so a lineage, once begun, never falls back to sequence 0
/// and re-authors a cid it already used). Otherwise (no history, or an
/// unrevoked unrenewed head: a lapse or a reach change) it is unrenewed and
/// keeps today's identity, so nothing that was never revoked re-keys.
pub fn renewal_for(history: &[crate::db::models::MishpatCommitment]) -> Option<ProvideRenewal> {
    let head = history.iter().max_by(|a, b| {
        renewal_sequence(a)
            .cmp(&renewal_sequence(b))
            .then_with(|| a.valid_from.cmp(&b.valid_from))
            .then_with(|| a.created_at.cmp(&b.created_at))
            .then_with(|| a.cid.cmp(&b.cid))
    })?;
    let sequence = renewal_sequence(head);
    (head.revoked_at.is_some() || sequence > 0).then(|| ProvideRenewal {
        sequence: sequence.saturating_add(1),
        supersedes: head.cid.clone(),
    })
}

/// One desired provide derived from a caught-up pin.
#[derive(Debug, Clone)]
pub struct DesiredProvide {
    pub pin_id: i32,
    pub head_ref: String,
    /// The CONTENT's own reach (Stage B), carried from the eligibility
    /// candidates `(head_ref, reach)` map so the authored commitment declares the
    /// content's reach rather than a hardcoded "commons". Defaults to "commons"
    /// when the caller supplies no per-content reach map.
    pub reach: String,
    /// Pre-existing commitment_cid back-reference (set on a prior tick).
    ///
    /// Read by the T10 un-pin revoke path (`http::handle_remove_pin`), which
    /// targets this CID directly to author a `revokes-commitment` when the pin is
    /// removed. Now wired — kept here so the desired set carries the back-ref
    /// even for callers that only observe.
    pub commitment_cid: Option<String>,
}

/// The provide-loop controller. Holds the per-process stage latch keyed by the
/// LOGICAL key `(provider, head_ref)`.
pub struct ProvideReconciler {
    /// `(provider, head_ref)` → stage. Pure optimisation — emptied on restart.
    latch: Arc<Mutex<HashMap<(String, String), ProvideStage>>>,
    /// What the revoke arm does; fixed at construction.
    revoke_mode: RevokeArmMode,
    /// Row `cid` → passes in a row it has been seen withdrawn. Cleared by any
    /// pass that could not observe. Whether a revocation was SUBMITTED is not
    /// kept here: that is durable, in `mishpat_revocations`.
    sightings: Arc<Mutex<HashMap<String, u32>>>,
    /// Cids a dry run has already logged, so each is logged once a process.
    dry_run_logged: Arc<Mutex<HashSet<String>>>,
    /// Stuck targets already warned about this process (they are reported on
    /// every pass's [`RevokeSweep`] regardless).
    stuck_reported: Arc<Mutex<HashSet<String>>>,
}

impl Default for ProvideReconciler {
    fn default() -> Self {
        Self::new()
    }
}

impl ProvideReconciler {
    /// A reconciler whose revoke arm is off.
    pub fn new() -> Self {
        Self::with_revoke_mode(RevokeArmMode::Off)
    }

    pub fn with_revoke_mode(revoke_mode: RevokeArmMode) -> Self {
        Self {
            latch: Arc::new(Mutex::new(HashMap::new())),
            revoke_mode,
            sightings: Arc::new(Mutex::new(HashMap::new())),
            dry_run_logged: Arc::new(Mutex::new(HashSet::new())),
            stuck_reported: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Build the reconciler from the raw value of [`REVOKE_ARM_ENV`], logging
    /// once which mode is in force. The startup path calls this, so the mode
    /// that was parsed is the mode the reconciler runs.
    pub fn from_revoke_env(raw: Option<&str>) -> Self {
        let mode = RevokeArmMode::parse(raw);
        let unrecognised = raw.is_some_and(|v| {
            mode == RevokeArmMode::Off
                && !matches!(v.trim().to_ascii_lowercase().as_str(), "" | "off")
        });
        if unrecognised {
            tracing::warn!(
                target: "elohim_storage::provide",
                value = raw.unwrap_or_default(),
                "provide revoke arm: unrecognised ELOHIM_PROVIDE_REVOKE_ARM value (expected off | dry-run | apply); the arm is OFF"
            );
        }
        match mode {
            RevokeArmMode::Off => tracing::info!(
                target: "elohim_storage::provide",
                "provide revoke arm: OFF (default). Commitments for removed pins are left standing; \
                 set ELOHIM_PROVIDE_REVOKE_ARM=dry-run to log what apply would revoke"
            ),
            RevokeArmMode::DryRun => tracing::info!(
                target: "elohim_storage::provide",
                "provide revoke arm: DRY-RUN. Logs the commitments apply would revoke; submits nothing"
            ),
            RevokeArmMode::Apply => tracing::warn!(
                target: "elohim_storage::provide",
                per_tick = REVOKES_PER_TICK,
                after_passes = REVOKE_AFTER_STRANDED_PASSES,
                "provide revoke arm: APPLY. A live content commitment of this node whose scope has a \
                 removed pin and no other pin is revoked (notarized, irreversible)"
            ),
        }
        Self::with_revoke_mode(mode)
    }

    /// The revoke arm's mode (fixed at construction).
    pub fn revoke_mode(&self) -> RevokeArmMode {
        self.revoke_mode
    }

    /// Read the current stage for a logical key (test/diagnostic accessor).
    pub async fn stage(&self, provider: &str, head_ref: &str) -> Option<ProvideStage> {
        self.latch
            .lock()
            .await
            .get(&(provider.to_string(), head_ref.to_string()))
            .copied()
    }

    /// One reconcile pass over the supplied desired set against the DHT-projected
    /// actual set, authoring/back-filling/revoking through `author`.
    ///
    /// `self_provider` is this peer's identity (the commitment provider).
    /// `desired` is the caught-up commons pin set (already filtered by the
    /// caller — see [`Self::derive_desired`]). The actual set is read fresh from
    /// the projection so dedup survives a restart.
    ///
    /// Returns the number of NEW author calls issued this pass (for metrics/tests).
    pub async fn reconcile_provides<A: CommitmentAuthor + ?Sized>(
        &self,
        pool: &DbPool,
        author: &A,
        self_provider: &str,
        desired: &[DesiredProvide],
    ) -> Result<usize, StorageError> {
        // The commitment's `provider` is the node's agent key (what the author
        // writes), not the transport `self_provider` — dedup reads that namespace.
        let own_providers = author.own_providers().await;
        if own_providers.is_empty() {
            tracing::debug!(
                target: "elohim_storage::provide",
                "provide reconcile: no agent provider resolvable; skipping pass"
            );
            return Ok(0);
        }

        // ── actual: live (non-revoked) replicates-commons rows under ANY own
        // provider, keyed by head_ref (== recipient) → row state.
        let own_rows = {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Internal(format!("pool: {e}")))?;
            crate::db::mishpat_commitments::live_commons_provides_for_providers(
                &mut conn,
                &own_providers,
            )
            .map_err(|e| StorageError::Database(e.to_string()))?
        };
        // Rows that still stand at `now`: a lapsed commitment must not suppress
        // renewal. Keyed by logical key (self_provider, head_ref==recipient).
        let now = chrono::Utc::now();
        let mut standing: HashMap<(String, String), Vec<&crate::db::models::MishpatCommitment>> =
            HashMap::new();
        for row in own_rows.iter().filter(|r| row_unexpired(r, now)) {
            standing
                .entry((self_provider.to_string(), row.recipient.clone()))
                .or_default()
                .push(row);
        }

        let desired_keys: HashSet<(String, String)> = desired
            .iter()
            .map(|d| (self_provider.to_string(), d.head_ref.clone()))
            .collect();

        let mut authored = 0usize;
        let mut latch = self.latch.lock().await;

        // ── author arm: desired keys with no live actual row ────────────────
        for d in desired {
            let key = (self_provider.to_string(), d.head_ref.clone());

            // A standing commitment that declares this reach (or whose reach
            // cannot be read) satisfies the desired provide.
            let matched = standing
                .get(&key)
                .and_then(|rows| rows.iter().find(|r| row_reach_matches(r, &d.reach)));
            if let Some(row) = matched {
                // A live commitment already exists (restart re-derivation lands
                // here even with an empty latch) — never re-author.
                let stage = if row.state == "active" {
                    ProvideStage::Active
                } else {
                    ProvideStage::Projected
                };
                latch.insert(key, stage);
                continue;
            }

            // No live row. If we already issued an author this lifetime and are
            // awaiting its projection, do not re-issue (within-process dedup).
            if matches!(latch.get(&key), Some(ProvideStage::Authoring)) {
                continue;
            }

            // A scope this node revoked is re-offered as a renewal with
            // lineage, never as a replay of the revoked entry (whose cid the
            // projection keeps revoked).
            let history = {
                let bare = scope_key(&d.head_ref);
                let spellings = vec![bare.to_string(), format!("epr:{bare}")];
                let read = pool.get().map_err(|e| e.to_string()).and_then(|mut conn| {
                    crate::db::mishpat_commitments::commons_provides_for_scope(
                        &mut conn,
                        &own_providers,
                        &spellings,
                    )
                    .map_err(|e| e.to_string())
                });
                match read {
                    Ok(rows) => rows,
                    Err(e) => {
                        // Authoring without the lineage could re-author a revoked
                        // cid; wait for a pass that can read it.
                        tracing::warn!(
                            target: "elohim_storage::provide",
                            head_ref = %d.head_ref,
                            error = %e,
                            "provide reconcile: scope lineage unreadable; not authoring this pass"
                        );
                        continue;
                    }
                }
            };
            latch.insert(key.clone(), ProvideStage::Authoring);
            let req = ProvideAuthorRequest {
                provider: self_provider.to_string(),
                head_ref: d.head_ref.clone(),
                reach: d.reach.clone(),
                renewal: renewal_for(&history),
            };
            match crate::chain_write_gate::as_writer(
                crate::chain_write_gate::WriterKind::Provide,
                author.author_commons(&req),
            )
            .await
            {
                Ok(cid) => {
                    authored += 1;
                    // Back-fill the pin's commitment_cid back-reference. The T10
                    // un-pin revoke path reads this, so a silent failure here
                    // would later strand the offer (un-pin couldn't target it) —
                    // warn loudly. The removed-pin revoke arm is the eventual
                    // backstop when it is switched on; the back-ref is the fast
                    // path.
                    match pool.get() {
                        Ok(mut conn) => {
                            if let Err(e) = crate::db::acquisition_pins::set_commitment_cid(
                                &mut conn, d.pin_id, &cid,
                            ) {
                                tracing::warn!(
                                    target: "elohim_storage::provide",
                                    pin_id = d.pin_id,
                                    commitment_cid = %cid,
                                    error = %e,
                                    "provide reconcile: set_commitment_cid back-fill failed; \
                                     un-pin revoke will fall back to the removed-pin arm"
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                target: "elohim_storage::provide",
                                pin_id = d.pin_id,
                                error = %e,
                                "provide reconcile: pool.get failed for commitment_cid back-fill"
                            );
                        }
                    }
                }
                Err(e) => {
                    // Roll the latch back so the next tick retries.
                    latch.insert(key, ProvideStage::NeedsCommitment);
                    tracing::warn!(
                        target: "elohim_storage::provide",
                        head_ref = %d.head_ref,
                        error = %e,
                        "provide reconcile: author_commons failed; will retry next tick"
                    );
                }
            }
        }

        // No revoke arm here: a key missing from `desired` is not evidence of
        // withdrawal. Revocation is [`Self::revoke_withdrawn`]'s, on removed
        // pins only.

        // Prune the per-process latch. It is a pure optimisation, but without a
        // sweep it accumulates a key for every (provider, head_ref) ever seen —
        // notably terminal `Revoked` keys from un-pinned offers, which otherwise
        // never leave. Keep only keys still desired or still live in `actual`; a
        // key that is re-needed next pass is re-inserted by the walks above.
        let live: HashSet<(String, String)> = standing.keys().cloned().collect();
        latch.retain(|k, _| desired_keys.contains(k) || live.contains(k));

        Ok(authored)
    }

    /// One pass of the revoke arm: withdraw this node's live content
    /// commitments for scopes the operator un-pinned. See the module doc for
    /// the rules; every one of them is decided here from state this method
    /// reads itself, never from the caller's desired set.
    ///
    /// Infallible by design: a pass that cannot observe (no signing key, a
    /// failed read) submits nothing and starts every count over.
    pub async fn revoke_withdrawn<A: CommitmentAuthor + ?Sized>(
        &self,
        pool: &DbPool,
        author: &A,
    ) -> RevokeSweep {
        self.revoke_withdrawn_at(pool, author, chrono::Utc::now())
            .await
    }

    /// [`Self::revoke_withdrawn`] at a given clock (tests drive the settle time
    /// with it).
    pub async fn revoke_withdrawn_at<A: CommitmentAuthor + ?Sized>(
        &self,
        pool: &DbPool,
        author: &A,
        now: chrono::DateTime<chrono::Utc>,
    ) -> RevokeSweep {
        if self.revoke_mode == RevokeArmMode::Off {
            return RevokeSweep::default();
        }
        let mut sightings = self.sightings.lock().await;
        let (withdrawn, stuck) = match Self::observe_withdrawn(pool, author, now).await {
            Ok(observed) => observed,
            Err(reason) => {
                // An unobserved pass is not a sighting, and it breaks the run.
                sightings.clear();
                tracing::debug!(
                    target: "elohim_storage::provide",
                    reason = %reason,
                    "provide revoke arm: pass could not observe; counts start over"
                );
                return RevokeSweep::default();
            }
        };
        self.report_stuck(&stuck).await;
        // Count this sighting. A row not withdrawn on this pass is dropped, so
        // its count starts over if it is withdrawn again.
        let counted: HashMap<String, u32> = withdrawn
            .iter()
            .map(|w| {
                let seen = sightings.get(&w.cid).copied().unwrap_or(0);
                (w.cid.clone(), seen.saturating_add(1))
            })
            .collect();
        *sightings = counted;

        let mut sweep = RevokeSweep {
            observed: true,
            withdrawn_rows: withdrawn.len(),
            stuck,
            ..RevokeSweep::default()
        };
        sweep.planned = withdrawn
            .into_iter()
            .filter(|w| sightings.get(&w.cid).copied().unwrap_or(0) >= REVOKE_AFTER_STRANDED_PASSES)
            .collect();

        if self.revoke_mode == RevokeArmMode::DryRun {
            let mut logged = self.dry_run_logged.lock().await;
            let mut fresh = 0usize;
            for plan in &sweep.planned {
                if logged.insert(plan.cid.clone()) {
                    fresh += 1;
                    tracing::info!(
                        target: "elohim_storage::provide",
                        target_cid = %plan.cid,
                        head_ref = %plan.head_ref,
                        pin_id = plan.pin_id,
                        removed_at = %plan.removed_at,
                        "provide revoke arm (dry-run): apply would revoke this commitment"
                    );
                }
            }
            if fresh > 0 {
                tracing::info!(
                    target: "elohim_storage::provide",
                    would_revoke = sweep.planned.len(),
                    withdrawn_rows = sweep.withdrawn_rows,
                    per_tick = REVOKES_PER_TICK,
                    "provide revoke arm (dry-run): totals; nothing was submitted"
                );
            }
            return sweep;
        }

        let mut submitted = 0usize;
        for plan in &sweep.planned {
            if submitted >= REVOKES_PER_TICK {
                break;
            }
            let outcome = submit_revocation(
                pool,
                author,
                plan,
                crate::chain_write_gate::WriterKind::Provide,
                now,
            )
            .await;
            sightings.remove(&plan.cid);
            match outcome {
                Submission::Issued => {
                    submitted += 1;
                    tracing::info!(
                        target: "elohim_storage::provide",
                        target_cid = %plan.cid,
                        head_ref = %plan.head_ref,
                        pin_id = plan.pin_id,
                        removed_at = %plan.removed_at,
                        "provide revoke arm: revoked the commitment of a removed pin"
                    );
                    sweep.issued.push(plan.cid.clone());
                }
                Submission::NotDue => {}
                Submission::NoLongerHolds(reason) => {
                    tracing::info!(
                        target: "elohim_storage::provide",
                        target_cid = %plan.cid,
                        head_ref = %plan.head_ref,
                        reason = %reason,
                        "provide revoke arm: the evidence no longer holds; nothing submitted"
                    );
                    sweep.withdrawn_at_the_last_moment.push(plan.cid.clone());
                }
                Submission::Unconfirmed(error) => {
                    // The conductor is not answering: nothing more this pass.
                    // The call may have committed, so its outcome is recorded
                    // as unknown and attempted again only after the settle
                    // time, a bounded number of times.
                    tracing::warn!(
                        target: "elohim_storage::provide",
                        target_cid = %plan.cid,
                        head_ref = %plan.head_ref,
                        error = %error,
                        "provide revoke arm: revoke_commons failed; recorded as unconfirmed, retried after the settle time"
                    );
                    sweep.unconfirmed.push(plan.cid.clone());
                    break;
                }
            }
        }
        sweep
    }

    /// Warn once a process for each target out of attempts. The pass also
    /// carries them in [`RevokeSweep::stuck`] every time.
    async fn report_stuck(&self, stuck: &[String]) {
        let mut reported = self.stuck_reported.lock().await;
        for cid in stuck {
            if reported.insert(cid.clone()) {
                tracing::warn!(
                    target: "elohim_storage::provide",
                    target_cid = %cid,
                    attempts = crate::db::mishpat_revocations::MAX_ATTEMPTS,
                    "provide revoke arm: a revocation this node may or may not have made is still not projected \
                     after every attempt; the commitment may still stand. Inspect it (mishpat_revocations) and \
                     clear the record to allow another attempt"
                );
            }
        }
    }

    /// Read what the revoke arm decides on: this node's live content rows for
    /// withdrawn scopes that are free to submit (no record) or due for another
    /// attempt (unknown outcome, settled, attempts left), oldest first; and the
    /// targets out of attempts. `Err` = the pass could not observe.
    async fn observe_withdrawn<A: CommitmentAuthor + ?Sized>(
        pool: &DbPool,
        author: &A,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<(Vec<PlannedRevocation>, Vec<String>), String> {
        let authority = author
            .revoke_authority()
            .await
            .ok_or_else(|| "no signing agent key established".to_string())?;
        let mut conn = pool.get().map_err(|e| format!("pool: {e}"))?;
        let stuck: Vec<String> = crate::db::mishpat_revocations::stuck(&mut conn, now)
            .map_err(|e| format!("revocation record read: {e}"))?
            .into_iter()
            .map(|r| r.target_cid)
            .collect();
        let pins = crate::db::acquisition_pins::list_all_pins(&mut conn)
            .map_err(|e| format!("pin read: {e}"))?;
        let withdrawn = withdrawn_scopes(&pins);
        if withdrawn.is_empty() {
            return Ok((Vec::new(), stuck));
        }
        // Rows the signing key provides, and no others (not a session key's).
        let mut rows = crate::db::mishpat_commitments::live_commons_provides_for_provider(
            &mut conn, &authority,
        )
        .map_err(|e| format!("commitment read: {e}"))?;
        let claimed = crate::db::mishpat_revocations::claimed_targets(&mut conn)
            .map_err(|e| format!("revocation record read: {e}"))?;
        let due: HashSet<String> = crate::db::mishpat_revocations::retry_due(&mut conn, now)
            .map_err(|e| format!("revocation record read: {e}"))?
            .into_iter()
            .map(|r| r.target_cid)
            .collect();
        rows.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.cid.cmp(&b.cid))
        });
        Ok((
            rows.into_iter()
                .filter(|r| is_content_provide(r) && row_stands_readably(r, now))
                .filter(|r| !claimed.contains(&r.cid) || due.contains(&r.cid))
                .filter_map(|r| {
                    let scope = withdrawn.get(scope_key(&r.recipient))?;
                    Some(PlannedRevocation {
                        cid: r.cid,
                        head_ref: r.recipient,
                        provider: r.provider,
                        pin_id: scope.pin_id,
                        removed_at: scope.removed_at.clone(),
                    })
                })
                .collect(),
            stuck,
        ))
    }

    /// Observe-only pass: re-derive the per-key stage latch from the live DHT
    /// projection WITHOUT authoring or revoking. This is the controller's
    /// read-half — it is what restores restart-safe stages when the in-memory
    /// latch is empty (e.g. on the first tick after process start, before any
    /// conductor author seam is wired). For each desired key it latches
    /// `Active`/`Projected` if a live commitment row exists, else
    /// `NeedsCommitment` (the next authoring tick, once an author seam is
    /// threaded, will act on those). Returns the number of desired keys that
    /// still need a commitment (no live actual row).
    ///
    /// Authoring proper goes through [`Self::reconcile_provides`] with a live
    /// [`CommitmentAuthor`]; this method exists so the loop can keep the latch
    /// current even where the conductor author seam is not yet composed.
    pub async fn observe(
        &self,
        pool: &DbPool,
        self_provider: &str,
        desired: &[DesiredProvide],
    ) -> Result<usize, StorageError> {
        let actual_rows = {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Internal(format!("pool: {e}")))?;
            crate::db::mishpat_commitments::live_commons_provides_for_provider(
                &mut conn,
                self_provider,
            )
            .map_err(|e| StorageError::Database(e.to_string()))?
        };
        let mut actual: HashMap<(String, String), String> = HashMap::new();
        for row in &actual_rows {
            actual.insert(
                (row.provider.clone(), row.recipient.clone()),
                row.state.clone(),
            );
        }

        let mut needs = 0usize;
        let mut latch = self.latch.lock().await;
        for d in desired {
            let key = (self_provider.to_string(), d.head_ref.clone());
            match actual.get(&key) {
                Some(state) if state == "active" => {
                    latch.insert(key, ProvideStage::Active);
                }
                Some(_) => {
                    latch.insert(key, ProvideStage::Projected);
                }
                None => {
                    needs += 1;
                    // Do not overwrite an in-flight Authoring latch from this
                    // process lifetime with NeedsCommitment.
                    if !matches!(latch.get(&key), Some(ProvideStage::Authoring)) {
                        latch.insert(key, ProvideStage::NeedsCommitment);
                    }
                }
            }
        }
        Ok(needs)
    }

    /// Derive the desired provide set: active `item` pins that are caught-up
    /// (acquisition byte-arrival complete) AND that this node is **eligible to
    /// provide** at their content's reach. Caught-up ids come from the live
    /// `AcquisitionState` rollup; the eligible set is computed by the caller via
    /// the reach-aware [`ProvideEligibility`] seam (commons is openly providable;
    /// non-commons requires embodied responsibility for the scope). Staying pure
    /// — the caller resolves both sets and passes them in — keeps the diff/dedup
    /// logic unit-testable without a live pool.
    ///
    /// `reach_by_head_ref` carries the CONTENT's own reach per head_ref (Stage B,
    /// from the eligibility `(head_ref, reach)` candidate map) so the authored
    /// commitment declares the content's reach. A head_ref missing from the map
    /// defaults to "commons" (back-compat: the commons loop never built one).
    pub fn derive_desired(
        pins: &[crate::db::models::AcquisitionPin],
        caught_up_head_refs: &HashSet<String>,
        provide_eligible_head_refs: &HashSet<String>,
        reach_by_head_ref: &HashMap<String, String>,
    ) -> Vec<DesiredProvide> {
        pins.iter()
            .filter(|p| p.kind == PIN_KIND_ITEM && p.status == PIN_STATUS_ACTIVE)
            .filter(|p| caught_up_head_refs.contains(&p.head_ref))
            .filter(|p| provide_eligible_head_refs.contains(&p.head_ref))
            .map(|p| DesiredProvide {
                pin_id: p.id,
                head_ref: p.head_ref.clone(),
                reach: reach_by_head_ref
                    .get(&p.head_ref)
                    .cloned()
                    .unwrap_or_else(|| "commons".to_string()),
                commitment_cid: p.commitment_cid.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::NewMishpatCommitment;
    use crate::test_util::test_pool;

    fn desired(pin_id: i32, head_ref: &str) -> DesiredProvide {
        DesiredProvide {
            pin_id,
            head_ref: head_ref.to_string(),
            reach: "commons".to_string(),
            commitment_cid: None,
        }
    }

    /// Seed a row under the mock author's agent provider, as the projection
    /// stores a commitment the production author wrote.
    fn seed_commons_row(pool: &DbPool, cid: &str, recipient: &str, state: &str, revoked: bool) {
        seed_commons_row_for(
            pool,
            MockAuthor::DEFAULT_PROVIDER,
            cid,
            recipient,
            state,
            revoked,
        );
    }

    fn seed_commons_row_for(
        pool: &DbPool,
        provider: &str,
        cid: &str,
        recipient: &str,
        state: &str,
        revoked: bool,
    ) {
        seed_commons_row_full(
            pool,
            provider,
            cid,
            recipient,
            state,
            revoked,
            "2099-12-01T00:00:00Z",
            r#"{"rate_per_minute":6,"reach_ceiling":"commons"}"#,
        );
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "a test seeder that names every column a provide row is read by"
    )]
    fn seed_commons_row_full(
        pool: &DbPool,
        provider: &str,
        cid: &str,
        recipient: &str,
        state: &str,
        revoked: bool,
        valid_until: &str,
        bounds_json: &str,
    ) {
        let mut conn = pool.get().expect("conn");
        crate::db::mishpat_commitments::upsert_with_anchor(
            &mut conn,
            NewMishpatCommitment {
                cid: cid.to_string(),
                action: "replicates-commons".to_string(),
                scope: "replicates-commons".to_string(),
                provider: provider.to_string(),
                recipient: recipient.to_string(),
                bounds_json: bounds_json.to_string(),
                valid_from: "2026-06-01T00:00:00Z".to_string(),
                valid_until: valid_until.to_string(),
                revoked_at: if revoked {
                    Some("2026-06-10T00:00:00Z".to_string())
                } else {
                    None
                },
                state: state.to_string(),
                dht_anchor_hash: Some(format!("anchor-{cid}")),
            },
        )
        .expect("seed commons row");
    }

    #[tokio::test]
    async fn authors_once_per_logical_key() {
        let pool = test_pool();
        let author = MockAuthor::new();
        let r = ProvideReconciler::new();
        let desired = vec![desired(1, "epr:album-1"), desired(2, "epr:album-2")];

        let n = r
            .reconcile_provides(&pool, &author, "agent:self", &desired)
            .await
            .expect("first pass");
        assert_eq!(n, 2, "two unprovided keys → two authors");
        assert_eq!(author.authored.lock().await.len(), 2);

        // Latch now says Authoring for both — a second pass with NO projection
        // landing must NOT re-author (within-process dedup).
        let n2 = r
            .reconcile_provides(&pool, &author, "agent:self", &desired)
            .await
            .expect("second pass");
        assert_eq!(n2, 0, "Authoring-latched keys must not re-author");
        assert_eq!(author.authored.lock().await.len(), 2);
    }

    #[tokio::test]
    async fn restart_rederives_from_projection_no_double_author() {
        // Simulate: a prior process authored + the projection landed (a live
        // proposed row exists). A FRESH reconciler (empty latch == restart)
        // must find the actual row and NOT re-author.
        let pool = test_pool();
        seed_commons_row(&pool, "cid:already", "epr:album-1", "proposed", false);

        let author = MockAuthor::new();
        let fresh = ProvideReconciler::new(); // empty latch — the restart case
        let n = fresh
            .reconcile_provides(&pool, &author, "agent:self", &[desired(1, "epr:album-1")])
            .await
            .expect("restart pass");

        assert_eq!(
            n, 0,
            "a key already live in the projection must not re-author"
        );
        assert!(
            author.authored.lock().await.is_empty(),
            "restart re-derivation = zero author calls"
        );
        assert_eq!(
            fresh.stage("agent:self", "epr:album-1").await,
            Some(ProvideStage::Projected),
            "an existing proposed row latches Projected"
        );
    }

    #[tokio::test]
    async fn graduated_row_latches_active() {
        let pool = test_pool();
        seed_commons_row(&pool, "cid:grad", "epr:album-1", "active", false);
        let author = MockAuthor::new();
        let r = ProvideReconciler::new();
        r.reconcile_provides(&pool, &author, "agent:self", &[desired(1, "epr:album-1")])
            .await
            .expect("pass");
        assert_eq!(
            r.stage("agent:self", "epr:album-1").await,
            Some(ProvideStage::Active),
            "an active projection row latches Active"
        );
        assert!(author.authored.lock().await.is_empty());
    }

    #[tokio::test]
    async fn revoked_projection_row_is_not_actual_so_key_reauthors() {
        // A revoked commitment is NOT a live actual row — a still-desired key
        // must author a fresh commitment (the old offer was withdrawn).
        let pool = test_pool();
        seed_commons_row(&pool, "cid:was-revoked", "epr:album-1", "proposed", true);
        let author = MockAuthor::new();
        let r = ProvideReconciler::new();
        let n = r
            .reconcile_provides(&pool, &author, "agent:self", &[desired(1, "epr:album-1")])
            .await
            .expect("pass");
        assert_eq!(n, 1, "a revoked row does not satisfy the desired key");
        assert_eq!(author.authored.lock().await.len(), 1);
    }

    const TRANSPORT_ID: &str = "12D3KooSelfTransport";

    #[tokio::test]
    async fn restart_does_not_reauthor_when_rows_carry_the_agent_provider() {
        // The projection stores the AGENT key as provider; the caller passes the
        // TRANSPORT id. A fresh reconciler (restart) must still find the row.
        let pool = test_pool();
        seed_commons_row(&pool, "cid:standing", "epr:X", "proposed", false);
        let author = MockAuthor::new();
        let fresh = ProvideReconciler::new();
        let n = fresh
            .reconcile_provides(&pool, &author, TRANSPORT_ID, &[desired(1, "epr:X")])
            .await
            .expect("restart pass");
        assert_eq!(n, 0);
        assert!(author.authored.lock().await.is_empty());
        assert_eq!(
            fresh.stage(TRANSPORT_ID, "epr:X").await,
            Some(ProvideStage::Projected)
        );
    }

    #[tokio::test]
    async fn a_row_under_any_own_provider_counts() {
        let pool = test_pool();
        seed_commons_row_for(&pool, "uhCAkCellKey", "cid:cell", "epr:X", "active", false);
        let author = MockAuthor::with_providers(vec![
            "uhCAkSessionKey".to_string(),
            "uhCAkCellKey".to_string(),
        ]);
        let n = ProvideReconciler::new()
            .reconcile_provides(&pool, &author, TRANSPORT_ID, &[desired(1, "epr:X")])
            .await
            .expect("pass");
        assert_eq!(n, 0);
        assert!(author.authored.lock().await.is_empty());
    }

    #[tokio::test]
    async fn no_resolvable_provider_authors_nothing() {
        let pool = test_pool();
        let author = MockAuthor::with_providers(vec![]);
        let n = ProvideReconciler::new()
            .reconcile_provides(&pool, &author, TRANSPORT_ID, &[desired(1, "epr:X")])
            .await
            .expect("pass");
        assert_eq!(n, 0);
        assert!(author.authored.lock().await.is_empty());
        assert!(author.revoked.lock().await.is_empty());
    }

    #[tokio::test]
    async fn a_revoked_row_does_not_count() {
        let pool = test_pool();
        seed_commons_row(&pool, "cid:was-revoked", "epr:X", "proposed", true);
        let author = MockAuthor::new();
        let n = ProvideReconciler::new()
            .reconcile_provides(&pool, &author, TRANSPORT_ID, &[desired(1, "epr:X")])
            .await
            .expect("pass");
        assert_eq!(n, 1);
        assert_eq!(author.authored.lock().await.len(), 1);
    }

    const FUTURE: &str = "2099-01-01T00:00:00Z";
    const PAST: &str = "2020-01-01T00:00:00Z";

    /// The renewal the single authored request carried.
    async fn authored_renewal(author: &MockAuthor) -> Option<ProvideRenewal> {
        let authored = author.authored.lock().await;
        assert_eq!(authored.len(), 1, "exactly one author call");
        authored[0].renewal.clone()
    }

    #[tokio::test]
    async fn a_scope_with_no_history_authors_an_unrenewed_commitment() {
        let pool = test_pool();
        let author = MockAuthor::new();
        reconcile_once(&pool, &author, desired(1, "epr:X")).await;
        assert_eq!(authored_renewal(&author).await, None);
    }

    #[tokio::test]
    async fn a_re_pin_after_a_revocation_renews_the_revoked_commitment() {
        let pool = test_pool();
        seed_commons_row(&pool, "uhCEkRevoked", "epr:X", "proposed", true);
        let author = MockAuthor::new();
        reconcile_once(&pool, &author, desired(1, "epr:X")).await;
        assert_eq!(
            authored_renewal(&author).await,
            Some(ProvideRenewal {
                sequence: 1,
                supersedes: "uhCEkRevoked".to_string(),
            }),
            "a re-pin after a revoke is a NEW commitment with lineage"
        );
    }

    #[tokio::test]
    async fn a_renewal_continues_the_sequence_of_the_commitment_it_renews() {
        let pool = test_pool();
        seed_commons_row(&pool, "uhCEkFirst", "epr:X", "proposed", true);
        seed_commons_row_full(
            &pool,
            MockAuthor::DEFAULT_PROVIDER,
            "uhCEkSecond",
            "epr:X",
            "proposed",
            true,
            FUTURE,
            r#"{"reach":"commons","reach_ceiling":"commons","renewal":1,"supersedes":"uhCEkFirst"}"#,
        );
        let author = MockAuthor::new();
        reconcile_once(&pool, &author, desired(1, "epr:X")).await;
        assert_eq!(
            authored_renewal(&author).await,
            Some(ProvideRenewal {
                sequence: 2,
                supersedes: "uhCEkSecond".to_string(),
            })
        );
    }

    #[tokio::test]
    async fn a_lapsed_unrevoked_history_authors_an_unrenewed_commitment() {
        let pool = test_pool();
        seed_commons_row_full(
            &pool,
            MockAuthor::DEFAULT_PROVIDER,
            "uhCEkLapsed",
            "epr:X",
            "active",
            false,
            PAST,
            &bounds_with_reach("commons"),
        );
        let author = MockAuthor::new();
        reconcile_once(&pool, &author, desired(1, "epr:X")).await;
        assert_eq!(
            authored_renewal(&author).await,
            None,
            "renewal is the revoke case; a lapse keeps today's identity rule"
        );
    }

    #[tokio::test]
    async fn the_lineage_reads_both_spellings_of_a_scope() {
        let pool = test_pool();
        // The commitment kept the spelling it was authored with; the pin is bare.
        seed_commons_row(&pool, "uhCEkPrefixed", "epr:X", "proposed", true);
        let author = MockAuthor::new();
        reconcile_once(&pool, &author, desired(1, "X")).await;
        assert_eq!(
            authored_renewal(&author).await.map(|r| r.supersedes),
            Some("uhCEkPrefixed".to_string())
        );
    }

    #[tokio::test]
    async fn another_providers_revoked_commitment_is_not_this_nodes_lineage() {
        let pool = test_pool();
        seed_commons_row_for(
            &pool,
            "uhCAkSomeoneElse",
            "uhCEkTheirs",
            "epr:X",
            "proposed",
            true,
        );
        let author = MockAuthor::new();
        reconcile_once(&pool, &author, desired(1, "epr:X")).await;
        assert_eq!(authored_renewal(&author).await, None);
    }

    fn bounds_with_reach(reach: &str) -> String {
        format!(r#"{{"reach":"{reach}","reach_ceiling":"commons","rate_per_minute":6}}"#)
    }

    async fn reconcile_once(pool: &DbPool, author: &MockAuthor, d: DesiredProvide) -> usize {
        ProvideReconciler::new()
            .reconcile_provides(pool, author, TRANSPORT_ID, &[d])
            .await
            .expect("pass")
    }

    #[tokio::test]
    async fn an_expired_row_does_not_suppress_renewal() {
        let pool = test_pool();
        let b = bounds_with_reach("commons");
        seed_commons_row_full(
            &pool,
            MockAuthor::DEFAULT_PROVIDER,
            "cid:old",
            "epr:X",
            "active",
            false,
            PAST,
            &b,
        );
        let author = MockAuthor::new();
        let n = reconcile_once(&pool, &author, desired(1, "epr:X")).await;
        assert_eq!(n, 1);
        assert_eq!(author.authored.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn a_row_valid_until_the_future_suppresses() {
        let pool = test_pool();
        let b = bounds_with_reach("commons");
        seed_commons_row_full(
            &pool,
            MockAuthor::DEFAULT_PROVIDER,
            "cid:ok",
            "epr:X",
            "active",
            false,
            FUTURE,
            &b,
        );
        let author = MockAuthor::new();
        let n = reconcile_once(&pool, &author, desired(1, "epr:X")).await;
        assert_eq!(n, 0);
        assert!(author.authored.lock().await.is_empty());
    }

    #[tokio::test]
    async fn an_unreadable_validity_counts_as_live() {
        let pool = test_pool();
        let b = bounds_with_reach("commons");
        seed_commons_row_full(
            &pool,
            MockAuthor::DEFAULT_PROVIDER,
            "cid:junk",
            "epr:X",
            "active",
            false,
            "not-a-date",
            &b,
        );
        let author = MockAuthor::new();
        let n = reconcile_once(&pool, &author, desired(1, "epr:X")).await;
        assert_eq!(n, 0);
        assert!(author.authored.lock().await.is_empty());
    }

    #[tokio::test]
    async fn a_changed_reach_is_redeclared_once() {
        let pool = test_pool();
        let b = bounds_with_reach("household");
        seed_commons_row_full(
            &pool,
            MockAuthor::DEFAULT_PROVIDER,
            "cid:hh",
            "epr:X",
            "active",
            false,
            FUTURE,
            &b,
        );
        let author = MockAuthor::new();
        let r = ProvideReconciler::new();
        let mut d = desired(1, "epr:X");
        d.reach = "commons".to_string();
        let first = r
            .reconcile_provides(&pool, &author, TRANSPORT_ID, std::slice::from_ref(&d))
            .await
            .expect("first");
        assert_eq!(first, 1);
        {
            let got = author.authored.lock().await;
            assert_eq!(got.len(), 1);
            assert_eq!(got[0].reach, "commons");
        }
        let second = r
            .reconcile_provides(&pool, &author, TRANSPORT_ID, std::slice::from_ref(&d))
            .await
            .expect("second");
        assert_eq!(second, 0);
        assert_eq!(author.authored.lock().await.len(), 1);
        assert!(author.revoked.lock().await.is_empty());
    }

    #[tokio::test]
    async fn the_same_reach_is_not_redeclared() {
        let pool = test_pool();
        let b = bounds_with_reach("household");
        seed_commons_row_full(
            &pool,
            MockAuthor::DEFAULT_PROVIDER,
            "cid:hh",
            "epr:X",
            "active",
            false,
            FUTURE,
            &b,
        );
        let author = MockAuthor::new();
        let mut d = desired(1, "epr:X");
        d.reach = "household".to_string();
        let n = reconcile_once(&pool, &author, d).await;
        assert_eq!(n, 0);
        assert!(author.authored.lock().await.is_empty());
    }

    #[tokio::test]
    async fn observe_latches_stage_from_projection_without_authoring() {
        // observe() is the controller's read-half: it must latch stages from the
        // live projection (Active/Projected/NeedsCommitment) without ever
        // authoring or revoking.
        let pool = test_pool();
        // observe() still reads under its `self_provider` argument (the P2PNode
        // caller has no agent key to hand it), so seed under that string.
        seed_commons_row_for(
            &pool,
            "agent:self",
            "cid:obs-active",
            "epr:active",
            "active",
            false,
        );
        seed_commons_row_for(
            &pool,
            "agent:self",
            "cid:obs-proposed",
            "epr:proposed",
            "proposed",
            false,
        );
        // epr:gap has no live row → NeedsCommitment, but no author call.
        let r = ProvideReconciler::new();
        let desired = vec![
            desired(1, "epr:active"),
            desired(2, "epr:proposed"),
            desired(3, "epr:gap"),
        ];

        let needs = r
            .observe(&pool, "agent:self", &desired)
            .await
            .expect("observe");
        assert_eq!(needs, 1, "exactly one desired key lacks a live actual row");

        assert_eq!(
            r.stage("agent:self", "epr:active").await,
            Some(ProvideStage::Active)
        );
        assert_eq!(
            r.stage("agent:self", "epr:proposed").await,
            Some(ProvideStage::Projected)
        );
        assert_eq!(
            r.stage("agent:self", "epr:gap").await,
            Some(ProvideStage::NeedsCommitment),
            "a desired key with no live row latches NeedsCommitment"
        );
    }

    #[test]
    fn derive_desired_filters_non_caught_up_and_non_eligible() {
        use crate::db::models::AcquisitionPin;
        let pin = |id: i32, head: &str, status: &str| AcquisitionPin {
            id,
            agent_pub_key: "local-device".to_string(),
            head_ref: head.to_string(),
            kind: "item".to_string(),
            closure_rule_json: None,
            priority: 1,
            status: status.to_string(),
            created_at: "t".to_string(),
            updated_at: "t".to_string(),
            commitment_cid: None,
            consecutive_retirements: 0,
            last_outcome: None,
            retired_at: None,
            round: 0,
            read_since_retired: 0,
            readmit_after: None,
        };
        let pins = vec![
            pin(1, "epr:ready-eligible", "active"),
            pin(2, "epr:not-caught-up", "active"),
            pin(3, "epr:caught-up-not-eligible", "active"),
            pin(4, "epr:paused", "paused"),
        ];
        let caught: HashSet<String> = [
            "epr:ready-eligible",
            "epr:caught-up-not-eligible",
            "epr:paused",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        // The eligible set (reach-aware, computed by the caller's seam) is the
        // third filter. Only a pin that is BOTH caught-up AND eligible is desired.
        let eligible: HashSet<String> = ["epr:ready-eligible", "epr:not-caught-up", "epr:paused"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        // Reach map (Stage B): the surviving pin carries its content's own reach.
        let reaches: HashMap<String, String> = [("epr:ready-eligible", "household")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let d = ProvideReconciler::derive_desired(&pins, &caught, &eligible, &reaches);
        assert_eq!(
            d.len(),
            1,
            "only the caught-up AND eligible active pin is desired"
        );
        assert_eq!(d[0].head_ref, "epr:ready-eligible");
        assert_eq!(
            d[0].reach, "household",
            "the desired provide carries the content's own reach from the map"
        );
    }

    #[test]
    fn derive_desired_reach_defaults_to_commons_when_unmapped() {
        use crate::db::models::AcquisitionPin;
        let pin = AcquisitionPin {
            id: 1,
            agent_pub_key: "local-device".to_string(),
            head_ref: "epr:no-reach-map".to_string(),
            kind: "item".to_string(),
            closure_rule_json: None,
            priority: 1,
            status: "active".to_string(),
            created_at: "t".to_string(),
            updated_at: "t".to_string(),
            commitment_cid: None,
            consecutive_retirements: 0,
            last_outcome: None,
            retired_at: None,
            round: 0,
            read_since_retired: 0,
            readmit_after: None,
        };
        let caught: HashSet<String> = ["epr:no-reach-map"].iter().map(|s| s.to_string()).collect();
        let eligible = caught.clone();
        let empty_reaches: HashMap<String, String> = HashMap::new();
        let d = ProvideReconciler::derive_desired(&[pin], &caught, &eligible, &empty_reaches);
        assert_eq!(d.len(), 1);
        assert_eq!(
            d[0].reach, "commons",
            "an unmapped head_ref defaults to commons (back-compat)"
        );
    }

    /// Mock eligibility seam — admit a fixed allow-list, reject the rest. Mirrors
    /// `MockAuthor`: it lets admit/reject be asserted without a live DB pool.
    struct MockEligibility {
        admit: HashSet<String>,
    }
    impl ProvideEligibility for MockEligibility {
        fn eligible_head_refs(&self, candidates: &[(String, String)]) -> HashSet<String> {
            candidates
                .iter()
                .filter(|(head_ref, _reach)| self.admit.contains(head_ref))
                .map(|(head_ref, _)| head_ref.clone())
                .collect()
        }
    }

    #[test]
    fn eligibility_seam_admits_allowlisted_rejects_rest() {
        let resolver = MockEligibility {
            admit: ["epr:household-eligible"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };
        let candidates = vec![
            (
                "epr:household-eligible".to_string(),
                "household".to_string(),
            ),
            (
                "epr:household-no-standing".to_string(),
                "household".to_string(),
            ),
        ];
        let eligible = resolver.eligible_head_refs(&candidates);
        assert!(
            eligible.contains("epr:household-eligible"),
            "a head_ref the node has standing for is admitted"
        );
        assert!(
            !eligible.contains("epr:household-no-standing"),
            "a head_ref the node lacks standing for is rejected (privacy gate)"
        );
        assert_eq!(eligible.len(), 1);
    }

    /// Stage B DEGRADE-OPEN: the production classifier admits commons
    /// unconditionally (commons is openly providable). A reach OUTSIDE the
    /// schema-8 vocabulary (`household`, `local`) does not parse to a topic, so
    /// the classifier now ADMITS it (degrade-open) rather than dropping it — the
    /// regression fix that lets household/local content get a provide author.
    /// A schema-8 *bound-tier* reach (e.g. `trusted`) still requires embodied
    /// responsibility and is rejected without standing.
    #[cfg(feature = "p2p")]
    #[test]
    fn classifier_eligibility_admits_commons_and_degrades_open_for_household() {
        use crate::test_util::test_pool;
        let pool = std::sync::Arc::new(test_pool());
        let resolver = ClassifierEligibility::new(pool, "lamad");
        let candidates = vec![
            ("epr:commons-item".to_string(), "commons".to_string()),
            ("epr:household-item".to_string(), "household".to_string()),
            ("epr:local-item".to_string(), "local".to_string()),
            ("epr:trusted-item".to_string(), "trusted".to_string()),
        ];
        let eligible = resolver.eligible_head_refs(&candidates);
        assert!(
            eligible.contains("epr:commons-item"),
            "commons is openly providable — always admitted"
        );
        assert!(
            eligible.contains("epr:household-item"),
            "household is outside schema-8 → degrade-open admits it (Stage B)"
        );
        assert!(
            eligible.contains("epr:local-item"),
            "local is outside schema-8 → degrade-open admits it (Stage B)"
        );
        assert!(
            !eligible.contains("epr:trusted-item"),
            "trusted IS schema-8 (bound-tier); requires embodied responsibility; none seeded → rejected"
        );
    }

    // ── revoke arm (removed pins only) ──────────────────────────────────

    const CELL: &str = MockAuthor::DEFAULT_PROVIDER;

    fn applying() -> ProvideReconciler {
        ProvideReconciler::with_revoke_mode(RevokeArmMode::Apply)
    }

    /// Insert a pin under `agent` and leave it in `status`. Returns its id.
    fn pin_as(pool: &DbPool, agent: &str, head_ref: &str, status: &str) -> i32 {
        let mut conn = pool.get().expect("conn");
        let pin = crate::db::acquisition_pins::upsert_pin(
            &mut conn,
            crate::db::models::NewAcquisitionPin {
                agent_pub_key: agent.to_string(),
                head_ref: head_ref.to_string(),
                kind: "item".to_string(),
                closure_rule_json: None,
                priority: 1,
            },
        )
        .expect("pin");
        if status != PIN_STATUS_ACTIVE {
            crate::db::acquisition_pins::set_pin_status(&mut conn, pin.id, status)
                .expect("pin status");
        }
        pin.id
    }

    fn pin(pool: &DbPool, head_ref: &str, status: &str) -> i32 {
        pin_as(pool, "local-device", head_ref, status)
    }

    fn removed_at(pool: &DbPool, pin_id: i32) -> String {
        let mut conn = pool.get().expect("conn");
        crate::db::acquisition_pins::get_pin_by_id(&mut conn, pin_id)
            .expect("pin read")
            .expect("pin")
            .updated_at
    }

    /// Bounds as the projection writes them for a content commitment.
    fn content_bounds(recipient: &str) -> String {
        format!(
            r#"{{"epr_scope":["{recipient}"],"reach":"commons","reach_ceiling":"commons","rate_per_minute":60}}"#
        )
    }

    /// A live, unexpired content row under `provider`.
    fn content_row(pool: &DbPool, provider: &str, cid: &str, recipient: &str) {
        seed_commons_row_full(
            pool,
            provider,
            cid,
            recipient,
            "active",
            false,
            FUTURE,
            &content_bounds(recipient),
        );
    }

    /// Run `passes` passes of the revoke arm; returns the last pass's report.
    async fn revoke_passes<A: CommitmentAuthor>(
        r: &ProvideReconciler,
        pool: &DbPool,
        author: &A,
        passes: u32,
    ) -> RevokeSweep {
        revoke_passes_at(r, pool, author, passes, chrono::Utc::now()).await
    }

    async fn revoke_passes_at<A: CommitmentAuthor>(
        r: &ProvideReconciler,
        pool: &DbPool,
        author: &A,
        passes: u32,
        now: chrono::DateTime<chrono::Utc>,
    ) -> RevokeSweep {
        let mut last = RevokeSweep::default();
        for _ in 0..passes {
            last = r.revoke_withdrawn_at(pool, author, now).await;
        }
        last
    }

    async fn revoked_targets(author: &MockAuthor) -> Vec<String> {
        author
            .revoked
            .lock()
            .await
            .iter()
            .map(|r| r.target_cid.clone())
            .collect()
    }

    fn commitment_signal(
        action: &str,
        payload_json: String,
        entry_hash: &str,
        action_hash: &str,
    ) -> crate::signals::MishpatSignal {
        crate::signals::MishpatSignal::CommitmentCommitted {
            action_hash: action_hash.into(),
            entry_hash: entry_hash.into(),
            author: CELL.into(),
            commitment: crate::mishpat_projection::CommitmentPayload {
                action: action.to_string(),
                payload_json,
                signed_at: "2026-10-01T00:00:00+00:00".to_string(),
            },
        }
    }

    /// Project a content commitment the way the post-commit signal does, so the
    /// row carries `cid` = entry hash and `dht_anchor_hash` = action hash.
    fn project_content_commitment(pool: &DbPool, head_ref: &str, entry: &str, action: &str) {
        let payload = crate::services::conductor_commitment_author::build_content_payload(
            CELL,
            head_ref,
            "commons",
            "2026-10-01T00:00:00+00:00",
            "2099-10-01T00:00:00+00:00",
            60,
        );
        let mut conn = pool.get().expect("conn");
        crate::signals::handle_mishpat_signal(
            &mut conn,
            "lamad",
            commitment_signal("replicates-commons", payload, entry, action),
        )
        .expect("project content commitment");
    }

    /// Project a revocation of `target` the way the post-commit signal does.
    fn project_revocation(pool: &DbPool, target: &str, signed_at: &str) {
        let payload =
            crate::services::conductor_commitment_author::build_revoke_payload(target, signed_at);
        let mut conn = pool.get().expect("conn");
        crate::signals::handle_mishpat_signal(
            &mut conn,
            "lamad",
            commitment_signal("revokes-commitment", payload, "uhCEkRevoke", "uhCkkRevoke"),
        )
        .expect("project revocation");
    }

    fn live_cids(pool: &DbPool) -> Vec<String> {
        let mut conn = pool.get().expect("conn");
        let mut cids: Vec<String> =
            crate::db::mishpat_commitments::live_commons_provides_for_provider(&mut conn, CELL)
                .expect("live rows")
                .into_iter()
                .map(|r| r.cid)
                .collect();
        cids.sort();
        cids
    }

    fn recorded_outcome(pool: &DbPool, cid: &str) -> Option<String> {
        let mut conn = pool.get().expect("conn");
        crate::db::mishpat_revocations::get(&mut conn, cid)
            .expect("record read")
            .and_then(|r| r.issue_outcome)
    }

    fn mirror_state(pool: &DbPool, cid: &str) -> Option<String> {
        use crate::db::diesel_schema::rea_commitments::dsl as rc;
        use diesel::prelude::*;
        let mut conn = pool.get().expect("conn");
        rc::rea_commitments
            .filter(rc::id.eq(cid))
            .select(rc::state)
            .first::<String>(&mut conn)
            .optional()
            .expect("mirror read")
    }

    /// An author that projects each revocation it is asked for, the production
    /// payload through `handle_mishpat_signal`, as a live conductor would.
    struct ProjectingAuthor {
        pool: DbPool,
        inner: MockAuthor,
    }

    #[async_trait]
    impl CommitmentRevoker for ProjectingAuthor {
        async fn revoke_authority(&self) -> Option<String> {
            self.inner.revoke_authority().await
        }
        async fn revoke_commons(
            &self,
            req: &ProvideRevokeRequest,
            precondition: &RevokePrecondition,
        ) -> Result<(), RevokeError> {
            self.inner.revoke_commons(req, precondition).await?;
            project_revocation(&self.pool, &req.target_cid, &req.signed_at);
            Ok(())
        }
    }

    #[async_trait]
    impl CommitmentAuthor for ProjectingAuthor {
        async fn own_providers(&self) -> Vec<String> {
            self.inner.own_providers().await
        }
        async fn author_commons(&self, req: &ProvideAuthorRequest) -> Result<String, StorageError> {
            self.inner.author_commons(req).await
        }
    }

    #[tokio::test]
    async fn a_revocation_issued_by_the_arm_marks_the_row_it_targets() {
        let pool = test_pool();
        // The commitment keeps the `epr:` spelling; the pin API stores it bare.
        project_content_commitment(&pool, "epr:gone", "uhCEkGone", "uhCkkGone");
        project_content_commitment(&pool, "epr:kept", "uhCEkKept", "uhCkkKept");
        let gone = pin(&pool, "gone", PIN_STATUS_REMOVED);
        pin(&pool, "kept", PIN_STATUS_ACTIVE);
        assert_eq!(live_cids(&pool), vec!["uhCEkGone", "uhCEkKept"]);

        let author = ProjectingAuthor {
            pool: pool.clone(),
            inner: MockAuthor::new(),
        };
        let r = applying();
        revoke_passes(&r, &pool, &author, REVOKE_AFTER_STRANDED_PASSES).await;

        {
            let revoked = author.inner.revoked.lock().await;
            assert_eq!(revoked.len(), 1);
            assert_eq!(
                revoked[0].target_cid, "uhCEkGone",
                "the target is the entry hash, the projection's key"
            );
            assert_eq!(revoked[0].head_ref, "epr:gone");
            assert_eq!(revoked[0].provider, CELL);
            assert_eq!(revoked[0].signed_at, removed_at(&pool, gone));
        }
        assert_eq!(
            live_cids(&pool),
            vec!["uhCEkKept"],
            "the revocation the arm issued marked the row it targeted"
        );
        assert_eq!(
            mirror_state(&pool, "uhCEkGone").as_deref(),
            Some("cancelled"),
            "and withdrew it from coverage"
        );
        assert_eq!(
            recorded_outcome(&pool, "uhCEkGone").as_deref(),
            Some(crate::db::mishpat_revocations::ISSUE_ISSUED)
        );

        revoke_passes(&r, &pool, &author, 5).await;
        assert_eq!(author.inner.revoked.lock().await.len(), 1);
    }

    #[test]
    fn a_revocation_targeting_the_action_hash_marks_nothing() {
        let pool = test_pool();
        project_content_commitment(&pool, "epr:gone", "uhCEkGone", "uhCkkGone");
        project_revocation(&pool, "uhCkkGone", "2026-10-13T00:00:00Z");
        assert_eq!(live_cids(&pool), vec!["uhCEkGone"]);
        project_revocation(&pool, "uhCEkGone", "2026-10-13T00:00:00Z");
        assert!(live_cids(&pool).is_empty());
    }

    #[test]
    fn a_replayed_commitment_does_not_un_revoke_its_row() {
        let pool = test_pool();
        project_content_commitment(&pool, "epr:gone", "uhCEkGone", "uhCkkGone");
        project_revocation(&pool, "uhCEkGone", "2026-10-13T00:00:00Z");
        assert!(live_cids(&pool).is_empty());
        // The commitment's own signal is delivered again.
        project_content_commitment(&pool, "epr:gone", "uhCEkGone", "uhCkkGone");
        assert!(live_cids(&pool).is_empty(), "revocation survives replay");
        assert_eq!(
            mirror_state(&pool, "uhCEkGone").as_deref(),
            Some("cancelled")
        );
    }

    #[test]
    fn a_revocation_that_lands_before_its_target_is_not_lost() {
        let pool = test_pool();
        project_revocation(&pool, "uhCEkLate", "2026-10-13T00:00:00Z");
        project_content_commitment(&pool, "epr:late", "uhCEkLate", "uhCkkLate");
        assert!(live_cids(&pool).is_empty());
        let mut conn = pool.get().expect("conn");
        let row = crate::db::mishpat_commitments::get_by_cid(&mut conn, "uhCEkLate")
            .expect("read")
            .expect("row");
        assert_eq!(row.revoked_at.as_deref(), Some("2026-10-13T00:00:00Z"));
        drop(conn);
        assert_eq!(
            mirror_state(&pool, "uhCEkLate").as_deref(),
            Some("cancelled"),
            "the coverage mirror must not read a revoked promise as live"
        );
    }

    #[test]
    fn only_a_removed_pin_with_no_other_pin_withdraws_a_scope() {
        let pool = test_pool();
        let gone = pin(&pool, "gone", PIN_STATUS_REMOVED);
        pin(&pool, "retired", "retired");
        pin(&pool, "paused", "paused");
        pin(&pool, "active", PIN_STATUS_ACTIVE);
        // Removed under one agent, still pinned under another.
        pin(&pool, "shared", PIN_STATUS_REMOVED);
        pin_as(&pool, "another-device", "shared", PIN_STATUS_ACTIVE);
        // Removed in one spelling, retired in the other.
        pin(&pool, "epr:respelt", PIN_STATUS_REMOVED);
        pin(&pool, "respelt", "retired");
        let pins = {
            let mut conn = pool.get().expect("conn");
            crate::db::acquisition_pins::list_all_pins(&mut conn).expect("pins")
        };
        let withdrawn = withdrawn_scopes(&pins);
        assert_eq!(withdrawn.len(), 1, "{withdrawn:?}");
        assert_eq!(withdrawn["gone"].pin_id, gone);
        assert!(withdrawn_scopes(&[]).is_empty());
    }

    #[test]
    fn scope_key_reads_both_spellings_as_one_scope() {
        assert_eq!(scope_key("epr:X"), "X");
        assert_eq!(scope_key("X"), "X");
        assert_eq!(scope_key("epr:X"), scope_key("X"));
    }

    #[tokio::test]
    async fn a_scope_is_matched_across_the_epr_prefix() {
        let pool = test_pool();
        // Pin stored bare, commitment spelt with the prefix; and the reverse.
        pin(&pool, "X", PIN_STATUS_REMOVED);
        content_row(&pool, CELL, "cid:x", "epr:X");
        pin(&pool, "epr:Y", PIN_STATUS_REMOVED);
        content_row(&pool, CELL, "cid:y", "Y");
        // Removed bare, but still actively pinned under the other spelling.
        pin(&pool, "Z", PIN_STATUS_REMOVED);
        pin(&pool, "epr:Z", PIN_STATUS_ACTIVE);
        content_row(&pool, CELL, "cid:z", "epr:Z");

        let author = MockAuthor::new();
        revoke_passes(&applying(), &pool, &author, 8).await;
        let mut revoked = revoked_targets(&author).await;
        revoked.sort();
        assert_eq!(revoked, vec!["cid:x", "cid:y"]);
    }

    #[tokio::test]
    async fn absence_is_never_evidence() {
        let pool = test_pool();
        // No pin row at all.
        content_row(&pool, CELL, "cid:unpinned", "epr:unpinned");
        // A retired pin (the substrate's temporary hold) and a paused one.
        pin(&pool, "retired", "retired");
        content_row(&pool, CELL, "cid:retired", "epr:retired");
        pin(&pool, "paused", "paused");
        content_row(&pool, CELL, "cid:paused", "epr:paused");
        // Removed, with another pin for the same scope still active.
        pin(&pool, "shared", PIN_STATUS_REMOVED);
        pin_as(&pool, "another-device", "shared", PIN_STATUS_ACTIVE);
        content_row(&pool, CELL, "cid:shared", "epr:shared");
        // An active pin with several rows: duplicates are not this arm's.
        pin(&pool, "dup", PIN_STATUS_ACTIVE);
        for i in 0..4 {
            content_row(&pool, CELL, &format!("cid:dup-{i}"), "epr:dup");
        }

        let author = MockAuthor::new();
        let r = applying();
        let sweep = revoke_passes(&r, &pool, &author, 8).await;
        assert!(sweep.observed);
        assert_eq!(sweep.withdrawn_rows, 0);
        assert!(revoked_targets(&author).await.is_empty());

        // The author arm never revokes either, whatever the desired set omits.
        r.reconcile_provides(&pool, &author, TRANSPORT_ID, &[desired(1, "epr:dup")])
            .await
            .expect("author pass");
        r.reconcile_provides(&pool, &author, TRANSPORT_ID, &[])
            .await
            .expect("empty author pass");
        assert!(revoked_targets(&author).await.is_empty());
    }

    #[tokio::test]
    async fn an_empty_pins_table_revokes_nothing() {
        let pool = test_pool();
        content_row(&pool, CELL, "cid:a", "epr:a");
        content_row(&pool, CELL, "cid:b", "epr:b");
        let author = MockAuthor::new();
        let sweep = revoke_passes(&applying(), &pool, &author, 8).await;
        assert!(sweep.observed);
        assert!(sweep.planned.is_empty());
        assert!(revoked_targets(&author).await.is_empty());
    }

    #[tokio::test]
    async fn every_row_of_a_removed_scope_is_revoked_once() {
        let pool = test_pool();
        pin(&pool, "gone", PIN_STATUS_REMOVED);
        for i in 0..3 {
            content_row(&pool, CELL, &format!("cid:gone-{i}"), "epr:gone");
        }
        // MockAuthor projects nothing, so the rows stay live: the worst case.
        let author = MockAuthor::new();
        let r = applying();
        revoke_passes(&r, &pool, &author, 12).await;
        let mut revoked = revoked_targets(&author).await;
        revoked.sort();
        assert_eq!(revoked, vec!["cid:gone-0", "cid:gone-1", "cid:gone-2"]);
    }

    #[tokio::test]
    async fn a_restart_does_not_issue_a_second_revocation() {
        let pool = test_pool();
        pin(&pool, "gone", PIN_STATUS_REMOVED);
        content_row(&pool, CELL, "cid:gone", "epr:gone");
        let author = MockAuthor::new();
        revoke_passes(&applying(), &pool, &author, REVOKE_AFTER_STRANDED_PASSES).await;
        assert_eq!(revoked_targets(&author).await, vec!["cid:gone"]);
        assert_eq!(
            live_cids(&pool),
            vec!["cid:gone"],
            "its projection never landed"
        );

        // A new process: empty memory, same database, a day later.
        let restarted = applying();
        let later = chrono::Utc::now() + chrono::Duration::days(1);
        let sweep = revoke_passes_at(&restarted, &pool, &author, 10, later).await;
        assert_eq!(
            sweep.withdrawn_rows, 0,
            "an accepted submission is not planned again"
        );
        assert_eq!(revoked_targets(&author).await, vec!["cid:gone"]);
    }

    /// An author that can be made to refuse revocations at the conductor, to
    /// lose its signing key, or to re-pin a scope while its call waits for the
    /// chain (before the precondition is evaluated).
    #[derive(Default)]
    struct FlakyAuthor {
        refuse: std::sync::atomic::AtomicBool,
        no_authority: std::sync::atomic::AtomicBool,
        attempts: std::sync::atomic::AtomicUsize,
        repin_while_waiting: Option<(DbPool, String)>,
        inner: MockAuthor,
    }

    #[async_trait]
    impl CommitmentRevoker for FlakyAuthor {
        async fn revoke_authority(&self) -> Option<String> {
            if self.no_authority.load(std::sync::atomic::Ordering::SeqCst) {
                return None;
            }
            self.inner.revoke_authority().await
        }
        async fn revoke_commons(
            &self,
            req: &ProvideRevokeRequest,
            precondition: &RevokePrecondition,
        ) -> Result<(), RevokeError> {
            use std::sync::atomic::Ordering;
            // The wait for the chain gate and the admission permit.
            if let Some((pool, head_ref)) = &self.repin_while_waiting {
                pin(pool, head_ref, PIN_STATUS_ACTIVE);
            }
            // The precondition runs after the wait, before the call.
            precondition().map_err(RevokeError::Refused)?;
            self.attempts.fetch_add(1, Ordering::SeqCst);
            if self.refuse.load(Ordering::SeqCst) {
                return Err(RevokeError::Failed(StorageError::Internal(
                    "conductor not answering".into(),
                )));
            }
            self.inner.revoke_commons(req, precondition).await
        }
    }

    #[async_trait]
    impl CommitmentAuthor for FlakyAuthor {
        async fn own_providers(&self) -> Vec<String> {
            self.inner.own_providers().await
        }
        async fn author_commons(&self, req: &ProvideAuthorRequest) -> Result<String, StorageError> {
            self.inner.author_commons(req).await
        }
    }

    #[tokio::test]
    async fn a_scope_re_pinned_while_the_call_waits_is_not_revoked() {
        let pool = test_pool();
        pin(&pool, "gone", PIN_STATUS_REMOVED);
        content_row(&pool, CELL, "cid:gone", "epr:gone");
        // The pin and row are re-checked when the pass plans and when it
        // claims; the operator re-pins after both, while the call is queued.
        let author = FlakyAuthor {
            repin_while_waiting: Some((pool.clone(), "gone".to_string())),
            ..FlakyAuthor::default()
        };
        let sweep = revoke_passes(&applying(), &pool, &author, REVOKE_AFTER_STRANDED_PASSES).await;
        assert_eq!(sweep.planned.len(), 1, "planned from the pass's read");
        assert!(sweep.issued.is_empty());
        assert_eq!(sweep.withdrawn_at_the_last_moment, vec!["cid:gone"]);
        assert_eq!(
            author.attempts.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "no conductor call is offered once the scope is pinned again"
        );
        assert!(revoked_targets(&author.inner).await.is_empty());
        assert_eq!(
            recorded_outcome(&pool, "cid:gone"),
            None,
            "a claim that submitted nothing is given back"
        );
    }

    #[tokio::test]
    async fn an_unanswered_call_is_retried_after_it_settles_then_reported() {
        use std::sync::atomic::Ordering;
        let pool = test_pool();
        pin(&pool, "a", PIN_STATUS_REMOVED);
        pin(&pool, "b", PIN_STATUS_REMOVED);
        content_row(&pool, CELL, "cid:a", "epr:a");
        content_row(&pool, CELL, "cid:b", "epr:b");
        let author = FlakyAuthor::default();
        author.refuse.store(true, Ordering::SeqCst);
        let start = chrono::Utc::now();
        let r = applying();

        let sweep = revoke_passes_at(&r, &pool, &author, REVOKE_AFTER_STRANDED_PASSES, start).await;
        assert_eq!(sweep.unconfirmed, vec!["cid:a"]);
        assert_eq!(
            author.attempts.load(Ordering::SeqCst),
            1,
            "one error ends the pass"
        );
        assert_eq!(
            recorded_outcome(&pool, "cid:a").as_deref(),
            Some(crate::db::mishpat_revocations::ISSUE_UNCONFIRMED)
        );

        // Before the settle time `a` is not taken again (it may have
        // committed and its projection may still arrive), and the conductor
        // still refuses `b`.
        revoke_passes_at(&r, &pool, &author, 1, start + chrono::Duration::minutes(1)).await;
        assert_eq!(author.attempts.load(Ordering::SeqCst), 2, "b was attempted");

        // The conductor answers again. Once settled, both unknown outcomes are
        // attempted again, carrying the signing time of their first claim.
        author.refuse.store(false, Ordering::SeqCst);
        let settled =
            start + crate::db::mishpat_revocations::RETRY_SETTLE + chrono::Duration::minutes(2);
        revoke_passes_at(
            &applying(),
            &pool,
            &author,
            REVOKE_AFTER_STRANDED_PASSES,
            settled,
        )
        .await;
        let mut revoked = revoked_targets(&author.inner).await;
        revoked.sort();
        assert_eq!(revoked, vec!["cid:a", "cid:b"], "never silently dropped");

        // A target that never answers is attempted a bounded number of times
        // and then reported on every pass.
        let pool = test_pool();
        pin(&pool, "c", PIN_STATUS_REMOVED);
        content_row(&pool, CELL, "cid:c", "epr:c");
        let stubborn = FlakyAuthor::default();
        stubborn.refuse.store(true, Ordering::SeqCst);
        let mut clock = start;
        for _ in 0..6 {
            revoke_passes_at(
                &applying(),
                &pool,
                &stubborn,
                REVOKE_AFTER_STRANDED_PASSES,
                clock,
            )
            .await;
            clock += crate::db::mishpat_revocations::RETRY_SETTLE + chrono::Duration::minutes(1);
        }
        assert_eq!(
            stubborn.attempts.load(Ordering::SeqCst),
            crate::db::mishpat_revocations::MAX_ATTEMPTS as usize
        );
        let sweep = revoke_passes_at(&applying(), &pool, &stubborn, 1, clock).await;
        assert_eq!(sweep.stuck, vec!["cid:c"], "reported, not silently stopped");
    }

    #[tokio::test]
    async fn a_claim_left_pending_by_a_crash_is_retried_with_the_same_entry() {
        let pool = test_pool();
        let gone = pin(&pool, "gone", PIN_STATUS_REMOVED);
        content_row(&pool, CELL, "cid:gone", "epr:gone");
        let crashed_at = chrono::Utc::now();
        // The process recorded its claim and stopped before the call.
        {
            let mut conn = pool.get().expect("conn");
            let claim = crate::db::mishpat_revocations::claim(
                &mut conn,
                "cid:gone",
                &removed_at(&pool, gone),
                crashed_at,
            )
            .expect("claim");
            assert!(matches!(
                claim,
                crate::db::mishpat_revocations::Claim::Submit { attempt: 1, .. }
            ));
        }
        let author = MockAuthor::new();
        let early = revoke_passes_at(
            &applying(),
            &pool,
            &author,
            REVOKE_AFTER_STRANDED_PASSES,
            crashed_at + chrono::Duration::minutes(1),
        )
        .await;
        assert!(
            revoked_targets(&author).await.is_empty(),
            "a pending claim is in flight"
        );
        assert_eq!(early.withdrawn_rows, 0);

        let settled = crashed_at
            + crate::db::mishpat_revocations::RETRY_SETTLE
            + chrono::Duration::minutes(1);
        revoke_passes_at(
            &applying(),
            &pool,
            &author,
            REVOKE_AFTER_STRANDED_PASSES,
            settled,
        )
        .await;
        let revoked = author.revoked.lock().await;
        assert_eq!(revoked.len(), 1, "the crashed attempt is retried");
        assert_eq!(revoked[0].signed_at, removed_at(&pool, gone));
        drop(revoked);
        assert_eq!(
            recorded_outcome(&pool, "cid:gone").as_deref(),
            Some(crate::db::mishpat_revocations::ISSUE_ISSUED)
        );
    }

    #[tokio::test]
    async fn the_route_and_the_arm_share_one_claim() {
        let pool = test_pool();
        let gone = pin(&pool, "gone", PIN_STATUS_REMOVED);
        content_row(&pool, CELL, "cid:gone", "epr:gone");
        let plan = PlannedRevocation {
            cid: "cid:gone".to_string(),
            head_ref: "epr:gone".to_string(),
            provider: CELL.to_string(),
            pin_id: gone,
            removed_at: removed_at(&pool, gone),
        };
        let author = MockAuthor::new();
        let now = chrono::Utc::now();

        // The arm holds the attempt (its call is in flight).
        {
            let mut conn = pool.get().expect("conn");
            crate::db::mishpat_revocations::claim(&mut conn, &plan.cid, &plan.removed_at, now)
                .expect("arm claim");
        }
        // The un-pin route runs meanwhile: it submits nothing.
        let route = submit_revocation(
            &pool,
            &author,
            &plan,
            crate::chain_write_gate::WriterKind::HttpAuthor,
            now,
        )
        .await;
        assert_eq!(route, Submission::NotDue);
        assert!(revoked_targets(&author).await.is_empty());

        // And the other way round: the route revokes first; the arm never
        // plans that row again, the route's repeated DELETE submits nothing.
        let pool = test_pool();
        let gone = pin(&pool, "gone", PIN_STATUS_REMOVED);
        content_row(&pool, CELL, "cid:gone", "epr:gone");
        let plan = PlannedRevocation {
            removed_at: removed_at(&pool, gone),
            pin_id: gone,
            ..plan
        };
        let author = MockAuthor::new();
        for _ in 0..2 {
            submit_revocation(
                &pool,
                &author,
                &plan,
                crate::chain_write_gate::WriterKind::HttpAuthor,
                now,
            )
            .await;
        }
        let sweep = revoke_passes(&applying(), &pool, &author, 8).await;
        assert_eq!(sweep.withdrawn_rows, 0);
        assert_eq!(revoked_targets(&author).await, vec!["cid:gone"]);
    }

    #[tokio::test]
    async fn a_pass_that_cannot_observe_starts_the_count_over() {
        use std::sync::atomic::Ordering;
        let pool = test_pool();
        pin(&pool, "gone", PIN_STATUS_REMOVED);
        content_row(&pool, CELL, "cid:gone", "epr:gone");
        let author = FlakyAuthor::default();
        let r = applying();

        revoke_passes(&r, &pool, &author, 2).await;
        // One pass with no signing key established: nothing observed.
        author.no_authority.store(true, Ordering::SeqCst);
        let held = revoke_passes(&r, &pool, &author, 1).await;
        assert!(!held.observed);
        author.no_authority.store(false, Ordering::SeqCst);

        // Two seen + a gap + two more must not revoke.
        revoke_passes(&r, &pool, &author, 2).await;
        assert_eq!(author.attempts.load(Ordering::SeqCst), 0);
        revoke_passes(&r, &pool, &author, 1).await;
        assert_eq!(revoked_targets(&author.inner).await, vec!["cid:gone"]);
    }

    #[tokio::test]
    async fn a_pin_restored_for_a_pass_starts_the_count_over() {
        let pool = test_pool();
        let id = pin(&pool, "flap", PIN_STATUS_REMOVED);
        content_row(&pool, CELL, "cid:flap", "epr:flap");
        let author = MockAuthor::new();
        let r = applying();
        revoke_passes(&r, &pool, &author, 2).await;
        pin(&pool, "flap", PIN_STATUS_ACTIVE);
        revoke_passes(&r, &pool, &author, 1).await;
        {
            let mut conn = pool.get().expect("conn");
            crate::db::acquisition_pins::set_pin_status(&mut conn, id, PIN_STATUS_REMOVED)
                .expect("remove");
        }
        revoke_passes(&r, &pool, &author, 2).await;
        assert!(revoked_targets(&author).await.is_empty());
        revoke_passes(&r, &pool, &author, 1).await;
        assert_eq!(revoked_targets(&author).await, vec!["cid:flap"]);
    }

    #[tokio::test]
    async fn revocations_are_bounded_per_tick() {
        assert_eq!(REVOKES_PER_TICK, 4);
        assert_eq!(REVOKE_AFTER_STRANDED_PASSES, 3);
        let pool = test_pool();
        for i in 0..10 {
            pin(&pool, &format!("gone-{i:02}"), PIN_STATUS_REMOVED);
            content_row(
                &pool,
                CELL,
                &format!("cid:gone-{i:02}"),
                &format!("epr:gone-{i:02}"),
            );
        }
        let author = MockAuthor::new();
        let r = applying();

        revoke_passes(&r, &pool, &author, 2).await;
        assert!(revoked_targets(&author).await.is_empty());
        revoke_passes(&r, &pool, &author, 1).await;
        assert_eq!(revoked_targets(&author).await.len(), 4);
        revoke_passes(&r, &pool, &author, 1).await;
        assert_eq!(revoked_targets(&author).await.len(), 8);
        revoke_passes(&r, &pool, &author, 5).await;
        let all = revoked_targets(&author).await;
        assert_eq!(all.len(), 10, "the backlog drains, then stops");
        let distinct: HashSet<&String> = all.iter().collect();
        assert_eq!(distinct.len(), 10, "no row is revoked twice");
    }

    #[tokio::test]
    async fn only_rows_the_signing_cell_key_provides_are_revoked() {
        let pool = test_pool();
        pin(&pool, "gone", PIN_STATUS_REMOVED);
        content_row(&pool, "uhCAkCellKey", "cid:cell", "epr:gone");
        // A session key this node may also author under, another agent, and
        // this node's transport id: none is the key that signs.
        content_row(&pool, "uhCAkSessionKey", "cid:session", "epr:gone");
        content_row(&pool, "uhCAkSomeoneElse", "cid:theirs", "epr:gone");
        content_row(&pool, TRANSPORT_ID, "cid:transport", "epr:gone");
        let author = MockAuthor::with_providers(vec![
            "uhCAkSessionKey".to_string(),
            "uhCAkCellKey".to_string(),
        ])
        .with_authority(Some("uhCAkCellKey"));
        revoke_passes(&applying(), &pool, &author, 8).await;
        {
            let revoked = author.revoked.lock().await;
            assert_eq!(revoked.len(), 1);
            assert_eq!(revoked[0].target_cid, "cid:cell");
            assert_eq!(revoked[0].provider, "uhCAkCellKey");
        }

        // With no signing key established, nothing is revoked at all.
        let pool = test_pool();
        pin(&pool, "gone", PIN_STATUS_REMOVED);
        content_row(&pool, CELL, "cid:cell", "epr:gone");
        let keyless = MockAuthor::new().with_authority(None);
        let sweep = revoke_passes(&applying(), &pool, &keyless, 8).await;
        assert!(!sweep.observed);
        assert!(revoked_targets(&keyless).await.is_empty());
    }

    #[tokio::test]
    async fn only_rows_that_positively_read_as_content_are_revoked() {
        let pool = test_pool();
        let row = |cid: &str, recipient: &str, valid_until: &str, bounds: &str| {
            seed_commons_row_full(
                &pool,
                CELL,
                cid,
                recipient,
                "active",
                false,
                valid_until,
                bounds,
            );
        };
        for scope in [
            "junk", "null", "array", "empty", "unscoped", "other", "two", "pledge", "lapsed",
            "undated", "good",
        ] {
            pin(&pool, scope, PIN_STATUS_REMOVED);
        }
        row("cid:junk", "epr:junk", FUTURE, "not json");
        row("cid:null", "epr:null", FUTURE, "null");
        row("cid:array", "epr:array", FUTURE, "[]");
        row("cid:empty", "epr:empty", FUTURE, "{}");
        row(
            "cid:unscoped",
            "epr:unscoped",
            FUTURE,
            r#"{"rate_per_minute":6,"reach_ceiling":"commons"}"#,
        );
        row(
            "cid:other",
            "epr:other",
            FUTURE,
            &content_bounds("epr:else"),
        );
        row(
            "cid:two",
            "epr:two",
            FUTURE,
            r#"{"epr_scope":["epr:two","epr:more"]}"#,
        );
        // A capacity pledge, with and without a counterparty.
        row(
            "cid:pledge",
            "epr:pledge",
            FUTURE,
            r#"{"epr_scope":["epr:pledge"],"commons_bytes":1}"#,
        );
        pin(&pool, "", PIN_STATUS_REMOVED);
        row(
            "cid:capacity",
            "",
            "",
            r#"{"commons_bytes":26843545600,"rate_per_minute":12,"reach_ceiling":"commons"}"#,
        );
        // A lapsed offer needs no revocation; unreadable validity stops the write.
        row(
            "cid:lapsed",
            "epr:lapsed",
            PAST,
            &content_bounds("epr:lapsed"),
        );
        row(
            "cid:undated",
            "epr:undated",
            "not-a-date",
            &content_bounds("epr:undated"),
        );
        row("cid:good", "epr:good", FUTURE, &content_bounds("epr:good"));

        let author = MockAuthor::new();
        revoke_passes(&applying(), &pool, &author, 8).await;
        assert_eq!(revoked_targets(&author).await, vec!["cid:good"]);
    }

    #[tokio::test]
    async fn a_dry_run_plans_and_submits_nothing() {
        let pool = test_pool();
        let gone = pin(&pool, "gone", PIN_STATUS_REMOVED);
        content_row(&pool, CELL, "cid:gone", "epr:gone");
        let author = FlakyAuthor::default();
        let r = ProvideReconciler::with_revoke_mode(RevokeArmMode::DryRun);

        let early = revoke_passes(&r, &pool, &author, 2).await;
        assert_eq!(early.withdrawn_rows, 1);
        assert!(
            early.planned.is_empty(),
            "the same three-pass rule as apply"
        );
        let sweep = revoke_passes(&r, &pool, &author, 8).await;
        assert_eq!(
            sweep.planned,
            vec![PlannedRevocation {
                cid: "cid:gone".to_string(),
                head_ref: "epr:gone".to_string(),
                provider: CELL.to_string(),
                pin_id: gone,
                removed_at: removed_at(&pool, gone),
            }]
        );
        assert!(sweep.issued.is_empty());
        assert_eq!(
            author.attempts.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "a dry run never calls the conductor"
        );
        assert_eq!(recorded_outcome(&pool, "cid:gone"), None);
        assert_eq!(live_cids(&pool), vec!["cid:gone"]);
    }

    #[tokio::test]
    async fn the_arm_is_off_unless_switched_on() {
        assert_eq!(RevokeArmMode::default(), RevokeArmMode::Off);
        assert_eq!(RevokeArmMode::parse(None), RevokeArmMode::Off);
        assert_eq!(RevokeArmMode::parse(Some("")), RevokeArmMode::Off);
        assert_eq!(RevokeArmMode::parse(Some("on")), RevokeArmMode::Off);
        assert_eq!(RevokeArmMode::parse(Some("true")), RevokeArmMode::Off);
        assert_eq!(RevokeArmMode::parse(Some(" Apply ")), RevokeArmMode::Apply);
        assert_eq!(RevokeArmMode::parse(Some("dry-run")), RevokeArmMode::DryRun);

        let pool = test_pool();
        pin(&pool, "gone", PIN_STATUS_REMOVED);
        content_row(&pool, CELL, "cid:gone", "epr:gone");
        let author = MockAuthor::new();
        let off = ProvideReconciler::new();
        assert_eq!(off.revoke_mode(), RevokeArmMode::Off);
        let sweep = revoke_passes(&off, &pool, &author, 8).await;
        assert!(!sweep.observed);
        assert!(revoked_targets(&author).await.is_empty());
        assert_eq!(
            ProvideReconciler::from_revoke_env(Some("apply")).revoke_mode(),
            RevokeArmMode::Apply
        );
    }
}
