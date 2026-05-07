//! F-T19/F-T20: Unit tests for `P2PCommand::ViewFederate` dispatch, `P2PHandle::view_federate`,
//! and the `build_response_slice` responder helper.
//!
//! F-T19 tests exercise the command-dispatch and oneshot-resolution paths in
//! isolation — no real swarm, no two-peer harness. Two-peer integration coverage
//! is measured in Jenkins (`feedback_shift_measure_jenkins` memory).
//!
//! F-T20 tests exercise `build_response_slice` as a pure helper, verifying signing
//! correctness and envelope echoing without touching the swarm event loop.
//!
//! F-T19 cases (3):
//!  1. **channel_dispatch** — calling `view_federate` on the `for_testing()` stub
//!     reaches the `ViewFederate` arm and returns the stub sentinel error, proving
//!     the command is dispatched (not panicked or silently dropped).
//!  2. **timeout** — a receiver that never replies produces `FederationError::Timeout`.
//!  3. **swarm_gone** — dropping the receiver before the call produces
//!     `FederationError::SwarmGone`.
//!
//! F-T20 cases (+3):
//!  4. **responder_signs_live** — agent_cid matches local → Live freshness, non-Null payload,
//!     signature verifies.
//!  5. **responder_offline** — agent_cid differs → Offline freshness, Null payload, signature
//!     still verifies.
//!  6. **responder_echoes_envelope** — response envelope echoes view_kind, agent_cid,
//!     request_id supplied to the helper (needed by F-T21 dedup).

use std::time::Duration;

use elohim_storage::{
    p2p::{replication::ReplicationStatus, P2PCommand, P2PHandle, P2PStatusInfo},
    views::{ViewFederationRequest, ViewKind},
    FederationError,
};
use libp2p::PeerId;

/// Build a `P2PStatusInfo` suitable for unit tests (all fields at safe defaults).
fn test_status() -> P2PStatusInfo {
    P2PStatusInfo {
        peer_id: "test-peer".to_string(),
        listen_addresses: vec![],
        connected_peers: 0,
        bootstrap_nodes: vec![],
        sync_documents: 0,
        nat_status: "unknown".to_string(),
        relay_reservations: 0,
        announce_addresses: vec![],
        relay_mode: "client".to_string(),
        replication: ReplicationStatus::default(),
        drain: None,
        sync_paused: false,
        dedup_unique_len: 0,
        dedup_total_seen: 0,
    }
}

/// Construct a minimal `ViewFederationRequest` for test use.
fn test_request() -> ViewFederationRequest {
    ViewFederationRequest {
        view_kind: ViewKind::Cluster,
        agent_cid: "bafkreibmzonpj42xk5vxltpl2h3mj5qnxmvprsnwkl3uml7yzhbpqu7c4a".to_string(),
        request_id: "test-req-001".to_string(),
    }
}

/// Test 1 (channel-dispatch): calling `view_federate` on a `for_testing()` handle
/// reaches the `P2PCommand::ViewFederate` arm in the stub drainer and returns
/// `Err(FederationError::TransportError)` — the sentinel the stub sends back.
///
/// This proves the command is dispatched (not panicked on a missing arm or
/// silently dropped on the floor).
#[tokio::test]
async fn channel_dispatch_sends_view_federate_command() {
    let handle = P2PHandle::for_testing();
    let peer = PeerId::random();

    let result = handle
        .view_federate(peer, test_request(), Duration::from_secs(1))
        .await;

    assert!(
        matches!(result, Err(FederationError::TransportError)),
        "expected FederationError::TransportError from for_testing() stub; got: {:?}",
        result
    );
}

/// Test 2 (timeout): a handle whose receiver accepts the command but never replies
/// returns `FederationError::Timeout` after the supplied duration elapses.
#[tokio::test]
async fn view_federate_timeout_when_no_reply() {
    use tokio::sync::{mpsc, watch};

    let (command_tx, mut command_rx) = mpsc::channel::<P2PCommand>(4);

    // Spawn a task that accepts commands but never sends a reply.
    // We hold the `respond` sender alive for the full sleep so the channel is
    // not considered "swarm gone" — the peer just never answered.
    tokio::spawn(async move {
        while let Some(cmd) = command_rx.recv().await {
            if let P2PCommand::ViewFederate { respond, .. } = cmd {
                // Keep the sender alive for a long time without calling send.
                tokio::time::sleep(Duration::from_secs(30)).await;
                drop(respond);
            }
        }
    });

    let (status_tx, status_rx) = watch::channel(test_status());
    // keep the watch sender alive so the receiver stays valid for the test duration
    std::mem::forget(status_tx);

    let handle = P2PHandle::from_parts_for_testing(status_rx, command_tx, "test-agent".to_string());

    let result = handle
        .view_federate(PeerId::random(), test_request(), Duration::from_millis(50))
        .await;

    assert!(
        matches!(result, Err(FederationError::Timeout)),
        "expected FederationError::Timeout; got: {:?}",
        result
    );
}

/// Test 3 (swarm_gone): dropping the command receiver before calling `view_federate`
/// returns `FederationError::SwarmGone` because the mpsc send fails immediately.
#[tokio::test]
async fn view_federate_swarm_gone_when_channel_closed() {
    use tokio::sync::{mpsc, watch};

    let (command_tx, command_rx) = mpsc::channel::<P2PCommand>(4);
    // Drop the receiver immediately — the swarm task has "exited".
    drop(command_rx);

    let (status_tx, status_rx) = watch::channel(test_status());
    // keep the watch sender alive so the receiver stays valid for the test duration
    std::mem::forget(status_tx);

    let handle = P2PHandle::from_parts_for_testing(status_rx, command_tx, "test-agent".to_string());

    let result = handle
        .view_federate(PeerId::random(), test_request(), Duration::from_secs(5))
        .await;

    assert!(
        matches!(result, Err(FederationError::SwarmGone)),
        "expected FederationError::SwarmGone; got: {:?}",
        result
    );
}

// ── F-T20 tests: build_response_slice ──────────────────────────────────────

/// Test 4 (responder_signs_live): when `agent_cid` matches `local_agent_cid`,
/// `build_response_slice` returns:
///   - `FreshnessState::Live`
///   - a non-Null payload (`json!({})` when pool=None)
///   - a base64 signature that verifies against `slice.canonical_bytes_for_signing()`
#[tokio::test]
async fn responder_signs_slice_with_agent_key_when_agent_matches() {
    use base64::Engine as _;
    use elohim_storage::{
        p2p::view_federation::{build_response_slice, SliceContext},
        views::{FreshnessState, ViewKind},
    };

    let keypair = libp2p::identity::Keypair::generate_ed25519();
    let local_agent_cid = "local_agent_test";
    let local_peer_id = "12D3KooWTestPeer".to_string();

    let response = build_response_slice(
        ViewKind::Cluster,
        SliceContext {
            agent_cid: local_agent_cid.to_string(), // agent_cid == local_agent_cid
            request_id: "req-live-001".to_string(),
            local_agent_cid,
            local_peer_id,
            connected_peers: &[],
            keypair: &keypair,
            pool: None, // pool=None → stub json!({}) payload
        },
    )
    .await
    .expect("build_response_slice should not fail for a valid keypair");

    // Freshness is Live when agent matches.
    assert_eq!(
        response.slice.freshness.state,
        FreshnessState::Live,
        "expected FreshnessState::Live for matching agent"
    );

    // Payload is the stub empty object, not Null (pool=None returns json!({})).
    assert!(
        response.slice.payload.0 != serde_json::Value::Null,
        "expected non-Null payload for matching agent"
    );

    // Signature base64-decodes without error.
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(&response.slice.signature)
        .expect("signature should be valid base64");

    // The public key verifies the canonical bytes of the signed slice.
    let canonical = response.slice.canonical_bytes_for_signing();
    assert!(
        keypair.public().verify(&canonical, &sig_bytes),
        "signature must verify against the slice canonical bytes"
    );
}

/// Test 5 (responder_offline): when `agent_cid` differs from `local_agent_cid`,
/// `build_response_slice` returns:
///   - `FreshnessState::Offline`
///   - `serde_json::Value::Null` payload
///   - a signature that still verifies (the slice is signed regardless of payload)
#[tokio::test]
async fn responder_returns_offline_for_unknown_agent_cid() {
    use base64::Engine as _;
    use elohim_storage::{
        p2p::view_federation::{build_response_slice, SliceContext},
        views::{FreshnessState, ViewKind},
    };

    let keypair = libp2p::identity::Keypair::generate_ed25519();
    let local_agent_cid = "local_agent_test";
    let other_agent_cid = "some_other_agent_cid";
    let local_peer_id = "12D3KooWTestPeer".to_string();

    let response = build_response_slice(
        ViewKind::PeerTopology,
        SliceContext {
            agent_cid: other_agent_cid.to_string(), // agent_cid != local_agent_cid
            request_id: "req-offline-001".to_string(),
            local_agent_cid,
            local_peer_id,
            connected_peers: &[],
            keypair: &keypair,
            pool: None, // pool=None; non-matching agent always returns Null regardless
        },
    )
    .await
    .expect("build_response_slice should not fail for a valid keypair");

    // Freshness is Offline when agent does not match.
    assert_eq!(
        response.slice.freshness.state,
        FreshnessState::Offline,
        "expected FreshnessState::Offline for non-local agent"
    );

    // Payload is Null for an agent we don't host.
    assert_eq!(
        response.slice.payload.0,
        serde_json::Value::Null,
        "expected Null payload for non-local agent"
    );

    // Signature still verifies — signing is unconditional.
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(&response.slice.signature)
        .expect("signature should be valid base64");
    let canonical = response.slice.canonical_bytes_for_signing();
    assert!(
        keypair.public().verify(&canonical, &sig_bytes),
        "signature must verify even when payload is Null"
    );
}

/// Test 6 (responder_echoes_envelope): the response envelope echoes `view_kind`,
/// `agent_cid`, and `request_id` exactly as supplied — required by the F-T21 dedup map.
#[tokio::test]
async fn responder_echoes_view_kind_and_request_id() {
    use elohim_storage::{
        p2p::view_federation::{build_response_slice, SliceContext},
        views::ViewKind,
    };

    let keypair = libp2p::identity::Keypair::generate_ed25519();
    let agent_cid = "echo_agent_cid_test";
    let request_id = "echo-req-999";
    let local_peer_id = "12D3KooWTestPeer".to_string();

    let response = build_response_slice(
        ViewKind::PeerTopology,
        SliceContext {
            agent_cid: agent_cid.to_string(),
            request_id: request_id.to_string(),
            local_agent_cid: "different_local_agent",
            local_peer_id,
            connected_peers: &[],
            keypair: &keypair,
            pool: None, // pool=None
        },
    )
    .await
    .expect("build_response_slice should not fail");

    assert_eq!(
        response.view_kind,
        ViewKind::PeerTopology,
        "view_kind must be echoed"
    );
    assert_eq!(response.agent_cid, agent_cid, "agent_cid must be echoed");
    assert_eq!(response.request_id, request_id, "request_id must be echoed");
}
