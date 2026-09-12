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
