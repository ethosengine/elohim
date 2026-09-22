//! S3 (2026-09-22 recall-Codex-trail sprint): prose that lags its value — `source`'s `.json`
//! scan names a stale numeric claim against the document's own current, unambiguous value, with
//! the dated `_`-prefixed sibling that recorded the change.
use serde_json::Value;

mod common;
use common::{contract_value, ok, save_contract, write};

/// A JSON fixture shaped exactly like the motivating case: `io._comment` claims
/// `max_concurrent_heavy=1`, `io.max_concurrent_heavy` is actually `2`, and
/// `io._capacity_2026_09_09` is the dated sibling that recorded the widening.
const STALE_POLICY: &str = r#"{
  "io": {
    "_comment": "I/O guard policy. max_concurrent_heavy=1 means one cargo build per container at a time.",
    "max_concurrent_heavy": 2,
    "_capacity_2026_09_09": "Operator widened the cargo slot to 2 on 2026-09-09.",
    "default_jobs": 4
  }
}
"#;

/// A JSON fixture where the prose and the field agree — must produce zero disagreements.
const AGREEING_POLICY: &str = r#"{
  "io": {
    "_comment": "I/O guard policy. max_concurrent_heavy=2 means two cargo builds per container at a time.",
    "max_concurrent_heavy": 2,
    "default_jobs": 4
  }
}
"#;

fn fixture(text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    write(root, "docs/pool-policy.json", text);
    save_contract(root, contract_value(false));
    dir
}

#[test]
fn source_names_a_stale_prose_claim_with_its_dated_provenance() {
    let dir = fixture(STALE_POLICY);
    ok(
        dir.path(),
        &["open", "--intent", "Check the pool policy's own standing"],
    );
    let view: Value = ok(dir.path(), &["source", "--path", "docs/pool-policy.json"]);
    let found = view["disagreements"]["found"]
        .as_array()
        .expect("found array");
    assert!(
        !found.is_empty(),
        "expected at least one disagreement, view: {view}"
    );
    let hit = found
        .iter()
        .find(|d| d["key"] == "max_concurrent_heavy")
        .unwrap_or_else(|| panic!("expected a max_concurrent_heavy disagreement, got: {found:?}"));
    assert_eq!(hit["prose_value"], "1");
    assert_eq!(hit["effective_value"], "2");
    let provenance = hit["provenance"].as_array().expect("provenance array");
    assert!(
        provenance
            .iter()
            .any(|p| p.as_str() == Some("_capacity_2026_09_09")),
        "expected the dated sibling in provenance, got: {provenance:?}"
    );
}

#[test]
fn source_finds_no_disagreement_when_prose_and_value_agree() {
    let dir = fixture(AGREEING_POLICY);
    ok(
        dir.path(),
        &["open", "--intent", "Check the pool policy's own standing"],
    );
    let view: Value = ok(dir.path(), &["source", "--path", "docs/pool-policy.json"]);
    let found = view["disagreements"]["found"]
        .as_array()
        .expect("found array");
    assert!(
        found.is_empty(),
        "expected no disagreements when prose matches the value, got: {found:?}"
    );
}

/// A testing note that names a range ("concurrency=1 through concurrency=8") is not a claim about
/// the current value, so it is never reported as a disagreement (code review, 2026-09-22).
#[test]
fn a_range_in_prose_is_not_a_claim_about_the_current_value() {
    let dir = fixture(
        r#"{
  "_comment": "tested with concurrency=1 through concurrency=8, settled on four",
  "concurrency": 4
}
"#,
    );
    ok(
        dir.path(),
        &["open", "--intent", "Check the pool policy's own standing"],
    );
    let view: Value = ok(dir.path(), &["source", "--path", "docs/pool-policy.json"]);
    let found = view["disagreements"]["found"]
        .as_array()
        .expect("found array");
    assert!(found.is_empty(), "a range is not a stale claim: {found:?}");
}
