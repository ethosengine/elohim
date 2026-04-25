//! Peer identity mapping — `PeerIdentityMap` trait + production implementation.
//!
//! Maps a libp2p PeerId to an agent CID string for reach enforcement.
//!
//! ## Implementations
//!
//! - `HolochainBackedPeerIdentityMap` — the production implementation. Reads from
//!   the `peer_identity_bindings` SQLite projection (Category C operational table).
//!   Populated by the ReconcileController as AgentPeerBinding DHT signals arrive.
//!
//! - `StubIdentityMap` — kept for the integration test harness in
//!   `tests/harness/mod.rs`. Not used by production `P2PNode` construction.
//!   Allows tests to register peer→agent mappings in-memory without a real DB.

use libp2p::PeerId;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::db::DbPool;

/// Identity of the remote caller for reach enforcement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallerIdentity {
    /// No identity established — serves only Commons/Public.
    Anonymous,
    /// Identity established; the string is a stable agent CID reference.
    Agent(String),
}

/// Trait allowing the swarm node to resolve a PeerId into a CallerIdentity.
///
/// Implementations must be `Send + Sync + 'static` so they can be held behind
/// `Arc<dyn PeerIdentityMap>` inside `P2PNode`.
pub trait PeerIdentityMap: Send + Sync + 'static {
    fn lookup(&self, peer: &PeerId) -> CallerIdentity;
}

// ============================================================================
// Production implementation — DHT-backed via peer_identity_bindings table
// ============================================================================

/// Production peer identity map backed by the `peer_identity_bindings` SQLite
/// table (Category C operational projection of AgentPeerBinding DHT entries).
///
/// `lookup` is synchronous: it takes a pooled r2d2 connection (blocking but
/// bounded) and queries the table for the most recently observed, currently-
/// active binding. No async machinery required — r2d2 `pool.get()` is sync.
///
/// The table is populated by the ReconcileController's `on_agent_peer_binding`
/// handler (Task A.4 / A.8). This struct is read-only.
pub struct HolochainBackedPeerIdentityMap {
    pool: DbPool,
}

impl HolochainBackedPeerIdentityMap {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

impl PeerIdentityMap for HolochainBackedPeerIdentityMap {
    fn lookup(&self, peer: &PeerId) -> CallerIdentity {
        let peer_id_str = peer.to_base58();
        let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    peer = %peer_id_str,
                    error = %e,
                    "peer_identity_bindings: pool exhausted — treating as Anonymous"
                );
                return CallerIdentity::Anonymous;
            }
        };

        match crate::db::peer_identity_bindings::lookup_active(&mut conn, &peer_id_str, &now_iso) {
            Ok(Some(row)) => CallerIdentity::Agent(row.agent_cid),
            Ok(None) => CallerIdentity::Anonymous,
            Err(e) => {
                tracing::warn!(
                    peer = %peer_id_str,
                    error = %e,
                    "peer_identity_bindings: lookup error — treating as Anonymous"
                );
                CallerIdentity::Anonymous
            }
        }
    }
}

// ============================================================================
// Stub implementation — test harness only, not constructed by P2PNode
// ============================================================================

/// In-memory stub — anonymous for all peers unless explicitly registered.
///
/// Used exclusively by the integration test harness in `tests/harness/mod.rs`
/// so tests can register peer→agent mappings without a real database.
/// Production code uses `HolochainBackedPeerIdentityMap`.
#[derive(Default, Debug)]
pub struct StubIdentityMap {
    inner: RwLock<HashMap<PeerId, String>>,
}

impl StubIdentityMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Test-only: register a peer → agent CID mapping.
    pub fn register(&self, peer: PeerId, agent_cid: impl Into<String>) {
        self.inner.write().unwrap().insert(peer, agent_cid.into());
    }
}

impl PeerIdentityMap for StubIdentityMap {
    fn lookup(&self, peer: &PeerId) -> CallerIdentity {
        self.inner
            .read()
            .unwrap()
            .get(peer)
            .cloned()
            .map(CallerIdentity::Agent)
            .unwrap_or(CallerIdentity::Anonymous)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::NewPeerIdentityBindingRow;
    use crate::db::{run_migrations, DbPool};
    use diesel::prelude::*;
    use diesel::r2d2::{ConnectionManager, Pool};

    // -------------------------------------------------------------------------
    // Shared test helpers
    // -------------------------------------------------------------------------

    fn test_pool() -> DbPool {
        let url = format!(
            "file:idmap_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn insert_binding(pool: &DbPool, peer_id: &str, agent_cid: &str, source: &str) {
        use crate::db::diesel_schema::peer_identity_bindings::dsl;
        let mut conn = pool.get().expect("conn");
        diesel::replace_into(dsl::peer_identity_bindings)
            .values(&NewPeerIdentityBindingRow {
                peer_id: peer_id.to_string(),
                agent_cid: agent_cid.to_string(),
                dht_anchor_hash: format!("anchor-{peer_id}"),
                valid_from: "2026-01-01T00:00:00Z".to_string(),
                valid_until: None,
                observed_at: "2026-04-24T00:00:00Z".to_string(),
                source: source.to_string(),
            })
            .execute(&mut conn)
            .expect("insert binding");
    }

    fn insert_binding_with_expiry(
        pool: &DbPool,
        peer_id: &str,
        agent_cid: &str,
        valid_until: &str,
    ) {
        use crate::db::diesel_schema::peer_identity_bindings::dsl;
        let mut conn = pool.get().expect("conn");
        diesel::replace_into(dsl::peer_identity_bindings)
            .values(&NewPeerIdentityBindingRow {
                peer_id: peer_id.to_string(),
                agent_cid: agent_cid.to_string(),
                dht_anchor_hash: format!("anchor-{peer_id}"),
                valid_from: "2026-01-01T00:00:00Z".to_string(),
                valid_until: Some(valid_until.to_string()),
                observed_at: "2026-04-24T00:00:00Z".to_string(),
                source: "dht".to_string(),
            })
            .execute(&mut conn)
            .expect("insert expiring binding");
    }

    // -------------------------------------------------------------------------
    // HolochainBackedPeerIdentityMap tests
    // -------------------------------------------------------------------------

    #[test]
    fn holochain_backed_map_resolves_binding_from_table() {
        let pool = test_pool();
        insert_binding(&pool, "12D3KooWXyz", "bafyreiabc", "dht");
        let map = HolochainBackedPeerIdentityMap::new(pool);

        // Construct a PeerId whose base58 encoding is "12D3KooWXyz".
        // Because we store the peer_id as peer.to_base58() on write, we need to
        // do the same on lookup. We insert with the raw string directly, so we
        // need a peer whose base58 matches. Since libp2p PeerIds are opaque, we
        // instead insert with the string derived from a known PeerId.
        //
        // Strategy: generate a PeerId, record its base58, insert that, then look up.
        let peer = PeerId::random();
        let peer_b58 = peer.to_base58();
        // Re-insert with the correct base58 key
        insert_binding(&map.pool, &peer_b58, "bafyreinew", "dht");

        let identity = map.lookup(&peer);
        assert!(
            matches!(identity, CallerIdentity::Agent(ref cid) if cid == "bafyreinew"),
            "expected Agent(bafyreinew), got {identity:?}"
        );
    }

    #[test]
    fn holochain_backed_map_returns_anonymous_for_unknown_peer() {
        let pool = test_pool();
        let map = HolochainBackedPeerIdentityMap::new(pool);
        let peer = PeerId::random();
        let identity = map.lookup(&peer);
        assert_eq!(identity, CallerIdentity::Anonymous);
    }

    #[test]
    fn holochain_backed_map_excludes_expired_bindings() {
        let pool = test_pool();
        let peer = PeerId::random();
        let peer_b58 = peer.to_base58();
        // Binding expired in the past (2026-01-01 → 2026-02-01; "now" is 2026-04-24)
        insert_binding_with_expiry(&pool, &peer_b58, "bafyreiabc", "2026-02-01T00:00:00Z");
        let map = HolochainBackedPeerIdentityMap::new(pool);
        let identity = map.lookup(&peer);
        assert_eq!(
            identity,
            CallerIdentity::Anonymous,
            "expired binding must resolve to Anonymous"
        );
    }

    // -------------------------------------------------------------------------
    // StubIdentityMap tests (legacy — kept for harness regression coverage)
    // -------------------------------------------------------------------------

    #[test]
    fn stub_anonymous_by_default() {
        let peer = PeerId::random();
        let map = StubIdentityMap::new();
        assert_eq!(map.lookup(&peer), CallerIdentity::Anonymous);
    }

    #[test]
    fn stub_returns_registered_agent() {
        let peer = PeerId::random();
        let map = StubIdentityMap::new();
        map.register(peer, "agent-pubkey-123");
        assert_eq!(
            map.lookup(&peer),
            CallerIdentity::Agent("agent-pubkey-123".to_string())
        );
    }
}
