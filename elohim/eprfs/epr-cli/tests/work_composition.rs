//! Work-composition.feature: shared engine, independent domains and immutable evidence.
use elohim_epr_cli::view::{project, reopen, save};
use serde_json::{json, Value};
use std::{fs, path::Path};
fn fixture() -> tempfile::TempDir {
    let root = tempfile::TempDir::new().unwrap();
    fn copy(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for e in fs::read_dir(from).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                copy(&e.path(), &to.join(e.file_name()));
            } else {
                fs::copy(e.path(), to.join(e.file_name())).unwrap();
            }
        }
    }
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/views/orchard"),
        root.path(),
    );
    root
}
#[test]
fn composition_retains_claims_links_sources_and_v1_compatibility() {
    let root = fixture();
    let captured = project(root.path(), "composition.json").unwrap();
    assert_eq!(captured.projection.version, 2);
    let goal = captured
        .projection
        .subjects
        .iter()
        .find(|s| s.identity == "goals:harvest")
        .unwrap();
    assert_eq!(goal.fields["$claims"][0]["authority"], "authored claim");
    assert_eq!(
        goal.fields["$conditions"][0],
        "Harvest pears with inspected quality"
    );
    assert!(captured
        .projection
        .composition
        .as_ref()
        .unwrap()
        .links
        .iter()
        .all(|l| l.state == "resolved"));
    let store = tempfile::TempDir::new().unwrap();
    let cid = save(store.path(), &captured).unwrap();
    fs::write(root.path().join("goals/harvest.md"), "changed").unwrap();
    assert_eq!(
        reopen(store.path(), &cid)
            .unwrap()
            .projection
            .subjects
            .len(),
        captured.projection.subjects.len()
    );
    assert!(project(root.path(), "view.json").is_ok());
}
#[test]
fn rejects_execution_escape_unknown_versions_and_recursive_composition() {
    for (key, value) in [
        ("execute", json!("touch source")),
        ("version", json!(99)),
        (
            "bounds",
            json!({"max_files":10000,"max_bytes":134217728,"max_depth":33}),
        ),
        (
            "bounds",
            json!({"max_files":10000,"max_bytes":134217728,"max_depth":0}),
        ),
        (
            "groups",
            json!([{"id":"goals","label":"Goals","view":"../outside.json","role":"focus"}]),
        ),
    ] {
        let root = fixture();
        let p = root.path().join("composition.json");
        let mut d: Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
        d[key] = value;
        fs::write(p, serde_json::to_vec(&d).unwrap()).unwrap();
        assert!(project(root.path(), "composition.json").is_err());
    }
    let root = fixture();
    let p = root.path().join("composition.json");
    let mut d: Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
    d["groups"][0]["view"] = json!("composition.json");
    fs::write(p, serde_json::to_vec(&d).unwrap()).unwrap();
    assert!(project(root.path(), "composition.json").is_err());
}
#[test]
fn unresolved_links_stay_explicit_and_declaration_changes_do_not_rewrite_history() {
    let root = fixture();
    let p = root.path().join("composition.json");
    let before = project(root.path(), "composition.json").unwrap();
    let mut d: Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
    d["links"][0]["targetField"] = json!("missing");
    fs::write(p, serde_json::to_vec(&d).unwrap()).unwrap();
    let after = project(root.path(), "composition.json").unwrap();
    assert_ne!(
        before.projection.declaration_cid,
        after.projection.declaration_cid
    );
    assert!(after
        .projection
        .composition
        .unwrap()
        .links
        .iter()
        .all(|l| l.state == "unresolved"));
    assert!(before
        .projection
        .composition
        .unwrap()
        .links
        .iter()
        .all(|l| l.state == "resolved"));
}

#[test]
fn wrapped_claims_retain_qualifications_and_changed_continuations() {
    let root = fixture();
    let path = root.path().join("goals/harvest.md");
    let text = fs::read_to_string(&path).unwrap()
        + "  Limitation: inspection is incomplete.\n  Evidence: not yet verified.\n";
    fs::write(&path, &text).unwrap();
    let before = project(root.path(), "composition.json").unwrap();
    let claim = before
        .projection
        .subjects
        .iter()
        .find(|s| s.identity == "goals:harvest")
        .unwrap()
        .fields["$claims"][0]["text"]
        .as_str()
        .unwrap();
    assert!(claim.contains("Limitation: inspection is incomplete."));
    fs::write(
        &path,
        text.replace("not yet verified", "now awaiting review"),
    )
    .unwrap();
    let after = project(root.path(), "composition.json").unwrap();
    assert_ne!(
        before.projection.subjects[0].fields["$claims"],
        after.projection.subjects[0].fields["$claims"]
    );
}
