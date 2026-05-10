//! Phase 11 — production [`IdentityHandshakeBackend`] and [`TrustBackend`]
//! backed by the transport-neutral services
//! [`crate::identity_handshake_service::IdentityHandshakeService`] and
//! [`crate::trust_service::TrustService`].
//!
//! Adapters between the iroh ALPN handlers ([`super::auth::IrohIdentityHandshakeProtocol`]
//! and [`super::auth::IrohTrustProtocol`]) and the daemon's
//! transport-neutral auth services. Mirror the libp2p inline handlers
//! in `P2PNode::run` so the two transports return wire-byte-identical
//! responses for the same request.
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
//! both planes are dual-stack permanent. Integrity is preserved by
//! Track 1 DHT-notarized agent identity (kitsune2/tx5) and the signed
//! wire frames each plane carries — NOT by any transport-level
//! security property. Both transports preserve DHT-derived integrity
//! equally.
//!
//! ## Trust cache hydration (Phase 12)
//!
//! [`TrustServiceBackend::with_trust_cache`] hydrates the libp2p-PeerId-keyed
//! trust cache from all manifest rows with a `libp2p_peer_id` at construction
//! time. This mirrors the libp2p side's per-handshake cache insertion at
//! `src/p2p/mod.rs`.
//!
//! ## Authoritative peer identifier (Phase 12)
//!
//! [`IdentityHandshakeServiceBackend::with_caller_resolver`] looks up the
//! connecting iroh NodeId in `peer_transport_manifest` and surfaces the
//! `agent_cid` as the authoritative peer label, replacing the Phase 11
//! fallback that used the claimed `binding.peer_id`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::auth::{IdentityHandshakeBackend, TrustBackend};
use crate::db::DbPool;
use crate::identity_handshake_service::IdentityHandshakeService;
use crate::p2p::identity_handshake::{IdentityHandshakeRequest, IdentityHandshakeResponse};
use crate::p2p::trust_cache::PeerTrustCache;
use crate::p2p::trust_protocol::{TrustHandshake, TrustResponse};
use crate::p2p_iroh::peer_map::{list_libp2p_to_agent, lookup_by_iroh_node_id};
use crate::trust_service::TrustService;
use crate::trust_verification::VerifiedTrustContext;
use libp2p::PeerId;

// ────────────────────────────────────────────────────────────────
// IdentityHandshakeServiceBackend
// ────────────────────────────────────────────────────────────────

/// Routes [`IdentityHandshakeRequest`]s into a shared
/// [`IdentityHandshakeService`] with caller-identity resolved from the
/// peer_transport_manifest when a resolver is wired.
pub struct IdentityHandshakeServiceBackend {
    service: Arc<IdentityHandshakeService>,
    local_peer_id_label: String,
    caller_resolver: Option<Arc<dyn Fn() -> Option<String> + Send + Sync>>,
    pool: Option<DbPool>,
}

impl IdentityHandshakeServiceBackend {
    pub fn new(service: Arc<IdentityHandshakeService>, local_peer_id_label: String) -> Self {
        Self {
            service,
            local_peer_id_label,
            caller_resolver: None,
            pool: None,
        }
    }

    /// Phase 12 wiring: pass a DbPool and a closure that returns the
    /// connecting peer's iroh NodeId. The backend looks up the NodeId
    /// in peer_transport_manifest and surfaces agent_cid as the
    /// authoritative peer label.
    pub fn with_caller_resolver(
        service: Arc<IdentityHandshakeService>,
        local_peer_id_label: String,
        pool: DbPool,
        resolver: Arc<dyn Fn() -> Option<String> + Send + Sync>,
    ) -> Self {
        Self {
            service,
            local_peer_id_label,
            caller_resolver: Some(resolver),
            pool: Some(pool),
        }
    }

    pub(crate) fn resolve_peer_label(&self, request: &IdentityHandshakeRequest) -> String {
        let Some(resolver) = self.caller_resolver.as_ref() else {
            return request.binding.peer_id.clone();
        };
        let Some(node_id) = resolver() else {
            return request.binding.peer_id.clone();
        };
        let Some(pool) = self.pool.as_ref() else {
            return node_id;
        };
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(_) => return node_id,
        };
        match lookup_by_iroh_node_id(&mut conn, &node_id) {
            Ok(Some(m)) => m.agent_cid,
            _ => node_id,
        }
    }
}

impl std::fmt::Debug for IdentityHandshakeServiceBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentityHandshakeServiceBackend")
            .field("local_peer_id_label", &self.local_peer_id_label)
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl IdentityHandshakeBackend for IdentityHandshakeServiceBackend {
    async fn handle(&self, request: IdentityHandshakeRequest) -> IdentityHandshakeResponse {
        let now_iso = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let peer_label = self.resolve_peer_label(&request);
        self.service.handle(request, &peer_label, &now_iso)
    }
}

// ────────────────────────────────────────────────────────────────
// TrustServiceBackend
// ────────────────────────────────────────────────────────────────

/// Routes [`TrustHandshake`] requests into a shared [`TrustService`]
/// and produces the matching [`TrustResponse`].
///
/// When constructed with [`Self::with_trust_cache`], hydrates the
/// libp2p-PeerId-keyed trust cache from the peer_transport_manifest
/// at construction time.
pub struct TrustServiceBackend {
    service: Arc<TrustService>,
    trust_cache: Option<PeerTrustCache>,
    pool: Option<DbPool>,
}

impl TrustServiceBackend {
    pub fn new(service: Arc<TrustService>) -> Self {
        Self {
            service,
            trust_cache: None,
            pool: None,
        }
    }

    /// Phase 12 wiring: hydrate the libp2p-keyed trust cache from
    /// the manifest at construction. Subsequent identity-handshake
    /// arrivals call `hydrate_one(node_id)` to refresh.
    pub fn with_trust_cache(
        service: Arc<TrustService>,
        trust_cache: PeerTrustCache,
        pool: DbPool,
    ) -> Self {
        let backend = Self {
            service,
            trust_cache: Some(trust_cache),
            pool: Some(pool),
        };
        backend.hydrate_all();
        backend
    }

    fn hydrate_all(&self) {
        let (Some(cache), Some(pool)) = (self.trust_cache.as_ref(), self.pool.as_ref()) else {
            return;
        };
        let Ok(mut conn) = pool.get() else { return; };
        let pairs = match list_libp2p_to_agent(&mut conn) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(target: "elohim_storage::trust",
                    error = %e, "iroh trust-cache hydrate: list failed");
                return;
            }
        };
        for (peer_str, agent_cid) in pairs {
            if let Ok(peer) = peer_str.parse::<PeerId>() {
                let ctx = VerifiedTrustContext {
                    agent_pubkey: agent_cid,
                    agent_verified: true,
                    reach_ceiling: "public".to_string(),
                    verified_memberships: vec![],
                    verified_relationships: vec![],
                    verified_attestations: vec![],
                    verified_stewardship: vec![],
                    verified_at: Instant::now(),
                    ttl: Duration::from_secs(3600),
                };
                // PeerTrustCache::insert is async; tokio::block_in_place
                // is the safest path inside a sync construction context.
                let cache = cache.clone();
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(cache.insert(peer, ctx));
                });
            }
        }
    }
}

impl std::fmt::Debug for TrustServiceBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrustServiceBackend")
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl TrustBackend for TrustServiceBackend {
    async fn handle(&self, request: TrustHandshake) -> TrustResponse {
        self.service.handle(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::identity_handshake::HandshakeBindingPayload;

    fn fresh_id_backend() -> IdentityHandshakeServiceBackend {
        let service = Arc::new(IdentityHandshakeService::new(None));
        IdentityHandshakeServiceBackend::new(service, "iroh-local-node".to_string())
    }

    fn sample_id_request(peer_id: &str, agent_cid: &str) -> IdentityHandshakeRequest {
        IdentityHandshakeRequest {
            timestamp: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            nonce: "AAAAAAAAAAAAAAAAAAAAAA==".to_string(),
            binding: HandshakeBindingPayload {
                peer_id: peer_id.to_string(),
                agent_cid: agent_cid.to_string(),
                signature: "AAA=".to_string(),
                valid_from: "2020-01-01T00:00:00Z".to_string(),
                valid_until: None,
                dht_anchor_hash: None,
                device_archetype: "node".to_string(),
            },
        }
    }

    #[tokio::test]
    async fn identity_handshake_with_self_consistent_payload_accepts() {
        let backend = fresh_id_backend();
        let res = backend
            .handle(sample_id_request("peer-self", "agent-self"))
            .await;
        // No DB pool → Accepted (Phase 11 stub semantics).
        assert!(matches!(res, IdentityHandshakeResponse::Accepted));
    }

    #[tokio::test]
    async fn identity_handshake_rejects_when_signature_empty() {
        let backend = fresh_id_backend();
        let mut req = sample_id_request("peer-self", "agent-self");
        req.binding.signature.clear();
        let res = backend.handle(req).await;
        match res {
            IdentityHandshakeResponse::Rejected { reason } => {
                assert!(reason.contains("signature is empty"));
            }
            other => panic!("expected Rejected, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn trust_handshake_returns_public_ceiling_one_hour_ttl() {
        let backend = TrustServiceBackend::new(Arc::new(TrustService::new()));
        let req = TrustHandshake {
            agent_pubkey: "agent-x".to_string(),
            membership_cids: vec![],
            relationship_cids: vec![],
            attestation_cids: vec![],
            stewardship_cids: vec![],
        };
        match backend.handle(req).await {
            TrustResponse::Verified {
                reach_ceiling,
                ttl_seconds,
            } => {
                assert_eq!(reach_ceiling, "public");
                assert_eq!(ttl_seconds, 3600);
            }
            other => panic!("expected Verified, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn trust_backend_hydrates_libp2p_rows_into_cache() {
        use crate::db::{init_pool_from_dir, run_migrations};
        use crate::p2p_iroh::peer_map::{record_libp2p_observation, Plane};
        use tempfile::tempdir;
        use libp2p::PeerId;

        let dir = tempdir().unwrap();
        let pool = init_pool_from_dir(dir.path()).expect("pool");
        run_migrations(&pool).expect("migrations");
        let known_peer = "12D3KooWNZyfTfb1ENd1HRgGTbvXebvA7iJfCRZHm9NzpDdEsTwo";
        {
            let mut conn = pool.get().unwrap();
            record_libp2p_observation(
                &mut conn,
                "bafyrei...trust",
                known_peer,
                &[],
                &[Plane::Trust],
                1746878400,
            ).unwrap();
        }
        let cache = PeerTrustCache::new();
        let _backend = TrustServiceBackend::with_trust_cache(
            Arc::new(TrustService::new()),
            cache.clone(),
            pool,
        );
        // The cache should now contain an entry keyed by the parsed PeerId.
        let peer = known_peer.parse::<PeerId>().unwrap();
        assert!(cache.try_get(&peer).is_some());
    }

    #[tokio::test]
    async fn identity_handshake_uses_manifest_agent_cid_when_resolver_yields_known_node() {
        use crate::db::{init_pool_from_dir, run_migrations};
        use crate::p2p_iroh::peer_map::{record_iroh_observation, Plane};
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let pool = init_pool_from_dir(dir.path()).expect("pool");
        run_migrations(&pool).expect("migrations");
        {
            let mut conn = pool.get().unwrap();
            record_iroh_observation(
                &mut conn,
                "bafyrei...idh",
                "node-idh",
                &[],
                &[Plane::IdentityHandshake],
                1746878400,
            ).unwrap();
        }
        let backend = IdentityHandshakeServiceBackend::with_caller_resolver(
            Arc::new(IdentityHandshakeService::new(None)),
            "iroh-local-node".to_string(),
            pool,
            Arc::new(|| Some("node-idh".to_string())),
        );
        let req = sample_id_request("12D3KooWClaim", "agent-cid-claimed");
        let label = backend.resolve_peer_label(&req);
        assert_eq!(label, "bafyrei...idh");
    }
}
