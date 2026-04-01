//! Token balance CRUD operations using Diesel with app scoping.
//!
//! Tracks current token holdings per agent per governance layer.
//! Category B (agent-scoped): source of truth is the agent's local chain;
//! storage is a materialized projection used for fast reads.

use diesel::prelude::*;

use super::context::AppContext;
use super::diesel_schema::token_balances;
use super::models::TokenBalance;
use crate::error::StorageError;

// ============================================================================
// Credit Source Enum
// ============================================================================

/// Origin of an incoming credit — used to update the appropriate accumulator.
pub enum CreditSource {
    /// Tokens created by the minting pipeline (increases `total_minted`)
    Mint,
    /// Tokens received via a peer-to-peer transfer (increases `total_transferred_in`)
    TransferIn,
}

// ============================================================================
// Read Operations
// ============================================================================

/// Fetch the balance row for a single agent + governance layer — scoped by app.
/// Returns `None` if the agent has never held tokens in that layer.
pub fn get_balance(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
    governance_layer: &str,
) -> Result<Option<TokenBalance>, StorageError> {
    token_balances::table
        .filter(token_balances::h_app_id.eq(&ctx.h_app_id))
        .filter(token_balances::agent_id.eq(agent_id))
        .filter(token_balances::governance_layer.eq(governance_layer))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Fetch all balance rows for an agent across all governance layers — scoped by app.
pub fn get_all_balances_for_agent(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
) -> Result<Vec<TokenBalance>, StorageError> {
    token_balances::table
        .filter(token_balances::h_app_id.eq(&ctx.h_app_id))
        .filter(token_balances::agent_id.eq(agent_id))
        .order(token_balances::governance_layer.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Write Operations
// ============================================================================

/// Upsert pattern: credit an agent's balance in a governance layer.
///
/// - If no row exists, inserts one with `balance = amount`.
/// - If a row exists, increments `balance` and the appropriate accumulator
///   (`total_minted` for `CreditSource::Mint`, `total_transferred_in` for
///   `CreditSource::TransferIn`).
///
/// Returns the updated balance row.
pub fn credit_balance(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
    governance_layer: &str,
    amount: f32,
    source: CreditSource,
) -> Result<TokenBalance, StorageError> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let existing = get_balance(conn, ctx, agent_id, governance_layer)?;

    match existing {
        None => {
            // INSERT — first time this agent holds tokens in this layer
            let (total_minted, total_transferred_in) = match source {
                CreditSource::Mint => (amount, 0.0_f32),
                CreditSource::TransferIn => (0.0_f32, amount),
            };

            diesel::insert_into(token_balances::table)
                .values((
                    token_balances::agent_id.eq(agent_id),
                    token_balances::h_app_id.eq(&ctx.h_app_id),
                    token_balances::governance_layer.eq(governance_layer),
                    token_balances::balance.eq(amount),
                    token_balances::total_minted.eq(total_minted),
                    token_balances::total_transferred_in.eq(total_transferred_in),
                    token_balances::total_transferred_out.eq(0.0_f32),
                    token_balances::last_activity_at.eq(&now),
                    token_balances::created_at.eq(&now),
                ))
                .execute(conn)
                .map_err(|e| {
                    StorageError::Internal(format!("Failed to insert token balance: {}", e))
                })?;
        }
        Some(ref row) => {
            // UPDATE — add to existing balance
            let new_balance = row.balance + amount;
            let (new_minted, new_transferred_in) = match source {
                CreditSource::Mint => (row.total_minted + amount, row.total_transferred_in),
                CreditSource::TransferIn => {
                    (row.total_minted, row.total_transferred_in + amount)
                }
            };

            diesel::update(
                token_balances::table
                    .filter(token_balances::h_app_id.eq(&ctx.h_app_id))
                    .filter(token_balances::agent_id.eq(agent_id))
                    .filter(token_balances::governance_layer.eq(governance_layer)),
            )
            .set((
                token_balances::balance.eq(new_balance),
                token_balances::total_minted.eq(new_minted),
                token_balances::total_transferred_in.eq(new_transferred_in),
                token_balances::last_activity_at.eq(&now),
            ))
            .execute(conn)
            .map_err(|e| {
                StorageError::Internal(format!("Failed to update token balance: {}", e))
            })?;
        }
    }

    // Return the fresh row
    get_balance(conn, ctx, agent_id, governance_layer)?
        .ok_or_else(|| StorageError::Internal("Balance row missing after upsert".to_string()))
}

/// Debit an agent's balance in a governance layer.
///
/// Returns `StorageError::InvalidInput` if the agent has insufficient balance.
/// Returns the updated balance row on success.
pub fn debit_balance(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
    governance_layer: &str,
    amount: f32,
) -> Result<TokenBalance, StorageError> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let row = get_balance(conn, ctx, agent_id, governance_layer)?.ok_or_else(|| {
        StorageError::InvalidInput(format!(
            "Agent {} has no balance in governance layer {}",
            agent_id, governance_layer
        ))
    })?;

    if row.balance < amount {
        return Err(StorageError::InvalidInput(format!(
            "Insufficient balance for agent {} in layer {}: has {:.6}, needs {:.6}",
            agent_id, governance_layer, row.balance, amount
        )));
    }

    let new_balance = row.balance - amount;
    let new_transferred_out = row.total_transferred_out + amount;

    diesel::update(
        token_balances::table
            .filter(token_balances::h_app_id.eq(&ctx.h_app_id))
            .filter(token_balances::agent_id.eq(agent_id))
            .filter(token_balances::governance_layer.eq(governance_layer)),
    )
    .set((
        token_balances::balance.eq(new_balance),
        token_balances::total_transferred_out.eq(new_transferred_out),
        token_balances::last_activity_at.eq(&now),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Failed to debit token balance: {}", e)))?;

    get_balance(conn, ctx, agent_id, governance_layer)?
        .ok_or_else(|| StorageError::Internal("Balance row missing after debit".to_string()))
}
