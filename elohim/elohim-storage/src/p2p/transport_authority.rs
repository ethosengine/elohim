//! Which agent a transport identity speaks for — the ONE authority read.
//!
//! Every admission that binds a signed claim to an agent asks this module, and only
//! this module: salvage ads (`p2p::salvage_gossip`), custody announcements
//! (`p2p::custody_announce`), inventory envelopes (`p2p::inventory_gossip`'s holder
//! resolver) and the custody floor's identity join (`reconcile::custody`).
//!
//! Authority comes from a cross-signed chain and nothing else:
//!
//! - a libp2p PeerId resolves through an active, non-superseded
//!   `peer_identity_bindings` row that classified `cross_signed`
//!   (`p2p::binding_proof_wire::classify_binding_signature`: the agent key and the
//!   transport key each signed the binding);
//! - an iroh NodeId resolves only through a verified [`TransportLink`] to a libp2p
//!   PeerId (both keys signed it), and then through that PeerId's cross-signed
//!   binding.
//!
//! `peer_transport_manifest` is never read here. Its `agent_cid` is a self-asserted
//! routing hint (anyone can announce `{iroh_node_id: K, agent_cid: victim}`); it may
//! decide where to dial, never whom a claim speaks for.
//!
//! An identity with no such chain resolves to `None` — including a peer whose
//! manifest carries no transport link yet (an older build). Callers refuse its claims
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

/// The active cross-signed bindings for one libp2p PeerId.
fn cross_signed_for_peer(
    conn: &mut SqliteConnection,
    peer_id: &str,
    now: &str,
) -> Vec<PeerIdentityBindingRow> {
    use peer_identity_bindings::dsl;
    dsl::peer_identity_bindings
        .filter(dsl::peer_id.eq(peer_id))
        .filter(dsl::valid_from.le(now))
        .filter(dsl::valid_until.is_null().or(dsl::valid_until.gt(now)))
        .filter(dsl::superseded_by.is_null())
        .load::<PeerIdentityBindingRow>(conn)
        .unwrap_or_default()
        .into_iter()
        .filter(PeerIdentityBindingRow::is_cross_signed)
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
    let rows = cross_signed_for_peer(conn, &peer_id, &now_iso());
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
