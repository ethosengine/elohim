//! Contributor presences CRUD operations using Diesel with app scoping
//!
//! Manages contributor presence lifecycle from unclaimed (anonymous contribution)
//! through stewarded (community maintenance) to claimed (verified identity).

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::contributor_presences;
use super::models::{ContributorPresence, NewContributorPresence, presence_states, current_timestamp};
use crate::error::StorageError;

// ============================================================================
// Query Types
// ============================================================================

/// Input for creating a contributor presence
#[derive(Debug, Clone, Deserialize)]
pub struct CreateContributorPresenceInput {
    #[serde(default)]
    pub id: Option<String>,
    pub display_name: String,
    #[serde(default)]
    pub external_identifiers_json: Option<String>,
    pub establishing_content_ids: Vec<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
}

/// Query parameters for listing contributor presences
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ContributorPresenceQuery {
    /// Filter by presence state
    pub presence_state: Option<String>,
    /// Filter by steward agent ID
    pub steward_id: Option<String>,
    /// Filter by claimed agent ID
    pub claimed_agent_id: Option<String>,
    /// Filter by minimum recognition score
    pub min_recognition_score: Option<f32>,
    /// Filter by minimum affinity total
    pub min_affinity: Option<f32>,
    /// Search by display name
    pub search: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 { 100 }

/// Result of bulk operation
#[derive(Debug, Clone, Serialize)]
pub struct BulkContributorPresenceResult {
    pub created: u64,
    pub errors: Vec<String>,
}

/// Input for initiating stewardship
#[derive(Debug, Clone, Deserialize)]
pub struct InitiateStewardshipInput {
    pub steward_id: String,
    #[serde(default)]
    pub stewardship_commitment_id: Option<String>,
}

/// Input for initiating a claim
#[derive(Debug, Clone, Deserialize)]
pub struct InitiateClaimInput {
    pub claiming_agent_id: String,
    pub verification_method: String,
    #[serde(default)]
    pub evidence_json: Option<String>,
    #[serde(default)]
    pub facilitated_by: Option<String>,
}

/// Recognition update input
#[derive(Debug, Clone, Deserialize)]
pub struct RecognitionUpdate {
    pub affinity_delta: f32,
    pub engager_id: String,
    #[serde(default)]
    pub content_id: Option<String>,
    #[serde(default)]
    pub is_citation: bool,
}

// ============================================================================
// Read Operations
// ============================================================================

/// Get contributor presence by ID - scoped by app
pub fn get_contributor_presence(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<ContributorPresence>, StorageError> {
    contributor_presences::table
        .filter(contributor_presences::app_id.eq(&ctx.app_id))
        .filter(contributor_presences::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// List contributor presences with filtering - scoped by app
pub fn list_contributor_presences(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &ContributorPresenceQuery,
) -> Result<Vec<ContributorPresence>, StorageError> {
    let search_pattern = query.search.as_ref().map(|s| format!("%{}%", s));

    let mut base_query = contributor_presences::table
        .filter(contributor_presences::app_id.eq(&ctx.app_id))
        .into_boxed();

    // Apply filters
    if let Some(ref state) = query.presence_state {
        base_query = base_query.filter(contributor_presences::presence_state.eq(state));
    }

    if let Some(ref steward_id) = query.steward_id {
        base_query = base_query.filter(contributor_presences::steward_id.eq(steward_id));
    }

    if let Some(ref claimed_id) = query.claimed_agent_id {
        base_query = base_query.filter(contributor_presences::claimed_agent_id.eq(claimed_id));
    }

    if let Some(min_score) = query.min_recognition_score {
        base_query = base_query.filter(contributor_presences::recognition_score.ge(min_score));
    }

    if let Some(min_affinity) = query.min_affinity {
        base_query = base_query.filter(contributor_presences::affinity_total.ge(min_affinity));
    }

    if let Some(ref pattern) = search_pattern {
        base_query = base_query.filter(contributor_presences::display_name.like(pattern));
    }

    base_query
        .order(contributor_presences::recognition_score.desc())
        .limit(query.limit)
        .offset(query.offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Find contributor presence by external identifier
pub fn find_by_external_identifier(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    identifier_type: &str,
    identifier_value: &str,
) -> Result<Option<ContributorPresence>, StorageError> {
    // Search for identifier in JSON field
    let search_pattern = format!("%\"{}\":\"%{}%\"%", identifier_type, identifier_value);

    contributor_presences::table
        .filter(contributor_presences::app_id.eq(&ctx.app_id))
        .filter(contributor_presences::external_identifiers_json.like(&search_pattern))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get presences for content (contributors who established via specific content)
pub fn get_presences_for_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<Vec<ContributorPresence>, StorageError> {
    // Search for content_id in JSON array
    let search_pattern = format!("%\"{}\",%", content_id);
    let search_pattern_last = format!("%\"{}\"\\]%", content_id);

    contributor_presences::table
        .filter(contributor_presences::app_id.eq(&ctx.app_id))
        .filter(
            contributor_presences::establishing_content_ids_json.like(&search_pattern)
                .or(contributor_presences::establishing_content_ids_json.like(&search_pattern_last))
        )
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get unclaimed presences ordered by recognition (for stewardship candidates)
pub fn get_stewardship_candidates(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
) -> Result<Vec<ContributorPresence>, StorageError> {
    contributor_presences::table
        .filter(contributor_presences::app_id.eq(&ctx.app_id))
        .filter(contributor_presences::presence_state.eq(presence_states::UNCLAIMED))
        .order(contributor_presences::recognition_score.desc())
        .limit(limit)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Write Operations
// ============================================================================

/// Create a single contributor presence - scoped by app
pub fn create_contributor_presence(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateContributorPresenceInput,
) -> Result<ContributorPresence, StorageError> {
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    // Convert establishing_content_ids to JSON
    let establishing_json = serde_json::to_string(&input.establishing_content_ids)
        .map_err(|e| StorageError::Internal(format!("JSON serialization failed: {}", e)))?;

    let new_presence = NewContributorPresence {
        id: &id,
        app_id: &ctx.app_id,
        display_name: &input.display_name,
        presence_state: presence_states::UNCLAIMED,
        external_identifiers_json: input.external_identifiers_json.as_deref(),
        establishing_content_ids_json: &establishing_json,
        image: input.image.as_deref(),
        note: input.note.as_deref(),
        metadata_json: input.metadata_json.as_deref(),
    };

    diesel::insert_into(contributor_presences::table)
        .values(&new_presence)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    get_contributor_presence(conn, ctx, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created presence".into()))
}

/// Accumulate recognition (affinity) for a contributor
pub fn accumulate_recognition(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    update: &RecognitionUpdate,
) -> Result<ContributorPresence, StorageError> {
    // Get current presence
    let presence = get_contributor_presence(conn, ctx, id)?
        .ok_or_else(|| StorageError::NotFound(format!("Presence {} not found", id)))?;

    // Calculate new values
    let new_affinity = presence.affinity_total + update.affinity_delta;
    let new_citation_count = if update.is_citation {
        presence.citation_count + 1
    } else {
        presence.citation_count
    };

    // Update recognition by content if content_id provided
    let new_recognition_by_content = if let Some(ref content_id) = update.content_id {
        let mut by_content: serde_json::Map<String, serde_json::Value> = presence
            .recognition_by_content_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let current = by_content.get(content_id)
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        by_content.insert(content_id.clone(), serde_json::json!(current + update.affinity_delta as f64));

        Some(serde_json::to_string(&by_content)
            .map_err(|e| StorageError::Internal(format!("JSON serialization failed: {}", e)))?)
    } else {
        presence.recognition_by_content_json.clone()
    };

    // Simple recognition score formula
    let new_recognition_score = new_affinity * 0.6 + (new_citation_count as f32) * 0.4;

    // TODO: Track unique engagers properly (would need separate table)
    let new_unique_engagers = presence.unique_engagers + 1;

    diesel::update(
        contributor_presences::table
            .filter(contributor_presences::app_id.eq(&ctx.app_id))
            .filter(contributor_presences::id.eq(id))
    )
    .set((
        contributor_presences::affinity_total.eq(new_affinity),
        contributor_presences::citation_count.eq(new_citation_count),
        contributor_presences::unique_engagers.eq(new_unique_engagers),
        contributor_presences::recognition_score.eq(new_recognition_score),
        contributor_presences::recognition_by_content_json.eq(new_recognition_by_content),
        contributor_presences::last_recognition_at.eq(current_timestamp()),
        contributor_presences::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    get_contributor_presence(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated presence".into()))
}

/// Initiate stewardship of an unclaimed presence
pub fn initiate_stewardship(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    input: &InitiateStewardshipInput,
) -> Result<ContributorPresence, StorageError> {
    // Verify current state
    let presence = get_contributor_presence(conn, ctx, id)?
        .ok_or_else(|| StorageError::NotFound(format!("Presence {} not found", id)))?;

    if presence.presence_state != presence_states::UNCLAIMED {
        return Err(StorageError::InvalidInput(format!(
            "Can only steward unclaimed presences. Current state: {}",
            presence.presence_state
        )));
    }

    diesel::update(
        contributor_presences::table
            .filter(contributor_presences::app_id.eq(&ctx.app_id))
            .filter(contributor_presences::id.eq(id))
    )
    .set((
        contributor_presences::presence_state.eq(presence_states::STEWARDED),
        contributor_presences::steward_id.eq(&input.steward_id),
        contributor_presences::stewardship_started_at.eq(current_timestamp()),
        contributor_presences::stewardship_commitment_id.eq(input.stewardship_commitment_id.as_deref()),
        contributor_presences::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    get_contributor_presence(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated presence".into()))
}

/// Initiate a claim on a presence (start verification process)
pub fn initiate_claim(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    input: &InitiateClaimInput,
) -> Result<ContributorPresence, StorageError> {
    // Verify current state
    let presence = get_contributor_presence(conn, ctx, id)?
        .ok_or_else(|| StorageError::NotFound(format!("Presence {} not found", id)))?;

    if presence.presence_state == presence_states::CLAIMED {
        return Err(StorageError::InvalidInput(
            "Presence is already claimed".into()
        ));
    }

    diesel::update(
        contributor_presences::table
            .filter(contributor_presences::app_id.eq(&ctx.app_id))
            .filter(contributor_presences::id.eq(id))
    )
    .set((
        contributor_presences::presence_state.eq(presence_states::CLAIMING),
        contributor_presences::claimed_agent_id.eq(&input.claiming_agent_id),
        contributor_presences::claim_initiated_at.eq(current_timestamp()),
        contributor_presences::claim_verification_method.eq(&input.verification_method),
        contributor_presences::claim_evidence_json.eq(input.evidence_json.as_deref()),
        contributor_presences::claim_facilitated_by.eq(input.facilitated_by.as_deref()),
        contributor_presences::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    get_contributor_presence(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated presence".into()))
}

/// Verify and complete a claim
pub fn verify_claim(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<ContributorPresence, StorageError> {
    // Verify current state
    let presence = get_contributor_presence(conn, ctx, id)?
        .ok_or_else(|| StorageError::NotFound(format!("Presence {} not found", id)))?;

    if presence.presence_state != presence_states::CLAIMING {
        return Err(StorageError::InvalidInput(format!(
            "Can only verify presences in claiming state. Current state: {}",
            presence.presence_state
        )));
    }

    // Transfer recognition value to claiming agent
    let transfer_value = presence.recognition_score;

    diesel::update(
        contributor_presences::table
            .filter(contributor_presences::app_id.eq(&ctx.app_id))
            .filter(contributor_presences::id.eq(id))
    )
    .set((
        contributor_presences::presence_state.eq(presence_states::CLAIMED),
        contributor_presences::claim_verified_at.eq(current_timestamp()),
        contributor_presences::claim_recognition_transferred_value.eq(transfer_value),
        contributor_presences::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    get_contributor_presence(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated presence".into()))
}

/// Update stewardship quality score
pub fn update_stewardship_quality(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    quality_score: f32,
) -> Result<ContributorPresence, StorageError> {
    diesel::update(
        contributor_presences::table
            .filter(contributor_presences::app_id.eq(&ctx.app_id))
            .filter(contributor_presences::id.eq(id))
    )
    .set((
        contributor_presences::stewardship_quality_score.eq(quality_score),
        contributor_presences::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    get_contributor_presence(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated presence".into()))
}

/// Delete a contributor presence by ID - scoped by app
pub fn delete_contributor_presence(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<bool, StorageError> {
    let deleted = diesel::delete(
        contributor_presences::table
            .filter(contributor_presences::app_id.eq(&ctx.app_id))
            .filter(contributor_presences::id.eq(id))
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    Ok(deleted > 0)
}

// ============================================================================
// Stats
// ============================================================================

/// Get contributor presence count for an app
pub fn contributor_presence_count(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<i64, StorageError> {
    contributor_presences::table
        .filter(contributor_presences::app_id.eq(&ctx.app_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

/// Get presence statistics by state
pub fn stats_by_state(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<Vec<(String, i64)>, StorageError> {
    contributor_presences::table
        .filter(contributor_presences::app_id.eq(&ctx.app_id))
        .group_by(contributor_presences::presence_state)
        .select((contributor_presences::presence_state, diesel::dsl::count_star()))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Stats query failed: {}", e)))
}

/// Get total recognition in system
pub fn total_recognition(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<f32, StorageError> {
    contributor_presences::table
        .filter(contributor_presences::app_id.eq(&ctx.app_id))
        .select(diesel::dsl::sum(contributor_presences::recognition_score))
        .first::<Option<f32>>(conn)
        .map(|v| v.unwrap_or(0.0))
        .map_err(|e| StorageError::Internal(format!("Sum query failed: {}", e)))
}

// ============================================================================
// Bulk Operations
// ============================================================================

/// Bulk create contributor presences (for seeding/import) - scoped by app
pub fn bulk_create_presences(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    inputs: Vec<CreateContributorPresenceInput>,
) -> Result<BulkContributorPresenceResult, StorageError> {
    let mut created = 0u64;
    let mut errors = vec![];

    conn.transaction(|conn| {
        for input in inputs {
            match create_contributor_presence(conn, ctx, input.clone()) {
                Ok(_) => created += 1,
                Err(e) => {
                    errors.push(format!("{}: {}", input.display_name, e));
                }
            }
        }

        Ok(BulkContributorPresenceResult { created, errors })
    })
}
