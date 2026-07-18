//! Integration tests over a synthetic tempdir repo — no dependence on live repo state.
//! Builds a tiny tree (a spec doc with frontmatter + a cite target, a gap-items JSON, a
//! feature file), git-inits with one commit, then exercises project → walk → status.

use std::path::Path;
use std::process::Command;

use elohim_epr_cli::flow::{project, walk};
use tempfile::TempDir;

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_AUTHOR_NAME", "Fixture Author")
        .env("GIT_AUTHOR_EMAIL", "author@example.test")
        .env("GIT_COMMITTER_NAME", "Fixture Author")
        .env("GIT_COMMITTER_EMAIL", "author@example.test")
        .output()
        .expect("git runs");
    assert!(status.status.success(), "git {args:?} failed");
}

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    write(
        root,
        ".claude/epr-meta/recipes.yaml",
        r#"version: 1
recipes:
  - id: elohim-dev-pipeline
    version: 1
    description: fixture pipeline
    stages:
      - name: spec
        artifactKind: "doc:spec"
        paths:
          - "docs/*.md"
      - name: intent
        artifactKind: "gap-item"
        paths:
          - "gap/*.json"
      - name: scenario
        artifactKind: "a2o:feature"
        paths:
          - "features/**/*.feature"
    edges:
      - { from: spec, to: intent, meaningful: true, validators: [decompose] }
      - { from: spec, to: scenario, meaningful: true, validators: [a2o-parse] }
"#,
    );

    // The cite target is a plain resource (no `id:`), so it is NOT itself a process
    // instance — only a content-addressed input the spec's process consumes.
    write(
        root,
        "docs/target.md",
        "---\ntitle: Target\n---\n\nThe target body that the spec cites.\n",
    );

    write(
        root,
        "docs/spec.md",
        "---\nid: fixture-spec\ntitle: Fixture Spec\ncites:\n  - fixture-target | the cited target | sha256:0000000000000000 | path: docs/target.md\n---\n\nThe fixture spec body.\n",
    );

    write(
        root,
        "gap/spec.json",
        r#"{
  "doc": "docs/spec.md",
  "slug": "docs__spec",
  "items": [
    { "id": "docs__spec#1", "state": "OPEN" },
    { "id": "docs__spec#2", "state": "CLAIMED" }
  ]
}
"#,
    );

    write(
        root,
        "features/thing.feature",
        "Feature: A fixture scenario\n  Scenario: it works\n    Given a thing\n    Then it holds\n",
    );

    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "fixture"]);
    dir
}

#[test]
fn project_derives_expected_record_kinds() {
    let dir = fixture();
    let root = dir.path();
    let recipes = root.join(".claude/epr-meta/recipes.yaml");

    let summary = project::project(root, &recipes).expect("project runs");

    assert_eq!(summary.recipes, 1);
    assert_eq!(summary.counts["spec"].new, 1, "one ProcessSpec atom");
    assert_eq!(
        summary.counts["process"].new, 1,
        "one Process for the id-bearing spec"
    );
    assert_eq!(summary.counts["event"].new, 1, "one Produce event");
    assert_eq!(summary.counts["intent"].new, 2, "two gap-item intents");
    // one CLAIMED gap commitment + one scenario commitment
    assert_eq!(
        summary.counts["commitment"].new, 2,
        "claim + scenario commitments"
    );

    // Sidecar + labels landed.
    assert!(root.join(".eprfs/status/flows.jsonl").exists());
    assert!(root.join(".eprfs/status/labels.json").exists());
}

#[test]
fn project_is_idempotent_dedup_by_cid() {
    let dir = fixture();
    let root = dir.path();
    let recipes = root.join(".claude/epr-meta/recipes.yaml");

    project::project(root, &recipes).expect("first run");
    let second = project::project(root, &recipes).expect("second run");

    for kind in ["spec", "process", "event", "intent", "commitment"] {
        assert_eq!(second.counts[kind].new, 0, "{kind}: nothing new on re-run");
        assert!(
            second.counts[kind].present > 0,
            "{kind}: all present on re-run"
        );
    }
}

#[test]
fn walk_shows_lineage_and_scoped_intent_frontier() {
    let dir = fixture();
    let root = dir.path();
    let recipes = root.join(".claude/epr-meta/recipes.yaml");
    project::project(root, &recipes).expect("project runs");

    let result = walk::walk(root, "docs/spec.md").expect("walk runs");

    // Lineage: a produce event (by the fixture author) + a process consuming the cite.
    assert_eq!(result.lineage.producing_events.len(), 1);
    assert_eq!(
        result.lineage.producing_events[0].provider,
        "author@example.test"
    );
    assert_eq!(result.lineage.processes.len(), 1);
    let inputs: Vec<&str> = result.lineage.processes[0]
        .inputs
        .iter()
        .map(|r| r.label.as_str())
        .collect();
    assert!(
        inputs.contains(&"docs/target.md"),
        "process input should resolve to the cited target, got {inputs:?}"
    );

    // Frontier: both gap-item intents scoped to this spec are open.
    assert_eq!(
        result.frontier.scoped_intents.len(),
        2,
        "both gap-item intents are scoped to the spec"
    );
    let classes: Vec<String> = result
        .frontier
        .scoped_intents
        .iter()
        .flat_map(|i| i.classified_as.clone())
        .collect();
    assert!(classes.iter().any(|c| c == "gap:open"));
    assert!(classes.iter().any(|c| c == "gap:claimed"));
}

#[test]
fn status_totals_the_sidecar() {
    let dir = fixture();
    let root = dir.path();
    let recipes = root.join(".claude/epr-meta/recipes.yaml");
    project::project(root, &recipes).expect("project runs");

    let status = walk::status(root).expect("status runs");
    assert_eq!(status.events, 1);
    assert_eq!(status.intents, 2);
    // claim + scenario commitments are both Active.
    assert_eq!(status.commitments_active, 2);
    // The scenario commitment is unfulfilled (scoped to the repo atom); the claim
    // commitment is unfulfilled too (no discharging event).
    assert!(status.unfulfilled_total >= 1);
    assert!(status.resources_labeled > 0);
}
