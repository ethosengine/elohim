//! `epr peer` and `epr flow fold`: a device folds another device's channel root from that
//! device's running peer, once, and reads what it folded back through `epr peer status`.
//!
//! Every key is a temp file: each child gets its own `ELOHIM_DEVICE_KEY_FILE` and
//! `XDG_CONFIG_HOME`, so no test touches this workspace's real device or peer key.
#![cfg(feature = "peer")]

use std::io::{BufRead, BufReader};
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use elohim_epr_cli::actor::{
    device_authorize, device_bind, device_endpoint, device_enroll, witness,
};
use elohim_epr_cli::device_key::{self, DeviceKey};
use elohim_epr_rea::{
    atom_cid, AgentRef, FlowEvent, FlowRecord, FlowStore, Intent, Magnitude, ParticipantRow,
    ReaVerb, ResourceSpec, SidecarFlowStore, SidecarRoster,
};
use eprfs_peer::{
    fold_events, project_channel, via_device, ChannelEvent, CrossingAllowList, DeviceHead,
    DeviceRecoveryStatus, FoldCoverage, HeadTable, PeerConfig, PeerDaemon, PeerKey,
    PeerRecoveryStatus, ProbeStatus, Rules, RulesState, StorePlace, RECOVERY_STATUS_FILE,
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

fn git_stdout(root: &Path, args: &[&str]) -> String {
    let out = elohim_epr_cli::process::build_command("git", args, root, &[])
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("git stdout is UTF-8")
        .trim()
        .to_string()
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

    fn command(&self) -> Command {
        command_for(
            &self.key_file,
            self.config.path(),
            &self.config.path().join("epr-store"),
        )
    }

    fn epr(&self, args: &[&str]) -> Output {
        self.command().args(args).output().expect("epr runs")
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

fn command_for(key_file: &Path, config: &Path, store: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
    command
        .env(device_key::DEVICE_KEY_ENV, key_file)
        .env("XDG_CONFIG_HOME", config)
        .env("XDG_CACHE_HOME", config.join("cache"))
        .env("EPR_STORE", store)
        // `epr peer serve` mounts a payload store; never the account's own.
        .env("EPR_PAYLOAD_STORE", config.join("payload"))
        .env("EPR_BIN", env!("CARGO_BIN_EXE_epr"))
        .env_remove("EPR_PEER_RELAY_URL")
        .env_remove("EPR_PEER_RELAYS")
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env_remove("CLAUDE_SESSION_ID")
        .env_remove("ELOHIM_SESSION_ID");
    command
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
    let rules = eprfs_peer::Rules::from_policy_file(&policy).unwrap();

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
        rules,
        policy: Some(policy.clone()),
        relay: None,
        listen: Some(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)),
        peers: Vec::new(),
        signer: Arc::new(move |m: &[u8]| b_signer.sign(m)),
        verifier: Arc::new(|d: &str, m: &[u8], s: &[u8]| device_key::verify(d, m, s)),
        log: Arc::new(|_: &str| {}),
        payload_store: None,
        install: Default::default(),
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
            nack: Default::default(),
            coverage: None,
        },
    );
    // Connections A refused: three of B's (say after a Contest), and strangers' in one total.
    table.channel.refused_connections.insert(b.node_id(), 3);
    table.channel.refused_strangers = 2;
    // The store the daemon served from: status reads the artifact gate's counts from it.
    table.store = Some(StorePlace {
        root: a.checkout.path().to_path_buf(),
        log: a.checkout.path().join(".eprfs/status/flows.jsonl"),
    });
    let status_rules = Rules::from_policy_file(&policy).unwrap();
    let status_digest = status_rules.digest();
    table.rules = Some(RulesState {
        digest: status_digest.clone(),
        policy: policy.clone(),
        loaded_at: 1,
        pending: None,
    });
    for index in 0..eprfs_peer::MAX_CROSSING_DROP_ROWS_PER_RULE_GROUP + 2 {
        table.record_crossing_drop(
            status_digest.as_str(),
            &b_did,
            "dev-landing@1",
            &format!("kind-{index:02}"),
            100 + u64::try_from(index).unwrap(),
        );
    }
    table.record_crossing_drop("rules-old-a", &b_did, "measure", "kind", 10);
    table.record_crossing_drop("rules-old-a", &b_did, "measure", "kind", 11);
    table.record_crossing_drop("rules-old-b", "device-b", "measure", "kind", 20);
    table.record_crossing_drop("rules-old-c", "device-c", "measure", "kind", 30);
    table.save(&a.peer_dir()).unwrap();
    let mut recovery = PeerRecoveryStatus::default();
    recovery.devices.insert(
        b_did.clone(),
        DeviceRecoveryStatus {
            last_heard_at: Some(2_000),
            rejoins_since_head: 3,
            neighbor_up_after_last_join: false,
            last_probe: Some(ProbeStatus {
                ok: true,
                unsupported: true,
                at: 3_000,
            }),
            resubscriptions: 2,
            connection_resets: 1,
        },
    );
    std::fs::write(
        a.peer_dir().join(RECOVERY_STATUS_FILE),
        serde_json::to_vec_pretty(&recovery).unwrap(),
    )
    .unwrap();
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
    assert_eq!(devices[0]["probeOk"], true);
    assert_eq!(devices[0]["probeUnsupported"], true);
    assert!(devices[0]["heardAgeS"].is_u64(), "{json}");
    assert!(devices[0]["foldedAgeS"].is_u64(), "{json}");
    assert_eq!(devices[0]["resubscriptions"], 2);
    assert_eq!(devices[0]["connectionResets"], 1);
    // Refused connections, by the device the roster names for the node id; strangers in a total.
    assert_eq!(
        json["refusedConnections"],
        serde_json::json!([{"nodeId": b.node_id(), "device": b_did, "count": 3}]),
        "{json}"
    );
    assert_eq!(json["refusedStrangers"], 2, "{json}");
    assert_eq!(json["crossingDrops"]["overflow"]["count"], 2, "{json}");
    assert_eq!(
        json["crossingDrops"]["evicted"],
        serde_json::json!([{"device": b_did, "count": 2, "lastSeen": 11}]),
        "{json}"
    );
    // No artifact was served or put in custody here (K12 design §4 exit counts).
    let gate = &json["artifactGate"];
    assert_eq!(gate["reachDraws"], 0, "{json}");
    assert_eq!(gate["rowDraws"], 0, "{json}");
    assert_eq!(gate["slotlessDesignated"], 0, "{json}");
    assert!(gate["since"].is_string(), "{json}");
    let text = a.epr(&["peer", "status", "--root", a.root()]);
    let text = String::from_utf8_lossy(&text.stdout).to_string();
    assert!(text.contains("artifact gate since"), "{text}");
    assert!(text.contains("head seq 4"), "{text}");
    assert!(text.contains("HEARD "), "{text}");
    assert!(text.contains("FOLDED "), "{text}");
    assert!(text.contains("probe unsupported (older peer)"), "{text}");
    assert!(text.contains("resubscribes 2  reset attempts 1"), "{text}");
    assert!(
        text.contains("fold coverage unavailable; sibling events use legacy unbounded replay"),
        "{text}"
    );
    assert!(text.contains("refused connections"), "{text}");
    assert!(text.contains(": 3"), "{text}");
    assert!(text.contains("strangers: 2"), "{text}");
    assert!(
        text.contains("2 additional events beyond the 32 persisted crossing-drop rows"),
        "{text}"
    );
    assert!(
        text.contains(&format!(
            "2 earlier events from {}…{} under evicted rules",
            &b_did[..16],
            &b_did[b_did.len() - 4..]
        )),
        "{text}"
    );

    // The one B event starts at offset 0. A valid zero-length completed prefix leaves it beyond
    // head 4, and status names that incomplete fold instead of counting the event in readers.
    table.devices.get_mut(&b_did).unwrap().coverage = Some(FoldCoverage {
        events: 0,
        log_offset: 0,
    });
    table.save(&a.peer_dir()).unwrap();
    let incomplete = a.epr(&["peer", "status", "--root", a.root()]);
    let incomplete = String::from_utf8_lossy(&incomplete.stdout);
    assert!(
        incomplete.contains("events beyond head 4 not counted: fold incomplete"),
        "{incomplete}"
    );

    // Nothing printed carries the peer key.
    let secret = std::fs::read(a.peer_dir().join(PeerKey::FILE_NAME)).unwrap();
    let hex: String = secret.iter().map(|b| format!("{b:02x}")).collect();
    assert!(!text.contains(&hex) && !String::from_utf8_lossy(&status.stdout).contains(&hex));
}

#[test]
fn peer_status_names_duplicate_sender_records_that_disable_coverage() {
    let (_keys, a, b) = household();
    let b_did = b.key.did_key();
    let mut duplicate = claim("sb");
    duplicate
        .classified_as
        .push(format!("{}{}", eprfs_peer::VIA_SLOT, b_did));
    let mut store = SidecarFlowStore::open(a.checkout.path()).unwrap();
    store.append(FlowRecord::Event(duplicate.clone())).unwrap();
    store.append(FlowRecord::Event(duplicate)).unwrap();
    let log = a.checkout.path().join(".eprfs/status/flows.jsonl");

    let mut table = HeadTable::default();
    table.devices.insert(
        b_did.clone(),
        DeviceHead {
            seq: 1,
            root: eprfs_core::BlobCid::compute(b"duplicate-covered-root"),
            at: 1,
            folded: Default::default(),
            folded_at: 1,
            events_folded: 1,
            nack: Default::default(),
            coverage: Some(FoldCoverage {
                events: 1,
                log_offset: std::fs::metadata(&log).unwrap().len(),
            }),
        },
    );
    table.store = Some(StorePlace {
        root: a.checkout.path().to_path_buf(),
        log,
    });
    table.save(&a.peer_dir()).unwrap();

    let status = a.epr(&["peer", "status", "--root", a.root()]);
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let text = String::from_utf8_lossy(&status.stdout);
    assert!(
        text.contains(
            "coverage does not apply (record count 2 != covered 1 below offset; duplicate or \
             rewritten records): full replay"
        ),
        "{text}"
    );
    assert!(
        !text.contains("fold coverage unavailable; sibling events use legacy unbounded replay"),
        "{text}"
    );
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

#[test]
fn serve_check_takes_no_serve_lock_and_writes_nothing() {
    let (keys, a, _b) = household();
    let policy = policy(keys.path());
    let table = HeadTable {
        store: Some(StorePlace {
            root: a.checkout.path().to_path_buf(),
            log: a.checkout.path().join(".eprfs/status/flows.jsonl"),
        }),
        ..HeadTable::default()
    };
    table.save(&a.peer_dir()).unwrap();
    let heads = a.peer_dir().join(HeadTable::FILE_NAME);
    let heads_before = std::fs::read(&heads).unwrap();
    let modified_before = std::fs::metadata(&heads).unwrap().modified().unwrap();
    let key = a.peer_dir().join(PeerKey::FILE_NAME);
    let key_before = std::fs::read(&key).unwrap();
    let flows = a.checkout.path().join(".eprfs/status/flows.jsonl");
    assert!(!flows.exists());

    let _serving = elohim_epr_cli::peer::ServeLock::acquire(&a.peer_dir()).expect("held lock");
    let out = a.epr(&[
        "peer",
        "serve",
        "--check",
        "--root",
        a.root(),
        "--policy",
        policy.to_str().unwrap(),
        "--json",
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["ok"], true, "{result}");
    assert_eq!(result["flows"], "absent", "{result}");
    assert_eq!(std::fs::read(&heads).unwrap(), heads_before);
    assert_eq!(
        std::fs::metadata(&heads).unwrap().modified().unwrap(),
        modified_before
    );
    assert_eq!(std::fs::read(&key).unwrap(), key_before);
    assert!(!flows.exists(), "check never creates flows.jsonl");
    assert!(
        !a.config.path().join("epr-store/sessions").exists(),
        "check never records a daemon lease"
    );
}

#[test]
fn doctor_exercise_answers_in_one_line_from_anywhere() {
    // Outside any checkout, with no device: the exercise needs nothing but the binary.
    let ok = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(["doctor", "--exercise"])
        .current_dir(std::env::temp_dir())
        .output()
        .expect("epr runs");
    let stdout = String::from_utf8_lossy(&ok.stdout);
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stderr)
    );
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    // The line is made of the answers: a 32-byte agreed secret, the RFC 8032 test 1 signature
    // (its first eight bytes) and the three SQLite rows.
    assert!(
        stdout.starts_with(
            "exercise: ok (x25519 32-byte secret, ed25519 signature e5564300c360ac72, sqlite 3 rows)"
        ),
        "{stdout}"
    );
}

#[test]
fn serve_check_refuses_bad_heads_policy_and_an_off_roster_device() {
    let (keys, a, _b) = household();
    let valid_policy = policy(keys.path());
    std::fs::write(a.peer_dir().join(HeadTable::FILE_NAME), "{bad heads").unwrap();
    let bad_heads = a.epr(&[
        "peer",
        "serve",
        "--check",
        "--root",
        a.root(),
        "--policy",
        valid_policy.to_str().unwrap(),
    ]);
    assert_eq!(bad_heads.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&bad_heads.stderr).contains("heads-invalid"),
        "{}",
        String::from_utf8_lossy(&bad_heads.stderr)
    );

    HeadTable::default().save(&a.peer_dir()).unwrap();
    let bad_policy = keys.path().join("bad-policy.json");
    std::fs::write(&bad_policy, "{bad policy").unwrap();
    let refused = a.epr(&[
        "peer",
        "serve",
        "--check",
        "--root",
        a.root(),
        "--policy",
        bad_policy.to_str().unwrap(),
    ]);
    assert_eq!(refused.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("policy-invalid"),
        "{}",
        String::from_utf8_lossy(&refused.stderr)
    );

    let stranger = Device::new(&keys, "off-roster");
    let off_roster = stranger.epr(&[
        "peer",
        "serve",
        "--check",
        "--root",
        a.root(),
        "--policy",
        valid_policy.to_str().unwrap(),
    ]);
    assert_eq!(off_roster.status.code(), Some(3));
    assert!(
        String::from_utf8_lossy(&off_roster.stderr).contains("not-a-device"),
        "{}",
        String::from_utf8_lossy(&off_roster.stderr)
    );
}

#[test]
fn serve_check_blocks_malformed_or_inconsistent_device_state() {
    let (keys, a, b) = household();
    let policy = policy(keys.path());
    let roster = a
        .checkout
        .path()
        .join(format!(".eprfs/status/participants/{HANDLE}.jsonl"));
    let valid_roster = std::fs::read(&roster).unwrap();
    std::fs::write(&roster, "{malformed roster").unwrap();

    let malformed = a.epr(&[
        "peer",
        "serve",
        "--check",
        "--root",
        a.root(),
        "--handle",
        HANDLE,
        "--policy",
        policy.to_str().unwrap(),
    ]);
    assert_eq!(malformed.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&malformed.stderr).contains("invalid-device-state"),
        "{}",
        String::from_utf8_lossy(&malformed.stderr)
    );
    let malformed_without_handle = a.epr(&[
        "peer",
        "serve",
        "--check",
        "--root",
        a.root(),
        "--policy",
        policy.to_str().unwrap(),
    ]);
    assert_eq!(malformed_without_handle.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&malformed_without_handle.stderr).contains("invalid-device-state"),
        "{}",
        String::from_utf8_lossy(&malformed_without_handle.stderr)
    );

    let unterminated = valid_roster
        .strip_suffix(b"\n")
        .expect("fixture roster ends with a record terminator");
    std::fs::write(&roster, unterminated).unwrap();
    let unterminated_without_handle = a.epr(&[
        "peer",
        "serve",
        "--check",
        "--root",
        a.root(),
        "--policy",
        policy.to_str().unwrap(),
    ]);
    assert_eq!(unterminated_without_handle.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&unterminated_without_handle.stderr);
    assert!(stderr.contains("invalid-device-state"), "{stderr}");
    assert!(stderr.contains("unterminated record"), "{stderr}");

    std::fs::write(&roster, valid_roster).unwrap();
    device_key::write_root_pin(&a.key_file, HANDLE, &b.key.did_key()).unwrap();
    let inconsistent = a.epr(&[
        "peer",
        "serve",
        "--check",
        "--root",
        a.root(),
        "--handle",
        HANDLE,
        "--policy",
        policy.to_str().unwrap(),
    ]);
    assert_eq!(inconsistent.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&inconsistent.stderr).contains("invalid-device-state"),
        "{}",
        String::from_utf8_lossy(&inconsistent.stderr)
    );
}

#[test]
fn serve_check_reports_rules_skew_without_failing() {
    let (keys, a, _b) = household();
    let loaded_policy = policy(keys.path());
    let checked_policy = keys.path().join("checked-policy.json");
    std::fs::write(
        &checked_policy,
        r#"{"berth":{"resources":{"lane:":{"capacity":1,"scope":"steward"}},"channel":{"crosses":{"gate-verdict@1":["verdict"]}}}}"#,
    )
    .unwrap();
    let table = HeadTable {
        rules: Some(RulesState {
            digest: Rules::from_policy_file(&loaded_policy).unwrap().digest(),
            policy: loaded_policy,
            loaded_at: 1,
            pending: None,
        }),
        ..HeadTable::default()
    };
    table.save(&a.peer_dir()).unwrap();

    let out = a.epr(&[
        "peer",
        "serve",
        "--check",
        "--root",
        a.root(),
        "--policy",
        checked_policy.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("rules: skew"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn refused_start_writes_no_daemon_lease_or_stranger_peer_key() {
    let (keys, a, _b) = household();
    let policy = policy(keys.path());
    let empty_config = TempDir::new().unwrap();
    let empty_store = empty_config.path().join("store");
    let checked = command_for(&a.key_file, empty_config.path(), &empty_store)
        .args([
            "peer",
            "serve",
            "--check",
            "--root",
            a.root(),
            "--policy",
            policy.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(checked.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&checked.stderr).contains("peer-key-mismatch"),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert!(!empty_config.path().join("epr/peer/iroh.key").exists());
    assert!(!empty_store.join("sessions").exists());

    let out = command_for(&a.key_file, empty_config.path(), &empty_store)
        .args([
            "peer",
            "serve",
            "--root",
            a.root(),
            "--policy",
            policy.to_str().unwrap(),
            "--listen",
            "127.0.0.1:0",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("peer-key-mismatch"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !empty_config.path().join("epr/peer/iroh.key").exists(),
        "serve never mints a key behind the roster"
    );
    assert!(
        !empty_store.join("sessions").exists(),
        "the key guard runs before record_running_daemon"
    );
}

#[test]
fn no_handle_refuses_clearly_when_the_device_stands_on_two_rosters() {
    let (keys, a, _b) = household();
    witness(
        a.checkout.path(),
        "human:ronald",
        "agent:orchestrator@claude-opus-5-5",
        "s2",
        "operator of this second fixture roster; present in this session",
        false,
        &a.key,
    )
    .unwrap();
    let policy = policy(keys.path());
    let out = a.epr(&[
        "peer",
        "serve",
        "--check",
        "--root",
        a.root(),
        "--policy",
        policy.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("invalid-device-state"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("more than one participant roster"));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--handle"));
}

fn spawn_serve(device: &Device, policy: &Path) -> Child {
    device
        .command()
        .args([
            "peer",
            "serve",
            "--root",
            device.root(),
            "--policy",
            policy.to_str().unwrap(),
            "--listen",
            "127.0.0.1:0",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("serve starts")
}

fn wait_for_serving(child: &mut Child) -> String {
    let stdout = child.stdout.take().expect("piped stdout");
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
        let _ = send.send(result);
    });
    receive
        .recv_timeout(Duration::from_secs(10))
        .expect("serving line within 10 seconds")
        .expect("read serving line")
}

fn signal_and_wait(child: &mut Child, signal: i32) {
    // SAFETY: `child.id()` is the live child started by this test, and kill does not retain the pid.
    let sent = unsafe { libc::kill(child.id() as i32, signal) };
    assert_eq!(sent, 0, "send signal {signal}");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().expect("wait for serve") {
            assert!(status.success(), "serve exits cleanly on signal {signal}");
            return;
        }
        if Instant::now() >= deadline {
            child.kill().expect("kill timed-out serve");
            panic!("serve did not stop on signal {signal}");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn the_serving_line_is_on_stdout_and_sigterm_sigint_stop_cleanly() {
    let (keys, a, _b) = household();
    let policy = policy(keys.path());
    for signal in [libc::SIGTERM, libc::SIGINT] {
        let mut child = spawn_serve(&a, &policy);
        let line = wait_for_serving(&mut child);
        assert!(
            line.starts_with(elohim_epr_cli::peer::SERVING_LINE),
            "stdout readiness line: {line:?}"
        );
        signal_and_wait(&mut child, signal);
    }
}

/// Every daemon lease in a device's epr store, with its bytes, in name order.
fn daemon_leases(device: &Device) -> Vec<(PathBuf, Vec<u8>)> {
    let dir = device.config.path().join("epr-store/sessions");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut leases: Vec<_> = entries
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("daemon-"))
        })
        .map(|path| {
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    leases.sort();
    leases
}

/// The refusal's stable first line exactly as `epr` prints it.
fn refusal_line() -> String {
    format!("epr peer: {}", elohim_epr_cli::peer::SERVE_LOCK_HELD_LINE)
}

/// One losing serve: exits the lock-held code, announces nothing, and returns its stderr.
fn lose(command: &mut Command) -> String {
    let out = command.output().expect("epr runs");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(
        out.status.code(),
        Some(i32::from(elohim_epr_cli::peer::EXIT_SERVE_LOCK_HELD)),
        "{stderr}"
    );
    assert!(
        out.stdout.is_empty(),
        "a refused serve never announces itself"
    );
    stderr
}

fn serve_command(device: &Device, policy: &Path) -> Command {
    let mut command = device.command();
    command.args([
        "peer",
        "serve",
        "--root",
        device.root(),
        "--policy",
        policy.to_str().unwrap(),
        "--listen",
        "127.0.0.1:0",
    ]);
    command
}

#[test]
fn a_second_serve_exits_lock_held_naming_the_holder_and_leaves_its_lease_intact() {
    let (keys, a, _b) = household();
    let policy = policy(keys.path());
    let mut first = spawn_serve(&a, &policy);
    wait_for_serving(&mut first);
    let before = daemon_leases(&a);
    assert_eq!(before.len(), 1, "the holder records one lease: {before:?}");
    let holder_lease = before[0].0.file_name().unwrap().to_string_lossy();
    assert!(
        holder_lease.starts_with(&format!("daemon-{}-", first.id())),
        "{holder_lease}"
    );
    let lock_file = a
        .peer_dir()
        .join(elohim_epr_cli::peer::ServeLock::FILE_NAME);
    let holder_record = std::fs::read(&lock_file).unwrap();
    assert!(
        String::from_utf8_lossy(&holder_record).starts_with(&format!("{} ", first.id())),
        "the holder records itself: {:?}",
        String::from_utf8_lossy(&holder_record)
    );

    for _ in 0..2 {
        let stderr = lose(&mut serve_command(&a, &policy));
        let mut lines = stderr.lines();
        // Line 1 never varies with the holder: it is what a supervisor's same-cause key reads.
        assert_eq!(lines.next(), Some(refusal_line().as_str()), "{stderr}");
        let holder = lines.next().unwrap_or_default();
        assert!(
            holder.contains(&format!("held by pid {}", first.id())),
            "{stderr}"
        );
        assert_eq!(daemon_leases(&a), before, "the loser writes no lease");
        assert_eq!(
            std::fs::read(&lock_file).unwrap(),
            holder_record,
            "the loser never rewrites the holder's record"
        );
    }

    signal_and_wait(&mut first, libc::SIGTERM);
}

#[test]
fn a_serve_against_a_lock_held_without_a_recorded_holder_writes_no_lease() {
    let (keys, a, _b) = household();
    let policy = policy(keys.path());
    let _held = elohim_epr_cli::peer::ServeLock::acquire(&a.peer_dir()).expect("lock");

    let stderr = lose(&mut serve_command(&a, &policy));

    let mut lines = stderr.lines();
    assert_eq!(
        lines.next(),
        Some(refusal_line().as_str()),
        "the same first line as a loss to a recorded holder: {stderr}"
    );
    assert!(
        lines.next().unwrap_or_default().contains("recorded no pid"),
        "{stderr}"
    );
    assert!(
        !a.config.path().join("epr-store/sessions").exists(),
        "a lost lock records no daemon lease"
    );
}

#[test]
fn relay_notices_never_precede_the_lock_refusal() {
    let (keys, a, _b) = household();
    let policy = policy(keys.path());
    let _held = elohim_epr_cli::peer::ServeLock::acquire(&a.peer_dir()).expect("lock");

    // Each relay source prints its own notice when serve reaches it: the THIRD-PARTY n0 line,
    // the legacy-variable deprecation, and (with neither set) the absent-Rulebook default.
    for (key, value) in [
        ("EPR_PEER_RELAYS", Some("n0")),
        ("EPR_PEER_RELAY_URL", Some("https://relay.example.test")),
        ("EPR_PEER_RELAYS", None),
    ] {
        let mut command = serve_command(&a, &policy);
        if let Some(value) = value {
            command.env(key, value);
        }
        let stderr = lose(&mut command);
        assert_eq!(
            stderr.lines().next(),
            Some(refusal_line().as_str()),
            "{key}={value:?}: {stderr}"
        );
        assert!(!stderr.contains("relay"), "{key}={value:?}: {stderr}");
    }
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

fn gate_verdict_record(
    project: &str,
    closure: &str,
    commit: &str,
    success: bool,
    via: Option<&str>,
) -> FlowEvent {
    let mut event = claim("gate");
    event.classified_as = vec![
        "run:observation".into(),
        ".".into(),
        "reason:verdict".into(),
        "measure:gate-verdict@1".into(),
        format!("env:project={project}"),
        format!("env:closure={closure}"),
        format!("env:commit={commit}"),
        format!("env:success={success}"),
        "env:seconds=12".into(),
        "env:attestationId=bafyattestation".into(),
        "env:agentId=agent-key".into(),
    ];
    if let Some(device) = via {
        event.classified_as.push(format!("via:{device}"));
    }
    event
}

fn attestation_enrichment_record(device: &str) -> FlowEvent {
    let mut event = claim("enrichment");
    event.resource = atom_cid(&"enriched-verdict".to_string()).unwrap();
    event.provider = AgentRef(device.to_string());
    event.classified_as = vec![
        "run:observation".into(),
        ".".into(),
        "reason:re-executed".into(),
        "measure:attestation-enrichment@1".into(),
        "value:1".into(),
        "unit:enrichments".into(),
        "env:closure=closure-a".into(),
        "env:commit=commit-a".into(),
        format!("env:device={device}"),
        "env:kind=re-executed".into(),
        "env:method=gate-v3/judge-v1".into(),
        "env:outcome=green".into(),
        "env:project=eprfs".into(),
        "env:verdict=bafyverdict".into(),
    ];
    event
}

fn landing_record(
    kind: &str,
    intent: &str,
    change: &str,
    env: &[(&str, &str)],
    via: Option<&str>,
) -> FlowEvent {
    let mut event = claim("landing");
    event.action = match kind {
        "landed" => ReaVerb::Produce,
        "dismiss" => ReaVerb::Dismiss,
        _ => ReaVerb::Cite,
    };
    event.resource = intent.parse().unwrap();
    event.quantity = Magnitude::Count {
        value: 1.0,
        unit: "dev-landing".into(),
    };
    event.classified_as = vec![
        format!("dev-landing:{kind}"),
        change.into(),
        format!("reason:{kind}"),
        "measure:dev-landing@1".into(),
        "value:1".into(),
        "unit:landings".into(),
    ];
    event
        .classified_as
        .extend(env.iter().map(|(key, value)| format!("env:{key}={value}")));
    if let Some(device) = via {
        event.classified_as.push(format!("via:{device}"));
    }
    event
}

fn intent_atom(
    root: &Path,
    change: &str,
    tree: &str,
    base: &str,
    closures: &[(&str, &str)],
    class: &str,
    supersedes: Option<&str>,
) -> String {
    let mut slots = vec![
        "dev-landing:intent".into(),
        change.into(),
        format!("tree:{tree}"),
        format!("base:{base}"),
        format!("class:{class}"),
    ];
    slots.extend(
        closures
            .iter()
            .map(|(project, closure)| format!("closure:{project}={closure}")),
    );
    if let Some(prior) = supersedes {
        slots.push(format!("supersedes:{prior}"));
    }
    let atom = FlowRecord::Intent(Intent {
        action: ReaVerb::Produce,
        resource_spec: ResourceSpec {
            classified_as: slots,
            quantity: None,
        },
        in_scope_of: elohim_epr_cli::flow::repository_scope(root).unwrap(),
        raised_by: AgentRef("human:fixture".into()),
    });
    let cid = atom.cid().unwrap();
    let mut store = SidecarFlowStore::open(root).unwrap();
    store.append(atom).unwrap();
    store
        .append(FlowRecord::Event(landing_record(
            "raise",
            &cid.to_string(),
            change,
            &[
                ("at", "1792769289.123"),
                ("chain_len", "0"),
                ("change", change),
                ("dseq", "1"),
                ("predicted_base", "dev"),
                ("tree", tree),
            ],
            None,
        )))
        .unwrap();
    cid.to_string()
}

fn steward_policy(dir: &Path) -> PathBuf {
    let policy = dir.join("steward-policy.json");
    std::fs::write(
        &policy,
        r#"{"berth": {"resources": {"cargo": 2,
              "dev-integration": {"capacity": 1, "scope": "steward"},
              "lane:": {"capacity": 1, "scope": "steward"}},
            "channel": {"confirm_wait_s": 10,
              "crosses": {"dev-berth-events@1": ["claim", "release"],
                "dev-landing@1": ["intent", "landed", "withdraw", "dismiss", "misses", "carry"],
                "gate-verdict@1": ["verdict"]}}}}"#,
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
fn peer_leases_never_observes_release_without_replacement_claim_batch() {
    let (keys, a, b) = household();
    let policy = steward_policy(keys.path());
    let a_did = a.key.did_key();
    let mut moored = berth_record("moor", "", "claude-1", "1792769288.000", Some(&a_did));
    moored
        .classified_as
        .retain(|slot| !slot.starts_with("env:resource=") && !slot.starts_with("env:claimed_at="));
    append(
        b.checkout.path(),
        [
            moored,
            berth_record(
                "claim",
                "dev-integration",
                "claude-1",
                "1792769289.000",
                Some(&a_did),
            ),
        ],
    );

    let initial = leases_json(&b, b.root(), &policy);
    assert_eq!(initial["dev-integration"]["session"], "claude-1");

    let root = b.checkout.path().to_path_buf();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (go_tx, go_rx) = std::sync::mpsc::channel();
    let writer = std::thread::spawn(move || {
        let padding = "x".repeat(8 * 1024 * 1024);
        let mut release = berth_record(
            "release",
            "dev-integration",
            "claude-1",
            "1792769290.000",
            Some(&a_did),
        );
        release
            .classified_as
            .retain(|slot| !slot.starts_with("env:claimed_at="));
        release
            .classified_as
            .push("env:claimed_at=1792769289.000".into());
        release
            .classified_as
            .push(format!("test-padding:{padding}"));
        let mut replacement = berth_record(
            "claim",
            "dev-integration",
            "claude-2",
            "1792769291.000",
            Some(&a_did),
        );
        replacement
            .classified_as
            .push(format!("test-padding:{padding}"));
        ready_tx.send(()).unwrap();
        go_rx.recv().unwrap();
        SidecarFlowStore::open(&root)
            .unwrap()
            .transaction()
            .unwrap()
            .append_batch(vec![
                FlowRecord::Event(release),
                FlowRecord::Event(replacement),
            ])
            .unwrap();
    });
    ready_rx.recv().unwrap();
    go_tx.send(()).unwrap();

    let mut samples = 0;
    while !writer.is_finished() {
        let view = leases_json(&b, b.root(), &policy);
        let session = view["dev-integration"]["session"].as_str();
        assert!(
            matches!(session, Some("claude-1" | "claude-2")),
            "lease reader observed a released-but-not-reclaimed batch prefix: {view}"
        );
        samples += 1;
    }
    writer.join().unwrap();
    assert!(samples > 0, "reader process must overlap the writer");
    let final_view = leases_json(&b, b.root(), &policy);
    assert_eq!(final_view["dev-integration"]["session"], "claude-2");
}

#[test]
fn peer_verdicts_reads_only_matching_folded_admitted_siblings() {
    let (keys, a, b) = household();
    let a_did = a.key.did_key();
    let stranger = Device::new(&keys, "stranger-verdict");
    // A's record naming closure-x and then closure-a: inconsistent with itself, so no verdict
    // (a last-value-wins read would have matched closure-a).
    let mut twice = gate_verdict_record("eprfs", "closure-x", "twice", true, Some(&a_did));
    twice.classified_as.push("env:closure=closure-a".into());
    append(b.checkout.path(), [twice]);
    let mut observed = gate_verdict_record("eprfs", "closure-a", "commit-a", true, Some(&a_did));
    observed
        .classified_as
        .push("env:observedAt=2026-10-23T16:30:45Z".into());
    append(
        b.checkout.path(),
        [
            observed,
            gate_verdict_record("eprfs", "closure-a", "commit-legacy", true, Some(&a_did)),
            gate_verdict_record("eprfs", "closure-b", "commit-b", false, Some(&a_did)),
            gate_verdict_record("eprfs", "closure-a", "own", false, None),
            gate_verdict_record(
                "eprfs",
                "closure-a",
                "stranger",
                false,
                Some(&stranger.key.did_key()),
            ),
        ],
    );
    let out = b.epr(&[
        "peer",
        "verdicts",
        "--project",
        "eprfs",
        "--closure",
        "closure-a",
        "--json",
        "--root",
        b.root(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rows: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        rows,
        serde_json::json!([
            {
                "device": a_did.clone(),
                "project": "eprfs",
                "closure": "closure-a",
                "commit": "commit-a",
                "success": true,
                "seconds": 12.0,
                "attestationId": "bafyattestation",
                "agentId": "agent-key",
                "at": "2026-10-23T15:28:09Z",
                "observedAt": "2026-10-23T16:30:45Z"
            },
            {
                "device": a_did,
                "project": "eprfs",
                "closure": "closure-a",
                "commit": "commit-legacy",
                "success": true,
                "seconds": 12.0,
                "attestationId": "bafyattestation",
                "agentId": "agent-key",
                "at": "2026-10-23T15:28:09Z",
                "observedAt": null
            }
        ])
    );

    let human = b.epr(&[
        "peer",
        "verdicts",
        "--project",
        "eprfs",
        "--closure",
        "closure-a",
        "--root",
        b.root(),
    ]);
    assert!(human.status.success());
    let text = String::from_utf8_lossy(&human.stdout);
    assert!(
        text.contains("judged commit date: 2026-10-23T15:28:09Z"),
        "{text}"
    );
    assert!(text.contains("observed: 2026-10-23T16:30:45Z"), "{text}");
    assert!(text.contains("observed: unknown"), "{text}");

    // A local enrichment is a separate measure and cannot alter the sibling-verdict projection.
    append(
        b.checkout.path(),
        [attestation_enrichment_record(&b.key.did_key())],
    );
    let after = b.epr(&[
        "peer",
        "verdicts",
        "--project",
        "eprfs",
        "--closure",
        "closure-a",
        "--json",
        "--root",
        b.root(),
    ]);
    assert!(
        after.status.success(),
        "{}",
        String::from_utf8_lossy(&after.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&after.stdout).unwrap(),
        rows
    );
}

#[test]
fn peer_leases_says_whether_this_devices_daemon_is_serving() {
    let (keys, _a, b) = household();
    let policy = steward_policy(keys.path());
    append(
        b.checkout.path(),
        [berth_record(
            "claim",
            "cargo",
            "codex-1",
            "1792769290.000",
            None,
        )],
    );
    // No daemon: an unconfirmed claim's cure is this device's own `epr peer serve`.
    let view = leases_json(&b, b.root(), &policy);
    assert_eq!(view["serving"], false, "{view}");
    // The probe took nothing it kept: a daemon can still take the lock after it.
    let serving = elohim_epr_cli::peer::ServeLock::acquire(&b.peer_dir()).expect("lock");
    let view = leases_json(&b, b.root(), &policy);
    assert_eq!(view["serving"], true, "{view}");
    drop(serving);
    assert_eq!(leases_json(&b, b.root(), &policy)["serving"], false);
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

const LANDING_CHANGE: &str = "I0123456789abcdef0123456789abcdef01234567";
const LANDING_TREE: &str = "0123456789abcdef0123456789abcdef01234567";
const LANDING_BASE: &str = "1123456789abcdef0123456789abcdef01234567";

fn tree_listing(root: &Path) -> Vec<PathBuf> {
    fn visit(root: &Path, at: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(at) else {
            return;
        };
        let mut entries: Vec<_> = entries.map(Result::unwrap).collect();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            out.push(path.strip_prefix(root).unwrap().to_path_buf());
            if path.is_dir() {
                visit(root, &path, out);
            }
        }
    }
    let mut paths = Vec::new();
    visit(root, root, &mut paths);
    paths
}

fn save_served_table(device: &Device, root: &Path, sibling: Option<(&str, u64)>) {
    let mut table = HeadTable {
        store: Some(StorePlace {
            root: root.canonicalize().unwrap(),
            log: root.join(".eprfs/status/flows.jsonl"),
        }),
        ..HeadTable::default()
    };
    if let Some((did, folded_at)) = sibling {
        table.devices.insert(
            did.to_string(),
            DeviceHead {
                seq: 1,
                root: project_channel(
                    &SidecarFlowStore::open(root).unwrap(),
                    &CrossingAllowList::new([(
                        "dev-landing@1",
                        ["intent", "landed", "withdraw", "dismiss", "misses", "carry"],
                    )]),
                )
                .unwrap()
                .root()
                .clone(),
                at: folded_at - 1,
                folded: Default::default(),
                folded_at,
                events_folded: 1,
                nack: Default::default(),
                coverage: None,
            },
        );
    }
    table.save(&device.peer_dir()).unwrap();
}

fn intents_output(device: &Device, root: &Path, policy: &Path, all: bool) -> Output {
    let before = tree_listing(root);
    let mut args = vec![
        "peer",
        "intents",
        "--json",
        "--root",
        root.to_str().unwrap(),
        "--policy",
        policy.to_str().unwrap(),
    ];
    if all {
        args.push("--all");
    }
    let output = device.epr(&args);
    assert_eq!(
        tree_listing(root),
        before,
        "peer intents changed the root tree"
    );
    output
}

fn intents_json(device: &Device, root: &Path, policy: &Path, all: bool) -> serde_json::Value {
    let output = intents_output(device, root, policy, all);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn intents_misses_json(device: &Device, root: &Path, policy: &Path) -> serde_json::Value {
    let before = tree_listing(root);
    let output = device.epr(&[
        "peer",
        "intents",
        "--misses",
        "--json",
        "--root",
        root.to_str().unwrap(),
        "--policy",
        policy.to_str().unwrap(),
    ]);
    assert_eq!(
        tree_listing(root),
        before,
        "peer intents --misses changed the root tree"
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn intents_misses_text(device: &Device, root: &Path, policy: &Path) -> String {
    let before = tree_listing(root);
    let output = device.epr(&[
        "peer",
        "intents",
        "--misses",
        "--root",
        root.to_str().unwrap(),
        "--policy",
        policy.to_str().unwrap(),
    ]);
    assert_eq!(
        tree_listing(root),
        before,
        "peer intents --misses changed the root tree"
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn misses_record(intent: &str, closure: &str, dseq: &str, via: &str) -> FlowEvent {
    let miss = format!("eprfs={closure}");
    landing_record(
        "misses",
        intent,
        LANDING_CHANGE,
        &[
            ("at", "1792769289.123"),
            ("base", LANDING_BASE),
            ("change", LANDING_CHANGE),
            ("commit", LANDING_TREE),
            ("dseq", dseq),
            ("miss", &miss),
            ("p90", "eprfs=42"),
            ("ref", "refs/heads/self/f4-ask"),
            ("tree", LANDING_TREE),
            ("wait_until", "4102444800"),
        ],
        Some(via),
    )
}

fn carry_record(
    intent: &str,
    closure: &str,
    miss: &str,
    valid_until: u64,
    dseq: &str,
    via: &str,
) -> FlowEvent {
    let commitment = atom_cid(&format!("commitment-{closure}-{valid_until}-{dseq}")).unwrap();
    let valid_until = valid_until.to_string();
    landing_record(
        "carry",
        intent,
        LANDING_CHANGE,
        &[
            ("at", "1792769290.123"),
            ("closure", closure),
            ("commitment", &commitment.to_string()),
            ("dseq", dseq),
            ("miss", miss),
            ("project", "eprfs"),
            ("valid_until", &valid_until),
        ],
        Some(via),
    )
}

fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn rfc3339(unix_s: u64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(i64::try_from(unix_s).unwrap(), 0)
        .unwrap()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn short_device(did: &str) -> String {
    format!("{}…{}", &did[..16], &did[did.len() - 4..])
}

fn announcement(intent: &str, via: Option<&str>, dseq: &str) -> FlowEvent {
    landing_record(
        "intent",
        intent,
        LANDING_CHANGE,
        &[
            ("at", "1792769289.123"),
            ("base", LANDING_BASE),
            ("chain_len", "2"),
            ("change", LANDING_CHANGE),
            ("class", "low"),
            ("closure", "eprfs=abcdef"),
            ("dseq", dseq),
            ("predicted_base", LANDING_BASE),
            ("tree", LANDING_TREE),
            ("verified_at", "1792769290.456"),
        ],
        via,
    )
}

#[test]
fn peer_intents_joins_own_atoms_and_sibling_cites_by_intent_cid() {
    let (keys, a, b) = household();
    let policy = steward_policy(keys.path());
    let own = intent_atom(
        b.checkout.path(),
        LANDING_CHANGE,
        LANDING_TREE,
        LANDING_BASE,
        &[("eprfs", "abcdef")],
        "low",
        None,
    );
    let sibling = atom_cid(&"sibling-intent".to_string()).unwrap().to_string();
    let stranger = Device::new(&keys, "stranger-intent");
    let stranger_intent = atom_cid(&"stranger-intent".to_string())
        .unwrap()
        .to_string();
    append(
        b.checkout.path(),
        [
            announcement(&own, None, "1"),
            announcement(&sibling, Some(&a.key.did_key()), "7"),
            announcement(&stranger_intent, Some(&stranger.key.did_key()), "9"),
        ],
    );
    save_served_table(
        &b,
        b.checkout.path(),
        Some((&a.key.did_key(), 1_792_769_291_000)),
    );
    let view = intents_json(&b, b.checkout.path(), &policy, false);
    let rows = view["intents"].as_array().unwrap();
    assert_eq!(rows.len(), 2, "{view}");
    let own_row = rows.iter().find(|row| row["intent"] == own).unwrap();
    assert_eq!(own_row["own"], true);
    assert_eq!(own_row["verified"], true);
    assert_eq!(own_row["raisedAt"], 1_792_769_289_123u64);
    assert_eq!(own_row["verifiedAt"], 1_792_769_290_456u64);
    assert_eq!(own_row["dseq"], 1);
    let sibling_row = rows.iter().find(|row| row["intent"] == sibling).unwrap();
    assert_eq!(sibling_row["own"], false);
    assert_eq!(sibling_row["announcer"], a.key.did_key());
    assert_eq!(sibling_row["deviceFoldedAt"], 1_792_769_291_000u64);
    assert!(!view.to_string().contains(&stranger_intent));
}

#[test]
fn peer_intents_lists_a_crossed_miss_until_a_crossed_verdict_settles_it() {
    let (keys, a, b) = household();
    let policy = steward_policy(keys.path());
    let allow = CrossingAllowList::from_policy_file(&policy).unwrap();
    let intent = atom_cid(&"sibling-misses-intent".to_string()).unwrap();
    let miss = landing_record(
        "misses",
        &intent.to_string(),
        LANDING_CHANGE,
        &[
            ("at", "1792769289.123"),
            ("base", LANDING_BASE),
            ("change", LANDING_CHANGE),
            ("commit", LANDING_TREE),
            ("dseq", "7"),
            ("miss", "eprfs=abcdef"),
            ("p90", "eprfs=42"),
            ("ref", "refs/heads/self/f4-ask"),
            ("tree", LANDING_TREE),
            ("wait_until", "4102444800"),
        ],
        None,
    );
    let miss_channel = ChannelEvent {
        cid: atom_cid(&miss).unwrap(),
        event: miss,
    };
    let store = SidecarFlowStore::open(b.checkout.path()).unwrap();
    let folded = fold_events(
        &store,
        &a.key.did_key(),
        std::slice::from_ref(&miss_channel),
        &allow,
    )
    .unwrap();
    assert_eq!(folded.appended, 1);
    save_served_table(
        &b,
        b.checkout.path(),
        Some((&a.key.did_key(), 1_792_769_291_000)),
    );

    let misses = intents_misses_json(&b, b.checkout.path(), &policy);

    assert_eq!(misses.as_array().unwrap().len(), 1, "{misses}");
    assert_eq!(misses[0]["intent"], intent.to_string());
    assert_eq!(misses[0]["announcer"], a.key.did_key());
    assert_eq!(misses[0]["project"], "eprfs");
    assert_eq!(misses[0]["closure"], "abcdef");
    assert_eq!(misses[0]["p90S"], 42);
    assert_eq!(
        intents_json(&b, b.checkout.path(), &policy, false)["misses"],
        misses
    );

    let verdict = gate_verdict_record("eprfs", "abcdef", LANDING_TREE, false, None);
    let verdict_channel = ChannelEvent {
        cid: atom_cid(&verdict).unwrap(),
        event: verdict,
    };
    let folded = fold_events(
        &store,
        &a.key.did_key(),
        std::slice::from_ref(&verdict_channel),
        &allow,
    )
    .unwrap();
    assert_eq!(folded.appended, 1);

    assert_eq!(
        intents_misses_json(&b, b.checkout.path(), &policy),
        serde_json::json!([])
    );
}

#[test]
fn peer_intents_does_not_offer_a_miss_whose_commit_landed_on_origin_dev() {
    let (keys, _a, b) = household();
    let policy = steward_policy(keys.path());
    let commit = git_stdout(b.checkout.path(), &["rev-parse", "HEAD"]);
    git(
        b.checkout.path(),
        &["update-ref", "refs/remotes/origin/dev", &commit],
    );
    let intent = atom_cid(&"landed-miss-intent".to_string()).unwrap();
    append(
        b.checkout.path(),
        [landing_record(
            "misses",
            &intent.to_string(),
            LANDING_CHANGE,
            &[
                ("at", "1792769289.123"),
                ("base", LANDING_BASE),
                ("change", LANDING_CHANGE),
                ("commit", &commit),
                ("dseq", "7"),
                ("miss", "eprfs=abcdef"),
                ("p90", "eprfs=42"),
                ("ref", "refs/heads/dev"),
                ("tree", &commit),
                ("wait_until", "4102444800"),
            ],
            None,
        )],
    );

    let view = intents_json(&b, b.checkout.path(), &policy, false);

    assert_eq!(view["misses"], serde_json::json!([]), "{view}");
    assert_eq!(view["settledMisses"].as_array().unwrap().len(), 1, "{view}");
    assert_eq!(view["settledMisses"][0]["by"], "landing", "{view}");
    assert_eq!(view["settledMisses"][0]["miss"]["commit"], commit, "{view}");
    assert_eq!(
        intents_misses_json(&b, b.checkout.path(), &policy),
        serde_json::json!([])
    );
    let text = intents_misses_text(&b, b.checkout.path(), &policy);
    assert!(
        text.contains("1 settled by landing on origin/dev or origin/main, 0 past their wait-until (not offered)"),
        "{text}"
    );
}

#[test]
fn peer_intents_misses_renders_a_live_carry_in_text_and_json() {
    let (keys, a, b) = household();
    let policy = steward_policy(keys.path());
    let intent = atom_cid(&"live-carry-intent".to_string())
        .unwrap()
        .to_string();
    let miss = misses_record(&intent, "abcdef", "7", &a.key.did_key());
    let miss_cid = atom_cid(&miss).unwrap().to_string();
    let until = now_unix_s() + 3_600;
    append(
        b.checkout.path(),
        [
            miss,
            carry_record(&intent, "abcdef", &miss_cid, until, "8", &a.key.did_key()),
        ],
    );
    save_served_table(
        &b,
        b.checkout.path(),
        Some((&a.key.did_key(), 1_792_769_291_000)),
    );

    let misses = intents_misses_json(&b, b.checkout.path(), &policy);
    assert_eq!(
        misses[0]["carriedBy"],
        serde_json::json!([{"device": a.key.did_key(), "until": rfc3339(until)}]),
        "{misses}"
    );
    let text = intents_misses_text(&b, b.checkout.path(), &policy);
    assert!(
        text.contains(&format!(
            "carried by {} until {}",
            short_device(&a.key.did_key()),
            rfc3339(until)
        )),
        "{text}"
    );
}

#[test]
fn peer_intents_misses_renders_an_expired_carry_as_dash() {
    let (keys, a, b) = household();
    let policy = steward_policy(keys.path());
    let intent = atom_cid(&"expired-carry-intent".to_string())
        .unwrap()
        .to_string();
    let miss = misses_record(&intent, "abcdef", "7", &a.key.did_key());
    let miss_cid = atom_cid(&miss).unwrap().to_string();
    append(
        b.checkout.path(),
        [
            miss,
            carry_record(
                &intent,
                "abcdef",
                &miss_cid,
                now_unix_s().saturating_sub(1),
                "8",
                &a.key.did_key(),
            ),
        ],
    );
    save_served_table(
        &b,
        b.checkout.path(),
        Some((&a.key.did_key(), 1_792_769_291_000)),
    );

    let misses = intents_misses_json(&b, b.checkout.path(), &policy);
    assert_eq!(misses[0]["carriedBy"], serde_json::json!([]), "{misses}");
    let text = intents_misses_text(&b, b.checkout.path(), &policy);
    assert!(text.trim_end().ends_with("carried -"), "{text}");
}

#[test]
fn peer_intents_misses_does_not_apply_a_carry_for_another_closure() {
    let (keys, a, b) = household();
    let policy = steward_policy(keys.path());
    let intent = atom_cid(&"other-closure-carry-intent".to_string())
        .unwrap()
        .to_string();
    let miss = misses_record(&intent, "abcdef", "7", &a.key.did_key());
    let other_miss = misses_record(&intent, "fedcba", "8", &a.key.did_key());
    let other_miss_cid = atom_cid(&other_miss).unwrap().to_string();
    append(
        b.checkout.path(),
        [
            miss,
            other_miss,
            carry_record(
                &intent,
                "fedcba",
                &other_miss_cid,
                now_unix_s() + 3_600,
                "9",
                &a.key.did_key(),
            ),
        ],
    );
    save_served_table(
        &b,
        b.checkout.path(),
        Some((&a.key.did_key(), 1_792_769_291_000)),
    );

    let misses = intents_misses_json(&b, b.checkout.path(), &policy);
    let target = misses
        .as_array()
        .unwrap()
        .iter()
        .find(|miss| miss["closure"] == "abcdef")
        .unwrap();
    assert_eq!(target["carriedBy"], serde_json::json!([]), "{misses}");
    let text = intents_misses_text(&b, b.checkout.path(), &policy);
    let target = text
        .lines()
        .find(|line| line.contains("closure abcdef"))
        .unwrap();
    assert!(target.ends_with("carried -"), "{text}");
}

#[test]
fn peer_intents_counts_a_settling_row_only_from_the_announcing_device() {
    let (keys, a, b) = household();
    let policy = steward_policy(keys.path());
    let own = intent_atom(
        b.checkout.path(),
        LANDING_CHANGE,
        LANDING_TREE,
        LANDING_BASE,
        &[],
        "low",
        None,
    );
    let sibling = atom_cid(&"sibling-authority".to_string())
        .unwrap()
        .to_string();
    append(
        b.checkout.path(),
        [
            announcement(&sibling, Some(&a.key.did_key()), "4"),
            landing_record(
                "withdraw",
                &own,
                LANDING_CHANGE,
                &[("dseq", "5")],
                Some(&a.key.did_key()),
            ),
            landing_record("withdraw", &own, LANDING_CHANGE, &[("dseq", "6")], None),
            landing_record(
                "landed",
                &sibling,
                LANDING_CHANGE,
                &[("dseq", "7"), ("landed_as", "ff")],
                Some(&a.key.did_key()),
            ),
            landing_record(
                "landed",
                &sibling,
                LANDING_CHANGE,
                &[("dseq", "8"), ("landed_as", "moved")],
                None,
            ),
        ],
    );
    save_served_table(&b, b.checkout.path(), Some((&a.key.did_key(), 50)));
    let view = intents_json(&b, b.checkout.path(), &policy, true);
    assert_eq!(
        view["intents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["intent"] == own)
            .unwrap()["state"],
        "withdrawn"
    );
    assert_eq!(
        view["intents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["intent"] == sibling)
            .unwrap()["state"],
        "landed"
    );
    assert_eq!(view["advisory"].as_array().unwrap().len(), 2, "{view}");
    assert!(view["advisory"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["intent"] == own && row["kind"] == "withdraw"));
    assert!(view["advisory"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["intent"] == sibling && row["kind"] == "landed"));
}

#[test]
fn peer_intents_never_orders_by_fold_position() {
    let (keys, a, b) = household();
    let policy = steward_policy(keys.path());
    let left = checkout();
    let right = checkout();
    let roster_rel = format!(".eprfs/status/participants/{HANDLE}.jsonl");
    for root in [left.path(), right.path()] {
        std::fs::create_dir_all(root.join(".eprfs/status/participants")).unwrap();
        std::fs::copy(b.checkout.path().join(&roster_rel), root.join(&roster_rel)).unwrap();
    }
    let sibling = atom_cid(&"fold-order".to_string()).unwrap().to_string();
    let events = [
        announcement(&sibling, Some(&a.key.did_key()), "41"),
        landing_record(
            "withdraw",
            &sibling,
            LANDING_CHANGE,
            &[("dseq", "43")],
            Some(&a.key.did_key()),
        ),
        landing_record(
            "landed",
            &sibling,
            LANDING_CHANGE,
            &[("dseq", "42"), ("landed_as", "ff")],
            Some(&a.key.did_key()),
        ),
        landing_record("dismiss", &sibling, LANDING_CHANGE, &[("dseq", "44")], None),
    ];
    append(left.path(), events.clone());
    append(right.path(), events.into_iter().rev());
    save_served_table(&b, left.path(), Some((&a.key.did_key(), 100)));
    let one = intents_json(&b, left.path(), &policy, true);
    save_served_table(&b, right.path(), Some((&a.key.did_key(), 100)));
    let two = intents_json(&b, right.path(), &policy, true);
    assert_eq!(one["intents"], two["intents"]);
    assert_eq!(one["advisory"], two["advisory"]);
    let row = one["intents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["intent"] == sibling)
        .unwrap();
    assert_eq!(row["dseq"], 41);
    assert_eq!(row["settled"][0]["dseq"], 42);
}

#[test]
fn peer_intents_reads_the_served_store_from_a_linked_worktree() {
    let (keys, _a, b) = household();
    let policy = steward_policy(keys.path());
    let served = checkout();
    let cid = intent_atom(
        served.path(),
        LANDING_CHANGE,
        LANDING_TREE,
        LANDING_BASE,
        &[],
        "low",
        None,
    );
    save_served_table(&b, served.path(), None);
    let view = intents_json(&b, b.checkout.path(), &policy, false);
    assert_eq!(view["intents"][0]["intent"], cid);
    assert_eq!(
        view["store"],
        served.path().canonicalize().unwrap().to_str().unwrap()
    );
}

#[test]
fn peer_intents_refuses_a_gone_store_and_creates_nothing() {
    let (keys, _a, b) = household();
    let policy = steward_policy(keys.path());
    let missing = b.checkout.path().join("gone/flows.jsonl");
    HeadTable {
        store: Some(StorePlace {
            root: b.checkout.path().to_path_buf(),
            log: missing.clone(),
        }),
        ..HeadTable::default()
    }
    .save(&b.peer_dir())
    .unwrap();
    let output = intents_output(&b, b.checkout.path(), &policy, false);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(missing.to_str().unwrap()));
    assert!(!missing.exists());
}

#[test]
fn peer_intents_on_a_device_without_a_flow_store_refuses_and_creates_nothing() {
    let (keys, _a, b) = household();
    let view = intents_json(&b, b.checkout.path(), &steward_policy(keys.path()), false);
    assert_eq!(view["store"], serde_json::Value::Null);
    assert_eq!(view["intents"], serde_json::json!([]));
    assert!(view["error"]
        .as_str()
        .unwrap()
        .contains("has no flow store at"));
    assert!(!b.checkout.path().join(".eprfs/status/flows.jsonl").exists());
}

#[test]
fn peer_intents_on_a_non_device_without_a_flow_store_is_empty() {
    let keys = TempDir::new().unwrap();
    let device = Device::new(&keys, "not-device-empty");
    let view = intents_json(
        &device,
        device.checkout.path(),
        &steward_policy(keys.path()),
        false,
    );
    assert_eq!(view["standing"], "not_a_device");
    assert_eq!(view["intents"], serde_json::json!([]));
    assert_eq!(view["error"], serde_json::Value::Null);
}

#[test]
fn peer_intents_without_a_served_store_reports_siblings_not_read() {
    let (keys, _a, b) = household();
    let cid = intent_atom(
        b.checkout.path(),
        LANDING_CHANGE,
        LANDING_TREE,
        LANDING_BASE,
        &[],
        "low",
        None,
    );
    let view = intents_json(&b, b.checkout.path(), &steward_policy(keys.path()), false);
    assert_eq!(view["intents"][0]["intent"], cid);
    assert_eq!(view["siblingsRead"], false);
    assert!(view["siblingsWhy"]
        .as_str()
        .unwrap()
        .contains("no epr peer serve"));
}

#[test]
fn peer_intents_supersession_leaves_only_the_re_raise_live() {
    let (keys, _a, b) = household();
    let policy = steward_policy(keys.path());
    let prior = intent_atom(
        b.checkout.path(),
        LANDING_CHANGE,
        LANDING_TREE,
        LANDING_BASE,
        &[],
        "low",
        None,
    );
    let successor = intent_atom(
        b.checkout.path(),
        LANDING_CHANGE,
        LANDING_TREE,
        LANDING_BASE,
        &[],
        "low",
        Some(&prior),
    );
    save_served_table(&b, b.checkout.path(), None);
    let live = intents_json(&b, b.checkout.path(), &policy, false);
    assert_eq!(live["intents"].as_array().unwrap().len(), 1);
    assert_eq!(live["intents"][0]["intent"], successor);
    let all = intents_json(&b, b.checkout.path(), &policy, true);
    assert_eq!(
        all["intents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["intent"] == prior)
            .unwrap()["state"],
        "superseded"
    );
}

#[test]
fn peer_intents_successor_of_a_withdrawn_intent_is_live() {
    let (keys, _a, b) = household();
    let policy = steward_policy(keys.path());
    let prior = intent_atom(
        b.checkout.path(),
        LANDING_CHANGE,
        LANDING_TREE,
        LANDING_BASE,
        &[],
        "low",
        None,
    );
    append(
        b.checkout.path(),
        [landing_record(
            "withdraw",
            &prior,
            LANDING_CHANGE,
            &[("dseq", "2")],
            None,
        )],
    );
    let successor = intent_atom(
        b.checkout.path(),
        LANDING_CHANGE,
        LANDING_TREE,
        LANDING_BASE,
        &[],
        "low",
        Some(&prior),
    );
    save_served_table(&b, b.checkout.path(), None);
    let all = intents_json(&b, b.checkout.path(), &policy, true);
    assert_eq!(
        all["intents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["intent"] == prior)
            .unwrap()["state"],
        "withdrawn"
    );
    assert_eq!(
        all["intents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["intent"] == successor)
            .unwrap()["state"],
        "live"
    );
}

#[test]
fn peer_intents_on_a_non_device_lists_own_intents_only() {
    let keys = TempDir::new().unwrap();
    let device = Device::new(&keys, "not-device-own");
    let own = intent_atom(
        device.checkout.path(),
        LANDING_CHANGE,
        LANDING_TREE,
        LANDING_BASE,
        &[],
        "low",
        None,
    );
    let sibling = atom_cid(&"not-device-sibling".to_string())
        .unwrap()
        .to_string();
    append(
        device.checkout.path(),
        [announcement(&sibling, Some("did:key:unread"), "8")],
    );
    let view = intents_json(
        &device,
        device.checkout.path(),
        &steward_policy(keys.path()),
        false,
    );
    assert_eq!(view["intents"].as_array().unwrap().len(), 1);
    assert_eq!(view["intents"][0]["intent"], own);
    assert_eq!(view["intents"][0]["own"], true);
}
