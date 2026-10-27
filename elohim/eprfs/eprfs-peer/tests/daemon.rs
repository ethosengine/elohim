//! Two device daemons in one process, over real iroh endpoints: each announces its signed head on
//! the steward topic, folds the other's crossing events, and acknowledges them (spec §3.2–§3.3).
//! A device contested on the roster is dropped with no restart (review focus item 3).

mod support;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use elohim_epr_rea::{ParticipantRow, SidecarRoster};
use eprfs_peer::{via_device, CrossingAllowList, HeadTable, PeerConfig, PeerDaemon, PeerKey};
use futures_lite::StreamExt;
use support::{
    berth_event, binding, contest, device, endpoint_row, genesis, policy, verifier, Checkout,
    Device, HANDLE,
};

const WITHIN: Duration = Duration::from_secs(20);

struct Side {
    checkout: Checkout,
    state: tempfile::TempDir,
}

impl Side {
    fn new(rows: &[ParticipantRow]) -> Side {
        let checkout = Checkout::new();
        let roster = SidecarRoster::open(checkout.dir.path(), HANDLE).expect("roster");
        for row in rows {
            roster.append(row.clone()).expect("row");
        }
        Side {
            checkout,
            state: tempfile::tempdir().expect("state"),
        }
    }

    fn roster_path(&self) -> std::path::PathBuf {
        SidecarRoster::open(self.checkout.dir.path(), HANDLE)
            .expect("roster")
            .log_path()
            .to_path_buf()
    }

    fn table(&self) -> HeadTable {
        HeadTable::load(self.state.path()).expect("table")
    }
}

fn config(
    me: &Device,
    chain_root: &str,
    key: PeerKey,
    side: &Side,
    devices: &[&Device],
    peers: Vec<iroh::NodeAddr>,
    listen: std::net::SocketAddrV4,
) -> PeerConfig {
    let signer = me.signer();
    PeerConfig {
        key,
        device: me.did.clone(),
        chain_root: chain_root.to_string(),
        flow_root: side.checkout.dir.path().to_path_buf(),
        roster: side.roster_path(),
        state_dir: side.state.path().to_path_buf(),
        allow: CrossingAllowList::from_policy(&policy()).expect("policy"),
        relay: None,
        listen: Some(listen),
        peers,
        signer: Arc::new(signer),
        verifier: verifier(devices),
        log: Arc::new(|_line: &str| {}),
    }
}

fn free_port() -> u16 {
    std::net::UdpSocket::bind("127.0.0.1:0")
        .and_then(|s| s.local_addr())
        .map(|a| a.port())
        .expect("a free port")
}

fn local(port: u16) -> std::net::SocketAddrV4 {
    std::net::SocketAddrV4::new(std::net::Ipv4Addr::LOCALHOST, port)
}

async fn eventually(what: &str, mut check: impl FnMut() -> bool) {
    tokio::time::timeout(WITHIN, async {
        while !check() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{what} within {WITHIN:?}"));
}

fn load_key(dir: &Path) -> PeerKey {
    PeerKey::load_or_generate(dir).expect("key")
}

#[tokio::test(flavor = "multi_thread")]
async fn two_daemons_fold_each_others_heads_and_drop_a_contested_device() {
    let a = device("DeviceA", 1);
    let b = device("DeviceB", 2);
    let (key_dir_a, key_dir_b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (key_a, key_b) = (load_key(key_dir_a.path()), load_key(key_dir_b.path()));
    let bound = binding(&a, &b);
    let rows = vec![
        genesis(&a),
        bound.clone(),
        endpoint_row(&a, &a, &key_a),
        endpoint_row(&a, &b, &key_b),
    ];
    let (id_a, id_b) = (key_a.node_id(), key_b.node_id());
    let side_a = Side::new(&rows);
    let side_b = Side::new(&rows);
    let claim = side_b
        .checkout
        .append(berth_event("claim", "dev-integration", "sb", "1.0"));
    let (port_a, port_b) = (free_port(), free_port());
    let addr = |id, port| {
        iroh::NodeAddr::new(id).with_direct_addresses([std::net::SocketAddr::V4(local(port))])
    };

    let daemon_a = PeerDaemon::start(config(
        &a,
        &a.did,
        key_a,
        &side_a,
        &[&a, &b],
        vec![addr(id_b, port_b)],
        local(port_a),
    ))
    .await
    .expect("A starts");
    let daemon_b = PeerDaemon::start(config(
        &b,
        &a.did,
        key_b,
        &side_b,
        &[&a, &b],
        vec![addr(id_a, port_a)],
        local(port_b),
    ))
    .await
    .expect("B starts");

    // Each lists the other's head, and A folds B's claim via B.
    eventually("A holds B's head", || {
        side_a.table().devices.contains_key(&b.did)
    })
    .await;
    eventually("B holds A's head", || {
        side_b.table().devices.contains_key(&a.did)
    })
    .await;
    eventually("A folds B's claim", || {
        side_a
            .checkout
            .events()
            .iter()
            .any(|(_, e)| via_device(e) == Some(b.did.as_str()))
    })
    .await;
    let known = eprfs_peer::folded_from(&side_a.checkout.store(), &b.did).expect("known");
    assert!(known.contains(&claim));

    // B's head carrying the claim is acknowledged: A's announced ack reaches B.
    let b_seq = side_a.table().devices[&b.did].seq;
    eventually("B sees A acknowledge its claim", || {
        side_b
            .table()
            .devices
            .get(&a.did)
            .and_then(|head| head.folded.get(&b.did).copied())
            .is_some_and(|acked| acked >= b_seq)
    })
    .await;

    // A contests B's binding in its own checkout: no restart, and B's heads are dropped.
    let dropped_before = side_a.table().dropped.unadmitted;
    let folded_before = side_a.table().devices[&b.did].seq;
    SidecarRoster::open(side_a.checkout.dir.path(), HANDLE)
        .expect("roster")
        .append(contest(&a, &bound))
        .expect("contest");
    // The reload forgets B's last head: heads.json, and every head A announces after it, stop
    // naming a device A no longer admits.
    eventually("A's head table forgets B", || {
        !side_a.table().devices.contains_key(&b.did)
    })
    .await;
    side_b
        .checkout
        .append(berth_event("claim", "alpha-fleet", "sb", "2.0"));
    // B's newer head is dropped as unadmitted if it still arrives; or B's connection is closed
    // and refused, and it never arrives at all.
    let node_b = hex(id_b);
    eventually("A drops B's newer head, or refuses B's connection", || {
        let table = side_a.table();
        table.dropped.unadmitted > dropped_before
            || table
                .channel
                .refused_connections
                .get(&node_b)
                .copied()
                .unwrap_or(0)
                > 0
    })
    .await;
    // The negative checks must see B's newer heads have been sent: wait until B itself announced
    // past what A had folded, then let B keep broadcasting (a change a second, each announced
    // after the 1 s debounce) for 3 s before asserting that nothing more was folded.
    eventually("B announces a head past what A folded", || {
        side_b
            .table()
            .own
            .as_ref()
            .is_some_and(|own| own.seq > folded_before)
    })
    .await;
    for i in 0..6 {
        side_b.checkout.append(berth_event(
            "claim",
            &format!("lane:settle-{i}"),
            "sb",
            &format!("{}.0", 3 + i),
        ));
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    assert!(
        side_b
            .table()
            .own
            .as_ref()
            .is_some_and(|own| own.seq > folded_before + 1),
        "B kept announcing while A was contested"
    );
    assert!(
        !side_a.table().devices.contains_key(&b.did),
        "no head of a contested device is recorded again"
    );
    assert_eq!(
        eprfs_peer::folded_from(&side_a.checkout.store(), &b.did)
            .expect("known")
            .len(),
        1,
        "nothing more is folded from a contested device"
    );

    daemon_a.shutdown().await;
    daemon_b.shutdown().await;
}

/// Two admitted devices homed to one relay, with no `--peer` direct address on either side, find
/// each other's heads: the roster's bare node ids are seeded with the relay URL at start.
#[tokio::test(flavor = "multi_thread")]
async fn two_daemons_find_each_other_through_the_relay_with_no_peer_address() {
    let (_map, relay_url, _relay) = iroh::test_utils::run_relay_server()
        .await
        .expect("in-process relay");
    let relay: url::Url = relay_url.into();
    let a = device("DeviceA", 1);
    let b = device("DeviceB", 2);
    let (key_dir_a, key_dir_b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (key_a, key_b) = (load_key(key_dir_a.path()), load_key(key_dir_b.path()));
    let rows = vec![
        genesis(&a),
        binding(&a, &b),
        endpoint_row(&a, &a, &key_a),
        endpoint_row(&a, &b, &key_b),
    ];
    let (side_a, side_b) = (Side::new(&rows), Side::new(&rows));
    let with_relay = |mut cfg: PeerConfig| {
        cfg.relay = Some(relay.clone());
        cfg.listen = None;
        cfg
    };
    let daemon_a = PeerDaemon::start(with_relay(config(
        &a,
        &a.did,
        key_a,
        &side_a,
        &[&a, &b],
        Vec::new(),
        local(0),
    )))
    .await
    .expect("A starts");
    let daemon_b = PeerDaemon::start(with_relay(config(
        &b,
        &a.did,
        key_b,
        &side_b,
        &[&a, &b],
        Vec::new(),
        local(0),
    )))
    .await
    .expect("B starts");
    eventually("A holds B's head through the relay", || {
        side_a.table().devices.contains_key(&b.did)
    })
    .await;
    eventually("B holds A's head through the relay", || {
        side_b.table().devices.contains_key(&a.did)
    })
    .await;
    daemon_a.shutdown().await;
    daemon_b.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn changed_then_unchanged_heads_reuse_the_index_and_skip_idle_fold_work() {
    let a = device("DeviceA", 1);
    let b = device("DeviceB", 2);
    let (key_dir_a, key_dir_b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (key_a, key_b) = (load_key(key_dir_a.path()), load_key(key_dir_b.path()));
    let rows = vec![
        genesis(&a),
        binding(&a, &b),
        endpoint_row(&a, &a, &key_a),
        endpoint_row(&a, &b, &key_b),
    ];
    let id_a = key_a.node_id();
    let side_a = Side::new(&rows);
    let b_checkout = Checkout::new();
    b_checkout.append(berth_event("claim", "dev-integration", "sb", "1.0"));
    let allow = CrossingAllowList::from_policy(&policy()).expect("policy");
    let b_store = Arc::new(eprfs_peer::ChannelStore::default());
    let first = eprfs_peer::project_channel(&b_checkout.store(), &allow).expect("projects");
    b_store.install(&first);
    let b_admission = Arc::new(std::sync::RwLock::new(eprfs_peer::Admission::from_roster(
        &rows,
        &a.did,
        &*verifier(&[&a, &b]),
    )));
    let endpoint_b = eprfs_peer::bind(
        &key_b,
        None,
        vec![eprfs_peer::TIER_ALPN.to_vec(), iroh_gossip::ALPN.to_vec()],
    )
    .await
    .expect("bind B");
    let addr_b = support::loopback(&endpoint_b);
    let gossip_b = iroh_gossip::net::Gossip::builder().spawn(endpoint_b.clone());
    let router_b = iroh::protocol::Router::builder(endpoint_b.clone())
        .accept(
            eprfs_peer::TIER_ALPN,
            eprfs_peer::TierProtocol::new(
                eprfs_peer::channel_tier_id(),
                b_store.clone(),
                b_admission,
                eprfs_peer::TierLimits::default(),
            )
            .expect("tier"),
        )
        .accept(iroh_gossip::ALPN, gossip_b.clone())
        .spawn();
    let port_a = free_port();
    let daemon_a = PeerDaemon::start(config(
        &a,
        &a.did,
        key_a,
        &side_a,
        &[&a, &b],
        vec![addr_b],
        local(port_a),
    ))
    .await
    .expect("A starts");
    endpoint_b
        .add_node_addr(
            iroh::NodeAddr::new(id_a)
                .with_direct_addresses([std::net::SocketAddr::V4(local(port_a))]),
        )
        .expect("A's address");
    let (sender_b, mut receiver_b) = gossip_b
        .subscribe(eprfs_peer::topic_for(&a.did), vec![id_a])
        .await
        .expect("B joins")
        .split();
    let joined = tokio::time::timeout(WITHIN, async {
        loop {
            match receiver_b.next().await {
                Some(Ok(iroh_gossip::api::Event::NeighborUp(node))) if node == id_a => break true,
                Some(_) => {}
                None => break false,
            }
        }
    })
    .await
    .expect("B sees A as a gossip neighbour");
    assert!(joined, "B's gossip stream stays open after joining A");
    let signer_b = b.signer();
    let send = |seq, root: &eprfs_core::BlobCid| {
        eprfs_peer::ChannelHead::sign(
            b.did.clone(),
            root.clone(),
            seq,
            seq,
            Default::default(),
            &signer_b,
        )
        .expect("signs")
        .to_wire()
        .expect("wire")
    };
    sender_b
        .broadcast(send(1, first.root()).into())
        .await
        .expect("first head");
    eventually("A folds B's first head", || {
        side_a
            .table()
            .devices
            .get(&b.did)
            .is_some_and(|head| head.seq >= 1)
    })
    .await;
    assert_eq!(daemon_a.fold_stats().store_parses, 1);

    b_checkout.append(berth_event("release", "dev-integration", "sb", "2.0"));
    let second = eprfs_peer::project_channel(&b_checkout.store(), &allow).expect("projects");
    b_store.install(&second);
    sender_b
        .broadcast(send(2, second.root()).into())
        .await
        .expect("second head");
    eventually("A folds B's changed second head", || {
        side_a
            .table()
            .devices
            .get(&b.did)
            .is_some_and(|head| head.seq >= 2)
    })
    .await;
    let after_changed = daemon_a.fold_stats();
    assert_eq!(
        after_changed.store_parses, 1,
        "the held-CID index was reused"
    );

    sender_b
        .broadcast(send(3, second.root()).into())
        .await
        .expect("unchanged head");
    eventually("A records B's unchanged third head", || {
        side_a
            .table()
            .devices
            .get(&b.did)
            .is_some_and(|head| head.seq >= 3)
    })
    .await;
    assert_eq!(
        daemon_a.fold_stats(),
        after_changed,
        "an unchanged root starts no task and parses no store"
    );

    daemon_a.shutdown().await;
    router_b.shutdown().await.expect("shutdown B");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_running_daemon_folds_a_neighbours_head_after_that_neighbour_restarts() {
    let a = device("DeviceA", 1);
    let b = device("DeviceB", 2);
    let (key_dir_a, key_dir_b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let key_a = load_key(key_dir_a.path());
    let key_b = load_key(key_dir_b.path());
    let rows = vec![
        genesis(&a),
        binding(&a, &b),
        endpoint_row(&a, &a, &key_a),
        endpoint_row(&a, &b, &key_b),
    ];
    let (id_a, id_b) = (key_a.node_id(), key_b.node_id());
    let (side_a, side_b) = (Side::new(&rows), Side::new(&rows));
    side_b
        .checkout
        .append(berth_event("claim", "dev-integration", "sb", "1.0"));
    let (port_a, port_b) = (free_port(), free_port());
    let addr = |id, port| {
        iroh::NodeAddr::new(id).with_direct_addresses([std::net::SocketAddr::V4(local(port))])
    };
    let daemon_a = PeerDaemon::start(config(
        &a,
        &a.did,
        key_a,
        &side_a,
        &[&a, &b],
        vec![addr(id_b, port_b)],
        local(port_a),
    ))
    .await
    .expect("A starts");
    let daemon_b = PeerDaemon::start(config(
        &b,
        &a.did,
        key_b,
        &side_b,
        &[&a, &b],
        vec![addr(id_a, port_a)],
        local(port_b),
    ))
    .await
    .expect("B starts");
    eventually("A folds B before the restart", || {
        side_a.table().devices.contains_key(&b.did)
    })
    .await;
    let before = side_a.table().devices[&b.did].seq;

    daemon_b.shutdown().await;
    side_b
        .checkout
        .append(berth_event("release", "dev-integration", "sb", "2.0"));
    let restarted_b = PeerDaemon::start(config(
        &b,
        &a.did,
        load_key(key_dir_b.path()),
        &side_b,
        &[&a, &b],
        vec![addr(id_a, port_a)],
        local(port_b),
    ))
    .await
    .expect("B restarts on the same key and port");
    tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            let newer = side_a
                .table()
                .devices
                .get(&b.did)
                .is_some_and(|head| head.seq > before);
            let crossed = side_a
                .checkout
                .events()
                .iter()
                .filter(|(_, event)| via_device(event) == Some(b.did.as_str()))
                .count()
                >= 2;
            if newer && crossed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("A re-joins and folds B's new head within 60 seconds");
    assert!(
        side_a
            .checkout
            .events()
            .iter()
            .filter(|(_, event)| via_device(event) == Some(b.did.as_str()))
            .count()
            >= 2,
        "the post-restart event crossed without restarting A"
    );

    daemon_a.shutdown().await;
    restarted_b.shutdown().await;
}

/// A device whose tier accepts a connection and never answers: an admitted peer that stalls.
#[derive(Debug, Clone, Default)]
struct StallingTier {
    accepted: Arc<std::sync::atomic::AtomicUsize>,
}

impl iroh::protocol::ProtocolHandler for StallingTier {
    async fn accept(
        &self,
        connection: iroh::endpoint::Connection,
    ) -> Result<(), iroh::protocol::AcceptError> {
        self.accepted
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // Hold the connection open and answer nothing.
        let _held = connection;
        std::future::pending::<()>().await;
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stalled_device_holds_back_neither_another_devices_fold_nor_our_announce() {
    let a = device("DeviceA", 1);
    let b = device("DeviceB", 2);
    let c = device("DeviceC", 3);
    let dirs: Vec<tempfile::TempDir> = (0..3).map(|_| tempfile::tempdir().unwrap()).collect();
    let (key_a, key_b, key_c) = (
        load_key(dirs[0].path()),
        load_key(dirs[1].path()),
        load_key(dirs[2].path()),
    );
    let rows = vec![
        genesis(&a),
        binding(&a, &b),
        binding(&a, &c),
        endpoint_row(&a, &a, &key_a),
        endpoint_row(&a, &b, &key_b),
        endpoint_row(&a, &c, &key_c),
    ];
    let (id_a, id_b) = (key_a.node_id(), key_b.node_id());
    let side_a = Side::new(&rows);
    let side_b = Side::new(&rows);
    let (port_a, port_b) = (free_port(), free_port());
    let addr = |id, port| {
        iroh::NodeAddr::new(id).with_direct_addresses([std::net::SocketAddr::V4(local(port))])
    };

    // C: an admitted device that gossips heads but whose tier never answers.
    let stall = StallingTier::default();
    let endpoint_c = eprfs_peer::bind(
        &key_c,
        None,
        vec![eprfs_peer::TIER_ALPN.to_vec(), iroh_gossip::ALPN.to_vec()],
    )
    .await
    .expect("bind C");
    let addr_c = support::loopback(&endpoint_c);
    let gossip_c = iroh_gossip::net::Gossip::builder().spawn(endpoint_c.clone());
    let router_c = iroh::protocol::Router::builder(endpoint_c.clone())
        .accept(eprfs_peer::TIER_ALPN, stall.clone())
        .accept(iroh_gossip::ALPN, gossip_c.clone())
        .spawn();

    let verify = [&a, &b, &c];
    let daemon_a = PeerDaemon::start(config(
        &a,
        &a.did,
        key_a,
        &side_a,
        &verify,
        vec![addr(id_b, port_b), addr_c.clone()],
        local(port_a),
    ))
    .await
    .expect("A starts");
    let daemon_b = PeerDaemon::start(config(
        &b,
        &a.did,
        key_b,
        &side_b,
        &verify,
        vec![addr(id_a, port_a)],
        local(port_b),
    ))
    .await
    .expect("B starts");

    endpoint_c
        .add_node_addr(addr(id_a, port_a))
        .expect("A's address");
    let (sender_c, _receiver_c) = gossip_c
        .subscribe(eprfs_peer::topic_for(&a.did), vec![id_a])
        .await
        .expect("C joins")
        .split();
    let signer_c = c.signer();
    let c_did = c.did.clone();
    let c_heads = tokio::spawn(async move {
        for seq in 1..200u64 {
            let head = eprfs_peer::ChannelHead::sign(
                c.did.clone(),
                eprfs_core::BlobCid::compute(b"c's root"),
                seq,
                seq,
                Default::default(),
                &signer_c,
            )
            .expect("signs");
            let _ = sender_c
                .broadcast(head.to_wire().expect("wire").into())
                .await;
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    });
    let accepted = stall.accepted.clone();
    eventually("A dials C's stalled tier", || {
        accepted.load(std::sync::atomic::Ordering::SeqCst) > 0
    })
    .await;

    // With C's fold stalled, B's claim still crosses to A promptly and A still announces.
    let started = std::time::Instant::now();
    side_b
        .checkout
        .append(berth_event("claim", "dev-integration", "sb", "1.0"));
    let fast = Duration::from_secs(8);
    // The device table is projected from the folded events, so wait for both: under a loaded
    // full-workspace run the event can land a beat before its table row.
    tokio::time::timeout(fast, async {
        while !side_a
            .checkout
            .events()
            .iter()
            .any(|(_, e)| via_device(e) == Some(b.did.as_str()))
            || !side_a.table().devices.contains_key(&b.did)
        {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("A folds B's claim while C's fold is stalled");
    let b_seq = side_a.table().devices[&b.did].seq;
    tokio::time::timeout(fast, async {
        while side_b
            .table()
            .devices
            .get(&a.did)
            .and_then(|head| head.folded.get(&b.did).copied())
            .is_none_or(|acked| acked < b_seq)
        {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("A acknowledges B while C's fold is stalled");
    assert!(started.elapsed() < fast);
    assert!(
        !side_a.table().devices.contains_key(&c_did),
        "C's stalled fold was never recorded as folded"
    );

    c_heads.abort();
    router_c.shutdown().await.expect("shutdown C");
    daemon_a.shutdown().await;
    daemon_b.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_started_daemon_serves_only_its_channel_tree() {
    let a = device("DeviceA", 1);
    let b = device("DeviceB", 2);
    let (key_dir_a, key_dir_b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (key_a, key_b) = (load_key(key_dir_a.path()), load_key(key_dir_b.path()));
    let rows = vec![
        genesis(&a),
        binding(&a, &b),
        endpoint_row(&a, &a, &key_a),
        endpoint_row(&a, &b, &key_b),
    ];
    let side_b = Side::new(&rows);
    side_b
        .checkout
        .append(berth_event("claim", "cargo", "sb", "1.0"));
    // In B's own flow store, never in what B serves.
    let refused = side_b
        .checkout
        .append(berth_event("refuse", "cargo", "sb", "2.0"));
    let port_b = free_port();
    let daemon_b = PeerDaemon::start(config(
        &b,
        &a.did,
        key_b,
        &side_b,
        &[&a, &b],
        Vec::new(),
        local(port_b),
    ))
    .await
    .expect("B starts");
    eventually("B announces", || side_b.table().own.is_some()).await;
    let root_b = side_b.table().own.expect("own head").root;
    // The head table names the one flow store this device's channel is projected from, and each
    // own head the event it counted up to (here B's second, the local `refuse`), so a reader of
    // the lease view reads that store and can tell when it is no longer the store counted.
    let table_b = side_b.table();
    let place = table_b.store.expect("the channel store is recorded");
    let checkout = side_b
        .checkout
        .dir
        .path()
        .canonicalize()
        .expect("canonical");
    assert_eq!(place.root, checkout);
    assert_eq!(place.log, checkout.join(".eprfs/status/flows.jsonl"));
    let logged = table_b.own_log.last().expect("an own announce");
    assert_eq!(logged.events, 2);
    assert_eq!(logged.last, Some(refused.to_string()));

    let client = eprfs_peer::bind(&key_a, None, vec![])
        .await
        .expect("bind A");
    let to_b = iroh::NodeAddr::new(daemon_b.node_id())
        .with_direct_addresses([std::net::SocketAddr::V4(local(port_b))]);
    let limits = eprfs_core::TreeLimits::default();
    let ask = |req| {
        let client = client.clone();
        let to_b = to_b.clone();
        async move {
            tokio::time::timeout(WITHIN, eprfs_peer::tier_call(&client, to_b, req, &limits))
                .await
                .expect("answers in time")
                .expect("answers")
        }
    };
    let listed = ask(eprfs_peer::TierRequest::List {
        root: root_b.clone(),
        path: eprfs_core::TreePath::from_slash_bytes(b"channel/events").expect("path"),
    })
    .await;
    assert!(
        matches!(listed, eprfs_peer::TierAnswer::List(ref l) if l.entries.len() == 1),
        "{listed:?}"
    );
    for root in [
        eprfs_core::BlobCid::parse(&refused.to_string()).expect("cid"),
        eprfs_core::BlobCid::compute(b"anything else this device holds"),
    ] {
        let answer = ask(eprfs_peer::TierRequest::List {
            root,
            path: eprfs_core::TreePath::root(),
        })
        .await;
        assert!(
            matches!(answer, eprfs_peer::TierAnswer::Unanswered { .. }),
            "{answer:?}"
        );
    }

    client.close().await;
    daemon_b.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_projection_failure_is_recorded_where_status_reads_it() {
    let a = device("DeviceA", 1);
    let key_dir = tempfile::tempdir().unwrap();
    let key_a = load_key(key_dir.path());
    let rows = vec![genesis(&a), endpoint_row(&a, &a, &key_a)];
    let side = Side::new(&rows);
    side.checkout
        .append(berth_event("claim", "cargo", "sa", "1.0"));
    let log = side.checkout.store().log_path().to_path_buf();
    let daemon = PeerDaemon::start(config(
        &a,
        &a.did,
        key_a,
        &side,
        &[&a],
        Vec::new(),
        local(free_port()),
    ))
    .await
    .expect("A starts");
    eventually("A announces", || side.table().own.is_some()).await;

    // A tampered line makes the flow store unreadable: the projection fails and says so.
    let text = std::fs::read_to_string(&log).expect("log");
    std::fs::write(&log, text.replace("cargo", "cargx")).expect("tamper");
    eventually("the failure is recorded", || {
        side.table().channel.last_projection_error.is_some()
    })
    .await;
    assert!(side.table().channel.projection_failures >= 1);

    daemon.shutdown().await;
}

#[test]
fn a_failing_projection_is_retried_with_backoff_until_the_store_changes() {
    use eprfs_peer::{ProjectionRetry, PROJECTION_RETRY_FIRST, PROJECTION_RETRY_MAX};
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let mut retry = ProjectionRetry::default();
    // Nothing has failed: every state of the store is projected at once.
    assert!(retry.due(&1u8, start));
    // The first failure of a state is counted once; the same state waits, doubling, to the cap.
    assert!(
        retry.failed(1u8, start),
        "the first failure of a state is new"
    );
    let mut at = start;
    let mut delay = PROJECTION_RETRY_FIRST;
    let mut waits = Vec::new();
    for _ in 0..8 {
        assert!(!retry.due(&1u8, at + delay - Duration::from_millis(1)));
        assert!(retry.due(&1u8, at + delay));
        at += delay;
        waits.push(delay);
        assert!(!retry.failed(1u8, at), "the same state is counted once");
        delay = (delay * 2).min(PROJECTION_RETRY_MAX);
    }
    assert_eq!(waits.first(), Some(&PROJECTION_RETRY_FIRST));
    assert_eq!(waits.last(), Some(&PROJECTION_RETRY_MAX), "{waits:?}");
    // At the 250 ms tick, a minute of one failing state re-parses the store a handful of times,
    // not 240.
    let tries = waits
        .iter()
        .scan(Duration::ZERO, |sum, wait| {
            *sum += *wait;
            Some(*sum)
        })
        .take_while(|sum| *sum <= Duration::from_secs(60))
        .count();
    assert!(tries <= 6, "{tries} retries within a minute");
    // A change to the store resets the wait: the new state is projected on the next tick.
    assert!(retry.due(&2u8, at));
    assert!(retry.failed(2u8, at), "a new state's failure is new");
    assert!(!retry.due(&2u8, at + PROJECTION_RETRY_FIRST - Duration::from_millis(1)));
    // A success forgets the failure.
    retry.succeeded();
    assert!(retry.due(&2u8, at));
}

/// A real channel tier whose answers wait until the test opens a gate: a fold stalled mid-fetch.
#[derive(Debug, Clone)]
struct GatedTier {
    inner: eprfs_peer::TierProtocol,
    open: Arc<tokio::sync::Semaphore>,
    accepted: Arc<std::sync::atomic::AtomicUsize>,
}

impl iroh::protocol::ProtocolHandler for GatedTier {
    async fn accept(
        &self,
        connection: iroh::endpoint::Connection,
    ) -> Result<(), iroh::protocol::AcceptError> {
        self.accepted
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // The gate is opened once, for good: the permit is never returned.
        if let Ok(permit) = self.open.acquire().await {
            permit.forget();
            self.open.add_permits(1);
        }
        self.inner.accept(connection).await
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_contest_during_a_stalled_fold_appends_nothing_and_records_no_head() {
    let a = device("DeviceA", 1);
    let c = device("DeviceC", 3);
    let dirs: Vec<tempfile::TempDir> = (0..2).map(|_| tempfile::tempdir().unwrap()).collect();
    let (key_a, key_c) = (load_key(dirs[0].path()), load_key(dirs[1].path()));
    let bound_c = binding(&a, &c);
    let rows = vec![
        genesis(&a),
        bound_c.clone(),
        endpoint_row(&a, &a, &key_a),
        endpoint_row(&a, &c, &key_c),
    ];
    let id_a = key_a.node_id();
    let side_a = Side::new(&rows);

    // C: a real device with a claim in its channel tree, whose tier answers only once the gate
    // opens.
    let c_checkout = Checkout::new();
    c_checkout.append(berth_event("claim", "alpha-fleet", "sc", "1.0"));
    let allow = CrossingAllowList::from_policy(&policy()).expect("policy");
    let tree = eprfs_peer::project_channel(&c_checkout.store(), &allow).expect("projects");
    let store = Arc::new(eprfs_peer::ChannelStore::default());
    store.install(&tree);
    let c_admission = Arc::new(std::sync::RwLock::new(eprfs_peer::Admission::from_roster(
        &rows,
        &a.did,
        &*verifier(&[&a, &c]),
    )));
    let gated = GatedTier {
        inner: eprfs_peer::TierProtocol::new(
            eprfs_peer::channel_tier_id(),
            store,
            c_admission,
            eprfs_peer::TierLimits::default(),
        )
        .expect("tier"),
        open: Arc::new(tokio::sync::Semaphore::new(0)),
        accepted: Arc::default(),
    };
    let endpoint_c = eprfs_peer::bind(
        &key_c,
        None,
        vec![eprfs_peer::TIER_ALPN.to_vec(), iroh_gossip::ALPN.to_vec()],
    )
    .await
    .expect("bind C");
    let addr_c = support::loopback(&endpoint_c);
    let gossip_c = iroh_gossip::net::Gossip::builder().spawn(endpoint_c.clone());
    let router_c = iroh::protocol::Router::builder(endpoint_c.clone())
        .accept(eprfs_peer::TIER_ALPN, gated.clone())
        .accept(iroh_gossip::ALPN, gossip_c.clone())
        .spawn();

    let port_a = free_port();
    let daemon_a = PeerDaemon::start(config(
        &a,
        &a.did,
        key_a,
        &side_a,
        &[&a, &c],
        vec![addr_c],
        local(port_a),
    ))
    .await
    .expect("A starts");
    endpoint_c
        .add_node_addr(
            iroh::NodeAddr::new(id_a)
                .with_direct_addresses([std::net::SocketAddr::V4(local(port_a))]),
        )
        .expect("A's address");
    let (sender_c, _receiver_c) = gossip_c
        .subscribe(eprfs_peer::topic_for(&a.did), vec![id_a])
        .await
        .expect("C joins")
        .split();
    let signer_c = c.signer();
    let c_did = c.did.clone();
    let root_c = tree.root().clone();
    let c_heads = tokio::spawn(async move {
        for seq in 1..200u64 {
            let head = eprfs_peer::ChannelHead::sign(
                c_did.clone(),
                root_c.clone(),
                seq,
                seq,
                Default::default(),
                &signer_c,
            )
            .expect("signs");
            let _ = sender_c
                .broadcast(head.to_wire().expect("wire").into())
                .await;
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    });
    let accepted = gated.accepted.clone();
    eventually("A's fold of C is in flight", || {
        accepted.load(std::sync::atomic::Ordering::SeqCst) > 0
    })
    .await;

    // The Contest lands in A's own checkout while the fold waits; then C's tier answers.
    SidecarRoster::open(side_a.checkout.dir.path(), HANDLE)
        .expect("roster")
        .append(contest(&a, &bound_c))
        .expect("contest");
    let node_c = hex(key_c.node_id());
    eventually(
        "A drops C's heads as unadmitted, or refuses C's connection",
        || {
            let table = side_a.table();
            table.dropped.unadmitted > 0
                || table
                    .channel
                    .refused_connections
                    .get(&node_c)
                    .copied()
                    .unwrap_or(0)
                    > 0
        },
    )
    .await;
    gated.open.add_permits(1);
    tokio::time::sleep(Duration::from_secs(3)).await;

    assert!(
        !side_a
            .checkout
            .events()
            .iter()
            .any(|(_, e)| via_device(e) == Some(c.did.as_str())),
        "nothing of C is appended after its Contest"
    );
    assert!(
        !side_a.table().devices.contains_key(&c.did),
        "the head table records no head of a contested device"
    );

    c_heads.abort();
    router_c.shutdown().await.expect("shutdown C");
    daemon_a.shutdown().await;
}

/// A node id as the head table keys refused connections: its 32 bytes in lowercase hex.
fn hex(node: iroh::NodeId) -> String {
    node.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

/// The QUIC application code a connection was closed with, once it closes.
async fn closed_with(connection: &iroh::endpoint::Connection) -> Option<u64> {
    match tokio::time::timeout(WITHIN, connection.closed()).await {
        Ok(iroh::endpoint::ConnectionError::ApplicationClosed(close)) => {
            Some(close.error_code.into_inner())
        }
        _ => None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_contested_device_is_disconnected_refused_like_a_stranger_and_counted() {
    let a = device("DeviceA", 1);
    let c = device("DeviceC", 3);
    let dirs: Vec<tempfile::TempDir> = (0..3).map(|_| tempfile::tempdir().unwrap()).collect();
    let (key_a, key_c, key_d) = (
        load_key(dirs[0].path()),
        load_key(dirs[1].path()),
        load_key(dirs[2].path()),
    );
    let bound = binding(&a, &c);
    let rows = vec![
        genesis(&a),
        bound.clone(),
        endpoint_row(&a, &a, &key_a),
        endpoint_row(&a, &c, &key_c),
    ];
    let side_a = Side::new(&rows);
    let port_a = free_port();
    let daemon_a = PeerDaemon::start(config(
        &a,
        &a.did,
        key_a,
        &side_a,
        &[&a, &c],
        vec![],
        local(port_a),
    ))
    .await
    .expect("A starts");
    let addr_a = iroh::NodeAddr::new(daemon_a.node_id())
        .with_direct_addresses([std::net::SocketAddr::V4(local(port_a))]);
    let alpns = vec![eprfs_peer::TIER_ALPN.to_vec(), iroh_gossip::ALPN.to_vec()];
    let endpoint_c = eprfs_peer::bind(&key_c, None, alpns.clone())
        .await
        .expect("C binds");
    let endpoint_d = eprfs_peer::bind(&key_d, None, alpns)
        .await
        .expect("D binds");
    let tree = eprfs_core::TreeLimits::default();
    let question = eprfs_peer::TierRequest::Read {
        root: eprfs_core::BlobCid::compute(b"no such root"),
        path: eprfs_core::TreePath::from_slash_bytes(b"channel").expect("path"),
    };

    // C, admitted, holds an open tier connection that A has answered on.
    let open = endpoint_c
        .connect(addr_a.clone(), eprfs_peer::TIER_ALPN)
        .await
        .expect("C connects");
    eprfs_peer::tier_request(&open, &question, &tree)
        .await
        .expect("A answers an admitted device");

    // A contests C in its own checkout: with no restart, A closes C's open connection.
    SidecarRoster::open(side_a.checkout.dir.path(), HANDLE)
        .expect("roster")
        .append(contest(&a, &bound))
        .expect("contest");
    assert_eq!(
        closed_with(&open).await,
        Some(u64::from(eprfs_peer::NOT_ADMITTED)),
        "the contested device's open connection is closed as not admitted"
    );

    // New connections from C are refused exactly as a stranger's (D's): the gossip ALPN closes
    // them with the same code, and the tier gives the same uniform refusal.
    for (who, endpoint) in [("C", &endpoint_c), ("stranger D", &endpoint_d)] {
        let gossip = endpoint
            .connect(addr_a.clone(), iroh_gossip::ALPN)
            .await
            .expect("the handshake completes");
        assert_eq!(
            closed_with(&gossip).await,
            Some(u64::from(eprfs_peer::NOT_ADMITTED)),
            "{who}'s gossip connection is refused"
        );
    }
    let ask = |endpoint: iroh::Endpoint| {
        let (addr, question) = (addr_a.clone(), question.clone());
        async move {
            eprfs_peer::tier_call(&endpoint, addr, question, &tree)
                .await
                .expect("an answer")
        }
    };
    let from_c = ask(endpoint_c.clone()).await;
    let from_d = ask(endpoint_d.clone()).await;
    assert!(
        matches!(from_c, eprfs_peer::TierAnswer::Unanswered { .. }),
        "{from_c:?}"
    );
    assert_eq!(from_c, from_d, "C gets a stranger's refusal");

    // A counts the refused connections per node id, in the head table `epr peer status` reads.
    // C is a device the roster names (contested): counted one by one. D is a stranger: counted in
    // the strangers total. Written at most once per re-announce interval, so allow for one.
    tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let counters = side_a.table().channel;
            if counters
                .refused_connections
                .get(&hex(key_c.node_id()))
                .copied()
                .unwrap_or(0)
                >= 3
                && counters.refused_strangers >= 2
                && !counters
                    .refused_connections
                    .contains_key(&hex(key_d.node_id()))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .expect("A records C's refused connections and D's as a stranger's within 45 s");

    daemon_a.shutdown().await;
    endpoint_c.close().await;
    endpoint_d.close().await;
}
