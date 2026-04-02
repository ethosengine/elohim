//! Token decay event CRUD operations using Diesel with app scoping.
//!
//! Decay events are immutable audit records — one row per decay application
//! against an agent's balance. They record the before/after balance, the decay
//! amount, the obligation level that triggered the decay, and the dignity floor
//! that was enforced.
//!
//! Category C (operational): local-only audit log; not notarized on DHT.

use diesel::prelude::*;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::token_decay_events;
use super::models::{NewTokenDecayEvent, TokenDecayEvent};
use crate::error::StorageError;

// ============================================================================
// Write Operations
// ============================================================================

/// Record a new token decay event — scoped by app.
///
/// The `id` field on `new` is ignored if empty; a UUID is generated instead.
pub fn create_decay_event(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    new: NewTokenDecayEvent<'_>,
) -> Result<TokenDecayEvent, StorageError> {
    let generated_id;
    let id = if new.id.is_empty() {
        generated_id = Uuid::new_v4().to_string();
        generated_id.as_str()
    } else {
        new.id
    };

    let record = NewTokenDecayEvent {
        id,
        h_app_id: &ctx.h_app_id,
        agent_id: new.agent_id,
        governance_layer: new.governance_layer,
        balance_before: new.balance_before,
        balance_after: new.balance_after,
        decay_amount: new.decay_amount,
        obligation_level: new.obligation_level,
        dignity_floor: new.dignity_floor,
    };

    diesel::insert_into(token_decay_events::table)
        .values(&record)
        .execute(conn)
        .map_err(|e| {
            StorageError::Internal(format!("Failed to insert token decay event: {}", e))
        })?;

    token_decay_events::table
        .filter(token_decay_events::id.eq(id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to fetch inserted decay event: {}", e)))
}

// ============================================================================
// Read Operations
// ============================================================================

/// List all decay events for an agent — scoped by app, ordered newest first.
pub fn get_decay_events_for_agent(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
) -> Result<Vec<TokenDecayEvent>, StorageError> {
    token_decay_events::table
        .filter(token_decay_events::h_app_id.eq(&ctx.h_app_id))
        .filter(token_decay_events::agent_id.eq(agent_id))
        .order(token_decay_events::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}
