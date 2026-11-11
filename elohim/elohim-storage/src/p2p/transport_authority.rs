//! Which agent a transport identity speaks for — the ONE authority read.
//!
//! Every admission that binds a signed claim to an agent asks this module, and only
//! this module: salvage ads (`p2p::salvage_gossip`), custody announcements
//! (`p2p::custody_announce`), inventory envelopes (`p2p::inventory_gossip`'s holder
//! resolver) and the custody floor's identity join (`reconcile::custody`).
//!
//! Authority comes from a cross-signed chain and nothing else:
//!
//! - a libp2p PeerId resolves through an active `peer_identity_bindings` row that
//!   classified `cross_signed` (`p2p::binding_proof_wire::classify_binding_signature`:
//!   the agent key and the transport key each signed the binding) AND that the DHT
//!   has confirmed (`db::binding_confirmations`: an `AgentPeerBinding` entry at the
//!   row's anchor with the same peer, agent and window). Cross-signing alone proves
//!   only that the sender holds both keys — anyone can mint N such pairs and gossip
//!   them — so a binding that only arrived by gossip is a hint that requests
//!   confirmation (`p2p::transport_authority::confirm`), never authority by itself;
//! - supersession applies to the signed core, not to the anchor: if ANY row carrying
//!   the same signed core (agent, peer, kind, window, nonce, issued_at) is superseded,
//!   every copy of that core is, so an old core re-published under a fresh anchor
//!   does not revive;
//! - an iroh NodeId resolves only through a verified [`TransportLink`] to a libp2p
//!   PeerId (both keys signed it), and then through that PeerId's cross-signed
//!   binding.
//!
//! `peer_transport_manifest` is never read here. Its `agent_cid` is a self-asserted
//! routing hint (anyone can announce `{iroh_node_id: K, agent_cid: victim}`); it may
//! decide where to dial, never whom a claim speaks for.
//!
//! An identity with no such chain resolves to `None` — including a peer whose
//! manifest carries no transport link yet (an older build). Callers hold its claims
//! as unbound: never projected, never counted.
//!
//! [`TransportLink`]: crate::p2p::transport_link::TransportLink

use diesel::prelude::*;

use crate::db::diesel_schema::peer_identity_bindings;
use crate::db::models::PeerIdentityBindingRow;

/// Is `id` spelled as an iroh NodeId (64 lowercase hex)? A base58 libp2p PeerId never
/// is, so the namespace is decided by the identifier, never by a claim about it.
pub fn is_iroh_node_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// The identity of a binding's SIGNED core — what both keys signed — independent of
/// the unsigned `dht_anchor_hash` it was projected under. `None` for a row whose
/// signature carries no proof (a sentinel can never be authority anyway).
pub fn core_identity(row: &PeerIdentityBindingRow) -> Option<String> {
    core_identity_of(
        &row.agent_cid,
        &row.peer_id,
        &row.valid_from,
        row.valid_until.as_deref(),
        &row.signature,
    )
}

/// [`core_identity`] from the fields themselves (a gossip payload, before projection).
pub fn core_identity_of(
    agent_cid: &str,
    peer_id: &str,
    valid_from: &str,
    valid_until: Option<&str>,
    signature: &str,
) -> Option<String> {
    let proof = crate::p2p::binding_proof_wire::decode_proof(signature).ok()?;
    let mut hasher = blake3::Hasher::new();
    for part in [
        agent_cid,
        peer_id,
        &proof.transport_kind.to_string(),
        valid_from,
        valid_until.unwrap_or("\u{0}none"),
        proof.nonce.as_str(),
        proof.issued_at.as_str(),
    ] {
        hasher.update(&(part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    Some(hasher.finalize().to_hex().to_string())
}

/// The bindings that are AUTHORITY for one libp2p PeerId: cross-signed, inside their
/// window, DHT-confirmed, and whose signed core no row marks superseded.
fn authority_for_peer(
    conn: &mut SqliteConnection,
    peer_id: &str,
    now: &str,
) -> Vec<PeerIdentityBindingRow> {
    use peer_identity_bindings::dsl;
    let rows: Vec<PeerIdentityBindingRow> = dsl::peer_identity_bindings
        .filter(dsl::peer_id.eq(peer_id))
        .load::<PeerIdentityBindingRow>(conn)
        .unwrap_or_default();
    let superseded_cores: std::collections::HashSet<String> = rows
        .iter()
        .filter(|row| row.superseded_by.is_some())
        .filter_map(core_identity)
        .collect();
    rows.into_iter()
        .filter(|row| {
            row.superseded_by.is_none()
                && row.is_cross_signed()
                && row.valid_from.as_str() <= now
                && row.valid_until.as_deref().is_none_or(|until| until > now)
                && core_identity(row).is_some_and(|core| !superseded_cores.contains(&core))
        })
        .filter(|row| crate::db::binding_confirmations::confirms(conn, row))
        .collect()
}

/// The agent `transport_id` (a libp2p PeerId or an iroh NodeId) is authenticated to
/// speak for, or `None`. Two cross-signed bindings naming different agents for one
/// PeerId are ambiguous and resolve to `None` (fail closed).
pub fn authenticated_agent_for_transport(
    conn: &mut SqliteConnection,
    transport_id: &str,
) -> Option<String> {
    let peer_id = if is_iroh_node_id(transport_id) {
        crate::db::transport_links::libp2p_for_iroh(conn, transport_id).ok()??
    } else {
        transport_id.to_string()
    };
    let rows = authority_for_peer(conn, &peer_id, &now_iso());
    let mut agents = rows.into_iter().map(|row| row.agent_cid);
    let agent = agents.next()?;
    agents.all(|other| other == agent).then_some(agent)
}

/// Every transport id authenticated to speak for `agent_cid`: the libp2p PeerIds its
/// active cross-signed bindings name, and the iroh NodeIds verified links tie to
/// them. The custody floor reads an agent's evidence under exactly these spellings.
pub fn authenticated_transports_for_agent(
    conn: &mut SqliteConnection,
    agent_cid: &str,
) -> Vec<String> {
    let rows = crate::db::peer_identity_bindings::list_cross_signed_for_agents(
        conn,
        std::slice::from_ref(&agent_cid.to_string()),
        &now_iso(),
    )
    .unwrap_or_default();
    let mut ids: Vec<String> = Vec::new();
    for row in rows {
        // A PeerId with a second, conflicting cross-signed agent speaks for no one.
        if authenticated_agent_for_transport(conn, &row.peer_id).as_deref() == Some(agent_cid)
            && !ids.contains(&row.peer_id)
        {
            ids.push(row.peer_id);
        }
    }
    let iroh = crate::db::transport_links::iroh_for_libp2p(conn, &ids).unwrap_or_default();
    for id in iroh {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

/// DHT confirmation of gossip-arrived cross-signed bindings (the confirmer task).
pub mod confirm;
