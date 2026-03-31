//! Risk Alert CRUD operations
//!
//! Generated alerts from vulnerability assessments.
//! Source of truth: SQLite (operational, Category C).
//! Reconstructable by re-running evaluate_and_generate_alerts on each Place.

use diesel::prelude::*;
use tracing::debug;

use super::context::AppContext;
use super::diesel_schema::risk_alerts;
use super::models::{current_timestamp, NewRiskAlert, RiskAlert};
use crate::error::StorageError;

// ============================================================================
// Query Types
// ============================================================================

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskAlertQuery {
    pub place_id: Option<String>,
    pub status: Option<String>,
    pub alert_type: Option<String>,
    pub limit: Option<i64>,
}

// ============================================================================
// CRUD Operations
// ============================================================================

/// Create a new risk alert
pub fn create_risk_alert(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    new: NewRiskAlert,
) -> Result<RiskAlert, StorageError> {
    debug!(id = %new.id, place_id = %new.place_id, alert_type = %new.alert_type, "Creating risk alert");

    diesel::insert_into(risk_alerts::table)
        .values(&new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to create risk alert: {}", e)))?;

    get_risk_alert_by_id(conn, ctx, &new.id)
}

/// Get risk alert by ID
pub fn get_risk_alert_by_id(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<RiskAlert, StorageError> {
    risk_alerts::table
        .filter(risk_alerts::id.eq(id))
        .filter(risk_alerts::h_app_id.eq(ctx.h_app_id()))
        .first(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                StorageError::NotFound(format!("RiskAlert not found: {}", id))
            }
            _ => StorageError::Internal(format!("Failed to fetch risk alert: {}", e)),
        })
}

/// List risk alerts with optional filters
pub fn list_risk_alerts(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &RiskAlertQuery,
) -> Result<Vec<RiskAlert>, StorageError> {
    let mut q = risk_alerts::table
        .filter(risk_alerts::h_app_id.eq(ctx.h_app_id()))
        .into_boxed();

    if let Some(ref place_id) = query.place_id {
        q = q.filter(risk_alerts::place_id.eq(place_id));
    }
    if let Some(ref status) = query.status {
        q = q.filter(risk_alerts::status.eq(status));
    }
    if let Some(ref alert_type) = query.alert_type {
        q = q.filter(risk_alerts::alert_type.eq(alert_type));
    }
    if let Some(limit) = query.limit {
        q = q.limit(limit);
    }

    q.order(risk_alerts::triggered_at.desc())
        .load::<RiskAlert>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list risk alerts: {}", e)))
}

/// Batch-load active alerts for multiple places in a single query.
/// Returns all matching alerts ordered by place_id (for grouping by caller).
pub fn list_alerts_for_places(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    place_ids: &[&str],
) -> Result<Vec<RiskAlert>, StorageError> {
    if place_ids.is_empty() {
        return Ok(vec![]);
    }

    risk_alerts::table
        .filter(risk_alerts::h_app_id.eq(ctx.h_app_id()))
        .filter(risk_alerts::place_id.eq_any(place_ids))
        .filter(risk_alerts::status.eq("active"))
        .order(risk_alerts::place_id.asc())
        .load::<RiskAlert>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to batch-load alerts: {}", e)))
}

/// Find an active alert by place, type, and optional trigger hazard (for deduplication)
pub fn find_active_alert(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    place_id: &str,
    alert_type: &str,
    trigger_hazard_id: Option<&str>,
) -> Result<Option<RiskAlert>, StorageError> {
    let mut q = risk_alerts::table
        .filter(risk_alerts::h_app_id.eq(ctx.h_app_id()))
        .filter(risk_alerts::place_id.eq(place_id))
        .filter(risk_alerts::alert_type.eq(alert_type))
        .filter(risk_alerts::status.eq("active"))
        .into_boxed();

    if let Some(hid) = trigger_hazard_id {
        q = q.filter(risk_alerts::trigger_hazard_id.eq(hid));
    }

    q.first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to find active alert: {}", e)))
}

/// Update a risk alert (status, acknowledgement, resolution, escalation)
#[allow(clippy::too_many_arguments)]
pub fn update_risk_alert(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    status: Option<String>,
    acknowledged_by: Option<String>,
    acknowledged_at: Option<String>,
    resolved_at: Option<String>,
    escalated_to: Option<String>,
) -> Result<RiskAlert, StorageError> {
    let now = current_timestamp();

    // Fetch existing to apply partial update
    let existing = get_risk_alert_by_id(conn, ctx, id)?;

    let new_status = status.unwrap_or(existing.status);
    let new_ack_by = acknowledged_by.or(existing.acknowledged_by);
    let new_ack_at = acknowledged_at.or(existing.acknowledged_at);
    let new_resolved_at = resolved_at.or(existing.resolved_at);
    let new_escalated_to = escalated_to.or(existing.escalated_to);

    diesel::update(
        risk_alerts::table
            .filter(risk_alerts::id.eq(id))
            .filter(risk_alerts::h_app_id.eq(ctx.h_app_id())),
    )
    .set((
        risk_alerts::status.eq(&new_status),
        risk_alerts::acknowledged_by.eq(&new_ack_by),
        risk_alerts::acknowledged_at.eq(&new_ack_at),
        risk_alerts::resolved_at.eq(&new_resolved_at),
        risk_alerts::escalated_to.eq(&new_escalated_to),
        risk_alerts::updated_at.eq(&now),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Failed to update risk alert: {}", e)))?;

    get_risk_alert_by_id(conn, ctx, id)
}
