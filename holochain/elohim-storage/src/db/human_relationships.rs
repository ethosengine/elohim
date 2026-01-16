//! Human relationships CRUD operations using Diesel with app scoping
//!
//! Tracks relationships between humans with intimacy levels, consent tracking,
//! custody enablement, and governance layer support.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::human_relationships;
use super::models::{HumanRelationship, NewHumanRelationship, intimacy_levels, current_timestamp};
use crate::error::StorageError;

// ============================================================================
// Query Types
// ============================================================================

/// Input for creating a human relationship
#[derive(Debug, Clone, Deserialize)]
pub struct CreateHumanRelationshipInput {
    #[serde(default)]
    pub id: Option<String>,
    pub party_a_id: String,
    pub party_b_id: String,
    pub relationship_type: String,
    #[serde(default = "default_intimacy")]
    pub intimacy_level: String,
    #[serde(default)]
    pub is_bidirectional: bool,
    #[serde(default)]
    pub consent_given_by_a: bool,
    #[serde(default)]
    pub consent_given_by_b: bool,
    pub initiated_by: String,
    #[serde(default)]
    pub governance_layer: Option<String>,
    #[serde(default = "default_private_reach")]
    pub reach: String,
    #[serde(default)]
    pub context_json: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
}

fn default_intimacy() -> String { intimacy_levels::RECOGNITION.to_string() }
fn default_private_reach() -> String { "private".to_string() }

/// Query parameters for listing human relationships - camelCase for URL params
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HumanRelationshipQuery {
    /// Filter by party (either side)
    pub party_id: Option<String>,
    /// Filter by relationship type
    pub relationship_type: Option<String>,
    /// Filter by intimacy level
    pub intimacy_level: Option<String>,
    /// Filter by minimum intimacy level (inclusive)
    pub min_intimacy_level: Option<String>,
    /// Filter to only relationships with mutual consent
    pub mutual_consent_only: Option<bool>,
    /// Filter to only relationships with custody enabled
    pub custody_enabled_only: Option<bool>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 { 100 }

/// Result of bulk operation
#[derive(Debug, Clone, Serialize)]
pub struct BulkHumanRelationshipResult {
    pub created: u64,
    pub errors: Vec<String>,
}

/// Consent update input
#[derive(Debug, Clone, Deserialize)]
pub struct ConsentUpdate {
    pub consent_given: bool,
}

/// Custody update input
#[derive(Debug, Clone, Deserialize)]
pub struct CustodyUpdate {
    pub custody_enabled: bool,
    #[serde(default)]
    pub auto_custody_enabled: Option<bool>,
    #[serde(default)]
    pub emergency_access_enabled: Option<bool>,
}

// ============================================================================
// Read Operations
// ============================================================================

/// Get human relationship by ID - scoped by app
pub fn get_human_relationship(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<HumanRelationship>, StorageError> {
    human_relationships::table
        .filter(human_relationships::app_id.eq(&ctx.app_id))
        .filter(human_relationships::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// List human relationships with filtering - scoped by app
pub fn list_human_relationships(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &HumanRelationshipQuery,
) -> Result<Vec<HumanRelationship>, StorageError> {
    let mut base_query = human_relationships::table
        .filter(human_relationships::app_id.eq(&ctx.app_id))
        .into_boxed();

    // Filter by party involvement
    if let Some(ref party_id) = query.party_id {
        base_query = base_query.filter(
            human_relationships::party_a_id.eq(party_id)
                .or(human_relationships::party_b_id.eq(party_id))
        );
    }

    // Apply optional filters
    if let Some(ref rel_type) = query.relationship_type {
        base_query = base_query.filter(human_relationships::relationship_type.eq(rel_type));
    }

    if let Some(ref level) = query.intimacy_level {
        base_query = base_query.filter(human_relationships::intimacy_level.eq(level));
    }

    // Filter by minimum intimacy level using index comparison
    if let Some(ref min_level) = query.min_intimacy_level {
        if let Some(min_index) = intimacy_levels::index_of(min_level) {
            // Get all levels at or above this index
            let valid_levels: Vec<&str> = intimacy_levels::ALL
                .iter()
                .enumerate()
                .filter(|(i, _)| *i >= min_index)
                .map(|(_, level)| *level)
                .collect();
            base_query = base_query.filter(human_relationships::intimacy_level.eq_any(valid_levels));
        }
    }

    // Filter for mutual consent
    if query.mutual_consent_only == Some(true) {
        base_query = base_query
            .filter(human_relationships::consent_given_by_a.eq(1))
            .filter(human_relationships::consent_given_by_b.eq(1));
    }

    // Filter for custody enabled
    if query.custody_enabled_only == Some(true) {
        base_query = base_query.filter(
            human_relationships::custody_enabled_by_a.eq(1)
                .or(human_relationships::custody_enabled_by_b.eq(1))
        );
    }

    base_query
        .order(human_relationships::created_at.desc())
        .limit(query.limit)
        .offset(query.offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get all relationships for a human (either party)
pub fn get_relationships_for_human(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
) -> Result<Vec<HumanRelationship>, StorageError> {
    list_human_relationships(conn, ctx, &HumanRelationshipQuery {
        party_id: Some(human_id.to_string()),
        ..Default::default()
    })
}

/// Get relationship between two specific humans
pub fn get_relationship_between(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    party_a_id: &str,
    party_b_id: &str,
    relationship_type: Option<&str>,
) -> Result<Vec<HumanRelationship>, StorageError> {
    let mut base_query = human_relationships::table
        .filter(human_relationships::app_id.eq(&ctx.app_id))
        .filter(
            (human_relationships::party_a_id.eq(party_a_id)
                .and(human_relationships::party_b_id.eq(party_b_id)))
            .or(human_relationships::party_a_id.eq(party_b_id)
                .and(human_relationships::party_b_id.eq(party_a_id)))
        )
        .into_boxed();

    if let Some(rel_type) = relationship_type {
        base_query = base_query.filter(human_relationships::relationship_type.eq(rel_type));
    }

    base_query
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get trusted contacts (custody-enabled relationships)
pub fn get_trusted_contacts(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
) -> Result<Vec<HumanRelationship>, StorageError> {
    human_relationships::table
        .filter(human_relationships::app_id.eq(&ctx.app_id))
        .filter(
            human_relationships::party_a_id.eq(human_id)
                .or(human_relationships::party_b_id.eq(human_id))
        )
        .filter(
            human_relationships::custody_enabled_by_a.eq(1)
                .or(human_relationships::custody_enabled_by_b.eq(1))
        )
        .filter(human_relationships::consent_given_by_a.eq(1))
        .filter(human_relationships::consent_given_by_b.eq(1))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Write Operations
// ============================================================================

/// Create a single human relationship - scoped by app
pub fn create_human_relationship(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateHumanRelationshipInput,
) -> Result<HumanRelationship, StorageError> {
    // Validate intimacy level
    if !intimacy_levels::is_valid(&input.intimacy_level) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid intimacy level: {}. Valid levels: {:?}",
            input.intimacy_level,
            intimacy_levels::ALL
        )));
    }

    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let new_rel = NewHumanRelationship {
        id: &id,
        app_id: &ctx.app_id,
        party_a_id: &input.party_a_id,
        party_b_id: &input.party_b_id,
        relationship_type: &input.relationship_type,
        intimacy_level: &input.intimacy_level,
        is_bidirectional: if input.is_bidirectional { 1 } else { 0 },
        consent_given_by_a: if input.consent_given_by_a { 1 } else { 0 },
        consent_given_by_b: if input.consent_given_by_b { 1 } else { 0 },
        custody_enabled_by_a: 0,
        custody_enabled_by_b: 0,
        auto_custody_enabled: 0,
        emergency_access_enabled: 0,
        initiated_by: &input.initiated_by,
        verified_at: None,
        governance_layer: input.governance_layer.as_deref(),
        reach: &input.reach,
        context_json: input.context_json.as_deref(),
        expires_at: input.expires_at.as_deref(),
    };

    diesel::insert_into(human_relationships::table)
        .values(&new_rel)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    get_human_relationship(conn, ctx, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created relationship".into()))
}

/// Update consent status for a party
pub fn update_consent(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    party_id: &str,
    consent: &ConsentUpdate,
) -> Result<HumanRelationship, StorageError> {
    // First get the relationship to determine which party
    let rel = get_human_relationship(conn, ctx, id)?
        .ok_or_else(|| StorageError::NotFound(format!("Relationship {} not found", id)))?;

    let consent_value = if consent.consent_given { 1 } else { 0 };

    if rel.party_a_id == party_id {
        diesel::update(
            human_relationships::table
                .filter(human_relationships::app_id.eq(&ctx.app_id))
                .filter(human_relationships::id.eq(id))
        )
        .set((
            human_relationships::consent_given_by_a.eq(consent_value),
            human_relationships::updated_at.eq(current_timestamp()),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;
    } else if rel.party_b_id == party_id {
        diesel::update(
            human_relationships::table
                .filter(human_relationships::app_id.eq(&ctx.app_id))
                .filter(human_relationships::id.eq(id))
        )
        .set((
            human_relationships::consent_given_by_b.eq(consent_value),
            human_relationships::updated_at.eq(current_timestamp()),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;
    } else {
        return Err(StorageError::InvalidInput(format!(
            "Party {} is not part of relationship {}",
            party_id, id
        )));
    }

    get_human_relationship(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated relationship".into()))
}

/// Update custody settings for a party
pub fn update_custody(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    party_id: &str,
    custody: &CustodyUpdate,
) -> Result<HumanRelationship, StorageError> {
    // First get the relationship to determine which party
    let rel = get_human_relationship(conn, ctx, id)?
        .ok_or_else(|| StorageError::NotFound(format!("Relationship {} not found", id)))?;

    let custody_value = if custody.custody_enabled { 1 } else { 0 };

    // Check that mutual consent exists before enabling custody
    if custody.custody_enabled {
        if rel.consent_given_by_a != 1 || rel.consent_given_by_b != 1 {
            return Err(StorageError::InvalidInput(
                "Cannot enable custody without mutual consent".into()
            ));
        }
    }

    if rel.party_a_id == party_id {
        diesel::update(
            human_relationships::table
                .filter(human_relationships::app_id.eq(&ctx.app_id))
                .filter(human_relationships::id.eq(id))
        )
        .set((
            human_relationships::custody_enabled_by_a.eq(custody_value),
            human_relationships::auto_custody_enabled.eq(
                custody.auto_custody_enabled.map(|v| if v { 1 } else { 0 }).unwrap_or(rel.auto_custody_enabled)
            ),
            human_relationships::emergency_access_enabled.eq(
                custody.emergency_access_enabled.map(|v| if v { 1 } else { 0 }).unwrap_or(rel.emergency_access_enabled)
            ),
            human_relationships::updated_at.eq(current_timestamp()),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;
    } else if rel.party_b_id == party_id {
        diesel::update(
            human_relationships::table
                .filter(human_relationships::app_id.eq(&ctx.app_id))
                .filter(human_relationships::id.eq(id))
        )
        .set((
            human_relationships::custody_enabled_by_b.eq(custody_value),
            human_relationships::auto_custody_enabled.eq(
                custody.auto_custody_enabled.map(|v| if v { 1 } else { 0 }).unwrap_or(rel.auto_custody_enabled)
            ),
            human_relationships::emergency_access_enabled.eq(
                custody.emergency_access_enabled.map(|v| if v { 1 } else { 0 }).unwrap_or(rel.emergency_access_enabled)
            ),
            human_relationships::updated_at.eq(current_timestamp()),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;
    } else {
        return Err(StorageError::InvalidInput(format!(
            "Party {} is not part of relationship {}",
            party_id, id
        )));
    }

    // Check for auto-custody trigger on intimate relationships
    let updated = get_human_relationship(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated relationship".into()))?;

    // Auto-enable custody at intimate level if both parties have enabled custody
    if intimacy_levels::triggers_auto_custody(&updated.intimacy_level) {
        if updated.custody_enabled_by_a == 1 && updated.custody_enabled_by_b == 1 {
            diesel::update(
                human_relationships::table
                    .filter(human_relationships::app_id.eq(&ctx.app_id))
                    .filter(human_relationships::id.eq(id))
            )
            .set(human_relationships::auto_custody_enabled.eq(1))
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Auto-custody update failed: {}", e)))?;
        }
    }

    get_human_relationship(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated relationship".into()))
}

/// Update intimacy level
pub fn update_intimacy_level(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    new_level: &str,
) -> Result<HumanRelationship, StorageError> {
    if !intimacy_levels::is_valid(new_level) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid intimacy level: {}. Valid levels: {:?}",
            new_level,
            intimacy_levels::ALL
        )));
    }

    diesel::update(
        human_relationships::table
            .filter(human_relationships::app_id.eq(&ctx.app_id))
            .filter(human_relationships::id.eq(id))
    )
    .set((
        human_relationships::intimacy_level.eq(new_level),
        human_relationships::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    get_human_relationship(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated relationship".into()))
}

/// Mark relationship as verified
pub fn verify_relationship(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<HumanRelationship, StorageError> {
    diesel::update(
        human_relationships::table
            .filter(human_relationships::app_id.eq(&ctx.app_id))
            .filter(human_relationships::id.eq(id))
    )
    .set((
        human_relationships::verified_at.eq(current_timestamp()),
        human_relationships::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    get_human_relationship(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated relationship".into()))
}

/// Delete a human relationship by ID - scoped by app
pub fn delete_human_relationship(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<bool, StorageError> {
    let deleted = diesel::delete(
        human_relationships::table
            .filter(human_relationships::app_id.eq(&ctx.app_id))
            .filter(human_relationships::id.eq(id))
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    Ok(deleted > 0)
}

// ============================================================================
// Stats
// ============================================================================

/// Get human relationship count for an app
pub fn human_relationship_count(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<i64, StorageError> {
    human_relationships::table
        .filter(human_relationships::app_id.eq(&ctx.app_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

/// Get relationship statistics by intimacy level
pub fn stats_by_intimacy(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<Vec<(String, i64)>, StorageError> {
    human_relationships::table
        .filter(human_relationships::app_id.eq(&ctx.app_id))
        .group_by(human_relationships::intimacy_level)
        .select((human_relationships::intimacy_level, diesel::dsl::count_star()))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Stats query failed: {}", e)))
}
