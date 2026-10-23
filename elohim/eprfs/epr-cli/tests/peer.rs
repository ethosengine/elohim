//! `epr peer` and `epr flow fold`: a device folds another device's channel root from that
//! device's running peer, once, and reads what it folded back through `epr peer status`.
//!
//! Every key is a temp file: each child gets its own `ELOHIM_DEVICE_KEY_FILE` and
//! `XDG_CONFIG_HOME`, so no test touches this workspace's real device or peer key.
#![cfg(feature = "peer")]

use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

use elohim_epr_cli::actor::{
    device_authorize, device_bind, device_endpoint, device_enroll, witness,
};
use elohim_epr_cli::device_key::{self, DeviceKey};
use elohim_epr_rea::{
    atom_cid, AgentRef, FlowEvent, FlowRecord, FlowStore, Magnitude, ParticipantRow, ReaVerb,
    SidecarFlowStore, SidecarRoster,
};
use eprfs_peer::{
    project_channel, via_device, CrossingAllowList, DeviceHead, HeadTable, PeerConfig, PeerDaemon,
    PeerKey, StorePlace,
};
use tempfile::TempDir;

const HANDLE: &str = "matthew";

fn git(root: &Path, args: &[&str]) {
    let out = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture Author")
        .env("GIT_AUTHOR_EMAIL", "author@example.test")
        .env("GIT_COMMITTER_NAME", "Fixture Author")
        .env("GIT_COMMITTER_EMAIL", "author@example.test")
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?} failed");
}

fn checkout() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join(".epr-meta")).unwrap();
    std::fs::write(
        dir.path().join(".epr-meta/repository.yaml"),
        "version: 1\nagent: repo:ethosengine/elohim\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("README.md"), "fixture\n").unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "fixture"]);
    dir
}

/// One device: its device key file, its config home (where its peer key lives) and checkout.
struct Device {
    key_file: PathBuf,
    key: DeviceKey,
    config: TempDir,
    checkout: TempDir,
}

impl Device {
    fn new(keys: &TempDir, name: &str) -> Device {
        let key_file = keys.path().join(name).join("ed25519.seed");
        let key = DeviceKey::load_or_generate(&key_file).unwrap();
        Device {
            key_file,
            key,
            config: TempDir::new().unwrap(),
            checkout: checkout(),
        }
    }

    fn epr(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_epr"))
            .args(args)
            .env(device_key::DEVICE_KEY_ENV, &self.key_file)
            .env("XDG_CONFIG_HOME", self.config.path())
            .env_remove("EPR_PEER_RELAY_URL")
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env_remove("CLAUDE_SESSION_ID")
            .env_remove("ELOHIM_SESSION_ID")
            .output()
            .expect("epr runs")
    }

    fn root(&self) -> &str {
        self.checkout.path().to_str().unwrap()
    }

    fn peer_dir(&self) -> PathBuf {
        self.config.path().join("epr").join("peer")
    }

    fn node_id(&self) -> String {
        let out = self.epr(&["peer", "id"]);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    }
}

fn policy(dir: &Path) -> PathBuf {
    let path = dir.join("pool-policy.json");
    std::fs::write(
        &path,
        r#"{"berth": {"channel": {"crosses": {"dev-berth-events@1": ["claim", "release"]}}}}"#,
    )
    .unwrap();
    path
}

fn claim(session: &str) -> FlowEvent {
    FlowEvent {
        action: ReaVerb::Cite,
        provider: AgentRef("human:matthew".into()),
        receiver: AgentRef("repo:ethosengine/elohim".into()),
        resource: atom_cid(&"berth".to_string()).unwrap(),
        quantity: Magnitude::Count {
            value: 1.0,
            unit: "run-note".into(),
        },
        process: None,
        in_scope_of: atom_cid(&"repo-scope".to_string()).unwrap(),
        fulfills: Vec::new(),
        satisfies: Vec::new(),
        occurred_at: "2026-10-23T15:28:09Z".into(),
        classified_as: vec![
            "run:observation".into(),
            ".".into(),
            "reason:claim".into(),
            "measure:dev-berth-events@1".into(),
            "env:kind=claim".into(),
            "env:resource=dev-integration".into(),
            format!("env:session={session}"),
        ],
    }
}

fn free_port() -> u16 {
    UdpSocket::bind("127.0.0.1:0")
        .and_then(|s| s.local_addr())
        .map(|a| a.port())
        .unwrap()
}

/// Devices A (the chain root) and B (bound), each with an endpoint row naming its peer key, and
/// the roster carried to B's checkout as git would carry it.
fn household() -> (TempDir, Device, Device) {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let b = Device::new(&keys, "b");
    let root = a.checkout.path();
    witness(
        root,
        "human:matthew",
        "agent:orchestrator@claude-opus-5-5",
        "s1",
        "operator of this stewarded device; present in this session",
        false,
        &a.key,
    )
    .unwrap();
    let request = device_enroll(root, HANDLE, &b.key).unwrap();
    let authorization =
        device_authorize(root, &serde_json::to_string(&request).unwrap(), &a.key).unwrap();
    device_bind(
        root,
        &serde_json::to_string(&authorization).unwrap(),
        &b.key,
    )
    .unwrap();
    device_endpoint(root, None, &a.node_id(), &a.key).unwrap();
    device_endpoint(root, None, &b.node_id(), &b.key).unwrap();
    let rel = format!(".eprfs/status/participants/{HANDLE}.jsonl");
    std::fs::create_dir_all(b.checkout.path().join(".eprfs/status/participants")).unwrap();
    std::fs::copy(root.join(&rel), b.checkout.path().join(&rel)).unwrap();
    (keys, a, b)
}

#[tokio::test(flavor = "multi_thread")]
async fn flow_fold_folds_a_devices_channel_root_once_and_status_reads_it() {
    let (keys, a, b) = household();
    let policy = policy(keys.path());
    let allow = CrossingAllowList::from_policy_file(&policy).unwrap();

    // B's own claim, and B's peer serving it.
    let mut b_store = SidecarFlowStore::open(b.checkout.path()).unwrap();
    let claim_cid = b_store.append(FlowRecord::Event(claim("sb"))).unwrap();
    let b_root = project_channel(&b_store, &allow).unwrap().root().clone();
    let port = free_port();
    let b_did = b.key.did_key();
    let b_signer = DeviceKey::load(&b.key_file).unwrap();
    let daemon = PeerDaemon::start(PeerConfig {
        key: PeerKey::load_or_generate(&b.peer_dir()).unwrap(),
        device: b_did.clone(),
        chain_root: a.key.did_key(),
        flow_root: b.checkout.path().to_path_buf(),
        roster: b
            .checkout
            .path()
            .join(format!(".eprfs/status/participants/{HANDLE}.jsonl")),
        state_dir: b.peer_dir(),
        allow,
        relay: None,
        listen: Some(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)),
        peers: Vec::new(),
        signer: Arc::new(move |m: &[u8]| b_signer.sign(m)),
        verifier: Arc::new(|d: &str, m: &[u8], s: &[u8]| device_key::verify(d, m, s)),
        log: Arc::new(|_: &str| {}),
    })
    .await
    .unwrap();
    // The daemon projects on its first tick.
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;

    let peer = format!("{}@127.0.0.1:{port}", b.node_id());
    let root = b_root.to_string();
    let fold = |a: &Device| {
        a.epr(&[
            "flow",
            "fold",
            "--from",
            &b_did,
            "--root",
            &root,
            "--repo",
            a.root(),
            "--peer",
            &peer,
            "--policy",
            policy.to_str().unwrap(),
        ])
    };
    let out = fold(&a);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        out.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("1 appended"),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let folded: Vec<FlowEvent> = SidecarFlowStore::open(a.checkout.path())
        .unwrap()
        .events()
        .unwrap()
        .into_iter()
        .map(|(_, e)| e)
        .filter(|e| via_device(e) == Some(b_did.as_str()))
        .collect();
    assert_eq!(folded.len(), 1);
    assert!(
        eprfs_peer::folded_from(&SidecarFlowStore::open(a.checkout.path()).unwrap(), &b_did)
            .unwrap()
            .contains(&claim_cid)
    );

    // Folding the same root again reads nothing it already holds and appends nothing.
    let again = fold(&a);
    let stdout = String::from_utf8_lossy(&again.stdout).to_string();
    assert!(again.status.success(), "{stdout}");
    assert!(stdout.contains("0 appended"), "{stdout}");
    assert_eq!(
        SidecarFlowStore::open(a.checkout.path())
            .unwrap()
            .events()
            .unwrap()
            .len(),
        1
    );

    daemon.shutdown().await;

    // A folded head in A's table is what status lists for B.
    let mut table = HeadTable::default();
    table.devices.insert(
        b_did.clone(),
        DeviceHead {
            seq: 4,
            root: b_root.clone(),
            at: 1,
            folded: Default::default(),
            folded_at: 1,
            events_folded: 1,
        },
    );
    // Connections A refused: three of B's (say after a Contest), two of a stranger's.
    let stranger_node = "ab".repeat(32);
    table.channel.refused_connections.insert(b.node_id(), 3);
    table
        .channel
        .refused_connections
        .insert(stranger_node.clone(), 2);
    table.save(&a.peer_dir()).unwrap();
    let status = a.epr(&["peer", "status", "--root", a.root(), "--json"]);
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    let devices = json["devices"].as_array().unwrap();
    assert_eq!(devices.len(), 1, "{json}");
    assert_eq!(devices[0]["device"], b_did);
    assert_eq!(devices[0]["seq"], 4);
    assert_eq!(devices[0]["eventsFolded"], 1);
    // Refused connections, by the device the roster names for the node id, else unnamed.
    let mut expected = vec![
        serde_json::json!({"nodeId": stranger_node, "device": null, "count": 2}),
        serde_json::json!({"nodeId": b.node_id(), "device": b_did, "count": 3}),
    ];
    expected.sort_by_key(|row| row["nodeId"].as_str().unwrap_or_default().to_string());
    assert_eq!(
        json["refusedConnections"],
        serde_json::Value::Array(expected),
        "{json}"
    );
    let text = a.epr(&["peer", "status", "--root", a.root()]);
    let text = String::from_utf8_lossy(&text.stdout).to_string();
    assert!(text.contains("head seq 4"), "{text}");
    assert!(text.contains("refused connections"), "{text}");
    assert!(text.contains(": 3"), "{text}");

    // Nothing printed carries the peer key.
    let secret = std::fs::read(a.peer_dir().join(PeerKey::FILE_NAME)).unwrap();
    let hex: String = secret.iter().map(|b| format!("{b:02x}")).collect();
    assert!(!text.contains(&hex) && !String::from_utf8_lossy(&status.stdout).contains(&hex));
}

#[test]
fn flow_fold_refuses_without_its_arguments_or_from_a_stranger() {
    let (keys, a, _b) = household();
    let out = a.epr(&[
        "flow",
        "fold",
        "--root",
        "bafyreiexample",
        "--repo",
        a.root(),
    ]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--from"));

    let stranger = Device::new(&keys, "stranger");
    let out = a.epr(&[
        "flow",
        "fold",
        "--from",
        &stranger.key.did_key(),
        "--root",
        "bafyreigh2akiscaildcqabsyg3dfr6chu3fgpregiymsck7e7aqa4s52zy",
        "--repo",
        a.root(),
        "--policy",
        policy(keys.path()).to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("not an admitted device"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn peer_answers_help_and_refuses_an_unknown_verb() {
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(["peer", "--help"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("usage: epr peer"));
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(["peer", "srve"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn flow_fold_refuses_while_this_devices_peer_serves() {
    let (keys, a, b) = household();
    // `epr peer serve` holds this lock for its lifetime; a fold dialling on the same node id
    // would confuse the other device's path to it.
    let _serving = elohim_epr_cli::peer::ServeLock::acquire(&a.peer_dir()).expect("lock");
    let out = a.epr(&[
        "flow",
        "fold",
        "--from",
        &b.key.did_key(),
        "--root",
        "bafyreigh2akiscaildcqabsyg3dfr6chu3fgpregiymsck7e7aqa4s52zy",
        "--repo",
        a.root(),
        "--peer",
        &format!("{}@127.0.0.1:9", b.node_id()),
        "--policy",
        policy(keys.path()).to_str().unwrap(),
    ]);
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert_eq!(out.status.code(), Some(2), "{stderr}");
    assert!(
        stderr.contains("epr peer serve (or another epr flow fold) is running"),
        "{stderr}"
    );
    assert!(stderr.contains("folds every newer head"), "{stderr}");
    // A second serve is refused the same way.
    assert!(elohim_epr_cli::peer::ServeLock::acquire(&a.peer_dir()).is_err());
}

/// A berth record as T1 folds it, `via` another device when folded from it.
fn berth_record(
    kind: &str,
    resource: &str,
    session: &str,
    at: &str,
    via: Option<&str>,
) -> FlowEvent {
    let mut event = claim(session);
    event.classified_as = vec![
        "run:observation".into(),
        ".".into(),
        format!("reason:{kind}"),
        "measure:dev-berth-events@1".into(),
        format!("env:at={at}"),
        format!("env:claimed_at={at}"),
        format!("env:kind={kind}"),
        format!("env:resource={resource}"),
        format!("env:session={session}"),
    ];
    if let Some(device) = via {
        event.classified_as.push(format!("via:{device}"));
    }
    event
}

fn steward_policy(dir: &Path) -> PathBuf {
    let policy = dir.join("steward-policy.json");
    std::fs::write(
        &policy,
        r#"{"berth": {"resources": {"cargo": 2,
              "dev-integration": {"capacity": 1, "scope": "steward"},
              "lane:": {"capacity": 1, "scope": "steward"}},
            "channel": {"confirm_wait_s": 10,
              "crosses": {"dev-berth-events@1": ["claim", "release"]}}}}"#,
    )
    .unwrap();
    policy
}

fn leases_json(device: &Device, root: &str, policy: &Path) -> serde_json::Value {
    let out = device.epr(&[
        "peer",
        "leases",
        "--json",
        "--root",
        root,
        "--policy",
        policy.to_str().unwrap(),
    ]);
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(out.status.success(), "{stderr}");
    serde_json::from_slice(&out.stdout).unwrap()
}

fn append(root: &Path, events: impl IntoIterator<Item = FlowEvent>) {
    let mut store = SidecarFlowStore::open(root).unwrap();
    for event in events {
        store.append(FlowRecord::Event(event)).unwrap();
    }
}

#[test]
fn peer_leases_prints_the_steward_view_while_serve_holds_its_lock() {
    let (keys, a, b) = household();
    let policy = steward_policy(keys.path());
    // B has folded A's claim on dev-integration; its own cargo claim is container-scoped.
    let a_did = a.key.did_key();
    append(
        b.checkout.path(),
        [
            berth_record(
                "claim",
                "dev-integration",
                "claude-1",
                "1792769289.000",
                Some(&a_did),
            ),
            berth_record("claim", "cargo", "codex-1", "1792769290.000", None),
        ],
    );
    // A moored claude-1 (a mooring names no resource), folded into B's store too.
    let mut moored = berth_record("moor", "", "claude-1", "1792769288.000", Some(&a_did));
    moored
        .classified_as
        .retain(|slot| !slot.starts_with("env:resource=") && !slot.starts_with("env:claimed_at="));
    append(b.checkout.path(), [moored]);
    // The view reads the store and the head table only: it answers while serve runs.
    let _serving = elohim_epr_cli::peer::ServeLock::acquire(&b.peer_dir()).expect("lock");
    let view = leases_json(&b, b.root(), &policy);
    // The other devices, read from B's fold: A, with claude-1 moored and no head folded yet.
    assert_eq!(
        view["others"],
        serde_json::json!({
            a_did.clone(): {
                "seq": null,
                "last_seen": null,
                "moorings": {"claude-1": 1_792_769_288_000u64}
            }
        }),
        "{view}"
    );
    assert!(view["now"].as_u64().is_some(), "{view}");
    assert_eq!(view["confirm_wait_s"], 10);
    assert_eq!(view["standing"], "device");
    assert_eq!(view["device"], b.key.did_key());
    // No daemon has served here yet, so the channel store is the checkout's own.
    assert_eq!(
        view["store"],
        b.checkout.path().canonicalize().unwrap().to_str().unwrap()
    );
    assert_eq!(view["seen"], serde_json::json!({}));
    assert_eq!(
        view["dev-integration"],
        serde_json::json!({
            "verdict": "held_by",
            "device": a_did,
            "session": "claude-1",
            "since": 1_792_769_289_000u64,
            "override": false
        })
    );
    assert!(view.get("cargo").is_none(), "container-scoped: {view}");
}

#[test]
fn peer_leases_reads_the_channel_store_the_daemon_serves_not_the_callers() {
    let (keys, a, b) = household();
    let policy = steward_policy(keys.path());
    let a_did = a.key.did_key();
    // B's daemon serves B's checkout, where A's claim was folded.
    append(
        b.checkout.path(),
        [berth_record(
            "claim",
            "dev-integration",
            "claude-1",
            "1792769289.000",
            Some(&a_did),
        )],
    );
    let served = b.checkout.path().canonicalize().unwrap();
    let mut table = HeadTable {
        store: Some(StorePlace {
            root: served.clone(),
            log: served.join(".eprfs/status/flows.jsonl"),
        }),
        ..HeadTable::default()
    };
    table.save(&b.peer_dir()).unwrap();
    // A worktree of B's with a flow store of its own, holding none of the fold.
    let worktree = checkout();
    append(
        worktree.path(),
        [berth_record(
            "claim",
            "cargo",
            "codex-1",
            "1792769290.000",
            None,
        )],
    );
    let view = leases_json(&b, worktree.path().to_str().unwrap(), &policy);
    assert_eq!(view["store"], served.to_str().unwrap());
    assert_eq!(view["dev-integration"]["verdict"], "held_by", "{view}");
    assert_eq!(view["dev-integration"]["device"], a_did);

    // A recorded channel store that is gone is a refusal, never an empty (free) view, and the
    // view creates nothing in its place.
    let gone = TempDir::new().unwrap();
    table.store = Some(StorePlace {
        root: gone.path().to_path_buf(),
        log: gone.path().join(".eprfs/status/flows.jsonl"),
    });
    table.save(&b.peer_dir()).unwrap();
    let out = b.epr(&[
        "peer",
        "leases",
        "--json",
        "--root",
        b.root(),
        "--policy",
        policy.to_str().unwrap(),
    ]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("channel store"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!gone.path().join(".eprfs").exists());
}

#[test]
fn peer_leases_names_a_workspace_that_is_not_one_of_its_stewards_devices() {
    let (keys, a, b) = household();
    let policy = steward_policy(keys.path());
    let _ = a;
    // No device key.
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args([
            "peer",
            "leases",
            "--json",
            "--root",
            b.root(),
            "--policy",
            policy.to_str().unwrap(),
        ])
        .env(
            device_key::DEVICE_KEY_ENV,
            keys.path().join("absent/ed25519.seed"),
        )
        .env("XDG_CONFIG_HOME", b.config.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let view: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(view["standing"], "not_a_device", "{view}");
    assert!(view["device"].is_null());
    assert!(
        view["why"].as_str().unwrap().contains("no device key"),
        "{view}"
    );
    // A key on no roster in this checkout.
    let stranger = Device::new(&keys, "stranger");
    let view = leases_json(&stranger, b.root(), &policy);
    assert_eq!(view["standing"], "not_a_device", "{view}");
    assert!(view["why"].as_str().unwrap().contains("roster"), "{view}");
}

#[test]
fn peer_leases_reports_a_device_whose_binding_is_contested_as_revoked() {
    let (keys, a, b) = household();
    let policy = steward_policy(keys.path());
    // A contests B's binding, and the row reaches B's checkout.
    let roster = SidecarRoster::open(b.checkout.path(), HANDLE).unwrap();
    let binding_cid = roster
        .read()
        .unwrap()
        .rows()
        .iter()
        .find_map(|(cid, row)| {
            matches!(row, ParticipantRow::Binding { .. }).then(|| cid.to_string())
        })
        .expect("B's binding");
    let mut row = ParticipantRow::Contest {
        handle: HANDLE.into(),
        target_cid: binding_cid,
        by: a.key.did_key(),
        basis: "that workspace is retired".into(),
        signature: String::new(),
    };
    let signature: String = a
        .key
        .sign(&row.signing_message().unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if let ParticipantRow::Contest { signature: s, .. } = &mut row {
        *s = signature;
    }
    roster.append(row).unwrap();
    let view = leases_json(&b, b.root(), &policy);
    assert_eq!(view["standing"], "revoked", "{view}");
    assert_eq!(view["device"], b.key.did_key());
    assert_eq!(view["why"], "this device's binding is contested");
}

#[test]
fn peer_leases_is_effective_at_once_for_a_steward_with_one_device() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "solo");
    let root = a.checkout.path();
    witness(
        root,
        "human:matthew",
        "agent:orchestrator@claude-opus-5-5",
        "s1",
        "operator of this stewarded device; present in this session",
        false,
        &a.key,
    )
    .unwrap();
    device_endpoint(root, None, &a.node_id(), &a.key).unwrap();
    let now_s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    append(
        root,
        [berth_record(
            "claim",
            "lane:solo",
            "claude-1",
            &format!("{now_s:.3}"),
            None,
        )],
    );
    let view = leases_json(&a, a.root(), &steward_policy(keys.path()));
    assert_eq!(
        view["lane:solo"],
        serde_json::json!({"verdict": "effective", "session": "claude-1"}),
        "{view}"
    );
}
