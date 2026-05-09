//! Phase 11 — production [`EprBackend`] backed by [`crate::epr_service::EprService`].
//!
//! Adapter between the iroh EPR ALPN handler ([`super::epr::IrohEprProtocol`])
//! and the daemon's transport-neutral EPR service. Mirrors the libp2p side's
//! dispatch (`P2PNode::handle_epr_request` in `src/p2p/mod.rs`) for the
//! read-side variants so the two transports return wire-byte-identical
//! responses for the same request.
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
//! the EPR plane is dual-stack permanent: identical wire frames, identical
//! authorization gates. The Announce variant is the asymmetric outlier —
//! libp2p side writes to Kademlia (`put_record`); iroh side has no
//! equivalent until pkarr / iroh-gossip identity-binding lands per the
//! n0 mitigation roadmap. Until then, this backend reports
//! `Announced { accepted: false, reason: ... }` so callers see a clean
//! "not yet wired" rather than a silent drop.
//!
//! Construction:
//! ```ignore
//! let epr_service = Arc::new(EprService::new(db_pool, policy, cache, trust_cache));
//! let backend: Arc<dyn EprBackend> = Arc::new(EprServiceBackend::new(epr_service));
//! let extras = vec![(EPR_ALPN.to_vec(),
//!                    Box::new(IrohEprProtocol::new(backend)) as Box<dyn DynProtocolHandler>)];
//! IrohNode::start_with_protocols(iroh_cfg, extras).await?;
//! ```

use std::sync::Arc;

use super::epr::EprBackend;
use crate::epr_service::EprService;
use crate::p2p::epr_protocol::{EprRequest, EprResponse};

/// Routes [`EprRequest`] read-side variants into a shared [`EprService`]
/// and produces the matching [`EprResponse`]. Announce is intentionally
/// stubbed pending the n0 mitigation roadmap — see module-level docs.
pub struct EprServiceBackend {
    service: Arc<EprService>,
}

impl EprServiceBackend {
    pub fn new(service: Arc<EprService>) -> Self {
        Self { service }
    }
}

impl std::fmt::Debug for EprServiceBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EprServiceBackend").finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl EprBackend for EprServiceBackend {
    async fn handle(&self, request: EprRequest) -> EprResponse {
        match request {
            EprRequest::Resolve { id, agent_pubkey } => {
                self.service.handle_resolve(id, agent_pubkey).await
            }
            EprRequest::ResolveBatch { ids } => self.service.handle_resolve_batch(ids),
            EprRequest::GetDocument { id } => self.service.handle_get_document(&id),
            EprRequest::QueryDelivery { blob_hash } => {
                self.service.handle_query_delivery(blob_hash).await
            }
            EprRequest::Announce { .. } => EprResponse::Announced {
                accepted: false,
                reason: Some(
                    "EPR Announce in iroh mode requires pkarr / iroh-gossip identity-binding \
                     — not yet wired (see complementarity spec n0 mitigation steps 1–4)"
                        .to_string(),
                ),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::trust_cache::PeerTrustCache;

    fn fresh_backend() -> EprServiceBackend {
        let service = Arc::new(EprService::new(None, None, None, PeerTrustCache::new()));
        EprServiceBackend::new(service)
    }

    #[tokio::test]
    async fn announce_returns_unimplemented_in_iroh_mode() {
        let backend = fresh_backend();
        let res = backend
            .handle(EprRequest::Announce {
                head: b"ignored".to_vec(),
            })
            .await;
        match res {
            EprResponse::Announced { accepted, reason } => {
                assert!(!accepted);
                assert!(
                    reason.unwrap_or_default().contains("not yet wired"),
                    "reason should mention n0 mitigation"
                );
            }
            other => panic!("expected Announced, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn resolve_with_no_db_pool_returns_not_found() {
        let backend = fresh_backend();
        let res = backend
            .handle(EprRequest::Resolve {
                id: "missing".into(),
                agent_pubkey: None,
            })
            .await;
        match res {
            EprResponse::NotFound => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_document_returns_unimplemented() {
        let backend = fresh_backend();
        let res = backend
            .handle(EprRequest::GetDocument {
                id: "doc-1".into(),
            })
            .await;
        match res {
            EprResponse::Error(msg) => assert!(msg.contains("not yet implemented")),
            other => panic!("expected Error, got {other:?}"),
        }
    }
}
