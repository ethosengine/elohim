//! Steward affinity CRUD operations
//!
//! Tracks earned relationships between stewards and content.
//! Affinity scores are accumulated over time through mastery, curation, and review.

use diesel::prelude::*;
use serde::Deserialize;
use tracing::debug;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::steward_affinity;
use super::models::{affinity_sources, current_timestamp, NewStewardAffinity, StewardAffinity};
use crate::error::StorageError;

// ============================================================================
// Input Types
// ============================================================================

/// Input for creating a steward affinity record
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAffinityInput {
    pub steward_id: String,
    pub content_id: String,
    #[serde(default)]
    pub affinity_score: f32,
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String {
    affinity_sources::GENESIS_SEED.to_string()
}

/// Query parameters for listing affinities — camelCase for URL params
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AffinityQuery {
    pub steward_id: Option<String>,
    pub content_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// CRUD Operations
// ============================================================================

/// Create a new steward affinity record
pub fn create_affinity(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: &CreateAffinityInput,
) -> Result<StewardAffinity, StorageError> {
    let id = Uuid::new_v4().to_string();

    // Validate source
    if !affinity_sources::is_valid(&input.source) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid affinity source: {}",
            input.source
        )));
    }

    // Validate score range
    if input.affinity_score < 0.0 || input.affinity_score > 1.0 {
        return Err(StorageError::InvalidInput(format!(
            "Affinity score must be between 0.0 and 1.0, got: {}",
            input.affinity_score
        )));
    }

    let new_affinity = NewStewardAffinity {
        id: &id,
        app_id: ctx.app_id(),
        steward_id: &input.steward_id,
        content_id: &input.content_id,
        affinity_score: input.affinity_score,
        source: &input.source,
    };

    diesel::insert_into(steward_affinity::table)
        .values(&new_affinity)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to create affinity: {}", e)))?;

    debug!(
        "Created steward affinity {} for steward {} content {}",
        id, input.steward_id, input.content_id
    );

    get_affinity_by_id(conn, ctx, &id)
}

/// Get an affinity by ID
pub fn get_affinity_by_id(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<StewardAffinity, StorageError> {
    steward_affinity::table
        .filter(steward_affinity::id.eq(id))
        .filter(steward_affinity::app_id.eq(ctx.app_id()))
        .first::<StewardAffinity>(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => StorageError::NotFound(id.to_string()),
            _ => StorageError::Internal(format!("Failed to get affinity: {}", e)),
        })
}

/// Get affinity for a specific steward-content pair
pub fn get_affinity_for_steward_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    steward_id: &str,
    content_id: &str,
) -> Result<Option<StewardAffinity>, StorageError> {
    steward_affinity::table
        .filter(steward_affinity::app_id.eq(ctx.app_id()))
        .filter(steward_affinity::steward_id.eq(steward_id))
        .filter(steward_affinity::content_id.eq(content_id))
        .first::<StewardAffinity>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to get affinity: {}", e)))
}

/// List affinities with optional filters, ordered by affinity_score descending
pub fn list_affinities(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &AffinityQuery,
) -> Result<Vec<StewardAffinity>, StorageError> {
    let mut q = steward_affinity::table
        .filter(steward_affinity::app_id.eq(ctx.app_id()))
        .into_boxed();

    if let Some(steward_id) = &query.steward_id {
        q = q.filter(steward_affinity::steward_id.eq(steward_id));
    }

    if let Some(content_id) = &query.content_id {
        q = q.filter(steward_affinity::content_id.eq(content_id));
    }

    // Order by affinity_score descending (strongest affinity first)
    q = q.order(steward_affinity::affinity_score.desc());

    if let Some(limit) = query.limit {
        q = q.limit(limit);
    }

    if let Some(offset) = query.offset {
        q = q.offset(offset);
    }

    q.load::<StewardAffinity>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list affinities: {}", e)))
}

/// Update affinity score by applying a delta, clamped to 0.0-1.0
pub fn update_affinity_score(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    steward_id: &str,
    content_id: &str,
    delta: f32,
    source: &str,
) -> Result<StewardAffinity, StorageError> {
    // Validate source
    if !affinity_sources::is_valid(source) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid affinity source: {}",
            source
        )));
    }

    // Get existing record
    let existing = get_affinity_for_steward_content(conn, ctx, steward_id, content_id)?
        .ok_or_else(|| {
            StorageError::NotFound(format!(
                "No affinity for steward {} content {}",
                steward_id, content_id
            ))
        })?;

    // Apply delta, clamped to valid range
    let new_score = (existing.affinity_score + delta).clamp(0.0, 1.0);
    let now = current_timestamp();

    diesel::update(steward_affinity::table)
        .filter(steward_affinity::id.eq(&existing.id))
        .filter(steward_affinity::app_id.eq(ctx.app_id()))
        .set((
            steward_affinity::affinity_score.eq(new_score),
            steward_affinity::source.eq(source),
            steward_affinity::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to update affinity score: {}", e)))?;

    debug!(
        "Updated affinity for steward {} content {}: {} -> {} (delta {})",
        steward_id, content_id, existing.affinity_score, new_score, delta
    );

    get_affinity_by_id(conn, ctx, &existing.id)
}

/// Bulk create affinity records, returning count of successes and any error messages
pub fn bulk_create_affinities(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    inputs: &[CreateAffinityInput],
) -> Result<(usize, Vec<String>), StorageError> {
    let mut created_count = 0;
    let mut errors = Vec::new();

    for input in inputs {
        match create_affinity(conn, ctx, input) {
            Ok(_) => created_count += 1,
            Err(e) => errors.push(format!(
                "Failed to create affinity for steward {} content {}: {}",
                input.steward_id, input.content_id, e
            )),
        }
    }

    Ok((created_count, errors))
}
