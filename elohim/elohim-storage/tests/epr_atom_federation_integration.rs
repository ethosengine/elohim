//! Cross-peer integration tests for /elohim/epr-atom/1.0.0.
//!
//! Tasks 17–19 add the substantive round-trip tests here.
//! Task 16 provides only the smoke test confirming the harness spins up and
//! two nodes can establish a TCP connection.

mod harness;
use harness::spawn_test_node;
use std::time::Duration;

#[tokio::test]
async fn harness_two_nodes_connect() {
    let node_a = spawn_test_node("a").await;
    let node_b = spawn_test_node("b").await;
    node_a.dial(node_b.addr()).await.expect("dial");
    node_a
        .wait_for_connection(&node_b.peer_id(), Duration::from_secs(5))
        .await;
}
