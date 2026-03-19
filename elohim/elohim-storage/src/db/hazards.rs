//! Hazard CRUD operations
//!
//! Geospatial risk events attached to Places.
//! Source of truth: SQLite (operational, Category C).
//! Reconstructable from external feeds / manual reports on cache miss.

use diesel::prelude::*;
use tracing::debug;

use super::context::AppContext;
use super::diesel_schema::hazards;
use super::models::{current_timestamp, Hazard, NewHazard};
use crate::error::StorageError;

// ============================================================================
// Query Types
// ============================================================================

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HazardQuery {
    pub place_id: Option<String>,
    pub status: Option<String>,
    pub hazard_type: Option<String>,
    pub limit: Option<i64>,
}

// ============================================================================
// CRUD Operations
// ============================================================================

/// Create a new hazard record
pub fn create_hazard(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    new: NewHazard,
) -> Result<Hazard, StorageError> {
    debug!(id = %new.id, place_id = %new.place_id, hazard_type = %new.hazard_type, "Creating hazard");

    diesel::insert_into(hazards::table)
        .values(&new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to create hazard: {}", e)))?;

    get_hazard_by_id(conn, ctx, &new.id)
}

/// Get hazard by ID
pub fn get_hazard_by_id(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Hazard, StorageError> {
    hazards::table
        .filter(hazards::id.eq(id))
        .filter(hazards::app_id.eq(ctx.app_id()))
        .first(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                StorageError::NotFound(format!("Hazard not found: {}", id))
            }
            _ => StorageError::Internal(format!("Failed to fetch hazard: {}", e)),
        })
}

/// List hazards with optional filters
pub fn list_hazards(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &HazardQuery,
) -> Result<Vec<Hazard>, StorageError> {
    let mut q = hazards::table
        .filter(hazards::app_id.eq(ctx.app_id()))
        .into_boxed();

    if let Some(ref place_id) = query.place_id {
        q = q.filter(hazards::place_id.eq(place_id));
    }
    if let Some(ref status) = query.status {
        q = q.filter(hazards::status.eq(status));
    }
    if let Some(ref hazard_type) = query.hazard_type {
        q = q.filter(hazards::hazard_type.eq(hazard_type));
    }
    if let Some(limit) = query.limit {
        q = q.limit(limit);
    }

    q.order(hazards::reported_at.desc())
        .load::<Hazard>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list hazards: {}", e)))
}

/// List active hazards for a place (status IN ('active', 'monitoring'))
pub fn list_active_hazards_for_place(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    place_id: &str,
) -> Result<Vec<Hazard>, StorageError> {
    hazards::table
        .filter(hazards::app_id.eq(ctx.app_id()))
        .filter(hazards::place_id.eq(place_id))
        .filter(
            hazards::status
                .eq("active")
                .or(hazards::status.eq("monitoring")),
        )
        .order(hazards::reported_at.desc())
        .load::<Hazard>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list active hazards: {}", e)))
}

/// Batch-load active/monitoring hazards for multiple places in a single query.
/// Returns all matching hazards ordered by place_id (for grouping by caller).
pub fn list_hazards_for_places(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    place_ids: &[&str],
) -> Result<Vec<Hazard>, StorageError> {
    if place_ids.is_empty() {
        return Ok(vec![]);
    }

    hazards::table
        .filter(hazards::app_id.eq(ctx.app_id()))
        .filter(hazards::place_id.eq_any(place_ids))
        .filter(
            hazards::status
                .eq("active")
                .or(hazards::status.eq("monitoring")),
        )
        .order(hazards::place_id.asc())
        .load::<Hazard>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to batch-load hazards: {}", e)))
}

/// Update hazard status (and optionally set resolved_at or actual_onset)
pub fn update_hazard_status(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    new_status: &str,
    resolved_at: Option<String>,
) -> Result<Hazard, StorageError> {
    let now = current_timestamp();

    diesel::update(
        hazards::table
            .filter(hazards::id.eq(id))
            .filter(hazards::app_id.eq(ctx.app_id())),
    )
    .set((
        hazards::status.eq(new_status),
        hazards::resolved_at.eq(&resolved_at),
        hazards::updated_at.eq(&now),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Failed to update hazard status: {}", e)))?;

    get_hazard_by_id(conn, ctx, id)
}
