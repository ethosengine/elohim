//! qahal view types — migrated from elohim-storage/src/views.rs (VIEWS.T2).

use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json::Value;
use ts_rs::TS;
use std::collections::HashMap;
use crate::shared::*;
#[allow(unused_imports)]
use crate::imagodei::*;
#[allow(unused_imports)]
use crate::shefa::*;
#[allow(unused_imports)]
use crate::lamad::*;
#[allow(unused_imports)]
use crate::infrastructure::*;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GovernanceStateView {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub reach: String,
    pub labels: JsonVal,
    pub voting_state: String,
    pub signal_count: i32,
    pub created_at: String,
    pub updated_at: String,
    pub dht_anchor_hash: Option<String>,
    // --- Wire-type fields from qahal-types::GovernanceState ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_basis: Option<JsonVal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_challenges: Option<JsonVal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_proposals: Option<JsonVal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precedent_ids: Option<JsonVal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonVal>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ChallengeView {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub challenger_id: String,
    pub standing_basis: String,
    pub grounds_primary: String,
    pub grounds_secondary: Option<String>,
    pub evidence: JsonVal,
    pub requested_outcome: Option<String>,
    pub state: String,
    pub response_outcome: Option<String>,
    pub response_reasoning: Option<String>,
    pub response_actions: Option<String>,
    pub response_by: Option<String>,
    pub sets_precedent: bool,
    pub filed_at: String,
    pub acknowledged_at: Option<String>,
    pub response_deadline: String,
    pub responded_at: Option<String>,
    pub resolved_at: Option<String>,
    pub created_at: String,
    pub sla_status: String,
    pub dht_anchor_hash: Option<String>,
    // --- Wire-type fields from qahal-types::Challenge ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenger_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenger_standing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grounds: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sla_deadline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_elohim: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<JsonVal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonVal>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AppealView {
    pub id: String,
    pub challenge_id: String,
    pub appellant_id: String,
    pub grounds: String,
    pub additional_evidence: Option<String>,
    pub state: String,
    pub escalation_level: Option<String>,
    pub decision: Option<String>,
    pub decision_reasoning: Option<String>,
    pub decided_by: Option<String>,
    pub filed_at: String,
    pub decided_at: Option<String>,
    pub created_at: String,
    pub dht_anchor_hash: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct FileChallengeInputView {
    pub entity_type: String,
    pub entity_id: String,
    pub challenger_id: String,
    pub standing_basis: String,
    pub grounds_primary: String,
    pub grounds_secondary: Option<String>,
    pub evidence: String,
    pub requested_outcome: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RespondToChallengeInputView {
    pub outcome: String,
    pub reasoning: String,
    pub actions: Option<String>,
    pub sets_precedent: bool,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct FileAppealInputView {
    pub grounds: String,
    pub additional_evidence: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ProposalView {
    pub id: String,
    pub content_id: String,
    pub proposer_presence_id: String,
    pub proposal_type: String,
    pub title: String,
    pub body: String,
    pub status: String,
    pub votes_for: i32,
    pub votes_against: i32,
    pub voting_anonymous: bool,
    pub created_at: String,
    pub updated_at: String,
    pub voting_mechanism: String,
    pub score_min: Option<i32>,
    pub score_max: Option<i32>,
    pub dots_per_voter: Option<i32>,
    pub quorum_percentage: Option<f64>,
    pub passage_threshold: Option<f64>,
    pub dht_anchor_hash: Option<String>,
    // --- Wire-type fields from qahal-types::Proposal ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposer_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amendments: Option<JsonVal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voting_config: Option<JsonVal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_votes: Option<JsonVal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<JsonVal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_entity_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_entity_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonVal>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PrecedentView {
    pub id: String,
    pub content_id: String,
    pub principle: String,
    pub interpretation: String,
    pub established_by: String,
    pub created_at: String,
    pub dht_anchor_hash: Option<String>,
    // --- Wire-type fields from qahal-types::Precedent ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_reasoning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<JsonVal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citations: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub established_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonVal>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DiscussionView {
    pub id: String,
    pub content_id: String,
    pub author_presence_id: String,
    pub body: String,
    pub parent_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    // --- Wire-type fields from qahal-types::Discussion ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub messages: Option<JsonVal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_activity_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonVal>,
}

/// Vote on a governance proposal — API response
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct VoteView {
    pub id: String,
    pub proposal_id: String,
    pub human_id: Option<String>,
    pub position: String,
    pub reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub dht_anchor_hash: Option<String>,
    // --- Wire-type fields from qahal-types::ProposalVote ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voter_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_position: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonVal>,
}

/// Create a proposal — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateProposalInputView {
    pub id: String,
    pub content_id: String,
    pub proposer_presence_id: String,
    pub proposal_type: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub voting_anonymous: bool,
    #[serde(default = "default_voting_mechanism")]
    pub voting_mechanism: String,
    pub score_min: Option<i32>,
    pub score_max: Option<i32>,
    pub dots_per_voter: Option<i32>,
    pub quorum_percentage: Option<f64>,
    pub passage_threshold: Option<f64>,
}

/// Cast or update a vote — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CastVoteInputView {
    pub human_id: String,
    pub position: String,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Proposal option — API response
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ProposalOptionView {
    pub id: String,
    pub proposal_id: String,
    pub label: String,
    pub description: String,
    pub position: i32,
    pub source: Option<String>,
    pub source_justification: Option<String>,
    pub created_at: String,
    pub dht_anchor_hash: Option<String>,
}

/// Create proposal options — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateProposalOptionInputView {
    pub id: String,
    pub label: String,
    pub description: String,
    pub position: i32,
    pub source: Option<String>,
    pub source_justification: Option<String>,
}

/// Ranked/scored/dot vote — API response
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RankedVoteView {
    pub id: String,
    pub proposal_id: String,
    pub human_id: Option<String>,
    pub option_id: String,
    pub rank: Option<i32>,
    pub score: Option<i32>,
    pub dots: Option<i32>,
    pub approved: Option<bool>,
    pub reasoning: Option<String>,
    pub proxy_elohim_id: Option<String>,
    pub created_at: String,
    pub dht_anchor_hash: Option<String>,
}

/// Single entry in a ranked/scored/dot ballot
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct BallotEntry {
    pub option_id: String,
    pub rank: Option<i32>,
    pub score: Option<i32>,
    pub dots: Option<i32>,
    pub approved: Option<bool>,
}

/// Cast a ranked/scored/dot/approval vote — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CastRankedVoteInputView {
    pub human_id: String,
    pub ballots: Vec<BallotEntry>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub proxy_elohim_id: Option<String>,
    #[serde(default)]
    pub proxy_justification: Option<String>,
}

/// Governance signal — API response
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GovernanceSignalView {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub human_id: String,
    pub signal_type: String,
    pub signal_value: String,
    pub mechanism_level: i32,
    pub proxy_elohim_id: Option<String>,
    pub created_at: String,
    pub dht_anchor_hash: Option<String>,
}

/// Aggregated governance signal statistics for an entity
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SignalAggregateView {
    pub entity_type: String,
    pub entity_id: String,
    pub total_signals: i64,
    pub by_type: HashMap<String, i64>,
    pub by_value: HashMap<String, i64>,
    pub unique_participants: i64,
    pub consensus_strength: f64,
}

/// Record a governance signal — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RecordSignalInputView {
    pub entity_type: String,
    pub entity_id: String,
    pub human_id: String,
    pub signal_type: String,
    pub signal_value: String,
    pub mechanism_level: i32,
    #[serde(default)]
    pub proxy_elohim_id: Option<String>,
}

/// Governance disposition — persistent governance profile (B: Agent-Scoped)
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GovernanceDispositionView {
    pub id: String,
    pub human_id: String,
    pub risk_tolerance: f64,
    pub change_openness: f64,
    pub consensus_preference: f64,
    pub priority_values: JsonVal,
    pub voting_pattern_summary: JsonVal,
    pub total_votes_cast: i32,
    pub total_challenges_filed: i32,
    pub total_signals_recorded: i32,
    pub dht_anchor_hash: Option<String>,
    pub last_computed_at: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Update disposition manual overrides — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpdateDispositionInputView {
    #[serde(default)]
    pub risk_tolerance: Option<f64>,
    #[serde(default)]
    pub change_openness: Option<f64>,
    #[serde(default)]
    pub consensus_preference: Option<f64>,
    #[serde(default)]
    pub priority_values: Option<JsonVal>,
}

/// Start a discussion — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateDiscussionInputView {
    pub id: String,
    pub content_id: String,
    pub author_presence_id: String,
    pub body: String,
    #[serde(default)]
    pub parent_id: Option<String>,
}

/// Post a message (reply) in a discussion — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PostMessageInputView {
    pub id: String,
    pub author_presence_id: String,
    pub body: String,
}

/// Governance reaction — API response
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GovernanceReactionView {
    pub id: String,
    pub content_id: String,
    pub content_type: String,
    pub reactor_id: String,
    pub reaction: String,
    pub intensity: u8,
    pub mediated: bool,
    pub mediation_accepted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<JsonVal>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonVal>,
}

/// Graduated feedback — API response
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GraduatedFeedbackView {
    pub id: String,
    pub content_id: String,
    pub content_type: String,
    pub responder_id: String,
    pub feedback_context: String,
    pub position: i8,
    pub intensity: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_count: Option<u32>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonVal>,
}

/// Statement — API response
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StatementView {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub human_id: String,
    pub text: String,
    pub agree_count: i32,
    pub disagree_count: i32,
    pub pass_count: i32,
    pub group_id: Option<String>,
    pub is_bridging: bool,
    pub created_at: String,
    pub dht_anchor_hash: Option<String>,
    // --- Wire-type fields from qahal-types::OpinionStatement ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vote_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consensus_score: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cluster: Option<JsonVal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonVal>,
}

/// Statement vote — API response
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StatementVoteView {
    pub id: String,
    pub statement_id: String,
    pub human_id: String,
    pub vote: String,
    pub created_at: String,
    pub dht_anchor_hash: Option<String>,
    // --- Wire-type fields from qahal-types::StatementVote ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonVal>,
}

/// Create a statement — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateStatementInputView {
    pub entity_type: String,
    pub entity_id: String,
    pub human_id: String,
    pub text: String,
}

/// Vote on a statement — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct VoteOnStatementInputView {
    pub human_id: String,
    pub vote: String,
}

/// Sensemaking clustering result — API response
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SensemakingResultView {
    pub entity_type: String,
    pub entity_id: String,
    pub clusters: Vec<OpinionClusterView>,
    pub bridging_statements: Vec<StatementView>,
    pub divisive_statements: Vec<StatementView>,
    pub statement_metrics: Vec<StatementMetricsView>,
    pub participant_positions: Vec<ParticipantPositionView>,
    pub total_participants: usize,
    pub total_statements: usize,
}

/// Opinion cluster within a sensemaking result
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct OpinionClusterView {
    pub id: String,
    pub member_count: usize,
    pub characteristic_statements: Vec<StatementView>,
    pub internal_agreement: f64,
}

/// 2D position of a participant in the PCA-projected opinion landscape
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ParticipantPositionView {
    pub human_id: String,
    pub x: f64,
    pub y: f64,
    pub cluster_id: String,
}

/// Per-statement metrics for sensemaking visualization
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StatementMetricsView {
    pub statement_id: String,
    /// Overall agreement across all participants (-1.0 = all disagree, 1.0 = all agree)
    pub overall_agreement: f64,
    /// Variance of agreement ratios across clusters (0 = uniform, high = divisive)
    pub cross_cluster_variance: f64,
    /// Classification: "bridging", "divisive", "characteristic", or "neutral"
    pub classification: String,
}

/// Wire view for a notarized challenge outcome.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ChallengeOutcomeView {
    /// CID of this outcome (self-addressing, globally unique).
    pub outcome_id: String,
    /// CID of the GateDecisionChallenge closed by this outcome.
    pub challenge_cid: String,
    /// Verdict: "upheld" | "dismissed" | "superseded".
    pub verdict: String,
    /// Comma-separated AgentPubKeys (base64) of reviewers who reached consensus.
    pub reviewer_consensus: String,
    /// Full ConstitutionalReasoning (opaque JSON string).
    pub reasoning_json: String,
    /// ISO 8601 timestamp of when the verdict was decided.
    pub decided_at: String,
    /// Indemnification actions as JSON array (empty array "[]" if no action required).
    pub indemnification_actions_json: String,
    /// ActionHash (base64) of the upstream DHT entry — provenance anchor.
    pub dht_anchor_hash: String,
    /// ISO 8601 timestamp of when this projection row was inserted.
    pub created_at: String,
}

/// Wire view for a notarized governance-action Content entry.
///
/// Source of truth: Holochain DHT (elohim DNA, Content entry,
/// `content_type LIKE 'governance-action:%'`, Category A per p2p-design-gate).
/// Parent entry; vote attestations reference its CID as
/// `parent_governance_action_cid`. DHT is authoritative.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GovernanceActionView {
    /// CID of this governance action (content-derived identity).
    pub id: String,
    /// ActionHash (hex) of the DHT entry — provenance anchor.
    pub dht_anchor_hash: String,
    /// Discriminator matching `governance-action:<subtype>`.
    pub governance_kind: String,
    /// CID of the entity being acted upon.
    pub subject_cid: String,
    /// CID of the proposing agent.
    pub proposer_cid: String,
    /// Serialised threshold JSON (e.g. `{"m":3}` or `{"percentage":0.51}`).
    pub threshold_json: String,
    /// Serialised eligibility predicate JSON, if constrained.
    pub eligibility_predicate_json: Option<String>,
    /// Ballot format: "approve-reject" | "ranked-choice" | "weighted".
    pub ballot_format: String,
    /// ISO 8601 close time for the voting window.
    pub closes_at: String,
    /// Serialised additional parameters JSON, if any.
    pub parameters_json: Option<String>,
    /// Human-readable title.
    pub title: String,
    /// Optional description.
    pub description: Option<String>,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
}

/// Wire view for a locally-derived governance-action vote tally.
///
/// Source of truth: local (operational, Category C per p2p-design-gate).
/// This record is recomputable from `governance_actions JOIN attestations`
/// at any time via `tally_projector::recompute`. It is NOT notarized on the
/// DHT — only the constituent vote attestations are.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GovernanceActionTallyView {
    /// CID of the parent governance-action.
    pub parent_cid: String,
    /// Governance kind discriminator (copied from parent).
    pub governance_kind: String,
    /// CID of the subject being acted upon (copied from parent).
    pub subject_cid: String,
    /// Required count for M-of-N quorum (threshold.m).
    pub threshold_m: i32,
    /// Optional N for M-of-N quorum (threshold.n).
    pub threshold_n: Option<i32>,
    /// Optional percentage for percentage-threshold ballots.
    pub threshold_percentage: Option<f64>,
    /// ISO 8601 close time (copied from parent).
    pub closes_at: String,
    /// Current count of approve votes.
    pub current_approve_count: i32,
    /// Current count of reject votes.
    pub current_reject_count: i32,
    /// Current count of abstain votes.
    pub current_abstain_count: i32,
    /// Computed status: "pending" | "reached-quorum" | "rejected" | "expired" | "tied".
    pub computed_status: String,
    /// ISO 8601 timestamp of the most recent child vote, if any.
    pub last_child_at: Option<String>,
    /// ISO 8601 timestamp when this tally row was last recomputed.
    pub rebuilt_at: String,
}

