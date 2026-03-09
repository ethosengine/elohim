//! Presence service - business logic for contributor presence lifecycle
//!
//! Encapsulates validation, default population, and state transitions
//! for the contributor presence lifecycle (unclaimed → stewarded → claiming → claimed).
//!
//! ## Architecture
//!
//! Controller (api/presence.rs) → **Service (this file)** → Model (db/contributor_presences.rs)

use crate::db::contributor_presences::{
    self, ContributorPresenceQuery, CreateContributorPresenceInput, InitiateClaimInput,
    InitiateStewardshipInput,
};
use crate::db::models::ContributorPresence;
use crate::db::{AppContext, PooledConn};
use crate::error::StorageError;

/// Presence service for contributor lifecycle management
pub struct PresenceService;

impl PresenceService {
    /// List presences with optional filters
    pub fn list_presences(
        conn: &mut PooledConn,
        ctx: &AppContext,
        query: &ContributorPresenceQuery,
    ) -> Result<Vec<ContributorPresence>, StorageError> {
        contributor_presences::list_contributor_presences(conn, ctx, query)
    }

    /// Get a single presence by ID
    pub fn get_presence(
        conn: &mut PooledConn,
        ctx: &AppContext,
        id: &str,
    ) -> Result<Option<ContributorPresence>, StorageError> {
        contributor_presences::get_contributor_presence(conn, ctx, id)
    }

    /// Create a new presence with validation
    pub fn create_presence(
        conn: &mut PooledConn,
        ctx: &AppContext,
        input: CreateContributorPresenceInput,
    ) -> Result<ContributorPresence, StorageError> {
        if input.display_name.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "displayName must not be empty".into(),
            ));
        }
        contributor_presences::create_contributor_presence(conn, ctx, input)
    }

    /// Begin stewardship of an unclaimed presence
    pub fn begin_stewardship(
        conn: &mut PooledConn,
        ctx: &AppContext,
        id: &str,
        input: InitiateStewardshipInput,
    ) -> Result<ContributorPresence, StorageError> {
        if input.steward_id.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "stewardAgentId must not be empty".into(),
            ));
        }
        contributor_presences::initiate_stewardship(conn, ctx, id, &input)
    }

    /// Initiate a claim on a presence
    pub fn initiate_claim(
        conn: &mut PooledConn,
        ctx: &AppContext,
        id: &str,
        input: InitiateClaimInput,
    ) -> Result<ContributorPresence, StorageError> {
        if input.claiming_agent_id.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "claimingAgentId must not be empty".into(),
            ));
        }
        contributor_presences::initiate_claim(conn, ctx, id, &input)
    }

    /// Verify and complete a pending claim
    pub fn verify_claim(
        conn: &mut PooledConn,
        ctx: &AppContext,
        id: &str,
    ) -> Result<ContributorPresence, StorageError> {
        contributor_presences::verify_claim(conn, ctx, id)
    }

    /// Delete a presence by ID; returns true if deleted, false if not found
    pub fn delete_presence(
        conn: &mut PooledConn,
        ctx: &AppContext,
        id: &str,
    ) -> Result<bool, StorageError> {
        contributor_presences::delete_contributor_presence(conn, ctx, id)
    }
}
