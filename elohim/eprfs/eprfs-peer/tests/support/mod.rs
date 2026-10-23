//! Shared fixtures for the steward channel's tests: a person's devices, their roster rows, and
//! flow events shaped as berth emits them (`dev-berth-events@1`, T1).
//!
//! Every key here is generated for the test; none is read from the workspace.

// Each integration test binary compiles this module and uses only part of it.
#![allow(
    dead_code,
    reason = "each test binary uses a different part of the shared fixtures"
)]

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, RwLock};

use elohim_epr::proof::{self, AgentKeypair};
use elohim_epr_rea::{
    atom_cid, ActorRecord, ActorWitness, AgentRef, FlowEvent, FlowRecord, FlowStore, Magnitude,
    ParticipantRow, ReaVerb, SidecarFlowStore, SignatureVerifier,
};
use eprfs_peer::{Admission, PeerKey, SharedAdmission};
use iroh::{Endpoint, NodeAddr};

pub const HANDLE: &str = "matthew";
pub const MEASURE: &str = "dev-berth-events@1";

/// One of the person's devices: its did:key and the ed25519 key behind it.
pub struct Device {
    pub did: String,
    pub key: AgentKeypair,
}

pub fn device(name: &str, seed: u8) -> Device {
    Device {
        did: format!("did:key:z{name}"),
        key: AgentKeypair::from_secret(&[seed; 32]).expect("seed"),
    }
}

impl Device {
    pub fn signer(&self) -> impl Fn(&[u8]) -> Vec<u8> + Send + Sync + 'static {
        let key = AgentKeypair::from_secret(&self.key.secret_bytes()).expect("seed");
        move |message: &[u8]| proof::sign(&key, message)
    }
}

pub type Verifier = Arc<dyn SignatureVerifier + Send + Sync>;

pub fn verifier(devices: &[&Device]) -> Verifier {
    let keys: HashMap<String, [u8; 32]> = devices
        .iter()
        .map(|d| (d.did.clone(), d.key.public_key_bytes()))
        .collect();
    Arc::new(move |did: &str, message: &[u8], signature: &[u8]| {
        keys.get(did)
            .is_some_and(|pk| proof::verify(pk, message, signature))
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sign(device: &Device, message: &[u8]) -> String {
    hex(&proof::sign(&device.key, message))
}

pub fn genesis(root: &Device) -> ParticipantRow {
    let founding = ActorRecord::Witness(
        ActorWitness::new(
            "human:matthew",
            "agent:orchestrator@claude-opus-5-5",
            "s1",
            "operator of this stewarded device; present in this session",
            "2026-10-23T00:00:00Z",
        )
        .expect("witness"),
    );
    let mut row = ParticipantRow::Genesis {
        handle: HANDLE.into(),
        chain_root: root.did.clone(),
        record_cid: founding.cid().expect("cid").to_string(),
        signer: root.did.clone(),
        signature: String::new(),
    };
    let message = row.signing_message().expect("message");
    if let ParticipantRow::Genesis { signature, .. } = &mut row {
        *signature = sign(root, &message);
    }
    row
}

pub fn binding(root: &Device, joiner: &Device) -> ParticipantRow {
    let mut row = ParticipantRow::Binding {
        handle: HANDLE.into(),
        chain_root: root.did.clone(),
        controller: joiner.did.clone(),
        authorized_by: root.did.clone(),
        sig_a: String::new(),
        sig_b: String::new(),
        nonce: "nonce-bind".into(),
    };
    let message = row.signing_message().expect("message");
    if let ParticipantRow::Binding { sig_a, sig_b, .. } = &mut row {
        *sig_a = sign(root, &message);
        *sig_b = sign(joiner, &message);
    }
    row
}

pub fn endpoint_row(root: &Device, dev: &Device, key: &PeerKey) -> ParticipantRow {
    let mut row = ParticipantRow::PeerEndpoint {
        handle: HANDLE.into(),
        chain_root: root.did.clone(),
        device: dev.did.clone(),
        node_id: key.node_id().to_string(),
        seq: 1,
        nonce: "nonce-1".into(),
        signature: String::new(),
    };
    let message = row.signing_message().expect("message");
    if let ParticipantRow::PeerEndpoint { signature, .. } = &mut row {
        *signature = sign(dev, &message);
    }
    row
}

pub fn contest(root: &Device, target: &ParticipantRow) -> ParticipantRow {
    let mut row = ParticipantRow::Contest {
        handle: HANDLE.into(),
        target_cid: target.cid().expect("cid").to_string(),
        by: root.did.clone(),
        basis: "that device is lost".into(),
        signature: String::new(),
    };
    let message = row.signing_message().expect("message");
    if let ParticipantRow::Contest { signature, .. } = &mut row {
        *signature = sign(root, &message);
    }
    row
}

pub fn peer_key() -> (tempfile::TempDir, PeerKey) {
    let dir = tempfile::tempdir().expect("tempdir");
    let key = PeerKey::load_or_generate(dir.path()).expect("key");
    (dir, key)
}

/// Device A (the chain root) and device B (bound by A), each with a peer key and an endpoint row.
pub struct Household {
    pub a: Device,
    pub b: Device,
    pub rows: Vec<ParticipantRow>,
    pub verify: Verifier,
    pub key_a: PeerKey,
    pub key_b: PeerKey,
    _dirs: Vec<tempfile::TempDir>,
}

pub fn household() -> Household {
    let a = device("DeviceA", 1);
    let b = device("DeviceB", 2);
    let (da, key_a) = peer_key();
    let (db, key_b) = peer_key();
    let rows = vec![
        genesis(&a),
        binding(&a, &b),
        endpoint_row(&a, &a, &key_a),
        endpoint_row(&a, &b, &key_b),
    ];
    let verify = verifier(&[&a, &b]);
    Household {
        a,
        b,
        rows,
        verify,
        key_a,
        key_b,
        _dirs: vec![da, db],
    }
}

impl Household {
    pub fn admission(&self) -> Admission {
        Admission::from_roster(&self.rows, &self.a.did, &*self.verify)
    }

    pub fn shared_admission(&self) -> SharedAdmission {
        Arc::new(RwLock::new(self.admission()))
    }
}

/// A berth record as T1 folds it: `reason:<kind>`, the measure, and every field as `env:<k>=<v>`.
pub fn berth_event(kind: &str, resource: &str, session: &str, at: &str) -> FlowEvent {
    event_with(
        MEASURE,
        kind,
        vec![
            format!("env:at={at}"),
            format!("env:kind={kind}"),
            format!("env:resource={resource}"),
            format!("env:session={session}"),
        ],
    )
}

pub fn event_with(measure: &str, kind: &str, env: Vec<String>) -> FlowEvent {
    let mut classified_as = vec![
        "run:observation".to_string(),
        ".".to_string(),
        format!("reason:{kind}"),
        format!("measure:{measure}"),
        "value:1".to_string(),
        "unit:events".to_string(),
    ];
    classified_as.extend(env);
    classified_as.push("actor-claim:bafyreiexampleactorclaim".to_string());
    classified_as.push("source:claim-signed".to_string());
    classified_as.push("steward:human:matthew".to_string());
    FlowEvent {
        action: ReaVerb::Cite,
        provider: AgentRef("human:matthew".to_string()),
        receiver: AgentRef("repo:ethosengine/elohim".to_string()),
        resource: atom_cid(&"berth".to_string()).expect("cid"),
        quantity: Magnitude::Count {
            value: 1.0,
            unit: "run-note".to_string(),
        },
        process: None,
        in_scope_of: atom_cid(&"repo-scope".to_string()).expect("cid"),
        fulfills: Vec::new(),
        satisfies: Vec::new(),
        occurred_at: "2026-10-23T15:28:09Z".to_string(),
        classified_as,
    }
}

/// A device's checkout: a scratch root holding its flow store.
pub struct Checkout {
    pub dir: tempfile::TempDir,
}

impl Checkout {
    pub fn new() -> Checkout {
        Checkout {
            dir: tempfile::tempdir().expect("tempdir"),
        }
    }

    pub fn store(&self) -> SidecarFlowStore {
        SidecarFlowStore::open(self.dir.path()).expect("flow store")
    }

    pub fn append(&self, event: FlowEvent) -> cid::Cid {
        self.store()
            .append(FlowRecord::Event(event))
            .expect("append")
    }

    pub fn events(&self) -> Vec<(cid::Cid, FlowEvent)> {
        self.store().events().expect("events")
    }
}

/// The pool-policy shape `berth.channel.crosses` is read from (spec §3.3, carry ruling 10).
pub fn policy() -> serde_json::Value {
    serde_json::json!({
        "berth": {
            "channel": {
                "crosses": {
                    MEASURE: ["moor", "unmoor", "claim", "renew", "release", "override", "note", "shed"]
                }
            }
        }
    })
}

/// The loopback address of a bound endpoint, to dial it directly.
pub fn loopback(endpoint: &Endpoint) -> NodeAddr {
    let port = endpoint
        .bound_sockets()
        .into_iter()
        .find(SocketAddr::is_ipv4)
        .expect("an ipv4 socket")
        .port();
    NodeAddr::new(endpoint.node_id())
        .with_direct_addresses([SocketAddr::from((Ipv4Addr::LOCALHOST, port))])
}
