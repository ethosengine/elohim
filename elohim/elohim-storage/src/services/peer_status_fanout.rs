//! Cross-agent PeerStatus fan-in.
//!
//! ## The self-only ceiling this breaks
//!
//! `peer_statuses` (SQLite) is STRUCTURALLY self-only. Its sole writer is the
//! post-commit signal projection (`signals::handle_signal` →
//! `db::peer_statuses::upsert`), which fires only for THIS conductor's own
//! `record_peer_status`. Consequence chain: `peer_selection` can only ever
//! nominate self → `distribute_shards` only self-holds → the resilience card's
//! `stewardingCollectives` caps at 1. Yet every pod ALREADY publishes its own
//! `PeerStatus` to the infrastructure DHT every 60s (`heartbeat.rs`), and the
//! infrastructure zome fn `get_latest_peer_status_for_agent(agent)` reads ANY
//! agent's latest snapshot. The only missing link was a storage-side reader that
//! fans in OTHER agents' statuses. This module is that reader.
//!
//! ## Two honesty guards
//!
//! 1. **Freshness-monotone write** — the fan-in writes exclusively through
//!    `db::peer_statuses::upsert_if_newer`, never plain `upsert`. A DHT read can
//!    lag, and the plain last-writer-wins `upsert` would let a stale read clobber
//!    a fresher row (the signal-projected self row, or a more-recent fan-in). The
//!    guard makes the fan-in strictly monotone on `timestamp`.
//!
//! 2. **Staleness window lives in the CONSUMERS, not here** — the fan-in records
//!    a peer's last-known snapshot verbatim (subject to guard 1). Whether that
//!    snapshot is still "live enough" to count is a consumer decision applied at
//!    read time (`services::peer_selection`, `services::household_resilience`) via
//!    `PEER_STATUS_STALENESS_SECS`. This module never drops a row for age — it
//!    would be lying about what the DHT last said.
//!
//! ## Testable seam
//!
//! `run_once` is pure loop logic over the [`PeerStatusFetcher`] trait; the real
//! [`ConductorPeerStatusFetcher`] (the thin, conductor-dependent half — a zome
//! call + wire decode) is isolated behind it so tests drive `run_once` with an
//! in-memory fake and no `HcClient`.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::broadcast;
use tokio::time::{interval, MissedTickBehavior};

use crate::db::DbPool;
use crate::hc_client::HcClient;
use crate::StorageError;

/// Default fan-in cadence (seconds). Overridable via `PEER_STATUS_FANOUT_SECS`;
/// `0` disables the loop entirely.
pub const DEFAULT_FANOUT_SECS: u64 = 300;

/// Settle delay before the first sweep (seconds) — mirrors the projection-
/// reconcile convention so the conductor + boot projections land first.
pub const DEFAULT_SETTLE_SECS: u64 = 30;

/// First rest (seconds) for an agent that answered `Ok(None)`, beyond the sweep
/// cadence. Doubles on each consecutive `None`.
pub const NONE_REST_BASE_SECS: u64 = 5 * 60;

/// Longest rest (seconds) for an agent that keeps answering `Ok(None)`. Cost: a
/// peer that starts publishing status is noticed at most one cap (plus a sweep)
/// late. Until its first snapshot lands there is no row for it, so consumers
/// (`peer_selection`, `household_resilience`) simply do not count it; the
/// `PEER_STATUS_STALENESS_SECS` window (default 900) only dims rows that exist.
pub const NONE_REST_CAP_SECS: u64 = 60 * 60;

/// A decoded, storage-ready PeerStatus snapshot for one agent, produced by a
/// [`PeerStatusFetcher`]. This is the boundary type between the conductor-facing
/// fetch half and the pure apply half — it carries exactly what a
/// `db::peer_statuses::PeerStatusRow` needs, in storage units.
#[derive(Debug, Clone, PartialEq)]
pub struct FannedPeerStatus {
    /// The peer's `agent_cid` (`uhCAk…`) — the canonical join key. Taken from the
    /// decoded DHT entry, so it is the DHT's own truth for this peer's id.
    pub peer_id: String,
    /// Lowercased lifecycle string (`online`/`degraded`/…), matching the
    /// signal-projection convention in the `peer_statuses.status` column.
    pub status: String,
    pub general_pool_member: i32,
    pub accepting_stewardship_reserves: i32,
    pub archetype_class: Option<String>,
    /// DHT entry timestamp, microseconds since epoch.
    pub timestamp: i64,
}

/// Outcome tallies for one sweep — surfaced in the tracing line (and available to
/// a future `/metrics` counter seam).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FanoutStats {
    /// Roster agents whose DHT read returned a snapshot (Some).
    pub fetched: usize,
    /// Snapshots that were strictly newer and landed a write.
    pub upserted: usize,
    /// Snapshots skipped because the stored row was already as-fresh-or-fresher.
    pub skipped_stale: usize,
    /// Roster agents whose read failed (logged, non-fatal), returned None, or were
    /// resting on a remembered None (still unanswered, so still missed).
    pub missed: usize,
    /// Of `missed`, the agents not asked this sweep because a remembered None
    /// is still resting.
    pub rested: usize,
}

/// Agents that answered a definite `Ok(None)`, kept in-process (a restart asks
/// everyone once) and bounded by the roster: [`RestMemo::retain_roster`] drops
/// agents that left it. A `None` rests the agent on a doubling window; `Some`
/// clears it; an `Err` is not an answer and touches nothing.
#[derive(Debug)]
pub struct RestMemo {
    cadence: Duration,
    /// agent -> (consecutive Nones, do not ask before)
    resting: HashMap<String, (u32, Instant)>,
}

impl RestMemo {
    /// `cadence` is the sweep interval; the rest is counted beyond it.
    pub fn new(cadence: Duration) -> Self {
        Self {
            cadence,
            resting: HashMap::new(),
        }
    }

    fn is_resting(&self, agent: &str, now: Instant) -> bool {
        self.resting
            .get(agent)
            .is_some_and(|(_, until)| now < *until)
    }

    fn remember_none(&mut self, agent: &str, now: Instant) {
        let n = self
            .resting
            .get(agent)
            .map_or(0, |(n, _)| *n)
            .saturating_add(1);
        let window = NONE_REST_BASE_SECS
            .saturating_mul(1u64 << (n - 1).min(16))
            .min(NONE_REST_CAP_SECS);
        // Half a cadence of slack so tick jitter cannot decide whether the
        // sweep after the window lands just before or just after it.
        let until = now + self.cadence + Duration::from_secs(window) - self.cadence / 2;
        self.resting.insert(agent.to_string(), (n, until));
    }

    fn clear(&mut self, agent: &str) {
        self.resting.remove(agent);
    }

    fn retain_roster(&mut self, roster: &[String]) {
        let keep: HashSet<&str> = roster.iter().map(String::as_str).collect();
        self.resting.retain(|a, _| keep.contains(a.as_str()));
    }
}

/// Seam for reading one agent's latest DHT PeerStatus. The real impl is a zome
/// call; tests supply a fake so `run_once` is exercised without a conductor.
#[async_trait::async_trait]
pub trait PeerStatusFetcher: Send + Sync {
    /// Fetch the latest PeerStatus for `agent_cid`. `Ok(None)` when the agent has
    /// never published one (skip, no row). `Err` is a per-agent read failure —
    /// the caller logs and continues so one dead agent never aborts the sweep.
    async fn fetch_latest(&self, agent_cid: &str) -> anyhow::Result<Option<FannedPeerStatus>>;
}

/// Build the fan-in roster from the raw `humans.agent_pub_key` column values.
///
/// Pure so it is unit-tested directly: drops NULL/empty keys, excludes this pod's
/// own `self_cid` (self rows arrive fresher via the signal projection), and dedups
/// while preserving first-seen order (deterministic sweeps).
pub fn build_roster(raw_keys: Vec<Option<String>>, self_cid: &str) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut roster: Vec<String> = Vec::new();
    for key in raw_keys.into_iter().flatten() {
        if key.is_empty() || key == self_cid {
            continue;
        }
        if seen.insert(key.clone()) {
            roster.push(key);
        }
    }
    roster
}

/// Run one fan-in sweep: build the roster, fetch each peer's latest DHT snapshot,
/// and monotone-upsert it into `peer_statuses`.
///
/// This is the pure loop half — the conductor dependency is entirely inside
/// `fetcher`. A per-agent read failure is logged and skipped (`missed`), never
/// propagated, so one unreachable peer cannot dark the whole sweep.
pub async fn run_once<F: PeerStatusFetcher + ?Sized>(
    pool: &DbPool,
    fetcher: &F,
    self_cid: &str,
) -> Result<FanoutStats, StorageError> {
    let mut memo = RestMemo::new(Duration::from_secs(DEFAULT_FANOUT_SECS));
    run_once_resting(pool, fetcher, self_cid, &mut memo, Instant::now()).await
}

/// As [`run_once`], but agents resting on a remembered `Ok(None)` in `memo` are
/// not asked. `now` is injected so tests do not sleep.
pub async fn run_once_resting<F: PeerStatusFetcher + ?Sized>(
    pool: &DbPool,
    fetcher: &F,
    self_cid: &str,
    memo: &mut RestMemo,
    now: Instant,
) -> Result<FanoutStats, StorageError> {
    let raw_keys = {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("peer_status_fanout: pool: {e}")))?;
        crate::db::humans::list_agent_pub_keys(&mut conn, crate::db::HUMANS_HAPP_ID)?
    };
    let roster = build_roster(raw_keys, self_cid);
    memo.retain_roster(&roster);

    let mut stats = FanoutStats::default();
    for agent in &roster {
        if memo.is_resting(agent, now) {
            stats.missed += 1;
            stats.rested += 1;
            continue;
        }
        match fetcher.fetch_latest(agent).await {
            Ok(Some(fanned)) => {
                memo.clear(agent);
                stats.fetched += 1;
                let row = crate::db::peer_statuses::PeerStatusRow {
                    peer_id: fanned.peer_id,
                    status: fanned.status,
                    general_pool_member: fanned.general_pool_member,
                    accepting_stewardship_reserves: fanned.accepting_stewardship_reserves,
                    archetype_class: fanned.archetype_class,
                    timestamp: fanned.timestamp,
                    // The fan-in read (`get_latest_peer_status_for_agent`) returns
                    // the PeerStatus ENTRY, not its ActionHash, so there is no
                    // action-hash anchor to record here (unlike the signal path,
                    // which carries `action_hash`). Empty = "fan-in projected,
                    // no local anchor"; the DHT remains the source of truth.
                    dht_anchor_hash: String::new(),
                    updated_at: chrono::Utc::now().timestamp_micros(),
                };
                let mut conn = pool.get().map_err(|e| {
                    StorageError::Internal(format!("peer_status_fanout: pool: {e}"))
                })?;
                match crate::db::peer_statuses::upsert_if_newer(&mut conn, &row) {
                    Ok(true) => stats.upserted += 1,
                    Ok(false) => stats.skipped_stale += 1,
                    Err(e) => {
                        stats.missed += 1;
                        tracing::warn!(agent = %agent, error = %e, "peer_status fan-in upsert failed (continuing)");
                    }
                }
            }
            Ok(None) => {
                memo.remember_none(agent, now);
                stats.missed += 1;
            }
            Err(e) => {
                stats.missed += 1;
                tracing::warn!(agent = %agent, error = %e, "peer_status fan-in fetch failed (continuing)");
            }
        }
    }
    Ok(stats)
}

/// The periodic fan-in loop. Mirrors the provide-reconcile task shape in
/// `main.rs`: `interval` + `MissedTickBehavior::Skip` + a shutdown branch, with a
/// one-time settle delay before the first sweep. `fanout_secs == 0` is handled by
/// the caller (the task is simply not spawned).
pub async fn run_fanout_loop<F: PeerStatusFetcher + ?Sized>(
    pool: DbPool,
    fetcher: Arc<F>,
    self_cid: String,
    fanout_secs: u64,
    settle_secs: u64,
    mut shutdown: broadcast::Receiver<()>,
) {
    // One-time settle so boot projections + the first heartbeat land first.
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(settle_secs)) => {}
        _ = shutdown.recv() => {
            tracing::debug!("peer_status fan-in: shutdown during settle, exiting");
            return;
        }
    }

    let mut ticker = interval(Duration::from_secs(fanout_secs.max(1)));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut memo = RestMemo::new(Duration::from_secs(fanout_secs.max(1)));
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                match run_once_resting(&pool, fetcher.as_ref(), &self_cid, &mut memo, Instant::now())
                    .await
                {
                    Ok(stats) if stats.upserted > 0 => {
                        crate::metrics::set_peer_status_fanout_missed(stats.missed as u64);
                        tracing::info!(
                            fetched = stats.fetched,
                            upserted = stats.upserted,
                            skipped_stale = stats.skipped_stale,
                            missed = stats.missed,
                            rested = stats.rested,
                            "peer_status fan-in swept (cross-agent statuses projected)"
                        )
                    }
                    Ok(stats) => {
                        crate::metrics::set_peer_status_fanout_missed(stats.missed as u64);
                        tracing::debug!(
                            fetched = stats.fetched,
                            skipped_stale = stats.skipped_stale,
                            missed = stats.missed,
                            rested = stats.rested,
                            "peer_status fan-in swept (no newer snapshots)"
                        )
                    }
                    Err(e) => tracing::warn!(error = %e, "peer_status fan-in sweep failed (retry next tick)"),
                }
            }
            _ = shutdown.recv() => {
                tracing::debug!("peer_status fan-in: shutdown signal received, exiting");
                break;
            }
        }
    }
}

// ===========================================================================
// Real fetcher — the conductor-dependent half (a zome call + wire decode).
// ===========================================================================

/// Wire-side mirror of the DNA `PeerLifecycleState` enum. Variant NAMES must
/// match exactly — serde externally-tagged encoding writes the variant name as a
/// MessagePack string, which is what the conductor emits for the returned entry.
#[derive(Debug, Clone, serde::Deserialize)]
enum WirePeerLifecycleState {
    Starting,
    Online,
    Degraded,
    Maintenance,
    Leaving,
}

impl WirePeerLifecycleState {
    fn as_status_str(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Online => "online",
            Self::Degraded => "degraded",
            Self::Maintenance => "maintenance",
            Self::Leaving => "leaving",
        }
    }
}

/// Wire-side mirror of the DNA `PeerCapabilityFlags` struct.
#[derive(Debug, Clone, serde::Deserialize)]
struct WirePeerCapabilityFlags {
    general_pool_member: bool,
    accepting_stewardship_reserves: bool,
}

/// Read mirror of the DNA `PeerStatus` ENTRY (the shape
/// `get_latest_peer_status_for_agent` returns). Distinct from `heartbeat.rs`'s
/// `WirePeerStatus` (which is Serialize-only, for publishing): this is the
/// Deserialize direction. `peer_id` is `HoloHashB64` because the conductor
/// serializes `AgentPubKey` as a raw 39-byte array over MessagePack — a plain
/// `String` mirror would silently fail the decode (the same 2026-06-12 root cause
/// documented on `signals::HoloHashB64`). `flags` is NESTED (matching the entry),
/// not flattened as in the post-commit signal.
#[derive(Debug, Clone, serde::Deserialize)]
struct WirePeerStatusRead {
    peer_id: crate::signals::HoloHashB64,
    status: WirePeerLifecycleState,
    flags: WirePeerCapabilityFlags,
    archetype_class: Option<String>,
    /// `Timestamp` serializes transparently as its inner i64 (microseconds) —
    /// the same decode the `PeerStatusRecorded` signal mirror uses.
    timestamp: i64,
}

impl From<WirePeerStatusRead> for FannedPeerStatus {
    fn from(w: WirePeerStatusRead) -> Self {
        Self {
            peer_id: w.peer_id.0,
            status: w.status.as_status_str().to_string(),
            general_pool_member: w.flags.general_pool_member as i32,
            accepting_stewardship_reserves: w.flags.accepting_stewardship_reserves as i32,
            archetype_class: w.archetype_class,
            timestamp: w.timestamp,
        }
    }
}

/// Real [`PeerStatusFetcher`]: calls the infrastructure coordinator
/// `get_latest_peer_status_for_agent(AgentPubKey) -> Option<PeerStatus>` and
/// decodes the returned entry.
pub struct ConductorPeerStatusFetcher {
    hc: Arc<HcClient>,
}

impl ConductorPeerStatusFetcher {
    pub fn new(hc: Arc<HcClient>) -> Self {
        Self { hc }
    }
}

#[async_trait::async_trait]
impl PeerStatusFetcher for ConductorPeerStatusFetcher {
    async fn fetch_latest(&self, agent_cid: &str) -> anyhow::Result<Option<FannedPeerStatus>> {
        // agent_cid (`uhCAk…`) → AgentPubKey. Same b64 path storage uses elsewhere
        // (`api::account`), the inverse of `HcClient::agent_key_uhcak`.
        let agent = holochain_types::prelude::AgentPubKey::try_from(agent_cid).map_err(|e| {
            anyhow::anyhow!("peer_status fan-in: invalid agent_cid {agent_cid}: {e}")
        })?;
        // Single positional arg — encoded the same way single-arg zome calls are
        // elsewhere (`conductor_writes::get_rea_commitment`).
        let payload = rmp_serde::to_vec_named(&agent)
            .map_err(|e| anyhow::anyhow!("peer_status fan-in: encode AgentPubKey: {e}"))?;
        let bytes = self
            .hc
            .call_zome(
                "infrastructure",
                "get_latest_peer_status_for_agent",
                payload,
            )
            .await
            .map_err(|e| anyhow::anyhow!("get_latest_peer_status_for_agent failed: {e}"))?;
        let wire: Option<WirePeerStatusRead> = rmp_serde::from_slice(&bytes)
            .map_err(|e| anyhow::anyhow!("peer_status fan-in: decode Option<PeerStatus>: {e}"))?;
        Ok(wire.map(FannedPeerStatus::from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::peer_statuses::{get_by_peer, PeerStatusRow};
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::SqliteConnection;
    use std::collections::HashMap;
    use std::sync::Mutex;

    fn test_pool() -> DbPool {
        let url = format!(
            "file:peer_status_fanout_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn seed_human(pool: &DbPool, id: &str, agent_key: Option<&str>) {
        use crate::db::humans::create_human;
        let mut conn = pool.get().unwrap();
        create_human(
            &mut conn,
            crate::db::humans::CreateHumanInput {
                id: id.to_string(),
                agent_pub_key: agent_key.map(|k| k.to_string()),
                display_name: id.to_string(),
                bio: None,
                affinities: "[]".to_string(),
                profile_reach: "commons".to_string(),
                location: None,
                profile_photo_url: None,
                h_app_id: crate::db::HUMANS_HAPP_ID.to_string(),
                household_id: None,
            },
        )
        .expect("seed human");
    }

    // ---- roster building (pure) ----

    #[test]
    fn build_roster_skips_null_excludes_self_and_dedups() {
        let raw = vec![
            Some("uhCAkSELF".to_string()), // self — excluded
            None,                          // NULL — skipped
            Some("uhCAkA".to_string()),
            Some("uhCAkA".to_string()), // dup — collapsed
            Some(String::new()),        // empty — skipped
            Some("uhCAkB".to_string()),
        ];
        let roster = build_roster(raw, "uhCAkSELF");
        assert_eq!(roster, vec!["uhCAkA".to_string(), "uhCAkB".to_string()]);
    }

    // ---- run_once loop logic with a faked zome-call seam ----

    /// In-memory fetcher: returns canned snapshots by agent, an explicit None, or
    /// an error — so `run_once`'s skip/continue arms are all exercised without a
    /// conductor.
    struct FakeFetcher {
        by_agent: HashMap<String, anyhow::Result<Option<FannedPeerStatus>>>,
        calls: Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl PeerStatusFetcher for FakeFetcher {
        async fn fetch_latest(&self, agent_cid: &str) -> anyhow::Result<Option<FannedPeerStatus>> {
            self.calls.lock().unwrap().push(agent_cid.to_string());
            match self.by_agent.get(agent_cid) {
                Some(Ok(v)) => Ok(v.clone()),
                Some(Err(e)) => Err(anyhow::anyhow!("{e}")),
                None => Ok(None),
            }
        }
    }

    fn fanned(peer_id: &str, status: &str, ts: i64) -> FannedPeerStatus {
        FannedPeerStatus {
            peer_id: peer_id.to_string(),
            status: status.to_string(),
            general_pool_member: 1,
            accepting_stewardship_reserves: 1,
            archetype_class: None,
            timestamp: ts,
        }
    }

    #[tokio::test]
    async fn run_once_projects_other_agents_skips_none_and_survives_errors() {
        let pool = test_pool();
        // Roster: self (excluded), a live peer, a peer with no status (None), and
        // a peer whose read errors.
        seed_human(&pool, "h-self", Some("uhCAkSELF"));
        seed_human(&pool, "h-alive", Some("uhCAkALIVE"));
        seed_human(&pool, "h-none", Some("uhCAkNONE"));
        seed_human(&pool, "h-err", Some("uhCAkERR"));
        seed_human(&pool, "h-nullkey", None); // NULL key — never in roster

        let mut by_agent: HashMap<String, anyhow::Result<Option<FannedPeerStatus>>> =
            HashMap::new();
        by_agent.insert(
            "uhCAkALIVE".to_string(),
            Ok(Some(fanned("uhCAkALIVE", "online", 1_700_000_000_000_000))),
        );
        by_agent.insert("uhCAkNONE".to_string(), Ok(None));
        by_agent.insert(
            "uhCAkERR".to_string(),
            Err(anyhow::anyhow!("simulated dead peer")),
        );
        let fetcher = FakeFetcher {
            by_agent,
            calls: Mutex::new(vec![]),
        };

        let stats = run_once(&pool, &fetcher, "uhCAkSELF").await.unwrap();

        // Self and NULL-key humans never fetched; the erroring peer did not abort.
        let calls = fetcher.calls.lock().unwrap().clone();
        assert!(!calls.contains(&"uhCAkSELF".to_string()));
        assert_eq!(
            calls.len(),
            3,
            "alive + none + err fetched, self/null excluded"
        );

        assert_eq!(stats.fetched, 1, "only the live peer returned a snapshot");
        assert_eq!(stats.upserted, 1);
        assert_eq!(
            stats.missed, 2,
            "the None and the error both counted as missed"
        );

        // The live peer's row landed — the cross-agent fan-in wrote a row this pod
        // could never have authored itself.
        let mut conn = pool.get().unwrap();
        let row = get_by_peer(&mut conn, "uhCAkALIVE").unwrap().unwrap();
        assert_eq!(row.status, "online");
        assert_eq!(row.timestamp, 1_700_000_000_000_000);
        // Self was never fetched, so no self row was written by the fan-in.
        assert!(get_by_peer(&mut conn, "uhCAkSELF").unwrap().is_none());
    }

    #[tokio::test]
    async fn run_once_does_not_regress_a_fresher_existing_row() {
        let pool = test_pool();
        seed_human(&pool, "h-peer", Some("uhCAkPEER"));

        // A fresher row already exists (e.g. projected moments ago).
        {
            let mut conn = pool.get().unwrap();
            crate::db::peer_statuses::upsert(
                &mut conn,
                &PeerStatusRow {
                    peer_id: "uhCAkPEER".into(),
                    status: "online".into(),
                    general_pool_member: 1,
                    accepting_stewardship_reserves: 1,
                    archetype_class: None,
                    timestamp: 1_700_000_000_000_500,
                    dht_anchor_hash: String::new(),
                    updated_at: 1_700_000_000_000_500,
                },
            )
            .unwrap();
        }

        // The fan-in reads an OLDER snapshot off the (lagging) DHT.
        let mut by_agent: HashMap<String, anyhow::Result<Option<FannedPeerStatus>>> =
            HashMap::new();
        by_agent.insert(
            "uhCAkPEER".to_string(),
            Ok(Some(fanned("uhCAkPEER", "leaving", 1_700_000_000_000_100))),
        );
        let fetcher = FakeFetcher {
            by_agent,
            calls: Mutex::new(vec![]),
        };

        let stats = run_once(&pool, &fetcher, "uhCAkSELF").await.unwrap();
        assert_eq!(stats.fetched, 1);
        assert_eq!(stats.upserted, 0, "the stale read is not written");
        assert_eq!(stats.skipped_stale, 1);

        let mut conn = pool.get().unwrap();
        let row = get_by_peer(&mut conn, "uhCAkPEER").unwrap().unwrap();
        assert_eq!(row.status, "online", "the fresher row is preserved");
    }

    // ---- rest on a remembered None ----

    const CADENCE: Duration = Duration::from_secs(DEFAULT_FANOUT_SECS);

    fn roster_pool(nones: &[&str], somes: &[&str]) -> (DbPool, FakeFetcher) {
        let pool = test_pool();
        seed_human(&pool, "h-self", Some("uhCAkSELF"));
        let mut by_agent: HashMap<String, anyhow::Result<Option<FannedPeerStatus>>> =
            HashMap::new();
        for a in nones.iter().chain(somes.iter()) {
            seed_human(&pool, &format!("h-{a}"), Some(a));
        }
        for a in somes {
            by_agent.insert(
                a.to_string(),
                Ok(Some(fanned(a, "online", 1_700_000_000_000_000))),
            );
        }
        (
            pool,
            FakeFetcher {
                by_agent,
                calls: Mutex::new(vec![]),
            },
        )
    }

    fn take_calls(f: &FakeFetcher) -> Vec<String> {
        let mut c = std::mem::take(&mut *f.calls.lock().unwrap());
        c.sort();
        c
    }

    #[tokio::test]
    async fn an_agent_that_answers_none_is_not_reasked_on_the_next_sweep() {
        let nones = ["uhCAkN1", "uhCAkN2", "uhCAkN3", "uhCAkN4"];
        let (pool, f) = roster_pool(&nones, &["uhCAkS1", "uhCAkS2"]);
        let mut memo = RestMemo::new(CADENCE);
        let t0 = Instant::now();
        run_once_resting(&pool, &f, "uhCAkSELF", &mut memo, t0)
            .await
            .unwrap();
        assert_eq!(take_calls(&f).len(), 6);
        run_once_resting(&pool, &f, "uhCAkSELF", &mut memo, t0 + CADENCE)
            .await
            .unwrap();
        assert_eq!(take_calls(&f), vec!["uhCAkS1", "uhCAkS2"]);
    }

    #[tokio::test]
    async fn a_rested_agent_is_reasked_when_its_window_lapses_and_the_window_doubles() {
        let (pool, f) = roster_pool(&["uhCAkN1"], &[]);
        let mut memo = RestMemo::new(CADENCE);
        let mut t = Instant::now();
        let mut gaps = vec![];
        // Sweep every cadence; record how many sweeps pass between asks.
        let mut since = 0u32;
        for _ in 0..60 {
            run_once_resting(&pool, &f, "uhCAkSELF", &mut memo, t)
                .await
                .unwrap();
            if take_calls(&f).is_empty() {
                since += 1;
            } else {
                if since > 0 || gaps.is_empty() {
                    gaps.push(since);
                }
                since = 0;
            }
            t += CADENCE;
        }
        // Asked at once, then 1, 2, 4, 8 sweeps skipped (5, 10, 20, 40 min
        // windows beyond the cadence), then the 60 min cap holds at 12.
        assert_eq!(&gaps[..7], &[0, 1, 2, 4, 8, 12, 12], "gaps={gaps:?}");
    }

    #[tokio::test]
    async fn an_answer_clears_the_rest() {
        let (pool, mut f) = roster_pool(&["uhCAkP"], &[]);
        let mut memo = RestMemo::new(CADENCE);
        let t0 = Instant::now();
        run_once_resting(&pool, &f, "uhCAkSELF", &mut memo, t0)
            .await
            .unwrap();
        take_calls(&f);
        // It starts publishing; the window lapses.
        f.by_agent.insert(
            "uhCAkP".into(),
            Ok(Some(fanned("uhCAkP", "online", 1_700_000_000_000_000))),
        );
        let t1 = t0 + CADENCE * 3;
        let s = run_once_resting(&pool, &f, "uhCAkSELF", &mut memo, t1)
            .await
            .unwrap();
        assert_eq!(s.fetched, 1);
        assert_eq!(take_calls(&f).len(), 1);
        for i in 1..=3u32 {
            run_once_resting(&pool, &f, "uhCAkSELF", &mut memo, t1 + CADENCE * i)
                .await
                .unwrap();
            assert_eq!(take_calls(&f).len(), 1, "asked every sweep again");
        }
    }

    #[tokio::test]
    async fn an_error_is_never_remembered() {
        let (pool, mut f) = roster_pool(&[], &[]);
        seed_human(&pool, "h-e", Some("uhCAkE"));
        f.by_agent
            .insert("uhCAkE".into(), Err(anyhow::anyhow!("conductor down")));
        let mut memo = RestMemo::new(CADENCE);
        let t0 = Instant::now();
        run_once_resting(&pool, &f, "uhCAkSELF", &mut memo, t0)
            .await
            .unwrap();
        take_calls(&f);
        let s = run_once_resting(&pool, &f, "uhCAkSELF", &mut memo, t0 + CADENCE)
            .await
            .unwrap();
        assert_eq!(take_calls(&f).len(), 1, "asked again after an Err");
        assert_eq!(s.rested, 0);
    }

    #[tokio::test]
    async fn an_error_does_not_clear_or_advance_a_window() {
        let (pool, mut f) = roster_pool(&["uhCAkX"], &[]);
        let mut memo = RestMemo::new(CADENCE);
        let t0 = Instant::now();
        run_once_resting(&pool, &f, "uhCAkSELF", &mut memo, t0)
            .await
            .unwrap();
        take_calls(&f);
        let until = memo.resting["uhCAkX"];
        f.by_agent
            .insert("uhCAkX".into(), Err(anyhow::anyhow!("conductor down")));
        // Window lapsed: the ask errs; the memo entry is untouched.
        run_once_resting(&pool, &f, "uhCAkSELF", &mut memo, t0 + CADENCE * 3)
            .await
            .unwrap();
        assert_eq!(take_calls(&f).len(), 1);
        assert_eq!(memo.resting["uhCAkX"], until);
    }

    #[tokio::test]
    async fn a_new_roster_agent_is_asked_at_once_and_a_departed_one_is_forgotten() {
        let (pool, f) = roster_pool(&["uhCAkOLD"], &[]);
        let mut memo = RestMemo::new(CADENCE);
        let t0 = Instant::now();
        run_once_resting(&pool, &f, "uhCAkSELF", &mut memo, t0)
            .await
            .unwrap();
        take_calls(&f);
        assert!(memo.resting.contains_key("uhCAkOLD"));
        // The old agent leaves the roster; a new one joins.
        {
            let mut conn = pool.get().unwrap();
            use diesel::RunQueryDsl;
            diesel::sql_query("DELETE FROM humans WHERE id = 'h-uhCAkOLD'")
                .execute(&mut conn)
                .unwrap();
        }
        seed_human(&pool, "h-new", Some("uhCAkNEW"));
        run_once_resting(&pool, &f, "uhCAkSELF", &mut memo, t0 + CADENCE)
            .await
            .unwrap();
        assert_eq!(take_calls(&f), vec!["uhCAkNEW"]);
        assert!(!memo.resting.contains_key("uhCAkOLD"));
        assert_eq!(memo.resting.len(), 1, "bounded by the roster");
    }

    #[tokio::test]
    async fn a_rested_agent_still_counts_as_missed() {
        let (pool, f) = roster_pool(&["uhCAkN1", "uhCAkN2"], &["uhCAkS1"]);
        let mut memo = RestMemo::new(CADENCE);
        let t0 = Instant::now();
        let s1 = run_once_resting(&pool, &f, "uhCAkSELF", &mut memo, t0)
            .await
            .unwrap();
        assert_eq!((s1.missed, s1.rested), (2, 0));
        let s2 = run_once_resting(&pool, &f, "uhCAkSELF", &mut memo, t0 + CADENCE)
            .await
            .unwrap();
        assert_eq!((s2.fetched, s2.missed, s2.rested), (1, 2, 2));
    }
}
