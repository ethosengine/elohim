//! Spatial Context CRUD operations
//!
//! Polymorphic geospatial attachment for any CID-addressed entity.
//! Follows the same pattern as schedules.rs — entityType + entityId.
//!
//! Source of truth: SQLite (operational, Category C)

use diesel::prelude::*;
use tracing::debug;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::{places, spatial_contexts};
use super::models::{current_timestamp, NewSpatialContext, SpatialContext};
use crate::error::StorageError;
use crate::services::spatial::compute_h3_cells;

// ============================================================================
// Input Types
// ============================================================================

/// Input for creating a spatial context
pub struct CreateSpatialContextInput {
    pub entity_type: String,
    pub entity_id: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub accuracy: Option<f64>,
    pub place_id: Option<String>,
    pub osm_type: Option<String>,
    pub osm_id: Option<i32>,
    pub label: Option<String>,
    pub context_type: Option<String>,
    pub geometry_json: Option<String>,
    pub metadata_json: Option<String>,
    pub observed_at: Option<String>,
}

/// Input for updating a spatial context (PATCH semantics)
#[derive(Debug, Default)]
pub struct UpdateSpatialContextInput {
    pub latitude: Option<Option<f64>>,
    pub longitude: Option<Option<f64>>,
    pub altitude: Option<Option<f64>>,
    pub accuracy: Option<Option<f64>>,
    pub place_id: Option<Option<String>>,
    pub label: Option<Option<String>>,
    pub context_type: Option<String>,
    pub geometry_json: Option<Option<String>>,
    pub metadata_json: Option<Option<String>>,
    pub observed_at: Option<Option<String>>,
}

/// Query params for listing spatial contexts
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialContextQuery {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub h3_res5: Option<String>,
    pub h3_res7: Option<String>,
    pub h3_res9: Option<String>,
    pub place_id: Option<String>,
    pub limit: Option<i64>,
}

// ============================================================================
// Place Resolution
// ============================================================================

/// Auto-resolve a Place from H3 cell indexes.
/// Tries neighborhood-level (res7) first, falls back to municipal (res5).
/// Returns the first active Place whose h3_index matches.
fn resolve_place_from_h3(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    h3_res7: Option<&str>,
    h3_res5: Option<&str>,
) -> Result<Option<String>, StorageError> {
    // Try neighborhood resolution first (more specific)
    if let Some(h3) = h3_res7 {
        let result: Result<String, _> = places::table
            .filter(places::h_app_id.eq(ctx.h_app_id()))
            .filter(places::h3_index.eq(h3))
            .filter(places::status.eq("active"))
            .select(places::id)
            .first(conn);

        if let Ok(place_id) = result {
            return Ok(Some(place_id));
        }
    }

    // Fall back to municipal resolution (broader)
    if let Some(h3) = h3_res5 {
        let result: Result<String, _> = places::table
            .filter(places::h_app_id.eq(ctx.h_app_id()))
            .filter(places::h3_index.eq(h3))
            .filter(places::status.eq("active"))
            .select(places::id)
            .first(conn);

        if let Ok(place_id) = result {
            return Ok(Some(place_id));
        }
    }

    Ok(None)
}

// ============================================================================
// CRUD Operations
// ============================================================================

/// Create a new spatial context. Auto-computes H3 cells and resolves Place from lat/lng.
pub fn create_spatial_context(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateSpatialContextInput,
) -> Result<SpatialContext, StorageError> {
    let now = current_timestamp();
    let id = Uuid::new_v4().to_string();

    // Auto-compute H3 cells from coordinates
    let (h3_res5, h3_res7, h3_res9) = match (input.latitude, input.longitude) {
        (Some(lat), Some(lng)) => compute_h3_cells(lat, lng),
        _ => (None, None, None),
    };

    // Auto-resolve Place from H3 if not explicitly provided
    let place_id = if input.place_id.is_some() {
        input.place_id
    } else {
        resolve_place_from_h3(conn, ctx, h3_res7.as_deref(), h3_res5.as_deref())?
    };

    // Mark any existing current entry for this entity as historical
    diesel::update(
        spatial_contexts::table
            .filter(spatial_contexts::h_app_id.eq(ctx.h_app_id()))
            .filter(spatial_contexts::entity_type.eq(&input.entity_type))
            .filter(spatial_contexts::entity_id.eq(&input.entity_id))
            .filter(spatial_contexts::is_current.eq(1)),
    )
    .set(spatial_contexts::is_current.eq(0))
    .execute(conn)
    .ok(); // Ignore if no previous entry exists

    let new = NewSpatialContext {
        id: id.clone(),
        h_app_id: ctx.h_app_id().to_string(),
        entity_type: input.entity_type,
        entity_id: input.entity_id,
        latitude: input.latitude,
        longitude: input.longitude,
        altitude: input.altitude,
        accuracy: input.accuracy,
        h3_res5,
        h3_res7,
        h3_res9,
        place_id,
        osm_type: input.osm_type,
        osm_id: input.osm_id,
        label: input.label,
        context_type: input.context_type.unwrap_or_else(|| "point".to_string()),
        geometry_json: input.geometry_json,
        metadata_json: input.metadata_json,
        observed_at: input.observed_at,
        created_at: now.clone(),
        updated_at: now,
        is_current: 1,
    };

    debug!(
        id = %new.id,
        entity_type = %new.entity_type,
        entity_id = %new.entity_id,
        h3_res5 = ?new.h3_res5,
        "Creating spatial context"
    );

    diesel::insert_into(spatial_contexts::table)
        .values(&new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to create spatial context: {}", e)))?;

    spatial_contexts::table
        .filter(spatial_contexts::id.eq(&id))
        .first(conn)
        .map_err(|e| {
            StorageError::Internal(format!("Failed to fetch created spatial context: {}", e))
        })
}

/// Get current spatial context by entity type and entity ID
pub fn get_spatial_context(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    entity_type: &str,
    entity_id: &str,
) -> Result<SpatialContext, StorageError> {
    spatial_contexts::table
        .filter(spatial_contexts::h_app_id.eq(ctx.h_app_id()))
        .filter(spatial_contexts::entity_type.eq(entity_type))
        .filter(spatial_contexts::entity_id.eq(entity_id))
        .filter(spatial_contexts::is_current.eq(1))
        .first(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => StorageError::NotFound(format!(
                "Spatial context not found for {}:{}",
                entity_type, entity_id
            )),
            _ => StorageError::Internal(format!("Failed to fetch spatial context: {}", e)),
        })
}

/// Get full spatial context history for an entity, ordered newest first.
/// Returns all entries (current + historical).
pub fn get_spatial_context_history(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<SpatialContext>, StorageError> {
    spatial_contexts::table
        .filter(spatial_contexts::h_app_id.eq(ctx.h_app_id()))
        .filter(spatial_contexts::entity_type.eq(entity_type))
        .filter(spatial_contexts::entity_id.eq(entity_id))
        .order(spatial_contexts::created_at.desc())
        .load::<SpatialContext>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to fetch spatial history: {}", e)))
}

/// Get spatial context by ID
pub fn get_spatial_context_by_id(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<SpatialContext, StorageError> {
    spatial_contexts::table
        .filter(spatial_contexts::id.eq(id))
        .filter(spatial_contexts::h_app_id.eq(ctx.h_app_id()))
        .first(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                StorageError::NotFound(format!("Spatial context not found: {}", id))
            }
            _ => StorageError::Internal(format!("Failed to fetch spatial context: {}", e)),
        })
}

/// List spatial contexts with optional filters.
/// Supports spatial queries via H3 cell indexes.
pub fn list_spatial_contexts(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &SpatialContextQuery,
) -> Result<Vec<SpatialContext>, StorageError> {
    let mut q = spatial_contexts::table
        .filter(spatial_contexts::h_app_id.eq(ctx.h_app_id()))
        .into_boxed();

    if let Some(ref entity_type) = query.entity_type {
        q = q.filter(spatial_contexts::entity_type.eq(entity_type));
    }
    if let Some(ref entity_id) = query.entity_id {
        q = q.filter(spatial_contexts::entity_id.eq(entity_id));
    }
    // H3 spatial queries — find all entities within a hex cell
    if let Some(ref h3) = query.h3_res5 {
        q = q.filter(spatial_contexts::h3_res5.eq(h3));
    }
    if let Some(ref h3) = query.h3_res7 {
        q = q.filter(spatial_contexts::h3_res7.eq(h3));
    }
    if let Some(ref h3) = query.h3_res9 {
        q = q.filter(spatial_contexts::h3_res9.eq(h3));
    }
    if let Some(ref pid) = query.place_id {
        q = q.filter(spatial_contexts::place_id.eq(pid));
    }
    if let Some(limit) = query.limit {
        q = q.limit(limit);
    }

    q.order(spatial_contexts::created_at.desc())
        .load::<SpatialContext>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list spatial contexts: {}", e)))
}

/// Update a spatial context (PATCH semantics). Recomputes H3 cells if coordinates change.
pub fn update_spatial_context(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    input: UpdateSpatialContextInput,
) -> Result<SpatialContext, StorageError> {
    let existing = get_spatial_context_by_id(conn, ctx, id)?;
    let now = current_timestamp();

    let new_lat = input.latitude.unwrap_or(existing.latitude);
    let new_lng = input.longitude.unwrap_or(existing.longitude);
    let new_alt = input.altitude.unwrap_or(existing.altitude);
    let new_acc = input.accuracy.unwrap_or(existing.accuracy);
    let explicit_place = input.place_id;
    let new_label = input.label.unwrap_or(existing.label);
    let new_ctx_type = input.context_type.unwrap_or(existing.context_type);
    let new_geo = input.geometry_json.unwrap_or(existing.geometry_json);
    let new_meta = input.metadata_json.unwrap_or(existing.metadata_json);
    let new_obs = input.observed_at.unwrap_or(existing.observed_at);

    // Recompute H3 if coordinates changed
    let (h3_5, h3_7, h3_9) = match (new_lat, new_lng) {
        (Some(lat), Some(lng)) => compute_h3_cells(lat, lng),
        _ => (None, None, None),
    };

    // Re-resolve Place if coords changed and no explicit place was provided
    let new_place = if let Some(explicit) = explicit_place {
        explicit
    } else if new_lat != existing.latitude || new_lng != existing.longitude {
        resolve_place_from_h3(conn, ctx, h3_7.as_deref(), h3_5.as_deref())?
    } else {
        existing.place_id
    };

    diesel::update(spatial_contexts::table.filter(spatial_contexts::id.eq(id)))
        .set((
            spatial_contexts::latitude.eq(&new_lat),
            spatial_contexts::longitude.eq(&new_lng),
            spatial_contexts::altitude.eq(&new_alt),
            spatial_contexts::accuracy.eq(&new_acc),
            spatial_contexts::h3_res5.eq(&h3_5),
            spatial_contexts::h3_res7.eq(&h3_7),
            spatial_contexts::h3_res9.eq(&h3_9),
            spatial_contexts::place_id.eq(&new_place),
            spatial_contexts::label.eq(&new_label),
            spatial_contexts::context_type.eq(&new_ctx_type),
            spatial_contexts::geometry_json.eq(&new_geo),
            spatial_contexts::metadata_json.eq(&new_meta),
            spatial_contexts::observed_at.eq(&new_obs),
            spatial_contexts::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to update spatial context: {}", e)))?;

    get_spatial_context_by_id(conn, ctx, id)
}

/// Delete a spatial context
pub fn delete_spatial_context(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<(), StorageError> {
    let count = diesel::delete(
        spatial_contexts::table
            .filter(spatial_contexts::id.eq(id))
            .filter(spatial_contexts::h_app_id.eq(ctx.h_app_id())),
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Failed to delete spatial context: {}", e)))?;

    if count == 0 {
        return Err(StorageError::NotFound(format!(
            "Spatial context not found: {}",
            id
        )));
    }
    Ok(())
}
