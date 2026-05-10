//! Phase 11 — production [`EprAtomBackend`] backed by [`crate::epr_atom_service::EprAtomService`].
//!
//! Adapter between the iroh EPR-atom ALPN handler ([`super::epr::IrohEprAtomProtocol`])
//! and the daemon's transport-neutral EPR-atom service. Mirrors the
//! libp2p side's dispatch (`P2PNode::handle_epr_atom_request` in
//! `src/p2p/mod.rs`) so the two transports return wire-byte-identical
//! responses for the same request.
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
//! the EPR-atom plane is dual-stack permanent.
//!
//! ## Caller identity (Phase 12)
//!
//! In iroh mode, caller identity is resolved by looking up the connecting
//! iroh `NodeId` in the `peer_transport_manifest` projection. The manifest
//! lookup is plumbed via [`EprAtomServiceBackend::with_caller_resolver`],
//! which takes a `DbPool` and a closure that returns the connecting peer's
//! iroh NodeId. Falls back to [`CallerIdentity::Anonymous`] when:
//! - no resolver is wired (`new` constructor path), or
//! - the resolver yields `None` (NodeId not yet known), or
//! - the manifest has no row for the NodeId.
//!
//! This matches the libp2p path's behavior for unauthenticated callers.
//!
//! ## Reach semantics
//!
//! - **Commons / Public atoms** are served correctly to iroh callers
//!   (the reach gate doesn't need an identity).
//! - **Higher reach tiers** (community / familiar / trusted / intimate /
//!   self / private) fall through to `NotFound` for Anonymous callers —
//!   leak-free, matches libp2p semantics for unauthenticated callers.

use std::sync::Arc;

use super::epr::EprAtomBackend;
use crate::db::DbPool;
use crate::epr_atom_service::EprAtomService;
use crate::p2p::epr_atom_protocol::{EprAtomRequest, EprAtomResponse};
use crate::p2p::identity_map::CallerIdentity;
use crate::p2p_iroh::peer_map::{lookup_by_iroh_node_id, PeerTransportManifest};

/// Routes [`EprAtomRequest`] variants into a shared
/// [`EprAtomService`] and produces the matching [`EprAtomResponse`].
/// Caller identity is resolved from the `peer_transport_manifest`
/// when a resolver and pool are wired via [`Self::with_caller_resolver`].
pub struct EprAtomServiceBackend {
    service: Arc<EprAtomService>,
    caller_resolver: Option<Arc<dyn Fn() -> Option<String> + Send + Sync>>,
    pool: Option<DbPool>,
}

impl EprAtomServiceBackend {
    pub fn new(service: Arc<EprAtomService>) -> Self {
        Self {
            service,
            caller_resolver: None,
            pool: None,
        }
    }

    /// Phase 12 wiring: pass a DbPool and a closure that returns
    /// the connecting peer's iroh NodeId (or None if the harness
    /// can't expose it). The backend resolves the NodeId to a
    /// PeerTransportManifest and uses agent_cid as caller identity.
    pub fn with_caller_resolver(
        service: Arc<EprAtomService>,
        pool: DbPool,
        resolver: Arc<dyn Fn() -> Option<String> + Send + Sync>,
    ) -> Self {
        Self {
            service,
            caller_resolver: Some(resolver),
            pool: Some(pool),
        }
    }

    pub(crate) fn resolve_caller(&self) -> (String, CallerIdentity) {
        let Some(resolver) = self.caller_resolver.as_ref() else {
            return ("iroh:peer".to_string(), CallerIdentity::Anonymous);
        };
        let Some(node_id) = resolver() else {
            return ("iroh:peer".to_string(), CallerIdentity::Anonymous);
        };
        let Some(pool) = self.pool.as_ref() else {
            return (format!("iroh:{node_id}"), CallerIdentity::Anonymous);
        };
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(target: "elohim_storage::epr_atom",
                    error = %e, "iroh caller resolve: pool unavailable");
                return (format!("iroh:{node_id}"), CallerIdentity::Anonymous);
            }
        };
        match lookup_by_iroh_node_id(&mut conn, &node_id) {
            Ok(Some(PeerTransportManifest { agent_cid, .. })) => (
                format!("iroh:{node_id}"),
                CallerIdentity::Agent(agent_cid),
            ),
            Ok(None) => (format!("iroh:{node_id}"), CallerIdentity::Anonymous),
            Err(e) => {
                tracing::warn!(target: "elohim_storage::epr_atom",
                    error = %e, node_id = %node_id, "iroh caller resolve: lookup failed");
                (format!("iroh:{node_id}"), CallerIdentity::Anonymous)
            }
        }
    }
}

impl std::fmt::Debug for EprAtomServiceBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EprAtomServiceBackend")
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl EprAtomBackend for EprAtomServiceBackend {
    async fn handle(&self, request: EprAtomRequest) -> EprAtomResponse {
        let (peer_label, identity) = self.resolve_caller();
        self.service.handle(&peer_label, identity, request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::dedup::DedupLru;
    use crate::p2p::epr_atom_protocol::MAX_BATCH_CIDS;

    fn fresh_backend() -> EprAtomServiceBackend {
        let service = Arc::new(EprAtomService::new(None, Arc::new(DedupLru::new())));
        EprAtomServiceBackend::new(service)
    }

    #[tokio::test]
    async fn fetch_with_no_db_pool_surfaces_storage_unavailable() {
        let backend = fresh_backend();
        let res = backend
            .handle(EprAtomRequest::Fetch {
                cid: "bafy-x".into(),
            })
            .await;
        match res {
            EprAtomResponse::Error { message } => {
                assert!(message.contains("storage unavailable"));
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn announce_decode_failure_surfaces_cbor_reason() {
        let backend = fresh_backend();
        let res = backend
            .handle(EprAtomRequest::Announce {
                envelope_bytes: b"not-cbor".to_vec(),
            })
            .await;
        match res {
            EprAtomResponse::Announced { accepted, reason } => {
                assert!(!accepted);
                assert!(reason.unwrap_or_default().contains("cbor decode"));
            }
            other => panic!("expected Announced, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn fetch_batch_oversized_is_rejected() {
        let backend = fresh_backend();
        let cids: Vec<String> = (0..MAX_BATCH_CIDS + 1)
            .map(|i| format!("bafy-{i}"))
            .collect();
        let res = backend.handle(EprAtomRequest::FetchBatch { cids }).await;
        match res {
            EprAtomResponse::Error { message } => {
                assert!(message.contains("batch too large"));
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn integrity_notify_unhandled_kind_acks() {
        let backend = fresh_backend();
        let res = backend
            .handle(EprAtomRequest::IntegrityNotify {
                kind: "MysteryKind".into(),
                payload_bytes: b"x".to_vec(),
            })
            .await;
        match res {
            EprAtomResponse::IntegrityAck { received, reason } => {
                assert!(!received);
                assert!(reason.unwrap_or_default().contains("MysteryKind"));
            }
            other => panic!("expected IntegrityAck, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn resolver_yielding_known_node_id_resolves_to_agent_identity() {
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
                "bafyrei...known",
                "node-known",
                &[],
                &[Plane::EprAtom],
                1746878400,
            )
            .unwrap();
        }

        let service = Arc::new(EprAtomService::new(None, Arc::new(DedupLru::new())));
        let backend = EprAtomServiceBackend::with_caller_resolver(
            service,
            pool,
            Arc::new(|| Some("node-known".to_string())),
        );
        let (label, identity) = backend.resolve_caller();
        assert_eq!(label, "iroh:node-known");
        match identity {
            CallerIdentity::Agent(cid) => assert_eq!(cid, "bafyrei...known"),
            other => panic!("expected Agent, got {other:?}"),
        }
    }
}
