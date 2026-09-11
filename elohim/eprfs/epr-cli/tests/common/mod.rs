//! Shared fixture helpers for the recall executor's integration test binaries.
//!
//! Moved out of `flow_memory_recall.rs` verbatim (station zero of the governed-discovery split,
//! task 0.1) so `flow_memory_recall_golden.rs` can build the same fixture repo without duplicating
//! it. Behaviour is unchanged — only the location moved.
//!
//! Shared across several independent test binaries (`flow_memory_recall*.rs`), each of which
//! `mod common;`-includes this whole file fresh and uses only the subset it needs. An item one
//! binary never reaches is genuinely dead FOR THAT BINARY'S compilation, so it carries its own
//! `#[allow(dead_code)]` rather than a blanket module-level one — the fix-round-1 ruling that
//! narrowed this from the earlier blanket attribute.

use std::path::{Path, PathBuf};
use std::process::Command;

use elohim_epr_cli::flow::memory::recall;
use elohim_epr_rea::{AgentRef, DepEdge, FlowRecord, FlowStore, Governor, SidecarFlowStore};
use eprfs_core::BlobCid;
use serde_json::{json, Value};
use tempfile::TempDir;

#[allow(dead_code)]
pub const SESSION: &str = "station-five";

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root")
}

/// The live algorithm artifact, read from its EPRFS owner — never a local copy.
///
/// A hand-written fixture contract would let the real one drift (a budget renamed, a stage added)
/// while these tests stayed green about an algorithm nobody runs.
pub fn live_contract() -> Value {
    let raw = std::fs::read(repo_root().join(recall::CONTRACT_REL)).expect("live contract");
    serde_json::from_slice(&raw).expect("contract parses")
}

pub fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, text).expect("write");
}

/// Pinned for the same reason the corrections fixture pins it: git HEAD's author date is part of a
/// note's content address, so a fixture built a second later is a different address.
pub const FIXTURE_DATE: &str = "2026-09-10T00:00:00+00:00";

pub fn git(root: &Path, args: &[&str]) {
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

pub fn edge(root: &Path, from: &str, to: &str) {
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
pub fn repo() -> TempDir {
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
pub fn contract_value(with_measurements: bool) -> Value {
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
    // Pinned explicitly here (station 1, fix round 1) rather than left to inherit whatever
    // `live_contract()` happens to declare: a test fixture that only inherits the real
    // `lens_table` cannot tell "lens.rs's parse_table() actually ran" from "it silently fell
    // through to the builtin table", because the two are value-identical. Pinning the exact
    // brief JSON here — same values as `lens::builtin_table()` — keeps every existing rendering
    // unchanged while making the DECLARED path a real, test-owned fact rather than an
    // assumption borrowed from the live file.
    contract["lens_table"] = json!({
        "defaults": {"level": "standard", "choice_count": 6, "density_bytes": 6000, "scaffold": "rank-and-let-choose"},
        "stated": {
            "claude-haiku-4-5": "minimal",
            "claude-sonnet-5": "simple",
            "claude-opus-5": "standard",
            "claude-fable-5-1": "detail",
            "gpt-5.6-sol": "standard"
        },
        "levels": {
            "minimal": {"choice_count": 1, "density_bytes": 1500, "scaffold": "locate-and-hand-one-command"},
            "simple": {"choice_count": 3, "density_bytes": 3000, "scaffold": "locate-and-hand-one-command"},
            "standard": {"choice_count": 6, "density_bytes": 6000, "scaffold": "rank-and-let-choose"},
            "detail": {"choice_count": 12, "density_bytes": 12000, "scaffold": "rank-and-let-choose"},
            "debug": {"choice_count": 12, "density_bytes": 24000, "scaffold": "rank-and-let-choose"},
            "trace": {"choice_count": 24, "density_bytes": 32768, "scaffold": "rank-and-let-choose"}
        },
        "revealed_rule": "a reader tier whose last 3 journeys reached authority at level L is offered the next level; a tier with a mistaken assertion in its last 3 is offered the previous level",
        "expiry_days": 90
    });
    contract
}

pub fn save_contract(root: &Path, contract: Value) {
    write(
        root,
        "contract.json",
        &serde_json::to_string(&contract).expect("encode"),
    );
}

pub struct Run {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

impl Run {
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.stdout)
            .unwrap_or_else(|error| panic!("stdout is not JSON ({error}): {}", self.stdout))
    }
}

pub fn run_in(root: &Path, session: &str, args: &[&str]) -> Run {
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

#[allow(dead_code)]
pub fn run(root: &Path, args: &[&str]) -> Run {
    run_in(root, SESSION, args)
}

#[allow(dead_code)]
pub fn ok(root: &Path, args: &[&str]) -> Value {
    let run = run(root, args);
    assert_eq!(
        run.code, 0,
        "{args:?} refused: {}{}",
        run.stdout, run.stderr
    );
    run.json()
}

/// `ok`, but for a caller that needs its own `--session` label instead of the shared [`SESSION`]
/// constant — the shape every governed-discovery lens test needs, since a lens is resolved per
/// session and three tests sharing one session would resolve against each other's claims.
///
/// `#[allow(dead_code)]`: this shared fixture module is included by every `flow_memory_recall*`
/// test binary, and not every one of them calls every helper — a binary that does not is not
/// dead code in the sense this lint means.
#[allow(dead_code)]
pub fn view_in(root: &Path, session: &str, args: &[&str]) -> Value {
    let run = run_in(root, session, args);
    assert_eq!(
        run.code, 0,
        "{args:?} refused: {}{}",
        run.stdout, run.stderr
    );
    run.json()
}

/// The same invocation as [`view_in`], but the HUMAN rendering (no `--json`) — for a caller
/// asserting on the printed screen rather than the payload.
#[allow(dead_code)]
pub fn text_in(root: &Path, session: &str, args: &[&str]) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
    command
        .args(["flow", "memory", "recall"])
        .args(args)
        .args(["--root", &root.to_string_lossy()])
        .args(["--contract", "contract.json"])
        .args(["--session", session]);
    let out = command.output().expect("epr runs");
    assert!(
        out.status.success(),
        "{args:?} refused: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// Register an actor claim against the fixture root — `epr actor claim --as <ref> --session <id>
/// --root <root>`. Requires a committed HEAD (claims are dated by the tree, never by wall clock —
/// see `elohim_epr_rea::ActorClaim`), which [`repo`] already provides.
#[allow(dead_code)]
pub fn claim_actor(root: &Path, claimed: &str, session: &str) {
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(["actor", "claim", "--as", claimed, "--session", session])
        .arg("--root")
        .arg(root)
        .output()
        .expect("epr actor claim runs");
    assert!(
        out.status.success(),
        "actor claim {claimed} refused: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Open a session with a question and no concern selection — the FOCUSED door.
///
/// `begin` selects a concern edge, which is the ceremony door; a focused journey has no edge to
/// select and is the shape several assertions below are about.
#[allow(dead_code)]
pub fn begin_focused(root: &Path, about: &str) {
    ok(
        root,
        &[
            "open",
            "--need",
            &format!("where does the pelican nest in {about}"),
        ],
    );
}
