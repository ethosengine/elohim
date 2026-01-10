//! Stewardship allocations CRUD operations
//!
//! Content stewardship with allocation ratios - one-to-many relationship
//! where content is stewarded by multiple contributors with weighted allocations.

use diesel::prelude::*;
use serde::Deserialize;
use tracing::debug;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::stewardship_allocations;
use super::models::{
    StewardshipAllocation, NewStewardshipAllocation,
    StewardshipAllocationWithPresence, ContentStewardship,
    ContributorPresence, current_timestamp,
    allocation_methods, contribution_types, governance_states,
};
use crate::error::StorageError;

// ============================================================================
// Input Types
// ============================================================================

/// Input for creating a stewardship allocation
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAllocationInput {
    pub content_id: String,
    pub steward_presence_id: String,
    #[serde(default = "default_allocation_ratio")]
    pub allocation_ratio: f32,
    #[serde(default = "default_allocation_method")]
    pub allocation_method: String,
    #[serde(default = "default_contribution_type")]
    pub contribution_type: String,
    pub contribution_evidence_json: Option<String>,
    pub note: Option<String>,
    pub metadata_json: Option<String>,
}

fn default_allocation_ratio() -> f32 {
    1.0
}

fn default_allocation_method() -> String {
    allocation_methods::MANUAL.to_string()
}

fn default_contribution_type() -> String {
    contribution_types::INHERITED.to_string()
}

/// Input for updating an allocation
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAllocationInput {
    pub allocation_ratio: Option<f32>,
    pub allocation_method: Option<String>,
    pub contribution_type: Option<String>,
    pub contribution_evidence_json: Option<String>,
    pub governance_state: Option<String>,
    pub dispute_id: Option<String>,
    pub dispute_reason: Option<String>,
    pub elohim_ratified_at: Option<String>,
    pub elohim_ratifier_id: Option<String>,
    pub note: Option<String>,
}

/// Query parameters for listing allocations
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AllocationQuery {
    pub content_id: Option<String>,
    pub steward_presence_id: Option<String>,
    pub governance_state: Option<String>,
    pub active_only: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// CRUD Operations
// ============================================================================

/// Create a new stewardship allocation
pub fn create_allocation(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: &CreateAllocationInput,
) -> Result<StewardshipAllocation, StorageError> {
    let id = Uuid::new_v4().to_string();

    // Validate allocation method
    if !allocation_methods::is_valid(&input.allocation_method) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid allocation method: {}",
            input.allocation_method
        )));
    }

    // Validate contribution type
    if !contribution_types::is_valid(&input.contribution_type) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid contribution type: {}",
            input.contribution_type
        )));
    }

    // Validate ratio
    if input.allocation_ratio < 0.0 || input.allocation_ratio > 1.0 {
        return Err(StorageError::InvalidInput(format!(
            "Allocation ratio must be between 0.0 and 1.0, got: {}",
            input.allocation_ratio
        )));
    }

    let new_allocation = NewStewardshipAllocation {
        id: &id,
        app_id: ctx.app_id(),
        content_id: &input.content_id,
        steward_presence_id: &input.steward_presence_id,
        allocation_ratio: input.allocation_ratio,
        allocation_method: &input.allocation_method,
        contribution_type: &input.contribution_type,
        contribution_evidence_json: input.contribution_evidence_json.as_deref(),
        governance_state: governance_states::ACTIVE,
        note: input.note.as_deref(),
        metadata_json: input.metadata_json.as_deref(),
    };

    diesel::insert_into(stewardship_allocations::table)
        .values(&new_allocation)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to create allocation: {}", e)))?;

    debug!("Created stewardship allocation {} for content {}", id, input.content_id);

    get_allocation_by_id(conn, ctx, &id)
}

/// Get an allocation by ID
pub fn get_allocation_by_id(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<StewardshipAllocation, StorageError> {
    stewardship_allocations::table
        .filter(stewardship_allocations::id.eq(id))
        .filter(stewardship_allocations::app_id.eq(ctx.app_id()))
        .first::<StewardshipAllocation>(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => StorageError::NotFound(id.to_string()),
            _ => StorageError::Internal(format!("Failed to get allocation: {}", e)),
        })
}

/// List allocations with optional filters
pub fn list_allocations(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &AllocationQuery,
) -> Result<Vec<StewardshipAllocation>, StorageError> {
    let mut q = stewardship_allocations::table
        .filter(stewardship_allocations::app_id.eq(ctx.app_id()))
        .into_boxed();

    if let Some(content_id) = &query.content_id {
        q = q.filter(stewardship_allocations::content_id.eq(content_id));
    }

    if let Some(steward_id) = &query.steward_presence_id {
        q = q.filter(stewardship_allocations::steward_presence_id.eq(steward_id));
    }

    if let Some(state) = &query.governance_state {
        q = q.filter(stewardship_allocations::governance_state.eq(state));
    }

    if query.active_only.unwrap_or(false) {
        q = q.filter(stewardship_allocations::governance_state.eq(governance_states::ACTIVE));
        q = q.filter(stewardship_allocations::effective_until.is_null());
    }

    // Order by allocation_ratio descending (primary steward first)
    q = q.order(stewardship_allocations::allocation_ratio.desc());

    if let Some(limit) = query.limit {
        q = q.limit(limit);
    }

    if let Some(offset) = query.offset {
        q = q.offset(offset);
    }

    q.load::<StewardshipAllocation>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list allocations: {}", e)))
}

/// Get all allocations for a content piece
pub fn get_allocations_for_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<Vec<StewardshipAllocation>, StorageError> {
    list_allocations(conn, ctx, &AllocationQuery {
        content_id: Some(content_id.to_string()),
        active_only: Some(true),
        ..Default::default()
    })
}

/// Get all allocations for a steward
pub fn get_allocations_for_steward(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    steward_presence_id: &str,
) -> Result<Vec<StewardshipAllocation>, StorageError> {
    list_allocations(conn, ctx, &AllocationQuery {
        steward_presence_id: Some(steward_presence_id.to_string()),
        active_only: Some(true),
        ..Default::default()
    })
}

/// Get content stewardship aggregate (all allocations with steward presences)
pub fn get_content_stewardship(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<ContentStewardship, StorageError> {
    use super::diesel_schema::contributor_presences;

    let allocations = get_allocations_for_content(conn, ctx, content_id)?;

    // Get steward presences for all allocations
    let steward_ids: Vec<&str> = allocations.iter()
        .map(|a| a.steward_presence_id.as_str())
        .collect();

    let presences: Vec<ContributorPresence> = contributor_presences::table
        .filter(contributor_presences::id.eq_any(&steward_ids))
        .filter(contributor_presences::app_id.eq(ctx.app_id()))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to load presences: {}", e)))?;

    // Build lookup map
    let presence_map: std::collections::HashMap<&str, &ContributorPresence> = presences.iter()
        .map(|p| (p.id.as_str(), p))
        .collect();

    // Build allocations with presences
    let allocations_with_presences: Vec<StewardshipAllocationWithPresence> = allocations.iter()
        .map(|a| StewardshipAllocationWithPresence {
            allocation: a.clone(),
            steward: presence_map.get(a.steward_presence_id.as_str()).map(|p| (*p).clone()),
        })
        .collect();

    // Find primary steward (highest allocation)
    let primary_steward = allocations.first().cloned();

    // Calculate totals
    let total_allocation: f32 = allocations.iter().map(|a| a.allocation_ratio).sum();
    let has_disputes = allocations.iter().any(|a| a.governance_state == governance_states::DISPUTED);

    Ok(ContentStewardship {
        content_id: content_id.to_string(),
        allocations: allocations_with_presences,
        total_allocation,
        has_disputes,
        primary_steward,
    })
}

/// Update an allocation
pub fn update_allocation(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    input: &UpdateAllocationInput,
) -> Result<StewardshipAllocation, StorageError> {
    // Verify allocation exists
    let _ = get_allocation_by_id(conn, ctx, id)?;

    let now = current_timestamp();

    // Build update query dynamically
    diesel::update(stewardship_allocations::table)
        .filter(stewardship_allocations::id.eq(id))
        .filter(stewardship_allocations::app_id.eq(ctx.app_id()))
        .set((
            stewardship_allocations::updated_at.eq(&now),
            input.allocation_ratio.map(|r| stewardship_allocations::allocation_ratio.eq(r)),
            input.allocation_method.as_ref().map(|m| stewardship_allocations::allocation_method.eq(m)),
            input.contribution_type.as_ref().map(|t| stewardship_allocations::contribution_type.eq(t)),
            input.governance_state.as_ref().map(|s| stewardship_allocations::governance_state.eq(s)),
            input.dispute_id.as_ref().map(|d| stewardship_allocations::dispute_id.eq(d)),
            input.dispute_reason.as_ref().map(|r| stewardship_allocations::dispute_reason.eq(r)),
            input.elohim_ratified_at.as_ref().map(|t| stewardship_allocations::elohim_ratified_at.eq(t)),
            input.elohim_ratifier_id.as_ref().map(|r| stewardship_allocations::elohim_ratifier_id.eq(r)),
            input.note.as_ref().map(|n| stewardship_allocations::note.eq(n)),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to update allocation: {}", e)))?;

    get_allocation_by_id(conn, ctx, id)
}

/// Supersede an allocation (mark as superseded and optionally create replacement)
pub fn supersede_allocation(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    replacement_id: Option<&str>,
) -> Result<(), StorageError> {
    let now = current_timestamp();

    diesel::update(stewardship_allocations::table)
        .filter(stewardship_allocations::id.eq(id))
        .filter(stewardship_allocations::app_id.eq(ctx.app_id()))
        .set((
            stewardship_allocations::governance_state.eq(governance_states::SUPERSEDED),
            stewardship_allocations::effective_until.eq(&now),
            stewardship_allocations::superseded_by.eq(replacement_id),
            stewardship_allocations::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to supersede allocation: {}", e)))?;

    Ok(())
}

/// Record recognition accumulation for an allocation
pub fn accumulate_recognition(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    recognition_amount: f32,
) -> Result<StewardshipAllocation, StorageError> {
    let now = current_timestamp();

    diesel::update(stewardship_allocations::table)
        .filter(stewardship_allocations::id.eq(id))
        .filter(stewardship_allocations::app_id.eq(ctx.app_id()))
        .set((
            stewardship_allocations::recognition_accumulated.eq(
                stewardship_allocations::recognition_accumulated + recognition_amount
            ),
            stewardship_allocations::last_recognition_at.eq(&now),
            stewardship_allocations::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to accumulate recognition: {}", e)))?;

    get_allocation_by_id(conn, ctx, id)
}

/// File a dispute on an allocation
pub fn file_dispute(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    allocation_id: &str,
    dispute_id: &str,
    disputed_by: &str,
    reason: &str,
) -> Result<StewardshipAllocation, StorageError> {
    let now = current_timestamp();

    diesel::update(stewardship_allocations::table)
        .filter(stewardship_allocations::id.eq(allocation_id))
        .filter(stewardship_allocations::app_id.eq(ctx.app_id()))
        .set((
            stewardship_allocations::governance_state.eq(governance_states::DISPUTED),
            stewardship_allocations::dispute_id.eq(dispute_id),
            stewardship_allocations::dispute_reason.eq(reason),
            stewardship_allocations::disputed_at.eq(&now),
            stewardship_allocations::disputed_by.eq(disputed_by),
            stewardship_allocations::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to file dispute: {}", e)))?;

    get_allocation_by_id(conn, ctx, allocation_id)
}

/// Resolve a dispute (Elohim ratification)
pub fn resolve_dispute(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    allocation_id: &str,
    ratifier_id: &str,
    new_state: &str,
) -> Result<StewardshipAllocation, StorageError> {
    let now = current_timestamp();

    if !governance_states::is_valid(new_state) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid governance state: {}",
            new_state
        )));
    }

    diesel::update(stewardship_allocations::table)
        .filter(stewardship_allocations::id.eq(allocation_id))
        .filter(stewardship_allocations::app_id.eq(ctx.app_id()))
        .set((
            stewardship_allocations::governance_state.eq(new_state),
            stewardship_allocations::elohim_ratified_at.eq(&now),
            stewardship_allocations::elohim_ratifier_id.eq(ratifier_id),
            stewardship_allocations::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to resolve dispute: {}", e)))?;

    get_allocation_by_id(conn, ctx, allocation_id)
}

/// Delete an allocation (hard delete - use supersede for soft delete)
pub fn delete_allocation(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<(), StorageError> {
    let deleted = diesel::delete(stewardship_allocations::table)
        .filter(stewardship_allocations::id.eq(id))
        .filter(stewardship_allocations::app_id.eq(ctx.app_id()))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to delete allocation: {}", e)))?;

    if deleted == 0 {
        return Err(StorageError::NotFound(id.to_string()));
    }

    Ok(())
}

/// Count allocations by governance state
pub fn count_by_state(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<std::collections::HashMap<String, i64>, StorageError> {
    use diesel::dsl::count_star;

    let results: Vec<(String, i64)> = stewardship_allocations::table
        .filter(stewardship_allocations::app_id.eq(ctx.app_id()))
        .group_by(stewardship_allocations::governance_state)
        .select((stewardship_allocations::governance_state, count_star()))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to count allocations: {}", e)))?;

    Ok(results.into_iter().collect())
}
