//! `epr flow report scope` and `epr flow hold --scope` — the plate against the substrate.
//!
//! Station two of `genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md`.
//!
//! Two kinds of test here, and the split is deliberate. The synthetic-tempdir tests exercise the
//! DECISIONS — gap-granularity, the act baseline, affirmative-evidence-to-publish, the unknown-cap
//! asymmetry between `.md` and `.feature` — because those arms are the ones a port loses quietly and
//! the live tree does not currently exhibit all of them. The live-tree test compares the whole
//! reading against `scope-reconcile.py --report` on the real `cluster-state.yaml`, which is the only
//! way to know the port agrees with the mover about the repository it actually governs.
//!
//! The live test is READ-ONLY. It never calls `hold_scope`, because the moves it would perform are
//! `git mv`s across the plate and a test suite has no business reconciling anybody's tree.

use std::path::{Path, PathBuf};
use std::process::Command;

use elohim_epr_cli::flow::scope::{
    doc_scope, env_line, focus_baseline, hold_scope, remote_compute_status, scope_report, set_state,
};
use tempfile::TempDir;

/// The scope line as recorded on 2026-09-10 against the live `cluster-state.yaml`.
///
/// This is the witness of record once `scope-reconcile.py` is gone. It moves the moment somebody
/// runs `epr flow hold --scope --apply` (the three features move to `held/` and the line becomes
/// `aligned`), which is correct and expected — re-pin it then. It is asserted only when the oracle
/// is unavailable, so an ordinary reconcile does not red the gate while the oracle is still there
/// to answer.
const RECORDED_SCOPE_LINE_2026_09_10: &str = "scope: ⚠ 3 to hold (local-conductor,owned-substrate)";

const CLUSTER_STATE: &str = r#"updated: 2026-09-10
resources:
  household-nodes:
    role: the household mesh
    available: true
  shem:
    role: remote compute
    available: false
  owned-substrate:
    role: the lane OWNS its substrate
    available: false
"#;

/// The Act I lane contract — where `owned-substrate` IS available.
const ACT1: &str = r#"updated: 2026-09-10
resources:
  owned-substrate:
    available: true
"#;

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

fn base() -> TempDir {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "genesis/manifests/cluster-state.yaml",
        CLUSTER_STATE,
    );
    write(
        tmp.path(),
        "genesis/manifests/cluster-state.act1-household.yaml",
        ACT1,
    );
    std::fs::create_dir_all(tmp.path().join(".claude")).unwrap();
    tmp
}

fn repo_root() -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
}

#[test]
fn a_uniformly_blocked_plan_is_held_and_a_mixed_one_is_not() {
    let tmp = base();
    let root = tmp.path();
    // Doc-level requires_env ⇒ every station inherits it ⇒ held whole.
    write(
        root,
        "genesis/docs/superpowers/plans/uniform.md",
        "---\ntitle: u\nrequires_env: [shem]\n---\n\n- [ ] one\n- [ ] two\n",
    );
    // No doc-level default; only one station names the absent capability ⇒ stays on the plate.
    write(
        root,
        "genesis/docs/superpowers/plans/mixed.md",
        "---\ntitle: m\n---\n\n- [ ] household work\n- [ ] remote work @requires:shem\n",
    );
    let report = scope_report(root, None).expect("scope derives");
    let held: Vec<&str> = report.to_held.iter().map(|m| m.src.as_str()).collect();
    assert_eq!(held, vec!["genesis/docs/superpowers/plans/uniform.md"]);
    assert_eq!(report.pending_moves, 1);
    assert_eq!(report.headline_parts(), "scope: ⚠ 1 to hold (shem)");
    assert!(report
        .headline()
        .contains("→  epr flow hold --scope --apply"));
}

#[test]
fn a_held_doc_returns_only_on_affirmative_evidence() {
    let tmp = base();
    let root = tmp.path();
    // Its capability is available again → returns.
    write(
        root,
        "genesis/docs/superpowers/held/plans/returnable.md",
        "---\ntitle: r\nrequires_env: [household-nodes]\n---\n\n# body\n",
    );
    // No parseable scope info → STAYS held, reported as an anomaly.
    write(
        root,
        "genesis/docs/superpowers/held/plans/mute.md",
        "# just prose\n",
    );
    let report = scope_report(root, None).expect("scope derives");
    assert_eq!(
        report
            .to_live
            .iter()
            .map(|m| m.src.as_str())
            .collect::<Vec<_>>(),
        vec!["genesis/docs/superpowers/held/plans/returnable.md"]
    );
    assert_eq!(
        report.held_anomalies,
        vec!["genesis/docs/superpowers/held/plans/mute.md".to_string()]
    );
    assert!(report.headline_parts().contains("1 to return to plate"));
}

#[test]
fn a_feature_is_rescued_by_its_own_acts_lane_contract() {
    let tmp = base();
    let root = tmp.path();
    // Untagged: the default lane withholds owned-substrate → held.
    write(
        root,
        "genesis/a2o/features/dataplane/plain.feature",
        "@requires:owned-substrate\nFeature: plain\n  Scenario: x\n",
    );
    // `@act:i`: exercised only via the Act I lane, where owned-substrate IS available → stays live.
    write(
        root,
        "genesis/a2o/features/dataplane/act1.feature",
        "@act:i @requires:owned-substrate\nFeature: act one\n  Scenario: x\n",
    );
    let report = scope_report(root, None).expect("scope derives");
    let held: Vec<&str> = report.to_held.iter().map(|m| m.src.as_str()).collect();
    assert_eq!(held, vec!["genesis/a2o/features/dataplane/plain.feature"]);
}

#[test]
fn an_unknown_cap_is_drift_on_a_doc_and_a_fixture_tag_on_a_feature() {
    let tmp = base();
    let root = tmp.path();
    write(
        root,
        "genesis/docs/superpowers/specs/typo.md",
        "---\ntitle: t\nrequires_env: [harbor]\n---\n\n# body\n",
    );
    write(
        root,
        "genesis/a2o/features/auth/fixture.feature",
        "@requires:doorway\nFeature: f\n  Scenario: x\n",
    );
    let report = scope_report(root, None).expect("scope derives");
    assert_eq!(report.vocab.len(), 1, "only the .md is vocab drift");
    assert_eq!(
        report.vocab[0].path,
        "genesis/docs/superpowers/specs/typo.md"
    );
    assert!(report.headline().contains("⚠ unknown-cap: harbor"));
    // An unknown cap does NOT hold: it is conservative about publishing, not about benching.
    assert!(report.to_held.is_empty());
}

#[test]
fn apply_moves_the_tree_writes_the_stop_and_refreshes_the_baseline() {
    let tmp = base();
    let root = tmp.path();
    write(
        root,
        "genesis/a2o/features/dataplane/blocked.feature",
        "@requires:owned-substrate\nFeature: b\n  Scenario: x\n",
    );
    let after = hold_scope(root, true, None).expect("apply reconciles");
    assert!(after.to_held.is_empty(), "the move is done, not pending");
    assert!(!root
        .join("genesis/a2o/features/dataplane/blocked.feature")
        .exists());
    assert!(root
        .join("genesis/a2o/held/features/dataplane/blocked.feature")
        .is_file());
    // The STOP marker lands at the zone root, so a reader entering the tree is warned once.
    let stop = root.join("genesis/a2o/held/CLAUDE.md");
    assert!(stop.is_file());
    assert!(std::fs::read_to_string(&stop).unwrap().contains("STOP"));
    // The co-located baseline is refreshed in lockstep with the plate.
    let baseline = root.join(".claude/subject-focus.md");
    assert!(baseline.is_file());
    assert!(std::fs::read_to_string(&baseline)
        .unwrap()
        .contains("SUBJECT FOCUS BASELINE"));
}

#[test]
fn a_dry_run_changes_nothing() {
    let tmp = base();
    let root = tmp.path();
    let path = "genesis/a2o/features/dataplane/blocked.feature";
    write(
        root,
        path,
        "@requires:owned-substrate\nFeature: b\n  Scenario: x\n",
    );
    let report = hold_scope(root, false, None).expect("dry run derives");
    assert_eq!(report.to_held.len(), 1);
    assert!(
        root.join(path).is_file(),
        "the dry run must not move anything"
    );
    assert!(!root.join(".claude/subject-focus.md").exists());
}

#[test]
fn the_developer_flip_edits_the_durable_home_and_derives_the_runtime_export() {
    let tmp = base();
    let root = tmp.path();
    assert_eq!(remote_compute_status(root, None), "unavailable");
    assert_eq!(
        env_line(root, None),
        "export ELOHIM_REMOTE_COMPUTE_STATUS=unavailable"
    );
    set_state(root, "shem=on", false, None).expect("flip succeeds");
    let text = std::fs::read_to_string(root.join("genesis/manifests/cluster-state.yaml")).unwrap();
    assert!(text.contains("  shem:\n    role: remote compute\n    available: true\n"));
    // The two homes cannot disagree: the export is DERIVED from the file just written.
    assert_eq!(remote_compute_status(root, None), "available");
    assert_eq!(
        env_line(root, None),
        "export ELOHIM_REMOTE_COMPUTE_STATUS=available"
    );
}

#[test]
fn an_unknown_resource_or_state_is_refused_by_name() {
    let tmp = base();
    let root = tmp.path();
    let err = set_state(root, "nosuchcap=on", false, None).unwrap_err();
    assert!(format!("{err}").contains("unknown resource `nosuchcap`"));
    let err = set_state(root, "shem=maybe", false, None).unwrap_err();
    assert!(format!("{err}").contains("unknown state `maybe`"));
    let err = set_state(root, "shem", false, None).unwrap_err();
    assert!(format!("{err}").contains("--set <resource>=<on|off|degraded>"));
}

#[test]
fn an_aligned_plate_prints_no_pointer() {
    let tmp = base();
    let report = scope_report(tmp.path(), None).expect("scope derives");
    assert_eq!(report.pending_moves, 0);
    assert_eq!(
        report.headline(),
        "scope: aligned ✅  (plate matches substrate)"
    );
}

#[test]
fn the_focus_baseline_says_full_focus_when_nothing_is_narrowed() {
    let tmp = base();
    write(
        tmp.path(),
        "genesis/a2o/features/lms/plain.feature",
        "Feature: plain\n  Scenario: x\n",
    );
    let baseline = focus_baseline(tmp.path(), None).expect("focus derives");
    assert!(baseline.render(None).contains("✅ FULL FOCUS"));
}

/// The oracle leg over the LIVE cluster-state. Read-only.
#[test]
fn the_kit_agrees_with_the_native_scope_reading() {
    let Some(repo) = repo_root() else {
        eprintln!("SKIP: could not resolve the repository root from CARGO_MANIFEST_DIR");
        return;
    };
    let script = repo.join(".claude/scripts/memory-kit/scope-reconcile.py");
    if !script.is_file() {
        eprintln!(
            "SKIP: {} is absent (expected once station six deletes the kit) — \
             RECORDED_SCOPE_LINE_2026_09_10 remains the witness",
            script.display()
        );
        return;
    }
    let output = match Command::new("python3")
        .arg(&script)
        .arg("--report")
        .current_dir(&repo)
        .output()
    {
        Ok(out) if out.status.success() => out,
        Ok(out) => {
            eprintln!("SKIP: the kit exited {:?}", out.status.code());
            return;
        }
        Err(err) => {
            eprintln!("SKIP: python3 is not runnable here ({err})");
            return;
        }
    };
    let kit_line = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let report = scope_report(&repo, None).expect("scope derives over the live tree");
    // Compare everything up to the next-action pointer: the counts, the capability names and the
    // vocab note must be identical. The pointer itself legitimately differs — the kit names its own
    // script, and the whole point of the native verb is that it names a different one.
    let kit_parts = kit_line
        .split("  →  ")
        .next()
        .unwrap_or(&kit_line)
        .to_string();
    let kit_vocab = kit_line
        .split_once("  ⚠ unknown-cap: ")
        .map(|(_, rest)| rest.to_string());
    assert_eq!(
        report.headline_parts(),
        kit_parts,
        "native scope parts diverged from scope-reconcile.py --report"
    );
    match kit_vocab {
        Some(rest) => assert!(
            report
                .vocab_note()
                .contains(rest.split(' ').next().unwrap_or("")),
            "native vocab note diverged from the kit's"
        ),
        None => assert!(
            !report.vocab_note().contains("unknown-cap"),
            "native reported vocab drift the kit did not"
        ),
    }
}

/// The pinned witness, asserted exactly when the oracle cannot be asked.
#[test]
fn the_recorded_scope_line_is_the_witness_once_the_kit_is_gone() {
    let Some(repo) = repo_root() else {
        eprintln!("SKIP: could not resolve the repository root");
        return;
    };
    let oracle_present = repo
        .join(".claude/scripts/memory-kit/scope-reconcile.py")
        .is_file()
        && Command::new("python3").arg("--version").output().is_ok();
    if oracle_present {
        eprintln!(
            "NOTE: the oracle is still present, so the recorded line is documentation rather than \
             the assertion. Recorded 2026-09-10: {RECORDED_SCOPE_LINE_2026_09_10}"
        );
        return;
    }
    let report = scope_report(&repo, None).expect("scope derives over the live tree");
    assert_eq!(
        report.headline_parts(),
        RECORDED_SCOPE_LINE_2026_09_10,
        "the plate moved since the line was recorded — re-pin RECORDED_SCOPE_LINE_2026_09_10"
    );
}

// ── station-two review round two ────────────────────────────────────────────────────────────────

#[test]
fn stations_are_not_scope_information() {
    // A held `.md` with unticked stations, no doc-level `requires_env` and no `@requires:` tag says
    // NOTHING about scope. Reading its stations as scope information made every one of them
    // trivially "not blocked", so the doc read Live and `--apply` would have `git mv`d it back onto
    // the plate for the sole reason that it has checkboxes. Ambiguity and station-derivation are
    // separable; this is the separation.
    let tmp = base();
    let root = tmp.path();
    write(
        root,
        "genesis/docs/superpowers/held/plans/mute.md",
        "---\ntitle: mute\n---\n\n- [ ] one\n- [ ] two\n",
    );
    let report = scope_report(root, None).expect("scope derives");
    assert!(report.to_live.is_empty(), "no move may be proposed");
    assert_eq!(
        report.held_anomalies,
        vec!["genesis/docs/superpowers/held/plans/mute.md".to_string()],
        "it stays held and is reported for operator triage"
    );
    assert_eq!(report.pending_moves, 0);

    // …and the same shape on the LIVE side is not held either: nothing is known about it.
    let tmp = base();
    let root = tmp.path();
    write(
        root,
        "genesis/docs/superpowers/plans/mute.md",
        "---\ntitle: mute\n---\n\n- [ ] one\n- [ ] two\n",
    );
    let report = scope_report(root, None).expect("scope derives");
    assert!(report.to_held.is_empty());
    assert_eq!(report.pending_moves, 0);
}

#[test]
fn one_tagged_station_is_enough_scope_information_to_decide() {
    // The boundary: the moment ANY station carries a tag, the station path is the right reading.
    let tmp = base();
    let root = tmp.path();
    write(
        root,
        "genesis/docs/superpowers/plans/all-blocked.md",
        "---\ntitle: b\n---\n\n- [ ] one @requires:shem\n",
    );
    let report = scope_report(root, None).expect("scope derives");
    assert_eq!(
        report
            .to_held
            .iter()
            .map(|m| m.src.as_str())
            .collect::<Vec<_>>(),
        vec!["genesis/docs/superpowers/plans/all-blocked.md"]
    );
}

#[test]
fn a_trailing_space_on_a_resource_key_does_not_move_a_capability() {
    // The `\s*$` arm, exercised through the whole scope reading rather than only the parser: the
    // lost key merged its `available: true` into the block above it, flipping an unavailable
    // capability to available and changing the proposed move set `--apply` would act on.
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    write(
        root,
        "genesis/manifests/cluster-state.yaml",
        "updated: 2026-09-10\nresources:\n  local-conductor:\n    available: false\n  shem: \n    available: true\n",
    );
    write(
        root,
        "genesis/docs/superpowers/plans/needs-conductor.md",
        "---\ntitle: c\nrequires_env: [local-conductor]\n---\n\n# body\n",
    );
    let report = scope_report(root, None).expect("scope derives");
    assert_eq!(report.available, vec!["shem".to_string()]);
    assert_eq!(
        report.to_held.len(),
        1,
        "local-conductor is unavailable, so the doc that needs it is held"
    );
}

#[test]
fn a_dry_run_surfaces_a_destination_collision() {
    // The operator must learn about a skipped move at DRY-RUN time, not mid-apply. The conflict is
    // therefore part of the DERIVATION — carried on the move, rendered with it, and refused before
    // any `git mv` — rather than discovered inside the writer.
    //
    // Every assertion below reads the SHIPPED renderer's return value. An earlier version of this
    // test re-implemented the rendering locally, which meant it passed unchanged after the real
    // `render_conflict()` calls were deleted: it was asserting against its own copy. `render`
    // returns a `String` precisely so a test cannot do that.
    let tmp = base();
    let root = tmp.path();
    let live = "genesis/a2o/features/dataplane/clash.feature";
    let held = "genesis/a2o/held/features/dataplane/clash.feature";
    write(
        root,
        live,
        "@requires:owned-substrate\nFeature: c\n  Scenario: x\n",
    );
    write(
        root,
        held,
        "@requires:owned-substrate\nFeature: stale\n  Scenario: x\n",
    );

    let report = hold_scope(root, false, None).expect("dry run derives");
    assert_eq!(report.to_held.len(), 1);
    assert!(
        report.to_held[0].conflict,
        "the collision is derived WITH the move, not discovered at write time"
    );

    // THE SHIPPED OUTPUT — the same string `hold_scope` prints.
    let rendered = report.render(false);
    assert!(
        rendered.contains("⚠ CONFLICT"),
        "the dry run must print the CONFLICT line — got:\n{rendered}"
    );
    assert!(
        rendered
            .lines()
            .any(|l| l.contains("⚠ CONFLICT") && l.contains(live)),
        "the CONFLICT line must NAME the colliding path — got:\n{rendered}"
    );
    // …and it sits under its own move, above the summary, never after it.
    let conflict_at = rendered.find("⚠ CONFLICT").expect("conflict line present");
    let move_at = rendered.find("→ HELD").expect("move line present");
    let summary_at = rendered
        .find("epr flow hold --scope [")
        .expect("summary line present");
    assert!(
        move_at < conflict_at && conflict_at < summary_at,
        "the CONFLICT belongs between its move and the summary — got:\n{rendered}"
    );

    // Both files survive the apply, which refuses the move rather than clobbering.
    let after = hold_scope(root, true, None).expect("apply runs");
    assert!(
        root.join(live).is_file(),
        "the source must not be clobbered away"
    );
    assert!(root.join(held).is_file());
    assert_eq!(
        after.to_held.len(),
        1,
        "the move is still pending after the refusal"
    );
    assert!(after.to_held[0].conflict);
    assert!(
        after.render(true).contains("⚠ CONFLICT"),
        "the apply rendering carries it too"
    );
}

#[test]
fn a_clean_move_set_renders_no_conflict_line() {
    // The other half: without this, "contains CONFLICT" could be satisfied by a renderer that always
    // emits it.
    let tmp = base();
    let root = tmp.path();
    write(
        root,
        "genesis/a2o/features/dataplane/clean.feature",
        "@requires:owned-substrate\nFeature: c\n  Scenario: x\n",
    );
    let report = scope_report(root, None).expect("scope derives");
    assert_eq!(report.to_held.len(), 1);
    assert!(!report.to_held[0].conflict);
    assert!(!report.render(false).contains("CONFLICT"));
}

#[test]
fn apply_creates_the_baseline_parent_rather_than_failing_after_the_work() {
    // A repository with no `.claude/` — a fresh clone, a scoped worktree — took every move, printed
    // APPLIED, and THEN failed writing the co-located baseline. An error raised after the work is
    // done reads as "the reconcile failed" when it succeeded.
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    write(root, "genesis/manifests/cluster-state.yaml", CLUSTER_STATE);
    write(
        root,
        "genesis/a2o/features/dataplane/blocked.feature",
        "@requires:owned-substrate\nFeature: b\n  Scenario: x\n",
    );
    assert!(
        !root.join(".claude").exists(),
        "the fixture must start without the parent directory"
    );
    let after = hold_scope(root, true, None).expect("apply must not fail on a missing parent");
    assert!(after.to_held.is_empty(), "the move still happened");
    assert!(root
        .join("genesis/a2o/held/features/dataplane/blocked.feature")
        .is_file());
    let baseline = root.join(".claude/subject-focus.md");
    assert!(
        baseline.is_file(),
        "the baseline is written, parent and all"
    );
    assert!(std::fs::read_to_string(&baseline)
        .unwrap()
        .contains("SUBJECT FOCUS BASELINE"));
}

#[test]
fn pruning_never_removes_the_held_zone_root() {
    let tmp = base();
    let root = tmp.path();
    // A held tree whose only real doc returns to the plate: descendants are pruned, the root stays.
    write(
        root,
        "genesis/a2o/held/features/dataplane/returning.feature",
        "@requires:household-nodes\nFeature: r\n  Scenario: x\n",
    );
    hold_scope(root, true, None).expect("apply runs");
    assert!(
        root.join("genesis/a2o/held/features").is_dir(),
        "the zone root is a location, not a file — it stays"
    );
    assert!(
        !root.join("genesis/a2o/held/features/dataplane").is_dir(),
        "the emptied subdirectory is pruned"
    );
}

#[test]
fn set_leaves_a_bare_available_line_alone() {
    // `^(    available:\s*)(\S+)(.*)$` needs a value to replace. Writing `    available:false` into
    // a bare key would be a line only this implementation can produce — and the parser does not read
    // a bare field as a declaration either.
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let manifest = "updated: 2026-09-10\nresources:\n  shem:\n    available:\n    role: bare\n";
    write(root, "genesis/manifests/cluster-state.yaml", manifest);
    set_state(root, "shem=on", false, None).expect("flip runs");
    assert_eq!(
        std::fs::read_to_string(root.join("genesis/manifests/cluster-state.yaml")).unwrap(),
        manifest,
        "nothing may be written into a line the kit's pattern does not match"
    );
}

#[test]
fn a_non_utf8_document_stays_in_the_scan() {
    let tmp = base();
    let root = tmp.path();
    let path = root.join("genesis/docs/superpowers/plans/mojibake.md");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    // Valid frontmatter, one invalid byte in the body.
    let mut bytes = b"---\ntitle: m\nrequires_env: [shem]\n---\n\nbody ".to_vec();
    bytes.push(0xFF);
    bytes.push(b'\n');
    std::fs::write(&path, bytes).unwrap();
    let report = scope_report(root, None).expect("scope derives");
    assert_eq!(
        report
            .to_held
            .iter()
            .map(|m| m.src.as_str())
            .collect::<Vec<_>>(),
        vec!["genesis/docs/superpowers/plans/mojibake.md"],
        "a bad byte must not remove a document from the scope walk"
    );
}

#[test]
fn the_doc_scope_reading_splits_by_availability() {
    let tmp = base();
    let root = tmp.path();
    write(
        root,
        "genesis/docs/superpowers/specs/in-scope.md",
        "---\ntitle: a\nrequires_env: [household-nodes]\n---\n\n# a\n",
    );
    write(
        root,
        "genesis/docs/superpowers/specs/blocked.md",
        "---\ntitle: b\nrequires_env: [shem]\n---\n\n# b\n",
    );
    write(
        root,
        "genesis/docs/superpowers/specs/silent.md",
        "---\ntitle: c\n---\n\n# c\n",
    );
    let docs = doc_scope(root, None).expect("doc scope derives");
    assert_eq!(
        docs.in_scope
            .iter()
            .map(|r| r.path.as_str())
            .collect::<Vec<_>>(),
        vec!["genesis/docs/superpowers/specs/in-scope.md"]
    );
    assert_eq!(
        docs.blocked
            .iter()
            .map(|r| r.path.as_str())
            .collect::<Vec<_>>(),
        vec!["genesis/docs/superpowers/specs/blocked.md"]
    );
    assert_eq!(docs.blocked[0].missing, vec!["shem".to_string()]);
    let rendered = {
        // The rendering is the parity surface prep-brainstorm.py embeds.
        let mut out = String::new();
        out.push_str(&format!("{:?}", docs.available));
        out
    };
    assert!(rendered.contains("household-nodes"));
}

#[test]
fn the_brief_baseline_names_the_narrowed_subjects_and_points_at_the_drill_in() {
    let tmp = base();
    let root = tmp.path();
    write(
        root,
        "genesis/a2o/features/lms/plain.feature",
        "Feature: plain\n  Scenario: x\n",
    );
    write(
        root,
        "genesis/a2o/held/features/dataplane/held.feature",
        "@requires:owned-substrate\nFeature: h\n  Scenario: x\n",
    );
    let baseline = focus_baseline(root, None).expect("focus derives");
    let brief = baseline.render_brief();
    assert!(brief.contains("FAIR-GAME subjects (1)"));
    assert!(brief.contains("NARROWED subjects (1)"));
    assert!(brief.contains("  dataplane"));
    assert!(brief.contains("→ drill in: epr flow report placement --focus <name>"));
    // …and it stops there: no per-subject section.
    assert!(!brief.contains("IN FOCUS  :"));
}
