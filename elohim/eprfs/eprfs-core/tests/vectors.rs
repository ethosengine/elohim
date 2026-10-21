//! The shared vectors (`elohim/eprfs/fixtures/tree-vectors.json`, interfaces spec §H.1): byte
//! compatibility between this crate and `@elohim/eprfs`, pinned by one file both suites read.
//!
//! This test recomputes every vector from its inputs and compares it with the file. When they
//! differ it fails and prints the recomputed file; `EPRFS_WRITE_VECTORS=1` rewrites it. The
//! TypeScript suite reads the same file and must reproduce the same values.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use async_trait::async_trait;
use bytes::Bytes;
use eprfs_core::{
    encode_tree, walk_batched, walk_collect, AbsentReason, AskPolicy, AttestationDraft, BlobCid,
    BlobHandle, BlobPresence, Coverage, EprRecord, EprRef, EprfsError, EprfsStorage, FetchPolicy,
    ItemAbsence, KeyCustody, OpennessAsk, Proximity, ProximityOrder, ReachLevel, ReachReason,
    ReachRefusal, TierAddress, TierClass, TierId, TierOrder, TierRecord, TierSource, TreeChange,
    TreeEntry, TreeEntryKind, TreeLimits, TreeNode, TreePath, TreeView, View, WalkItem, WalkedView,
};
use serde_json::{json, Value};

const FILE: &str = include_str!("../../fixtures/tree-vectors.json");

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
    out
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
        if std::env::var_os("EPRFS_WRITE_VECTORS").is_some() {
            let path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/tree-vectors.json");
            std::fs::write(path, pretty).unwrap();
            return;
        }
        panic!(
            "tree-vectors.json is stale; rerun with EPRFS_WRITE_VECTORS=1. Recomputed:\n{pretty}"
        );
    }
}
