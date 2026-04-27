//! AgentPeerBinding Coordinator Functions — EPR Phase 2B, Task A.13
//!
//! Coordinator functions that create and query AgentPeerBinding entries.
//! The integrity entry type and validator live in `imagodei_integrity`.
//!
//! ## Pre-commit gate
//!
//! `create_agent_peer_binding` enforces: the calling agent's pubkey must match
//! the Agent EPR identified by `agent_cid`. This is a coordinator-level gate
//! (not integrity-level) because it requires link traversal via `get_links`,
//! which is HDK-only and unavailable in HDI validation callbacks.
//!
//! See memory pin: `project_hdi_no_get_links_in_validators`.
//!
//! ## Signal emission
//!
//! On successful create, emits `ImagodeiSignal::AgentPeerBindingCreated`.
//! The elohim-storage `HolochainAppSignalStream` (Task A.11) translates this
//! signal into a `DnaSignal::AgentPeerBinding` projection event.

use hdk::prelude::*;
use imagodei_integrity::*;

use crate::ImagodeiSignal;

// =============================================================================
// I/O types
// =============================================================================

/// Input to `create_agent_peer_binding`.
///
/// Field names match the wire schema at
/// `elohim/sdk/schemas/v1/objects/agent-peer-binding.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentPeerBindingInput {
    /// Multibase-encoded libp2p PeerId (base58btc multihash).
    pub peer_id: String,
    /// CIDv1 (dag-cbor sha256) of the Agent EPR in imagodei.
    /// The calling agent's pubkey must match this EPR's `holochain_agent_key`.
    pub agent_cid: String,
    /// UTC start of binding validity, microseconds since epoch.
    pub valid_from_micros: i64,
    /// Optional expiry, microseconds since epoch. `None` = no declared expiry.
    pub valid_until_micros: Option<i64>,
    /// Hardware/deployment archetype: "node" | "desktop" | "mobile" | "steward".
    pub device_archetype: String,
    /// Raw Ed25519 signature bytes over `AgentPeerBinding::canonical_bytes()`.
    pub signature: Vec<u8>,
}

/// Output from `create_agent_peer_binding`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentPeerBindingOutput {
    pub action_hash: ActionHash,
}

/// Wire view of an AgentPeerBinding for query responses.
///
/// `#[serde(rename_all = "camelCase")]` so the JSON keys (`peerId`, `agentCid`,
/// `validFromMicros`, etc.) match what Angular / sweettests assert.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPeerBindingView {
    pub action_hash: ActionHash,
    pub peer_id: String,
    pub agent_cid: String,
    /// UTC validity start, microseconds since epoch.
    pub valid_from_micros: i64,
    /// UTC validity end, microseconds since epoch. `None` = no declared expiry.
    pub valid_until_micros: Option<i64>,
    pub device_archetype: String,
    /// ActionHash of the binding that supersedes this one; `None` when current.
    pub superseded_by: Option<ActionHash>,
}

impl AgentPeerBindingView {
    fn from_entry(action_hash: ActionHash, b: AgentPeerBinding) -> Self {
        let archetype_str = match b.device_archetype {
            DeviceArchetype::Node => "node".to_string(),
            DeviceArchetype::Desktop => "desktop".to_string(),
            DeviceArchetype::Mobile => "mobile".to_string(),
            DeviceArchetype::Steward => "steward".to_string(),
        };
        AgentPeerBindingView {
            action_hash,
            peer_id: b.peer_id,
            agent_cid: b.agent_cid,
            valid_from_micros: b.valid_from.as_micros(),
            valid_until_micros: b.valid_until.map(|t| t.as_micros()),
            device_archetype: archetype_str,
            superseded_by: b.superseded_by,
        }
    }
}

// =============================================================================
// create_agent_peer_binding
// =============================================================================

/// Create an AgentPeerBinding entry on the DHT.
///
/// ## Pre-commit gate: signer-match
///
/// The caller's agent pubkey (from `agent_info()`) must equal the
/// `holochain_agent_key` stored in the Agent EPR identified by `input.agent_cid`.
/// If they do not match, the call is rejected with `WasmErrorInner::Guest`.
///
/// ## Links created
/// - `AgentToPeerBinding`: `caller_pubkey` → `binding_action_hash`
/// - `PeerToBinding`:      `StringAnchor("peer_binding", peer_id)` → `binding_action_hash`
///
/// ## Signal emitted
/// `ImagodeiSignal::AgentPeerBindingCreated { action_hash, binding }`
#[hdk_extern]
pub fn create_agent_peer_binding(
    input: CreateAgentPeerBindingInput,
) -> ExternResult<CreateAgentPeerBindingOutput> {
    // -------------------------------------------------------------------------
    // Pre-commit gate: verify signer matches the Agent EPR identified by agent_cid
    // -------------------------------------------------------------------------
    let caller_info = agent_info()?;
    let caller_pubkey = caller_info.agent_initial_pubkey.clone();

    // Resolve agent_cid (the Agent EPR's id field) to its Agent entry.
    let maybe_agent = get_agent_by_id_internal(&input.agent_cid)?;

    // THREAT MODEL NOTE (Stage 1, see project_bootstrap_to_elohim_security_gradient):
    // This carve-out means any caller may bind an arbitrary agent_cid that does
    // not correspond to a registered Agent EPR. Stage 1 acceptable; Stage 2 should
    // require the EPR to exist and reject otherwise. The structural validator and
    // signature check (when wired in Stage 2) provide additional defense-in-depth.
    // If the Agent EPR IS found, the caller must match — the gate fires correctly.
    if let Some(agent) = maybe_agent {
        let registered_key_str = agent.holochain_agent_key.ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Agent EPR '{}' has no holochain_agent_key registered",
                input.agent_cid
            )))
        })?;

        // Agent.holochain_agent_key is stored as AgentPubKey::to_string()
        let caller_key_str = caller_pubkey.to_string();
        if caller_key_str != registered_key_str {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "signer mismatch: caller '{}' does not match Agent EPR '{}' key '{}'",
                caller_key_str, input.agent_cid, registered_key_str
            ))));
        }
    }

    // -------------------------------------------------------------------------
    // Parse device_archetype
    // -------------------------------------------------------------------------
    let device_archetype = parse_device_archetype(&input.device_archetype)?;

    // -------------------------------------------------------------------------
    // Construct and commit the entry
    // -------------------------------------------------------------------------
    // Timestamp::from_micros accepts any i64; out-of-range values would be caught by
    // integrity validator's valid_from < valid_until check.
    let binding = AgentPeerBinding {
        peer_id: input.peer_id.clone(),
        agent_cid: input.agent_cid.clone(),
        valid_from: Timestamp::from_micros(input.valid_from_micros),
        valid_until: input.valid_until_micros.map(Timestamp::from_micros),
        device_archetype,
        signature: input.signature.clone(),
        superseded_by: None,
    };

    let action_hash = create_entry(&EntryTypes::AgentPeerBinding(binding.clone()))?;

    // -------------------------------------------------------------------------
    // Forward link: AgentPubKey -> AgentPeerBinding
    // -------------------------------------------------------------------------
    create_link(
        caller_pubkey,
        action_hash.clone(),
        LinkTypes::AgentToPeerBinding,
        (),
    )?;

    // -------------------------------------------------------------------------
    // Reverse link: StringAnchor(peer_id) -> AgentPeerBinding
    // -------------------------------------------------------------------------
    let peer_anchor = StringAnchor::new("peer_binding", &input.peer_id);
    // Commit the anchor entry so peers can fetch it directly via get(anchor_hash) — matches M4 recovery pattern (lib.rs:1811).
    create_entry(&EntryTypes::StringAnchor(peer_anchor.clone()))?;
    let peer_anchor_hash = hash_entry(&EntryTypes::StringAnchor(peer_anchor))?;
    create_link(
        peer_anchor_hash,
        action_hash.clone(),
        LinkTypes::PeerToBinding,
        (),
    )?;

    // -------------------------------------------------------------------------
    // Emit signal for elohim-storage projection (Task A.11)
    // -------------------------------------------------------------------------
    emit_signal(ImagodeiSignal::AgentPeerBindingCreated {
        action_hash: action_hash.clone(),
        binding,
    })?;

    Ok(CreateAgentPeerBindingOutput { action_hash })
}

// =============================================================================
// get_agent_peer_bindings
// =============================================================================

/// Return all AgentPeerBindings associated with the given agent pubkey.
///
/// Traverses the forward link `AgentToPeerBinding` from the given
/// `AgentPubKey`. Returns a `Vec<AgentPeerBindingView>` in camelCase.
///
/// Returns only currently-valid bindings (superseded entries are filtered out).
/// Callers needing historical bindings should query the entry directly.
#[hdk_extern]
pub fn get_agent_peer_bindings(
    agent_pubkey: AgentPubKey,
) -> ExternResult<Vec<AgentPeerBindingView>> {
    let query = LinkQuery::try_new(agent_pubkey, LinkTypes::AgentToPeerBinding)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(binding) = record
                    .entry()
                    .to_app_option::<AgentPeerBinding>()
                    .ok()
                    .flatten()
                {
                    results.push(AgentPeerBindingView::from_entry(action_hash, binding));
                }
            }
        }
    }

    results.retain(|v| v.superseded_by.is_none());
    Ok(results)
}

// =============================================================================
// get_bindings_for_peer
// =============================================================================

/// Return all AgentPeerBindings for the given libp2p peer_id.
///
/// Traverses the reverse link `PeerToBinding` from
/// `StringAnchor("peer_binding", peer_id)`. Returns a `Vec<AgentPeerBindingView>`
/// in camelCase.
///
/// Returns only currently-valid bindings (superseded entries are filtered out).
/// Callers needing historical bindings should query the entry directly.
#[hdk_extern]
pub fn get_bindings_for_peer(peer_id: String) -> ExternResult<Vec<AgentPeerBindingView>> {
    let peer_anchor = StringAnchor::new("peer_binding", &peer_id);
    let peer_anchor_hash = hash_entry(&EntryTypes::StringAnchor(peer_anchor))?;

    let query = LinkQuery::try_new(peer_anchor_hash, LinkTypes::PeerToBinding)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(binding) = record
                    .entry()
                    .to_app_option::<AgentPeerBinding>()
                    .ok()
                    .flatten()
                {
                    results.push(AgentPeerBindingView::from_entry(action_hash, binding));
                }
            }
        }
    }

    results.retain(|v| v.superseded_by.is_none());
    Ok(results)
}

// =============================================================================
// Private helpers
// =============================================================================

/// Resolve an Agent EPR by its `id` field (which the caller passes as `agent_cid`).
///
/// Returns `None` if no Agent EPR with that id exists in this DNA.
/// Returns the `Agent` integrity entry so the caller can inspect
/// `holochain_agent_key` for the signer-match gate.
fn get_agent_by_id_internal(id: &str) -> ExternResult<Option<Agent>> {
    let id_anchor = StringAnchor::new("agent_id", id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToAgent)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(agent) = record.entry().to_app_option::<Agent>().ok().flatten() {
                    return Ok(Some(agent));
                }
            }
        }
    }

    Ok(None)
}

/// Parse a device_archetype string into the `DeviceArchetype` enum.
fn parse_device_archetype(s: &str) -> ExternResult<DeviceArchetype> {
    match s {
        "node" => Ok(DeviceArchetype::Node),
        "desktop" => Ok(DeviceArchetype::Desktop),
        "mobile" => Ok(DeviceArchetype::Mobile),
        "steward" => Ok(DeviceArchetype::Steward),
        other => Err(wasm_error!(WasmErrorInner::Guest(format!(
            "invalid device_archetype '{}'; must be one of: node, desktop, mobile, steward",
            other
        )))),
    }
}
