//! Cache Refresh — keep `/api/v1/cache` honest after warm-up.
//!
//! `warm_stream` fills the projection store (the Mongo copy behind
//! `/api/v1/cache/{type}/{id}`) ONCE at boot from storage's
//! `GET /api/v1/cache/stream`. Before this module nothing refreshed it: a
//! steward's republish, a seeded update or a new atom reached learners only
//! after a doorway restart (F22 — the household served a six-chapter path while
//! storage held the five-movement one, and new atoms answered 404 from the
//! cache).
//!
//! ## What moves the cache now
//!
//! | Storage event (`/api/v1/events`)        | Cache action                                   |
//! |-----------------------------------------|------------------------------------------------|
//! | `content.created` / `content.updated`   | re-read `GET /db/content/{id}` → upsert or evict; re-read the atom's outgoing edges |
//! | `content.bulk-created` (`ids`)          | the same, per id                               |
//! | `content.deleted`                       | evict `Content:{id}` and every cached edge touching it (no read) |
//! | `relationship.created`                  | re-read `GET /db/relationships/{id}` → upsert or evict |
//! | `relationship.deleted`                  | evict `Relationship:{id}` (no read)            |
//! | `relationship.bulk-created` (count only), `: lagged` SSE comment, SSE reconnect | request a reconciliation sweep |
//!
//! Storage content events carry only `{id}` (plus title/type on create), never
//! the row, so every upsert is a targeted read of the ONE row — against this
//! doorway's primary storage only (single-target: no fan-out, see
//! `doorway/CLAUDE.md`).
//!
//! ## Reach (never newly expose a row)
//!
//! The reads are anonymous — no agent identity — so storage's read reach gate
//! (`api::content_reach_gate`) answers exactly as it would for an anonymous
//! visitor, and its serving floor (`MinTrust::Amber`, provenance) applies. On
//! top of that the doorway re-checks the row itself: only `reach` in
//! [`CACHEABLE_REACHES`] — the same set storage's `list_cacheable_*` feeds the
//! warm stream — is ever written. A row that fails either check is EVICTED, not
//! merely skipped, so a reach narrowing (commons → intimate) withdraws the cached
//! copy. `404`/`403`/`410` read as "not servable" (evict); transport errors and
//! `5xx` leave the cached copy untouched (a storage blip never blanks the cache).
//!
//! ## Coalescing / backpressure
//!
//! Events land in a pending map keyed by `(type, id)`; the LAST event for a key
//! wins (a delete after an update evicts; an update after a delete re-reads).
//! The worker waits [`REFRESH_DEBOUNCE_MS`] after the first wake-up so a burst
//! collapses, then drains the whole map, one row at a time. A burst of N updates
//! for one id therefore costs one read (two if the burst straddles a drain). The
//! map is capped at [`MAX_PENDING`]; overflow drops the new key and requests a
//! sweep, which covers every dropped key. All writes go through
//! `ProjectionStore::set`, whose equivalence guard makes a repeat a no-op.
//!
//! ## Freshness bound (the stream CAN drop events)
//!
//! The event stream is lossy: storage's broadcast drops events for a lagging
//! subscriber (it says so with a `: lagged` comment), events emitted while the
//! SSE connection is down are gone, and some storage writes emit no event at
//! all (the CRDT heal in `p2p/mod.rs` writes `blob_hash` directly). So a
//! **reconciliation sweep** runs every `DOORWAY_CACHE_RECONCILE_SECS`
//! (default [`RECONCILE_DEFAULT_SECS`] = 10 min; `0` disables the timer only),
//! and on demand after a reconnect, a `: lagged` notice, a bulk edge write or a
//! pending-map overflow — never more often than every
//! [`RECONCILE_MIN_SPACING_SECS`]. A sweep re-streams the primary's cacheable
//! set (upserting changes and new rows, idempotent) and then verifies, one read
//! each, every cached row this path owns that the stream did not mention —
//! absence from the stream alone is never proof (the stream pages by offset over
//! a mutating table and ends with `cache.done` even after a query error). So the
//! documented bound is: an event-visible change reaches the cache within about a
//! second; any change reaches it within one sweep period.
//!
//! Sweep verifications ride a second, lower-priority lane: every drain serves
//! all pending EVENT keys first, then at most [`CANDIDATES_PER_DRAIN`]
//! verifications, and yields back if more remain — so a large sweep backlog
//! (each read of a gone row can make storage try a P2P resolve) never holds an
//! event refresh behind it for more than one small batch.
//!
//! Drains and sweeps hold one lock, so a sweep's streamed snapshot can never
//! land on top of a fresher per-id read.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bson::doc;
use serde::Serialize;
use serde_json::Value as JsonValue;
use tokio::sync::Notify;
use tracing::{debug, info, warn};

use super::document::{ProjectedDocument, ProjectionQuery};
use super::store::ProjectionStore;
use super::warm_stream::stream_from_peer;

/// Reach values the doorway may hold in its public projection cache — the same
/// set storage's `db::cache_queries::list_cacheable_{content,relationships}`
/// streams to the warm path, and storage's `api::content_reach_gate::OPEN_REACHES`
/// (what an anonymous reader may see). Anything else is never written and is
/// evicted.
pub const CACHEABLE_REACHES: &[&str] = &["commons", "public"];

/// Provenance label (`action_hash` and `author`) of every document the storage
/// cache path writes. The warm stream uses it, and so does this module, so a
/// sweep replay of an unchanged row is equivalent (`is_equivalent_projection`)
/// and costs no Mongo write. Documents the DHT-signal engine writes carry real
/// hashes and are never sweep candidates.
pub const CACHE_STREAM_PROVENANCE: &str = "cache-stream";

/// Provenance of the deprecated HTTP-pull warm path (`warm.rs`); treated as
/// owned by the storage cache path for sweep verification.
const LEGACY_WARM_PROVENANCE: &str = "cache-warm";

/// Coalescing window: how long the worker waits after the first event of a
/// burst before draining.
pub const REFRESH_DEBOUNCE_MS: u64 = 250;

/// Cap on distinct pending keys. Overflow requests a sweep instead of growing.
pub const MAX_PENDING: usize = 10_000;

/// Cap on per-row verifications queued at once (across sweeps).
pub const SWEEP_MAX_CANDIDATES: usize = 2_000;

/// Sweep verifications served per drain, after every pending event key.
pub const CANDIDATES_PER_DRAIN: usize = 16;

/// Default reconciliation period (seconds).
pub const RECONCILE_DEFAULT_SECS: u64 = 600;

/// Minimum spacing between two sweeps, whatever requested them.
pub const RECONCILE_MIN_SPACING_SECS: u64 = 60;

/// Whole-request timeout for one targeted storage read.
const STORAGE_READ_TIMEOUT_SECS: u64 = 10;

/// Page size for an atom's outgoing edges — storage's `MAX_LIST_LIMIT`. A full
/// page proves nothing about absence, so eviction of dropped edges is skipped
/// when a page comes back full.
const EDGE_PAGE_LIMIT: usize = 500;

const CONTENT: &str = "Content";
const RELATIONSHIP: &str = "Relationship";

/// Parse `DOORWAY_CACHE_RECONCILE_SECS`. `None` input → default; `0` → the
/// periodic timer is off (event- and reconnect-triggered sweeps still run);
/// unparseable → default.
pub fn parse_reconcile_period(raw: Option<String>) -> Option<Duration> {
    let secs = raw
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(RECONCILE_DEFAULT_SECS);
    (secs > 0).then(|| Duration::from_secs(secs))
}

/// True when a projected row's `reach` is one the public cache may hold.
/// A row that states no reach is NOT cacheable (fail closed).
pub fn is_cacheable_reach(data: &JsonValue) -> bool {
    data.get("reach")
        .and_then(JsonValue::as_str)
        .is_some_and(|r| CACHEABLE_REACHES.contains(&r))
}

/// Which cached projection a pending refresh addresses.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CacheKey {
    Content(String),
    Relationship(String),
}

/// What the latest event asked for a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshKind {
    /// Re-read the row from storage; upsert when servable, evict otherwise.
    Refresh,
    /// The row is gone upstream; evict without reading.
    Remove,
}

/// Outcome of one targeted storage read.
enum Fetched {
    /// 200 with a JSON body.
    Found(JsonValue),
    /// 404 / 403 / 410: storage will not serve this row to an anonymous reader.
    NotServable,
    /// Transport error, timeout, 5xx, unexpected status or undecodable body —
    /// says nothing about the row; the cache is left as it is.
    Unknown(String),
}

/// Observable counters (snapshot via [`CacheRefresher::stats`]).
#[derive(Debug, Default)]
struct Counters {
    events: AtomicU64,
    coalesced: AtomicU64,
    reads: AtomicU64,
    upserted: AtomicU64,
    evicted: AtomicU64,
    read_errors: AtomicU64,
    overflows: AtomicU64,
    sweep_requests: AtomicU64,
    sweeps: AtomicU64,
}

/// Snapshot of the refresher's counters.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CacheRefreshStats {
    /// Keys enqueued (every accepted event or sweep candidate).
    pub events: u64,
    /// Enqueues that collapsed into a key already pending.
    pub coalesced: u64,
    /// Targeted storage reads issued.
    pub reads: u64,
    /// Documents written (or confirmed current) from a read.
    pub upserted: u64,
    /// Documents evicted.
    pub evicted: u64,
    /// Reads that said nothing about the row (cache left untouched).
    pub read_errors: u64,
    /// Enqueues refused at the pending cap (each requested a sweep).
    pub overflows: u64,
    /// Sweeps requested (reconnect, `: lagged`, bulk edge write, overflow).
    pub sweep_requests: u64,
    /// Reconciliation sweeps run.
    pub sweeps: u64,
}

/// Result of one reconciliation sweep.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SweepReport {
    /// The stream reached `cache.done`.
    pub completed: bool,
    /// Rows the stream carried (content + relationships + humans).
    pub streamed: usize,
    /// Cached rows the stream did not mention, queued for a per-row check.
    pub candidates_queued: usize,
}

/// Keeps the doorway's projection cache in step with its primary storage.
pub struct CacheRefresher {
    store: Arc<ProjectionStore>,
    storage_url: String,
    http: reqwest::Client,
    pending: Mutex<HashMap<CacheKey, RefreshKind>>,
    /// Sweep verifications (lower priority than `pending`), deduplicated.
    candidates: Mutex<CandidateQueue>,
    wake: Notify,
    sweep_wanted: Notify,
    overflowed: AtomicBool,
    /// Serializes drains and sweeps (see module doc).
    work_lock: tokio::sync::Mutex<()>,
    counters: Counters,
    debounce: Duration,
    min_sweep_spacing: Duration,
}

impl CacheRefresher {
    /// A refresher reading from `storage_url` (this doorway's primary storage).
    pub fn new(store: Arc<ProjectionStore>, storage_url: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(STORAGE_READ_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();
        Self {
            store,
            storage_url: storage_url.into().trim_end_matches('/').to_string(),
            http,
            pending: Mutex::new(HashMap::new()),
            candidates: Mutex::new(CandidateQueue::default()),
            wake: Notify::new(),
            sweep_wanted: Notify::new(),
            overflowed: AtomicBool::new(false),
            work_lock: tokio::sync::Mutex::new(()),
            counters: Counters::default(),
            debounce: Duration::from_millis(REFRESH_DEBOUNCE_MS),
            min_sweep_spacing: Duration::from_secs(RECONCILE_MIN_SPACING_SECS),
        }
    }

    /// Override the coalescing window and sweep spacing (tests).
    pub fn with_timing(mut self, debounce: Duration, min_sweep_spacing: Duration) -> Self {
        self.debounce = debounce;
        self.min_sweep_spacing = min_sweep_spacing;
        self
    }

    /// Counter snapshot.
    pub fn stats(&self) -> CacheRefreshStats {
        let c = &self.counters;
        CacheRefreshStats {
            events: c.events.load(Ordering::Relaxed),
            coalesced: c.coalesced.load(Ordering::Relaxed),
            reads: c.reads.load(Ordering::Relaxed),
            upserted: c.upserted.load(Ordering::Relaxed),
            evicted: c.evicted.load(Ordering::Relaxed),
            read_errors: c.read_errors.load(Ordering::Relaxed),
            overflows: c.overflows.load(Ordering::Relaxed),
            sweep_requests: c.sweep_requests.load(Ordering::Relaxed),
            sweeps: c.sweeps.load(Ordering::Relaxed),
        }
    }

    /// Number of keys waiting (events plus sweep verifications).
    pub fn pending_len(&self) -> usize {
        let events = self.pending.lock().map(|p| p.len()).unwrap_or(0);
        let candidates = self.candidates.lock().map(|c| c.order.len()).unwrap_or(0);
        events + candidates
    }

    // ------------------------------------------------------------------
    // Event intake
    // ------------------------------------------------------------------

    /// Queue a key; the latest kind wins. Returns false when the pending map
    /// is full (a sweep is requested instead).
    pub fn enqueue(&self, key: CacheKey, kind: RefreshKind) -> bool {
        let accepted = {
            let Ok(mut pending) = self.pending.lock() else {
                return false;
            };
            if let Some(existing) = pending.get_mut(&key) {
                *existing = kind;
                self.counters.coalesced.fetch_add(1, Ordering::Relaxed);
                true
            } else if pending.len() >= MAX_PENDING {
                false
            } else {
                pending.insert(key, kind);
                true
            }
        };
        if accepted {
            self.counters.events.fetch_add(1, Ordering::Relaxed);
            self.wake.notify_one();
        } else {
            self.counters.overflows.fetch_add(1, Ordering::Relaxed);
            if !self.overflowed.swap(true, Ordering::Relaxed) {
                warn!(
                    cap = MAX_PENDING,
                    "cache_refresh: pending map full; dropping new keys and requesting a sweep"
                );
            }
            self.request_sweep();
        }
        accepted
    }

    /// Translate one storage SSE event into cache work. Returns true when the
    /// event kind is one this refresher consumes.
    pub fn on_storage_event(&self, event_type: &str, data: &str) -> bool {
        let parsed: Option<JsonValue> = serde_json::from_str(data).ok();
        let id = || {
            parsed
                .as_ref()
                .and_then(|v| v.get("id"))
                .and_then(JsonValue::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        match event_type {
            "content.created" | "content.updated" => {
                if let Some(id) = id() {
                    self.enqueue(CacheKey::Content(id), RefreshKind::Refresh);
                }
                true
            }
            "content.deleted" => {
                if let Some(id) = id() {
                    self.enqueue(CacheKey::Content(id), RefreshKind::Remove);
                }
                true
            }
            "content.bulk-created" => {
                let ids = parsed
                    .as_ref()
                    .and_then(|v| v.get("ids"))
                    .and_then(JsonValue::as_array);
                match ids {
                    Some(ids) => {
                        for id in ids.iter().filter_map(JsonValue::as_str) {
                            if !id.is_empty() {
                                self.enqueue(
                                    CacheKey::Content(id.to_string()),
                                    RefreshKind::Refresh,
                                );
                            }
                        }
                    }
                    // A bulk create that names no ids can only be caught by a sweep.
                    None => self.request_sweep(),
                }
                true
            }
            "relationship.created" => {
                if let Some(id) = id() {
                    self.enqueue(CacheKey::Relationship(id), RefreshKind::Refresh);
                }
                true
            }
            "relationship.deleted" => {
                if let Some(id) = id() {
                    self.enqueue(CacheKey::Relationship(id), RefreshKind::Remove);
                }
                true
            }
            // Carries only a count: nothing to target.
            "relationship.bulk-created" => {
                self.request_sweep();
                true
            }
            _ => false,
        }
    }

    /// Ask for a reconciliation sweep (coalesced; rate-limited by the worker).
    pub fn request_sweep(&self) {
        self.counters.sweep_requests.fetch_add(1, Ordering::Relaxed);
        self.sweep_wanted.notify_one();
    }

    // ------------------------------------------------------------------
    // Workers
    // ------------------------------------------------------------------

    /// Spawn the drain worker and the sweep worker. `reconcile_period = None`
    /// disables only the timer; requested sweeps still run.
    pub fn spawn(self: &Arc<Self>, reconcile_period: Option<Duration>) {
        let drainer = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                drainer.wake.notified().await;
                // Let the burst land before reading anything.
                tokio::time::sleep(drainer.debounce).await;
                drainer.drain_once().await;
            }
        });

        let sweeper = Arc::clone(self);
        tokio::spawn(async move {
            let mut ticker = reconcile_period.map(|p| {
                let mut t = tokio::time::interval_at(tokio::time::Instant::now() + p, p);
                t.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                t
            });
            let mut last_sweep: Option<Instant> = None;
            loop {
                let cause = tokio::select! {
                    _ = next_tick(&mut ticker) => "timer",
                    _ = sweeper.sweep_wanted.notified() => "requested",
                };
                if let Some(last) = last_sweep {
                    let since = last.elapsed();
                    if since < sweeper.min_sweep_spacing {
                        tokio::time::sleep(sweeper.min_sweep_spacing - since).await;
                    }
                }
                let report = sweeper.sweep_once().await;
                last_sweep = Some(Instant::now());
                info!(
                    cause,
                    completed = report.completed,
                    streamed = report.streamed,
                    candidates = report.candidates_queued,
                    "cache_refresh: reconciliation sweep done"
                );
            }
        });
    }

    /// Drain every pending key once. Public so tests (and callers that need a
    /// synchronous flush) can drive it without the timer.
    pub async fn drain_once(&self) {
        let _work = self.work_lock.lock().await;
        let batch: Vec<(CacheKey, RefreshKind)> = match self.pending.lock() {
            Ok(mut pending) => pending.drain().collect(),
            Err(_) => return,
        };
        self.overflowed.store(false, Ordering::Relaxed);
        let handled: HashSet<CacheKey> = batch.iter().map(|(k, _)| k.clone()).collect();
        for (key, kind) in batch {
            self.apply(key, kind).await;
        }

        // Then a bounded slice of sweep verifications. A key an event already
        // settled in this drain (or one now pending again) needs no second read.
        let slice: Vec<CacheKey> = match self.candidates.lock() {
            Ok(mut c) => c.pop_up_to(CANDIDATES_PER_DRAIN),
            Err(_) => Vec::new(),
        };
        for key in slice {
            let pending_again = self
                .pending
                .lock()
                .map(|p| p.contains_key(&key))
                .unwrap_or(false);
            if handled.contains(&key) || pending_again {
                continue;
            }
            self.apply(key, RefreshKind::Refresh).await;
        }
        let more = self
            .candidates
            .lock()
            .map(|c| !c.order.is_empty())
            .unwrap_or(false);
        if more {
            // Come back after the debounce; events that land meanwhile go first.
            self.wake.notify_one();
        }
    }

    async fn apply(&self, key: CacheKey, kind: RefreshKind) {
        match (key, kind) {
            (CacheKey::Content(id), RefreshKind::Refresh) => self.refresh_content(&id).await,
            (CacheKey::Content(id), RefreshKind::Remove) => self.remove_content(&id).await,
            (CacheKey::Relationship(id), RefreshKind::Refresh) => {
                self.refresh_relationship(&id).await
            }
            (CacheKey::Relationship(id), RefreshKind::Remove) => {
                self.evict(RELATIONSHIP, &id).await
            }
        }
    }

    /// One reconciliation pass (see module doc). Public for tests.
    pub async fn sweep_once(&self) -> SweepReport {
        self.counters.sweeps.fetch_add(1, Ordering::Relaxed);
        let (result, candidates) = {
            let _work = self.work_lock.lock().await;
            let result = stream_from_peer(Arc::clone(&self.store), &self.storage_url).await;
            if !result.completed {
                warn!(
                    storage_url = %self.storage_url,
                    errors = ?result.errors,
                    "cache_refresh: sweep stream did not complete; skipping absence checks"
                );
                (result, Vec::new())
            } else {
                let mut candidates = Vec::new();
                for (doc_type, seen) in [
                    (CONTENT, &result.content_ids),
                    (RELATIONSHIP, &result.relationship_ids),
                ] {
                    for id in self.owned_cached_ids(doc_type).await {
                        if !seen.contains(&id) {
                            candidates.push(if doc_type == CONTENT {
                                CacheKey::Content(id)
                            } else {
                                CacheKey::Relationship(id)
                            });
                        }
                    }
                }
                (result, candidates)
            }
        };

        let queued = match self.candidates.lock() {
            Ok(mut c) => c.push_all(candidates, SWEEP_MAX_CANDIDATES),
            Err(_) => 0,
        };
        if queued > 0 {
            self.counters
                .events
                .fetch_add(queued as u64, Ordering::Relaxed);
            self.wake.notify_one();
        }
        SweepReport {
            completed: result.completed,
            streamed: result.content_count + result.relationship_count + result.human_count,
            candidates_queued: queued,
        }
    }

    // ------------------------------------------------------------------
    // Per-row work
    // ------------------------------------------------------------------

    async fn refresh_content(&self, id: &str) {
        let url = format!(
            "{}/db/content/{}",
            self.storage_url,
            urlencoding::encode(id)
        );
        match self.fetch(&url).await {
            Fetched::Found(view) => {
                if view.get("id").and_then(JsonValue::as_str) != Some(id) {
                    self.counters.read_errors.fetch_add(1, Ordering::Relaxed);
                    warn!(
                        id,
                        "cache_refresh: storage answered a different row; cache left as is"
                    );
                    return;
                }
                if is_cacheable_reach(&view) {
                    self.upsert(CONTENT, id, view).await;
                    self.refresh_outgoing_edges(id).await;
                } else {
                    debug!(
                        id,
                        "cache_refresh: row not cacheable at its reach; evicting"
                    );
                    self.remove_content(id).await;
                }
            }
            Fetched::NotServable => self.remove_content(id).await,
            Fetched::Unknown(why) => {
                self.counters.read_errors.fetch_add(1, Ordering::Relaxed);
                warn!(id, error = %why, "cache_refresh: content read failed; cache left as is");
            }
        }
    }

    /// Replace the cached outgoing edges of `id` with what storage serves.
    async fn refresh_outgoing_edges(&self, id: &str) {
        let url = format!(
            "{}/db/relationships?contentId={}&direction=outgoing&limit={EDGE_PAGE_LIMIT}",
            self.storage_url,
            urlencoding::encode(id)
        );
        match self.fetch(&url).await {
            Fetched::Found(body) => {
                let items = body
                    .get("items")
                    .and_then(JsonValue::as_array)
                    .cloned()
                    .unwrap_or_default();
                let full_page = items.len() >= EDGE_PAGE_LIMIT;
                let mut kept: HashSet<String> = HashSet::new();
                for edge in items {
                    let Some(edge_id) = edge.get("id").and_then(JsonValue::as_str) else {
                        continue;
                    };
                    if edge.get("sourceId").and_then(JsonValue::as_str) != Some(id) {
                        continue;
                    }
                    if !is_cacheable_reach(&edge) {
                        continue;
                    }
                    let edge_id = edge_id.to_string();
                    self.upsert(RELATIONSHIP, &edge_id, edge).await;
                    kept.insert(edge_id);
                }
                if full_page {
                    debug!(
                        id,
                        "cache_refresh: full edge page; not evicting unlisted edges"
                    );
                    return;
                }
                for doc in self.cached_edges_touching(id, false).await {
                    if !kept.contains(&doc.doc_id) {
                        self.evict(RELATIONSHIP, &doc.doc_id).await;
                    }
                }
            }
            Fetched::NotServable => {
                for doc in self.cached_edges_touching(id, false).await {
                    self.evict(RELATIONSHIP, &doc.doc_id).await;
                }
            }
            Fetched::Unknown(why) => {
                self.counters.read_errors.fetch_add(1, Ordering::Relaxed);
                warn!(id, error = %why, "cache_refresh: edge read failed; cached edges left as is");
            }
        }
    }

    async fn refresh_relationship(&self, id: &str) {
        let url = format!(
            "{}/db/relationships/{}",
            self.storage_url,
            urlencoding::encode(id)
        );
        match self.fetch(&url).await {
            Fetched::Found(edge) => {
                let same_id = edge.get("id").and_then(JsonValue::as_str) == Some(id);
                if same_id && is_cacheable_reach(&edge) {
                    self.upsert(RELATIONSHIP, id, edge).await;
                } else if same_id {
                    self.evict(RELATIONSHIP, id).await;
                } else {
                    self.counters.read_errors.fetch_add(1, Ordering::Relaxed);
                }
            }
            Fetched::NotServable => self.evict(RELATIONSHIP, id).await,
            Fetched::Unknown(why) => {
                self.counters.read_errors.fetch_add(1, Ordering::Relaxed);
                warn!(id, error = %why, "cache_refresh: edge read failed; cache left as is");
            }
        }
    }

    /// Evict an atom and every cached edge that touches it: an edge is served
    /// only when both endpoints are readable, so an unservable atom takes its
    /// incoming edges down too.
    async fn remove_content(&self, id: &str) {
        self.evict(CONTENT, id).await;
        for doc in self.cached_edges_touching(id, true).await {
            self.evict(RELATIONSHIP, &doc.doc_id).await;
        }
    }

    async fn upsert(&self, doc_type: &str, id: &str, data: JsonValue) {
        let doc = ProjectedDocument::new(
            doc_type,
            id,
            CACHE_STREAM_PROVENANCE,
            CACHE_STREAM_PROVENANCE,
            data,
        );
        match self.store.set(doc).await {
            Ok(()) => {
                self.counters.upserted.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                warn!(doc_type, id, error = %e, "cache_refresh: projection write failed");
            }
        }
    }

    async fn evict(&self, doc_type: &str, id: &str) {
        match self.store.invalidate(&format!("{doc_type}:{id}")).await {
            Ok(n) => {
                if n > 0 {
                    self.counters.evicted.fetch_add(1, Ordering::Relaxed);
                    debug!(doc_type, id, "cache_refresh: evicted");
                }
            }
            Err(e) => warn!(doc_type, id, error = %e, "cache_refresh: eviction failed"),
        }
    }

    /// Cached `Relationship` documents whose source (and, when asked, target)
    /// is `id`. The Mongo filter narrows the scan; the in-Rust filter is the
    /// authority (memory-only stores ignore custom filters).
    async fn cached_edges_touching(
        &self,
        id: &str,
        include_target: bool,
    ) -> Vec<ProjectedDocument> {
        let filter = if include_target {
            doc! { "$or": [ { "data.sourceId": id }, { "data.targetId": id } ] }
        } else {
            doc! { "data.sourceId": id }
        };
        let query = ProjectionQuery {
            doc_type: Some(RELATIONSHIP.to_string()),
            filter: Some(filter),
            ..Default::default()
        };
        let docs = match self.store.query(query).await {
            Ok(docs) => docs,
            Err(e) => {
                warn!(id, error = %e, "cache_refresh: cached-edge lookup failed");
                return Vec::new();
            }
        };
        docs.into_iter()
            .filter(|d| {
                let field = |k: &str| d.data.get(k).and_then(JsonValue::as_str) == Some(id);
                field("sourceId") || (include_target && field("targetId"))
            })
            .collect()
    }

    /// Ids of cached documents of `doc_type` written by the storage cache path.
    async fn owned_cached_ids(&self, doc_type: &str) -> Vec<String> {
        let query = ProjectionQuery {
            doc_type: Some(doc_type.to_string()),
            filter: Some(doc! {
                "action_hash": { "$in": [CACHE_STREAM_PROVENANCE, LEGACY_WARM_PROVENANCE] }
            }),
            ..Default::default()
        };
        match self.store.query(query).await {
            Ok(docs) => docs
                .into_iter()
                .filter(|d| {
                    d.action_hash == CACHE_STREAM_PROVENANCE
                        || d.action_hash == LEGACY_WARM_PROVENANCE
                })
                .map(|d| d.doc_id)
                .collect(),
            Err(e) => {
                warn!(doc_type, error = %e, "cache_refresh: cached-id listing failed");
                Vec::new()
            }
        }
    }

    async fn fetch(&self, url: &str) -> Fetched {
        self.counters.reads.fetch_add(1, Ordering::Relaxed);
        let resp = match self.http.get(url).send().await {
            Ok(r) => r,
            Err(e) => return Fetched::Unknown(format!("request: {e}")),
        };
        let status = resp.status();
        if status.is_success() {
            return match resp.json::<JsonValue>().await {
                Ok(v) => Fetched::Found(v),
                Err(e) => Fetched::Unknown(format!("decode: {e}")),
            };
        }
        match status.as_u16() {
            403 | 404 | 410 => Fetched::NotServable,
            other => Fetched::Unknown(format!("HTTP {other}")),
        }
    }
}

/// FIFO of sweep verifications with membership dedup.
#[derive(Debug, Default)]
struct CandidateQueue {
    order: VecDeque<CacheKey>,
    members: HashSet<CacheKey>,
}

impl CandidateQueue {
    /// Append new keys up to `cap` queued in total; returns how many were added.
    fn push_all(&mut self, keys: Vec<CacheKey>, cap: usize) -> usize {
        let mut added = 0;
        for key in keys {
            if self.order.len() >= cap {
                break;
            }
            if self.members.insert(key.clone()) {
                self.order.push_back(key);
                added += 1;
            }
        }
        added
    }

    fn pop_up_to(&mut self, n: usize) -> Vec<CacheKey> {
        let take = n.min(self.order.len());
        let out: Vec<CacheKey> = self.order.drain(..take).collect();
        for key in &out {
            self.members.remove(key);
        }
        out
    }
}

async fn next_tick(ticker: &mut Option<tokio::time::Interval>) {
    match ticker {
        Some(t) => {
            t.tick().await;
        }
        None => std::future::pending::<()>().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::ProjectionConfig;
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn store() -> Arc<ProjectionStore> {
        Arc::new(ProjectionStore::memory_only(ProjectionConfig::default()))
    }

    fn content(id: &str, title: &str, reach: &str) -> JsonValue {
        json!({ "id": id, "title": title, "reach": reach, "tags": [] })
    }

    fn edge(id: &str, source: &str, target: &str, reach: &str) -> JsonValue {
        json!({
            "id": id, "sourceId": source, "targetId": target,
            "relationshipType": "RELATES_TO", "reach": reach
        })
    }

    async fn seed(store: &ProjectionStore, doc_type: &str, id: &str, data: JsonValue) {
        store
            .set(ProjectedDocument::new(
                doc_type,
                id,
                CACHE_STREAM_PROVENANCE,
                CACHE_STREAM_PROVENANCE,
                data,
            ))
            .await
            .unwrap();
    }

    async fn title_of(store: &ProjectionStore, id: &str) -> Option<String> {
        store.get(CONTENT, id).await.and_then(|d| {
            d.data
                .get("title")
                .and_then(|t| t.as_str())
                .map(String::from)
        })
    }

    /// Mount an empty outgoing-edge page for `id` (most content tests).
    async fn no_edges(server: &MockServer, id: &str) {
        Mock::given(method("GET"))
            .and(path("/db/relationships"))
            .and(query_param("contentId", id))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "items": [] })))
            .mount(server)
            .await;
    }

    #[test]
    fn reconcile_period_parses() {
        assert_eq!(
            parse_reconcile_period(None),
            Some(Duration::from_secs(RECONCILE_DEFAULT_SECS))
        );
        assert_eq!(
            parse_reconcile_period(Some("120".into())),
            Some(Duration::from_secs(120))
        );
        assert_eq!(parse_reconcile_period(Some("0".into())), None);
        assert_eq!(
            parse_reconcile_period(Some("soon".into())),
            Some(Duration::from_secs(RECONCILE_DEFAULT_SECS))
        );
    }

    #[test]
    fn cacheable_reach_is_the_warm_stream_set_and_fails_closed() {
        assert!(is_cacheable_reach(&json!({ "reach": "commons" })));
        assert!(is_cacheable_reach(&json!({ "reach": "public" })));
        assert!(!is_cacheable_reach(&json!({ "reach": "intimate" })));
        assert!(!is_cacheable_reach(&json!({ "reach": "self" })));
        assert!(!is_cacheable_reach(&json!({ "title": "no reach stated" })));
    }

    /// F22 itself: the cached copy is the old version; an update event makes
    /// the cache serve storage's current row.
    #[tokio::test]
    async fn an_updated_row_replaces_the_cached_copy() {
        let server = MockServer::start().await;
        let store = store();
        seed(
            &store,
            CONTENT,
            "fct",
            content("fct", "Six chapters", "commons"),
        )
        .await;
        Mock::given(method("GET"))
            .and(path("/db/content/fct"))
            .respond_with(ResponseTemplate::new(200).set_body_json(content(
                "fct",
                "Five movements",
                "commons",
            )))
            .expect(1)
            .mount(&server)
            .await;
        no_edges(&server, "fct").await;

        let r = CacheRefresher::new(Arc::clone(&store), server.uri());
        assert!(r.on_storage_event("content.updated", r#"{"id":"fct"}"#));
        r.drain_once().await;

        assert_eq!(
            title_of(&store, "fct").await.as_deref(),
            Some("Five movements")
        );
        let doc = store.get(CONTENT, "fct").await.unwrap();
        assert_eq!(doc.action_hash, CACHE_STREAM_PROVENANCE);
    }

    #[tokio::test]
    async fn a_created_row_appears() {
        let server = MockServer::start().await;
        let store = store();
        let id = "fct-module-01-church-dilemma-story";
        Mock::given(method("GET"))
            .and(path(format!("/db/content/{id}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(content(
                id,
                "The Church Dilemma",
                "commons",
            )))
            .mount(&server)
            .await;
        no_edges(&server, id).await;

        let r = CacheRefresher::new(Arc::clone(&store), server.uri());
        assert!(
            store.get(CONTENT, id).await.is_none(),
            "precondition: a cache miss"
        );
        r.on_storage_event(
            "content.created",
            &json!({ "id": id, "title": "The Church Dilemma", "contentType": "story" }).to_string(),
        );
        r.drain_once().await;

        assert_eq!(
            title_of(&store, id).await.as_deref(),
            Some("The Church Dilemma")
        );
    }

    #[tokio::test]
    async fn bulk_created_ids_each_appear() {
        let server = MockServer::start().await;
        let store = store();
        for id in ["a", "b"] {
            Mock::given(method("GET"))
                .and(path(format!("/db/content/{id}")))
                .respond_with(ResponseTemplate::new(200).set_body_json(content(id, id, "public")))
                .mount(&server)
                .await;
            no_edges(&server, id).await;
        }
        let r = CacheRefresher::new(Arc::clone(&store), server.uri());
        r.on_storage_event("content.bulk-created", r#"{"count":2,"ids":["a","b"]}"#);
        r.drain_once().await;
        assert!(store.get(CONTENT, "a").await.is_some());
        assert!(store.get(CONTENT, "b").await.is_some());
    }

    /// A delete evicts the row and its edges without reading storage.
    #[tokio::test]
    async fn a_deleted_row_disappears() {
        let server = MockServer::start().await;
        let store = store();
        seed(&store, CONTENT, "gone", content("gone", "Gone", "commons")).await;
        seed(
            &store,
            RELATIONSHIP,
            "rel-out",
            edge("rel-out", "gone", "x", "commons"),
        )
        .await;
        seed(
            &store,
            RELATIONSHIP,
            "rel-in",
            edge("rel-in", "y", "gone", "commons"),
        )
        .await;
        seed(
            &store,
            RELATIONSHIP,
            "rel-other",
            edge("rel-other", "y", "x", "commons"),
        )
        .await;

        let r = CacheRefresher::new(Arc::clone(&store), server.uri());
        r.on_storage_event("content.deleted", r#"{"id":"gone"}"#);
        r.drain_once().await;

        assert!(store.get(CONTENT, "gone").await.is_none());
        assert!(store.get(RELATIONSHIP, "rel-out").await.is_none());
        assert!(store.get(RELATIONSHIP, "rel-in").await.is_none());
        assert!(store.get(RELATIONSHIP, "rel-other").await.is_some());
        assert!(
            server.received_requests().await.unwrap().is_empty(),
            "a delete is applied without a storage read"
        );
    }

    /// Defense in depth: even if storage answered an intimate row to the
    /// doorway, the cache never writes it — and a previously cached commons copy
    /// is withdrawn (a reach narrowing must not leave the old copy served).
    #[tokio::test]
    async fn an_intimate_row_is_not_newly_exposed_and_a_narrowed_row_is_withdrawn() {
        let server = MockServer::start().await;
        let store = store();
        seed(
            &store,
            CONTENT,
            "narrowed",
            content("narrowed", "Was commons", "commons"),
        )
        .await;
        seed(
            &store,
            RELATIONSHIP,
            "rel-n",
            edge("rel-n", "narrowed", "x", "commons"),
        )
        .await;
        for (id, title) in [("love-map", "Our love map"), ("narrowed", "Now intimate")] {
            Mock::given(method("GET"))
                .and(path(format!("/db/content/{id}")))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(content(id, title, "intimate")),
                )
                .mount(&server)
                .await;
        }

        let r = CacheRefresher::new(Arc::clone(&store), server.uri());
        r.on_storage_event("content.created", r#"{"id":"love-map"}"#);
        r.on_storage_event("content.updated", r#"{"id":"narrowed"}"#);
        r.drain_once().await;

        assert!(
            store.get(CONTENT, "love-map").await.is_none(),
            "never newly exposed"
        );
        assert!(
            store.get(CONTENT, "narrowed").await.is_none(),
            "narrowed copy withdrawn"
        );
        assert!(
            store.get(RELATIONSHIP, "rel-n").await.is_none(),
            "its edges go with it"
        );
    }

    /// The anonymous read's reach gate refuses (403) or storage no longer has
    /// the row (404): the cached copy goes.
    #[tokio::test]
    async fn a_refused_or_missing_row_is_evicted() {
        let server = MockServer::start().await;
        let store = store();
        seed(
            &store,
            CONTENT,
            "refused",
            content("refused", "x", "commons"),
        )
        .await;
        seed(
            &store,
            CONTENT,
            "missing",
            content("missing", "x", "commons"),
        )
        .await;
        Mock::given(method("GET"))
            .and(path("/db/content/refused"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/db/content/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let r = CacheRefresher::new(Arc::clone(&store), server.uri());
        r.on_storage_event("content.updated", r#"{"id":"refused"}"#);
        r.on_storage_event("content.updated", r#"{"id":"missing"}"#);
        r.drain_once().await;

        assert!(store.get(CONTENT, "refused").await.is_none());
        assert!(store.get(CONTENT, "missing").await.is_none());
    }

    /// A storage blip says nothing about the row: the cache keeps serving.
    #[tokio::test]
    async fn a_storage_error_leaves_the_cached_copy() {
        let server = MockServer::start().await;
        let store = store();
        seed(
            &store,
            CONTENT,
            "steady",
            content("steady", "Still here", "commons"),
        )
        .await;
        Mock::given(method("GET"))
            .and(path("/db/content/steady"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let r = CacheRefresher::new(Arc::clone(&store), server.uri());
        r.on_storage_event("content.updated", r#"{"id":"steady"}"#);
        r.drain_once().await;

        assert_eq!(
            title_of(&store, "steady").await.as_deref(),
            Some("Still here")
        );
        assert_eq!(r.stats().read_errors, 1);
    }

    /// A burst of updates for one id refreshes it once.
    #[tokio::test]
    async fn a_burst_of_updates_for_one_id_reads_once() {
        let server = MockServer::start().await;
        let store = store();
        Mock::given(method("GET"))
            .and(path("/db/content/hot"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(content("hot", "v50", "commons")),
            )
            .expect(1)
            .mount(&server)
            .await;
        no_edges(&server, "hot").await;

        let r = CacheRefresher::new(Arc::clone(&store), server.uri());
        for _ in 0..50 {
            r.on_storage_event("content.updated", r#"{"id":"hot"}"#);
        }
        assert_eq!(r.pending_len(), 1, "fifty events, one pending key");
        r.drain_once().await;

        let stats = r.stats();
        assert_eq!(stats.coalesced, 49);
        assert_eq!(title_of(&store, "hot").await.as_deref(), Some("v50"));
        // `expect(1)` on the content read is verified when `server` drops.
    }

    /// The same bound through the spawned worker: a burst spread over time
    /// costs at most two reads (one drain in flight, one after).
    #[tokio::test]
    async fn the_worker_coalesces_a_burst_to_a_bounded_number_of_reads() {
        let server = MockServer::start().await;
        let store = store();
        Mock::given(method("GET"))
            .and(path("/db/content/hot"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(content("hot", "hot", "commons")),
            )
            .expect(1..=2)
            .mount(&server)
            .await;
        no_edges(&server, "hot").await;

        let r = Arc::new(
            CacheRefresher::new(Arc::clone(&store), server.uri())
                .with_timing(Duration::from_millis(50), Duration::from_secs(3600)),
        );
        r.spawn(None);
        for _ in 0..20 {
            r.on_storage_event("content.updated", r#"{"id":"hot"}"#);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        for _ in 0..100 {
            if store.get(CONTENT, "hot").await.is_some() && r.pending_len() == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(store.get(CONTENT, "hot").await.is_some());
    }

    /// Last event wins: update then delete evicts; delete then update re-reads.
    #[tokio::test]
    async fn the_latest_event_for_a_key_wins() {
        let r = CacheRefresher::new(store(), "http://unused");
        r.on_storage_event("content.updated", r#"{"id":"k"}"#);
        r.on_storage_event("content.deleted", r#"{"id":"k"}"#);
        assert_eq!(
            r.pending
                .lock()
                .unwrap()
                .get(&CacheKey::Content("k".into())),
            Some(&RefreshKind::Remove)
        );
        r.on_storage_event("content.updated", r#"{"id":"k"}"#);
        assert_eq!(
            r.pending
                .lock()
                .unwrap()
                .get(&CacheKey::Content("k".into())),
            Some(&RefreshKind::Refresh)
        );
    }

    #[test]
    fn unhandled_and_malformed_events_queue_nothing() {
        let r = CacheRefresher::new(store(), "http://unused");
        assert!(!r.on_storage_event("projection.registered", r#"{"commitmentId":"c"}"#));
        assert!(r.on_storage_event("content.updated", "not json"));
        assert!(r.on_storage_event("content.updated", r#"{"id":""}"#));
        assert_eq!(r.pending_len(), 0);
    }

    /// Edges change when an atom's signed head changes: the atom's refresh
    /// replaces its cached outgoing edges — a new edge appears, a dropped edge
    /// and a narrowed edge go, an incoming edge from another atom stays.
    #[tokio::test]
    async fn a_content_refresh_replaces_its_outgoing_edges() {
        let server = MockServer::start().await;
        let store = store();
        seed(
            &store,
            RELATIONSHIP,
            "rel-dropped",
            edge("rel-dropped", "lesson", "old", "commons"),
        )
        .await;
        seed(
            &store,
            RELATIONSHIP,
            "rel-narrow",
            edge("rel-narrow", "lesson", "n", "commons"),
        )
        .await;
        seed(
            &store,
            RELATIONSHIP,
            "rel-in",
            edge("rel-in", "path", "lesson", "commons"),
        )
        .await;
        Mock::given(method("GET"))
            .and(path("/db/content/lesson"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(content("lesson", "L", "commons")),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/db/relationships"))
            .and(query_param("contentId", "lesson"))
            .and(query_param("direction", "outgoing"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [
                    edge("rel-new", "lesson", "scripture", "commons"),
                    edge("rel-narrow", "lesson", "n", "intimate"),
                ]
            })))
            .mount(&server)
            .await;

        let r = CacheRefresher::new(Arc::clone(&store), server.uri());
        r.on_storage_event("content.updated", r#"{"id":"lesson"}"#);
        r.drain_once().await;

        assert!(
            store.get(RELATIONSHIP, "rel-new").await.is_some(),
            "new edge appears"
        );
        assert!(
            store.get(RELATIONSHIP, "rel-dropped").await.is_none(),
            "dropped edge goes"
        );
        assert!(
            store.get(RELATIONSHIP, "rel-narrow").await.is_none(),
            "narrowed edge goes"
        );
        assert!(
            store.get(RELATIONSHIP, "rel-in").await.is_some(),
            "incoming edge untouched"
        );
    }

    #[tokio::test]
    async fn relationship_events_refresh_and_evict_single_edges() {
        let server = MockServer::start().await;
        let store = store();
        seed(
            &store,
            RELATIONSHIP,
            "rel-del",
            edge("rel-del", "a", "b", "commons"),
        )
        .await;
        Mock::given(method("GET"))
            .and(path("/db/relationships/rel-add"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(edge("rel-add", "a", "c", "commons")),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/db/relationships/rel-private"))
            .respond_with(ResponseTemplate::new(200).set_body_json(edge(
                "rel-private",
                "a",
                "d",
                "intimate",
            )))
            .mount(&server)
            .await;

        let r = CacheRefresher::new(Arc::clone(&store), server.uri());
        r.on_storage_event("relationship.created", r#"{"id":"rel-add","sourceId":"a"}"#);
        r.on_storage_event("relationship.created", r#"{"id":"rel-private"}"#);
        r.on_storage_event("relationship.deleted", r#"{"id":"rel-del"}"#);
        r.drain_once().await;

        assert!(store.get(RELATIONSHIP, "rel-add").await.is_some());
        assert!(store.get(RELATIONSHIP, "rel-private").await.is_none());
        assert!(store.get(RELATIONSHIP, "rel-del").await.is_none());
    }

    fn sse(events: &[(&str, &str, JsonValue)], done: bool) -> String {
        let mut body = String::new();
        for (etype, id, data) in events {
            body.push_str(&format!("event: {etype}\nid: {id}\ndata: {data}\n\n"));
        }
        if done {
            body.push_str("event: cache.done\ndata: {}\n\n");
        }
        body
    }

    /// The sweep catches what events missed: a new row with no event appears,
    /// a changed row updates, and a cached row the stream no longer carries is
    /// verified individually and evicted when storage refuses it.
    #[tokio::test]
    async fn a_sweep_catches_missed_creates_updates_and_deletes() {
        let server = MockServer::start().await;
        let store = store();
        seed(
            &store,
            CONTENT,
            "changed",
            content("changed", "old", "commons"),
        )
        .await;
        seed(
            &store,
            CONTENT,
            "vanished",
            content("vanished", "v", "commons"),
        )
        .await;
        seed(
            &store,
            RELATIONSHIP,
            "rel-vanished",
            edge("rel-vanished", "p", "q", "commons"),
        )
        .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/cache/stream"))
            .respond_with(ResponseTemplate::new(200).set_body_string(sse(
                &[
                    (
                        "cache.content",
                        "changed",
                        content("changed", "new", "commons"),
                    ),
                    (
                        "cache.content",
                        "fresh",
                        content("fresh", "fresh", "commons"),
                    ),
                ],
                true,
            )))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/db/content/vanished"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/db/relationships/rel-vanished"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&server)
            .await;

        let r = CacheRefresher::new(Arc::clone(&store), server.uri());
        let report = r.sweep_once().await;
        assert!(report.completed);
        assert_eq!(report.candidates_queued, 2);
        r.drain_once().await;

        assert_eq!(title_of(&store, "changed").await.as_deref(), Some("new"));
        assert_eq!(title_of(&store, "fresh").await.as_deref(), Some("fresh"));
        assert!(store.get(CONTENT, "vanished").await.is_none());
        assert!(store.get(RELATIONSHIP, "rel-vanished").await.is_none());
    }

    /// A stream cut short (no `cache.done`) proves nothing about absence: no
    /// cached row is questioned.
    #[tokio::test]
    async fn an_incomplete_sweep_questions_nothing() {
        let server = MockServer::start().await;
        let store = store();
        seed(&store, CONTENT, "kept", content("kept", "k", "commons")).await;
        Mock::given(method("GET"))
            .and(path("/api/v1/cache/stream"))
            .respond_with(ResponseTemplate::new(200).set_body_string(sse(&[], false)))
            .mount(&server)
            .await;

        let r = CacheRefresher::new(Arc::clone(&store), server.uri());
        let report = r.sweep_once().await;
        assert!(!report.completed);
        assert_eq!(report.candidates_queued, 0);
        assert_eq!(r.pending_len(), 0);
        assert!(store.get(CONTENT, "kept").await.is_some());
    }

    /// A streamed row whose reach the cache may not hold is not written, and
    /// an older cached copy of it is questioned (and withdrawn).
    #[tokio::test]
    async fn a_streamed_row_at_a_closed_reach_is_not_written_and_is_questioned() {
        let server = MockServer::start().await;
        let store = store();
        seed(
            &store,
            CONTENT,
            "closed",
            content("closed", "was open", "commons"),
        )
        .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/cache/stream"))
            .respond_with(ResponseTemplate::new(200).set_body_string(sse(
                &[
                    (
                        "cache.content",
                        "closed",
                        content("closed", "now closed", "intimate"),
                    ),
                    (
                        "cache.content",
                        "secret",
                        content("secret", "s", "intimate"),
                    ),
                ],
                true,
            )))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/db/content/closed"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let r = CacheRefresher::new(Arc::clone(&store), server.uri());
        let report = r.sweep_once().await;
        assert_eq!(report.candidates_queued, 1);
        assert!(
            store.get(CONTENT, "secret").await.is_none(),
            "never written"
        );
        r.drain_once().await;
        assert!(
            store.get(CONTENT, "closed").await.is_none(),
            "old copy withdrawn"
        );
    }

    /// Only rows the storage cache path wrote are sweep candidates — a DHT
    /// signal projection is another writer's document.
    #[tokio::test]
    async fn a_sweep_leaves_other_writers_documents_alone() {
        let server = MockServer::start().await;
        let store = store();
        store
            .set(ProjectedDocument::new(
                CONTENT,
                "dht-doc",
                "uhCkkActionHash",
                "uhCAkAuthor",
                content("dht-doc", "d", "commons"),
            ))
            .await
            .unwrap();
        Mock::given(method("GET"))
            .and(path("/api/v1/cache/stream"))
            .respond_with(ResponseTemplate::new(200).set_body_string(sse(&[], true)))
            .mount(&server)
            .await;

        let r = CacheRefresher::new(Arc::clone(&store), server.uri());
        let report = r.sweep_once().await;
        assert!(report.completed);
        assert_eq!(report.candidates_queued, 0);
        assert!(store.get(CONTENT, "dht-doc").await.is_some());
    }

    /// A large sweep backlog never holds an event refresh behind it: one drain
    /// serves every event key and only a bounded slice of verifications.
    #[tokio::test]
    async fn sweep_verifications_yield_to_events() {
        let server = MockServer::start().await;
        let store = store();
        let backlog = CANDIDATES_PER_DRAIN * 3;
        for i in 0..backlog {
            let id = format!("old-{i}");
            seed(&store, CONTENT, &id, content(&id, "o", "commons")).await;
        }
        Mock::given(method("GET"))
            .and(path("/api/v1/cache/stream"))
            .respond_with(ResponseTemplate::new(200).set_body_string(sse(&[], true)))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/db/content/urgent"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(content("urgent", "now", "commons")),
            )
            .mount(&server)
            .await;
        no_edges(&server, "urgent").await;
        // Every other content read: gone.
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let r = CacheRefresher::new(Arc::clone(&store), server.uri());
        assert_eq!(r.sweep_once().await.candidates_queued, backlog);
        r.on_storage_event("content.created", r#"{"id":"urgent"}"#);
        r.drain_once().await;

        assert_eq!(title_of(&store, "urgent").await.as_deref(), Some("now"));
        assert_eq!(
            r.pending_len(),
            backlog - CANDIDATES_PER_DRAIN,
            "one drain verifies a bounded slice"
        );
        while r.pending_len() > 0 {
            r.drain_once().await;
        }
        assert!(store.get(CONTENT, "old-0").await.is_none());
        // A second sweep before the backlog drains adds no duplicates.
        let again = CacheRefresher::new(Arc::clone(&store), server.uri());
        let mut q = again.candidates.lock().unwrap();
        let k = CacheKey::Content("x".into());
        assert_eq!(q.push_all(vec![k.clone(), k], 10), 1);
    }

    /// Overflow never grows the pending map; it requests a sweep instead.
    #[test]
    fn the_pending_map_is_bounded() {
        let r = CacheRefresher::new(store(), "http://unused");
        for i in 0..MAX_PENDING {
            assert!(r.enqueue(CacheKey::Content(format!("c{i}")), RefreshKind::Refresh));
        }
        assert!(!r.enqueue(
            CacheKey::Content("one-too-many".into()),
            RefreshKind::Refresh
        ));
        assert_eq!(r.pending_len(), MAX_PENDING);
        assert_eq!(r.stats().overflows, 1);
        // An already-pending key still coalesces at the cap.
        assert!(r.enqueue(CacheKey::Content("c0".into()), RefreshKind::Remove));
    }
}
