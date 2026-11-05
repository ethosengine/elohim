//! The shared vectors (`elohim/eprfs/fixtures/tree-vectors.json`, interfaces spec §H.1): byte
//! compatibility between this crate and `@elohim/eprfs`, pinned by one file both suites read.
//!
//! This test recomputes every vector from its inputs and compares it with the file. When they
//! differ it fails and prints the recomputed file. The file is compare-only once written:
//! `EPRFS_WRITE_VECTORS=1` may ADD an entry that is absent, and refuses to change one that is
//! present (a changed expectation needs review and, on behaviour change, a new profile epoch).
//! The TypeScript suite reads the same file and must reproduce the same values.
//!
//! Sections owned elsewhere are checked here, never rewritten: `chunker` is written by
//! `elohim/epr/tests/chunker_vectors.rs` (the byte profile's `fastcdc-2020` vectors) and
//! re-derived below from the same fixture generator; `address` carries the kernel's byte-CID
//! conformance rows (`crates/elohim-address/vectors/address-vectors.json`) through eprfs's own
//! constructors.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use async_trait::async_trait;
use bytes::Bytes;
use eprfs_core::{
    encode_tree, project_view, walk_batched, walk_collect, walk_spine, AbsentReason, AskPolicy,
    AttestationDraft, BatchOptions, BlobCid, BlobHandle, BlobPresence, ByteCid, Coverage,
    EprRecord, EprRef, EprfsError, EprfsStorage, FetchPolicy, ItemAbsence, KeyCustody, OpennessAsk,
    Proximity, ProximityOrder, ReachLevel, ReachReason, ReachRefusal, TierAddress, TierClass,
    TierId, TierOrder, TierRecord, TierSource, TreeChange, TreeEntry, TreeEntryKind, TreeLimits,
    TreeNode, TreePath, TreeView, View, WalkItem, WalkedView,
};
use serde_json::{json, Value};

const FILE: &str = include_str!("../../fixtures/tree-vectors.json");
// gate-reads: crates/elohim-address/vectors/address-vectors.json
const KERNEL_VECTORS: &str =
    include_str!("../../../../crates/elohim-address/vectors/address-vectors.json");

/// The chunker fixtures' generator, frozen with the byte profile (design gate CDC 2): xorshift64*
/// (shifts 12/25/27, multiplier 0x2545F4914F6CDD1D) from the seed, each step's 64-bit product
/// emitted little-endian, the stream truncated to `len`. A twin of
/// `elohim/epr/tests/chunker_vectors.rs::xorshift_bytes`, read from the section's own header.
fn xorshift_bytes(seed: u64, multiplier: u64, len: usize) -> Vec<u8> {
    let mut state = seed;
    let mut out = Vec::with_capacity(len + 8);
    while out.len() < len {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        out.extend_from_slice(&state.wrapping_mul(multiplier).to_le_bytes());
    }
    out.truncate(len);
    out
}

fn hex_u64(text: &str) -> u64 {
    u64::from_str_radix(text.trim_start_matches("0x"), 16).unwrap()
}

#[derive(Default)]
struct Store(HashMap<BlobCid, Bytes>);

#[async_trait]
impl EprfsStorage for Store {
    async fn resolve_epr(&self, r: &EprRef) -> eprfs_core::Result<EprRecord> {
        Err(EprfsError::EprNotFound(r.clone()))
    }
    async fn has_blob(&self, cid: &BlobCid) -> eprfs_core::Result<BlobPresence> {
        Ok(if self.0.contains_key(cid) {
            BlobPresence::Local
        } else {
            BlobPresence::Missing
        })
    }
    async fn fetch_blob(&self, cid: &BlobCid, _: FetchPolicy) -> eprfs_core::Result<BlobHandle> {
        self.0
            .get(cid)
            .map(|bytes| BlobHandle {
                cid: cid.clone(),
                bytes: bytes.clone(),
            })
            .ok_or_else(|| EprfsError::BlobNotFound(cid.clone()))
    }
    async fn put_blob(&self, _: Bytes) -> eprfs_core::Result<BlobCid> {
        unreachable!()
    }
    async fn publish_attestation(&self, _: AttestationDraft) -> eprfs_core::Result<EprRef> {
        unreachable!()
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

/// A fixture tree: a string is a file's UTF-8 content; an object is a directory, except the two
/// forms `{"$external": {"kind", "referenceHex"}}` and `{"$symlink": target}`. A key starting
/// `hex:` names a non-UTF-8 entry by its hex bytes.
fn build(
    store: &mut Store,
    fixture: &Value,
    paths: &mut BTreeMap<TreePath, BlobCid>,
    at: &TreePath,
) -> BlobCid {
    let Value::Object(entries) = fixture else {
        panic!("a fixture tree is an object")
    };
    let mut out = Vec::new();
    for (key, value) in entries {
        let name = key
            .strip_prefix("hex:")
            .map(unhex)
            .unwrap_or_else(|| key.as_bytes().to_vec());
        let kind = match value {
            Value::String(content) => {
                let cid = BlobCid::compute_raw(content.as_bytes());
                store
                    .0
                    .insert(cid.clone(), Bytes::from(content.clone().into_bytes()));
                TreeEntryKind::File {
                    blob: cid.into(),
                    executable: false,
                }
            }
            Value::Object(map) if map.contains_key("$external") => {
                let e = &map["$external"];
                TreeEntryKind::External {
                    kind: e["kind"].as_str().unwrap().to_string(),
                    reference: unhex(e["referenceHex"].as_str().unwrap()),
                }
            }
            Value::Object(map) if map.contains_key("$symlink") => {
                let target = map["$symlink"].as_str().unwrap();
                let cid = BlobCid::compute_raw(target.as_bytes());
                store
                    .0
                    .insert(cid.clone(), Bytes::from(target.as_bytes().to_vec()));
                TreeEntryKind::Symlink { target: cid.into() }
            }
            Value::Object(_) => {
                let child = build(store, value, paths, &at.join(&name));
                TreeEntryKind::Directory { tree: child.into() }
            }
            other => panic!("unexpected fixture value {other}"),
        };
        out.push(TreeEntry { name, kind });
    }
    let (cid, bytes) = encode_tree(&TreeNode::new(out).unwrap(), &TreeLimits::default()).unwrap();
    store.0.insert(cid.clone(), Bytes::from(bytes));
    paths.insert(at.clone(), cid.clone());
    cid
}

fn path_text(path: &TreePath) -> String {
    path.to_slash_string().unwrap_or_else(|_| {
        path.components()
            .iter()
            .map(|c| match std::str::from_utf8(c) {
                Ok(text) => text.to_string(),
                Err(_) => format!("hex:{}", hex(c)),
            })
            .collect::<Vec<_>>()
            .join("/")
    })
}

fn parse_path(text: &str) -> TreePath {
    if text.is_empty() {
        return TreePath::root();
    }
    TreePath::from_components(
        text.split('/')
            .map(|c| {
                c.strip_prefix("hex:")
                    .map(unhex)
                    .unwrap_or_else(|| c.as_bytes().to_vec())
            })
            .collect(),
    )
    .unwrap()
}

fn paths_of(value: &Value) -> impl Iterator<Item = TreePath> + '_ {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|p| parse_path(p.as_str().unwrap()))
}

fn coverage(c: Coverage) -> &'static str {
    match c {
        Coverage::InView => "in-view",
        Coverage::Spine => "spine",
        Coverage::Outside => "outside",
    }
}

/// The walk's result in the shape both suites compare (interfaces spec §H.1's mapping table).
fn walked_json(walked: &WalkedView) -> Value {
    let nodes: Vec<Value> = walked
        .nodes
        .iter()
        .map(|n| json!({"path": path_text(&n.path), "coverage": coverage(n.coverage)}))
        .collect();
    let boundaries: Vec<Value> = walked
        .boundaries
        .iter()
        .map(|b| match b {
            WalkItem::Boundary {
                path,
                kind,
                coverage: c,
                ..
            } => json!({"path": path_text(path), "kind": kind, "coverage": coverage(*c)}),
            other => panic!("not a boundary: {other:?}"),
        })
        .collect();
    let absent: Vec<Value> = walked
        .absent
        .iter()
        .map(|a| match a {
            WalkItem::Absent {
                view_path,
                reason,
                unanswered,
                ..
            } => {
                let mut v = match reason {
                    AbsentReason::NotFound { missing_at, .. } => {
                        json!({"kind": "not-found", "path": path_text(missing_at)})
                    }
                    AbsentReason::ThroughLeaf { leaf, .. } => {
                        json!({"kind": "through-leaf", "leaf": path_text(leaf)})
                    }
                    AbsentReason::NotHeld {
                        missing_at, cid, ..
                    }
                    | AbsentReason::Withheld {
                        missing_at, cid, ..
                    } => json!({
                        "kind": "absent",
                        "path": path_text(missing_at),
                        "cid": cid.to_string(),
                        "unanswered": serde_json::to_value(unanswered.as_ref().unwrap()).unwrap(),
                    }),
                    other => panic!("unexpected reason {other:?}"),
                };
                v["viewPath"] = json!(path_text(view_path));
                v
            }
            other => panic!("not an absence: {other:?}"),
        })
        .collect();
    json!({"nodes": nodes, "boundaries": boundaries, "absent": absent})
}

fn change_json(change: &TreeChange) -> Value {
    json!({
        "path": path_text(&change.path),
        "old": change.old.as_ref().map(|k| serde_json::to_value(k).unwrap()),
        "new": change.new.as_ref().map(|k| serde_json::to_value(k).unwrap()),
    })
}

async fn recompute(file: &Value) -> Value {
    let mut out = file.clone();

    // The frozen tag-42 link and the fixture trees' canonical bytes.
    let link =
        eprfs_core::BlobLink::from(BlobCid::parse(file["link"]["cid"].as_str().unwrap()).unwrap());
    out["link"]["dagCborHex"] = json!(hex(&serde_ipld_dagcbor::to_vec(&link).unwrap()));
    for (i, tree) in file["trees"].as_array().unwrap().iter().enumerate() {
        let mut store = Store::default();
        let root = build(
            &mut store,
            &tree["tree"],
            &mut BTreeMap::new(),
            &TreePath::root(),
        );
        out["trees"][i]["cid"] = json!(root.to_string());
        out["trees"][i]["bytesHex"] = json!(hex(&store.0[&root]));
    }

    // Walk vectors over one fixture tree, the depth-first and the level-batched walk agreeing.
    let mut store = Store::default();
    let mut paths = BTreeMap::new();
    let root = build(
        &mut store,
        &file["walk"]["tree"],
        &mut paths,
        &TreePath::root(),
    );
    out["walk"]["root"] = json!(root.to_string());
    for (i, case) in file["walk"]["cases"].as_array().unwrap().iter().enumerate() {
        let mut held = store.0.clone();
        for missing in paths_of(&case["missing"]) {
            held.retain(|cid, _| cid != &paths[&missing]);
        }
        let case_store = Store(held);
        let view = View::from_tree_paths(paths_of(&case["view"]));
        let opts = FetchPolicy::LocalOnly.into();
        let depth_first = walk_collect(&case_store, &root, &view, &opts, &TreeLimits::default())
            .await
            .unwrap();
        let batched = walk_batched(&case_store, &root, &view, &opts, &TreeLimits::default())
            .await
            .unwrap();
        let expect = walked_json(&depth_first);
        assert_eq!(
            expect,
            walked_json(&batched),
            "case {i}: the two walks disagree"
        );
        out["walk"]["cases"][i]["expect"] = expect;
    }

    // Projected-root P vectors (§C C6): sparse roots and every re-encoded spine node.
    let mut project_store = Store::default();
    let project_root = build(
        &mut project_store,
        &file["project"]["tree"],
        &mut BTreeMap::new(),
        &TreePath::root(),
    );
    for (i, case) in file["project"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let view = View::from_tree_paths(paths_of(&case["view"]));
        let spine = walk_spine(
            &project_store,
            &project_root,
            &view,
            &BatchOptions::default(),
            &TreeLimits::default(),
        )
        .await
        .unwrap();
        let (projected, nodes) = project_view(&spine, &TreeLimits::default()).unwrap();
        out["project"]["cases"][i]["expect"] = json!({
            "root": projected.cid().to_string(),
            "nodes": nodes
                .iter()
                .map(|(cid, bytes)| json!({"cid": cid.to_string(), "bytesHex": hex(bytes)}))
                .collect::<Vec<_>>(),
        });
    }

    // Diff vectors.
    for (i, case) in file["diff"].as_array().unwrap().iter().enumerate() {
        let mut store = Store::default();
        let old = build(
            &mut store,
            &case["old"],
            &mut BTreeMap::new(),
            &TreePath::root(),
        );
        let new = build(
            &mut store,
            &case["new"],
            &mut BTreeMap::new(),
            &TreePath::root(),
        );
        let view = View::from_tree_paths(paths_of(&case["view"]));
        let changes = eprfs_core::diff_trees(
            &store,
            &old,
            &new,
            &view,
            &FetchPolicy::LocalOnly.into(),
            &TreeLimits::default(),
        )
        .await
        .unwrap();
        out["diff"][i]["expect"] = Value::Array(changes.iter().map(change_json).collect());
    }

    // TreeView: normalization, the decode refusal, and the canonical CID.
    let tv = &file["treeView"];
    let tv_root = BlobCid::parse(tv["root"].as_str().unwrap()).unwrap();
    let normalized = TreeView::new(
        tv_root.clone(),
        &View::from_tree_paths(paths_of(&tv["declared"])),
    );
    out["treeView"]["normalized"] = serde_json::to_value(&normalized).unwrap();
    out["treeView"]["cid"] = json!(normalized.cid().unwrap().to_string());
    out["treeView"]["dagCborHex"] = json!(hex(&serde_ipld_dagcbor::to_vec(&normalized).unwrap()));
    let refused = json!({"root": tv_root.to_string(), "paths": tv["declared"]});
    assert!(
        serde_json::from_value::<TreeView>(refused).is_err(),
        "a covered path list decodes"
    );

    // The requester-side ask rule (§A.6), one parameterized rule for every host: each case's
    // verdict from eprfs-core's `OpennessAsk`, which the TypeScript `standingPolicy` must match.
    let levels = &file["ask"]["levels"];
    let level = |v: &Value| ReachLevel(v.as_u64().unwrap() as u8);
    let ask = OpennessAsk {
        household: level(&levels["household"]),
        public: level(&levels["public"]),
    };
    let ask_root = BlobCid::compute(b"ask root");
    for (i, case) in file["ask"]["cases"].as_array().unwrap().iter().enumerate() {
        let record = TierRecord {
            id: TierId::agent(format!("uhCAkask-{i}")),
            class: serde_json::from_value(case["class"].clone()).unwrap(),
            sources: vec![TierSource::HeldRecord {
                record: "vectors".into(),
            }],
            endpoint: None,
            standing: case["standing"].as_u64().map(|v| ReachLevel(v as u8)),
            proximity: None,
        };
        let root_reach = case["rootReach"].as_u64().map(|v| ReachLevel(v as u8));
        out["ask"]["cases"][i]["expect"] = json!(ask.may_ask(&record, &ask_root, root_reach));
    }

    // The default order across two roots (§A.2, H17): warmth is per root, so the same records
    // order differently for each, which a chain ordering once at construction would miss.
    let order = &file["order"];
    let roots: Vec<BlobCid> = order["rootSeeds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|seed| BlobCid::compute(seed.as_str().unwrap().as_bytes()))
        .collect();
    let records: Vec<TierRecord> = order["tiers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| TierRecord {
            id: serde_json::from_value(t["id"].clone()).unwrap(),
            class: serde_json::from_value(t["class"].clone()).unwrap(),
            sources: vec![TierSource::HeldRecord {
                record: "vectors".into(),
            }],
            endpoint: None,
            standing: None,
            proximity: Some(Proximity {
                network_rank: t["networkRank"].as_u64().unwrap() as u8,
                warm_roots: t["warm"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|i| roots[i.as_u64().unwrap() as usize].clone())
                    .collect(),
                serves_extracted: false,
                rtt_ms: t["rttMs"].as_u64().map(|v| v as u32),
                last_seen_ms: None,
            }),
        })
        .collect();
    out["order"]["roots"] = json!(roots.iter().map(|r| r.to_string()).collect::<Vec<_>>());
    out["order"]["records"] = serde_json::to_value(&records).unwrap();
    out["order"]["expect"] = json!(roots
        .iter()
        .map(|root| {
            let mut ordered = records.clone();
            ProximityOrder.order(root, &mut ordered);
            ordered
                .iter()
                .map(|r| serde_json::to_value(&r.id).unwrap())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>());

    // Wire types: one TierRecord, one of each ItemAbsence, one Unanswered.
    let hub = TierId::agent("uhCAkhub");
    let record = TierRecord {
        id: hub.clone(),
        class: TierClass::Hub,
        sources: vec![
            TierSource::HeldRecord {
                record: "household-roster".into(),
            },
            TierSource::Doorway {
                doorway: TierId::agent("uhCAkdoorway-w1"),
            },
        ],
        endpoint: Some(TierAddress::Url {
            base: "https://hub.dowell.example".into(),
        }),
        standing: Some(ReachLevel(2)),
        proximity: Some(Proximity {
            network_rank: 0,
            warm_roots: vec![root.clone()],
            serves_extracted: true,
            rtt_ms: Some(4),
            last_seen_ms: None,
        }),
    };
    out["wire"]["tierRecord"] = serde_json::to_value(&record).unwrap();
    let absences: Vec<ItemAbsence> = vec![
        ItemAbsence::NotHeld {
            tried: vec![hub.clone(), TierId::transport("12D3KooWpeer")],
        },
        ItemAbsence::Refused {
            by: hub.clone(),
            refusal: ReachRefusal::new(ReachReason::OutsideStanding, Some("household".into())),
        },
        ItemAbsence::OverBudget {
            by: hub.clone(),
            rule: "household-forward".into(),
        },
        ItemAbsence::Unreachable { tier: hub.clone() },
        ItemAbsence::MembershipUnknown { by: hub.clone() },
        ItemAbsence::Integrity { from: hub.clone() },
    ];
    out["wire"]["itemAbsences"] = serde_json::to_value(&absences).unwrap();
    let unanswered = ItemAbsence::not_held(vec![hub.clone()])
        .escalate("nothing".into(), vec!["uhCAkjessica".into()]);
    out["wire"]["unanswered"] = serde_json::to_value(&unanswered).unwrap();
    out["wire"]["keyCustody"] = serde_json::to_value(KeyCustody::DoorwayHeld).unwrap();
    // The byte profile's wire form, as every layout record (the tier blob manifest's `profile`,
    // the store index) persists it.
    out["wire"]["byteProfile"] = serde_json::to_value(elohim_epr::profile::BYTE_PROFILE_0).unwrap();

    out["address"] = address_section();
    recompute_chunker(&mut out);
    out
}

/// The byte-CID section: the kernel's positive inputs (and the cite-fingerprint body) through
/// eprfs's own constructors, and the kernel's negative rows, each refused at the byte surface
/// with the check the kernel names.
fn address_section() -> Value {
    let kernel: Value = serde_json::from_str(KERNEL_VECTORS).expect("kernel vectors parse");
    let mut positive = Vec::new();
    let inputs = kernel["positive"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["input"].as_str().unwrap().to_string())
        .chain(std::iter::once("ok target body".to_string()));
    for input in inputs {
        let bytes = input.as_bytes();
        let byte = BlobCid::compute_bytes(bytes);
        let cite = BlobCid::compute_raw(bytes);
        let record = BlobCid::compute(bytes);
        assert!(byte.verifies(bytes) && cite.verifies(bytes) && record.verifies(bytes));
        assert!(
            byte.byte_cid().is_ok(),
            "{byte} is admitted at the byte surface"
        );
        assert_eq!(
            byte.short_fingerprint().unwrap_err().check(),
            "not-sha2",
            "a BLAKE3 address never renders a sha256 fingerprint"
        );
        let row = json!({
            "input": input,
            "byteCid": byte.to_string(),
            "citeCid": cite.to_string(),
            "recordCid": record.to_string(),
            "shortFingerprint": cite.short_fingerprint().unwrap(),
        });
        // eprfs and the kernel's independently derived rows agree, field for field.
        if let Some(pinned) = kernel["positive"]
            .as_array()
            .unwrap()
            .iter()
            .find(|k| k["input"] == row["input"])
        {
            for field in ["byteCid", "citeCid", "recordCid", "shortFingerprint"] {
                assert_eq!(row[field], pinned[field], "{field} for {input:?}");
            }
        }
        positive.push(row);
    }
    let negative: Vec<Value> = kernel["negative"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let input = row["input"].as_str().unwrap();
            let check = row["check"].as_str().unwrap();
            let refused = ByteCid::parse(input).expect_err(input);
            assert_eq!(refused.check(), check, "{}", row["name"]);
            // A profile refusal holds for the parsed CID too. (Spelling refusals — case, padding,
            // alphabet — are text checks: a generic CID parse normalizes them away, which is why
            // text at a byte surface goes through `ByteCid::parse`, never `BlobCid::parse`.)
            if matches!(check, "version" | "codec" | "multihash" | "digest-length") {
                if let Ok(parsed) = BlobCid::parse(input) {
                    assert!(parsed.byte_cid().is_err(), "{}", row["name"]);
                }
            }
            json!({ "name": row["name"], "input": input, "check": check })
        })
        .collect();
    json!({
        "about": "Byte-CID vectors (re-genesis 1, amended R1). byteCid = raw 0x55 + BLAKE3-256 (payload bytes); citeCid = raw 0x55 + sha2-256 (citations only; the short fingerprint renders it); recordCid = dag-cbor 0x71 + sha2-256. Positive inputs are the elohim-address kernel's plus the cite-fingerprint body; negatives are the kernel's byte-surface refusals with the check that fails. Compare-only.",
        "positive": positive,
        "negative": negative,
    })
}

/// Re-derive the `chunker` section's chunk layouts and CIDs from its own header (generator,
/// parameters, table digest) through the byte profile's chunker and eprfs's byte constructor.
fn recompute_chunker(out: &mut Value) {
    use elohim_epr::profile::{
        chunk_bytes, fastcdc_2020_table_digest, Chunker, BYTE_PROFILE_0, CHUNKER_FASTCDC_2020,
    };
    let section = &out["chunker"];
    let Chunker::Fastcdc2020 {
        min,
        avg,
        max,
        level,
    } = BYTE_PROFILE_0.chunker;
    assert_eq!(
        section["chunker"],
        json!({"id": CHUNKER_FASTCDC_2020, "min": min, "avg": avg, "max": max, "level": level}),
        "the section declares the epoch-0 chunker"
    );
    assert_eq!(section["profileEpoch"], json!(BYTE_PROFILE_0.epoch));
    assert_eq!(
        section["tableDigest"].as_str().unwrap(),
        fastcdc_2020_table_digest()
    );
    let generator = &section["generator"];
    let seed = hex_u64(generator["seed"].as_str().unwrap());
    let multiplier = hex_u64(generator["multiplier"].as_str().unwrap());
    let mut derived = Vec::new();
    for vector in section["vectors"].as_array().unwrap() {
        let len = vector["length"].as_u64().unwrap() as usize;
        let bytes = match vector["source"].as_str().unwrap() {
            "xorshift64*" => xorshift_bytes(seed, multiplier, len),
            "zeros" => vec![0u8; len],
            "pattern" => vector["pattern"]
                .as_str()
                .unwrap()
                .bytes()
                .cycle()
                .take(len)
                .collect(),
            other => panic!("unknown chunker fixture source {other}"),
        };
        let whole = BlobCid::compute_bytes(&bytes);
        let mut vector = vector.clone();
        vector["inputBlake3"] = json!(hex(whole.as_cid().hash().digest()));
        vector["wholeCid"] = json!(whole.to_string());
        vector["chunks"] = Value::Array(
            chunk_bytes(&BYTE_PROFILE_0, &bytes)
                .into_iter()
                .map(|(offset, len)| {
                    json!([
                        offset,
                        len,
                        BlobCid::compute_bytes(&bytes[offset..offset + len]).to_string()
                    ])
                })
                .collect(),
        );
        derived.push(vector);
    }
    out["chunker"]["vectors"] = Value::Array(derived);
}

/// Where `computed` differs from `pinned`: every present entry must be unchanged; an entry
/// absent from `pinned` is an addition. Returns the merged file or the paths that would change.
fn merge_additive(pinned: &Value, computed: &Value, at: &str) -> Result<Value, Vec<String>> {
    match (pinned, computed) {
        (Value::Object(old), Value::Object(new)) => {
            let mut merged = old.clone();
            let mut changed = Vec::new();
            for (key, value) in new {
                let here = format!("{at}/{key}");
                match old.get(key) {
                    None => {
                        merged.insert(key.clone(), value.clone());
                    }
                    Some(prior) => match merge_additive(prior, value, &here) {
                        Ok(value) => {
                            merged.insert(key.clone(), value);
                        }
                        Err(paths) => changed.extend(paths),
                    },
                }
            }
            for key in old.keys().filter(|k| !new.contains_key(*k)) {
                changed.push(format!("{at}/{key} (removed)"));
            }
            if changed.is_empty() {
                Ok(Value::Object(merged))
            } else {
                Err(changed)
            }
        }
        (Value::Array(old), Value::Array(new)) if new.len() >= old.len() => {
            let mut merged = Vec::with_capacity(new.len());
            let mut changed = Vec::new();
            for (i, value) in new.iter().enumerate() {
                match old.get(i) {
                    None => merged.push(value.clone()),
                    Some(prior) => match merge_additive(prior, value, &format!("{at}/{i}")) {
                        Ok(value) => merged.push(value),
                        Err(paths) => changed.extend(paths),
                    },
                }
            }
            if changed.is_empty() {
                Ok(Value::Array(merged))
            } else {
                Err(changed)
            }
        }
        (old, new) if old == new => Ok(old.clone()),
        _ => Err(vec![at.to_string()]),
    }
}

#[tokio::test]
async fn shared_vectors_match_what_this_crate_computes() {
    let file: Value = serde_json::from_str(FILE).expect("tree-vectors.json parses");
    let computed = recompute(&file).await;
    // Every wire vector decodes back into its Rust type.
    let _: TierRecord = serde_json::from_value(computed["wire"]["tierRecord"].clone()).unwrap();
    let _: Vec<ItemAbsence> =
        serde_json::from_value(computed["wire"]["itemAbsences"].clone()).unwrap();
    if computed != file {
        let pretty = serde_json::to_string_pretty(&computed).unwrap() + "\n";
        let merged = match merge_additive(&file, &computed, "") {
            Ok(merged) => merged,
            Err(changed) => panic!(
                "tree-vectors.json is compare-only: this crate now computes a different value for \
                 {changed:?}. A changed expectation needs review and, on behaviour change, a new \
                 profile epoch; EPRFS_WRITE_VECTORS=1 never rewrites a pinned entry. Recomputed:\n\
                 {pretty}"
            ),
        };
        if std::env::var_os("EPRFS_WRITE_VECTORS").is_some() {
            let path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/tree-vectors.json");
            std::fs::write(path, serde_json::to_string_pretty(&merged).unwrap() + "\n").unwrap();
            return;
        }
        panic!(
            "tree-vectors.json lacks entries this crate computes; add them once with \
             EPRFS_WRITE_VECTORS=1. Recomputed:\n{pretty}"
        );
    }
}

#[test]
fn write_vectors_refuses_to_change_a_pinned_entry() {
    let pinned = json!({
        "address": {"positive": [{"input": "abc", "byteCid": "bafkr4i-pinned"}]},
        "trees": [{"cid": "bafyrei-pinned"}],
    });
    // An addition (a new section, a new row, a new field) merges.
    let added = json!({
        "address": {"positive": [
            {"input": "abc", "byteCid": "bafkr4i-pinned", "citeCid": "bafkrei-new"},
            {"input": "new"},
        ]},
        "chunker": {"vectors": []},
        "trees": [{"cid": "bafyrei-pinned"}],
    });
    assert_eq!(merge_additive(&pinned, &added, ""), Ok(added.clone()));
    // A changed epoch-0 value, a removed entry and a shortened list are each refused by path.
    let changed = json!({
        "address": {"positive": [{"input": "abc", "byteCid": "bafkr4i-CHANGED"}]},
        "trees": [{"cid": "bafyrei-pinned"}],
    });
    assert_eq!(
        merge_additive(&pinned, &changed, ""),
        Err(vec!["/address/positive/0/byteCid".to_string()])
    );
    let removed = json!({ "trees": [{"cid": "bafyrei-pinned"}] });
    assert_eq!(
        merge_additive(&pinned, &removed, ""),
        Err(vec!["/address (removed)".to_string()])
    );
    let shortened = json!({
        "address": {"positive": []},
        "trees": [{"cid": "bafyrei-pinned"}],
    });
    assert!(merge_additive(&pinned, &shortened, "").is_err());
}
