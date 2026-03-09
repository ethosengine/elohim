//! Collectives CRUD operations using Diesel with app scoping
//!
//! Collectives are governance contexts with graduated participation.
//! Unifies communities and organizations under a single model.

use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::{collective_participations, collectives};
use super::models::{
    consent_states, current_timestamp, governance_layers, intimacy_levels, Collective,
    CollectiveParticipation, NewCollective, NewCollectiveParticipation,
};
use crate::error::StorageError;

// ============================================================================
// Query Types
// ============================================================================

/// Input for creating a collective
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCollectiveInput {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_governance_layer")]
    pub governance_layer: String,
    #[serde(default)]
    pub constitutional_parent_id: Option<String>,
    #[serde(default = "default_community_reach")]
    pub reach: String,
    #[serde(default)]
    pub metadata_json: Option<String>,
    #[serde(default)]
    pub created_by: Option<String>,
}

fn default_governance_layer() -> String {
    governance_layers::COMMUNITY.to_string()
}

fn default_community_reach() -> String {
    "community".to_string()
}

/// Query parameters for listing collectives
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectiveQuery {
    pub governance_layer: Option<String>,
    pub reach: Option<String>,
    /// If true, only return active (non-dissolved) collectives
    #[serde(default = "default_true")]
    pub active_only: bool,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_true() -> bool {
    true
}
fn default_limit() -> i64 {
    100
}

/// Input for creating a participation
#[derive(Debug, Clone, Deserialize)]
pub struct CreateParticipationInput {
    #[serde(default)]
    pub id: Option<String>,
    pub collective_id: String,
    pub human_id: String,
    #[serde(default = "default_intimacy")]
    pub intimacy_level: String,
    #[serde(default)]
    pub role_context: Option<String>,
    #[serde(default = "default_governance_weight")]
    pub governance_weight: f32,
    #[serde(default = "default_consent")]
    pub consent_state: String,
    #[serde(default)]
    pub metadata_json: Option<String>,
}

fn default_intimacy() -> String {
    intimacy_levels::RECOGNITION.to_string()
}
fn default_governance_weight() -> f32 {
    1.0
}
fn default_consent() -> String {
    consent_states::CONSENTED.to_string()
}

// ============================================================================
// Collective Read Operations
// ============================================================================

/// Get collective by ID
pub fn get_collective(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<Collective>, StorageError> {
    collectives::table
        .filter(collectives::app_id.eq(&ctx.app_id))
        .filter(collectives::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// List collectives with filtering
pub fn list_collectives(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &CollectiveQuery,
) -> Result<Vec<Collective>, StorageError> {
    let mut base_query = collectives::table
        .filter(collectives::app_id.eq(&ctx.app_id))
        .into_boxed();

    if let Some(ref layer) = query.governance_layer {
        base_query = base_query.filter(collectives::governance_layer.eq(layer));
    }

    if let Some(ref reach) = query.reach {
        base_query = base_query.filter(collectives::reach.eq(reach));
    }

    if query.active_only {
        base_query = base_query.filter(collectives::dissolved_at.is_null());
    }

    base_query
        .order(collectives::name.asc())
        .limit(query.limit)
        .offset(query.offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Collective Write Operations
// ============================================================================

/// Create or upsert a collective (for seeding)
pub fn create_collective(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: &CreateCollectiveInput,
) -> Result<Collective, StorageError> {
    if !governance_layers::is_valid(&input.governance_layer) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid governance layer: {}. Valid layers: {:?}",
            input.governance_layer,
            governance_layers::ALL
        )));
    }

    // Upsert: if exists, update name/description/layer; if not, insert
    let existing = get_collective(conn, ctx, &input.id)?;

    if existing.is_some() {
        diesel::update(
            collectives::table
                .filter(collectives::app_id.eq(&ctx.app_id))
                .filter(collectives::id.eq(&input.id)),
        )
        .set((
            collectives::name.eq(&input.name),
            collectives::description.eq(&input.description),
            collectives::governance_layer.eq(&input.governance_layer),
            collectives::reach.eq(&input.reach),
            collectives::metadata_json.eq(&input.metadata_json),
            collectives::updated_at.eq(current_timestamp()),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;
    } else {
        let new = NewCollective {
            id: &input.id,
            app_id: &ctx.app_id,
            name: &input.name,
            description: input.description.as_deref(),
            governance_layer: &input.governance_layer,
            constitutional_parent_id: input.constitutional_parent_id.as_deref(),
            reach: &input.reach,
            metadata_json: input.metadata_json.as_deref(),
            created_by: input.created_by.as_deref(),
        };

        diesel::insert_into(collectives::table)
            .values(&new)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;
    }

    get_collective(conn, ctx, &input.id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created collective".into()))
}

/// Dissolve a collective (sets dissolved_at timestamp)
pub fn dissolve_collective(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Collective, StorageError> {
    diesel::update(
        collectives::table
            .filter(collectives::app_id.eq(&ctx.app_id))
            .filter(collectives::id.eq(id)),
    )
    .set((
        collectives::dissolved_at.eq(current_timestamp()),
        collectives::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    get_collective(conn, ctx, id)?
        .ok_or_else(|| StorageError::NotFound(format!("Collective {} not found", id)))
}

// ============================================================================
// Participation Read Operations
// ============================================================================

/// Get all active participations for a human
pub fn get_participations_for_human(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
) -> Result<Vec<CollectiveParticipation>, StorageError> {
    collective_participations::table
        .filter(collective_participations::app_id.eq(&ctx.app_id))
        .filter(collective_participations::human_id.eq(human_id))
        .filter(collective_participations::departed_at.is_null())
        .order(collective_participations::joined_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get all active participants of a collective
pub fn get_participants_of_collective(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    collective_id: &str,
) -> Result<Vec<CollectiveParticipation>, StorageError> {
    collective_participations::table
        .filter(collective_participations::app_id.eq(&ctx.app_id))
        .filter(collective_participations::collective_id.eq(collective_id))
        .filter(collective_participations::departed_at.is_null())
        .order(collective_participations::joined_at.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Participation Write Operations
// ============================================================================

/// Create a participation (tolerates UNIQUE constraint violations for re-seeding)
pub fn create_participation(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: &CreateParticipationInput,
) -> Result<CollectiveParticipation, StorageError> {
    if !intimacy_levels::is_valid(&input.intimacy_level) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid intimacy level: {}. Valid levels: {:?}",
            input.intimacy_level,
            intimacy_levels::ALL
        )));
    }

    let id = input
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Check if participation already exists (upsert for re-seeding)
    let existing: Option<CollectiveParticipation> = collective_participations::table
        .filter(collective_participations::app_id.eq(&ctx.app_id))
        .filter(collective_participations::collective_id.eq(&input.collective_id))
        .filter(collective_participations::human_id.eq(&input.human_id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    if let Some(existing) = existing {
        // Update existing participation
        diesel::update(
            collective_participations::table.filter(collective_participations::id.eq(&existing.id)),
        )
        .set((
            collective_participations::intimacy_level.eq(&input.intimacy_level),
            collective_participations::role_context.eq(&input.role_context),
            collective_participations::governance_weight.eq(input.governance_weight),
            collective_participations::consent_state.eq(&input.consent_state),
            collective_participations::departed_at.eq(None::<String>), // Re-join if departed
            collective_participations::updated_at.eq(current_timestamp()),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

        return collective_participations::table
            .filter(collective_participations::id.eq(&existing.id))
            .first(conn)
            .optional()
            .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?
            .ok_or_else(|| {
                StorageError::Internal("Failed to retrieve updated participation".into())
            });
    }

    let new = NewCollectiveParticipation {
        id: &id,
        app_id: &ctx.app_id,
        collective_id: &input.collective_id,
        human_id: &input.human_id,
        intimacy_level: &input.intimacy_level,
        role_context: input.role_context.as_deref(),
        governance_weight: input.governance_weight,
        consent_state: &input.consent_state,
        metadata_json: input.metadata_json.as_deref(),
    };

    diesel::insert_into(collective_participations::table)
        .values(&new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    collective_participations::table
        .filter(collective_participations::id.eq(&id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created participation".into()))
}

/// Update participation intimacy level
pub fn update_participation_intimacy(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    participation_id: &str,
    new_level: &str,
) -> Result<CollectiveParticipation, StorageError> {
    if !intimacy_levels::is_valid(new_level) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid intimacy level: {}. Valid levels: {:?}",
            new_level,
            intimacy_levels::ALL
        )));
    }

    diesel::update(
        collective_participations::table
            .filter(collective_participations::app_id.eq(&ctx.app_id))
            .filter(collective_participations::id.eq(participation_id)),
    )
    .set((
        collective_participations::intimacy_level.eq(new_level),
        collective_participations::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    collective_participations::table
        .filter(collective_participations::id.eq(participation_id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?
        .ok_or_else(|| {
            StorageError::NotFound(format!("Participation {} not found", participation_id))
        })
}

/// Depart from a collective (sets departed_at — soft exit)
pub fn depart_collective(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    collective_id: &str,
    human_id: &str,
) -> Result<bool, StorageError> {
    let updated = diesel::update(
        collective_participations::table
            .filter(collective_participations::app_id.eq(&ctx.app_id))
            .filter(collective_participations::collective_id.eq(collective_id))
            .filter(collective_participations::human_id.eq(human_id))
            .filter(collective_participations::departed_at.is_null()),
    )
    .set((
        collective_participations::departed_at.eq(current_timestamp()),
        collective_participations::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    Ok(updated > 0)
}

// ============================================================================
// Stats
// ============================================================================

/// Get collective count for an app
pub fn collective_count(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<i64, StorageError> {
    collectives::table
        .filter(collectives::app_id.eq(&ctx.app_id))
        .filter(collectives::dissolved_at.is_null())
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}
