use hdk::prelude::*;
use mishpat_integrity::*;

// Re-export all wire types from qahal-types crate.
// Integrity entry types (Challenge, Proposal, etc.) stay in mishpat_integrity
// and are converted to wire types at construction sites below.
pub use qahal_types::{
    // Challenge
    ChallengeOutput,
    CreateChallengeInput,
    // Discussion
    CreateDiscussionInput,
    // GateDecisionAttestation
    CreateGateDecisionAttestationInput,
    // GovernanceReaction
    CreateGovernanceReactionInput,
    // GovernanceState
    CreateGovernanceStateInput,
    // GraduatedFeedback
    CreateGraduatedFeedbackInput,
    // OpinionStatement
    CreateOpinionStatementInput,
    // Precedent
    CreatePrecedentInput,
    // Proposal
    CreateProposalInput,
    // ProposalVote
    CreateProposalVoteInput,
    // StatementVote
    CreateStatementVoteInput,
    // Credential verification
    CredentialVerification,
    DiscussionOutput,
    GateDecisionAttestationOutput,
    GetGovernanceStateInput,
    GovernanceReactionOutput,
    GovernanceStateOutput,
    GraduatedFeedbackOutput,
    OpinionStatementOutput,
    PrecedentOutput,
    ProposalOutput,
    ProposalVoteOutput,
    QueryChallengesInput,
    QueryDiscussionsInput,
    QueryGateDecisionsInput,
    QueryGovernanceReactionsInput,
    QueryGovernanceStatesInput,
    QueryGraduatedFeedbackInput,
    QueryOpinionStatementsInput,
    QueryPrecedentsInput,
    QueryProposalVotesInput,
    QueryProposalsInput,
    QueryStatementVotesInput,
    StatementVoteOutput,
    VerificationStatus,
};

// ============================================================
// INTEGRITY → WIRE TYPE CONVERTERS
// ============================================================
// These convert integrity entry types (which use #[hdk_entry_helper])
// to qahal-types wire types (plain serde) at construction sites.

fn wire_challenge(e: &mishpat_integrity::Challenge) -> qahal_types::Challenge {
    qahal_types::Challenge {
        id: e.id.clone(),
        entity_type: e.entity_type.clone(),
        entity_id: e.entity_id.clone(),
        challenger_id: e.challenger_id.clone(),
        challenger_name: e.challenger_name.clone(),
        challenger_standing: e.challenger_standing.clone(),
        grounds: e.grounds.clone(),
        description: e.description.clone(),
        evidence_json: e.evidence_json.clone(),
        status: e.status.clone(),
        filed_at: e.filed_at.clone(),
        acknowledged_at: e.acknowledged_at.clone(),
        sla_deadline: e.sla_deadline.clone(),
        assigned_elohim: e.assigned_elohim.clone(),
        priority: e.priority.clone(),
        resolution_json: e.resolution_json.clone(),
        created_at: e.created_at.clone(),
        updated_at: e.updated_at.clone(),
        metadata_json: e.metadata_json.clone(),
    }
}

fn wire_proposal(e: &mishpat_integrity::Proposal) -> qahal_types::Proposal {
    qahal_types::Proposal {
        id: e.id.clone(),
        title: e.title.clone(),
        proposal_type: e.proposal_type.clone(),
        description: e.description.clone(),
        proposer_id: e.proposer_id.clone(),
        proposer_name: e.proposer_name.clone(),
        rationale: e.rationale.clone(),
        status: e.status.clone(),
        phase: e.phase.clone(),
        amendments_json: e.amendments_json.clone(),
        voting_config_json: e.voting_config_json.clone(),
        current_votes_json: e.current_votes_json.clone(),
        outcome_json: e.outcome_json.clone(),
        related_entity_type: e.related_entity_type.clone(),
        related_entity_id: e.related_entity_id.clone(),
        created_at: e.created_at.clone(),
        updated_at: e.updated_at.clone(),
        metadata_json: e.metadata_json.clone(),
    }
}

fn wire_precedent(e: &mishpat_integrity::Precedent) -> qahal_types::Precedent {
    qahal_types::Precedent {
        id: e.id.clone(),
        title: e.title.clone(),
        summary: e.summary.clone(),
        full_reasoning: e.full_reasoning.clone(),
        binding: e.binding.clone(),
        scope_json: e.scope_json.clone(),
        citations: e.citations,
        status: e.status.clone(),
        established_by: e.established_by.clone(),
        established_at: e.established_at.clone(),
        superseded_by: e.superseded_by.clone(),
        created_at: e.created_at.clone(),
        updated_at: e.updated_at.clone(),
        metadata_json: e.metadata_json.clone(),
    }
}

fn wire_discussion(e: &mishpat_integrity::Discussion) -> qahal_types::Discussion {
    qahal_types::Discussion {
        id: e.id.clone(),
        entity_type: e.entity_type.clone(),
        entity_id: e.entity_id.clone(),
        category: e.category.clone(),
        title: e.title.clone(),
        messages_json: e.messages_json.clone(),
        status: e.status.clone(),
        message_count: e.message_count,
        last_activity_at: e.last_activity_at.clone(),
        created_at: e.created_at.clone(),
        updated_at: e.updated_at.clone(),
        metadata_json: e.metadata_json.clone(),
    }
}

fn wire_governance_state(e: &mishpat_integrity::GovernanceState) -> qahal_types::GovernanceState {
    qahal_types::GovernanceState {
        id: e.id.clone(),
        entity_type: e.entity_type.clone(),
        entity_id: e.entity_id.clone(),
        status: e.status.clone(),
        status_basis_json: e.status_basis_json.clone(),
        labels_json: e.labels_json.clone(),
        active_challenges_json: e.active_challenges_json.clone(),
        active_proposals_json: e.active_proposals_json.clone(),
        precedent_ids_json: e.precedent_ids_json.clone(),
        last_updated: e.last_updated.clone(),
        created_at: e.created_at.clone(),
        updated_at: e.updated_at.clone(),
        metadata_json: e.metadata_json.clone(),
    }
}

fn wire_governance_reaction(
    e: &mishpat_integrity::GovernanceReaction,
) -> qahal_types::GovernanceReaction {
    qahal_types::GovernanceReaction {
        id: e.id.clone(),
        content_id: e.content_id.clone(),
        content_type: e.content_type.clone(),
        reactor_id: e.reactor_id.clone(),
        reaction: e.reaction.clone(),
        intensity: e.intensity,
        mediated: e.mediated,
        mediation_accepted: e.mediation_accepted,
        context_json: e.context_json.clone(),
        created_at: e.created_at.clone(),
        updated_at: e.updated_at.clone(),
        metadata_json: e.metadata_json.clone(),
    }
}

fn wire_graduated_feedback(
    e: &mishpat_integrity::GraduatedFeedback,
) -> qahal_types::GraduatedFeedback {
    qahal_types::GraduatedFeedback {
        id: e.id.clone(),
        content_id: e.content_id.clone(),
        content_type: e.content_type.clone(),
        responder_id: e.responder_id.clone(),
        feedback_context: e.feedback_context.clone(),
        position: e.position,
        intensity: e.intensity,
        reasoning: e.reasoning.clone(),
        updated_count: e.updated_count,
        created_at: e.created_at.clone(),
        updated_at: e.updated_at.clone(),
        metadata_json: e.metadata_json.clone(),
    }
}

fn wire_proposal_vote(e: &mishpat_integrity::ProposalVote) -> qahal_types::ProposalVote {
    qahal_types::ProposalVote {
        id: e.id.clone(),
        proposal_id: e.proposal_id.clone(),
        voter_id: e.voter_id.clone(),
        voter_name: e.voter_name.clone(),
        position: e.position.clone(),
        reasoning: e.reasoning.clone(),
        version: e.version,
        previous_position: e.previous_position.clone(),
        created_at: e.created_at.clone(),
        updated_at: e.updated_at.clone(),
        metadata_json: e.metadata_json.clone(),
    }
}

fn wire_opinion_statement(
    e: &mishpat_integrity::OpinionStatement,
) -> qahal_types::OpinionStatement {
    qahal_types::OpinionStatement {
        id: e.id.clone(),
        context_id: e.context_id.clone(),
        author_id: e.author_id.clone(),
        text: e.text.clone(),
        status: e.status.clone(),
        vote_count: e.vote_count,
        agree_count: e.agree_count,
        disagree_count: e.disagree_count,
        pass_count: e.pass_count,
        consensus_score: e.consensus_score,
        cluster_json: e.cluster_json.clone(),
        created_at: e.created_at.clone(),
        updated_at: e.updated_at.clone(),
        metadata_json: e.metadata_json.clone(),
    }
}

fn wire_statement_vote(e: &mishpat_integrity::StatementVote) -> qahal_types::StatementVote {
    qahal_types::StatementVote {
        id: e.id.clone(),
        statement_id: e.statement_id.clone(),
        voter_id: e.voter_id.clone(),
        vote: e.vote.clone(),
        created_at: e.created_at.clone(),
        metadata_json: e.metadata_json.clone(),
    }
}

fn wire_gate_decision_attestation(
    e: &mishpat_integrity::GateDecisionAttestation,
) -> qahal_types::GateDecisionAttestation {
    qahal_types::GateDecisionAttestation {
        decision_id: e.decision_id.clone(),
        phase: e.phase.clone(),
        elohim_id: e.elohim_id.clone(),
        elohim_substance_cid: e.elohim_substance_cid.clone(),
        gate_name: e.gate_name.clone(),
        gate_process_cid: e.gate_process_cid.clone(),
        request_ref_json: e.request_ref_json.clone(),
        decision: e.decision.clone(),
        reasoning_json: e.reasoning_json.clone(),
        context_summary_cid: e.context_summary_cid.clone(),
        decided_at: e.decided_at.clone(),
        universal_band_cid: e.universal_band_cid.clone(),
    }
}

// ============================================================
// CHALLENGE FUNCTIONS
// ============================================================

/// Create a challenge
#[hdk_extern]
pub fn create_challenge(input: CreateChallengeInput) -> ExternResult<ChallengeOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let challenge_id = input
        .id
        .unwrap_or_else(|| format!("chal-{}-{}", input.entity_id, timestamp));

    let challenge = Challenge {
        id: challenge_id.clone(),
        entity_type: input.entity_type.clone(),
        entity_id: input.entity_id.clone(),
        challenger_id: input.challenger_id.clone(),
        challenger_name: input.challenger_name,
        challenger_standing: input.challenger_standing,
        grounds: input.grounds,
        description: input.description,
        evidence_json: input.evidence_json,
        status: input.status.clone(),
        filed_at: timestamp.clone(),
        acknowledged_at: None,
        sla_deadline: input.sla_deadline,
        assigned_elohim: input.assigned_elohim,
        priority: input.priority,
        resolution_json: input.resolution_json,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        metadata_json: input.metadata_json,
    };

    let action_hash = create_entry(&EntryTypes::Challenge(challenge.clone()))?;

    // Link by ID
    let id_anchor = StringAnchor::new("challenge_id", &challenge_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToChallenge,
        (),
    )?;

    // Link by entity
    let entity_key = format!("{}:{}", input.entity_type, input.entity_id);
    let entity_anchor = StringAnchor::new("challenge_entity", &entity_key);
    let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;
    create_link(
        entity_anchor_hash,
        action_hash.clone(),
        LinkTypes::EntityToChallenge,
        (),
    )?;

    // Link by challenger
    let challenger_anchor = StringAnchor::new("challenge_challenger", &input.challenger_id);
    let challenger_anchor_hash = hash_entry(&EntryTypes::StringAnchor(challenger_anchor))?;
    create_link(
        challenger_anchor_hash,
        action_hash.clone(),
        LinkTypes::ChallengerToChallenge,
        (),
    )?;

    // Link by status
    let status_anchor = StringAnchor::new("challenge_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(
        status_anchor_hash,
        action_hash.clone(),
        LinkTypes::ChallengeByStatus,
        (),
    )?;

    Ok(ChallengeOutput {
        action_hash,
        challenge: wire_challenge(&challenge),
    })
}

/// Get challenge by ID
#[hdk_extern]
pub fn get_challenge_by_id(id: String) -> ExternResult<Option<ChallengeOutput>> {
    let id_anchor = StringAnchor::new("challenge_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToChallenge)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest("Invalid challenge hash".to_string()))
        })?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(challenge) = record.entry().to_app_option::<Challenge>().ok().flatten() {
                return Ok(Some(ChallengeOutput {
                    action_hash,
                    challenge: wire_challenge(&challenge),
                }));
            }
        }
    }

    Ok(None)
}

/// Query challenges
#[hdk_extern]
pub fn query_challenges(input: QueryChallengesInput) -> ExternResult<Vec<ChallengeOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if input.entity_type.is_some() && input.entity_id.is_some() {
        let entity_key = format!(
            "{}:{}",
            input.entity_type.as_ref().unwrap(),
            input.entity_id.as_ref().unwrap()
        );
        let entity_anchor = StringAnchor::new("challenge_entity", &entity_key);
        let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;

        let query = LinkQuery::try_new(entity_anchor_hash, LinkTypes::EntityToChallenge)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest("Invalid challenge hash".to_string()))
            })?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(challenge) = record.entry().to_app_option::<Challenge>().ok().flatten()
                {
                    // Filter by status if specified
                    if let Some(ref status) = input.status {
                        if &challenge.status != status {
                            continue;
                        }
                    }
                    results.push(ChallengeOutput {
                        action_hash,
                        challenge: wire_challenge(&challenge),
                    });
                }
            }
        }
    } else if let Some(status) = &input.status {
        let status_anchor = StringAnchor::new("challenge_status", status);
        let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;

        let query = LinkQuery::try_new(status_anchor_hash, LinkTypes::ChallengeByStatus)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest("Invalid challenge hash".to_string()))
            })?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(challenge) = record.entry().to_app_option::<Challenge>().ok().flatten()
                {
                    results.push(ChallengeOutput {
                        action_hash,
                        challenge: wire_challenge(&challenge),
                    });
                }
            }
        }
    }

    Ok(results)
}

// ============================================================
// PROPOSAL FUNCTIONS
// ============================================================

/// Create a proposal
#[hdk_extern]
pub fn create_proposal(input: CreateProposalInput) -> ExternResult<ProposalOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let proposal_id = input.id.unwrap_or_else(|| format!("prop-{}", timestamp));

    let proposal = Proposal {
        id: proposal_id.clone(),
        title: input.title,
        proposal_type: input.proposal_type.clone(),
        description: input.description,
        proposer_id: input.proposer_id.clone(),
        proposer_name: input.proposer_name,
        rationale: input.rationale,
        status: input.status.clone(),
        phase: input.phase,
        amendments_json: input.amendments_json,
        voting_config_json: input.voting_config_json,
        current_votes_json: input.current_votes_json,
        outcome_json: input.outcome_json,
        related_entity_type: input.related_entity_type,
        related_entity_id: input.related_entity_id,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        metadata_json: input.metadata_json,
    };

    let action_hash = create_entry(&EntryTypes::Proposal(proposal.clone()))?;

    // Link by ID
    let id_anchor = StringAnchor::new("proposal_id", &proposal_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToProposal,
        (),
    )?;

    // Link by type
    let type_anchor = StringAnchor::new("proposal_type", &input.proposal_type);
    let type_anchor_hash = hash_entry(&EntryTypes::StringAnchor(type_anchor))?;
    create_link(
        type_anchor_hash,
        action_hash.clone(),
        LinkTypes::ProposalByType,
        (),
    )?;

    // Link by proposer
    let proposer_anchor = StringAnchor::new("proposal_proposer", &input.proposer_id);
    let proposer_anchor_hash = hash_entry(&EntryTypes::StringAnchor(proposer_anchor))?;
    create_link(
        proposer_anchor_hash,
        action_hash.clone(),
        LinkTypes::ProposerToProposal,
        (),
    )?;

    // Link by status
    let status_anchor = StringAnchor::new("proposal_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(
        status_anchor_hash,
        action_hash.clone(),
        LinkTypes::ProposalByStatus,
        (),
    )?;

    Ok(ProposalOutput {
        action_hash,
        proposal: wire_proposal(&proposal),
    })
}

/// Get proposal by ID
#[hdk_extern]
pub fn get_proposal_by_id(id: String) -> ExternResult<Option<ProposalOutput>> {
    let id_anchor = StringAnchor::new("proposal_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToProposal)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid proposal hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(proposal) = record.entry().to_app_option::<Proposal>().ok().flatten() {
                return Ok(Some(ProposalOutput {
                    action_hash,
                    proposal: wire_proposal(&proposal),
                }));
            }
        }
    }

    Ok(None)
}

/// Query proposals
#[hdk_extern]
pub fn query_proposals(input: QueryProposalsInput) -> ExternResult<Vec<ProposalOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(status) = &input.status {
        let status_anchor = StringAnchor::new("proposal_status", status);
        let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;

        let query = LinkQuery::try_new(status_anchor_hash, LinkTypes::ProposalByStatus)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest("Invalid proposal hash".to_string()))
            })?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(proposal) = record.entry().to_app_option::<Proposal>().ok().flatten() {
                    // Filter by type and proposer if specified
                    if let Some(ref proposal_type) = input.proposal_type {
                        if &proposal.proposal_type != proposal_type {
                            continue;
                        }
                    }
                    if let Some(ref proposer_id) = input.proposer_id {
                        if &proposal.proposer_id != proposer_id {
                            continue;
                        }
                    }
                    results.push(ProposalOutput {
                        action_hash,
                        proposal: wire_proposal(&proposal),
                    });
                }
            }
        }
    } else if let Some(proposal_type) = &input.proposal_type {
        let type_anchor = StringAnchor::new("proposal_type", proposal_type);
        let type_anchor_hash = hash_entry(&EntryTypes::StringAnchor(type_anchor))?;

        let query = LinkQuery::try_new(type_anchor_hash, LinkTypes::ProposalByType)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest("Invalid proposal hash".to_string()))
            })?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(proposal) = record.entry().to_app_option::<Proposal>().ok().flatten() {
                    results.push(ProposalOutput {
                        action_hash,
                        proposal: wire_proposal(&proposal),
                    });
                }
            }
        }
    }

    Ok(results)
}

// ============================================================
// PRECEDENT FUNCTIONS
// ============================================================

/// Create a precedent
#[hdk_extern]
pub fn create_precedent(input: CreatePrecedentInput) -> ExternResult<PrecedentOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let precedent_id = input.id.unwrap_or_else(|| format!("prec-{}", timestamp));

    let precedent = Precedent {
        id: precedent_id.clone(),
        title: input.title,
        summary: input.summary,
        full_reasoning: input.full_reasoning,
        binding: input.binding.clone(),
        scope_json: input.scope_json.clone(),
        citations: 0, // Starts at 0, incremented when cited
        status: input.status.clone(),
        established_by: input.established_by,
        established_at: timestamp.clone(),
        superseded_by: input.superseded_by,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        metadata_json: input.metadata_json,
    };

    let action_hash = create_entry(&EntryTypes::Precedent(precedent.clone()))?;

    // Link by ID
    let id_anchor = StringAnchor::new("precedent_id", &precedent_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToPrecedent,
        (),
    )?;

    // Link by scope (use raw JSON as key — caller controls granularity)
    let scope_anchor = StringAnchor::new("precedent_scope", &input.scope_json);
    let scope_anchor_hash = hash_entry(&EntryTypes::StringAnchor(scope_anchor))?;
    create_link(
        scope_anchor_hash,
        action_hash.clone(),
        LinkTypes::PrecedentByScope,
        (),
    )?;

    // Link by status
    let status_anchor = StringAnchor::new("precedent_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(
        status_anchor_hash,
        action_hash.clone(),
        LinkTypes::PrecedentByStatus,
        (),
    )?;

    Ok(PrecedentOutput {
        action_hash,
        precedent: wire_precedent(&precedent),
    })
}

/// Get precedent by ID
#[hdk_extern]
pub fn get_precedent_by_id(id: String) -> ExternResult<Option<PrecedentOutput>> {
    let id_anchor = StringAnchor::new("precedent_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToPrecedent)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest("Invalid precedent hash".to_string()))
        })?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(precedent) = record.entry().to_app_option::<Precedent>().ok().flatten() {
                return Ok(Some(PrecedentOutput {
                    action_hash,
                    precedent: wire_precedent(&precedent),
                }));
            }
        }
    }

    Ok(None)
}

/// Query precedents
#[hdk_extern]
pub fn query_precedents(input: QueryPrecedentsInput) -> ExternResult<Vec<PrecedentOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(status) = &input.status {
        let status_anchor = StringAnchor::new("precedent_status", status);
        let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;

        let query = LinkQuery::try_new(status_anchor_hash, LinkTypes::PrecedentByStatus)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest("Invalid precedent hash".to_string()))
            })?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(precedent) = record.entry().to_app_option::<Precedent>().ok().flatten()
                {
                    // Filter by binding if specified
                    if let Some(ref binding) = input.binding {
                        if &precedent.binding != binding {
                            continue;
                        }
                    }
                    results.push(PrecedentOutput {
                        action_hash,
                        precedent: wire_precedent(&precedent),
                    });
                }
            }
        }
    }

    Ok(results)
}

// ============================================================
// DISCUSSION FUNCTIONS
// ============================================================

/// Create a discussion
#[hdk_extern]
pub fn create_discussion(input: CreateDiscussionInput) -> ExternResult<DiscussionOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let discussion_id = input
        .id
        .unwrap_or_else(|| format!("disc-{}-{}", input.entity_id, timestamp));

    let discussion = Discussion {
        id: discussion_id.clone(),
        entity_type: input.entity_type.clone(),
        entity_id: input.entity_id.clone(),
        category: input.category.clone(),
        title: input.title,
        messages_json: input.messages_json,
        status: input.status.clone(),
        message_count: 0,
        last_activity_at: timestamp.clone(),
        created_at: timestamp.clone(),
        updated_at: timestamp,
        metadata_json: input.metadata_json,
    };

    let action_hash = create_entry(&EntryTypes::Discussion(discussion.clone()))?;

    // Link by ID
    let id_anchor = StringAnchor::new("discussion_id", &discussion_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToDiscussion,
        (),
    )?;

    // Link by entity
    let entity_key = format!("{}:{}", input.entity_type, input.entity_id);
    let entity_anchor = StringAnchor::new("discussion_entity", &entity_key);
    let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;
    create_link(
        entity_anchor_hash,
        action_hash.clone(),
        LinkTypes::EntityToDiscussion,
        (),
    )?;

    // Link by category
    let category_anchor = StringAnchor::new("discussion_category", &input.category);
    let category_anchor_hash = hash_entry(&EntryTypes::StringAnchor(category_anchor))?;
    create_link(
        category_anchor_hash,
        action_hash.clone(),
        LinkTypes::DiscussionByCategory,
        (),
    )?;

    // Link by status
    let status_anchor = StringAnchor::new("discussion_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(
        status_anchor_hash,
        action_hash.clone(),
        LinkTypes::DiscussionByStatus,
        (),
    )?;

    Ok(DiscussionOutput {
        action_hash,
        discussion: wire_discussion(&discussion),
    })
}

/// Get discussion by ID
#[hdk_extern]
pub fn get_discussion_by_id(id: String) -> ExternResult<Option<DiscussionOutput>> {
    let id_anchor = StringAnchor::new("discussion_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToDiscussion)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest("Invalid discussion hash".to_string()))
        })?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(discussion) = record.entry().to_app_option::<Discussion>().ok().flatten() {
                return Ok(Some(DiscussionOutput {
                    action_hash,
                    discussion: wire_discussion(&discussion),
                }));
            }
        }
    }

    Ok(None)
}

/// Query discussions
#[hdk_extern]
pub fn query_discussions(input: QueryDiscussionsInput) -> ExternResult<Vec<DiscussionOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if input.entity_type.is_some() && input.entity_id.is_some() {
        let entity_key = format!(
            "{}:{}",
            input.entity_type.as_ref().unwrap(),
            input.entity_id.as_ref().unwrap()
        );
        let entity_anchor = StringAnchor::new("discussion_entity", &entity_key);
        let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;

        let query = LinkQuery::try_new(entity_anchor_hash, LinkTypes::EntityToDiscussion)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest("Invalid discussion hash".to_string()))
            })?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(discussion) =
                    record.entry().to_app_option::<Discussion>().ok().flatten()
                {
                    // Filter by category/status if specified
                    if let Some(ref category) = input.category {
                        if &discussion.category != category {
                            continue;
                        }
                    }
                    if let Some(ref status) = input.status {
                        if &discussion.status != status {
                            continue;
                        }
                    }
                    results.push(DiscussionOutput {
                        action_hash,
                        discussion: wire_discussion(&discussion),
                    });
                }
            }
        }
    } else if let Some(category) = &input.category {
        let category_anchor = StringAnchor::new("discussion_category", category);
        let category_anchor_hash = hash_entry(&EntryTypes::StringAnchor(category_anchor))?;

        let query = LinkQuery::try_new(category_anchor_hash, LinkTypes::DiscussionByCategory)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest("Invalid discussion hash".to_string()))
            })?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(discussion) =
                    record.entry().to_app_option::<Discussion>().ok().flatten()
                {
                    results.push(DiscussionOutput {
                        action_hash,
                        discussion: wire_discussion(&discussion),
                    });
                }
            }
        }
    }

    Ok(results)
}

// ============================================================
// GOVERNANCE STATE FUNCTIONS
// ============================================================

/// Create or update governance state for an entity
#[hdk_extern]
pub fn set_governance_state(
    input: CreateGovernanceStateInput,
) -> ExternResult<GovernanceStateOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let governance_state_id = input
        .id
        .unwrap_or_else(|| format!("gs-{}:{}", input.entity_type, input.entity_id));

    let governance_state = GovernanceState {
        id: governance_state_id,
        entity_type: input.entity_type.clone(),
        entity_id: input.entity_id.clone(),
        status: input.status.clone(),
        status_basis_json: input.status_basis_json,
        labels_json: input.labels_json,
        active_challenges_json: input.active_challenges_json,
        active_proposals_json: input.active_proposals_json,
        precedent_ids_json: input.precedent_ids_json,
        last_updated: timestamp.clone(),
        created_at: timestamp.clone(),
        updated_at: timestamp,
        metadata_json: input.metadata_json,
    };

    let action_hash = create_entry(&EntryTypes::GovernanceState(governance_state.clone()))?;

    // Link by entity (primary lookup)
    let entity_key = format!("{}:{}", input.entity_type, input.entity_id);
    let entity_anchor = StringAnchor::new("governance_state_entity", &entity_key);
    let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;
    create_link(
        entity_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToGovernanceState,
        (),
    )?;

    // Link by status
    let status_anchor = StringAnchor::new("governance_state_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(
        status_anchor_hash,
        action_hash.clone(),
        LinkTypes::GovernanceStateByStatus,
        (),
    )?;

    Ok(GovernanceStateOutput {
        action_hash,
        governance_state: wire_governance_state(&governance_state),
    })
}

/// Get governance state for an entity
#[hdk_extern]
pub fn get_governance_state(
    input: GetGovernanceStateInput,
) -> ExternResult<Option<GovernanceStateOutput>> {
    let entity_key = format!("{}:{}", input.entity_type, input.entity_id);
    let entity_anchor = StringAnchor::new("governance_state_entity", &entity_key);
    let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;

    let query = LinkQuery::try_new(entity_anchor_hash, LinkTypes::IdToGovernanceState)?;
    let links = get_links(query, GetStrategy::default())?;

    // Return the most recent governance state
    if let Some(link) = links.last() {
        let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest(
                "Invalid governance state hash".to_string()
            ))
        })?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(governance_state) = record
                .entry()
                .to_app_option::<GovernanceState>()
                .ok()
                .flatten()
            {
                return Ok(Some(GovernanceStateOutput {
                    action_hash,
                    governance_state: wire_governance_state(&governance_state),
                }));
            }
        }
    }

    Ok(None)
}

/// Query governance states by status
#[hdk_extern]
pub fn query_governance_states(
    input: QueryGovernanceStatesInput,
) -> ExternResult<Vec<GovernanceStateOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(status) = &input.status {
        let status_anchor = StringAnchor::new("governance_state_status", status);
        let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;

        let query = LinkQuery::try_new(status_anchor_hash, LinkTypes::GovernanceStateByStatus)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest(
                    "Invalid governance state hash".to_string()
                ))
            })?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(governance_state) = record
                    .entry()
                    .to_app_option::<GovernanceState>()
                    .ok()
                    .flatten()
                {
                    results.push(GovernanceStateOutput {
                        action_hash,
                        governance_state: wire_governance_state(&governance_state),
                    });
                }
            }
        }
    }

    Ok(results)
}

// ============================================================
// GOVERNANCE REACTION FUNCTIONS
// ============================================================

/// Record a low-friction emotional reaction to content
#[hdk_extern]
pub fn create_governance_reaction(
    input: CreateGovernanceReactionInput,
) -> ExternResult<GovernanceReactionOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let reaction_id = input.id.unwrap_or_else(|| {
        format!(
            "rxn-{}-{}-{}",
            input.content_id, input.reactor_id, timestamp
        )
    });

    let reaction = GovernanceReaction {
        id: reaction_id.clone(),
        content_id: input.content_id.clone(),
        content_type: input.content_type.clone(),
        reactor_id: input.reactor_id.clone(),
        reaction: input.reaction.clone(),
        intensity: input.intensity,
        mediated: input.mediated,
        mediation_accepted: input.mediation_accepted,
        context_json: input.context_json,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        metadata_json: input.metadata_json,
    };

    let action_hash = create_entry(&EntryTypes::GovernanceReaction(reaction.clone()))?;

    // Link by content
    let content_anchor = StringAnchor::new("reaction_content", &input.content_id);
    let content_anchor_hash = hash_entry(&EntryTypes::StringAnchor(content_anchor))?;
    create_link(
        content_anchor_hash,
        action_hash.clone(),
        LinkTypes::ContentToReactions,
        (),
    )?;

    // Link by agent
    let agent_anchor = StringAnchor::new("reaction_agent", &input.reactor_id);
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;
    create_link(
        agent_anchor_hash,
        action_hash.clone(),
        LinkTypes::AgentToReactions,
        (),
    )?;

    // Link by reaction type
    let type_anchor = StringAnchor::new("reaction_type", &input.reaction);
    let type_anchor_hash = hash_entry(&EntryTypes::StringAnchor(type_anchor))?;
    create_link(
        type_anchor_hash,
        action_hash.clone(),
        LinkTypes::ReactionByType,
        (),
    )?;

    Ok(GovernanceReactionOutput {
        action_hash,
        reaction: wire_governance_reaction(&reaction),
    })
}

/// Query governance reactions
#[hdk_extern]
pub fn query_governance_reactions(
    input: QueryGovernanceReactionsInput,
) -> ExternResult<Vec<GovernanceReactionOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(content_id) = &input.content_id {
        let content_anchor = StringAnchor::new("reaction_content", content_id);
        let content_anchor_hash = hash_entry(&EntryTypes::StringAnchor(content_anchor))?;

        let query = LinkQuery::try_new(content_anchor_hash, LinkTypes::ContentToReactions)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest("Invalid reaction hash".to_string()))
            })?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(reaction) = record
                    .entry()
                    .to_app_option::<GovernanceReaction>()
                    .ok()
                    .flatten()
                {
                    if let Some(ref reaction_type) = input.reaction_type {
                        if &reaction.reaction != reaction_type {
                            continue;
                        }
                    }
                    results.push(GovernanceReactionOutput {
                        action_hash,
                        reaction: wire_governance_reaction(&reaction),
                    });
                }
            }
        }
    } else if let Some(reactor_id) = &input.reactor_id {
        let agent_anchor = StringAnchor::new("reaction_agent", reactor_id);
        let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;

        let query = LinkQuery::try_new(agent_anchor_hash, LinkTypes::AgentToReactions)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest("Invalid reaction hash".to_string()))
            })?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(reaction) = record
                    .entry()
                    .to_app_option::<GovernanceReaction>()
                    .ok()
                    .flatten()
                {
                    results.push(GovernanceReactionOutput {
                        action_hash,
                        reaction: wire_governance_reaction(&reaction),
                    });
                }
            }
        }
    }

    Ok(results)
}

// ============================================================
// GRADUATED FEEDBACK FUNCTIONS
// ============================================================

/// Record medium-friction scaled feedback (Loomio/Forby style)
#[hdk_extern]
pub fn create_graduated_feedback(
    input: CreateGraduatedFeedbackInput,
) -> ExternResult<GraduatedFeedbackOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let feedback_id = input.id.unwrap_or_else(|| {
        format!(
            "fb-{}-{}-{}",
            input.content_id, input.responder_id, input.feedback_context
        )
    });

    let feedback = GraduatedFeedback {
        id: feedback_id.clone(),
        content_id: input.content_id.clone(),
        content_type: input.content_type.clone(),
        responder_id: input.responder_id.clone(),
        feedback_context: input.feedback_context.clone(),
        position: input.position,
        intensity: input.intensity,
        reasoning: input.reasoning,
        updated_count: 0,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        metadata_json: input.metadata_json,
    };

    let action_hash = create_entry(&EntryTypes::GraduatedFeedback(feedback.clone()))?;

    // Link by content
    let content_anchor = StringAnchor::new("feedback_content", &input.content_id);
    let content_anchor_hash = hash_entry(&EntryTypes::StringAnchor(content_anchor))?;
    create_link(
        content_anchor_hash,
        action_hash.clone(),
        LinkTypes::ContentToFeedback,
        (),
    )?;

    // Link by agent
    let agent_anchor = StringAnchor::new("feedback_agent", &input.responder_id);
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;
    create_link(
        agent_anchor_hash,
        action_hash.clone(),
        LinkTypes::AgentToFeedback,
        (),
    )?;

    // Link by context
    let context_anchor = StringAnchor::new("feedback_context", &input.feedback_context);
    let context_anchor_hash = hash_entry(&EntryTypes::StringAnchor(context_anchor))?;
    create_link(
        context_anchor_hash,
        action_hash.clone(),
        LinkTypes::FeedbackByContext,
        (),
    )?;

    Ok(GraduatedFeedbackOutput {
        action_hash,
        feedback: wire_graduated_feedback(&feedback),
    })
}

/// Query graduated feedback
#[hdk_extern]
pub fn query_graduated_feedback(
    input: QueryGraduatedFeedbackInput,
) -> ExternResult<Vec<GraduatedFeedbackOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(content_id) = &input.content_id {
        let content_anchor = StringAnchor::new("feedback_content", content_id);
        let content_anchor_hash = hash_entry(&EntryTypes::StringAnchor(content_anchor))?;

        let query = LinkQuery::try_new(content_anchor_hash, LinkTypes::ContentToFeedback)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest("Invalid feedback hash".to_string()))
            })?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(feedback) = record
                    .entry()
                    .to_app_option::<GraduatedFeedback>()
                    .ok()
                    .flatten()
                {
                    if let Some(ref ctx) = input.feedback_context {
                        if &feedback.feedback_context != ctx {
                            continue;
                        }
                    }
                    results.push(GraduatedFeedbackOutput {
                        action_hash,
                        feedback: wire_graduated_feedback(&feedback),
                    });
                }
            }
        }
    } else if let Some(responder_id) = &input.responder_id {
        let agent_anchor = StringAnchor::new("feedback_agent", responder_id);
        let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;

        let query = LinkQuery::try_new(agent_anchor_hash, LinkTypes::AgentToFeedback)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest("Invalid feedback hash".to_string()))
            })?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(feedback) = record
                    .entry()
                    .to_app_option::<GraduatedFeedback>()
                    .ok()
                    .flatten()
                {
                    results.push(GraduatedFeedbackOutput {
                        action_hash,
                        feedback: wire_graduated_feedback(&feedback),
                    });
                }
            }
        }
    }

    Ok(results)
}

// ============================================================
// PROPOSAL VOTE FUNCTIONS
// ============================================================

/// Cast a Loomio-style 4-position vote on a proposal
#[hdk_extern]
pub fn create_proposal_vote(input: CreateProposalVoteInput) -> ExternResult<ProposalVoteOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let vote_id = input
        .id
        .unwrap_or_else(|| format!("pv-{}-{}", input.proposal_id, input.voter_id));

    let vote = ProposalVote {
        id: vote_id.clone(),
        proposal_id: input.proposal_id.clone(),
        voter_id: input.voter_id.clone(),
        voter_name: input.voter_name,
        position: input.position.clone(),
        reasoning: input.reasoning,
        version: input.version,
        previous_position: input.previous_position,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        metadata_json: input.metadata_json,
    };

    let action_hash = create_entry(&EntryTypes::ProposalVote(vote.clone()))?;

    // Link from proposal
    let proposal_anchor = StringAnchor::new("proposal_vote_proposal", &input.proposal_id);
    let proposal_anchor_hash = hash_entry(&EntryTypes::StringAnchor(proposal_anchor))?;
    create_link(
        proposal_anchor_hash,
        action_hash.clone(),
        LinkTypes::ProposalToVotes,
        (),
    )?;

    // Link by agent
    let agent_anchor = StringAnchor::new("proposal_vote_agent", &input.voter_id);
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;
    create_link(
        agent_anchor_hash,
        action_hash.clone(),
        LinkTypes::AgentToVotes,
        (),
    )?;

    // Link by position
    let position_anchor = StringAnchor::new("proposal_vote_position", &input.position);
    let position_anchor_hash = hash_entry(&EntryTypes::StringAnchor(position_anchor))?;
    create_link(
        position_anchor_hash,
        action_hash.clone(),
        LinkTypes::VoteByPosition,
        (),
    )?;

    Ok(ProposalVoteOutput {
        action_hash,
        vote: wire_proposal_vote(&vote),
    })
}

/// Query proposal votes
#[hdk_extern]
pub fn query_proposal_votes(
    input: QueryProposalVotesInput,
) -> ExternResult<Vec<ProposalVoteOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(proposal_id) = &input.proposal_id {
        let proposal_anchor = StringAnchor::new("proposal_vote_proposal", proposal_id);
        let proposal_anchor_hash = hash_entry(&EntryTypes::StringAnchor(proposal_anchor))?;

        let query = LinkQuery::try_new(proposal_anchor_hash, LinkTypes::ProposalToVotes)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid vote hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(vote) = record
                    .entry()
                    .to_app_option::<ProposalVote>()
                    .ok()
                    .flatten()
                {
                    if let Some(ref position) = input.position {
                        if &vote.position != position {
                            continue;
                        }
                    }
                    results.push(ProposalVoteOutput {
                        action_hash,
                        vote: wire_proposal_vote(&vote),
                    });
                }
            }
        }
    } else if let Some(voter_id) = &input.voter_id {
        let agent_anchor = StringAnchor::new("proposal_vote_agent", voter_id);
        let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;

        let query = LinkQuery::try_new(agent_anchor_hash, LinkTypes::AgentToVotes)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid vote hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(vote) = record
                    .entry()
                    .to_app_option::<ProposalVote>()
                    .ok()
                    .flatten()
                {
                    results.push(ProposalVoteOutput {
                        action_hash,
                        vote: wire_proposal_vote(&vote),
                    });
                }
            }
        }
    }

    Ok(results)
}

// ============================================================
// OPINION STATEMENT FUNCTIONS (Polis-style)
// ============================================================

/// Create a Polis-style opinion statement for clustering
#[hdk_extern]
pub fn create_opinion_statement(
    input: CreateOpinionStatementInput,
) -> ExternResult<OpinionStatementOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let statement_id = input
        .id
        .unwrap_or_else(|| format!("stmt-{}-{}", input.context_id, timestamp));

    let statement = OpinionStatement {
        id: statement_id.clone(),
        context_id: input.context_id.clone(),
        author_id: input.author_id.clone(),
        text: input.text,
        status: input.status,
        vote_count: 0,
        agree_count: 0,
        disagree_count: 0,
        pass_count: 0,
        consensus_score: 0,
        cluster_json: None,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        metadata_json: input.metadata_json,
    };

    let action_hash = create_entry(&EntryTypes::OpinionStatement(statement.clone()))?;

    // Link by context
    let context_anchor = StringAnchor::new("opinion_context", &input.context_id);
    let context_anchor_hash = hash_entry(&EntryTypes::StringAnchor(context_anchor))?;
    create_link(
        context_anchor_hash,
        action_hash.clone(),
        LinkTypes::ContextToStatements,
        (),
    )?;

    // Link by agent
    let agent_anchor = StringAnchor::new("opinion_agent", &input.author_id);
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;
    create_link(
        agent_anchor_hash,
        action_hash.clone(),
        LinkTypes::AgentToStatements,
        (),
    )?;

    Ok(OpinionStatementOutput {
        action_hash,
        statement: wire_opinion_statement(&statement),
    })
}

/// Get opinion statement by action hash
#[hdk_extern]
pub fn get_opinion_statement(
    action_hash: ActionHash,
) -> ExternResult<Option<OpinionStatementOutput>> {
    let record = get(action_hash.clone(), GetOptions::default())?;
    if let Some(record) = record {
        if let Some(statement) = record
            .entry()
            .to_app_option::<OpinionStatement>()
            .ok()
            .flatten()
        {
            return Ok(Some(OpinionStatementOutput {
                action_hash,
                statement: wire_opinion_statement(&statement),
            }));
        }
    }
    Ok(None)
}

/// Query opinion statements for a deliberation context
#[hdk_extern]
pub fn query_opinion_statements(
    input: QueryOpinionStatementsInput,
) -> ExternResult<Vec<OpinionStatementOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(context_id) = &input.context_id {
        let context_anchor = StringAnchor::new("opinion_context", context_id);
        let context_anchor_hash = hash_entry(&EntryTypes::StringAnchor(context_anchor))?;

        let query = LinkQuery::try_new(context_anchor_hash, LinkTypes::ContextToStatements)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest("Invalid statement hash".to_string()))
            })?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(statement) = record
                    .entry()
                    .to_app_option::<OpinionStatement>()
                    .ok()
                    .flatten()
                {
                    results.push(OpinionStatementOutput {
                        action_hash,
                        statement: wire_opinion_statement(&statement),
                    });
                }
            }
        }
    } else if let Some(author_id) = &input.author_id {
        let agent_anchor = StringAnchor::new("opinion_agent", author_id);
        let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;

        let query = LinkQuery::try_new(agent_anchor_hash, LinkTypes::AgentToStatements)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest("Invalid statement hash".to_string()))
            })?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(statement) = record
                    .entry()
                    .to_app_option::<OpinionStatement>()
                    .ok()
                    .flatten()
                {
                    results.push(OpinionStatementOutput {
                        action_hash,
                        statement: wire_opinion_statement(&statement),
                    });
                }
            }
        }
    }

    Ok(results)
}

// ============================================================
// STATEMENT VOTE FUNCTIONS
// ============================================================

/// Cast a vote on an opinion statement (agree/disagree/pass)
#[hdk_extern]
pub fn create_statement_vote(input: CreateStatementVoteInput) -> ExternResult<StatementVoteOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let vote_id = input
        .id
        .unwrap_or_else(|| format!("sv-{}-{}", input.statement_id, input.voter_id));

    let statement_vote = StatementVote {
        id: vote_id.clone(),
        statement_id: input.statement_id.clone(),
        voter_id: input.voter_id.clone(),
        vote: input.vote,
        created_at: timestamp,
        metadata_json: input.metadata_json,
    };

    let action_hash = create_entry(&EntryTypes::StatementVote(statement_vote.clone()))?;

    // Link from statement
    let statement_anchor = StringAnchor::new("statement_vote_statement", &input.statement_id);
    let statement_anchor_hash = hash_entry(&EntryTypes::StringAnchor(statement_anchor))?;
    create_link(
        statement_anchor_hash,
        action_hash.clone(),
        LinkTypes::StatementToVotes,
        (),
    )?;

    // Link by agent
    let agent_anchor = StringAnchor::new("statement_vote_agent", &input.voter_id);
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;
    create_link(
        agent_anchor_hash,
        action_hash.clone(),
        LinkTypes::AgentToStatementVotes,
        (),
    )?;

    Ok(StatementVoteOutput {
        action_hash,
        statement_vote: wire_statement_vote(&statement_vote),
    })
}

/// Query statement votes
#[hdk_extern]
pub fn query_statement_votes(
    input: QueryStatementVotesInput,
) -> ExternResult<Vec<StatementVoteOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(statement_id) = &input.statement_id {
        let statement_anchor = StringAnchor::new("statement_vote_statement", statement_id);
        let statement_anchor_hash = hash_entry(&EntryTypes::StringAnchor(statement_anchor))?;

        let query = LinkQuery::try_new(statement_anchor_hash, LinkTypes::StatementToVotes)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest(
                    "Invalid statement vote hash".to_string()
                ))
            })?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(statement_vote) = record
                    .entry()
                    .to_app_option::<StatementVote>()
                    .ok()
                    .flatten()
                {
                    results.push(StatementVoteOutput {
                        action_hash,
                        statement_vote: wire_statement_vote(&statement_vote),
                    });
                }
            }
        }
    } else if let Some(voter_id) = &input.voter_id {
        let agent_anchor = StringAnchor::new("statement_vote_agent", voter_id);
        let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;

        let query = LinkQuery::try_new(agent_anchor_hash, LinkTypes::AgentToStatementVotes)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest(
                    "Invalid statement vote hash".to_string()
                ))
            })?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(statement_vote) = record
                    .entry()
                    .to_app_option::<StatementVote>()
                    .ok()
                    .flatten()
                {
                    results.push(StatementVoteOutput {
                        action_hash,
                        statement_vote: wire_statement_vote(&statement_vote),
                    });
                }
            }
        }
    }

    Ok(results)
}

// ============================================================
// GATE DECISION ATTESTATION FUNCTIONS
// ============================================================
//
// GateDecisionAttestation is a notarized record of every gate evaluation.
// Indexed by: decision_id, elohim_id, gate_name, phase.
// elohim-storage (Task 4.2) receives the post-commit signal and projects
// into SQLite with dht_anchor_hash for fast query.

/// Signal emitted after a GateDecisionAttestation is committed to the DHT.
///
/// elohim-storage listens for this signal and projects the entry into its
/// local SQLite index with dht_anchor_hash for downstream query.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum MishpatSignal {
    GateDecisionCreated {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        entry: mishpat_integrity::GateDecisionAttestation,
        author: AgentPubKey,
    },
}

/// Create and notarize a GateDecisionAttestation on the Mishpat DHT.
///
/// Indexes via four StringAnchor links so storage can query by
/// decision_id, elohim_id, gate_name, or phase without a full DHT scan.
///
/// Returns the ActionHash of the committed record. The caller receives the
/// entry hash via the post-commit signal (MishpatSignal::GateDecisionCreated).
#[hdk_extern]
pub fn create_gate_decision_attestation(
    input: CreateGateDecisionAttestationInput,
) -> ExternResult<ActionHash> {
    let entry = mishpat_integrity::GateDecisionAttestation {
        decision_id: input.decision_id.clone(),
        phase: input.phase.clone(),
        elohim_id: input.elohim_id.clone(),
        elohim_substance_cid: input.elohim_substance_cid.clone(),
        gate_name: input.gate_name.clone(),
        gate_process_cid: input.gate_process_cid.clone(),
        request_ref_json: input.request_ref_json.clone(),
        decision: input.decision.clone(),
        reasoning_json: input.reasoning_json.clone(),
        context_summary_cid: input.context_summary_cid.clone(),
        decided_at: input.decided_at.clone(),
        universal_band_cid: input.universal_band_cid.clone(),
    };

    let action_hash = create_entry(&EntryTypes::GateDecisionAttestation(entry.clone()))?;

    // Index by decision_id (primary lookup — one attestation per CID)
    let id_anchor = StringAnchor::new("gate_decision_id", &input.decision_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToGateDecision,
        (),
    )?;

    // Index by elohim_id (all decisions by a given elohim agent)
    let elohim_anchor = StringAnchor::new("gate_decision_elohim", &input.elohim_id);
    let elohim_anchor_hash = hash_entry(&EntryTypes::StringAnchor(elohim_anchor))?;
    create_link(
        elohim_anchor_hash,
        action_hash.clone(),
        LinkTypes::ElohimToGateDecisions,
        (),
    )?;

    // Index by gate_name (all decisions produced by a given gate)
    let gate_anchor = StringAnchor::new("gate_decision_gate_name", &input.gate_name);
    let gate_anchor_hash = hash_entry(&EntryTypes::StringAnchor(gate_anchor))?;
    create_link(
        gate_anchor_hash,
        action_hash.clone(),
        LinkTypes::GateNameToDecisions,
        (),
    )?;

    // Index by phase (dev-context vs elohim-active audit split)
    let phase_anchor = StringAnchor::new("gate_decision_phase", &input.phase);
    let phase_anchor_hash = hash_entry(&EntryTypes::StringAnchor(phase_anchor))?;
    create_link(
        phase_anchor_hash,
        action_hash.clone(),
        LinkTypes::PhaseToDecisions,
        (),
    )?;

    Ok(action_hash)
}

/// Get a GateDecisionAttestation by its decision_id CID.
#[hdk_extern]
pub fn get_gate_decision_by_id(
    decision_id: String,
) -> ExternResult<Option<GateDecisionAttestationOutput>> {
    let id_anchor = StringAnchor::new("gate_decision_id", &decision_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToGateDecision)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest(
                "Invalid gate decision hash".to_string()
            ))
        })?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(entry) = record
                .entry()
                .to_app_option::<mishpat_integrity::GateDecisionAttestation>()
                .ok()
                .flatten()
            {
                return Ok(Some(GateDecisionAttestationOutput {
                    action_hash,
                    attestation: wire_gate_decision_attestation(&entry),
                }));
            }
        }
    }

    Ok(None)
}

/// Query all GateDecisionAttestations by elohim_id.
///
/// Returns all gate decisions made by the given elohim agent, up to limit.
#[hdk_extern]
pub fn get_decisions_by_elohim(
    input: QueryGateDecisionsInput,
) -> ExternResult<Vec<GateDecisionAttestationOutput>> {
    let elohim_id = input.elohim_id.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "elohim_id is required for get_decisions_by_elohim".to_string()
        ))
    })?;
    let limit = input.limit.unwrap_or(100) as usize;

    let elohim_anchor = StringAnchor::new("gate_decision_elohim", &elohim_id);
    let elohim_anchor_hash = hash_entry(&EntryTypes::StringAnchor(elohim_anchor))?;

    let query = LinkQuery::try_new(elohim_anchor_hash, LinkTypes::ElohimToGateDecisions)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links.iter().take(limit) {
        let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest(
                "Invalid gate decision hash".to_string()
            ))
        })?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(entry) = record
                .entry()
                .to_app_option::<mishpat_integrity::GateDecisionAttestation>()
                .ok()
                .flatten()
            {
                results.push(GateDecisionAttestationOutput {
                    action_hash,
                    attestation: wire_gate_decision_attestation(&entry),
                });
            }
        }
    }

    Ok(results)
}

/// Query all GateDecisionAttestations by gate_name.
///
/// Returns all gate decisions produced by the named gate, up to limit.
#[hdk_extern]
pub fn get_decisions_by_gate(
    input: QueryGateDecisionsInput,
) -> ExternResult<Vec<GateDecisionAttestationOutput>> {
    let gate_name = input.gate_name.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "gate_name is required for get_decisions_by_gate".to_string()
        ))
    })?;
    let limit = input.limit.unwrap_or(100) as usize;

    let gate_anchor = StringAnchor::new("gate_decision_gate_name", &gate_name);
    let gate_anchor_hash = hash_entry(&EntryTypes::StringAnchor(gate_anchor))?;

    let query = LinkQuery::try_new(gate_anchor_hash, LinkTypes::GateNameToDecisions)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links.iter().take(limit) {
        let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest(
                "Invalid gate decision hash".to_string()
            ))
        })?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(entry) = record
                .entry()
                .to_app_option::<mishpat_integrity::GateDecisionAttestation>()
                .ok()
                .flatten()
            {
                results.push(GateDecisionAttestationOutput {
                    action_hash,
                    attestation: wire_gate_decision_attestation(&entry),
                });
            }
        }
    }

    Ok(results)
}

/// Query all GateDecisionAttestations by phase.
///
/// Returns all gate decisions for the given phase discriminator, up to limit.
#[hdk_extern]
pub fn get_decisions_by_phase(
    input: QueryGateDecisionsInput,
) -> ExternResult<Vec<GateDecisionAttestationOutput>> {
    let phase = input.phase.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "phase is required for get_decisions_by_phase".to_string()
        ))
    })?;
    let limit = input.limit.unwrap_or(100) as usize;

    let phase_anchor = StringAnchor::new("gate_decision_phase", &phase);
    let phase_anchor_hash = hash_entry(&EntryTypes::StringAnchor(phase_anchor))?;

    let query = LinkQuery::try_new(phase_anchor_hash, LinkTypes::PhaseToDecisions)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links.iter().take(limit) {
        let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest(
                "Invalid gate decision hash".to_string()
            ))
        })?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(entry) = record
                .entry()
                .to_app_option::<mishpat_integrity::GateDecisionAttestation>()
                .ok()
                .flatten()
            {
                results.push(GateDecisionAttestationOutput {
                    action_hash,
                    attestation: wire_gate_decision_attestation(&entry),
                });
            }
        }
    }

    Ok(results)
}

/// Post-commit hook — emits MishpatSignal::GateDecisionCreated for each
/// committed GateDecisionAttestation so elohim-storage can project it.
///
/// Other entry types currently have no post-commit projection needs;
/// this hook is a no-op for them.
#[hdk_extern]
pub fn post_commit(committed_actions: Vec<SignedActionHashed>) -> ExternResult<()> {
    for signed_action in committed_actions {
        let action = signed_action.action();
        let author = action.author().clone();

        // Only handle Create/Update actions that carry an entry
        let (action_hash, entry_hash) = match action {
            Action::Create(create) => (signed_action.as_hash().clone(), create.entry_hash.clone()),
            Action::Update(update) => (signed_action.as_hash().clone(), update.entry_hash.clone()),
            _ => continue,
        };

        let record = match get(action_hash.clone(), GetOptions::default())? {
            Some(r) => r,
            None => continue,
        };

        if let Some(entry) = record
            .entry()
            .to_app_option::<mishpat_integrity::GateDecisionAttestation>()
            .ok()
            .flatten()
        {
            let _ = emit_signal(MishpatSignal::GateDecisionCreated {
                action_hash,
                entry_hash,
                entry,
                author,
            });
        }
    }
    Ok(())
}

// ============================================================
// CROSS-DNA BRIDGES (future)
// ============================================================
//
// Bridge to imagodei for identity verification will be added
// when signal wiring connects governance to identity attestations.
//
// Example pattern:
//   let response: ZomeCallResponse = call(
//       CallTargetCell::OtherRole("imagodei".into()),
//       "imagodei",
//       "get_agent_attestations".into(),
//       None,
//       challenger_id,
//   )?;

// =============================================================================
// Credential Verification (for P2P trust negotiation)
// =============================================================================
// VerificationStatus and CredentialVerification re-exported from qahal_types above.

/// Verify multiple credentials against the DHT.
/// Used by elohim-storage during P2P trust negotiation to verify
/// collective membership CIDs presented by connecting peers.
#[hdk_extern]
pub fn verify_credentials(hashes: Vec<ActionHash>) -> ExternResult<Vec<CredentialVerification>> {
    let mut results = Vec::with_capacity(hashes.len());

    for hash in hashes {
        let record = get(hash.clone(), GetOptions::default())?;
        let verification = match record {
            Some(record) => {
                let entry_type_name = match record.action().entry_type() {
                    Some(EntryType::App(app_entry)) => Some(format!("{:?}", app_entry)),
                    _ => None,
                };
                let agent = Some(record.action().author().clone());
                CredentialVerification {
                    hash,
                    status: VerificationStatus::Valid,
                    entry_type: entry_type_name,
                    agent,
                }
            }
            None => CredentialVerification {
                hash,
                status: VerificationStatus::NotFound,
                entry_type: None,
                agent: None,
            },
        };
        results.push(verification);
    }

    Ok(results)
}
