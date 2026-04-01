//! Token mint event CRUD operations using Diesel with app scoping.
//!
//! Every mint is an immutable record coupled to a witnessed REA economic event.
//! Category A (notarized): source of truth is the DHT; storage is a projection.

use diesel::prelude::*;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::token_mint_events;
use super::models::{NewTokenMintEvent, TokenMintEvent};
use crate::error::StorageError;

// ============================================================================
// Write Operations
// ============================================================================

/// Record a new token mint event — scoped by app.
///
/// The `id` field on `new` is ignored if empty; a UUID is generated instead.
pub fn create_mint_event(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    new: NewTokenMintEvent<'_>,
) -> Result<TokenMintEvent, StorageError> {
    let generated_id;
    let id = if new.id.is_empty() {
        generated_id = Uuid::new_v4().to_string();
        generated_id.as_str()
    } else {
        new.id
    };

    let record = NewTokenMintEvent {
        id,
        h_app_id: &ctx.h_app_id,
        amount: new.amount,
        provenance_event_id: new.provenance_event_id,
        mint_tier: new.mint_tier,
        source_epr_id: new.source_epr_id,
        agent_id: new.agent_id,
        constitutional_context: new.constitutional_context,
        elohim_attestation: new.elohim_attestation,
        reasoning_trace: new.reasoning_trace,
        dht_anchor_hash: new.dht_anchor_hash,
    };

    diesel::insert_into(token_mint_events::table)
        .values(&record)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to insert token mint event: {}", e)))?;

    token_mint_events::table
        .filter(token_mint_events::id.eq(id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to fetch inserted mint event: {}", e)))
}

// ============================================================================
// Read Operations
// ============================================================================

/// List all mint events for an agent — scoped by app.
pub fn get_mints_for_agent(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
) -> Result<Vec<TokenMintEvent>, StorageError> {
    token_mint_events::table
        .filter(token_mint_events::h_app_id.eq(&ctx.h_app_id))
        .filter(token_mint_events::agent_id.eq(agent_id))
        .order(token_mint_events::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// List all mint events linked to a specific provenance REA event — scoped by app.
pub fn get_mints_for_event(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    provenance_event_id: &str,
) -> Result<Vec<TokenMintEvent>, StorageError> {
    token_mint_events::table
        .filter(token_mint_events::h_app_id.eq(&ctx.h_app_id))
        .filter(token_mint_events::provenance_event_id.eq(provenance_event_id))
        .order(token_mint_events::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}
