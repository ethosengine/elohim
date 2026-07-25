//! Periodic LOCAL capacity reporter — the writer that fixes the always-0
//! `elohim_custodian_{free,used,stewarded}_bytes` gauges.
//!
//! # The gap this closes
//!
//! `operational_weave_facing::emit_capacity_gauges` (the ONLY place those three
//! Prometheus gauges are `.set()`) is called exactly once, from
//! `build_weave_view`, which runs only on `GET /api/v1/weave`. Its input,
//! `load_custodian_relation`, folds over the `custodian_metrics` table — which
//! has been permanently empty: the table's only writer was a browser-side
//! Angular service that was never started (and only ever carried placeholder
//! data). So the gauges have always read 0, regardless of real disk state.
//!
//! This module is the missing periodic writer. Every tick it:
//!
//! 1. measures THIS node's real storage numbers (used / free / total / held),
//! 2. publishes them straight to the `/metrics` gauges (this node's own
//!    measured truth — no DB join required), and
//! 3. upserts a `custodian_metrics` row so `GET /api/v1/weave`'s
//!    cluster-aggregate fold has real data to read instead of an eternally
//!    empty table.
//!
//! # Identity discipline (the join-key-sensitive column)
//!
//! `custodian_metrics.custodian_id` is NOT a free-form label — the
//! `operational_weave_facing::load_custodian_relation` loader joins it against
//! `rea_commitments.provider` (`"Join key: custodian_metrics.custodian_id ==
//! rea_commitments.provider == agent_cid"`) to compute stewarded bytes. Both
//! sides of that join are expected to be the pod's Holochain `agent_cid`
//! (`uhCAk…`) — see `elohim-storage/CLAUDE.md` → "Identity &
//! Transport-Identity Coherence". Writing a libp2p (`12D3Koo…`) or iroh
//! transport id here would mint a row that can never join — worse than
//! absence (mirrors the reasoning in `identity_namespace` and
//! `conductor_commitment_author::resolve_provider`).
//!
//! So the upsert is GATED on [`crate::identity_namespace::resolve_agent_cid_write`]
//! resolving a real `agent_cid` from (a) the active local session's key, then
//! (b) the pod's own conductor cell key — the same candidate order
//! `conductor_commitment_author::resolve_provider` uses. When NEITHER
//! resolves (e.g. the conductor cell is not yet available this tick), the
//! upsert is SKIPPED with a WARN rather than poisoning the join — but the
//! Prometheus gauges are still set from local truth regardless, because they
//! carry no join and do not need an `agent_cid` to be meaningful.
//!
//! `stewarded_bytes` itself is only knowable once identity resolves (it is
//! `peer_capacity_service::compute_unique_shard_bytes`, keyed by our OWN
//! `agent_cid`): with no identity resolvable this tick it reports 0 — a
//! measured absence, never a fabricated guess under the wrong namespace.
//!
//! # Task-spawn idiom
//!
//! Mirrors `services::peer_status_fanout::run_fanout_loop`: `interval` +
//! `MissedTickBehavior::Skip` + a one-time settle delay, gated by an
//! `_SECS` env var (`0` disables — the caller simply does not spawn the task).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use diesel::sqlite::SqliteConnection;
use tokio::sync::broadcast;
use tokio::time::{interval, MissedTickBehavior};

use crate::db::DbPool;
use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::identity_namespace::resolve_agent_cid_write;
use crate::services::{operational_weave_facing, peer_capacity_service, system_metrics};

/// Default report cadence (seconds). Overridable via `CAPACITY_REPORT_SECS`;
/// `0` disables the loop entirely (the task is simply not spawned).
pub const DEFAULT_REPORT_SECS: u64 = 300;

/// Settle delay before the first sweep (seconds) — mirrors the peer_status
/// fan-in / identity_fill convention so boot projections land first.
pub const DEFAULT_SETTLE_SECS: u64 = 30;

// ─────────────────────────────────────────────────────────────────────────────
// Pure composition — the measured numbers → the wire shapes this reporter
// writes. No I/O; unit-tested directly.
// ─────────────────────────────────────────────────────────────────────────────

/// One tick's raw local measurement — the impure probes' outputs, gathered
/// before any composition.
///
/// `free_bytes` MUST be filesystem-AVAILABLE space (`fs4::available_space`) —
/// never `total_bytes - used_bytes`. Other tenants can occupy bytes on the
/// same volume that our blob-store walk never counts as "used" (shared PVCs,
/// sidecar logs, the conductor's own DB), so `total - used` would silently
/// OVER-report how much room is actually left. This is the same distinction
/// `heartbeat::measure_free_pct` already makes for the free-storage %.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CapacityMeasurement {
    /// Bytes actually consumed by this node's blob store (`directory_size`).
    pub used_bytes: u64,
    /// Filesystem-AVAILABLE bytes on the blob store's volume (`fs4::available_space`).
    pub free_bytes: u64,
    /// Filesystem TOTAL capacity of the blob store's volume (`fs4::total_space`).
    pub total_bytes: u64,
    /// Deduped bytes this node actually holds shards for (`compute_unique_shard_bytes`,
    /// keyed by our own resolved `agent_cid`). `0` when identity did not
    /// resolve this tick (measured absence, not a guess).
    pub stewarded_bytes: u64,
}

/// Pure: compose the measurement into the [`elohim_views::ComputeTriptych`]
/// the `/metrics` gauges mirror. Callers pass this straight to
/// [`operational_weave_facing::emit_capacity_gauges`] — the ONLY function
/// that ever calls `.set()` on those three gauges, so this reporter reuses it
/// rather than duplicating the gauge-setting logic.
pub fn compose_gauge_triptych(m: &CapacityMeasurement) -> elohim_views::ComputeTriptych {
    elohim_views::ComputeTriptych {
        free: Some(m.free_bytes),
        used: Some(m.used_bytes),
        stewarded: Some(m.stewarded_bytes),
    }
}

/// Pure: compose the measurement into the `CustodianStorageMetricsView`
/// fields that become `custodian_metrics.storage_json` — the one metric
/// group this reporter actually measures.
pub fn compose_storage_view(m: &CapacityMeasurement) -> elohim_views::CustodianStorageMetricsView {
    let utilization_percent = if m.total_bytes == 0 {
        0.0
    } else {
        ((m.used_bytes as f64 / m.total_bytes as f64) * 100.0).clamp(0.0, 100.0)
    };
    elohim_views::CustodianStorageMetricsView {
        total_capacity_bytes: clamp_i64(m.total_bytes),
        used_bytes: clamp_i64(m.used_bytes),
        free_bytes: clamp_i64(m.free_bytes),
        utilization_percent,
        by_domain: None,
        // Not measured by this reporter (no RS-encoding breakdown source here) —
        // 0, never fabricated.
        full_replica_bytes: 0,
        threshold_bytes: 0,
        erasure_coded_bytes: 0,
    }
}

fn clamp_i64(v: u64) -> i64 {
    v.min(i64::MAX as u64) as i64
}

/// Neutral placeholders for the five `custodian_metrics` metric groups this
/// reporter does NOT measure (health/bandwidth/computation/reputation/economic).
/// The schema packs six NOT-NULL JSON blobs into one row; storage is the only
/// real signal this reporter produces. `availability`/`uptime_percent`/
/// `error_rate` are chosen to avoid manufacturing false alerts on
/// `GET /api/v1/custodians/metrics/alerts` (which reads exactly those three
/// fields) — `availability: true` is not a guess (the row was just written by
/// a live process), the rest are documented placeholders, not measurements.
fn neutral_health_view() -> elohim_views::CustodianHealthView {
    elohim_views::CustodianHealthView {
        uptime_percent: 100.0,
        availability: true,
        response_time_p50_ms: 0.0,
        response_time_p95_ms: 0.0,
        response_time_p99_ms: 0.0,
        error_rate: 0.0,
        sla_compliance: true,
    }
}

fn neutral_bandwidth_view() -> elohim_views::CustodianBandwidthView {
    elohim_views::CustodianBandwidthView {
        declared_mbps: 0.0,
        current_usage_mbps: 0.0,
        peak_usage_mbps: 0.0,
        average_usage_mbps: 0.0,
        utilization_percent: 0.0,
        inbound_mbps: 0.0,
        outbound_mbps: 0.0,
        by_domain: None,
    }
}

fn neutral_computation_view() -> elohim_views::CustodianComputationView {
    elohim_views::CustodianComputationView {
        cpu_cores: 0,
        cpu_usage_percent: 0.0,
        memory_gb: 0.0,
        memory_usage_percent: 0.0,
        zome_ops_per_second: 0.0,
        reconstruction_workload_percent: 0.0,
    }
}

fn neutral_reputation_view() -> elohim_views::CustodianReputationView {
    elohim_views::CustodianReputationView {
        reliability_rating: 0.0,
        speed_rating: 0.0,
        reputation_score: 0.0,
        specialization_bonus: 0.0,
        commitment_fulfillment: 0.0,
    }
}

fn neutral_economic_view() -> elohim_views::CustodianEconomicView {
    elohim_views::CustodianEconomicView {
        steward_tier: 0,
        price_per_gb: 0.0,
        monthly_earnings: 0.0,
        lifetime_earnings: 0.0,
        active_commitments: 0,
        total_committed_bytes: 0,
    }
}

/// Pure: compose the full upsert row from a resolved `custodian_id` + the
/// measurement. Isolated from the DB call itself so the composition is
/// unit-tested without a pool.
pub fn build_upsert_row(
    custodian_id: &str,
    h_app_id: &str,
    m: &CapacityMeasurement,
    now_ms: i64,
) -> crate::db::models::UpsertCustodianMetrics {
    crate::db::models::UpsertCustodianMetrics {
        custodian_id: custodian_id.to_string(),
        h_app_id: h_app_id.to_string(),
        tier: 0,
        health_json: serde_json::to_string(&neutral_health_view()).unwrap_or_default(),
        storage_json: serde_json::to_string(&compose_storage_view(m)).unwrap_or_default(),
        bandwidth_json: serde_json::to_string(&neutral_bandwidth_view()).unwrap_or_default(),
        computation_json: serde_json::to_string(&neutral_computation_view()).unwrap_or_default(),
        reputation_json: serde_json::to_string(&neutral_reputation_view()).unwrap_or_default(),
        economic_json: serde_json::to_string(&neutral_economic_view()).unwrap_or_default(),
        collected_at: now_ms,
        last_updated_at: now_ms,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Impure orchestration — one tick.
// ─────────────────────────────────────────────────────────────────────────────

/// Outcome of one report tick — surfaced in the tracing line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapacityReportOutcome {
    pub measurement: CapacityMeasurement,
    /// `true` when a `custodian_metrics` row was upserted this tick.
    pub upserted: bool,
    /// The resolved `agent_cid` this tick, if any.
    pub custodian_id: Option<String>,
}

/// Measure this node's real storage numbers.
///
/// `stewarded_bytes` needs the RESOLVED `agent_cid` to filter
/// `peer_blob_inventory` (`compute_unique_shard_bytes`) — with `custodian_id
/// == None` it is `0` (we cannot know what WE hold without our own join key;
/// this is a measured absence, not a guess under some other namespace).
fn measure_capacity(
    conn: &mut SqliteConnection,
    used_bytes: u64,
    free_bytes: u64,
    total_bytes: u64,
    custodian_id: Option<&str>,
) -> CapacityMeasurement {
    let stewarded_bytes = match custodian_id {
        Some(id) => peer_capacity_service::compute_unique_shard_bytes(conn, id).unwrap_or(0),
        None => 0,
    };
    CapacityMeasurement {
        used_bytes,
        free_bytes,
        total_bytes,
        stewarded_bytes,
    }
}

/// Resolve the `custodian_id` `agent_cid` to write this tick, from (a) the
/// active local session's key, then (b) the pod's own conductor cell key
/// (`self_cell_key`, the caller's `hc.agent_key_uhcak()`) — the same
/// candidate order `conductor_commitment_author::resolve_provider` uses.
/// `None` when neither candidate is `agent_cid`-shaped.
fn resolve_custodian_id(conn: &mut SqliteConnection, self_cell_key: &str) -> Option<String> {
    let session_cid = crate::db::local_sessions::get_active_session(conn)
        .ok()
        .flatten()
        .map(|s| s.agent_pub_key);
    resolve_agent_cid_write(&[session_cid.as_deref(), Some(self_cell_key)])
}

/// Run one report tick: measure, ALWAYS set the `/metrics` gauges from local
/// truth, then upsert `custodian_metrics` only if an `agent_cid` resolved.
///
/// `self_cell_key` is the caller's `hc.agent_key_uhcak()` — threaded in as a
/// plain string (rather than an `Arc<HcClient>`) so this function needs no
/// conductor connection to unit-test; the one conductor-touching call
/// (`agent_key_uhcak`, synchronous — reads the already-connected cell id, no
/// zome call) stays in the loop wrapper.
///
/// The blob-store walk (`directory_size`) is offloaded to
/// [`tokio::task::spawn_blocking`] — mirrors `heartbeat::DefaultProbe`, which
/// makes the same call for the lighter single-`statvfs` probe; a full
/// recursive walk is the heavier case for the same reason (never stall the
/// runtime on a slow/large mount).
pub async fn run_report_once(
    pool: &DbPool,
    blob_path: &Path,
    h_app_id: &str,
    self_cell_key: &str,
) -> Result<CapacityReportOutcome, StorageError> {
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(format!("capacity_reporter: pool: {e}")))?;

    let custodian_id = resolve_custodian_id(&mut conn, self_cell_key);

    let path_for_probe = blob_path.to_path_buf();
    let (used_bytes, total_bytes, free_bytes) = tokio::task::spawn_blocking(move || {
        let used = system_metrics::directory_size(&path_for_probe).unwrap_or(0);
        let total = system_metrics::filesystem_capacity_bytes(&path_for_probe).unwrap_or(0);
        let free = fs4::available_space(&path_for_probe).unwrap_or(0);
        (used, total, free)
    })
    .await
    .unwrap_or((0, 0, 0)); // blocking-task join error — degrade to zeros, non-fatal

    let measurement = measure_capacity(
        &mut conn,
        used_bytes,
        free_bytes,
        total_bytes,
        custodian_id.as_deref(),
    );

    // Gauges are LOCAL truth — set every tick regardless of identity
    // resolution; they carry no join and do not need an agent_cid.
    operational_weave_facing::emit_capacity_gauges(&compose_gauge_triptych(&measurement));

    match &custodian_id {
        Some(id) => {
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            let upsert = build_upsert_row(id, h_app_id, &measurement, now_ms);
            crate::db::custodian_metrics::upsert_metrics(&mut conn, upsert)?;
            Ok(CapacityReportOutcome {
                measurement,
                upserted: true,
                custodian_id,
            })
        }
        None => {
            tracing::warn!(
                "capacity_reporter: no agent_cid resolvable this tick (session + own cell key \
                 both non-agent_cid) — skipping custodian_metrics upsert (a transport-id row \
                 could never join the aggregate-capacity fold); Prometheus gauges set from \
                 local truth regardless"
            );
            Ok(CapacityReportOutcome {
                measurement,
                upserted: false,
                custodian_id: None,
            })
        }
    }
}

/// The periodic report loop. Mirrors `peer_status_fanout::run_fanout_loop`:
/// `interval` + `MissedTickBehavior::Skip` + a shutdown branch, with a
/// one-time settle delay before the first sweep. `report_secs == 0` is
/// handled by the caller (the task is simply not spawned).
pub async fn run_report_loop(
    pool: DbPool,
    hc: Arc<HcClient>,
    blob_path: PathBuf,
    h_app_id: String,
    report_secs: u64,
    settle_secs: u64,
    mut shutdown: broadcast::Receiver<()>,
) {
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(settle_secs)) => {}
        _ = shutdown.recv() => {
            tracing::debug!("capacity_reporter: shutdown during settle, exiting");
            return;
        }
    }

    let mut ticker = interval(Duration::from_secs(report_secs.max(1)));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                // Synchronous, no zome call — reads the already-connected cell id.
                let self_cell_key = hc.agent_key_uhcak();
                match run_report_once(&pool, &blob_path, &h_app_id, &self_cell_key).await {
                    Ok(outcome) if outcome.upserted => {
                        tracing::debug!(
                            used_bytes = outcome.measurement.used_bytes,
                            free_bytes = outcome.measurement.free_bytes,
                            total_bytes = outcome.measurement.total_bytes,
                            stewarded_bytes = outcome.measurement.stewarded_bytes,
                            custodian_id = outcome.custodian_id.as_deref().unwrap_or(""),
                            "capacity_reporter: swept (custodian_metrics upserted + gauges set)"
                        );
                    }
                    Ok(outcome) => {
                        tracing::debug!(
                            used_bytes = outcome.measurement.used_bytes,
                            free_bytes = outcome.measurement.free_bytes,
                            total_bytes = outcome.measurement.total_bytes,
                            "capacity_reporter: swept (gauges set from local truth; upsert \
                             skipped, see prior WARN)"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "capacity_reporter: sweep failed (retry next tick)")
                    }
                }
            }
            _ = shutdown.recv() => {
                tracing::debug!("capacity_reporter: shutdown signal received, exiting");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::local_sessions::{create_session, CreateLocalSessionInput};
    use crate::test_util::test_pool;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    /// Serializes the `run_report_once` tests that assert on the process-global
    /// `ELOHIM_CUSTODIAN_*_BYTES` gauges immediately after setting them — without
    /// this, the two tests below (and, in principle, any other test touching the
    /// same gauges) could race each other in parallel test threads. Poisoning
    /// (a prior panic while holding the lock) is tolerated.
    static CUSTODIAN_GAUGE_TEST_LOCK: Mutex<()> = Mutex::new(());
    fn lock_custodian_gauges() -> std::sync::MutexGuard<'static, ()> {
        CUSTODIAN_GAUGE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    fn measurement(used: u64, free: u64, total: u64, stewarded: u64) -> CapacityMeasurement {
        CapacityMeasurement {
            used_bytes: used,
            free_bytes: free,
            total_bytes: total,
            stewarded_bytes: stewarded,
        }
    }

    // ---- pure composition: gauges ----

    #[test]
    fn compose_gauge_triptych_mirrors_the_measurement_verbatim() {
        let m = measurement(5, 30, 100, 12);
        let t = compose_gauge_triptych(&m);
        assert_eq!(t.free, Some(30));
        assert_eq!(t.used, Some(5));
        assert_eq!(t.stewarded, Some(12));
    }

    // ---- pure composition: storage view — the free-vs-total-minus-used guard ----

    #[test]
    fn compose_storage_view_uses_measured_free_not_total_minus_used() {
        // total=100, used=5 → total-used would be 95, but OTHER TENANTS occupy
        // the volume too, so the real available space (free=30) must win.
        let m = measurement(5, 30, 100, 0);
        let view = compose_storage_view(&m);
        assert_eq!(
            view.free_bytes, 30,
            "free_bytes must be the measured available-space probe, never total-used"
        );
        assert_ne!(
            view.free_bytes,
            (m.total_bytes - m.used_bytes) as i64,
            "total-used (95) is NOT the same as the measured free (30) — the whole point \
             of the distinction"
        );
        assert_eq!(view.used_bytes, 5);
        assert_eq!(view.total_capacity_bytes, 100);
    }

    #[test]
    fn compose_storage_view_computes_utilization_from_used_over_total() {
        let m = measurement(25, 50, 100, 0);
        let view = compose_storage_view(&m);
        assert_eq!(view.utilization_percent, 25.0);
    }

    #[test]
    fn compose_storage_view_zero_total_is_zero_utilization_not_div_by_zero() {
        let m = measurement(0, 0, 0, 0);
        let view = compose_storage_view(&m);
        assert_eq!(view.utilization_percent, 0.0);
    }

    #[test]
    fn compose_storage_view_leaves_unmeasured_domains_at_zero() {
        let m = measurement(1, 2, 3, 0);
        let view = compose_storage_view(&m);
        assert_eq!(view.full_replica_bytes, 0);
        assert_eq!(view.threshold_bytes, 0);
        assert_eq!(view.erasure_coded_bytes, 0);
        assert!(view.by_domain.is_none());
    }

    // ---- pure composition: the neutral placeholder groups don't fabricate alerts ----

    #[test]
    fn build_upsert_row_health_group_never_trips_the_alerts_endpoint() {
        let m = measurement(1, 2, 3, 0);
        let row = build_upsert_row("uhCAkSELF", "lamad", &m, 1_700_000_000_000);
        let health: elohim_views::CustodianHealthView =
            serde_json::from_str(&row.health_json).expect("valid health JSON");
        // GET /api/v1/custodians/metrics/alerts fires "unavailable" when
        // !availability, "uptime" when < 95.0, "error_rate" when > 0.05 — none
        // of those must fire from an unmeasured placeholder.
        assert!(health.availability);
        assert!(health.uptime_percent >= 95.0);
        assert!(health.error_rate <= 0.05);
    }

    #[test]
    fn build_upsert_row_carries_the_resolved_id_and_scope() {
        let m = measurement(10, 20, 30, 5);
        let row = build_upsert_row("uhCAkSELF", "lamad", &m, 42);
        assert_eq!(row.custodian_id, "uhCAkSELF");
        assert_eq!(row.h_app_id, "lamad");
        assert_eq!(row.collected_at, 42);
        assert_eq!(row.last_updated_at, 42);
        let storage: elohim_views::CustodianStorageMetricsView =
            serde_json::from_str(&row.storage_json).expect("valid storage JSON");
        assert_eq!(storage.used_bytes, 10);
        assert_eq!(storage.free_bytes, 20);
        assert_eq!(storage.total_capacity_bytes, 30);
    }

    // ---- identity resolution: session wins, then cell key, else None ----

    fn seed_session(pool: &DbPool, agent_pub_key: &str) {
        let mut conn = pool.get().unwrap();
        create_session(
            &mut conn,
            CreateLocalSessionInput {
                id: None,
                human_id: "human-1".to_string(),
                agent_pub_key: agent_pub_key.to_string(),
                doorway_url: "https://doorway.example".to_string(),
                doorway_id: None,
                identifier: "human-1@example.com".to_string(),
                display_name: None,
                profile_image_hash: None,
                bootstrap_url: None,
            },
        )
        .expect("seed session");
    }

    #[test]
    fn resolve_custodian_id_prefers_session_over_cell_key() {
        let pool = test_pool();
        seed_session(&pool, "uhCAkSESSION");
        let mut conn = pool.get().unwrap();
        assert_eq!(
            resolve_custodian_id(&mut conn, "uhCAkCELLKEY").as_deref(),
            Some("uhCAkSESSION")
        );
    }

    #[test]
    fn resolve_custodian_id_falls_back_to_cell_key_when_no_session() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        assert_eq!(
            resolve_custodian_id(&mut conn, "uhCAkCELLKEY").as_deref(),
            Some("uhCAkCELLKEY")
        );
    }

    #[test]
    fn resolve_custodian_id_none_when_cell_key_is_a_transport_id() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        assert_eq!(
            resolve_custodian_id(&mut conn, "12D3KooWNotAnAgentCid"),
            None
        );
    }

    // ---- run_report_once: end-to-end tick over a real (test) pool + tempdir ----

    #[tokio::test]
    async fn run_report_once_upserts_and_sets_gauges_when_identity_resolves() {
        let _guard = lock_custodian_gauges();
        crate::metrics::register_all(); // idempotent (Once-guarded)
        let pool = test_pool();
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("blob-a"), b"hello world").unwrap(); // 11 bytes

        let outcome = run_report_once(&pool, tmp.path(), "lamad", "uhCAkSELFCELL")
            .await
            .expect("tick succeeds");

        assert!(
            outcome.upserted,
            "a valid agent_cid resolved from the cell key"
        );
        assert_eq!(outcome.custodian_id.as_deref(), Some("uhCAkSELFCELL"));
        assert_eq!(outcome.measurement.used_bytes, 11);
        assert!(
            outcome.measurement.total_bytes > 0,
            "the tempdir's volume has a real total capacity"
        );

        // The row landed under the resolved agent_cid.
        let mut conn = pool.get().unwrap();
        let row = crate::db::custodian_metrics::get_metrics_by_id(
            &mut conn,
            "uhCAkSELFCELL",
            &crate::db::AppContext::new("lamad"),
        )
        .unwrap()
        .expect("row upserted");
        assert_eq!(row.custodian_id, "uhCAkSELFCELL");

        // Gauges reflect this tick's measurement (mirrors compose_gauge_triptych).
        assert_eq!(
            crate::metrics::ELOHIM_CUSTODIAN_USED_BYTES.get(),
            11,
            "the used-bytes gauge reflects the real blob-store walk"
        );
    }

    #[tokio::test]
    async fn run_report_once_skips_upsert_but_still_sets_gauges_when_no_identity_resolves() {
        let _guard = lock_custodian_gauges();
        crate::metrics::register_all(); // idempotent (Once-guarded)
        let pool = test_pool();
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("blob-a"), b"12345").unwrap(); // 5 bytes

        // No session seeded, and the "cell key" is a transport id — no agent_cid
        // resolves this tick.
        let outcome = run_report_once(&pool, tmp.path(), "lamad", "12D3KooWTransportOnly")
            .await
            .expect("tick still succeeds (gauges-only path)");

        assert!(
            !outcome.upserted,
            "no agent_cid resolvable — upsert skipped"
        );
        assert!(outcome.custodian_id.is_none());
        assert_eq!(
            outcome.measurement.stewarded_bytes, 0,
            "stewarded is a measured absence, never guessed, with no resolved identity"
        );
        assert_eq!(outcome.measurement.used_bytes, 5);

        // Gauges STILL reflect local truth — they carry no join.
        assert_eq!(crate::metrics::ELOHIM_CUSTODIAN_USED_BYTES.get(), 5);

        // No row was written under either candidate.
        let mut conn = pool.get().unwrap();
        assert!(crate::db::custodian_metrics::get_metrics_by_id(
            &mut conn,
            "12D3KooWTransportOnly",
            &crate::db::AppContext::new("lamad"),
        )
        .unwrap()
        .is_none());
    }
}
