//! Wire types for qahal (governance) domain coordinator functions.
//!
//! These types define the MessagePack-serialized inputs and outputs for
//! mishpat zome calls. They are consumed by:
//! - The mishpat coordinator zome (WASM target)
//! - Any future client that calls mishpat functions
//!
//! This crate is an IoC artifact in `sdk/domains/qahal/`. It must NOT
//! depend on HDK, HDI, or any WASM-specific crates.

use holo_hash::{ActionHash, AgentPubKey};
use serde::{Deserialize, Serialize};

// =============================================================================
// Challenge Types
// =============================================================================

/// Input for mishpat::create_challenge coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateChallengeInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub challenger_id: String,
    pub challenger_name: String,
    pub challenger_standing: String,
    pub grounds: String,
    pub description: String,
    pub evidence_json: String,
    pub status: String,
    pub priority: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sla_deadline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_elohim: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_json: Option<String>,
    pub metadata_json: String,
}

/// Challenge fields.
///
/// Matches the integrity zome's Challenge entry type field-for-field.
/// The integrity zome wraps this with `#[hdk_entry_helper]` for DHT storage;
/// this version uses plain serde for wire format compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Challenge {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub challenger_id: String,
    pub challenger_name: String,
    pub challenger_standing: String,
    pub grounds: String,
    pub description: String,
    pub evidence_json: String,
    pub status: String,
    pub filed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acknowledged_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sla_deadline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_elohim: Option<String>,
    pub priority: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Output from mishpat::create_challenge coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChallengeOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub challenge: Challenge,
}

/// Input for querying challenges.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryChallengesInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenger_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

// =============================================================================
// Proposal Types
// =============================================================================

/// Input for mishpat::create_proposal coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateProposalInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub title: String,
    pub proposal_type: String,
    pub description: String,
    pub proposer_id: String,
    pub proposer_name: String,
    pub rationale: String,
    pub status: String,
    pub phase: String,
    pub amendments_json: String,
    pub voting_config_json: String,
    pub current_votes_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_entity_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_entity_id: Option<String>,
    pub metadata_json: String,
}

/// Proposal fields.
///
/// Matches the integrity zome's Proposal entry type field-for-field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub proposal_type: String,
    pub description: String,
    pub proposer_id: String,
    pub proposer_name: String,
    pub rationale: String,
    pub status: String,
    pub phase: String,
    pub amendments_json: String,
    pub voting_config_json: String,
    pub current_votes_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_entity_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_entity_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Output from mishpat::create_proposal coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProposalOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub proposal: Proposal,
}

/// Input for querying proposals.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryProposalsInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

// =============================================================================
// Precedent Types
// =============================================================================

/// Input for mishpat::create_precedent coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreatePrecedentInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub title: String,
    pub summary: String,
    pub full_reasoning: String,
    pub binding: String,
    pub scope_json: String,
    pub established_by: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    pub metadata_json: String,
}

/// Precedent fields.
///
/// Matches the integrity zome's Precedent entry type field-for-field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Precedent {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub full_reasoning: String,
    pub binding: String,
    pub scope_json: String,
    pub citations: u32,
    pub status: String,
    pub established_by: String,
    pub established_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Output from mishpat::create_precedent coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PrecedentOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub precedent: Precedent,
}

/// Input for querying precedents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryPrecedentsInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

// =============================================================================
// Discussion Types
// =============================================================================

/// Input for mishpat::create_discussion coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateDiscussionInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub category: String,
    pub title: String,
    pub messages_json: String,
    pub status: String,
    pub metadata_json: String,
}

/// Discussion fields.
///
/// Matches the integrity zome's Discussion entry type field-for-field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Discussion {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub category: String,
    pub title: String,
    pub messages_json: String,
    pub status: String,
    pub message_count: u32,
    pub last_activity_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Output from mishpat::create_discussion coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DiscussionOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub discussion: Discussion,
}

/// Input for querying discussions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryDiscussionsInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

// =============================================================================
// GovernanceState Types
// =============================================================================

/// Input for mishpat::set_governance_state coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateGovernanceStateInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub status: String,
    pub status_basis_json: String,
    pub labels_json: String,
    pub active_challenges_json: String,
    pub active_proposals_json: String,
    pub precedent_ids_json: String,
    pub metadata_json: String,
}

/// GovernanceState fields.
///
/// Matches the integrity zome's GovernanceState entry type field-for-field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GovernanceState {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub status: String,
    pub status_basis_json: String,
    pub labels_json: String,
    pub active_challenges_json: String,
    pub active_proposals_json: String,
    pub precedent_ids_json: String,
    pub last_updated: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Output from mishpat::set_governance_state coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GovernanceStateOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub governance_state: GovernanceState,
}

/// Input for getting governance state by entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetGovernanceStateInput {
    pub entity_type: String,
    pub entity_id: String,
}

/// Input for querying governance states.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryGovernanceStatesInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

// =============================================================================
// GovernanceReaction Types
// =============================================================================

/// Input for mishpat::create_governance_reaction coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateGovernanceReactionInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub content_id: String,
    pub content_type: String,
    pub reactor_id: String,
    pub reaction: String,
    pub intensity: u8,
    pub mediated: bool,
    pub mediation_accepted: bool,
    pub context_json: String,
    pub metadata_json: String,
}

/// GovernanceReaction fields.
///
/// Matches the integrity zome's GovernanceReaction entry type field-for-field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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

/// Output from mishpat::create_governance_reaction coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GovernanceReactionOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub reaction: GovernanceReaction,
}

/// Input for querying governance reactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryGovernanceReactionsInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reactor_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reaction_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

// =============================================================================
// GraduatedFeedback Types
// =============================================================================

/// Input for mishpat::create_graduated_feedback coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateGraduatedFeedbackInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub content_id: String,
    pub content_type: String,
    pub responder_id: String,
    pub feedback_context: String,
    pub position: i8,
    pub intensity: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    pub metadata_json: String,
}

/// GraduatedFeedback fields.
///
/// Matches the integrity zome's GraduatedFeedback entry type field-for-field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GraduatedFeedback {
    pub id: String,
    pub content_id: String,
    pub content_type: String,
    pub responder_id: String,
    pub feedback_context: String,
    pub position: i8,
    pub intensity: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    pub updated_count: u32,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Output from mishpat::create_graduated_feedback coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GraduatedFeedbackOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub feedback: GraduatedFeedback,
}

/// Input for querying graduated feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryGraduatedFeedbackInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub responder_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback_context: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

// =============================================================================
// ProposalVote Types
// =============================================================================

/// Input for mishpat::create_proposal_vote coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateProposalVoteInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub proposal_id: String,
    pub voter_id: String,
    pub voter_name: String,
    pub position: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_position: Option<String>,
    pub metadata_json: String,
}

/// ProposalVote fields.
///
/// Matches the integrity zome's ProposalVote entry type field-for-field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProposalVote {
    pub id: String,
    pub proposal_id: String,
    pub voter_id: String,
    pub voter_name: String,
    pub position: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_position: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Output from mishpat::create_proposal_vote coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProposalVoteOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub vote: ProposalVote,
}

/// Input for querying proposal votes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryProposalVotesInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

// =============================================================================
// OpinionStatement Types
// =============================================================================

/// Input for mishpat::create_opinion_statement coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateOpinionStatementInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub context_id: String,
    pub author_id: String,
    pub text: String,
    pub status: String,
    pub metadata_json: String,
}

/// OpinionStatement fields.
///
/// Matches the integrity zome's OpinionStatement entry type field-for-field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cluster_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Output from mishpat::create_opinion_statement coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OpinionStatementOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub statement: OpinionStatement,
}

/// Input for querying opinion statements.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryOpinionStatementsInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

// =============================================================================
// StatementVote Types
// =============================================================================

/// Input for mishpat::create_statement_vote coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateStatementVoteInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub statement_id: String,
    pub voter_id: String,
    pub vote: String,
    pub metadata_json: String,
}

/// StatementVote fields.
///
/// Matches the integrity zome's StatementVote entry type field-for-field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StatementVote {
    pub id: String,
    pub statement_id: String,
    pub voter_id: String,
    pub vote: String,
    pub created_at: String,
    pub metadata_json: String,
}

/// Output from mishpat::create_statement_vote coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StatementVoteOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub statement_vote: StatementVote,
}

/// Input for querying statement votes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryStatementVotesInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

// =============================================================================
// Gate Decision Attestation Types
// =============================================================================

/// Input for mishpat::create_gate_decision_attestation coordinator function.
///
/// All fields are flat Strings — no AgentPubKey or specific types — to avoid
/// HDK version drift concerns when this crate is consumed from native targets.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateGateDecisionAttestationInput {
    /// CID of this attestation (self-addressing, computed by caller from content hash).
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

/// GateDecisionAttestation wire type.
///
/// Matches the integrity zome's GateDecisionAttestation entry type field-for-field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GateDecisionAttestation {
    pub decision_id: String,
    pub phase: String,
    pub elohim_id: String,
    pub elohim_substance_cid: String,
    pub gate_name: String,
    pub gate_process_cid: String,
    pub request_ref_json: String,
    pub decision: String,
    pub reasoning_json: String,
    pub context_summary_cid: String,
    pub decided_at: String,
    pub universal_band_cid: String,
}

/// Output from mishpat::create_gate_decision_attestation coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GateDecisionAttestationOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub attestation: GateDecisionAttestation,
}

/// Input for querying gate decision attestations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryGateDecisionsInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elohim_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

// =============================================================================
// Gate Decision Challenge + Indemnification Types (Phase 11 Task 11.1)
// =============================================================================

/// Input for mishpat::create_gate_decision_challenge coordinator function.
///
/// A GateDecisionChallenge is a formal challenge filed against a prior
/// GateDecisionAttestation. Per the Challenge + Indemnification spec
/// (genesis/docs/superpowers/specs/2026-04-19-gate-challenge-and-indemnification-design.md).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateGateDecisionChallengeInput {
    /// CID of this challenge (self-addressing, computed by caller from content hash).
    pub challenge_id: String,
    /// CID of the GateDecisionAttestation being challenged.
    pub challenged_decision_cid: String,
    /// AgentPubKey of the challenger (base64 encoded).
    pub challenger_id: String,
    /// Grounds: factual-error | safety | policy | constitutional | indemnification-request
    pub grounds: String,
    /// Challenger's articulation of the grievance.
    pub summary: String,
    /// Content-addressed evidence refs (comma-separated CIDs; empty if none).
    pub evidence_refs: String,
    /// ISO-8601 timestamp of filing.
    pub filed_at: String,
    /// Reach level: self | intimate | community | commons
    pub reach: String,
}

/// GateDecisionChallenge wire type. Field-for-field mirror of the integrity entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GateDecisionChallenge {
    pub challenge_id: String,
    pub challenged_decision_cid: String,
    pub challenger_id: String,
    pub grounds: String,
    pub summary: String,
    pub evidence_refs: String,
    pub filed_at: String,
    pub reach: String,
}

/// Output from mishpat::create_gate_decision_challenge coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GateDecisionChallengeOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub challenge: GateDecisionChallenge,
}

/// Input for mishpat::create_challenge_outcome coordinator function.
///
/// ChallengeOutcome records the reviewer consensus verdict closing a
/// GateDecisionChallenge. Indemnification actions are carried as JSON to
/// keep the integrity entry shape flat; richer typed shape is future work.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateChallengeOutcomeInput {
    /// CID of this outcome (self-addressing).
    pub outcome_id: String,
    /// CID of the GateDecisionChallenge this outcome closes.
    pub challenge_cid: String,
    /// Verdict: upheld | dismissed | superseded
    pub verdict: String,
    /// AgentPubKeys of the reviewers who reached consensus (comma-separated, base64).
    pub reviewer_consensus: String,
    /// Full ConstitutionalReasoning as JSON.
    pub reasoning_json: String,
    /// ISO-8601 timestamp of decision.
    pub decided_at: String,
    /// Indemnification actions as JSON array (empty if no action required).
    pub indemnification_actions_json: String,
}

/// ChallengeOutcome wire type. Field-for-field mirror of the integrity entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChallengeOutcome {
    pub outcome_id: String,
    pub challenge_cid: String,
    pub verdict: String,
    pub reviewer_consensus: String,
    pub reasoning_json: String,
    pub decided_at: String,
    pub indemnification_actions_json: String,
}

/// Output from mishpat::create_challenge_outcome coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChallengeOutcomeOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub outcome: ChallengeOutcome,
}

/// Input for querying challenge outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryChallengeOutcomesInput {
    /// When set, filter by verdict: upheld | dismissed | superseded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<String>,
    /// When set, filter by challenge_cid (returns at most one outcome).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenge_cid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// Input for querying gate decision challenges.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryGateChallengesInput {
    /// When set, filter by challenged_decision_cid (all challenges against a decision).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenged_decision_cid: Option<String>,
    /// When set, filter by challenger_id (all challenges by an agent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenger_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

// =============================================================================
// Credential Verification Types
// =============================================================================

/// Status of a verified credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum VerificationStatus {
    Valid,
    NotFound,
    Revoked,
    Expired,
}

/// Result of verifying a single credential against the DHT.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CredentialVerification {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub hash: ActionHash,
    pub status: VerificationStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(type = "string | null"))]
    pub agent: Option<AgentPubKey>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_challenge_input_msgpack_roundtrip() {
        let input = CreateChallengeInput {
            id: Some("chal-1".into()),
            entity_type: "content".into(),
            entity_id: "content-123".into(),
            challenger_id: "user-1".into(),
            challenger_name: "Test User".into(),
            challenger_standing: "steward".into(),
            grounds: "factual-error".into(),
            description: "Incorrect data".into(),
            evidence_json: "[]".into(),
            status: "filed".into(),
            priority: "normal".into(),
            sla_deadline: None,
            assigned_elohim: None,
            resolution_json: None,
            metadata_json: "{}".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateChallengeInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.entity_type, "content");
        assert_eq!(decoded.id, Some("chal-1".into()));
    }

    #[test]
    fn create_proposal_input_msgpack_roundtrip() {
        let input = CreateProposalInput {
            id: None,
            title: "Test Proposal".into(),
            proposal_type: "consent".into(),
            description: "A test".into(),
            proposer_id: "user-1".into(),
            proposer_name: "Test User".into(),
            rationale: "Because".into(),
            status: "draft".into(),
            phase: "discussion".into(),
            amendments_json: "[]".into(),
            voting_config_json: "{}".into(),
            current_votes_json: "{}".into(),
            outcome_json: None,
            related_entity_type: None,
            related_entity_id: None,
            metadata_json: "{}".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateProposalInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.title, "Test Proposal");
    }

    #[test]
    fn create_precedent_input_msgpack_roundtrip() {
        let input = CreatePrecedentInput {
            id: None,
            title: "Test Precedent".into(),
            summary: "Summary".into(),
            full_reasoning: "Reasoning".into(),
            binding: "constitutional".into(),
            scope_json: "{}".into(),
            established_by: "council".into(),
            status: "active".into(),
            superseded_by: None,
            metadata_json: "{}".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreatePrecedentInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.title, "Test Precedent");
    }

    #[test]
    fn create_discussion_input_msgpack_roundtrip() {
        let input = CreateDiscussionInput {
            id: None,
            entity_type: "proposal".into(),
            entity_id: "prop-1".into(),
            category: "general".into(),
            title: "Discussion".into(),
            messages_json: "[]".into(),
            status: "open".into(),
            metadata_json: "{}".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateDiscussionInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.category, "general");
    }

    #[test]
    fn create_governance_state_input_msgpack_roundtrip() {
        let input = CreateGovernanceStateInput {
            id: None,
            entity_type: "content".into(),
            entity_id: "content-1".into(),
            status: "approved".into(),
            status_basis_json: "{}".into(),
            labels_json: "[]".into(),
            active_challenges_json: "[]".into(),
            active_proposals_json: "[]".into(),
            precedent_ids_json: "[]".into(),
            metadata_json: "{}".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateGovernanceStateInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.status, "approved");
    }

    #[test]
    fn create_governance_reaction_input_msgpack_roundtrip() {
        let input = CreateGovernanceReactionInput {
            id: None,
            content_id: "content-1".into(),
            content_type: "article".into(),
            reactor_id: "user-1".into(),
            reaction: "moved".into(),
            intensity: 3,
            mediated: false,
            mediation_accepted: false,
            context_json: "{}".into(),
            metadata_json: "{}".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateGovernanceReactionInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.reaction, "moved");
        assert_eq!(decoded.intensity, 3);
    }

    #[test]
    fn create_graduated_feedback_input_msgpack_roundtrip() {
        let input = CreateGraduatedFeedbackInput {
            id: None,
            content_id: "content-1".into(),
            content_type: "article".into(),
            responder_id: "user-1".into(),
            feedback_context: "accuracy".into(),
            position: 2,
            intensity: 4,
            reasoning: Some("Good content".into()),
            metadata_json: "{}".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateGraduatedFeedbackInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.position, 2);
        assert_eq!(decoded.reasoning, Some("Good content".into()));
    }

    #[test]
    fn create_proposal_vote_input_msgpack_roundtrip() {
        let input = CreateProposalVoteInput {
            id: None,
            proposal_id: "prop-1".into(),
            voter_id: "user-1".into(),
            voter_name: "Test Voter".into(),
            position: "agree".into(),
            reasoning: None,
            version: 1,
            previous_position: None,
            metadata_json: "{}".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateProposalVoteInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.position, "agree");
        assert_eq!(decoded.version, 1);
    }

    #[test]
    fn create_opinion_statement_input_msgpack_roundtrip() {
        let input = CreateOpinionStatementInput {
            id: None,
            context_id: "ctx-1".into(),
            author_id: "user-1".into(),
            text: "I think this is important".into(),
            status: "active".into(),
            metadata_json: "{}".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateOpinionStatementInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.text, "I think this is important");
    }

    #[test]
    fn create_statement_vote_input_msgpack_roundtrip() {
        let input = CreateStatementVoteInput {
            id: None,
            statement_id: "stmt-1".into(),
            voter_id: "user-1".into(),
            vote: "agree".into(),
            metadata_json: "{}".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateStatementVoteInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.vote, "agree");
    }

    #[test]
    fn create_gate_decision_attestation_input_msgpack_roundtrip() {
        let input = CreateGateDecisionAttestationInput {
            decision_id: "bafybeigdecision1".into(),
            phase: "dev-context".into(),
            elohim_id: "uhCAk...".into(),
            elohim_substance_cid: "bafybeielohimsubstance".into(),
            gate_name: "discernment-gate-v1-mechanical".into(),
            gate_process_cid: "bafybeigateprocess".into(),
            request_ref_json: r#"{"eventId":"evt-1","requestedAt":"2026-04-18T00:00:00Z"}"#.into(),
            decision: "allow".into(),
            reasoning_json: r#"{"summary":"No concerns","rules":[]}"#.into(),
            context_summary_cid: "bafybeicontextsummary".into(),
            decided_at: "2026-04-18T00:00:00Z".into(),
            universal_band_cid: "bafybeiband".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateGateDecisionAttestationInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.phase, "dev-context");
        assert_eq!(decoded.decision, "allow");
        assert_eq!(decoded.gate_name, "discernment-gate-v1-mechanical");
    }

    #[test]
    fn create_gate_decision_challenge_input_msgpack_roundtrip() {
        let input = CreateGateDecisionChallengeInput {
            challenge_id: "bafybeichallenge1".into(),
            challenged_decision_cid: "bafybeigdecision1".into(),
            challenger_id: "uhCAk-challenger".into(),
            grounds: "constitutional".into(),
            summary: "Decision appears to violate P4 principle".into(),
            evidence_refs: "bafybeievidence1,bafybeievidence2".into(),
            filed_at: "2026-04-19T00:00:00Z".into(),
            reach: "community".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateGateDecisionChallengeInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.grounds, "constitutional");
        assert_eq!(decoded.challenged_decision_cid, "bafybeigdecision1");
    }

    #[test]
    fn gate_decision_challenge_msgpack_roundtrip() {
        let entry = GateDecisionChallenge {
            challenge_id: "bafybeichallenge1".into(),
            challenged_decision_cid: "bafybeigdecision1".into(),
            challenger_id: "uhCAk-challenger".into(),
            grounds: "safety".into(),
            summary: "Concern about content safety".into(),
            evidence_refs: String::new(),
            filed_at: "2026-04-19T00:00:00Z".into(),
            reach: "intimate".into(),
        };
        let bytes = rmp_serde::to_vec_named(&entry).unwrap();
        let decoded: GateDecisionChallenge = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.grounds, "safety");
        assert_eq!(decoded.reach, "intimate");
    }

    #[test]
    fn create_challenge_outcome_input_msgpack_roundtrip() {
        let input = CreateChallengeOutcomeInput {
            outcome_id: "bafybeioutcome1".into(),
            challenge_cid: "bafybeichallenge1".into(),
            verdict: "upheld".into(),
            reviewer_consensus: "uhCAk-reviewer1,uhCAk-reviewer2".into(),
            reasoning_json: r#"{"summary":"Evidence confirms challenger's grounds"}"#.into(),
            decided_at: "2026-04-20T00:00:00Z".into(),
            indemnification_actions_json: r#"[{"kind":"reputation-degrade","dimensions":["appeals-sustained"],"magnitude":0.15}]"#.into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateChallengeOutcomeInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.verdict, "upheld");
        assert_eq!(decoded.challenge_cid, "bafybeichallenge1");
    }

    #[test]
    fn challenge_outcome_msgpack_roundtrip() {
        let entry = ChallengeOutcome {
            outcome_id: "bafybeioutcome1".into(),
            challenge_cid: "bafybeichallenge1".into(),
            verdict: "dismissed".into(),
            reviewer_consensus: "uhCAk-reviewer1".into(),
            reasoning_json: r#"{"summary":"Insufficient evidence"}"#.into(),
            decided_at: "2026-04-20T00:00:00Z".into(),
            indemnification_actions_json: "[]".into(),
        };
        let bytes = rmp_serde::to_vec_named(&entry).unwrap();
        let decoded: ChallengeOutcome = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.verdict, "dismissed");
    }

    #[test]
    fn query_challenge_outcomes_input_msgpack_roundtrip() {
        let input = QueryChallengeOutcomesInput {
            verdict: Some("upheld".into()),
            challenge_cid: None,
            limit: Some(50),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: QueryChallengeOutcomesInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.verdict.as_deref(), Some("upheld"));
        assert_eq!(decoded.limit, Some(50));
    }
}


#[cfg(test)]
#[cfg(feature = "ts")]
mod ts_export {
    use super::*;
    use ts_rs::TS;

    #[test]
    fn export_bindings() {
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bindings");
        CreateChallengeInput::export_all_to(&out).unwrap();
        Challenge::export_all_to(&out).unwrap();
        ChallengeOutput::export_all_to(&out).unwrap();
        QueryChallengesInput::export_all_to(&out).unwrap();
        CreateProposalInput::export_all_to(&out).unwrap();
        Proposal::export_all_to(&out).unwrap();
        ProposalOutput::export_all_to(&out).unwrap();
        QueryProposalsInput::export_all_to(&out).unwrap();
        CreatePrecedentInput::export_all_to(&out).unwrap();
        Precedent::export_all_to(&out).unwrap();
        PrecedentOutput::export_all_to(&out).unwrap();
        QueryPrecedentsInput::export_all_to(&out).unwrap();
        CreateDiscussionInput::export_all_to(&out).unwrap();
        Discussion::export_all_to(&out).unwrap();
        DiscussionOutput::export_all_to(&out).unwrap();
        QueryDiscussionsInput::export_all_to(&out).unwrap();
        CreateGovernanceStateInput::export_all_to(&out).unwrap();
        GovernanceState::export_all_to(&out).unwrap();
        GovernanceStateOutput::export_all_to(&out).unwrap();
        GetGovernanceStateInput::export_all_to(&out).unwrap();
        QueryGovernanceStatesInput::export_all_to(&out).unwrap();
        CreateGovernanceReactionInput::export_all_to(&out).unwrap();
        GovernanceReaction::export_all_to(&out).unwrap();
        GovernanceReactionOutput::export_all_to(&out).unwrap();
        QueryGovernanceReactionsInput::export_all_to(&out).unwrap();
        CreateGraduatedFeedbackInput::export_all_to(&out).unwrap();
        GraduatedFeedback::export_all_to(&out).unwrap();
        GraduatedFeedbackOutput::export_all_to(&out).unwrap();
        QueryGraduatedFeedbackInput::export_all_to(&out).unwrap();
        CreateProposalVoteInput::export_all_to(&out).unwrap();
        ProposalVote::export_all_to(&out).unwrap();
        ProposalVoteOutput::export_all_to(&out).unwrap();
        QueryProposalVotesInput::export_all_to(&out).unwrap();
        CreateOpinionStatementInput::export_all_to(&out).unwrap();
        OpinionStatement::export_all_to(&out).unwrap();
        OpinionStatementOutput::export_all_to(&out).unwrap();
        QueryOpinionStatementsInput::export_all_to(&out).unwrap();
        CreateStatementVoteInput::export_all_to(&out).unwrap();
        StatementVote::export_all_to(&out).unwrap();
        StatementVoteOutput::export_all_to(&out).unwrap();
        QueryStatementVotesInput::export_all_to(&out).unwrap();
        VerificationStatus::export_all_to(&out).unwrap();
        CredentialVerification::export_all_to(&out).unwrap();
        CreateGateDecisionAttestationInput::export_all_to(&out).unwrap();
        GateDecisionAttestation::export_all_to(&out).unwrap();
        GateDecisionAttestationOutput::export_all_to(&out).unwrap();
        QueryGateDecisionsInput::export_all_to(&out).unwrap();
        CreateGateDecisionChallengeInput::export_all_to(&out).unwrap();
        GateDecisionChallenge::export_all_to(&out).unwrap();
        GateDecisionChallengeOutput::export_all_to(&out).unwrap();
        QueryGateChallengesInput::export_all_to(&out).unwrap();
        CreateChallengeOutcomeInput::export_all_to(&out).unwrap();
        ChallengeOutcome::export_all_to(&out).unwrap();
        ChallengeOutcomeOutput::export_all_to(&out).unwrap();
        QueryChallengeOutcomesInput::export_all_to(&out).unwrap();
    }
}
