//! Qahal-domain Wire→View converters.
//!
//! Converts internal DB models for governance, voting, affinity, and challenge
//! domains to View types defined in `elohim_views::qahal`.

use elohim_views::shared::{parse_json, parse_json_opt};
use elohim_views::{
    AppealView, AttestationView, ChallengeOutcomeView, ChallengeView,
    CollectiveParticipationView, CollectiveView, ContentAttestationView, DiscussionView,
    GateDecisionAttestationView, GateDecisionChallengeView, GovernanceActionTallyView,
    GovernanceActionView, GovernanceDispositionView, GovernanceSignalView, GovernanceStateView,
    PrecedentView, ProposalOptionView, ProposalView, RankedVoteView, StatementView,
    StatementVoteView, VoteView,
};

use crate::db::challenge_outcomes::ChallengeOutcomeRow;
use crate::db::gate_decision_attestations::GateDecisionAttestationRow;
use crate::db::gate_decision_challenges::GateDecisionChallengeRow;
use crate::db::models::{
    Appeal, AttestationRow, Challenge, Collective, CollectiveParticipation, ContentAttestation,
    Discussion, GovernanceActionRow, GovernanceActionTallyRow, GovernanceDisposition,
    GovernanceSignal, GovernanceState, Precedent, Proposal, ProposalOption, RankedVote, Statement,
    StatementVote, Vote,
};

// ============================================================================
// Collective Views (Qahal — Governance Contexts)
// ============================================================================

impl From<Collective> for CollectiveView {
    fn from(c: Collective) -> Self {
        Self {
            id: c.id,
            name: c.name,
            description: c.description,
            governance_layer: c.governance_layer,
            constitutional_parent_id: c.constitutional_parent_id,
            reach: c.reach,
            metadata: parse_json_opt(&c.metadata_json),
            created_by: c.created_by,
            created_at: c.created_at,
            updated_at: c.updated_at,
            dissolved_at: c.dissolved_at,
        }
    }
}

impl From<CollectiveParticipation> for CollectiveParticipationView {
    fn from(p: CollectiveParticipation) -> Self {
        Self {
            id: p.id,
            collective_id: p.collective_id,
            human_id: p.human_id,
            intimacy_level: p.intimacy_level,
            role_context: p.role_context,
            governance_weight: p.governance_weight,
            consent_state: p.consent_state,
            metadata: parse_json_opt(&p.metadata_json),
            joined_at: p.joined_at,
            updated_at: p.updated_at,
            departed_at: p.departed_at,
        }
    }
}

// ============================================================================
// Governance State Views
// ============================================================================

impl From<GovernanceState> for GovernanceStateView {
    fn from(g: GovernanceState) -> Self {
        Self {
            id: g.id,
            entity_type: g.entity_type,
            entity_id: g.entity_id,
            reach: g.reach,
            labels: parse_json(&g.labels),
            voting_state: g.voting_state,
            signal_count: g.signal_count,
            created_at: g.created_at,
            updated_at: g.updated_at,
            dht_anchor_hash: g.dht_anchor_hash,
            status: None,
            status_basis: None,
            active_challenges: None,
            active_proposals: None,
            precedent_ids: None,
            last_updated: None,
            metadata: None,
        }
    }
}

// ============================================================================
// Challenge Views
// ============================================================================

impl From<Challenge> for ChallengeView {
    fn from(c: Challenge) -> Self {
        Self {
            id: c.id,
            entity_type: c.entity_type,
            entity_id: c.entity_id,
            challenger_id: c.challenger_id,
            standing_basis: c.standing_basis,
            grounds_primary: c.grounds_primary,
            grounds_secondary: c.grounds_secondary,
            evidence: parse_json(&c.evidence),
            requested_outcome: c.requested_outcome,
            state: c.state,
            response_outcome: c.response_outcome,
            response_reasoning: c.response_reasoning,
            response_actions: c.response_actions,
            response_by: c.response_by,
            sets_precedent: c.sets_precedent != 0,
            filed_at: c.filed_at,
            acknowledged_at: c.acknowledged_at,
            response_deadline: c.response_deadline,
            responded_at: c.responded_at,
            resolved_at: c.resolved_at,
            created_at: c.created_at,
            sla_status: String::new(),
            dht_anchor_hash: c.dht_anchor_hash,
            challenger_name: None,
            challenger_standing: None,
            grounds: None,
            description: None,
            status: None,
            priority: None,
            sla_deadline: None,
            assigned_elohim: None,
            resolution: None,
            updated_at: None,
            metadata: None,
        }
    }
}

// ============================================================================
// Appeal Views
// ============================================================================

impl From<Appeal> for AppealView {
    fn from(a: Appeal) -> Self {
        Self {
            id: a.id,
            challenge_id: a.challenge_id,
            appellant_id: a.appellant_id,
            grounds: a.grounds,
            additional_evidence: a.additional_evidence,
            state: a.state,
            escalation_level: a.escalation_level,
            decision: a.decision,
            decision_reasoning: a.decision_reasoning,
            decided_by: a.decided_by,
            filed_at: a.filed_at,
            decided_at: a.decided_at,
            created_at: a.created_at,
            dht_anchor_hash: a.dht_anchor_hash,
        }
    }
}

// ============================================================================
// Proposal Views
// ============================================================================

impl From<Proposal> for ProposalView {
    fn from(p: Proposal) -> Self {
        Self {
            id: p.id,
            content_id: p.content_id,
            proposer_presence_id: p.proposer_presence_id,
            proposal_type: p.proposal_type,
            title: p.title,
            body: p.body,
            status: p.status,
            votes_for: p.votes_for,
            votes_against: p.votes_against,
            voting_anonymous: p.voting_anonymous == 1,
            created_at: p.created_at,
            updated_at: p.updated_at,
            voting_mechanism: p.voting_mechanism,
            score_min: p.score_min,
            score_max: p.score_max,
            dots_per_voter: p.dots_per_voter,
            quorum_percentage: p.quorum_percentage,
            passage_threshold: p.passage_threshold,
            dht_anchor_hash: p.dht_anchor_hash,
            description: None,
            proposer_id: None,
            proposer_name: None,
            rationale: None,
            phase: None,
            amendments: None,
            voting_config: None,
            current_votes: None,
            outcome: None,
            related_entity_type: None,
            related_entity_id: None,
            metadata: None,
        }
    }
}

// ============================================================================
// Precedent Views
// ============================================================================

impl From<Precedent> for PrecedentView {
    fn from(p: Precedent) -> Self {
        Self {
            id: p.id,
            content_id: p.content_id,
            principle: p.principle,
            interpretation: p.interpretation,
            established_by: p.established_by,
            created_at: p.created_at,
            dht_anchor_hash: p.dht_anchor_hash,
            title: None,
            summary: None,
            full_reasoning: None,
            binding: None,
            scope: None,
            citations: None,
            status: None,
            established_at: None,
            superseded_by: None,
            updated_at: None,
            metadata: None,
        }
    }
}

// ============================================================================
// Discussion Views
// ============================================================================

impl From<Discussion> for DiscussionView {
    fn from(d: Discussion) -> Self {
        Self {
            id: d.id,
            content_id: d.content_id,
            author_presence_id: d.author_presence_id,
            body: d.body,
            parent_id: d.parent_id,
            created_at: d.created_at,
            updated_at: d.updated_at,
            entity_type: None,
            entity_id: None,
            category: None,
            title: None,
            messages: None,
            status: None,
            message_count: None,
            last_activity_at: None,
            metadata: None,
        }
    }
}

// ============================================================================
// Proposal Option Views (multi-mechanism voting)
// ============================================================================

impl From<ProposalOption> for ProposalOptionView {
    fn from(o: ProposalOption) -> Self {
        Self {
            id: o.id,
            proposal_id: o.proposal_id,
            label: o.label,
            description: o.description,
            position: o.position,
            source: o.source,
            source_justification: o.source_justification,
            created_at: o.created_at,
            dht_anchor_hash: o.dht_anchor_hash,
        }
    }
}

// ============================================================================
// Governance Signal Views
// ============================================================================

impl From<GovernanceSignal> for GovernanceSignalView {
    fn from(s: GovernanceSignal) -> Self {
        Self {
            id: s.id,
            entity_type: s.entity_type,
            entity_id: s.entity_id,
            human_id: s.human_id,
            signal_type: s.signal_type,
            signal_value: s.signal_value,
            mechanism_level: s.mechanism_level,
            proxy_elohim_id: s.proxy_elohim_id,
            created_at: s.created_at,
            dht_anchor_hash: s.dht_anchor_hash,
        }
    }
}

// ============================================================================
// Governance Disposition Views
// ============================================================================

impl From<GovernanceDisposition> for GovernanceDispositionView {
    fn from(d: GovernanceDisposition) -> Self {
        Self {
            id: d.id,
            human_id: d.human_id,
            risk_tolerance: d.risk_tolerance as f64,
            change_openness: d.change_openness as f64,
            consensus_preference: d.consensus_preference as f64,
            priority_values: parse_json(&d.priority_values),
            voting_pattern_summary: parse_json(&d.voting_pattern_summary),
            total_votes_cast: d.total_votes_cast,
            total_challenges_filed: d.total_challenges_filed,
            total_signals_recorded: d.total_signals_recorded,
            dht_anchor_hash: d.dht_anchor_hash,
            last_computed_at: d.last_computed_at,
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}

// ============================================================================
// Attestation Views (content-level attestations)
// ============================================================================

impl From<ContentAttestation> for ContentAttestationView {
    fn from(c: ContentAttestation) -> Self {
        Self {
            id: c.id,
            content_id: c.content_id,
            attestor_presence_id: c.attestor_presence_id,
            scope: c.scope,
            attestation_type: c.attestation_type,
            evidence: parse_json_opt(&c.evidence),
            grantor: parse_json_opt(&c.grantor),
            is_revoked: c.is_revoked == 1,
            revocation: parse_json_opt(&c.revocation),
            created_at: c.created_at,
            updated_at: c.updated_at,
            dht_anchor_hash: c.dht_anchor_hash,
        }
    }
}

// ============================================================================
// Statement + StatementVote Views
// ============================================================================

impl From<Statement> for StatementView {
    fn from(s: Statement) -> Self {
        Self {
            id: s.id,
            entity_type: s.entity_type,
            entity_id: s.entity_id,
            human_id: s.human_id,
            text: s.text,
            agree_count: s.agree_count,
            disagree_count: s.disagree_count,
            pass_count: s.pass_count,
            group_id: s.group_id,
            is_bridging: s.is_bridging != 0,
            created_at: s.created_at,
            dht_anchor_hash: s.dht_anchor_hash,
            context_id: None,
            author_id: None,
            status: None,
            vote_count: None,
            consensus_score: None,
            cluster: None,
            updated_at: None,
            metadata: None,
        }
    }
}

impl From<StatementVote> for StatementVoteView {
    fn from(v: StatementVote) -> Self {
        Self {
            id: v.id,
            statement_id: v.statement_id,
            human_id: v.human_id,
            vote: v.vote,
            created_at: v.created_at,
            dht_anchor_hash: v.dht_anchor_hash,
            voter_id: None,
            metadata: None,
        }
    }
}

// ============================================================================
// Gate Decision Attestation View
// ============================================================================

impl From<GateDecisionAttestationRow> for GateDecisionAttestationView {
    fn from(row: GateDecisionAttestationRow) -> Self {
        Self {
            decision_id: row.decision_id,
            phase: row.phase,
            elohim_id: row.elohim_id,
            elohim_substance_cid: row.elohim_substance_cid,
            gate_name: row.gate_name,
            gate_process_cid: row.gate_process_cid,
            request_ref_json: row.request_ref_json,
            decision: row.decision,
            reasoning_json: row.reasoning_json,
            context_summary_cid: row.context_summary_cid,
            decided_at: row.decided_at,
            universal_band_cid: row.universal_band_cid,
            dht_anchor_hash: row.dht_anchor_hash,
            created_at: row.created_at,
        }
    }
}

// ============================================================================
// Gate Decision Challenge View
// ============================================================================

impl From<GateDecisionChallengeRow> for GateDecisionChallengeView {
    fn from(row: GateDecisionChallengeRow) -> Self {
        Self {
            challenge_id: row.challenge_id,
            challenged_decision_cid: row.challenged_decision_cid,
            challenger_id: row.challenger_id,
            grounds: row.grounds,
            summary: row.summary,
            evidence_refs: row.evidence_refs,
            filed_at: row.filed_at,
            reach: row.reach,
            dht_anchor_hash: row.dht_anchor_hash,
            created_at: row.created_at,
        }
    }
}

// ============================================================================
// Challenge Outcome View
// ============================================================================

impl From<ChallengeOutcomeRow> for ChallengeOutcomeView {
    fn from(row: ChallengeOutcomeRow) -> Self {
        Self {
            outcome_id: row.outcome_id,
            challenge_cid: row.challenge_cid,
            verdict: row.verdict,
            reviewer_consensus: row.reviewer_consensus,
            reasoning_json: row.reasoning_json,
            decided_at: row.decided_at,
            indemnification_actions_json: row.indemnification_actions_json,
            dht_anchor_hash: row.dht_anchor_hash,
            created_at: row.created_at,
        }
    }
}

// ============================================================================
// Governance Action View (Category A — source of truth: Holochain DHT)
// ============================================================================

impl From<GovernanceActionRow> for GovernanceActionView {
    fn from(row: GovernanceActionRow) -> Self {
        Self {
            id: row.id,
            dht_anchor_hash: hex::encode(&row.dht_anchor_hash),
            governance_kind: row.governance_kind,
            subject_cid: row.subject_cid,
            proposer_cid: row.proposer_cid,
            threshold_json: row.threshold_json,
            eligibility_predicate_json: row.eligibility_predicate_json,
            ballot_format: row.ballot_format,
            closes_at: row.closes_at,
            parameters_json: row.parameters_json,
            title: row.title,
            description: row.description,
            created_at: row.created_at,
        }
    }
}

// ============================================================================
// Governance Action Tally View (Category C — local operational derived projection)
// ============================================================================

impl From<GovernanceActionTallyRow> for GovernanceActionTallyView {
    fn from(row: GovernanceActionTallyRow) -> Self {
        Self {
            parent_cid: row.parent_cid,
            governance_kind: row.governance_kind,
            subject_cid: row.subject_cid,
            threshold_m: row.threshold_m,
            threshold_n: row.threshold_n,
            threshold_percentage: row.threshold_percentage,
            closes_at: row.closes_at,
            current_approve_count: row.current_approve_count,
            current_reject_count: row.current_reject_count,
            current_abstain_count: row.current_abstain_count,
            computed_status: row.computed_status,
            last_child_at: row.last_child_at,
            rebuilt_at: row.rebuilt_at,
        }
    }
}

// ============================================================================
// Unified Attestation View (Category A — source of truth: Holochain DHT)
// ============================================================================

impl From<AttestationRow> for AttestationView {
    fn from(row: AttestationRow) -> Self {
        Self {
            id: row.id,
            dht_anchor_hash: hex::encode(&row.dht_anchor_hash),
            attestation_kind: row.attestation_kind,
            subject_cid: row.subject_cid,
            subject_kind: row.subject_kind,
            issuer_cid: row.issuer_cid,
            parent_governance_action_cid: row.parent_governance_action_cid,
            vote_value: row.vote_value,
            vote_weight: row.vote_weight,
            proof_class: row.proof_class,
            proof_evidence_json: row.proof_evidence_json,
            evidence_json: row.evidence_json,
            expires_at: row.expires_at,
            supersedes_cid: row.supersedes_cid,
            revocation_reason: row.revocation_reason,
            revoked_at: row.revoked_at,
            created_at: row.created_at,
            manifest_ref: row.manifest_ref,
            title: row.title,
            description: row.description,
        }
    }
}

// ============================================================================
// Free functions
// ============================================================================

/// Construct a VoteView from a DB Vote model.
pub fn vote_view_from_vote(v: Vote, hide_identity: bool) -> VoteView {
    VoteView {
        id: v.id,
        proposal_id: v.proposal_id,
        human_id: if hide_identity { None } else { Some(v.human_id) },
        position: v.position,
        reason: v.reason,
        created_at: v.created_at,
        updated_at: v.updated_at,
        dht_anchor_hash: v.dht_anchor_hash,
        voter_id: None,
        voter_name: None,
        reasoning: None,
        version: None,
        previous_position: None,
        metadata: None,
    }
}

/// Construct a RankedVoteView from a DB RankedVote model.
pub fn ranked_vote_view_from_ranked_vote(v: RankedVote, hide_identity: bool) -> RankedVoteView {
    RankedVoteView {
        id: v.id,
        proposal_id: v.proposal_id,
        human_id: if hide_identity { None } else { Some(v.human_id) },
        option_id: v.option_id,
        rank: v.rank,
        score: v.score,
        dots: v.dots,
        approved: v.approved.map(|a| a == 1),
        reasoning: v.reasoning,
        proxy_elohim_id: v.proxy_elohim_id,
        created_at: v.created_at,
        dht_anchor_hash: v.dht_anchor_hash,
    }
}
