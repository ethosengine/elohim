//! The sink half of the projection — artifacts that LEFT a resource stage.
//!
//! Until this landed, `epr flow project` minted only `ReaVerb::Produce`: the repository's own
//! ValueFlows model of its own development had a source and no sink, so the doc corpus had an
//! inflow, no modelled outflow, and a turnover time that was unbounded *by construction* rather
//! than by observation. Nothing could tell those two apart, which is the whole failure mode
//! Meadows names — you cannot defend an inflow you never wrote down, and you certainly cannot
//! diagnose a stock whose drain you never modelled.
//!
//! The discrimination under test is the one that needs the recipe and that git alone cannot
//! make: a move OUT of the value chain is absorption; a rename INSIDE it is not.

use std::path::Path;

use elohim_epr_cli::flow::project;
use tempfile::TempDir;

fn git(root: &Path, args: &[&str]) {
    // build_command, not a bare Command — git exports GIT_DIR into hook environments and this
    // suite runs under the pre-push hook, where a bare spawn hits the ambient repo.
    let out = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture Author")
        .env("GIT_AUTHOR_EMAIL", "author@example.test")
        .env("GIT_COMMITTER_NAME", "Fixture Author")
        .env("GIT_COMMITTER_EMAIL", "author@example.test")
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?} failed");
}

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

const RECIPES: &str = r#"version: 1
recipes:
  - id: elohim-dev-pipeline
    version: 1
    description: fixture pipeline
    stages:
      - name: spec
        artifactKind: "doc:spec"
        paths:
          - "docs/*.md"
    edges: []
"#;

fn doc(id: &str) -> String {
    format!("---\nid: {id}\ntitle: {id}\n---\n\nBody of {id}.\n")
}

/// A repo whose history contains one of each removal shape.
fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, ".claude/epr-meta/recipes.yaml", RECIPES);

    write(root, "docs/kept.md", &doc("kept"));
    write(root, "docs/deleted.md", &doc("deleted"));
    write(root, "docs/renamed.md", &doc("renamed"));
    write(root, "docs/held-later.md", &doc("held-later"));
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "author four docs"]);

    // 1. A plain deletion — unambiguous absorption.
    std::fs::remove_file(root.join("docs/deleted.md")).unwrap();
    // 2. An IN-PLACE rename inside the stage — the artifact never left the plate.
    std::fs::rename(
        root.join("docs/renamed.md"),
        root.join("docs/renamed-better-name.md"),
    )
    .unwrap();
    // 3. A move to held/ — outside every stage glob, so the artifact left the chain.
    std::fs::create_dir_all(root.join("docs/held")).unwrap();
    std::fs::rename(
        root.join("docs/held-later.md"),
        root.join("docs/held/held-later.md"),
    )
    .unwrap();
    git(root, &["add", "-A"]);
    git(
        root,
        &[
            "commit",
            "-q",
            "-m",
            "delete, rename in place, move to held",
        ],
    );
    dir
}

#[test]
fn a_deletion_and_a_move_out_of_the_chain_are_absorption_but_an_in_place_rename_is_not() {
    let dir = fixture();
    let root = dir.path();
    let summary =
        project::project(root, &root.join(".claude/epr-meta/recipes.yaml")).expect("project runs");

    let log = std::fs::read_to_string(root.join(".eprfs/status/flows.jsonl")).unwrap();
    let consumes = log.matches("\"consume\"").count();
    assert_eq!(
        consumes, 2,
        "deleted.md and held-later.md are absorption; renamed.md moved INSIDE the stage and is \
         not. Got {consumes} consume events in:\n{log}"
    );

    // The labels index resolves them, so `epr flow walk` can name what left rather than showing
    // a bare CID — the same courtesy the produce side already had.
    let labels = std::fs::read_to_string(root.join(".eprfs/status/labels.json")).unwrap();
    assert!(labels.contains("docs/deleted.md"));
    assert!(labels.contains("docs/held-later.md"));

    // Both halves of the model are now present in one projection.
    assert!(summary.counts["event"].new >= 2);
}

#[test]
fn absorption_is_deduped_by_cid_like_every_other_record() {
    // Idempotence is the projection's standing law: identity is content, timestamps come from
    // git, never `now()`. A second run over unchanged history must mint nothing new — otherwise
    // the outflow rate would climb every time anyone ran the tool, and a measure that responds
    // to being MEASURED is worthless.
    let dir = fixture();
    let root = dir.path();
    let recipes = root.join(".claude/epr-meta/recipes.yaml");

    project::project(root, &recipes).expect("first run");
    let second = project::project(root, &recipes).expect("second run");
    assert_eq!(
        second.counts["event"].new, 0,
        "a re-run must mint no new events"
    );
    assert!(second.counts["event"].present >= 2);
}

#[test]
fn a_corpus_with_no_removals_yields_no_absorption_rather_than_a_zero_estimate() {
    // Honest absence at the projection layer: no removals means no consume events, which folds
    // to an outflow of zero and therefore an UNKNOWN turnover — not a confident claim that the
    // corpus never turns over.
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, ".claude/epr-meta/recipes.yaml", RECIPES);
    write(root, "docs/only.md", &doc("only"));
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "one doc, never removed"]);

    project::project(root, &root.join(".claude/epr-meta/recipes.yaml")).expect("project runs");
    let log = std::fs::read_to_string(root.join(".eprfs/status/flows.jsonl")).unwrap();
    assert_eq!(log.matches("\"consume\"").count(), 0);
    assert!(log.contains("\"produce\""), "the source side still mints");
}
