//! Phase 9 acceptance — identity-handshake + trust ALPNs over iroh QUIC.
//!
//! Two parity tests:
//! - identity-handshake: Request → Accepted/Rejected (MessagePack)
//! - trust: TrustHandshake → Verified (MessagePack)
//!
//! Wire shapes reused unchanged from
//! [`crate::p2p::identity_handshake`] / [`crate::p2p::trust_protocol`].

#![cfg(feature = "p2p-iroh")]

use std::sync::Arc;

use anyhow::Result;
use elohim_storage::p2p::identity_handshake::{
    HandshakeBindingPayload, IdentityHandshakeRequest, IdentityHandshakeResponse,
};
use elohim_storage::p2p::trust_protocol::{TrustHandshake, TrustResponse};
use elohim_storage::p2p_iroh::{
    parity_harness::TwoNodeFixture, AlpnRegistration, IdentityHandshakeBackend,
    IrohIdentityHandshakeClient, IrohIdentityHandshakeProtocol, IrohTrustClient,
    IrohTrustProtocol, TrustBackend, IDENTITY_HANDSHAKE_ALPN, TRUST_ALPN,
};
use tempfile::tempdir;

// ============================================================================
// Identity handshake parity
// ============================================================================

struct AlwaysAcceptBackend;

#[async_trait::async_trait]
impl IdentityHandshakeBackend for AlwaysAcceptBackend {
    async fn handle(&self, _req: IdentityHandshakeRequest) -> IdentityHandshakeResponse {
        IdentityHandshakeResponse::Accepted
    }
}

fn identity_handshake_protocols() -> Vec<AlpnRegistration> {
    let backend: Arc<dyn IdentityHandshakeBackend> = Arc::new(AlwaysAcceptBackend);
    vec![(
        IDENTITY_HANDSHAKE_ALPN.to_vec(),
        Box::new(IrohIdentityHandshakeProtocol::new(backend)),
    )]
}

#[tokio::test]
async fn identity_handshake_round_trips_over_iroh_quic() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture = TwoNodeFixture::new(
        provider_dir.path(),
        fetcher_dir.path(),
        identity_handshake_protocols,
    )
    .await?;

    let req = IdentityHandshakeRequest {
        binding: HandshakeBindingPayload {
            peer_id: "12D3Koo...test".into(),
            agent_cid: "bafyrei...test".into(),
            valid_from: "2026-05-08T00:00:00Z".into(),
            valid_until: None,
            device_archetype: "node".into(),
            signature: "AA==".into(),
            dht_anchor_hash: None,
        },
        timestamp: "2026-05-08T04:50:00Z".into(),
        nonce: "AAAAAAAAAAAAAAAAAAAAAA==".into(),
    };

    let client = IrohIdentityHandshakeClient::new(fixture.fetcher.endpoint());
    let res = client.request(fixture.provider_addr.clone(), &req).await?;

    match res {
        IdentityHandshakeResponse::Accepted => {}
        other => panic!("unexpected response: {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

// ============================================================================
// Trust handshake parity
// ============================================================================

struct FixedTrustBackend;

#[async_trait::async_trait]
impl TrustBackend for FixedTrustBackend {
    async fn handle(&self, _req: TrustHandshake) -> TrustResponse {
        TrustResponse::Verified {
            reach_ceiling: "Community".into(),
            ttl_seconds: 3600,
        }
    }
}

fn trust_protocols() -> Vec<AlpnRegistration> {
    let backend: Arc<dyn TrustBackend> = Arc::new(FixedTrustBackend);
    vec![(
        TRUST_ALPN.to_vec(),
        Box::new(IrohTrustProtocol::new(backend)),
    )]
}

#[tokio::test]
async fn trust_handshake_round_trips_over_iroh_quic() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        TwoNodeFixture::new(provider_dir.path(), fetcher_dir.path(), trust_protocols).await?;

    let req = TrustHandshake {
        agent_pubkey: "ed25519-pubkey-base64".into(),
        membership_cids: vec!["cid-1".into()],
        relationship_cids: vec![],
        attestation_cids: vec![],
        stewardship_cids: vec![],
    };

    let client = IrohTrustClient::new(fixture.fetcher.endpoint());
    let res = client.request(fixture.provider_addr.clone(), &req).await?;

    match res {
        TrustResponse::Verified {
            reach_ceiling,
            ttl_seconds,
        } => {
            assert_eq!(reach_ceiling, "Community");
            assert_eq!(ttl_seconds, 3600);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}
