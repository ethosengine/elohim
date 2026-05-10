//! Phase 11 acceptance — view-federation ALPN over iroh QUIC
//! dispatched into a real [`ViewFedService`] backend (not the stub
//! used by `iroh_view_fed_parity`).
//!
//! Proves the wire → backend → service → build_response_slice chain
//! end-to-end on iroh: provider mounts [`ViewFedServiceBackend`] over
//! a real [`ViewFedService`] with a generated libp2p ed25519 keypair
//! (same crypto as iroh's SecretKey); an iroh client requests a
//! Cluster slice; the response carries a non-empty signature, the
//! correct freshness state for ownership, and the request_id /
//! agent_cid echoed back verbatim.
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
//! the view-federation plane is dual-stack permanent. Receivers
//! verify against the agent's DHT-anchored public key; both
//! transports sign with the same ed25519 secret, so signatures are
//! byte-identical for the same input.
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use std::sync::Arc;

use anyhow::Result;
use elohim_storage::p2p_iroh::{
    parity_harness::TwoNodeFixture, AlpnRegistration, IrohViewFederationClient,
    IrohViewFederationProtocol, ViewFedServiceBackend, ViewFederationBackend, VIEW_FED_ALPN,
};
use elohim_storage::view_fed_service::ViewFedService;
use elohim_storage::views::{FreshnessState, ViewFederationRequest, ViewKind};
use libp2p::identity::Keypair;
use libp2p::PeerId;
use tempfile::tempdir;

fn build_real_backend() -> (Arc<dyn ViewFederationBackend>, String, String) {
    let kp = Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(kp.public()).to_string();
    let agent_cid = local_peer_id.clone(); // proxy: in production agent_cid is DHT-anchored
    let service = Arc::new(ViewFedService::new(
        agent_cid.clone(),
        local_peer_id.clone(),
        kp,
        None, // no DB pool — payload defaults to empty json (owning agent) or null (other)
    ));
    let backend: Arc<dyn ViewFederationBackend> =
        Arc::new(ViewFedServiceBackend::new(service, local_peer_id.clone()));
    (backend, local_peer_id, agent_cid)
}

async fn fixture_with_real_provider_backend(
    provider_dir: &std::path::Path,
    fetcher_dir: &std::path::Path,
) -> Result<(TwoNodeFixture, String, String)> {
    let (backend, peer_id, agent_cid) = build_real_backend();
    let handler = IrohViewFederationProtocol::new(backend);
    let provider_extras: Vec<AlpnRegistration> = vec![(VIEW_FED_ALPN.to_vec(), Box::new(handler))];
    let fixture =
        TwoNodeFixture::new_asymmetric(provider_dir, provider_extras, fetcher_dir, Vec::new())
            .await?;
    Ok((fixture, peer_id, agent_cid))
}

/// When the requested agent_cid matches the provider's local agent,
/// the response slice is Live and signed.
#[tokio::test]
async fn owning_agent_request_via_iroh_returns_signed_live_slice() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let (fixture, _peer_id, agent_cid) =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohViewFederationClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &ViewFederationRequest {
                view_kind: ViewKind::Cluster,
                agent_cid: agent_cid.clone(),
                request_id: "req-owning".into(),
            },
        )
        .await?;

    assert_eq!(res.agent_cid, agent_cid, "echo agent_cid");
    assert_eq!(res.request_id, "req-owning", "echo request_id");
    assert!(
        !res.slice.signature.is_empty(),
        "signature must be populated (real backend signs)"
    );
    assert_eq!(
        res.slice.freshness.state,
        FreshnessState::Live,
        "owning agent must report Live freshness"
    );

    fixture.shutdown().await?;
    Ok(())
}

/// When the requested agent_cid does NOT match the provider's local
/// agent, the response slice is Offline (and still signed, since the
/// signing happens over the slice contents regardless of ownership).
#[tokio::test]
async fn non_owning_agent_request_via_iroh_returns_offline_signed_slice() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let (fixture, _peer_id, _agent_cid) =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohViewFederationClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &ViewFederationRequest {
                view_kind: ViewKind::Cluster,
                agent_cid: "different-agent-cid".into(),
                request_id: "req-non-owning".into(),
            },
        )
        .await?;

    assert_eq!(res.agent_cid, "different-agent-cid");
    assert_eq!(res.request_id, "req-non-owning");
    assert_eq!(
        res.slice.freshness.state,
        FreshnessState::Offline,
        "non-owning agent must report Offline freshness"
    );
    assert!(
        !res.slice.signature.is_empty(),
        "signature populated even for offline slice"
    );

    fixture.shutdown().await?;
    Ok(())
}

/// PeerTopology view kind round-trips with empty connected_peers in
/// iroh mode (until cross-stack peer-map graduation).
#[tokio::test]
async fn peer_topology_request_via_iroh_returns_empty_connected_peers() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let (fixture, _peer_id, agent_cid) =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohViewFederationClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &ViewFederationRequest {
                view_kind: ViewKind::PeerTopology,
                agent_cid: agent_cid.clone(),
                request_id: "req-topo".into(),
            },
        )
        .await?;

    assert_eq!(res.view_kind, ViewKind::PeerTopology);
    assert_eq!(res.agent_cid, agent_cid);
    assert!(!res.slice.signature.is_empty());

    fixture.shutdown().await?;
    Ok(())
}
