//! Compute Dashboard API controller
//!
//! Routes: `/api/v1/compute/dashboard[/refresh]`
//!
//! Assembles `SheafaDashboardStateView` from multiple sources:
//! - Compute metrics (live probes via `services::system_metrics`)
//! - Allocation snapshots from stewarded resources (env-driven defaults
//!   until DHT-attested operator policy lands; see plan
//!   `2026-05-10-compute-metrics-un-stub.md`)
//! - Custodian health from the `custodian_metrics` table
//! - Infrastructure token balances from economic events (action='produce')
//! - Constitutional limits (env-driven ceiling defaults; dignity floor still
//!   bootstrap-only until policy service integration)
//!
//! Network metrics + history series remain zero until a network probe
//! foundation lands (separate sprint). This is per-node only — cross-node
//! aggregation is the household-hub surface's concern, not this layer.
//!
//! Angular is a thin display client — no aggregation happens in TypeScript.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::config::default_storage_dir;
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::{response, system_metrics};
use crate::views::{
    AllocationBlockView, AllocationSnapshotView, CeilingLimitView, ComputeMetricsView,
    ConstitutionalLimitsStatusView, DignityFloorView, ExchangeRateView,
    FamilyCommunityProtectionStatusView, GovernanceLevelAllocationView,
    InfrastructureTokenBalanceView, RecentEconomicEventView, SheafaDashboardStateView,
    UpTimeMetricsView,
};

use super::get_conn;

// ---------------------------------------------------------------------------
// Route dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/compute*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');
    match (&method, path) {
        (&Method::GET, "dashboard") => handle_dashboard(req, pool, ctx).await,
        (&Method::POST, "dashboard/refresh") => handle_refresh(pool, ctx).await,
        _ => Ok(response::not_found(&format!(
            "Unknown compute route: {path}"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/v1/compute/dashboard[?level=<governance_level>]
async fn handle_dashboard(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Parse optional ?level= query param (unused in initial assembly but
    // available for future governance-level filtering).
    let _governance_level = req
        .uri()
        .query()
        .and_then(|q| {
            q.split('&')
                .find(|kv| kv.starts_with("level="))
                .map(|kv| kv.trim_start_matches("level=").to_string())
        })
        .unwrap_or_else(|| "individual".to_string());

    assemble_dashboard(pool, ctx).await
}

/// Core dashboard assembly — queries DB and returns the full view.
///
/// Separated from the HTTP handler so both GET and POST refresh can share it
/// without constructing a synthetic `Request<Incoming>`.
async fn assemble_dashboard(
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let conn = &mut get_conn(pool)?;

    // --- Query custodian health from custodian_metrics table ----------------
    use crate::db::diesel_schema::custodian_metrics::dsl as cm_dsl;
    use crate::db::models::CustodianMetrics;
    use diesel::prelude::*;

    let custodian_rows: Vec<CustodianMetrics> = cm_dsl::custodian_metrics
        .filter(cm_dsl::h_app_id.eq(&ctx.h_app_id))
        .order(cm_dsl::last_updated_at.desc())
        .limit(50)
        .load(conn)
        .unwrap_or_default();

    // Derive a simple protection status from custodian count.
    let total_custodians = custodian_rows.len() as u32;
    let protection_level = match total_custodians {
        0 => "vulnerable",
        1..=2 => "protected",
        _ => "highly-protected",
    };

    // --- Query recent economic events (action='produce') for token balance --
    use crate::db::diesel_schema::economic_events::dsl as ee_dsl;
    use crate::db::models::EconomicEvent;

    let produce_events: Vec<EconomicEvent> = ee_dsl::economic_events
        .filter(ee_dsl::h_app_id.eq(&ctx.h_app_id))
        .filter(ee_dsl::action.eq("produce"))
        .order(ee_dsl::has_point_in_time.desc())
        .limit(20)
        .load(conn)
        .unwrap_or_default();

    let balance_tokens: f64 = produce_events
        .iter()
        .filter_map(|e| e.resource_quantity_value.map(|v| v as f64))
        .sum();

    let recent_economic_events: Vec<RecentEconomicEventView> = produce_events
        .iter()
        .map(|e| RecentEconomicEventView {
            id: e.id.clone(),
            timestamp: e.has_point_in_time.clone(),
            event_type: e.action.clone(),
            provider: Some(e.provider.clone()),
            receiver: Some(e.receiver.clone()),
            quantity_has_unit: e
                .resource_quantity_unit
                .clone()
                .unwrap_or_else(|| "token".to_string()),
            quantity_has_numerical_value: e.resource_quantity_value.unwrap_or(0.0) as f64,
            tokens_minted: e.resource_quantity_value.map(|v| v as f64),
            note: e.note.clone().unwrap_or_default(),
        })
        .collect();

    // --- Timestamp ----------------------------------------------------------
    let now = chrono::Utc::now().to_rfc3339();

    // --- Assemble SheafaDashboardStateView with live + default values -------
    let view = SheafaDashboardStateView {
        // Identity — operator is the local agent; detailed lookup pending
        // imagodei integration.
        operator_id: ctx.h_app_id.clone(),
        operator_name: "Local Operator".to_string(),
        stewarded_resource_id: String::new(),
        node_id: "local".to_string(),
        node_location_region: None,
        node_location_country: None,
        node_location_latitude: None,
        node_location_longitude: None,

        // Status
        status: "online".to_string(),
        last_heartbeat: now.clone(),
        uptime: UpTimeMetricsView {
            up_percent: 100.0,
            downtime_hours_24: 0.0,
            downtime_hours_7d: 0.0,
            downtime_hours_30d: 0.0,
            last_failure: None,
            consecutive_uptime: "0d".to_string(),
            reliability: "good".to_string(),
        },

        // Compute — live probes via services::system_metrics
        compute_metrics: current_compute_metrics(),

        // Allocations — env-driven defaults until DHT-attested operator policy lands
        allocations: current_allocation_snapshot(),

        // Protection — assembled from custodian_metrics count
        family_community_protection: FamilyCommunityProtectionStatusView {
            redundancy_strategy: "full_replica".to_string(),
            redundancy_factor: if total_custodians > 0 {
                total_custodians as f64
            } else {
                1.0
            },
            recovery_threshold: total_custodians.max(1),
            custodians: vec![],
            total_custodians,
            geographic_regions: vec![],
            geographic_risk_profile: "distributed".to_string(),
            trust_graph: vec![],
            protection_level: protection_level.to_string(),
            estimated_recovery_time: "unknown".to_string(),
            last_verification: now.clone(),
            verification_status: "pending".to_string(),
        },

        // Economics — token balance from produce events
        infrastructure_tokens: InfrastructureTokenBalanceView {
            balance_tokens,
            balance_estimated_value: 0.0,
            balance_currency: "USD".to_string(),
            earning_rate_tokens_per_hour: 0.0,
            earning_rate_cpu_allocation: 0.0,
            earning_rate_storage_allocation: 0.0,
            earning_rate_bandwidth_allocation: 0.0,
            earning_rate_estimated_monthly: 0.0,
            decay_demurrage_rate: 0.0,
            decay_last_calculated: now.clone(),
            decay_projected_next_month_tokens: 0.0,
            decay_projected_next_month_value_usd: 0.0,
            token_history_last_24h: 0.0,
            token_history_last_7d: 0.0,
            token_history_last_30d: 0.0,
            token_history_all_time: balance_tokens,
            transactions: vec![],
            exchange_rates: vec![ExchangeRateView {
                from: "token".to_string(),
                to: "USD".to_string(),
                rate: 0.0,
                source: "algorithm".to_string(),
                last_updated: now.clone(),
            }],
        },
        economic_events: recent_economic_events,

        // Constitutional — env-driven ceiling; dignity floor bootstrap-only
        constitutional_limits: current_constitutional_limits(),

        last_updated: now,
        update_frequency_ms: 5000,
    };

    Ok(response::ok(&view))
}

/// POST /api/v1/compute/dashboard/refresh
///
/// Triggers a recalculation cycle. For now this is a read-through that
/// returns fresh dashboard state; a background refresh task can be wired
/// in once the compute metrics service exists.
async fn handle_refresh(
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    assemble_dashboard(pool, ctx).await
}

// ---------------------------------------------------------------------------
// Live constructors — probes + operator-driven virtual limits
// ---------------------------------------------------------------------------

const BYTES_PER_GB: f64 = 1_000_000_000.0;

/// Build a [`ComputeMetricsView`] from `services::system_metrics` probes.
///
/// CPU + memory + filesystem capacity + load average are live. Network
/// metrics, per-pod storage breakdowns beyond `cache_gb`, and history
/// series remain at zero pending separate probe sprints.
pub fn current_compute_metrics() -> ComputeMetricsView {
    let cpu_total = system_metrics::cpu_count().unwrap_or(0);
    let total_mem_bytes = system_metrics::total_memory_bytes().unwrap_or(0);
    let used_mem_bytes = system_metrics::process_memory_bytes().unwrap_or(0);
    let memory_total_gb = total_mem_bytes as f64 / BYTES_PER_GB;
    let memory_used_gb = used_mem_bytes as f64 / BYTES_PER_GB;
    let memory_available_gb = (memory_total_gb - memory_used_gb).max(0.0);
    let memory_usage_percent = if memory_total_gb > 0.0 {
        (memory_used_gb / memory_total_gb * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };

    let storage_dir = default_storage_dir();
    let storage_total_bytes = system_metrics::filesystem_capacity_bytes(&storage_dir).unwrap_or(0);
    let storage_total_gb = storage_total_bytes as f64 / BYTES_PER_GB;
    let blob_dir = storage_dir.join("blobs");
    let cache_bytes = system_metrics::directory_size(&blob_dir).unwrap_or(0);
    let storage_used_gb = cache_bytes as f64 / BYTES_PER_GB;
    let storage_available_gb = (storage_total_gb - storage_used_gb).max(0.0);
    let storage_usage_percent = if storage_total_gb > 0.0 {
        (storage_used_gb / storage_total_gb * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };

    let (load_one, load_five, load_fifteen) =
        system_metrics::load_average().unwrap_or((0.0, 0.0, 0.0));

    ComputeMetricsView {
        cpu_total_cores: cpu_total,
        // No per-process partitioning yet — available == total.
        cpu_available: cpu_total,
        // Instantaneous CPU usage requires sampling; derive a coarse signal
        // from 1-min load average normalized to core count (capped at 100).
        cpu_usage_percent: if cpu_total > 0 {
            (load_one / cpu_total as f64 * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        },
        // History series belongs to a sampler sprint; ship empty for now.
        cpu_usage_history: vec![],
        cpu_temperature: None,
        memory_total_gb,
        memory_used_gb,
        memory_available_gb,
        memory_usage_percent,
        memory_usage_history: vec![],
        storage_total_gb,
        storage_used_gb,
        storage_available_gb,
        storage_usage_percent,
        storage_usage_history: vec![],
        // This pod's slice of storage is its blob-store directory. Holochain,
        // custodian-data, and user-applications breakdowns are owned by
        // other pods — surface them via the household-hub aggregator, not here.
        storage_breakdown_holochain_gb: 0.0,
        storage_breakdown_cache_gb: storage_used_gb,
        storage_breakdown_custodian_data_gb: 0.0,
        storage_breakdown_user_applications_gb: 0.0,
        // TODO(network-probe-sprint): no foundation today. All zeros until
        // a netlink/sampling primitive lands. Tracking field shapes against
        // the SheafaDashboardStateView wire schema.
        network_upstream_mbps: 0.0,
        network_downstream_mbps: 0.0,
        network_used_upstream_mbps: 0.0,
        network_used_downstream_mbps: 0.0,
        network_latency_p50: 0.0,
        network_latency_p95: 0.0,
        network_latency_p99: 0.0,
        network_connections_total: 0,
        network_connections_holochain: 0,
        network_connections_cache: 0,
        network_connections_custodian: 0,
        load_average_one_minute: load_one,
        load_average_five_minutes: load_five,
        load_average_fifteen_minutes: load_fifteen,
        power_consumption_watts: None,
        power_thermal_output: None,
    }
}

// ---------------------------------------------------------------------------
// Operator virtual limits (env-driven until DHT-attested policy lands)
// ---------------------------------------------------------------------------

/// Read an `f64` env var, falling back to `default` on unset / parse failure.
fn env_f64(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(default)
}

/// Operator-declared ceiling: the hard upper bound this peer agrees not to
/// exceed in this dwellinghub. Defaults to "no virtual cap" (probes themselves
/// are the cap). When DHT-attested policy lands, this becomes the projection
/// of the policy entry; the env vars stay as a debugging escape hatch.
pub fn current_ceiling_limit() -> CeilingLimitView {
    let probe_cores = system_metrics::cpu_count().unwrap_or(0) as f64;
    let probe_memory_gb = system_metrics::total_memory_bytes()
        .map(|b| b as f64 / BYTES_PER_GB)
        .unwrap_or(0.0);
    CeilingLimitView {
        compute_max_cores: env_f64("ELOHIM_OPERATOR_CEILING_MAX_CORES", probe_cores),
        compute_max_memory_gb: env_f64("ELOHIM_OPERATOR_CEILING_MAX_MEMORY_GB", probe_memory_gb),
        compute_max_storage_gb: env_f64("ELOHIM_OPERATOR_CEILING_MAX_STORAGE_GB", 0.0),
        compute_max_bandwidth_mbps: env_f64("ELOHIM_OPERATOR_CEILING_MAX_BANDWIDTH_MBPS", 0.0),
        token_accumulation_ceiling: 0.0,
        current_accumulation: 0.0,
        percent_of_ceiling: 0.0,
        status: "safe".to_string(),
        enforcement: "voluntary".to_string(),
    }
}

/// Operator-declared allocation block: how the operator has virtually
/// partitioned compute on this pod for this workload. Defaults to the
/// full probe values (no virtual partitioning). Same env-as-escape-hatch
/// model as the ceiling.
pub fn current_allocation_block() -> AllocationBlockView {
    let probe_cores = system_metrics::cpu_count().unwrap_or(0) as f64;
    let probe_memory_gb = system_metrics::total_memory_bytes()
        .map(|b| b as f64 / BYTES_PER_GB)
        .unwrap_or(0.0);
    AllocationBlockView {
        id: "operator-default".to_string(),
        label: "Operator allocation".to_string(),
        governance_level: "individual".to_string(),
        priority: 0,
        cpu_cores: env_f64("ELOHIM_OPERATOR_ALLOCATION_CPU_CORES", probe_cores),
        cpu_percent: 100.0,
        memory_gb: env_f64("ELOHIM_OPERATOR_ALLOCATION_MEMORY_GB", probe_memory_gb),
        memory_percent: 100.0,
        storage_gb: 0.0,
        storage_percent: 0.0,
        bandwidth_mbps: 0.0,
        bandwidth_percent: 0.0,
        utilized_cpu_percent: 0.0,
        utilized_memory_percent: 0.0,
        utilized_storage_percent: 0.0,
        utilized_bandwidth_percent: 0.0,
        commitment_id: None,
        related_agents: vec![],
    }
}

fn current_governance_level_allocation() -> GovernanceLevelAllocationView {
    GovernanceLevelAllocationView {
        cpu_percent: 0.0,
        memory_percent: 0.0,
        storage_percent: 0.0,
        bandwidth_percent: 0.0,
    }
}

fn current_allocation_snapshot() -> AllocationSnapshotView {
    AllocationSnapshotView {
        by_governance_individual: current_governance_level_allocation(),
        by_governance_household: current_governance_level_allocation(),
        by_governance_community: current_governance_level_allocation(),
        by_governance_network: current_governance_level_allocation(),
        total_allocated_cpu_percent: 0.0,
        total_allocated_memory_percent: 0.0,
        total_allocated_storage_percent: 0.0,
        total_allocated_bandwidth_percent: 0.0,
        allocation_blocks: vec![current_allocation_block()],
    }
}

fn current_constitutional_limits() -> ConstitutionalLimitsStatusView {
    ConstitutionalLimitsStatusView {
        dignity_floor: DignityFloorView {
            compute_min_cores: 1.0,
            compute_min_memory_gb: 1.0,
            compute_min_storage_gb: 10.0,
            compute_min_bandwidth_mbps: 1.0,
            status: "met".to_string(),
            percent_of_floor: 100.0,
            enforcement: "voluntary".to_string(),
        },
        ceiling_limit: current_ceiling_limit(),
        safe_zone_cpu: 100.0,
        safe_zone_memory: 100.0,
        safe_zone_storage: 100.0,
        safe_zone_bandwidth: 100.0,
        safe_zone_tokens: 100.0,
        alerts: vec![],
    }
}
