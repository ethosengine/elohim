//! Integration tests for the habit plane over a synthetic tempdir repo
//! (`genesis/docs/superpowers/specs/2026-10-22-habit-progress-scalar-design.md` §4–§6).
//!
//! One history exercises every movement the red-mass stock reads, each commit dated (author AND
//! committer) so a window can name it:
//!
//! | date       | change                                   | red mass |
//! |------------|------------------------------------------|----------|
//! | 2026-10-01 | alpha born red (2 parts), beta born green, gamma born red | +1 alpha, +1 gamma |
//! | 2026-10-05 | alpha reads x = 1/2                      | −0.25 progress |
//! | 2026-10-08 | beta flips red                           | +1 flip-red |
//! | 2026-10-10 | alpha's x rescoped to 1/4                | +0.125 rescope |
//! | 2026-10-12 | alpha flips green                        | −0.875 flip-green + discharge |
//! | 2026-10-14 | beta's atom deleted                      | −1 retired + discharge |
//! | 2026-10-15 | gamma's atom moved between directories   | none — a move is not a retirement |

use std::path::Path;

use elohim_epr::measure::Period;
use elohim_epr_cli::flow::habits::{self, RED_MASS_UNIT};
use elohim_epr_cli::flow::note::NoteActor;
use elohim_epr_cli::flow::project;
use elohim_epr_cli::flow::stocks::{self, EquilibriumVerdict, StockName};
use elohim_epr_rea::{FlowRecord, FlowStore, Magnitude, SidecarFlowStore};

fn git_at(root: &Path, when: &str, args: &[&str]) {
    let out = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture Author")
        .env("GIT_AUTHOR_EMAIL", "author@example.test")
        .env("GIT_COMMITTER_NAME", "Fixture Author")
        .env("GIT_COMMITTER_EMAIL", "author@example.test")
        .env("GIT_AUTHOR_DATE", when)
        .env("GIT_COMMITTER_DATE", when)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn commit(root: &Path, day: &str, message: &str) {
    let when = format!("2026-10-{day}T12:00:00+00:00");
    git_at(root, &when, &["add", "-A"]);
    git_at(root, &when, &["commit", "-q", "-m", message]);
}

fn atom(id: &str, status: &str, extra: &str) -> String {
    let checks = if status == "unwired" {
        String::new()
    } else {
        format!("checks:\n  - \"a2o @concern:x-{id} and @concern:y-{id}\"\n")
    };
    format!(
        "---\nepr-habit-version: 1\nid: {id}\ninvariant: {id} holds\nstatus: {status}\n{checks}retire-when: when {id} is superseded\n{extra}---\nledger\n"
    )
}

fn reading(id: &str, value: &str) -> String {
    format!("rea:\n  parts:\n    - part: x-{id}\n      reading: {value}\n      at: 2026-10-01\n      evidence: run-{id}\n")
}

const ALPHA: &str = "a/.epr-meta/alpha.habit.md";
const BETA: &str = "b/.epr-meta/beta.habit.md";
const GAMMA: &str = "c/.epr-meta/gamma.habit.md";
const GAMMA_MOVED: &str = "d/.epr-meta/gamma.habit.md";

fn history() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        ".epr-meta/repository.yaml",
        "version: 1\nagent: repo:ethosengine/elohim\n",
    );
    git_at(root, "2026-10-01T00:00:00+00:00", &["init", "-q"]);
    write(root, ALPHA, &atom("alpha", "red", ""));
    write(root, BETA, &atom("beta", "green", ""));
    let gamma_budget = "rea:\n  budget:\n    hours: 2\n";
    write(root, GAMMA, &atom("gamma", "red", gamma_budget));
    commit(root, "01", "born");
    write(root, ALPHA, &atom("alpha", "red", &reading("alpha", "1/2")));
    commit(root, "05", "alpha progresses");
    write(root, BETA, &atom("beta", "red", ""));
    commit(root, "08", "beta regresses");
    write(root, ALPHA, &atom("alpha", "red", &reading("alpha", "1/4")));
    commit(root, "10", "alpha rescoped");
    write(
        root,
        ALPHA,
        &atom("alpha", "green", &reading("alpha", "4/4")),
    );
    commit(root, "12", "alpha flips green");
    std::fs::remove_file(root.join(BETA)).unwrap();
    commit(root, "14", "beta retired");
    std::fs::create_dir_all(root.join("d/.epr-meta")).unwrap();
    std::fs::rename(root.join(GAMMA), root.join(GAMMA_MOVED)).unwrap();
    commit(root, "15", "gamma moves");
    dir
}

fn window(spec: &str) -> elohim_epr_rea::Window {
    stocks::parse_window(spec, Period::Day).unwrap()
}

fn red_mass(root: &Path, spec: &str) -> stocks::StockReport {
    let outcome = stocks::stocks(root, &window(spec), &[StockName::RedMass]).expect("stocks runs");
    outcome.stocks.into_iter().next().unwrap()
}

fn signed(event: &elohim_epr_rea::FlowEvent) -> f64 {
    let Magnitude::Count { value, unit } = &event.quantity else {
        panic!("red mass is a count");
    };
    assert_eq!(unit, RED_MASS_UNIT);
    if event.action == elohim_epr_rea::ReaVerb::Produce {
        *value
    } else {
        -*value
    }
}

#[test]
fn the_walk_names_every_movement_and_a_move_is_not_a_retirement() {
    let dir = history();
    let history = habits::walk_history(dir.path()).unwrap();
    assert!(history.unreadable.is_empty(), "{:?}", history.unreadable);
    let moves: Vec<(String, String, f64)> = history
        .events
        .iter()
        .filter(|e| {
            e.classified_as
                .first()
                .is_some_and(|t| t.starts_with("habit:"))
        })
        .filter(|e| e.fulfills.is_empty())
        .map(|e| {
            (
                e.classified_as[0].clone(),
                e.classified_as[1].clone(),
                signed(e),
            )
        })
        .collect();
    let expect = [
        ("habit:born", "alpha", 1.0),
        ("habit:born", "gamma", 1.0),
        ("habit:progress", "alpha", -0.25),
        ("habit:flip-red", "beta", 1.0),
        ("habit:rescope", "alpha", 0.125),
        ("habit:flip-green", "alpha", -0.875),
        ("habit:retired", "beta", -1.0),
    ];
    assert_eq!(moves.len(), expect.len(), "{moves:?}");
    for ((tag, id, delta), (want_tag, want_id, want_delta)) in moves.iter().zip(expect) {
        assert_eq!((tag.as_str(), id.as_str()), (want_tag, want_id));
        assert!(
            (delta - want_delta).abs() < 1e-9,
            "{tag} {id}: {delta} != {want_delta}"
        );
    }
    // Two discharges: the flip, and the retirement — each naming its own episode.
    let discharges: Vec<&String> = history
        .events
        .iter()
        .filter(|e| !e.fulfills.is_empty())
        .map(|e| &e.classified_as[0])
        .collect();
    assert_eq!(discharges, ["habit:flip-green", "habit:retired"]);
    // alpha, gamma (born red) and beta (flipped red) each opened one episode.
    assert_eq!(history.episodes.len(), 3);
    // gamma moved, it was not retired: it still stands, at its new path, with its episode open.
    let gamma = &history.current["gamma"];
    assert_eq!(gamma.path, GAMMA_MOVED);
    assert!(gamma.episode.is_some());
    assert!(!history.current.contains_key("beta"));
}

#[test]
fn a_window_reads_fills_and_drains_as_events_not_rounded_quantities() {
    let dir = history();
    // 2026-10-04 .. 2026-10-13: progress, flip-red, rescope, flip-green.
    let report = red_mass(dir.path(), "2026-10-04..2026-10-13");
    assert_eq!(report.observed_in_window, 4);
    let m = report.measures.as_ref().expect("a folded stock");
    let periods = 9.0;
    assert!(
        (m.inflow * periods - 1.125).abs() < 1e-9,
        "inflow {}",
        m.inflow
    );
    assert!(
        (m.outflow * periods - 1.125).abs() < 1e-9,
        "outflow {}",
        m.outflow
    );
    // Level at the window's end: beta 1 + gamma 1 (alpha flipped green inside the window).
    assert!((m.level - 2.0).abs() < 1e-9, "level {}", m.level);
    assert_eq!(report.verdict, EquilibriumVerdict::Draining);

    let habits = report.habits.as_ref().expect("the per-habit reading");
    assert_eq!(habits.red_mass, m.level);
    assert_eq!(habits.habits, 3);
    assert_eq!(habits.non_green, 2);
    let beta = habits.rows.iter().find(|r| r.id == "beta").unwrap();
    assert_eq!(beta.state.status, "red", "beta was not retired yet");
    let gamma = habits.rows.iter().find(|r| r.id == "gamma").unwrap();
    assert_eq!(gamma.path, GAMMA, "gamma had not moved yet");
    let alpha = habits.rows.iter().find(|r| r.id == "alpha").unwrap();
    assert_eq!(
        alpha.red_mass_at_start, 1.0,
        "its first reading lands inside the window"
    );
    assert!(alpha.red_mass_at_end.abs() < 1e-9);
    assert!((alpha.movement.drained - 1.125).abs() < 1e-9);
    assert!((alpha.movement.filled - 0.125).abs() < 1e-9);
    assert!(alpha.movement.by_kind.contains_key("habit:rescope"));

    // A window in which only the retirement happens drains, and a move fills nothing.
    let later = red_mass(dir.path(), "2026-10-13..2026-10-16");
    let m = later.measures.as_ref().unwrap();
    assert_eq!(later.observed_in_window, 1);
    assert!((m.outflow * 3.0 - 1.0).abs() < 1e-9);
    assert_eq!(m.inflow, 0.0);
    assert!((m.level - 1.0).abs() < 1e-9, "only gamma remains red");
    let habits = later.habits.as_ref().unwrap();
    assert_eq!((habits.habits, habits.non_green), (2, 1));
    assert_eq!(habits.red_mass, m.level);
    assert_eq!(
        habits.rows.iter().find(|r| r.id == "gamma").unwrap().path,
        GAMMA_MOVED
    );
}

#[test]
fn later_edits_do_not_rewrite_a_historical_reading() {
    let dir = history();
    let root = dir.path();
    let actor = NoteActor {
        as_ref: Some("agent:implementer@test-model".into()),
        session: None,
    };
    habits::spend(root, "gamma", 1.9, "gate", "historical-budget", &actor).unwrap();
    let past = red_mass(root, "2026-10-15..2026-10-16").habits.unwrap();
    let gamma = past.rows.iter().find(|r| r.id == "gamma").unwrap();
    assert_eq!(gamma.state.status, "red");
    assert_eq!(gamma.promised_budget.as_ref().unwrap().limit, 2.0);
    assert_eq!(gamma.spent.in_episode, 1.9);
    assert!(gamma.budget_pain.is_some());

    // Later edits change every part of the standing state, including its open episode.
    let future = "rea:\n  budget:\n    hours: 40\n  parts:\n    - part: wait\n      unit: s\n      baseline: 43\n      target: 12\n      reading: 10\n      at: 2026-10-18\n      evidence: future-gate\n      per_week: 40\n";
    write(root, GAMMA_MOVED, &atom("gamma", "green", future));
    write(
        root,
        "e/.epr-meta/later.habit.md",
        &atom("later", "green", future),
    );
    commit(root, "18", "gamma green and a new returning habit");
    write(root, GAMMA_MOVED, &atom("gamma", "red", future));
    commit(root, "19", "gamma opens a different budgeted episode");
    std::fs::create_dir_all(root.join("f/.epr-meta")).unwrap();
    std::fs::rename(
        root.join(GAMMA_MOVED),
        root.join("f/.epr-meta/gamma.habit.md"),
    )
    .unwrap();
    commit(root, "20", "gamma moves again");

    let reread = red_mass(root, "2026-10-15..2026-10-16").habits.unwrap();
    assert_eq!(
        serde_json::to_value(&past).unwrap(),
        serde_json::to_value(&reread).unwrap()
    );
    // Older windows retain then-live beta and alpha's pre-green state as well.
    let older = red_mass(root, "2026-10-04..2026-10-11");
    let reading = older.habits.unwrap();
    assert_eq!(reading.red_mass, older.measures.unwrap().level);
    assert_eq!((reading.habits, reading.non_green), (3, 3));
    assert_eq!(reading.hours_returned_per_week, 0.0);
    assert_eq!(
        reading.rows.iter().find(|r| r.id == "gamma").unwrap().path,
        GAMMA
    );

    // A current window must still read the new path, episode, budget and returns.
    let current = red_mass(root, "2026-10-18..2026-10-21").habits.unwrap();
    let gamma = current.rows.iter().find(|r| r.id == "gamma").unwrap();
    assert_eq!(gamma.path, "f/.epr-meta/gamma.habit.md");
    assert_eq!(gamma.promised_budget.as_ref().unwrap().limit, 40.0);
    assert_eq!(
        gamma.episode_opened.as_deref(),
        Some("2026-10-19T12:00:00Z")
    );
    assert_eq!(gamma.spent.in_episode, 0.0);
    assert!(current.hours_returned_per_week > 0.0);
    assert_eq!((current.habits, current.non_green), (3, 1));
}

#[test]
fn standing_changes_at_the_exclusive_end_are_not_in_the_window() {
    let dir = history();
    let mut before_retirement = window("2026-10-13..2026-10-15");
    before_retirement.end = "2026-10-14T12:00:00Z".into();
    let history = habits::walk_history(dir.path()).unwrap();
    let before = habits::reading(&history, &[], &before_retirement).unwrap();
    assert_eq!(
        (before.habits, before.non_green, before.red_mass),
        (3, 2, 2.0)
    );
    before_retirement.end = "2026-10-14T12:00:01Z".into();
    let after = habits::reading(&history, &[], &before_retirement).unwrap();
    assert_eq!((after.habits, after.non_green, after.red_mass), (2, 1, 1.0));
}

#[test]
fn a_window_in_which_nothing_moved_is_refused_not_drained() {
    let dir = history();
    let quiet = red_mass(dir.path(), "2026-10-16..2026-10-20");
    assert_eq!(quiet.observed_in_window, 0);
    assert!(matches!(quiet.verdict, EquilibriumVerdict::Refused { .. }));
}

#[test]
fn project_appends_episodes_and_discharges_to_the_commitments_ledger() {
    let dir = history();
    let root = dir.path();
    write(
        root,
        ".claude/epr-meta/recipes.yaml",
        "version: 1\nrecipes: []\n",
    );
    project::project(root, &root.join(".claude/epr-meta/recipes.yaml")).expect("project runs");
    let records = SidecarFlowStore::open(root).unwrap().records().unwrap();
    let episodes = records
        .iter()
        .filter(|(_, r)| matches!(r, FlowRecord::Commitment(c) if c.resource_spec.classified_as.first().map(String::as_str) == Some(habits::REGISTER_TAG)))
        .count();
    assert_eq!(episodes, 3);
    let discharges = records
        .iter()
        .filter(|(_, r)| matches!(r, FlowRecord::Event(e) if !e.fulfills.is_empty()))
        .count();
    assert_eq!(discharges, 2);
    // Idempotent: a second projection appends nothing.
    let before = records.len();
    project::project(root, &root.join(".claude/epr-meta/recipes.yaml")).expect("project runs");
    assert_eq!(
        SidecarFlowStore::open(root)
            .unwrap()
            .records()
            .unwrap()
            .len(),
        before
    );
}

#[test]
fn time_spent_is_scoped_to_the_habit_and_judged_against_its_episode_budget() {
    let dir = history();
    let root = dir.path();
    let actor = NoteActor {
        as_ref: Some("agent:implementer@test-model".into()),
        session: None,
    };
    let spent = habits::spend(root, "gamma", 1.9, "gate", "gate-run-7", &actor).unwrap();
    assert!(spent.appended);
    assert_eq!(spent.occurred_at, "2026-10-15T12:00:00Z");
    // Byte-identical spend is the same act.
    assert!(
        !habits::spend(root, "gamma", 1.9, "gate", "gate-run-7", &actor)
            .unwrap()
            .appended
    );
    assert!(habits::spend(root, "nobody", 1.0, "gate", "r", &actor).is_err());
    assert!(habits::spend(root, "gamma", 1.0, "telepathy", "r", &actor).is_err());

    let report = red_mass(root, "2026-10-15..2026-10-16");
    let gamma = report
        .habits
        .as_ref()
        .unwrap()
        .rows
        .iter()
        .find(|r| r.id == "gamma")
        .unwrap()
        .clone();
    assert!((gamma.spent.in_window - 1.9).abs() < 1e-9);
    assert!((gamma.spent.in_episode - 1.9).abs() < 1e-9);
    // 1.9 h against a 2 h promise with an 85% band edge (1.7 h) is an approach, not a breach.
    let pain = gamma.budget_pain.expect("the band edge is crossed");
    assert_eq!(pain.kind().as_str(), "algedonic-approach");
    // Spending names no promise: the commitments ledger sees no discharge from it.
    let records = SidecarFlowStore::open(root).unwrap().records().unwrap();
    assert!(records.iter().all(|(_, r)| match r {
        FlowRecord::Event(e) => e.fulfills.is_empty(),
        _ => true,
    }));
}
