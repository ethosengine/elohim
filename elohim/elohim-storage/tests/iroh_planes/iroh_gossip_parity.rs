//! Phase 4 acceptance test — iroh-gossip on the parallel stack.
//!
//! Two-node parity: provider broadcasts a signed inventory delta
//! ([`elohim_storage::p2p::inventory_gossip::InventoryEnvelope`], built by
//! `inventory_broadcaster::build_delta_pages`) on the iroh-gossip topic that maps
//! from `INVENTORY_TOPIC`; fetcher subscribes to the same topic name and asserts
//! byte-identical receipt within a bounded time.
//!
//! This test exercises:
//! - `IrohGossip::topic_id_for` produces the same id on both peers
//! - `iroh_gossip::ALPN` is mounted on the Router by default
//! - subscribe / joined / broadcast round-trips on a single bootstrap
//!
//! Wire format and message schema are the same as on libp2p — gossipsub
//! and iroh-gossip differ only on transport and topic-id encoding.
//!
//! Gated on `p2p-iroh`.

use std::time::Duration;

use anyhow::Result;
use elohim_epr::address::compute_bytes_cid;
use elohim_epr::proof::AgentKeypair;
use elohim_storage::p2p::inventory_broadcaster::{build_delta_pages, SequenceAllocator};
use elohim_storage::p2p::inventory_gossip::{BlobAddress, InventoryEnvelope, INVENTORY_TOPIC};
use elohim_storage::p2p_iroh::{parity_harness::TwoNodeFixture, GossipEvent};
use tempfile::tempdir;
use tokio::time::timeout;

const TOPIC_NAME: &str = INVENTORY_TOPIC;

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

    // Provider broadcasts a signed inventory delta naming one byte CID.
    let keypair = AgentKeypair::from_secret(&[9u8; 32]).unwrap();
    let delta = build_delta_pages(
        &keypair,
        &provider_id.to_string(),
        &[BlobAddress::from(compute_bytes_cid(
            b"a blob the provider holds",
        ))],
        &[],
        &SequenceAllocator::new(0),
        1_700_000_000_000_000,
        &[],
    )
    .remove(0);
    let payload = delta.to_bytes();
    provider_sender.broadcast(payload.clone().into()).await?;

    // Fetcher receives the same MessagePack bytes within bounded time.
    let received: InventoryEnvelope = timeout(Duration::from_secs(30), async {
        loop {
            match fetcher_recv.next().await {
                Some(Ok(GossipEvent::Received(msg))) => {
                    assert_eq!(msg.content.as_ref(), payload.as_slice(), "byte-identical");
                    return InventoryEnvelope::from_bytes(&msg.content).map_err(anyhow::Error::msg);
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
