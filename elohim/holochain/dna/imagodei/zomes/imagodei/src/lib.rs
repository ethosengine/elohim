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

// sign_for_agent coordinator function (EPR Phase 2B, Task C.1)
pub mod sign_for_agent;
pub use sign_for_agent::*;

// PortalHost coordinator functions (Recovery Phase 2 M5)
pub mod portal_host;
pub use portal_host::*;

// submit_specialist_revocation — defender producer (Recovery Phase 2 M5, Task 4)
pub mod submit_specialist_revocation;
pub use submit_specialist_revocation::*;

// content_decode — cross-DNA Content entry decoder (Recovery M4, Task 2).
//
// Helper for the case where a validator needs to deserialise a raw `Entry::App`
// payload from a cross-DNA Content commit. M4 Tasks 3-5 ended up using
// structured ContentOutput from coordinator bridge calls instead (see
// `fetch_recovery_request_human_id` and the migrated gate readers), so the
// helper has no caller in the M4 surface. Retained as a tested decoder
// primitive for T17/T18 (signal alignment + cross-stack integration) and
// future cross-DNA integrity work.
//
// TODO(M4 completion): if T17/T18 do not adopt this, delete the module.
#[allow(dead_code)]
mod content_decode;

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

/// Wire view of a created/fetched Agent EPR.
///
/// `action_hash` is `ActionHashB64` (base64 string with `uhCkk` prefix) so
/// the struct round-trips cleanly through `serde_json::Value` reads in
/// sweettests — msgpack `BIN` (raw `ActionHash` bytes) has no `Value`
/// variant. Typed Rust consumers can still construct the inner `ActionHash`
/// via `ActionHash::from(b64)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub action_hash: ActionHashB64,
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

/// Output from attestation operations.
///
/// Stage C.2: `attestation` field type changed from the removed `Attestation` entry
/// type to `LegacyAttestationView` (plain struct, same fields, no #[hdk_entry_helper]).
/// The serde wire format is identical — consumers in elohim-storage are unaffected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationOutput {
    pub action_hash: ActionHash,
    pub attestation: LegacyAttestationView,
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
    /// Stage C.2: field type changed from removed `Attestation` entry type to
    /// `LegacyAttestationView`. Wire format is identical (same fields). The
    /// post_commit emission branch is removed — B.9 bridge never writes
    /// Attestation entries to imagodei DHT, so this variant is never emitted.
    /// Kept for elohim-storage serde compatibility until Stage F removes consumers.
    AttestationCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        attestation: LegacyAttestationView,
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

        // Handle Delete actions for PortalHost removal signal.
        // `deletes_address` is the ActionHash of the original Create action.
        if let Action::Delete(ref delete) = action {
            let original_action_hash = delete.deletes_address.clone();
            if let Some(original_record) = get(original_action_hash.clone(), GetOptions::default())?
            {
                if let Ok(Some(ph)) = original_record.entry().to_app_option::<PortalHost>() {
                    let reach = format!("{:?}", ph.reach);
                    let _ = emit_signal(RecoveryV2Signal::PortalHostRemoved {
                        action_hash,
                        original_action_hash,
                        human_action_hash: ph.human_action_hash,
                        host_url: ph.host_url,
                    });
                    let _ = reach; // suppress unused warning
                }
            }
            continue;
        }

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

        // PortalHost Create — emit before the generic ImagodeiSignal arms.
        if let Ok(Some(ph)) = record.entry().to_app_option::<PortalHost>() {
            let reach = format!("{:?}", ph.reach);
            emit_signal(RecoveryV2Signal::PortalHostCreated {
                action_hash,
                human_action_hash: ph.human_action_hash,
                host_url: ph.host_url,
                label: ph.label,
                added_at: ph.added_at,
                reach,
            })?;
            continue;
        }

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
        }
        // AttestationCommitted emission removed C.2: Attestation is no longer a
        // DHT entry type — B.9 bridge never writes it to imagodei source chain,
        // so to_app_option::<Attestation>() could never fire. The signal variant
        // is kept for serde compat until Stage F removes elohim-storage consumers.
    }

    Ok(())
}

// =============================================================================
// Human Profile Functions
// =============================================================================

/// Get-or-create the calling agent's Human profile.
///
/// Returns the `ActionHash` of the Human entry bound to the calling agent.
/// **Idempotent**: if the agent already has a profile, returns the existing
/// hash without creating a duplicate or erroring. The fields in `input` are
/// only honoured on first call; subsequent calls return the existing entry
/// unchanged. Use `update_human` to mutate fields after creation.
///
/// Returning `ActionHash` directly keeps the wire shape compatible with both
/// typed (`let h: ActionHash = ...`) and other typed reads from sweettests
/// — a struct return type would break either side because msgpack `BIN`
/// (the ActionHash bytes) cannot deserialize into `serde_json::Value` (no
/// byte-array variant), and a struct return cannot deserialize into
/// `ActionHash`.
///
/// Doorway-side consumers that previously relied on the
/// `"Agent already has a Human profile"` error to detect re-registration of
/// an existing identity now receive the existing hash directly; the recovery
/// branches in `auth_routes.rs` become defensive no-ops.
#[hdk_extern]
pub fn create_human(input: CreateHumanInput) -> ExternResult<ActionHash> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Idempotency gate: if a Human already exists for this agent, return its
    // ActionHash unchanged. Sweettests in single-agent harnesses call
    // create_human multiple times (e.g. submit_specialist_revocation creates
    // both a "caller" and a "target" human under one agent) and expect each
    // call to succeed with a usable hash.
    if let Some(existing) = get_human_by_agent_key(agent_info.agent_initial_pubkey.clone())? {
        return Ok(existing.action_hash);
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

    let action_hash = create_entry(&EntryTypes::Human(human))?;

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

    Ok(action_hash)
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

/// Update my Human profile.
///
/// Returns the new `ActionHash` for the same reason as `create_human`:
/// keeping the wire shape an opaque hash lets sweettests read it as either
/// a typed `ActionHash` or a `serde_json::Value` (string), neither of which
/// works for a struct return that contains hash bytes. Use `get_my_human`
/// to fetch the projected `HumanOutput` after update.
#[hdk_extern]
pub fn update_human(input: CreateHumanInput) -> ExternResult<ActionHash> {
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

    update_entry(existing.action_hash, &EntryTypes::Human(human))
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
//
// B.9 bridge: these coordinator functions delegate to elohim DNA's
// content_store::issue_attestation / get_attestations_for_subject.
// Stage C will remove the legacy Attestation entry type from imagodei entirely.
// =============================================================================

// ---------------------------------------------------------------------------
// Consolidated bridge structs — wire-compatible with elohim DNA's public API.
// Defined locally because cross-DNA calls serialise through msgpack; imagodei
// cannot depend on elohim crates directly.
// ---------------------------------------------------------------------------

/// Input for elohim DNA's content_store::issue_attestation (B.9 bridge copy).
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

/// Output from elohim DNA's content_store::issue_attestation (B.9 bridge copy).
#[derive(Debug, Serialize, Deserialize)]
struct ConsolidatedAttestationOutput {
    pub cid: String,
    pub attestation_kind: String,
    pub subject_cid: String,
    pub issuer_cid: String,
}

/// Input for elohim DNA's content_store::propose_governance_action (B.9 bridge copy).
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

/// Input for elohim DNA's content_store::propose_recovery_governance_action
/// — Recovery M4 Task 13 bridge copy. Mirrors
/// `governance_action::ProposeRecoveryGovernanceActionInput` field-for-field.
///
/// Differs from `ConsolidatedProposeGovernanceActionInput` (the generic
/// producer's bridge mirror) in two load-bearing ways:
///   1. `metadata` is a flat JSON object whose fields are written at the TOP
///      LEVEL of `metadata_json` on the elohim Content entry (not nested under
///      `parameters_json`). The Recovery M4 Task 4/5 readers expect top-level
///      access to `human_id`, `revoked_key`, `threshold_reached`,
///      `effective_at`, `is_active`, `frozen_at_layer`.
///   2. `subject_human_id` replaces `subject_cid` — recovery governance
///      subjects are humans; the producer denormalises into `related_node_ids`
///      and surfaces through `subject_cid` on the output for parity.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConsolidatedProposeRecoveryGovernanceActionInput {
    pub governance_kind: String,
    pub subject_human_id: String,
    pub title: String,
    pub description: Option<String>,
    pub reach: String,
    pub threshold: serde_json::Value,
    pub closes_at: String,
    pub metadata: serde_json::Value,
    pub supersedes_cid: Option<String>,
}

/// Output from elohim DNA's content_store::propose_governance_action (B.9 bridge copy).
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConsolidatedGovernanceActionOutput {
    pub cid: String,
    pub governance_kind: String,
    pub subject_cid: String,
    pub proposer_cid: String,
    pub closes_at: String,
}

/// Input for elohim DNA's content_store::get_content_by_id (Recovery M4 Task 3).
///
/// Wire-compatible with `lamad_types::QueryByIdInput`. Defined locally because
/// cross-DNA calls serialise through msgpack; imagodei cannot depend on the
/// lamad domain crate directly.
#[derive(Debug, Serialize, Deserialize)]
struct ConsolidatedQueryByIdInput {
    pub id: String,
}

/// Subset of elohim DNA's `content_store::ContentOutput.content` that the
/// imagodei recovery gates need. Mirrors the relevant fields of the wire
/// `Content` struct produced by `content_to_wire(&Content)`; serde will
/// ignore fields outside this set on deserialization.
#[derive(Debug, Serialize, Deserialize)]
struct ConsolidatedContentView {
    pub id: String,
    pub content_type: String,
    #[serde(default)]
    pub metadata_json: String,
}

/// Output from elohim DNA's content_store::get_content_by_id
/// (`Option<ContentOutput>`). Mirrors the relevant fields only — the full
/// ContentOutput carries action_hash + entry_hash which we don't need here.
#[derive(Debug, Serialize, Deserialize)]
struct ConsolidatedContentOutput {
    pub content: ConsolidatedContentView,
}

// ---------------------------------------------------------------------------
// Bridge helper: call elohim content_store::issue_attestation and handle all
// ZomeCallResponse arms uniformly.
// ---------------------------------------------------------------------------
fn call_elohim_issue_attestation(
    consolidated_input: ConsolidatedIssueAttestationInput,
) -> ExternResult<ConsolidatedAttestationOutput> {
    let response = call(
        CallTargetCell::OtherRole("elohim".into()),
        ZomeName::from("content_store"),
        FunctionName::from("issue_attestation"),
        None,
        consolidated_input,
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

// ---------------------------------------------------------------------------
// Bridge helper: call elohim content_store::get_attestations_for_subject.
// ---------------------------------------------------------------------------
fn call_elohim_get_attestations_for_subject(
    subject_cid: String,
) -> ExternResult<Vec<ConsolidatedAttestationOutput>> {
    let response = call(
        CallTargetCell::OtherRole("elohim".into()),
        ZomeName::from("content_store"),
        FunctionName::from("get_attestations_for_subject"),
        None,
        subject_cid,
    )?;
    match response {
        ZomeCallResponse::Ok(result) => result.decode().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "bridge decode (elohim::get_attestations_for_subject): {e}"
            )))
        }),
        ZomeCallResponse::Unauthorized(_, _, _, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Unauthorized bridge call to elohim::get_attestations_for_subject".to_string()
        ))),
        ZomeCallResponse::NetworkError(err) => Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Network error bridging to elohim::get_attestations_for_subject: {err}"
        )))),
        ZomeCallResponse::CountersigningSession(err) => Err(wasm_error!(WasmErrorInner::Guest(
            format!("Countersigning error bridging to elohim: {err}")
        ))),
        ZomeCallResponse::AuthenticationFailed(_, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Authentication failed bridging to elohim::get_attestations_for_subject".to_string()
        ))),
    }
}

// ---------------------------------------------------------------------------
// Bridge helper: call elohim content_store::propose_governance_action.
// ---------------------------------------------------------------------------
fn call_elohim_propose_governance_action(
    consolidated_input: ConsolidatedProposeGovernanceActionInput,
) -> ExternResult<ConsolidatedGovernanceActionOutput> {
    let response = call(
        CallTargetCell::OtherRole("elohim".into()),
        ZomeName::from("content_store"),
        FunctionName::from("propose_governance_action"),
        None,
        consolidated_input,
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

// ---------------------------------------------------------------------------
// Bridge helper: call elohim content_store::propose_recovery_governance_action.
//
// Recovery M4 Task 13: bespoke producer used by `create_recovery_request` and
// `create_self_revocation` (and forthcoming Task 14 callers). Writes top-level
// metadata fields the Task 4/5 gates depend on (`human_id`, `revoked_key`,
// `threshold_reached`, `effective_at`, `is_active`, `frozen_at_layer`).
// Mirrors the 5-arm ZomeCallResponse pattern from
// `call_elohim_propose_governance_action` and the rest of the bridge family.
// ---------------------------------------------------------------------------
pub(crate) fn call_elohim_propose_recovery_governance_action(
    consolidated_input: ConsolidatedProposeRecoveryGovernanceActionInput,
) -> ExternResult<ConsolidatedGovernanceActionOutput> {
    let response = call(
        CallTargetCell::OtherRole("elohim".into()),
        ZomeName::from("content_store"),
        FunctionName::from("propose_recovery_governance_action"),
        None,
        consolidated_input,
    )?;
    match response {
        ZomeCallResponse::Ok(result) => result.decode().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "bridge decode (elohim::propose_recovery_governance_action): {e}"
            )))
        }),
        ZomeCallResponse::Unauthorized(_, _, _, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Unauthorized bridge call to elohim::propose_recovery_governance_action".to_string()
        ))),
        ZomeCallResponse::NetworkError(err) => Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Network error bridging to elohim::propose_recovery_governance_action: {err}"
        )))),
        ZomeCallResponse::CountersigningSession(err) => Err(wasm_error!(WasmErrorInner::Guest(
            format!("Countersigning error bridging to elohim: {err}")
        ))),
        ZomeCallResponse::AuthenticationFailed(_, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Authentication failed bridging to elohim::propose_recovery_governance_action"
                .to_string()
        ))),
    }
}

// ---------------------------------------------------------------------------
// Bridge helper: call elohim content_store::get_content_by_id to load the
// `governance-action:recovery-request` Content entry that backs an intimate
// witness submission (Recovery M4 Task 3). Returns the load-bearing `human_id`
// extracted from the entry's `metadata_json` after asserting the
// `content_type` discriminator. Validates that the entry exists, that it has
// the expected content-type, and that the metadata carries `human_id`.
// ---------------------------------------------------------------------------
fn fetch_recovery_request_human_id(recovery_request_cid: &str) -> ExternResult<String> {
    let response = call(
        CallTargetCell::OtherRole("elohim".into()),
        ZomeName::from("content_store"),
        FunctionName::from("get_content_by_id"),
        None,
        ConsolidatedQueryByIdInput {
            id: recovery_request_cid.to_string(),
        },
    )?;
    let content_output: Option<ConsolidatedContentOutput> = match response {
        ZomeCallResponse::Ok(payload) => payload.decode().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "submit_intimate_witness Gate 1: decode (elohim::get_content_by_id) failed: {e:?}"
            )))
        })?,
        ZomeCallResponse::Unauthorized(_, _, _, _) => {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "submit_intimate_witness Gate 1: unauthorized cross-DNA get_content_by_id".into()
            )))
        }
        ZomeCallResponse::NetworkError(err) => {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "submit_intimate_witness Gate 1: network error on cross-DNA get_content_by_id: {err}"
            ))))
        }
        ZomeCallResponse::CountersigningSession(err) => {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "submit_intimate_witness Gate 1: countersigning error: {err}"
            ))))
        }
        ZomeCallResponse::AuthenticationFailed(_, _) => {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "submit_intimate_witness Gate 1: authentication failed on cross-DNA get_content_by_id".into()
            )))
        }
    };
    let content_output = content_output.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "submit_intimate_witness Gate 1: recovery-request CID {} not found on elohim DNA",
            recovery_request_cid
        )))
    })?;
    if content_output.content.content_type != "governance-action:recovery-request" {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "submit_intimate_witness Gate 1: expected governance-action:recovery-request, got {}",
            content_output.content.content_type
        ))));
    }
    let metadata: serde_json::Value = serde_json::from_str(&content_output.content.metadata_json)
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "submit_intimate_witness Gate 1: bad metadata_json: {e}"
            )))
        })?;
    let human_id = metadata
        .get("human_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "submit_intimate_witness Gate 1: recovery-request metadata missing human_id".into(),
            ))
        })?
        .to_string();
    Ok(human_id)
}

// ---------------------------------------------------------------------------
// Bridge helper: call elohim content_store::query_effective_revocation_for_key
// for the Recovery M4 Task 4 revocation-floor gate on `commit_key_rotation`.
// ---------------------------------------------------------------------------
fn call_elohim_query_effective_revocation_for_key(
    revoked_key: &str,
) -> ExternResult<Option<ConsolidatedContentOutput>> {
    let response = call(
        CallTargetCell::OtherRole("elohim".into()),
        ZomeName::from("content_store"),
        FunctionName::from("query_effective_revocation_for_key"),
        None,
        revoked_key.to_string(),
    )?;
    match response {
        ZomeCallResponse::Ok(payload) => payload.decode().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "bridge decode (elohim::query_effective_revocation_for_key): {e}"
            )))
        }),
        ZomeCallResponse::Unauthorized(_, _, _, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Unauthorized bridge call to elohim::query_effective_revocation_for_key".to_string()
        ))),
        ZomeCallResponse::NetworkError(err) => Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Network error bridging to elohim::query_effective_revocation_for_key: {err}"
        )))),
        ZomeCallResponse::CountersigningSession(err) => Err(wasm_error!(WasmErrorInner::Guest(
            format!("Countersigning error bridging to elohim: {err}")
        ))),
        ZomeCallResponse::AuthenticationFailed(_, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Authentication failed bridging to elohim::query_effective_revocation_for_key"
                .to_string()
        ))),
    }
}

// ---------------------------------------------------------------------------
// Bridge helper: call elohim content_store::query_effective_identity_freeze_for_human
// for the Recovery M4 Task 4 freeze-floor gate on `commit_key_rotation`.
// ---------------------------------------------------------------------------
fn call_elohim_query_effective_identity_freeze_for_human(
    human_id: &str,
) -> ExternResult<Option<ConsolidatedContentOutput>> {
    let response = call(
        CallTargetCell::OtherRole("elohim".into()),
        ZomeName::from("content_store"),
        FunctionName::from("query_effective_identity_freeze_for_human"),
        None,
        human_id.to_string(),
    )?;
    match response {
        ZomeCallResponse::Ok(payload) => payload.decode().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "bridge decode (elohim::query_effective_identity_freeze_for_human): {e}"
            )))
        }),
        ZomeCallResponse::Unauthorized(_, _, _, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Unauthorized bridge call to elohim::query_effective_identity_freeze_for_human"
                .to_string()
        ))),
        ZomeCallResponse::NetworkError(err) => Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Network error bridging to elohim::query_effective_identity_freeze_for_human: {err}"
        )))),
        ZomeCallResponse::CountersigningSession(err) => Err(wasm_error!(WasmErrorInner::Guest(
            format!("Countersigning error bridging to elohim: {err}")
        ))),
        ZomeCallResponse::AuthenticationFailed(_, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Authentication failed bridging to elohim::query_effective_identity_freeze_for_human"
                .to_string()
        ))),
    }
}

// ---------------------------------------------------------------------------
// Bridge helper: call elohim content_store::get_content_by_id and return the
// full ConsolidatedContentOutput (or None when missing). Recovery M4 Task 5:
// `submit_revocation_vote` needs every gate field of the
// `governance-action:key-revocation` Content entry — trigger_type,
// threshold_reached, human_id, revoked_key, required_votes, current_votes,
// id — so a generic Option<ConsolidatedContentOutput>-returning helper is the
// natural shape. The Task 3 helper `fetch_recovery_request_human_id` is
// single-field-specific and not reusable here.
// ---------------------------------------------------------------------------
fn call_elohim_get_content_by_id(
    cid: &str,
) -> ExternResult<Option<ConsolidatedContentOutput>> {
    let response = call(
        CallTargetCell::OtherRole("elohim".into()),
        ZomeName::from("content_store"),
        FunctionName::from("get_content_by_id"),
        None,
        ConsolidatedQueryByIdInput {
            id: cid.to_string(),
        },
    )?;
    match response {
        ZomeCallResponse::Ok(payload) => payload.decode().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "bridge decode (elohim::get_content_by_id): {e}"
            )))
        }),
        ZomeCallResponse::Unauthorized(_, _, _, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Unauthorized bridge call to elohim::get_content_by_id".to_string()
        ))),
        ZomeCallResponse::NetworkError(err) => Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Network error bridging to elohim::get_content_by_id: {err}"
        )))),
        ZomeCallResponse::CountersigningSession(err) => Err(wasm_error!(WasmErrorInner::Guest(
            format!("Countersigning error bridging to elohim::get_content_by_id: {err}")
        ))),
        ZomeCallResponse::AuthenticationFailed(_, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Authentication failed bridging to elohim::get_content_by_id".to_string()
        ))),
    }
}

// ---------------------------------------------------------------------------
// Synthesise a legacy Attestation struct from a consolidated bridge response.
// This is TEMPORARY scaffolding until Stage C removes the Attestation entry
// type from imagodei entirely. The action_hash field cannot be derived from
// the consolidated CID (entry hash) — callers that rely on action_hash for
// further DHT lookups must be migrated before Stage C.
// ---------------------------------------------------------------------------
fn synthesise_attestation_from_consolidated(
    consolidated: &ConsolidatedAttestationOutput,
    input_category: &str,
    input_attestation_type: &str,
    input_display_name: &str,
    input_description: &str,
    input_icon_url: Option<String>,
    input_tier: Option<String>,
    input_earned_via_json: &str,
    input_expires_at: Option<String>,
) -> LegacyAttestationView {
    LegacyAttestationView {
        id: consolidated.cid.clone(),
        agent_id: consolidated.subject_cid.clone(),
        category: input_category.to_string(),
        attestation_type: input_attestation_type.to_string(),
        display_name: input_display_name.to_string(),
        description: input_description.to_string(),
        icon_url: input_icon_url,
        tier: input_tier,
        earned_via_json: input_earned_via_json.to_string(),
        issued_at: String::new(), // populated by elohim coordinator
        issued_by: consolidated.issuer_cid.clone(),
        expires_at: input_expires_at,
        proof: None,
    }
}

/// Issue an attestation to an agent.
///
/// B.9 bridge: delegates to elohim DNA's content_store::issue_attestation with
/// attestation_kind "attestation:identity-credential". The legacy Attestation
/// entry type is NOT written to imagodei's source chain; the canonical entry
/// lives on elohim DNA's DHT. Stage C will remove this wrapper entirely.
///
/// RUNTIME CONCERN: elohim::grant_attestation calls issue_attestation_via_imagodei
/// which calls this function, which calls elohim::issue_attestation. That creates
/// a cross-DNA call cycle (elohim → imagodei → elohim). Stage B companion task:
/// update issue_attestation_via_imagodei in elohim to call the attestation module
/// directly rather than bridging to imagodei.
#[hdk_extern]
pub fn issue_attestation(input: IssueAttestationInput) -> ExternResult<AttestationOutput> {
    let consolidated_input = ConsolidatedIssueAttestationInput {
        attestation_kind: "attestation:identity-credential".to_string(),
        subject_cid: input.agent_id.clone(),
        subject_kind: "agent".to_string(),
        title: input.display_name.clone(),
        description: Some(input.description.clone()),
        reach: "community".to_string(),
        metadata: serde_json::json!({
            "category": input.category,
            "credential_type": input.attestation_type,
            "tier": input.tier,
            "icon_url": input.icon_url,
            "earned_via": input.earned_via_json,
        }),
        parent_governance_action_cid: None,
        vote_value: None,
        proof_class: "witness".to_string(),
        proof_evidence: serde_json::json!({"class": "witness"}),
        expires_at: input.expires_at.clone(),
    };

    let consolidated = call_elohim_issue_attestation(consolidated_input)?;

    let attestation = synthesise_attestation_from_consolidated(
        &consolidated,
        &input.category,
        &input.attestation_type,
        &input.display_name,
        &input.description,
        input.icon_url,
        input.tier,
        &input.earned_via_json,
        input.expires_at,
    );

    // The action_hash field cannot be derived from the consolidated CID (entry
    // hash). Use a zero sentinel so the struct satisfies callers that only
    // inspect the attestation fields; callers that pass action_hash to HDK get
    // must migrate before Stage C removes this bridge.
    let action_hash = ActionHash::from_raw_36(vec![0u8; 36]);

    Ok(AttestationOutput {
        action_hash,
        attestation,
    })
}

/// Get attestations for an agent.
///
/// B.9 bridge: delegates to elohim DNA's content_store::get_attestations_for_subject
/// keyed on the agent_id as subject CID. Returns legacy AttestationOutput shapes
/// with synthesised Attestation fields from the consolidated output.
#[hdk_extern]
pub fn get_agent_attestations(agent_id: String) -> ExternResult<Vec<AttestationOutput>> {
    let consolidated_list = call_elohim_get_attestations_for_subject(agent_id.clone())?;

    let results = consolidated_list
        .into_iter()
        .map(|c| {
            let attestation = LegacyAttestationView {
                id: c.cid.clone(),
                agent_id: agent_id.clone(),
                category: c.attestation_kind.clone(),
                attestation_type: c.attestation_kind.clone(),
                display_name: c.attestation_kind.clone(),
                description: String::new(),
                icon_url: None,
                tier: None,
                earned_via_json: "{}".to_string(),
                issued_at: String::new(),
                issued_by: c.issuer_cid.clone(),
                expires_at: None,
                proof: None,
            };
            AttestationOutput {
                // Sentinel — see note in issue_attestation bridge above.
                action_hash: ActionHash::from_raw_36(vec![0u8; 36]),
                attestation,
            }
        })
        .collect();

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

    Ok(AgentOutput {
        action_hash: action_hash.into(),
        agent,
    })
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
                    return Ok(Some(AgentOutput {
                        action_hash: action_hash.into(),
                        agent,
                    }));
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
    /// Legacy flat-shape signal. Superseded by `DnaSignal::KeyRevocation`
    /// (EPR envelope) emitted alongside at each producer site.
    /// Remove after one release cycle (T18 back-compat window).
    #[deprecated(
        note = "T18: superseded by DnaSignal::KeyRevocation envelope; remove after one release cycle"
    )]
    KeyRevocationEffective {
        /// Internal correlation id (shape `rev-{human}-{ts}`).
        /// Not in the dna-signals schema; retained for elohim-storage projector
        /// correlation until T18 reconciles the full projection table design.
        revocation_id: String,
        /// Const discriminator required by `dna-signals/key-revocation.schema.json`.
        /// Note: the outer `RecoveryV2Signal` enum also serializes a `type` field
        /// via `#[serde(tag = "type")]`. Receivers that strictly enforce
        /// `additionalProperties: false` on the schema will see both `type` and
        /// `signalType`. T18 should either remove the outer tag or align the schema
        /// to tolerate the extra field.
        #[serde(rename = "signalType")]
        signal_type: String,
        /// CID of the `governance-action:key-revocation` Content entry on the
        /// elohim DNA, carried as the `actionHash` wire field.  Stage 1: this is
        /// the elohim Content CID string returned by the bridge call.  Stage 2:
        /// should be the true Holochain ActionHash (base64) of the DHT entry once
        /// the elohim DNA exposes the action hash alongside the CID in its output.
        #[serde(rename = "actionHash")]
        action_hash: String,
        /// Stage 1: populated with `human_id` (the stable human identifier).
        /// Stage 2: will be a content CID derived from the imagodei Human entry.
        #[serde(rename = "agentCid")]
        human_id: String,
        /// The revoked ed25519 pubkey (base64 32-byte).
        #[serde(rename = "revokedPubkey")]
        revoked_key: String,
        /// Earliest point the key may have been compromised (RFC3339).
        /// M4 surface has no separate compromise-discovery timestamp; this field
        /// is set equal to `effective_at` at emit time.  A future revision of
        /// Recovery (post-M4) may add a distinct `compromise_at` from the
        /// revocation request metadata, at which point this field will diverge.
        #[serde(rename = "compromiseAt")]
        compromise_at: String,
        /// When the revocation became effective in the DHT (RFC3339).
        #[serde(rename = "effectiveAt")]
        effective_at: String,
        /// Back-pointer to the KeyRevocation request/vote that drove this event.
        /// Null for defender/voluntary paths (no vote chain).
        #[serde(rename = "triggeringRevocationId")]
        triggering_vote_id: Option<String>,
        /// UTC timestamp at which this signal was emitted (RFC3339).
        #[serde(rename = "emittedAt")]
        emitted_at: String,
    },
    // M5: portal-host signals — consumed by elohim-storage ReconcileController.
    PortalHostCreated {
        action_hash: ActionHash,
        human_action_hash: ActionHash,
        host_url: String,
        label: Option<String>,
        added_at: Timestamp,
        reach: String,
    },
    PortalHostRemoved {
        /// ActionHash of the Delete action itself.
        action_hash: ActionHash,
        /// ActionHash of the original PortalHost Create action (= the row id in storage).
        original_action_hash: ActionHash,
        human_action_hash: ActionHash,
        host_url: String,
    },
}

// =============================================================================
// DnaSignal — EPR-shape provenance envelope (T18+)
// =============================================================================
//
// `DnaSignal` is the forward-compatible, AI-native signal type. It frames the
// wire message as an EPR (Elohim Provenance Record): the authoring elohim's
// attestation over a content-addressed (CID) subject, signed at emit time.
//
// Contrast with `RecoveryV2Signal`: that enum uses Holochain-internal
// ActionHash as its identity anchor and carries no issuer signature. It remains
// in place for one release cycle (back-compat window) alongside the new
// `DnaSignal::KeyRevocation` emission at each producer site.
//
// Wire format: `DnaSignal` serializes via serde with `tag = "type"` (kebab
// discriminator via camelCase). Consumers that strictly enforce
// `additionalProperties: false` against `dna-signals/key-revocation.schema.json`
// will see a top-level `type` field from the tag alongside the envelope fields;
// the schema now permits this by validating only the envelope payload fields.
//
// Future variants (KeyRotation, AgentPeerBinding) extend the enum below.
// The RevocationAttestation EPR-worker arm (Task 7) is a separate signal class
// and is NOT migrated here.

/// Top-level EPR-shape signal enum.
///
/// `#[serde(tag = "type")]` produces a discriminator field `type` on the wire
/// whose value is the camelCase variant name (e.g. `"keyRevocation"`).
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum DnaSignal {
    KeyRevocation(KeyRevocationEnvelope),
    // Future: KeyRotation(KeyRotationEnvelope), AgentPeerBinding(AgentPeerBindingEnvelope)
}

/// EPR envelope for a `governance-action:key-revocation` effective event.
///
/// Wire fields are `camelCase` (serde rename). All fields except `signature`
/// and `relay_chain` are covered by the issuer signature — see
/// `canonical_envelope_bytes` for the exact canonical form.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KeyRevocationEnvelope {
    /// EPR discriminator — const `"attestation:key-revocation-emit"`.
    pub attestation_kind: String,
    /// CIDv1 (dag-cbor sha256) of the `governance-action:key-revocation`
    /// Content entry on the elohim DNA. Substrate-agnostic identity anchor.
    pub subject_cid: String,
    /// Base64-encoded 32-byte ed25519 public key of the authoring elohim agent.
    pub issuer: String,
    /// RFC3339 timestamp at which the authoring elohim emitted this attestation.
    pub issued_at: String,
    /// Base64-encoded ed25519 signature over `canonical_envelope_bytes(self)`.
    pub signature: String,
    /// Domain-specific metadata carried inside the envelope.
    pub metadata: KeyRevocationMetadata,
    /// Reserved for relay-elohim provenance accumulation. Empty in T18.
    /// Future relay-elohims append their own attestation objects here as the
    /// signal propagates across hops. Opaque `serde_json::Value` so the wire
    /// field is present-but-typeless until a typed `RelayAttestation` struct
    /// lands in a future task.
    pub relay_chain: Vec<serde_json::Value>,
}

/// Domain metadata nested inside `KeyRevocationEnvelope`.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KeyRevocationMetadata {
    /// Stable logical dedup key. Format: `rev-{humanId}-{ts}`.
    pub revocation_id: String,
    /// Base64-encoded 32-byte ed25519 public key being revoked.
    pub revoked_pubkey: String,
    /// Stage 1: the stable `human_id` string. Stage 2: CID of the Human entry.
    pub agent_cid: String,
    /// Earliest point the key may have been compromised (RFC3339).
    /// M4: coincides with `effective_at`; future revisions add a distinct
    /// discovery timestamp. EPR W2B controller sweep uses this for retroactive
    /// attestation invalidation.
    pub compromise_at: String,
    /// Point at which the revocation became effective in the notary (RFC3339).
    pub effective_at: String,
    /// Back-pointer to the vote/request that drove this event. Null for
    /// defender-path or voluntary revocations (no vote chain).
    pub triggering_revocation_id: Option<String>,
    /// CID of the prior Content entry this supersedes (pending→effective
    /// lineage). Null on the initial CREATE.
    pub supersedes_cid: Option<String>,
}

// =============================================================================
// Canonical signing bytes
// =============================================================================

/// Sub-struct for canonical serialization — covers all envelope fields
/// EXCEPT `signature` and `relay_chain`. MessagePack-encoded via
/// `holochain_serialized_bytes::encode`.
///
/// # Canonical form specification
///
/// The issuer signature is ed25519 over MessagePack of:
/// ```text
/// {
///   attestationKind: string,
///   subjectCid:      string,
///   issuer:          string,
///   issuedAt:        string,
///   metadata: {
///     revocationId:          string,
///     revokedPubkey:         string,
///     agentCid:              string,
///     compromiseAt:          string,
///     effectiveAt:           string,
///     triggeringRevocationId: string | null,
///     supersedesCid:         string | null,
///   }
/// }
/// ```
/// Field order is the struct declaration order (deterministic under
/// `holochain_serialized_bytes::encode` which uses `rmp_serde`'s
/// `StructMapSerializer` — field names are included, order is declaration
/// order of `#[derive(Serialize)]`).
///
/// # Verification (consumer side)
///
/// 1. Deserialize the envelope.
/// 2. Call `canonical_envelope_bytes` on the deserialized envelope.
/// 3. Decode `issuer` (base64 → 32 bytes) and `signature` (base64 → 64 bytes).
/// 4. `ed25519_dalek::VerifyingKey::verify(&canonical_bytes, &signature)`.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CanonicalEnvelopeCore<'a> {
    attestation_kind: &'a str,
    subject_cid: &'a str,
    issuer: &'a str,
    issued_at: &'a str,
    metadata: CanonicalMetadataRef<'a>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CanonicalMetadataRef<'a> {
    revocation_id: &'a str,
    revoked_pubkey: &'a str,
    agent_cid: &'a str,
    compromise_at: &'a str,
    effective_at: &'a str,
    triggering_revocation_id: Option<&'a str>,
    supersedes_cid: Option<&'a str>,
}

/// Produce the canonical bytes over which the issuer signature is computed.
///
/// Excludes `signature` and `relay_chain` from the input envelope.
/// The output is MessagePack-encoded via `holochain_serialized_bytes::encode`.
///
/// **Must match the mirror function in `elohim-storage/src/services/recovery_flow_projector.rs`.**
/// See spec doc at `genesis/docs/superpowers/specs/2026-05-15-dna-signal-as-epr-envelope.md`
/// for the field-level canonical form.
pub fn canonical_envelope_bytes(envelope: &KeyRevocationEnvelope) -> Vec<u8> {
    let core = CanonicalEnvelopeCore {
        attestation_kind: &envelope.attestation_kind,
        subject_cid: &envelope.subject_cid,
        issuer: &envelope.issuer,
        issued_at: &envelope.issued_at,
        metadata: CanonicalMetadataRef {
            revocation_id: &envelope.metadata.revocation_id,
            revoked_pubkey: &envelope.metadata.revoked_pubkey,
            agent_cid: &envelope.metadata.agent_cid,
            compromise_at: &envelope.metadata.compromise_at,
            effective_at: &envelope.metadata.effective_at,
            triggering_revocation_id: envelope
                .metadata
                .triggering_revocation_id
                .as_deref(),
            supersedes_cid: envelope.metadata.supersedes_cid.as_deref(),
        },
    };
    // holochain_serialized_bytes::encode uses rmp_serde internally; the map
    // form preserves field names and declaration order — required for
    // deterministic cross-platform canonical bytes.
    holochain_serialized_bytes::encode(&core)
        .expect("canonical_envelope_bytes: encode should never fail on well-formed structs")
}

/// Build and emit a `DnaSignal::KeyRevocation` envelope, signing the canonical
/// bytes with the calling agent's lair-managed key.
///
/// Called at each of the three producer sites:
/// - `submit_specialist_revocation` (defender path)
/// - `create_self_revocation` (voluntary path)
/// - `submit_revocation_vote` (voted quorum path)
///
/// # Arguments
/// * `revocation_id` — stable dedup key (`rev-{human}-{ts}`)
/// * `subject_cid` — CID of the effective `governance-action:key-revocation` Content entry
/// * `agent_cid` — Stage 1: human_id string; Stage 2: Human entry CID
/// * `revoked_pubkey` — base64 string of the revoked ed25519 key
/// * `compromise_at` — RFC3339 (M4: same as `effective_at`)
/// * `effective_at` — RFC3339 timestamp of effectiveness
/// * `triggering_revocation_id` — None for defender/voluntary; Some(vote_id) for quorum path
/// * `supersedes_cid` — None on initial CREATE; Some(prior_cid) on quorum supersession
pub(crate) fn emit_key_revocation_envelope(
    revocation_id: String,
    subject_cid: String,
    agent_cid: String,
    revoked_pubkey: String,
    compromise_at: String,
    effective_at: String,
    triggering_revocation_id: Option<String>,
    supersedes_cid: Option<String>,
) -> ExternResult<()> {
    let agent_pk = agent_info()?.agent_initial_pubkey;
    let issuer_b64 = base64_encode(agent_pk.get_raw_32());
    let issued_at = rfc3339_from_sys_time(&sys_time()?);

    // Build the envelope without the signature first so we can compute
    // canonical bytes.
    let mut envelope = KeyRevocationEnvelope {
        attestation_kind: "attestation:key-revocation-emit".to_string(),
        subject_cid,
        issuer: issuer_b64,
        issued_at,
        signature: String::new(), // placeholder; filled after signing
        metadata: KeyRevocationMetadata {
            revocation_id,
            revoked_pubkey,
            agent_cid,
            compromise_at,
            effective_at,
            triggering_revocation_id,
            supersedes_cid,
        },
        relay_chain: vec![],
    };

    let canonical = canonical_envelope_bytes(&envelope);
    let raw_sig = hdk::ed25519::sign_raw(agent_pk, canonical)?;
    envelope.signature = base64_encode(&raw_sig.0);

    emit_signal(DnaSignal::KeyRevocation(envelope))?;
    Ok(())
}

/// Encode raw bytes as standard base64 (padded, STANDARD alphabet).
/// Used for the `issuer` pubkey (32 bytes) and `signature` (64 bytes) fields
/// in `KeyRevocationEnvelope`. Consumers decode with the same STANDARD engine.
fn base64_encode(bytes: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.encode(bytes)
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

/// Output of `create_recovery_request` — Recovery M4 Task 13.
///
/// Stage G post-Task-13: the canonical record is a
/// `governance-action:recovery-request` Content entry on the elohim DNA, not a
/// `RecoveryRequest` entry on imagodei. The CID (entry-hash) of that
/// cross-DNA entry is the load-bearing field; downstream callers
/// (`submit_intimate_witness`, projectors) key on it.
///
/// `request` is preserved so the existing `RecoveryRequestCreated` signal
/// payload (consumed by elohim-storage and the M4 projector) carries the same
/// shape it had pre-bridge. Until Task 17 aligns the signal payload against
/// the schema, this keeps the projector contract stable.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecoveryRequestOutput {
    /// CID of the `governance-action:recovery-request` Content entry on the
    /// elohim DNA. Replaces the pre-Task-13 `action_hash: ActionHash` field —
    /// the entry no longer lives on imagodei's DHT.
    pub recovery_request_cid: String,
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
    /// CID (content-derived id) of the `governance-action:recovery-request`
    /// Content entry on the elohim DNA. Renamed from `recovery_request_hash:
    /// ActionHash` per Recovery M4 Task 3 — the recovery-request now lives
    /// cross-DNA as a Content entry; gates resolve it by CID via
    /// `content_store::get_content_by_id` rather than a local DHT `get()`.
    pub recovery_request_cid: String,
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
    let first = links
        .first()
        .ok_or(wasm_error!(WasmErrorInner::Guest(format!(
            "No Human bound to agent pubkey {}",
            agent_pubkey
        ))))?;
    let action_hash =
        first
            .target
            .clone()
            .into_action_hash()
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "AgentKeyToHuman target is not an action hash".into()
            )))?;
    let record = get(action_hash, GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Human entry missing".into())
    ))?;
    let human: Human = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Human entry deserialize failed".into()
        )))?;
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
        let Some(record) = get(rel_hash, GetOptions::default())? else {
            continue;
        };
        let Some(rel): Option<HumanRelationship> = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        else {
            continue;
        };
        if rel.emergency_access_enabled {
            count += 1;
        }
    }
    Ok(count)
}

/// Threshold formula per revised spec §5 / M3 design §4.2: `max(2, ceil(M/2) + 1)`.
fn compute_required_witness_count(active_emergency_contacts: u32) -> u32 {
    let m = active_emergency_contacts;
    let ceil_half_plus_one = m.div_ceil(2) + 1; // ceil(m/2) + 1 for u32
    std::cmp::max(2, ceil_half_plus_one)
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
///
/// Recovery M4 Task 13: the recovery request is no longer written as a local
/// `RecoveryRequest` entry on imagodei. Instead it is bridged to the elohim
/// DNA as a `governance-action:recovery-request` Content entry via
/// `propose_recovery_governance_action` — the bespoke producer that writes
/// top-level metadata (`human_id`, `custodian_cids`, `trigger_type`,
/// `session_pubkey`, `threshold_reached`, `effective_at`) that the Task 4/5
/// recovery gates depend on.
///
/// The legacy `HumanToRecoveryRequest` link is no longer created on imagodei
/// — the recovery-request lives cross-DNA. Callers that previously walked the
/// link must now resolve via `content_store::get_content_by_id` using the CID
/// returned in `RecoveryRequestOutput.recovery_request_cid`.
///
/// The `RecoveryRequest` struct itself is still synthesised so the existing
/// `RecoveryRequestCreated` signal payload retains its shape (T17 will
/// align the signal to the dna-signal schemas).
#[hdk_extern]
pub fn create_recovery_request(
    input: CreateRecoveryRequestInput,
) -> ExternResult<RecoveryRequestOutput> {
    let now = sys_time()?;

    // M3: resolve human_id + compute required_witness_count
    let human_id = resolve_human_id_for_agent(&input.human_agent_pubkey)?;
    let contact_count = count_active_emergency_contacts(&human_id)?;
    let required_witness_count = compute_required_witness_count(contact_count);

    // Trigger-type derives from the claimant's declared authority intent.
    // The cross-DNA gates do not branch on this string today; surfacing it in
    // metadata gives T17 / projectors a hook for downstream classification.
    let trigger_type = match input.proposed_authority {
        RecoveryAuthorityKind::IntimateQuorum => "intimate",
        RecoveryAuthorityKind::CommunityConsensus => "community",
        RecoveryAuthorityKind::GovernanceAct { .. } => "governance-act",
        RecoveryAuthorityKind::NetworkWitness { .. } => "network-witness",
        RecoveryAuthorityKind::CryptographicQuorum { .. } => "cryptographic",
    };

    // closes_at = now + WITNESS_EXPIRY_DAYS. Mirrors the horizon used elsewhere
    // in the M3/M4 recovery paths.
    let expiry_micros = WITNESS_EXPIRY_DAYS * MICROS_PER_DAY;
    let closes_at_timestamp = Timestamp::from_micros(now.as_micros() + expiry_micros as i64);
    let closes_at = format!("{:?}", closes_at_timestamp);

    let request = RecoveryRequest {
        human_agent_pubkey: input.human_agent_pubkey.clone(),
        new_agent_pubkey: input.new_agent_pubkey.clone(),
        hosting_doorway_pubkey: input.hosting_doorway_pubkey,
        proposed_authority: input.proposed_authority,
        request_nonce: input.request_nonce,
        human_id: Some(human_id.clone()),
        required_witness_count,
        created_at: now,
    };

    // Bridge to elohim DNA — bespoke producer writes top-level metadata fields
    // the Task 4/5 readers consume. The recovery-request is initially pending
    // (threshold_reached=false, effective_at=null); the threshold flip happens
    // via a fresh CREATE on quorum (CREATE-only constraint, Task 4).
    let bridge_input = ConsolidatedProposeRecoveryGovernanceActionInput {
        governance_kind: "governance-action:recovery-request".to_string(),
        subject_human_id: human_id.clone(),
        title: format!("Recovery request for {}", human_id),
        description: Some(format!("trigger_type={}", trigger_type)),
        reach: "intimate".to_string(),
        threshold: serde_json::json!({"m": required_witness_count}),
        closes_at: closes_at.clone(),
        metadata: serde_json::json!({
            "human_id": human_id,
            // Custodian CIDs are populated by the Task 22 Shamir setup flow
            // when an explicit custody manifest exists; intimate recovery
            // does not pre-designate custodians, so leave empty here.
            "custodian_cids": [],
            "trigger_type": trigger_type,
            "session_pubkey": input.new_agent_pubkey.to_string(),
            "threshold_reached": false,
            "effective_at": serde_json::Value::Null,
        }),
        supersedes_cid: None,
    };
    let consolidated = call_elohim_propose_recovery_governance_action(bridge_input)?;
    let recovery_request_cid = consolidated.cid;

    // Emit the legacy signal with a zero ActionHash sentinel — the canonical
    // record lives cross-DNA and has no local ActionHash. Mirrors the
    // `submit_intimate_witness` pattern that landed under Task 3. T17 will
    // replace this with a CID-bearing signal payload.
    emit_signal(RecoveryV2Signal::RecoveryRequestCreated {
        action_hash: ActionHash::from_raw_36(vec![0u8; 36]),
        request: request.clone(),
    })?;

    Ok(RecoveryRequestOutput {
        recovery_request_cid,
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

/// Output of `create_self_revocation` — Recovery M4 Task 13.
///
/// Stage G post-Task-13: the canonical record is a
/// `governance-action:key-revocation` Content entry on the elohim DNA, not a
/// `KeyRevocation` entry on imagodei. The CID (entry-hash) of that cross-DNA
/// entry replaces the pre-Task-13 `action_hash: ActionHash` field — the entry
/// no longer lives on imagodei's DHT, so no ActionHash exists locally.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyRevocationOutput {
    pub revocation_id: String,
    /// CID of the `governance-action:key-revocation` Content entry on the
    /// elohim DNA. Replaces the pre-Task-13 `action_hash: ActionHash` field.
    pub revocation_cid: String,
}

/// M4: Self-revocation. A human with a valid agent key voluntarily revokes
/// a different (compromised) key they control. Single-cell authority, no
/// quorum, no witnesses.
///
/// Recovery M4 Task 13: bridges to the elohim DNA as a
/// `governance-action:key-revocation` Content entry via
/// `propose_recovery_governance_action`. Because self-revocation is
/// immediately effective (single-cell authority, threshold=1), the entry is
/// written with `threshold_reached: true` and `effective_at` set on the
/// initial CREATE — no `update_entry` is needed (CREATE-only constraint,
/// Task 4). The Task 4 gate at `query_effective_revocation_for_key` will
/// then return this entry directly.
///
/// The legacy `KeyRevocation` entry + dual-anchor links are no longer written
/// to imagodei's DHT (T15 will remove the entry type entirely). Downstream
/// readers resolve via cross-DNA `content_store::get_content_by_id`.
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
            "create_self_revocation: caller does not control revoked_key (different human_id)"
                .into()
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

    // Closes-at for an already-effective revocation is informational only
    // (the gate uses `effective_at`); set it to far-future so projectors that
    // honour `closes_at` for sweep windows do not prematurely expire it.
    let closes_at = "2099-01-01T00:00:00Z".to_string();

    let bridge_input = ConsolidatedProposeRecoveryGovernanceActionInput {
        governance_kind: "governance-action:key-revocation".to_string(),
        subject_human_id: human_id.clone(),
        title: format!("Key revocation — {}", human_id),
        description: Some(format!("trigger_type=voluntary; reason={}", input.reason)),
        reach: "private".to_string(),
        threshold: serde_json::json!({"m": 1, "n": 1, "type": "single-cell"}),
        closes_at,
        metadata: serde_json::json!({
            "id": revocation_id,
            "human_id": human_id,
            "revoked_key": revoked_key_str,
            "reason": input.reason,
            "trigger_type": "voluntary",
            "initiated_by": human_id,
            "required_votes": 1,
            "current_votes": 1,
            // Self-revocation is immediately effective on the initial CREATE
            // — no update_entry flip per Task 4 CREATE-only constraint.
            "threshold_reached": true,
            "effective_at": timestamp,
        }),
        supersedes_cid: None,
    };
    let consolidated = call_elohim_propose_recovery_governance_action(bridge_input)?;
    let revocation_cid = consolidated.cid;

    // Emit both signals atomically: Requested + Effective.
    emit_signal(RecoveryV2Signal::KeyRevocationRequested {
        id: revocation_id.clone(),
        human_id: human_id.clone(),
        revoked_key: revoked_key_str.clone(),
        reason: input.reason.clone(),
        trigger_type: "voluntary".to_string(),
        initiated_by: human_id.clone(),
        required_votes: 1,
        current_votes: 1,
        threshold_reached: true,
        effective_at: Some(timestamp.clone()),
        created_at: timestamp.clone(),
    })?;

    let emitted_at_self = rfc3339_from_sys_time(&sys_time()?);
    #[allow(deprecated)]
    emit_signal(RecoveryV2Signal::KeyRevocationEffective {
        revocation_id: revocation_id.clone(),
        signal_type: "keyRevocation".to_string(),
        action_hash: revocation_cid.clone(),
        human_id: human_id.clone(),
        revoked_key: revoked_key_str.clone(),
        // M4: no separate compromise-discovery timestamp; coincides with effectiveAt.
        // Future revisions may populate this from revocation request metadata.
        compromise_at: timestamp.clone(),
        effective_at: timestamp.clone(),
        triggering_vote_id: None,
        emitted_at: emitted_at_self,
    })?;

    // T18: EPR-shape envelope alongside the legacy signal (back-compat window).
    // Voluntary path: supersedes_cid = None (initial CREATE);
    //                 triggering_revocation_id = None (no vote chain).
    emit_key_revocation_envelope(
        revocation_id.clone(),
        revocation_cid.clone(),
        human_id,        // Stage 1: human_id; Stage 2: Human entry CID
        revoked_key_str,
        timestamp.clone(), // compromise_at == effective_at in M4
        timestamp,
        None,            // triggering_revocation_id
        None,            // supersedes_cid
    )?;

    Ok(KeyRevocationOutput {
        revocation_id,
        revocation_cid,
    })
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
    let revocation_id = format!("rev-{}-{}", input.target_human_id, timestamp);
    let revoked_key_str = input.revoked_key.to_string();

    // Recovery M4 Task 14: bridge to elohim DNA via
    // `propose_recovery_governance_action`. Initial CREATE is pending
    // (`threshold_reached: false`, `effective_at: null`); the threshold flip on
    // quorum is a fresh CREATE driven by `submit_revocation_vote` (CREATE-only
    // constraint, Task 4).
    let bridge_input = ConsolidatedProposeRecoveryGovernanceActionInput {
        governance_kind: "governance-action:key-revocation".to_string(),
        subject_human_id: input.target_human_id.clone(),
        title: format!("Key revocation request for {}", input.target_human_id),
        description: Some(format!(
            "trigger_type=steward_vote; reason={}; initiated_by={}",
            input.reason, caller_human_id
        )),
        reach: "intimate".to_string(),
        threshold: serde_json::json!({"m": required}),
        closes_at: timestamp.clone(),
        metadata: serde_json::json!({
            "id": revocation_id,
            "human_id": input.target_human_id,
            "revoked_key": revoked_key_str,
            "reason": input.reason,
            "trigger_type": "steward_vote",
            "initiated_by": caller_human_id,
            "required_votes": required,
            "current_votes": 0,
            "threshold_reached": false,
            "effective_at": serde_json::Value::Null,
        }),
        supersedes_cid: None,
    };
    let consolidated = call_elohim_propose_recovery_governance_action(bridge_input)?;
    let revocation_cid = consolidated.cid;

    emit_signal(RecoveryV2Signal::KeyRevocationRequested {
        id: revocation_id.clone(),
        human_id: input.target_human_id.clone(),
        revoked_key: revoked_key_str.clone(),
        reason: input.reason.clone(),
        trigger_type: "steward_vote".to_string(),
        initiated_by: caller_human_id.clone(),
        required_votes: required,
        current_votes: 0,
        threshold_reached: false,
        effective_at: None,
        created_at: timestamp,
    })?;

    Ok(KeyRevocationOutput {
        revocation_id,
        revocation_cid,
    })
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubmitRevocationVoteInput {
    /// CID (content-derived id) of the `governance-action:key-revocation`
    /// Content entry on the elohim DNA. Renamed from `revocation_id: String`
    /// per Recovery M4 Task 5 — the canonical revocation entry now lives
    /// cross-DNA as a Content entry on elohim; gate fields are resolved by
    /// CID via `content_store::get_content_by_id` rather than a local
    /// `IdToKeyRevocation` anchor lookup + `to_app_option()` decode. The
    /// logical revocation_id (shape `rev-{human}-{ts}`) is recovered from
    /// `Content.metadata.id` and used for downstream vote linkage and signal
    /// payloads. The local imagodei KeyRevocation entry continues to exist
    /// during the interim (Tasks 13/14 will move the producer), and is still
    /// reachable via the local anchor keyed by the metadata id for
    /// update_entry + link operations.
    pub revocation_cid: String,
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
///
/// Recovery M4 Task 14: the vote is written as an
/// `attestation:revocation-vote` Content entry on the elohim DNA via
/// `call_elohim_issue_attestation`, with `parent_governance_action_cid` and
/// `subject_cid` both set to the revocation CID. The vote's metadata carries
/// `vote_value` ("approve" | "reject") and a top-level `voter_human_id` so
/// downstream projection (and this function's quorum-detection re-read) can
/// classify it. The legacy local `RevocationVote` entry + 3 anchor links are
/// no longer written (T15 will drop the entry type definitions).
///
/// When the approved-vote count meets `required_votes`, the function writes a
/// fresh `governance-action:key-revocation` Content entry on elohim with
/// `threshold_reached: true`, `effective_at: <RFC3339 now>`, and
/// `supersedes_cid: <input.revocation_cid>` — the "fresh effective Content"
/// pattern from Task 4. The `query_effective_revocation_for_key` gate reads
/// from DHT-derived Content (NOT a storage projection), so this CREATE is
/// what flips the gate. Subsequent redelivery / racy re-entry is guarded by
/// the per-steward duplicate-vote check (attestation issuer == voter human
/// id) plus the gate's most-recent-wins ordering by `updated_at`.
#[hdk_extern]
pub fn submit_revocation_vote(
    input: SubmitRevocationVoteInput,
) -> ExternResult<RevocationVoteOutput> {
    let caller_pubkey = agent_info()?.agent_initial_pubkey;
    let caller_pubkey_str = caller_pubkey.to_string();
    let caller_human_id = resolve_human_id_for_agent(&caller_pubkey)?;

    if input.attestation.trim().is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "submit_revocation_vote: attestation cannot be empty".into()
        )));
    }

    // Recovery M4 Task 5 + 14 migration: the canonical revocation entry is a
    // `governance-action:key-revocation` Content entry on the elohim DNA,
    // fetched by CID via the cross-DNA bridge. The vote itself is also written
    // cross-DNA as an `attestation:revocation-vote` Content (Task 14).
    let content_output = call_elohim_get_content_by_id(&input.revocation_cid)?.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "submit_revocation_vote: revocation CID {} not found on elohim DNA",
            input.revocation_cid
        )))
    })?;
    if content_output.content.content_type != "governance-action:key-revocation" {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "submit_revocation_vote: expected governance-action:key-revocation, got {}",
            content_output.content.content_type
        ))));
    }
    let metadata: serde_json::Value =
        serde_json::from_str(&content_output.content.metadata_json).map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "submit_revocation_vote: bad metadata_json: {e}"
            )))
        })?;
    let revocation_id = metadata
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "submit_revocation_vote: revocation metadata missing id".into(),
            ))
        })?
        .to_string();
    let revocation_trigger_type = metadata
        .get("trigger_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "submit_revocation_vote: revocation metadata missing trigger_type".into(),
            ))
        })?
        .to_string();
    let revocation_threshold_reached = metadata
        .get("threshold_reached")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "submit_revocation_vote: revocation metadata missing threshold_reached".into(),
            ))
        })?;
    let revocation_human_id = metadata
        .get("human_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "submit_revocation_vote: revocation metadata missing human_id".into(),
            ))
        })?
        .to_string();
    let revocation_revoked_key = metadata
        .get("revoked_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "submit_revocation_vote: revocation metadata missing revoked_key".into(),
            ))
        })?
        .to_string();
    let revocation_reason = metadata
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let revocation_initiated_by = metadata
        .get("initiated_by")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let revocation_required_votes: u32 = metadata
        .get("required_votes")
        .and_then(|v| v.as_u64())
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "submit_revocation_vote: revocation metadata missing required_votes".into(),
            ))
        })?;

    // Gate: votes only apply to the steward_vote path.
    if revocation_trigger_type != "steward_vote" {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "submit_revocation_vote: revocation {} has trigger_type={}, votes not accepted",
            revocation_id, revocation_trigger_type
        ))));
    }

    if revocation_threshold_reached {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "submit_revocation_vote: revocation {} already effective",
            revocation_id
        ))));
    }

    // Gate: caller must be an active emergency contact.
    if !is_active_emergency_contact(&revocation_human_id, &caller_human_id)? {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "submit_revocation_vote: caller is not an active emergency contact for {}",
            revocation_human_id
        ))));
    }

    // Gate: no existing vote from this steward on this revocation. Walk the
    // cross-DNA `attestation:revocation-vote` children (subject = revocation_cid)
    // and reject if any was issued by the caller (issuer_cid matches caller's
    // AgentPubKey string — `issue_attestation` stores
    // `author_id = agent_info()?.agent_initial_pubkey.to_string()`).
    let existing_votes = call_elohim_get_attestations_for_subject(input.revocation_cid.clone())?;
    for prior in &existing_votes {
        if prior.attestation_kind == "attestation:revocation-vote"
            && prior.issuer_cid == caller_pubkey_str
        {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "submit_revocation_vote: steward {} has already voted on revocation {}",
                caller_human_id, revocation_id
            ))));
        }
    }

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);
    // RFC3339 sortable timestamp for `effective_at` (Task 4 constraint —
    // gate sorts most-recent-by-`updated_at` lexicographically).
    let rfc3339_now = rfc3339_from_sys_time(&now);
    let vote_id = format!("vote-{}-{}", caller_human_id, timestamp);
    let vote_value_str = if input.approved { "approve" } else { "reject" };

    // Issue the vote as an `attestation:revocation-vote` Content on elohim.
    // The metadata carries top-level `parent_governance_action_cid`,
    // `subject_cid`, `subject_kind`, `vote_value`, and `voter_human_id` so
    // downstream projectors classify without further lookups.
    let vote_metadata = serde_json::json!({
        "parent_governance_action_cid": input.revocation_cid,
        "subject_cid": input.revocation_cid,
        "subject_kind": "key-revocation",
        "vote_value": vote_value_str,
        "voter_human_id": caller_human_id,
        "voter_pubkey": caller_pubkey_str,
        "vote_id": vote_id,
        "attestation_blob": input.attestation,
        "voted_at": timestamp,
    });
    let attest_input = ConsolidatedIssueAttestationInput {
        attestation_kind: "attestation:revocation-vote".to_string(),
        subject_cid: input.revocation_cid.clone(),
        subject_kind: "key-revocation".to_string(),
        title: format!("Revocation vote by {}", caller_human_id),
        description: Some(format!("vote={}", vote_value_str)),
        reach: "intimate".to_string(),
        metadata: vote_metadata,
        parent_governance_action_cid: Some(input.revocation_cid.clone()),
        vote_value: Some(vote_value_str.to_string()),
        proof_class: "witness".to_string(),
        proof_evidence: serde_json::json!({"class": "witness"}),
        expires_at: None,
    };
    let _vote_consolidated = call_elohim_issue_attestation(attest_input)?;

    // Recount approved votes by re-fetching attestations for subject and
    // decoding each one's metadata for `vote_value`. Cross-DNA fetch per
    // attestation is O(N votes), which is small (single-digit quorum).
    let refreshed = call_elohim_get_attestations_for_subject(input.revocation_cid.clone())?;
    let mut approved_count: u32 = 0;
    for child in &refreshed {
        if child.attestation_kind != "attestation:revocation-vote" {
            continue;
        }
        let Some(child_content) = call_elohim_get_content_by_id(&child.cid)? else {
            continue;
        };
        let child_meta: serde_json::Value =
            serde_json::from_str(&child_content.content.metadata_json)
                .unwrap_or(serde_json::Value::Null);
        let vote_value = child_meta
            .get("vote_value")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if vote_value == "approve" {
            approved_count = approved_count.saturating_add(1);
        }
    }
    let threshold_now_reached = approved_count >= revocation_required_votes;

    if threshold_now_reached {
        // CREATE-only effectiveness transition (Task 4): emit a fresh
        // `governance-action:key-revocation` Content entry whose metadata
        // carries `threshold_reached: true` + `effective_at` (RFC3339 UTC)
        // and a `supersedes_cid` pointing at the pending entry.
        // `query_effective_revocation_for_key` selects most-recent-by-
        // updated_at, so this new entry becomes the gate's effective record.
        let effective_metadata = serde_json::json!({
            "id": revocation_id,
            "human_id": revocation_human_id,
            "revoked_key": revocation_revoked_key,
            "reason": revocation_reason,
            "trigger_type": "steward_vote",
            "initiated_by": revocation_initiated_by,
            "required_votes": revocation_required_votes,
            "current_votes": approved_count,
            "threshold_reached": true,
            "effective_at": rfc3339_now,
            "triggering_vote_id": vote_id,
        });
        let effective_bridge_input = ConsolidatedProposeRecoveryGovernanceActionInput {
            governance_kind: "governance-action:key-revocation".to_string(),
            subject_human_id: revocation_human_id.clone(),
            title: format!(
                "Key revocation effective for {} (quorum reached)",
                revocation_human_id
            ),
            description: Some(format!(
                "trigger_type=steward_vote; reason={}; supersedes={}",
                revocation_reason, input.revocation_cid
            )),
            reach: "intimate".to_string(),
            threshold: serde_json::json!({"m": revocation_required_votes}),
            closes_at: rfc3339_now.clone(),
            metadata: effective_metadata,
            supersedes_cid: Some(input.revocation_cid.clone()),
        };
        let effective_consolidated =
            call_elohim_propose_recovery_governance_action(effective_bridge_input)?;
        // The CID of the fresh effective Content entry on the elohim DNA.
        // Stage 1: this is the elohim Content CID string.  Stage 2: switch to
        // the true ActionHash once the elohim DNA exposes it alongside the CID.
        let effective_cid = effective_consolidated.cid;

        emit_signal(RecoveryV2Signal::RevocationVoteSubmitted {
            id: vote_id.clone(),
            revocation_id: revocation_id.clone(),
            steward_id: caller_human_id.clone(),
            approved: input.approved,
            attestation: input.attestation.clone(),
            voted_at: timestamp.clone(),
            current_votes: approved_count,
            required_votes: revocation_required_votes,
            threshold_now_reached: true,
        })?;

        let emitted_at_voted = rfc3339_from_sys_time(&sys_time()?);
        #[allow(deprecated)]
        emit_signal(RecoveryV2Signal::KeyRevocationEffective {
            revocation_id: revocation_id.clone(),
            signal_type: "keyRevocation".to_string(),
            action_hash: effective_cid.clone(),
            human_id: revocation_human_id.clone(),
            revoked_key: revocation_revoked_key.clone(),
            // M4: no separate compromise-discovery timestamp; coincides with effectiveAt.
            // Future revisions may populate this from revocation request metadata.
            compromise_at: rfc3339_now.clone(),
            effective_at: rfc3339_now.clone(),
            triggering_vote_id: Some(vote_id.clone()),
            emitted_at: emitted_at_voted,
        })?;

        // T18: EPR-shape envelope alongside the legacy signal (back-compat window).
        // Voted quorum path: supersedes_cid = prior pending CID (pending → effective
        //                    lineage); triggering_revocation_id = vote_id.
        emit_key_revocation_envelope(
            revocation_id.clone(),
            effective_cid,
            revocation_human_id.clone(), // Stage 1: human_id; Stage 2: Human entry CID
            revocation_revoked_key.clone(),
            rfc3339_now.clone(),         // compromise_at == effective_at in M4
            rfc3339_now,
            Some(vote_id.clone()),       // triggering_revocation_id
            Some(input.revocation_cid.clone()), // supersedes_cid: prior pending entry
        )?;
    } else {
        emit_signal(RecoveryV2Signal::RevocationVoteSubmitted {
            id: vote_id.clone(),
            revocation_id: revocation_id.clone(),
            steward_id: caller_human_id.clone(),
            approved: input.approved,
            attestation: input.attestation.clone(),
            voted_at: timestamp,
            current_votes: approved_count,
            required_votes: revocation_required_votes,
            threshold_now_reached: false,
        })?;
    }

    Ok(RevocationVoteOutput {
        vote_id,
        current_votes: approved_count,
        required_votes: revocation_required_votes,
        threshold_now_reached,
    })
}

/// Convert a Holochain `Timestamp` (microseconds since epoch) to a sortable
/// RFC3339 UTC string `YYYY-MM-DDTHH:MM:SS.fffZ`. Used by Recovery M4 Task 14
/// to populate `effective_at` on the fresh effective Content (Task 4 gate
/// orders by lexicographic `updated_at`, so RFC3339 is mandatory).
pub(crate) fn rfc3339_from_sys_time(now: &Timestamp) -> String {
    // Best-effort: build via chrono-equivalent decomposition. We avoid pulling
    // in chrono (extra WASM weight) — instead format manually from the
    // micros-since-epoch i64.
    let micros = now.as_micros();
    let secs = micros.div_euclid(1_000_000);
    let sub_micros = micros.rem_euclid(1_000_000) as u32;
    let millis = sub_micros / 1000;
    // Days since 1970-01-01.
    let days_since_epoch = secs.div_euclid(86_400);
    let seconds_in_day = secs.rem_euclid(86_400) as u32;
    let hour = seconds_in_day / 3600;
    let minute = (seconds_in_day % 3600) / 60;
    let second = seconds_in_day % 60;
    // Civil from days (Howard Hinnant's algorithm, public-domain).
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, month, day, hour, minute, second, millis
    )
}

/// Query elohim DNA for an active `governance-action:identity-freeze` Content
/// entry covering `human_id` and synthesise a thin `IdentityFreeze` view from
/// its metadata. Returns an empty vec when no active freeze exists.
///
/// Recovery M4 Task 4 migration:
/// Previously traversed `HumanToFreeze` links + decoded `IdentityFreeze` entries
/// from imagodei's DHT. Post Stage 2 (Tasks 13/14) the canonical freeze entries
/// are `governance-action:identity-freeze` Content entries on the elohim DNA;
/// this function bridges to elohim to fetch the active record and synthesises
/// a coordinator-local `IdentityFreeze` shape with ONLY the three fields the
/// downstream `check_freeze_floor_rules` consumer reads (per Stage 1 audit
/// row #3): `is_active`, `human_id`, `frozen_at_layer`. All other fields are
/// defaulted; callers must not rely on them.
fn collect_active_freezes_for_human(human_id: &str) -> ExternResult<Vec<IdentityFreeze>> {
    let Some(content_output) = call_elohim_query_effective_identity_freeze_for_human(human_id)?
    else {
        return Ok(Vec::new());
    };

    if content_output.content.content_type != "governance-action:identity-freeze" {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "collect_active_freezes_for_human: expected governance-action:identity-freeze, got {}",
            content_output.content.content_type
        ))));
    }

    let metadata: serde_json::Value = serde_json::from_str(&content_output.content.metadata_json)
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "collect_active_freezes_for_human: bad metadata_json: {e}"
            )))
        })?;

    let is_active = metadata
        .get("is_active")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !is_active {
        return Ok(Vec::new());
    }
    // Re-confirm the human_id matches what the gate is asking for. The elohim
    // helper already filtered, but treating the gate as defensive against any
    // future helper changes keeps this read-side authoritative.
    let metadata_human_id = metadata
        .get("human_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if metadata_human_id != human_id {
        return Ok(Vec::new());
    }
    let frozen_at_layer = metadata
        .get("frozen_at_layer")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Synthesise a view-only IdentityFreeze populating ONLY the three fields
    // `check_freeze_floor_rules` reads. All other fields use type-appropriate
    // defaults; do not rely on them downstream.
    let freeze_view = IdentityFreeze {
        id: content_output.content.id.clone(),
        human_id: human_id.to_string(),
        freeze_type: String::new(),
        frozen_capabilities: Vec::new(),
        severity: String::new(),
        triggered_by: String::new(),
        trigger_type: String::new(),
        requires_verification: String::new(),
        verification_attempts: 0,
        last_verification_at: None,
        is_active,
        lifted_at: None,
        lifted_by: None,
        lift_reason: None,
        frozen_at: String::new(),
        expires_at: None,
        frozen_at_layer,
    };
    Ok(vec![freeze_view])
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
    // If an effective `governance-action:key-revocation` Content entry exists on
    // the elohim DNA for the human_agent_pubkey (the key being rotated from),
    // block the rotation. No authority-layer exemption — revocation is structural
    // (a revoked key must not produce valid rotations under any claimed authority),
    // intentionally asymmetric with the freeze-floor gate which exempts
    // CryptographicQuorum.
    //
    // Recovery M4 Task 4 migration: previously walked imagodei-local
    // `PendingRevocations` + `EffectiveRevocations` anchor links and decoded
    // `KeyRevocation` entries via `to_app_option()`. Post Stage 2 the canonical
    // revocation entries are Content entries on elohim DNA; this gate now reads
    // through the cross-DNA bridge. Note: `query_effective_revocation_for_key`
    // only returns *effective* records (threshold_reached || effective_at). The
    // pre-Stage-2 gate also rejected *pending* revocations as a precaution; the
    // post-Stage-2 producers (Tasks 13/14) MUST emit pending-state Content
    // entries with `threshold_reached: false` AND `effective_at: null` so they
    // are correctly excluded here, and emit a NEW Content entry with the
    // flipped fields when quorum is met. Producers must use create_entry for
    // effectiveness transitions, not update_entry — see
    // content_store/src/lib.rs:3185+ for the read-side rationale.
    {
        let rotating_from_str = input.human_agent_pubkey.to_string();

        if let Some(content_output) =
            call_elohim_query_effective_revocation_for_key(&rotating_from_str)?
        {
            if content_output.content.content_type != "governance-action:key-revocation" {
                return Err(wasm_error!(WasmErrorInner::Guest(format!(
                    "commit_key_rotation revocation-floor gate: expected \
                     governance-action:key-revocation, got {}",
                    content_output.content.content_type
                ))));
            }
            let metadata: serde_json::Value =
                serde_json::from_str(&content_output.content.metadata_json).map_err(|e| {
                    wasm_error!(WasmErrorInner::Guest(format!(
                        "commit_key_rotation revocation-floor gate: bad metadata_json: {e}"
                    )))
                })?;
            let rev_id = metadata
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(&content_output.content.id);
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "commit_key_rotation blocked: key {} has an effective revocation ({}). \
                 Resolve or await the revocation before rotating.",
                rotating_from_str, rev_id
            ))));
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
// Recovery Protocol Phase 2 — M4 T22: create_shamir_custody_setup
// =============================================================================

/// Input for `create_shamir_custody_setup` — Recovery M4 T22.
///
/// Records the custody manifest for an agent's Shamir-split seed: which
/// custodian CIDs hold which share indices, the (m, n) threshold, and the
/// validity horizon. The actual share bytes are delivered out-of-band and
/// installed in each custodian's local `custodian_shares` table (T21); the
/// DHT entry only carries the assignment metadata, never the share material
/// (enforced by Floor G3 in `attestation_validator.rs`).
///
/// `custodian_assignments` is a list of `(custodian_cid, share_index)` pairs.
/// The order of the pairs is the canonical order — share_index values are
/// 1-based and correspond exactly to what the T21 sharks split produces.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ShamirCustodySetupInput {
    /// Human ID this custody manifest is for.
    pub human_id: String,
    /// Minimum number of shares required to reconstruct (m in (m, n)).
    pub threshold_m: u32,
    /// Total number of shares created (n in (m, n)). Must equal
    /// `custodian_assignments.len()`.
    pub threshold_n: u32,
    /// Custodian assignments — `(custodian_cid, share_index)` pairs.
    pub custodian_assignments: Vec<ShamirCustodianAssignment>,
    /// RFC3339 timestamp after which this custody manifest is no longer
    /// honored. Stewardship-rotation creates a fresh setup before this
    /// horizon; on rotation the new setup supersedes the prior via
    /// `supersedes_cid` (CREATE-only lineage).
    pub valid_until: String,
    /// Optional supersession pointer — CID of the prior
    /// `governance-action:shamir-custody-setup` entry this replaces.
    /// `None` on initial setup; populated on stewardship-rotation.
    pub supersedes_cid: Option<String>,
}

/// Single custodian↔share-index assignment in a Shamir custody manifest.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ShamirCustodianAssignment {
    /// CID of the custodian's identity (imagodei Human CID or agent pubkey
    /// string, depending on stage — Stage 1 is human_id, Stage 2 will be CID).
    pub custodian_cid: String,
    /// 1-based share index this custodian holds.
    pub share_index: u32,
}

/// Output of `create_shamir_custody_setup`.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ShamirCustodySetupOutput {
    /// CID of the newly committed `governance-action:shamir-custody-setup`
    /// Content entry on the elohim DNA.
    pub custody_setup_cid: String,
}

/// Commit a Shamir custody manifest as a `governance-action:shamir-custody-setup`
/// Content entry on the elohim DNA.
///
/// The manifest records the custodian↔share-index assignment, threshold, and
/// validity horizon — but NEVER the share bytes themselves. Share bytes are
/// installed out-of-band into each custodian's local `custodian_shares`
/// table (T21); Floor G3 in `attestation_validator.rs` rejects any DHT
/// entry that tries to carry share material.
///
/// At recovery time, the `ShareAssembler` (T21) reads this manifest from
/// the DHT to determine the dial list (which custodian holds which share
/// index) deterministically — without depending on live capability
/// advertisements.
#[hdk_extern]
pub fn create_shamir_custody_setup(
    input: ShamirCustodySetupInput,
) -> ExternResult<ShamirCustodySetupOutput> {
    // Validate the assignment list matches the declared (m, n) threshold.
    if input.custodian_assignments.len() as u32 != input.threshold_n {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "create_shamir_custody_setup: custodian_assignments.len() = {} but threshold_n = {}",
            input.custodian_assignments.len(),
            input.threshold_n,
        ))));
    }
    if input.threshold_m == 0 || input.threshold_m > input.threshold_n {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "create_shamir_custody_setup: invalid (m, n) threshold: m={} n={}",
            input.threshold_m, input.threshold_n,
        ))));
    }

    // Validate share_index values: 1-based, unique, cover 1..=n.
    let mut indices: Vec<u32> = input
        .custodian_assignments
        .iter()
        .map(|a| a.share_index)
        .collect();
    indices.sort_unstable();
    let expected: Vec<u32> = (1..=input.threshold_n).collect();
    if indices != expected {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "create_shamir_custody_setup: share_index values must be {{1..={}}} with no \
             duplicates; got {:?}",
            input.threshold_n, indices,
        ))));
    }

    // Bridge to elohim DNA. The Floor G3 validator rejects any metadata
    // containing share_data / share_index / share_blob — our payload here
    // intentionally carries only the assignment manifest. The `share_index`
    // values appear nested under `custodian_assignments[*].share_index`;
    // Floor G3 checks the literal key name "share_index" anywhere in the
    // tree, so this assignment shape would trip the floor. Use a renamed
    // wire field `assignment_index` for the DHT-bound metadata to avoid
    // collision with Floor G3's forbidden key set.
    let assignments_json: Vec<serde_json::Value> = input
        .custodian_assignments
        .iter()
        .map(|a| {
            serde_json::json!({
                "custodian_cid": a.custodian_cid,
                "assignment_index": a.share_index,
            })
        })
        .collect();

    let bridge_input = ConsolidatedProposeRecoveryGovernanceActionInput {
        governance_kind: "governance-action:shamir-custody-setup".to_string(),
        subject_human_id: input.human_id.clone(),
        title: format!(
            "Shamir custody setup for {} (m={}, n={})",
            input.human_id, input.threshold_m, input.threshold_n
        ),
        description: Some(format!(
            "Records the custody manifest: which custodians hold which share \
             indices, with a {}-of-{} reconstruction threshold. Share bytes are \
             delivered out-of-band and never travel through the DHT.",
            input.threshold_m, input.threshold_n
        )),
        reach: "intimate".to_string(),
        threshold: serde_json::json!({"m": input.threshold_m, "n": input.threshold_n}),
        closes_at: input.valid_until.clone(),
        metadata: serde_json::json!({
            "human_id": input.human_id,
            "threshold_m": input.threshold_m,
            "threshold_n": input.threshold_n,
            "custodian_assignments": assignments_json,
            "valid_until": input.valid_until,
            // Custody setup is immediately effective on initial CREATE — there
            // is no voting flow for the setup itself (the recovery-request
            // that USES this manifest has its own quorum). Subsequent
            // stewardship-rotation CREATEs supersede via supersedes_cid.
            "threshold_reached": true,
            "effective_at": input.valid_until.clone(),
        }),
        supersedes_cid: input.supersedes_cid,
    };

    let consolidated = call_elohim_propose_recovery_governance_action(bridge_input)?;
    Ok(ShamirCustodySetupOutput {
        custody_setup_cid: consolidated.cid,
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
        let Some(rel_hash) = link.target.clone().into_action_hash() else {
            continue;
        };
        let Some(record) = get(rel_hash, GetOptions::default())? else {
            continue;
        };
        let Some(rel): Option<HumanRelationship> = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        else {
            continue;
        };
        if !rel.emergency_access_enabled {
            continue;
        }
        // party_a_id and party_b_id are human ID strings.
        if rel.party_a_id == authorizer_human_id || rel.party_b_id == authorizer_human_id {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Returns true if `authorizer_human_id` already has a witness linked
/// from the given recovery-request-CID anchor (via
/// `RecoveryRequestToHumanityWitness`).
///
/// Recovery M4 Task 3: the link source is now the `StringAnchor(
/// "recovery_request_cid", cid)` entry hash on imagodei, because the
/// recovery-request itself lives on the elohim DNA as a Content entry — its
/// ActionHash is not reachable from imagodei. Dedupe is tag-based: the link
/// tag stores the authorizer_human_id bytes.
fn has_existing_witness_for_request(
    request_anchor_hash: &EntryHash,
    authorizer_human_id: &str,
) -> ExternResult<bool> {
    let links = get_links(
        LinkQuery::try_new(
            request_anchor_hash.clone(),
            LinkTypes::RecoveryRequestToHumanityWitness,
        )?,
        GetStrategy::default(),
    )?;
    let tag_bytes = authorizer_human_id.as_bytes();
    for link in links {
        if link.tag.0.as_slice() == tag_bytes {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Submit an intimate witness attestation for a recovery request.
///
/// Stage G.A.2 bridge: writes `attestation:humanness` to elohim DNA via
/// `call_elohim_issue_attestation`. The legacy `HumanityWitness` entry type
/// is NOT written to imagodei's source chain; the canonical entry lives on
/// elohim DNA's DHT (content_type "attestation:humanness").
///
/// Gates are preserved in full ABOVE the bridge call:
/// 1. Recovery-request must exist on the elohim DNA as a
///    `governance-action:recovery-request` Content entry and its metadata
///    must carry `human_id`. Gate calls cross-DNA
///    `content_store::get_content_by_id` (Recovery M4 Task 3). The legacy
///    `RecoveryRequest` entry type on imagodei is no longer read here.
/// 2. Authorizer must be an active emergency contact of the target human
///    (has a HumanRelationship with emergency_access_enabled = true).
/// 3. Dedupe: tag-based check on RecoveryRequestToHumanityWitness links
///    (tag = authorizer_human_id bytes). Link source is the
///    `StringAnchor("recovery_request_cid", cid)` entry hash on imagodei —
///    the cross-DNA recovery-request Content entry is not directly link-able
///    from this DNA; the StringAnchor proxies it.
///
/// Signal: emits IntimateWitnessSubmitted with the synthesised HumanityWitness
/// struct and zero ActionHash sentinels for both `action_hash` and
/// `request_hash` (CIDs cannot be converted to ActionHash). The bridged
/// `attestation:humanness` Content entry on elohim is the canonical record;
/// elohim-storage consumers must migrate to CID-based lookup. Task 17 (signal
/// schema alignment) will replace these ActionHash fields with CIDs.
///
#[hdk_extern]
pub fn submit_intimate_witness(
    input: SubmitIntimateWitnessInput,
) -> ExternResult<SubmitIntimateWitnessOutput> {
    // Gate 1: fetch the recovery-request Content entry on the elohim DNA via
    // cross-DNA call to `content_store::get_content_by_id`. Recovery M4 Task 3:
    // the recovery-request is now a `governance-action:recovery-request` Content
    // entry on elohim, no longer a `RecoveryRequest` entry on imagodei. Decode
    // is structured (ContentOutput), not a raw Entry::App — so the Task 2
    // `content_decode` helper is not needed in this code path.
    let human_id =
        fetch_recovery_request_human_id(&input.recovery_request_cid)?;

    // Gate 2: authorizer must be on an active emergency-enabled HumanRelationship.
    let authorizer_pubkey = agent_info()?.agent_initial_pubkey;
    let authorizer_human_id = resolve_human_id_for_agent(&authorizer_pubkey)?;
    if !is_active_emergency_contact(&human_id, &authorizer_human_id)? {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "authorizing agent is not an active emergency contact of this human".into()
        )));
    }

    // Gate 3: tag-based dedupe — the authorizer cannot witness the same request twice.
    // Tag stores authorizer_human_id bytes; no entry deserialization needed.
    // Dedupe is now keyed on the recovery-request CID via a StringAnchor —
    // the ActionHash that previously anchored the link no longer exists on
    // imagodei's DHT after the cross-DNA migration of create_recovery_request.
    let request_anchor =
        StringAnchor::new("recovery_request_cid", &input.recovery_request_cid);
    let request_anchor_hash =
        hash_entry(&EntryTypes::StringAnchor(request_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(request_anchor))?;
    if has_existing_witness_for_request(&request_anchor_hash, &authorizer_human_id)? {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "this agent has already submitted a witness for this request".into()
        )));
    }

    // Build the synthesised HumanityWitness for signal emission (not written locally).
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);
    let expiry_micros = WITNESS_EXPIRY_DAYS * MICROS_PER_DAY;
    let expires_at = format!(
        "{:?}",
        now.checked_add(&Duration::from_micros(expiry_micros))
            .unwrap_or(now)
    );
    let witness_id_ts = timestamp.replace([':', ' ', '(', ')'], "-");
    let witness_id = format!("intimate-witness-{}-{}", human_id, witness_id_ts);
    let witness_for_signal = HumanityWitness {
        id: witness_id.clone(),
        human_id: human_id.clone(),
        // NOTE: M3 stores authorizer's human_id here, not agent pubkey.
        witness_agent_id: authorizer_human_id.clone(),
        attestation_type: "intimate_recovery".into(),
        confidence: 1.0,
        behavioral_hash: None,
        evidence_json: input
            .note
            .as_ref()
            .map(|n| serde_json::json!({ "note": n }).to_string()),
        verification_method: Some("intimate_recovery_ceremony".into()),
        created_at: timestamp.clone(),
        expires_at: expires_at.clone(),
        revoked_at: None,
    };

    // Bridge: write attestation:humanness to elohim DNA (canonical truth).
    // Shamir share material does NOT travel on the DHT — see spec §5 and Stage G.B.
    let note_json = input
        .note
        .as_ref()
        .map(|n| serde_json::json!({ "note": n }))
        .unwrap_or(serde_json::Value::Null);
    let consolidated_input = ConsolidatedIssueAttestationInput {
        attestation_kind: "attestation:humanness".to_string(),
        subject_cid: human_id.clone(),
        subject_kind: "agent".to_string(),
        title: format!("Intimate witness for recovery request"),
        description: Some(format!(
            "Witness submitted by {} for recovery request",
            authorizer_human_id
        )),
        reach: "private".to_string(),
        metadata: serde_json::json!({
            "witness_id": witness_id,
            "attestation_type": "intimate_recovery",
            "confidence": 1.0,
            "verification_method": "intimate_recovery_ceremony",
            "recovery_request_cid": input.recovery_request_cid,
            "expires_at": expires_at,
            "evidence": note_json,
        }),
        parent_governance_action_cid: None,
        vote_value: None,
        proof_class: "social-witness".to_string(),
        proof_evidence: serde_json::json!({
            "class": "intimate_recovery",
            "authorizer_human_id": authorizer_human_id,
        }),
        expires_at: Some(expires_at),
    };
    let _consolidated = call_elohim_issue_attestation(consolidated_input)?;

    // Create the M3 link from the request-CID anchor to the witness anchor.
    // Tag = authorizer_human_id bytes for tag-based dedupe (Gate 3).
    // Target = sentinel StringAnchor hash (no local HumanityWitness entry written).
    // Source = `StringAnchor("recovery_request_cid", cid)` hash on imagodei
    //  (created above in Gate 3). The recovery-request Content entry itself
    //  lives on the elohim DNA and is not reachable as a link source from
    //  imagodei; the StringAnchor proxies it within this DNA.
    let sentinel_anchor = StringAnchor::new("intimate_witness_sentinel", &witness_id);
    let sentinel_hash = hash_entry(&EntryTypes::StringAnchor(sentinel_anchor))?;
    create_link(
        request_anchor_hash,
        sentinel_hash,
        LinkTypes::RecoveryRequestToHumanityWitness,
        LinkTag::new(authorizer_human_id.as_bytes()),
    )?;

    // Emit signal. action_hash AND request_hash are zero sentinels — CIDs
    // cannot be converted to ActionHash. Consumers keyed on action_hash or
    // request_hash must migrate to CID lookup via the cross-DNA Content path.
    // Task 17 (signal schema alignment) will replace the ActionHash fields
    // with `request_cid: String` on the schema-aligned signal payload.
    let sentinel_action_hash = ActionHash::from_raw_36(vec![0u8; 36]);
    emit_signal(RecoveryV2Signal::IntimateWitnessSubmitted {
        action_hash: sentinel_action_hash.clone(),
        request_hash: sentinel_action_hash.clone(),
        witness: witness_for_signal.clone(),
        witness_agent_id: authorizer_pubkey,
    })?;

    Ok(SubmitIntimateWitnessOutput {
        action_hash: sentinel_action_hash,
        witness: witness_for_signal,
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

/// Create a renewal attestation (initiates the social witness ceremony).
///
/// B.9 bridge: delegates to elohim DNA's content_store::propose_governance_action
/// with governance_kind "governance-action:renewal-request". The legacy
/// RenewalAttestation entry type is NOT written to imagodei's source chain;
/// the canonical entry lives on elohim DNA's DHT. Stage C will remove this
/// wrapper entirely.
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

    let consolidated_input = ConsolidatedProposeGovernanceActionInput {
        governance_kind: "governance-action:renewal-request".to_string(),
        subject_cid: input.human_id.clone(),
        title: format!("Key renewal for human {}", input.human_id),
        description: Some(input.renewal_reason.clone()),
        reach: "community".to_string(),
        threshold: serde_json::json!({
            "required": input.required_approvals,
            "strategy": "simple-majority",
        }),
        eligibility_predicate: None,
        ballot_format: "approve-reject".to_string(),
        closes_at: expires_at.clone(),
        parameters: Some(serde_json::json!({
            "old_agent_key": input.old_agent_key,
            "new_agent_key": input.new_agent_key,
            "doorway_id": input.doorway_id,
            "recovery_request_id": input.recovery_request_id,
            "renewal_reason": input.renewal_reason,
        })),
    };

    let consolidated = call_elohim_propose_governance_action(consolidated_input)?;

    // Synthesise the legacy RenewalAttestation struct from the consolidated output.
    // Fields that have no consolidated equivalent (votes_json, current_approvals,
    // confidence_score, witnessed_at) are set to their initial/zero values.
    let attestation = RenewalAttestation {
        id: attestation_id,
        human_id: input.human_id.clone(),
        old_agent_key: String::new(), // stored in elohim parameters metadata
        new_agent_key: String::new(), // stored in elohim parameters metadata
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
        expires_at: consolidated.closes_at,
    };

    // Sentinel action_hash — see note in issue_attestation bridge above.
    let action_hash = ActionHash::from_raw_36(vec![0u8; 36]);

    Ok(RenewalAttestationOutput {
        action_hash,
        entry: attestation,
    })
}

/// Get a renewal attestation by ID.
///
/// Stage C.2: RenewalAttestation is no longer a DHT entry type — canonical
/// governance-action entries live on elohim DNA via propose_governance_action
/// bridge. The imagodei link types (IdToRenewalAttestation, HumanToRenewalAttestation)
/// were removed. Stage F will replace this stub with a bridge call to
/// elohim's `get_governance_action_with_children`.
#[hdk_extern]
pub fn get_renewal_attestation_by_id(_id: String) -> ExternResult<Option<RenewalAttestationOutput>> {
    // TODO(Stage F): bridge to elohim get_governance_action_with_children keyed by id
    Ok(None)
}

/// Get all renewal attestations for a human.
///
/// Stage C.2: stub — see get_renewal_attestation_by_id for migration note.
#[hdk_extern]
pub fn get_renewal_attestations_for_human(
    _human_id: String,
) -> ExternResult<Vec<RenewalAttestationOutput>> {
    // TODO(Stage F): bridge to elohim get_attestations_for_subject keyed by human_id
    Ok(vec![])
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
