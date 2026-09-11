//! `epr flow report placement --coverage` / `--stasis`, the surface-walk derive, and supersession.
//!
//! Station six round (b) of `genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md`.
//!
//! **The expected outputs are pinned as constants.** A fixture assertion that recomputes the
//! expectation from the same code it is testing proves only that the code is deterministic. These
//! numbers were derived by hand from the fixture below and from `placement-audit.py`'s arithmetic,
//! so a change in the derivation has to argue with a written-down number.

use std::collections::BTreeMap;
use std::path::Path;

use elohim_epr_cli::flow::stasis::{self, DIMENSION_KEYS};
use tempfile::TempDir;

// ── the fixture's pinned expectations ──────────────────────────────────────────────────────────
//
// Four active plans: two carry `- [ ]` stations, one is agent-decomposed (a store record with
// items but no checkbox in its own bytes), one is untouched prose.
const EXPECT_ACTIVE: usize = 4;
const EXPECT_CAPTURED: usize = 3; // two with stations + one the store records as having items
const EXPECT_NEEDS_AGENT: usize = 0;
const EXPECT_UNDECOMPOSED: usize = 1;
const EXPECT_UNCAPTURED: usize = 1;

// Dimensions over the same fixture. Each is `covered / total` with the denominators above.
const EXPECT_CAPTURE: f64 = 0.75; // 3 of 4
const EXPECT_STATUS: f64 = 0.75; // 3 of 4 declare a status
                                 // 2 of 4. `a-stationed` links OUT (to b, and to PLACEMENT.md); `b-also-stationed` carries no link
                                 // of its own but is linked TO by a, which is the inbound half the kit counts and a one-pass reading
                                 // would miss. `c-agent-decomposed` and `d-untouched` have neither direction and are real orphans.
const EXPECT_WELL_FORMED: f64 = 0.5;
const EXPECT_TRACEABILITY: f64 = 0.25; // 1 of 4 carries an EXPLAINED trace
const EXPECT_MEMORY_LINKED: f64 = 0.5; // 1 of 2 memory entries cites a system

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

/// A tempdir carrying one of every shape the two readings distinguish.
fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let plans = "genesis/docs/superpowers/plans";

    // captured — stations in its own bytes, a status, an explained trace, links out
    write(
        root,
        &format!("{plans}/a-stationed.md"),
        "---\nstatus: proposed\ntraces:\n  - genesis/docs/PLACEMENT.md — the placement rule this plan implements\ncites:\n  - b-also-stationed.md\n---\n\n- [ ] first station\n- [x] second station\n",
    );
    // captured — stations, a status, no trace, no links (but IS linked TO by a-stationed)
    write(
        root,
        &format!("{plans}/b-also-stationed.md"),
        "---\nstatus: draft\n---\n\n- [ ] only station\n",
    );
    // captured via the store only — no checkbox in its own bytes
    write(
        root,
        &format!("{plans}/c-agent-decomposed.md"),
        "---\nstatus: design\n---\n\nProse only; an agent extracted the work.\n",
    );
    write(
        root,
        ".eprfs/status/gap-items/plans__c-agent-decomposed.json",
        r#"{"method": "agent", "items": [{"id": "1", "state": "open"}]}"#,
    );
    // uncaptured — no stations, no store record, and no status either
    write(
        root,
        &format!("{plans}/d-untouched.md"),
        "Prose with no frontmatter at all.\n",
    );

    write(
        root,
        ".claude/memory/linked.md",
        "---\nname: linked\ncites:\n  - genesis/docs/PLACEMENT.md\n---\n\nbody\n",
    );
    write(
        root,
        ".claude/memory/unlinked.md",
        "---\nname: unlinked\n---\n\nbody\n",
    );
    write(root, ".claude/memory/MEMORY.md", "# index\n");
    write(root, "CLAUDE.md", "# small gate\n");
    dir
}

#[test]
fn coverage_counts_the_un_reviewed_workload_the_loop_drains() {
    let dir = fixture();
    let report = stasis::coverage(dir.path()).unwrap();
    assert_eq!(report.active, EXPECT_ACTIVE);
    assert_eq!(report.captured, EXPECT_CAPTURED);
    assert_eq!(report.needs_agent, EXPECT_NEEDS_AGENT);
    assert_eq!(report.undecomposed, EXPECT_UNDECOMPOSED);
    assert_eq!(
        report.uncaptured, EXPECT_UNCAPTURED,
        "uncaptured = undecomposed + needs_agent is the number memory-stasis-loop.js drains"
    );
    assert_eq!(report.uncaptured_docs.len(), EXPECT_UNCAPTURED);
    assert!(report.uncaptured_docs[0].path.ends_with("d-untouched.md"));
    assert_eq!(report.uncaptured_docs[0].why, "undecomposed");
    // The changed reading is ON the payload, not only in a doc comment.
    assert!(report.method.contains("yields `- [ ]` stations"));
}

#[test]
fn a_store_record_with_no_items_is_needs_agent_not_undecomposed() {
    // The one case the store is still the sole witness of: an agent looked, and found no structure.
    // Collapsing it into `undecomposed` would send the loop to re-dispatch work already done.
    let dir = fixture();
    write(
        dir.path(),
        ".eprfs/status/gap-items/plans__d-untouched.json",
        r#"{"method": "none", "items": []}"#,
    );
    let report = stasis::coverage(dir.path()).unwrap();
    assert_eq!(report.needs_agent, 1);
    assert_eq!(report.undecomposed, 0);
    assert_eq!(
        report.uncaptured, 1,
        "the workload is the same size either way"
    );
    assert_eq!(report.uncaptured_docs[0].why, "needs-agent");
}

#[test]
fn the_stasis_dimensions_are_the_pinned_ratios_and_the_score_is_their_weighted_mean() {
    let dir = fixture();
    let report = stasis::stasis(dir.path(), &BTreeMap::new()).unwrap();

    let dim = |k: &str| *report.dimensions.get(k).unwrap();
    assert_eq!(dim("capture"), EXPECT_CAPTURE);
    assert_eq!(dim("status"), EXPECT_STATUS);
    assert_eq!(dim("well_formed"), EXPECT_WELL_FORMED);
    assert_eq!(dim("traceability"), EXPECT_TRACEABILITY);
    assert_eq!(dim("memory_linked"), EXPECT_MEMORY_LINKED);

    // Every declared dimension is present — a missing key would silently drop a term from the mean.
    for key in DIMENSION_KEYS {
        assert!(report.dimensions.contains_key(key), "missing {key}");
    }
    // The score is the weighted mean of the nine, at the fixture's default weights of 1.0.
    let mean: f64 = DIMENSION_KEYS.iter().map(|k| dim(k)).sum::<f64>() / 9.0;
    assert!(
        (report.score - (mean * 1000.0).round() / 1000.0).abs() < 1e-9,
        "score {} is not the mean {mean} of {:?}",
        report.score,
        report.dimensions
    );
    assert_eq!(report.benchmark, 1.0);
    assert_eq!(report.margin, 0.15);
    assert_eq!(report.threshold, 0.85);
    assert!(
        !report.at_stasis,
        "the fixture is deliberately not at stasis"
    );
    // The tuning ladder says which rung it used rather than leaving a reader to assume a file.
    assert_eq!(report.tuning.source, "built-in defaults");
    // The kit's unmeasured list is carried, not quietly dropped into the score.
    assert_eq!(report.unmeasured.len(), 3);
}

#[test]
fn the_hard_gates_carry_the_count_that_decided_them() {
    let dir = fixture();
    let clean = stasis::stasis(dir.path(), &BTreeMap::new()).unwrap();
    assert!(clean.hard_ok);
    assert!(clean.hard_gates.iter().all(|g| g.observed == 0));

    // A retired doc with no verification evidence is a dump, and a dump gates the score outright.
    write(
        dir.path(),
        "genesis/docs/_retired/no-evidence.md",
        "---\nstatus: landed\n---\n\nbody\n",
    );
    let dumped = stasis::stasis(dir.path(), &BTreeMap::new()).unwrap();
    assert!(!dumped.hard_ok);
    assert!(!dumped.at_stasis, "no amount of coverage offsets a dump");
    let gate = dumped
        .hard_gates
        .iter()
        .find(|g| g.name.contains("_retired"))
        .unwrap();
    assert_eq!(
        gate.observed, 1,
        "a boolean with no number behind it is unauditable"
    );
    assert!(
        (dumped.score - clean.score).abs() < 1e-9,
        "a hard gate gates the VERDICT, never the score"
    );
}

#[test]
fn an_unbaselined_dimension_is_neither_a_regression_nor_a_pass() {
    let dir = fixture();
    let none = stasis::stasis(dir.path(), &BTreeMap::new()).unwrap();
    assert!(none.ratchet.iter().all(|r| r.state == "unbaselined"));
    assert!(!none.ratchet_regressed, "no baseline is not a regression");
    assert!(none.ratchet.iter().all(|r| r.baseline.is_none()));

    let mut baselines = BTreeMap::new();
    baselines.insert("capture".to_string(), 0.75); // held exactly
    baselines.insert("status".to_string(), 0.95); // current 0.75 — a real fall
    baselines.insert("traceability".to_string(), 0.10); // current 0.25 — an improvement
    let read = stasis::stasis(dir.path(), &baselines).unwrap();
    let state = |k: &str| {
        read.ratchet
            .iter()
            .find(|r| r.dimension == k)
            .unwrap()
            .state
            .clone()
    };
    assert_eq!(state("capture"), "held");
    assert_eq!(state("status"), "regressed");
    assert_eq!(state("traceability"), "improved");
    assert_eq!(state("well_formed"), "unbaselined");
    assert!(read.ratchet_regressed);
}

#[test]
fn the_epr_meta_census_credits_only_a_valid_subtree_claim() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    // A structurally-substantial code directory: 5 subdirs clears min_subdirs, and a .rs file
    // clears min_exts.
    for n in 0..5 {
        write(root, &format!("src/region/sub{n}/mod.rs"), "// code\n");
    }
    write(root, "src/region/lib.rs", "// code\n");
    let tuning = stasis::Tuning::resolve(root);
    assert_eq!(
        stasis::epr_meta_coverage(root, &tuning),
        0.0,
        "a substantial region with no claim is a gap"
    );

    // `covers: subtree` at the right altitude owns the whole region and terminates descent.
    write(
        root,
        "src/region/.epr-meta",
        "---\nepr-meta-version: 1\nid: region-governance\ncovers: subtree\npurpose: >\n  owned\n---\n",
    );
    assert_eq!(stasis::epr_meta_coverage(root, &tuning), 1.0);

    // A manifest that claims the subtree but does not parse must NOT be credited — a broken
    // manifest the resolver would reject would otherwise inflate the ratio and hide a real gap.
    write(
        root,
        "src/region/.epr-meta",
        "---\nepr-meta-version: 99\nid: region-governance\ncovers: subtree\n---\n",
    );
    assert_eq!(
        stasis::epr_meta_coverage(root, &tuning),
        0.0,
        "an invalid manifest is not coverage"
    );
}

#[test]
fn a_tree_with_nothing_governable_is_full_coverage_not_zero() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "notes/one.md", "prose\n");
    let tuning = stasis::Tuning::resolve(dir.path());
    assert_eq!(
        stasis::epr_meta_coverage(dir.path(), &tuning),
        1.0,
        "0/0 is 1.0: a repository with no governable region has no governance gap"
    );
}

#[test]
fn the_tuning_ladder_prefers_the_relocated_home_and_says_which_rung_it_used() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    assert_eq!(stasis::Tuning::resolve(root).source, "built-in defaults");

    write(
        root,
        ".claude/memory-kit/context-coverage.yaml",
        "targets:\n  margin:\n    value: 0.25\n",
    );
    let kit = stasis::Tuning::resolve(root);
    assert_eq!(kit.source, ".claude/memory-kit/context-coverage.yaml");
    assert_eq!(kit.margin, 0.25);

    // The relocated home wins, so the reading survives the report tier's removal.
    write(
        root,
        ".epr-meta/elohim/lenses/context-coverage.yaml",
        "targets:\n  margin:\n    value: 0.05\n",
    );
    let moved = stasis::Tuning::resolve(root);
    assert_eq!(
        moved.source,
        ".epr-meta/elohim/lenses/context-coverage.yaml"
    );
    assert_eq!(moved.margin, 0.05);
}
