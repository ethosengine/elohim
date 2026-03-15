//! Governance CRUD operations using Diesel
//!
//! Manages governance states, challenges, proposals, precedents, and discussions
//! for content entities. These tables are NOT app-scoped — governance is per-entity.

use diesel::prelude::*;

use super::diesel_schema::{
    challenges, discussions, governance_signals, governance_states, precedents, proposal_options,
    proposals, ranked_votes, votes,
};
use super::models::{
    Challenge, Discussion, GovernanceSignal, GovernanceState, NewChallenge, NewDiscussion,
    NewGovernanceSignal, NewGovernanceState, NewPrecedent, NewProposal, NewProposalOption, NewRankedVote,
    NewVote, Precedent, Proposal, ProposalOption, RankedVote, Vote,
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

// ============================================================================
// Votes
// ============================================================================

/// Get all votes for a proposal
pub fn query_votes(
    conn: &mut SqliteConnection,
    proposal_id: &str,
) -> Result<Vec<Vote>, StorageError> {
    votes::table
        .filter(votes::proposal_id.eq(proposal_id))
        .order(votes::created_at.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get a specific human's vote on a proposal
pub fn get_vote(
    conn: &mut SqliteConnection,
    proposal_id: &str,
    human_id: &str,
) -> Result<Option<Vote>, StorageError> {
    votes::table
        .filter(votes::proposal_id.eq(proposal_id))
        .filter(votes::human_id.eq(human_id))
        .first::<Vote>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Cast or update a vote (upsert via delete+insert for SQLite)
pub fn cast_vote(
    conn: &mut SqliteConnection,
    new: &NewVote,
) -> Result<Vote, StorageError> {
    // Delete existing vote if any (UNIQUE constraint enforcement)
    diesel::delete(
        votes::table
            .filter(votes::proposal_id.eq(new.proposal_id))
            .filter(votes::human_id.eq(new.human_id)),
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    diesel::insert_into(votes::table)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    votes::table
        .filter(votes::id.eq(new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// =========================================================================
// Proposal Options
// =========================================================================

pub fn query_proposal_options(
    conn: &mut SqliteConnection,
    proposal_id: &str,
) -> Result<Vec<ProposalOption>, StorageError> {
    proposal_options::table
        .filter(proposal_options::proposal_id.eq(proposal_id))
        .order(proposal_options::position.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

pub fn create_proposal_option(
    conn: &mut SqliteConnection,
    new: &NewProposalOption,
) -> Result<ProposalOption, StorageError> {
    diesel::insert_into(proposal_options::table)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;
    proposal_options::table
        .filter(proposal_options::id.eq(new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

pub fn create_proposal_options(
    conn: &mut SqliteConnection,
    options: &[NewProposalOption],
) -> Result<Vec<ProposalOption>, StorageError> {
    for opt in options {
        diesel::insert_into(proposal_options::table)
            .values(opt)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;
    }
    let pid = options.first().map(|o| o.proposal_id).unwrap_or("");
    proposal_options::table
        .filter(proposal_options::proposal_id.eq(pid))
        .order(proposal_options::position.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// =========================================================================
// Ranked Votes (multi-mechanism)
// =========================================================================

pub fn query_ranked_votes(
    conn: &mut SqliteConnection,
    proposal_id: &str,
) -> Result<Vec<RankedVote>, StorageError> {
    ranked_votes::table
        .filter(ranked_votes::proposal_id.eq(proposal_id))
        .order(ranked_votes::created_at.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

pub fn get_ranked_votes_for_human(
    conn: &mut SqliteConnection,
    proposal_id: &str,
    human_id: &str,
) -> Result<Vec<RankedVote>, StorageError> {
    ranked_votes::table
        .filter(ranked_votes::proposal_id.eq(proposal_id))
        .filter(ranked_votes::human_id.eq(human_id))
        .order(ranked_votes::rank.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

pub fn cast_ranked_votes(
    conn: &mut SqliteConnection,
    proposal_id: &str,
    human_id: &str,
    votes: &[NewRankedVote],
) -> Result<Vec<RankedVote>, StorageError> {
    diesel::delete(
        ranked_votes::table
            .filter(ranked_votes::proposal_id.eq(proposal_id))
            .filter(ranked_votes::human_id.eq(human_id)),
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;
    for vote in votes {
        diesel::insert_into(ranked_votes::table)
            .values(vote)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;
    }
    get_ranked_votes_for_human(conn, proposal_id, human_id)
}

// =========================================================================
// Governance Signals
// =========================================================================

pub fn record_signal(
    conn: &mut SqliteConnection,
    new: &NewGovernanceSignal,
) -> Result<GovernanceSignal, StorageError> {
    diesel::insert_into(governance_signals::table)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;
    governance_signals::table
        .filter(governance_signals::id.eq(new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

pub fn query_signals(
    conn: &mut SqliteConnection,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<GovernanceSignal>, StorageError> {
    governance_signals::table
        .filter(governance_signals::entity_type.eq(entity_type))
        .filter(governance_signals::entity_id.eq(entity_id))
        .order(governance_signals::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

pub fn count_signals(
    conn: &mut SqliteConnection,
    entity_type: &str,
    entity_id: &str,
) -> Result<i64, StorageError> {
    governance_signals::table
        .filter(governance_signals::entity_type.eq(entity_type))
        .filter(governance_signals::entity_id.eq(entity_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count failed: {}", e)))
}
