use hdi::prelude::*;

// ============================================================
// ENTRY TYPES — Formal Governance (Mishpat DNA)
// Separated from Lamad DNA to give governance its own
// validation rules and free Lamad entry type capacity.
// ============================================================

// =============================================================================
// Governance Entry Types
// =============================================================================

/// Challenge - A formal challenge to content or decisions.
///
/// Enables community members with standing to challenge content quality,
/// accuracy, safety, or constitutional alignment.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Challenge {
    pub id: String,
    pub entity_type: String,          // content, path, extension, attestation, decision
    pub entity_id: String,
    pub challenger_id: String,
    pub challenger_name: String,
    pub challenger_standing: String,  // Attestation level granting standing
    pub grounds: String,              // factual-error, safety, policy, constitutional
    pub description: String,
    pub evidence_json: String,        // Evidence[] as JSON
    pub status: String,               // filed, acknowledged, under-review, resolved, dismissed
    pub filed_at: String,
    pub acknowledged_at: Option<String>,
    pub sla_deadline: Option<String>,
    pub assigned_elohim: Option<String>,
    pub priority: String,             // normal, high, critical
    pub resolution_json: Option<String>, // ChallengeResolution as JSON
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Challenge grounds
pub const CHALLENGE_GROUNDS: [&str; 5] = [
    "factual-error",
    "new-evidence",
    "safety",
    "policy",
    "constitutional",
];

/// Challenge status states
pub const CHALLENGE_STATUS: [&str; 5] = [
    "filed",
    "acknowledged",
    "under-review",
    "resolved",
    "dismissed",
];

/// Proposal - A formal proposal for changes.
///
/// Supports various governance decision-making mechanisms.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub proposal_type: String,        // sense-check, consent, consensus, supermajority
    pub description: String,
    pub proposer_id: String,
    pub proposer_name: String,
    pub rationale: String,
    pub status: String,               // draft, discussion, voting, decided, dismissed
    pub phase: String,                // Current phase
    pub amendments_json: String,      // Amendment[] as JSON
    pub voting_config_json: String,   // VotingConfig as JSON
    pub current_votes_json: String,   // VoteCount as JSON
    pub outcome_json: Option<String>, // ProposalOutcome as JSON
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Proposal types
pub const PROPOSAL_TYPES: [&str; 4] = [
    "sense-check",
    "consent",
    "consensus",
    "supermajority",
];

/// Proposal status states
pub const PROPOSAL_STATUS: [&str; 5] = [
    "draft",
    "discussion",
    "voting",
    "decided",
    "dismissed",
];

/// Precedent - A binding decision that guides future decisions.
///
/// Precedents form the case law of the governance system.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Precedent {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub full_reasoning: String,
    pub binding: String,              // constitutional, binding-network, binding-local, persuasive
    pub scope_json: String,           // { entityTypes, categories, roles } as JSON
    pub citations: u32,               // How often this precedent is cited
    pub status: String,               // active, superseded, under-review
    pub established_by: String,       // Proposal ID or governance body
    pub established_at: String,
    pub superseded_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Precedent binding levels
pub const PRECEDENT_BINDING: [&str; 4] = [
    "constitutional",
    "binding-network",
    "binding-local",
    "persuasive",
];

/// Discussion - A threaded discussion on an entity.
///
/// Enables structured deliberation on content, proposals, or challenges.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Discussion {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub category: String,             // general, proposal, challenge, feedback
    pub title: String,
    pub messages_json: String,        // DiscussionMessage[] as JSON
    pub status: String,               // open, closed, archived
    pub message_count: u32,
    pub last_activity_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Discussion categories
pub const DISCUSSION_CATEGORIES: [&str; 4] = [
    "general",
    "proposal",
    "challenge",
    "feedback",
];

/// GovernanceState - Current governance status of an entity.
///
/// Tracks the governance posture of content, paths, etc.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GovernanceState {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub status: String,               // approved, pending, challenged, suspended
    pub status_basis_json: String,    // StatusBasis as JSON
    pub labels_json: String,          // Label[] as JSON
    pub active_challenges_json: String, // String[] as JSON
    pub active_proposals_json: String,  // String[] as JSON
    pub precedent_ids_json: String,     // String[] as JSON
    pub last_updated: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Governance status states
pub const GOVERNANCE_STATUS: [&str; 4] = [
    "approved",
    "pending",
    "challenged",
    "suspended",
];

// =============================================================================
// Qahal: Governance Signals - Contextual Feedback & Consensus
// =============================================================================
//
// Governance signals enable constitutional feedback mechanisms:
// - Low friction: Emotional reactions (moved, grateful, challenged, concerned)
// - Medium friction: Graduated feedback (Loomio-style scales)
// - High friction: Formal proposals (binding decisions)
//
// Inspired by:
// - Loomio: 4-position voting (Agree/Abstain/Disagree/Block)
// - Forby: ARCH intensity-based voting
// - Polis: 2D opinion clustering and consensus discovery

/// GovernanceReaction - Low friction emotional feedback on content.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GovernanceReaction {
    pub id: String,
    pub content_id: String,
    pub content_type: String,
    pub reactor_id: String,
    pub reaction: String,
    pub intensity: u8,
    pub mediated: bool,
    pub mediation_accepted: bool,
    pub context_json: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

pub const REACTION_TYPES: [&str; 6] = ["moved", "grateful", "challenged", "concerned", "surprised", "illuminated"];

/// GraduatedFeedback - Medium friction scaled feedback (Loomio/Forby style).
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GraduatedFeedback {
    pub id: String,
    pub content_id: String,
    pub content_type: String,
    pub responder_id: String,
    pub feedback_context: String,
    pub position: i8,
    pub intensity: u8,
    pub reasoning: Option<String>,
    pub updated_count: u32,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

pub const FEEDBACK_CONTEXTS: [&str; 5] = ["accuracy", "usefulness", "proposal", "clarity", "relevance"];

/// ProposalVote - Loomio-style 4-position voting on proposals.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ProposalVote {
    pub id: String,
    pub proposal_id: String,
    pub voter_id: String,
    pub voter_name: String,
    pub position: String,
    pub reasoning: Option<String>,
    pub version: u32,
    pub previous_position: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

pub const VOTE_POSITIONS: [&str; 4] = ["agree", "abstain", "disagree", "block"];

/// OpinionStatement - Polis-style statement for clustering.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct OpinionStatement {
    pub id: String,
    pub context_id: String,
    pub author_id: String,
    pub text: String,
    pub status: String,
    pub vote_count: u32,
    pub agree_count: u32,
    pub disagree_count: u32,
    pub pass_count: u32,
    pub consensus_score: i32,
    pub cluster_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// StatementVote - Individual vote on an OpinionStatement.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct StatementVote {
    pub id: String,
    pub statement_id: String,
    pub voter_id: String,
    pub vote: String,
    pub created_at: String,
    pub metadata_json: String,
}

pub const STATEMENT_VOTES: [&str; 3] = ["agree", "disagree", "pass"];

// =============================================================================
// Anchor Entry (for link indexing)
// =============================================================================

/// Generic string anchor for creating deterministic link bases.
///
/// Used by coordinator zome functions to index entries by ID, status, type,
/// entity, etc. without requiring a separate anchor DNA.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct StringAnchor {
    pub anchor_type: String,
    pub anchor_value: String,
}

impl StringAnchor {
    pub fn new(anchor_type: &str, anchor_value: &str) -> Self {
        Self {
            anchor_type: anchor_type.to_string(),
            anchor_value: anchor_value.to_string(),
        }
    }
}

// ============================================================
// ENTRY TYPES ENUM
// ============================================================

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Challenge(Challenge),
    Proposal(Proposal),
    Precedent(Precedent),
    Discussion(Discussion),
    GovernanceState(GovernanceState),
    GovernanceReaction(GovernanceReaction),
    GraduatedFeedback(GraduatedFeedback),
    ProposalVote(ProposalVote),
    OpinionStatement(OpinionStatement),
    StatementVote(StatementVote),
    StringAnchor(StringAnchor),
}

// ============================================================
// LINK TYPES
// ============================================================

#[hdk_link_types]
pub enum LinkTypes {
    // =========================================================================
    // Qahal: Governance Signal links (Loomio/Forby/Polis patterns)
    // =========================================================================
    ContentToReactions,         // Content -> GovernanceReaction
    AgentToReactions,           // Anchor(agent_id) -> GovernanceReaction
    ReactionByType,             // Anchor(reaction_type) -> GovernanceReaction
    ContentToFeedback,          // Content -> GraduatedFeedback
    AgentToFeedback,            // Anchor(agent_id) -> GraduatedFeedback
    FeedbackByContext,          // Anchor(feedback_context) -> GraduatedFeedback
    ProposalToVotes,            // Proposal -> ProposalVote
    AgentToVotes,               // Anchor(agent_id) -> ProposalVote
    VoteByPosition,             // Anchor(position) -> ProposalVote
    ContextToStatements,        // Anchor(context_id) -> OpinionStatement
    AgentToStatements,          // Anchor(agent_id) -> OpinionStatement
    StatementToVotes,           // OpinionStatement -> StatementVote
    AgentToStatementVotes,      // Anchor(agent_id) -> StatementVote

    // =========================================================================
    // Qahal: Formal Governance links
    // =========================================================================
    // Challenge
    IdToChallenge,              // Anchor(challenge_id) -> Challenge
    EntityToChallenge,          // Anchor(entity_type:entity_id) -> Challenge
    ChallengerToChallenge,      // Anchor(challenger_id) -> Challenge
    ChallengeByStatus,          // Anchor(status) -> Challenge

    // Proposal
    IdToProposal,               // Anchor(proposal_id) -> Proposal
    ProposalByType,             // Anchor(proposal_type) -> Proposal
    ProposerToProposal,         // Anchor(proposer_id) -> Proposal
    ProposalByStatus,           // Anchor(status) -> Proposal

    // Precedent
    IdToPrecedent,              // Anchor(precedent_id) -> Precedent
    PrecedentByScope,           // Anchor(scope) -> Precedent
    PrecedentByStatus,          // Anchor(status) -> Precedent

    // Discussion
    IdToDiscussion,             // Anchor(discussion_id) -> Discussion
    EntityToDiscussion,         // Anchor(entity_type:entity_id) -> Discussion
    DiscussionByCategory,       // Anchor(category) -> Discussion
    DiscussionByStatus,         // Anchor(status) -> Discussion

    // GovernanceState
    IdToGovernanceState,        // Anchor(entity_type:entity_id) -> GovernanceState
    GovernanceStateByStatus,    // Anchor(status) -> GovernanceState
}

// =============================================================================
// Validation
// =============================================================================

#[hdk_extern]
pub fn genesis_self_check(_data: GenesisSelfCheckData) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

/// Validate DHT operations for all entry types
///
/// This validation callback runs on both:
/// - Author's node when creating entries (blocks invalid entries from source chain)
/// - All peers when gossiping entries (blocks invalid entries from DHT)
///
/// Validation must be deterministic - identical outcomes regardless of validator or timing.
/// Reference: https://developer.holochain.org/build/validation/
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => validate_create_entry(&app_entry),
            OpEntry::UpdateEntry { app_entry, .. } => validate_update_entry(&app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::StoreRecord(store_record) => match store_record {
            OpRecord::CreateEntry { app_entry, .. } => validate_create_entry(&app_entry),
            OpRecord::UpdateEntry { app_entry, .. } => validate_update_entry(&app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(app_entry: &EntryTypes) -> ExternResult<ValidateCallbackResult> {
    match app_entry {
        EntryTypes::Challenge(challenge) => validate_challenge(challenge),
        EntryTypes::Proposal(proposal) => validate_proposal(proposal),
        EntryTypes::Precedent(precedent) => validate_precedent(precedent),
        EntryTypes::Discussion(discussion) => validate_discussion(discussion),
        EntryTypes::GovernanceState(state) => validate_governance_state(state),
        EntryTypes::GovernanceReaction(reaction) => validate_governance_reaction(reaction),
        EntryTypes::GraduatedFeedback(feedback) => validate_graduated_feedback(feedback),
        EntryTypes::ProposalVote(vote) => validate_proposal_vote(vote),
        EntryTypes::OpinionStatement(statement) => validate_opinion_statement(statement),
        EntryTypes::StatementVote(vote) => validate_statement_vote(vote),
        EntryTypes::StringAnchor(_) => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_update_entry(app_entry: &EntryTypes) -> ExternResult<ValidateCallbackResult> {
    validate_create_entry(app_entry)
}

fn validate_challenge(challenge: &Challenge) -> ExternResult<ValidateCallbackResult> {
    if challenge.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Challenge id cannot be empty".into()));
    }
    if !CHALLENGE_GROUNDS.contains(&challenge.grounds.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid challenge grounds: {}", challenge.grounds),
        ));
    }
    if !CHALLENGE_STATUS.contains(&challenge.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid challenge status: {}", challenge.status),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_proposal(proposal: &Proposal) -> ExternResult<ValidateCallbackResult> {
    if proposal.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Proposal id cannot be empty".into()));
    }
    if !PROPOSAL_TYPES.contains(&proposal.proposal_type.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid proposal type: {}", proposal.proposal_type),
        ));
    }
    if !PROPOSAL_STATUS.contains(&proposal.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid proposal status: {}", proposal.status),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_precedent(precedent: &Precedent) -> ExternResult<ValidateCallbackResult> {
    if precedent.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Precedent id cannot be empty".into()));
    }
    if !PRECEDENT_BINDING.contains(&precedent.binding.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid precedent binding level: {}", precedent.binding),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_discussion(discussion: &Discussion) -> ExternResult<ValidateCallbackResult> {
    if discussion.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Discussion id cannot be empty".into()));
    }
    if !DISCUSSION_CATEGORIES.contains(&discussion.category.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid discussion category: {}", discussion.category),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_governance_state(state: &GovernanceState) -> ExternResult<ValidateCallbackResult> {
    if state.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("GovernanceState id cannot be empty".into()));
    }
    if !GOVERNANCE_STATUS.contains(&state.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid governance status: {}", state.status),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_governance_reaction(reaction: &GovernanceReaction) -> ExternResult<ValidateCallbackResult> {
    if reaction.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("GovernanceReaction id cannot be empty".into()));
    }
    if !REACTION_TYPES.contains(&reaction.reaction.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid reaction type: {}", reaction.reaction),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_graduated_feedback(feedback: &GraduatedFeedback) -> ExternResult<ValidateCallbackResult> {
    if feedback.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("GraduatedFeedback id cannot be empty".into()));
    }
    if !FEEDBACK_CONTEXTS.contains(&feedback.feedback_context.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid feedback context: {}", feedback.feedback_context),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_proposal_vote(vote: &ProposalVote) -> ExternResult<ValidateCallbackResult> {
    if vote.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("ProposalVote id cannot be empty".into()));
    }
    if !VOTE_POSITIONS.contains(&vote.position.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid vote position: {}", vote.position),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_opinion_statement(statement: &OpinionStatement) -> ExternResult<ValidateCallbackResult> {
    if statement.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("OpinionStatement id cannot be empty".into()));
    }
    if statement.text.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("OpinionStatement text cannot be empty".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_statement_vote(vote: &StatementVote) -> ExternResult<ValidateCallbackResult> {
    if vote.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("StatementVote id cannot be empty".into()));
    }
    if !STATEMENT_VOTES.contains(&vote.vote.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid statement vote: {}", vote.vote),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
