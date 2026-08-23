#![cfg(all(feature = "p2p", feature = "p2p-iroh"))]

use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{Connection, SqliteConnection};
use elohim_storage::db::{run_migrations, DbPool};
use elohim_storage::p2p::dedup::DedupLru;
use elohim_storage::p2p::gossip_dispatch::{handle_gossip, GossipDispatchCtx, GossipSource};
use elohim_storage::p2p::inventory_gossip::{
    BlobAddress, BlobHint, BlobInventorySnapshot, INVENTORY_TOPIC,
};
use elohim_storage::p2p::P2PCommand;
use elohim_storage::p2p_iroh::IrohInventoryFetch;

fn test_pool(label: &str) -> DbPool {
    let url = format!(
        "file:iroh_inventory_fetch_{label}_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4().as_simple()
    );
    let pool = Pool::builder()
        .max_size(1)
        .build(ConnectionManager::<SqliteConnection>::new(&url))
        .expect("pool");
    run_migrations(&pool).expect("migrations");
    pool
}

fn dispatch(pool: &DbPool, fetch: &IrohInventoryFetch, source: GossipSource, payload: &[u8]) {
    let dedup = DedupLru::new();
    let ctx = GossipDispatchCtx {
        db_pool: Some(pool),
        dedup: &dedup,
        agent_info_inbound_tx: None,
        inventory_fetch: Some(fetch),
        transport_manifest_sink: None,
        self_iroh_node_id: None,
    };
    handle_gossip(&ctx, INVENTORY_TOPIC, payload, &source);
}

#[test]
fn iroh_and_libp2p_dispatch_queue_identical_inventory_work() {
    let hash = format!("sha256-{}", "a".repeat(64));
    let hint = BlobHint {
        address: BlobAddress::new(hash.clone()).expect("address"),
        size_bytes: Some(4096),
        recipient_hub_id: Some("hub:matthew".to_string()),
        epr_kind: Some("content".to_string()),
        tier: Some("dwelling".to_string()),
    };
    let snapshot = BlobInventorySnapshot {
        peer_id: "12D3KooWTransportParity".to_string(),
        hashes: vec![BlobAddress::new(hash.clone()).expect("address")],
        hints: vec![hint],
        snapshot_at: 1_700_000_000_000_000,
        sequence: 7,
        signature: vec![0x42],
    };
    let payload = snapshot.to_bytes().expect("snapshot bytes");

    let (libp2p_tx, mut libp2p_rx) = tokio::sync::mpsc::channel(4);
    let (iroh_tx, mut iroh_rx) = tokio::sync::mpsc::channel(4);
    let libp2p_fetch = IrohInventoryFetch::new(Some(libp2p_tx));
    let iroh_fetch = IrohInventoryFetch::new(Some(iroh_tx));

    dispatch(
        &test_pool("libp2p"),
        &libp2p_fetch,
        GossipSource::Libp2p {
            peer: "12D3KooWRelay".to_string(),
            message_id: "matrix-libp2p".to_string(),
        },
        &payload,
    );
    dispatch(
        &test_pool("iroh"),
        &iroh_fetch,
        GossipSource::Iroh {
            node: "iroh-node-matrix".to_string(),
        },
        &payload,
    );

    let libp2p_work = libp2p_rx.try_recv().expect("libp2p queued work");
    let iroh_work = iroh_rx.try_recv().expect("iroh queued work");
    match (libp2p_work, iroh_work) {
        (
            P2PCommand::InventoryAdvertisement {
                source_peer_id: left_peer,
                hints: left_hints,
                hashes: left_hashes,
            },
            P2PCommand::InventoryAdvertisement {
                source_peer_id: right_peer,
                hints: right_hints,
                hashes: right_hashes,
            },
        ) => {
            assert_eq!(left_peer, right_peer);
            assert_eq!(left_hints, right_hints);
            assert_eq!(left_hashes, right_hashes);
            assert_eq!(left_hashes, vec![hash]);
        }
        _ => panic!("both planes must queue InventoryAdvertisement work"),
    }
    assert_eq!(libp2p_fetch.stats().enqueued, 1);
    assert_eq!(iroh_fetch.stats().enqueued, 1);
}

#[test]
fn pure_iroh_absence_of_byte_fetcher_is_counted() {
    use elohim_storage::p2p::gossip_dispatch::InventoryFetch;

    let fetch = IrohInventoryFetch::new(None);
    let mut conn = SqliteConnection::establish(":memory:").expect("sqlite");
    fetch.score_and_enqueue(
        &mut conn,
        "12D3KooWNoLibp2pLeg",
        &[],
        &[format!("sha256-{}", "b".repeat(64))],
    );

    assert_eq!(fetch.stats().unavailable, 1);
    assert_eq!(fetch.stats().enqueued, 0);
}
