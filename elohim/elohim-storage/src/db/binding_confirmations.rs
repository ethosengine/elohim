//! CRUD for `binding_confirmations` — which cross-signed bindings this node has seen
//! ON THE DHT.
//!
//! Source of truth: the DHT (this node's own conductor's `AgentPeerBinding` arrival
//! signal, or a `get_bindings_for_peer` read of the network). A confirmation records
//! the exact peer, agent and validity window the DHT entry at `dht_anchor_hash`
//! carries; `p2p::transport_authority` counts a cross-signed binding as authority only
//! when a confirmation with those same fields exists. Kept apart from
//! `peer_identity_bindings`, whose gossip and handshake writers must never be able to
//! forge or erase it. Category C.

use diesel::prelude::*;

use super::diesel_schema::binding_confirmations;
use crate::db::models::PeerIdentityBindingRow;
use crate::error::StorageError;

/// The DHT fields a confirmation vouches for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DhtBinding<'a> {
    pub peer_id: &'a str,
    pub dht_anchor_hash: &'a str,
    pub agent_cid: &'a str,
    pub valid_from: &'a str,
    pub valid_until: Option<&'a str>,
}

/// Record that the DHT carries `binding` (idempotent; the newest read wins).
pub fn confirm(conn: &mut SqliteConnection, binding: &DhtBinding<'_>) -> Result<(), StorageError> {
    diesel::replace_into(binding_confirmations::table)
        .values((
            binding_confirmations::peer_id.eq(binding.peer_id),
            binding_confirmations::dht_anchor_hash.eq(binding.dht_anchor_hash),
            binding_confirmations::agent_cid.eq(binding.agent_cid),
            binding_confirmations::valid_from.eq(binding.valid_from),
            binding_confirmations::valid_until.eq(binding.valid_until),
        ))
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("binding_confirmations::confirm: {e}")))
}

/// Does a confirmation vouch for exactly this projected row (same anchor, peer, agent
/// and validity window)?
pub fn confirms(conn: &mut SqliteConnection, row: &PeerIdentityBindingRow) -> bool {
    binding_confirmations::table
        .find((&row.peer_id, &row.dht_anchor_hash))
        .select((
            binding_confirmations::agent_cid,
            binding_confirmations::valid_from,
            binding_confirmations::valid_until,
        ))
        .first::<(String, String, Option<String>)>(conn)
        .optional()
        .ok()
        .flatten()
        .is_some_and(|(agent, from, until)| {
            agent == row.agent_cid && from == row.valid_from && until == row.valid_until
        })
}
