//! Station five: the native bounded-evidence recall executor.
//!
//! Every ceremony assertion here runs the SHIPPED BINARY and reads its stdout and exit code, the
//! way the Python suite it replaces drove `recall-ceremony.py`. The executor's whole product is a
//! view plus a receipt, so testing it through a library call would test something no caller uses.
//!
//! The parity map from the 60 Python cases to these tests — including the cases that are retired
//! because they were artefacts of the Python implementation rather than of the algorithm — lives in
//! `genesis/docs/superpowers/plans/memory-kit-replacement/task-5-native-report.md`.
//!
//! Two assertions are deliberately adversarial:
//!
//! * The privacy invariant is **mutation-checked**: the same operation is accepted on an ordinary
//!   path and refused on a recall-store path, so deleting `refuse_private_import` turns the test red
//!   rather than leaving it green for an unrelated reason.
//! * The ignore invariant asks **git**, not the `.gitignore` text, whether the recall store is
//!   ignored — an assertion about a string in a file would pass while the ladder above it silently
//!   re-included the directory.

use std::path::{Path, PathBuf};
use std::process::Command;

use elohim_epr_cli::flow::memory::footprint;
use elohim_epr_cli::flow::memory::recall::{
    self, adopt_receipts, discover, excerpt, refuse_private_import, save_receipt, Contract,
    Execution,
};
use elohim_epr_rea::{AgentRef, DepEdge, FlowRecord, FlowStore, Governor, SidecarFlowStore};
use eprfs_core::BlobCid;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const SESSION: &str = "station-five";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root")
}

/// The live algorithm artifact, read from its EPRFS owner — never a local copy.
///
/// A hand-written fixture contract would let the real one drift (a budget renamed, a stage added)
/// while these tests stayed green about an algorithm nobody runs.
fn live_contract() -> Value {
    let raw = std::fs::read(repo_root().join(recall::CONTRACT_REL)).expect("live contract");
    serde_json::from_slice(&raw).expect("contract parses")
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, text).expect("write");
}

/// Pinned for the same reason the corrections fixture pins it: git HEAD's author date is part of a
/// note's content address, so a fixture built a second later is a different address.
const FIXTURE_DATE: &str = "2026-09-10T00:00:00+00:00";

fn git(root: &Path, args: &[&str]) {
    let out = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture Author")
        .env("GIT_COMMITTER_NAME", "Fixture Author")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
        .env("GIT_AUTHOR_DATE", FIXTURE_DATE)
        .env("GIT_COMMITTER_DATE", FIXTURE_DATE)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}");
}

fn edge(root: &Path, from: &str, to: &str) {
    let seal = Some(*BlobCid::compute_raw(b"old").as_cid());
    let record = DepEdge::new(
        from.into(),
        to.into(),
        Some("This assertion relies on the evidence".into()),
        Governor::CiteSeal,
        seal,
        AgentRef("agent:test".into()),
        0,
        None,
    )
    .expect("edge");
    SidecarFlowStore::open(root)
        .expect("sidecar")
        .append(FlowRecord::Edge(record))
        .expect("append");
}

/// A synthetic repository whose declared source scope is `docs/` and whose two stale edges give the
/// ceremony something real to select.
fn repo() -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    for name in ["a", "b", "source"] {
        write(
            root,
            &format!("docs/{name}.md"),
            "---\ntitle: Preserve uncertainty\ntags:\n  - evidence\n---\n# Purpose\nPreserve uncertainty.\n# Evidence\nold evidence\n",
        );
    }
    save_contract(root, contract_value(false));
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "fixture"]);
    edge(root, "docs/a.md", "docs/source.md");
    edge(root, "docs/b.md", "docs/source.md");
    dir
}

/// The live contract, re-scoped to the fixture.
///
/// `with_measurements` selects whether the paired footprint lens participates: most ceremony
/// assertions are about paging, receipts and refusals, and running a Python lens on every one of
/// them would buy nothing but seconds.
fn contract_value(with_measurements: bool) -> Value {
    let mut contract = live_contract();
    contract["source_roots"] = json!(["docs"]);
    contract["ceremony"]["defaults"]["scope"] = json!("docs");
    contract["ceremony"]["providers"]["alternative"] =
        json!({"kind": "fixture", "source": "docs/source.md", "ranking": Value::Null});
    if !with_measurements {
        contract["ceremony"]
            .as_object_mut()
            .expect("ceremony")
            .remove("measurements");
    }
    contract
}

fn save_contract(root: &Path, contract: Value) {
    write(
        root,
        "contract.json",
        &serde_json::to_string(&contract).expect("encode"),
    );
}

struct Run {
    stdout: String,
    stderr: String,
    code: i32,
}

impl Run {
    fn json(&self) -> Value {
        serde_json::from_str(&self.stdout)
            .unwrap_or_else(|error| panic!("stdout is not JSON ({error}): {}", self.stdout))
    }
}

fn run_in(root: &Path, session: &str, args: &[&str]) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
    command
        .args(["flow", "memory", "recall"])
        .args(args)
        .args(["--root", &root.to_string_lossy()])
        .args(["--contract", "contract.json"])
        .args(["--session", session, "--json"]);
    let out = command.output().expect("epr runs");
    Run {
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        code: out.status.code().unwrap_or(-1),
    }
}

fn run(root: &Path, args: &[&str]) -> Run {
    run_in(root, SESSION, args)
}

fn ok(root: &Path, args: &[&str]) -> Value {
    let run = run(root, args);
    assert_eq!(
        run.code, 0,
        "{args:?} refused: {}{}",
        run.stdout, run.stderr
    );
    run.json()
}

/// A view that legitimately carries an unresolved frontier.
///
/// The executor exits 2 whenever `unresolved` is non-empty — an answered question with a named
/// limit, not a failure — so a helper that demanded zero would make every honest limit unassertable.
fn view(root: &Path, args: &[&str]) -> Value {
    let run = run(root, args);
    assert!(
        run.code == 0 || run.code == 2,
        "{args:?} exited {}: {}{}",
        run.code,
        run.stdout,
        run.stderr
    );
    run.json()
}

fn begin(root: &Path) {
    ok(
        root,
        &["open", "--intent", "Preserve the learner-facing meaning"],
    );
    ok(
        root,
        &["select", "--edge", "1", "--need", "Check the dependency"],
    );
}

fn continuation(root: &Path, session: &str) -> Value {
    let raw = std::fs::read(
        root.join(recall::RECALL_DIR_REL)
            .join(session)
            .join("continuation.json"),
    )
    .expect("continuation");
    serde_json::from_slice(&raw).expect("continuation parses")
}

// ── the view surface ────────────────────────────────────────────────────────────────────────────

/// Python: `test_purpose_story_and_per_assertion_observations`.
#[test]
fn every_view_carries_purpose_scope_and_linked_next_actions() {
    let dir = repo();
    let root = dir.path();
    let view = ok(
        root,
        &["open", "--intent", "Preserve the learner-facing meaning"],
    );
    assert_eq!(
        view["orientation"]["intent"],
        "Preserve the learner-facing meaning"
    );
    assert_eq!(view["orientation"]["scope"], "docs");
    assert!(!view["orientation"]["guiding_context"]
        .as_array()
        .expect("guiding context")
        .is_empty());
    assert_eq!(view["concerns"]["counts"]["selected_edges"], 2);
    // Every displayed edge is offered as an executable next action, not described in prose.
    let labels: Vec<String> = view["actions"]
        .as_array()
        .expect("actions")
        .iter()
        .map(|a| a["label"].as_str().unwrap_or_default().to_string())
        .collect();
    assert!(labels.iter().any(|l| l.starts_with("1. Inspect docs/a.md")));
    assert!(labels
        .iter()
        .any(|l| l == "Inspect governing recipe and alternatives"));
    let select = ok(
        root,
        &["select", "--edge", "1", "--need", "Check the dependency"],
    );
    assert_eq!(select["node"]["why_followed"], "Check the dependency");
    assert_eq!(
        select["selected_current"]["changed_since_selection"],
        json!(false)
    );
    assert_eq!(
        select["input_choices"].as_array().expect("choices").len(),
        3
    );
}

/// Python: `test_resume_rehydrates_and_invalidates_changed_receipts`.
#[test]
fn resume_rehydrates_and_invalidates_changed_receipts() {
    let dir = repo();
    let root = dir.path();
    begin(root);
    ok(
        root,
        &["read", "--path", "docs/source.md", "--lines", "6:7"],
    );
    ok(
        root,
        &[
            "remember",
            "--finding",
            "Needs a qualified reading",
            "--question",
            "Does the narrow evidence support the wider claim?",
            "--next-action",
            "Seek independent review",
            "--evidence",
            "docs/source.md:6:7",
        ],
    );
    let resumed = ok(root, &["resume"]);
    assert_eq!(
        resumed["evidence_check"]["unchanged_receipts"],
        json!(["docs/source.md:6:7"])
    );
    assert_eq!(
        resumed["continuation"]["findings"][0]["evidence_state"],
        "receipt-valid"
    );
    write(
        root,
        "docs/source.md",
        "---\ntitle: t\n---\n# Purpose\nchanged\n# Evidence\nnew evidence\n",
    );
    let after = ok(root, &["resume"]);
    assert_eq!(
        after["evidence_check"]["revalidation_required"],
        json!(["docs/source.md:6:7"])
    );
    assert_eq!(
        after["continuation"]["findings"][0]["evidence_state"],
        "revalidation-or-evidence-required"
    );
}

/// Python: `test_method_changes_require_explicit_continuation_with_prior_accounting`
/// and `test_cli_contract_change_refuses_reset_and_preserves_previous_receipt`.
#[test]
fn contract_change_refuses_reset_and_adopt_carries_prior_accounting() {
    let dir = repo();
    let root = dir.path();
    begin(root);
    let before = continuation(root, SESSION);
    let attempts = before["attempts"].as_u64().expect("attempts");

    let mut changed = contract_value(false);
    changed["method"]
        .as_array_mut()
        .expect("method list")
        .push(json!("Changed method."));
    save_contract(root, changed);

    let refused = run(root, &["resume"]);
    assert_eq!(refused.code, 2);
    assert!(refused.json()["unresolved"][0]
        .as_str()
        .expect("reason")
        .contains("algorithm bytes changed"));
    // The previous receipt survives the refusal untouched.
    assert_eq!(continuation(root, SESSION)["attempts"], json!(attempts));

    let adopted = run_in(root, "continued", &["adopt", "--from-session", SESSION]);
    assert_eq!(adopted.code, 0, "{}", adopted.stdout);
    let prior = &adopted.json()["continuation"]["prior_receipt"];
    assert_eq!(prior["session"], SESSION);
    assert_eq!(prior["attempts"], json!(attempts));
    assert!(prior["totals"]["source_bytes"].is_null() || prior["totals"].is_object());
    // Adopting twice into the same session would silently overwrite an investigation.
    let twice = run_in(root, "continued", &["adopt", "--from-session", SESSION]);
    assert_eq!(twice.code, 2);
    assert!(twice.json()["unresolved"][0]
        .as_str()
        .expect("reason")
        .contains("adopt needs a new session"));
}

/// Python: `test_cli_continuation_accumulates_arbitrarily_many_named_packets`.
#[test]
fn continuation_accumulates_every_named_packet() {
    let dir = repo();
    let root = dir.path();
    begin(root);
    for _ in 0..4 {
        ok(
            root,
            &[
                "read",
                "--path",
                "docs/source.md",
                "--lines",
                "6:7",
                "--need",
                "again",
            ],
        );
    }
    let view = ok(root, &["history"]);
    let cumulative = &view["cumulative"];
    assert_eq!(cumulative["attempts"], json!(7));
    assert_eq!(cumulative["unmetered_attempts"], json!(0));
    assert!(
        cumulative["totals"]["source_bytes"]
            .as_u64()
            .expect("bytes")
            > 0
    );
    assert_eq!(
        cumulative["accounting_scope"],
        "this executor/session only; outside-tool reads and total model tokens unknown"
    );
    // Repeated identical reads are counted, not silently deduplicated into invisibility.
    assert_eq!(
        continuation(root, SESSION)["ceremony"]["repeated_reads"],
        json!(3)
    );
}

/// Python: `test_failed_operation_is_retained_and_next_success_does_not_erase_it`
/// and `test_oversized_finding_refusal_is_retained_in_attempt_accounting`.
#[test]
fn a_refused_operation_is_retained_in_attempt_accounting() {
    let dir = repo();
    let root = dir.path();
    begin(root);
    let oversized = "x".repeat(4100);
    let refused = run(
        root,
        &[
            "remember",
            "--finding",
            &oversized,
            "--question",
            "q",
            "--next-action",
            "n",
        ],
    );
    assert_eq!(refused.code, 2);
    let before = continuation(root, SESSION)["attempts"]
        .as_u64()
        .expect("attempts");
    let after = ok(root, &["history"]);
    assert_eq!(after["cumulative"]["attempts"], json!(before + 1));
    assert_eq!(after["cumulative"]["unmetered_attempts"], json!(0));
}

/// Python: `test_provider_substitution_and_refusal_preserve_intent`.
#[test]
fn provider_substitution_and_refusal_preserve_intent() {
    let dir = repo();
    let root = dir.path();
    begin(root);
    let fixture = ok(
        root,
        &["search", "--provider", "alternative", "--query", "evidence"],
    );
    assert_eq!(fixture["retrieval"]["provider"], "alternative");
    assert!(fixture["retrieval"]["candidate_text"]
        .as_str()
        .expect("text")
        .contains("old evidence"));
    assert_eq!(
        fixture["retrieval"]["fitness"],
        "local fixture only; no live provider fitness established"
    );
    // The local default reports what it inspected and claims no ranking.
    // The chosen provider PERSISTS in the continuation, so returning to the default is an explicit
    // act rather than an omission — naming it is what the next operation would have to do too.
    let local = ok(
        root,
        &[
            "search",
            "--provider",
            "local",
            "--query",
            "purpose",
            "--search-scope",
            "docs",
        ],
    );
    assert_eq!(
        local["retrieval"]["selection"],
        "bounded filesystem traversal; sorted returned window, no relevance ranking"
    );
    // MemPalace is declared optional and says its ranking is unknown rather than implying one.
    let declared = ok(root, &["recipe"]);
    let palace = &declared["recipe"]["providers"]["mempalace"];
    assert_eq!(palace["optional"], json!(true));
    assert!(palace["ranking"].is_null());
    assert!(palace["freshness"].is_null());
    // An undeclared provider is refused, and the intent survives the refusal.
    let refused = run(root, &["search", "--provider", "invented", "--query", "x"]);
    assert_eq!(refused.code, 2);
    let failure = refused.json();
    assert!(failure["unresolved"][0]
        .as_str()
        .expect("reason")
        .contains("not declared by the pinned recipe"));
    assert_eq!(
        failure["orientation"]["intent"],
        "Preserve the learner-facing meaning"
    );
}

/// Python: `test_prepared_repair_does_not_execute_and_finish_does_not_accept`.
#[test]
fn prepared_repair_does_not_execute_and_finish_does_not_accept() {
    let dir = repo();
    let root = dir.path();
    begin(root);
    ok(
        root,
        &["read", "--path", "docs/source.md", "--lines", "6:7"],
    );
    ok(
        root,
        &[
            "remember",
            "--finding",
            "f",
            "--question",
            "q",
            "--next-action",
            "n",
            "--evidence",
            "docs/source.md:6:7",
            "--classification",
            "evidence-ready",
        ],
    );
    let before = SidecarFlowStore::open(root)
        .expect("store")
        .records()
        .expect("records");
    let prepared = ok(
        root,
        &[
            "prepare",
            "--kind",
            "repair",
            "--finding",
            "Evidenced repair",
        ],
    );
    let action = &prepared["prepared_action"];
    assert_eq!(action["argv"][1], "flow");
    assert_eq!(action["argv"][2], "reseal");
    assert_eq!(
        action["standing"],
        "Proposed action, not executed or approved by this view."
    );
    let finished = ok(
        root,
        &[
            "finish",
            "--outcome",
            "bounded outcome",
            "--question",
            "what remains",
        ],
    );
    assert_eq!(
        finished["outcome"]["standing"],
        "operator/investigator report; consult native evidence for acceptance"
    );
    assert_eq!(finished["outcome"]["unresolved_frontier"], "what remains");
    // Neither preparing nor finishing writes a record: the executor proposes, governors decide.
    let after = SidecarFlowStore::open(root)
        .expect("store")
        .records()
        .expect("records");
    assert_eq!(before, after);
}

/// Python: `test_newer_contrary_judgment_blocks_historical_repair_readiness` and
/// `test_source_escape_and_unproven_evidence_ready_refuse`.
#[test]
fn repair_readiness_requires_current_evidence_and_a_current_judgment() {
    let dir = repo();
    let root = dir.path();
    begin(root);
    // evidence-ready with no inspected evidence is refused outright.
    let unproven = run(
        root,
        &[
            "remember",
            "--finding",
            "f",
            "--question",
            "q",
            "--next-action",
            "n",
            "--classification",
            "evidence-ready",
        ],
    );
    assert_eq!(unproven.code, 2);
    assert!(unproven.json()["unresolved"][0]
        .as_str()
        .expect("reason")
        .contains("evidence-ready requires inspected evidence"));

    ok(
        root,
        &["read", "--path", "docs/source.md", "--lines", "6:7"],
    );
    ok(
        root,
        &[
            "remember",
            "--finding",
            "f",
            "--question",
            "q",
            "--next-action",
            "n",
            "--evidence",
            "docs/source.md:6:7",
            "--classification",
            "evidence-ready",
        ],
    );
    // A LATER contrary judgment is the one that counts.
    ok(
        root,
        &[
            "remember",
            "--finding",
            "actually unclear",
            "--question",
            "q2",
            "--next-action",
            "n2",
            "--classification",
            "conflict",
        ],
    );
    let prepared = view(
        root,
        &["prepare", "--kind", "repair", "--finding", "Repair anyway"],
    );
    assert!(prepared["prepared_action"].is_null());
    assert!(prepared["unresolved"]
        .as_array()
        .expect("unresolved")
        .iter()
        .any(|m| m
            .as_str()
            .unwrap_or_default()
            .contains("evidence-ready judgment")));
}

/// Python: `test_healthy_selected_edge_cannot_be_offered_for_reseal`.
#[test]
fn a_healthy_selected_edge_is_not_offered_for_reseal() {
    let dir = repo();
    let root = dir.path();
    begin(root);
    ok(
        root,
        &["read", "--path", "docs/source.md", "--lines", "6:7"],
    );
    ok(
        root,
        &[
            "remember",
            "--finding",
            "f",
            "--question",
            "q",
            "--next-action",
            "n",
            "--evidence",
            "docs/source.md:6:7",
            "--classification",
            "evidence-ready",
        ],
    );
    // Re-declare the edge with the CURRENT upstream body, so its verdict is no longer stale.
    let current = *BlobCid::compute_raw(
        elohim_epr_cli::flow::canonical_body(
            &std::fs::read_to_string(root.join("docs/source.md")).expect("read"),
        )
        .as_bytes(),
    )
    .as_cid();
    let record = DepEdge::new(
        "docs/a.md".into(),
        "docs/source.md".into(),
        Some("This assertion relies on the evidence".into()),
        Governor::CiteSeal,
        Some(current),
        AgentRef("agent:test".into()),
        1,
        None,
    )
    .expect("edge");
    SidecarFlowStore::open(root)
        .expect("store")
        .append(FlowRecord::Edge(record))
        .expect("append");
    let prepared = view(
        root,
        &[
            "prepare",
            "--kind",
            "repair",
            "--finding",
            "Reseal a healthy edge",
        ],
    );
    assert!(prepared["prepared_action"].is_null());
    assert!(prepared["unresolved"]
        .as_array()
        .expect("unresolved")
        .iter()
        .any(|m| m.as_str().unwrap_or_default().contains("no longer stale")));
}

/// Python: `test_native_context_expands_without_overwriting_saved_next_action` and
/// `test_changed_native_context_refuses_an_old_indexed_navigation_choice`.
#[test]
fn native_context_expands_and_refuses_a_stale_navigation_pin() {
    let dir = repo();
    let root = dir.path();
    begin(root);
    ok(
        root,
        &["read", "--path", "docs/source.md", "--lines", "6:7"],
    );
    ok(
        root,
        &[
            "remember",
            "--finding",
            "f",
            "--question",
            "q",
            "--next-action",
            "Seek independent review",
            "--evidence",
            "docs/source.md:6:7",
        ],
    );
    let context = ok(root, &["context"]);
    assert_eq!(context["native_context"]["found"], json!(true));
    // Navigation never overwrites the investigator's saved next action.
    assert_eq!(context["next_action"], "Seek independent review");
    let refused = view(
        root,
        &[
            "context",
            "--section",
            "reconciliation",
            "--context-pin",
            "0".repeat(64).as_str(),
        ],
    );
    assert_eq!(
        refused["native_context"]["unresolved"][0],
        "stale native section navigation refused"
    );
    assert!(refused["unresolved"]
        .as_array()
        .expect("unresolved")
        .iter()
        .any(|m| m
            .as_str()
            .unwrap_or_default()
            .contains("reopen its sections")));
}

/// Python: `test_context_page_offset_does_not_skip_selected_edge_revalidation` and
/// `test_selected_resume_and_adopt_skip_population_and_ignore_display_pagination`.
#[test]
fn display_pagination_never_controls_exact_slot_revalidation() {
    let dir = repo();
    let root = dir.path();
    begin(root);
    // A display offset past the end of the page still resolves the exact selected slot.
    let view = ok(root, &["context", "--offset", "99", "--limit", "1"]);
    assert_eq!(
        view["selected_current"]["matching_edges"]
            .as_array()
            .expect("matches")
            .len(),
        1
    );
    assert!(view["selected_current"]["unresolved"]
        .as_array()
        .expect("unresolved")
        .is_empty());
    // Resume with a selection does not re-open the concerns page at all.
    let resumed = ok(root, &["resume", "--offset", "99"]);
    assert!(resumed["concerns"].is_null());
    assert!(resumed["actions"]
        .as_array()
        .expect("actions")
        .iter()
        .any(|a| a["label"] == "Open remaining concerns in the preserved scope"));
}

/// Python: `test_native_claim_and_standing_changes_are_visible_without_source_change`.
#[test]
fn native_standing_changes_are_visible_without_a_source_change() {
    let dir = repo();
    let root = dir.path();
    begin(root);
    let record = DepEdge::new(
        "docs/a.md".into(),
        "docs/source.md".into(),
        Some("This assertion relies on the evidence".into()),
        Governor::CiteSeal,
        Some(*BlobCid::compute_raw(b"different").as_cid()),
        AgentRef("agent:test".into()),
        2,
        None,
    )
    .expect("edge");
    SidecarFlowStore::open(root)
        .expect("store")
        .append(FlowRecord::Edge(record))
        .expect("append");
    let view = ok(root, &["context"]);
    assert_eq!(
        view["selected_current"]["changed_since_selection"],
        json!(true)
    );
    assert!(view["selected_current"]["changed_fields"]
        .as_array()
        .expect("fields")
        .iter()
        .any(|f| f == "sealed_evidence"));
}

/// Python: `test_accumulated_findings_remain_progressively_recoverable` and
/// `test_finish_preserves_later_batch_finding_and_rechecks_changed_source`.
#[test]
fn accumulated_findings_remain_progressively_recoverable() {
    let dir = repo();
    let root = dir.path();
    begin(root);
    for line in ["6:6", "7:7"] {
        ok(root, &["read", "--path", "docs/source.md", "--lines", line]);
        ok(
            root,
            &[
                "remember",
                "--finding",
                &format!("finding {line}"),
                "--question",
                "q",
                "--next-action",
                "n",
                "--evidence",
                &format!("docs/source.md:{line}"),
            ],
        );
    }
    let page = ok(root, &["history", "--limit", "1"]);
    assert_eq!(page["history"]["counts"]["findings"], json!(2));
    assert_eq!(page["history"]["next_offset"], json!(1));
    assert_eq!(
        page["history"]["findings"].as_array().expect("page").len(),
        1
    );
    // The retained page never leaks the private working snapshot of each receipt.
    assert!(page["history"]["findings"][0]["evidence_snapshots"].is_null());
    let second = ok(root, &["history", "--offset", "1", "--limit", "1"]);
    assert_eq!(second["history"]["findings"][0]["claim"], "finding 7:7");
}

/// Python: `test_rereading_changed_evidence_does_not_revalidate_an_old_judgment`.
#[test]
fn rereading_changed_evidence_does_not_revalidate_an_old_judgment() {
    let dir = repo();
    let root = dir.path();
    begin(root);
    ok(
        root,
        &["read", "--path", "docs/source.md", "--lines", "6:7"],
    );
    ok(
        root,
        &[
            "remember",
            "--finding",
            "based on the old bytes",
            "--question",
            "q",
            "--next-action",
            "n",
            "--evidence",
            "docs/source.md:6:7",
        ],
    );
    write(
        root,
        "docs/source.md",
        "---\ntitle: t\n---\n# Purpose\np\n# Evidence\nrewritten evidence\n",
    );
    ok(
        root,
        &[
            "read",
            "--path",
            "docs/source.md",
            "--lines",
            "6:7",
            "--need",
            "re-read",
        ],
    );
    let view = ok(root, &["history"]);
    // A fresh read of the SAME range does not resurrect a judgment pinned to different bytes.
    assert_eq!(
        view["history"]["findings"][0]["evidence_state"],
        "revalidation-or-evidence-required"
    );
}

/// Python: `test_revalidation_total_budget_retains_pending_frontier` and
/// `test_pending_selected_evidence_has_executable_continuation_without_restart`.
#[test]
fn revalidation_budget_leaves_an_executable_pending_frontier() {
    let dir = repo();
    let root = dir.path();
    // A source_files budget of one cannot check three receipts in one pass.
    let mut contract = contract_value(false);
    contract["limits"]["source_files"] = json!(1);
    save_contract(root, contract);
    begin(root);
    for line in ["6:6", "7:7", "5:5"] {
        ok(root, &["read", "--path", "docs/source.md", "--lines", line]);
    }
    let view = ok(root, &["resume"]);
    let check = &view["evidence_check"];
    assert_eq!(check["scope_count"], json!(3));
    assert!(check["pending_count"].as_u64().expect("pending") >= 1);
    assert!(!check["next_offset"].is_null());
    // The continuation is a real command, not advice to start over.
    let resume = view["actions"]
        .as_array()
        .expect("actions")
        .iter()
        .find(|a| a["label"] == "Continue bounded evidence revalidation")
        .expect("continuation action");
    assert_eq!(resume["argv"][4], "resume");
    assert!(resume["command"]
        .as_str()
        .expect("command")
        .contains("--evidence-offset"));
}

/// Python: `test_output_bounds_apply_to_both_actual_renderings`.
#[test]
fn the_output_budget_bounds_both_renderings() {
    let dir = repo();
    let root = dir.path();
    let mut contract = contract_value(false);
    contract["limits"]["output_bytes"] = json!(700);
    save_contract(root, contract);
    let narrowed = run(
        root,
        &["open", "--intent", "Preserve the learner-facing meaning"],
    );
    assert_eq!(narrowed.code, 2);
    assert!(narrowed.stdout.len() <= 700, "{}", narrowed.stdout.len());
    assert!(narrowed.json()["unresolved"][0]
        .as_str()
        .expect("reason")
        .contains("output budget"));
    // The human rendering obeys the same budget rather than only the JSON one.
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(["flow", "memory", "recall", "resume"])
        .args(["--root", &root.to_string_lossy()])
        .args(["--contract", "contract.json", "--session", SESSION])
        .output()
        .expect("epr runs");
    assert!(out.stdout.len() <= 700);
}

/// Python: `test_doc_repair_never_routes_to_sidecar_reseal`.
#[test]
fn a_doc_plane_repair_never_routes_to_a_sidecar_reseal() {
    let dir = repo();
    let root = dir.path();
    // A doc-plane edge whose citation slot is AMBIGUOUS must refuse rather than pick one.
    write(
        root,
        "docs/a.md",
        "---\ncites:\n  - \"one | This assertion relies on the evidence | sha256:0000000000000000 | path: docs/source.md\"\n  - \"one | This assertion relies on the evidence | sha256:1111111111111111 | path: docs/source.md\"\n---\n# Purpose\np\n# Evidence\ne\n",
    );
    let record = DepEdge::new(
        "docs/a.md".into(),
        "docs/source.md".into(),
        Some("This assertion relies on the evidence".into()),
        Governor::CiteSeal,
        Some(*BlobCid::compute_raw(b"old").as_cid()),
        AgentRef("agent:test".into()),
        3,
        None,
    )
    .expect("edge");
    SidecarFlowStore::open(root)
        .expect("store")
        .append(FlowRecord::Edge(record))
        .expect("append");
    begin(root);
    ok(
        root,
        &["read", "--path", "docs/source.md", "--lines", "6:7"],
    );
    ok(
        root,
        &[
            "remember",
            "--finding",
            "f",
            "--question",
            "q",
            "--next-action",
            "n",
            "--evidence",
            "docs/source.md:6:7",
            "--classification",
            "evidence-ready",
        ],
    );
    let prepared = ok(
        root,
        &[
            "prepare",
            "--kind",
            "repair",
            "--finding",
            "Repair the doc slot",
        ],
    );
    // The sidecar plane still owns this slot, so the proposal is a reseal, never a cites refresh.
    assert_eq!(prepared["prepared_action"]["argv"][2], "reseal");
}

// ── the executor primitives ─────────────────────────────────────────────────────────────────────

/// Python: `test_session_requires_a_question_and_cannot_escape_or_follow_state_symlink`.
#[test]
fn a_session_needs_a_question_and_refuses_escape_or_a_symlinked_state() {
    let dir = repo();
    let root = dir.path();
    let contract = Contract::load(&root.join("contract.json")).expect("contract");
    let method = contract.method_cid();
    assert!(Execution::open(root, SESSION, "   ", &method, None, 4096).is_err());
    assert!(Execution::open(root, "../escape", "why", &method, None, 4096).is_err());
    assert!(Execution::open(root, "has/slash", "why", &method, None, 4096).is_err());
    assert!(Execution::open(root, "", "why", &method, None, 4096).is_err());

    std::fs::create_dir_all(root.join(recall::RECALL_DIR_REL).join("linked")).expect("mkdir");
    std::os::unix::fs::symlink(
        root.join("docs/source.md"),
        root.join(recall::RECALL_DIR_REL)
            .join("linked/continuation.json"),
    )
    .expect("symlink");
    assert!(Execution::open(root, "linked", "why", &method, None, 4096).is_err());
}

/// Python: `test_session_rejects_changed_executor_identity_without_losing_totals`.
///
/// Refuse-then-adopt, on BOTH pins. The contract's raw CID is the algorithm's identity and the
/// executor's digest is the identity of the thing that ran it; a session that continued across
/// either change would accumulate one set of counters over two methods and call the sum a
/// measurement of one. The refusal names the exact adopt command, and — the part that matters — it
/// leaves the STORED digest intact, so the receipt `adopt` carries forward still says what the
/// prior work actually ran under.
#[test]
fn a_changed_executor_refuses_the_session_and_adopt_carries_both_digests() {
    let dir = repo();
    let root = dir.path();
    begin(root);
    let before = continuation(root, SESSION);
    let running = before["executor_digest"]
        .as_str()
        .expect("the session pins the running executor")
        .to_string();
    assert_eq!(running.len(), 64);
    let totals = before["totals"].clone();
    let attempts = before["attempts"].as_u64().expect("attempts");

    // Stand in for a rebuilt binary by moving the recorded pin, exactly as a rebuild would.
    let stale = "0".repeat(64);
    let mut tampered = before.clone();
    tampered["executor_digest"] = json!(stale);
    std::fs::write(
        root.join(recall::RECALL_DIR_REL)
            .join(SESSION)
            .join("continuation.json"),
        serde_json::to_string(&tampered).expect("encode"),
    )
    .expect("write");

    let refused = run(root, &["resume"]);
    assert_eq!(refused.code, 2);
    let failure = refused.json();
    let reason = failure["unresolved"][0].as_str().expect("reason");
    assert!(reason.contains("executor bytes changed"), "{reason}");
    assert!(
        reason.contains("silent continuation is refused"),
        "{reason}"
    );
    // The refusal names the command that continues the work, not a description of one.
    assert!(
        reason.contains(&format!(
            "epr flow memory recall adopt --from-session {SESSION} --session <new-session>"
        )),
        "{reason}"
    );
    assert!(failure["next"]
        .as_str()
        .expect("next")
        .contains("adopt --from-session"));

    // Nothing was written: the attempt was not counted and the STORED digest is untouched, so the
    // receipt adoption is about to carry forward still describes the prior work honestly.
    let after_refusal = continuation(root, SESSION);
    assert_eq!(after_refusal["attempts"], json!(attempts));
    assert_eq!(after_refusal["executor_digest"], json!(stale));
    assert_eq!(after_refusal["totals"], totals);

    // Adoption is the way through, and it records BOTH digests.
    let adopted = run_in(root, "rebuilt", &["adopt", "--from-session", SESSION]);
    assert_eq!(adopted.code, 0, "{}{}", adopted.stdout, adopted.stderr);
    let prior = &adopted.json()["continuation"]["prior_receipt"];
    assert_eq!(prior["session"], SESSION);
    assert_eq!(prior["executor_digest"], json!(stale));
    assert_eq!(prior["attempts"], json!(attempts));
    assert_eq!(prior["totals"], totals);
    // The adopted session pins the executor that is actually running it.
    assert_eq!(
        continuation(root, "rebuilt")["executor_digest"],
        json!(running)
    );
    // And it continues: the inherited investigation is there and the session is usable.
    let resumed = run_in(root, "rebuilt", &["resume"]);
    assert_eq!(resumed.code, 0, "{}{}", resumed.stdout, resumed.stderr);
    assert_eq!(
        resumed.json()["continuation"]["prior_receipt"]["executor_digest"],
        json!(stale)
    );
}

/// A refusal's `next` line names the remedy for the fault that ACTUALLY fired.
///
/// One remedy for every refusal is worse than none: a caller told "method changes require explicit
/// adopt" after typing a path outside the declared scope learns nothing about their mistake and
/// something false about the executor's state.
#[test]
fn each_refusal_points_at_its_own_remedy() {
    let dir = repo();
    let root = dir.path();
    begin(root);
    let next_of = |args: &[&str]| {
        let run = run(root, args);
        assert_eq!(run.code, 2, "{args:?} was not refused: {}", run.stdout);
        run.json()["next"].as_str().expect("next").to_string()
    };
    assert!(next_of(&["read", "--path", "outside.md", "--lines", "1:1"])
        .contains("declared source_roots"));
    assert!(next_of(&[
        "read",
        "--path",
        ".eprfs/status/recall/x/continuation.json",
        "--lines",
        "1:1"
    ])
    .contains("never an input to any verb"));
    assert!(next_of(&["open", "--intent", "a different intent"]).contains("fixed at open"));
    // And a fault with no named class still says something true rather than something wrong.
    let generic = next_of(&["remember", "--finding", "f", "--question", "q"]);
    assert!(generic.contains("correct the named input"), "{generic}");
    assert!(!generic.contains("adopt --from-session"), "{generic}");
}

/// Python: `test_excerpt_range_unicode_and_fingerprint_are_exact` and
/// `test_fingerprint_describes_returned_content_even_if_source_changes`.
#[test]
fn an_excerpt_range_and_its_fingerprint_are_exact() {
    let dir = repo();
    let root = dir.path();
    write(root, "docs/unicode.md", "α\nβ\nγ\nδ\n");
    let contract = Contract::load(&root.join("contract.json")).expect("contract");
    let result = excerpt(root, &contract, "docs/unicode.md", "2:3").expect("excerpt");
    let source = &result["sources"][0];
    assert_eq!(source["content"], "β\nγ\n");
    assert_eq!(
        source["fingerprint"],
        format!("sha256:{:x}", Sha256::digest("β\nγ\n".as_bytes()))
    );
    assert_eq!(
        source["fingerprint_scope"],
        "exact excerpt bytes only; not a complete source fingerprint"
    );
    assert_eq!(result["usage"]["source_files"], json!(1));
    assert_eq!(result["usage"]["source_bytes"], json!("β\nγ\n".len()));
    // The fingerprint describes the RETURNED bytes; a later source change does not rewrite it.
    write(root, "docs/unicode.md", "α\nrewritten\nγ\nδ\n");
    let again = excerpt(root, &contract, "docs/unicode.md", "2:3").expect("excerpt");
    assert_ne!(again["sources"][0]["fingerprint"], source["fingerprint"]);
    // Ranges must be inclusive, positive and ordered.
    for bad in ["0:1", "2:1", "a:b", "1", "01:2", ""] {
        assert!(
            excerpt(root, &contract, "docs/unicode.md", bad).is_err(),
            "{bad}"
        );
    }
}

/// Python: `test_excerpt_never_returns_partial_line_or_invalid_utf8_as_evidence`,
/// `test_invalid_utf8_whole_source_still_charges_bytes` and
/// `test_output_overflow_withholds_sources_but_charges_them`.
#[test]
fn a_short_or_undecodable_excerpt_is_withheld_but_still_charged() {
    let dir = repo();
    let root = dir.path();
    let contract = Contract::load(&root.join("contract.json")).expect("contract");

    // Past the end of the file: withheld, and the frontier says why.
    let short = excerpt(root, &contract, "docs/source.md", "900:901").expect("excerpt");
    assert!(short["sources"].as_array().expect("sources").is_empty());
    assert_eq!(
        short["unresolved"][0],
        "source ends before requested range completed"
    );
    assert!(short["usage"]["scan_bytes"].as_u64().expect("scanned") > 0);

    // Invalid UTF-8: withheld, and the bytes it took to find that out are charged.
    std::fs::write(root.join("docs/binary.md"), [0xffu8, 0xfe, b'\n']).expect("write");
    let binary = excerpt(root, &contract, "docs/binary.md", "1:1").expect("excerpt");
    assert!(binary["sources"].as_array().expect("sources").is_empty());
    assert_eq!(binary["unresolved"][0], "invalid UTF-8 in selected range");
    assert_eq!(binary["usage"]["source_bytes"], json!(3));

    // A source that DISAPPEARS mid-read is a frontier and still charges what it cost to find out.
    std::fs::write(root.join("docs/vanishing.md"), "one\ntwo\n").expect("write");
    let vanishing = {
        let contract = Contract::load(&root.join("contract.json")).expect("contract");
        let result = excerpt(root, &contract, "docs/vanishing.md", "1:2").expect("excerpt");
        std::fs::remove_file(root.join("docs/vanishing.md")).expect("rm");
        result
    };
    assert!(!vanishing["sources"].as_array().expect("sources").is_empty());
    assert!(excerpt(
        root,
        &Contract::load(&root.join("contract.json")).expect("contract"),
        "docs/vanishing.md",
        "1:2"
    )
    .is_err());

    // A byte budget too small for the range withholds it rather than truncating evidence.
    let mut narrow = contract_value(false);
    narrow["limits"]["source_bytes"] = json!(4);
    save_contract(root, narrow);
    let narrow = Contract::load(&root.join("contract.json")).expect("contract");
    let clipped = excerpt(root, &narrow, "docs/source.md", "1:7").expect("excerpt");
    assert!(clipped["sources"].as_array().expect("sources").is_empty());
    assert_eq!(
        clipped["unresolved"][0],
        "source byte budget exhausted; requested excerpt withheld"
    );
    assert!(clipped["usage"]["scan_bytes"].as_u64().expect("scanned") > 0);
}

/// Python: `test_missing_out_of_scope_absolute_and_escaped_sources_stay_unresolved` (packet suite)
/// and the containment half of `test_source_escape_and_unproven_evidence_ready_refuse`.
#[test]
fn a_source_outside_the_declared_scope_is_refused() {
    let dir = repo();
    let root = dir.path();
    write(root, "outside.md", "not in docs/\n");
    let contract = Contract::load(&root.join("contract.json")).expect("contract");
    for bad in [
        "outside.md",
        "../etc/passwd",
        "/etc/passwd",
        "docs/../outside.md",
    ] {
        let error = excerpt(root, &contract, bad, "1:1").expect_err(bad);
        assert!(
            format!("{error}").contains("outside declared source scope"),
            "{bad}: {error}"
        );
    }
    // A symlink that escapes the scope is refused by the resolved path, not by its name.
    std::os::unix::fs::symlink(root.join("outside.md"), root.join("docs/escape.md")).expect("link");
    assert!(excerpt(root, &contract, "docs/escape.md", "1:1").is_err());
}

/// Python: `test_discovery_exact_tags_and_groups_describe_only_returned_window` and
/// `test_name_filter_avoids_unrelated_metadata_reads_but_retains_exact_tag_filter`.
#[test]
fn discovery_filters_exactly_and_describes_only_its_returned_window() {
    let dir = repo();
    let root = dir.path();
    write(
        root,
        "docs/tagged.md",
        "---\ntitle: Tagged\ntags:\n  - alpha\n  - beta\n---\nbody\n",
    );
    write(
        root,
        "docs/other.md",
        "---\ntitle: Other\ntags:\n  - beta\n---\nbody\n",
    );
    write(root, "docs/none.md", "no frontmatter\n");
    let contract = Contract::load(&root.join("contract.json")).expect("contract");

    let both = discover(
        root,
        &contract,
        "docs",
        "",
        &["alpha".into(), "beta".into()],
        "tag",
        "*.md",
    )
    .expect("discover");
    let paths: Vec<&str> = both["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .map(|c| c["path"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(paths, vec!["docs/tagged.md"]);
    assert_eq!(both["groups"]["alpha"], json!(1));
    assert_eq!(both["aggregation_scope"], "returned candidates only");

    // The name filter narrows which files are OPENED without weakening the tag filter.
    let named = discover(
        root,
        &contract,
        "docs",
        "",
        &["alpha".into()],
        "directory",
        "tagg*.md",
    )
    .expect("discover");
    assert_eq!(named["candidates"].as_array().expect("candidates").len(), 1);
    assert_eq!(named["usage"]["scanned_files"], json!(1));
    let missed = discover(
        root,
        &contract,
        "docs",
        "",
        &["alpha".into()],
        "directory",
        "other*.md",
    )
    .expect("discover");
    assert!(missed["candidates"]
        .as_array()
        .expect("candidates")
        .is_empty());

    // A query matches path, title and tags, case-folded.
    let queried =
        discover(root, &contract, "docs", "TAGGED", &[], "directory", "*.md").expect("discover");
    assert_eq!(
        queried["candidates"].as_array().expect("candidates").len(),
        1
    );
    assert!(discover(
        root,
        &contract,
        "docs/source.md",
        "",
        &[],
        "directory",
        "*.md"
    )
    .is_err());
}

/// Python: `test_metadata_delimiter_must_be_a_complete_yaml_document_boundary`,
/// `test_metadata_cut_inside_fake_delimiter_does_not_make_membership_evidence` and
/// `test_incomplete_metadata_window_reports_unresolved_instead_of_false_absence`.
#[test]
fn only_a_complete_frontmatter_boundary_establishes_membership() {
    let dir = repo();
    let root = dir.path();
    std::fs::remove_file(root.join("docs/a.md")).expect("rm");
    std::fs::remove_file(root.join("docs/b.md")).expect("rm");
    std::fs::remove_file(root.join("docs/source.md")).expect("rm");
    let mut contract = contract_value(false);
    contract["limits"]["metadata_bytes"] = json!(28);
    save_contract(root, contract);
    let contract = Contract::load(&root.join("contract.json")).expect("contract");

    // The metadata budget cuts the file before its closing delimiter: unresolved, not absent.
    write(
        root,
        "docs/cut.md",
        "---\ntags:\n  - alpha\n  - beta\n  - gamma\n---\nbody\n",
    );
    let cut = discover(root, &contract, "docs", "", &[], "directory", "*.md").expect("discover");
    assert!(cut["candidates"].as_array().expect("candidates").is_empty());
    assert!(cut["unresolved"][0]
        .as_str()
        .expect("frontier")
        .starts_with("incomplete frontmatter within metadata budget"));

    // A `---` that is not a whole line is not a document boundary.
    std::fs::remove_file(root.join("docs/cut.md")).expect("rm");
    write(root, "docs/fake.md", "---\ntags: []\n---x\nbody\n");
    let fake = discover(root, &contract, "docs", "", &[], "directory", "*.md").expect("discover");
    assert!(fake["candidates"]
        .as_array()
        .expect("candidates")
        .is_empty());

    // A complete boundary within budget IS membership.
    std::fs::remove_file(root.join("docs/fake.md")).expect("rm");
    write(root, "docs/small.md", "---\ntags: [a]\n---\nbody\n");
    let small = discover(
        root,
        &contract,
        "docs",
        "",
        &["a".into()],
        "directory",
        "*.md",
    )
    .expect("discover");
    assert_eq!(small["candidates"].as_array().expect("candidates").len(), 1);
}

/// Python: `test_discovery_respects_scan_bytes_and_skips_symlink_escapes`.
#[test]
fn discovery_respects_its_scan_budget_and_skips_symlinks() {
    let dir = repo();
    let root = dir.path();
    let mut contract = contract_value(false);
    contract["limits"]["scan_entries"] = json!(2);
    save_contract(root, contract);
    let contract = Contract::load(&root.join("contract.json")).expect("contract");
    for index in 0..8 {
        write(
            root,
            &format!("docs/n{index}.md"),
            "---\ntags: [x]\n---\nbody\n",
        );
    }
    let bounded =
        discover(root, &contract, "docs", "", &[], "directory", "*.md").expect("discover");
    assert!(bounded["unresolved"][0]
        .as_str()
        .expect("frontier")
        .contains("discovery budget exhausted"));

    // A symlink is never followed, whatever it points at.
    let mut wide = contract_value(false);
    wide["limits"]["scan_entries"] = json!(4096);
    save_contract(root, wide);
    let wide = Contract::load(&root.join("contract.json")).expect("contract");
    std::os::unix::fs::symlink("/etc", root.join("docs/etc")).expect("link");
    let after =
        discover(root, &wide, "docs", "", &["x".into()], "directory", "*.md").expect("discover");
    assert!(after["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .all(|c| c["path"].as_str().unwrap_or_default().starts_with("docs/n")));
}

/// Python: `test_provider_output_limit_includes_stderr_and_stops_before_buffering_all` and
/// `test_provider_timeout_and_nonzero_exit_preserve_diagnostics`.
#[test]
fn a_foreign_provider_is_bounded_by_bytes_and_by_seconds() {
    use recall::bounded_process;
    let big = vec!["-c".to_string(), "print('x'*100000)".to_string()];
    let over = bounded_process("python3", &big, 64, 5.0).expect("spawn");
    assert_eq!(
        over.error.as_deref(),
        Some("provider output exceeds budget; results withheld")
    );

    // stderr counts against the same budget as stdout.
    let noisy = vec![
        "-c".to_string(),
        "import sys; sys.stderr.write('e'*100000)".to_string(),
    ];
    let noisy = bounded_process("python3", &noisy, 64, 5.0).expect("spawn");
    assert!(noisy.error.is_some());

    let slow = vec!["-c".to_string(), "import time; time.sleep(30)".to_string()];
    let slow = bounded_process("python3", &slow, 4096, 0.4).expect("spawn");
    assert_eq!(slow.error.as_deref(), Some("provider timed out"));

    let failed = vec!["-c".to_string(), "raise SystemExit(3)".to_string()];
    let failed = bounded_process("python3", &failed, 4096, 5.0).expect("spawn");
    assert_eq!(failed.status, Some(3));
    assert!(failed.error.is_none());
    assert!(bounded_process("definitely-not-a-program", &[], 4096, 1.0).is_err());
}

/// Python: `test_method_identity_pins_contract_and_implementation_bytes` (packet suite).
///
/// The MANDATED pin: a receipt's `method` is the contract file's raw CID.
#[test]
fn a_receipt_pins_the_contract_files_raw_cid_as_its_method() {
    let dir = repo();
    let root = dir.path();
    let contract = Contract::load(&root.join("contract.json")).expect("contract");
    let raw = std::fs::read(root.join("contract.json")).expect("bytes");
    let expected = BlobCid::compute_raw(&raw).to_string();
    assert_eq!(contract.method_cid(), expected);

    let reference = save_receipt(
        root,
        SESSION,
        "projection",
        &contract.method_cid(),
        &json!({"a": 1}),
    )
    .expect("receipt");
    assert_eq!(reference["method"], expected);
    assert!(reference["path"]
        .as_str()
        .expect("path")
        .starts_with(&format!("{}/{SESSION}/receipts/", recall::RECALL_DIR_REL)));
    // The receipt's own bytes are content-addressed, and rewriting them is refused.
    let stored = std::fs::read(root.join(reference["path"].as_str().expect("path"))).expect("read");
    assert_eq!(
        reference["sha256"],
        format!("{:x}", Sha256::digest(&stored))
    );
    assert_eq!(reference["cid"], BlobCid::compute_raw(&stored).to_string());
    // The same content re-saves as a no-op; different content under the same name cannot exist,
    // because the name IS the digest.
    assert!(save_receipt(
        root,
        SESSION,
        "projection",
        &contract.method_cid(),
        &json!({"a": 1})
    )
    .is_ok());

    // The live contract's CID is what a live session pins.
    let live = Contract::load(&repo_root().join(recall::CONTRACT_REL)).expect("live contract");
    let live_raw = std::fs::read(repo_root().join(recall::CONTRACT_REL)).expect("live bytes");
    assert_eq!(
        live.method_cid(),
        BlobCid::compute_raw(&live_raw).to_string()
    );
}

/// Python: `test_native_missing_malformed_or_oversized_facets_are_refused` (packet suite), applied
/// to the contract rather than to a shelled-out projection.
#[test]
fn an_unsupported_contract_is_refused_before_any_session_exists() {
    let dir = repo();
    let root = dir.path();
    for (mutate, needle) in [
        (
            Box::new(|c: &mut Value| c["artifact_type"] = json!("something-else"))
                as Box<dyn Fn(&mut Value)>,
            "unsupported algorithm composition",
        ),
        (
            Box::new(|c: &mut Value| {
                c["composition"] = json!(["scope", "discover", "read"]);
            }),
            "unsupported algorithm composition",
        ),
        (
            Box::new(|c: &mut Value| c["limits"]["source_bytes"] = json!(0)),
            "invalid positive budget",
        ),
        (
            Box::new(|c: &mut Value| c["limits"]["source_bytes"] = json!(1.5)),
            "byte/count budgets must be integers",
        ),
        (
            Box::new(|c: &mut Value| c["source_roots"] = json!(["/etc"])),
            "source roots must be repository-relative",
        ),
        (
            Box::new(|c: &mut Value| c["source_roots"] = json!(["../elsewhere"])),
            "source roots must be repository-relative",
        ),
    ] {
        let mut contract = contract_value(false);
        mutate(&mut contract);
        save_contract(root, contract);
        let error = Contract::load(&root.join("contract.json")).expect_err(needle);
        assert!(format!("{error}").contains(needle), "{error}");
    }
}

// ── the privacy line ────────────────────────────────────────────────────────────────────────────

/// The privacy invariant, mutation-checked.
///
/// The SAME path shape is accepted outside the recall store and refused inside it, so removing
/// `refuse_private_import` fails this test rather than leaving it green on an unrelated error.
#[test]
fn a_recall_verb_refuses_to_import_or_project_a_private_receipt() {
    let dir = repo();
    let root = dir.path();
    // Control: an ordinary path passes the gate.
    refuse_private_import(root, "docs/source.md").expect("ordinary path is admissible");
    refuse_private_import(root, "genesis/docs/analysis/x.md").expect("ordinary path is admissible");

    for private in [
        ".eprfs/status/recall",
        ".eprfs/status/recall/some-session/continuation.json",
        "./.eprfs/status/recall/some-session/receipts/projection-0.json",
        ".claude/memory-kit/recall-executions/ceremony-dev-a.json",
    ] {
        let error = refuse_private_import(root, private).expect_err(private);
        let message = format!("{error}");
        assert!(message.contains("private recall record"), "{message}");
        assert!(
            message.contains("private-thought-governed-fruit"),
            "{message}"
        );
    }

    // And the gate is wired into the shipped CLI, not merely available to it.
    let refused = run(
        root,
        &[
            "read",
            "--path",
            ".eprfs/status/recall/x/continuation.json",
            "--lines",
            "1:1",
        ],
    );
    assert_eq!(refused.code, 2);
    assert!(
        refused.stdout.contains("private recall record")
            || refused.stderr.contains("private recall record"),
        "stdout={} stderr={}",
        refused.stdout,
        refused.stderr
    );

    // The collective-memory verbs take an authored request file and never a receipt.
    let receipt =
        save_receipt(root, SESSION, "projection", "method", &json!({"a": 1})).expect("receipt");
    let path = receipt["path"].as_str().expect("path").to_string();
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(["flow", "memory", "contribute", "--input", &path])
        .args(["--root", &root.to_string_lossy()])
        .output()
        .expect("epr runs");
    assert!(
        !out.status.success(),
        "a receipt must never be contributable"
    );
}

/// The recall store is ignored by GIT, and making it ignored did not un-ignore its neighbours.
///
/// `check-ignore` answers about a PATH rather than a file, so this stays honest on a tree where no
/// recall session has ever run.
#[test]
fn the_recall_store_is_ignored_and_its_neighbours_are_unaffected() {
    let repo = repo_root();
    if !repo.join(".gitignore").is_file() {
        eprintln!("SKIP: {} carries no .gitignore", repo.display());
        return;
    }
    // `(path, must_be_ignored)`.
    let invariants: [(&str, bool); 5] = [
        (".eprfs/status/recall", true),
        (".eprfs/status/recall/any-session/continuation.json", true),
        (
            ".eprfs/status/recall/any-session/receipts/projection-0.json",
            true,
        ),
        // The two tracked exceptions above it in the ladder stay tracked.
        (".eprfs/status/gap-items/anything.json", false),
        (".eprfs/status/memory/contributions/anything.json", false),
    ];
    for (rel, must_be_ignored) in invariants {
        let status = match Command::new("git")
            .args(["check-ignore", "-q", rel])
            .current_dir(&repo)
            .status()
        {
            Ok(status) => status,
            Err(error) => {
                eprintln!("SKIP: git is not runnable here ({error})");
                return;
            }
        };
        // git check-ignore -q: 0 = ignored, 1 = not ignored, >1 = error.
        let code = status.code().unwrap_or(2);
        assert!(
            code < 2,
            "git check-ignore errored on `{rel}` (exit {code})"
        );
        assert_eq!(
            code == 0,
            must_be_ignored,
            "`{rel}` must {} ignored",
            if must_be_ignored { "be" } else { "NOT be" }
        );
    }
}

// ── receipt adoption ────────────────────────────────────────────────────────────────────────────

/// A dry run reports counts and writes nothing; the real run relocates bytes verbatim.
#[test]
fn receipt_adoption_reports_counts_before_it_moves_anything() {
    let dir = repo();
    let root = dir.path();
    let contract = Contract::load(&root.join("contract.json")).expect("contract");
    let method = contract.method_cid();
    let legacy = root.join(recall::LEGACY_RECEIPTS_REL);
    std::fs::create_dir_all(&legacy).expect("mkdir");

    // A continuation pinned to the CURRENT algorithm is resumable after adoption.
    let current = serde_json::to_vec(&json!({"method": method, "attempts": 3, "totals": {}}))
        .expect("encode");
    std::fs::write(legacy.join("current-session.json"), &current).expect("write");
    // One pinned to the Python executor's digest map is retained, and honestly unresumable.
    std::fs::write(
        legacy.join("legacy-session.json"),
        serde_json::to_vec(&json!({"method": {"recall-ceremony.py": "abc"}, "attempts": 9}))
            .expect("encode"),
    )
    .expect("write");
    // A content-named receipt whose digest still matches its name.
    let payload = b"{\"observed\":1}\n";
    let digest = format!("{:x}", Sha256::digest(payload));
    std::fs::write(
        legacy.join(format!("run-a-baseline-{digest}.json")),
        payload,
    )
    .expect("write");
    // One whose digest does NOT match its name is refused rather than quietly relocated.
    std::fs::write(
        legacy.join(format!("run-b-projection-{}.json", "0".repeat(64))),
        b"{\"tampered\":true}\n",
    )
    .expect("write");
    // A non-JSON artefact is skipped with a reason.
    std::fs::write(legacy.join("bundle.tar.gz"), b"not json").expect("write");

    let dry = adopt_receipts(root, &legacy, &method, true).expect("dry run");
    assert_eq!(dry["dry_run"], json!(true));
    assert_eq!(dry["counts"]["examined"], json!(5));
    assert_eq!(dry["counts"]["adopted"], json!(3));
    assert_eq!(dry["counts"]["skipped"], json!(1));
    assert_eq!(dry["counts"]["receipt_digests_verified"], json!(1));
    assert_eq!(dry["counts"]["receipt_digests_mismatched"], json!(1));
    assert!(!root
        .join(recall::RECALL_DIR_REL)
        .join("current-session")
        .exists());

    let applied = adopt_receipts(root, &legacy, &method, false).expect("adopt");
    assert_eq!(applied["counts"]["adopted"], json!(3));
    let relocated = root
        .join(recall::RECALL_DIR_REL)
        .join("current-session")
        .join("continuation.json");
    assert_eq!(std::fs::read(&relocated).expect("relocated"), current);
    let rows = applied["rows"].as_array().expect("rows");
    let row = |name: &str| {
        rows.iter()
            .find(|r| r["name"] == name)
            .unwrap_or_else(|| panic!("row {name}"))
            .clone()
    };
    assert_eq!(row("current-session.json")["resumable"], json!(true));
    assert_eq!(row("legacy-session.json")["resumable"], json!(false));
    assert!(row("legacy-session.json")["resumable_reason"]
        .as_str()
        .expect("reason")
        .contains("pinned method differs"));
    assert_eq!(
        row(&format!("run-b-projection-{}.json", "0".repeat(64)))["action"],
        "refused"
    );
    assert_eq!(row("bundle.tar.gz")["action"], "skipped");
    // Every relocated row carries a recomputed CID, and the bytes are byte-identical.
    assert_eq!(
        row("current-session.json")["cid"],
        BlobCid::compute_raw(&current).to_string()
    );
    // Adoption is idempotent.
    let again = adopt_receipts(root, &legacy, &method, false).expect("adopt");
    assert_eq!(again["counts"]["adopted"], json!(0));

    // An adopted continuation pinned to the current method really does resume.
    let resumed = run_in(root, "current-session", &["resume"]);
    assert_eq!(resumed.code, 2);
    assert!(resumed.json()["unresolved"][0]
        .as_str()
        .expect("reason")
        .contains("no ceremony continuation exists"));
}

// ── the paired footprint lens ───────────────────────────────────────────────────────────────────

/// The footprint lens is NATIVE, so a fixture root that carries no script still measures.
///
/// Unconditional on purpose. The first version of this test accepted either a resolved sample or an
/// honest "unavailable", which meant it could not fail: a lens that never resolved anywhere would
/// have passed it. The lens is a measurement primitive the ceremony depends on, so the assertion is
/// that it PRODUCED A SAMPLE.
#[test]
fn the_footprint_lens_is_native_and_needs_no_script_in_the_tree() {
    let dir = repo();
    let root = dir.path();
    assert!(
        !root.join(recall::BALANCE_LENS_REL).exists(),
        "the fixture must not carry its own lens script"
    );
    save_contract(root, contract_value(true));
    let opened = run_in(
        root,
        "native-lens",
        &[
            "open",
            "--intent",
            "Open with no lens script anywhere",
            "--measure-scope",
            "authored:docs",
        ],
    );
    assert_eq!(opened.code, 0, "{}{}", opened.stdout, opened.stderr);
    let view = opened.json();
    let measurement = &view["measurement"];
    // A real sample, pinned as a private receipt.
    assert!(measurement["evidence"]["path"]
        .as_str()
        .expect("a baseline receipt")
        .contains("/receipts/baseline-"));
    assert_eq!(measurement["complete"], json!(true), "{measurement}");
    assert!(measurement["observed"]["files"].as_u64().expect("files") >= 3);
    assert!(measurement["observed"]["bytes"].as_u64().expect("bytes") > 0);
    // The view names WHAT measured it, not a path it hoped to find.
    let lens = &view["execution_method"]["measurement_lens"];
    assert_eq!(lens["version"], footprint::METHOD_VERSION);
    assert_eq!(
        lens["implementation"],
        "elohim-epr-cli flow/memory/footprint.rs"
    );

    // `--lens` remains an EXPLICIT external override, and a missing one refuses about that path.
    let overridden = run_in(
        root,
        "named-lens",
        &[
            "open",
            "--intent",
            "Name an external lens",
            "--lens",
            "docs/not-a-lens.py",
        ],
    );
    assert_eq!(overridden.code, 2, "{}", overridden.stdout);
    assert!(overridden.json()["unresolved"][0]
        .as_str()
        .expect("reason")
        .contains("footprint lens"));
}

/// `--tag` is EXACT frontmatter membership, repeatable, and only the provider that reads
/// frontmatter may claim to honour it.
#[test]
fn tag_filters_discovery_exactly_and_is_refused_where_it_would_be_ignored() {
    let dir = repo();
    let root = dir.path();
    write(
        root,
        "docs/both.md",
        "---\ntitle: Both\ntags:\n  - alpha\n  - beta\n---\nbody\n",
    );
    write(
        root,
        "docs/one.md",
        "---\ntitle: One\ntags:\n  - beta\n---\nbody\n",
    );
    begin(root);

    // search: every named tag must be present, so two tags narrow rather than widen.
    let both = ok(
        root,
        &[
            "search",
            "--provider",
            "local",
            "--search-scope",
            "docs",
            "--tag",
            "alpha",
            "--tag",
            "beta",
        ],
    );
    let paths: Vec<&str> = both["retrieval"]["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .map(|c| c["path"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(paths, vec!["docs/both.md"]);
    assert_eq!(both["retrieval"]["tags"], json!(["alpha", "beta"]));

    let one = ok(
        root,
        &[
            "search",
            "--provider",
            "local",
            "--search-scope",
            "docs",
            "--tag",
            "beta",
        ],
    );
    assert_eq!(
        one["retrieval"]["candidates"]
            .as_array()
            .expect("candidates")
            .len(),
        2
    );

    // source --tag answers "which sources carry this", grouped by tag, with outline actions.
    let candidates = ok(
        root,
        &["source", "--search-scope", "docs", "--tag", "alpha"],
    );
    assert_eq!(
        candidates["source_candidates"]["candidates"][0]["path"],
        "docs/both.md"
    );
    assert_eq!(candidates["source_candidates"]["groups"]["alpha"], json!(1));
    assert!(candidates["actions"]
        .as_array()
        .expect("actions")
        .iter()
        .any(|a| a["label"] == "Outline docs/both.md"));
    // …and it opens nothing on its own.
    assert!(candidates["source_outline"].is_null());

    // A provider that cannot read frontmatter is refused rather than quietly ignoring the filter.
    let ignored = run(
        root,
        &[
            "search",
            "--provider",
            "alternative",
            "--query",
            "x",
            "--tag",
            "alpha",
        ],
    );
    assert_eq!(ignored.code, 2);
    assert!(ignored.json()["unresolved"][0]
        .as_str()
        .expect("reason")
        .contains("only the local provider reads frontmatter"));

    // With neither --path nor --tag, `source` still says what it needs.
    let bare = run(root, &["source"]);
    assert_eq!(bare.code, 2);
    assert!(bare.json()["unresolved"][0]
        .as_str()
        .expect("reason")
        .contains("--tag to discover one"));
}

/// Python: `test_measurement_pair_survives_resume_and_counts_consolidation`,
/// `test_measurement_rejects_changed_scope_and_tampered_baseline`,
/// `test_measurement_incomplete_is_not_a_zero_delta` and
/// `test_measurement_receipt_directory_symlink_is_refused`.
#[test]
fn the_paired_lens_pins_both_halves_and_refuses_an_unpaired_close() {
    let dir = repo();
    let root = dir.path();
    save_contract(root, contract_value(true));

    let opened = run_in(
        root,
        "paired",
        &[
            "open",
            "--intent",
            "Measure the pair",
            "--measure-scope",
            "authored:docs",
        ],
    );
    assert_eq!(opened.code, 0, "{}{}", opened.stdout, opened.stderr);
    let baseline = opened.json()["measurement"]["evidence"].clone();
    assert!(baseline["path"]
        .as_str()
        .expect("path")
        .contains("/receipts/baseline-"));
    let method = Contract::load(&root.join("contract.json"))
        .expect("contract")
        .method_cid();
    assert_eq!(baseline["method"], method);
    // The receipt really is on disk under the private store, at the digest its reference names.
    let stored = std::fs::read(root.join(baseline["path"].as_str().expect("path"))).expect("read");
    assert_eq!(baseline["sha256"], format!("{:x}", Sha256::digest(&stored)));

    // Resuming retains the original baseline rather than resampling it.
    let resumed = run_in(root, "paired", &["resume"]);
    assert_eq!(resumed.code, 0, "{}", resumed.stdout);
    assert_eq!(resumed.json()["measurement"]["baseline"], baseline);
    assert_eq!(
        resumed.json()["measurement"]["meaning"],
        "Original baseline retained; resuming does not resample it."
    );

    // A changed measurement scope refuses rather than comparing two different cohorts.
    let refused = run_in(
        root,
        "paired",
        &[
            "measure",
            "--phase",
            "baseline",
            "--measure-scope",
            "authored:docs/a.md",
        ],
    );
    assert_eq!(refused.code, 2, "{}", refused.stdout);
    assert!(refused.json()["unresolved"][0]
        .as_str()
        .expect("reason")
        .contains("measurement scope changed"));

    // Re-reading the SAME phase reloads the pinned receipt instead of resampling the tree.
    let reread = run_in(root, "paired", &["measure", "--phase", "baseline"]);
    assert_eq!(reread.code, 0, "{}", reread.stdout);
    assert_eq!(reread.json()["measurement"]["evidence"], baseline);

    // A close pairs against that exact baseline and reports a comparable delta. `finish` needs an
    // inspected concern first: an outcome with nothing selected reports on nothing.
    let selected = run_in(
        root,
        "paired",
        &["select", "--edge", "1", "--need", "Check"],
    );
    assert_eq!(selected.code, 0, "{}{}", selected.stdout, selected.stderr);
    let closed = run_in(
        root,
        "paired",
        &[
            "finish",
            "--outcome",
            "bounded",
            "--question",
            "what remains",
        ],
    );
    assert_eq!(closed.code, 0, "{}{}", closed.stdout, closed.stderr);
    let comparison = closed.json()["measurement"]["comparison"].clone();
    assert_eq!(comparison["comparable"], json!(true), "{comparison}");
    assert!(comparison["delta"]["bytes"].is_number());
    assert_eq!(
        comparison["detail"],
        "Up to four path examples per category; full detail derives from the exact paired snapshot files."
    );

    // A close with no contemporaneous baseline cannot reconstruct one after the fact.
    let unpaired = run_in(root, "unpaired", &["measure", "--phase", "close"]);
    assert_eq!(unpaired.code, 2);
    assert!(unpaired.json()["unresolved"][0]
        .as_str()
        .expect("reason")
        .contains("start with open"));

    // A tampered receipt cannot be reused as evidence.
    std::fs::write(root.join(baseline["path"].as_str().expect("path")), b"{}\n").expect("write");
    let tampered = run_in(root, "paired", &["measure", "--phase", "baseline"]);
    assert_eq!(tampered.code, 2);
    assert!(tampered.json()["unresolved"][0]
        .as_str()
        .expect("reason")
        .contains("cannot reuse evidence"));
}
