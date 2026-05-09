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
//! ## Caller identity
//!
//! In iroh mode, the caller's [`CallerIdentity`] currently defaults to
//! [`CallerIdentity::Anonymous`]. The libp2p path resolves caller via
//! a `PeerIdentityMap` keyed on `PeerId`; the iroh equivalent — looking
//! up the iroh `NodeId` against the cross-stack peer-map's
//! `peer_transport_manifest` projection — graduates with Phase 12.
//! Until then:
//!
//! - **Commons / Public atoms** are served correctly to iroh callers
//!   (the reach gate doesn't need an identity).
//! - **Higher reach tiers** (community / familiar / trusted / intimate /
//!   self / private) fall through to `NotFound` — leak-free, matches
//!   libp2p semantics for unauthenticated callers.
//!
//! When a future caller-resolution wiring lands, a new constructor
//! ([`EprAtomServiceBackend::with_caller_resolver`]) takes a closure
//! that maps iroh `NodeId` to `CallerIdentity` via the cross-stack
//! peer-map.

use std::sync::Arc;

use super::epr::EprAtomBackend;
use crate::epr_atom_service::EprAtomService;
use crate::p2p::epr_atom_protocol::{EprAtomRequest, EprAtomResponse};
use crate::p2p::identity_map::CallerIdentity;

/// Routes [`EprAtomRequest`] variants into a shared
/// [`EprAtomService`] and produces the matching [`EprAtomResponse`].
/// Caller identity defaults to Anonymous in iroh mode pending the
/// Phase 12 cross-stack peer-map graduation.
pub struct EprAtomServiceBackend {
    service: Arc<EprAtomService>,
}

impl EprAtomServiceBackend {
    pub fn new(service: Arc<EprAtomService>) -> Self {
        Self { service }
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
        // Iroh-mode peer label is intentionally a synthetic placeholder
        // until the cross-stack peer-map graduation gives us a stable
        // way to surface the iroh NodeId at the service-call boundary.
        // Logs from this path will tag the source as `iroh:peer`; the
        // wire-format response is unaffected.
        self.service
            .handle("iroh:peer", CallerIdentity::Anonymous, request)
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
        let res = backend
            .handle(EprAtomRequest::FetchBatch { cids })
            .await;
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
}
