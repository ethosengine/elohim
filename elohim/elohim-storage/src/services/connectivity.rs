//! ## Source of Truth
//!
//! Operational helper (Category C). Ephemeral — consumes a snapshot
//! of libp2p `connected_peers()` taken by the caller at query time.
//! No persistence. Reads, never writes.

use std::collections::HashSet;

/// Returns `true` if `peer_id` is present in the connected-peers snapshot.
pub fn is_online(peer_id: &str, snapshot: &HashSet<String>) -> bool {
    snapshot.contains(peer_id)
}

/// Returns `true` if any of `peers` is present in the connected-peers snapshot.
pub fn any_online_in(peers: &[&str], snapshot: &HashSet<String>) -> bool {
    peers.iter().any(|p| snapshot.contains(*p))
}
