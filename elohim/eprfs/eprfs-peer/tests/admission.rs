//! Admission: an authenticated iroh node id is admitted only when the device's own roster names
//! it in a valid `PeerEndpoint` row of the same lineage.

use std::collections::HashMap;
use std::fs;

use elohim_epr::proof::{self, AgentKeypair};
use elohim_epr_rea::{ActorRecord, ActorWitness, ParticipantRow, Roster};
use eprfs_peer::{roster_rows_from_file, Admission, PeerKey};

const HANDLE: &str = "matthew";

struct Device {
    did: String,
    key: AgentKeypair,
}

fn device(name: &str, seed: u8) -> Device {
    Device {
        did: format!("did:key:z{name}"),
        key: AgentKeypair::from_secret(&[seed; 32]).expect("seed"),
    }
}

fn verifier(devices: &[&Device]) -> impl Fn(&str, &[u8], &[u8]) -> bool {
    let keys: HashMap<String, [u8; 32]> = devices
        .iter()
        .map(|d| (d.did.clone(), d.key.public_key_bytes()))
        .collect();
    move |did, message, signature| {
        keys.get(did)
            .is_some_and(|pk| proof::verify(pk, message, signature))
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sign(device: &Device, message: &[u8]) -> String {
    hex(&proof::sign(&device.key, message))
}

fn genesis(root: &Device) -> ParticipantRow {
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

fn binding(root: &Device, joiner: &Device) -> ParticipantRow {
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

fn endpoint(root: &Device, dev: &Device, node_id: &str, seq: u64) -> ParticipantRow {
    let mut row = ParticipantRow::PeerEndpoint {
        handle: HANDLE.into(),
        chain_root: root.did.clone(),
        device: dev.did.clone(),
        node_id: node_id.into(),
        seq,
        nonce: format!("nonce-{seq}"),
        signature: String::new(),
    };
    let message = row.signing_message().expect("message");
    if let ParticipantRow::PeerEndpoint { signature, .. } = &mut row {
        *signature = sign(dev, &message);
    }
    row
}

fn peer_key() -> (tempfile::TempDir, PeerKey) {
    let dir = tempfile::tempdir().expect("tempdir");
    let key = PeerKey::load_or_generate(dir.path()).expect("key");
    (dir, key)
}

#[test]
fn admission_maps_node_id_to_device() {
    let a = device("DeviceA", 1);
    let b = device("DeviceB", 2);
    let v = verifier(&[&a, &b]);
    let (_da, key_a) = peer_key();
    let (_db, key_b) = peer_key();
    let (_ds, stranger) = peer_key();
    let rows = vec![
        genesis(&a),
        binding(&a, &b),
        endpoint(&a, &a, &key_a.node_id().to_string(), 1),
        endpoint(&a, &b, &key_b.node_id().to_string(), 1),
    ];

    let admission = Admission::from_roster(&rows, &a.did, &v);

    assert_eq!(admission.chain_root(), a.did);
    assert_eq!(admission.admits(&key_a.node_id()), Some(a.did.as_str()));
    assert_eq!(admission.admits(&key_b.node_id()), Some(b.did.as_str()));
    assert_eq!(admission.admits(&stranger.node_id()), None, "default deny");

    let other_lineage = Admission::from_roster(&rows, &b.did, &v);
    assert_eq!(
        other_lineage.admits(&key_a.node_id()),
        None,
        "a node id is admitted only under the lineage its row names"
    );
}

#[test]
fn an_unverifiable_endpoint_row_admits_nobody() {
    let a = device("DeviceA", 1);
    let (_d, key_a) = peer_key();
    let rows = vec![
        genesis(&a),
        endpoint(&a, &a, &key_a.node_id().to_string(), 1),
    ];
    let refuses_all = |_: &str, _: &[u8], _: &[u8]| false;

    let admission = Admission::from_roster(&rows, &a.did, &refuses_all);
    assert_eq!(admission.admits(&key_a.node_id()), None);
}

#[test]
fn reload_takes_a_rotation_and_a_revocation_at_once() {
    let a = device("DeviceA", 1);
    let b = device("DeviceB", 2);
    let v = verifier(&[&a, &b]);
    let (_d1, old_b) = peer_key();
    let (_d2, new_b) = peer_key();
    let mut rows = vec![
        genesis(&a),
        binding(&a, &b),
        endpoint(&a, &b, &old_b.node_id().to_string(), 1),
    ];
    let mut admission = Admission::from_roster(&rows, &a.did, &v);
    assert_eq!(admission.admits(&old_b.node_id()), Some(b.did.as_str()));

    rows.push(endpoint(&a, &b, &new_b.node_id().to_string(), 2));
    admission.reload(&rows, &v);
    assert_eq!(
        admission.admits(&old_b.node_id()),
        None,
        "a rotated key stops"
    );
    assert_eq!(admission.admits(&new_b.node_id()), Some(b.did.as_str()));

    let binding_cid = binding(&a, &b).cid().expect("cid").to_string();
    let mut contest = ParticipantRow::Contest {
        handle: HANDLE.into(),
        target_cid: binding_cid,
        by: a.did.clone(),
        basis: "that device is lost".into(),
        signature: String::new(),
    };
    let message = contest.signing_message().expect("message");
    if let ParticipantRow::Contest { signature, .. } = &mut contest {
        *signature = sign(&a, &message);
    }
    rows.push(contest);
    admission.reload(&rows, &v);
    assert_eq!(
        admission.admits(&new_b.node_id()),
        None,
        "a contested binding revokes the device on the next reload"
    );
}

#[test]
fn roster_rows_come_from_the_devices_own_roster_file() {
    let a = device("DeviceA", 1);
    let v = verifier(&[&a]);
    let (_d, key_a) = peer_key();
    let mut roster = Roster::new(HANDLE).expect("roster");
    let mut text = String::new();
    for row in [
        genesis(&a),
        endpoint(&a, &a, &key_a.node_id().to_string(), 1),
    ] {
        text.push_str(&Roster::line_for(&row).expect("line"));
        text.push('\n');
        roster.append(row).expect("append");
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(format!("{HANDLE}.jsonl"));
    fs::write(&path, &text).expect("write");

    let rows = roster_rows_from_file(&path).expect("read");
    let expected: Vec<ParticipantRow> = roster.rows().iter().map(|(_, r)| r.clone()).collect();
    assert_eq!(rows, expected);
    let admission = Admission::from_roster(&rows, &a.did, &v);
    assert_eq!(admission.admits(&key_a.node_id()), Some(a.did.as_str()));

    let missing = roster_rows_from_file(&dir.path().join("nobody.jsonl")).expect("absent");
    assert!(
        missing.is_empty(),
        "an absent roster is an empty one: it admits nobody"
    );

    fs::write(&path, text.replacen("nonce-1", "nonce-X", 1)).expect("tamper");
    assert!(
        roster_rows_from_file(&path).is_err(),
        "a line whose payload no longer matches its CID is refused"
    );
}
