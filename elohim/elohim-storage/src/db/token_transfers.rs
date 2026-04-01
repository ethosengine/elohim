//! Token transfer CRUD operations using Diesel with app scoping.
//!
//! Records witnessed peer-to-peer token exchanges between agents.
//! Category A (notarized): each transfer is an immutable event record.

use diesel::prelude::*;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::token_transfers;
use super::models::{NewTokenTransfer, TokenTransfer};
use crate::error::StorageError;

// ============================================================================
// Write Operations
// ============================================================================

/// Record a new token transfer — scoped by app.
///
/// The `id` field on `new` is ignored if empty; a UUID is generated instead.
pub fn create_transfer(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    new: NewTokenTransfer<'_>,
) -> Result<TokenTransfer, StorageError> {
    let generated_id;
    let id = if new.id.is_empty() {
        generated_id = Uuid::new_v4().to_string();
        generated_id.as_str()
    } else {
        new.id
    };

    let record = NewTokenTransfer {
        id,
        h_app_id: &ctx.h_app_id,
        from_agent: new.from_agent,
        to_agent: new.to_agent,
        amount: new.amount,
        governance_layer: new.governance_layer,
        note: new.note,
        dht_anchor_hash: new.dht_anchor_hash,
    };

    diesel::insert_into(token_transfers::table)
        .values(&record)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to insert token transfer: {}", e)))?;

    token_transfers::table
        .filter(token_transfers::id.eq(id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to fetch inserted transfer: {}", e)))
}

// ============================================================================
// Read Operations
// ============================================================================

/// List all transfers where the agent is either sender or receiver — scoped by app.
pub fn get_transfers_for_agent(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
) -> Result<Vec<TokenTransfer>, StorageError> {
    token_transfers::table
        .filter(token_transfers::h_app_id.eq(&ctx.h_app_id))
        .filter(
            token_transfers::from_agent
                .eq(agent_id)
                .or(token_transfers::to_agent.eq(agent_id)),
        )
        .order(token_transfers::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}
