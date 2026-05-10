//! Plan 4 Task 5, Step 5.6 — EPR Announce accepted: true.
//!
//! Verifies that `EprServiceBackend::handle(EprRequest::Announce { head })` now
//! returns `Announced { accepted: true, reason: None }` after the n0-mitigation
//! gap was closed by Plan 4 (identity-binding gossip dual-published).
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use std::sync::Arc;

use elohim_storage::p2p::epr_protocol::{EprRequest, EprResponse};
use elohim_storage::p2p::trust_cache::PeerTrustCache;
use elohim_storage::p2p_iroh::epr_backend::EprServiceBackend;
use elohim_storage::p2p_iroh::EprBackend;
use elohim_storage::epr_service::EprService;

fn fresh_backend() -> EprServiceBackend {
    let service = Arc::new(EprService::new(None, None, None, PeerTrustCache::new()));
    EprServiceBackend::new(service)
}

/// Plan 4 cutover gate #4: EPR Announce must now be accepted: true.
///
/// This test closes the gap that was documented in the original
/// `announce_returns_unimplemented_in_iroh_mode` test. The gate is:
/// identity-binding gossip publishes via DualGossipPublisher → iroh peers
/// can resolve EPR head→peer mapping → Announce is accepted.
#[tokio::test]
async fn announce_accepted_after_dual_publish_identity_binding() {
    let backend = fresh_backend();
    let res = backend
        .handle(EprRequest::Announce {
            head: b"test-epr-head-bytes".to_vec(),
        })
        .await;

    match res {
        EprResponse::Announced { accepted, reason } => {
            assert!(
                accepted,
                "EPR Announce must be accepted: true after Plan 4 dual-publish lands"
            );
            assert!(
                reason.is_none(),
                "Accepted announce must have no error reason, got: {reason:?}"
            );
        }
        other => panic!("expected EprResponse::Announced, got {other:?}"),
    }
}

/// Verify that the read-side variants (Resolve, etc.) still work correctly
/// alongside the Announce change.
#[tokio::test]
async fn resolve_still_returns_not_found_without_db() {
    let backend = fresh_backend();
    let res = backend
        .handle(EprRequest::Resolve {
            id: "missing-epr-id".into(),
            agent_pubkey: None,
        })
        .await;
    match res {
        EprResponse::NotFound => {}
        other => panic!("expected NotFound (no DB pool), got {other:?}"),
    }
}
