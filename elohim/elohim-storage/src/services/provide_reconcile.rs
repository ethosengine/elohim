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
//! row, and authors a revocation only when a live actual row has NO desired key
//! (un-pin → withdraw the offer).
//!
//! ## The revoke arm (stranded rows only)
//!
//! A revocation is a notarized, irreversible act, so the arm is narrow:
//!
//! - it reads this node's rows under the author's own agent keys (the namespace
//!   they are written in), and considers only CONTENT provides — the commons
//!   capacity pledge shares the action and is never touched;
//! - a row is **stranded** when its scope (`head_ref`) is in NO desired provide.
//!   Several live rows for a scope that IS desired are duplicates, not stranded
//!   rows: the arm leaves every one of them standing, whatever their reach;
//! - it runs only under [`StrandedRevoke::Sweep`], which the caller passes when
//!   the desired set was fully computed this pass and the arm is switched on;
//! - a row must be seen stranded on [`REVOKE_AFTER_STRANDED_PASSES`] passes in
//!   a row before it is revoked, at most [`REVOKES_PER_TICK`] rows a pass, and
//!   once per row for the life of the process;
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

/// Passes in a row a row must be seen stranded before it is revoked. The tick
/// is 60 s, so a scope that drops out of the desired set for under two minutes
/// (a pin retired and revived, a content row replaced) costs no revocation and
/// no re-authoring. It is also the grace after boot: the first pass of a
/// process never revokes. The un-pin path revokes at once and does not wait
/// on this.
pub const REVOKE_AFTER_STRANDED_PASSES: u32 = 3;

/// Most revocations one pass may issue. Each is one `create_commitment` on the
/// node's single source chain, taken one at a time behind the chain write
/// gate, so four keeps the provide writer's turn at the gate to a few conductor
/// round trips. At a 60 s tick that is 240 an hour: a backlog of several
/// hundred rows drains within hours, and a wrong desired set costs at most four
/// irreversible acts a minute while the operator reaches the switch.
pub const REVOKES_PER_TICK: usize = 4;

/// Environment switch for the revoke arm, read once at startup. Unset or any
/// other value leaves the arm on; `0`, `false`, `no` or `off` turns it off.
pub const REVOKE_ARM_ENV: &str = "ELOHIM_PROVIDE_REVOKE_ARM";

/// Is the revoke arm switched on, given the raw value of [`REVOKE_ARM_ENV`]?
pub fn revoke_arm_enabled(raw: Option<&str>) -> bool {
    !matches!(
        raw.map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Some("0" | "false" | "no" | "off")
    )
}

/// Whether one reconcile pass may revoke stranded rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrandedRevoke {
    /// Issue no revocation: the arm is switched off, or the caller could not
    /// fully compute the desired set this pass.
    Hold,
    /// The desired set is complete: a row whose scope is outside it is stranded.
    Sweep,
}

/// What the revoke arm remembers about one live row, keyed by its `cid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StrandedRow {
    /// Seen stranded on this many passes in a row; not yet revoked.
    Seen(u32),
    /// A revocation was issued this lifetime; awaiting its projection.
    Issued,
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
}

/// A revocation the reconciler decided to author for a removed pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvideRevokeRequest {
    /// `cid` of the live commitment to revoke: its ENTRY hash, the key the
    /// revoke projection matches. Never the action hash (`dht_anchor_hash`).
    pub target_cid: String,
    pub head_ref: String,
}

/// Author seam — the conductor write of a `replicates-commons` Commitment or a
/// `revokes-commitment`. Production impl wraps `conductor_writes`; tests use
/// [`MockAuthor`] to assert exactly-once authoring across ticks and restarts.
#[async_trait]
pub trait CommitmentAuthor: Send + Sync {
    /// Author a replicates-commons Commitment for the given provide. Returns
    /// the new commitment action_hash (the back-reference stored on the pin).
    async fn author_commons(&self, req: &ProvideAuthorRequest) -> Result<String, StorageError>;

    /// Author a revokes-commitment targeting `req.target_cid`.
    async fn revoke_commons(&self, req: &ProvideRevokeRequest) -> Result<(), StorageError>;

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
    async fn revoke_commons(&self, req: &ProvideRevokeRequest) -> Result<(), StorageError> {
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

/// Is `row` a content provide, the only kind this loop authors? The commons
/// capacity pledge is projected under the same action with no counterparty and
/// `commons_bytes` in its bounds; it is not this loop's to revoke.
fn is_content_provide(row: &crate::db::models::MishpatCommitment) -> bool {
    !row.recipient.is_empty()
        && serde_json::from_str::<serde_json::Value>(&row.bounds_json)
            .ok()
            .is_none_or(|b| b.get("commons_bytes").is_none())
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
    /// Row `cid` → what the revoke arm has seen or done. A scope can hold
    /// several live rows, so revoke-once is kept per ROW, not per logical key.
    /// Emptied on restart: a revocation whose projection landed removed its row
    /// from the live set, so it is not issued again.
    stranded: Arc<Mutex<HashMap<String, StrandedRow>>>,
}

impl Default for ProvideReconciler {
    fn default() -> Self {
        Self::new()
    }
}

impl ProvideReconciler {
    pub fn new() -> Self {
        Self {
            latch: Arc::new(Mutex::new(HashMap::new())),
            stranded: Arc::new(Mutex::new(HashMap::new())),
        }
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
    /// `revoke` says whether this pass may withdraw stranded rows. Pass
    /// [`StrandedRevoke::Sweep`] only when `desired` is the node's WHOLE desired
    /// set; with a partial or unknown set every missing scope would read as
    /// stranded.
    ///
    /// Returns the number of NEW author calls issued this pass (for metrics/tests).
    pub async fn reconcile_provides<A: CommitmentAuthor + ?Sized>(
        &self,
        pool: &DbPool,
        author: &A,
        self_provider: &str,
        desired: &[DesiredProvide],
        revoke: StrandedRevoke,
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

            latch.insert(key.clone(), ProvideStage::Authoring);
            let req = ProvideAuthorRequest {
                provider: self_provider.to_string(),
                head_ref: d.head_ref.clone(),
                reach: d.reach.clone(),
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
                    // warn loudly. The reconciler's stranded-row revoke arm is
                    // the eventual backstop, but the back-ref is the fast path.
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
                                     un-pin revoke will fall back to the stranded-row arm"
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

        // ── revoke arm: live content rows whose scope left the desired set ───
        // Stranded is decided per SCOPE, never per row: a scope that is still
        // desired keeps every live row it has (duplicates included).
        let mut stranded = self.stranded.lock().await;
        match revoke {
            StrandedRevoke::Hold => {
                // No sighting counts on a pass that may not revoke.
                let live: HashSet<&str> = own_rows.iter().map(|r| r.cid.as_str()).collect();
                stranded.retain(|cid, s| *s == StrandedRow::Issued && live.contains(cid.as_str()));
            }
            StrandedRevoke::Sweep => {
                let desired_scopes: HashSet<&str> =
                    desired.iter().map(|d| d.head_ref.as_str()).collect();
                let mut rows: Vec<&crate::db::models::MishpatCommitment> = own_rows
                    .iter()
                    .filter(|r| is_content_provide(r) && row_stands_readably(r, now))
                    .filter(|r| !desired_scopes.contains(r.recipient.as_str()))
                    .collect();
                // Oldest first, so a backlog drains in a stable order.
                rows.sort_by(|a, b| {
                    a.created_at
                        .cmp(&b.created_at)
                        .then_with(|| a.cid.cmp(&b.cid))
                });

                let mut issued = 0usize;
                let mut conductor_refused = false;
                for row in &rows {
                    let seen = match stranded.get(&row.cid) {
                        // Issued once this lifetime; its projection is pending.
                        Some(StrandedRow::Issued) => continue,
                        Some(StrandedRow::Seen(n)) => n.saturating_add(1),
                        None => 1,
                    };
                    stranded.insert(row.cid.clone(), StrandedRow::Seen(seen));
                    if seen < REVOKE_AFTER_STRANDED_PASSES
                        || issued >= REVOKES_PER_TICK
                        || conductor_refused
                    {
                        continue;
                    }
                    let req = ProvideRevokeRequest {
                        target_cid: row.cid.clone(),
                        head_ref: row.recipient.clone(),
                    };
                    issued += 1;
                    match crate::chain_write_gate::as_writer(
                        crate::chain_write_gate::WriterKind::Provide,
                        author.revoke_commons(&req),
                    )
                    .await
                    {
                        Ok(()) => {
                            stranded.insert(row.cid.clone(), StrandedRow::Issued);
                            latch.insert(
                                (self_provider.to_string(), row.recipient.clone()),
                                ProvideStage::Revoked,
                            );
                            tracing::info!(
                                target: "elohim_storage::provide",
                                head_ref = %row.recipient,
                                target_cid = %row.cid,
                                "provide reconcile: revoked a stranded commitment (scope no longer desired)"
                            );
                        }
                        Err(e) => {
                            // The conductor is not answering: issue nothing more
                            // this pass. The row keeps its count and is retried.
                            conductor_refused = true;
                            tracing::warn!(
                                target: "elohim_storage::provide",
                                head_ref = %row.recipient,
                                target_cid = %row.cid,
                                error = %e,
                                "provide reconcile: revoke_commons failed; will retry next tick"
                            );
                        }
                    }
                }
                // Forget rows that are no longer stranded (their count starts
                // over if they strand again). An issued row is remembered for as
                // long as it is live, so it is never revoked twice.
                let live: HashSet<&str> = own_rows.iter().map(|r| r.cid.as_str()).collect();
                let now_stranded: HashSet<&str> = rows.iter().map(|r| r.cid.as_str()).collect();
                stranded.retain(|cid, s| {
                    live.contains(cid.as_str())
                        && (*s == StrandedRow::Issued || now_stranded.contains(cid.as_str()))
                });
            }
        }
        drop(stranded);

        // Prune the per-process latch. It is a pure optimisation, but without a
        // sweep it accumulates a key for every (provider, head_ref) ever seen —
        // notably terminal `Revoked` keys from un-pinned offers, which otherwise
        // never leave. Keep only keys still desired or still live in `actual`; a
        // key that is re-needed next pass is re-inserted by the walks above.
        let live: HashSet<(String, String)> = own_rows
            .iter()
            .map(|r| (self_provider.to_string(), r.recipient.clone()))
            .collect();
        latch.retain(|k, _| desired_keys.contains(k) || live.contains(k));

        Ok(authored)
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

    #[allow(clippy::too_many_arguments)]
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
            .reconcile_provides(
                &pool,
                &author,
                "agent:self",
                &desired,
                StrandedRevoke::Sweep,
            )
            .await
            .expect("first pass");
        assert_eq!(n, 2, "two unprovided keys → two authors");
        assert_eq!(author.authored.lock().await.len(), 2);

        // Latch now says Authoring for both — a second pass with NO projection
        // landing must NOT re-author (within-process dedup).
        let n2 = r
            .reconcile_provides(
                &pool,
                &author,
                "agent:self",
                &desired,
                StrandedRevoke::Sweep,
            )
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
            .reconcile_provides(
                &pool,
                &author,
                "agent:self",
                &[desired(1, "epr:album-1")],
                StrandedRevoke::Sweep,
            )
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
        r.reconcile_provides(
            &pool,
            &author,
            "agent:self",
            &[desired(1, "epr:album-1")],
            StrandedRevoke::Sweep,
        )
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
            .reconcile_provides(
                &pool,
                &author,
                "agent:self",
                &[desired(1, "epr:album-1")],
                StrandedRevoke::Sweep,
            )
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
            .reconcile_provides(
                &pool,
                &author,
                TRANSPORT_ID,
                &[desired(1, "epr:X")],
                StrandedRevoke::Sweep,
            )
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
            .reconcile_provides(
                &pool,
                &author,
                TRANSPORT_ID,
                &[desired(1, "epr:X")],
                StrandedRevoke::Sweep,
            )
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
            .reconcile_provides(
                &pool,
                &author,
                TRANSPORT_ID,
                &[desired(1, "epr:X")],
                StrandedRevoke::Sweep,
            )
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
            .reconcile_provides(
                &pool,
                &author,
                TRANSPORT_ID,
                &[desired(1, "epr:X")],
                StrandedRevoke::Sweep,
            )
            .await
            .expect("pass");
        assert_eq!(n, 1);
        assert_eq!(author.authored.lock().await.len(), 1);
    }

    const FUTURE: &str = "2099-01-01T00:00:00Z";
    const PAST: &str = "2020-01-01T00:00:00Z";

    fn bounds_with_reach(reach: &str) -> String {
        format!(r#"{{"reach":"{reach}","reach_ceiling":"commons","rate_per_minute":6}}"#)
    }

    async fn reconcile_once(pool: &DbPool, author: &MockAuthor, d: DesiredProvide) -> usize {
        ProvideReconciler::new()
            .reconcile_provides(pool, author, TRANSPORT_ID, &[d], StrandedRevoke::Sweep)
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
            .reconcile_provides(
                &pool,
                &author,
                TRANSPORT_ID,
                std::slice::from_ref(&d),
                StrandedRevoke::Sweep,
            )
            .await
            .expect("first");
        assert_eq!(first, 1);
        {
            let got = author.authored.lock().await;
            assert_eq!(got.len(), 1);
            assert_eq!(got[0].reach, "commons");
        }
        let second = r
            .reconcile_provides(
                &pool,
                &author,
                TRANSPORT_ID,
                std::slice::from_ref(&d),
                StrandedRevoke::Sweep,
            )
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

    // ── revoke arm ──────────────────────────────────────────────────────

    /// Run `passes` sweep passes over `desired` (the whole desired set).
    async fn sweep<A: CommitmentAuthor>(
        r: &ProvideReconciler,
        pool: &DbPool,
        author: &A,
        desired: &[DesiredProvide],
        passes: u32,
    ) {
        for _ in 0..passes {
            r.reconcile_provides(pool, author, TRANSPORT_ID, desired, StrandedRevoke::Sweep)
                .await
                .expect("sweep pass");
        }
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
            author: MockAuthor::DEFAULT_PROVIDER.into(),
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
            MockAuthor::DEFAULT_PROVIDER,
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
    fn project_revocation(pool: &DbPool, target: &str) {
        let payload = crate::services::conductor_commitment_author::build_revoke_payload(
            target,
            "2026-10-13T00:00:00+00:00",
        );
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
            crate::db::mishpat_commitments::live_commons_provides_for_provider(
                &mut conn,
                MockAuthor::DEFAULT_PROVIDER,
            )
            .expect("live rows")
            .into_iter()
            .map(|r| r.cid)
            .collect();
        cids.sort();
        cids
    }

    /// An author whose revocation takes the production path back into the
    /// database: the production payload, through `handle_mishpat_signal`.
    struct ProjectingAuthor {
        pool: DbPool,
        revoked: Mutex<Vec<ProvideRevokeRequest>>,
    }

    #[async_trait]
    impl CommitmentAuthor for ProjectingAuthor {
        async fn own_providers(&self) -> Vec<String> {
            vec![MockAuthor::DEFAULT_PROVIDER.to_string()]
        }
        async fn author_commons(&self, req: &ProvideAuthorRequest) -> Result<String, StorageError> {
            Err(StorageError::Internal(format!(
                "unexpected author for {}",
                req.head_ref
            )))
        }
        async fn revoke_commons(&self, req: &ProvideRevokeRequest) -> Result<(), StorageError> {
            self.revoked.lock().await.push(req.clone());
            project_revocation(&self.pool, &req.target_cid);
            Ok(())
        }
    }

    #[tokio::test]
    async fn a_revocation_issued_by_the_arm_marks_the_row_it_targets() {
        let pool = test_pool();
        project_content_commitment(&pool, "epr:gone", "uhCEkStranded", "uhCkkStranded");
        project_content_commitment(&pool, "epr:kept", "uhCEkKept", "uhCkkKept");
        assert_eq!(live_cids(&pool), vec!["uhCEkKept", "uhCEkStranded"]);

        let author = ProjectingAuthor {
            pool: pool.clone(),
            revoked: Mutex::new(Vec::new()),
        };
        let r = ProvideReconciler::new();
        let kept = [desired(1, "epr:kept")];
        sweep(&r, &pool, &author, &kept, REVOKE_AFTER_STRANDED_PASSES).await;

        {
            let revoked = author.revoked.lock().await;
            assert_eq!(revoked.len(), 1);
            assert_eq!(
                revoked[0].target_cid, "uhCEkStranded",
                "the target is the entry hash, the projection's key"
            );
            assert_eq!(revoked[0].head_ref, "epr:gone");
        }
        assert_eq!(
            live_cids(&pool),
            vec!["uhCEkKept"],
            "the revocation the arm issued marked the row it targeted"
        );

        // The row has left the live set: later passes issue nothing.
        sweep(&r, &pool, &author, &kept, 5).await;
        assert_eq!(author.revoked.lock().await.len(), 1);
    }

    #[test]
    fn a_revocation_targeting_the_action_hash_marks_nothing() {
        // The old arm's target. The projection matches `cid`, the entry hash.
        let pool = test_pool();
        project_content_commitment(&pool, "epr:gone", "uhCEkStranded", "uhCkkStranded");
        project_revocation(&pool, "uhCkkStranded");
        assert_eq!(live_cids(&pool), vec!["uhCEkStranded"]);
        project_revocation(&pool, "uhCEkStranded");
        assert!(live_cids(&pool).is_empty());
    }

    #[tokio::test]
    async fn a_stranded_row_is_revoked_once() {
        // MockAuthor projects nothing, so the row stays live after the
        // revocation: the worst case for re-issue.
        let pool = test_pool();
        seed_commons_row(&pool, "cid:stranded", "epr:gone", "proposed", false);
        let author = MockAuthor::new();
        let r = ProvideReconciler::new();

        // An empty desired set that is known still revokes.
        sweep(&r, &pool, &author, &[], REVOKE_AFTER_STRANDED_PASSES).await;
        assert_eq!(revoked_targets(&author).await, vec!["cid:stranded"]);
        assert_eq!(
            r.stage(TRANSPORT_ID, "epr:gone").await,
            Some(ProvideStage::Revoked)
        );

        sweep(&r, &pool, &author, &[], 10).await;
        assert_eq!(
            revoked_targets(&author).await,
            vec!["cid:stranded"],
            "one revocation per row, however long its projection takes"
        );
        assert!(author.authored.lock().await.is_empty());
    }

    #[tokio::test]
    async fn a_row_is_revoked_only_after_it_stays_stranded() {
        let pool = test_pool();
        seed_commons_row(&pool, "cid:flap", "epr:flap", "active", false);
        let author = MockAuthor::new();
        let r = ProvideReconciler::new();

        sweep(&r, &pool, &author, &[], REVOKE_AFTER_STRANDED_PASSES - 1).await;
        assert!(revoked_targets(&author).await.is_empty());

        // The scope is desired again for one pass: the count starts over.
        sweep(&r, &pool, &author, &[desired(1, "epr:flap")], 1).await;
        sweep(&r, &pool, &author, &[], REVOKE_AFTER_STRANDED_PASSES - 1).await;
        assert!(revoked_targets(&author).await.is_empty());

        sweep(&r, &pool, &author, &[], 1).await;
        assert_eq!(revoked_targets(&author).await, vec!["cid:flap"]);
    }

    #[tokio::test]
    async fn revocations_are_bounded_per_tick() {
        let pool = test_pool();
        let total = REVOKES_PER_TICK * 2 + 2;
        for i in 0..total {
            seed_commons_row(
                &pool,
                &format!("cid:gone-{i:02}"),
                &format!("epr:gone-{i:02}"),
                "active",
                false,
            );
        }
        let author = MockAuthor::new();
        let r = ProvideReconciler::new();

        sweep(&r, &pool, &author, &[], REVOKE_AFTER_STRANDED_PASSES - 1).await;
        assert!(revoked_targets(&author).await.is_empty());
        sweep(&r, &pool, &author, &[], 1).await;
        assert_eq!(revoked_targets(&author).await.len(), REVOKES_PER_TICK);
        sweep(&r, &pool, &author, &[], 1).await;
        assert_eq!(revoked_targets(&author).await.len(), REVOKES_PER_TICK * 2);
        sweep(&r, &pool, &author, &[], 5).await;
        let all = revoked_targets(&author).await;
        assert_eq!(all.len(), total, "the backlog drains, then stops");
        let distinct: HashSet<&String> = all.iter().collect();
        assert_eq!(distinct.len(), total, "no row is revoked twice");
    }

    #[tokio::test]
    async fn nothing_is_revoked_while_desire_is_unknown() {
        // `Hold` is what the caller passes when it could not compute the whole
        // desired set, and what the disable switch turns every pass into.
        let pool = test_pool();
        seed_commons_row(&pool, "cid:stranded", "epr:gone", "active", false);
        let author = MockAuthor::new();
        let r = ProvideReconciler::new();
        for _ in 0..10 {
            r.reconcile_provides(&pool, &author, TRANSPORT_ID, &[], StrandedRevoke::Hold)
                .await
                .expect("hold pass");
        }
        assert!(revoked_targets(&author).await.is_empty());

        // Held passes count for nothing: the row still needs its full run.
        sweep(&r, &pool, &author, &[], REVOKE_AFTER_STRANDED_PASSES - 1).await;
        assert!(revoked_targets(&author).await.is_empty());
        r.reconcile_provides(&pool, &author, TRANSPORT_ID, &[], StrandedRevoke::Hold)
            .await
            .expect("hold pass");
        sweep(&r, &pool, &author, &[], REVOKE_AFTER_STRANDED_PASSES - 1).await;
        assert!(revoked_targets(&author).await.is_empty());
    }

    #[test]
    fn the_revoke_arm_switch_defaults_on_and_turns_off() {
        assert!(revoke_arm_enabled(None));
        assert!(revoke_arm_enabled(Some("")));
        assert!(revoke_arm_enabled(Some("1")));
        assert!(revoke_arm_enabled(Some("on")));
        for off in ["0", "false", "no", "off", " OFF ", "False"] {
            assert!(!revoke_arm_enabled(Some(off)), "{off:?} turns the arm off");
        }
    }

    #[tokio::test]
    async fn duplicates_of_a_desired_scope_are_left_standing() {
        // The scope boundary: stranded is decided per scope. A desired scope
        // keeps every live row it has; de-duplicating them is a separate,
        // owner-authored pass.
        let pool = test_pool();
        for i in 0..4 {
            seed_commons_row(&pool, &format!("cid:dup-{i}"), "epr:X", "active", false);
        }
        // A duplicate that declares another reach is still a row of a desired scope.
        seed_commons_row_full(
            &pool,
            MockAuthor::DEFAULT_PROVIDER,
            "cid:dup-household",
            "epr:X",
            "active",
            false,
            FUTURE,
            &bounds_with_reach("household"),
        );
        // An undesired scope loses every row it has, duplicates included.
        for i in 0..3 {
            seed_commons_row(&pool, &format!("cid:gone-{i}"), "epr:gone", "active", false);
        }
        let author = MockAuthor::new();
        let r = ProvideReconciler::new();
        sweep(&r, &pool, &author, &[desired(1, "epr:X")], 8).await;

        let mut revoked = revoked_targets(&author).await;
        revoked.sort();
        assert_eq!(revoked, vec!["cid:gone-0", "cid:gone-1", "cid:gone-2"]);
        assert!(author.authored.lock().await.is_empty());
    }

    #[tokio::test]
    async fn only_standing_content_rows_of_this_node_are_revoked() {
        let pool = test_pool();
        // The commons capacity pledge: same action, no counterparty.
        seed_commons_row_full(
            &pool,
            MockAuthor::DEFAULT_PROVIDER,
            "cid:capacity",
            "",
            "proposed",
            false,
            "",
            r#"{"commons_bytes":26843545600,"rate_per_minute":12,"reach_ceiling":"commons"}"#,
        );
        seed_commons_row_full(
            &pool,
            MockAuthor::DEFAULT_PROVIDER,
            "cid:capacity-with-recipient",
            "epr:pledge",
            "proposed",
            false,
            FUTURE,
            r#"{"commons_bytes":1,"reach_ceiling":"commons"}"#,
        );
        // A lapsed offer needs no revocation; unreadable validity stops the write.
        let b = bounds_with_reach("commons");
        seed_commons_row_full(
            &pool,
            MockAuthor::DEFAULT_PROVIDER,
            "cid:lapsed",
            "epr:lapsed",
            "active",
            false,
            PAST,
            &b,
        );
        seed_commons_row_full(
            &pool,
            MockAuthor::DEFAULT_PROVIDER,
            "cid:junk",
            "epr:junk",
            "active",
            false,
            "not-a-date",
            &b,
        );
        // Rows under another agent, and under this node's transport id.
        seed_commons_row_for(
            &pool,
            "uhCAkSomeoneElse",
            "cid:theirs",
            "epr:theirs",
            "active",
            false,
        );
        seed_commons_row_for(
            &pool,
            TRANSPORT_ID,
            "cid:transport",
            "epr:transport",
            "active",
            false,
        );
        seed_commons_row(&pool, "cid:stranded", "epr:gone", "active", false);

        let author = MockAuthor::new();
        let r = ProvideReconciler::new();
        sweep(&r, &pool, &author, &[], 8).await;
        assert_eq!(revoked_targets(&author).await, vec!["cid:stranded"]);
    }

    /// An author whose conductor refuses revocations while `refuse` is set.
    #[derive(Default)]
    struct RefusingAuthor {
        refuse: std::sync::atomic::AtomicBool,
        attempts: std::sync::atomic::AtomicUsize,
        inner: MockAuthor,
    }

    #[async_trait]
    impl CommitmentAuthor for RefusingAuthor {
        async fn own_providers(&self) -> Vec<String> {
            self.inner.own_providers().await
        }
        async fn author_commons(&self, req: &ProvideAuthorRequest) -> Result<String, StorageError> {
            self.inner.author_commons(req).await
        }
        async fn revoke_commons(&self, req: &ProvideRevokeRequest) -> Result<(), StorageError> {
            use std::sync::atomic::Ordering;
            self.attempts.fetch_add(1, Ordering::SeqCst);
            if self.refuse.load(Ordering::SeqCst) {
                return Err(StorageError::Internal("conductor not answering".into()));
            }
            self.inner.revoke_commons(req).await
        }
    }

    #[tokio::test]
    async fn a_refused_revocation_stops_the_pass_and_is_retried() {
        use std::sync::atomic::Ordering;
        let pool = test_pool();
        for i in 0..REVOKES_PER_TICK + 1 {
            seed_commons_row(
                &pool,
                &format!("cid:gone-{i}"),
                &format!("epr:gone-{i}"),
                "active",
                false,
            );
        }
        let author = RefusingAuthor::default();
        author.refuse.store(true, Ordering::SeqCst);
        let r = ProvideReconciler::new();

        sweep(&r, &pool, &author, &[], REVOKE_AFTER_STRANDED_PASSES).await;
        assert_eq!(
            author.attempts.load(Ordering::SeqCst),
            1,
            "one refusal ends the pass"
        );
        assert!(revoked_targets(&author.inner).await.is_empty());

        author.refuse.store(false, Ordering::SeqCst);
        sweep(&r, &pool, &author, &[], 1).await;
        assert_eq!(revoked_targets(&author.inner).await.len(), REVOKES_PER_TICK);
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
}
