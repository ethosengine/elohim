//! Governed-discovery station 3, task 3.1: the question bank as `Intent`s in scope of the
//! recipe. The live contract (`recall::CONTRACT_REL`) declares `question_bank`, pointing at
//! `.epr-meta/elohim/algorithms/recall-questions.json` — six fixed questions, each a `consume`
//! `Intent` whose `in_scope_of` is the contract's own `method_cid()`.
mod common;

use std::path::Path;

use elohim_epr_cli::flow::memory::recall::{self, Contract};
use serde_json::json;

/// The live bank loads, names at least six questions, and every one is `in_scope_of` the exact
/// contract that named it — the recipe/bank pairing this station's whole design rests on. Every
/// question's authoritative path also actually exists in this repository (a dangling `reached_when
/// .path` would be a question nobody could ever answer).
#[test]
fn the_question_bank_loads_as_intents_in_scope_of_the_recipe() {
    let root = common::repo_root();
    let contract = Contract::load(&root.join(recall::CONTRACT_REL)).expect("live contract loads");
    let bank = contract.question_bank().expect("live question bank loads");
    assert!(
        bank.len() >= 6,
        "expected >= 6 questions, got {}",
        bank.len()
    );

    let recipe_cid = contract.method_cid();
    for question in &bank {
        assert_eq!(
            question.intent.in_scope_of.to_string(),
            recipe_cid,
            "question `{}` is not in scope of the recipe that names it",
            question.id
        );
        let evidence_path = root.join(&question.reached_when.path);
        assert!(
            evidence_path.exists(),
            "question `{}` names a reached_when.path that does not exist: {}",
            question.id,
            evidence_path.display()
        );
        assert!(
            !question.reached_when.assertion.trim().is_empty(),
            "question `{}` has an empty assertion",
            question.id
        );
        assert!(
            !question.scope.trim().is_empty(),
            "question `{}` has an empty scope",
            question.id
        );
    }

    let ids: std::collections::BTreeSet<&str> = bank.iter().map(|q| q.id.as_str()).collect();
    for expected in [
        "q-remine",
        "q-corrections",
        "q-top-red",
        "q-hook-binary",
        "q-body-scan",
        "q-journey-folds",
    ] {
        assert!(ids.contains(expected), "missing question id `{expected}`");
    }
}

/// A contract with no `question_bank` key at all (every pre-v12 contract) is genuinely unwired,
/// not malformed — `question_bank()` returns an empty bank rather than refusing.
#[test]
fn an_absent_question_bank_is_an_empty_bank_not_a_refusal() {
    let mut value = common::contract_value(false);
    value
        .as_object_mut()
        .expect("contract is an object")
        .remove("question_bank");
    let contract = Contract::from_value(value).expect("contract without question_bank validates");
    let bank = contract.question_bank().expect("absent bank is Ok");
    assert!(bank.is_empty(), "expected an empty bank, got {bank:?}");
}

/// A DECLARED `question_bank` whose value is not a repository-relative path string is refused —
/// the contract asserted a bank exists, so this is a hand-edit error, never silently treated as
/// "no bank."
#[test]
fn a_question_bank_value_that_is_not_a_string_refuses() {
    let mut value = common::contract_value(false);
    value["question_bank"] = json!(42);
    let contract = Contract::from_value(value).expect("contract still validates");
    let error = contract
        .question_bank()
        .expect_err("a non-string question_bank must refuse");
    assert!(
        error.to_string().contains("malformed"),
        "unexpected refusal message: {error}"
    );
}

/// A `question_bank` file that exists but does not parse as JSON is refused with the same
/// "declared but malformed" phrasing `process_spec`/`bounds` already use for this class of
/// hand-edit error.
#[test]
fn a_question_bank_file_that_does_not_parse_refuses() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let mut contract_value = common::contract_value(false);
    contract_value["question_bank"] = json!(".epr-meta/elohim/algorithms/recall-questions.json");
    common::write(
        root,
        recall::CONTRACT_REL,
        &serde_json::to_string(&contract_value).expect("encode"),
    );
    common::write(
        root,
        ".epr-meta/elohim/algorithms/recall-questions.json",
        "{ not valid json",
    );
    let contract =
        Contract::load(&root.join(recall::CONTRACT_REL)).expect("fixture contract loads");
    let error = contract
        .question_bank()
        .expect_err("unparseable bank file must refuse");
    assert!(
        error.to_string().contains("malformed"),
        "unexpected refusal message: {error}"
    );
}

/// A `question_bank` file whose declared `recipe` does not match the contract's own `method_cid()`
/// is refused — a bank pinned to a stale or different contract must not be silently accepted as
/// this one's.
#[test]
fn a_question_bank_recipe_mismatch_refuses() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let mut contract_value = common::contract_value(false);
    contract_value["question_bank"] = json!(".epr-meta/elohim/algorithms/recall-questions.json");
    common::write(
        root,
        recall::CONTRACT_REL,
        &serde_json::to_string(&contract_value).expect("encode"),
    );
    let stale_bank = json!({
        "version": 1,
        "recipe": "bafkreistalestalestalestalestalestalestalestalestalestales",
        "questions": []
    });
    common::write(
        root,
        ".epr-meta/elohim/algorithms/recall-questions.json",
        &serde_json::to_string(&stale_bank).expect("encode"),
    );
    let contract =
        Contract::load(&root.join(recall::CONTRACT_REL)).expect("fixture contract loads");
    let error = contract
        .question_bank()
        .expect_err("a stale recipe cid must refuse");
    assert!(
        error.to_string().contains("malformed"),
        "unexpected refusal message: {error}"
    );
}

/// Sanity check on the seam this whole task turns on: the contract really does resolve its
/// `question_bank` pointer relative to its own file's directory (the same repository root every
/// other declared source root is read against), not some other base.
#[test]
fn question_bank_path_resolves_relative_to_the_repository_root() {
    let root = common::repo_root();
    let contract = Contract::load(&root.join(recall::CONTRACT_REL)).expect("live contract loads");
    let declared = contract
        .value
        .get("question_bank")
        .and_then(serde_json::Value::as_str)
        .expect("question_bank is declared as a string");
    assert!(
        Path::new(declared).is_relative(),
        "question_bank should be repository-relative, got {declared}"
    );
    assert!(
        root.join(declared).exists(),
        "declared question_bank path does not exist under the repository root: {declared}"
    );
}

// ── task 3.2: `sample` and `judge` — a journey as a FlowEvent, a second-seat Verdict ───────────────

/// `sample` composes `open`/`read`/`finish` in-process, reaches authority on the fixture's located
/// candidate, and folds `recall-metered-bytes@1`, `recall-screens-to-shape@1` and
/// `recall-unmetered-bytes@1` — each exactly once — through the same registry-validated writer
/// `flow note --measure` uses.
#[test]
fn sample_reaches_authority_on_the_fixture_and_folds_its_measures() {
    let dir = common::repo_with_bank();
    let v = common::ok_in_bank(
        dir.path(),
        "smp",
        &[
            "sample",
            "--question",
            "q-fixture",
            "--reader",
            "agent:reader@claude-sonnet-5",
        ],
    );
    assert_eq!(v["event"]["action"], "consume");
    assert_eq!(v["event"]["fulfills"].as_array().unwrap().len(), 1);
    assert_eq!(v["reached"], true);
    assert_eq!(v["folds"].as_array().unwrap().len(), 3);
    // Fix round 1, C1: located == declared on this fixture, so the view names the match rather
    // than a divergence.
    assert_eq!(v["location"], "located tooling/skill.md (matches declared)");
    // Fix round 1, Q2: the honesty floor's `outcome.receipts`/`first_screen.ranking` read real
    // data off the aggregate view, not an absent default.
    assert!(
        !v["outcome"]["receipts"].as_array().unwrap().is_empty(),
        "{v:#}"
    );
    assert!(!v["first_screen"].is_null(), "{v:#}");

    let folds = common::flows(dir.path());
    let metered = folds
        .iter()
        .filter(|r| r["measure"] == "recall-metered-bytes@1")
        .count();
    assert_eq!(metered, 1, "folds: {folds:?}");
    let screens = folds
        .iter()
        .find(|r| r["measure"] == "recall-screens-to-shape@1")
        .expect("recall-screens-to-shape@1 folded");
    // Fix round 1, Q3 (controller ruling): reached → fold 1.
    assert_eq!(screens["value"], 1.0);
    let unmetered = folds
        .iter()
        .find(|r| r["measure"] == "recall-unmetered-bytes@1")
        .expect("recall-unmetered-bytes@1 folded");
    assert_eq!(unmetered["value"], 0.0);
    assert_eq!(unmetered["env"]["question"], "q-fixture");
    // Fix round 1, F1 (controller ruling): every fold this journey writes names the journey's
    // own `FlowEvent` cid, so `rate-over-window` can group folds back into journeys instead of
    // counting each fold as its own population member.
    let event_cid = v["event"]["cid"].as_str().unwrap();
    assert_eq!(unmetered["env"]["journey"], event_cid);
    assert_eq!(screens["env"]["journey"], event_cid);
}

/// S1 (spec miss): `judge`'s argv carries no `--session` at all — the brief's own shape. It must
/// not refuse on a missing session; `judge` claims no session of its own.
#[test]
fn judge_needs_no_session_at_all() {
    let dir = common::repo_with_bank();
    let v = common::ok_in_bank(
        dir.path(),
        "smp-s1",
        &[
            "sample",
            "--question",
            "q-fixture",
            "--reader",
            "agent:reader@claude-sonnet-5",
        ],
    );
    let cid = v["event"]["cid"].as_str().unwrap().to_string();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(["flow", "memory", "recall", "judge"])
        .args(["--event", &cid])
        .args(["--as", "agent:seat@claude-opus-5"])
        .args(["--mistaken", "0"])
        .args(["--reason", "clean journey"])
        .args(["--root", &dir.path().to_string_lossy()])
        .args(["--json"])
        .output()
        .expect("epr runs");
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A reader named as the second seat is refused outright — a journey is never its own judge.
#[test]
fn a_reader_cannot_judge_its_own_journey() {
    let dir = common::repo_with_bank();
    let v = common::ok_in_bank(
        dir.path(),
        "smp2",
        &[
            "sample",
            "--question",
            "q-fixture",
            "--reader",
            "agent:reader@claude-sonnet-5",
        ],
    );
    let cid = v["event"]["cid"].as_str().unwrap();
    let r = common::run_in_bank(
        dir.path(),
        "smp2",
        &[
            "judge",
            "--event",
            cid,
            "--as",
            "agent:reader@claude-sonnet-5",
            "--mistaken",
            "0",
            "--reason",
            "self",
        ],
    );
    assert_eq!(r.code, 2, "{}{}", r.stdout, r.stderr);
    assert!(
        r.stdout
            .contains("refused: a reader never judges its own journey"),
        "{}",
        r.stdout
    );
}

/// Q1: a leading/trailing space on `--as` must not slip an untrimmed seat past the self-judge
/// comparison — `sample` validates `--reader` the same trimmed way.
#[test]
fn a_reader_cannot_judge_its_own_journey_even_with_untrimmed_as() {
    let dir = common::repo_with_bank();
    let v = common::ok_in_bank(
        dir.path(),
        "smp2b",
        &[
            "sample",
            "--question",
            "q-fixture",
            "--reader",
            "agent:reader@claude-sonnet-5",
        ],
    );
    let cid = v["event"]["cid"].as_str().unwrap();
    let r = common::run_in_bank(
        dir.path(),
        "smp2b",
        &[
            "judge",
            "--event",
            cid,
            "--as",
            " agent:reader@claude-sonnet-5 ",
            "--mistaken",
            "0",
            "--reason",
            "self",
        ],
    );
    assert_eq!(r.code, 2, "{}{}", r.stdout, r.stderr);
    assert!(
        r.stdout
            .contains("refused: a reader never judges its own journey"),
        "{}",
        r.stdout
    );
}

/// A genuine second seat's verdict folds `recall-mistaken-assertions@1` at the observed count and
/// renders a `refuse` decision when any assertion was mistaken.
#[test]
fn a_second_seat_verdict_folds_mistaken_assertions() {
    let dir = common::repo_with_bank();
    let v = common::ok_in_bank(
        dir.path(),
        "smp3",
        &[
            "sample",
            "--question",
            "q-fixture",
            "--reader",
            "agent:reader@claude-sonnet-5",
        ],
    );
    let cid = v["event"]["cid"].as_str().unwrap();
    let j = common::ok_in_bank(
        dir.path(),
        "smp3-seat",
        &[
            "judge",
            "--event",
            cid,
            "--as",
            "agent:seat@claude-opus-5",
            "--mistaken",
            "1",
            "--reason",
            "misread the stamp rule",
        ],
    );
    assert_eq!(j["verdict"]["witness"]["checks"][0]["observed"], 1);
    assert_eq!(j["verdict"]["decision"], "refuse");
    // Fix round 1, Q4: the lens is the SEAT's own reading (`agent:seat@claude-opus-5` → `standard`
    // in this fixture's `lens_table.stated`) — `smp3-seat` never claimed anything, so a
    // session-derived lens would read `none` here instead.
    assert!(
        j["lens"]["provenance"]["stated"][0]
            .as_str()
            .unwrap()
            .contains("claude-opus-5"),
        "{j:#}"
    );

    let folds = common::flows(dir.path());
    let mistaken = folds
        .iter()
        .find(|r| r["measure"] == "recall-mistaken-assertions@1")
        .expect("recall-mistaken-assertions@1 folded");
    assert_eq!(mistaken["value"], 1.0);
    assert_eq!(mistaken["env"]["reader"], "agent:reader@claude-sonnet-5");
    // Fix round 1, F1: the mistaken-assertions fold names the SAME journey `sample` folded, so
    // `rate-over-window` groups it with `sample`'s three folds as one journey rather than two.
    assert_eq!(mistaken["env"]["journey"], cid);
}

/// A question whose `reached_when.terms` never appear in the located excerpt writes `fulfills: []`
/// and `reached: false` — reaching authority is checked against the excerpt actually read, not
/// assumed from a located candidate alone.
#[test]
fn a_sample_whose_terms_are_absent_writes_no_fulfillment() {
    let dir = common::repo_with_bank();
    let v = common::ok_in_bank(
        dir.path(),
        "smp4",
        &[
            "sample",
            "--question",
            "q-fixture-miss",
            "--reader",
            "agent:reader@claude-sonnet-5",
        ],
    );
    assert_eq!(v["reached"], false);
    assert_eq!(v["event"]["fulfills"].as_array().unwrap().len(), 0);

    // Fix round 1, Q3 (controller ruling): not reached → the journey's rendered-screen count
    // (open, read, finish = 3) plus one — 4, not the flat 1 a reached journey folds.
    let folds = common::flows(dir.path());
    let screens = folds
        .iter()
        .find(|r| r["measure"] == "recall-screens-to-shape@1")
        .expect("recall-screens-to-shape@1 folded");
    assert_eq!(screens["value"], 4.0, "{screens:?}");
}

/// Fix round 2, R1 (controller ruling): a question whose entry LOCATES NOTHING (no first-screen
/// candidate at all — `q-fixture-nolocate`'s need is all sub-4-char tokens, so
/// `discovery::question_terms` drops every one and `open` renders the whole-scope ceremony door
/// instead) is a MEASURED MISS, never a refusal. `sample` records it: no `read` (no receipt, and
/// `finish` would refuse a focused journey with zero receipts — skipped, not forced), a `FlowEvent`
/// with empty `fulfills`, and the standing reader's usual three folds.
#[test]
fn sample_records_a_measured_miss_when_open_locates_no_candidate() {
    let dir = common::repo_with_bank();
    let v = common::ok_in_bank(
        dir.path(),
        "smp5",
        &[
            "sample",
            "--question",
            "q-fixture-nolocate",
            "--reader",
            "agent:reader@claude-sonnet-5",
        ],
    );
    assert_eq!(v["reached"], false);
    assert!(v["read"].is_null(), "{v:#}");
    assert_eq!(
        v["location"],
        "no candidate located — the entry rendered the ceremony door"
    );
    assert_eq!(v["event"]["action"], "consume");
    assert_eq!(v["event"]["fulfills"].as_array().unwrap().len(), 0);
    assert_eq!(v["folds"].as_array().unwrap().len(), 3);

    let folds = common::flows(dir.path());
    let screens = folds
        .iter()
        .find(|r| r["measure"] == "recall-screens-to-shape@1")
        .expect("recall-screens-to-shape@1 folded");
    assert_eq!(screens["value"], 4.0, "{screens:?}");
    assert_eq!(screens["env"]["question"], "q-fixture-nolocate");
    // Task 3.3's `env:journey=<FlowEvent cid>` slot must still be present on a miss journey's
    // folds — the window bound groups folds back into the journey they came from either way.
    assert_eq!(
        screens["env"]["journey"],
        v["event"]["cid"].as_str().unwrap()
    );

    let metered = folds
        .iter()
        .find(|r| r["measure"] == "recall-metered-bytes@1")
        .expect("recall-metered-bytes@1 folded");
    assert_eq!(
        metered["env"]["journey"],
        v["event"]["cid"].as_str().unwrap()
    );
    let unmetered = folds
        .iter()
        .find(|r| r["measure"] == "recall-unmetered-bytes@1")
        .expect("recall-unmetered-bytes@1 folded");
    assert_eq!(unmetered["value"], 0.0);
}

/// `judge` accepts a measured-miss event exactly like any other sampled journey.
#[test]
fn judge_accepts_a_measured_miss_event() {
    let dir = common::repo_with_bank();
    let v = common::ok_in_bank(
        dir.path(),
        "smp5-seat",
        &[
            "sample",
            "--question",
            "q-fixture-nolocate",
            "--reader",
            "agent:reader@claude-sonnet-5",
        ],
    );
    let cid = v["event"]["cid"].as_str().unwrap();
    let j = common::ok_in_bank(
        dir.path(),
        "smp5-seat-judge",
        &[
            "judge",
            "--event",
            cid,
            "--as",
            "agent:seat@claude-opus-5",
            "--mistaken",
            "0",
            "--reason",
            "the miss itself is honestly recorded",
        ],
    );
    assert_eq!(j["verdict"]["decision"], "permit");
    assert_eq!(j["verdict"]["subject"], cid);
}
