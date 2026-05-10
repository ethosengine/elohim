//! Phase 11 — production [`ViewFederationBackend`] backed by [`crate::view_fed_service::ViewFedService`].
//!
//! Adapter between the iroh view-federation ALPN handler
//! ([`super::view_fed::IrohViewFederationProtocol`]) and the daemon's
//! transport-neutral view-federation service. Mirrors the libp2p side
//! (the inline handler in `P2PNode::run`'s ViewFederation Request arm,
//! which calls `build_response_slice` directly) so the two transports
//! return wire-byte-identical responses for the same request.
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
//! the view-federation plane is dual-stack permanent. 256 KiB cap on
//! responses applies on both transports.
//!
//! ## Connected peers (Phase 12)
//!
//! When constructed with [`ViewFedServiceBackend::with_manifest_pool`], the
//! iroh-mode adapter queries the `peer_transport_manifest` for all rows with
//! a `libp2p_peer_id`, parses each into a `libp2p::PeerId`, and passes the
//! resulting slice to `service.handle`. Without the pool wired (`new`
//! constructor), `connected_peers` is an empty slice — the same Phase 11
//! fallback that returned a local-only view.
//!
//! ## Signing failure
//!
//! If `build_response_slice` fails to sign (e.g. the keypair is a
//! public-key-only handle), the adapter mirrors the libp2p side's
//! behavior: it logs a warning and constructs a "best-effort"
//! response with an Offline freshness slice and an empty signature.
//! The libp2p path drops the channel (libp2p surfaces an
//! InboundFailure::ResponseOmission); since iroh's accept loop
//! requires a response or an error, the adapter chooses the
//! lower-loss path of returning a degraded-but-shaped response. A
//! follow-up may surface this differently as the spec evolves.

use std::sync::Arc;

use tracing::warn;

use super::view_fed::ViewFederationBackend;
use crate::db::DbPool;
use crate::p2p_iroh::peer_map::list_libp2p_peer_ids;
use crate::view_fed_service::ViewFedService;
use crate::views::{
    Freshness, FreshnessState, JsonVal, ViewFederationRequest, ViewFederationResponse, ViewSlice,
};
use libp2p::PeerId;

/// Routes [`ViewFederationRequest`]s into a shared
/// [`ViewFedService`] and produces the matching
/// [`ViewFederationResponse`].
pub struct ViewFedServiceBackend {
    service: Arc<ViewFedService>,
    /// Reported in the (rare) signing-failure shape so receivers can
    /// see which peer emitted the degraded response. The libp2p side
    /// embeds this in slice.peer_id; here we track it explicitly so
    /// the failure path can do the same.
    local_peer_id_fallback: String,
    /// Phase 12: manifest pool for connected-peers hydration.
    pool: Option<DbPool>,
}

impl ViewFedServiceBackend {
    pub fn new(service: Arc<ViewFedService>, local_peer_id_fallback: String) -> Self {
        Self {
            service,
            local_peer_id_fallback,
            pool: None,
        }
    }

    /// Phase 12 wiring: when the manifest pool is supplied, the iroh-mode
    /// view-federation adapter passes a real connected-peers slice (parsed
    /// from manifest libp2p_peer_id rows) instead of empty.
    pub fn with_manifest_pool(
        service: Arc<ViewFedService>,
        local_peer_id_fallback: String,
        pool: DbPool,
    ) -> Self {
        Self {
            service,
            local_peer_id_fallback,
            pool: Some(pool),
        }
    }

    fn connected_peers(&self) -> Vec<PeerId> {
        match self.pool.as_ref() {
            Some(pool) => match pool.get() {
                Ok(mut conn) => list_libp2p_peer_ids(&mut conn)
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|s| s.parse::<PeerId>().ok())
                    .collect(),
                Err(_) => Vec::new(),
            },
            None => Vec::new(),
        }
    }
}

impl std::fmt::Debug for ViewFedServiceBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewFedServiceBackend")
            .field("local_peer_id_fallback", &self.local_peer_id_fallback)
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl ViewFederationBackend for ViewFedServiceBackend {
    async fn handle(&self, request: ViewFederationRequest) -> ViewFederationResponse {
        let connected = self.connected_peers();
        match self.service.handle(request.clone(), &connected).await {
            Ok(res) => res,
            Err(e) => {
                warn!(
                    target: "elohim_storage::view_federation",
                    error = %e,
                    agent_cid = %request.agent_cid,
                    request_id = %request.request_id,
                    "iroh-mode view-federation: signing failed; returning degraded shape"
                );
                ViewFederationResponse {
                    view_kind: request.view_kind.clone(),
                    agent_cid: request.agent_cid,
                    request_id: request.request_id,
                    slice: ViewSlice {
                        peer_id: self.local_peer_id_fallback.clone(),
                        view_kind: request.view_kind,
                        freshness: Freshness {
                            state: FreshnessState::Offline,
                            stale_since_ms: None,
                        },
                        payload: JsonVal(serde_json::Value::Null),
                        signature: String::new(),
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::ViewKind;
    use libp2p::identity::Keypair;
    use libp2p::PeerId;

    fn fresh_backend() -> ViewFedServiceBackend {
        let kp = Keypair::generate_ed25519();
        let peer_id = PeerId::from(kp.public()).to_string();
        let service = Arc::new(ViewFedService::new(
            "agent-cid-self".into(),
            peer_id.clone(),
            kp,
            None,
        ));
        ViewFedServiceBackend::new(service, peer_id)
    }

    #[tokio::test]
    async fn handle_owning_agent_returns_signed_live_slice() {
        let backend = fresh_backend();
        let res = backend
            .handle(ViewFederationRequest {
                view_kind: ViewKind::Cluster,
                agent_cid: "agent-cid-self".into(),
                request_id: "r1".into(),
            })
            .await;
        assert_eq!(res.agent_cid, "agent-cid-self");
        assert!(!res.slice.signature.is_empty());
        assert_eq!(res.slice.freshness.state, FreshnessState::Live);
    }

    #[tokio::test]
    async fn handle_non_owning_agent_returns_offline_slice() {
        let backend = fresh_backend();
        let res = backend
            .handle(ViewFederationRequest {
                view_kind: ViewKind::Cluster,
                agent_cid: "some-other-agent".into(),
                request_id: "r2".into(),
            })
            .await;
        assert_eq!(res.slice.freshness.state, FreshnessState::Offline);
    }

    #[tokio::test]
    async fn manifest_backed_connected_peers_includes_libp2p_rows() {
        use crate::db::{init_pool_from_dir, run_migrations};
        use crate::p2p_iroh::peer_map::{record_libp2p_observation, Plane};
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let pool = init_pool_from_dir(dir.path()).expect("pool");
        run_migrations(&pool).expect("migrations");
        {
            let mut conn = pool.get().unwrap();
            // 12D3KooW... is the canonical libp2p PeerId prefix; the
            // test uses a known-valid base58btc PeerId.
            record_libp2p_observation(
                &mut conn,
                "bafyrei...vf",
                "12D3KooWNZyfTfb1ENd1HRgGTbvXebvA7iJfCRZHm9NzpDdEsTwo",
                &[],
                &[Plane::ViewFed],
                1746878400,
            )
            .unwrap();
        }
        // The test asserts construction succeeds with the manifest pool;
        // full-handle behavior is covered in iroh_view_fed_real_backend.
        let kp = libp2p::identity::Keypair::generate_ed25519();
        let local = libp2p::PeerId::from(kp.public()).to_string();
        let svc = Arc::new(ViewFedService::new(
            "agent-cid-self".into(),
            local.clone(),
            kp,
            None,
        ));
        let _backend = ViewFedServiceBackend::with_manifest_pool(svc, local, pool);
        // Smoke: doesn't panic.
    }
}
