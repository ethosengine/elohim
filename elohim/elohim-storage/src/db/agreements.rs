//! Agreement CRUD operations
//!
//! REA/ValueFlows Agreement — a mutually understood arrangement between agents.
//! Commitments reference agreements via their `clause_of` field.

use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::agreements;
use super::models::{AgreementRow, NewAgreement};
use crate::error::StorageError;

// ============================================================================
// Query Types
// ============================================================================

/// Input for creating an agreement
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAgreementInput {
    #[serde(default)]
    pub id: Option<String>,
    pub name: Option<String>,
    pub note: Option<String>,
    pub metadata_json: Option<String>,
}

/// Query parameters for listing agreements
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgreementQuery {
    /// Filter by name (exact match)
    pub name: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

// ============================================================================
// Read Operations
// ============================================================================

/// Get agreement by ID — scoped by app
pub fn get_agreement(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<AgreementRow>, StorageError> {
    agreements::table
        .filter(agreements::h_app_id.eq(&ctx.h_app_id))
        .filter(agreements::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// List agreements with filtering — scoped by app
pub fn list_agreements(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &AgreementQuery,
) -> Result<Vec<AgreementRow>, StorageError> {
    let mut base_query = agreements::table
        .filter(agreements::h_app_id.eq(&ctx.h_app_id))
        .into_boxed();

    if let Some(ref name) = query.name {
        base_query = base_query.filter(agreements::name.eq(name));
    }

    base_query
        .order(agreements::created_at.desc())
        .limit(query.limit)
        .offset(query.offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Write Operations
// ============================================================================

/// Create an agreement — scoped by app
pub fn create_agreement(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateAgreementInput,
) -> Result<AgreementRow, StorageError> {
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let new = NewAgreement {
        id: &id,
        h_app_id: &ctx.h_app_id,
        name: input.name.as_deref(),
        note: input.note.as_deref(),
        dht_anchor_hash: None,
        metadata_json: input.metadata_json.as_deref(),
    };

    diesel::insert_into(agreements::table)
        .values(&new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    get_agreement(conn, ctx, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created agreement".into()))
}

/// Upsert an agreement — insert or update dht_anchor_hash if it already exists.
/// Used by notary projection to anchor agreements onto the DHT.
pub fn upsert_agreement(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateAgreementInput,
    dht_anchor_hash: Option<&str>,
) -> Result<AgreementRow, StorageError> {
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let existing = get_agreement(conn, ctx, &id)?;

    if existing.is_some() {
        // Update dht_anchor_hash (and optionally name/note/metadata)
        diesel::update(
            agreements::table
                .filter(agreements::h_app_id.eq(&ctx.h_app_id))
                .filter(agreements::id.eq(&id)),
        )
        .set((
            agreements::dht_anchor_hash.eq(dht_anchor_hash),
            agreements::name.eq(input.name.as_deref()),
            agreements::note.eq(input.note.as_deref()),
            agreements::metadata_json.eq(input.metadata_json.as_deref()),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;
    } else {
        let new = NewAgreement {
            id: &id,
            h_app_id: &ctx.h_app_id,
            name: input.name.as_deref(),
            note: input.note.as_deref(),
            dht_anchor_hash,
            metadata_json: input.metadata_json.as_deref(),
        };

        diesel::insert_into(agreements::table)
            .values(&new)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;
    }

    get_agreement(conn, ctx, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve upserted agreement".into()))
}
