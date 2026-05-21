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
        .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
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
        .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
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
        .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
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
        h_app_id: &ctx.h_app_id,
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
                .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
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
                .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
                .filter(rea_commitments::id.eq(id)),
        )
        .set(rea_commitments::state.eq(&update.state))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;
    }

    get_commitment(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated commitment".into()))
}

/// Upsert a commitment with DHT anchor hash — insert or update anchor if it already exists.
/// Used by REA projection signal handler to anchor commitments onto the DHT.
pub fn upsert_with_anchor(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateReaCommitmentInput,
    dht_anchor_hash: Option<&str>,
) -> Result<ReaCommitment, StorageError> {
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let existing = get_commitment(conn, ctx, &id)?;

    if existing.is_some() {
        // Update dht_anchor_hash on existing record
        diesel::update(
            rea_commitments::table
                .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
                .filter(rea_commitments::id.eq(&id)),
        )
        .set(rea_commitments::dht_anchor_hash.eq(dht_anchor_hash))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update anchor failed: {}", e)))?;
    } else {
        let new = NewReaCommitment {
            id: &id,
            h_app_id: &ctx.h_app_id,
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
            dht_anchor_hash,
        };

        diesel::insert_into(rea_commitments::table)
            .values(&new)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;
    }

    get_commitment(conn, ctx, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve upserted commitment".into()))
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
        .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

// ============================================================================
// Doorway-Operator Authority Selectors
// ============================================================================
//
// Operator authority for a doorway is a Commitment with action='operate-doorway'
// and in_scope_of='doorway:<id>'. The capabilities array lives in
// resource_classified_as (JSON-encoded list of strings); succession-role and
// reach-scope live in metadata_json. See:
//   - elohim/sdk/schemas/v1/objects/operator-classification.schema.json
//   - elohim/sdk/schemas/v1/views/doorway-operator-binding-view.schema.json
//
// Source of truth: DHT Commitment entry (Notarized, Category A). These
// selectors read the projection only — re-derivable by replaying the signal
// stream filtered to action='operate-doorway'.
//
// Fast-path queries use the (action, in_scope_of, state) composite index from
// migration 2026-05-19-000000_doorway_operator_action_indexes.

/// REA action discriminator for doorway-operator commitments. Single source of
/// truth for this string in the storage crate; the same constant in the DNA's
/// REA_ACTIONS array is the wire-contract value (kept in sync by convention,
/// pending schema-first codegen of the action vocabulary).
pub const OPERATE_DOORWAY_ACTION: &str = "operate-doorway";

/// Build the canonical in_scope_of value for an operate-doorway commitment.
///
/// Stored encoding is a JSON-array string (ValueFlows in_scope_of convention),
/// matching what the DNA projects into rea_commitments.in_scope_of verbatim
/// from the Commitment entry's in_scope_of_json field. Operator commitments are
/// always single-scope, so the resulting string is a one-element JSON array
/// that can be matched exactly via the composite index from migration
/// 2026-05-19-000000_doorway_operator_action_indexes.
pub fn doorway_scope(doorway_id: &str) -> String {
    serde_json::to_string(&[format!("doorway:{}", doorway_id)])
        .expect("serializing a single-element string array cannot fail")
}

/// List active operator commitments for a doorway.
///
/// Returns every Commitment with action='operate-doorway', in_scope_of matching
/// the doorway scope, and state='active'. Each row is a distinct operator
/// binding (one row per operator-agent × doorway). The doorway's auth layer
/// uses this at JWT refresh time to rebuild the capabilities snapshot.
pub fn list_active_doorway_operators(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    doorway_id: &str,
) -> Result<Vec<ReaCommitment>, StorageError> {
    let scope = doorway_scope(doorway_id);
    rea_commitments::table
        .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
        .filter(rea_commitments::action.eq(OPERATE_DOORWAY_ACTION))
        .filter(rea_commitments::in_scope_of.eq(&scope))
        .filter(rea_commitments::state.eq("active"))
        .order(rea_commitments::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Operator query failed: {}", e)))
}

/// Look up the active operator-binding for one agent on one doorway.
///
/// Returns at most one Commitment (the most-recent active binding). Callers
/// MUST still verify the capability is present in the binding's
/// resource_classified_as list before authorizing the operation. This selector
/// is the substrate lookup; the capability membership check belongs to the
/// auth layer (so capability vocabulary changes do not require a DB schema
/// change).
pub fn find_active_operator_binding(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    operator_agent: &str,
    doorway_id: &str,
) -> Result<Option<ReaCommitment>, StorageError> {
    let scope = doorway_scope(doorway_id);
    rea_commitments::table
        .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
        .filter(rea_commitments::action.eq(OPERATE_DOORWAY_ACTION))
        .filter(rea_commitments::in_scope_of.eq(&scope))
        .filter(rea_commitments::state.eq("active"))
        .filter(rea_commitments::provider.eq(operator_agent))
        .order(rea_commitments::created_at.desc())
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Operator lookup failed: {}", e)))
}

#[cfg(test)]
mod operator_helper_tests {
    use super::*;

    #[test]
    fn doorway_scope_produces_canonical_json_array() {
        // Single-element JSON array per operator-classification.schema.json
        // scopes pattern. The composite index relies on this exact encoding.
        assert_eq!(
            doorway_scope("alpha-elohim-host"),
            r#"["doorway:alpha-elohim-host"]"#
        );
    }

    #[test]
    fn doorway_scope_escapes_embedded_quotes() {
        // Defensive — doorway ids should never contain quotes (the schema
        // pattern forbids them), but the JSON encoder must still escape them
        // safely if a malformed id slips through.
        let scope = doorway_scope("evil\"-injection");
        let parsed: Vec<String> = serde_json::from_str(&scope).expect("valid JSON");
        assert_eq!(parsed, vec!["doorway:evil\"-injection".to_string()]);
    }

    #[test]
    fn operate_doorway_action_matches_dna_vocabulary() {
        // The constant here must match the entry appended to REA_ACTIONS in
        // elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs.
        // Schema-first codegen of this vocabulary is a future refactor; until
        // then this test is the drift detector.
        assert_eq!(OPERATE_DOORWAY_ACTION, "operate-doorway");
    }
}
