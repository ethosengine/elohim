//! Place CRUD operations
//!
//! Governed spatial entities — storage projection of Mishpat DHT entries.
//! Source of truth: DHT (Category A, notarized). This table is a read-optimized index.

use diesel::prelude::*;
use tracing::debug;

use super::context::AppContext;
use super::diesel_schema::places;
use super::models::{current_timestamp, NewPlace, Place};
use crate::error::StorageError;

// ============================================================================
// Query Types
// ============================================================================

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceQuery {
    pub place_type: Option<String>,
    pub constitutional_layer: Option<String>,
    pub h3_index: Option<String>,
    pub parent_place_id: Option<String>,
    pub status: Option<String>,
    pub governing_collective_id: Option<String>,
    pub limit: Option<i64>,
}

// ============================================================================
// CRUD Operations
// ============================================================================

/// Upsert a place (projection from DHT signal or direct creation)
pub fn upsert_place(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    new: NewPlace,
) -> Result<Place, StorageError> {
    debug!(id = %new.id, name = %new.name, h3 = %new.h3_index, "Upserting place");

    // Try insert; on conflict update
    diesel::insert_into(places::table)
        .values(&new)
        .on_conflict(places::id)
        .do_update()
        .set((
            places::name.eq(&new.name),
            places::place_type.eq(&new.place_type),
            places::constitutional_layer.eq(&new.constitutional_layer),
            places::h3_index.eq(&new.h3_index),
            places::h3_resolution.eq(&new.h3_resolution),
            places::geometry_json.eq(&new.geometry_json),
            places::centroid_lat.eq(&new.centroid_lat),
            places::centroid_lng.eq(&new.centroid_lng),
            places::parent_place_id.eq(&new.parent_place_id),
            places::osm_reference_json.eq(&new.osm_reference_json),
            places::carrying_capacity_json.eq(&new.carrying_capacity_json),
            places::governing_collective_id.eq(&new.governing_collective_id),
            places::status.eq(&new.status),
            places::metadata_json.eq(&new.metadata_json),
            places::updated_at.eq(&current_timestamp()),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to upsert place: {}", e)))?;

    get_place_by_id(conn, ctx, &new.id)
}

/// Get place by ID
pub fn get_place_by_id(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Place, StorageError> {
    places::table
        .filter(places::id.eq(id))
        .filter(places::app_id.eq(ctx.app_id()))
        .first(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                StorageError::NotFound(format!("Place not found: {}", id))
            }
            _ => StorageError::Internal(format!("Failed to fetch place: {}", e)),
        })
}

/// List places with optional filters (including H3 spatial queries)
pub fn list_places(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &PlaceQuery,
) -> Result<Vec<Place>, StorageError> {
    let mut q = places::table
        .filter(places::app_id.eq(ctx.app_id()))
        .into_boxed();

    if let Some(ref pt) = query.place_type {
        q = q.filter(places::place_type.eq(pt));
    }
    if let Some(ref layer) = query.constitutional_layer {
        q = q.filter(places::constitutional_layer.eq(layer));
    }
    if let Some(ref h3) = query.h3_index {
        q = q.filter(places::h3_index.eq(h3));
    }
    if let Some(ref parent) = query.parent_place_id {
        q = q.filter(places::parent_place_id.eq(parent));
    }
    if let Some(ref status) = query.status {
        q = q.filter(places::status.eq(status));
    }
    if let Some(ref cid) = query.governing_collective_id {
        q = q.filter(places::governing_collective_id.eq(cid));
    }
    if let Some(limit) = query.limit {
        q = q.limit(limit);
    }

    q.order(places::name.asc())
        .load::<Place>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list places: {}", e)))
}
