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
    pub entity_type: String, // content, path, extension, attestation, decision
    pub entity_id: String,
    pub challenger_id: String,
    pub challenger_name: String,
    pub challenger_standing: String, // Attestation level granting standing
    pub grounds: String,             // factual-error, safety, policy, constitutional
    pub description: String,
    pub evidence_json: String, // Evidence[] as JSON
    pub status: String,        // filed, acknowledged, under-review, resolved, dismissed
    pub filed_at: String,
    pub acknowledged_at: Option<String>,
    pub sla_deadline: Option<String>,
    pub assigned_elohim: Option<String>,
    pub priority: String,                // normal, high, critical
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
    pub proposal_type: String, // sense-check, consent, consensus, supermajority
    pub description: String,
    pub proposer_id: String,
    pub proposer_name: String,
    pub rationale: String,
    pub status: String,          // draft, discussion, voting, decided, dismissed
    pub phase: String,           // Current phase
    pub amendments_json: String, // Amendment[] as JSON
    pub voting_config_json: String, // VotingConfig as JSON
    pub current_votes_json: String, // VoteCount as JSON
    pub outcome_json: Option<String>, // ProposalOutcome as JSON
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Proposal types
pub const PROPOSAL_TYPES: [&str; 4] = ["sense-check", "consent", "consensus", "supermajority"];

/// Proposal status states
pub const PROPOSAL_STATUS: [&str; 5] = ["draft", "discussion", "voting", "decided", "dismissed"];

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
    pub binding: String, // constitutional, binding-network, binding-local, persuasive
    pub scope_json: String, // { entityTypes, categories, roles } as JSON
    pub citations: u32,  // How often this precedent is cited
    pub status: String,  // active, superseded, under-review
    pub established_by: String, // Proposal ID or governance body
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
    pub category: String, // general, proposal, challenge, feedback
    pub title: String,
    pub messages_json: String, // DiscussionMessage[] as JSON
    pub status: String,        // open, closed, archived
    pub message_count: u32,
    pub last_activity_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Discussion categories
pub const DISCUSSION_CATEGORIES: [&str; 4] = ["general", "proposal", "challenge", "feedback"];

/// GovernanceState - Current governance status of an entity.
///
/// Tracks the governance posture of content, paths, etc.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GovernanceState {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub status: String,                 // approved, pending, challenged, suspended
    pub status_basis_json: String,      // StatusBasis as JSON
    pub labels_json: String,            // Label[] as JSON
    pub active_challenges_json: String, // String[] as JSON
    pub active_proposals_json: String,  // String[] as JSON
    pub precedent_ids_json: String,     // String[] as JSON
    pub last_updated: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Governance status states
pub const GOVERNANCE_STATUS: [&str; 4] = ["approved", "pending", "challenged", "suspended"];

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

pub const REACTION_TYPES: [&str; 6] = [
    "moved",
    "grateful",
    "challenged",
    "concerned",
    "surprised",
    "illuminated",
];

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

pub const FEEDBACK_CONTEXTS: [&str; 5] =
    ["accuracy", "usefulness", "proposal", "clarity", "relevance"];

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
// Spatial Governance — Place (governed spatial entity)
// =============================================================================
//
// "This is our watershed." Communities witness and govern spatial boundaries.
// If centralized, someone becomes the boundary authority — the land registrar.
// Place is notarized because boundary-drawing IS governance.

/// Place - A named, governed spatial entity.
///
/// Cities, watersheds, land parcels, solar farms, gathering spaces.
/// Notarized on Mishpat DNA because the community must witness
/// "these are our boundaries" — if centralized, someone draws
/// the lines and everyone else lives inside them.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Place {
    pub id: String,
    pub name: String,
    pub place_type: String,           // PlaceType enum value
    pub constitutional_layer: String, // ConstitutionalLayer this place maps to
    /// Primary H3 cell at canonical resolution for this place
    pub h3_index: String,
    pub h3_resolution: u8,
    /// GeoJSON geometry (boundary polygon, point, or multipolygon)
    pub geometry_json: String,
    /// Centroid for quick spatial lookups
    pub centroid_lat: f64,
    pub centroid_lng: f64,
    /// Parent place ID (nesting: parcel → community → bioregion → global)
    pub parent_place_id: Option<String>,
    /// OpenStreetMap reference (OsmReference as JSON)
    pub osm_reference_json: Option<String>,
    /// Carrying capacity constraints (CarryingCapacity[] as JSON)
    pub carrying_capacity_json: String,
    /// Governance collective with authority over this place
    pub governing_collective_id: Option<String>,
    pub status: String, // active, proposed, disputed, dissolved
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

pub const PLACE_TYPES: [&str; 8] = [
    "administrative",
    "bioregional",
    "parcel",
    "infrastructure",
    "gathering",
    "watershed",
    "agricultural",
    "custom",
];

pub const PLACE_STATUS: [&str; 4] = ["active", "proposed", "disputed", "dissolved"];

// =============================================================================
// Gate Decision Attestation — Agent-gate interaction record
// =============================================================================
//
// Notarized on Mishpat DNA because gate decisions are constitutional artifacts.
// If an agent's gate decision were controlled by a single party, that party
// becomes the arbiter of all AI-mediated interactions — the rent-extraction
// vector the protocol is designed to prevent.
//
// Source of truth: Mishpat DHT.
// elohim-storage projection is a read-optimized index with dht_anchor_hash.

/// GateDecisionAttestation — Immutable record of a gate's decision on an interaction.
///
/// Created by the elohim agent after evaluating a RelationalImpactEvent through
/// a declared gate process. All references are content-addressed (CIDs) so the
/// full reasoning chain is auditable without centralized storage.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GateDecisionAttestation {
    /// CID of this attestation (self-addressing). Computed by caller from content hash.
    pub decision_id: String,
    /// Phase discriminator: "dev-context" | "elohim-active"
    pub phase: String,
    /// AgentPubKey of the elohim that made the decision (base64 encoded).
    pub elohim_id: String,
    /// CID of the elohim's substance declaration (model-weights + constitution + deployment-context).
    pub elohim_substance_cid: String,
    /// Name of the gate that produced this decision (e.g. "discernment-gate-v1-mechanical").
    pub gate_name: String,
    /// CID of the GateProcessDeclaration DAG that was executed.
    pub gate_process_cid: String,
    /// Serialized RequestRef — identifies the RelationalImpactEvent that triggered the gate.
    pub request_ref_json: String,
    /// Serialized GateStatus: "allow" | "decline" | "escalate" | "verdict"
    pub decision: String,
    /// Full ConstitutionalReasoning as JSON.
    pub reasoning_json: String,
    /// CID of the assembled GateContext summary (privacy-respecting snapshot).
    pub context_summary_cid: String,
    /// ISO-8601 timestamp of the decision.
    pub decided_at: String,
    /// CID of the universal-band DAG declaration that ran above the domain gate.
    pub universal_band_cid: String,
}

/// Valid phase discriminators for GateDecisionAttestation.
pub const GATE_DECISION_PHASES: [&str; 2] = ["dev-context", "elohim-active"];

/// Valid decision status values for GateDecisionAttestation.
pub const GATE_DECISION_STATUSES: [&str; 4] = ["allow", "decline", "escalate", "verdict"];

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
    Place(Place),
    StringAnchor(StringAnchor),
    GateDecisionAttestation(GateDecisionAttestation),
}

// ============================================================
// LINK TYPES
// ============================================================

#[hdk_link_types]
pub enum LinkTypes {
    // =========================================================================
    // Qahal: Governance Signal links (Loomio/Forby/Polis patterns)
    // =========================================================================
    ContentToReactions,    // Content -> GovernanceReaction
    AgentToReactions,      // Anchor(agent_id) -> GovernanceReaction
    ReactionByType,        // Anchor(reaction_type) -> GovernanceReaction
    ContentToFeedback,     // Content -> GraduatedFeedback
    AgentToFeedback,       // Anchor(agent_id) -> GraduatedFeedback
    FeedbackByContext,     // Anchor(feedback_context) -> GraduatedFeedback
    ProposalToVotes,       // Proposal -> ProposalVote
    AgentToVotes,          // Anchor(agent_id) -> ProposalVote
    VoteByPosition,        // Anchor(position) -> ProposalVote
    ContextToStatements,   // Anchor(context_id) -> OpinionStatement
    AgentToStatements,     // Anchor(agent_id) -> OpinionStatement
    StatementToVotes,      // OpinionStatement -> StatementVote
    AgentToStatementVotes, // Anchor(agent_id) -> StatementVote

    // =========================================================================
    // Qahal: Formal Governance links
    // =========================================================================
    // Challenge
    IdToChallenge,         // Anchor(challenge_id) -> Challenge
    EntityToChallenge,     // Anchor(entity_type:entity_id) -> Challenge
    ChallengerToChallenge, // Anchor(challenger_id) -> Challenge
    ChallengeByStatus,     // Anchor(status) -> Challenge

    // Proposal
    IdToProposal,       // Anchor(proposal_id) -> Proposal
    ProposalByType,     // Anchor(proposal_type) -> Proposal
    ProposerToProposal, // Anchor(proposer_id) -> Proposal
    ProposalByStatus,   // Anchor(status) -> Proposal

    // Precedent
    IdToPrecedent,     // Anchor(precedent_id) -> Precedent
    PrecedentByScope,  // Anchor(scope) -> Precedent
    PrecedentByStatus, // Anchor(status) -> Precedent

    // Discussion
    IdToDiscussion,       // Anchor(discussion_id) -> Discussion
    EntityToDiscussion,   // Anchor(entity_type:entity_id) -> Discussion
    DiscussionByCategory, // Anchor(category) -> Discussion
    DiscussionByStatus,   // Anchor(status) -> Discussion

    // GovernanceState
    IdToGovernanceState,     // Anchor(entity_type:entity_id) -> GovernanceState
    GovernanceStateByStatus, // Anchor(status) -> GovernanceState

    // =========================================================================
    // Place — Governed Spatial Entity
    // =========================================================================
    IdToPlace,          // Anchor(place_id) -> Place
    H3CellToPlace,      // Anchor(h3_index) -> Place (THE key spatial query link)
    PlaceByType,        // Anchor(place_type) -> Place
    PlaceByLayer,       // Anchor(constitutional_layer) -> Place
    ParentToChildPlace, // Place -> Place (containment hierarchy)
    PlaceToCollective,  // Place -> Anchor(collective_id)

    // =========================================================================
    // Gate Decision Attestation — AI gate interaction records
    // =========================================================================
    IdToGateDecision,      // Anchor(decision_id) -> GateDecisionAttestation
    ElohimToGateDecisions, // Anchor(elohim_id) -> GateDecisionAttestation
    GateNameToDecisions,   // Anchor(gate_name) -> GateDecisionAttestation
    PhaseToDecisions,      // Anchor(phase) -> GateDecisionAttestation
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
        EntryTypes::Place(place) => validate_place(place),
        EntryTypes::StringAnchor(_) => Ok(ValidateCallbackResult::Valid),
        EntryTypes::GateDecisionAttestation(attestation) => {
            validate_gate_decision_attestation(attestation)
        }
    }
}

fn validate_update_entry(app_entry: &EntryTypes) -> ExternResult<ValidateCallbackResult> {
    validate_create_entry(app_entry)
}

fn validate_challenge(challenge: &Challenge) -> ExternResult<ValidateCallbackResult> {
    if challenge.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Challenge id cannot be empty".into(),
        ));
    }
    if !CHALLENGE_GROUNDS.contains(&challenge.grounds.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid challenge grounds: {}",
            challenge.grounds
        )));
    }
    if !CHALLENGE_STATUS.contains(&challenge.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid challenge status: {}",
            challenge.status
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_proposal(proposal: &Proposal) -> ExternResult<ValidateCallbackResult> {
    if proposal.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Proposal id cannot be empty".into(),
        ));
    }
    if !PROPOSAL_TYPES.contains(&proposal.proposal_type.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid proposal type: {}",
            proposal.proposal_type
        )));
    }
    if !PROPOSAL_STATUS.contains(&proposal.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid proposal status: {}",
            proposal.status
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_precedent(precedent: &Precedent) -> ExternResult<ValidateCallbackResult> {
    if precedent.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Precedent id cannot be empty".into(),
        ));
    }
    if !PRECEDENT_BINDING.contains(&precedent.binding.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid precedent binding level: {}",
            precedent.binding
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_discussion(discussion: &Discussion) -> ExternResult<ValidateCallbackResult> {
    if discussion.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Discussion id cannot be empty".into(),
        ));
    }
    if !DISCUSSION_CATEGORIES.contains(&discussion.category.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid discussion category: {}",
            discussion.category
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_governance_state(state: &GovernanceState) -> ExternResult<ValidateCallbackResult> {
    if state.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "GovernanceState id cannot be empty".into(),
        ));
    }
    if !GOVERNANCE_STATUS.contains(&state.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid governance status: {}",
            state.status
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_governance_reaction(
    reaction: &GovernanceReaction,
) -> ExternResult<ValidateCallbackResult> {
    if reaction.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "GovernanceReaction id cannot be empty".into(),
        ));
    }
    if !REACTION_TYPES.contains(&reaction.reaction.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid reaction type: {}",
            reaction.reaction
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_graduated_feedback(
    feedback: &GraduatedFeedback,
) -> ExternResult<ValidateCallbackResult> {
    if feedback.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "GraduatedFeedback id cannot be empty".into(),
        ));
    }
    if !FEEDBACK_CONTEXTS.contains(&feedback.feedback_context.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid feedback context: {}",
            feedback.feedback_context
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_proposal_vote(vote: &ProposalVote) -> ExternResult<ValidateCallbackResult> {
    if vote.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ProposalVote id cannot be empty".into(),
        ));
    }
    if !VOTE_POSITIONS.contains(&vote.position.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid vote position: {}",
            vote.position
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_opinion_statement(
    statement: &OpinionStatement,
) -> ExternResult<ValidateCallbackResult> {
    if statement.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "OpinionStatement id cannot be empty".into(),
        ));
    }
    if statement.text.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "OpinionStatement text cannot be empty".into(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_statement_vote(vote: &StatementVote) -> ExternResult<ValidateCallbackResult> {
    if vote.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "StatementVote id cannot be empty".into(),
        ));
    }
    if !STATEMENT_VOTES.contains(&vote.vote.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid statement vote: {}",
            vote.vote
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_place(place: &Place) -> ExternResult<ValidateCallbackResult> {
    if place.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Place id cannot be empty".into(),
        ));
    }
    if place.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Place name cannot be empty".into(),
        ));
    }
    if !PLACE_TYPES.contains(&place.place_type.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid place type: {}",
            place.place_type
        )));
    }
    if !PLACE_STATUS.contains(&place.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid place status: {}",
            place.status
        )));
    }
    if place.h3_index.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Place h3_index cannot be empty".into(),
        ));
    }
    if place.h3_resolution > 15 {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid H3 resolution: {} (must be 0-15)",
            place.h3_resolution
        )));
    }
    // Validate latitude range
    if place.centroid_lat < -90.0 || place.centroid_lat > 90.0 {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid centroid latitude: {} (must be -90 to 90)",
            place.centroid_lat
        )));
    }
    // Validate longitude range
    if place.centroid_lng < -180.0 || place.centroid_lng > 180.0 {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid centroid longitude: {} (must be -180 to 180)",
            place.centroid_lng
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_gate_decision_attestation(
    attestation: &GateDecisionAttestation,
) -> ExternResult<ValidateCallbackResult> {
    if attestation.decision_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "GateDecisionAttestation decision_id cannot be empty".into(),
        ));
    }
    if attestation.elohim_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "GateDecisionAttestation elohim_id cannot be empty".into(),
        ));
    }
    if attestation.gate_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "GateDecisionAttestation gate_name cannot be empty".into(),
        ));
    }
    if !GATE_DECISION_PHASES.contains(&attestation.phase.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid gate decision phase: {} (expected one of {:?})",
            attestation.phase, GATE_DECISION_PHASES
        )));
    }
    if !GATE_DECISION_STATUSES.contains(&attestation.decision.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid gate decision status: {} (expected one of {:?})",
            attestation.decision, GATE_DECISION_STATUSES
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

// =============================================================================
// Tests (native-compilable — no HDK WASM calls, pure logic)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valid_attestation() -> GateDecisionAttestation {
        GateDecisionAttestation {
            decision_id: "bafybeigdecision1".into(),
            phase: "dev-context".into(),
            elohim_id: "uhCAktest".into(),
            elohim_substance_cid: "bafybeielohimsubstance".into(),
            gate_name: "discernment-gate-v1-mechanical".into(),
            gate_process_cid: "bafybeigateprocess".into(),
            request_ref_json: r#"{"eventId":"evt-1"}"#.into(),
            decision: "allow".into(),
            reasoning_json: r#"{"summary":"ok"}"#.into(),
            context_summary_cid: "bafybeicontextsummary".into(),
            decided_at: "2026-04-18T00:00:00Z".into(),
            universal_band_cid: "bafybeiband".into(),
        }
    }

    #[test]
    fn valid_attestation_passes_validation() {
        let att = make_valid_attestation();
        let result = validate_gate_decision_attestation(&att).unwrap();
        assert_eq!(result, ValidateCallbackResult::Valid);
    }

    #[test]
    fn valid_elohim_active_phase_passes() {
        let mut att = make_valid_attestation();
        att.phase = "elohim-active".into();
        let result = validate_gate_decision_attestation(&att).unwrap();
        assert_eq!(result, ValidateCallbackResult::Valid);
    }

    #[test]
    fn all_valid_decision_statuses_pass() {
        for status in &["allow", "decline", "escalate", "verdict"] {
            let mut att = make_valid_attestation();
            att.decision = (*status).into();
            let result = validate_gate_decision_attestation(&att).unwrap();
            assert_eq!(
                result,
                ValidateCallbackResult::Valid,
                "status {} should pass",
                status
            );
        }
    }

    #[test]
    fn invalid_phase_fails_validation() {
        let mut att = make_valid_attestation();
        att.phase = "unknown-phase".into();
        let result = validate_gate_decision_attestation(&att).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn invalid_decision_status_fails_validation() {
        let mut att = make_valid_attestation();
        att.decision = "maybe".into();
        let result = validate_gate_decision_attestation(&att).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn empty_decision_id_fails_validation() {
        let mut att = make_valid_attestation();
        att.decision_id = "".into();
        let result = validate_gate_decision_attestation(&att).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn empty_elohim_id_fails_validation() {
        let mut att = make_valid_attestation();
        att.elohim_id = "".into();
        let result = validate_gate_decision_attestation(&att).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn empty_gate_name_fails_validation() {
        let mut att = make_valid_attestation();
        att.gate_name = "".into();
        let result = validate_gate_decision_attestation(&att).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn gate_decision_attestation_serde_roundtrip() {
        let att = make_valid_attestation();
        // Use serde_json for roundtrip (native-compilable, no HDK required)
        let json = serde_json::to_string(&att).unwrap();
        let decoded: GateDecisionAttestation = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.decision_id, att.decision_id);
        assert_eq!(decoded.phase, att.phase);
        assert_eq!(decoded.elohim_id, att.elohim_id);
        assert_eq!(decoded.gate_name, att.gate_name);
        assert_eq!(decoded.decision, att.decision);
    }
}
