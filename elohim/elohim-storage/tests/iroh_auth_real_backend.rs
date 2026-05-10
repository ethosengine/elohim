//! Phase 11 acceptance — identity-handshake + trust ALPNs over iroh
//! QUIC dispatched into real services (not the stubs used by
//! `iroh_auth_parity`).
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
//! both planes are dual-stack permanent. Integrity is preserved by
//! DHT-anchored signed wire frames (Track 1), not transport-level
//! security; both transports preserve DHT-derived integrity equally.
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use std::sync::Arc;

use anyhow::Result;
use elohim_storage::identity_handshake_service::IdentityHandshakeService;
use elohim_storage::p2p::identity_handshake::{
    HandshakeBindingPayload, IdentityHandshakeRequest, IdentityHandshakeResponse,
};
use elohim_storage::p2p::trust_protocol::{TrustHandshake, TrustResponse};
use elohim_storage::p2p_iroh::{
    parity_harness::TwoNodeFixture, AlpnRegistration, IdentityHandshakeBackend,
    IdentityHandshakeServiceBackend, IrohIdentityHandshakeClient, IrohIdentityHandshakeProtocol,
    IrohTrustClient, IrohTrustProtocol, TrustBackend, TrustServiceBackend, IDENTITY_HANDSHAKE_ALPN,
    TRUST_ALPN,
};
use elohim_storage::trust_service::TrustService;
use tempfile::tempdir;

fn build_id_backend() -> Arc<dyn IdentityHandshakeBackend> {
    let service = Arc::new(IdentityHandshakeService::new(None));
    Arc::new(IdentityHandshakeServiceBackend::new(
        service,
        "iroh-provider-node".to_string(),
    ))
}

fn build_trust_backend() -> Arc<dyn TrustBackend> {
    Arc::new(TrustServiceBackend::new(Arc::new(TrustService::new())))
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

async fn fixture_with_real_provider_backends(
    provider_dir: &std::path::Path,
    fetcher_dir: &std::path::Path,
) -> Result<TwoNodeFixture> {
    let id_handler = IrohIdentityHandshakeProtocol::new(build_id_backend());
    let trust_handler = IrohTrustProtocol::new(build_trust_backend());
    let provider_extras: Vec<AlpnRegistration> = vec![
        (IDENTITY_HANDSHAKE_ALPN.to_vec(), Box::new(id_handler)),
        (TRUST_ALPN.to_vec(), Box::new(trust_handler)),
    ];
    TwoNodeFixture::new_asymmetric(provider_dir, provider_extras, fetcher_dir, Vec::new()).await
}

/// Identity-handshake against a no-DB-pool service: when the binding
/// is self-consistent (peer_id matches the binding's claimed peer_id),
/// the response is Accepted.
#[tokio::test]
async fn identity_handshake_via_iroh_against_real_backend_accepts() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        fixture_with_real_provider_backends(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohIdentityHandshakeClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &sample_id_request("peer-claim", "agent-claim"),
        )
        .await?;

    assert!(
        matches!(res, IdentityHandshakeResponse::Accepted),
        "Accepted expected for self-consistent payload, got {res:?}"
    );

    fixture.shutdown().await?;
    Ok(())
}

/// Identity-handshake with empty signature must be Rejected by the
/// real verify path — proves the service ran the verify_handshake_request
/// rules through the iroh wire.
#[tokio::test]
async fn identity_handshake_via_iroh_rejects_empty_signature() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        fixture_with_real_provider_backends(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohIdentityHandshakeClient::new(fixture.fetcher.endpoint());
    let mut req = sample_id_request("peer-claim", "agent-claim");
    req.binding.signature.clear();
    let res = client.request(fixture.provider_addr.clone(), &req).await?;

    match res {
        IdentityHandshakeResponse::Rejected { reason } => {
            assert!(
                reason.contains("signature is empty"),
                "expected reason about empty signature; got: {reason}"
            );
        }
        other => panic!("expected Rejected, got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

/// Identity-handshake with stale timestamp must be Rejected by the
/// real anti-replay window check.
#[tokio::test]
async fn identity_handshake_via_iroh_rejects_stale_timestamp() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        fixture_with_real_provider_backends(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohIdentityHandshakeClient::new(fixture.fetcher.endpoint());
    let mut req = sample_id_request("peer-claim", "agent-claim");
    req.timestamp = "2010-01-01T00:00:00Z".into(); // way outside ±5 min
    let res = client.request(fixture.provider_addr.clone(), &req).await?;

    match res {
        IdentityHandshakeResponse::Rejected { reason } => {
            assert!(
                reason.contains("replay window"),
                "expected reason about replay window; got: {reason}"
            );
        }
        other => panic!("expected Rejected, got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

/// Trust handshake: the real TrustService returns Verified with
/// `reach_ceiling: "public"` and `ttl_seconds: 3600` — the Phase 11
/// stub semantics, identical to the libp2p inline handler.
#[tokio::test]
async fn trust_handshake_via_iroh_returns_public_one_hour_verified() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        fixture_with_real_provider_backends(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohTrustClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &TrustHandshake {
                agent_pubkey: "agent-trust".to_string(),
                membership_cids: vec![],
                relationship_cids: vec![],
                attestation_cids: vec![],
                stewardship_cids: vec![],
            },
        )
        .await?;

    match res {
        TrustResponse::Verified {
            reach_ceiling,
            ttl_seconds,
        } => {
            assert_eq!(reach_ceiling, "public");
            assert_eq!(ttl_seconds, 3600);
        }
        other => panic!("expected Verified, got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}
