//! Governance CRUD operations using Diesel
//!
//! Manages governance states, challenges, proposals, precedents, and discussions
//! for content entities. These tables are NOT app-scoped — governance is per-entity.

use diesel::prelude::*;

use super::diesel_schema::{
    appeals, challenges, discussions, governance_signals, governance_states, precedents,
    proposal_options, proposals, ranked_votes, statement_votes, statements, votes,
};
use super::models::{
    Appeal, Challenge, Discussion, GovernanceSignal, GovernanceState, NewAppeal, NewChallenge,
    NewDiscussion, NewGovernanceSignal, NewGovernanceState, NewPrecedent, NewProposal,
    NewProposalOption, NewRankedVote, NewStatementVote, NewVote, Precedent, Proposal,
    ProposalOption, RankedVote, Statement, StatementVote, Vote,
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

/// Query challenges by entity type and entity ID
pub fn query_challenges(
    conn: &mut SqliteConnection,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<Challenge>, StorageError> {
    challenges::table
        .filter(challenges::entity_type.eq(entity_type))
        .filter(challenges::entity_id.eq(entity_id))
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

/// Respond to a challenge — update state, outcome, reasoning, actions, responded_at, sets_precedent
pub fn respond_to_challenge(
    conn: &mut SqliteConnection,
    id: &str,
    outcome: &str,
    reasoning: &str,
    actions: Option<&str>,
    response_by: &str,
    sets_precedent: bool,
) -> Result<Challenge, StorageError> {
    let now = crate::db::models::current_timestamp();
    diesel::update(challenges::table.filter(challenges::id.eq(id)))
        .set((
            challenges::state.eq("responded"),
            challenges::response_outcome.eq(Some(outcome)),
            challenges::response_reasoning.eq(Some(reasoning)),
            challenges::response_actions.eq(actions),
            challenges::response_by.eq(Some(response_by)),
            challenges::sets_precedent.eq(if sets_precedent { 1 } else { 0 }),
            challenges::responded_at.eq(Some(&now)),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    challenges::table
        .filter(challenges::id.eq(id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Appeals
// ============================================================================

/// Get appeal by ID
pub fn get_appeal(conn: &mut SqliteConnection, id: &str) -> Result<Option<Appeal>, StorageError> {
    appeals::table
        .filter(appeals::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Query appeals for a challenge
pub fn query_appeals_for_challenge(
    conn: &mut SqliteConnection,
    challenge_id: &str,
) -> Result<Vec<Appeal>, StorageError> {
    appeals::table
        .filter(appeals::challenge_id.eq(challenge_id))
        .order(appeals::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Insert a new appeal
pub fn create_appeal(conn: &mut SqliteConnection, new: &NewAppeal) -> Result<Appeal, StorageError> {
    diesel::insert_into(appeals::table)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    appeals::table
        .filter(appeals::id.eq(new.id))
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
pub fn cast_vote(conn: &mut SqliteConnection, new: &NewVote) -> Result<Vote, StorageError> {
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

/// Aggregate signals for an entity: totals, by-type, by-value, unique participants, consensus strength
pub fn aggregate_signals(
    conn: &mut SqliteConnection,
    entity_type: &str,
    entity_id: &str,
) -> Result<crate::views::SignalAggregateView, StorageError> {
    let signals = query_signals(conn, entity_type, entity_id)?;
    let total_signals = signals.len() as i64;

    if total_signals == 0 {
        return Ok(crate::views::SignalAggregateView {
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            total_signals: 0,
            by_type: std::collections::HashMap::new(),
            by_value: std::collections::HashMap::new(),
            unique_participants: 0,
            consensus_strength: 0.0,
        });
    }

    let mut by_type: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut by_value: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut participants: std::collections::HashSet<String> = std::collections::HashSet::new();

    for signal in &signals {
        *by_type.entry(signal.signal_type.clone()).or_insert(0) += 1;
        *by_value.entry(signal.signal_value.clone()).or_insert(0) += 1;
        participants.insert(signal.human_id.clone());
    }

    let unique_participants = participants.len() as i64;

    // Compute consensus_strength using normalized Shannon entropy
    let consensus_strength = {
        let num_distinct = by_value.len();
        if num_distinct <= 1 {
            1.0
        } else {
            let total = total_signals as f64;
            let entropy: f64 = by_value
                .values()
                .map(|&count| {
                    let p = count as f64 / total;
                    -p * p.ln()
                })
                .sum();
            let max_entropy = (num_distinct as f64).ln();
            1.0 - (entropy / max_entropy)
        }
    };

    Ok(crate::views::SignalAggregateView {
        entity_type: entity_type.to_string(),
        entity_id: entity_id.to_string(),
        total_signals,
        by_type,
        by_value,
        unique_participants,
        consensus_strength,
    })
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

// ============================================================================
// Sensemaking Statements
// ============================================================================

/// Create a new statement
pub fn create_statement(
    conn: &mut SqliteConnection,
    new: &super::models::NewStatement,
) -> Result<Statement, StorageError> {
    diesel::insert_into(statements::table)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    statements::table
        .filter(statements::id.eq(new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Query statements by entity type and entity ID
pub fn query_statements(
    conn: &mut SqliteConnection,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<Statement>, StorageError> {
    statements::table
        .filter(statements::entity_type.eq(entity_type))
        .filter(statements::entity_id.eq(entity_id))
        .order(statements::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Vote on a statement (upsert): delete existing vote for this human+statement,
/// insert new vote, then recount agree/disagree/pass from statement_votes.
pub fn vote_on_statement(
    conn: &mut SqliteConnection,
    new_vote: &NewStatementVote,
) -> Result<StatementVote, StorageError> {
    // Delete existing vote for this human+statement
    diesel::delete(
        statement_votes::table
            .filter(statement_votes::statement_id.eq(new_vote.statement_id))
            .filter(statement_votes::human_id.eq(new_vote.human_id)),
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    // Insert new vote
    diesel::insert_into(statement_votes::table)
        .values(new_vote)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    // Recount all votes for this statement
    let all_votes: Vec<StatementVote> = statement_votes::table
        .filter(statement_votes::statement_id.eq(new_vote.statement_id))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    let agree_count = all_votes.iter().filter(|v| v.vote == "agree").count() as i32;
    let disagree_count = all_votes.iter().filter(|v| v.vote == "disagree").count() as i32;
    let pass_count = all_votes.iter().filter(|v| v.vote == "pass").count() as i32;

    diesel::update(statements::table.filter(statements::id.eq(new_vote.statement_id)))
        .set((
            statements::agree_count.eq(agree_count),
            statements::disagree_count.eq(disagree_count),
            statements::pass_count.eq(pass_count),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    statement_votes::table
        .filter(statement_votes::id.eq(new_vote.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get all votes for a specific statement
pub fn get_statement_votes(
    conn: &mut SqliteConnection,
    statement_id: &str,
) -> Result<Vec<StatementVote>, StorageError> {
    statement_votes::table
        .filter(statement_votes::statement_id.eq(statement_id))
        .order(statement_votes::created_at.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get all votes for all statements of a given entity (for clustering)
pub fn get_all_votes_for_entity(
    conn: &mut SqliteConnection,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<StatementVote>, StorageError> {
    let statement_ids: Vec<String> = statements::table
        .filter(statements::entity_type.eq(entity_type))
        .filter(statements::entity_id.eq(entity_id))
        .select(statements::id)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    statement_votes::table
        .filter(statement_votes::statement_id.eq_any(&statement_ids))
        .order(statement_votes::created_at.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}
