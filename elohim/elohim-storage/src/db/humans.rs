//! Human identity CRUD operations using Diesel
//!
//! Manages mutable profile data for the imagodei pillar.
//! Cryptographic provenance (attestations, agent keys) lives in the Holochain DNA;
//! this layer owns the fast, queryable, offline-capable profile record.

use diesel::prelude::*;
use uuid::Uuid;

use super::diesel_schema::humans;
use super::models::{current_timestamp, Human, NewHuman};
use crate::error::StorageError;

// ============================================================================
// Input Types
// ============================================================================

/// Input for creating a human identity record
#[derive(Debug, Clone)]
pub struct CreateHumanInput {
    pub id: String,
    pub agent_pub_key: Option<String>,
    pub display_name: String,
    pub bio: Option<String>,
    /// JSON-serialised Vec<String>
    pub affinities: String,
    pub profile_reach: String,
    pub location: Option<String>,
    pub profile_photo_url: Option<String>,
    pub app_id: String,
}

/// Input for updating mutable profile fields (all optional)
#[derive(Debug, Clone, Default)]
pub struct UpdateHumanInput {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    /// JSON-serialised Vec<String> — replaces the stored value when present
    pub affinities: Option<String>,
    pub profile_reach: Option<String>,
    pub location: Option<String>,
    pub profile_photo_url: Option<String>,
}

// ============================================================================
// CRUD Operations
// ============================================================================

/// Insert a new human identity record.
///
/// Returns the created `Human` row. Errors if the `id` already exists.
pub fn create_human(
    conn: &mut SqliteConnection,
    input: CreateHumanInput,
) -> Result<Human, StorageError> {
    let id = if input.id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        input.id
    };

    let new_human = NewHuman {
        id: id.clone(),
        agent_pub_key: input.agent_pub_key,
        display_name: input.display_name,
        bio: input.bio,
        affinities: input.affinities,
        profile_reach: input.profile_reach,
        location: input.location,
        profile_photo_url: input.profile_photo_url,
        app_id: input.app_id,
    };

    diesel::insert_into(humans::table)
        .values(&new_human)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to insert human: {}", e)))?;

    get_human_by_id(conn, &id)?
        .ok_or_else(|| StorageError::Internal("Human not found after insert".to_string()))
}

/// Retrieve a human by its stable ID.
pub fn get_human_by_id(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<Human>, StorageError> {
    humans::table
        .filter(humans::id.eq(id))
        .first::<Human>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to fetch human by id: {}", e)))
}

/// Retrieve a human by Holochain agent public key.
pub fn get_human_by_agent_key(
    conn: &mut SqliteConnection,
    agent_pub_key: &str,
) -> Result<Option<Human>, StorageError> {
    humans::table
        .filter(humans::agent_pub_key.eq(agent_pub_key))
        .first::<Human>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to fetch human by agent key: {}", e)))
}

/// Update mutable profile fields for an existing human.
///
/// Only fields present in `input` (i.e., `Some(...)`) are written.
/// Returns the updated row, or `NotFound` if the ID does not exist.
pub fn update_human(
    conn: &mut SqliteConnection,
    id: &str,
    input: UpdateHumanInput,
) -> Result<Human, StorageError> {
    let now = current_timestamp();

    // Fetch existing row first so we can fill in fields not supplied by the caller.
    let existing = get_human_by_id(conn, id)?
        .ok_or_else(|| StorageError::NotFound(format!("Human not found: {}", id)))?;

    let display_name = input.display_name.unwrap_or(existing.display_name);
    let bio = input.bio.or(existing.bio);
    let affinities = input.affinities.unwrap_or(existing.affinities);
    let profile_reach = input.profile_reach.unwrap_or(existing.profile_reach);
    let location = input.location.or(existing.location);
    let profile_photo_url = input.profile_photo_url.or(existing.profile_photo_url);

    let rows_affected = diesel::update(humans::table.filter(humans::id.eq(id)))
        .set((
            humans::display_name.eq(display_name),
            humans::bio.eq(bio),
            humans::affinities.eq(affinities),
            humans::profile_reach.eq(profile_reach),
            humans::location.eq(location),
            humans::profile_photo_url.eq(profile_photo_url),
            humans::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to update human: {}", e)))?;

    if rows_affected == 0 {
        return Err(StorageError::NotFound(format!("Human not found: {}", id)));
    }

    get_human_by_id(conn, id)?
        .ok_or_else(|| StorageError::Internal("Human not found after update".to_string()))
}
