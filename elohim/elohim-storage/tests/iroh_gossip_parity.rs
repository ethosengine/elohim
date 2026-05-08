//! Phase 4 acceptance test — iroh-gossip on the parallel stack.
//!
//! Two-node parity: provider broadcasts a MessagePack-encoded
//! [`crate::p2p::inventory_gossip::BlobInventoryDelta`]-shaped payload on
//! the iroh-gossip topic that maps from `INVENTORY_TOPIC`; fetcher
//! subscribes to the same topic name and asserts byte-identical receipt
//! within a bounded time.
//!
//! This test exercises:
//! - `IrohGossip::topic_id_for` produces the same id on both peers
//! - `iroh_gossip::ALPN` is mounted on the Router by default
//! - subscribe / joined / broadcast round-trips on a single bootstrap
//!
//! Wire format and message schema is unchanged from libp2p — gossipsub
//! and iroh-gossip differ only on transport and topic-id encoding.
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use std::time::Duration;

use anyhow::Result;
use elohim_storage::p2p_iroh::{parity_harness::TwoNodeFixture, GossipEvent};
use serde::{Deserialize, Serialize};
use tempfile::tempdir;
use tokio::time::timeout;

const TOPIC_NAME: &str = "elohim/inventory/blob";

/// Local copy of the libp2p inventory delta wire shape — kept in this
/// test rather than importing because the production module is libp2p-
/// gated and the test runs in p2p-iroh-only mode for some Phase-4 CI
/// permutations. After cutover we'll consolidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BlobInventoryDeltaWire {
    peer_id: String,
    added: Vec<String>,
    removed: Vec<String>,
    emitted_at: i64,
    sequence: u64,
    signature: Vec<u8>,
}

#[tokio::test]
async fn inventory_delta_round_trips_via_iroh_gossip() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture = TwoNodeFixture::new(provider_dir.path(), fetcher_dir.path(), Vec::new).await?;

    // The fetcher's endpoint must know the provider's NodeAddr before it
    // can dial it for the gossip overlay — relays are disabled in this
    // fixture and discovery is not configured, so the bootstrap NodeId
    // alone is insufficient. Mirror what iroh-gossip's own smoke tests do.
    let provider_id = fixture.provider.node_id();
    fixture
        .fetcher
        .endpoint()
        .add_node_addr(fixture.provider_addr.clone())?;

    // Both peers subscribe to the same logical topic name.
    let (provider_sender, _provider_recv) = fixture
        .provider
        .gossip()
        .subscribe(TOPIC_NAME, vec![])
        .await?;
    let (_fetcher_sender, mut fetcher_recv) = fixture
        .fetcher
        .gossip()
        .subscribe(TOPIC_NAME, vec![provider_id])
        .await?;

    // Wait for the gossip overlay to converge — fetcher must see provider
    // as a NeighborUp before we broadcast, otherwise the message is
    // dropped on a topic with zero peers. iroh-gossip emits NeighborUp
    // when the swarm sample includes the new peer.
    use futures_util::StreamExt;
    timeout(Duration::from_secs(30), async {
        while let Some(evt) = fetcher_recv.next().await {
            if let Ok(GossipEvent::NeighborUp(_)) = evt {
                break;
            }
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("fetcher never saw provider as neighbor"))?;

    // Provider broadcasts a structurally-valid inventory delta payload.
    let delta = BlobInventoryDeltaWire {
        peer_id: format!("{provider_id}"),
        added: vec!["0".repeat(64)],
        removed: vec![],
        emitted_at: 1_700_000_000_000_000,
        sequence: 1,
        signature: vec![0u8; 1],
    };
    let payload = rmp_serde::to_vec_named(&delta)?;
    provider_sender.broadcast(payload.clone().into()).await?;

    // Fetcher receives the same MessagePack bytes within bounded time.
    let received: BlobInventoryDeltaWire = timeout(Duration::from_secs(30), async {
        loop {
            match fetcher_recv.next().await {
                Some(Ok(GossipEvent::Received(msg))) => {
                    return rmp_serde::from_slice(&msg.content).map_err(anyhow::Error::from);
                }
                Some(_) => continue,
                None => return Err(anyhow::anyhow!("gossip stream closed")),
            }
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("fetcher timed out waiting for inventory delta"))??;

    assert_eq!(received, delta, "received payload matches sent");

    fixture.shutdown().await?;
    Ok(())
}
