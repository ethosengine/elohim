//! Planet-Scale Governance Dashboard Service (Sprint 8)
//!
//! Composes existing data sources into a single read-only dashboard response.
//! No new tables. No migrations. Pure aggregation over:
//!   - places (db projection of Mishpat DHT)
//!   - hazards (operational, Category C)
//!   - risk_alerts (operational, Category C)
//!   - vulnerability cache (15-min TTL)
//!   - weather cache (1-hour TTL)
//!
//! Source of truth: each sub-system owns its own layer.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::db::context::AppContext;
use crate::db::models::{Hazard, RiskAlert};
use crate::db::places::PlaceQuery;
use crate::db::DbPool;
use crate::error::StorageError;
use crate::services::spatial::CarryingCapacity;
use crate::services::vulnerability::RiskTier;
use crate::services::weather::WeatherRisk;
use crate::views::{
    AlertEntrySummaryView, CapacitySummaryView, DashboardSummaryView, HazardEntrySummaryView,
    JsonVal, PlaceDashboardEntry, RiskTierDistribution, RouteEntrySummaryView,
    SpatialDashboardView, VulnerabilitySummaryView, WeatherEntrySummaryView,
};

// ============================================================================
// Query + Flags
// ============================================================================

/// Which enrichment layers to include in the dashboard response.
/// Defaults to all when `include` query param is absent or empty.
pub struct IncludeFlags {
    pub capacity: bool,
    pub hazards: bool,
    pub vulnerability: bool,
    pub weather: bool,
    pub alerts: bool,
    pub routes: bool,
}

impl IncludeFlags {
    /// All enrichment layers enabled.
    pub fn all() -> Self {
        Self {
            capacity: true,
            hazards: true,
            vulnerability: true,
            weather: true,
            alerts: true,
            routes: true,
        }
    }

    /// Parse a comma-separated include string (e.g. "hazards,weather").
    /// Returns `all()` when the string is None or empty.
    pub fn parse(include_str: Option<&str>) -> Self {
        match include_str {
            None | Some("") => Self::all(),
            Some(s) => {
                let parts: HashSet<&str> = s.split(',').map(str::trim).collect();
                Self {
                    capacity: parts.contains("capacity"),
                    hazards: parts.contains("hazards"),
                    vulnerability: parts.contains("vulnerability"),
                    weather: parts.contains("weather"),
                    alerts: parts.contains("alerts"),
                    routes: parts.contains("routes"),
                }
            }
        }
    }
}

/// URL query parameters for the spatial dashboard endpoint.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardQuery {
    pub constitutional_layer: Option<String>,
    pub h3_index: Option<String>,
    pub parent_place_id: Option<String>,
    pub status: Option<String>,
    /// Comma-separated enrichment layers: "capacity,hazards,weather,vulnerability,alerts,routes"
    /// Absent or empty → all layers included.
    pub include: Option<String>,
    pub limit: Option<i64>,
}

// ============================================================================
// Main Entry Point
// ============================================================================

/// Build the full spatial dashboard: places enriched with optional layers.
///
/// Uses batch queries for hazards and alerts (one query per layer, not N+1).
/// Vulnerability and weather are served from their respective caches.
pub async fn build_dashboard(
    pool: &DbPool,
    ctx: &AppContext,
    query: &DashboardQuery,
) -> Result<SpatialDashboardView, StorageError> {
    let flags = IncludeFlags::parse(query.include.as_deref());

    // 1. Build PlaceQuery from dashboard filters
    let place_query = PlaceQuery {
        constitutional_layer: query.constitutional_layer.clone(),
        h3_index: query.h3_index.clone(),
        parent_place_id: query.parent_place_id.clone(),
        status: Some(query.status.clone().unwrap_or_else(|| "active".to_string())),
        limit: Some(query.limit.unwrap_or(200)),
        ..Default::default()
    };

    // 2. Load places
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

    let places = crate::db::places::list_places(&mut conn, ctx, &place_query)?;
    drop(conn);

    if places.is_empty() {
        let queried_at = crate::db::models::current_timestamp();
        return Ok(SpatialDashboardView {
            places: vec![],
            summary: DashboardSummaryView {
                total_places: 0,
                places_by_risk_tier: RiskTierDistribution {
                    low: 0,
                    moderate: 0,
                    elevated: 0,
                    critical: 0,
                    unknown: 0,
                },
                total_active_hazards: 0,
                total_active_alerts: 0,
                total_unacknowledged_alerts: 0,
                worst_overall_risk: "unknown".to_string(),
            },
            queried_at,
        });
    }

    // 3. Collect place IDs for batch queries
    let place_id_strings: Vec<String> = places.iter().map(|p| p.id.clone()).collect();
    let place_ids: Vec<&str> = place_id_strings.iter().map(String::as_str).collect();

    // 4. Batch load hazards (one query)
    let hazards_by_place: HashMap<String, Vec<Hazard>> = if flags.hazards || flags.vulnerability {
        let mut batch_conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;
        let hazards = crate::db::hazards::list_hazards_for_places(&mut batch_conn, ctx, &place_ids)
            .unwrap_or_default();
        group_by_place(hazards, |h| h.place_id.as_str())
    } else {
        HashMap::new()
    };

    // 5. Batch load alerts (one query)
    let alerts_by_place: HashMap<String, Vec<RiskAlert>> = if flags.alerts {
        let mut batch_conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;
        let alerts =
            crate::db::risk_alerts::list_alerts_for_places(&mut batch_conn, ctx, &place_ids)
                .unwrap_or_default();
        group_by_place(alerts, |a| a.place_id.as_str())
    } else {
        HashMap::new()
    };

    // 6. Build dashboard entries (with async enrichment for vulnerability + weather)
    let mut entries: Vec<PlaceDashboardEntry> = Vec::with_capacity(places.len());

    for place in &places {
        let place_hazards = hazards_by_place.get(&place.id);
        let place_alerts = alerts_by_place.get(&place.id);

        // Capacity summary from carrying_capacity_json
        let capacity = if flags.capacity {
            Some(compute_capacity_summary(&place.carrying_capacity_json))
        } else {
            None
        };

        // Hazard summary from batch-loaded data
        let hazards_summary = if flags.hazards {
            Some(compute_hazard_entry_summary(place_hazards))
        } else {
            None
        };

        // Vulnerability — hits cache, async
        let vulnerability = if flags.vulnerability {
            let v =
                crate::services::vulnerability::compute_vulnerability(pool, ctx, &place.id).await;
            Some(VulnerabilitySummaryView {
                overall_score: v.overall_vulnerability,
                risk_tier: risk_tier_str(&v.risk_tier),
                preparation_status: preparation_status_str(&v.preparation_status),
            })
        } else {
            None
        };

        // Weather — hits cache, async
        let weather = if flags.weather {
            let forecast = crate::services::weather::get_forecast(
                &place.id,
                place.centroid_lat,
                place.centroid_lng,
            )
            .await;
            Some(WeatherEntrySummaryView {
                risk_level: weather_risk_str(&forecast.overall_risk),
                temperature_c: forecast.current.temperature_c,
                precipitation_mm: forecast.current.precipitation_mm,
                wind_speed_kmh: forecast.current.wind_speed_kmh,
            })
        } else {
            None
        };

        // Alert summary from batch-loaded data
        let alerts_summary = if flags.alerts {
            Some(compute_alert_entry_summary(place_alerts))
        } else {
            None
        };

        // Routes — MVP stub
        let routes = if flags.routes {
            Some(RouteEntrySummaryView {
                active_route_count: 0,
                total_distance_km: 0.0,
            })
        } else {
            None
        };

        let geometry_json: Option<JsonVal> = {
            let parsed: Option<serde_json::Value> = serde_json::from_str(&place.geometry_json).ok();
            parsed.map(JsonVal)
        };

        entries.push(PlaceDashboardEntry {
            id: place.id.clone(),
            name: place.name.clone(),
            place_type: place.place_type.clone(),
            constitutional_layer: place.constitutional_layer.clone(),
            h3_index: place.h3_index.clone(),
            centroid_lat: place.centroid_lat,
            centroid_lng: place.centroid_lng,
            geometry_json,
            status: place.status.clone(),
            parent_place_id: place.parent_place_id.clone(),
            capacity,
            hazards: hazards_summary,
            vulnerability,
            weather,
            alerts: alerts_summary,
            routes,
        });
    }

    // 7. Accumulate summary statistics
    let queried_at = crate::db::models::current_timestamp();
    let summary = build_summary(&entries);

    Ok(SpatialDashboardView {
        places: entries,
        summary,
        queried_at,
    })
}

// ============================================================================
// Summary Accumulation
// ============================================================================

fn build_summary(entries: &[PlaceDashboardEntry]) -> DashboardSummaryView {
    let total_places = entries.len() as u32;

    let mut risk_dist = RiskTierDistribution {
        low: 0,
        moderate: 0,
        elevated: 0,
        critical: 0,
        unknown: 0,
    };

    let mut total_active_hazards = 0u32;
    let mut total_active_alerts = 0u32;
    let mut total_unacknowledged_alerts = 0u32;
    let mut worst_tier_rank: u8 = 0;

    for entry in entries {
        // Risk tier counts
        if let Some(ref v) = entry.vulnerability {
            match v.risk_tier.as_str() {
                "low" => {
                    risk_dist.low += 1;
                    worst_tier_rank = worst_tier_rank.max(1);
                }
                "moderate" => {
                    risk_dist.moderate += 1;
                    worst_tier_rank = worst_tier_rank.max(2);
                }
                "elevated" => {
                    risk_dist.elevated += 1;
                    worst_tier_rank = worst_tier_rank.max(3);
                }
                "critical" => {
                    risk_dist.critical += 1;
                    worst_tier_rank = worst_tier_rank.max(4);
                }
                _ => {
                    risk_dist.unknown += 1;
                }
            }
        } else {
            risk_dist.unknown += 1;
        }

        // Hazard totals
        if let Some(ref h) = entry.hazards {
            total_active_hazards += h.active_count;
        }

        // Alert totals
        if let Some(ref a) = entry.alerts {
            total_active_alerts += a.active_count;
            total_unacknowledged_alerts += a.unacknowledged_count;
        }
    }

    let worst_overall_risk = match worst_tier_rank {
        0 => "unknown",
        1 => "low",
        2 => "moderate",
        3 => "elevated",
        _ => "critical",
    }
    .to_string();

    DashboardSummaryView {
        total_places,
        places_by_risk_tier: risk_dist,
        total_active_hazards,
        total_active_alerts,
        total_unacknowledged_alerts,
        worst_overall_risk,
    }
}

// ============================================================================
// Private Helpers
// ============================================================================

/// Group a Vec<T> into HashMap<place_id, Vec<T>> using a key extractor.
fn group_by_place<T>(items: Vec<T>, get_place_id: impl Fn(&T) -> &str) -> HashMap<String, Vec<T>> {
    let mut map: HashMap<String, Vec<T>> = HashMap::new();
    for item in items {
        let key = get_place_id(&item).to_string();
        map.entry(key).or_default().push(item);
    }
    map
}

/// Summarise carrying capacity from the JSON column.
/// Returns a zero-count summary when the JSON is absent or unparseable.
fn compute_capacity_summary(carrying_capacity_json: &str) -> CapacitySummaryView {
    let capacities: Vec<CarryingCapacity> =
        serde_json::from_str(carrying_capacity_json).unwrap_or_default();

    if capacities.is_empty() {
        return CapacitySummaryView {
            resource_count: 0,
            worst_utilization: 0.0,
            worst_category: String::new(),
            trigger_governance: false,
        };
    }

    let worst = capacities
        .iter()
        .max_by(|a, b| {
            a.current_utilization
                .partial_cmp(&b.current_utilization)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap(); // safe: non-empty

    CapacitySummaryView {
        resource_count: capacities.len() as u32,
        worst_utilization: worst.current_utilization,
        worst_category: worst.resource_category.clone(),
        trigger_governance: worst.current_utilization > 0.85,
    }
}

/// Summarise hazards for a single place tile.
fn compute_hazard_entry_summary(hazards: Option<&Vec<Hazard>>) -> HazardEntrySummaryView {
    let empty = vec![];
    let hazards = hazards.unwrap_or(&empty);

    if hazards.is_empty() {
        return HazardEntrySummaryView {
            active_count: 0,
            worst_severity: "none".to_string(),
            nearest_onset_hours: None,
            types: vec![],
        };
    }

    let worst_rank = hazards
        .iter()
        .map(|h| severity_rank(&h.severity))
        .max()
        .unwrap_or(0);

    let worst_severity = rank_to_severity_str(worst_rank).to_string();

    // Nearest projected onset across all hazards
    let now_dt = chrono::Utc::now();
    let nearest_onset_hours = hazards
        .iter()
        .filter_map(|h| h.projected_onset.as_deref())
        .filter_map(|onset| {
            chrono::DateTime::parse_from_rfc3339(onset)
                .ok()
                .map(|dt| (dt.with_timezone(&chrono::Utc) - now_dt).num_minutes() as f64 / 60.0)
                .filter(|&h| h >= 0.0)
        })
        .fold(f64::INFINITY, f64::min);

    let nearest_onset_hours = if nearest_onset_hours.is_finite() {
        Some(nearest_onset_hours)
    } else {
        None
    };

    // Unique hazard types
    let mut types: Vec<String> = hazards
        .iter()
        .map(|h| h.hazard_type.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    types.sort();

    HazardEntrySummaryView {
        active_count: hazards.len() as u32,
        worst_severity,
        nearest_onset_hours,
        types,
    }
}

/// Summarise risk alerts for a single place tile.
fn compute_alert_entry_summary(alerts: Option<&Vec<RiskAlert>>) -> AlertEntrySummaryView {
    let empty = vec![];
    let alerts = alerts.unwrap_or(&empty);

    if alerts.is_empty() {
        return AlertEntrySummaryView {
            active_count: 0,
            worst_severity: "none".to_string(),
            unacknowledged_count: 0,
            types: vec![],
        };
    }

    let worst_rank = alerts
        .iter()
        .map(|a| severity_rank(&a.severity))
        .max()
        .unwrap_or(0);

    let worst_severity = rank_to_severity_str(worst_rank).to_string();

    let unacknowledged_count = alerts
        .iter()
        .filter(|a| a.acknowledged_by.is_none())
        .count() as u32;

    let mut types: Vec<String> = alerts
        .iter()
        .map(|a| a.alert_type.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    types.sort();

    AlertEntrySummaryView {
        active_count: alerts.len() as u32,
        worst_severity,
        unacknowledged_count,
        types,
    }
}

/// Maps severity string to a numeric rank for max-finding.
fn severity_rank(severity: &str) -> u8 {
    match severity {
        "emergency" | "critical" => 4,
        "warning" => 3,
        "advisory" => 2,
        "watch" | "info" => 1,
        _ => 0,
    }
}

/// Maps a numeric rank back to the canonical severity string.
fn rank_to_severity_str(rank: u8) -> &'static str {
    match rank {
        4 => "emergency",
        3 => "warning",
        2 => "advisory",
        1 => "watch",
        _ => "none",
    }
}

fn risk_tier_str(tier: &RiskTier) -> String {
    match tier {
        RiskTier::Low => "low",
        RiskTier::Moderate => "moderate",
        RiskTier::Elevated => "elevated",
        RiskTier::Critical => "critical",
    }
    .to_string()
}

fn preparation_status_str(status: &crate::services::vulnerability::PreparationStatus) -> String {
    use crate::services::vulnerability::PreparationStatus;
    match status {
        PreparationStatus::Clear => "clear",
        PreparationStatus::Monitoring => "monitoring",
        PreparationStatus::Preparing => "preparing",
        PreparationStatus::Responding => "responding",
        PreparationStatus::Recovering => "recovering",
    }
    .to_string()
}

fn weather_risk_str(risk: &WeatherRisk) -> String {
    match risk {
        WeatherRisk::None => "none",
        WeatherRisk::Advisory => "advisory",
        WeatherRisk::Warning => "warning",
        WeatherRisk::Severe => "severe",
    }
    .to_string()
}
