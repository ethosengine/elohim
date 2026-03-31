//! Risk Alert Evaluation Service
//!
//! Evaluates a VulnerabilityAssessmentView against threshold conditions
//! and generates RiskAlert records for newly-triggered thresholds.
//! Existing active alerts are deduplicated to prevent duplicates.

use diesel::SqliteConnection;
use uuid::Uuid;

use crate::db::context::AppContext;
use crate::db::models::{current_timestamp, NewRiskAlert};
use crate::error::StorageError;
use crate::services::vulnerability::VulnerabilityAssessmentView;

/// Evaluate threshold conditions and generate risk alerts as needed.
///
/// For each threshold condition:
/// 1. Check if an active alert already exists (dedup via find_active_alert)
/// 2. If not, create a new alert via create_risk_alert
///
/// Returns the list of alert IDs that were newly created.
pub fn evaluate_and_generate_alerts(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    place_id: &str,
    assessment: &VulnerabilityAssessmentView,
) -> Result<Vec<String>, StorageError> {
    let mut created_ids: Vec<String> = Vec::new();
    let now = current_timestamp();

    // ------------------------------------------------------------------
    // Threshold 1: Capacity stress > 0.9 → capacity-threshold (critical)
    // ------------------------------------------------------------------
    if assessment.capacity_stress > 0.9 {
        let existing = crate::db::risk_alerts::find_active_alert(
            conn,
            ctx,
            place_id,
            "capacity-threshold",
            None,
        )?;
        if existing.is_none() {
            let id = Uuid::new_v4().to_string();
            let alert = NewRiskAlert {
                id: id.clone(),
                h_app_id: ctx.h_app_id().to_string(),
                place_id: place_id.to_string(),
                alert_type: "capacity-threshold".to_string(),
                severity: "critical".to_string(),
                title: "Carrying capacity critically stressed".to_string(),
                description: format!(
                    "Resource utilization at {:.0}% — exceeds 90% threshold",
                    assessment.capacity_stress * 100.0
                ),
                trigger_hazard_id: None,
                trigger_data_json: serde_json::json!({
                    "capacityStress": assessment.capacity_stress
                })
                .to_string(),
                triggered_at: now.clone(),
                lead_time_hours: None,
                expires_at: None,
                status: "active".to_string(),
                metadata_json: "{}".to_string(),
                created_at: now.clone(),
                updated_at: now.clone(),
            };
            crate::db::risk_alerts::create_risk_alert(conn, ctx, alert)?;
            created_ids.push(id);
        }
    }

    // ------------------------------------------------------------------
    // Threshold 2: Hazard approaching within 72h → hazard-approaching
    // ------------------------------------------------------------------
    for hazard in &assessment.active_hazards {
        if let Some(lead_h) = hazard.lead_time_hours {
            if lead_h <= 72.0 {
                let existing = crate::db::risk_alerts::find_active_alert(
                    conn,
                    ctx,
                    place_id,
                    "hazard-approaching",
                    Some(&hazard.id),
                )?;
                if existing.is_none() {
                    let severity = hazard_severity_to_alert_severity(&hazard.severity);
                    let id = Uuid::new_v4().to_string();
                    let alert = NewRiskAlert {
                        id: id.clone(),
                        h_app_id: ctx.h_app_id().to_string(),
                        place_id: place_id.to_string(),
                        alert_type: "hazard-approaching".to_string(),
                        severity,
                        title: format!("Hazard approaching: {}", hazard.title),
                        description: format!(
                            "{} expected within {:.1} hours",
                            hazard.hazard_type, lead_h
                        ),
                        trigger_hazard_id: Some(hazard.id.clone()),
                        trigger_data_json: serde_json::json!({
                            "hazardId": hazard.id,
                            "hazardType": hazard.hazard_type,
                            "leadTimeHours": lead_h
                        })
                        .to_string(),
                        triggered_at: now.clone(),
                        lead_time_hours: Some(lead_h),
                        expires_at: None,
                        status: "active".to_string(),
                        metadata_json: "{}".to_string(),
                        created_at: now.clone(),
                        updated_at: now.clone(),
                    };
                    crate::db::risk_alerts::create_risk_alert(conn, ctx, alert)?;
                    created_ids.push(id);
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Threshold 3: Overall vulnerability > 0.8 → vulnerability-critical
    // ------------------------------------------------------------------
    if assessment.overall_vulnerability > 0.8 {
        let existing = crate::db::risk_alerts::find_active_alert(
            conn,
            ctx,
            place_id,
            "vulnerability-critical",
            None,
        )?;
        if existing.is_none() {
            let id = Uuid::new_v4().to_string();
            let alert = NewRiskAlert {
                id: id.clone(),
                h_app_id: ctx.h_app_id().to_string(),
                place_id: place_id.to_string(),
                alert_type: "vulnerability-critical".to_string(),
                severity: "critical".to_string(),
                title: "Place vulnerability at critical level".to_string(),
                description: format!(
                    "Overall vulnerability score {:.2} exceeds critical threshold (0.8)",
                    assessment.overall_vulnerability
                ),
                trigger_hazard_id: None,
                trigger_data_json: serde_json::json!({
                    "overallVulnerability": assessment.overall_vulnerability
                })
                .to_string(),
                triggered_at: now.clone(),
                lead_time_hours: None,
                expires_at: None,
                status: "active".to_string(),
                metadata_json: "{}".to_string(),
                created_at: now.clone(),
                updated_at: now.clone(),
            };
            crate::db::risk_alerts::create_risk_alert(conn, ctx, alert)?;
            created_ids.push(id);
        }
    }

    // ------------------------------------------------------------------
    // Threshold 4: Multiple concurrent hazards → multi-hazard-overlap
    // ------------------------------------------------------------------
    if assessment.active_hazard_count > 1 {
        let existing = crate::db::risk_alerts::find_active_alert(
            conn,
            ctx,
            place_id,
            "multi-hazard-overlap",
            None,
        )?;
        if existing.is_none() {
            let id = Uuid::new_v4().to_string();
            let alert = NewRiskAlert {
                id: id.clone(),
                h_app_id: ctx.h_app_id().to_string(),
                place_id: place_id.to_string(),
                alert_type: "multi-hazard-overlap".to_string(),
                severity: "warning".to_string(),
                title: "Multiple concurrent hazards".to_string(),
                description: format!(
                    "{} active hazards overlap at this location — compound risk elevated",
                    assessment.active_hazard_count
                ),
                trigger_hazard_id: None,
                trigger_data_json: serde_json::json!({
                    "activeHazardCount": assessment.active_hazard_count
                })
                .to_string(),
                triggered_at: now.clone(),
                lead_time_hours: None,
                expires_at: None,
                status: "active".to_string(),
                metadata_json: "{}".to_string(),
                created_at: now.clone(),
                updated_at: now.clone(),
            };
            crate::db::risk_alerts::create_risk_alert(conn, ctx, alert)?;
            created_ids.push(id);
        }
    }

    // ------------------------------------------------------------------
    // Threshold 5: Weather risk > 0.8 → weather-severe
    // ------------------------------------------------------------------
    if assessment.weather_risk > 0.8 {
        let existing =
            crate::db::risk_alerts::find_active_alert(conn, ctx, place_id, "weather-severe", None)?;
        if existing.is_none() {
            let id = Uuid::new_v4().to_string();
            let alert = NewRiskAlert {
                id: id.clone(),
                h_app_id: ctx.h_app_id().to_string(),
                place_id: place_id.to_string(),
                alert_type: "weather-severe".to_string(),
                severity: "warning".to_string(),
                title: "Severe weather conditions".to_string(),
                description: format!(
                    "Weather risk score {:.2} exceeds severe threshold (0.8) — halt operations, protect resources",
                    assessment.weather_risk
                ),
                trigger_hazard_id: None,
                trigger_data_json: serde_json::json!({
                    "weatherRisk": assessment.weather_risk
                })
                .to_string(),
                triggered_at: now.clone(),
                lead_time_hours: None,
                expires_at: None,
                status: "active".to_string(),
                metadata_json: "{}".to_string(),
                created_at: now.clone(),
                updated_at: now.clone(),
            };
            crate::db::risk_alerts::create_risk_alert(conn, ctx, alert)?;
            created_ids.push(id);
        }
    }

    Ok(created_ids)
}

// ============================================================================
// Helpers
// ============================================================================

fn hazard_severity_to_alert_severity(hazard_severity: &str) -> String {
    match hazard_severity {
        "emergency" => "critical".to_string(),
        "warning" => "warning".to_string(),
        "advisory" => "advisory".to_string(),
        "watch" => "watch".to_string(),
        _ => "warning".to_string(),
    }
}
