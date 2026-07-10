//! Python↔Rust `.epr-meta` resolver parity (Task 6 / B2).
//!
//! These fixtures live on disk once, under
//! `.claude/scripts/_lib/__tests__/fixtures/epr_meta_parity/`, and are resolved by BOTH the Python
//! cascade test (`.claude/scripts/_lib/__tests__/epr_meta_cascade_test.py`) and this Rust test, so
//! the two `.epr-meta` interpreters cannot silently drift on cascade order or nearest-wins conflict
//! resolution — the standing hazard tracked at
//! `genesis/data/timeline/backlog/epr-meta-python-rust-parser-parity.md`.
//!
//! **Why this test does not assert on `EprMetaResolution::effective_rules`'s order:** that field is
//! built from a `BTreeMap<String, GovernanceRule>` (see `eprfs-meta/src/lib.rs::resolve_path`), so its
//! iteration order is always alphabetical by rule id — an implementation artifact of the dedup
//! structure, not the cascade/nearest-wins order. Python's `merge_rules` instead returns a plain
//! dict, whose key order is first-seen-position (root-first insertion) with nearest-wins only
//! overwriting the VALUE in place. Those two orders are NOT the same thing in general (fixture 3
//! below is constructed so they visibly diverge: alphabetically `alpha < collide < zeta`, but the
//! cascade/nearest-wins order is `zeta, collide, alpha`). The genuinely ordered, inspectable
//! resolution `eprfs-meta` exposes is `EprMetaResolution::records` — one entry per manifest, in
//! root-first cascade order, each carrying its own declared rules in file order. This test reduces
//! `records` with the identical first-seen/nearest-wins algorithm Python's dict performs natively, so
//! the two languages are compared on the same semantic contract instead of on an accidental sort.

use std::path::{Path, PathBuf};

use eprfs_core::{EprMetaRecord, GovernanceRuleClass};
use eprfs_meta::resolve_path;

fn fixtures_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = elohim/eprfs/eprfs-meta — walk up to the repo root, then down to the
    // shared fixture corpus under .claude.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(".claude/scripts/_lib/__tests__/fixtures/epr_meta_parity")
}

fn class_str(class: &GovernanceRuleClass) -> &'static str {
    match class {
        GovernanceRuleClass::Deny => "deny",
        GovernanceRuleClass::Ask => "ask",
        GovernanceRuleClass::Inject => "inject",
        GovernanceRuleClass::Measure => "measure",
        GovernanceRuleClass::Dispatch => "dispatch",
    }
}

/// Mirrors `epr_meta.merge_rules`'s dict semantics: walk `records` root-first (the same cascade
/// order Python's `collect_cascade` chain produces), and for each declared rule, keep first-seen
/// POSITION but let the nearest (later-cascaded) record win the VALUE.
fn ordered_rule_classes(records: &[EprMetaRecord]) -> Vec<(String, String)> {
    let mut order: Vec<String> = Vec::new();
    let mut classes: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for record in records {
        let Some(governance) = &record.governance else {
            continue;
        };
        for binding in &governance.bindings {
            let Some(rule) = &binding.rule else { continue };
            if !classes.contains_key(&rule.id) {
                order.push(rule.id.clone());
            }
            classes.insert(rule.id.clone(), class_str(&rule.class).to_string());
        }
    }
    order
        .into_iter()
        .map(|id| {
            let class = classes.get(&id).cloned().unwrap_or_default();
            (id, class)
        })
        .collect()
}

#[test]
fn parity_root_directory_form_resolves_single_rule() {
    let root = fixtures_root().join("root-directory-form");
    let resolution = resolve_path(&root, root.join("notes/new.md")).expect("resolve fixture 1");

    assert_eq!(resolution.records.len(), 1);
    assert_eq!(resolution.records[0].id, "root-dir-form");

    let ordered = ordered_rule_classes(&resolution.records);
    assert_eq!(
        ordered,
        vec![("root-only-rule".to_string(), "inject".to_string())]
    );
}

#[test]
fn parity_legacy_nested_resolves_single_rule() {
    let root = fixtures_root().join("legacy-nested");
    let resolution = resolve_path(&root, root.join("sub/leaf.md")).expect("resolve fixture 2");

    assert_eq!(resolution.records.len(), 1);
    assert_eq!(resolution.records[0].id, "nested-legacy-meta");

    let ordered = ordered_rule_classes(&resolution.records);
    assert_eq!(
        ordered,
        vec![("nested-legacy-rule".to_string(), "ask".to_string())]
    );
}

#[test]
fn parity_cascade_conflict_root_first_nearest_wins() {
    let root = fixtures_root().join("cascade-conflict");
    let resolution = resolve_path(&root, root.join("src/leaf.md")).expect("resolve fixture 3");

    assert_eq!(resolution.records.len(), 2);
    // root-first cascade order — matches the Python test's chain[0]/chain[-1] assertions.
    assert_eq!(resolution.records[0].id, "cascade-root-meta");
    assert_eq!(resolution.records[1].id, "cascade-src-meta");

    // The SAME ordered rule-ids the Python parity test asserts on:
    // ["zeta-root-rule", "collide-rule", "alpha-nested-rule"], with collide-rule resolved to the
    // nested manifest's "deny" (nearest-wins on value; first-seen wins on position).
    let ordered = ordered_rule_classes(&resolution.records);
    assert_eq!(
        ordered,
        vec![
            ("zeta-root-rule".to_string(), "inject".to_string()),
            ("collide-rule".to_string(), "deny".to_string()),
            ("alpha-nested-rule".to_string(), "ask".to_string()),
        ]
    );
}
