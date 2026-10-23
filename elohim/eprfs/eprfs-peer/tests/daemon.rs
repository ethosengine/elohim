//! Two device daemons in one process, over real iroh endpoints: each announces its signed head on
//! the steward topic, folds the other's crossing events, and acknowledges them (spec §3.2–§3.3).
//! A device contested on the roster is dropped with no restart (review focus item 3).

mod support;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use elohim_epr_rea::{ParticipantRow, SidecarRoster};
use eprfs_peer::{via_device, CrossingAllowList, HeadTable, PeerConfig, PeerDaemon, PeerKey};
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
    SidecarRoster::open(side_a.checkout.dir.path(), HANDLE)
        .expect("roster")
        .append(contest(&a, &bound))
        .expect("contest");
    let dropped_before = side_a.table().dropped.unadmitted;
    let folded_before = side_a.table().devices[&b.did].seq;
    side_b
        .checkout
        .append(berth_event("claim", "alpha-fleet", "sb", "2.0"));
    eventually("A drops B's newer head as unadmitted", || {
        side_a.table().dropped.unadmitted > dropped_before
    })
    .await;
    assert_eq!(side_a.table().devices[&b.did].seq, folded_before);
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
    tokio::time::timeout(fast, async {
        while !side_a
            .checkout
            .events()
            .iter()
            .any(|(_, e)| via_device(e) == Some(b.did.as_str()))
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
