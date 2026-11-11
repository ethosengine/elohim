use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{Connection, SqliteConnection};
use elohim_epr::address::compute_bytes_cid;
use elohim_epr::proof::AgentKeypair;
use elohim_storage::db::{run_migrations, DbPool};
use elohim_storage::p2p::dedup::DedupLru;
use elohim_storage::p2p::gossip_dispatch::{handle_gossip, GossipDispatchCtx, GossipSource};
use elohim_storage::p2p::inventory_gossip::{
    BlobAddress, BlobHint, InventoryDelta, InventoryEnvelope, InventoryLevel, INVENTORY_TOPIC,
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

/// A pool where a cross-signed `AgentPeerBinding` binds the libp2p PeerId of
/// `transport_secret` to an agent, so that author's signed inventory envelopes are
/// admitted (the holder key is the one its PeerId embeds).
fn bound_pool(label: &str, transport_secret: [u8; 32]) -> DbPool {
    let pool = test_pool(label);
    let mut conn = pool.get().unwrap();
    elohim_storage::test_util::bind_cross_signed(
        &mut conn,
        "uhCAkTransportParityHolder",
        transport_secret,
    );
    drop(conn);
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
    let secret = [0x42u8; 32];
    let author = libp2p::identity::Keypair::ed25519_from_bytes(secret)
        .unwrap()
        .public()
        .to_peer_id()
        .to_string();
    let cid = compute_bytes_cid(b"a blob both planes hear about");
    let hint = BlobHint {
        address: BlobAddress::from(cid),
        size_bytes: Some(4096),
        recipient_hub_id: Some("hub:matthew".to_string()),
        epr_kind: Some("content".to_string()),
        tier: Some("dwelling".to_string()),
    };
    let envelope = InventoryEnvelope::sign(
        &AgentKeypair::from_secret(&secret).unwrap(),
        &author,
        InventoryLevel::Full,
        InventoryDelta {
            added: vec![BlobAddress::from(cid)],
            hints: vec![hint],
            removed: vec![],
            sequence: 1,
            emitted_at: 1_700_000_000_000_000,
        },
    );
    let payload = envelope.to_bytes();

    let (libp2p_tx, mut libp2p_rx) = tokio::sync::mpsc::channel(4);
    let (iroh_tx, mut iroh_rx) = tokio::sync::mpsc::channel(4);
    let libp2p_fetch = IrohInventoryFetch::new(Some(libp2p_tx));
    let iroh_fetch = IrohInventoryFetch::new(Some(iroh_tx));

    dispatch(
        &bound_pool("libp2p", secret),
        &libp2p_fetch,
        GossipSource::Libp2p {
            peer: "12D3KooWRelay".to_string(),
            message_id: "matrix-libp2p".to_string(),
            author: Some(author.clone()),
        },
        &payload,
    );
    dispatch(
        &bound_pool("iroh", secret),
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
                cids: left_cids,
            },
            P2PCommand::InventoryAdvertisement {
                source_peer_id: right_peer,
                hints: right_hints,
                cids: right_cids,
            },
        ) => {
            assert_eq!(left_peer, right_peer);
            assert_eq!(left_peer, author);
            assert_eq!(left_hints, right_hints);
            assert_eq!(left_cids, right_cids);
            assert_eq!(left_cids, vec![cid.to_string()]);
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
        &[compute_bytes_cid(b"no fetcher for this").to_string()],
    );

    assert_eq!(fetch.stats().unavailable, 1);
    assert_eq!(fetch.stats().enqueued, 0);
}
