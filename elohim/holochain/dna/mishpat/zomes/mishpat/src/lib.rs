use hdk::prelude::*;
use mishpat_integrity::*;

// ============================================================
// INPUT / OUTPUT TYPES
// ============================================================

// ---- Challenge ----

/// Input for creating a challenge
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateChallengeInput {
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
    pub sla_deadline: Option<String>,
    pub assigned_elohim: Option<String>,
    pub resolution_json: Option<String>,
    pub metadata_json: String,
}

/// Output for challenge
#[derive(Serialize, Deserialize, Debug)]
pub struct ChallengeOutput {
    pub action_hash: ActionHash,
    pub challenge: Challenge,
}

/// Input for querying challenges
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryChallengesInput {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub challenger_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<u32>,
}

// ---- Proposal ----

/// Input for creating a proposal
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateProposalInput {
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
    pub outcome_json: Option<String>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<String>,
    pub metadata_json: String,
}

/// Output for proposal
#[derive(Serialize, Deserialize, Debug)]
pub struct ProposalOutput {
    pub action_hash: ActionHash,
    pub proposal: Proposal,
}

/// Input for querying proposals
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryProposalsInput {
    pub proposal_type: Option<String>,
    pub proposer_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<u32>,
}

// ---- Precedent ----

/// Input for creating a precedent
#[derive(Serialize, Deserialize, Debug)]
pub struct CreatePrecedentInput {
    pub id: Option<String>,
    pub title: String,
    pub summary: String,
    pub full_reasoning: String,
    pub binding: String,              // constitutional, binding-network, binding-local, persuasive
    pub scope_json: String,
    pub established_by: String,
    pub status: String,
    pub superseded_by: Option<String>,
    pub metadata_json: String,
}

/// Output for precedent
#[derive(Serialize, Deserialize, Debug)]
pub struct PrecedentOutput {
    pub action_hash: ActionHash,
    pub precedent: Precedent,
}

/// Input for querying precedents
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryPrecedentsInput {
    pub status: Option<String>,
    pub binding: Option<String>,
    pub limit: Option<u32>,
}

// ---- Discussion ----

/// Input for creating a discussion
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateDiscussionInput {
    pub id: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub category: String,
    pub title: String,
    pub messages_json: String,
    pub status: String,
    pub metadata_json: String,
}

/// Output for discussion
#[derive(Serialize, Deserialize, Debug)]
pub struct DiscussionOutput {
    pub action_hash: ActionHash,
    pub discussion: Discussion,
}

/// Input for querying discussions
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryDiscussionsInput {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub category: Option<String>,
    pub status: Option<String>,
    pub limit: Option<u32>,
}

// ---- GovernanceState ----

/// Input for creating/updating governance state
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateGovernanceStateInput {
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

/// Output for governance state
#[derive(Serialize, Deserialize, Debug)]
pub struct GovernanceStateOutput {
    pub action_hash: ActionHash,
    pub governance_state: GovernanceState,
}

/// Input for getting governance state
#[derive(Serialize, Deserialize, Debug)]
pub struct GetGovernanceStateInput {
    pub entity_type: String,
    pub entity_id: String,
}

/// Input for querying governance states
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryGovernanceStatesInput {
    pub status: Option<String>,
    pub limit: Option<u32>,
}

// ---- GovernanceReaction ----

/// Input for creating a governance reaction
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateGovernanceReactionInput {
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

/// Output for governance reaction
#[derive(Serialize, Deserialize, Debug)]
pub struct GovernanceReactionOutput {
    pub action_hash: ActionHash,
    pub reaction: GovernanceReaction,
}

/// Input for querying governance reactions
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryGovernanceReactionsInput {
    pub content_id: Option<String>,
    pub reactor_id: Option<String>,
    pub reaction_type: Option<String>,
    pub limit: Option<u32>,
}

// ---- GraduatedFeedback ----

/// Input for creating graduated feedback
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateGraduatedFeedbackInput {
    pub id: Option<String>,
    pub content_id: String,
    pub content_type: String,
    pub responder_id: String,
    pub feedback_context: String,
    pub position: i8,
    pub intensity: u8,
    pub reasoning: Option<String>,
    pub metadata_json: String,
}

/// Output for graduated feedback
#[derive(Serialize, Deserialize, Debug)]
pub struct GraduatedFeedbackOutput {
    pub action_hash: ActionHash,
    pub feedback: GraduatedFeedback,
}

/// Input for querying graduated feedback
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryGraduatedFeedbackInput {
    pub content_id: Option<String>,
    pub responder_id: Option<String>,
    pub feedback_context: Option<String>,
    pub limit: Option<u32>,
}

// ---- ProposalVote ----

/// Input for casting a proposal vote
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateProposalVoteInput {
    pub id: Option<String>,
    pub proposal_id: String,
    pub voter_id: String,
    pub voter_name: String,
    pub position: String,
    pub reasoning: Option<String>,
    pub version: u32,
    pub previous_position: Option<String>,
    pub metadata_json: String,
}

/// Output for proposal vote
#[derive(Serialize, Deserialize, Debug)]
pub struct ProposalVoteOutput {
    pub action_hash: ActionHash,
    pub vote: ProposalVote,
}

/// Input for querying proposal votes
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryProposalVotesInput {
    pub proposal_id: Option<String>,
    pub voter_id: Option<String>,
    pub position: Option<String>,
    pub limit: Option<u32>,
}

// ---- OpinionStatement ----

/// Input for creating an opinion statement
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateOpinionStatementInput {
    pub id: Option<String>,
    pub context_id: String,
    pub author_id: String,
    pub text: String,
    pub status: String,
    pub metadata_json: String,
}

/// Output for opinion statement
#[derive(Serialize, Deserialize, Debug)]
pub struct OpinionStatementOutput {
    pub action_hash: ActionHash,
    pub statement: OpinionStatement,
}

/// Input for querying opinion statements
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryOpinionStatementsInput {
    pub context_id: Option<String>,
    pub author_id: Option<String>,
    pub limit: Option<u32>,
}

// ---- StatementVote ----

/// Input for casting a statement vote
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateStatementVoteInput {
    pub id: Option<String>,
    pub statement_id: String,
    pub voter_id: String,
    pub vote: String,
    pub metadata_json: String,
}

/// Output for statement vote
#[derive(Serialize, Deserialize, Debug)]
pub struct StatementVoteOutput {
    pub action_hash: ActionHash,
    pub statement_vote: StatementVote,
}

/// Input for querying statement votes
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryStatementVotesInput {
    pub statement_id: Option<String>,
    pub voter_id: Option<String>,
    pub limit: Option<u32>,
}

// ============================================================
// CHALLENGE FUNCTIONS
// ============================================================

/// Create a challenge
#[hdk_extern]
pub fn create_challenge(input: CreateChallengeInput) -> ExternResult<ChallengeOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let challenge_id = input.id.unwrap_or_else(|| {
        format!("chal-{}-{}", input.entity_id, timestamp)
    });

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
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToChallenge, ())?;

    // Link by entity
    let entity_key = format!("{}:{}", input.entity_type, input.entity_id);
    let entity_anchor = StringAnchor::new("challenge_entity", &entity_key);
    let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;
    create_link(entity_anchor_hash, action_hash.clone(), LinkTypes::EntityToChallenge, ())?;

    // Link by challenger
    let challenger_anchor = StringAnchor::new("challenge_challenger", &input.challenger_id);
    let challenger_anchor_hash = hash_entry(&EntryTypes::StringAnchor(challenger_anchor))?;
    create_link(challenger_anchor_hash, action_hash.clone(), LinkTypes::ChallengerToChallenge, ())?;

    // Link by status
    let status_anchor = StringAnchor::new("challenge_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::ChallengeByStatus, ())?;

    Ok(ChallengeOutput {
        action_hash,
        challenge,
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
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid challenge hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(challenge) = record.entry().to_app_option::<Challenge>().ok().flatten() {
                return Ok(Some(ChallengeOutput { action_hash, challenge }));
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
        let entity_key = format!("{}:{}", input.entity_type.as_ref().unwrap(), input.entity_id.as_ref().unwrap());
        let entity_anchor = StringAnchor::new("challenge_entity", &entity_key);
        let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;

        let query = LinkQuery::try_new(entity_anchor_hash, LinkTypes::EntityToChallenge)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid challenge hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(challenge) = record.entry().to_app_option::<Challenge>().ok().flatten() {
                    // Filter by status if specified
                    if let Some(ref status) = input.status {
                        if &challenge.status != status {
                            continue;
                        }
                    }
                    results.push(ChallengeOutput { action_hash, challenge });
                }
            }
        }
    } else if let Some(status) = &input.status {
        let status_anchor = StringAnchor::new("challenge_status", status);
        let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;

        let query = LinkQuery::try_new(status_anchor_hash, LinkTypes::ChallengeByStatus)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid challenge hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(challenge) = record.entry().to_app_option::<Challenge>().ok().flatten() {
                    results.push(ChallengeOutput { action_hash, challenge });
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

    let proposal_id = input.id.unwrap_or_else(|| {
        format!("prop-{}", timestamp)
    });

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
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToProposal, ())?;

    // Link by type
    let type_anchor = StringAnchor::new("proposal_type", &input.proposal_type);
    let type_anchor_hash = hash_entry(&EntryTypes::StringAnchor(type_anchor))?;
    create_link(type_anchor_hash, action_hash.clone(), LinkTypes::ProposalByType, ())?;

    // Link by proposer
    let proposer_anchor = StringAnchor::new("proposal_proposer", &input.proposer_id);
    let proposer_anchor_hash = hash_entry(&EntryTypes::StringAnchor(proposer_anchor))?;
    create_link(proposer_anchor_hash, action_hash.clone(), LinkTypes::ProposerToProposal, ())?;

    // Link by status
    let status_anchor = StringAnchor::new("proposal_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::ProposalByStatus, ())?;

    Ok(ProposalOutput {
        action_hash,
        proposal,
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
                return Ok(Some(ProposalOutput { action_hash, proposal }));
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
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid proposal hash".to_string())))?;

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
                    results.push(ProposalOutput { action_hash, proposal });
                }
            }
        }
    } else if let Some(proposal_type) = &input.proposal_type {
        let type_anchor = StringAnchor::new("proposal_type", proposal_type);
        let type_anchor_hash = hash_entry(&EntryTypes::StringAnchor(type_anchor))?;

        let query = LinkQuery::try_new(type_anchor_hash, LinkTypes::ProposalByType)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid proposal hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(proposal) = record.entry().to_app_option::<Proposal>().ok().flatten() {
                    results.push(ProposalOutput { action_hash, proposal });
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

    let precedent_id = input.id.unwrap_or_else(|| {
        format!("prec-{}", timestamp)
    });

    let precedent = Precedent {
        id: precedent_id.clone(),
        title: input.title,
        summary: input.summary,
        full_reasoning: input.full_reasoning,
        binding: input.binding.clone(),
        scope_json: input.scope_json.clone(),
        citations: 0,  // Starts at 0, incremented when cited
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
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToPrecedent, ())?;

    // Link by scope (use raw JSON as key — caller controls granularity)
    let scope_anchor = StringAnchor::new("precedent_scope", &input.scope_json);
    let scope_anchor_hash = hash_entry(&EntryTypes::StringAnchor(scope_anchor))?;
    create_link(scope_anchor_hash, action_hash.clone(), LinkTypes::PrecedentByScope, ())?;

    // Link by status
    let status_anchor = StringAnchor::new("precedent_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::PrecedentByStatus, ())?;

    Ok(PrecedentOutput {
        action_hash,
        precedent,
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
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid precedent hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(precedent) = record.entry().to_app_option::<Precedent>().ok().flatten() {
                return Ok(Some(PrecedentOutput { action_hash, precedent }));
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
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid precedent hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(precedent) = record.entry().to_app_option::<Precedent>().ok().flatten() {
                    // Filter by binding if specified
                    if let Some(ref binding) = input.binding {
                        if &precedent.binding != binding {
                            continue;
                        }
                    }
                    results.push(PrecedentOutput { action_hash, precedent });
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

    let discussion_id = input.id.unwrap_or_else(|| {
        format!("disc-{}-{}", input.entity_id, timestamp)
    });

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
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToDiscussion, ())?;

    // Link by entity
    let entity_key = format!("{}:{}", input.entity_type, input.entity_id);
    let entity_anchor = StringAnchor::new("discussion_entity", &entity_key);
    let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;
    create_link(entity_anchor_hash, action_hash.clone(), LinkTypes::EntityToDiscussion, ())?;

    // Link by category
    let category_anchor = StringAnchor::new("discussion_category", &input.category);
    let category_anchor_hash = hash_entry(&EntryTypes::StringAnchor(category_anchor))?;
    create_link(category_anchor_hash, action_hash.clone(), LinkTypes::DiscussionByCategory, ())?;

    // Link by status
    let status_anchor = StringAnchor::new("discussion_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::DiscussionByStatus, ())?;

    Ok(DiscussionOutput {
        action_hash,
        discussion,
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
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid discussion hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(discussion) = record.entry().to_app_option::<Discussion>().ok().flatten() {
                return Ok(Some(DiscussionOutput { action_hash, discussion }));
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
        let entity_key = format!("{}:{}", input.entity_type.as_ref().unwrap(), input.entity_id.as_ref().unwrap());
        let entity_anchor = StringAnchor::new("discussion_entity", &entity_key);
        let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;

        let query = LinkQuery::try_new(entity_anchor_hash, LinkTypes::EntityToDiscussion)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid discussion hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(discussion) = record.entry().to_app_option::<Discussion>().ok().flatten() {
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
                    results.push(DiscussionOutput { action_hash, discussion });
                }
            }
        }
    } else if let Some(category) = &input.category {
        let category_anchor = StringAnchor::new("discussion_category", category);
        let category_anchor_hash = hash_entry(&EntryTypes::StringAnchor(category_anchor))?;

        let query = LinkQuery::try_new(category_anchor_hash, LinkTypes::DiscussionByCategory)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid discussion hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(discussion) = record.entry().to_app_option::<Discussion>().ok().flatten() {
                    results.push(DiscussionOutput { action_hash, discussion });
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
pub fn set_governance_state(input: CreateGovernanceStateInput) -> ExternResult<GovernanceStateOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let governance_state_id = input.id.unwrap_or_else(|| {
        format!("gs-{}:{}", input.entity_type, input.entity_id)
    });

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
    create_link(entity_anchor_hash, action_hash.clone(), LinkTypes::IdToGovernanceState, ())?;

    // Link by status
    let status_anchor = StringAnchor::new("governance_state_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::GovernanceStateByStatus, ())?;

    Ok(GovernanceStateOutput {
        action_hash,
        governance_state,
    })
}

/// Get governance state for an entity
#[hdk_extern]
pub fn get_governance_state(input: GetGovernanceStateInput) -> ExternResult<Option<GovernanceStateOutput>> {
    let entity_key = format!("{}:{}", input.entity_type, input.entity_id);
    let entity_anchor = StringAnchor::new("governance_state_entity", &entity_key);
    let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;

    let query = LinkQuery::try_new(entity_anchor_hash, LinkTypes::IdToGovernanceState)?;
    let links = get_links(query, GetStrategy::default())?;

    // Return the most recent governance state
    if let Some(link) = links.last() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid governance state hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(governance_state) = record.entry().to_app_option::<GovernanceState>().ok().flatten() {
                return Ok(Some(GovernanceStateOutput { action_hash, governance_state }));
            }
        }
    }

    Ok(None)
}

/// Query governance states by status
#[hdk_extern]
pub fn query_governance_states(input: QueryGovernanceStatesInput) -> ExternResult<Vec<GovernanceStateOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(status) = &input.status {
        let status_anchor = StringAnchor::new("governance_state_status", status);
        let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;

        let query = LinkQuery::try_new(status_anchor_hash, LinkTypes::GovernanceStateByStatus)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid governance state hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(governance_state) = record.entry().to_app_option::<GovernanceState>().ok().flatten() {
                    results.push(GovernanceStateOutput { action_hash, governance_state });
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
pub fn create_governance_reaction(input: CreateGovernanceReactionInput) -> ExternResult<GovernanceReactionOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let reaction_id = input.id.unwrap_or_else(|| {
        format!("rxn-{}-{}-{}", input.content_id, input.reactor_id, timestamp)
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
    create_link(content_anchor_hash, action_hash.clone(), LinkTypes::ContentToReactions, ())?;

    // Link by agent
    let agent_anchor = StringAnchor::new("reaction_agent", &input.reactor_id);
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;
    create_link(agent_anchor_hash, action_hash.clone(), LinkTypes::AgentToReactions, ())?;

    // Link by reaction type
    let type_anchor = StringAnchor::new("reaction_type", &input.reaction);
    let type_anchor_hash = hash_entry(&EntryTypes::StringAnchor(type_anchor))?;
    create_link(type_anchor_hash, action_hash.clone(), LinkTypes::ReactionByType, ())?;

    Ok(GovernanceReactionOutput {
        action_hash,
        reaction,
    })
}

/// Query governance reactions
#[hdk_extern]
pub fn query_governance_reactions(input: QueryGovernanceReactionsInput) -> ExternResult<Vec<GovernanceReactionOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(content_id) = &input.content_id {
        let content_anchor = StringAnchor::new("reaction_content", content_id);
        let content_anchor_hash = hash_entry(&EntryTypes::StringAnchor(content_anchor))?;

        let query = LinkQuery::try_new(content_anchor_hash, LinkTypes::ContentToReactions)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid reaction hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(reaction) = record.entry().to_app_option::<GovernanceReaction>().ok().flatten() {
                    if let Some(ref reaction_type) = input.reaction_type {
                        if &reaction.reaction != reaction_type {
                            continue;
                        }
                    }
                    results.push(GovernanceReactionOutput { action_hash, reaction });
                }
            }
        }
    } else if let Some(reactor_id) = &input.reactor_id {
        let agent_anchor = StringAnchor::new("reaction_agent", reactor_id);
        let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;

        let query = LinkQuery::try_new(agent_anchor_hash, LinkTypes::AgentToReactions)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid reaction hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(reaction) = record.entry().to_app_option::<GovernanceReaction>().ok().flatten() {
                    results.push(GovernanceReactionOutput { action_hash, reaction });
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
pub fn create_graduated_feedback(input: CreateGraduatedFeedbackInput) -> ExternResult<GraduatedFeedbackOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let feedback_id = input.id.unwrap_or_else(|| {
        format!("fb-{}-{}-{}", input.content_id, input.responder_id, input.feedback_context)
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
    create_link(content_anchor_hash, action_hash.clone(), LinkTypes::ContentToFeedback, ())?;

    // Link by agent
    let agent_anchor = StringAnchor::new("feedback_agent", &input.responder_id);
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;
    create_link(agent_anchor_hash, action_hash.clone(), LinkTypes::AgentToFeedback, ())?;

    // Link by context
    let context_anchor = StringAnchor::new("feedback_context", &input.feedback_context);
    let context_anchor_hash = hash_entry(&EntryTypes::StringAnchor(context_anchor))?;
    create_link(context_anchor_hash, action_hash.clone(), LinkTypes::FeedbackByContext, ())?;

    Ok(GraduatedFeedbackOutput {
        action_hash,
        feedback,
    })
}

/// Query graduated feedback
#[hdk_extern]
pub fn query_graduated_feedback(input: QueryGraduatedFeedbackInput) -> ExternResult<Vec<GraduatedFeedbackOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(content_id) = &input.content_id {
        let content_anchor = StringAnchor::new("feedback_content", content_id);
        let content_anchor_hash = hash_entry(&EntryTypes::StringAnchor(content_anchor))?;

        let query = LinkQuery::try_new(content_anchor_hash, LinkTypes::ContentToFeedback)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid feedback hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(feedback) = record.entry().to_app_option::<GraduatedFeedback>().ok().flatten() {
                    if let Some(ref ctx) = input.feedback_context {
                        if &feedback.feedback_context != ctx {
                            continue;
                        }
                    }
                    results.push(GraduatedFeedbackOutput { action_hash, feedback });
                }
            }
        }
    } else if let Some(responder_id) = &input.responder_id {
        let agent_anchor = StringAnchor::new("feedback_agent", responder_id);
        let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;

        let query = LinkQuery::try_new(agent_anchor_hash, LinkTypes::AgentToFeedback)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid feedback hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(feedback) = record.entry().to_app_option::<GraduatedFeedback>().ok().flatten() {
                    results.push(GraduatedFeedbackOutput { action_hash, feedback });
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

    let vote_id = input.id.unwrap_or_else(|| {
        format!("pv-{}-{}", input.proposal_id, input.voter_id)
    });

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
    create_link(proposal_anchor_hash, action_hash.clone(), LinkTypes::ProposalToVotes, ())?;

    // Link by agent
    let agent_anchor = StringAnchor::new("proposal_vote_agent", &input.voter_id);
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;
    create_link(agent_anchor_hash, action_hash.clone(), LinkTypes::AgentToVotes, ())?;

    // Link by position
    let position_anchor = StringAnchor::new("proposal_vote_position", &input.position);
    let position_anchor_hash = hash_entry(&EntryTypes::StringAnchor(position_anchor))?;
    create_link(position_anchor_hash, action_hash.clone(), LinkTypes::VoteByPosition, ())?;

    Ok(ProposalVoteOutput {
        action_hash,
        vote,
    })
}

/// Query proposal votes
#[hdk_extern]
pub fn query_proposal_votes(input: QueryProposalVotesInput) -> ExternResult<Vec<ProposalVoteOutput>> {
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
                if let Some(vote) = record.entry().to_app_option::<ProposalVote>().ok().flatten() {
                    if let Some(ref position) = input.position {
                        if &vote.position != position {
                            continue;
                        }
                    }
                    results.push(ProposalVoteOutput { action_hash, vote });
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
                if let Some(vote) = record.entry().to_app_option::<ProposalVote>().ok().flatten() {
                    results.push(ProposalVoteOutput { action_hash, vote });
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
pub fn create_opinion_statement(input: CreateOpinionStatementInput) -> ExternResult<OpinionStatementOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let statement_id = input.id.unwrap_or_else(|| {
        format!("stmt-{}-{}", input.context_id, timestamp)
    });

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
    create_link(context_anchor_hash, action_hash.clone(), LinkTypes::ContextToStatements, ())?;

    // Link by agent
    let agent_anchor = StringAnchor::new("opinion_agent", &input.author_id);
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;
    create_link(agent_anchor_hash, action_hash.clone(), LinkTypes::AgentToStatements, ())?;

    Ok(OpinionStatementOutput {
        action_hash,
        statement,
    })
}

/// Get opinion statement by action hash
#[hdk_extern]
pub fn get_opinion_statement(action_hash: ActionHash) -> ExternResult<Option<OpinionStatementOutput>> {
    let record = get(action_hash.clone(), GetOptions::default())?;
    if let Some(record) = record {
        if let Some(statement) = record.entry().to_app_option::<OpinionStatement>().ok().flatten() {
            return Ok(Some(OpinionStatementOutput { action_hash, statement }));
        }
    }
    Ok(None)
}

/// Query opinion statements for a deliberation context
#[hdk_extern]
pub fn query_opinion_statements(input: QueryOpinionStatementsInput) -> ExternResult<Vec<OpinionStatementOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(context_id) = &input.context_id {
        let context_anchor = StringAnchor::new("opinion_context", context_id);
        let context_anchor_hash = hash_entry(&EntryTypes::StringAnchor(context_anchor))?;

        let query = LinkQuery::try_new(context_anchor_hash, LinkTypes::ContextToStatements)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid statement hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(statement) = record.entry().to_app_option::<OpinionStatement>().ok().flatten() {
                    results.push(OpinionStatementOutput { action_hash, statement });
                }
            }
        }
    } else if let Some(author_id) = &input.author_id {
        let agent_anchor = StringAnchor::new("opinion_agent", author_id);
        let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;

        let query = LinkQuery::try_new(agent_anchor_hash, LinkTypes::AgentToStatements)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid statement hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(statement) = record.entry().to_app_option::<OpinionStatement>().ok().flatten() {
                    results.push(OpinionStatementOutput { action_hash, statement });
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

    let vote_id = input.id.unwrap_or_else(|| {
        format!("sv-{}-{}", input.statement_id, input.voter_id)
    });

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
    create_link(statement_anchor_hash, action_hash.clone(), LinkTypes::StatementToVotes, ())?;

    // Link by agent
    let agent_anchor = StringAnchor::new("statement_vote_agent", &input.voter_id);
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;
    create_link(agent_anchor_hash, action_hash.clone(), LinkTypes::AgentToStatementVotes, ())?;

    Ok(StatementVoteOutput {
        action_hash,
        statement_vote,
    })
}

/// Query statement votes
#[hdk_extern]
pub fn query_statement_votes(input: QueryStatementVotesInput) -> ExternResult<Vec<StatementVoteOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(statement_id) = &input.statement_id {
        let statement_anchor = StringAnchor::new("statement_vote_statement", statement_id);
        let statement_anchor_hash = hash_entry(&EntryTypes::StringAnchor(statement_anchor))?;

        let query = LinkQuery::try_new(statement_anchor_hash, LinkTypes::StatementToVotes)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid statement vote hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(statement_vote) = record.entry().to_app_option::<StatementVote>().ok().flatten() {
                    results.push(StatementVoteOutput { action_hash, statement_vote });
                }
            }
        }
    } else if let Some(voter_id) = &input.voter_id {
        let agent_anchor = StringAnchor::new("statement_vote_agent", voter_id);
        let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;

        let query = LinkQuery::try_new(agent_anchor_hash, LinkTypes::AgentToStatementVotes)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid statement vote hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(statement_vote) = record.entry().to_app_option::<StatementVote>().ok().flatten() {
                    results.push(StatementVoteOutput { action_hash, statement_vote });
                }
            }
        }
    }

    Ok(results)
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

/// Status of a verified credential
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationStatus {
    Valid,
    NotFound,
    Revoked,
    Expired,
}

/// Result of verifying a single credential against the DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialVerification {
    pub hash: ActionHash,
    pub status: VerificationStatus,
    pub entry_type: Option<String>,
    pub agent: Option<AgentPubKey>,
}

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
