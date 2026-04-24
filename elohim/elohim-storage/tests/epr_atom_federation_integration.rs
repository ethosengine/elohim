//! Cross-peer integration tests for /elohim/epr-atom/1.0.0.
//!
//! Phase 2C Batch D: cross-peer behavioural proofs over the harness from
//! commit 88cc4fdf.

mod harness;
use elohim_storage::p2p::verify_incoming_epr;
use harness::spawn_test_node;
use std::time::Duration;
use tokio::time::timeout;

const DIAL_SETTLE: Duration = Duration::from_millis(100);
const CONNECT_WAIT: Duration = Duration::from_secs(10);
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::test]
async fn harness_two_nodes_connect() {
    let node_a = spawn_test_node("a").await;
    let node_b = spawn_test_node("b").await;
    node_a.dial(node_b.addr()).await.expect("dial");
    node_a
        .wait_for_connection(&node_b.peer_id(), Duration::from_secs(5))
        .await;
}

// ---------------------------------------------------------------------------
// Task 15 — Round-trip integrity (P0)
//
// A authors a signed Commons atom; B fetches by CID via /elohim/epr-atom/1.0.0
// and re-verifies the envelope. Proves wire bytes are byte-preserving and the
// signature re-verifies on the far side.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn signed_atom_round_trips_and_verifies() {
    let node_a = spawn_test_node("a").await;
    let node_b = spawn_test_node("b").await;

    node_a.dial(node_b.addr()).await.expect("dial");
    tokio::time::sleep(DIAL_SETTLE).await;
    node_a
        .wait_for_connection(&node_b.peer_id(), CONNECT_WAIT)
        .await;

    let epr = node_a.author_test_atom("commons", b"hello from A").await;
    let cid = epr.envelope.cid.to_string();
    node_a.ingest_local(epr.clone()).await;

    let response = timeout(
        FETCH_TIMEOUT,
        node_b.fetch_atom_from(&node_a.peer_id(), &cid),
    )
    .await
    .expect("fetch timed out")
    .unwrap_or_else(|e| panic!("fetch error: {e}"));

    let wire = response.expect("atom was None — B did not receive the atom");

    let (verified_epr, verified_cid) =
        verify_incoming_epr(&wire).expect("B should reverify the envelope");

    assert_eq!(
        verified_cid.to_string(),
        cid,
        "CID drifted across the wire",
    );
    assert_eq!(
        verified_epr.payload, epr.payload,
        "payload changed on the wire",
    );
    assert_eq!(
        verified_epr.envelope.proof.signer, epr.envelope.proof.signer,
        "signer changed on the wire",
    );
}
