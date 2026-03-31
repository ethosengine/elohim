//! Custodians API controller
//!
//! Routes: `/api/v1/custodians[/metrics[/{id}|/alerts|/{id}/recommendations]]`
//!         `/api/v1/custodians/protection[/trust-graph|/geographic|/redundancy|/summary]`
//!
//! Delegates to `db::custodian_metrics` for persistence.  Protection endpoints
//! return synthesised views assembled from local metrics data (no separate DB
//! table required).

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::custodian_metrics::{list_metrics, list_unhealthy_metrics, CustodianMetricsQuery};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response::{self, from_option};
use crate::views::{
    CustodianAlertView, CustodianMetricsView, CustodianNodeView, CustodianRecommendationView,
    FamilyCommunityProtectionStatusView, RegionalPresenceView, ReportCustodianMetricsInputView,
    TrustRelationshipView,
};

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Route dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/custodians*` requests.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // GET /api/v1/custodians/metrics
        (&Method::GET, "metrics") => handle_list_metrics(req, pool, ctx).await,

        // POST /api/v1/custodians/metrics
        (&Method::POST, "metrics") => handle_report_metrics(req, pool, ctx).await,

        // GET /api/v1/custodians/metrics/alerts
        (&Method::GET, "metrics/alerts") => handle_alerts(pool, ctx).await,

        // GET /api/v1/custodians/metrics/{id}/recommendations
        (&Method::GET, p) if p.starts_with("metrics/") && p.ends_with("/recommendations") => {
            let id = p
                .strip_prefix("metrics/")
                .unwrap()
                .strip_suffix("/recommendations")
                .unwrap();
            handle_recommendations(id, pool, ctx).await
        }

        // GET /api/v1/custodians/metrics/{id}
        (&Method::GET, p) if p.starts_with("metrics/") => {
            let id = p.strip_prefix("metrics/").unwrap();
            handle_get_metrics(id, pool, ctx).await
        }

        // GET /api/v1/custodians/protection/*
        (&Method::GET, p) if p.starts_with("protection") => handle_protection(p, pool, ctx).await,

        _ => Ok(response::not_found(&format!(
            "Unknown custodians route: /api/v1/custodians/{}",
            path
        ))),
    }
}

// ---------------------------------------------------------------------------
// Handler implementations
// ---------------------------------------------------------------------------

/// GET /api/v1/custodians/metrics[?rank=health|speed|reputation&available=true]
async fn handle_list_metrics(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let query_str = req.uri().query().unwrap_or("");
    let query: CustodianMetricsQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

    let mut conn = get_conn(pool)?;
    let rows = list_metrics(&mut conn, ctx, &query)?;

    let mut views: Vec<CustodianMetricsView> = rows.into_iter().map(Into::into).collect();

    // Sort in Rust — metric groups live inside JSON blobs so SQLite cannot
    // order by them directly.
    use crate::db::custodian_metrics::MetricsRank;
    match query.rank.unwrap_or_default() {
        MetricsRank::Health => {
            views.sort_by(|a, b| {
                b.health
                    .uptime_percent
                    .partial_cmp(&a.health.uptime_percent)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        MetricsRank::Speed => {
            views.sort_by(|a, b| {
                a.health
                    .response_time_p50_ms
                    .partial_cmp(&b.health.response_time_p50_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        MetricsRank::Reputation => {
            views.sort_by(|a, b| {
                b.reputation
                    .reputation_score
                    .partial_cmp(&a.reputation.reputation_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }

    // Optional availability filter.
    if query.available == Some(true) {
        views.retain(|v| v.health.availability);
    }

    Ok(response::ok(&views))
}

/// GET /api/v1/custodians/metrics/{id}
async fn handle_get_metrics(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let row = crate::db::custodian_metrics::get_metrics_by_id(&mut conn, id, ctx)?;
    Ok(from_option(
        Ok(row.map(CustodianMetricsView::from)),
        &format!("custodian metrics not found: {}", id),
    ))
}

/// GET /api/v1/custodians/metrics/alerts
///
/// Returns warning/critical alerts derived from stored health metrics.
async fn handle_alerts(
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let rows = list_unhealthy_metrics(&mut conn, ctx)?;

    let mut alerts: Vec<CustodianAlertView> = Vec::new();

    for row in rows {
        let view = CustodianMetricsView::from(row);

        if !view.health.availability {
            alerts.push(CustodianAlertView {
                custodian_id: view.custodian_id.clone(),
                severity: "critical".into(),
                category: "availability".into(),
                message: format!("Custodian {} is currently unavailable", view.custodian_id),
                suggestion: Some(
                    "Check node connectivity and restart the elohim-node service.".into(),
                ),
            });
        } else if view.health.uptime_percent < 95.0 {
            alerts.push(CustodianAlertView {
                custodian_id: view.custodian_id.clone(),
                severity: "warning".into(),
                category: "uptime".into(),
                message: format!(
                    "Custodian {} uptime is {:.1}% (below 95% threshold)",
                    view.custodian_id, view.health.uptime_percent
                ),
                suggestion: Some("Review system logs for crash or restart cycles.".into()),
            });
        }

        if view.health.error_rate > 0.05 {
            alerts.push(CustodianAlertView {
                custodian_id: view.custodian_id.clone(),
                severity: if view.health.error_rate > 0.15 {
                    "critical"
                } else {
                    "warning"
                }
                .into(),
                category: "error_rate".into(),
                message: format!(
                    "Custodian {} error rate is {:.1}% (threshold: 5%)",
                    view.custodian_id,
                    view.health.error_rate * 100.0
                ),
                suggestion: Some(
                    "Inspect conductor logs for validation failures or storage errors.".into(),
                ),
            });
        }

        if view.storage.utilization_percent > 90.0 {
            alerts.push(CustodianAlertView {
                custodian_id: view.custodian_id.clone(),
                severity: "warning".into(),
                category: "storage".into(),
                message: format!(
                    "Custodian {} storage utilisation is {:.1}%",
                    view.custodian_id, view.storage.utilization_percent
                ),
                suggestion: Some("Expand storage capacity or reduce committed data volume.".into()),
            });
        }
    }

    Ok(response::ok(&alerts))
}

/// GET /api/v1/custodians/metrics/{id}/recommendations
///
/// Returns placeholder improvement recommendations for a single custodian.
/// Real recommendations will be computed by the service layer once the
/// economic and commitment models are fully implemented.
async fn handle_recommendations(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let row_opt = crate::db::custodian_metrics::get_metrics_by_id(&mut conn, id, ctx)?;

    let view = match row_opt {
        Some(r) => CustodianMetricsView::from(r),
        None => {
            return Ok(response::not_found(&format!(
                "custodian metrics not found: {}",
                id
            )))
        }
    };

    let mut recs: Vec<CustodianRecommendationView> = Vec::new();

    if view.storage.utilization_percent < 50.0 {
        recs.push(CustodianRecommendationView {
            custodian_id: id.to_string(),
            category: "storage".into(),
            opportunity: "Accept additional stewardship commitments — storage is under-utilised"
                .into(),
            potential_revenue: Some(
                (view.storage.free_bytes as f64 / 1_073_741_824.0) * view.economic.price_per_gb,
            ),
        });
    }

    if view.reputation.reliability_rating < 0.8 {
        recs.push(CustodianRecommendationView {
            custodian_id: id.to_string(),
            category: "reputation".into(),
            opportunity: "Improve reliability rating by maintaining >99% uptime for 30 days".into(),
            potential_revenue: None,
        });
    }

    if view.bandwidth.utilization_percent < 40.0 {
        recs.push(CustodianRecommendationView {
            custodian_id: id.to_string(),
            category: "bandwidth".into(),
            opportunity: "Offer higher-bandwidth tiers — declared capacity is under-used".into(),
            potential_revenue: None,
        });
    }

    Ok(response::ok(&recs))
}

/// POST /api/v1/custodians/metrics
async fn handle_report_metrics(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: ReportCustodianMetricsInputView = parse_body(req).await?;
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let upsert = input.into_upsert(&ctx.h_app_id, now_ms);
    let mut conn = get_conn(pool)?;
    let row = crate::db::custodian_metrics::upsert_metrics(&mut conn, upsert)?;
    Ok(response::ok(&CustodianMetricsView::from(row)))
}

/// GET /api/v1/custodians/protection[/trust-graph|/geographic|/redundancy|/summary]
///
/// All sub-paths return synthesised aggregation views assembled from local
/// custodian_metrics rows.  No separate DB table is required.
async fn handle_protection(
    sub_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let query = CustodianMetricsQuery::default();
    let rows = list_metrics(&mut conn, ctx, &query)?;
    let all_metrics: Vec<CustodianMetricsView> = rows.into_iter().map(Into::into).collect();

    match sub_path {
        "protection/trust-graph" => {
            let trust_graph = build_trust_graph(&all_metrics);
            Ok(response::ok(&trust_graph))
        }
        "protection/geographic" => {
            let regions = build_geographic_distribution(&all_metrics);
            Ok(response::ok(&regions))
        }
        "protection/redundancy" => {
            #[derive(serde::Serialize)]
            #[serde(rename_all = "camelCase")]
            struct RedundancySummary {
                total_custodians: usize,
                available_custodians: usize,
                total_storage_bytes: i64,
                average_uptime_percent: f64,
            }

            let available = all_metrics.iter().filter(|m| m.health.availability).count();
            let total_storage: i64 = all_metrics
                .iter()
                .map(|m| m.storage.total_capacity_bytes)
                .sum();
            let avg_uptime = if all_metrics.is_empty() {
                0.0
            } else {
                all_metrics
                    .iter()
                    .map(|m| m.health.uptime_percent)
                    .sum::<f64>()
                    / all_metrics.len() as f64
            };

            Ok(response::ok(&RedundancySummary {
                total_custodians: all_metrics.len(),
                available_custodians: available,
                total_storage_bytes: total_storage,
                average_uptime_percent: avg_uptime,
            }))
        }
        // protection or protection/summary — return full status
        _ => {
            let status = build_protection_status(all_metrics);
            Ok(response::ok(&status))
        }
    }
}

// ---------------------------------------------------------------------------
// Aggregation helpers
// ---------------------------------------------------------------------------

fn build_trust_graph(metrics: &[CustodianMetricsView]) -> Vec<TrustRelationshipView> {
    // Stub: synthesise pairwise trust edges from reputation scores.
    // Full implementation will use the relationship graph from the imagodei DNA.
    metrics
        .iter()
        .flat_map(|a| {
            metrics.iter().filter_map(move |b| {
                if a.custodian_id == b.custodian_id {
                    return None;
                }
                let trust_score =
                    (a.reputation.reliability_rating + b.reputation.reliability_rating) / 2.0;
                let strength = if trust_score > 0.8 {
                    "strong"
                } else if trust_score > 0.5 {
                    "moderate"
                } else {
                    "weak"
                };
                Some(TrustRelationshipView {
                    from: a.custodian_id.clone(),
                    to: b.custodian_id.clone(),
                    relationship_type: "community-peer".into(),
                    trust_score,
                    depth: 1,
                    strength: strength.into(),
                })
            })
        })
        .collect()
}

fn build_geographic_distribution(metrics: &[CustodianMetricsView]) -> Vec<RegionalPresenceView> {
    // Stub: no geographic fields in CustodianMetricsView; real implementation
    // will pull from CustodianNodeView / commitment data.
    if metrics.is_empty() {
        return Vec::new();
    }
    vec![RegionalPresenceView {
        region: "unknown".into(),
        custodian_count: metrics.len() as u32,
        data_shards: metrics.len() as u32 * 7,
        redundancy: 4,
        risk_factors: vec![],
    }]
}

fn build_protection_status(
    all_metrics: Vec<CustodianMetricsView>,
) -> FamilyCommunityProtectionStatusView {
    let available_count = all_metrics.iter().filter(|m| m.health.availability).count();
    let total = all_metrics.len();

    let protection_level = if available_count >= 4 {
        "highly-protected"
    } else if available_count >= 2 {
        "protected"
    } else {
        "vulnerable"
    };

    let custodians: Vec<CustodianNodeView> = all_metrics
        .iter()
        .map(|m| CustodianNodeView {
            id: m.custodian_id.clone(),
            name: m.custodian_id.clone(),
            custodian_type: "community".into(),
            location_region: None,
            location_country: None,
            data_stored_total_gb: m.storage.used_bytes as f64 / 1_073_741_824.0,
            data_stored_shard_count: 0,
            data_stored_redundancy_level: m.tier,
            health_up_percent: m.health.uptime_percent,
            health_last_heartbeat: m.last_updated_at.to_string(),
            health_response_time_ms: m.health.response_time_p50_ms,
            commitment_id: String::new(),
            commitment_status: if m.health.availability {
                "active"
            } else {
                "breached"
            }
            .into(),
            commitment_start_date: m.collected_at.to_string(),
            commitment_expiry_date: String::new(),
            commitment_renewal_status: "manual".into(),
            trust_level: m.reputation.reputation_score,
            relationship: "community".into(),
        })
        .collect();

    let geographic_regions = build_geographic_distribution(&all_metrics);
    let trust_graph = build_trust_graph(&all_metrics);

    FamilyCommunityProtectionStatusView {
        redundancy_strategy: "erasure_coded".into(),
        redundancy_factor: if total > 0 {
            available_count as f64 / total as f64
        } else {
            0.0
        },
        recovery_threshold: 4,
        custodians,
        total_custodians: total as u32,
        geographic_regions,
        geographic_risk_profile: if total >= 3 {
            "distributed"
        } else {
            "centralized"
        }
        .into(),
        trust_graph,
        protection_level: protection_level.into(),
        estimated_recovery_time: "< 1 hour".into(),
        last_verification: String::new(),
        verification_status: "pending".into(),
    }
}
