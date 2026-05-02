//! F-T19: Unit tests for `P2PCommand::ViewFederate` dispatch and `P2PHandle::view_federate`.
//!
//! These tests exercise the command-dispatch and oneshot-resolution paths in
//! isolation — no real swarm, no two-peer harness. Two-peer integration coverage
//! is deferred to F-T20 + Jenkins (`feedback_shift_measure_jenkins` memory).
//!
//! Three cases:
//!  1. **channel_dispatch** — calling `view_federate` on the `for_testing()` stub
//!     reaches the `ViewFederate` arm and returns the stub sentinel error, proving
//!     the command is dispatched (not panicked or silently dropped).
//!  2. **timeout** — a receiver that never replies produces `FederationError::Timeout`.
//!  3. **swarm_gone** — dropping the receiver before the call produces
//!     `FederationError::SwarmGone`.

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
