//! Governance CRUD operations using Diesel
//!
//! Manages governance states, challenges, proposals, precedents, and discussions
//! for content entities. These tables are NOT app-scoped — governance is per-entity.

use diesel::prelude::*;

use super::diesel_schema::{challenges, discussions, governance_states, precedents, proposals};
use super::models::{
    Challenge, Discussion, GovernanceState, NewChallenge, NewDiscussion, NewGovernanceState,
    NewPrecedent, NewProposal, Precedent, Proposal,
};
use crate::error::StorageError;

// ============================================================================
// Governance States
// ============================================================================

/// Get governance state by entity type and entity ID
pub fn get_governance_state(
    conn: &mut SqliteConnection,
    entity_type: &str,
    entity_id: &str,
) -> Result<Option<GovernanceState>, StorageError> {
    governance_states::table
        .filter(governance_states::entity_type.eq(entity_type))
        .filter(governance_states::entity_id.eq(entity_id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Query governance states by entity type
pub fn query_governance_states(
    conn: &mut SqliteConnection,
    entity_type: &str,
) -> Result<Vec<GovernanceState>, StorageError> {
    governance_states::table
        .filter(governance_states::entity_type.eq(entity_type))
        .order(governance_states::updated_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Insert a new governance state
pub fn create_governance_state(
    conn: &mut SqliteConnection,
    new: &NewGovernanceState,
) -> Result<GovernanceState, StorageError> {
    diesel::insert_into(governance_states::table)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    governance_states::table
        .filter(governance_states::id.eq(new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Challenges
// ============================================================================

/// Get challenge by ID
pub fn get_challenge(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<Challenge>, StorageError> {
    challenges::table
        .filter(challenges::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Query challenges by content ID
pub fn query_challenges(
    conn: &mut SqliteConnection,
    content_id: &str,
) -> Result<Vec<Challenge>, StorageError> {
    challenges::table
        .filter(challenges::content_id.eq(content_id))
        .order(challenges::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Insert a new challenge
pub fn create_challenge(
    conn: &mut SqliteConnection,
    new: &NewChallenge,
) -> Result<Challenge, StorageError> {
    diesel::insert_into(challenges::table)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    challenges::table
        .filter(challenges::id.eq(new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Proposals
// ============================================================================

/// Get proposal by ID
pub fn get_proposal(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<Proposal>, StorageError> {
    proposals::table
        .filter(proposals::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Query proposals by content ID and optional status filter
pub fn query_proposals(
    conn: &mut SqliteConnection,
    content_id: &str,
    status: Option<&str>,
) -> Result<Vec<Proposal>, StorageError> {
    let mut query = proposals::table
        .filter(proposals::content_id.eq(content_id))
        .into_boxed();

    if let Some(status) = status {
        query = query.filter(proposals::status.eq(status));
    }

    query
        .order(proposals::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Insert a new proposal
pub fn create_proposal(
    conn: &mut SqliteConnection,
    new: &NewProposal,
) -> Result<Proposal, StorageError> {
    diesel::insert_into(proposals::table)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    proposals::table
        .filter(proposals::id.eq(new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Precedents
// ============================================================================

/// Get precedent by ID
pub fn get_precedent(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<Precedent>, StorageError> {
    precedents::table
        .filter(precedents::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Query precedents by content ID
pub fn query_precedents(
    conn: &mut SqliteConnection,
    content_id: &str,
) -> Result<Vec<Precedent>, StorageError> {
    precedents::table
        .filter(precedents::content_id.eq(content_id))
        .order(precedents::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Insert a new precedent
pub fn create_precedent(
    conn: &mut SqliteConnection,
    new: &NewPrecedent,
) -> Result<Precedent, StorageError> {
    diesel::insert_into(precedents::table)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    precedents::table
        .filter(precedents::id.eq(new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Discussions
// ============================================================================

/// Get discussion by ID
pub fn get_discussion(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<Discussion>, StorageError> {
    discussions::table
        .filter(discussions::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Query discussions by content ID
pub fn query_discussions(
    conn: &mut SqliteConnection,
    content_id: &str,
) -> Result<Vec<Discussion>, StorageError> {
    discussions::table
        .filter(discussions::content_id.eq(content_id))
        .order(discussions::created_at.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Insert a new discussion
pub fn create_discussion(
    conn: &mut SqliteConnection,
    new: &NewDiscussion,
) -> Result<Discussion, StorageError> {
    diesel::insert_into(discussions::table)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    discussions::table
        .filter(discussions::id.eq(new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}
