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
