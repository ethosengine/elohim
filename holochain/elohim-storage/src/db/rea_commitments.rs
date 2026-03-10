//! REA Commitment CRUD operations
//!
//! hREA/ValueFlows compatible commitment tracking for the learning economy.
//! Tracks binding promises of future economic activity (compute sharing,
//! content provision, assessment availability, etc.).

use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::rea_commitments;
use super::models::{NewReaCommitment, ReaCommitment};
use crate::error::StorageError;

// ============================================================================
// Query Types
// ============================================================================

/// Input for creating an REA commitment
#[derive(Debug, Clone, Deserialize)]
pub struct CreateReaCommitmentInput {
    #[serde(default)]
    pub id: Option<String>,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_conforms_to: Option<String>,
    #[serde(default)]
    pub resource_classified_as: Option<String>,
    #[serde(default)]
    pub resource_quantity_value: Option<f32>,
    #[serde(default)]
    pub resource_quantity_unit: Option<String>,
    #[serde(default)]
    pub effort_quantity_value: Option<f32>,
    #[serde(default)]
    pub effort_quantity_unit: Option<String>,
    #[serde(default)]
    pub has_beginning: Option<String>,
    #[serde(default)]
    pub has_end: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub clause_of: Option<String>,
    #[serde(default)]
    pub in_scope_of: Option<String>,
    #[serde(default)]
    pub medium_of_exchange_id: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
}

/// Query parameters for listing REA commitments - camelCase for URL params
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaCommitmentQuery {
    /// Filter by action type
    pub action: Option<String>,
    /// Filter by provider agent
    pub provider: Option<String>,
    /// Filter by receiver agent
    pub receiver: Option<String>,
    /// Filter by commitment state
    pub state: Option<String>,
    /// Filter by agreement/clause
    pub clause_of: Option<String>,
    /// Filter by medium of exchange
    pub medium_of_exchange_id: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

/// Input for updating commitment state
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateReaCommitmentState {
    pub state: String,
    #[serde(default)]
    pub finished: Option<bool>,
}

// ============================================================================
// Read Operations
// ============================================================================

/// Get commitment by ID - scoped by app
pub fn get_commitment(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<ReaCommitment>, StorageError> {
    rea_commitments::table
        .filter(rea_commitments::app_id.eq(&ctx.app_id))
        .filter(rea_commitments::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// List commitments with filtering - scoped by app
pub fn list_commitments(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &ReaCommitmentQuery,
) -> Result<Vec<ReaCommitment>, StorageError> {
    let mut base_query = rea_commitments::table
        .filter(rea_commitments::app_id.eq(&ctx.app_id))
        .into_boxed();

    if let Some(ref action) = query.action {
        base_query = base_query.filter(rea_commitments::action.eq(action));
    }

    if let Some(ref provider) = query.provider {
        base_query = base_query.filter(rea_commitments::provider.eq(provider));
    }

    if let Some(ref receiver) = query.receiver {
        base_query = base_query.filter(rea_commitments::receiver.eq(receiver));
    }

    if let Some(ref state) = query.state {
        base_query = base_query.filter(rea_commitments::state.eq(state));
    }

    if let Some(ref clause) = query.clause_of {
        base_query = base_query.filter(rea_commitments::clause_of.eq(clause));
    }

    if let Some(ref medium) = query.medium_of_exchange_id {
        base_query = base_query.filter(rea_commitments::medium_of_exchange_id.eq(medium));
    }

    base_query
        .order(rea_commitments::created_at.desc())
        .limit(query.limit)
        .offset(query.offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get commitments for an agent (as provider or receiver)
pub fn get_commitments_for_agent(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
    limit: i64,
) -> Result<Vec<ReaCommitment>, StorageError> {
    rea_commitments::table
        .filter(rea_commitments::app_id.eq(&ctx.app_id))
        .filter(
            rea_commitments::provider
                .eq(agent_id)
                .or(rea_commitments::receiver.eq(agent_id)),
        )
        .order(rea_commitments::created_at.desc())
        .limit(limit)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Write Operations
// ============================================================================

/// Create an REA commitment - scoped by app
pub fn create_commitment(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateReaCommitmentInput,
) -> Result<ReaCommitment, StorageError> {
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let new = NewReaCommitment {
        id: &id,
        app_id: &ctx.app_id,
        action: &input.action,
        provider: &input.provider,
        receiver: &input.receiver,
        resource_conforms_to: input.resource_conforms_to.as_deref(),
        resource_classified_as: input.resource_classified_as.as_deref(),
        resource_quantity_value: input.resource_quantity_value,
        resource_quantity_unit: input.resource_quantity_unit.as_deref(),
        effort_quantity_value: input.effort_quantity_value,
        effort_quantity_unit: input.effort_quantity_unit.as_deref(),
        has_beginning: input.has_beginning.as_deref(),
        has_end: input.has_end.as_deref(),
        due: input.due.as_deref(),
        clause_of: input.clause_of.as_deref(),
        in_scope_of: input.in_scope_of.as_deref(),
        medium_of_exchange_id: input.medium_of_exchange_id.as_deref(),
        state: "proposed",
        finished: 0,
        note: input.note.as_deref(),
        metadata_json: input.metadata_json.as_deref(),
        dht_anchor_hash: None,
    };

    diesel::insert_into(rea_commitments::table)
        .values(&new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    get_commitment(conn, ctx, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created commitment".into()))
}

/// Update commitment state
pub fn update_commitment_state(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    update: &UpdateReaCommitmentState,
) -> Result<ReaCommitment, StorageError> {
    let finished_val = update.finished.map(|b| if b { 1 } else { 0 });

    if let Some(f) = finished_val {
        diesel::update(
            rea_commitments::table
                .filter(rea_commitments::app_id.eq(&ctx.app_id))
                .filter(rea_commitments::id.eq(id)),
        )
        .set((
            rea_commitments::state.eq(&update.state),
            rea_commitments::finished.eq(f),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;
    } else {
        diesel::update(
            rea_commitments::table
                .filter(rea_commitments::app_id.eq(&ctx.app_id))
                .filter(rea_commitments::id.eq(id)),
        )
        .set(rea_commitments::state.eq(&update.state))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;
    }

    get_commitment(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated commitment".into()))
}

// ============================================================================
// Stats
// ============================================================================

/// Get commitment count for an app
pub fn commitment_count(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<i64, StorageError> {
    rea_commitments::table
        .filter(rea_commitments::app_id.eq(&ctx.app_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}
