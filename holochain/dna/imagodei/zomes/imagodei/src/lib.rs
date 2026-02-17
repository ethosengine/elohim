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
use std::time::Duration;

// Stewardship coordinator functions
pub mod stewardship;
pub use stewardship::*;

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
// Recovery Types
// =============================================================================

/// Output from recovery request operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryRequestOutput {
    pub action_hash: ActionHash,
    pub request: RecoveryRequest,
}

/// Output from recovery vote operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryVoteOutput {
    pub action_hash: ActionHash,
    pub vote: RecoveryVote,
}

/// Output from recovery hint operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryHintOutput {
    pub action_hash: ActionHash,
    pub hint: RecoveryHint,
}

/// Input for creating a recovery request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRecoveryRequestInput {
    pub human_id: String,
    pub doorway_id: String,
    pub recovery_method: String,
    pub expires_in_hours: Option<u32>,
}

/// Input for voting on a recovery request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteOnRecoveryInput {
    pub request_id: String,
    pub approved: bool,
    pub attestation: String,
    pub verification_method: String,
}

/// Input for creating/updating a recovery hint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertRecoveryHintInput {
    pub hint_type: String,
    pub encrypted_data: String,
    pub encryption_nonce: String,
}

/// Input for Elohim verification score update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateElohimScoreInput {
    pub request_id: String,
    pub questions_json: String,
    pub score: f64,
}

/// Recovery signals for real-time notification
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum RecoverySignal {
    RecoveryRequested {
        action_hash: ActionHash,
        request: RecoveryRequest,
        eligible_voters: Vec<String>,
    },
    RecoveryVoteCast {
        action_hash: ActionHash,
        vote: RecoveryVote,
        request_id: String,
        current_approvals: u32,
        required_approvals: u32,
    },
    RecoveryApproved {
        action_hash: ActionHash,
        request: RecoveryRequest,
    },
    RecoveryRejected {
        action_hash: ActionHash,
        request_id: String,
        reason: String,
    },
}

// =============================================================================
// Recovery Functions
// =============================================================================

/// Helper to calculate confidence weight based on intimacy level
fn get_intimacy_weight(intimacy_level: &str) -> f64 {
    match intimacy_level {
        "intimate" => 0.25,   // Family counts 25% each
        "trusted" => 0.20,   // Close friends 20%
        "familiar" => 0.15,  // Acquaintances 15%
        "acquainted" => 0.10,
        "public" => 0.05,
        _ => 0.05,
    }
}

/// Helper to calculate required approvals based on relationships
fn calculate_required_approvals(relationships: &[HumanRelationship]) -> u32 {
    let weighted_count: f64 = relationships
        .iter()
        .filter(|r| r.emergency_access_enabled)
        .map(|r| match r.intimacy_level.as_str() {
            "intimate" => 2.0,
            "trusted" => 1.5,
            "familiar" => 1.0,
            _ => 0.5,
        })
        .sum();

    // M = ceil(weighted / 3), minimum 2
    let m = (weighted_count / 3.0).ceil() as u32;
    m.max(2)
}

/// Create a recovery request
#[hdk_extern]
pub fn create_recovery_request(input: CreateRecoveryRequestInput) -> ExternResult<RecoveryRequestOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Calculate expiry (default 48 hours)
    let hours = input.expires_in_hours.unwrap_or(48);
    let expiry_ms = hours as u64 * 60 * 60 * 1000 * 1000; // microseconds
    let expires_at = format!("{:?}", now.checked_add(&Duration::from_micros(expiry_ms)).unwrap_or(now));

    // Get relationships with emergency_access_enabled to determine M
    let relationships = get_emergency_access_relationships(&input.human_id)?;
    let required_approvals = calculate_required_approvals(&relationships);

    let request_id = format!("recovery-{}-{}", input.human_id, timestamp.replace([':', ' ', '(', ')'], "-"));

    let request = RecoveryRequest {
        id: request_id.clone(),
        human_id: input.human_id.clone(),
        doorway_id: input.doorway_id,
        recovery_method: input.recovery_method,
        status: "pending".to_string(),
        required_approvals,
        current_approvals: 0,
        confidence_score: 0.0,
        elohim_questions_json: None,
        elohim_score: None,
        elohim_verified_at: None,
        requested_at: timestamp.clone(),
        expires_at,
        approved_at: None,
        completed_at: None,
    };

    let action_hash = create_entry(&EntryTypes::RecoveryRequest(request.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("recovery_request_id", &request_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToRecoveryRequest, ())?;

    // Create human lookup link
    let human_anchor = StringAnchor::new("human_recovery_requests", &input.human_id);
    let human_anchor_hash = hash_entry(&EntryTypes::StringAnchor(human_anchor))?;
    create_link(human_anchor_hash, action_hash.clone(), LinkTypes::HumanToRecoveryRequest, ())?;

    // Create status link
    let status_anchor = StringAnchor::new("recovery_status", "pending");
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::RecoveryRequestByStatus, ())?;

    // Create pending vote links for each eligible voter
    let eligible_voters: Vec<String> = relationships
        .iter()
        .filter(|r| r.emergency_access_enabled)
        .flat_map(|r| {
            if r.party_a_id == input.human_id {
                vec![r.party_b_id.clone()]
            } else {
                vec![r.party_a_id.clone()]
            }
        })
        .collect();

    for voter_id in &eligible_voters {
        let voter_anchor = StringAnchor::new("pending_recovery_votes", voter_id);
        let voter_anchor_hash = hash_entry(&EntryTypes::StringAnchor(voter_anchor))?;
        create_link(
            voter_anchor_hash,
            action_hash.clone(),
            LinkTypes::PendingRecoveryVote,
            request_id.as_bytes().to_vec(),
        )?;
    }

    // Emit signal for real-time notification
    emit_signal(RecoverySignal::RecoveryRequested {
        action_hash: action_hash.clone(),
        request: request.clone(),
        eligible_voters,
    })?;

    Ok(RecoveryRequestOutput { action_hash, request })
}

/// Helper to get relationships with emergency_access_enabled for a human
fn get_emergency_access_relationships(human_id: &str) -> ExternResult<Vec<HumanRelationship>> {
    let anchor = StringAnchor::new("agent_relationships", human_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::AgentToRelationship)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(relationship) = record
                    .entry()
                    .to_app_option::<HumanRelationship>()
                    .ok()
                    .flatten()
                {
                    if relationship.emergency_access_enabled {
                        results.push(relationship);
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Vote on a recovery request
#[hdk_extern]
pub fn vote_on_recovery(input: VoteOnRecoveryInput) -> ExternResult<RecoveryVoteOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Get my human profile
    let my_human = get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile to vote".to_string())))?;

    // Get the recovery request
    let request_output = get_recovery_request_by_id(input.request_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Recovery request not found".to_string())))?;

    if request_output.request.status != "pending" {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Can only vote on pending recovery requests".to_string()
        )));
    }

    // Verify voter has emergency_access_enabled relationship with requestor
    let relationships = get_emergency_access_relationships(&request_output.request.human_id)?;
    let voter_relationship = relationships
        .iter()
        .find(|r| r.party_a_id == my_human.human.id || r.party_b_id == my_human.human.id);

    let relationship = voter_relationship
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            "You do not have emergency access enabled with this human".to_string()
        )))?;

    let confidence_weight = get_intimacy_weight(&relationship.intimacy_level);

    let vote_id = format!("{}-{}", input.request_id, my_human.human.id);

    let vote = RecoveryVote {
        id: vote_id.clone(),
        request_id: input.request_id.clone(),
        voter_human_id: my_human.human.id.clone(),
        approved: input.approved,
        attestation: input.attestation,
        intimacy_level: relationship.intimacy_level.clone(),
        confidence_weight,
        verification_method: input.verification_method,
        voted_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::RecoveryVote(vote.clone()))?;

    // Link vote to request
    create_link(
        request_output.action_hash.clone(),
        action_hash.clone(),
        LinkTypes::RecoveryVoteToRequest,
        my_human.human.id.as_bytes().to_vec(),
    )?;

    // Remove pending vote link for this voter
    let voter_anchor = StringAnchor::new("pending_recovery_votes", &my_human.human.id);
    let voter_anchor_hash = hash_entry(&EntryTypes::StringAnchor(voter_anchor))?;
    let pending_links = get_links(
        LinkQuery::try_new(voter_anchor_hash, LinkTypes::PendingRecoveryVote)?,
        GetStrategy::default(),
    )?;
    for link in pending_links {
        if link.target == request_output.action_hash.clone().into() {
            delete_link(link.create_link_hash, GetOptions::default())?;
        }
    }

    // Update request with new approval count and check threshold
    let all_votes = get_recovery_votes(input.request_id.clone())?;
    let current_approvals = all_votes.iter().filter(|v| v.vote.approved).count() as u32;
    let total_confidence: f64 = all_votes
        .iter()
        .filter(|v| v.vote.approved)
        .map(|v| v.vote.confidence_weight)
        .sum();

    // Emit vote signal
    emit_signal(RecoverySignal::RecoveryVoteCast {
        action_hash: action_hash.clone(),
        vote: vote.clone(),
        request_id: input.request_id.clone(),
        current_approvals,
        required_approvals: request_output.request.required_approvals,
    })?;

    // Check if threshold reached
    if current_approvals >= request_output.request.required_approvals {
        // Update request status to approved
        let mut updated_request = request_output.request.clone();
        updated_request.status = "approved".to_string();
        updated_request.current_approvals = current_approvals;
        updated_request.confidence_score = total_confidence.min(1.0);
        updated_request.approved_at = Some(format!("{:?}", sys_time()?));

        let new_request_hash = create_entry(&EntryTypes::RecoveryRequest(updated_request.clone()))?;

        // Update status links
        let old_status_anchor = StringAnchor::new("recovery_status", "pending");
        let old_status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(old_status_anchor))?;
        let old_links = get_links(
            LinkQuery::try_new(old_status_anchor_hash, LinkTypes::RecoveryRequestByStatus)?,
            GetStrategy::default(),
        )?;
        for link in old_links {
            if link.target == request_output.action_hash.clone().into() {
                delete_link(link.create_link_hash, GetOptions::default())?;
            }
        }

        let new_status_anchor = StringAnchor::new("recovery_status", "approved");
        let new_status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(new_status_anchor))?;
        create_link(new_status_anchor_hash, new_request_hash.clone(), LinkTypes::RecoveryRequestByStatus, ())?;

        // Emit approval signal
        emit_signal(RecoverySignal::RecoveryApproved {
            action_hash: new_request_hash,
            request: updated_request,
        })?;
    }

    Ok(RecoveryVoteOutput { action_hash, vote })
}

/// Get recovery request by ID
#[hdk_extern]
pub fn get_recovery_request_by_id(id: String) -> ExternResult<Option<RecoveryRequestOutput>> {
    let id_anchor = StringAnchor::new("recovery_request_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToRecoveryRequest)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(request) = record.entry().to_app_option::<RecoveryRequest>().ok().flatten() {
                    return Ok(Some(RecoveryRequestOutput { action_hash, request }));
                }
            }
        }
    }

    Ok(None)
}

/// Get votes for a recovery request
#[hdk_extern]
pub fn get_recovery_votes(request_id: String) -> ExternResult<Vec<RecoveryVoteOutput>> {
    let request_output = get_recovery_request_by_id(request_id)?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Recovery request not found".to_string())))?;

    let query = LinkQuery::try_new(request_output.action_hash, LinkTypes::RecoveryVoteToRequest)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(vote) = record.entry().to_app_option::<RecoveryVote>().ok().flatten() {
                    results.push(RecoveryVoteOutput { action_hash, vote });
                }
            }
        }
    }

    Ok(results)
}

/// Get pending recovery votes for the calling agent
#[hdk_extern]
pub fn get_my_pending_recovery_votes(_: ()) -> ExternResult<Vec<RecoveryRequestOutput>> {
    let my_human = get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    let voter_anchor = StringAnchor::new("pending_recovery_votes", &my_human.human.id);
    let voter_anchor_hash = hash_entry(&EntryTypes::StringAnchor(voter_anchor))?;

    let query = LinkQuery::try_new(voter_anchor_hash, LinkTypes::PendingRecoveryVote)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(request) = record.entry().to_app_option::<RecoveryRequest>().ok().flatten() {
                    results.push(RecoveryRequestOutput { action_hash, request });
                }
            }
        }
    }

    Ok(results)
}

/// Update recovery request with Elohim verification score
#[hdk_extern]
pub fn update_elohim_score(input: UpdateElohimScoreInput) -> ExternResult<RecoveryRequestOutput> {
    let request_output = get_recovery_request_by_id(input.request_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Recovery request not found".to_string())))?;

    if request_output.request.status != "pending" {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Can only update pending recovery requests".to_string()
        )));
    }

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let mut updated_request = request_output.request.clone();
    updated_request.elohim_questions_json = Some(input.questions_json);
    updated_request.elohim_score = Some(input.score);
    updated_request.elohim_verified_at = Some(timestamp);

    // Add Elohim score to confidence (max 60% from Elohim)
    let elohim_confidence = input.score * 0.6;
    updated_request.confidence_score = (updated_request.confidence_score + elohim_confidence).min(1.0);

    // Check if confidence threshold reached (80%)
    if updated_request.confidence_score >= 0.8 {
        updated_request.status = "approved".to_string();
        updated_request.approved_at = Some(format!("{:?}", sys_time()?));
    }

    let action_hash = create_entry(&EntryTypes::RecoveryRequest(updated_request.clone()))?;

    // Update ID link
    let id_anchor = StringAnchor::new("recovery_request_id", &input.request_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    let old_links = get_links(
        LinkQuery::try_new(id_anchor_hash.clone(), LinkTypes::IdToRecoveryRequest)?,
        GetStrategy::default(),
    )?;
    for link in old_links {
        delete_link(link.create_link_hash, GetOptions::default())?;
    }
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToRecoveryRequest, ())?;

    if updated_request.status == "approved" {
        emit_signal(RecoverySignal::RecoveryApproved {
            action_hash: action_hash.clone(),
            request: updated_request.clone(),
        })?;
    }

    Ok(RecoveryRequestOutput { action_hash, request: updated_request })
}

/// Create or update a recovery hint for the calling agent
#[hdk_extern]
pub fn upsert_recovery_hint(input: UpsertRecoveryHintInput) -> ExternResult<RecoveryHintOutput> {
    let my_human = get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let hint_id = format!("{}-{}", my_human.human.id, input.hint_type);
    let hint_anchor = StringAnchor::new("recovery_hint", &hint_id);
    let hint_anchor_hash = hash_entry(&EntryTypes::StringAnchor(hint_anchor))?;

    // Check if hint exists
    let query = LinkQuery::try_new(hint_anchor_hash.clone(), LinkTypes::HumanToRecoveryHint)?;
    let links = get_links(query, GetStrategy::default())?;

    let version = if let Some(link) = links.first() {
        // Get existing version
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(existing) = record.entry().to_app_option::<RecoveryHint>().ok().flatten() {
                    existing.version + 1
                } else {
                    1
                }
            } else {
                1
            }
        } else {
            1
        }
    } else {
        1
    };

    let hint = RecoveryHint {
        id: hint_id.clone(),
        human_id: my_human.human.id.clone(),
        hint_type: input.hint_type.clone(),
        encrypted_data: input.encrypted_data,
        encryption_nonce: input.encryption_nonce,
        version,
        created_at: if version == 1 { timestamp.clone() } else { "".to_string() },
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::RecoveryHint(hint.clone()))?;

    // Update links
    for link in links {
        delete_link(link.create_link_hash, GetOptions::default())?;
    }
    create_link(hint_anchor_hash, action_hash.clone(), LinkTypes::HumanToRecoveryHint, ())?;

    // Create type lookup link
    let type_anchor = StringAnchor::new("recovery_hint_type", &input.hint_type);
    let type_anchor_hash = hash_entry(&EntryTypes::StringAnchor(type_anchor))?;
    create_link(type_anchor_hash, action_hash.clone(), LinkTypes::RecoveryHintByType, ())?;

    Ok(RecoveryHintOutput { action_hash, hint })
}

/// Get recovery hints for the calling agent
#[hdk_extern]
pub fn get_my_recovery_hints(_: ()) -> ExternResult<Vec<RecoveryHintOutput>> {
    let my_human = get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    let mut results = Vec::new();

    // Check each hint type
    for hint_type in &["password_hint", "security_qa", "trusted_doorways", "trusted_contacts"] {
        let hint_id = format!("{}-{}", my_human.human.id, hint_type);
        let hint_anchor = StringAnchor::new("recovery_hint", &hint_id);
        let hint_anchor_hash = hash_entry(&EntryTypes::StringAnchor(hint_anchor))?;

        let query = LinkQuery::try_new(hint_anchor_hash, LinkTypes::HumanToRecoveryHint)?;
        let links = get_links(query, GetStrategy::default())?;

        if let Some(link) = links.first() {
            if let Some(action_hash) = link.target.clone().into_action_hash() {
                if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                    if let Some(hint) = record.entry().to_app_option::<RecoveryHint>().ok().flatten() {
                        results.push(RecoveryHintOutput { action_hash, hint });
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Mark recovery as completed (called by doorway after successful re-custody)
#[hdk_extern]
pub fn complete_recovery(request_id: String) -> ExternResult<RecoveryRequestOutput> {
    let request_output = get_recovery_request_by_id(request_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Recovery request not found".to_string())))?;

    if request_output.request.status != "approved" {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Can only complete approved recovery requests".to_string()
        )));
    }

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let mut updated_request = request_output.request.clone();
    updated_request.status = "completed".to_string();
    updated_request.completed_at = Some(timestamp);

    let action_hash = create_entry(&EntryTypes::RecoveryRequest(updated_request.clone()))?;

    // Update links
    let id_anchor = StringAnchor::new("recovery_request_id", &request_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    let old_links = get_links(
        LinkQuery::try_new(id_anchor_hash.clone(), LinkTypes::IdToRecoveryRequest)?,
        GetStrategy::default(),
    )?;
    for link in old_links {
        delete_link(link.create_link_hash, GetOptions::default())?;
    }
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToRecoveryRequest, ())?;

    // Update status link
    let old_status_anchor = StringAnchor::new("recovery_status", "approved");
    let old_status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(old_status_anchor))?;
    let old_status_links = get_links(
        LinkQuery::try_new(old_status_anchor_hash, LinkTypes::RecoveryRequestByStatus)?,
        GetStrategy::default(),
    )?;
    for link in old_status_links {
        if link.target == request_output.action_hash.clone().into() {
            delete_link(link.create_link_hash, GetOptions::default())?;
        }
    }

    let new_status_anchor = StringAnchor::new("recovery_status", "completed");
    let new_status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(new_status_anchor))?;
    create_link(new_status_anchor_hash, action_hash.clone(), LinkTypes::RecoveryRequestByStatus, ())?;

    Ok(RecoveryRequestOutput { action_hash, request: updated_request })
}

// =============================================================================
// Renewal Protocol Types
// =============================================================================

/// Output from renewal attestation operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenewalAttestationOutput {
    pub action_hash: ActionHash,
    pub entry: RenewalAttestation,
}

/// Output from agent retirement operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRetirementOutput {
    pub action_hash: ActionHash,
    pub entry: AgentRetirement,
}

/// Output from relationship renewal operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipRenewalOutput {
    pub action_hash: ActionHash,
    pub entry: RelationshipRenewal,
}

/// Input for creating a renewal attestation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRenewalAttestationInput {
    pub human_id: String,
    pub old_agent_key: String,
    pub new_agent_key: String,
    pub renewal_reason: String,
    pub doorway_id: Option<String>,
    pub recovery_request_id: Option<String>,
    pub required_approvals: u32,
    pub expires_in_hours: Option<u32>,
}

/// Input for creating an agent retirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentRetirementInput {
    pub human_id: String,
    pub retired_agent_key: String,
    pub renewed_into_agent_key: String,
    pub renewal_attestation_id: String,
    pub retirement_reason: String,
}

/// Input for creating a relationship renewal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRelationshipRenewalInput {
    pub original_relationship_id: String,
    pub renewal_attestation_id: String,
    pub human_id: String,
    pub new_agent_key: String,
    pub counterparty_id: String,
    pub counterparty_agent_key: String,
    pub relationship_type: String,
    pub intimacy_level: String,
    pub emergency_access_enabled: bool,
}

// =============================================================================
// Renewal Protocol Functions
// =============================================================================

/// Create a renewal attestation (initiates the social witness ceremony)
#[hdk_extern]
pub fn create_renewal_attestation(input: CreateRenewalAttestationInput) -> ExternResult<RenewalAttestationOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let hours = input.expires_in_hours.unwrap_or(72);
    let expiry_ms = hours as u64 * 60 * 60 * 1000 * 1000;
    let expires_at = format!("{:?}", now.checked_add(&Duration::from_micros(expiry_ms)).unwrap_or(now));

    let attestation_id = format!("renewal-{}-{}", input.human_id, timestamp.replace([':', ' ', '(', ')'], "-"));

    let attestation = RenewalAttestation {
        id: attestation_id.clone(),
        human_id: input.human_id.clone(),
        old_agent_key: input.old_agent_key,
        new_agent_key: input.new_agent_key,
        renewal_reason: input.renewal_reason,
        doorway_id: input.doorway_id,
        recovery_request_id: input.recovery_request_id,
        votes_json: "[]".to_string(),
        required_approvals: input.required_approvals,
        current_approvals: 0,
        confidence_score: 0.0,
        status: "pending".to_string(),
        witnessed_at: None,
        created_at: timestamp,
        expires_at,
    };

    let action_hash = create_entry(&EntryTypes::RenewalAttestation(attestation.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("renewal_id", &attestation_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToRenewalAttestation, ())?;

    // Create human lookup link
    let human_anchor = StringAnchor::new("human_renewals", &input.human_id);
    let human_anchor_hash = hash_entry(&EntryTypes::StringAnchor(human_anchor))?;
    create_link(human_anchor_hash, action_hash.clone(), LinkTypes::HumanToRenewalAttestation, ())?;

    // Create status link
    let status_anchor = StringAnchor::new("renewal_status", "pending");
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::RenewalAttestationByStatus, ())?;

    Ok(RenewalAttestationOutput { action_hash, entry: attestation })
}

/// Get a renewal attestation by ID
#[hdk_extern]
pub fn get_renewal_attestation_by_id(id: String) -> ExternResult<Option<RenewalAttestationOutput>> {
    let id_anchor = StringAnchor::new("renewal_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToRenewalAttestation)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(entry) = record.entry().to_app_option::<RenewalAttestation>().ok().flatten() {
                    return Ok(Some(RenewalAttestationOutput { action_hash, entry }));
                }
            }
        }
    }

    Ok(None)
}

/// Get all renewal attestations for a human
#[hdk_extern]
pub fn get_renewal_attestations_for_human(human_id: String) -> ExternResult<Vec<RenewalAttestationOutput>> {
    let human_anchor = StringAnchor::new("human_renewals", &human_id);
    let human_anchor_hash = hash_entry(&EntryTypes::StringAnchor(human_anchor))?;

    let query = LinkQuery::try_new(human_anchor_hash, LinkTypes::HumanToRenewalAttestation)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(entry) = record.entry().to_app_option::<RenewalAttestation>().ok().flatten() {
                    results.push(RenewalAttestationOutput { action_hash, entry });
                }
            }
        }
    }

    Ok(results)
}

/// Create an agent retirement (marks old key as superseded)
#[hdk_extern]
pub fn create_agent_retirement(input: CreateAgentRetirementInput) -> ExternResult<AgentRetirementOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let retirement_id = format!("retirement-{}-{}", input.retired_agent_key.chars().take(8).collect::<String>(),
        timestamp.replace([':', ' ', '(', ')'], "-"));

    let retirement = AgentRetirement {
        id: retirement_id.clone(),
        human_id: input.human_id,
        retired_agent_key: input.retired_agent_key.clone(),
        renewed_into_agent_key: input.renewed_into_agent_key.clone(),
        renewal_attestation_id: input.renewal_attestation_id,
        retirement_reason: input.retirement_reason,
        retired_at: timestamp.clone(),
        created_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::AgentRetirement(retirement.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("retirement_id", &retirement_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToAgentRetirement, ())?;

    // Create old agent → retirement link (for "who is this agent now?" queries)
    let old_agent_anchor = StringAnchor::new("retired_agent", &input.retired_agent_key);
    let old_agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(old_agent_anchor))?;
    create_link(old_agent_anchor_hash, action_hash.clone(), LinkTypes::OldAgentToRetirement, ())?;

    // Create new agent ← retirement link (for "where did this agent come from?" queries)
    let new_agent_anchor = StringAnchor::new("renewed_from", &input.renewed_into_agent_key);
    let new_agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(new_agent_anchor))?;
    create_link(new_agent_anchor_hash, action_hash.clone(), LinkTypes::NewAgentFromRetirement, ())?;

    Ok(AgentRetirementOutput { action_hash, entry: retirement })
}

/// Get retirement record for a specific agent key
#[hdk_extern]
pub fn get_retirement_for_agent(agent_key: String) -> ExternResult<Option<AgentRetirementOutput>> {
    let old_agent_anchor = StringAnchor::new("retired_agent", &agent_key);
    let old_agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(old_agent_anchor))?;

    let query = LinkQuery::try_new(old_agent_anchor_hash, LinkTypes::OldAgentToRetirement)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(entry) = record.entry().to_app_option::<AgentRetirement>().ok().flatten() {
                    return Ok(Some(AgentRetirementOutput { action_hash, entry }));
                }
            }
        }
    }

    Ok(None)
}

/// Follow the retirement chain: old_agent → retirement → new_agent → retirement → newer_agent
/// This is how queries resolve "who is this agent now?"
#[hdk_extern]
pub fn get_retirement_chain(agent_key: String) -> ExternResult<Vec<AgentRetirementOutput>> {
    let mut chain = Vec::new();
    let mut current_key = agent_key;

    // Safety limit to prevent infinite loops (max 100 retirements deep)
    for _ in 0..100 {
        match get_retirement_for_agent(current_key.clone())? {
            Some(retirement) => {
                current_key = retirement.entry.renewed_into_agent_key.clone();
                chain.push(retirement);
            }
            None => break,
        }
    }

    Ok(chain)
}

/// Create a relationship renewal (initiated by the renewed human)
#[hdk_extern]
pub fn create_relationship_renewal(input: CreateRelationshipRenewalInput) -> ExternResult<RelationshipRenewalOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let renewal_id = format!("rel-renewal-{}-{}", input.original_relationship_id,
        timestamp.replace([':', ' ', '(', ')'], "-"));

    let renewal = RelationshipRenewal {
        id: renewal_id.clone(),
        original_relationship_id: input.original_relationship_id.clone(),
        renewal_attestation_id: input.renewal_attestation_id,
        human_id: input.human_id,
        new_agent_key: input.new_agent_key,
        counterparty_id: input.counterparty_id,
        counterparty_agent_key: input.counterparty_agent_key,
        relationship_type: input.relationship_type,
        intimacy_level: input.intimacy_level,
        emergency_access_enabled: input.emergency_access_enabled,
        reaffirmed_by_counterparty: false,
        reaffirmed_at: None,
        created_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::RelationshipRenewal(renewal.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("rel_renewal_id", &renewal_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToRelationshipRenewal, ())?;

    // Create original relationship → renewal link
    let rel_anchor = StringAnchor::new("original_rel", &input.original_relationship_id);
    let rel_anchor_hash = hash_entry(&EntryTypes::StringAnchor(rel_anchor))?;
    create_link(rel_anchor_hash, action_hash.clone(), LinkTypes::OriginalRelToRenewal, ())?;

    Ok(RelationshipRenewalOutput { action_hash, entry: renewal })
}

/// Get all renewals for a specific original relationship
#[hdk_extern]
pub fn get_renewals_for_relationship(original_rel_id: String) -> ExternResult<Vec<RelationshipRenewalOutput>> {
    let rel_anchor = StringAnchor::new("original_rel", &original_rel_id);
    let rel_anchor_hash = hash_entry(&EntryTypes::StringAnchor(rel_anchor))?;

    let query = LinkQuery::try_new(rel_anchor_hash, LinkTypes::OriginalRelToRenewal)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(entry) = record.entry().to_app_option::<RelationshipRenewal>().ok().flatten() {
                    results.push(RelationshipRenewalOutput { action_hash, entry });
                }
            }
        }
    }

    Ok(results)
}

/// Counterparty reaffirms a relationship renewal (co-signs)
#[hdk_extern]
pub fn reaffirm_relationship_renewal(renewal_id: String) -> ExternResult<RelationshipRenewalOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Get existing renewal
    let id_anchor = StringAnchor::new("rel_renewal_id", &renewal_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash.clone(), LinkTypes::IdToRelationshipRenewal)?;
    let links = get_links(query, GetStrategy::default())?;

    let link = links.first().ok_or(wasm_error!(WasmErrorInner::Guest(
        "RelationshipRenewal not found".to_string()
    )))?;

    let old_action_hash = link.target.clone().into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid renewal hash".to_string())))?;

    let record = get(old_action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Renewal record not found".to_string())))?;

    let mut renewal: RelationshipRenewal = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not deserialize renewal".to_string()
        )))?;

    if renewal.reaffirmed_by_counterparty {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Relationship renewal already reaffirmed".to_string()
        )));
    }

    renewal.reaffirmed_by_counterparty = true;
    renewal.reaffirmed_at = Some(timestamp);

    let action_hash = create_entry(&EntryTypes::RelationshipRenewal(renewal.clone()))?;

    // Update ID link to point to new entry
    let old_links = get_links(
        LinkQuery::try_new(id_anchor_hash.clone(), LinkTypes::IdToRelationshipRenewal)?,
        GetStrategy::default(),
    )?;
    for link in old_links {
        delete_link(link.create_link_hash, GetOptions::default())?;
    }
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToRelationshipRenewal, ())?;

    Ok(RelationshipRenewalOutput { action_hash, entry: renewal })
}

// =============================================================================
// Init
// =============================================================================

#[hdk_extern]
pub fn init(_: ()) -> ExternResult<InitCallbackResult> {
    Ok(InitCallbackResult::Pass)
}
