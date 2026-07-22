//! Transport-identity resolution for the shard PUSH path.
//!
//! # Why this exists
//!
//! `peer_selection` (and every custody/resilience join) speaks `agent_cid`
//! (`uhCAk…`) — the canonical identity join key. But `P2PNode::push_shard` must
//! dial a libp2p `PeerId` (bs58 multihash). An `agent_cid` NEVER parses as a
//! libp2p `PeerId`, so `distribute_shards` could historically never push a single
//! shard live: every call errored at `peer_id.parse::<PeerId>()` and
//! `shard_locations` was only ever populated by other paths (placement gaps got
//! recorded instead). This is the push-side twin of the FETCH-side resolver
//! (`p2p/mod.rs` inventory task), which already resolves `agent_cid → libp2p` via
//! `peer_transport_manifest`.
//!
//! # Transport-layer only
//!
//! Resolution happens at SEND time and translates ONLY the dial target. The
//! identity stored in `shard_locations.peer_id` stays the `agent_cid` (the
//! resilience card's join key) — we never write a transport id into an
//! `agent_cid` column. See `elohim-storage/CLAUDE.md` "Identity &
//! Transport-Identity Coherence".
//!
//! # Sources (first hit wins) — never manifest-lookup alone
//!
//! Mirrors the fetch side's "manifest first, then fallback":
//!  1. `peer_transport_manifest` (`peer_map::lookup_by_agent_cid`) — the canonical
//!     `agent_cid ↔ transport` projection, populated by iroh-plane observations.
//!     Feature-gated on `p2p-iroh` (the manifest table lives in `p2p_iroh`).
//!  2. `peer_identity_bindings` (`list_active_for_agent`) — the AgentPeerBinding /
//!     identity-handshake projection (libp2p `source` rows). Always available;
//!     the load-bearing fallback when the manifest is empty (as in prod today).
//!
//! A still-unresolvable `agent_cid` returns `None`; the caller SKIPS that peer
//! with a log + metric and records the placement gap, never a hard error.

use diesel::sqlite::SqliteConnection;

/// True when `s` is a plausible dial target: non-empty and NOT an `agent_cid`
/// (`uhCAk…`) landing in a transport column. The final validity gate is
/// `push_shard`'s `PeerId::parse`; this only refuses the known-wrong
/// cross-namespace value so a mis-populated row can never be dialed as itself.
fn is_dialable_transport_id(s: &str) -> bool {
    let t = s.trim();
    !t.is_empty() && !crate::identity_namespace::is_agent_cid(t)
}

/// Resolve an `agent_cid` (`uhCAk…`) to a dialable libp2p `PeerId` string, or
/// `None` when no transport binding is known yet. Pure lookup — no I/O beyond the
/// two DB reads. See module docs for source order and rationale.
pub fn resolve_agent_cid_to_libp2p(conn: &mut SqliteConnection, agent_cid: &str) -> Option<String> {
    let cid = agent_cid.trim();
    if cid.is_empty() {
        return None;
    }

    // Defensive passthrough: a caller that already holds a transport id (not the
    // `agent_cid` peer_selection yields) needs no resolution. Keeps the resolver
    // correct if a libp2p id ever reaches it directly.
    if crate::identity_namespace::is_libp2p_peer_id(cid) {
        return Some(cid.to_string());
    }

    // 1. Canonical transport manifest (iroh-plane populated).
    #[cfg(feature = "p2p-iroh")]
    {
        if let Ok(Some(manifest)) = crate::p2p_iroh::peer_map::lookup_by_agent_cid(conn, cid) {
            if let Some(profile) = manifest.libp2p {
                if is_dialable_transport_id(&profile.peer_id) {
                    return Some(profile.peer_id);
                }
            }
        }
    }

    // 2. Identity-handshake / AgentPeerBinding projection (most-recent first).
    let now_iso = chrono::Utc::now().to_rfc3339();
    if let Ok(bindings) =
        crate::db::peer_identity_bindings::list_active_for_agent(conn, cid, &now_iso)
    {
        for b in bindings {
            if is_dialable_transport_id(&b.peer_id) {
                return Some(b.peer_id);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::NewPeerIdentityBindingRow;
    use crate::db::DbPool;
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::SqliteConnection;

    const AGENT: &str = "uhCAkResolveMeExampleAgentCidForTransportResolve";
    const LIBP2P: &str = "12D3KooWResolvedLibp2pPeerIdForTransportResolveTest";

    fn test_pool() -> DbPool {
        let url = format!(
            "file:transport_resolve_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        crate::db::run_migrations(&pool).expect("migrations");
        pool
    }

    fn seed_binding(conn: &mut SqliteConnection, agent_cid: &str, peer_id: &str) {
        crate::db::peer_identity_bindings::upsert(
            conn,
            &NewPeerIdentityBindingRow {
                peer_id: peer_id.to_string(),
                agent_cid: agent_cid.to_string(),
                dht_anchor_hash: format!("anchor-{peer_id}"),
                valid_from: "2020-01-01T00:00:00Z".to_string(),
                valid_until: None,
                observed_at: "2026-01-01T00:00:00Z".to_string(),
                source: "handshake".to_string(),
                device_archetype: "unknown".to_string(),
                superseded_by: None,
            },
        )
        .expect("seed binding");
    }

    #[test]
    fn resolves_via_identity_binding_fallback() {
        // The manifest is empty (prod shape); the binding projection carries the
        // libp2p id — the load-bearing fallback.
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        seed_binding(&mut conn, AGENT, LIBP2P);

        assert_eq!(
            resolve_agent_cid_to_libp2p(&mut conn, AGENT).as_deref(),
            Some(LIBP2P)
        );
    }

    #[cfg(feature = "p2p-iroh")]
    #[test]
    fn manifest_hit_wins_over_binding() {
        // With both sources populated, the canonical manifest wins.
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let manifest_libp2p = "12D3KooWManifestWinsOverTheBindingProjectionEntry";
        crate::p2p_iroh::peer_map::record_libp2p_observation(
            &mut conn,
            AGENT,
            manifest_libp2p,
            &[],
            &[],
            1_767_225_600,
        )
        .expect("seed manifest");
        seed_binding(&mut conn, AGENT, LIBP2P);

        assert_eq!(
            resolve_agent_cid_to_libp2p(&mut conn, AGENT).as_deref(),
            Some(manifest_libp2p),
            "manifest is the canonical source and must win over the binding fallback"
        );
    }

    #[test]
    fn unresolvable_agent_cid_returns_none() {
        // No manifest row, no binding → None (caller SKIPS + records the gap).
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        assert_eq!(resolve_agent_cid_to_libp2p(&mut conn, AGENT), None);
    }

    #[test]
    fn empty_input_returns_none() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        assert_eq!(resolve_agent_cid_to_libp2p(&mut conn, ""), None);
        assert_eq!(resolve_agent_cid_to_libp2p(&mut conn, "   "), None);
    }

    #[test]
    fn already_libp2p_input_passes_through() {
        // A caller holding a libp2p id directly needs no DB lookup.
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        assert_eq!(
            resolve_agent_cid_to_libp2p(&mut conn, LIBP2P).as_deref(),
            Some(LIBP2P)
        );
    }

    #[test]
    fn binding_with_non_dialable_peer_id_is_skipped() {
        // A coherence-broken binding row that stored an agent_cid in peer_id must
        // NOT be returned as a dial target (it would fail PeerId::parse anyway).
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        seed_binding(&mut conn, AGENT, "uhCAkNotALibp2pIdWrongNamespaceInPeerId");
        assert_eq!(
            resolve_agent_cid_to_libp2p(&mut conn, AGENT),
            None,
            "an agent_cid mistakenly in peer_id is not a dialable transport id"
        );
    }
}
