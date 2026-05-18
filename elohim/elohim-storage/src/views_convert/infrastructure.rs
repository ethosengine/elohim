//! Infrastructure-domain Wire→View converters.
//!
//! Converts internal DB models for custodian metrics, peer status, spatial context,
//! scheduling, blob responses, and observation aggregation to View types defined
//! in `elohim_views::infrastructure`.

use elohim_views::shared::{parse_json, parse_json_opt};
use elohim_views::{
    CustodianBandwidthView, CustodianComputationView, CustodianEconomicView, CustodianHealthView,
    CustodianMetricsView, CustodianReputationView, CustodianStorageMetricsView,
    ElohimCapabilityProfile, ElohimReputationProfileView, HazardView, JsonVal,
    ObservationDiversitySummaryView, ObservationView, PeerStatusView, PlacementGapView, PlaceView,
    PutBlobResponseView, RenderCapabilityProfile, ReportCustodianMetricsInputView, RiskAlertView,
    ScheduleView, SpatialContextView,
};

use crate::db::models::{CustodianMetrics, Hazard, RiskAlert, Schedule};
use crate::db::peer_statuses::PeerStatusRow;

// CapabilityExtensions and CapabilityExtensionEntry are defined in elohim_views::infrastructure
// and re-exported at the root. They are NOT re-defined here.
pub use elohim_views::CapabilityExtensions;

// ============================================================================
// Custodian Metrics Views
// ============================================================================

impl From<CustodianMetrics> for CustodianMetricsView {
    fn from(m: CustodianMetrics) -> Self {
        // Metric groups are stored as JSON blobs; parse or fall back to defaults.
        let health: CustodianHealthView =
            serde_json::from_str(&m.health_json).unwrap_or(CustodianHealthView {
                uptime_percent: 0.0,
                availability: false,
                response_time_p50_ms: 0.0,
                response_time_p95_ms: 0.0,
                response_time_p99_ms: 0.0,
                error_rate: 0.0,
                sla_compliance: false,
            });
        let storage: CustodianStorageMetricsView = serde_json::from_str(&m.storage_json)
            .unwrap_or_else(|_| CustodianStorageMetricsView {
                total_capacity_bytes: 0,
                used_bytes: 0,
                free_bytes: 0,
                utilization_percent: 0.0,
                by_domain: None,
                full_replica_bytes: 0,
                threshold_bytes: 0,
                erasure_coded_bytes: 0,
            });
        let bandwidth: CustodianBandwidthView = serde_json::from_str(&m.bandwidth_json)
            .unwrap_or_else(|_| CustodianBandwidthView {
                declared_mbps: 0.0,
                current_usage_mbps: 0.0,
                peak_usage_mbps: 0.0,
                average_usage_mbps: 0.0,
                utilization_percent: 0.0,
                inbound_mbps: 0.0,
                outbound_mbps: 0.0,
                by_domain: None,
            });
        let computation: CustodianComputationView = serde_json::from_str(&m.computation_json)
            .unwrap_or(CustodianComputationView {
                cpu_cores: 0,
                cpu_usage_percent: 0.0,
                memory_gb: 0.0,
                memory_usage_percent: 0.0,
                zome_ops_per_second: 0.0,
                reconstruction_workload_percent: 0.0,
            });
        let reputation: CustodianReputationView = serde_json::from_str(&m.reputation_json)
            .unwrap_or(CustodianReputationView {
                reliability_rating: 0.0,
                speed_rating: 0.0,
                reputation_score: 0.0,
                specialization_bonus: 0.0,
                commitment_fulfillment: 0.0,
            });
        let economic: CustodianEconomicView =
            serde_json::from_str(&m.economic_json).unwrap_or(CustodianEconomicView {
                steward_tier: 0,
                price_per_gb: 0.0,
                monthly_earnings: 0.0,
                lifetime_earnings: 0.0,
                active_commitments: 0,
                total_committed_bytes: 0,
            });
        Self {
            custodian_id: m.custodian_id,
            tier: m.tier as u32,
            health,
            storage,
            bandwidth,
            computation,
            reputation,
            economic,
            collected_at: m.collected_at,
            last_updated_at: m.last_updated_at,
        }
    }
}

/// Convert ReportCustodianMetricsInputView to the insertable DB type.
pub fn report_custodian_metrics_into_upsert(
    view: ReportCustodianMetricsInputView,
    h_app_id: impl Into<String>,
    now_ms: i64,
) -> crate::db::models::UpsertCustodianMetrics {
    crate::db::models::UpsertCustodianMetrics {
        custodian_id: view.custodian_id,
        h_app_id: h_app_id.into(),
        tier: view.tier as i32,
        health_json: serde_json::to_string(&view.health).unwrap_or_default(),
        storage_json: serde_json::to_string(&view.storage).unwrap_or_default(),
        bandwidth_json: serde_json::to_string(&view.bandwidth).unwrap_or_default(),
        computation_json: serde_json::to_string(&view.computation).unwrap_or_default(),
        reputation_json: serde_json::to_string(&view.reputation).unwrap_or_default(),
        economic_json: serde_json::to_string(&view.economic).unwrap_or_default(),
        collected_at: view.collected_at.unwrap_or(now_ms),
        last_updated_at: now_ms,
    }
}

// ============================================================================
// Schedule Views (Kairos temporal dimension)
// ============================================================================

impl From<Schedule> for ScheduleView {
    fn from(s: Schedule) -> Self {
        Self {
            id: s.id,
            entity_type: s.entity_type,
            entity_id: s.entity_id,
            scheduled_at: s.scheduled_at,
            expires_at: s.expires_at,
            rrule: s.rrule,
            next_occurrence_at: s.next_occurrence_at,
            occurrence_count: s.occurrence_count,
            created_at: s.created_at,
        }
    }
}

// ============================================================================
// Spatial Context Views
// ============================================================================

impl From<crate::db::models::SpatialContext> for SpatialContextView {
    fn from(s: crate::db::models::SpatialContext) -> Self {
        Self {
            id: s.id,
            entity_type: s.entity_type,
            entity_id: s.entity_id,
            latitude: s.latitude,
            longitude: s.longitude,
            altitude: s.altitude,
            accuracy: s.accuracy,
            h3_res5: s.h3_res5,
            h3_res7: s.h3_res7,
            h3_res9: s.h3_res9,
            place_id: s.place_id,
            osm_type: s.osm_type,
            osm_id: s.osm_id,
            label: s.label,
            context_type: s.context_type,
            geometry_json: parse_json_opt(&s.geometry_json),
            metadata: parse_json_opt(&s.metadata_json),
            observed_at: s.observed_at,
            created_at: s.created_at,
            updated_at: s.updated_at,
            is_current: s.is_current == 1,
        }
    }
}

// ============================================================================
// Place Views (governed spatial entity — DHT projection)
// ============================================================================

impl From<crate::db::models::Place> for PlaceView {
    fn from(p: crate::db::models::Place) -> Self {
        Self {
            id: p.id,
            dht_anchor_hash: p.dht_anchor_hash,
            name: p.name,
            place_type: p.place_type,
            constitutional_layer: p.constitutional_layer,
            h3_index: p.h3_index,
            h3_resolution: p.h3_resolution,
            geometry_json: Some(parse_json(&p.geometry_json)),
            centroid_lat: p.centroid_lat,
            centroid_lng: p.centroid_lng,
            parent_place_id: p.parent_place_id,
            osm_reference: parse_json_opt(&p.osm_reference_json),
            carrying_capacity: Some(parse_json(&p.carrying_capacity_json)),
            governing_collective_id: p.governing_collective_id,
            status: p.status,
            created_by: p.created_by,
            created_at: p.created_at,
            updated_at: p.updated_at,
            metadata: Some(parse_json(&p.metadata_json)),
        }
    }
}

// ============================================================================
// Hazard Views (Sprint 7 — Risk + Resilience Mapping)
// ============================================================================

impl From<Hazard> for HazardView {
    fn from(h: Hazard) -> Self {
        Self {
            id: h.id,
            h_app_id: h.h_app_id,
            place_id: h.place_id,
            hazard_type: h.hazard_type,
            severity: h.severity,
            title: h.title,
            description: h.description,
            reported_at: h.reported_at,
            projected_onset: h.projected_onset,
            projected_end: h.projected_end,
            actual_onset: h.actual_onset,
            resolved_at: h.resolved_at,
            affected_h3_cells: parse_json(&h.affected_h3_cells),
            radius_km: h.radius_km,
            source: h.source,
            source_reference: h.source_reference,
            metadata: parse_json_opt(&Some(h.metadata_json)),
            status: h.status,
            created_at: h.created_at,
            updated_at: h.updated_at,
        }
    }
}

// ============================================================================
// RiskAlert Views (Sprint 7 — Risk + Resilience Mapping)
// ============================================================================

impl From<RiskAlert> for RiskAlertView {
    fn from(r: RiskAlert) -> Self {
        Self {
            id: r.id,
            h_app_id: r.h_app_id,
            place_id: r.place_id,
            alert_type: r.alert_type,
            severity: r.severity,
            title: r.title,
            description: r.description,
            trigger_hazard_id: r.trigger_hazard_id,
            trigger_data: parse_json_opt(&Some(r.trigger_data_json)),
            triggered_at: r.triggered_at,
            lead_time_hours: r.lead_time_hours,
            expires_at: r.expires_at,
            status: r.status,
            acknowledged_by: r.acknowledged_by,
            acknowledged_at: r.acknowledged_at,
            resolved_at: r.resolved_at,
            escalated_to: r.escalated_to,
            metadata: parse_json_opt(&Some(r.metadata_json)),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ============================================================================
// Peer Status Views
// Source of truth: Holochain infrastructure DNA DHT (Notarized, Category A).
// ============================================================================

impl From<PeerStatusRow> for PeerStatusView {
    fn from(row: PeerStatusRow) -> Self {
        Self {
            peer_id: row.peer_id,
            status: row.status,
            general_pool_member: row.general_pool_member != 0,
            accepting_stewardship_reserves: row.accepting_stewardship_reserves != 0,
            archetype_class: row.archetype_class,
            timestamp: row.timestamp.to_string(),
            dht_anchor_hash: row.dht_anchor_hash,
            updated_at: row.updated_at.to_string(),
            elohim_capability: None, // Layered post-construction via build_peer_status_view()
            render_capability: None, // Layered post-construction via build_peer_status_view()
            extensions: None,        // Layered post-construction via build_peer_status_view()
        }
    }
}

/// Build a `PeerStatusView` from a projection row plus the operator-configured capability.
///
/// The capability is Category C — operational, local state, not stored in the projection
/// table. It is loaded once at startup from `ELOHIM_CAPABILITY_CONFIG_FILE` and layered
/// here so that all construction sites stay consistent.
///
/// Use this instead of `PeerStatusView::from(row)` in handlers and tests.
pub fn build_peer_status_view(
    row: PeerStatusRow,
    elohim_capability: Option<&ElohimCapabilityProfile>,
    render_capability: Option<&RenderCapabilityProfile>,
    extensions: Option<&CapabilityExtensions>,
) -> PeerStatusView {
    let mut view = PeerStatusView::from(row);
    view.elohim_capability = elohim_capability.cloned();
    view.render_capability = render_capability.cloned();
    view.extensions = extensions.cloned();
    view
}

/// Load the operator-configured `ElohimCapabilityProfile` from the path
/// given in `ELOHIM_CAPABILITY_CONFIG_FILE`.
///
/// Returns `None` (honest degradation) when:
/// - The env var is unset
/// - The file does not exist or is not readable
/// - The file contains invalid JSON or does not match the profile shape
pub fn load_elohim_capability_from_env() -> Option<ElohimCapabilityProfile> {
    let path = std::env::var("ELOHIM_CAPABILITY_CONFIG_FILE").ok()?;
    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                path = %path,
                error = %e,
                "ELOHIM_CAPABILITY_CONFIG_FILE unreadable — elohim_capability will be None"
            );
            return None;
        }
    };
    match serde_json::from_str::<ElohimCapabilityProfile>(&contents) {
        Ok(profile) => Some(profile),
        Err(e) => {
            tracing::warn!(
                path = %path,
                error = %e,
                "ELOHIM_CAPABILITY_CONFIG_FILE contains invalid JSON — elohim_capability will be None"
            );
            None
        }
    }
}

/// Load the render capability profile from a doorway's `/admin/capability` HTTP endpoint.
pub async fn load_render_capability_from_url() -> Option<RenderCapabilityProfile> {
    let url = std::env::var("DOORWAY_CAPABILITY_URL").ok()?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                url = %url,
                error = %e,
                "DOORWAY_CAPABILITY_URL unreachable — render_capability will be None"
            );
            return None;
        }
    };
    if !resp.status().is_success() {
        tracing::warn!(
            url = %url,
            status = %resp.status(),
            "DOORWAY_CAPABILITY_URL returned non-success — render_capability will be None"
        );
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    match serde_json::from_slice::<RenderCapabilityProfile>(&bytes) {
        Ok(profile) => Some(profile),
        Err(e) => {
            tracing::warn!(
                url = %url,
                error = %e,
                "DOORWAY_CAPABILITY_URL response did not parse as RenderCapabilityProfile"
            );
            None
        }
    }
}

/// Synchronous wrapper for tests / non-async startup paths.
pub fn load_render_capability_from_url_blocking() -> Option<RenderCapabilityProfile> {
    if std::env::var("DOORWAY_CAPABILITY_URL").is_err() {
        return None;
    }
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    rt.block_on(load_render_capability_from_url())
}

// ============================================================================
// Placement Gap + Resilience Snapshot Views
// ============================================================================

impl From<crate::db::models::PlacementGapRow> for PlacementGapView {
    fn from(r: crate::db::models::PlacementGapRow) -> Self {
        Self {
            id: r.id,
            content_id: r.content_id,
            shard_hash: r.shard_hash,
            requested_steward_count: r.requested_steward_count,
            achieved_steward_count: r.achieved_steward_count,
            contract_coverage: r.contract_coverage,
            gap_kind: r.gap_kind,
            first_seen_at: r.first_seen_at,
            last_seen_at: r.last_seen_at,
        }
    }
}

// ============================================================================
// Observation Views
// ============================================================================

impl From<crate::db::models::ObservationRow> for ObservationView {
    fn from(r: crate::db::models::ObservationRow) -> Self {
        Self {
            observer_cid: r.observer_cid,
            log_cid: r.log_cid,
            log_offset: r.log_offset,
            observed_at: r.observed_at,
            seq: r.seq,
            observation_kind: r.observation_kind,
            subject_cid: r.subject_cid,
            subject_kind: r.subject_kind,
            payload_json: r.payload_json,
            observer_household_cid: r.observer_household_cid,
            observer_collective_cid: r.observer_collective_cid,
            observer_region: r.observer_region,
            observer_archetype: r.observer_archetype,
            observer_compute_class: r.observer_compute_class,
            signature_b64: r.signature_b64,
        }
    }
}

impl From<crate::db::models::ObservationDiversitySummaryRow> for ObservationDiversitySummaryView {
    fn from(r: crate::db::models::ObservationDiversitySummaryRow) -> Self {
        Self {
            subject_cid: r.subject_cid,
            observation_kind: r.observation_kind,
            distinct_agents: r.distinct_agents,
            distinct_households: r.distinct_households,
            distinct_collectives: r.distinct_collectives,
            distinct_regions: r.distinct_regions,
            distinct_archetypes: r.distinct_archetypes,
            distinct_compute_classes: r.distinct_compute_classes,
            total_count: r.total_count,
            first_observed_at: r.first_observed_at,
            last_observed_at: r.last_observed_at,
        }
    }
}

// ============================================================================
// Elohim Reputation Profile View
// Source of truth: computed aggregation over the mishpat DNA DHT outcome graph.
// ============================================================================

/// Build an ElohimReputationProfileView from a computed aggregation result.
pub fn elohim_reputation_profile_view_from_result(
    elohim_id: String,
    window_start: String,
    window_end: String,
    r: crate::db::elohim_reputation::ReputationResult,
) -> ElohimReputationProfileView {
    use serde_json::Map;
    let grounds_map: Map<String, serde_json::Value> = r
        .challenges_by_grounds
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::Number(v.into())))
        .collect();
    let verdicts_map: Map<String, serde_json::Value> = r
        .outcomes_by_verdict
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::Number(v.into())))
        .collect();
    ElohimReputationProfileView {
        elohim_id,
        window_start,
        window_end,
        current_substance_cid: r.current_substance_cid,
        total_decisions: r.total_decisions,
        challenged_count: r.challenged_count,
        upheld_count: r.upheld_count,
        dismissed_count: r.dismissed_count,
        superseded_count: r.superseded_count,
        pending_count: r.pending_count,
        challenges_by_grounds: JsonVal(serde_json::Value::Object(grounds_map)),
        outcomes_by_verdict: JsonVal(serde_json::Value::Object(verdicts_map)),
    }
}

// ============================================================================
// Blob Response View
// ============================================================================

/// Build a PutBlobResponseView from an existing ShardManifest plus optional BLAKE3 hash.
pub fn put_blob_response_view_from_manifest(
    m: crate::sharding::ShardManifest,
    blake3_hash: Option<String>,
) -> PutBlobResponseView {
    PutBlobResponseView {
        blob_hash: m.blob_hash,
        total_size: m.total_size,
        mime_type: m.mime_type,
        encoding: m.encoding,
        data_shards: m.data_shards,
        total_shards: m.total_shards,
        shard_size: m.shard_size,
        shard_hashes: m.shard_hashes,
        reach: m.reach,
        author_id: m.author_id,
        created_at: m.created_at,
        verified_at: m.verified_at,
        blake3_hash,
    }
}
