//! F-T21 integration tests for the federation aggregator.
//!
//! These tests use the `P2PHandle::for_testing()` stub to drive synthetic peer
//! responses. The stub always returns `Err(FederationError::TransportError)` for
//! `ViewFederate`, which the aggregator classifies as `FreshnessState::Offline`.
//!
//! Full multi-peer integration tests (peers A+B+C with peer B stopped mid-run)
//! are deferred to Jenkins per project memory `feedback_shift_measure_jenkins`.

use std::time::Duration;

use elohim_storage::{
    db::models::PeerIdentityBindingRow,
    services::federator::Federator,
    views::{FreshnessState, ViewKind},
    P2PHandle,
};

/// Build a minimal `PeerIdentityBindingRow` for test use.
fn make_binding(peer_id: &str, agent_cid: &str) -> PeerIdentityBindingRow {
    PeerIdentityBindingRow {
        peer_id: peer_id.to_string(),
        agent_cid: agent_cid.to_string(),
        dht_anchor_hash: "anchor".to_string(),
        valid_from: "2026-01-01T00:00:00Z".to_string(),
        valid_until: None,
        observed_at: "2026-05-02T00:00:00Z".to_string(),
        source: "dht".to_string(),
        device_archetype: "node".to_string(),
        superseded_by: None,
    }
}

#[tokio::test]
async fn federator_returns_unverifiable_for_unparseable_peer_id() {
    // The peer_id parse step fires before view_federate is called, so a
    // malformed binding row is Unverifiable, not Offline.
    let p2p = P2PHandle::for_testing();
    let federator = Federator::new(p2p);
    let bindings = vec![make_binding("not-a-real-peer-id", "agent_test")];

    let results = federator
        .query(
            ViewKind::Cluster,
            "agent_test",
            &bindings,
            Duration::from_secs(1),
        )
        .await;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].freshness.state, FreshnessState::Unverifiable);
    assert!(results[0].slice.is_none());
}

#[tokio::test]
async fn federator_returns_offline_when_p2p_returns_transport_error() {
    // for_testing() always returns Err(TransportError) for ViewFederate.
    // The aggregator must classify this as FreshnessState::Offline.
    let p2p = P2PHandle::for_testing();
    let federator = Federator::new(p2p);
    let valid_peer_id = libp2p::PeerId::random().to_string();
    let bindings = vec![make_binding(&valid_peer_id, "agent_test")];

    let results = federator
        .query(
            ViewKind::Cluster,
            "agent_test",
            &bindings,
            Duration::from_secs(1),
        )
        .await;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].freshness.state, FreshnessState::Offline);
    assert!(results[0].slice.is_none());
}

#[tokio::test]
async fn federator_handles_zero_bindings() {
    let p2p = P2PHandle::for_testing();
    let federator = Federator::new(p2p);

    let results = federator
        .query(ViewKind::Cluster, "agent_test", &[], Duration::from_secs(1))
        .await;

    assert!(results.is_empty());
}

#[tokio::test]
async fn federator_fans_out_to_multiple_bindings() {
    // All three peers will return TransportError from the stub → all Offline.
    let p2p = P2PHandle::for_testing();
    let federator = Federator::new(p2p);
    let bindings = vec![
        make_binding(&libp2p::PeerId::random().to_string(), "agent_test"),
        make_binding(&libp2p::PeerId::random().to_string(), "agent_test"),
        make_binding(&libp2p::PeerId::random().to_string(), "agent_test"),
    ];

    let results = federator
        .query(
            ViewKind::Cluster,
            "agent_test",
            &bindings,
            Duration::from_secs(1),
        )
        .await;

    assert_eq!(results.len(), 3);
    for r in &results {
        assert_eq!(r.freshness.state, FreshnessState::Offline);
        assert!(r.slice.is_none());
    }
}
