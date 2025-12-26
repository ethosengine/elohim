//! Imago Dei Coordinator Zome
//!
//! Provides functions for identity management:
//! - Human/Agent profile CRUD
//! - Relationship management (social graph)
//! - Attestation issuing
//! - Content mastery tracking
//!
//! Key design: Self-sovereign identity. Agents own their data.
//! Doorways project it but never own it.

use hdk::prelude::*;
use imagodei_integrity::*;

// =============================================================================
// Input/Output Types
// =============================================================================

/// Input for creating/updating a Human profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateHumanInput {
    pub id: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub affinities: Vec<String>,
    pub profile_reach: String,
    pub location: Option<String>,
}

/// Input for creating/updating an Agent profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentInput {
    pub id: String,
    pub agent_type: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar: Option<String>,
    pub affinities: Vec<String>,
    pub visibility: String,
    pub location: Option<String>,
    pub did: Option<String>,
    pub activity_pub_type: Option<String>,
}

/// Output from profile operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanOutput {
    pub action_hash: ActionHash,
    pub human: Human,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub action_hash: ActionHash,
    pub agent: Agent,
}

/// Input for creating a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRelationshipInput {
    pub party_b_id: String,
    pub relationship_type: String,
    pub intimacy_level: String,
    pub custody_enabled: bool,
    pub emergency_access_enabled: bool,
    pub reach: String,
    pub context_json: Option<String>,
}

/// Output from relationship operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipOutput {
    pub action_hash: ActionHash,
    pub relationship: HumanRelationship,
}

/// Input for issuing an attestation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueAttestationInput {
    pub agent_id: String,
    pub category: String,
    pub attestation_type: String,
    pub display_name: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub tier: Option<String>,
    pub earned_via_json: String,
    pub expires_at: Option<String>,
}

/// Output from attestation operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationOutput {
    pub action_hash: ActionHash,
    pub attestation: Attestation,
}

// =============================================================================
// Signals for Projection
// =============================================================================

/// Signal types emitted after commits for real-time projection.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum ImagodeiSignal {
    HumanCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        human: Human,
        author: AgentPubKey,
    },
    AgentCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        agent: Agent,
        author: AgentPubKey,
    },
    RelationshipCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        relationship: HumanRelationship,
        author: AgentPubKey,
    },
    AttestationCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        attestation: Attestation,
        author: AgentPubKey,
    },
}

// =============================================================================
// Post-Commit Callback
// =============================================================================

#[hdk_extern]
pub fn post_commit(committed_actions: Vec<SignedActionHashed>) -> ExternResult<()> {
    for signed_action in committed_actions {
        let action = signed_action.hashed.content.clone();
        let action_hash = signed_action.hashed.hash.clone();

        let entry_hash = match &action {
            Action::Create(create) => create.entry_hash.clone(),
            Action::Update(update) => update.entry_hash.clone(),
            _ => continue,
        };

        let record = match get(action_hash.clone(), GetOptions::default())? {
            Some(r) => r,
            None => continue,
        };

        let author = action.author().clone();

        if let Some(human) = record.entry().to_app_option::<Human>().ok().flatten() {
            emit_signal(ImagodeiSignal::HumanCommitted {
                action_hash,
                entry_hash,
                human,
                author,
            })?;
        } else if let Some(agent) = record.entry().to_app_option::<Agent>().ok().flatten() {
            emit_signal(ImagodeiSignal::AgentCommitted {
                action_hash,
                entry_hash,
                agent,
                author,
            })?;
        } else if let Some(relationship) = record
            .entry()
            .to_app_option::<HumanRelationship>()
            .ok()
            .flatten()
        {
            emit_signal(ImagodeiSignal::RelationshipCommitted {
                action_hash,
                entry_hash,
                relationship,
                author,
            })?;
        } else if let Some(attestation) = record.entry().to_app_option::<Attestation>().ok().flatten()
        {
            emit_signal(ImagodeiSignal::AttestationCommitted {
                action_hash,
                entry_hash,
                attestation,
                author,
            })?;
        }
    }

    Ok(())
}

// =============================================================================
// Human Profile Functions
// =============================================================================

/// Create a new Human profile (bound to calling agent)
#[hdk_extern]
pub fn create_human(input: CreateHumanInput) -> ExternResult<HumanOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Check if this agent already has a Human profile
    let existing = get_human_by_agent_key(agent_info.agent_initial_pubkey.clone())?;
    if existing.is_some() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Agent already has a Human profile".to_string()
        )));
    }

    let human = Human {
        id: input.id.clone(),
        display_name: input.display_name,
        bio: input.bio,
        affinities: input.affinities.clone(),
        profile_reach: input.profile_reach,
        location: input.location,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::Human(human.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("human_id", &input.id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToHuman, ())?;

    // Bind to agent key (one-to-one)
    create_link(
        agent_info.agent_initial_pubkey,
        action_hash.clone(),
        LinkTypes::AgentKeyToHuman,
        (),
    )?;

    // Create affinity links
    for affinity in input.affinities {
        let affinity_anchor = StringAnchor::new("human_affinity", &affinity);
        let affinity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(affinity_anchor))?;
        create_link(
            affinity_anchor_hash,
            action_hash.clone(),
            LinkTypes::HumanByAffinity,
            (),
        )?;
    }

    Ok(HumanOutput { action_hash, human })
}

/// Get my Human profile (bound to calling agent)
#[hdk_extern]
pub fn get_my_human(_: ()) -> ExternResult<Option<HumanOutput>> {
    let agent_info = agent_info()?;
    get_human_by_agent_key(agent_info.agent_initial_pubkey)
}

/// Get Human by agent public key
#[hdk_extern]
pub fn get_human_by_agent_key(agent_key: AgentPubKey) -> ExternResult<Option<HumanOutput>> {
    let query = LinkQuery::try_new(agent_key, LinkTypes::AgentKeyToHuman)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(human) = record.entry().to_app_option::<Human>().ok().flatten() {
                    return Ok(Some(HumanOutput { action_hash, human }));
                }
            }
        }
    }

    Ok(None)
}

/// Get Human by ID
#[hdk_extern]
pub fn get_human_by_id(id: String) -> ExternResult<Option<HumanOutput>> {
    let id_anchor = StringAnchor::new("human_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToHuman)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(human) = record.entry().to_app_option::<Human>().ok().flatten() {
                    return Ok(Some(HumanOutput { action_hash, human }));
                }
            }
        }
    }

    Ok(None)
}

/// Update my Human profile
#[hdk_extern]
pub fn update_human(input: CreateHumanInput) -> ExternResult<HumanOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let existing = get_human_by_agent_key(agent_info.agent_initial_pubkey)?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("No Human profile found".to_string())))?;

    let human = Human {
        id: existing.human.id, // Keep original ID
        display_name: input.display_name,
        bio: input.bio,
        affinities: input.affinities,
        profile_reach: input.profile_reach,
        location: input.location,
        created_at: existing.human.created_at,
        updated_at: timestamp,
    };

    let action_hash = update_entry(existing.action_hash, &EntryTypes::Human(human.clone()))?;

    Ok(HumanOutput { action_hash, human })
}

// =============================================================================
// Relationship Functions
// =============================================================================

/// Create a new relationship (initiator = calling agent)
#[hdk_extern]
pub fn create_relationship(input: CreateRelationshipInput) -> ExternResult<RelationshipOutput> {
    let _agent_info = agent_info()?; // Reserved for future authorization checks
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Get my Human profile
    let my_human = get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile first".to_string())))?;

    let relationship_id = format!("{}-{}-{}", my_human.human.id, input.party_b_id, timestamp);

    let relationship = HumanRelationship {
        id: relationship_id.clone(),
        party_a_id: my_human.human.id.clone(),
        party_b_id: input.party_b_id.clone(),
        relationship_type: input.relationship_type.clone(),
        intimacy_level: input.intimacy_level.clone(),
        is_bidirectional: false, // Becomes true when party_b consents
        consent_given_by_a: true,
        consent_given_by_b: false,
        custody_enabled_by_a: input.custody_enabled,
        custody_enabled_by_b: false,
        auto_custody_enabled: input.intimacy_level == "intimate",
        shared_encryption_key_id: None,
        emergency_access_enabled: input.emergency_access_enabled,
        initiated_by: my_human.human.id.clone(),
        verified_at: None,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        expires_at: None,
        context_json: input.context_json,
        reach: input.reach,
    };

    let action_hash = create_entry(&EntryTypes::HumanRelationship(relationship.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("relationship_id", &relationship_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToHumanRelationship,
        (),
    )?;

    // Create agent lookup links (for both parties)
    let party_a_anchor = StringAnchor::new("agent_relationships", &my_human.human.id);
    let party_a_anchor_hash = hash_entry(&EntryTypes::StringAnchor(party_a_anchor))?;
    create_link(
        party_a_anchor_hash,
        action_hash.clone(),
        LinkTypes::AgentToRelationship,
        (),
    )?;

    let party_b_anchor = StringAnchor::new("agent_relationships", &input.party_b_id);
    let party_b_anchor_hash = hash_entry(&EntryTypes::StringAnchor(party_b_anchor))?;
    create_link(
        party_b_anchor_hash,
        action_hash.clone(),
        LinkTypes::AgentToRelationship,
        (),
    )?;

    // Link to pending consent queue
    let pending_anchor = StringAnchor::new("relationship_pending", "pending");
    let pending_anchor_hash = hash_entry(&EntryTypes::StringAnchor(pending_anchor))?;
    create_link(
        pending_anchor_hash,
        action_hash.clone(),
        LinkTypes::RelationshipPendingConsent,
        (),
    )?;

    Ok(RelationshipOutput {
        action_hash,
        relationship,
    })
}

/// Get my relationships
#[hdk_extern]
pub fn get_my_relationships(_: ()) -> ExternResult<Vec<RelationshipOutput>> {
    let my_human = get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    let anchor = StringAnchor::new("agent_relationships", &my_human.human.id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::AgentToRelationship)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(relationship) = record
                    .entry()
                    .to_app_option::<HumanRelationship>()
                    .ok()
                    .flatten()
                {
                    results.push(RelationshipOutput {
                        action_hash,
                        relationship,
                    });
                }
            }
        }
    }

    Ok(results)
}

// =============================================================================
// Attestation Functions
// =============================================================================

/// Issue an attestation to an agent
#[hdk_extern]
pub fn issue_attestation(input: IssueAttestationInput) -> ExternResult<AttestationOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let attestation_id = format!("{}-{}-{}", input.agent_id, input.attestation_type, timestamp);

    let attestation = Attestation {
        id: attestation_id.clone(),
        agent_id: input.agent_id.clone(),
        category: input.category.clone(),
        attestation_type: input.attestation_type.clone(),
        display_name: input.display_name,
        description: input.description,
        icon_url: input.icon_url,
        tier: input.tier,
        earned_via_json: input.earned_via_json,
        issued_at: timestamp,
        issued_by: agent_info.agent_initial_pubkey.to_string(),
        expires_at: input.expires_at,
        proof: None, // TODO: Add signature
    };

    let action_hash = create_entry(&EntryTypes::Attestation(attestation.clone()))?;

    // Create agent lookup link
    let agent_anchor = StringAnchor::new("agent_attestations", &input.agent_id);
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;
    create_link(
        agent_anchor_hash,
        action_hash.clone(),
        LinkTypes::AgentToAttestation,
        (),
    )?;

    // Create category lookup link
    let category_anchor = StringAnchor::new("attestation_category", &input.category);
    let category_anchor_hash = hash_entry(&EntryTypes::StringAnchor(category_anchor))?;
    create_link(
        category_anchor_hash,
        action_hash.clone(),
        LinkTypes::AttestationByCategory,
        (),
    )?;

    // Create type lookup link
    let type_anchor = StringAnchor::new("attestation_type", &input.attestation_type);
    let type_anchor_hash = hash_entry(&EntryTypes::StringAnchor(type_anchor))?;
    create_link(
        type_anchor_hash,
        action_hash.clone(),
        LinkTypes::AttestationByType,
        (),
    )?;

    Ok(AttestationOutput {
        action_hash,
        attestation,
    })
}

/// Get attestations for an agent
#[hdk_extern]
pub fn get_agent_attestations(agent_id: String) -> ExternResult<Vec<AttestationOutput>> {
    let anchor = StringAnchor::new("agent_attestations", &agent_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::AgentToAttestation)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(attestation) = record.entry().to_app_option::<Attestation>().ok().flatten()
                {
                    results.push(AttestationOutput {
                        action_hash,
                        attestation,
                    });
                }
            }
        }
    }

    Ok(results)
}

/// Get my attestations
#[hdk_extern]
pub fn get_my_attestations(_: ()) -> ExternResult<Vec<AttestationOutput>> {
    let my_human = get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    get_agent_attestations(my_human.human.id)
}

// =============================================================================
// Agent Functions
// =============================================================================

/// Output from agent operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProgressOutput {
    pub action_hash: ActionHash,
    pub progress: AgentProgress,
}

/// Input for creating agent progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentProgressInput {
    pub agent_id: String,
    pub path_id: String,
}

/// Input for updating agent progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAgentProgressInput {
    pub agent_id: String,
    pub path_id: String,
    pub current_step_index: Option<u32>,
    pub completed_step_index: Option<u32>,
    pub completed_content_id: Option<String>,
}

/// Create an Agent profile
#[hdk_extern]
pub fn create_agent(input: CreateAgentInput) -> ExternResult<AgentOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let agent = Agent {
        id: input.id.clone(),
        agent_type: input.agent_type.clone(),
        display_name: input.display_name,
        bio: input.bio,
        avatar: input.avatar,
        affinities: input.affinities.clone(),
        visibility: input.visibility,
        location: input.location,
        holochain_agent_key: Some(agent_info.agent_initial_pubkey.to_string()),
        did: input.did,
        activity_pub_type: input.activity_pub_type,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::Agent(agent.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("agent_id", &input.id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToAgent, ())?;

    // Bind to agent key
    create_link(
        agent_info.agent_initial_pubkey,
        action_hash.clone(),
        LinkTypes::AgentKeyToAgent,
        (),
    )?;

    // Create affinity links
    for affinity in input.affinities {
        let affinity_anchor = StringAnchor::new("agent_affinity", &affinity);
        let affinity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(affinity_anchor))?;
        create_link(
            affinity_anchor_hash,
            action_hash.clone(),
            LinkTypes::AgentByAffinity,
            (),
        )?;
    }

    Ok(AgentOutput { action_hash, agent })
}

/// Get Agent by ID
#[hdk_extern]
pub fn get_agent_by_id(id: String) -> ExternResult<Option<AgentOutput>> {
    let id_anchor = StringAnchor::new("agent_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToAgent)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(agent) = record.entry().to_app_option::<Agent>().ok().flatten() {
                    return Ok(Some(AgentOutput { action_hash, agent }));
                }
            }
        }
    }

    Ok(None)
}

// =============================================================================
// Agent Progress Functions
// =============================================================================

/// Create or get agent progress for a path
#[hdk_extern]
pub fn get_or_create_agent_progress(input: CreateAgentProgressInput) -> ExternResult<AgentProgressOutput> {
    let progress_id = format!("{}-{}", input.agent_id, input.path_id);
    let progress_anchor = StringAnchor::new("agent_progress", &progress_id);
    let progress_anchor_hash = hash_entry(&EntryTypes::StringAnchor(progress_anchor))?;

    // Check if progress exists
    let query = LinkQuery::try_new(progress_anchor_hash.clone(), LinkTypes::AgentToProgress)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(progress) = record.entry().to_app_option::<AgentProgress>().ok().flatten() {
                    return Ok(AgentProgressOutput { action_hash, progress });
                }
            }
        }
    }

    // Create new progress
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let progress = AgentProgress {
        id: progress_id,
        agent_id: input.agent_id,
        path_id: input.path_id,
        current_step_index: 0,
        completed_step_indices: Vec::new(),
        completed_content_ids: Vec::new(),
        step_affinity_json: "{}".to_string(),
        step_notes_json: "{}".to_string(),
        reflection_responses_json: "{}".to_string(),
        attestations_earned: Vec::new(),
        started_at: timestamp.clone(),
        last_activity_at: timestamp,
        completed_at: None,
    };

    let action_hash = create_entry(&EntryTypes::AgentProgress(progress.clone()))?;
    create_link(progress_anchor_hash, action_hash.clone(), LinkTypes::AgentToProgress, ())?;

    Ok(AgentProgressOutput { action_hash, progress })
}

/// Update agent progress
#[hdk_extern]
pub fn update_agent_progress(input: UpdateAgentProgressInput) -> ExternResult<AgentProgressOutput> {
    let progress_id = format!("{}-{}", input.agent_id, input.path_id);
    let progress_anchor = StringAnchor::new("agent_progress", &progress_id);
    let progress_anchor_hash = hash_entry(&EntryTypes::StringAnchor(progress_anchor))?;

    let query = LinkQuery::try_new(progress_anchor_hash.clone(), LinkTypes::AgentToProgress)?;
    let links = get_links(query, GetStrategy::default())?;

    let link = links.first().ok_or(wasm_error!(WasmErrorInner::Guest(
        "Progress not found. Create it first.".to_string()
    )))?;

    let action_hash = link.target.clone().into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid progress hash".to_string())))?;

    let record = get(action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Progress record not found".to_string())))?;

    let mut progress: AgentProgress = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not deserialize progress".to_string()
        )))?;

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Apply updates
    if let Some(idx) = input.current_step_index {
        progress.current_step_index = idx;
    }
    if let Some(idx) = input.completed_step_index {
        if !progress.completed_step_indices.contains(&idx) {
            progress.completed_step_indices.push(idx);
        }
    }
    if let Some(content_id) = input.completed_content_id {
        if !progress.completed_content_ids.contains(&content_id) {
            progress.completed_content_ids.push(content_id);
        }
    }

    progress.last_activity_at = timestamp;

    // Create new entry (immutable DHT pattern)
    let new_action_hash = create_entry(&EntryTypes::AgentProgress(progress.clone()))?;

    // Update link
    delete_link(link.create_link_hash.clone(), GetOptions::default())?;
    create_link(progress_anchor_hash, new_action_hash.clone(), LinkTypes::AgentToProgress, ())?;

    Ok(AgentProgressOutput {
        action_hash: new_action_hash,
        progress,
    })
}

// =============================================================================
// Content Mastery Functions
// =============================================================================

/// Output from mastery operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentMasteryOutput {
    pub action_hash: ActionHash,
    pub mastery: ContentMastery,
}

/// Input for upserting mastery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertMasteryInput {
    pub human_id: String,
    pub content_id: String,
    pub mastery_level: String,
    pub engagement_type: String,
}

/// Helper to get mastery level index
fn get_mastery_level_index(level: &str) -> u32 {
    match level {
        "not_started" => 0,
        "aware" => 1,
        "remember" => 2,
        "understand" => 3,
        "apply" => 4,
        "analyze" => 5,
        "evaluate" => 6,
        "create" => 7,
        _ => 0,
    }
}

/// Upsert content mastery (create or update)
#[hdk_extern]
pub fn upsert_mastery(input: UpsertMasteryInput) -> ExternResult<ContentMasteryOutput> {
    let mastery_id = format!("{}-{}", input.human_id, input.content_id);
    let mastery_anchor = StringAnchor::new("mastery", &mastery_id);
    let mastery_anchor_hash = hash_entry(&EntryTypes::StringAnchor(mastery_anchor))?;

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);
    let level_index = get_mastery_level_index(&input.mastery_level);

    // Check if mastery exists
    let query = LinkQuery::try_new(mastery_anchor_hash.clone(), LinkTypes::HumanToMastery)?;
    let links = get_links(query, GetStrategy::default())?;

    let mastery = if let Some(link) = links.first() {
        // Update existing
        let action_hash = link.target.clone().into_action_hash()
            .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid mastery hash".to_string())))?;

        let record = get(action_hash, GetOptions::default())?
            .ok_or(wasm_error!(WasmErrorInner::Guest("Mastery record not found".to_string())))?;

        let mut existing: ContentMastery = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(e))?
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Could not deserialize mastery".to_string()
            )))?;

        // Update fields
        existing.mastery_level = input.mastery_level;
        existing.mastery_level_index = level_index;
        existing.engagement_count += 1;
        existing.last_engagement_type = input.engagement_type;
        existing.last_engagement_at = timestamp.clone();
        existing.updated_at = timestamp;

        existing
    } else {
        // Create new
        ContentMastery {
            id: mastery_id,
            human_id: input.human_id.clone(),
            content_id: input.content_id.clone(),
            mastery_level: input.mastery_level,
            mastery_level_index: level_index,
            freshness_score: 1.0,
            needs_refresh: false,
            engagement_count: 1,
            last_engagement_type: input.engagement_type,
            last_engagement_at: timestamp.clone(),
            level_achieved_at: timestamp.clone(),
            content_version_at_mastery: None,
            assessment_evidence_json: "[]".to_string(),
            privileges_json: "[]".to_string(),
            created_at: timestamp.clone(),
            updated_at: timestamp,
            schema_version: 1,
            validation_status: "valid".to_string(),
        }
    };

    let action_hash = create_entry(&EntryTypes::ContentMastery(mastery.clone()))?;

    // Update link
    if let Some(link) = links.first() {
        delete_link(link.create_link_hash.clone(), GetOptions::default())?;
    }
    create_link(mastery_anchor_hash, action_hash.clone(), LinkTypes::HumanToMastery, ())?;

    Ok(ContentMasteryOutput { action_hash, mastery })
}

/// Get mastery for a specific content
#[hdk_extern]
pub fn get_mastery(input: UpsertMasteryInput) -> ExternResult<Option<ContentMasteryOutput>> {
    let mastery_id = format!("{}-{}", input.human_id, input.content_id);
    let mastery_anchor = StringAnchor::new("mastery", &mastery_id);
    let mastery_anchor_hash = hash_entry(&EntryTypes::StringAnchor(mastery_anchor))?;

    let query = LinkQuery::try_new(mastery_anchor_hash, LinkTypes::HumanToMastery)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(mastery) = record.entry().to_app_option::<ContentMastery>().ok().flatten() {
                    return Ok(Some(ContentMasteryOutput { action_hash, mastery }));
                }
            }
        }
    }

    Ok(None)
}

/// Get my mastery for a content (using calling agent's human profile)
#[hdk_extern]
pub fn get_my_mastery(content_id: String) -> ExternResult<Option<ContentMasteryOutput>> {
    let my_human = get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    get_mastery(UpsertMasteryInput {
        human_id: my_human.human.id,
        content_id,
        mastery_level: String::new(),
        engagement_type: String::new(),
    })
}

/// Get all mastery records for calling agent
#[hdk_extern]
pub fn get_my_all_mastery(_: ()) -> ExternResult<Vec<ContentMasteryOutput>> {
    let my_human = get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    let anchor = StringAnchor::new("human_mastery", &my_human.human.id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::HumanToMastery)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(mastery) = record.entry().to_app_option::<ContentMastery>().ok().flatten() {
                    results.push(ContentMasteryOutput { action_hash, mastery });
                }
            }
        }
    }

    Ok(results)
}

// =============================================================================
// ContributorPresence Functions
// =============================================================================

/// Output from presence operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceOutput {
    pub action_hash: ActionHash,
    pub presence: ContributorPresence,
}

/// Input for creating a contributor presence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePresenceInput {
    pub display_name: String,
    pub external_identifiers_json: Option<String>,
    pub establishing_content_ids_json: Option<String>,
    pub note: Option<String>,
    pub image: Option<String>,
    pub metadata_json: Option<String>,
}

/// Input for beginning stewardship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeginStewardshipInput {
    pub presence_id: String,
    pub steward_agent_id: String,
    pub commitment_note: Option<String>,
}

/// Input for initiating a claim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiateClaimInput {
    pub presence_id: String,
    pub claim_evidence_json: String,
    pub verification_method: String,
}

/// Create a new contributor presence (for absent contributors)
#[hdk_extern]
pub fn create_contributor_presence(input: CreatePresenceInput) -> ExternResult<PresenceOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Generate unique ID
    let presence_id = format!("presence-{}", timestamp.replace([':', ' ', '(', ')'], "-"));

    let presence = ContributorPresence {
        id: presence_id.clone(),
        display_name: input.display_name,
        presence_state: "unclaimed".to_string(),
        external_identifiers_json: input.external_identifiers_json.unwrap_or_else(|| "[]".to_string()),
        establishing_content_ids_json: input.establishing_content_ids_json.unwrap_or_else(|| "[]".to_string()),
        established_at: timestamp.clone(),
        affinity_total: 0,
        unique_engagers: 0,
        citation_count: 0,
        endorsements_json: "[]".to_string(),
        recognition_score: 0.0,
        recognition_by_content_json: "{}".to_string(),
        accumulating_since: timestamp.clone(),
        last_recognition_at: timestamp.clone(),
        steward_id: None,
        stewardship_started_at: None,
        stewardship_commitment_id: None,
        stewardship_quality_score: None,
        claim_initiated_at: None,
        claim_verified_at: None,
        claim_verification_method: None,
        claim_evidence_json: None,
        claimed_agent_id: None,
        claim_recognition_transferred_value: None,
        claim_recognition_transferred_unit: None,
        claim_facilitated_by: None,
        invitations_json: "[]".to_string(),
        note: input.note,
        image: input.image,
        metadata_json: input.metadata_json.unwrap_or_else(|| "{}".to_string()),
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::ContributorPresence(presence.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("presence_id", &presence_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToPresence, ())?;

    // Create state link
    let state_anchor = StringAnchor::new("presence_state", "unclaimed");
    let state_anchor_hash = hash_entry(&EntryTypes::StringAnchor(state_anchor))?;
    create_link(state_anchor_hash, action_hash.clone(), LinkTypes::PresenceByState, ())?;

    Ok(PresenceOutput { action_hash, presence })
}

/// Begin stewardship of an unclaimed presence
#[hdk_extern]
pub fn begin_stewardship(input: BeginStewardshipInput) -> ExternResult<PresenceOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Get the existing presence
    let existing = get_contributor_presence_by_id(input.presence_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Presence not found".to_string())))?;

    if existing.presence.presence_state != "unclaimed" {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Can only begin stewardship of unclaimed presences".to_string()
        )));
    }

    // Update presence with stewardship info
    let mut updated = existing.presence.clone();
    updated.presence_state = "stewarded".to_string();
    updated.steward_id = Some(input.steward_agent_id.clone());
    updated.stewardship_started_at = Some(timestamp.clone());
    updated.updated_at = timestamp;

    // Create new entry
    let action_hash = create_entry(&EntryTypes::ContributorPresence(updated.clone()))?;

    // Update ID link to point to new entry
    let id_anchor = StringAnchor::new("presence_id", &input.presence_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    // Delete old link and create new
    let old_links = get_links(
        LinkQuery::try_new(id_anchor_hash.clone(), LinkTypes::IdToPresence)?,
        GetStrategy::default(),
    )?;
    for link in old_links {
        delete_link(link.create_link_hash, GetOptions::default())?;
    }
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToPresence, ())?;

    // Update state links
    let old_state_anchor = StringAnchor::new("presence_state", "unclaimed");
    let old_state_anchor_hash = hash_entry(&EntryTypes::StringAnchor(old_state_anchor))?;
    let old_state_links = get_links(
        LinkQuery::try_new(old_state_anchor_hash, LinkTypes::PresenceByState)?,
        GetStrategy::default(),
    )?;
    for link in old_state_links {
        if link.target == existing.action_hash.clone().into() {
            delete_link(link.create_link_hash, GetOptions::default())?;
        }
    }

    let new_state_anchor = StringAnchor::new("presence_state", "stewarded");
    let new_state_anchor_hash = hash_entry(&EntryTypes::StringAnchor(new_state_anchor))?;
    create_link(new_state_anchor_hash, action_hash.clone(), LinkTypes::PresenceByState, ())?;

    // Create steward link
    let steward_anchor = StringAnchor::new("steward_presences", &input.steward_agent_id);
    let steward_anchor_hash = hash_entry(&EntryTypes::StringAnchor(steward_anchor))?;
    create_link(steward_anchor_hash, action_hash.clone(), LinkTypes::StewardToPresence, ())?;

    Ok(PresenceOutput { action_hash, presence: updated })
}

/// Get presences by steward ID
#[hdk_extern]
pub fn get_presences_by_steward(steward_agent_id: String) -> ExternResult<Vec<PresenceOutput>> {
    let steward_anchor = StringAnchor::new("steward_presences", &steward_agent_id);
    let steward_anchor_hash = hash_entry(&EntryTypes::StringAnchor(steward_anchor))?;

    let query = LinkQuery::try_new(steward_anchor_hash, LinkTypes::StewardToPresence)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(presence) = record.entry().to_app_option::<ContributorPresence>().ok().flatten() {
                    results.push(PresenceOutput { action_hash, presence });
                }
            }
        }
    }

    Ok(results)
}

/// Initiate a claim on a presence
#[hdk_extern]
pub fn initiate_claim(input: InitiateClaimInput) -> ExternResult<PresenceOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);
    let agent_info = agent_info()?;

    // Get the existing presence
    let existing = get_contributor_presence_by_id(input.presence_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Presence not found".to_string())))?;

    if existing.presence.presence_state == "claimed" {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Presence is already claimed".to_string()
        )));
    }

    // Update presence with claim info
    let mut updated = existing.presence.clone();
    updated.claim_initiated_at = Some(timestamp.clone());
    updated.claim_evidence_json = Some(input.claim_evidence_json);
    updated.claim_verification_method = Some(input.verification_method);
    updated.claimed_agent_id = Some(agent_info.agent_initial_pubkey.to_string());
    updated.updated_at = timestamp;

    // Create new entry
    let action_hash = create_entry(&EntryTypes::ContributorPresence(updated.clone()))?;

    // Update ID link
    let id_anchor = StringAnchor::new("presence_id", &input.presence_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    let old_links = get_links(
        LinkQuery::try_new(id_anchor_hash.clone(), LinkTypes::IdToPresence)?,
        GetStrategy::default(),
    )?;
    for link in old_links {
        delete_link(link.create_link_hash, GetOptions::default())?;
    }
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToPresence, ())?;

    Ok(PresenceOutput { action_hash, presence: updated })
}

/// Verify and complete a claim
#[hdk_extern]
pub fn verify_claim(presence_id: String) -> ExternResult<PresenceOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);
    let agent_info = agent_info()?;

    // Get the existing presence
    let existing = get_contributor_presence_by_id(presence_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Presence not found".to_string())))?;

    if existing.presence.claimed_agent_id.is_none() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Claim must be initiated first".to_string()
        )));
    }

    if existing.presence.presence_state == "claimed" {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Presence is already claimed".to_string()
        )));
    }

    let old_state = existing.presence.presence_state.clone();

    // Update presence to claimed
    let mut updated = existing.presence.clone();
    updated.presence_state = "claimed".to_string();
    updated.claim_verified_at = Some(timestamp.clone());
    updated.claim_facilitated_by = Some(agent_info.agent_initial_pubkey.to_string());
    updated.updated_at = timestamp;

    // Create new entry
    let action_hash = create_entry(&EntryTypes::ContributorPresence(updated.clone()))?;

    // Update ID link
    let id_anchor = StringAnchor::new("presence_id", &presence_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    let old_links = get_links(
        LinkQuery::try_new(id_anchor_hash.clone(), LinkTypes::IdToPresence)?,
        GetStrategy::default(),
    )?;
    for link in old_links {
        delete_link(link.create_link_hash, GetOptions::default())?;
    }
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToPresence, ())?;

    // Update state links
    let old_state_anchor = StringAnchor::new("presence_state", &old_state);
    let old_state_anchor_hash = hash_entry(&EntryTypes::StringAnchor(old_state_anchor))?;
    let old_state_links = get_links(
        LinkQuery::try_new(old_state_anchor_hash, LinkTypes::PresenceByState)?,
        GetStrategy::default(),
    )?;
    for link in old_state_links {
        if link.target == existing.action_hash.clone().into() {
            delete_link(link.create_link_hash, GetOptions::default())?;
        }
    }

    let new_state_anchor = StringAnchor::new("presence_state", "claimed");
    let new_state_anchor_hash = hash_entry(&EntryTypes::StringAnchor(new_state_anchor))?;
    create_link(new_state_anchor_hash, action_hash.clone(), LinkTypes::PresenceByState, ())?;

    // Create claimed agent link
    if let Some(ref claimed_agent) = updated.claimed_agent_id {
        let agent_anchor = StringAnchor::new("claimed_agent_presence", claimed_agent);
        let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;
        create_link(agent_anchor_hash, action_hash.clone(), LinkTypes::ClaimedAgentToPresence, ())?;
    }

    Ok(PresenceOutput { action_hash, presence: updated })
}

/// Get contributor presence by ID
#[hdk_extern]
pub fn get_contributor_presence_by_id(id: String) -> ExternResult<Option<PresenceOutput>> {
    let id_anchor = StringAnchor::new("presence_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToPresence)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(presence) = record.entry().to_app_option::<ContributorPresence>().ok().flatten() {
                    return Ok(Some(PresenceOutput { action_hash, presence }));
                }
            }
        }
    }

    Ok(None)
}

/// Get presences by state
#[hdk_extern]
pub fn get_presences_by_state(state: String) -> ExternResult<Vec<PresenceOutput>> {
    let state_anchor = StringAnchor::new("presence_state", &state);
    let state_anchor_hash = hash_entry(&EntryTypes::StringAnchor(state_anchor))?;

    let query = LinkQuery::try_new(state_anchor_hash, LinkTypes::PresenceByState)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(presence) = record.entry().to_app_option::<ContributorPresence>().ok().flatten() {
                    results.push(PresenceOutput { action_hash, presence });
                }
            }
        }
    }

    Ok(results)
}

// =============================================================================
// Init
// =============================================================================

#[hdk_extern]
pub fn init(_: ()) -> ExternResult<InitCallbackResult> {
    Ok(InitCallbackResult::Pass)
}
