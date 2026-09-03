//! HostedAgentBinding — the "hosted-at" relationship between a human's agent
//! identity and the doorway that hosts their conductor install.
//!
//! Sprint task 2.2 (doorway-federation-failover). A hosted user landing on a
//! *sibling* doorway must get an honest redirect to their home doorway rather
//! than a 502 or a silently-forked empty identity. The sibling can only do that
//! if "where is this agent hosted?" is answerable from substrate truth — hence
//! this binding lives on the DHT, not in one doorway's Mongo.
//!
//! ## p2p-design-gate classification (re-confirmed in-session, 2026-07-31)
//!
//! - **Category A2 — Derived**: a *relationship* between two already-notarized
//!   entities (the human's imagodei agent identity ↔ the home doorway's
//!   `DoorwayRegistration` in the infrastructure DNA). NOT a standalone entry
//!   type.
//! - **Content address strategy**: Agent-Scoped Composite —
//!   `(AgentPubKey, "hosted-at")`. The human's stance toward a doorway *is* the
//!   identity. Not content-derived (it is a mutable relationship), not a slug.
//! - **Cross-DNA**: `DoorwayRegistration` lives in the *infrastructure* DNA, so
//!   it cannot be a link target from imagodei. Per the gate output, the doorway
//!   reference therefore travels as **tag data** (`doorway_id` + `doorway_url` +
//!   `installed_app_id`), never as a cross-DNA link target.
//!
//! ## DNA-hash neutrality (hard constraint)
//!
//! Adding a new integrity link type or entry type to imagodei would move the DNA
//! hash → fleet partition. This module therefore **reuses existing integrity
//! surface only**:
//!
//! - Link type: [`LinkTypes::PeerToBinding`] — already declared, already
//!   `StringAnchor`-based. A doorway *is* a peer that hosts an agent, so the
//!   agent↔host-peer relation is the same family as the agent↔libp2p-peer
//!   relation the variant was minted for.
//! - Anchor entry type: `StringAnchor` — already declared.
//! - **Anchor-base disjointness**: bindings anchor on
//!   `StringAnchor("hosted_at", <agent_pub_key>)`, while
//!   `get_bindings_for_peer` anchors on `StringAnchor("peer_binding", <peer_id>)`.
//!   Different anchor entries hash differently, so the two link sets never
//!   overlap in either direction. This is the house style already documented on
//!   `LinkTypes::CharterAnchor` ("distinct anchor base, queries never overlap").
//! - Link validation in `imagodei_integrity` is
//!   `FlatOp::Link(OpLink::CreateLink { .. }) => Valid` — unconditional — so no
//!   validator change is required either.
//!
//! Everything here is **coordinator-zome only**: the DNA hash covers integrity
//! zomes + modifiers, so this ships via the `update_coordinators` hot-swap path
//! (`ALLOW_COORDINATOR_UPDATE`), never a reinstall/re-key.
//!
//! ## Delegate write semantics
//!
//! The doorway calls this as the *user's delegate*: it authorizes signing
//! credentials for the user's own hosted cell and issues the zome call through
//! it, so `agent_info()` inside this function is the **user's** agent key. There
//! is no third-party write path — an agent can only ever bind itself.

use hdk::prelude::*;
use imagodei_integrity::*;

use crate::ImagodeiSignal;

/// Anchor type prefix for hosted-at bindings.
///
/// Deliberately distinct from `"peer_binding"` (the `AgentPeerBinding` reverse
/// index that shares `LinkTypes::PeerToBinding`) so the two link sets are
/// disjoint by construction — see the module doc.
pub const HOSTED_AT_ANCHOR: &str = "hosted_at";

/// Hard ceiling on the serialized link tag, per the design-gate output.
/// Holochain permits larger tags; this bound keeps the binding cheap to gossip
/// and forces oversized inputs to fail loudly at write time rather than
/// silently bloating the DHT.
pub const MAX_HOSTED_BINDING_TAG_BYTES: usize = 256;

// =============================================================================
// I/O types
// =============================================================================

/// Input to `create_hosted_agent_binding`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateHostedAgentBindingInput {
    /// Stable id of the home doorway (matches the `DoorwayRegistration`
    /// `doorway_id` in the infrastructure DNA, and the `kid` this doorway
    /// publishes at `/.well-known/doorway-keys`).
    pub doorway_id: String,
    /// Public base URL of the home doorway (e.g. `https://elohim.host`).
    pub doorway_url: String,
    /// Conductor-side installed app id for this human's hosted install.
    pub installed_app_id: String,
}

/// Output from `create_hosted_agent_binding`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateHostedAgentBindingOutput {
    /// `ActionHash` of the CreateLink action — the DHT anchor for the storage
    /// projection's `dht_anchor_hash` column.
    pub action_hash: ActionHashB64,
}

/// Wire view of a hosted-at binding.
///
/// Hash fields are `*B64` (base64 strings) rather than raw hashes so the view
/// round-trips through `serde_json::Value` reads in sweettests — msgpack `BIN`
/// has no `Value` variant (see `AgentPeerBindingView` for the same treatment).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedAgentBindingView {
    pub agent_pub_key: AgentPubKeyB64,
    pub doorway_id: String,
    pub doorway_url: String,
    pub installed_app_id: String,
    /// Microseconds since epoch when the binding link was created.
    pub bound_at_micros: i64,
    /// `ActionHash` of the CreateLink action.
    pub action_hash: ActionHashB64,
}

/// The serialized link-tag payload. Field names are single-letter to stay well
/// inside [`MAX_HOSTED_BINDING_TAG_BYTES`].
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostedBindingTag {
    /// doorway_id
    d: String,
    /// doorway_url
    u: String,
    /// installed_app_id
    a: String,
    /// bound_at, microseconds since epoch
    t: i64,
}

// =============================================================================
// Anchor helper
// =============================================================================

/// Deterministic anchor hash for an agent's hosted-at binding set.
fn hosted_at_anchor_hash(agent_pub_key: &AgentPubKey) -> ExternResult<EntryHash> {
    let anchor = StringAnchor::new(HOSTED_AT_ANCHOR, &agent_pub_key.to_string());
    hash_entry(&EntryTypes::StringAnchor(anchor))
}

// =============================================================================
// create_hosted_agent_binding
// =============================================================================

/// Record that the calling agent is hosted at the given doorway.
///
/// ## Link created
/// `PeerToBinding`: `StringAnchor("hosted_at", <caller_pubkey>)` → `<caller_pubkey>`,
/// tag = `{d,u,a,t}` JSON.
///
/// The link **target is the caller's own AgentPubKey**, not an entry hash: the
/// binding carries all of its payload in the tag, so no entry needs to exist.
/// This is what keeps the change hash-neutral — no new entry type.
///
/// ## Signal emitted
/// `ImagodeiSignal::HostedAgentBindingCreated` — projected by elohim-storage's
/// `ReconcileController` into the `hosted_agent_bindings` table.
#[hdk_extern]
pub fn create_hosted_agent_binding(
    input: CreateHostedAgentBindingInput,
) -> ExternResult<CreateHostedAgentBindingOutput> {
    if input.doorway_id.trim().is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "doorway_id cannot be empty".to_string()
        )));
    }
    if input.doorway_url.trim().is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "doorway_url cannot be empty".to_string()
        )));
    }

    let caller = agent_info()?.agent_initial_pubkey;
    let bound_at = sys_time()?.as_micros();

    let tag_payload = HostedBindingTag {
        d: input.doorway_id.clone(),
        u: input.doorway_url.clone(),
        a: input.installed_app_id.clone(),
        t: bound_at,
    };
    let tag_bytes = encode_hosted_binding_tag(&tag_payload)?;

    // Commit the anchor entry so peers can fetch it directly via
    // `get(anchor_hash)` — matches the AgentPeerBinding / M4 recovery pattern.
    let anchor = StringAnchor::new(HOSTED_AT_ANCHOR, &caller.to_string());
    create_entry(&EntryTypes::StringAnchor(anchor.clone()))?;
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let action_hash = create_link(
        anchor_hash,
        caller.clone(),
        LinkTypes::PeerToBinding,
        LinkTag::new(tag_bytes),
    )?;

    emit_signal(ImagodeiSignal::HostedAgentBindingCreated {
        action_hash: action_hash.clone(),
        agent_pub_key: caller,
        doorway_id: input.doorway_id,
        doorway_url: input.doorway_url,
        installed_app_id: input.installed_app_id,
        bound_at_micros: bound_at,
    })?;

    Ok(CreateHostedAgentBindingOutput {
        action_hash: action_hash.into(),
    })
}

// =============================================================================
// get_hosted_agent_binding
// =============================================================================

/// Resolve the current hosted-at binding for an agent, or `None`.
///
/// When several bindings exist (a human migrated between doorways), the one with
/// the newest `bound_at` wins — the binding is a mutable relationship, and the
/// last write is the current home.
#[hdk_extern]
pub fn get_hosted_agent_binding(
    agent_pub_key: AgentPubKey,
) -> ExternResult<Option<HostedAgentBindingView>> {
    let anchor_hash = hosted_at_anchor_hash(&agent_pub_key)?;
    let query = LinkQuery::try_new(anchor_hash, LinkTypes::PeerToBinding)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut best: Option<HostedAgentBindingView> = None;
    for link in links {
        let Some(tag) = decode_hosted_binding_tag(link.tag.as_ref()) else {
            // A `PeerToBinding` link whose tag is not a hosted-binding payload
            // is not ours. Degrade per-row (skip), never fail the whole read —
            // one poisoned row must not empty a resolution table.
            continue;
        };
        let view = HostedAgentBindingView {
            agent_pub_key: agent_pub_key.clone().into(),
            doorway_id: tag.d,
            doorway_url: tag.u,
            installed_app_id: tag.a,
            bound_at_micros: tag.t,
            action_hash: link.create_link_hash.clone().into(),
        };
        best = match best {
            Some(prev) if prev.bound_at_micros >= view.bound_at_micros => Some(prev),
            _ => Some(view),
        };
    }

    Ok(best)
}

// =============================================================================
// Tag codec (pure — unit-testable without a conductor)
// =============================================================================

fn encode_hosted_binding_tag(tag: &HostedBindingTag) -> ExternResult<Vec<u8>> {
    let bytes = serde_json::to_vec(tag).map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "failed to encode hosted-binding tag: {e}"
        )))
    })?;
    if bytes.len() > MAX_HOSTED_BINDING_TAG_BYTES {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "hosted-binding tag is {} bytes; limit is {}",
            bytes.len(),
            MAX_HOSTED_BINDING_TAG_BYTES
        ))));
    }
    Ok(bytes)
}

fn decode_hosted_binding_tag(bytes: &[u8]) -> Option<HostedBindingTag> {
    serde_json::from_slice::<HostedBindingTag>(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tag() -> HostedBindingTag {
        HostedBindingTag {
            d: "alpha-a".to_string(),
            u: "https://doorway-alpha.elohim.host".to_string(),
            a: "elohim-hosted-abcdef0123456789".to_string(),
            t: 1_754_000_000_000_000,
        }
    }

    #[test]
    fn tag_round_trips() {
        let tag = sample_tag();
        let bytes = encode_hosted_binding_tag(&tag).expect("encode");
        let back = decode_hosted_binding_tag(&bytes).expect("decode");
        assert_eq!(back.d, tag.d);
        assert_eq!(back.u, tag.u);
        assert_eq!(back.a, tag.a);
        assert_eq!(back.t, tag.t);
    }

    #[test]
    fn realistic_tag_fits_the_budget() {
        let bytes = encode_hosted_binding_tag(&sample_tag()).expect("encode");
        assert!(
            bytes.len() <= MAX_HOSTED_BINDING_TAG_BYTES,
            "realistic tag was {} bytes",
            bytes.len()
        );
    }

    #[test]
    fn oversized_tag_is_rejected_not_truncated() {
        let mut tag = sample_tag();
        tag.a = "x".repeat(MAX_HOSTED_BINDING_TAG_BYTES);
        assert!(encode_hosted_binding_tag(&tag).is_err());
    }

    #[test]
    fn foreign_tag_decodes_to_none() {
        // An `AgentPeerBinding` reverse link carries a unit tag `()`; a
        // hosted-binding reader must skip it rather than mis-parse it.
        assert!(decode_hosted_binding_tag(&[]).is_none());
        assert!(decode_hosted_binding_tag(b"not-json").is_none());
        assert!(decode_hosted_binding_tag(br#"{"peerId":"12D3Koo"}"#).is_none());
    }
}
