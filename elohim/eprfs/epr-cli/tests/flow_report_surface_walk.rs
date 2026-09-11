//! `derive: files-newer-than` and `status: superseded` — the two registry mechanics station six
//! round (b) needed so `mempalace-currency.py` and `memkit-retention.py` can be deleted.
//!
//! Both are about the same distinction the three-valued vocabulary exists for. A missing build
//! stamp is `skipped`, not a confident zero and not the kit's confident maximum. A retired bound is
//! neither of those: it is not a measurement at all, and rendering it as `skipped` would turn a
//! decision into an accusation of neglect.

use std::path::Path;

use elohim_epr_cli::flow::report::{report, OutcomeStatus, ReportOptions};
use tempfile::TempDir;

const MEASURES: &str = r#"
measures:
  - id: mempalace-surfaces-changed
    version: 1
    family: fixture
    unit: files
    procedure: "epr flow report — the native surface walk"
    status: active

  - id: memkit-tier-megabytes
    version: 1
    family: fixture
    unit: megabytes
    procedure: "none — the thing this measured was removed"
    status: superseded
    superseded_by: "the fixture's placement ledger"
    superseded_reason: "report tier removed 2026-09-11"

lenses:
  - id: mempalace-surfaces-changed-ceiling
    version: 1
    headline: mempalace
    compare: at-or-above
    class: inject
    consumes: [mempalace-surfaces-changed@1]
    hard: 1
    derive: files-newer-than
    marker: .stamp/.last-mine
    marker-fallback: .stamp/config.json
    surfaces:
      - surface/one
      - surface/two
      - surface/absent
    suffix: .md
    grace-seconds: 120
    status: active

  - id: memkit-tier-megabytes-ceiling
    version: 1
    headline: memkit
    class: inject
    consumes: [memkit-tier-megabytes@1]
    soft: 8
    status: superseded
    superseded_by: "placement-budget-ceiling"
    superseded_reason: "report tier removed 2026-09-11"
"#;

fn git(root: &Path, args: &[&str]) {
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

/// Seconds since the epoch, now.
fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock is past the epoch")
        .as_secs_f64()
}

/// Write the build stamp at `epoch`.
///
/// The fixture moves the STAMP rather than the files' mtimes, which is what keeps it honest
/// without a new dependency: every file is written "now", and the stamp decides whether now counts
/// as newer. A stamp far in the past makes all three surface files changed; a stamp at now makes
/// none of them changed, because the declared 120-second grace covers the gap.
fn stamp_at(root: &Path, epoch: f64) {
    write(root, ".stamp/.last-mine", &format!("{epoch}\n"));
}

/// Three `.md` files across two present surfaces, one `.txt` that is not surface, one `.md` inside
/// a `.git` tree that must never be walked, and a third surface that is declared and absent.
fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, ".claude/epr-meta/measures.yaml", MEASURES);
    write(root, "surface/one/a.md", "a\n");
    write(root, "surface/one/b.md", "b\n");
    write(root, "surface/two/c.md", "c\n");
    write(root, "surface/two/not-markdown.txt", "ignored\n");
    write(
        root,
        "surface/two/.git/objects/x.md",
        "must not be walked\n",
    );
    stamp_at(root, now() - 10_000.0);
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "fixture"]);
    dir
}

/// NOTE on the fixture's measure ids. A LIVE outcome's headline slot is derived from its measure
/// id (`slot_of_outcome`), not from the row's `headline:` key — the outcome payload deliberately
/// carries no slot field, because the headline is a projection of the outcomes rather than a
/// property of them. A RETIRED bound, which never becomes an outcome, is matched by its `headline:`
/// key instead. So the fixture's ids carry the live prefixes (`mempalace-`, `memkit-`), which is
/// what the registry's real ids do. The asymmetry is real and is named in the round's report.
fn slot(root: &Path, name: &str) -> String {
    report(root, &ReportOptions::new(root))
        .unwrap()
        .headline_line(name)
}

fn outcome(root: &Path, bound: &str) -> Option<elohim_epr_cli::flow::report::BoundOutcome> {
    report(root, &ReportOptions::new(root))
        .unwrap()
        .outcomes()
        .iter()
        .find(|o| o.bound.starts_with(bound))
        .cloned()
}

#[test]
fn the_surface_walk_counts_only_matching_files_past_the_grace() {
    let dir = fixture();
    let root = dir.path();
    let out = outcome(root, "mempalace-surfaces-changed-ceiling").expect("the bound is evaluated");
    assert_eq!(
        out.observed,
        Some(3.0),
        "all three surface .md files are past a stamp 10,000s old: {}",
        out.summary
    );
    assert_eq!(
        out.contributing_folds,
        Some(3),
        "three .md files were walked; the .txt is not surface and the .git tree is never entered"
    );
    assert_eq!(
        out.outcome,
        OutcomeStatus::Failed,
        "hard watermark is 1, at-or-above"
    );
    assert!(out.summary.contains("surface/absent not present"));
    assert_eq!(out.derive.as_deref(), Some("files-newer-than"));
}

#[test]
fn a_file_inside_the_grace_is_a_witnessed_zero_which_is_not_a_skip() {
    let dir = fixture();
    let root = dir.path();
    // A stamp AT now: every file was written a moment before it, and the declared 120-second grace
    // covers the gap. Zero changed is a real measurement — the surface was walked and nothing was
    // past the tolerance — and it must render as a pass, not as the absence a missing stamp gives.
    stamp_at(root, now());
    let out = outcome(root, "mempalace-surfaces-changed-ceiling").unwrap();
    assert_eq!(out.observed, Some(0.0));
    assert_eq!(
        out.contributing_folds,
        Some(3),
        "walked three, counted none"
    );
    assert_eq!(out.outcome, OutcomeStatus::Passed);
    let line = slot(root, "mempalace");
    assert!(
        !line.contains("skipped") && !line.contains("failed"),
        "a witnessed zero renders as a healthy slot, not an absent one: {line}"
    );
    assert!(
        line.contains("0 of 3"),
        "and it says what it walked: {line}"
    );
}

#[test]
fn a_missing_stamp_is_skipped_never_zero_and_never_everything() {
    let dir = fixture();
    let root = dir.path();
    std::fs::remove_file(root.join(".stamp/.last-mine")).unwrap();

    // With no marker and no fallback, nobody knows when the surface was indexed. The kit falls back
    // to epoch 0, which reports EVERY file as changed — maximum staleness asserted from an absence
    // of evidence. Neither that nor a zero is a measurement.
    let out = outcome(root, "mempalace-surfaces-changed-ceiling").expect("the bound still reports");
    assert_eq!(out.outcome, OutcomeStatus::Skipped);
    assert_eq!(out.observed, None, "a skipped bound reports no magnitude");
    assert!(
        out.summary.contains("no readable build stamp"),
        "a skipped outcome must name what is missing: {}",
        out.summary
    );
    assert!(slot(root, "mempalace").starts_with("mempalace: skipped"));

    // The declared fallback IS a real timestamp (its own mtime), so it measures rather than
    // refusing. Written now, so the grace covers every surface file and the honest answer is zero.
    write(root, ".stamp/config.json", "{}\n");
    let fell_back = outcome(root, "mempalace-surfaces-changed-ceiling").unwrap();
    assert_eq!(fell_back.observed, Some(0.0));
    assert!(fell_back.summary.contains("marker absent"));
}

#[test]
fn an_unparseable_stamp_is_a_broken_stamp_not_a_stamp_reading_zero() {
    let dir = fixture();
    let root = dir.path();
    write(root, ".stamp/.last-mine", "not-an-epoch\n");
    let out = outcome(root, "mempalace-surfaces-changed-ceiling").unwrap();
    assert_eq!(out.outcome, OutcomeStatus::Skipped);
    assert!(out.summary.contains("no readable build stamp"));
}

#[test]
fn a_walk_declaring_no_surfaces_degrades_to_a_plain_reading_rather_than_a_confident_zero() {
    let dir = fixture();
    let root = dir.path();
    let text = std::fs::read_to_string(root.join(".claude/epr-meta/measures.yaml")).unwrap();
    // Strip the surfaces list, leaving `derive: files-newer-than` with nothing to walk.
    let broken: String = text
        .lines()
        .filter(|l| {
            let t = l.trim();
            !(t == "surfaces:" || t.starts_with("- surface/"))
        })
        .collect::<Vec<_>>()
        .join("\n");
    write(root, ".claude/epr-meta/measures.yaml", &broken);
    let out = outcome(root, "mempalace-surfaces-changed-ceiling").unwrap();
    assert_eq!(
        out.outcome,
        OutcomeStatus::Skipped,
        "a bound that declared a surface scan and scanned no surfaces must not report 0"
    );
    assert_eq!(out.derive, None, "it fell back to a plain reading");
}

#[test]
fn a_superseded_bound_leaves_evaluation_and_renders_as_retired() {
    let dir = fixture();
    let root = dir.path();
    let payload = report(root, &ReportOptions::new(root)).unwrap();

    assert!(
        payload
            .outcomes()
            .iter()
            .all(|o| !o.bound.starts_with("memkit-tier-megabytes")),
        "a superseded bound is not an outcome"
    );
    let primary = payload.primary().unwrap();
    assert_eq!(
        primary.totals.passed + primary.totals.failed + primary.totals.skipped,
        primary.outcomes.len(),
        "totals count MEASUREMENTS; a retired row belongs to no bucket"
    );

    let retired = payload.retired();
    assert_eq!(retired.len(), 1);
    assert_eq!(retired[0].bound, "memkit-tier-megabytes-ceiling@1");
    assert_eq!(retired[0].headline.as_deref(), Some("memkit"));
    assert_eq!(retired[0].reason, "report tier removed 2026-09-11");
    assert_eq!(
        retired[0].superseded_by.as_deref(),
        Some("placement-budget-ceiling"),
        "the never-delete rule keeps the lineage readable"
    );

    assert_eq!(
        slot(root, "memkit"),
        "memkit: retired — report tier removed 2026-09-11",
        "NOT `skipped (no bound declared)`: nobody failed to measure this"
    );
}
