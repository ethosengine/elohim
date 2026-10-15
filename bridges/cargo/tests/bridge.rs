//! The cargo bridge over a hand-written fixture workspace: one member (`app`) with a registry
//! dependency (`serde`), its transitive registry dependency (`serde_derive`, the same upstream
//! project), a registry dependency with no repository (`itoa`), an in-repo path dependency
//! (`localutil`) and a git dependency (`gitcrate`). No cargo is run.

use std::collections::BTreeSet;

use cargo_bridge::metadata::{parse_lockfile, parse_metadata};
use cargo_bridge::{observe, reliance, translate, verify, Config, Inputs, Origin};
use elohim_epr_rea::{ExternalClaim, FlowRecord, FlowStore, Governor, ReaVerb, SidecarFlowStore};

const METADATA: &str = include_str!("fixtures/ws/metadata.json");
const LOCKFILE: &str = include_str!("fixtures/ws/Cargo.lock");
const AT: &str = "2026-10-15T00:00:00Z";
/// Strings from the fixture's `authors` fields: none may reach any record or the offer.
const AUTHOR_STRINGS: [&str; 8] = [
    "Erin Example",
    "erin.example@fixture-mail.example",
    "Dana Fixture",
    "dana@fixture-mail.example",
    "Ira Undisclosed",
    "Gita Committer",
    "fixture-mail.example",
    "member@fixture.example",
];

fn config() -> Config {
    Config::for_repository("fixture-ws", "/fixture/repo", AT)
}

fn inputs() -> Inputs {
    Inputs {
        metadata: METADATA.to_string(),
        lockfile: LOCKFILE.as_bytes().to_vec(),
    }
}

fn flows(dir: &std::path::Path) -> Vec<u8> {
    std::fs::read(dir.join(".eprfs/status/flows.jsonl")).unwrap()
}

#[test]
fn one_cite_input_per_resolved_package_and_a_deterministic_process() {
    let r = reliance(&inputs(), &config()).unwrap();
    // 6 resolved, 1 member → 5 cited, each the Process's input.
    assert_eq!(r.report.packages_resolved, 6);
    assert_eq!(r.report.workspace_members, 1);
    assert_eq!(r.report.cited, 5);
    assert_eq!(r.events.len(), 5);
    assert_eq!(r.process.inputs.len(), 5);
    assert!(r.events.iter().all(|e| e.action == ReaVerb::Cite));
    for (event, input) in r.events.iter().zip(&r.process.inputs) {
        assert_eq!(&FlowRecord::Event(event.clone()).cid().unwrap(), input);
        assert_eq!(event.in_scope_of, r.offer_cid);
        assert_eq!(event.receiver.0, "collective:ethosengine/elohim");
        let claim = ExternalClaim::read(&event.classified_as).unwrap().unwrap();
        assert_eq!(claim.source(), "service:cargo");
        assert_eq!(claim.provenance(), "ws/Cargo.lock");
    }
    assert_eq!(
        (r.report.registry_with_checksum, r.report.git, r.report.path),
        (3, 1, 1)
    );

    // Deterministic: a second translation is the same records, address for address.
    let again = reliance(&inputs(), &config()).unwrap();
    assert_eq!(again.process_cid, r.process_cid);
    assert_eq!(again.offer.text(), r.offer.text());
}

#[test]
fn registry_inputs_are_the_cid_wrapping_the_lockfile_checksum() {
    let r = reliance(&inputs(), &config()).unwrap();
    let checksums = parse_lockfile(LOCKFILE).unwrap();
    let mut seen = 0;
    for c in r.cited.iter().filter(|c| c.origin == Origin::Registry) {
        let (name, version) = c
            .coordinate
            .strip_prefix("crates.io:")
            .and_then(|s| s.split_once('@'))
            .unwrap();
        let hex = checksums
            .get(&(
                name.to_string(),
                version.to_string(),
                "registry+https://github.com/rust-lang/crates.io-index".to_string(),
            ))
            .unwrap();
        let expected = elohim_epr::cid::raw_cid_from_sha256_hex(hex).unwrap();
        assert_eq!(c.resource, expected.to_string());
        assert_eq!(c.identity, "checksum");
        assert!(c.resource.starts_with("bafkrei"));
        seen += 1;
    }
    assert_eq!(seen, 3);
}

#[test]
fn path_and_git_packages_are_coordinate_only_and_never_get_an_invented_byte_cid() {
    let r = reliance(&inputs(), &config()).unwrap();
    for c in r.cited.iter().filter(|c| c.origin != Origin::Registry) {
        assert_eq!(c.identity, "coordinate-only", "{}", c.coordinate);
        let cid: cid::Cid = c.resource.parse().unwrap();
        assert_eq!(
            cid.codec(),
            0x71,
            "a coordinate record, not a raw byte address"
        );
    }
    let coordinates: BTreeSet<&str> = r.cited.iter().map(|c| c.coordinate.as_str()).collect();
    assert!(coordinates.contains("path:libs/localutil"));
    assert!(coordinates.contains("git:gitcrate@0.3.0#0123456789abcdef0123456789abcdef01234567"));
    for event in &r.events {
        let coordinate = &event.classified_as[1];
        let identity = event
            .classified_as
            .iter()
            .find_map(|s| s.strip_prefix("identity:"))
            .unwrap();
        let expect = if coordinate.starts_with("crates.io:") {
            "checksum"
        } else {
            "coordinate-only"
        };
        assert_eq!(identity, expect, "{coordinate}");
    }
}

#[test]
fn one_presence_per_upstream_project_never_a_person() {
    let r = reliance(&inputs(), &config()).unwrap();
    let inflows: Vec<&str> = r
        .offer
        .declaration()
        .disclosure
        .inflows
        .iter()
        .map(|i| i.presence.as_str())
        .collect();
    // serde and serde_derive share one project; itoa falls back to its crate name; the git
    // package is named by its git URL; the in-repo path package and the repository's own
    // packages get no presence at all.
    assert_eq!(
        inflows,
        [
            "presence:crates-io/itoa",
            "presence:github-com/example/gitcrate",
            "presence:github-com/serde-rs/serde",
        ]
    );
    let provider_of = |coordinate: &str| {
        r.events
            .iter()
            .find(|e| e.classified_as[1] == coordinate)
            .map(|e| e.provider.0.clone())
            .unwrap()
    };
    assert_eq!(
        provider_of("crates.io:serde_derive@1.0.200"),
        "presence:github-com/serde-rs/serde"
    );
    assert_eq!(
        provider_of("path:libs/localutil"),
        "repo:ethosengine/elohim"
    );

    // No author name or email reaches any record or the offer document.
    let mut everything = r.offer.text().to_string();
    for record in r.records() {
        everything.push_str(&serde_json::to_string(&record).unwrap());
    }
    for author in AUTHOR_STRINGS {
        assert!(
            !everything.contains(author),
            "`{author}` leaked into a record"
        );
    }
}

#[test]
fn each_input_carries_its_declared_license() {
    let r = reliance(&inputs(), &config()).unwrap();
    let license_of = |coordinate: &str| {
        r.events
            .iter()
            .find(|e| e.classified_as[1] == coordinate)
            .and_then(|e| {
                e.classified_as
                    .iter()
                    .find_map(|s| s.strip_prefix("license:").map(str::to_string))
            })
            .unwrap()
    };
    assert_eq!(license_of("crates.io:serde@1.0.200"), "MIT OR Apache-2.0");
    assert_eq!(
        license_of("git:gitcrate@0.3.0#0123456789abcdef0123456789abcdef01234567"),
        "MPL-2.0"
    );
    assert_eq!(license_of("path:libs/localutil"), "undeclared");
}

#[test]
fn direct_member_edges_to_registry_packages_name_their_bytes() {
    let r = reliance(&inputs(), &config()).unwrap();
    let edges: Vec<(&str, &str)> = r
        .edges
        .iter()
        .map(|e| (e.from.as_str(), e.to.as_str()))
        .collect();
    // serde_derive is transitive (no member edge); path and git deps carry no checksum.
    assert_eq!(
        edges,
        [
            ("ws/app", "crates.io:itoa@1.0.11"),
            ("ws/app", "crates.io:serde@1.0.200"),
        ]
    );
    for edge in &r.edges {
        assert_eq!(edge.governor, Governor::Compiler("fixture-ws".into()));
        assert!(edge.sealed_cid.is_none());
        let cited = r.cited.iter().find(|c| c.coordinate == edge.to).unwrap();
        assert_eq!(edge.to_resource.unwrap().to_string(), cited.resource);
        edge.validate().unwrap();
    }
}

#[test]
fn observe_is_idempotent_and_two_fresh_stores_are_byte_identical() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let first = observe(a.path(), &inputs(), &config()).unwrap();
    // offer Intent + 5 events + Process + 2 edges.
    assert_eq!((first.appended, first.present), (9, 0));
    let bytes = flows(a.path());

    let again = observe(a.path(), &inputs(), &config()).unwrap();
    assert_eq!((again.appended, again.present), (0, 9));
    assert_eq!(
        flows(a.path()),
        bytes,
        "a rerun leaves flows.jsonl byte-identical"
    );

    observe(b.path(), &inputs(), &config()).unwrap();
    assert_eq!(
        flows(b.path()),
        bytes,
        "two runs, two stores, the same bytes"
    );

    // What was written reads back through the shared store, CIDs re-verified.
    let records = SidecarFlowStore::open(a.path()).unwrap().records().unwrap();
    assert_eq!(records.len(), 9);
    let process = records
        .iter()
        .find_map(|(_, r)| match r {
            FlowRecord::Process(p) => Some(p.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(process.inputs.len(), 5);
    let text = String::from_utf8(bytes).unwrap();
    for author in AUTHOR_STRINGS {
        assert!(!text.contains(author), "`{author}` in flows.jsonl");
    }
}

#[test]
fn verify_is_fresh_after_observe_and_stale_after_a_lockfile_bump_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let stale_empty = verify(dir.path(), &inputs(), &config()).unwrap();
    assert!(!stale_empty.is_fresh());
    assert!(!dir.path().join(".eprfs").exists(), "verify never writes");

    observe(dir.path(), &inputs(), &config()).unwrap();
    let fresh = verify(dir.path(), &inputs(), &config()).unwrap();
    assert!(fresh.is_fresh(), "{}", fresh.notice());

    // Bump itoa without regenerating: the record is stale and the notice names the workspace.
    let before = flows(dir.path());
    let bumped = Inputs {
        metadata: METADATA
            .replace("itoa@1.0.11", "itoa@1.0.12")
            .replace("\"version\": \"1.0.11\"", "\"version\": \"1.0.12\""),
        lockfile: LOCKFILE
            .replace("version = \"1.0.11\"", "version = \"1.0.12\"")
            .into_bytes(),
    };
    let stale = verify(dir.path(), &bumped, &config()).unwrap();
    assert!(!stale.is_fresh());
    assert!(stale.notice().starts_with("notice:"));
    assert!(stale.notice().contains("`fixture-ws`"));
    assert_eq!(flows(dir.path()), before, "verify never writes");
}

#[test]
fn malformed_input_refuses_with_nothing_written() {
    let dir = tempfile::tempdir().unwrap();
    for (metadata, lockfile, why) in [
        ("{", LOCKFILE, "malformed JSON"),
        (
            &*METADATA.replace("\"version\": 1,", "\"version\": 2,"),
            LOCKFILE,
            "unknown metadata version",
        ),
        (METADATA, "[[package]\n", "malformed lockfile"),
        (
            &*LOCKFILE.to_string(),
            LOCKFILE,
            "metadata that is a lockfile",
        ),
    ] {
        let bad = Inputs {
            metadata: metadata.to_string(),
            lockfile: lockfile.as_bytes().to_vec(),
        };
        assert!(observe(dir.path(), &bad, &config()).is_err(), "{why}");
        assert!(
            !dir.path().join(".eprfs").exists(),
            "{why}: nothing written"
        );
    }
    // A checksum that is not sha256 hex refuses too.
    let corrupt = LOCKFILE.replace(
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "not-a-checksum",
    );
    let metadata = parse_metadata(METADATA).unwrap();
    let checksums = parse_lockfile(&corrupt).unwrap();
    assert!(translate(&metadata, &checksums, corrupt.as_bytes(), &config()).is_err());
}
