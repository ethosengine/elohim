use hdk::prelude::*;
use mishpat_integrity::*;

// Bootstrap-steward pattern (mishpat copy; reference lives in imagodei).
pub mod bootstrap_steward;
pub use bootstrap_steward::{
    am_i_bootstrap_steward, bootstrap_steward, maybe_bootstrap_steward, BootstrapStewardError,
    DnaProperties,
};

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
    // GateDecisionChallenge (Phase 11)
    CreateGateDecisionChallengeInput,
    GateDecisionChallengeOutput,
    QueryGateChallengesInput,
    // ChallengeOutcome (Phase 11)
    CreateChallengeOutcomeInput,
    ChallengeOutcomeOutput,
    QueryChallengeOutcomesInput,
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

fn wire_challenge_outcome(
    e: &mishpat_integrity::ChallengeOutcome,
) -> qahal_types::ChallengeOutcome {
    qahal_types::ChallengeOutcome {
        outcome_id: e.outcome_id.clone(),
        challenge_cid: e.challenge_cid.clone(),
        verdict: e.verdict.clone(),
        reviewer_consensus: e.reviewer_consensus.clone(),
        reasoning_json: e.reasoning_json.clone(),
        decided_at: e.decided_at.clone(),
        indemnification_actions_json: e.indemnification_actions_json.clone(),
    }
}

// ============================================================
// CHALLENGE FUNCTIONS
// ============================================================

/// Create a challenge (Stage C: full-replacement bridge — local Challenge entry removed).
///
/// Writes exclusively to elohim DNA via governance-action:challenge.
/// get_challenge_by_id / query_challenges removed (Stage F will query elohim DNA directly).
#[hdk_extern]
pub fn create_challenge(input: CreateChallengeInput) -> ExternResult<ChallengeOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let challenge_id = input
        .id
        .unwrap_or_else(|| format!("chal-{}-{}", input.entity_id, timestamp));

    // Build a local value for the wire output without creating a DHT entry.
    let challenge_wire = qahal_types::Challenge {
        id: challenge_id.clone(),
        entity_type: input.entity_type.clone(),
        entity_id: input.entity_id.clone(),
        challenger_id: input.challenger_id.clone(),
        challenger_name: input.challenger_name.clone(),
        challenger_standing: input.challenger_standing.clone(),
        grounds: input.grounds.clone(),
        description: input.description.clone(),
        evidence_json: input.evidence_json.clone(),
        status: input.status.clone(),
        filed_at: timestamp.clone(),
        acknowledged_at: None,
        sla_deadline: input.sla_deadline.clone(),
        assigned_elohim: input.assigned_elohim.clone(),
        priority: input.priority.clone(),
        resolution_json: input.resolution_json.clone(),
        created_at: timestamp.clone(),
        updated_at: timestamp.clone(),
        metadata_json: input.metadata_json.clone(),
    };

    let _ = call_elohim_propose_governance_action(ConsolidatedProposeGovernanceActionInput {
        governance_kind: "governance-action:challenge".to_string(),
        subject_cid: format!("{}:{}", input.entity_type, input.entity_id),
        title: format!(
            "Challenge on {}:{} by {}",
            input.entity_type, input.entity_id, input.challenger_id
        ),
        description: Some(input.description.clone()),
        reach: "community".to_string(),
        threshold: serde_json::json!({"type": "adjudication"}),
        eligibility_predicate: None,
        ballot_format: "adjudication".to_string(),
        closes_at: input
            .sla_deadline
            .clone()
            .unwrap_or_else(|| timestamp.clone()),
        parameters: Some(serde_json::json!({
            "entity_type": input.entity_type,
            "entity_id": input.entity_id,
            "challenger_id": input.challenger_id,
            "grounds": input.grounds,
            "status": input.status,
            "priority": input.priority,
            "assigned_elohim": input.assigned_elohim,
            "metadata_json": input.metadata_json,
        })),
    });

    // Sentinel ActionHash — canonical state now lives on elohim DNA.
    // Stage F will migrate callers to query elohim::get_governance_action_with_children.
    let action_hash = ActionHash::from_raw_36(vec![0u8; 36]);

    Ok(ChallengeOutput {
        action_hash,
        challenge: challenge_wire,
    })
}

// ============================================================
// PROPOSAL FUNCTIONS
// ============================================================

/// Create a proposal (Stage C: full-replacement bridge — local Proposal entry removed).
///
/// Writes exclusively to elohim DNA via governance-action:proposal.
/// get_proposal_by_id / query_proposals removed (Stage F queries elohim DNA directly).
#[hdk_extern]
pub fn create_proposal(input: CreateProposalInput) -> ExternResult<ProposalOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let proposal_id = input.id.clone().unwrap_or_else(|| format!("prop-{}", timestamp));

    let proposal_wire = qahal_types::Proposal {
        id: proposal_id.clone(),
        title: input.title.clone(),
        proposal_type: input.proposal_type.clone(),
        description: input.description.clone(),
        proposer_id: input.proposer_id.clone(),
        proposer_name: input.proposer_name.clone(),
        rationale: input.rationale.clone(),
        status: input.status.clone(),
        phase: input.phase.clone(),
        amendments_json: input.amendments_json.clone(),
        voting_config_json: input.voting_config_json.clone(),
        current_votes_json: input.current_votes_json.clone(),
        outcome_json: input.outcome_json.clone(),
        related_entity_type: input.related_entity_type.clone(),
        related_entity_id: input.related_entity_id.clone(),
        created_at: timestamp.clone(),
        updated_at: timestamp.clone(),
        metadata_json: input.metadata_json.clone(),
    };

    let closes_at = serde_json::from_str::<serde_json::Value>(&input.voting_config_json)
        .ok()
        .and_then(|v| v.get("closes_at").and_then(|c| c.as_str()).map(String::from))
        .unwrap_or_else(|| timestamp.clone());
    let ballot_format = serde_json::from_str::<serde_json::Value>(&input.voting_config_json)
        .ok()
        .and_then(|v| v.get("ballot_format").and_then(|b| b.as_str()).map(String::from))
        .unwrap_or_else(|| "consent".to_string());

    let _ = call_elohim_propose_governance_action(ConsolidatedProposeGovernanceActionInput {
        governance_kind: "governance-action:proposal".to_string(),
        subject_cid: input.proposer_id.clone(),
        title: input.title.clone(),
        description: Some(input.description.clone()),
        reach: "community".to_string(),
        threshold: serde_json::json!({"type": "majority"}),
        eligibility_predicate: None,
        ballot_format,
        closes_at,
        parameters: Some(serde_json::json!({
            "proposal_type": input.proposal_type,
            "phase": input.phase,
            "status": input.status,
            "rationale": input.rationale,
            "related_entity_type": input.related_entity_type,
            "related_entity_id": input.related_entity_id,
            "voting_config_json": input.voting_config_json,
            "metadata_json": input.metadata_json,
        })),
    });

    // Sentinel ActionHash — canonical state now lives on elohim DNA.
    // Stage F will migrate callers to query elohim::get_governance_action_with_children.
    let action_hash = ActionHash::from_raw_36(vec![0u8; 36]);

    Ok(ProposalOutput {
        action_hash,
        proposal: proposal_wire,
    })
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

/// Record a low-friction emotional reaction to content (Stage C: full-replacement bridge).
///
/// GovernanceReaction had no B.10 bridge. Stage C bridges it to attestation:governance-reaction
/// on elohim DNA. Local entry removed; query_governance_reactions removed (no live consumers).
#[hdk_extern]
pub fn create_governance_reaction(
    input: CreateGovernanceReactionInput,
) -> ExternResult<GovernanceReactionOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let reaction_id = input.id.clone().unwrap_or_else(|| {
        format!(
            "rxn-{}-{}-{}",
            input.content_id, input.reactor_id, timestamp
        )
    });

    let reaction_wire = qahal_types::GovernanceReaction {
        id: reaction_id.clone(),
        content_id: input.content_id.clone(),
        content_type: input.content_type.clone(),
        reactor_id: input.reactor_id.clone(),
        reaction: input.reaction.clone(),
        intensity: input.intensity,
        mediated: input.mediated,
        mediation_accepted: input.mediation_accepted,
        context_json: input.context_json.clone(),
        created_at: timestamp.clone(),
        updated_at: timestamp.clone(),
        metadata_json: input.metadata_json.clone(),
    };

    let _ = call_elohim_issue_attestation(ConsolidatedIssueAttestationInput {
        attestation_kind: "attestation:governance-reaction".to_string(),
        subject_cid: input.content_id.clone(),
        subject_kind: "content".to_string(),
        title: format!(
            "Reaction {} on {} by {}",
            input.reaction, input.content_id, input.reactor_id
        ),
        description: None,
        reach: "community".to_string(),
        metadata: serde_json::json!({
            "reaction": input.reaction,
            "intensity": input.intensity,
            "mediated": input.mediated,
            "mediation_accepted": input.mediation_accepted,
            "context_json": input.context_json,
            "metadata_json": input.metadata_json,
        }),
        parent_governance_action_cid: None,
        vote_value: Some(input.reaction.clone()),
        proof_class: "witness".to_string(),
        proof_evidence: serde_json::json!({"class": "witness"}),
        expires_at: None,
    });

    // Sentinel ActionHash — canonical state now lives on elohim DNA.
    let action_hash = ActionHash::from_raw_36(vec![0u8; 36]);

    Ok(GovernanceReactionOutput {
        action_hash,
        reaction: reaction_wire,
    })
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

/// Cast a Loomio-style 4-position vote on a proposal (Stage C: full-replacement bridge).
///
/// Local ProposalVote entry removed; writes exclusively to elohim DNA via
/// attestation:proposal-vote. query_proposal_votes removed (Stage F queries elohim DNA directly).
#[hdk_extern]
pub fn create_proposal_vote(input: CreateProposalVoteInput) -> ExternResult<ProposalVoteOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let vote_id = input
        .id
        .clone()
        .unwrap_or_else(|| format!("pv-{}-{}", input.proposal_id, input.voter_id));

    let vote_wire = qahal_types::ProposalVote {
        id: vote_id.clone(),
        proposal_id: input.proposal_id.clone(),
        voter_id: input.voter_id.clone(),
        voter_name: input.voter_name.clone(),
        position: input.position.clone(),
        reasoning: input.reasoning.clone(),
        version: input.version,
        previous_position: input.previous_position.clone(),
        created_at: timestamp.clone(),
        updated_at: timestamp.clone(),
        metadata_json: input.metadata_json.clone(),
    };

    let _ = call_elohim_issue_attestation(ConsolidatedIssueAttestationInput {
        attestation_kind: "attestation:proposal-vote".to_string(),
        subject_cid: input.proposal_id.clone(),
        subject_kind: "governance-action".to_string(),
        title: format!("Vote on proposal {} by {}", input.proposal_id, input.voter_id),
        description: Some(format!("Position: {}", input.position)),
        reach: "community".to_string(),
        metadata: serde_json::json!({
            "proposal_id": input.proposal_id,
            "voter_id": input.voter_id,
            "voter_name": input.voter_name,
            "position": input.position,
            "version": input.version,
            "previous_position": input.previous_position,
            "metadata_json": input.metadata_json,
        }),
        parent_governance_action_cid: Some(input.proposal_id.clone()),
        vote_value: Some(input.position.clone()),
        proof_class: "witness".to_string(),
        proof_evidence: serde_json::json!({
            "class": "witness",
            "reasoning": input.reasoning,
        }),
        expires_at: None,
    });

    // Sentinel ActionHash — canonical state now lives on elohim DNA.
    let action_hash = ActionHash::from_raw_36(vec![0u8; 36]);

    Ok(ProposalVoteOutput {
        action_hash,
        vote: vote_wire,
    })
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

/// Cast a vote on an opinion statement (Stage C: full-replacement bridge).
///
/// Local StatementVote entry removed; writes exclusively to elohim DNA via
/// attestation:statement-vote. query_statement_votes removed (Stage F queries elohim DNA directly).
#[hdk_extern]
pub fn create_statement_vote(input: CreateStatementVoteInput) -> ExternResult<StatementVoteOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let vote_id = input
        .id
        .clone()
        .unwrap_or_else(|| format!("sv-{}-{}", input.statement_id, input.voter_id));

    let vote_wire = qahal_types::StatementVote {
        id: vote_id.clone(),
        statement_id: input.statement_id.clone(),
        voter_id: input.voter_id.clone(),
        vote: input.vote.clone(),
        created_at: timestamp.clone(),
        metadata_json: input.metadata_json.clone(),
    };

    let _ = call_elohim_issue_attestation(ConsolidatedIssueAttestationInput {
        attestation_kind: "attestation:statement-vote".to_string(),
        subject_cid: input.statement_id.clone(),
        subject_kind: "governance-action".to_string(),
        title: format!(
            "Statement vote on {} by {}",
            input.statement_id, input.voter_id
        ),
        description: Some(format!("Vote: {}", input.vote)),
        reach: "community".to_string(),
        metadata: serde_json::json!({
            "statement_id": input.statement_id,
            "voter_id": input.voter_id,
            "vote": input.vote,
            "metadata_json": input.metadata_json,
        }),
        parent_governance_action_cid: Some(input.statement_id.clone()),
        vote_value: Some(input.vote.clone()),
        proof_class: "witness".to_string(),
        proof_evidence: serde_json::json!({"class": "witness"}),
        expires_at: None,
    });

    // Sentinel ActionHash — canonical state now lives on elohim DNA.
    let action_hash = ActionHash::from_raw_36(vec![0u8; 36]);

    Ok(StatementVoteOutput {
        action_hash,
        statement_vote: vote_wire,
    })
}

// ============================================================
// GATE DECISION ATTESTATION FUNCTIONS (Stage C: bridge-only)
// ============================================================
//
// GateDecisionAttestation and GateDecisionChallenge local entry types removed.
// All writes now go exclusively to elohim DNA via consolidated attestation store.
// Query functions removed — Stage F will query elohim::get_attestations_for_subject.

/// Signal emitted when a ChallengeOutcome is committed.
///
/// GateDecisionCreated and GateDecisionChallengeCreated variants removed (Stage C) —
/// those entry types now live on elohim DNA and elohim-storage projects from
/// elohim's content_store signals.
/// elohim-storage Task 11.2 handles ChallengeOutcome projection from this signal.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum MishpatSignal {
    /// Emitted when a ChallengeOutcome is committed (Phase 11 Task 11.1).
    /// elohim-storage Task 11.2 handles projection from this signal.
    ChallengeOutcomeCreated {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        entry: mishpat_integrity::ChallengeOutcome,
        author: AgentPubKey,
    },
}

/// Create a gate decision attestation (Stage C: full-replacement bridge).
///
/// Local GateDecisionAttestation entry removed; writes exclusively to elohim DNA
/// via attestation:gate-decision. get_gate_decision_by_id / get_decisions_by_* removed
/// (Stage F queries elohim::get_attestations_for_subject directly).
#[hdk_extern]
pub fn create_gate_decision_attestation(
    input: CreateGateDecisionAttestationInput,
) -> ExternResult<ActionHash> {
    let _ = call_elohim_issue_attestation(ConsolidatedIssueAttestationInput {
        attestation_kind: "attestation:gate-decision".to_string(),
        subject_cid: input.elohim_id.clone(),
        subject_kind: "agent".to_string(),
        title: format!("Gate decision: {} by {}", input.gate_name, input.elohim_id),
        description: Some(format!("Phase: {} — decision: {}", input.phase, input.decision)),
        reach: "community".to_string(),
        metadata: serde_json::json!({
            "decision_id": input.decision_id,
            "phase": input.phase,
            "elohim_id": input.elohim_id,
            "elohim_substance_cid": input.elohim_substance_cid,
            "gate_name": input.gate_name,
            "gate_process_cid": input.gate_process_cid,
            "request_ref_json": input.request_ref_json,
            "decision": input.decision,
            "context_summary_cid": input.context_summary_cid,
            "decided_at": input.decided_at,
            "universal_band_cid": input.universal_band_cid,
        }),
        parent_governance_action_cid: None,
        vote_value: None,
        proof_class: "audit".to_string(),
        proof_evidence: serde_json::json!({
            "class": "audit",
            "reasoning_json": input.reasoning_json,
        }),
        expires_at: None,
    });

    // Sentinel ActionHash — canonical state now lives on elohim DNA.
    Ok(ActionHash::from_raw_36(vec![0u8; 36]))
}

// =============================================================================
// Gate Decision Challenge + Outcome coordinators (Phase 11 / Stage C)
// =============================================================================

/// Create a gate decision challenge (Stage C: full-replacement bridge).
///
/// Local GateDecisionChallenge entry removed; writes exclusively to elohim DNA
/// via attestation:gate-decision-challenge. get_challenge_by_id_v2 /
/// get_challenges_against_decision removed (Stage F queries elohim DNA directly).
#[hdk_extern]
pub fn create_gate_decision_challenge(
    input: CreateGateDecisionChallengeInput,
) -> ExternResult<ActionHash> {
    let _ = call_elohim_issue_attestation(ConsolidatedIssueAttestationInput {
        attestation_kind: "attestation:gate-decision-challenge".to_string(),
        subject_cid: input.challenged_decision_cid.clone(),
        subject_kind: "governance-action".to_string(),
        title: format!(
            "Challenge on gate decision {} by {}",
            input.challenged_decision_cid, input.challenger_id
        ),
        description: Some(input.summary.clone()),
        reach: input.reach.clone(),
        metadata: serde_json::json!({
            "challenge_id": input.challenge_id,
            "challenged_decision_cid": input.challenged_decision_cid,
            "challenger_id": input.challenger_id,
            "grounds": input.grounds,
            "evidence_refs": input.evidence_refs,
            "filed_at": input.filed_at,
            "reach": input.reach,
        }),
        parent_governance_action_cid: Some(input.challenged_decision_cid.clone()),
        vote_value: None,
        proof_class: "witness".to_string(),
        proof_evidence: serde_json::json!({"class": "witness"}),
        expires_at: None,
    });

    // Sentinel ActionHash — canonical state now lives on elohim DNA.
    Ok(ActionHash::from_raw_36(vec![0u8; 36]))
}

/// Create and notarize a ChallengeOutcome on the Mishpat DHT.
///
/// Indexes by outcome_id, challenge_cid (one outcome per challenge), and verdict.
#[hdk_extern]
pub fn create_challenge_outcome(
    input: CreateChallengeOutcomeInput,
) -> ExternResult<ActionHash> {
    let entry = mishpat_integrity::ChallengeOutcome {
        outcome_id: input.outcome_id.clone(),
        challenge_cid: input.challenge_cid.clone(),
        verdict: input.verdict.clone(),
        reviewer_consensus: input.reviewer_consensus.clone(),
        reasoning_json: input.reasoning_json.clone(),
        decided_at: input.decided_at.clone(),
        indemnification_actions_json: input.indemnification_actions_json.clone(),
    };

    let action_hash = create_entry(&EntryTypes::ChallengeOutcome(entry.clone()))?;

    // Index by outcome_id
    let id_anchor = StringAnchor::new("challenge_outcome_id", &input.outcome_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToOutcome,
        (),
    )?;

    // Index by challenge_cid (one outcome per challenge)
    let challenge_anchor = StringAnchor::new("outcome_for_challenge", &input.challenge_cid);
    let challenge_anchor_hash = hash_entry(&EntryTypes::StringAnchor(challenge_anchor))?;
    create_link(
        challenge_anchor_hash,
        action_hash.clone(),
        LinkTypes::ChallengeToOutcome,
        (),
    )?;

    // Index by verdict (for reputation aggregation queries)
    let verdict_anchor = StringAnchor::new("outcome_verdict", &input.verdict);
    let verdict_anchor_hash = hash_entry(&EntryTypes::StringAnchor(verdict_anchor))?;
    create_link(
        verdict_anchor_hash,
        action_hash.clone(),
        LinkTypes::VerdictToOutcomes,
        (),
    )?;

    Ok(action_hash)
}

/// Get the outcome that resolved a given challenge (if any).
#[hdk_extern]
pub fn get_outcome_for_challenge(
    input: QueryChallengeOutcomesInput,
) -> ExternResult<Option<ChallengeOutcomeOutput>> {
    let challenge_cid = input.challenge_cid.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "challenge_cid is required for get_outcome_for_challenge".to_string()
        ))
    })?;

    let anchor = StringAnchor::new("outcome_for_challenge", &challenge_cid);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::ChallengeToOutcome)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest(
                "Invalid outcome hash".to_string()
            ))
        })?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(entry) = record
                .entry()
                .to_app_option::<mishpat_integrity::ChallengeOutcome>()
                .ok()
                .flatten()
            {
                return Ok(Some(ChallengeOutcomeOutput {
                    action_hash,
                    outcome: wire_challenge_outcome(&entry),
                }));
            }
        }
    }

    Ok(None)
}

/// Query outcomes by verdict (upheld | dismissed | superseded).
#[hdk_extern]
pub fn get_outcomes_by_verdict(
    input: QueryChallengeOutcomesInput,
) -> ExternResult<Vec<ChallengeOutcomeOutput>> {
    let verdict = input.verdict.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "verdict is required for get_outcomes_by_verdict".to_string()
        ))
    })?;
    let limit = input.limit.unwrap_or(100) as usize;

    let anchor = StringAnchor::new("outcome_verdict", &verdict);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::VerdictToOutcomes)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links.iter().take(limit) {
        let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest(
                "Invalid outcome hash".to_string()
            ))
        })?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(entry) = record
                .entry()
                .to_app_option::<mishpat_integrity::ChallengeOutcome>()
                .ok()
                .flatten()
            {
                results.push(ChallengeOutcomeOutput {
                    action_hash,
                    outcome: wire_challenge_outcome(&entry),
                });
            }
        }
    }

    Ok(results)
}

/// Post-commit hook — emits MishpatSignal::ChallengeOutcomeCreated for each
/// committed ChallengeOutcome so elohim-storage can project it.
///
/// GateDecisionAttestation and GateDecisionChallenge arms removed (Stage C) —
/// those entry types now live on elohim DNA and elohim-storage projects from
/// elohim's content_store signals.
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
            .to_app_option::<mishpat_integrity::ChallengeOutcome>()
            .ok()
            .flatten()
        {
            let _ = emit_signal(MishpatSignal::ChallengeOutcomeCreated {
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
// CROSS-DNA BRIDGES (Stage C consolidation — full-replacement)
// ============================================================
//
// All bridges are FULL-REPLACEMENT (Stage C). Local entry writes removed;
// canonical state lives on elohim DNA. Stage F will migrate callers to
// query elohim DNA directly (get_governance_action_with_children /
// get_attestations_for_subject).
//
// Bridged functions:
//   create_gate_decision_attestation  → attestation:gate-decision         (bridge-only)
//   create_gate_decision_challenge    → attestation:gate-decision-challenge (bridge-only, new in C)
//   create_proposal_vote              → attestation:proposal-vote          (bridge-only)
//   create_statement_vote             → attestation:statement-vote         (bridge-only)
//   create_governance_reaction        → attestation:governance-reaction    (bridge-only, new in C)
//   create_proposal                   → governance-action:proposal         (bridge-only)
//   create_challenge                  → governance-action:challenge        (bridge-only)

/// Input wire type — wire-compatible with elohim DNA's content_store::issue_attestation.
/// Defined locally because cross-DNA calls serialise through msgpack;
/// mishpat cannot depend on elohim crates directly.
#[derive(Debug, Serialize, Deserialize)]
struct ConsolidatedIssueAttestationInput {
    pub attestation_kind: String,
    pub subject_cid: String,
    pub subject_kind: String,
    pub title: String,
    pub description: Option<String>,
    pub reach: String,
    pub metadata: serde_json::Value,
    pub parent_governance_action_cid: Option<String>,
    pub vote_value: Option<String>,
    pub proof_class: String,
    pub proof_evidence: serde_json::Value,
    pub expires_at: Option<String>,
}

/// Output wire type — wire-compatible with elohim DNA's content_store::issue_attestation.
#[derive(Debug, Serialize, Deserialize)]
struct ConsolidatedAttestationOutput {
    pub cid: String,
    pub attestation_kind: String,
    pub subject_cid: String,
    pub issuer_cid: String,
}

/// Input wire type — wire-compatible with elohim DNA's content_store::propose_governance_action.
#[derive(Debug, Serialize, Deserialize)]
struct ConsolidatedProposeGovernanceActionInput {
    pub governance_kind: String,
    pub subject_cid: String,
    pub title: String,
    pub description: Option<String>,
    pub reach: String,
    pub threshold: serde_json::Value,
    pub eligibility_predicate: Option<serde_json::Value>,
    pub ballot_format: String,
    pub closes_at: String,
    pub parameters: Option<serde_json::Value>,
}

/// Output wire type — wire-compatible with elohim DNA's content_store::propose_governance_action.
#[derive(Debug, Serialize, Deserialize)]
struct ConsolidatedGovernanceActionOutput {
    pub cid: String,
    pub governance_kind: String,
    pub subject_cid: String,
    pub proposer_cid: String,
    pub closes_at: String,
}

/// Bridge helper: call elohim content_store::issue_attestation.
/// Mirrors the pattern from imagodei B.9 bridge. Returns Ok(output) on success;
/// errors are non-fatal (bridge best-effort during Stage B).
fn call_elohim_issue_attestation(
    input: ConsolidatedIssueAttestationInput,
) -> ExternResult<ConsolidatedAttestationOutput> {
    let response = call(
        CallTargetCell::OtherRole("elohim".into()),
        ZomeName::from("content_store"),
        FunctionName::from("issue_attestation"),
        None,
        input,
    )?;
    match response {
        ZomeCallResponse::Ok(result) => result.decode().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "bridge decode (elohim::issue_attestation): {e}"
            )))
        }),
        ZomeCallResponse::Unauthorized(_, _, _, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Unauthorized bridge call to elohim::issue_attestation".to_string()
        ))),
        ZomeCallResponse::NetworkError(err) => Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Network error bridging to elohim::issue_attestation: {err}"
        )))),
        ZomeCallResponse::CountersigningSession(err) => Err(wasm_error!(WasmErrorInner::Guest(
            format!("Countersigning error bridging to elohim: {err}")
        ))),
        ZomeCallResponse::AuthenticationFailed(_, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Authentication failed bridging to elohim::issue_attestation".to_string()
        ))),
    }
}

/// Bridge helper: call elohim content_store::propose_governance_action.
fn call_elohim_propose_governance_action(
    input: ConsolidatedProposeGovernanceActionInput,
) -> ExternResult<ConsolidatedGovernanceActionOutput> {
    let response = call(
        CallTargetCell::OtherRole("elohim".into()),
        ZomeName::from("content_store"),
        FunctionName::from("propose_governance_action"),
        None,
        input,
    )?;
    match response {
        ZomeCallResponse::Ok(result) => result.decode().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "bridge decode (elohim::propose_governance_action): {e}"
            )))
        }),
        ZomeCallResponse::Unauthorized(_, _, _, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Unauthorized bridge call to elohim::propose_governance_action".to_string()
        ))),
        ZomeCallResponse::NetworkError(err) => Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Network error bridging to elohim::propose_governance_action: {err}"
        )))),
        ZomeCallResponse::CountersigningSession(err) => Err(wasm_error!(WasmErrorInner::Guest(
            format!("Countersigning error bridging to elohim: {err}")
        ))),
        ZomeCallResponse::AuthenticationFailed(_, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Authentication failed bridging to elohim::propose_governance_action".to_string()
        ))),
    }
}

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
