//! sign_for_agent Coordinator Function — EPR Phase 2B, Task C.1
//!
//! Exposes the conductor's ed25519 signing primitive to elohim-storage so
//! storage can compose EPR Envelopes from /api/v1/signal/emit (Task C.2)
//! when the caller cannot sign in-browser. Agent keys live in the conductor's
//! lair — storage holds neither key material nor signing authority.
//!
//! ## Wire schema
//! `elohim/sdk/schemas/v1/conductor-signing.schema.json`
//!
//! ## Pre-commit gate
//!
//! An Agent EPR for `input.agent_cid` MUST exist, and the calling agent's pubkey
//! (from `agent_info()`) must equal that EPR's `holochain_agent_key`. Either
//! failure is rejected with `WasmErrorInner::Guest`. This mirrors the gate
//! pattern established by `create_agent_peer_binding` (see memory pin:
//! `project_hdi_no_get_links_in_validators`).
//!
//! Note: lair will only sign with private keys it holds. A coordinator function
//! cannot sign with a different agent's key even if requested — `sign_raw`
//! errors when the private half is absent. The signer-match gate is
//! defense-in-depth so callers fail fast with a clear error rather than a
//! cryptic lair miss.
//!
//! ## HARD INVARIANT — a signature here is not proof of `agent_cid` control
//!
//! A `sign_for_agent` output is NEVER, on its own, proof that the signer
//! controls `agent_cid`. This gate is *coordinator-local and receiver-blind*: it
//! runs on the calling node, so a third party reading the resulting artifact
//! learns nothing from the fact that it exists. Every consuming verifier must
//! independently bind `signer_pubkey` to the claimed identity — by derivation
//! where the identity names its own key (a HoloHash `AgentPubKey`), or through
//! the head/lineage resolver otherwise — and must FAIL CLOSED when no head or
//! EPR resolves. The cross-signed `AgentPeerBinding` verifier does exactly that
//! (`elohim-storage` `p2p::binding_proof_wire::classify_binding_signature`);
//! any new consumer owes the same check.
//!
//! ## Why the Stage-1 no-EPR carve-out was closed (C2-S2)
//!
//! This function previously PERMITTED the call when no Agent EPR resolved for
//! `input.agent_cid` — a bootstrap convenience. It meant the gate was silently
//! absent in exactly the case an attacker would arrange: name an unregistered
//! `agent_cid` and the signer-match check never runs. Closing it makes "the
//! conductor signed for X" mean "an Agent EPR for X exists and this caller holds
//! its registered key". Callers that legitimately have no EPR yet must create
//! one first (`create_agent`) rather than being waved through.
//!
//! `create_agent_peer_binding`'s analogous carve-out is deliberately left OPEN
//! and is tracked separately: closing it in the same change would refuse the
//! very binding this slice mints (a node binds its `uhCAk…` agent key, for which
//! no slug-keyed Agent EPR exists).
//!
//! ## No signal emission
//!
//! This is a synchronous RPC — no DHT entry is created, no state changes,
//! no signal is emitted. The signed bytes returned become part of an EPR
//! Envelope (Category A) only after the storage caller invokes
//! `EprService::ingest`.

use hdk::prelude::*;
use imagodei_integrity::*;

// =============================================================================
// I/O types
// =============================================================================

/// Input to `sign_for_agent`.
///
/// Field names match the wire schema at
/// `elohim/sdk/schemas/v1/conductor-signing.schema.json` (request).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignForAgentInput {
    /// CIDv1 of the Agent EPR whose key should sign. The calling agent's
    /// pubkey must match this EPR's `holochain_agent_key`.
    pub agent_cid: String,
    /// Raw bytes to sign (canonical CBOR of the Envelope ex-proof + payload
    /// in the EPR composition path; arbitrary in other call paths).
    pub canonical_bytes: Vec<u8>,
}

/// Output from `sign_for_agent`.
///
/// Field names match the wire schema at
/// `elohim/sdk/schemas/v1/conductor-signing.schema.json` (response).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignForAgentOutput {
    /// Raw 64-byte Ed25519 signature over `canonical_bytes`.
    pub signature: Vec<u8>,
    /// Raw 32-byte Ed25519 public key of the signing agent. Returned for
    /// defense-in-depth: callers can verify the signature against this pubkey
    /// before ingesting the resulting EPR.
    pub signer_pubkey: Vec<u8>,
}

// =============================================================================
// sign_for_agent
// =============================================================================

/// Sign canonical bytes under the calling agent's lair-managed key.
///
/// ## Pre-commit gate: signer-match (no carve-out)
///
/// An Agent EPR for `input.agent_cid` MUST resolve, and the caller's pubkey MUST
/// equal its `holochain_agent_key`. A missing EPR is a refusal, not a bypass —
/// see the module doc for why the Stage-1 carve-out was closed in C2-S2, and for
/// the hard invariant that a signature from here is never on its own proof of
/// `agent_cid` control.
#[hdk_extern]
pub fn sign_for_agent(input: SignForAgentInput) -> ExternResult<SignForAgentOutput> {
    // -------------------------------------------------------------------------
    // Pre-commit gate: the Agent EPR must exist AND name this caller's key
    // -------------------------------------------------------------------------
    let caller_info = agent_info()?;
    let caller_pubkey = caller_info.agent_initial_pubkey.clone();

    let agent = get_agent_by_id_internal(&input.agent_cid)?.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "no Agent EPR registered for agent_cid '{}' — sign_for_agent refuses to sign \
             for an identity it cannot bind to a key (the Stage 1 no-EPR carve-out was \
             closed in C2-S2; register the Agent EPR first)",
            input.agent_cid
        )))
    })?;

    let registered_key_str = agent.holochain_agent_key.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Agent EPR '{}' has no holochain_agent_key registered",
            input.agent_cid
        )))
    })?;

    let caller_key_str = caller_pubkey.to_string();
    if caller_key_str != registered_key_str {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "signer mismatch: caller '{}' does not match Agent EPR '{}' key '{}'",
            caller_key_str, input.agent_cid, registered_key_str
        ))));
    }

    // -------------------------------------------------------------------------
    // Sign with the caller's lair-managed key
    // -------------------------------------------------------------------------
    let signature = hdk::ed25519::sign_raw(caller_pubkey.clone(), input.canonical_bytes)?;

    Ok(SignForAgentOutput {
        signature: signature.0.to_vec(),
        signer_pubkey: caller_pubkey.get_raw_32().to_vec(),
    })
}

// =============================================================================
// Private helpers
// =============================================================================

/// Resolve an Agent EPR by its `id` field (which the caller passes as `agent_cid`).
///
/// Returns `None` if no Agent EPR with that id exists in this DNA. Mirrors
/// the lookup helper in `agent_peer_binding.rs` — kept private here to avoid
/// cross-module coupling on a small helper that may evolve independently.
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
