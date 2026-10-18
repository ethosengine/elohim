//! Executable acceptance stories for declared-local-views.feature, including hostile inputs.
use elohim_epr_cli::{
    flow::body_cid,
    view::{self, project, reopen, save},
};
use elohim_epr_rea::{
    AgentRef, Commitment, CommitmentState, FlowEvent, FlowRecord, Intent, Magnitude, ReaVerb,
    ResourceSpec,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

fn write(root: &Path, path: &str, bytes: impl AsRef<[u8]>) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}
fn fixture(name: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/views")
        .join(name);
    fn copy(source: &Path, dest: &Path) {
        fs::create_dir_all(dest).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &dest.join(entry.file_name()));
            } else {
                fs::copy(entry.path(), dest.join(entry.file_name())).unwrap();
            }
        }
    }
    copy(&source, dir.path());
    dir
}
fn declaration(root: &Path, change: impl FnOnce(&mut Value)) {
    let mut value: Value =
        serde_json::from_slice(&fs::read(root.join("view.json")).unwrap()).unwrap();
    change(&mut value);
    write(
        root,
        "view.json",
        serde_json::to_vec_pretty(&value).unwrap(),
    );
}
fn cli(args: &[&str]) -> Value {
    let result = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}
#[test]
fn same_binary_reads_two_declarations_without_repository_configuration() {
    for (name, id, field) in [
        ("workshop", "table", "status"),
        ("orchard", "north-bed", "phase"),
    ] {
        let root = fixture(name);
        let value = cli(&[
            "view",
            "view.json",
            "--root",
            root.path().to_str().unwrap(),
            "--json",
        ]);
        assert_eq!(value["projection"]["subjects"][0]["identity"], id);
        assert!(value["projection"]["subjects"][0]["fields"][field].is_string());
        for absent in ["genesis", ".claude", ".eprfs", ".git"] {
            assert!(!root.path().join(absent).exists());
        }
    }
}
#[test]
fn selection_placement_and_display_are_declaration_driven() {
    let root = fixture("orchard");
    fs::rename(root.path().join("plots"), root.path().join("nursery")).unwrap();
    declaration(root.path(), |d| {
        d["sources"]["items"]["path"] = json!("nursery");
        d["fields"] = json!(["category"]);
        d["readings"] = json!({});
    });
    let capture = project(root.path(), "view.json").unwrap();
    assert_eq!(capture.projection.subjects[0].source, "nursery/north.md");
    assert_eq!(capture.projection.subjects[0].fields.len(), 1);
    assert!(capture.projection.subjects[0].readings.is_empty());
    declaration(root.path(), |d| d["select"]["value"] = json!("apple"));
    assert!(project(root.path(), "view.json")
        .unwrap()
        .projection
        .subjects
        .is_empty());
}
fn add_record(log: &mut String, record: FlowRecord) -> cid::Cid {
    let cid = record.cid().unwrap();
    log.push_str(&serde_json::to_string(&json!({"cid":cid.to_string(), "record":record})).unwrap());
    log.push('\n');
    cid
}
#[test]
fn authored_status_production_review_acceptance_and_measurements_are_independent() {
    let root = fixture("workshop");
    let scope = body_cid(&fs::read_to_string(root.path().join("tasks/table.md")).unwrap());
    let spec = ResourceSpec {
        classified_as: vec!["gap:open".into(), "build-table".into()],
        quantity: None,
    };
    let mut log = String::new();
    let intent = add_record(
        &mut log,
        FlowRecord::Intent(Intent {
            action: ReaVerb::Produce,
            resource_spec: spec.clone(),
            in_scope_of: scope,
            raised_by: AgentRef("maker".into()),
        }),
    );
    let promise = add_record(
        &mut log,
        FlowRecord::Commitment(Commitment {
            action: ReaVerb::Produce,
            provider: AgentRef("maker".into()),
            receiver: AgentRef("owner".into()),
            resource_spec: spec,
            in_scope_of: scope,
            valid_from: None,
            valid_until: None,
            state: CommitmentState::Active,
            satisfies: vec![intent],
            bound: None,
        }),
    );
    let event = |resource, action, fulfills, tags| {
        FlowRecord::Event(FlowEvent {
            action,
            provider: AgentRef("maker".into()),
            receiver: AgentRef("owner".into()),
            resource,
            quantity: Magnitude::Count {
                value: 1.0,
                unit: "run-note".into(),
            },
            process: None,
            in_scope_of: scope,
            fulfills,
            satisfies: vec![],
            occurred_at: "2026-10-13T00:00:00Z".into(),
            classified_as: tags,
        })
    };
    let produced = add_record(
        &mut log,
        event(body_cid("report"), ReaVerb::Produce, vec![promise], vec![]),
    );
    let review = add_record(
        &mut log,
        event(
            promise,
            ReaVerb::Cite,
            vec![],
            vec![
                "run:verdict".into(),
                "table".into(),
                "verdict:approved".into(),
            ],
        ),
    );
    add_record(
        &mut log,
        event(
            scope,
            ReaVerb::Cite,
            vec![],
            vec![
                "run:observation".into(),
                "tasks/table.md".into(),
                "measure:sound@1".into(),
                "value:3".into(),
                "env:place=workshop".into(),
            ],
        ),
    );
    write(root.path(), "evidence/acts.jsonl", log);
    let captured = project(root.path(), "view.json").unwrap();
    let s = &captured.projection.subjects[0];
    assert_eq!(s.fields["status"], "open");
    let assertion = &s.readings["experience"]["value"]["assertions"][0];
    assert_eq!(assertion["acceptance"], "acceptance-unestablished");
    assert_eq!(
        assertion["commitments"][0]["fulfillments"][0],
        produced.to_string()
    );
    assert_eq!(
        assertion["commitments"][0]["technical_reviews"][0],
        review.to_string()
    );
    assert_eq!(s.readings["field measurements"]["value"][0]["value"], 3.0);
    assert_eq!(
        s.readings["field measurements"]["value"][0]["env"]["place"],
        "workshop"
    );
}
#[test]
fn sources_are_exact_binary_and_text_bytes_and_reopen_without_checkout() {
    let root = fixture("workshop");
    write(
        root.path(),
        "reports/inspection.md",
        "Narrative approval is not acceptance.",
    );
    write(root.path(), "image.bin", [0xff, 0, 0x81]);
    declaration(root.path(), |d| {
        d["sources"]["reports"] = json!({"path":"reports", "pattern":"*.md", "reader":"bytes"});
        d["sources"]["image"] = json!({"path":"image.bin", "reader":"bytes"});
        d["relationships"]["evidence"] =
            json!({"field":"evidence", "source":"reports", "target":"$path"});
    });
    let capture = project(root.path(), "view.json").unwrap();
    let store = TempDir::new().unwrap();
    let cid = save(store.path(), &capture).unwrap();
    let expected = fs::read(root.path().join("tasks/table.md")).unwrap();
    assert!(capture
        .projection
        .diagnostics
        .iter()
        .any(|s| s.contains("https://example.invalid")));
    root.close().unwrap();
    let reopened = reopen(store.path(), &cid).unwrap();
    assert_eq!(reopened.bytes["tasks/table.md"], expected);
    assert_eq!(reopened.bytes["image.bin"], [0xff, 0, 0x81]);
    let value = cli(&[
        "view",
        "--snapshot",
        &cid,
        "--store",
        store.path().to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(
        value["projection"]["subjects"][0]["readings"]["experience"]["state"],
        "unavailable"
    );
}
#[test]
fn baseline_preserves_cohort_and_missing_never_means_completed() {
    let root = fixture("orchard");
    let store = TempDir::new().unwrap();
    let old = project(root.path(), "view.json").unwrap();
    let cid = save(store.path(), &old).unwrap();
    let text = fs::read_to_string(root.path().join("plots/north.md")).unwrap();
    write(
        root.path(),
        "plots/north.md",
        text.replace("growing", "fruiting"),
    );
    let next = project(root.path(), "view.json").unwrap();
    assert_eq!(
        view::compare(&old.projection, &next.projection)["changed_fields"],
        json!(["north-bed"])
    );
    assert_ne!(save(store.path(), &next).unwrap(), cid);
    fs::remove_file(root.path().join("plots/north.md")).unwrap();
    write(
        root.path(),
        "plots/south.md",
        text.replace("north-bed", "south-bed"),
    );
    let latest = project(root.path(), "view.json").unwrap();
    let diff = view::compare(&old.projection, &latest.projection);
    assert_eq!(diff["missing"], json!(["north-bed"]));
    assert_eq!(diff["additions"], json!(["south-bed"]));
    assert_eq!(diff["cohort"][0]["state"], "missing");
    assert_eq!(
        reopen(store.path(), &cid).unwrap().bytes["plots/north.md"],
        text.as_bytes()
    );
}
#[test]
fn malformed_unsupported_duplicate_and_escaping_inputs_refuse() {
    for (change, expected) in [
        (0, "unknown variant"),
        (1, "duplicate subject"),
        (2, "repository-relative"),
        (3, "unknown field"),
        (5, "bounds"),
        (6, "relationship"),
    ] {
        let root = fixture("workshop");
        match change {
            0 => declaration(root.path(), |d| d["select"]["op"] = json!("exec")),
            1 => {
                fs::copy(
                    root.path().join("tasks/table.md"),
                    root.path().join("tasks/duplicate.md"),
                )
                .unwrap();
            }
            2 => declaration(root.path(), |d| {
                d["sources"]["items"]["path"] = json!("../outside")
            }),
            3 => declaration(root.path(), |d| d["shell"] = json!("anything")),
            4 => declaration(root.path(), |d| {
                d["sources"]["alias"] = json!({"path":"tasks/table.md", "reader":"bytes"})
            }),
            5 => declaration(root.path(), |d| d["bounds"]["max_bytes"] = json!(1)),
            6 => declaration(root.path(), |d| {
                d["readings"]["experience"]["relationship"] = json!("bad")
            }),
            _ => unreachable!(),
        }
        let error = project(root.path(), "view.json").err().unwrap();
        assert!(error.contains(expected), "{error}");
    }
}
#[test]
fn overlapping_bindings_keep_their_own_readers() {
    let root = fixture("workshop");
    declaration(root.path(), |d| {
        d["sources"]["z-alias"] = json!({"path":"tasks/table.md", "reader":"bytes"})
    });
    assert_eq!(
        project(root.path(), "view.json")
            .unwrap()
            .projection
            .subjects
            .len(),
        1
    );
}
#[test]
#[cfg(unix)]
fn symlinks_and_special_files_refuse_without_following_or_blocking() {
    let root = fixture("workshop");
    let external = TempDir::new().unwrap();
    write(external.path(), "secret", "private");
    std::os::unix::fs::symlink(
        external.path().join("secret"),
        root.path().join("tasks/link.md"),
    )
    .unwrap();
    assert!(project(root.path(), "view.json")
        .err()
        .unwrap()
        .contains("symlink"));
    fs::remove_file(root.path().join("tasks/link.md")).unwrap();
    rustix::fs::mknodat(
        rustix::fs::CWD,
        root.path().join("tasks/pipe.md"),
        rustix::fs::FileType::Fifo,
        rustix::fs::Mode::RUSR,
        0,
    )
    .unwrap();
    assert!(project(root.path(), "view.json")
        .err()
        .unwrap()
        .contains("regular file"));
}
#[test]
fn corrupt_or_missing_snapshot_bytes_never_fall_back_to_checkout() {
    let root = fixture("workshop");
    let store = TempDir::new().unwrap();
    let captured = project(root.path(), "view.json").unwrap();
    let cid = save(store.path(), &captured).unwrap();
    let source = store
        .path()
        .join(&captured.projection.sources["tasks/table.md"]);
    fs::write(&source, "corrupt").unwrap();
    assert!(reopen(store.path(), &cid)
        .err()
        .unwrap()
        .contains("corrupt"));
    assert!(save(store.path(), &captured).is_err());
    fs::remove_file(source).unwrap();
    assert!(reopen(store.path(), &cid).is_err());
}
#[test]
fn plain_read_is_byte_for_byte_read_only_and_resolves_governance() {
    let root = fixture("workshop");
    write(root.path(), ".epr-meta", "---\nepr-meta-version: 1\nid: workshop\nroot: true\npurpose: Local fixture governance\n---\n");
    let before = fs::read(root.path().join("tasks/table.md")).unwrap();
    let captured = project(root.path(), "view.json").unwrap();
    assert_eq!(captured.bytes["tasks/table.md"], before);
    assert_eq!(
        fs::read(root.path().join("tasks/table.md")).unwrap(),
        before
    );
    assert!(
        captured.projection.governance["tasks/table.md"]["records"]
            .as_array()
            .unwrap()
            .len()
            == 1
    );
    assert!(!root.path().join(".eprfs").exists());
}

#[test]
fn selected_capture_always_retains_governance_even_when_subject_glob_excludes_it() {
    let root = fixture("workshop");
    write(
        root.path(),
        "tasks/.epr-meta/manifest.md",
        "---\nepr-meta-version: 1\nid: tasks\npurpose: Fixture governance\ncovers: subtree\n---\n",
    );
    let captured = project(root.path(), "view.json").unwrap();
    assert_eq!(captured.projection.subjects.len(), 1);
    assert!(captured.bytes.contains_key("tasks/.epr-meta/manifest.md"));
    let store = TempDir::new().unwrap();
    let cid = save(store.path(), &captured).unwrap();
    root.close().unwrap();
    assert!(reopen(store.path(), &cid)
        .unwrap()
        .bytes
        .contains_key("tasks/.epr-meta/manifest.md"));
}
