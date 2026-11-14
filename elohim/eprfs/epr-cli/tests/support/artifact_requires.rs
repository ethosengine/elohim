//! `epr artifact requires --output <cid>` (plan item K13): an output's recorded requirement read
//! from this device's own stores, with `self` decided by this device's key and never a flag.
use super::*;

fn clause(axis: &str, value: &str) -> serde_json::Value {
    serde_json::json!({"axis": axis, "predicate": {"kind": "equal", "value": value},
        "source": "extracted"})
}

/// Canonical bytes of one requirement document and their CID.
fn document(clauses: &[serde_json::Value]) -> (BlobCid, Vec<u8>) {
    let mut value = serde_json::Value::Array(clauses.to_vec());
    value.sort_all_objects();
    let bytes = serde_json::to_vec(&value).unwrap();
    (BlobCid::compute_bytes(&bytes), bytes)
}

/// [`document`], put into `device`'s local payload store.
fn hold_document(device: &Device, clauses: &[serde_json::Value]) -> (BlobCid, Vec<u8>) {
    let (cid, bytes) = document(clauses);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let store = elohim_epr_cli::payload::open_payload_store(
            &device.payload(),
            elohim_epr_cli::payload::Access::Shared,
        )
        .await
        .unwrap()
        .store;
        store
            .put_blob_verified(&cid, eprfs_peer::Bytes::from(bytes.clone()))
            .await
            .unwrap();
    });
    (cid, bytes)
}

/// A custody Produce of `output` recorded by `device` itself, optionally carrying a requirement.
fn own_produce(device: &Device, output: &BlobCid, requires: Option<&BlobCid>) -> FlowEvent {
    let key = BlobCid::compute(b"requires-key").to_string();
    let mut row = produce_record(
        &key,
        output,
        &BlobCid::compute(b"manifest").to_string(),
        None,
    );
    row.receiver = AgentRef(device.key.did_key());
    row.classified_as
        .retain(|slot| !slot.starts_with("measure:") && !slot.starts_with("reason:"));
    row.classified_as
        .extend(requires.map(|cid| format!("env:requires={cid}")));
    row
}

fn query(device: &Device, output: &BlobCid, extra: &[&str]) -> (u8, serde_json::Value, String) {
    let output = output.to_string();
    let mut args = vec![
        "artifact",
        "requires",
        "--output",
        &output,
        "--repo",
        device.root().to_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    let json = device.epr(&[args.as_slice(), &["--json"]].concat());
    let plain = device.epr(&args);
    assert_eq!(json.status.code(), plain.status.code(), "{}", text(&plain));
    (
        u8::try_from(json.status.code().unwrap()).unwrap(),
        serde_json::from_slice(&json.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&json))),
        String::from_utf8(plain.stdout).unwrap(),
    )
}

#[test]
fn requires_answers_unknown_unmeasured_and_self_from_the_device_key() {
    let (_keys, a, b) = household();
    let output = BlobCid::compute_bytes(b"requires-output");
    let (code, json, plain) = query(&b, &output, &[]);
    assert_eq!(code, 2, "an output with no custody Produce is unknown");
    assert_eq!(json["known"], false);
    assert!(plain.starts_with(&format!("unknown {output}")), "{plain}");

    // A sibling's crossed Produce with no slot: known, unmeasured, and not this device's build.
    let mut store = SidecarFlowStore::open(b.root()).unwrap();
    let key = BlobCid::compute(b"requires-key").to_string();
    let manifest = BlobCid::compute(b"manifest").to_string();
    store
        .append(FlowRecord::Event(produce_record(
            &key,
            &output,
            &manifest,
            Some(&a.key.did_key()),
        )))
        .unwrap();
    let (code, json, plain) = query(&b, &output, &[]);
    assert_eq!(
        code, 0,
        "no recorded requirement is unmeasured, not a refusal"
    );
    assert_eq!(json["unmeasured"], "no recorded requirement");
    assert_eq!(json["self"], false);
    assert_eq!(json["produce"], serde_json::Value::Null);
    assert_eq!(
        plain.trim(),
        format!("unmeasured {output}: no recorded requirement other")
    );

    // This device's own put makes it `self`, and names that Produce record.
    store
        .append(FlowRecord::Event(own_produce(&b, &output, None)))
        .unwrap();
    let (code, json, plain) = query(&b, &output, &[]);
    assert_eq!(code, 0);
    assert_eq!(json["self"], true);
    let produce = json["produce"].as_str().unwrap();
    let records: Vec<_> = store
        .records()
        .unwrap()
        .into_iter()
        .map(|(cid, _)| cid.to_string())
        .collect();
    assert_eq!(records.last().map(String::as_str), Some(produce));
    assert!(plain.trim().ends_with(" self"), "{plain}");

    // `self` is per store and per key: the sibling's own store holds no custody of b's build.
    let (_, json, _) = query(&a, &output, &[]);
    assert_eq!(json["known"], false);
}

#[test]
fn requires_hands_on_one_document_byte_for_byte_and_merges_several() {
    let (_keys, a, b) = household();
    let output = BlobCid::compute_bytes(b"requires-merge-output");
    let arch = host_contract::names::axis::ARCH;
    let (cid, bytes) = hold_document(&b, &[clause(arch, "x86_64")]);
    let mut store = SidecarFlowStore::open(b.root()).unwrap();
    store
        .append(FlowRecord::Event(own_produce(&b, &output, Some(&cid))))
        .unwrap();
    // A later Produce with no slot adds nothing to the answer.
    store
        .append(FlowRecord::Event(own_produce(&b, &output, None)))
        .unwrap();
    let scratch = TempDir::new().unwrap();
    let written = scratch.path().join("requires.json");
    let (code, json, plain) = query(&b, &output, &["--write", written.to_str().unwrap()]);
    assert_eq!(code, 0, "{json}");
    assert_eq!(json["requires"], cid.to_string());
    assert_eq!(json["sources"], serde_json::json!([cid.to_string()]));
    assert_eq!(json["self"], true);
    assert_eq!(std::fs::read(&written).unwrap(), bytes);
    assert_eq!(plain.trim(), format!("requires {cid} self"));

    // A sibling's document for the same output that is not held here: unavailable, exit 3.
    let os = host_contract::names::axis::OS;
    let (sibling, _) = document(&[clause(os, "linux")]);
    let mut crossed = produce_record(
        &BlobCid::compute(b"requires-key").to_string(),
        &output,
        &BlobCid::compute(b"manifest").to_string(),
        Some(&a.key.did_key()),
    );
    crossed
        .classified_as
        .push(format!("env:requires={sibling}"));
    store.append(FlowRecord::Event(crossed)).unwrap();
    let (code, json, _) = query(&b, &output, &[]);
    assert_eq!(code, 3, "{json}");
    assert_eq!(
        json["unavailable"],
        serde_json::json!([sibling.to_string()])
    );

    // Once held, the two compatible documents merge into one canonical document.
    let (held, _) = hold_document(&b, &[clause(os, "linux")]);
    assert_eq!(held, sibling);
    let (code, json, _) = query(&b, &output, &["--write", written.to_str().unwrap()]);
    assert_eq!(code, 0, "{json}");
    assert_eq!(json["sources"].as_array().unwrap().len(), 2);
    let merged = std::fs::read(&written).unwrap();
    assert_eq!(
        json["requires"],
        BlobCid::compute_bytes(&merged).to_string()
    );
    let decoded: host_contract::Requires = serde_json::from_slice(&merged).unwrap();
    assert_eq!(decoded.clauses().len(), 2);
}

#[test]
fn requires_refuses_conflicting_documents_as_a_merge_error() {
    let (_keys, a, b) = household();
    let output = BlobCid::compute_bytes(b"requires-conflict-output");
    let arch = host_contract::names::axis::ARCH;
    let (x86, _) = hold_document(&b, &[clause(arch, "x86_64")]);
    let (arm, _) = hold_document(&b, &[clause(arch, "aarch64")]);
    let mut store = SidecarFlowStore::open(b.root()).unwrap();
    store
        .append(FlowRecord::Event(own_produce(&b, &output, Some(&x86))))
        .unwrap();
    let mut crossed = produce_record(
        &BlobCid::compute(b"requires-key").to_string(),
        &output,
        &BlobCid::compute(b"manifest").to_string(),
        Some(&a.key.did_key()),
    );
    crossed.classified_as.push(format!("env:requires={arm}"));
    store.append(FlowRecord::Event(crossed)).unwrap();
    let scratch = TempDir::new().unwrap();
    let written = scratch.path().join("requires.json");
    let (code, json, plain) = query(&b, &output, &["--write", written.to_str().unwrap()]);
    assert_eq!(code, 4, "{json}");
    assert!(json["conflict"].as_str().is_some(), "{json}");
    assert!(
        plain.starts_with(&format!("conflict {output}: MergeError")),
        "{plain}"
    );
    assert!(!written.exists(), "a conflict writes no document");
}
