//! Compatibility between the Rulebook-backed R1 relay resolver and a peer fixed to one relay.

mod support;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use elohim_epr_rea::{ParticipantRow, SidecarRoster};
use eprfs_peer::{relay_from_env, HeadTable, PeerConfig, PeerDaemon, PeerKey, Rules};
use support::{binding, device, endpoint_row, genesis, policy, verifier, Checkout, Device, HANDLE};

const WITHIN: Duration = Duration::from_secs(30);

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

    fn roster_path(&self) -> PathBuf {
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
    relay: url::Url,
) -> PeerConfig {
    let signer = me.signer();
    PeerConfig {
        key,
        device: me.did.clone(),
        chain_root: chain_root.to_string(),
        flow_root: side.checkout.dir.path().to_path_buf(),
        roster: side.roster_path(),
        state_dir: side.state.path().to_path_buf(),
        rules: Rules::from_policy(&policy()).expect("policy"),
        policy: None,
        relay: Some(relay),
        listen: None,
        peers: Vec::new(),
        signer: Arc::new(signer),
        verifier: verifier(devices),
        log: Arc::new(|_line: &str| {}),
    }
}

fn load_key(dir: &Path) -> PeerKey {
    PeerKey::load_or_generate(dir).expect("key")
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

#[tokio::test(flavor = "multi_thread")]
async fn r1_daemon_links_with_a_daemon_fixed_to_one_relay() {
    let (_first_map, first_relay_url, _first_relay) = iroh::test_utils::run_relay_server()
        .await
        .expect("first in-process relay");
    let (_second_map, second_relay_url, _second_relay) = iroh::test_utils::run_relay_server()
        .await
        .expect("second in-process relay");
    let fixed_relay: url::Url = first_relay_url.into();
    let second_relay: url::Url = second_relay_url.into();
    assert_ne!(fixed_relay, second_relay);
    let rulebook = tempfile::tempdir().expect("Rulebook checkout");
    let policy_path = rulebook.path().join("genesis/agentic/pool-policy.json");
    std::fs::create_dir_all(policy_path.parent().expect("policy parent"))
        .expect("policy parent exists");
    std::fs::write(
        policy_path,
        serde_json::to_vec(&serde_json::json!({
            "berth": {
                "channel": {
                    "relays": [fixed_relay.as_str(), second_relay.as_str()],
                    "relay_mode": "single"
                }
            }
        }))
        .expect("policy JSON"),
    )
    .expect("policy file");
    std::env::set_var("EPR_CHECKOUT", rulebook.path());
    std::env::remove_var("EPR_PEER_RELAYS");
    std::env::remove_var("EPR_PEER_RELAY_URL");
    let r1_relay = relay_from_env()
        .expect("R1 relay resolution")
        .expect("R1 relaying remains enabled");
    assert_eq!(r1_relay, fixed_relay);

    let a = device("DeviceA", 1);
    let b = device("DeviceB", 2);
    let (key_dir_a, key_dir_b) = (
        tempfile::tempdir().expect("A key dir"),
        tempfile::tempdir().expect("B key dir"),
    );
    let (key_a, key_b) = (load_key(key_dir_a.path()), load_key(key_dir_b.path()));
    let rows = vec![
        genesis(&a),
        binding(&a, &b),
        endpoint_row(&a, &a, &key_a),
        endpoint_row(&a, &b, &key_b),
    ];
    let (side_a, side_b) = (Side::new(&rows), Side::new(&rows));

    let daemon_a = PeerDaemon::start(config(&a, &a.did, key_a, &side_a, &[&a, &b], r1_relay))
        .await
        .expect("R1 daemon starts");
    let daemon_b = PeerDaemon::start(config(&b, &a.did, key_b, &side_b, &[&a, &b], fixed_relay))
        .await
        .expect("fixed-relay daemon starts");

    eventually("R1 receives the fixed-relay daemon's head", || {
        side_a.table().devices.contains_key(&b.did)
    })
    .await;
    eventually("the fixed-relay daemon receives R1's head", || {
        side_b.table().devices.contains_key(&a.did)
    })
    .await;

    daemon_a.shutdown().await;
    daemon_b.shutdown().await;
}
