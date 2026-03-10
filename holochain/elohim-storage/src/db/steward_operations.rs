//! Steward operations CRUD — credentials, gates, and grants

use diesel::prelude::*;
use serde::Serialize;

use super::diesel_schema::{access_grants, premium_gates, steward_credentials};
use super::models::{
    AccessGrant, NewAccessGrant, NewPremiumGate, NewStewardCredential, PremiumGate,
    StewardCredential,
};
use crate::error::StorageError;

// ============================================================================
// Steward Credentials
// ============================================================================

/// Insert a new steward credential
pub fn create_credential(
    conn: &mut SqliteConnection,
    new: &NewStewardCredential,
) -> Result<StewardCredential, StorageError> {
    diesel::insert_into(steward_credentials::table)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    steward_credentials::table
        .filter(steward_credentials::id.eq(new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get credential by ID
pub fn get_credential(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<StewardCredential>, StorageError> {
    steward_credentials::table
        .filter(steward_credentials::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Query credentials for a presence
pub fn query_credentials_for_presence(
    conn: &mut SqliteConnection,
    presence_id: &str,
) -> Result<Vec<StewardCredential>, StorageError> {
    steward_credentials::table
        .filter(steward_credentials::presence_id.eq(presence_id))
        .order(steward_credentials::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Query credentials for a content item
pub fn query_credentials_for_content(
    conn: &mut SqliteConnection,
    content_id: &str,
) -> Result<Vec<StewardCredential>, StorageError> {
    steward_credentials::table
        .filter(steward_credentials::content_id.eq(content_id))
        .order(steward_credentials::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Premium Gates
// ============================================================================

/// Insert a new premium gate
pub fn create_gate(
    conn: &mut SqliteConnection,
    new: &NewPremiumGate,
) -> Result<PremiumGate, StorageError> {
    diesel::insert_into(premium_gates::table)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    premium_gates::table
        .filter(premium_gates::id.eq(new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get gate by ID
pub fn get_gate(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<PremiumGate>, StorageError> {
    premium_gates::table
        .filter(premium_gates::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Query gates by resource type
pub fn query_gates_for_resource(
    conn: &mut SqliteConnection,
    resource_type: &str,
) -> Result<Vec<PremiumGate>, StorageError> {
    premium_gates::table
        .filter(premium_gates::gated_resource_type.eq(resource_type))
        .order(premium_gates::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Access Grants
// ============================================================================

/// Insert a new access grant
pub fn create_grant(
    conn: &mut SqliteConnection,
    new: &NewAccessGrant,
) -> Result<AccessGrant, StorageError> {
    diesel::insert_into(access_grants::table)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    access_grants::table
        .filter(access_grants::id.eq(new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get grant by ID
pub fn get_grant(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<AccessGrant>, StorageError> {
    access_grants::table
        .filter(access_grants::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Query grants for a gate
pub fn query_grants_for_gate(
    conn: &mut SqliteConnection,
    gate_id: &str,
) -> Result<Vec<AccessGrant>, StorageError> {
    access_grants::table
        .filter(access_grants::gate_id.eq(gate_id))
        .order(access_grants::granted_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Check if a grantee has active access to a gate
pub fn check_access(
    conn: &mut SqliteConnection,
    gate_id: &str,
    grantee_id: &str,
) -> Result<bool, StorageError> {
    let count: i64 = access_grants::table
        .filter(access_grants::gate_id.eq(gate_id))
        .filter(access_grants::grantee_presence_id.eq(grantee_id))
        .filter(access_grants::status.eq("active"))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    Ok(count > 0)
}

// ============================================================================
// Revenue Summary
// ============================================================================

/// Summary of steward revenue activity
#[derive(Debug, Clone, Serialize)]
pub struct RevenueSummary {
    pub total_gates: i64,
    pub total_grants: i64,
    pub total_credentials: i64,
}

/// Get revenue summary for a presence
pub fn get_revenue_summary(
    conn: &mut SqliteConnection,
    presence_id: &str,
) -> Result<RevenueSummary, StorageError> {
    let total_credentials: i64 = steward_credentials::table
        .filter(steward_credentials::presence_id.eq(presence_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    let total_gates: i64 = premium_gates::table
        .filter(premium_gates::steward_presence_id.eq(presence_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    // Count grants across all gates owned by this presence
    let gate_ids: Vec<String> = premium_gates::table
        .filter(premium_gates::steward_presence_id.eq(presence_id))
        .select(premium_gates::id)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    let total_grants: i64 = if gate_ids.is_empty() {
        0
    } else {
        access_grants::table
            .filter(access_grants::gate_id.eq_any(&gate_ids))
            .count()
            .get_result(conn)
            .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?
    };

    Ok(RevenueSummary {
        total_gates,
        total_grants,
        total_credentials,
    })
}
