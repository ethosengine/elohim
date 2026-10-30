use elohim_hub::local_views::{Config, LocalViews};
use std::{collections::BTreeMap, fs};
fn reader(root: &std::path::Path, store: &std::path::Path) -> LocalViews {
    LocalViews::new(
        Config {
            root: root.into(),
            store: store.into(),
            views: BTreeMap::from([("orchard".into(), "view.json".into())]),
            listen: "127.0.0.1:0".parse().unwrap(),
        },
        "test-token-with-at-least-thirty-two-bytes".into(),
    )
    .unwrap()
}
fn fixture(root: &std::path::Path) {
    fs::create_dir(root.join("plots")).unwrap();
    fs::write(
        root.join("view.json"),
        include_str!("../../eprfs/epr-cli/tests/fixtures/views/orchard/board.view.json"),
    )
    .unwrap();
    fs::write(
        root.join("plots/north.md"),
        include_str!("../../eprfs/epr-cli/tests/fixtures/views/orchard/plots/north.md"),
    )
    .unwrap();
}
#[test]
fn history_preserves_identical_captures_failures_and_original_bytes() {
    let root = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    fixture(root.path());
    let reader = reader(root.path(), &store.path().join("private"));
    let first = reader.capture("orchard").unwrap();
    let second = reader.capture("orchard").unwrap();
    assert_eq!(first.snapshot_cid, second.snapshot_cid);
    assert_eq!(second.sequence, 2);
    fs::write(root.path().join("view.json"), "malformed").unwrap();
    assert!(reader.capture("orchard").is_err());
    assert_eq!(reader.history("orchard").unwrap().len(), 2);
    fs::remove_dir_all(root.path()).unwrap();
    let old = reader.snapshot("orchard", &first.snapshot_cid).unwrap();
    assert!(old.envelope.contains("north-bed"));
    assert!(!old.envelope.contains("observedAt"));
    assert!(reader.history("../private").is_err());
    assert!(reader.snapshot("orchard", "../../etc/passwd").is_err());
}
#[test]
fn a_second_writer_and_corrupt_objects_refuse() {
    let root = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    fixture(root.path());
    let path = store.path().join("private");
    let reader = reader(root.path(), &path);
    assert!(LocalViews::new(
        Config {
            root: root.path().into(),
            store: path.clone(),
            views: BTreeMap::from([("orchard".into(), "view.json".into())]),
            listen: "127.0.0.1:0".parse().unwrap()
        },
        "test-token-with-at-least-thirty-two-bytes".into()
    )
    .is_err());
    let observed = reader.capture("orchard").unwrap();
    fs::write(path.join("objects").join(&observed.snapshot_cid), "corrupt").unwrap();
    assert!(reader.snapshot("orchard", &observed.snapshot_cid).is_err());
}
#[test]
fn presentation_is_bounded_and_cannot_define_acceptance() {
    use elohim_epr_cli::view::declaration::Declaration;
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../eprfs/epr-cli/tests/fixtures/views/orchard/board.view.json"
    ))
    .unwrap();
    let mut bad = value.clone();
    bad["presentation"]["acceptanceRule"] = "status=done".into();
    assert!(serde_json::from_value::<Declaration>(bad).is_err());
    let mut bad = value;
    bad["presentation"]["reconciliationReading"] = "invented".into();
    assert!(serde_json::from_value::<Declaration>(bad)
        .unwrap()
        .validate()
        .is_err());
}
