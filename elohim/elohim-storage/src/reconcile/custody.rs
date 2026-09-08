//! Custody reconciliation controller.
//!
//! Multi-trigger reconcile pass:
//! - Gossip arrival (`BlobInventorySnapshot` / `BlobInventoryDelta` for peer X)
//! - Connection event (`ConnectionEstablished` for peer X)
//! - Periodic sweep (timer; cadence per `custody_sweep_seconds`)
//!
//! Each trigger calls `reconcile_pass`, which is idempotent.
//!
//! For each `custody-blob` commitment in `rea_commitments`:
//! - If this peer is the provider AND blob_hash is missing locally:
//!   query `peer_blob_inventory` for hosts; if any candidates exist,
//!   kick a fetch via `blob_fetch::race_fetch` (T17). When the inventory
//!   is EMPTY, fall back to racing the currently-connected peers (capped
//!   at [`FALLBACK_CANDIDATE_CAP`]) — gossip inventory publish dies on
//!   every pod restart (`InsufficientPeers`), so an empty inventory is
//!   NOT evidence the mesh lacks the blob. Mirrors `get_blob_or_heal`'s
//!   connected-fallback, which is what makes the CI cross-pod fetch pass.
//! - If this peer is the receiver AND last_seen for the provider is older
//!   than `placement_grace_seconds`: emit a `placement-gap` REA event
//!   (with cooldown).
//!
//! Local store presence is queried via the trait `LocalBlobStore` so the
//! reconcile pass can be unit-tested without a real blob store.

use crate::blob_store::BlobStore;
use crate::db::diesel_schema::{economic_events, peer_blob_inventory, rea_commitments};
use crate::db::models::{NewEconomicEvent, ReaCommitment};
use crate::error::StorageError;
use chrono::{DateTime, Utc};
use diesel::prelude::*;

use super::placement::{PlacementCandidate, PlacementStrategy};

/// Trait for checking whether a blob exists in this peer's local store.
/// Production: queries the blob_store. Tests: returns a fixed set.
pub trait LocalBlobStore: Send + Sync {
    fn has(&self, blob_hash: &str) -> bool;
}

/// Trait for kicking a fetch. Production: calls `blob_fetch::race_fetch` (T17).
/// Tests: records the kick request.
pub trait FetchKicker: Send + Sync {
    fn kick(&self, blob_hash: &str, candidates: Vec<String>);
}

/// Snapshot of the local blob store taken at reconcile-pass start.
///
/// Implements [`LocalBlobStore`] for the duration of a single pass by
/// pre-fetching the full hash set once via [`crate::blob_store::BlobStore::list_hashes`]
/// and answering `has` from a [`std::collections::HashSet`]. This is cheaper
/// (and avoids holding the blob store across diesel queries) than calling
/// `BlobStore::exists` per commitment.
///
/// Hash format: matches whatever `list_hashes` returns (currently
/// `sha256-<hex>`); reconcile_pass does string-equality lookups against
/// `rea_commitments.resource_classified_as`, so the two stores must share a
/// canonical form. T17 / T22 already standardised on `sha256-<hex>`.
pub struct BlobStoreSnapshot {
    hashes: std::collections::HashSet<String>,
}

impl BlobStoreSnapshot {
    /// Build a snapshot by reading the local pantry once. Returns
    /// [`StorageError`] if the directory walk fails; the caller should skip
    /// the reconcile tick rather than running with a partial view.
    pub fn from_store(store: &crate::blob_store::BlobStore) -> Result<Self, StorageError> {
        let hashes = store.list_hashes()?.into_iter().collect();
        Ok(Self { hashes })
    }

    /// Number of hashes captured in the snapshot. Useful for tests + tracing.
    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    /// `true` when the snapshot captured no hashes.
    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }

    /// Copy-on-write: a new snapshot that also contains `blob_hash`.
    ///
    /// Used by [`SnapshotCache::note_stored`] so a blob that just landed is
    /// visible to the very next pass without a fresh `O(corpus)` pantry walk.
    pub fn inserted(&self, blob_hash: &str) -> Self {
        let mut hashes = self.hashes.clone();
        hashes.insert(blob_hash.to_string());
        Self { hashes }
    }
}

impl LocalBlobStore for BlobStoreSnapshot {
    fn has(&self, blob_hash: &str) -> bool {
        self.hashes.contains(blob_hash)
    }
}

/// Cap on connected peers raced by an inventory-blind fallback kick.
/// Mirrors the `get_blob_or_heal` connected-fallback cap in `http.rs`.
pub const FALLBACK_CANDIDATE_CAP: usize = 8;

/// TTL for the cached pantry snapshot. The reconcile sweep ticks every
/// `custody_sweep_seconds` (and again on every `ConnectionEstablished`), but a
/// full `list_hashes` walk is `O(corpus)` — ~3 590 `read_dir` entries plus a
/// `metadata()` stat each on the large-corpus household peer. Rebuilding that
/// per pass at ~8 passes/min is the CPU floor the 2026-09-07 diagnosis measured
/// (1 345 MB RSS / ~150-195 % CPU against 330 MB / 9 % siblings on the same
/// binary). The walk now runs at most once per TTL; blobs that land in between
/// are folded in by [`SnapshotCache::note_stored`], so freshness does not
/// depend on the TTL for the one case that matters (a kicked fetch hitting).
pub const SNAPSHOT_CACHE_TTL_SECONDS: u64 = 60;

/// Cached [`BlobStoreSnapshot`], rebuilt on its own slow cadence rather than
/// once per reconcile pass.
///
/// `now` is a caller-supplied [`std::time::Instant`] so the cadence is
/// deterministic under test. Interior mutability (one `Mutex` around an
/// `Option`) keeps the cache shareable behind an `Arc` without making the
/// reconcile pass async.
pub struct SnapshotCache {
    ttl: std::time::Duration,
    state: std::sync::Mutex<Option<CachedSnapshot>>,
    rebuilds: std::sync::atomic::AtomicU64,
}

struct CachedSnapshot {
    built_at: std::time::Instant,
    snapshot: std::sync::Arc<BlobStoreSnapshot>,
}

impl Default for SnapshotCache {
    fn default() -> Self {
        Self::new(std::time::Duration::from_secs(SNAPSHOT_CACHE_TTL_SECONDS))
    }
}

impl SnapshotCache {
    pub fn new(ttl: std::time::Duration) -> Self {
        Self {
            ttl,
            state: std::sync::Mutex::new(None),
            rebuilds: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Return the cached snapshot, walking the pantry only when there is none
    /// or the cached one is older than the TTL.
    pub fn get(
        &self,
        store: &crate::blob_store::BlobStore,
        now: std::time::Instant,
    ) -> Result<std::sync::Arc<BlobStoreSnapshot>, StorageError> {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cached) = guard.as_ref() {
            if now.saturating_duration_since(cached.built_at) < self.ttl {
                return Ok(cached.snapshot.clone());
            }
        }
        let snapshot = std::sync::Arc::new(BlobStoreSnapshot::from_store(store)?);
        self.rebuilds
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        crate::metrics::inc_custody_snapshot_rebuild();
        *guard = Some(CachedSnapshot {
            built_at: now,
            snapshot: snapshot.clone(),
        });
        Ok(snapshot)
    }

    /// Drop the cached snapshot; the next [`SnapshotCache::get`] walks the
    /// pantry again.
    pub fn invalidate(&self) {
        *self.state.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Fold a blob that just landed into the cached snapshot WITHOUT a walk.
    ///
    /// Called on every `FetchOutcome::Hit`. Without it a cached snapshot would
    /// keep reporting the hash as missing until the TTL expired, and the sweep
    /// would re-kick a blob it already holds — the storm this cache is meant to
    /// end. Copy-on-write: the shared `Arc` is replaced, never mutated, so
    /// in-flight passes keep a consistent view.
    pub fn note_stored(&self, blob_hash: &str) {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cached) = guard.as_mut() {
            cached.snapshot = std::sync::Arc::new(cached.snapshot.inserted(blob_hash));
        }
    }

    /// Number of full pantry walks performed. Test + tracing hook.
    pub fn rebuilds(&self) -> u64 {
        self.rebuilds.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Whether a missing-blob target may be kicked on this pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KickAdmission {
    /// Kick now.
    Fire,
    /// An earlier attempt's backoff window is still open.
    Backoff,
    /// Terminal until re-armed: the configured number of consecutive
    /// inventory-blind attempts produced no bytes, so no reachable peer is
    /// known to hold this blob. `first` is true exactly once per exhaustion so
    /// the caller emits the honest placement-gap signal once, not per pass.
    Exhausted { first: bool },
    /// The ledger is at capacity and declined to track this hash; the caller
    /// degrades to the pre-ledger behaviour (always kick).
    Untracked,
}

/// Backoff / exhaustion policy for [`CustodyKickLedger`].
#[derive(Debug, Clone, Copy)]
pub struct KickPolicy {
    /// Backoff after the first non-hit attempt; doubles per attempt.
    pub base_backoff_seconds: u64,
    /// Ceiling for the doubling.
    pub max_backoff_seconds: u64,
    /// Consecutive inventory-blind attempts before a target is declared
    /// unsatisfiable.
    pub blind_attempt_limit: u32,
    /// Hard bound on tracked hashes so the ledger cannot grow without limit.
    pub max_tracked: usize,
}

impl Default for KickPolicy {
    fn default() -> Self {
        Self {
            base_backoff_seconds: 30,
            max_backoff_seconds: 300,
            blind_attempt_limit: 5,
            max_tracked: 4096,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct KickRecord {
    /// Consecutive attempts since the last hit (drives the backoff curve).
    attempts: u32,
    /// Consecutive attempts made with ZERO named holders in the gossip
    /// inventory (drives exhaustion).
    blind_attempts: u32,
    next_eligible_at: Option<DateTime<Utc>>,
    exhausted: bool,
    gap_signalled: bool,
}

/// The missing convergence condition, made explicit.
///
/// Before this ledger the sweep had a *satisfied* state (bytes present) and a
/// *malformed* state (`FetchOutcome::InvalidAddress`) but **no exhausted
/// state**: a custody-blob commitment whose bytes no reachable peer holds was
/// re-kicked every pass forever — 502 kicks in 11 minutes for the same 5
/// commitments on the 2026-09-07 household mesh, at full fan-out, while the
/// substrate never said "this obligation cannot be met right now."
///
/// The ledger is deliberately in-memory (a `DashMap`) and need not survive
/// restart: a restart genuinely changes the peer set, which is one of the two
/// re-arm conditions anyway.
///
/// **Failure is the default.** An attempt increments the counters at admission
/// time; only [`CustodyKickLedger::record_hit`] clears them. A kick is
/// scheduled asynchronously, so the pass cannot know its outcome — treating
/// "attempted and not since cleared" as a non-hit keeps the whole decision
/// synchronous and deterministic, and the backoff window (30 s and doubling) is
/// wider than the fetch timeout, so a slow success still clears before the next
/// admission.
pub struct CustodyKickLedger {
    entries: dashmap::DashMap<String, KickRecord>,
    policy: KickPolicy,
}

impl Default for CustodyKickLedger {
    fn default() -> Self {
        Self::new(KickPolicy::default())
    }
}

impl CustodyKickLedger {
    pub fn new(policy: KickPolicy) -> Self {
        Self {
            entries: dashmap::DashMap::new(),
            policy,
        }
    }

    fn backoff_for(&self, attempts: u32) -> chrono::Duration {
        let shift = attempts.saturating_sub(1).min(16);
        let secs = self
            .policy
            .base_backoff_seconds
            .saturating_mul(1u64 << shift)
            .min(self.policy.max_backoff_seconds);
        chrono::Duration::seconds(secs as i64)
    }

    /// Decide whether `blob_hash` may be kicked now, and record the attempt.
    ///
    /// `named_candidates` is true when the gossip inventory named at least one
    /// holder for this hash. A named holder is evidence the mesh has the bytes,
    /// so it RE-ARMS an exhausted target — this is re-arm condition (a) from
    /// the cure design, and it needs no external hook because the pass already
    /// queries the inventory on every examination.
    pub fn admit(
        &self,
        blob_hash: &str,
        named_candidates: bool,
        now: DateTime<Utc>,
    ) -> KickAdmission {
        if !self.entries.contains_key(blob_hash) && self.entries.len() >= self.policy.max_tracked {
            return KickAdmission::Untracked;
        }
        let mut entry = self.entries.entry(blob_hash.to_string()).or_default();

        if named_candidates && (entry.exhausted || entry.blind_attempts > 0) {
            // Re-arm (a): the inventory now names a holder.
            entry.exhausted = false;
            entry.gap_signalled = false;
            entry.blind_attempts = 0;
        }

        if entry.exhausted {
            let first = !entry.gap_signalled;
            entry.gap_signalled = true;
            return KickAdmission::Exhausted { first };
        }

        if let Some(next) = entry.next_eligible_at {
            if now < next {
                return KickAdmission::Backoff;
            }
        }

        entry.attempts = entry.attempts.saturating_add(1);
        if !named_candidates {
            entry.blind_attempts = entry.blind_attempts.saturating_add(1);
        }
        entry.next_eligible_at = Some(now + self.backoff_for(entry.attempts));
        if !named_candidates && entry.blind_attempts >= self.policy.blind_attempt_limit {
            entry.exhausted = true;
        }
        KickAdmission::Fire
    }

    /// The bytes landed: forget everything about this hash.
    pub fn record_hit(&self, blob_hash: &str) {
        self.entries.remove(blob_hash);
    }

    /// Re-arm condition (b): a new peer connected, so the candidate set this
    /// node can reach genuinely changed. Clears exhaustion and the backoff
    /// window for every tracked hash — the next pass gets a fresh try.
    pub fn rearm_all(&self) {
        for mut entry in self.entries.iter_mut() {
            entry.exhausted = false;
            entry.gap_signalled = false;
            entry.blind_attempts = 0;
            entry.next_eligible_at = None;
        }
    }

    /// Number of hashes currently tracked. Test + tracing hook.
    pub fn tracked(&self) -> usize {
        self.entries.len()
    }
}

/// Cross-pass state for the custody reconcile sweep, owned by the node.
///
/// Three things that must NOT be rebuilt per pass live here together: the kick
/// ledger (backoff + exhaustion), the cached pantry snapshot, and the rotating
/// commitment-scan cursor.
#[derive(Default)]
pub struct CustodySweepState {
    pub ledger: CustodyKickLedger,
    pub snapshot: SnapshotCache,
    /// Rows examined per pass; `<= 0` means unbounded.
    scan_limit: i64,
    scan_cursor: std::sync::atomic::AtomicI64,
    pass_guard: PassGuard,
}

/// One-permit single-flight lane for the custody reconcile pass.
///
/// The sweep is driven from BOTH a timer arm and a `ConnectionEstablished`
/// trigger. On the 2026-09-07 household mesh those stacked: passes were logged
/// 0.54 s apart while two runtime threads sat at 82-95 % CPU on a 2-worker
/// server runtime. One permit means a second trigger is COUNTED and dropped
/// rather than doubling the pass cost — and the sweep can never occupy more
/// than one of those two threads.
pub struct PassGuard {
    sem: std::sync::Arc<tokio::sync::Semaphore>,
}

impl Default for PassGuard {
    fn default() -> Self {
        Self {
            sem: std::sync::Arc::new(tokio::sync::Semaphore::new(1)),
        }
    }
}

impl PassGuard {
    /// Enter the lane. `None` means a pass is already in flight; the overlap is
    /// recorded on `storage_custody_pass_skipped_overlap_total`. The returned
    /// permit must be held for the whole pass.
    pub fn try_enter(&self) -> Option<tokio::sync::OwnedSemaphorePermit> {
        match self.sem.clone().try_acquire_owned() {
            Ok(permit) => Some(permit),
            Err(_) => {
                crate::metrics::inc_custody_pass_skipped_overlap();
                None
            }
        }
    }
}

impl CustodySweepState {
    /// Bound each pass to `scan_limit` custody-blob rows, rotating the window
    /// so every row is still examined across passes. `0` (or negative) keeps
    /// the pre-existing unbounded scan.
    pub fn with_scan_limit(scan_limit: i64) -> Self {
        Self {
            scan_limit,
            ..Default::default()
        }
    }

    /// Single-flight lane for the reconcile pass.
    pub fn pass_guard(&self) -> &PassGuard {
        &self.pass_guard
    }

    /// Offset for this pass, advanced (and wrapped) by the caller via
    /// [`CustodySweepState::advance_cursor`].
    fn scan_window(&self) -> Option<(i64, i64)> {
        if self.scan_limit <= 0 {
            return None;
        }
        Some((
            self.scan_limit,
            self.scan_cursor
                .load(std::sync::atomic::Ordering::Relaxed)
                .max(0),
        ))
    }

    /// Advance the rotating cursor. A short page means we reached the end of
    /// the table, so the next pass starts over at 0 — correct without needing a
    /// separate `COUNT(*)`.
    fn advance_cursor(&self, offset: i64, limit: i64, returned: usize) {
        let next = if (returned as i64) < limit {
            0
        } else {
            offset.saturating_add(limit)
        };
        self.scan_cursor
            .store(next, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Outcome of a single reconcile pass; useful for tests + metrics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReconcileOutcome {
    pub kicks_fired: u32,
    /// Subset of `kicks_fired` that raced connected peers because the
    /// gossip inventory had no candidates (inventory-blind fallback).
    pub fallback_kicks: u32,
    pub placement_gaps_emitted: u32,
    pub commitments_examined: u32,
    /// Commitments whose blob marker is not a valid content address in ANY
    /// accepted form (`blob_fetch::normalize_fetch_address` refused it).
    /// These are skipped WITHOUT a fetch kick: no responder can accept the
    /// address, so retrying every sweep tick would drum the wire forever
    /// (the T21 rejection drumbeat). The count is the honest failure record
    /// — a non-zero value means rows need healing, not more retries.
    pub invalid_markers: u32,
    /// Missing own-provider targets NOT kicked this pass because an earlier
    /// attempt's backoff window was still open. Deferred, not abandoned.
    pub kicks_deferred: u32,
    /// Missing own-provider targets refused because they are EXHAUSTED — the
    /// configured number of consecutive inventory-blind attempts produced no
    /// bytes, so no reachable peer is known to hold them. This is the
    /// convergence signal the sweep previously lacked entirely.
    pub kicks_exhausted: u32,
    /// Kicks fired without ledger tracking because the ledger was at capacity
    /// (honest degradation to the pre-ledger always-kick behaviour).
    pub kicks_untracked: u32,
}

/// Timing parameters for a reconcile pass.
#[derive(Debug, Clone, Copy)]
pub struct ReconcileConfig {
    /// How long a custody commitment can be unhonored before placement-gap fires.
    pub placement_grace_seconds: u64,
    /// Minimum time between repeated placement-gap events for the same commitment.
    pub placement_gap_cooldown_seconds: u64,
    /// TTL for peer_blob_inventory entries before they're considered stale.
    pub inventory_freshness_seconds: u64,
}

/// Run one reconcile pass over the custody-blob commitments visible in this
/// peer's projection. Idempotent.
/// `self_cid` is this node's TRANSPORT id (`Config::self_cid`); `self_agent_cid`
/// is its resolved holochain `agent_cid` (`uhCAk…`), when available. Self-matching
/// (own-provider → fetch-on-miss; own-receiver → placement-gap) matches a
/// commitment row whose `provider`/`receiver` equals EITHER namespace. This is
/// the transition semantics: legacy rows (and pre-resolver salvage writes) carry
/// the transport id, while new salvage/seeder writes carry the agent_cid — so
/// both must be recognised or the node's own rows are orphaned (the fetch never
/// fires for an agent_cid-provider row this node authored). `self_agent_cid` is
/// `None` when the conductor cell key was not resolvable at P2P boot; matching
/// Resolve THIS node's `agent_cid` for reconcile self-matching, IN LOCKSTEP with
/// the salvage author's per-tick resolution
/// (`services::salvage_commitment_author::resolve_self_agent_cid`): the active
/// session's `agent_pub_key` re-resolved from `conn` on EVERY pass, falling back
/// to the boot-time conductor cell-key snapshot. Re-resolving the session arm per
/// pass is what keeps the two in lockstep — otherwise a session that logs in (or
/// changes) AFTER P2P boot would make salvage author `provider = <session
/// agent_cid>` rows that a frozen reconcile value could never match (the orphaned
/// own-row class). Returns `None` only when neither candidate is an `agent_cid`;
/// matching then degrades to transport-only (the pre-existing behavior).
pub fn resolve_self_agent_cid(
    conn: &mut SqliteConnection,
    cellkey_snapshot: Option<&str>,
) -> Option<String> {
    let session_cid = crate::db::local_sessions::get_active_session(conn)
        .ok()
        .flatten()
        .map(|s| s.agent_pub_key);
    crate::identity_namespace::resolve_agent_cid_write(&[session_cid.as_deref(), cellkey_snapshot])
}

/// then degrades to transport-only (the pre-existing behavior — no regression).
///
/// `state` carries the cross-pass memory the sweep needs to CONVERGE: the kick
/// ledger (backoff + exhaustion) and the rotating commitment-scan cursor.
/// Without it every pass is amnesiac and an unsatisfiable custody obligation is
/// re-kicked forever.
#[allow(clippy::too_many_arguments)]
pub fn reconcile_pass_with_state(
    conn: &mut SqliteConnection,
    self_cid: &str,
    self_agent_cid: Option<&str>,
    local_store: &dyn LocalBlobStore,
    fetch_kicker: &dyn FetchKicker,
    cfg: ReconcileConfig,
    now: DateTime<Utc>,
    connected_peers: &[String],
    state: &CustodySweepState,
) -> Result<ReconcileOutcome, StorageError> {
    // Does `s` name THIS node in either identity namespace (transport OR agent)?
    let is_self = |s: &str| s == self_cid || self_agent_cid == Some(s);
    let placement_grace_seconds = cfg.placement_grace_seconds;
    let placement_gap_cooldown_seconds = cfg.placement_gap_cooldown_seconds;
    let inventory_freshness_seconds = cfg.inventory_freshness_seconds;
    let mut outcome = ReconcileOutcome::default();

    // Bounded + rotating scan: a pass costs O(batch), not O(corpus). A short
    // page wraps the cursor back to 0, so every row is still examined across
    // passes without a separate COUNT(*). `scan_limit <= 0` keeps the
    // pre-existing unbounded read.
    let custody_rows = match state.scan_window() {
        Some((limit, offset)) => {
            let rows = rea_commitments::table
                .filter(rea_commitments::action.eq("custody-blob"))
                .order(rea_commitments::id.asc())
                .limit(limit)
                .offset(offset)
                .load::<ReaCommitment>(conn)
                .map_err(|e| {
                    StorageError::Database(format!("load custody-blob commitments: {e}"))
                })?;
            state.advance_cursor(offset, limit, rows.len());
            rows
        }
        None => rea_commitments::table
            .filter(rea_commitments::action.eq("custody-blob"))
            .load::<ReaCommitment>(conn)
            .map_err(|e| StorageError::Database(format!("load custody-blob commitments: {e}")))?,
    };

    let stale_before = (now - chrono::Duration::seconds(inventory_freshness_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let unhonored_before = (now - chrono::Duration::seconds(placement_grace_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let cooldown_after = (now - chrono::Duration::seconds(placement_gap_cooldown_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    for commitment in custody_rows {
        outcome.commitments_examined += 1;
        // `resource_classified_as` is a JSON list by contract (non-commons spec
        // §11.2, Option A); a custody-blob row carries a single blob hash as the
        // list's primary element. Reading the raw column directly was
        // latent-buggy on array-wrapped `["sha256-…"]` rows (U1, 2026-06-19) —
        // route through the accessor so both bare and list forms resolve.
        let Some(raw_marker) = commitment.primary_classification() else {
            continue;
        };

        // Address hygiene BEFORE any fetch decision. Markers arrive in three
        // legitimate forms (bare hex, `sha256-<hex>`, CID) plus one known
        // defect: seed-side normalizeBlobHash double-wrapped CID-form blob
        // hashes as `sha256-<cid>` (backlog
        // blob-fetch-sha256-prefixed-cid-rejection) — those rows made every
        // sweep tick kick a wire request no responder accepts (T21 rejection,
        // ~2min drumbeat, no give-up). normalize_fetch_address repairs the
        // double-wrap and refuses anything unrecognizable, so a malformed row
        // is counted + skipped instead of drummed.
        if crate::p2p::blob_fetch::normalize_fetch_address(&raw_marker).is_err() {
            outcome.invalid_markers += 1;
            tracing::warn!(
                target: "elohim_storage::reconcile",
                commitment_id = %commitment.id,
                marker = %raw_marker,
                "T23: custody-blob marker is not a valid content address in any \
                 form; skipping (no fetch kick — the row needs healing, retries \
                 cannot succeed)"
            );
            continue;
        }
        // Hex-form markers (bare hex or `sha256-<hex>`) keep their exact
        // stored spelling — they were always wire-acceptable and are the form
        // the local snapshot + gossip inventory speak. CID-form and repaired
        // double-wrapped markers are RE-KEYED to the canonical on-disk key
        // (`sha256-<wrapped digest hex>`): that is what the blob store files
        // bytes under and what inventory advertises, so the presence check
        // stops re-kicking once the bytes land instead of drumming forever.
        let is_hex_form = {
            let bare = raw_marker.strip_prefix("sha256-").unwrap_or(&raw_marker);
            bare.len() == 64 && bare.chars().all(|c| c.is_ascii_hexdigit())
        };
        let blob_hash = if is_hex_form {
            raw_marker
        } else {
            match crate::p2p::blob_fetch::content_address_hex(&raw_marker) {
                Some(hex) => format!("sha256-{hex}"),
                // Unreachable given normalize_fetch_address succeeded, but
                // never panic a sweep on it.
                None => raw_marker,
            }
        };

        if is_self(&commitment.provider) {
            // Own commitment — act if missing.
            if !local_store.has(&blob_hash) {
                let candidates =
                    crate::db::peer_blob_inventory::lookup_hosts(conn, &blob_hash, &stale_before)?;
                let named = !candidates.is_empty();
                // Nothing raceable at all — neither a named holder nor a
                // connected peer. Don't spend a ledger attempt on a pass that
                // could not have kicked anything.
                if !named && connected_peers.is_empty() {
                    continue;
                }
                match state.ledger.admit(&blob_hash, named, now) {
                    KickAdmission::Backoff => {
                        outcome.kicks_deferred += 1;
                        continue;
                    }
                    KickAdmission::Exhausted { first } => {
                        // The honest signal the blind fallback kick used to
                        // consume: this node is the provider and no reachable
                        // peer holds the bytes. Emitted ONCE per exhaustion
                        // (and still subject to the placement-gap cooldown),
                        // never per pass.
                        outcome.kicks_exhausted += 1;
                        if first {
                            tracing::warn!(
                                target: "elohim_storage::reconcile",
                                commitment_id = %commitment.id,
                                hash = %blob_hash,
                                "T23: custody obligation UNSATISFIABLE — no reachable peer \
                                 holds these bytes; kicking stops until a named holder \
                                 appears or a new peer connects"
                            );
                            if emit_placement_gap(
                                conn,
                                &commitment,
                                &blob_hash,
                                now,
                                &cooldown_after,
                                Some("custody-fetch-exhausted"),
                            )? {
                                outcome.placement_gaps_emitted += 1;
                            }
                        }
                        continue;
                    }
                    KickAdmission::Untracked => {
                        outcome.kicks_untracked += 1;
                    }
                    KickAdmission::Fire => {}
                }
                if named {
                    let inventory: Vec<String> =
                        candidates.into_iter().map(|c| c.peer_id).collect();

                    // Wave-3: re-order inventory by capability/headroom/RTT/bond/diversity.
                    // `select_serve_peers` → agent_cids → resolved to libp2p via manifest.
                    // Empty manifest (prod default) → prefix empty → inventory unchanged.
                    // Resolution requires peer_transport_manifest (iroh feature).
                    // Without it the prefix is empty → inventory unchanged (safe degradation).
                    #[cfg(feature = "p2p-iroh")]
                    let score_prefix: Vec<String> =
                        crate::services::serve_routing::select_serve_peers(
                            conn,
                            &blob_hash,
                            inventory.len().max(8),
                        )
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|cid| {
                            crate::p2p_iroh::peer_map::lookup_by_agent_cid(conn, &cid)
                                .ok()
                                .flatten()
                                .and_then(|m| m.libp2p.map(|l| l.peer_id))
                        })
                        .collect();
                    #[cfg(not(feature = "p2p-iroh"))]
                    let score_prefix: Vec<String> = Vec::new();

                    let candidate_peers: Vec<String> = if score_prefix.is_empty() {
                        inventory
                    } else {
                        let prefix_set: std::collections::HashSet<&str> =
                            score_prefix.iter().map(String::as_str).collect();
                        let tail: Vec<String> = inventory
                            .into_iter()
                            .filter(|p| !prefix_set.contains(p.as_str()))
                            .collect();
                        let mut merged = score_prefix;
                        merged.extend(tail);
                        merged
                    };

                    fetch_kicker.kick(&blob_hash, candidate_peers);
                    outcome.kicks_fired += 1;
                } else {
                    // Inventory-blind fallback: gossip inventory publish dies
                    // on every pod restart (InsufficientPeers), so an empty
                    // candidate set is NOT evidence the mesh lacks the blob.
                    // Race the connected peers, capped — the same fallback
                    // that makes heal-on-read pass the CI cross-pod fetch.
                    // Now BOUNDED by the kick ledger: after
                    // `KickPolicy::blind_attempt_limit` consecutive blind
                    // attempts the target is declared unsatisfiable above and
                    // this branch is never reached again until a re-arm.
                    let fallback: Vec<String> = connected_peers
                        .iter()
                        .take(FALLBACK_CANDIDATE_CAP)
                        .cloned()
                        .collect();
                    fetch_kicker.kick(&blob_hash, fallback);
                    outcome.kicks_fired += 1;
                    outcome.fallback_kicks += 1;
                }
            }
        } else if is_self(&commitment.receiver) {
            // Other peer's commitment to me — observe; signal on stale.
            let last_seen: Option<String> = peer_blob_inventory::table
                .filter(peer_blob_inventory::peer_id.eq(&commitment.provider))
                .filter(peer_blob_inventory::blob_hash.eq(&blob_hash))
                .select(peer_blob_inventory::last_seen_at)
                .first::<String>(conn)
                .optional()
                .map_err(|e| {
                    StorageError::Database(format!("lookup last_seen for placement-gap: {e}"))
                })?;

            let unhonored = match last_seen {
                None => true, // never observed
                Some(ts) => ts.as_str() < unhonored_before.as_str(),
            };

            if !unhonored {
                continue;
            }

            if emit_placement_gap(conn, &commitment, &blob_hash, now, &cooldown_after, None)? {
                outcome.placement_gaps_emitted += 1;
            }
        }
    }

    Ok(outcome)
}

/// Emit a `placement-gap` REA economic event for `commitment`, honoring the
/// cooldown. Returns `true` when an event was actually inserted.
///
/// Two callers, one semantic: an obligation that is not being met. The receiver
/// arm fires it when a provider this node depends on has gone quiet; the
/// provider arm fires it when THIS node committed to hold bytes it cannot
/// obtain from any reachable peer (`note = "custody-fetch-exhausted"`).
fn emit_placement_gap(
    conn: &mut SqliteConnection,
    commitment: &ReaCommitment,
    blob_hash: &str,
    now: DateTime<Utc>,
    cooldown_after: &str,
    note: Option<&str>,
) -> Result<bool, StorageError> {
    let recent_gap = economic_events::table
        .filter(economic_events::action.eq("placement-gap"))
        .filter(economic_events::output_of.eq(&commitment.id))
        .filter(economic_events::has_point_in_time.gt(cooldown_after))
        .count()
        .get_result::<i64>(conn)
        .map_err(|e| StorageError::Database(format!("count recent placement-gap: {e}")))?;
    if recent_gap > 0 {
        return Ok(false);
    }

    // Bind owned strings so we can borrow them into NewEconomicEvent<'_>.
    let event_id = uuid::Uuid::new_v4().to_string();
    let has_point = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let new_event = NewEconomicEvent {
        id: &event_id,
        h_app_id: &commitment.h_app_id,
        action: "placement-gap",
        provider: &commitment.provider,
        receiver: &commitment.receiver,
        resource_conforms_to: commitment.resource_conforms_to.as_deref(),
        resource_inventoried_as: Some(blob_hash),
        resource_classified_as_json: None,
        resource_quantity_value: None,
        resource_quantity_unit: None,
        effort_quantity_value: None,
        effort_quantity_unit: None,
        has_point_in_time: &has_point,
        has_duration: None,
        input_of: None,
        output_of: Some(commitment.id.as_str()),
        lamad_event_type: None,
        content_id: None,
        contributor_presence_id: None,
        path_id: None,
        triggered_by: None,
        state: "observed",
        note,
        metadata_json: None,
        dht_anchor_hash: None,
        at_location: None,
        verified_at: None,
        scope_collab_cid: None,
        bounded_by: None,
        substrate_signal: None,
    };
    diesel::insert_into(economic_events::table)
        .values(&new_event)
        .execute(conn)
        .map_err(|e| StorageError::Database(format!("insert placement-gap event: {e}")))?;
    Ok(true)
}

/// Back-compat shim for the pre-ledger behavioural tests: a fresh, permissive
/// [`CustodySweepState`] per call, so each call behaves exactly as the amnesiac
/// pass did. `#[cfg(test)]` on purpose — production MUST go through
/// [`reconcile_pass_with_state`] with node-owned state, or the convergence
/// condition is silently bypassed.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub fn reconcile_pass(
    conn: &mut SqliteConnection,
    self_cid: &str,
    self_agent_cid: Option<&str>,
    local_store: &dyn LocalBlobStore,
    fetch_kicker: &dyn FetchKicker,
    cfg: ReconcileConfig,
    now: DateTime<Utc>,
    connected_peers: &[String],
) -> Result<ReconcileOutcome, StorageError> {
    reconcile_pass_with_state(
        conn,
        self_cid,
        self_agent_cid,
        local_store,
        fetch_kicker,
        cfg,
        now,
        connected_peers,
        &CustodySweepState::default(),
    )
}

/// Whether this node offers salvage custody, and the target replication level.
///
/// `enabled` is the opt-in consent gate (default false at the config layer; a
/// node is NEVER conscripted — the imago-dei floor). `target_replicas` is the
/// desired distinct-holder count per blob; `inventory_freshness_seconds` bounds
/// what counts as "currently hosting" when computing honored replicas.
#[derive(Debug, Clone, Copy)]
pub struct SalvageConfig {
    pub enabled: bool,
    pub target_replicas: usize,
    pub inventory_freshness_seconds: u64,
}

/// Authors a salvage `custody-blob` commitment naming `provider` as the new
/// holder of `blob_marker`, owed to content steward `receiver`.
///
/// Production (P3-5) mints a NOTARIZED Mishpat commitment via the conductor so
/// the placement intent stays Class-A and projects back into `rea_commitments`.
/// Tests record the request. Implementors must be idempotent-friendly — the
/// caller already skips when self is a provider, but a duplicate author must not
/// corrupt state.
pub trait CommitmentAuthor: Send + Sync {
    fn author_custody_blob(
        &self,
        blob_marker: &str,
        provider: &str,
        receiver: &str,
    ) -> Result<(), StorageError>;
}

/// Outcome of a single salvage pass; useful for tests + metrics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SalvageOutcome {
    pub blobs_examined: u32,
    pub under_replicated: u32,
    pub commitments_authored: u32,
    pub skipped_not_closest: u32,
    pub skipped_opted_out: u32,
}

/// Run one salvage pass (Phase 3). For each under-replicated `custody-blob`
/// blob, if this node is opted in, not already a provider, and among the
/// deterministic closest-N holders for the blob (per `strategy`), author a new
/// custody-blob commitment naming self as provider. The existing
/// [`reconcile_pass`] provider branch then fetches the bytes — **salvage authors
/// intent; reconcile moves bytes** (no new fetch path).
///
/// Coordination-free: every peer computes the same closest-N over the same pool,
/// so exactly the intended holders self-select (no thundering herd). Additive —
/// does NOT change [`reconcile_pass`]. Idempotent. The XOR vs intentional choice
/// lives behind `strategy` (the [`PlacementStrategy`] seam) — see
/// `2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md`.
/// ## Self-identity (two namespaces)
///
/// `self_read_cid` is the node's TRANSPORT id (`Config::self_cid`) and matches
/// only existing rows this node authored BEFORE the agent_cid resolver landed.
/// `self_write_cid` is the resolved holochain `agent_cid` (`uhCAk…`) — the
/// candidate-pool identity, the placement rank-match, and the `provider` this
/// pass AUTHORS. The "already a provider" idempotency check matches EITHER (so a
/// legacy transport-id row and a new agent_cid row both count), but every WRITE
/// uses `self_write_cid`. `run_salvage_pass` guarantees `self_write_cid` is a
/// valid agent_cid (it skips the tick otherwise).
#[allow(clippy::too_many_arguments)]
pub fn salvage_pass(
    conn: &mut SqliteConnection,
    self_read_cid: &str,
    self_write_cid: &str,
    strategy: &dyn PlacementStrategy,
    author: &dyn CommitmentAuthor,
    salvage_pool: &[PlacementCandidate],
    cfg: SalvageConfig,
    now: DateTime<Utc>,
) -> Result<SalvageOutcome, StorageError> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut outcome = SalvageOutcome::default();

    let custody_rows = rea_commitments::table
        .filter(rea_commitments::action.eq("custody-blob"))
        .load::<ReaCommitment>(conn)
        .map_err(|e| StorageError::Database(format!("load custody-blob commitments: {e}")))?;

    let fresh_before = (now - chrono::Duration::seconds(cfg.inventory_freshness_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    // Group commitments by blob marker → (distinct providers, a content steward).
    // BTree* keeps the pass deterministic (stable blob iteration order).
    struct BlobWant {
        providers: BTreeSet<String>,
        receiver: String,
    }
    let mut wants: BTreeMap<String, BlobWant> = BTreeMap::new();
    for c in &custody_rows {
        let Some(blob) = c.primary_classification() else {
            continue;
        };
        let entry = wants.entry(blob).or_insert_with(|| BlobWant {
            providers: BTreeSet::new(),
            receiver: c.receiver.clone(),
        });
        entry.providers.insert(c.provider.clone());
    }

    for (blob, want) in wants {
        outcome.blobs_examined += 1;

        // honored = distinct committed providers currently hosting (fresh inventory).
        // Joins provider against `peer_blob_inventory.peer_id` exactly as the
        // receiver-role check in `reconcile_pass` does; the pool is resolved to
        // `agent_cid` upstream (P3-6) so the namespaces line up.
        let mut honored = 0usize;
        for provider in &want.providers {
            let fresh: i64 = peer_blob_inventory::table
                .filter(peer_blob_inventory::peer_id.eq(provider))
                .filter(peer_blob_inventory::blob_hash.eq(&blob))
                .filter(peer_blob_inventory::last_seen_at.gt(&fresh_before))
                .count()
                .get_result(conn)
                .map_err(|e| StorageError::Database(format!("count honored providers: {e}")))?;
            if fresh > 0 {
                honored += 1;
            }
        }

        if honored >= cfg.target_replicas {
            continue; // already resilient
        }
        outcome.under_replicated += 1;

        if !cfg.enabled {
            outcome.skipped_opted_out += 1;
            continue; // opt-in consent gate (imago-dei floor; a node is never conscripted)
        }
        // "Already a provider" is checked in BOTH namespaces: a row this node
        // authored last tick carries `self_write_cid` (agent_cid), while a legacy
        // row it authored before the resolver carries `self_read_cid` (transport).
        // Either means "no duplicate adopt".
        if want.providers.contains(self_write_cid) || want.providers.contains(self_read_cid) {
            continue; // already a provider for this blob — idempotent, no duplicate adopt
        }

        // Deterministic self-selection: adopt iff self is among the closest-N
        // holders for this blob. The pool identifies self by its agent_cid, so the
        // match is against `self_write_cid`. Every peer computes the same set over
        // the same pool, so exactly the intended holders adopt (coordination-free).
        let closest = strategy.rank(&blob, salvage_pool, cfg.target_replicas);
        if !closest.iter().any(|cid| cid == self_write_cid) {
            outcome.skipped_not_closest += 1;
            continue;
        }

        // Adopt: author the notarized placement intent naming self by agent_cid
        // (NEVER a transport id); the next reconcile_pass provider branch fetches
        // the bytes.
        author.author_custody_blob(&blob, self_write_cid, &want.receiver)?;
        outcome.commitments_authored += 1;
    }

    Ok(outcome)
}

// ────────────────────────────────────────────────────────────────────────
// Custody manifest + self-held evidence backfill.
//
// `services::shard_manifest_backfill` heals CONTENT rows (walks the `content`
// table for a `blob_hash` with no matching `shard_manifests` row). That leaves
// a gap: blobs stored through the raw blob-plane ingest path (`PUT /blob/{hash}`
// → `http.rs::put_blob_bytes`) that predate the manifest producer (commit
// 123fd4bd5) have NO content row to anchor on at all, so the content-table scan
// can never see them. This pass instead enumerates the BLOB STORE directly
// (`BlobStore::list_hashes`) — the same surface the inventory broadcaster and
// `BlobStoreSnapshot` already use — so it also catches those content-less
// legacy blobs.
//
// For each blob missing a `shard_manifests` row it derives + persists a
// manifest the EXACT way `put_blob_bytes` does (same `ShardEncoder`, same
// `record_generated_manifest` call, same `blob:{cid}` projection key), then —
// new — records self-held evidence for the shard(s) this node just verified it
// holds (`services::self_stewardship::record_self_held_shard`). This flips the
// custody fold's `unknown` class to a real class (`stocked`, when an active
// custody-blob commitment names the holder) WITHOUT a fresh PUT or a
// `distribute_shards` round.
//
// Reconciliation, not conscription: this is bytes-we-already-hold, so it is
// NEVER gated behind `salvage_capacity_enabled` (that flag governs volunteering
// for NEW custody). It has its own config flag (`Config::manifest_backfill_enabled`,
// default ON) wired in `main.rs`.
// ────────────────────────────────────────────────────────────────────────

/// Per-pass cap on blobs actually ENCODED + PERSISTED (the missing-manifest
/// candidate set), not on the raw scan. The candidacy pre-filter
/// (`hash → CID → existing-manifest?`) is a cheap DB-only check with no byte
/// I/O, so it walks every hash the store reports; only the genuine gap is
/// capped. This mirrors `shard_manifest_backfill::REDISTRIBUTE_CAP_PER_BOOT`'s
/// reasoning: a huge already-manifested pantry can never starve later hashes
/// across ticks, because the cap only ever bites the real candidates.
pub const MANIFEST_BACKFILL_BATCH_CAP: usize = 256;

/// Outcome counters for one [`manifest_backfill_pass`] run; logged verbatim by
/// the caller and useful for tests.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ManifestBackfillOutcome {
    /// Total blob hashes the local store reported this pass.
    pub blobs_scanned: usize,
    /// New `shard_manifests` rows persisted.
    pub manifests_written: usize,
    /// New `shard_locations` self-held rows recorded.
    pub self_held_recorded: usize,
    /// Self-held rows RE-claimed for blobs that already had a manifest — the
    /// re-key repair arm (see `manifest_backfill_pass`). Counted separately from
    /// `self_held_recorded` so "the node re-attributed old custody to its live
    /// key" is distinguishable in logs from "the node manifested new bytes".
    pub self_held_reclaimed: usize,
    /// Candidates that turned out to already have a manifest (pre-filter hit,
    /// or a race detected at the write-time re-check).
    pub skipped_existing: usize,
    /// Lookup/read/encode/persist failures, or a local-bytes hash-verification
    /// failure (never a fatal abort — logged and the pass continues).
    pub errors: usize,
}

/// Trivial [`LocalBlobStore`] naming exactly one hash: the bytes this pass just
/// read and hash-verified. Avoids a second full store listing — the candidate
/// bytes are already in hand and proven authentic by the time self-held
/// evidence is considered.
struct VerifiedShardProbe<'a>(&'a str);

impl LocalBlobStore for VerifiedShardProbe<'_> {
    fn has(&self, hash: &str) -> bool {
        hash == self.0
    }
}

/// Run one bounded custody-backfill pass. Idempotent — safe to call on every
/// tick; a blob that gained a manifest on a prior pass is no longer a *manifest*
/// candidate on the next.
///
/// It IS still a self-held candidate. Manifest-existence does not imply the
/// self-held claim is current: this node's `agent_cid` changes across a non-prod
/// DNA reinstall, and a manifest written by the ingest path never recorded a
/// self-held row at all. The pre-filter therefore carries a RE-CLAIM ARM that
/// re-asserts custody under the node's live identity for already-manifested
/// blobs (`self_held_reclaimed`), so the resilience holder join keeps finding
/// this node after a re-key instead of stranding it behind a fossil key.
///
/// `self_agent_cid` is this node's resolved `agent_cid` (`uhCAk…`), obtained by
/// the caller EXACTLY the way `distribute_shards` does
/// (`reconcile::custody::resolve_self_agent_cid`, re-resolved fresh every
/// tick — see `p2p/mod.rs`'s self-stewardship seam). `None` skips self-held
/// recording for the pass (manifests are still backfilled); `record_self_held_shard`
/// itself refuses anything that is not a well-formed `agent_cid`, so a
/// transport id passed here is inert, never written.
pub async fn manifest_backfill_pass(
    conn: &mut SqliteConnection,
    blob_store: &BlobStore,
    self_agent_cid: Option<&str>,
    h_app_id: &str,
    batch_cap: usize,
) -> Result<ManifestBackfillOutcome, StorageError> {
    let mut outcome = ManifestBackfillOutcome::default();

    let hashes = blob_store.list_hashes()?;
    outcome.blobs_scanned = hashes.len();

    // Cheap pre-filter: derive the blob CID from the hash string ALONE (no byte
    // I/O — `hash_to_cid` is a pure function of the sha256 digest already
    // encoded in the filename) and check for an existing manifest under the
    // same `blob:{cid}` key `put_blob_bytes` writes. Only hashes that are
    // genuinely missing a manifest become candidates for the (comparatively
    // expensive) read + encode + persist below.
    let mut candidates: Vec<(String, String)> = Vec::new(); // (hash, content_id)
                                                            // Re-claim work is capped independently of `candidates` so a store full of
                                                            // already-manifested blobs cannot make one pass unbounded.
    let mut reclaims: usize = 0;
    for hash in &hashes {
        if candidates.len() >= batch_cap {
            break;
        }
        let cid = match BlobStore::hash_to_cid(hash) {
            Ok(c) => c.to_string(),
            Err(e) => {
                tracing::warn!(
                    target: "elohim_storage::reconcile",
                    hash = %hash,
                    error = %e,
                    "custody-backfill: hash→CID derivation failed; skipping"
                );
                outcome.errors += 1;
                continue;
            }
        };
        let content_id = format!("blob:{cid}");
        match crate::db::shard_manifests::get_manifest(conn, h_app_id, &content_id) {
            Ok(Some(existing)) => {
                outcome.skipped_existing += 1;
                // RE-CLAIM ARM — the repair for a node that re-keyed.
                //
                // Manifest-existence used to imply "self-held evidence was
                // already recorded on the pass that created it". That holds only
                // while this node's `agent_cid` never changes. It does change: a
                // non-prod DNA reinstall mints a NEW AgentPubKey per pod, and a
                // manifest written by the INGEST path (`put_blob_bytes`) records
                // no self-held row at all. Either way the blob is filtered out
                // here as `skipped_existing` FOREVER, so the node holds bytes it
                // never claims under its live key: the resilience holder join
                // (`shard_locations.peer_id == humans.agent_pub_key`) sees only
                // fossil-keyed rows and the card reads `stewardingCollectives: 0`
                // while the bytes are demonstrably local. Observed live on both
                // alpha doorways 2026-07-31 (holder key matched no `humans` row
                // and no live conductor agent, yet `GET /blob/{hash}` served the
                // full object).
                //
                // So: re-assert the claim under the CURRENT identity every pass.
                // `upsert_location` is keyed `(shard_hash, peer_id)`, so this is
                // idempotent — steady state writes nothing, and the fossil row is
                // left untouched (removing it is `membership_identity_reconcile`'s
                // rekey cascade, not ours; a stale row is inert, never a lie about
                // the live key).
                //
                // Honesty gate, three ways: only `encoding == "none"` (where the
                // single shard IS the whole blob), only when the manifest's shard
                // hash equals the blob hash we are iterating, and only for a hash
                // the local store just listed — the same listing-based custody
                // evidence `BlobStoreSnapshot` supplies to the main reconcile
                // pass. `record_self_held_shard` independently refuses any
                // identity that is not a well-formed `agent_cid`.
                if let Some(agent_cid) =
                    self_agent_cid.filter(|c| crate::identity_namespace::is_agent_cid(c))
                {
                    if reclaims < batch_cap && existing.encoding == "none" {
                        let first_shard: Option<String> =
                            serde_json::from_str::<Vec<String>>(&existing.shard_hashes_json)
                                .ok()
                                .and_then(|v| v.into_iter().next());
                        // Steady state must be a true no-op: once the claim
                        // exists under the live key there is nothing to repair,
                        // so skip the write (and the counter) rather than
                        // re-asserting an identical row on every tick. Without
                        // this the pass would log a non-zero `self_held_reclaimed`
                        // forever and read as perpetual churn.
                        let already_claimed = first_shard.as_deref() == Some(hash.as_str())
                            && crate::db::shard_locations::get_locations_for_shard(conn, hash)
                                .map(|rows| rows.iter().any(|l| l.peer_id == agent_cid))
                                .unwrap_or(false);
                        if first_shard.as_deref() == Some(hash.as_str()) && !already_claimed {
                            reclaims += 1;
                            let probe = VerifiedShardProbe(hash.as_str());
                            match crate::services::self_stewardship::record_self_held_shard(
                                conn, &probe, hash, agent_cid, h_app_id,
                            ) {
                                Ok(
                                    crate::services::self_stewardship::SelfHeldOutcome::Recorded,
                                ) => {
                                    outcome.self_held_reclaimed += 1;
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    tracing::warn!(
                                        target: "elohim_storage::reconcile",
                                        hash = %hash,
                                        error = %e,
                                        "custody-backfill: self-held re-claim failed"
                                    );
                                    outcome.errors += 1;
                                }
                            }
                        }
                    }
                }
            }
            Ok(None) => candidates.push((hash.clone(), content_id)),
            Err(e) => {
                tracing::warn!(
                    target: "elohim_storage::reconcile",
                    hash = %hash,
                    error = %e,
                    "custody-backfill: manifest lookup failed; skipping"
                );
                outcome.errors += 1;
            }
        }
    }

    let encoder = crate::sharding::ShardEncoder::new(crate::sharding::ShardConfig::default());

    for (hash, content_id) in candidates {
        let data = match blob_store.get(&hash).await {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(
                    target: "elohim_storage::reconcile",
                    hash = %hash,
                    error = %e,
                    "custody-backfill: blob bytes unreadable; skipping"
                );
                outcome.errors += 1;
                continue;
            }
        };

        // Never manifest unverified bytes: recompute the hash and require it
        // to match the filename before deriving anything from the content.
        let recomputed = BlobStore::compute_hash(&data);
        if recomputed != hash {
            tracing::warn!(
                target: "elohim_storage::reconcile",
                hash = %hash,
                recomputed = %recomputed,
                "custody-backfill: local bytes FAILED hash verification; skipping \
                 (never manifesting unverified/corrupt bytes)"
            );
            outcome.errors += 1;
            continue;
        }

        // Same derivation `put_blob_bytes` uses — same `ShardEncoder`, same
        // config, same `create_manifest` call — so the encoding decision and
        // shard hashes can never drift between the ingest path and this
        // backfill. Mime type is unknown for a raw enumerate (no request
        // header to read); this is metadata only and does not affect encoding
        // or shard-hash derivation (`ShardEncoder::determine_encoding` is
        // purely size-based).
        let manifest = match encoder.create_manifest(&data, "application/octet-stream", "commons") {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    target: "elohim_storage::reconcile",
                    hash = %hash,
                    error = %e,
                    "custody-backfill: manifest encoding failed; skipping"
                );
                outcome.errors += 1;
                continue;
            }
        };

        // Defensive re-check: a concurrent writer (a fresh PUT, another pass)
        // may have raced a manifest into existence since the pre-filter above.
        match crate::db::shard_manifests::get_manifest(conn, h_app_id, &content_id) {
            Ok(Some(_)) => {
                outcome.skipped_existing += 1;
                continue;
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    target: "elohim_storage::reconcile",
                    hash = %hash,
                    error = %e,
                    "custody-backfill: manifest re-check failed; skipping"
                );
                outcome.errors += 1;
                continue;
            }
        }

        if let Err(e) = crate::db::shard_manifests::record_generated_manifest(
            conn,
            &content_id,
            h_app_id,
            &manifest,
        ) {
            tracing::warn!(
                target: "elohim_storage::reconcile",
                hash = %hash,
                error = %e,
                "custody-backfill: manifest persist failed"
            );
            outcome.errors += 1;
            continue;
        }
        outcome.manifests_written += 1;

        // Self-held evidence — only for `encoding == "none"`, where the single
        // shard IS the whole blob: bytes this node has just read and
        // hash-verified. Larger (`chunked` / `rs-*`) encodings produce derived
        // shard payloads that are NOT present in the local store under their
        // own hash (see `services::self_stewardship` module docs), so no
        // self-held claim is made for them here.
        if manifest.encoding == "none" {
            if let (Some(agent_cid), Some(shard_hash)) =
                (self_agent_cid, manifest.shard_hashes.first())
            {
                let probe = VerifiedShardProbe(shard_hash.as_str());
                match crate::services::self_stewardship::record_self_held_shard(
                    conn, &probe, shard_hash, agent_cid, h_app_id,
                ) {
                    Ok(crate::services::self_stewardship::SelfHeldOutcome::Recorded) => {
                        outcome.self_held_recorded += 1;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!(
                            target: "elohim_storage::reconcile",
                            hash = %hash,
                            error = %e,
                            "custody-backfill: self-held recording failed"
                        );
                        outcome.errors += 1;
                    }
                }
            }
        }
    }

    tracing::info!(
        target: "elohim_storage::reconcile",
        blobs_scanned = outcome.blobs_scanned,
        manifests_written = outcome.manifests_written,
        self_held_recorded = outcome.self_held_recorded,
        self_held_reclaimed = outcome.self_held_reclaimed,
        skipped_existing = outcome.skipped_existing,
        errors = outcome.errors,
        "custody-backfill: pass complete"
    );

    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::diesel_schema::rea_commitments;
    use crate::db::models::NewReaCommitment;
    use crate::db::peer_blob_inventory::apply_snapshot;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};
    use std::sync::Mutex;

    fn test_pool() -> DbPool {
        let url = format!(
            "file:cust_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn default_cfg() -> ReconcileConfig {
        ReconcileConfig {
            placement_grace_seconds: 300,
            placement_gap_cooldown_seconds: 1800,
            inventory_freshness_seconds: 600,
        }
    }

    struct StaticStore(Vec<String>);
    impl LocalBlobStore for StaticStore {
        fn has(&self, hash: &str) -> bool {
            self.0.iter().any(|h| h == hash)
        }
    }

    struct RecordingKicker {
        kicks: Mutex<Vec<(String, Vec<String>)>>,
    }
    impl FetchKicker for RecordingKicker {
        fn kick(&self, blob_hash: &str, candidates: Vec<String>) {
            self.kicks
                .lock()
                .unwrap()
                .push((blob_hash.to_string(), candidates));
        }
    }

    // ── Station A: the pantry snapshot is cached, not rebuilt every pass ──

    /// Write a blob file directly into the two-level pantry tree so
    /// `list_hashes` sees it without going through the async store path.
    fn plant_blob(store: &crate::blob_store::BlobStore, hash: &str) {
        let dir = store.root_dir().join("blobs").join(&hash[..4]);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(hash), b"payload").unwrap();
    }

    #[test]
    fn snapshot_cache_serves_within_ttl_and_rebuilds_after() {
        let store = crate::blob_store::BlobStore::new_memory();
        let cache = SnapshotCache::new(std::time::Duration::from_secs(60));
        let t0 = std::time::Instant::now();

        let first = cache.get(&store, t0).unwrap();
        assert_eq!(first.len(), 0);
        assert_eq!(cache.rebuilds(), 1, "first get builds once");

        // A blob lands on disk AFTER the snapshot was taken.
        plant_blob(&store, &format!("sha256-{}", "a".repeat(64)));

        // Inside the TTL the cached snapshot is reused — no second walk.
        let second = cache
            .get(&store, t0 + std::time::Duration::from_secs(59))
            .unwrap();
        assert_eq!(second.len(), 0, "cached snapshot is served, not rebuilt");
        assert_eq!(cache.rebuilds(), 1, "no rebuild inside the TTL");

        // Past the TTL it rebuilds and sees the new blob.
        let third = cache
            .get(&store, t0 + std::time::Duration::from_secs(61))
            .unwrap();
        assert_eq!(third.len(), 1);
        assert_eq!(cache.rebuilds(), 2, "one rebuild after the TTL expires");
    }

    #[test]
    fn snapshot_cache_note_stored_makes_a_landed_blob_visible_without_a_walk() {
        let store = crate::blob_store::BlobStore::new_memory();
        let cache = SnapshotCache::new(std::time::Duration::from_secs(60));
        let t0 = std::time::Instant::now();
        let hash = format!("sha256-{}", "b".repeat(64));

        assert!(!cache.get(&store, t0).unwrap().has(&hash));
        cache.note_stored(&hash);
        assert!(
            cache.get(&store, t0).unwrap().has(&hash),
            "a blob that just landed must be visible to the very next pass, \
             otherwise the sweep re-kicks a hash it already holds"
        );
        assert_eq!(cache.rebuilds(), 1, "note_stored must not trigger a walk");
    }

    // ── Station B: the sweep has an exhausted state ──
    //
    // Before the kick ledger the sweep had a *satisfied* state (bytes present)
    // and a *malformed* state (InvalidAddress) but no *exhausted* state, so a
    // custody-blob commitment whose bytes no reachable peer holds was re-kicked
    // every pass forever — 502 kicks in 11 minutes for the same 5 commitments
    // on the 2026-09-07 household mesh.

    fn blind_pass(
        conn: &mut SqliteConnection,
        kicker: &RecordingKicker,
        state: &CustodySweepState,
        now: DateTime<Utc>,
    ) -> ReconcileOutcome {
        reconcile_pass_with_state(
            conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            kicker,
            default_cfg(),
            now,
            std::slice::from_ref(&String::from("peer-A")),
            state,
        )
        .unwrap()
    }

    #[test]
    fn the_pass_guard_admits_one_pass_at_a_time() {
        let state = CustodySweepState::default();
        let before = crate::metrics::custody_pass_skipped_overlap_count();

        let first = state.pass_guard().try_enter().expect("first pass enters");
        assert!(
            state.pass_guard().try_enter().is_none(),
            "a second trigger must be refused while a pass is in flight — \
             overlapping passes were observed 0.54 s apart on the household mesh"
        );
        assert_eq!(
            crate::metrics::custody_pass_skipped_overlap_count(),
            before + 1,
            "the refused trigger is counted, not silently dropped"
        );

        drop(first);
        assert!(
            state.pass_guard().try_enter().is_some(),
            "the lane reopens once the in-flight pass finishes"
        );
    }

    #[test]
    fn blind_kicks_exhaust_and_emit_one_placement_gap() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let blob_hash = "c".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "self_cid", "steward_cid", &blob_hash);
        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let state = CustodySweepState::default();

        // Five inventory-blind attempts fire, each once its backoff window opens.
        let mut now = chrono::Utc::now();
        for i in 0..5 {
            let o = blind_pass(&mut conn, &kicker, &state, now);
            assert_eq!(o.kicks_fired, 1, "blind attempt {i} must fire");
            assert_eq!(o.fallback_kicks, 1);
            now += chrono::Duration::seconds(600);
        }

        // The sixth is refused: no reachable peer is known to hold these bytes.
        let o = blind_pass(&mut conn, &kicker, &state, now);
        assert_eq!(
            o.kicks_fired, 0,
            "an unsatisfiable custody target must stop being kicked"
        );
        assert_eq!(o.kicks_exhausted, 1);
        assert_eq!(
            o.placement_gaps_emitted, 1,
            "the honest placement-gap signal replaces the blind fallback kick"
        );

        // It stays refused, and the gap is not re-emitted every pass.
        now += chrono::Duration::seconds(600);
        let o = blind_pass(&mut conn, &kicker, &state, now);
        assert_eq!(o.kicks_fired, 0);
        assert_eq!(o.kicks_exhausted, 1);
        assert_eq!(o.placement_gaps_emitted, 0);
        assert_eq!(
            kicker.kicks.lock().unwrap().len(),
            5,
            "exactly five kicks for this target, ever — not ~8/min forever"
        );
    }

    #[test]
    fn backoff_suppresses_a_second_kick_inside_the_window() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_custody_commitment(&mut conn, "c1", "self_cid", "steward_cid", &"d".repeat(64));
        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let state = CustodySweepState::default();
        let now = chrono::Utc::now();

        assert_eq!(blind_pass(&mut conn, &kicker, &state, now).kicks_fired, 1);
        // Overlapping / back-to-back passes (observed 0.54 s apart) must not
        // each fire their own kick.
        let o = blind_pass(
            &mut conn,
            &kicker,
            &state,
            now + chrono::Duration::seconds(1),
        );
        assert_eq!(o.kicks_fired, 0);
        assert_eq!(o.kicks_deferred, 1);
        assert_eq!(kicker.kicks.lock().unwrap().len(), 1);
    }

    #[test]
    fn a_named_inventory_holder_rearms_an_exhausted_target() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let blob_hash = "e".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "self_cid", "steward_cid", &blob_hash);
        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let state = CustodySweepState::default();

        let mut now = chrono::Utc::now();
        for _ in 0..5 {
            blind_pass(&mut conn, &kicker, &state, now);
            now += chrono::Duration::seconds(600);
        }
        assert_eq!(
            blind_pass(&mut conn, &kicker, &state, now).kicks_exhausted,
            1
        );

        // Gossip now names a holder — evidence the mesh HAS the bytes.
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "peer_X",
            std::slice::from_ref(&blob_hash),
            1,
            &when,
        )
        .unwrap();

        let o = blind_pass(&mut conn, &kicker, &state, now);
        assert_eq!(
            o.kicks_fired, 1,
            "a named holder re-arms an exhausted target"
        );
        assert_eq!(o.fallback_kicks, 0, "named holders are not a blind kick");
    }

    #[test]
    fn a_new_peer_connection_rearms_every_exhausted_target() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_custody_commitment(&mut conn, "c1", "self_cid", "steward_cid", &"f".repeat(64));
        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let state = CustodySweepState::default();

        let mut now = chrono::Utc::now();
        for _ in 0..5 {
            blind_pass(&mut conn, &kicker, &state, now);
            now += chrono::Duration::seconds(600);
        }
        assert_eq!(
            blind_pass(&mut conn, &kicker, &state, now).kicks_exhausted,
            1
        );

        state.ledger.rearm_all();
        assert_eq!(
            blind_pass(&mut conn, &kicker, &state, now).kicks_fired,
            1,
            "a new peer changes the reachable candidate set, so re-arm"
        );
    }

    #[test]
    fn a_hit_clears_the_kick_ledger() {
        let ledger = CustodyKickLedger::default();
        let now = chrono::Utc::now();
        let hash = "sha256-deadbeef";
        for i in 0..5 {
            assert_eq!(
                ledger.admit(hash, false, now + chrono::Duration::seconds(600 * i)),
                KickAdmission::Fire
            );
        }
        assert_eq!(
            ledger.admit(hash, false, now + chrono::Duration::seconds(9999)),
            KickAdmission::Exhausted { first: true }
        );
        ledger.record_hit(hash);
        assert_eq!(ledger.tracked(), 0);
        assert_eq!(
            ledger.admit(hash, false, now + chrono::Duration::seconds(9999)),
            KickAdmission::Fire,
            "the bytes landed once; the target is a fresh subject again"
        );
    }

    #[test]
    fn the_commitment_scan_is_bounded_and_rotates() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        for i in 0..3 {
            insert_custody_commitment(
                &mut conn,
                &format!("c{i}"),
                "other_cid",
                "another_cid",
                &format!("{i}{}", "a".repeat(63)),
            );
        }
        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let state = CustodySweepState::with_scan_limit(2);
        let now = chrono::Utc::now();

        assert_eq!(
            blind_pass(&mut conn, &kicker, &state, now).commitments_examined,
            2
        );
        assert_eq!(
            blind_pass(&mut conn, &kicker, &state, now).commitments_examined,
            1,
            "the window rotates rather than re-reading the same head"
        );
        assert_eq!(
            blind_pass(&mut conn, &kicker, &state, now).commitments_examined,
            2,
            "a short page wraps the cursor back to the start"
        );
    }

    fn insert_custody_commitment(
        conn: &mut SqliteConnection,
        id: &str,
        provider: &str,
        receiver: &str,
        blob_hash: &str,
    ) {
        let row = NewReaCommitment {
            id,
            h_app_id: "test",
            action: "custody-blob",
            provider,
            receiver,
            resource_conforms_to: None,
            resource_classified_as: Some(blob_hash),
            resource_quantity_value: Some(1024.0),
            resource_quantity_unit: Some("bytes"),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            in_scope_of: None,
            medium_of_exchange_id: None,
            state: "active",
            finished: 0,
            note: None,
            metadata_json: None,
            dht_anchor_hash: Some("hash1"),
        };
        diesel::insert_into(rea_commitments::table)
            .values(&row)
            .execute(conn)
            .unwrap();
    }

    #[test]
    fn own_commitment_with_local_blob_no_kick() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &"a".repeat(64));

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec!["a".repeat(64)]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
            &[],
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 0);
        assert!(kicker.kicks.lock().unwrap().is_empty());
    }

    #[test]
    fn own_commitment_with_missing_blob_kicks_when_candidate_exists() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &blob_hash);

        // Use a fresh timestamp so the inventory entry is within freshness window.
        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "peer_X",
            std::slice::from_ref(&blob_hash),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]), // local store empty
            &kicker,
            default_cfg(),
            now,
            &[],
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 1);
        let kicks = kicker.kicks.lock().unwrap();
        assert_eq!(kicks.len(), 1);
        assert_eq!(kicks[0].0, blob_hash);
        assert_eq!(kicks[0].1, vec!["peer_X".to_string()]);
    }

    /// Insert a custody-blob commitment whose `resource_classified_as` is
    /// **array-wrapped** (`["<hash>"]`) — the JSON-list form the seeder/HTTP
    /// path writes (non-commons spec §11.2, Option A). Before the accessor fix
    /// this row read its raw `["<hash>"]` literal as the blob hash and never
    /// matched inventory.
    fn insert_custody_commitment_list(
        conn: &mut SqliteConnection,
        id: &str,
        provider: &str,
        receiver: &str,
        blob_hash: &str,
    ) {
        let classified = serde_json::to_string(&vec![blob_hash]).unwrap();
        let row = NewReaCommitment {
            id,
            h_app_id: "test",
            action: "custody-blob",
            provider,
            receiver,
            resource_conforms_to: None,
            resource_classified_as: Some(&classified),
            resource_quantity_value: Some(1024.0),
            resource_quantity_unit: Some("bytes"),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            in_scope_of: None,
            medium_of_exchange_id: None,
            state: "active",
            finished: 0,
            note: None,
            metadata_json: None,
            dht_anchor_hash: Some("hash1"),
        };
        diesel::insert_into(rea_commitments::table)
            .values(&row)
            .execute(conn)
            .unwrap();
    }

    /// THE INTEGRATION TRAP: a custody-blob row stored array-wrapped
    /// (`["<hash>"]`) must still resolve to the real hash and fire the heal kick.
    /// The reader routes through `primary_classification()`, so the wrap is
    /// transparent. (RED before the accessor fix — the raw `["<hash>"]` string
    /// missed inventory and `kicks_fired == 0`.)
    #[test]
    fn own_commitment_array_wrapped_classified_kicks() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "b".repeat(64);
        insert_custody_commitment_list(&mut conn, "c1", "self_cid", "other_cid", &blob_hash);

        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "peer_X",
            std::slice::from_ref(&blob_hash),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]), // local store empty
            &kicker,
            default_cfg(),
            now,
            &[],
        )
        .unwrap();

        assert_eq!(
            outcome.kicks_fired, 1,
            "array-wrapped custody row must resolve to the real hash and kick"
        );
        let kicks = kicker.kicks.lock().unwrap();
        assert_eq!(kicks.len(), 1);
        assert_eq!(
            kicks[0].0, blob_hash,
            "the kicked hash is the unwrapped real blob hash, not the [..] literal"
        );
    }

    /// Bare-form regression: the pre-migration bare scalar still resolves
    /// (the accessor wraps it to a single-element list).
    #[test]
    fn own_commitment_bare_classified_still_kicks() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "c".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &blob_hash);

        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "peer_X",
            std::slice::from_ref(&blob_hash),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            now,
            &[],
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 1, "bare custody row still resolves");
        assert_eq!(kicker.kicks.lock().unwrap()[0].0, blob_hash);
    }

    /// Backlog `blob-fetch-sha256-prefixed-cid-rejection`: a custody row whose
    /// marker is a CIDv1 double-wrapped in the legacy `sha256-` prefix (the
    /// seed-side normalizeBlobHash defect) must be REPAIRED and re-keyed to
    /// the canonical on-disk form (`sha256-<wrapped digest hex>`) before any
    /// fetch decision — the kick goes out under an address peers accept and
    /// inventory actually advertises, never the double-wrapped literal that
    /// every responder rejects (T21) on a ~2min drumbeat.
    #[test]
    fn double_wrapped_cid_marker_is_repaired_before_kick() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let bytes = b"evolution-of-trust-bundle";
        let (cid, store_key) = crate::blob_store::BlobStore::compute_addresses(bytes);
        let double_wrapped = format!("sha256-{cid}");
        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &double_wrapped);

        // Inventory advertises the canonical on-disk key (what list_hashes /
        // the broadcaster actually publish).
        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "peer_X",
            std::slice::from_ref(&store_key),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            now,
            &[],
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 1, "repaired marker must still heal");
        assert_eq!(outcome.invalid_markers, 0);
        let kicks = kicker.kicks.lock().unwrap();
        assert_eq!(
            kicks[0].0, store_key,
            "kick must use the canonical store key, never the double-wrapped literal"
        );
        assert_eq!(kicks[0].1, vec!["peer_X".to_string()]);
    }

    /// The drum-stop half of the repair: once the bytes land (filed under the
    /// canonical `sha256-<hex>` key), a double-wrapped marker must read as
    /// PRESENT — no further kicks, ever.
    #[test]
    fn double_wrapped_cid_marker_stops_kicking_once_bytes_land() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let bytes = b"evolution-of-trust-bundle";
        let (cid, store_key) = crate::blob_store::BlobStore::compute_addresses(bytes);
        let double_wrapped = format!("sha256-{cid}");
        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &double_wrapped);

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![store_key]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
            &["peer_A".to_string()],
        )
        .unwrap();

        assert_eq!(
            outcome.kicks_fired, 0,
            "bytes present under the canonical key must stop the drum"
        );
        assert!(kicker.kicks.lock().unwrap().is_empty());
    }

    /// Retry hygiene: a marker that is no content address in ANY form is
    /// counted and skipped — never kicked (a kick could only be rejected by
    /// every responder on every sweep tick forever).
    #[test]
    fn invalid_marker_is_counted_and_never_kicked() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        insert_custody_commitment(
            &mut conn,
            "c1",
            "self_cid",
            "other_cid",
            "sha256-not-hex-not-a-cid",
        );

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
            &["peer_A".to_string()], // connected fallback available — must still not kick
        )
        .unwrap();

        assert_eq!(outcome.invalid_markers, 1);
        assert_eq!(outcome.kicks_fired, 0);
        assert!(kicker.kicks.lock().unwrap().is_empty());
    }

    #[test]
    fn own_commitment_with_no_candidates_no_kick() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &"a".repeat(64));

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
            &[],
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 0);
    }

    /// Inventory empty + connected peers present → the provider races the
    /// connected set (inventory-blind fallback). Gossip publish dies on every
    /// pod restart, so sweep convergence must not depend on inventory health.
    #[test]
    fn own_commitment_no_inventory_falls_back_to_connected_peers() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &blob_hash);

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let connected = vec!["peer_A".to_string(), "peer_B".to_string()];
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
            &connected,
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 1);
        assert_eq!(outcome.fallback_kicks, 1);
        let kicks = kicker.kicks.lock().unwrap();
        assert_eq!(kicks.len(), 1);
        assert_eq!(kicks[0].0, blob_hash);
        assert_eq!(kicks[0].1, connected);
    }

    /// The fallback set is capped at FALLBACK_CANDIDATE_CAP connected peers
    /// (mirrors get_blob_or_heal's cap-8 race).
    #[test]
    fn fallback_caps_connected_candidates() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &blob_hash);

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let connected: Vec<String> = (0..12).map(|i| format!("peer_{i}")).collect();
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
            &connected,
        )
        .unwrap();

        assert_eq!(outcome.fallback_kicks, 1);
        let kicks = kicker.kicks.lock().unwrap();
        assert_eq!(kicks[0].1.len(), FALLBACK_CANDIDATE_CAP);
        assert_eq!(kicks[0].1, connected[..FALLBACK_CANDIDATE_CAP].to_vec());
    }

    /// Fresh inventory candidates win over the fallback: the targeted kick
    /// uses the advertised host set, and fallback_kicks stays 0.
    #[test]
    fn inventory_candidates_preferred_over_connected_fallback() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &blob_hash);

        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "peer_X",
            std::slice::from_ref(&blob_hash),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            now,
            &["peer_FALLBACK".to_string()],
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 1);
        assert_eq!(outcome.fallback_kicks, 0);
        let kicks = kicker.kicks.lock().unwrap();
        assert_eq!(kicks[0].1, vec!["peer_X".to_string()]);
    }

    #[test]
    fn others_commitment_unhonored_emits_placement_gap() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "other_cid", "self_cid", &blob_hash);

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
            &[],
        )
        .unwrap();

        assert_eq!(outcome.placement_gaps_emitted, 1);
        assert_eq!(outcome.kicks_fired, 0);

        // Verify the event landed.
        use crate::db::diesel_schema::economic_events;
        let count: i64 = economic_events::table
            .filter(economic_events::action.eq("placement-gap"))
            .filter(economic_events::output_of.eq("c1"))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn others_commitment_with_fresh_inventory_no_gap() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "other_cid", "self_cid", &blob_hash);

        // Fresh inventory entry from other_cid.
        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "other_cid",
            std::slice::from_ref(&blob_hash),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            now,
            &[],
        )
        .unwrap();

        assert_eq!(outcome.placement_gaps_emitted, 0);
    }

    #[test]
    fn placement_gap_cooldown_suppresses_repeat() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "other_cid", "self_cid", &blob_hash);

        // First pass: emit gap.
        let now = chrono::Utc::now();
        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let _ = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            now,
            &[],
        )
        .unwrap();

        // Second pass within cooldown — should suppress.
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            now + chrono::Duration::seconds(60),
            &[],
        )
        .unwrap();

        assert_eq!(outcome.placement_gaps_emitted, 0);
    }

    // -----------------------------------------------------------------------
    // Identity backward-compat: reconcile matches self in EITHER namespace
    // -----------------------------------------------------------------------

    /// A NEW salvage-authored row carries `provider = agent_cid`. With the node's
    /// transport `self_cid` distinct from its `agent_cid`, the own-provider branch
    /// must still fire (kick on miss) by matching the agent_cid — otherwise the
    /// node's own salvage rows are orphaned and their bytes never fetched.
    #[test]
    fn reconcile_matches_own_agent_cid_provider_row() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        // provider = agent_cid (NOT the transport self_cid).
        insert_custody_commitment(&mut conn, "c1", "uhCAk-self", "steward-Z", &blob_hash);

        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "peer_X",
            std::slice::from_ref(&blob_hash),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "12D3KooTransportSelfId", // transport self_cid
            Some("uhCAk-self"),       // resolved agent_cid
            &StaticStore(vec![]),     // blob missing locally
            &kicker,
            default_cfg(),
            now,
            &[],
        )
        .unwrap();

        assert_eq!(
            outcome.kicks_fired, 1,
            "own agent_cid-provider row must match and kick"
        );
    }

    /// A LEGACY row carries `provider = transport self_cid` (authored before the
    /// resolver, or an env-pinned transport SELF_CID). Even with an agent_cid now
    /// resolved, the transport match must still fire — legacy rows are not orphaned.
    #[test]
    fn reconcile_still_matches_legacy_transport_provider_row() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        // provider = transport self_cid (legacy write).
        insert_custody_commitment(
            &mut conn,
            "c1",
            "12D3KooTransportSelfId",
            "steward-Z",
            &blob_hash,
        );

        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "peer_X",
            std::slice::from_ref(&blob_hash),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "12D3KooTransportSelfId",
            Some("uhCAk-self"), // agent_cid resolved, but the row is legacy-transport
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            now,
            &[],
        )
        .unwrap();

        assert_eq!(
            outcome.kicks_fired, 1,
            "legacy transport-id provider row must still match (not orphaned)"
        );
    }

    /// The receiver-side (placement-gap) branch also matches the agent_cid: a row
    /// whose `receiver = agent_cid` and provider is a stale peer emits a gap.
    #[test]
    fn reconcile_matches_own_agent_cid_receiver_row() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "other_cid", "uhCAk-self", &blob_hash);

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "12D3KooTransportSelfId",
            Some("uhCAk-self"),
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
            &[],
        )
        .unwrap();

        assert_eq!(
            outcome.placement_gaps_emitted, 1,
            "own agent_cid-receiver row must emit a placement-gap"
        );
    }

    /// Seed an active local session with the given agent_cid.
    fn seed_active_session(conn: &mut SqliteConnection, agent_cid: &str) {
        crate::db::local_sessions::create_session(
            conn,
            crate::db::local_sessions::CreateLocalSessionInput {
                id: None,
                human_id: "human-1".into(),
                agent_pub_key: agent_cid.into(),
                doorway_url: "http://doorway.test".into(),
                doorway_id: None,
                identifier: "tester".into(),
                display_name: None,
                profile_image_hash: None,
                bootstrap_url: None,
            },
        )
        .expect("seed session");
    }

    /// The lockstep unfreeze: with NO cell-key snapshot at boot (`None`), a session
    /// that logs in AFTER boot is picked up per pass — so reconcile resolves the
    /// same agent_cid salvage authors, and the node's own rows are never orphaned.
    #[test]
    fn resolve_self_agent_cid_picks_up_post_boot_session() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        // Boot: no conductor cell key, no session → unresolved (transport-only).
        assert_eq!(resolve_self_agent_cid(&mut conn, None), None);

        // A session logs in after boot.
        seed_active_session(&mut conn, "uhCAk-session-self");
        assert_eq!(
            resolve_self_agent_cid(&mut conn, None).as_deref(),
            Some("uhCAk-session-self"),
            "post-boot session agent_cid is resolved without a restart"
        );
    }

    /// The cell-key snapshot is the stable fallback when no session is active, and
    /// a session (candidate a) wins over the cell key (candidate b) when present.
    #[test]
    fn resolve_self_agent_cid_prefers_session_then_falls_back_to_cellkey() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        // No session → the boot cell-key snapshot is used.
        assert_eq!(
            resolve_self_agent_cid(&mut conn, Some("uhCAk-cellkey")).as_deref(),
            Some("uhCAk-cellkey")
        );

        // Session present → it wins over the cell key (lockstep with salvage's
        // [session → cell key] order).
        seed_active_session(&mut conn, "uhCAk-session-self");
        assert_eq!(
            resolve_self_agent_cid(&mut conn, Some("uhCAk-cellkey")).as_deref(),
            Some("uhCAk-session-self")
        );
    }

    // -----------------------------------------------------------------------
    // T23: BlobStoreSnapshot adapter
    // -----------------------------------------------------------------------

    /// Snapshot taken after a blob is stored reports `has(hash) == true`,
    /// and reports `has` of an absent hash as `false`.
    #[tokio::test]
    async fn blob_store_snapshot_reflects_local_pantry() {
        let store = crate::blob_store::BlobStore::new_memory();
        let payload = b"reconcile-snapshot-fixture-v1";
        let result = store.store(payload).await.expect("store fixture blob");

        let snapshot = BlobStoreSnapshot::from_store(&store).expect("snapshot");

        assert!(
            snapshot.has(&result.hash),
            "snapshot must report stored hash present"
        );
        assert!(
            !snapshot
                .has("sha256-deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"),
            "snapshot must report unknown hash absent"
        );
        assert_eq!(snapshot.len(), 1, "snapshot captured one hash");
        assert!(!snapshot.is_empty(), "snapshot is non-empty after store");
    }

    // -----------------------------------------------------------------------
    // P3-2: salvage_pass (salvage self-selection behind the placement seam)
    // -----------------------------------------------------------------------

    /// Strategy stub returning a fixed closest-set, so salvage tests control
    /// whether self is "closest" independently of the XOR internals (those are
    /// covered by `reconcile::placement` tests).
    struct StaticStrategy(Vec<String>);
    impl PlacementStrategy for StaticStrategy {
        fn rank(&self, _blob: &str, _candidates: &[PlacementCandidate], _n: usize) -> Vec<String> {
            self.0.clone()
        }
    }

    struct RecordingAuthor {
        authored: Mutex<Vec<(String, String, String)>>,
    }
    impl RecordingAuthor {
        fn new() -> Self {
            Self {
                authored: Mutex::new(Vec::new()),
            }
        }
    }
    impl CommitmentAuthor for RecordingAuthor {
        fn author_custody_blob(
            &self,
            blob_marker: &str,
            provider: &str,
            receiver: &str,
        ) -> Result<(), StorageError> {
            self.authored.lock().unwrap().push((
                blob_marker.to_string(),
                provider.to_string(),
                receiver.to_string(),
            ));
            Ok(())
        }
    }

    fn salvage_cfg(enabled: bool, target: usize) -> SalvageConfig {
        SalvageConfig {
            enabled,
            target_replicas: target,
            inventory_freshness_seconds: 600,
        }
    }

    fn pool(cids: &[&str]) -> Vec<PlacementCandidate> {
        cids.iter()
            .map(|c| PlacementCandidate::from_agent_cid(*c))
            .collect()
    }

    /// Mark a provider as currently hosting a blob (fresh inventory row).
    fn mark_hosting(conn: &mut SqliteConnection, peer: &str, blob: &str, now: DateTime<Utc>) {
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let b = blob.to_string();
        apply_snapshot(conn, peer, std::slice::from_ref(&b), 1, &when).unwrap();
    }

    #[test]
    fn salvage_under_replicated_self_closest_authors() {
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = "b".repeat(64);
        // One existing custodian, target 2 → under-replicated.
        insert_custody_commitment(&mut conn, "c1", "prov-A", "steward-Z", &blob);
        mark_hosting(&mut conn, "prov-A", &blob, now);

        let author = RecordingAuthor::new();
        let strat = StaticStrategy(vec!["prov-A".into(), "self".into()]); // self is closest
        let outcome = salvage_pass(
            &mut conn,
            "self",
            "self",
            &strat,
            &author,
            &pool(&["self", "prov-A", "prov-B"]),
            salvage_cfg(true, 2),
            now,
        )
        .unwrap();

        assert_eq!(outcome.commitments_authored, 1);
        let recorded = author.authored.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(
            recorded[0],
            (blob.clone(), "self".to_string(), "steward-Z".to_string()),
            "salvager authors provider=self, receiver=content steward"
        );
    }

    #[test]
    fn salvage_already_replicated_no_author() {
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = "c".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "prov-A", "steward-Z", &blob);
        insert_custody_commitment(&mut conn, "c2", "prov-B", "steward-Z", &blob);
        mark_hosting(&mut conn, "prov-A", &blob, now);
        mark_hosting(&mut conn, "prov-B", &blob, now);

        let author = RecordingAuthor::new();
        let strat = StaticStrategy(vec!["self".into()]);
        let outcome = salvage_pass(
            &mut conn,
            "self",
            "self",
            &strat,
            &author,
            &pool(&["self"]),
            salvage_cfg(true, 2),
            now,
        )
        .unwrap();

        assert_eq!(outcome.commitments_authored, 0);
        assert_eq!(
            outcome.under_replicated, 0,
            "honored==target → not under-replicated"
        );
        assert!(author.authored.lock().unwrap().is_empty());
    }

    #[test]
    fn salvage_opted_out_no_author() {
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = "d".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "prov-A", "steward-Z", &blob);
        mark_hosting(&mut conn, "prov-A", &blob, now);

        let author = RecordingAuthor::new();
        let strat = StaticStrategy(vec!["self".into()]); // would be closest, but opted out
        let outcome = salvage_pass(
            &mut conn,
            "self",
            "self",
            &strat,
            &author,
            &pool(&["self"]),
            salvage_cfg(false, 2),
            now,
        )
        .unwrap();

        assert_eq!(outcome.commitments_authored, 0);
        assert_eq!(outcome.under_replicated, 1);
        assert_eq!(outcome.skipped_opted_out, 1);
        assert!(author.authored.lock().unwrap().is_empty());
    }

    #[test]
    fn salvage_not_closest_no_author() {
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = "e".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "prov-A", "steward-Z", &blob);
        mark_hosting(&mut conn, "prov-A", &blob, now);

        let author = RecordingAuthor::new();
        let strat = StaticStrategy(vec!["prov-A".into(), "prov-B".into()]); // self NOT closest
        let outcome = salvage_pass(
            &mut conn,
            "self",
            "self",
            &strat,
            &author,
            &pool(&["self", "prov-A", "prov-B"]),
            salvage_cfg(true, 2),
            now,
        )
        .unwrap();

        assert_eq!(outcome.commitments_authored, 0);
        assert_eq!(outcome.skipped_not_closest, 1);
        assert!(author.authored.lock().unwrap().is_empty());
    }

    #[test]
    fn salvage_already_provider_no_author() {
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = "f".repeat(64);
        // self is already a provider (not yet hosting → honored 0 → under-replicated),
        // so it must NOT author a duplicate adopt.
        insert_custody_commitment(&mut conn, "c1", "self", "steward-Z", &blob);

        let author = RecordingAuthor::new();
        let strat = StaticStrategy(vec!["self".into()]);
        let outcome = salvage_pass(
            &mut conn,
            "self",
            "self",
            &strat,
            &author,
            &pool(&["self"]),
            salvage_cfg(true, 2),
            now,
        )
        .unwrap();

        assert_eq!(
            outcome.commitments_authored, 0,
            "already a provider — no duplicate adopt"
        );
        assert!(author.authored.lock().unwrap().is_empty());
    }

    /// The write/read split: when self's transport id (`self_read`) differs from
    /// its agent_cid (`self_write`), the authored `provider` is the AGENT_CID, and
    /// the candidate-pool / rank identity is the agent_cid too. The transport id is
    /// NEVER written.
    #[test]
    fn salvage_authors_agent_cid_not_transport_when_they_differ() {
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "prov-A", "steward-Z", &blob);
        mark_hosting(&mut conn, "prov-A", &blob, now);

        let author = RecordingAuthor::new();
        // Strategy names the agent_cid as closest (the pool identity).
        let strat = StaticStrategy(vec!["prov-A".into(), "uhCAk-self".into()]);
        let outcome = salvage_pass(
            &mut conn,
            "12D3KooTransportSelf", // self_read (transport)
            "uhCAk-self",           // self_write (agent_cid)
            &strat,
            &author,
            &pool(&["uhCAk-self", "prov-A"]),
            salvage_cfg(true, 2),
            now,
        )
        .unwrap();

        assert_eq!(outcome.commitments_authored, 1);
        let recorded = author.authored.lock().unwrap();
        assert_eq!(
            recorded[0],
            (blob, "uhCAk-self".to_string(), "steward-Z".to_string()),
            "provider is the agent_cid, never the transport id"
        );
    }

    /// Backward-compat idempotency: a LEGACY row whose `provider` is the transport
    /// `self_read` id must still count as "already a provider" (no duplicate adopt)
    /// even though the node now writes agent_cid.
    #[test]
    fn salvage_already_provider_matches_legacy_transport_row() {
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = "a".repeat(64);
        // Legacy row: provider is the transport id this node used to write.
        insert_custody_commitment(&mut conn, "c1", "12D3KooTransportSelf", "steward-Z", &blob);

        let author = RecordingAuthor::new();
        let strat = StaticStrategy(vec!["uhCAk-self".into()]);
        let outcome = salvage_pass(
            &mut conn,
            "12D3KooTransportSelf", // self_read matches the legacy provider
            "uhCAk-self",           // self_write (agent_cid)
            &strat,
            &author,
            &pool(&["uhCAk-self"]),
            salvage_cfg(true, 2),
            now,
        )
        .unwrap();

        assert_eq!(
            outcome.commitments_authored, 0,
            "legacy transport-id provider row counts as already-provider (no duplicate)"
        );
        assert!(author.authored.lock().unwrap().is_empty());
    }

    /// REGRESSION (alpha 2026-07-31, both doorways): a node that re-keys must
    /// keep claiming bytes it still holds.
    ///
    /// Before the re-claim arm, the second pass filtered the already-manifested
    /// blob out as `skipped_existing` and recorded nothing, so `shard_locations`
    /// held ONLY the fossil-keyed row. The resilience holder join
    /// (`shard_locations.peer_id == humans.agent_pub_key`) then matched no live
    /// human and the card read `stewardingCollectives: 0` while
    /// `GET /blob/{hash}` served the object in full — exactly the live symptom.
    #[tokio::test]
    async fn reclaims_self_held_custody_under_a_new_agent_key() {
        const OLD_KEY: &str = "uhCAkFossilAgentKeyFromABeforeTimesReinstall";
        const NEW_KEY: &str = "uhCAkLiveAgentKeyMintedByTheLatestReinstall";

        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let store = BlobStore::new_memory();

        // Small payload → `encoding == "none"`, so the single shard IS the blob.
        let stored = store.store(b"the landing page bytes").await.expect("store");
        let hash = stored.hash.clone();

        // Pass 1 — under the OLD identity: manifests the blob and claims it.
        let first = manifest_backfill_pass(&mut conn, &store, Some(OLD_KEY), "lamad", 16)
            .await
            .expect("first pass");
        assert_eq!(first.manifests_written, 1, "blob is manifested once");
        assert_eq!(first.self_held_recorded, 1, "and claimed under the old key");
        assert_eq!(first.self_held_reclaimed, 0, "nothing to re-claim yet");

        // Pass 2 — the pod has been re-keyed. The manifest already exists, so the
        // blob is `skipped_existing`; the re-claim arm must still fire.
        let second = manifest_backfill_pass(&mut conn, &store, Some(NEW_KEY), "lamad", 16)
            .await
            .expect("second pass");
        assert_eq!(second.manifests_written, 0, "manifest is not rewritten");
        assert_eq!(second.skipped_existing, 1, "the blob is a pre-filter hit");
        assert_eq!(
            second.self_held_reclaimed, 1,
            "custody must be re-asserted under the live key — this is the fix"
        );

        let holders: std::collections::HashSet<String> =
            crate::db::shard_locations::get_locations_for_shard(&mut conn, &hash)
                .expect("load locations")
                .into_iter()
                .map(|l| l.peer_id)
                .collect();
        assert!(
            holders.contains(NEW_KEY),
            "the live key must be a recorded holder, else the resilience join \
             stays empty; got {holders:?}"
        );
        assert!(
            holders.contains(OLD_KEY),
            "the fossil row is left inert, not deleted (rekey cascade is \
             membership_identity_reconcile's job); got {holders:?}"
        );

        // Pass 3 — steady state under the same key writes nothing new (upsert is
        // keyed `(shard_hash, peer_id)`), so the arm cannot grow the table.
        let third = manifest_backfill_pass(&mut conn, &store, Some(NEW_KEY), "lamad", 16)
            .await
            .expect("third pass");
        assert_eq!(
            third.self_held_reclaimed, 0,
            "an already-current claim is not re-recorded"
        );
        assert_eq!(
            crate::db::shard_locations::get_locations_for_shard(&mut conn, &hash)
                .expect("load locations")
                .len(),
            2,
            "exactly the fossil row + the live row"
        );
    }

    /// The re-claim arm inherits `record_self_held_shard`'s namespace refusal: a
    /// transport id must never reach the `agent_cid` join-key column.
    #[tokio::test]
    async fn reclaim_refuses_a_transport_identity() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let store = BlobStore::new_memory();
        let stored = store
            .store(b"bytes held by a node with no agent key")
            .await
            .expect("store");

        manifest_backfill_pass(&mut conn, &store, Some("uhCAkRealAgentKey"), "lamad", 16)
            .await
            .expect("first pass");

        let second = manifest_backfill_pass(
            &mut conn,
            &store,
            Some("12D3KooWTransportIdNotAnAgentCid"),
            "lamad",
            16,
        )
        .await
        .expect("second pass");
        assert_eq!(
            second.self_held_reclaimed, 0,
            "a libp2p id can never join humans.agent_pub_key; it must not be written"
        );

        let holders: Vec<String> =
            crate::db::shard_locations::get_locations_for_shard(&mut conn, &stored.hash)
                .expect("load locations")
                .into_iter()
                .map(|l| l.peer_id)
                .collect();
        assert_eq!(holders, vec!["uhCAkRealAgentKey".to_string()]);
    }
}
