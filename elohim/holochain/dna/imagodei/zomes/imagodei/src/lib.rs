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

// AgentPeerBinding coordinator functions (EPR Phase 2B, Task A.13)
pub mod agent_peer_binding;
pub use agent_peer_binding::*;

// PortalHost coordinator functions (Recovery Phase 2 M5)
pub mod portal_host;
pub use portal_host::*;

// Bootstrap-steward pattern — reference implementation for the protocol
// (also ported to mishpat, node-registry, lamad). See bootstrap_steward.rs.
pub mod bootstrap_steward;
pub use bootstrap_steward::{
    am_i_bootstrap_steward, bootstrap_steward, maybe_bootstrap_steward, BootstrapStewardError,
    DnaProperties,
};

// =============================================================================
// Input/Output Types
// =============================================================================

pub use imagodei_types::CreateHumanInput;

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

pub use imagodei_types::HumanOutput;

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
    /// Emitted by `create_agent_peer_binding` (Task A.13).
    /// Consumed by elohim-storage `HolochainAppSignalStream` (Task A.11)
    /// to project into the `peer_identity_bindings` SQLite table.
    ///
    /// NOTE: This variant intentionally omits `entry_hash` and `author` that
    /// other variants carry. The `action_hash` alone is sufficient — storage
    /// projects by calling `get(action_hash)` to fetch the full entry. Adding
    /// `entry_hash`/`author` here would duplicate data already in the action
    /// record and diverge from the minimal-signal pattern used in Task A.11.
    AgentPeerBindingCreated {
        action_hash: ActionHash,
        binding: AgentPeerBinding,
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
        } else if let Some(attestation) =
            record.entry().to_app_option::<Attestation>().ok().flatten()
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
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToHuman,
        (),
    )?;

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

    Ok(HumanOutput {
        action_hash,
        human: imagodei_types::Human {
            id: human.id,
            display_name: human.display_name,
            bio: human.bio,
            affinities: human.affinities,
            profile_reach: human.profile_reach,
            location: human.location,
            created_at: human.created_at,
            updated_at: human.updated_at,
        },
    })
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
                    return Ok(Some(HumanOutput {
                        action_hash,
                        human: imagodei_types::Human {
                            id: human.id,
                            display_name: human.display_name,
                            bio: human.bio,
                            affinities: human.affinities,
                            profile_reach: human.profile_reach,
                            location: human.location,
                            created_at: human.created_at,
                            updated_at: human.updated_at,
                        },
                    }));
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
                    return Ok(Some(HumanOutput {
                        action_hash,
                        human: imagodei_types::Human {
                            id: human.id,
                            display_name: human.display_name,
                            bio: human.bio,
                            affinities: human.affinities,
                            profile_reach: human.profile_reach,
                            location: human.location,
                            created_at: human.created_at,
                            updated_at: human.updated_at,
                        },
                    }));
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

    Ok(HumanOutput {
        action_hash,
        human: imagodei_types::Human {
            id: human.id,
            display_name: human.display_name,
            bio: human.bio,
            affinities: human.affinities,
            profile_reach: human.profile_reach,
            location: human.location,
            created_at: human.created_at,
            updated_at: human.updated_at,
        },
    })
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
    let my_human = get_my_human(())?.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "Must have Human profile first".to_string()
        ))
    })?;

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

    let attestation_id = format!(
        "{}-{}-{}",
        input.agent_id, input.attestation_type, timestamp
    );

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
                if let Some(attestation) =
                    record.entry().to_app_option::<Attestation>().ok().flatten()
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
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToAgent,
        (),
    )?;

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
pub fn get_or_create_agent_progress(
    input: CreateAgentProgressInput,
) -> ExternResult<AgentProgressOutput> {
    let progress_id = format!("{}-{}", input.agent_id, input.path_id);
    let progress_anchor = StringAnchor::new("agent_progress", &progress_id);
    let progress_anchor_hash = hash_entry(&EntryTypes::StringAnchor(progress_anchor))?;

    // Check if progress exists
    let query = LinkQuery::try_new(progress_anchor_hash.clone(), LinkTypes::AgentToProgress)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(progress) = record
                    .entry()
                    .to_app_option::<AgentProgress>()
                    .ok()
                    .flatten()
                {
                    return Ok(AgentProgressOutput {
                        action_hash,
                        progress,
                    });
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
    create_link(
        progress_anchor_hash,
        action_hash.clone(),
        LinkTypes::AgentToProgress,
        (),
    )?;

    Ok(AgentProgressOutput {
        action_hash,
        progress,
    })
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

    let action_hash =
        link.target
            .clone()
            .into_action_hash()
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Invalid progress hash".to_string()
            )))?;

    let record = get(action_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Progress record not found".to_string())
    ))?;

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
    create_link(
        progress_anchor_hash,
        new_action_hash.clone(),
        LinkTypes::AgentToProgress,
        (),
    )?;

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

    let mastery =
        if let Some(link) = links.first() {
            // Update existing
            let action_hash = link.target.clone().into_action_hash().ok_or(wasm_error!(
                WasmErrorInner::Guest("Invalid mastery hash".to_string())
            ))?;

            let record = get(action_hash, GetOptions::default())?.ok_or(wasm_error!(
                WasmErrorInner::Guest("Mastery record not found".to_string())
            ))?;

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
    create_link(
        mastery_anchor_hash,
        action_hash.clone(),
        LinkTypes::HumanToMastery,
        (),
    )?;

    Ok(ContentMasteryOutput {
        action_hash,
        mastery,
    })
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
                if let Some(mastery) = record
                    .entry()
                    .to_app_option::<ContentMastery>()
                    .ok()
                    .flatten()
                {
                    return Ok(Some(ContentMasteryOutput {
                        action_hash,
                        mastery,
                    }));
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
                if let Some(mastery) = record
                    .entry()
                    .to_app_option::<ContentMastery>()
                    .ok()
                    .flatten()
                {
                    results.push(ContentMasteryOutput {
                        action_hash,
                        mastery,
                    });
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
        external_identifiers_json: input
            .external_identifiers_json
            .unwrap_or_else(|| "[]".to_string()),
        establishing_content_ids_json: input
            .establishing_content_ids_json
            .unwrap_or_else(|| "[]".to_string()),
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
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToPresence,
        (),
    )?;

    // Create state link
    let state_anchor = StringAnchor::new("presence_state", "unclaimed");
    let state_anchor_hash = hash_entry(&EntryTypes::StringAnchor(state_anchor))?;
    create_link(
        state_anchor_hash,
        action_hash.clone(),
        LinkTypes::PresenceByState,
        (),
    )?;

    Ok(PresenceOutput {
        action_hash,
        presence,
    })
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
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToPresence,
        (),
    )?;

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
    create_link(
        new_state_anchor_hash,
        action_hash.clone(),
        LinkTypes::PresenceByState,
        (),
    )?;

    // Create steward link
    let steward_anchor = StringAnchor::new("steward_presences", &input.steward_agent_id);
    let steward_anchor_hash = hash_entry(&EntryTypes::StringAnchor(steward_anchor))?;
    create_link(
        steward_anchor_hash,
        action_hash.clone(),
        LinkTypes::StewardToPresence,
        (),
    )?;

    Ok(PresenceOutput {
        action_hash,
        presence: updated,
    })
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
                if let Some(presence) = record
                    .entry()
                    .to_app_option::<ContributorPresence>()
                    .ok()
                    .flatten()
                {
                    results.push(PresenceOutput {
                        action_hash,
                        presence,
                    });
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
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToPresence,
        (),
    )?;

    Ok(PresenceOutput {
        action_hash,
        presence: updated,
    })
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
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToPresence,
        (),
    )?;

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
    create_link(
        new_state_anchor_hash,
        action_hash.clone(),
        LinkTypes::PresenceByState,
        (),
    )?;

    // Create claimed agent link
    if let Some(ref claimed_agent) = updated.claimed_agent_id {
        let agent_anchor = StringAnchor::new("claimed_agent_presence", claimed_agent);
        let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;
        create_link(
            agent_anchor_hash,
            action_hash.clone(),
            LinkTypes::ClaimedAgentToPresence,
            (),
        )?;
    }

    Ok(PresenceOutput {
        action_hash,
        presence: updated,
    })
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
                if let Some(presence) = record
                    .entry()
                    .to_app_option::<ContributorPresence>()
                    .ok()
                    .flatten()
                {
                    return Ok(Some(PresenceOutput {
                        action_hash,
                        presence,
                    }));
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
                if let Some(presence) = record
                    .entry()
                    .to_app_option::<ContributorPresence>()
                    .ok()
                    .flatten()
                {
                    results.push(PresenceOutput {
                        action_hash,
                        presence,
                    });
                }
            }
        }
    }

    Ok(results)
}

// =============================================================================
// Recovery Types
// =============================================================================

/// Output from recovery vote operations (RecoveryVote entry type still registered)
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

/// Input for creating/updating a recovery hint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertRecoveryHintInput {
    pub hint_type: String,
    pub encrypted_data: String,
    pub encryption_nonce: String,
}

// =============================================================================
// Recovery Protocol Phase 2 — Signals
// =============================================================================

/// Signals for Recovery Protocol Phase 2.
/// Emitted by coordinator functions for real-time projection into elohim-storage.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum RecoveryV2Signal {
    RecoveryRequestCreated {
        action_hash: ActionHash,
        request: RecoveryRequest,
    },
    IntimateWitnessSubmitted {
        action_hash: ActionHash,
        request_hash: ActionHash,
        witness: HumanityWitness,
        witness_agent_id: AgentPubKey,
    },
    KeyRotationCommitted {
        action_hash: ActionHash,
        rotation: KeyRotation,
    },
    // M4: fast-path revocation signals.
    KeyRevocationRequested {
        id: String,
        human_id: String,
        revoked_key: String,
        reason: String,
        trigger_type: String,
        initiated_by: String,
        required_votes: u32,
        current_votes: u32,
        threshold_reached: bool,
        effective_at: Option<String>,
        created_at: String,
    },
    RevocationVoteSubmitted {
        id: String,
        revocation_id: String,
        steward_id: String,
        approved: bool,
        attestation: String,
        voted_at: String,
        current_votes: u32,
        required_votes: u32,
        threshold_now_reached: bool,
    },
    KeyRevocationEffective {
        revocation_id: String,
        revoked_key: String,
        human_id: String,
        effective_at: String,
        triggering_vote_id: Option<String>,
    },
}

// =============================================================================
// Recovery Protocol Phase 2 — Input/Output Types
// =============================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateRecoveryRequestInput {
    pub human_agent_pubkey: AgentPubKey,
    pub new_agent_pubkey: AgentPubKey,
    pub hosting_doorway_pubkey: AgentPubKey,
    pub proposed_authority: RecoveryAuthorityKind,
    pub request_nonce: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecoveryRequestOutput {
    pub action_hash: ActionHash,
    pub request: RecoveryRequest,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommitKeyRotationInput {
    pub human_agent_pubkey: AgentPubKey,
    pub new_agent_pubkey: AgentPubKey,
    pub superseded_agent_pubkey: AgentPubKey,
    pub recovery_request_hash: ActionHash,
    pub authority: RecoveryAuthority,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KeyRotationOutput {
    pub action_hash: ActionHash,
    pub rotation: KeyRotation,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SubmitIntimateWitnessInput {
    pub recovery_request_hash: ActionHash,
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SubmitIntimateWitnessOutput {
    pub action_hash: ActionHash,
    pub witness: HumanityWitness,
}

// =============================================================================
// Recovery Protocol Phase 2 — M3 Helpers
// =============================================================================

/// Resolve an `AgentPubKey` to the `human_id` String by traversing
/// `AgentKeyToHuman` link → Human entry → human.id. Returns a coordinator
/// error if no Human is bound to the given pubkey.
fn resolve_human_id_for_agent(agent_pubkey: &AgentPubKey) -> ExternResult<String> {
    let links = get_links(
        LinkQuery::try_new(agent_pubkey.clone(), LinkTypes::AgentKeyToHuman)?,
        GetStrategy::default(),
    )?;
    let first = links.first().ok_or(wasm_error!(WasmErrorInner::Guest(format!(
        "No Human bound to agent pubkey {}",
        agent_pubkey
    ))))?;
    let action_hash = first
        .target
        .clone()
        .into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "AgentKeyToHuman target is not an action hash".into()
        )))?;
    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Human entry missing".into())))?;
    let human: Human = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Human entry deserialize failed".into())))?;
    Ok(human.id)
}

/// Count the human's active `HumanRelationship` entries where
/// `emergency_access_enabled = true`. Active means no revocation path applied;
/// the simple rule for M3 is "entry still exists and flag is true."
fn count_active_emergency_contacts(human_id: &str) -> ExternResult<u32> {
    let anchor = StringAnchor::new("agent_relationships", human_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    let links = get_links(
        LinkQuery::try_new(anchor_hash, LinkTypes::AgentToRelationship)?,
        GetStrategy::default(),
    )?;
    let mut count: u32 = 0;
    for link in links {
        let rel_hash = match link.target.clone().into_action_hash() {
            Some(h) => h,
            None => continue,
        };
        let Some(record) = get(rel_hash, GetOptions::default())? else { continue };
        let Some(rel): Option<HumanRelationship> = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        else { continue };
        if rel.emergency_access_enabled {
            count += 1;
        }
    }
    Ok(count)
}

/// Threshold formula per revised spec §5 / M3 design §4.2: `max(2, ceil(M/2) + 1)`.
fn compute_required_witness_count(active_emergency_contacts: u32) -> u32 {
    let m = active_emergency_contacts;
    let ceil_half_plus_one = (m + 1) / 2 + 1; // ceil(m/2) + 1 for u32
    std::cmp::max(2, ceil_half_plus_one)
}

/// Count approved votes on a KeyRevocation by traversing RevocationToVote links.
///
/// Only votes with `approved == true` count toward the quorum threshold.
/// Rejections are preserved in the DHT for audit but never advance the
/// pending -> effective transition.
fn count_approved_revocation_votes(revocation_id: &str) -> ExternResult<u32> {
    let anchor = StringAnchor::new("revocation_votes", revocation_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    let links = get_links(
        LinkQuery::try_new(anchor_hash, LinkTypes::RevocationToVote)?,
        GetStrategy::default(),
    )?;

    let mut approved_count: u32 = 0;
    for link in links {
        let Some(vote_hash) = link.target.clone().into_action_hash() else { continue };
        let Some(record) = get(vote_hash, GetOptions::default())? else { continue };
        let Some(vote): Option<RevocationVote> = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        else { continue };
        if vote.approved {
            approved_count += 1;
        }
    }

    Ok(approved_count)
}

/// Intimate-recovery witness validity horizon. Matches protocol spec §5.
const WITNESS_EXPIRY_DAYS: u64 = 90;
const MICROS_PER_DAY: u64 = 24 * 60 * 60 * 1_000_000;

#[cfg(test)]
mod m3_witness_threshold_tests {
    use super::compute_required_witness_count;

    #[test]
    fn floor_at_two_when_no_contacts() {
        assert_eq!(compute_required_witness_count(0), 2);
    }

    #[test]
    fn floor_at_two_when_one_contact() {
        assert_eq!(compute_required_witness_count(1), 2);
    }

    #[test]
    fn two_contacts_yields_two() {
        // ceil(2/2) + 1 = 1 + 1 = 2; floor is 2; max(2, 2) = 2.
        // Boundary: the point where the floor stops dominating.
        assert_eq!(compute_required_witness_count(2), 2);
    }

    #[test]
    fn three_contacts_yields_three() {
        // ceil(3/2) + 1 = 2 + 1 = 3
        assert_eq!(compute_required_witness_count(3), 3);
    }

    #[test]
    fn four_contacts_yields_three() {
        // ceil(4/2) + 1 = 2 + 1 = 3
        assert_eq!(compute_required_witness_count(4), 3);
    }

    #[test]
    fn five_contacts_yields_four() {
        // ceil(5/2) + 1 = 3 + 1 = 4
        assert_eq!(compute_required_witness_count(5), 4);
    }
}

// =============================================================================
// Recovery Protocol Phase 2 — Coordinator Functions
// =============================================================================

/// Create a recovery request.
/// Authored by the hosting doorway on behalf of the claimant's new device.
/// M3: resolves human_id from agent pubkey and computes required_witness_count.
/// Anchors on human_id (was: pubkey string). See design §12.
#[hdk_extern]
pub fn create_recovery_request(
    input: CreateRecoveryRequestInput,
) -> ExternResult<RecoveryRequestOutput> {
    let now = sys_time()?;

    // M3: resolve human_id + compute required_witness_count
    let human_id = resolve_human_id_for_agent(&input.human_agent_pubkey)?;
    let contact_count = count_active_emergency_contacts(&human_id)?;
    let required_witness_count = compute_required_witness_count(contact_count);

    let request = RecoveryRequest {
        human_agent_pubkey: input.human_agent_pubkey.clone(),
        new_agent_pubkey: input.new_agent_pubkey,
        hosting_doorway_pubkey: input.hosting_doorway_pubkey,
        proposed_authority: input.proposed_authority,
        request_nonce: input.request_nonce,
        human_id: Some(human_id.clone()),
        required_witness_count,
        created_at: now,
    };

    let action_hash = create_entry(&EntryTypes::RecoveryRequest(request.clone()))?;

    // M3 decision log #2: anchor on human_id (was: pubkey). See design §12.
    let anchor = StringAnchor::new("recovery_request", &human_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(anchor))?;
    create_link(
        anchor_hash,
        action_hash.clone(),
        LinkTypes::HumanToRecoveryRequest,
        (),
    )?;

    emit_signal(RecoveryV2Signal::RecoveryRequestCreated {
        action_hash: action_hash.clone(),
        request: request.clone(),
    })?;

    Ok(RecoveryRequestOutput {
        action_hash,
        request,
    })
}

// =============================================================================
// Recovery Protocol Phase 2 — M4: Fast-Path Key Revocation
// =============================================================================

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateSelfRevocationInput {
    pub revoked_key: AgentPubKey,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyRevocationOutput {
    pub revocation_id: String,
    pub action_hash: ActionHash,
}

/// M4: Self-revocation. A human with a valid agent key voluntarily revokes
/// a different (compromised) key they control. Single-cell authority, no
/// quorum, no witnesses.
#[hdk_extern]
pub fn create_self_revocation(
    input: CreateSelfRevocationInput,
) -> ExternResult<KeyRevocationOutput> {
    let caller_pubkey = agent_info()?.agent_initial_pubkey;
    let human_id = resolve_human_id_for_agent(&caller_pubkey)?;

    // Gate: revoked_key must belong to the same human.
    let owner_human_id = resolve_human_id_for_agent(&input.revoked_key)?;
    if owner_human_id != human_id {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "create_self_revocation: caller does not control revoked_key (different human_id)".into()
        )));
    }

    if !REVOCATION_REASONS.contains(&input.reason.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "create_self_revocation: invalid reason '{}'. Must be one of {:?}",
            input.reason, REVOCATION_REASONS
        ))));
    }

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);
    let revocation_id = format!("rev-{}-{}", human_id, timestamp);
    let revoked_key_str = input.revoked_key.to_string();

    let revocation = KeyRevocation {
        id: revocation_id.clone(),
        human_id: human_id.clone(),
        revoked_key: revoked_key_str.clone(),
        reason: input.reason.clone(),
        initiated_by: human_id.clone(),
        trigger_type: "voluntary".to_string(),
        required_votes: 1,
        current_votes: 1,
        votes_json: String::new(), // legacy field, unused by M4
        threshold_reached: true,
        effective_at: Some(timestamp.clone()),
        created_at: timestamp.clone(),
        updated_at: timestamp.clone(),
    };

    let action_hash = create_entry(&EntryTypes::KeyRevocation(revocation.clone()))?;

    // IdToKeyRevocation anchor
    let id_anchor = StringAnchor::new("revocation_id", &revocation_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToKeyRevocation, ())?;

    // HumanToKeyRevocation anchor (dual-anchor primacy: human listing)
    let human_anchor = StringAnchor::new("human_revocations", &human_id);
    let human_anchor_hash = hash_entry(&EntryTypes::StringAnchor(human_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(human_anchor))?;
    create_link(human_anchor_hash, action_hash.clone(), LinkTypes::HumanToKeyRevocation, ())?;

    // RevokedKeyToRevocation anchor (dual-anchor primacy: hot gate query)
    let revoked_key_anchor = StringAnchor::new("revoked_key", &revoked_key_str);
    let revoked_key_anchor_hash =
        hash_entry(&EntryTypes::StringAnchor(revoked_key_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(revoked_key_anchor))?;
    create_link(
        revoked_key_anchor_hash,
        action_hash.clone(),
        LinkTypes::RevokedKeyToRevocation,
        (),
    )?;

    // EffectiveRevocations anchor — voluntary is effective on creation.
    let effective_anchor = StringAnchor::new("effective_revocations", "global");
    let effective_anchor_hash =
        hash_entry(&EntryTypes::StringAnchor(effective_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(effective_anchor))?;
    create_link(
        effective_anchor_hash,
        action_hash.clone(),
        LinkTypes::EffectiveRevocations,
        (),
    )?;

    // Emit both signals atomically: Requested + Effective.
    emit_signal(RecoveryV2Signal::KeyRevocationRequested {
        id: revocation.id.clone(),
        human_id: revocation.human_id.clone(),
        revoked_key: revocation.revoked_key.clone(),
        reason: revocation.reason.clone(),
        trigger_type: revocation.trigger_type.clone(),
        initiated_by: revocation.initiated_by.clone(),
        required_votes: revocation.required_votes,
        current_votes: revocation.current_votes,
        threshold_reached: revocation.threshold_reached,
        effective_at: revocation.effective_at.clone(),
        created_at: revocation.created_at.clone(),
    })?;

    emit_signal(RecoveryV2Signal::KeyRevocationEffective {
        revocation_id: revocation.id.clone(),
        revoked_key: revocation.revoked_key.clone(),
        human_id: revocation.human_id.clone(),
        effective_at: timestamp,
        triggering_vote_id: None,
    })?;

    Ok(KeyRevocationOutput { revocation_id, action_hash })
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateRevocationRequestInput {
    pub target_human_id: String,
    pub revoked_key: AgentPubKey,
    pub reason: String,
}

/// M4: Emergency-contact revocation request. The caller must be an active
/// emergency contact of `target_human_id`. Creates a pending KeyRevocation
/// with quorum threshold = compute_required_witness_count(active_emergency_contact_count).
#[hdk_extern]
pub fn create_revocation_request(
    input: CreateRevocationRequestInput,
) -> ExternResult<KeyRevocationOutput> {
    let caller_pubkey = agent_info()?.agent_initial_pubkey;
    let caller_human_id = resolve_human_id_for_agent(&caller_pubkey)?;

    // Gate: caller must be an active emergency contact for target_human_id.
    if !is_active_emergency_contact(&input.target_human_id, &caller_human_id)? {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "create_revocation_request: caller is not an active emergency contact for {}",
            input.target_human_id
        ))));
    }

    // Gate: revoked_key must belong to target_human_id.
    let owner_human_id = resolve_human_id_for_agent(&input.revoked_key)?;
    if owner_human_id != input.target_human_id {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "create_revocation_request: revoked_key does not belong to target_human_id".into()
        )));
    }

    if !REVOCATION_REASONS.contains(&input.reason.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "create_revocation_request: invalid reason '{}'. Must be one of {:?}",
            input.reason, REVOCATION_REASONS
        ))));
    }

    // TODO(M4-post): revisit whether revocation quorum should diverge from
    // recovery quorum. For now, parity with M3 keeps the two paths coherent.
    let contact_count = count_active_emergency_contacts(&input.target_human_id)?;
    let required = compute_required_witness_count(contact_count);

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);
    let revocation_id =
        format!("rev-{}-{}", input.target_human_id, timestamp);
    let revoked_key_str = input.revoked_key.to_string();

    let revocation = KeyRevocation {
        id: revocation_id.clone(),
        human_id: input.target_human_id.clone(),
        revoked_key: revoked_key_str.clone(),
        reason: input.reason.clone(),
        initiated_by: caller_human_id.clone(),
        trigger_type: "steward_vote".to_string(),
        required_votes: required,
        current_votes: 0,
        votes_json: String::new(),
        threshold_reached: false,
        effective_at: None,
        created_at: timestamp.clone(),
        updated_at: timestamp.clone(),
    };

    let action_hash = create_entry(&EntryTypes::KeyRevocation(revocation.clone()))?;

    // IdToKeyRevocation anchor
    let id_anchor = StringAnchor::new("revocation_id", &revocation_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToKeyRevocation, ())?;

    // HumanToKeyRevocation anchor
    let human_anchor = StringAnchor::new("human_revocations", &input.target_human_id);
    let human_anchor_hash = hash_entry(&EntryTypes::StringAnchor(human_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(human_anchor))?;
    create_link(human_anchor_hash, action_hash.clone(), LinkTypes::HumanToKeyRevocation, ())?;

    // RevokedKeyToRevocation anchor
    let revoked_key_anchor = StringAnchor::new("revoked_key", &revoked_key_str);
    let revoked_key_anchor_hash =
        hash_entry(&EntryTypes::StringAnchor(revoked_key_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(revoked_key_anchor))?;
    create_link(
        revoked_key_anchor_hash,
        action_hash.clone(),
        LinkTypes::RevokedKeyToRevocation,
        (),
    )?;

    // PendingRevocations anchor — quorum is not yet met.
    let pending_anchor = StringAnchor::new("pending_revocations", "global");
    let pending_anchor_hash = hash_entry(&EntryTypes::StringAnchor(pending_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(pending_anchor))?;
    create_link(pending_anchor_hash, action_hash.clone(), LinkTypes::PendingRevocations, ())?;

    emit_signal(RecoveryV2Signal::KeyRevocationRequested {
        id: revocation.id.clone(),
        human_id: revocation.human_id.clone(),
        revoked_key: revocation.revoked_key.clone(),
        reason: revocation.reason.clone(),
        trigger_type: revocation.trigger_type.clone(),
        initiated_by: revocation.initiated_by.clone(),
        required_votes: revocation.required_votes,
        current_votes: revocation.current_votes,
        threshold_reached: revocation.threshold_reached,
        effective_at: revocation.effective_at.clone(),
        created_at: revocation.created_at.clone(),
    })?;

    Ok(KeyRevocationOutput { revocation_id, action_hash })
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubmitRevocationVoteInput {
    pub revocation_id: String,
    pub approved: bool,
    pub attestation: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RevocationVoteOutput {
    pub vote_id: String,
    pub current_votes: u32,
    pub required_votes: u32,
    pub threshold_now_reached: bool,
}

/// M4: Submit an emergency-contact vote on a pending KeyRevocation.
/// On the threshold-meeting vote, the coordinator updates the KeyRevocation
/// entry (flips threshold_reached, sets effective_at), moves it from
/// PendingRevocations to EffectiveRevocations, and emits both
/// RevocationVoteSubmitted and KeyRevocationEffective signals.
#[hdk_extern]
pub fn submit_revocation_vote(
    input: SubmitRevocationVoteInput,
) -> ExternResult<RevocationVoteOutput> {
    let caller_pubkey = agent_info()?.agent_initial_pubkey;
    let caller_human_id = resolve_human_id_for_agent(&caller_pubkey)?;

    if input.attestation.trim().is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "submit_revocation_vote: attestation cannot be empty".into()
        )));
    }

    // Load the KeyRevocation via IdToKeyRevocation anchor.
    let revocation_id_anchor = StringAnchor::new("revocation_id", &input.revocation_id);
    let revocation_anchor_hash =
        hash_entry(&EntryTypes::StringAnchor(revocation_id_anchor))?;
    let revocation_links = get_links(
        LinkQuery::try_new(revocation_anchor_hash, LinkTypes::IdToKeyRevocation)?,
        GetStrategy::default(),
    )?;
    let revocation_link = revocation_links.first().ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "submit_revocation_vote: no KeyRevocation with id {}",
            input.revocation_id
        )))
    })?;
    let revocation_action_hash = revocation_link
        .target
        .clone()
        .into_action_hash()
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "IdToKeyRevocation target was not an ActionHash".into()
            ))
        })?;
    let revocation_record =
        get(revocation_action_hash.clone(), GetOptions::default())?.ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest("KeyRevocation record not found".into()))
        })?;
    let revocation: KeyRevocation = revocation_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest("KeyRevocation record missing entry".into()))
        })?;

    // Gate: votes only apply to the steward_vote path.
    if revocation.trigger_type != "steward_vote" {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "submit_revocation_vote: revocation {} has trigger_type={}, votes not accepted",
            input.revocation_id, revocation.trigger_type
        ))));
    }

    if revocation.threshold_reached {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "submit_revocation_vote: revocation {} already effective",
            input.revocation_id
        ))));
    }

    // Gate: caller must be an active emergency contact.
    if !is_active_emergency_contact(&revocation.human_id, &caller_human_id)? {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "submit_revocation_vote: caller is not an active emergency contact for {}",
            revocation.human_id
        ))));
    }

    // Gate: no existing vote from this steward on this revocation.
    let steward_anchor = StringAnchor::new("steward_revocation_votes", &caller_human_id);
    let steward_anchor_hash = hash_entry(&EntryTypes::StringAnchor(steward_anchor.clone()))?;
    let steward_vote_links = get_links(
        LinkQuery::try_new(steward_anchor_hash.clone(), LinkTypes::StewardToRevocationVote)?,
        GetStrategy::default(),
    )?;
    for link in &steward_vote_links {
        let Some(vote_hash) = link.target.clone().into_action_hash() else { continue };
        let Some(rec) = get(vote_hash, GetOptions::default())? else { continue };
        let Some(prior_vote): Option<RevocationVote> = rec
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        else { continue };
        if prior_vote.revocation_id == input.revocation_id {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "submit_revocation_vote: steward {} has already voted on revocation {}",
                caller_human_id, input.revocation_id
            ))));
        }
    }

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);
    let vote_id = format!("vote-{}-{}", caller_human_id, timestamp);

    let vote = RevocationVote {
        id: vote_id.clone(),
        revocation_id: input.revocation_id.clone(),
        steward_id: caller_human_id.clone(),
        approved: input.approved,
        attestation: input.attestation.clone(),
        voted_at: timestamp.clone(),
    };

    let vote_action_hash = create_entry(&EntryTypes::RevocationVote(vote.clone()))?;

    // IdToRevocationVote anchor
    let vote_id_anchor = StringAnchor::new("revocation_vote_id", &vote_id);
    let vote_id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(vote_id_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(vote_id_anchor))?;
    create_link(
        vote_id_anchor_hash,
        vote_action_hash.clone(),
        LinkTypes::IdToRevocationVote,
        (),
    )?;

    // RevocationToVote anchor (per-revocation vote list)
    let revocation_votes_anchor =
        StringAnchor::new("revocation_votes", &input.revocation_id);
    let revocation_votes_anchor_hash =
        hash_entry(&EntryTypes::StringAnchor(revocation_votes_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(revocation_votes_anchor))?;
    create_link(
        revocation_votes_anchor_hash,
        vote_action_hash.clone(),
        LinkTypes::RevocationToVote,
        (),
    )?;

    // StewardToRevocationVote anchor
    create_entry(&EntryTypes::StringAnchor(steward_anchor))?;
    create_link(
        steward_anchor_hash,
        vote_action_hash.clone(),
        LinkTypes::StewardToRevocationVote,
        (),
    )?;

    // Recompute threshold (count approved votes from link traversal).
    let approved_count = count_approved_revocation_votes(&input.revocation_id)?;
    let threshold_now_reached = approved_count >= revocation.required_votes;

    if threshold_now_reached {
        // Update the KeyRevocation entry: flip threshold_reached, set effective_at.
        let mut updated = revocation.clone();
        updated.current_votes = approved_count;
        updated.threshold_reached = true;
        updated.effective_at = Some(timestamp.clone());
        updated.updated_at = timestamp.clone();
        update_entry(revocation_action_hash, &EntryTypes::KeyRevocation(updated.clone()))?;

        // Move from PendingRevocations to EffectiveRevocations.
        let pending_global_anchor = StringAnchor::new("pending_revocations", "global");
        let pending_global_anchor_hash =
            hash_entry(&EntryTypes::StringAnchor(pending_global_anchor))?;
        let pending_links = get_links(
            LinkQuery::try_new(pending_global_anchor_hash, LinkTypes::PendingRevocations)?,
            GetStrategy::default(),
        )?;
        for link in pending_links {
            // Each pending link points to a specific revocation's action_hash.
            // We identify by fetching the entry and matching the id.
            let Some(link_target_hash) = link.target.clone().into_action_hash() else { continue };
            let Some(rec) = get(link_target_hash, GetOptions::default())? else { continue };
            let Some(rev): Option<KeyRevocation> = rec
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
            else { continue };
            if rev.id == input.revocation_id {
                delete_link(link.create_link_hash, GetOptions::default())?;
            }
        }

        let effective_anchor = StringAnchor::new("effective_revocations", "global");
        let effective_anchor_hash =
            hash_entry(&EntryTypes::StringAnchor(effective_anchor.clone()))?;
        create_entry(&EntryTypes::StringAnchor(effective_anchor))?;
        create_link(
            effective_anchor_hash,
            revocation_link.target.clone(), // points to the revocation entry
            LinkTypes::EffectiveRevocations,
            (),
        )?;

        emit_signal(RecoveryV2Signal::RevocationVoteSubmitted {
            id: vote_id.clone(),
            revocation_id: input.revocation_id.clone(),
            steward_id: caller_human_id.clone(),
            approved: input.approved,
            attestation: input.attestation.clone(),
            voted_at: timestamp.clone(),
            current_votes: approved_count,
            required_votes: revocation.required_votes,
            threshold_now_reached: true,
        })?;

        emit_signal(RecoveryV2Signal::KeyRevocationEffective {
            revocation_id: input.revocation_id.clone(),
            revoked_key: revocation.revoked_key.clone(),
            human_id: revocation.human_id.clone(),
            effective_at: timestamp,
            triggering_vote_id: Some(vote_id.clone()),
        })?;
    } else {
        emit_signal(RecoveryV2Signal::RevocationVoteSubmitted {
            id: vote_id.clone(),
            revocation_id: input.revocation_id.clone(),
            steward_id: caller_human_id.clone(),
            approved: input.approved,
            attestation: input.attestation.clone(),
            voted_at: timestamp,
            current_votes: approved_count,
            required_votes: revocation.required_votes,
            threshold_now_reached: false,
        })?;
    }

    Ok(RevocationVoteOutput {
        vote_id,
        current_votes: approved_count,
        required_votes: revocation.required_votes,
        threshold_now_reached,
    })
}

/// Traverse `HumanToFreeze` links for the given `human_id` and return all
/// `IdentityFreeze` entries with `is_active = true`. Used by the M3 freeze-
/// floor gate on `commit_key_rotation`.
fn collect_active_freezes_for_human(human_id: &str) -> ExternResult<Vec<IdentityFreeze>> {
    let anchor = StringAnchor::new("identity_freeze_by_human", human_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    let links = get_links(
        LinkQuery::try_new(anchor_hash, LinkTypes::HumanToFreeze)?,
        GetStrategy::default(),
    )?;
    let mut freezes = Vec::new();
    for link in links {
        let Some(hash) = link.target.clone().into_action_hash() else { continue };
        let Some(record) = get(hash, GetOptions::default())? else { continue };
        let Some(freeze): Option<IdentityFreeze> = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        else { continue };
        if freeze.is_active {
            freezes.push(freeze);
        }
    }
    Ok(freezes)
}

/// Commit a key rotation to the DHT.
/// M3: runs freeze-floor pre-commit gate (CryptographicQuorum exempt) before
/// creating the entry.
#[hdk_extern]
pub fn commit_key_rotation(input: CommitKeyRotationInput) -> ExternResult<KeyRotationOutput> {
    let now = sys_time()?;

    // M3: freeze-floor gate (skips for CryptographicQuorum per design §4.3).
    let is_cryptographic = matches!(
        &input.authority,
        RecoveryAuthority::CryptographicQuorum { .. }
    );
    if !is_cryptographic {
        // Resolve human_id from the rotating pubkey (same path as create_recovery_request).
        let human_id = resolve_human_id_for_agent(&input.human_agent_pubkey)?;
        let active_freezes = collect_active_freezes_for_human(&human_id)?;
        let freeze_refs: Vec<&IdentityFreeze> = active_freezes.iter().collect();

        if let Some(reason) = check_freeze_floor_rules(&input.authority, &human_id, &freeze_refs) {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "freeze-floor gate rejected rotation: {reason}"
            ))));
        }
    }

    // M4: revocation-floor gate.
    // If a pending or effective KeyRevocation exists for the human_agent_pubkey
    // (the key being rotated from), block the rotation. No authority-layer exemption
    // — revocation is structural (a revoked key must not produce valid rotations under
    // any claimed authority), intentionally asymmetric with the freeze-floor gate
    // which exempts CryptographicQuorum.
    {
        let rotating_from_str = input.human_agent_pubkey.to_string();

        let pending_anchor = StringAnchor::new("pending_revocations", "global");
        let pending_anchor_hash = hash_entry(&EntryTypes::StringAnchor(pending_anchor))?;
        let pending_links = get_links(
            LinkQuery::try_new(pending_anchor_hash, LinkTypes::PendingRevocations)?,
            GetStrategy::default(),
        )?;

        let effective_anchor = StringAnchor::new("effective_revocations", "global");
        let effective_anchor_hash = hash_entry(&EntryTypes::StringAnchor(effective_anchor))?;
        let effective_links = get_links(
            LinkQuery::try_new(effective_anchor_hash, LinkTypes::EffectiveRevocations)?,
            GetStrategy::default(),
        )?;

        for (link, status) in pending_links
            .iter()
            .map(|l| (l, "pending"))
            .chain(effective_links.iter().map(|l| (l, "effective")))
        {
            let Some(rev_hash) = link.target.clone().into_action_hash() else { continue };
            let Some(rec) = get(rev_hash, GetOptions::default())? else { continue };
            let Some(rev): Option<KeyRevocation> = rec
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
            else { continue };
            if rev.revoked_key == rotating_from_str {
                return Err(wasm_error!(WasmErrorInner::Guest(format!(
                    "commit_key_rotation blocked: key {} has a {} revocation ({}). \
                     Resolve or await the revocation before rotating.",
                    rotating_from_str, status, rev.id
                ))));
            }
        }
    }

    let rotation = KeyRotation {
        human_agent_pubkey: input.human_agent_pubkey.clone(),
        new_agent_pubkey: input.new_agent_pubkey.clone(),
        superseded_agent_pubkey: input.superseded_agent_pubkey,
        recovery_request_hash: input.recovery_request_hash,
        authority: input.authority,
        rotated_at: now,
    };

    let action_hash = create_entry(&EntryTypes::KeyRotation(rotation.clone()))?;

    let current_agent_anchor =
        StringAnchor::new("current_agent", &input.human_agent_pubkey.to_string());
    let current_agent_anchor_hash =
        hash_entry(&EntryTypes::StringAnchor(current_agent_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(current_agent_anchor))?;
    create_link(
        current_agent_anchor_hash,
        action_hash.clone(),
        LinkTypes::HumanToCurrentAgent,
        (),
    )?;

    let agent_rotation_anchor =
        StringAnchor::new("agent_rotation", &input.new_agent_pubkey.to_string());
    let agent_rotation_anchor_hash =
        hash_entry(&EntryTypes::StringAnchor(agent_rotation_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(agent_rotation_anchor))?;
    create_link(
        agent_rotation_anchor_hash,
        action_hash.clone(),
        LinkTypes::AgentToKeyRotation,
        (),
    )?;

    emit_signal(RecoveryV2Signal::KeyRotationCommitted {
        action_hash: action_hash.clone(),
        rotation: rotation.clone(),
    })?;

    Ok(KeyRotationOutput {
        action_hash,
        rotation,
    })
}

// =============================================================================
// Recovery Protocol Phase 2 — M3: submit_intimate_witness
// =============================================================================

/// Returns true if `authorizer_human_id` holds an active `HumanRelationship`
/// with `emergency_access_enabled = true` targeting `target_human_id`.
/// Both `party_a_id` and `party_b_id` are human ID strings.
fn is_active_emergency_contact(
    target_human_id: &str,
    authorizer_human_id: &str,
) -> ExternResult<bool> {
    // Traverse AgentToRelationship links anchored on the target human's id.
    let anchor = StringAnchor::new("agent_relationships", target_human_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    let links = get_links(
        LinkQuery::try_new(anchor_hash, LinkTypes::AgentToRelationship)?,
        GetStrategy::default(),
    )?;
    for link in links {
        let Some(rel_hash) = link.target.clone().into_action_hash() else { continue };
        let Some(record) = get(rel_hash, GetOptions::default())? else { continue };
        let Some(rel): Option<HumanRelationship> = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        else { continue };
        if !rel.emergency_access_enabled { continue; }
        // party_a_id and party_b_id are human ID strings.
        if rel.party_a_id == authorizer_human_id || rel.party_b_id == authorizer_human_id {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Returns true if `authorizer_human_id` already has a `HumanityWitness` linked
/// from the given request (via `RecoveryRequestToHumanityWitness`).
fn has_existing_witness_for_request(
    request_hash: &ActionHash,
    authorizer_human_id: &str,
) -> ExternResult<bool> {
    let links = get_links(
        LinkQuery::try_new(request_hash.clone(), LinkTypes::RecoveryRequestToHumanityWitness)?,
        GetStrategy::default(),
    )?;
    for link in links {
        let Some(w_hash) = link.target.clone().into_action_hash() else { continue };
        let Some(record) = get(w_hash, GetOptions::default())? else { continue };
        let Some(w): Option<HumanityWitness> = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        else { continue };
        if w.witness_agent_id == authorizer_human_id {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Submit an intimate witness attestation for a recovery request.
///
/// Gates:
/// 1. RecoveryRequest must exist and have a populated human_id.
/// 2. Authorizer must be an active emergency contact of the target human
///    (has a HumanRelationship with emergency_access_enabled = true).
/// 3. Dedupe: authorizer cannot witness the same request twice.
#[hdk_extern]
pub fn submit_intimate_witness(
    input: SubmitIntimateWitnessInput,
) -> ExternResult<SubmitIntimateWitnessOutput> {
    // Gate 1: fetch the RecoveryRequest; must exist.
    let request_record = get(input.recovery_request_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "RecoveryRequest not found at given hash".into()
        )))?;
    let request: RecoveryRequest = request_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "RecoveryRequest record has no entry".into()
        )))?;
    let human_id = request.human_id.clone().ok_or(wasm_error!(WasmErrorInner::Guest(
        "RecoveryRequest has no human_id (pre-M3 entry?)".into()
    )))?;

    // Gate 2: authorizer must be on an active emergency-enabled HumanRelationship.
    let authorizer_pubkey = agent_info()?.agent_initial_pubkey;
    let authorizer_human_id = resolve_human_id_for_agent(&authorizer_pubkey)?;
    if !is_active_emergency_contact(&human_id, &authorizer_human_id)? {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "authorizing agent is not an active emergency contact of this human".into()
        )));
    }

    // Gate 3: dedupe — the authorizer cannot witness the same request twice.
    if has_existing_witness_for_request(&input.recovery_request_hash, &authorizer_human_id)? {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "this agent has already submitted a witness for this request".into()
        )));
    }

    // Commit the HumanityWitness. HumanityWitness has no `note` field;
    // the optional note is stored in evidence_json.
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);
    let expiry_micros = WITNESS_EXPIRY_DAYS * MICROS_PER_DAY;
    let expires_at = format!(
        "{:?}",
        now.checked_add(&Duration::from_micros(expiry_micros))
            .unwrap_or(now)
    );
    // Sanitize characters that appear in Timestamp Debug output; match the
    // convention used by other id generators in this zome (e.g., renewals).
    let witness_id_ts = timestamp.replace([':', ' ', '(', ')'], "-");
    let witness_id = format!("intimate-witness-{}-{}", human_id, witness_id_ts);
    let witness = HumanityWitness {
        id: witness_id,
        human_id: human_id.clone(),
        // NOTE: M3 stores authorizer's human_id here, not agent pubkey.
        // The field's name is a pre-M3 misnomer the protocol accepts.
        // Dedupe gate `has_existing_witness_for_request` depends on this convention.
        witness_agent_id: authorizer_human_id.clone(),
        attestation_type: "intimate_recovery".into(),
        confidence: 1.0,
        behavioral_hash: None,
        evidence_json: input.note.map(|n| serde_json::json!({ "note": n }).to_string()),
        verification_method: Some("intimate_recovery_ceremony".into()),
        created_at: timestamp,
        expires_at,
        revoked_at: None,
    };
    let action_hash = create_entry(&EntryTypes::HumanityWitness(witness.clone()))?;

    // Create the M3 link from the request to the witness.
    create_link(
        input.recovery_request_hash.clone(),
        action_hash.clone(),
        LinkTypes::RecoveryRequestToHumanityWitness,
        (),
    )?;

    // Emit rich signal.
    emit_signal(RecoveryV2Signal::IntimateWitnessSubmitted {
        action_hash: action_hash.clone(),
        request_hash: input.recovery_request_hash,
        witness: witness.clone(),
        witness_agent_id: authorizer_pubkey,
    })?;

    Ok(SubmitIntimateWitnessOutput {
        action_hash,
        witness,
    })
}

// =============================================================================
// Recovery Hint Functions
// =============================================================================

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
                if let Some(existing) = record
                    .entry()
                    .to_app_option::<RecoveryHint>()
                    .ok()
                    .flatten()
                {
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
        created_at: if version == 1 {
            timestamp.clone()
        } else {
            "".to_string()
        },
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::RecoveryHint(hint.clone()))?;

    // Update links
    for link in links {
        delete_link(link.create_link_hash, GetOptions::default())?;
    }
    create_link(
        hint_anchor_hash,
        action_hash.clone(),
        LinkTypes::HumanToRecoveryHint,
        (),
    )?;

    // Create type lookup link
    let type_anchor = StringAnchor::new("recovery_hint_type", &input.hint_type);
    let type_anchor_hash = hash_entry(&EntryTypes::StringAnchor(type_anchor))?;
    create_link(
        type_anchor_hash,
        action_hash.clone(),
        LinkTypes::RecoveryHintByType,
        (),
    )?;

    Ok(RecoveryHintOutput { action_hash, hint })
}

/// Get recovery hints for the calling agent
#[hdk_extern]
pub fn get_my_recovery_hints(_: ()) -> ExternResult<Vec<RecoveryHintOutput>> {
    let my_human = get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    let mut results = Vec::new();

    // Check each hint type
    for hint_type in &[
        "password_hint",
        "security_qa",
        "trusted_doorways",
        "trusted_contacts",
    ] {
        let hint_id = format!("{}-{}", my_human.human.id, hint_type);
        let hint_anchor = StringAnchor::new("recovery_hint", &hint_id);
        let hint_anchor_hash = hash_entry(&EntryTypes::StringAnchor(hint_anchor))?;

        let query = LinkQuery::try_new(hint_anchor_hash, LinkTypes::HumanToRecoveryHint)?;
        let links = get_links(query, GetStrategy::default())?;

        if let Some(link) = links.first() {
            if let Some(action_hash) = link.target.clone().into_action_hash() {
                if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                    if let Some(hint) = record
                        .entry()
                        .to_app_option::<RecoveryHint>()
                        .ok()
                        .flatten()
                    {
                        results.push(RecoveryHintOutput { action_hash, hint });
                    }
                }
            }
        }
    }

    Ok(results)
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
pub fn create_renewal_attestation(
    input: CreateRenewalAttestationInput,
) -> ExternResult<RenewalAttestationOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let hours = input.expires_in_hours.unwrap_or(72);
    let expiry_ms = hours as u64 * 60 * 60 * 1000 * 1000;
    let expires_at = format!(
        "{:?}",
        now.checked_add(&Duration::from_micros(expiry_ms))
            .unwrap_or(now)
    );

    let attestation_id = format!(
        "renewal-{}-{}",
        input.human_id,
        timestamp.replace([':', ' ', '(', ')'], "-")
    );

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
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToRenewalAttestation,
        (),
    )?;

    // Create human lookup link
    let human_anchor = StringAnchor::new("human_renewals", &input.human_id);
    let human_anchor_hash = hash_entry(&EntryTypes::StringAnchor(human_anchor))?;
    create_link(
        human_anchor_hash,
        action_hash.clone(),
        LinkTypes::HumanToRenewalAttestation,
        (),
    )?;

    // Create status link
    let status_anchor = StringAnchor::new("renewal_status", "pending");
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(
        status_anchor_hash,
        action_hash.clone(),
        LinkTypes::RenewalAttestationByStatus,
        (),
    )?;

    Ok(RenewalAttestationOutput {
        action_hash,
        entry: attestation,
    })
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
                if let Some(entry) = record
                    .entry()
                    .to_app_option::<RenewalAttestation>()
                    .ok()
                    .flatten()
                {
                    return Ok(Some(RenewalAttestationOutput { action_hash, entry }));
                }
            }
        }
    }

    Ok(None)
}

/// Get all renewal attestations for a human
#[hdk_extern]
pub fn get_renewal_attestations_for_human(
    human_id: String,
) -> ExternResult<Vec<RenewalAttestationOutput>> {
    let human_anchor = StringAnchor::new("human_renewals", &human_id);
    let human_anchor_hash = hash_entry(&EntryTypes::StringAnchor(human_anchor))?;

    let query = LinkQuery::try_new(human_anchor_hash, LinkTypes::HumanToRenewalAttestation)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(entry) = record
                    .entry()
                    .to_app_option::<RenewalAttestation>()
                    .ok()
                    .flatten()
                {
                    results.push(RenewalAttestationOutput { action_hash, entry });
                }
            }
        }
    }

    Ok(results)
}

/// Create an agent retirement (marks old key as superseded)
#[hdk_extern]
pub fn create_agent_retirement(
    input: CreateAgentRetirementInput,
) -> ExternResult<AgentRetirementOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let retirement_id = format!(
        "retirement-{}-{}",
        input.retired_agent_key.chars().take(8).collect::<String>(),
        timestamp.replace([':', ' ', '(', ')'], "-")
    );

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
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToAgentRetirement,
        (),
    )?;

    // Create old agent → retirement link (for "who is this agent now?" queries)
    let old_agent_anchor = StringAnchor::new("retired_agent", &input.retired_agent_key);
    let old_agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(old_agent_anchor))?;
    create_link(
        old_agent_anchor_hash,
        action_hash.clone(),
        LinkTypes::OldAgentToRetirement,
        (),
    )?;

    // Create new agent ← retirement link (for "where did this agent come from?" queries)
    let new_agent_anchor = StringAnchor::new("renewed_from", &input.renewed_into_agent_key);
    let new_agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(new_agent_anchor))?;
    create_link(
        new_agent_anchor_hash,
        action_hash.clone(),
        LinkTypes::NewAgentFromRetirement,
        (),
    )?;

    Ok(AgentRetirementOutput {
        action_hash,
        entry: retirement,
    })
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
                if let Some(entry) = record
                    .entry()
                    .to_app_option::<AgentRetirement>()
                    .ok()
                    .flatten()
                {
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
pub fn create_relationship_renewal(
    input: CreateRelationshipRenewalInput,
) -> ExternResult<RelationshipRenewalOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let renewal_id = format!(
        "rel-renewal-{}-{}",
        input.original_relationship_id,
        timestamp.replace([':', ' ', '(', ')'], "-")
    );

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
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToRelationshipRenewal,
        (),
    )?;

    // Create original relationship → renewal link
    let rel_anchor = StringAnchor::new("original_rel", &input.original_relationship_id);
    let rel_anchor_hash = hash_entry(&EntryTypes::StringAnchor(rel_anchor))?;
    create_link(
        rel_anchor_hash,
        action_hash.clone(),
        LinkTypes::OriginalRelToRenewal,
        (),
    )?;

    Ok(RelationshipRenewalOutput {
        action_hash,
        entry: renewal,
    })
}

/// Get all renewals for a specific original relationship
#[hdk_extern]
pub fn get_renewals_for_relationship(
    original_rel_id: String,
) -> ExternResult<Vec<RelationshipRenewalOutput>> {
    let rel_anchor = StringAnchor::new("original_rel", &original_rel_id);
    let rel_anchor_hash = hash_entry(&EntryTypes::StringAnchor(rel_anchor))?;

    let query = LinkQuery::try_new(rel_anchor_hash, LinkTypes::OriginalRelToRenewal)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(entry) = record
                    .entry()
                    .to_app_option::<RelationshipRenewal>()
                    .ok()
                    .flatten()
                {
                    results.push(RelationshipRenewalOutput { action_hash, entry });
                }
            }
        }
    }

    Ok(results)
}

/// Counterparty reaffirms a relationship renewal (co-signs)
#[hdk_extern]
pub fn reaffirm_relationship_renewal(
    renewal_id: String,
) -> ExternResult<RelationshipRenewalOutput> {
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

    let old_action_hash =
        link.target
            .clone()
            .into_action_hash()
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Invalid renewal hash".to_string()
            )))?;

    let record = get(old_action_hash, GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Renewal record not found".to_string())
    ))?;

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
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToRelationshipRenewal,
        (),
    )?;

    Ok(RelationshipRenewalOutput {
        action_hash,
        entry: renewal,
    })
}

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
/// For each ActionHash, attempts to retrieve the record and returns its status.
/// Used by elohim-storage during P2P trust negotiation.
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

// =============================================================================
// Init
// =============================================================================

#[hdk_extern]
pub fn init(_: ()) -> ExternResult<InitCallbackResult> {
    Ok(InitCallbackResult::Pass)
}
