//! `epr flow walk` / `epr flow status` evidence columns (native-delivery sprint, Lane G1;
//! evidence-ladder spec §5 increment 3) and the `cost:` headline slot.
//!
//! One synthetic tempdir repository carries every input the walker joins: a habit atom and its
//! generated register, a2o features tagged `@concern:`/`@requires:`, sprint-report receipts at two
//! rungs, the Act I lane contract, declared cost lenses addressed to the habit, their folds, and
//! claim commitments in the flows sidecar. No live repository state is read.

use std::collections::BTreeMap;
use std::path::Path;

use elohim_epr_cli::flow::note::{observe, NoteActor};
use elohim_epr_cli::flow::report::{report, ReportOptions};
use elohim_epr_cli::flow::{body_cid_of_file, walk};
use elohim_epr_rea::{
    AgentRef, Commitment, CommitmentState, FlowRecord, FlowStore, ReaVerb, ResourceSpec,
    SidecarFlowStore,
};
use tempfile::TempDir;

const HABIT: &str = "push-delivers";
const ATOM: &str = "ops/.epr-meta/push-delivers.habit.md";

const MEASURES: &str = r#"measures-version: 1
measures:
  - id: stage-wallclock
    version: 1
    family: delivery
    unit: minutes
    default-authority: observation
    status: active
  - id: delivery-cost
    version: 1
    family: delivery
    unit: pipeline-hours
    default-authority: observation
    status: active
  - id: operator-surfaced-pain
    version: 1
    family: algedonic-sensing
    unit: count
    default-authority: observation
    status: active
  - id: operator-surfaced-pain-reset
    version: 1
    family: algedonic-sensing
    unit: reset-marks
    default-authority: observation
    status: active

lenses:
  - id: stage-wallclock-ceiling
    version: 1
    binding: binding-local
    class: measure
    consumes: [stage-wallclock@1]
    subject: .
    context: session-headline
    hard: 20
    concern: push-delivers
    status: active
  - id: delivery-cost-ceiling
    version: 1
    binding: binding-local
    class: measure
    consumes: [delivery-cost@1]
    subject: .
    context: session-headline
    hard: 1
    concern: push-delivers
    status: active
  - id: operator-surfaced-pain-ceiling
    version: 1
    headline: cost
    binding: binding-local
    class: measure
    derive: count-since-reset
    reset: operator-surfaced-pain-reset@1
    consumes: [operator-surfaced-pain@1]
    context: session-headline
    hard: 0
    concern: some-other-habit
    status: active
"#;

const ATOM_BODY: &str = r#"---
epr-habit-version: 1
id: push-delivers
invariant: A push delivers.
status: red
active: true
checks:
  - "live series: `node ops/series.mjs --window 10` reads the archived graphs."
  - "gone: `node --test ops/missing.test.mjs` was deleted and nobody noticed."
  - "prose only: see https://jenkins.example/job/x and `just gate`."
retire-when: never, in a fixture
---
The ledger body.
"#;

const REGISTER: &str = r#"habits:
  - id: push-delivers
    declared: ops/.epr-meta/push-delivers.habit.md
    status: red
    active: true
    checks:
      - "live series: `node ops/series.mjs --window 10` reads the archived graphs."
      - "gone: `node --test ops/missing.test.mjs` was deleted and nobody noticed."
  - id: some-other-habit
    declared: .epr-meta/some-other-habit.habit.md
    status: green
    checks:
      - "the tag expression: `just test mesh @concern:some-other-habit`"
"#;

const HOUSEHOLD_LANE: &str = r#"schema_version: 1
resources:
  household-nodes:
    role: three peers
    available: true
  shem:
    role: the big box
    available: false
"#;

const LIVE_LANE: &str = r#"schema_version: 1
resources:
  shem:
    role: the big box
    available: false
"#;

fn git(root: &Path, args: &[&str]) {
    let status = elohim_epr_cli::process::build_command("git", args, root, &[])
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

fn sprint_report(lane: &str, when: &str, passed: u64, failed: u64) -> String {
    format!(
        r#"{{"generatedAt":"{when}","profile":"mesh","env":{{"lane":"{lane}"}},
"summary":{{"byConcern":{{"{HABIT}":{{"passed":{passed},"failed":{failed},"pending":0,"skipped":0}}}}}}}}"#
    )
}

fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, ".claude/epr-meta/measures.yaml", MEASURES);
    write(root, "genesis/manifests/habits.yaml", REGISTER);
    write(root, ATOM, ATOM_BODY);
    write(root, "ops/series.mjs", "// the series\n");
    write(
        root,
        "genesis/manifests/cluster-state.act1-household.yaml",
        HOUSEHOLD_LANE,
    );
    write(root, "genesis/manifests/cluster-state.yaml", LIVE_LANE);
    // A household-runnable proof the habit's checks never name: an orphan.
    write(
        root,
        "genesis/a2o/features/delivery/refuses-fast.feature",
        "# prose mentioning @concern:not-a-tag is not read\n\
         @e2e @concern:push-delivers @requires:household-nodes\n\
         Feature: refuses fast\n  Scenario: it refuses\n    Given a thing\n",
    );
    // A proof that only a substrate nobody has can run.
    write(
        root,
        "genesis/a2o/features/delivery/needs-shem.feature",
        "@concern:push-delivers @requires:shem\nFeature: needs shem\n  Scenario: s\n    Given x\n",
    );
    // Receipts: an OLD red household run, a NEWER green one, and the newest live-local run red.
    write(
        root,
        "genesis/a2o/reports/sprint-report-household-20260901T000000Z-a.json",
        &sprint_report("household", "2026-09-01T00:00:00.000Z", 1, 2),
    );
    write(
        root,
        "genesis/a2o/reports/sprint-report-household-20260920T000000Z-b.json",
        &sprint_report("household", "2026-09-20T00:00:00.000Z", 3, 0),
    );
    write(
        root,
        "genesis/a2o/reports/sprint-report-dataplane.json",
        &sprint_report("alpha-fleet", "2026-09-22T00:00:00.000Z", 1, 1),
    );
    // A lane that names no rung is evidence of nothing.
    write(
        root,
        "genesis/a2o/reports/sprint-report-unknown.json",
        &sprint_report("unknown", "2026-09-23T00:00:00.000Z", 9, 0),
    );
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "fixture"]);
    dir
}

fn fold(root: &Path, measure: &str, subject: &str, value: f64) {
    observe(
        root,
        "observation",
        measure,
        subject,
        value,
        None,
        &BTreeMap::new(),
        None,
        &NoteActor::default(),
        &root.join(".claude/epr-meta/measures.yaml"),
    )
    .expect("observation runs");
}

fn claim(root: &Path, gap: &str, habit: &str, valid_from: &str) {
    let scope = body_cid_of_file(&root.join(ATOM)).expect("the atom has a body");
    let record = FlowRecord::Commitment(Commitment {
        action: ReaVerb::Produce,
        provider: AgentRef("agent:implementer@fixture".into()),
        receiver: AgentRef("repo".into()),
        resource_spec: ResourceSpec {
            classified_as: vec!["gap:claimed".into(), gap.into(), format!("habit:{habit}")],
            quantity: None,
        },
        in_scope_of: scope,
        valid_from: Some(valid_from.into()),
        valid_until: None,
        state: CommitmentState::Active,
        satisfies: vec![],
        bound: None,
    });
    SidecarFlowStore::open(root)
        .expect("sidecar opens")
        .append(record)
        .expect("append");
}

fn lines(diagnostics: &[walk::Diagnostic]) -> Vec<String> {
    diagnostics.iter().map(walk::Diagnostic::line).collect()
}

#[test]
fn a_habit_walk_reads_its_tier_from_the_newest_receipt_per_rung() {
    let dir = fixture();
    let root = dir.path();
    let result = walk::walk(root, ATOM).expect("walk runs");
    let habit = result.habit.expect("a habit atom walks as a habit");
    assert_eq!(habit.id, HABIT);
    assert_eq!(habit.status, "red");

    let tier = &habit.tier;
    let green = tier.highest_green.as_ref().expect("household went green");
    assert_eq!(green.rung, "T2", "the NEWER household receipt is green");
    assert_eq!(green.label, "household");
    assert_eq!(green.when, "2026-09-20T00:00:00.000Z");
    let rungs: Vec<(&str, bool)> = tier
        .rungs
        .iter()
        .map(|r| (r.rung.as_str(), r.green))
        .collect();
    assert_eq!(
        rungs,
        vec![("T2", true), ("T3", false)],
        "a red above a green is shown, never folded away; an unknown lane is no rung"
    );
    assert_eq!(tier.unread, vec!["T0", "T1"]);
    assert_eq!(
        tier.scenarios,
        vec![
            "genesis/a2o/features/delivery/needs-shem.feature",
            "genesis/a2o/features/delivery/refuses-fast.feature",
        ]
    );
    assert_eq!(tier.requires, vec!["household-nodes", "shem"]);
    assert_eq!(
        tier.runnable_from.as_deref(),
        Some("T2"),
        "the lowest rung ANY proof file can run at"
    );
}

#[test]
fn a_habit_walk_prices_itself_from_the_cost_bounds_addressed_to_it() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "stage-wallclock@1", ".", 131.9);
    fold(root, "delivery-cost@1", ".", 16.5);
    fold(root, "operator-surfaced-pain@1", ".", 1.0);

    let result = walk::walk(root, ATOM).expect("walk runs");
    let cost = &result.habit.as_ref().unwrap().cost;
    let measures: Vec<(&str, &str)> = cost
        .bounds
        .iter()
        .map(|b| (b.measure.as_str(), b.outcome.as_str()))
        .collect();
    assert_eq!(
        measures,
        vec![
            ("stage-wallclock@1", "failed"),
            ("delivery-cost@1", "failed")
        ],
        "only the bounds whose concern is THIS habit; the pain lens prices another"
    );
    assert_eq!(cost.bounds[0].observed, Some(131.9));
    assert!(
        cost.attested_builds.is_empty() && cost.attested_build_ms.is_none(),
        "no brit build note on HEAD is no attestation, never a zero"
    );

    // The JSON a portal reads carries both columns.
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["habit"]["tier"]["highest_green"]["rung"], "T2");
    assert_eq!(json["habit"]["cost"]["bounds"][1]["observed"], 16.5);
}

#[test]
fn every_commitment_serving_the_habit_carries_its_tier_and_cost() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "stage-wallclock@1", ".", 12.0);
    claim(root, "plan#1", HABIT, "2026-09-23T10:00:00Z");
    claim(root, "plan#2", "some-other-habit", "2026-09-23T10:00:00Z");

    let result = walk::walk(root, ATOM).expect("walk runs");
    let serving = &result.habit.as_ref().unwrap().serving;
    assert_eq!(serving.len(), 1, "only the claim accounted to this habit");
    let view = &serving[0].commitment;
    assert_eq!(view.habit.as_deref(), Some(HABIT));
    let tier = view
        .tier
        .as_ref()
        .expect("an accounted commitment is tiered");
    assert_eq!(tier.rung.as_deref(), Some("T2"));
    assert_eq!(tier.when.as_deref(), Some("2026-09-20T00:00:00.000Z"));
    let cost: Vec<(&str, &str)> = view
        .cost
        .iter()
        .map(|c| (c.measure.as_str(), c.outcome.as_str()))
        .collect();
    assert_eq!(
        cost,
        vec![
            ("stage-wallclock@1", "passed"),
            ("delivery-cost@1", "skipped")
        ],
        "an unmeasured cost is skipped, never zero"
    );
    assert!(!serving[0].fulfilled);
}

#[test]
fn the_walk_speaks_compiler_format_for_dangling_proof_orphans_and_stale_claims() {
    let dir = fixture();
    let root = dir.path();
    claim(root, "plan#7", HABIT, "2026-08-01T09:00:00Z");
    claim(root, "plan#8", HABIT, "2099-01-01T00:00:00Z");

    let result = walk::walk(root, ATOM).expect("walk runs");
    let got = lines(&result.diagnostics);
    assert!(
        got.contains(
            &"ERROR dangling-proof ops/missing.test.mjs: habit `push-delivers` check 2 names it, \
              and it is not on disk"
                .to_string()
        ),
        "{got:#?}"
    );
    assert!(
        !got.iter().any(|l| l.contains("ops/series.mjs")),
        "a proof on disk is not dangling: {got:#?}"
    );
    assert!(
        !got.iter().any(|l| l.contains("jenkins.example")),
        "a URL is not a repository path: {got:#?}"
    );
    for feature in ["needs-shem.feature", "refuses-fast.feature"] {
        assert!(
            got.iter()
                .any(|l| l.starts_with("WARN orphan ") && l.contains(feature)),
            "{feature}: {got:#?}"
        );
    }
    let stale: Vec<&String> = got
        .iter()
        .filter(|l| l.starts_with("ERROR stale-claim "))
        .collect();
    assert_eq!(stale.len(), 1, "{got:#?}");
    assert!(stale[0].starts_with(
        "ERROR stale-claim plan#7: claimed by agent:implementer@fixture on 2026-08-01"
    ));
}

#[test]
fn status_reads_every_habit_and_names_a_claim_whose_habit_is_gone() {
    let dir = fixture();
    let root = dir.path();
    claim(root, "plan#9", "retired-habit", "2099-01-01T00:00:00Z");
    let status = walk::status(root).expect("status runs");
    let got = lines(&status.diagnostics);
    assert!(
        got.contains(
            &"WARN orphan plan#9: claim serves habit `retired-habit`, which the register no \
              longer declares"
                .to_string()
        ),
        "{got:#?}"
    );
    assert!(
        got.iter()
            .any(|l| l.starts_with("ERROR dangling-proof ops/missing.test.mjs")),
        "{got:#?}"
    );
    assert!(
        !got.iter().any(|l| l.contains("some-other-habit") && l.contains("orphan")),
        "a habit whose check names its @concern tag expression owns every tagged scenario: {got:#?}"
    );
}

#[test]
fn a_plain_document_walk_is_unchanged_when_there_is_nothing_to_join() {
    let dir = fixture();
    let root = dir.path();
    let result = walk::walk(root, "ops/series.mjs").expect("walk runs");
    assert!(result.habit.is_none());
    assert!(result.diagnostics.is_empty());
    let json = serde_json::to_value(&result).unwrap();
    assert!(json.get("habit").is_none() && json.get("diagnostics").is_none());
}

#[test]
fn the_cost_headline_line_composes_every_member_and_takes_the_worst_verdict() {
    let dir = fixture();
    let root = dir.path();
    let before = report(root, &ReportOptions::new(root)).unwrap();
    let line = before.headline_line("cost");
    assert!(
        line.starts_with("cost: skipped (") && line.contains("stage-wallclock@1"),
        "nothing folded: skipped, naming what is missing — {line}"
    );

    fold(root, "stage-wallclock@1", ".", 12.0);
    fold(root, "delivery-cost@1", ".", 0.5);
    let green = report(root, &ReportOptions::new(root))
        .unwrap()
        .headline_line("cost");
    assert!(
        green.starts_with("cost: stage-wallclock 12 minutes"),
        "{green}"
    );
    assert!(
        green.contains(" · delivery-cost 0.5 pipeline-hours"),
        "{green}"
    );
    assert!(green.contains("operator-surfaced-pain skipped"), "{green}");
    assert!(green.ends_with('✅'), "{green}");

    fold(root, "operator-surfaced-pain@1", ".", 1.0);
    let red = report(root, &ReportOptions::new(root))
        .unwrap()
        .headline_line("cost");
    assert!(
        red.starts_with("cost: ⚠ failed — stage-wallclock 12"),
        "{red}"
    );
    assert!(
        red.contains(
            "operator-surfaced-pain 1 count since the beginning is past the hard watermark 0"
        ),
        "{red}"
    );

    fold(root, "operator-surfaced-pain-reset@1", ".", 1.0);
    let drained = report(root, &ReportOptions::new(root))
        .unwrap()
        .headline_line("cost");
    assert!(
        drained.ends_with('✅') && drained.contains("operator-surfaced-pain 0"),
        "a reset drains the pain to a witnessed 0 — {drained}"
    );
}
