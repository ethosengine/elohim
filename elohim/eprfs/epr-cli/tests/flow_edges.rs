//! Seal-aware walk integration — the slice's end-to-end proof (plan Task 3/4). A synthetic
//! git repo: seal an edge → status shows it sealed; mutate the upstream → status shows it
//! stale AND walk-forward from the upstream lists the downstream in `stale_edges`; reseal →
//! stale clears; a held edge stays held (excluded from stale); a compiler-governed edge
//! never goes stale under upstream mutation.

use std::path::Path;

use elohim_epr_cli::flow::note::NoteActor;
use elohim_epr_cli::flow::{claim, context, fulfill, ledger, note, project, seal, stocks, walk};
use elohim_epr_rea::{FlowRecord, FlowStore, SidecarFlowStore};
use tempfile::TempDir;

fn git(root: &Path, args: &[&str]) {
    // build_command, not a bare Command: git exports GIT_DIR into hook
    // environments and this suite runs under the pre-push hook, where a bare
    // spawn is redirected at the ambient repo instead of this TempDir.
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

/// A fixture repo with NO recipe registry — the doc plane is empty, so every counted edge
/// comes from the sidecar seals this test writes (precise counts).
fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    // Downstream artifacts (code — cannot carry frontmatter) and their upstream contracts.
    write(
        root,
        "app/foo.ts",
        "// downstream that conforms to the schema\n",
    );
    write(root, "spec/bar.md", "The upstream contract, version one.\n");
    write(
        root,
        "app/held.ts",
        "// a downstream with a declared deviation\n",
    );
    write(root, "spec/held-up.md", "Held upstream, version one.\n");
    write(root, "app/gov.ts", "// a compiler-governed downstream\n");
    write(root, "spec/gov-up.md", "Governed upstream, version one.\n");
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn mutate(root: &Path, rel: &str, contents: &str) {
    std::fs::write(root.join(rel), contents).unwrap();
    // A new commit so the seal/reseal git-derived timestamp advances.
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "mutate"]);
}

#[test]
fn seal_stale_reseal_hold_and_governed_end_to_end() {
    let dir = fixture();
    let root = dir.path();

    // ── seal → 1 sealed ──────────────────────────────────────────────────────────
    seal::seal(
        root,
        "app/foo.ts",
        "spec/bar.md",
        Some("cite-seal"),
        Some("foo conforms to bar".into()),
    )
    .expect("seal runs");
    let s = walk::status(root).expect("status");
    assert_eq!(s.edges_sealed, 1, "one healthy sealed edge");
    assert_eq!(s.edges_stale, 0);

    // ── mutate upstream → 1 stale, and walk-forward from upstream lists downstream ─
    mutate(
        root,
        "spec/bar.md",
        "The upstream contract, version TWO — drifted.\n",
    );
    let s = walk::status(root).expect("status");
    assert_eq!(s.edges_stale, 1, "the sealed edge is now stale");
    assert_eq!(s.edges_sealed, 0);

    let w = walk::walk(root, "spec/bar.md").expect("walk upstream");
    let stale_froms: Vec<&str> = w
        .frontier
        .stale_edges
        .iter()
        .map(|e| e.from.as_str())
        .collect();
    assert!(
        stale_froms.contains(&"app/foo.ts"),
        "walk-forward from the upstream must surface the stale downstream, got {stale_froms:?}"
    );
    // The same edge appears as an incoming edge on the upstream's Edges section.
    assert!(w
        .edges
        .incoming
        .iter()
        .any(|e| e.from == "app/foo.ts" && e.verdict == "stale"));

    // ── reseal → 0 stale ─────────────────────────────────────────────────────────
    let resealed = seal::reseal(root, "app/foo.ts", Some("spec/bar.md"), false).expect("reseal");
    assert_eq!(resealed.len(), 1);
    let s = walk::status(root).expect("status");
    assert_eq!(s.edges_stale, 0, "reseal cleared the stale edge");
    assert_eq!(s.edges_sealed, 1);

    // ── held edge stays held, excluded from stale even when the upstream drifts ────
    seal::hold(
        root,
        "app/held.ts",
        "spec/held-up.md",
        "read-legacy, write-canonical — transitional".into(),
        None,
    )
    .expect("hold runs");
    mutate(
        root,
        "spec/held-up.md",
        "Held upstream, version TWO — drifted.\n",
    );
    let s = walk::status(root).expect("status");
    assert_eq!(s.edges_held, 1, "the held edge is counted held");
    assert_eq!(s.edges_stale, 0, "a held edge never enters the stale set");

    // ── compiler-governed edge never goes stale under mutation ────────────────────
    seal::seal(
        root,
        "app/gov.ts",
        "spec/gov-up.md",
        Some("compiler:tsc"),
        None,
    )
    .expect("seal governed");
    mutate(
        root,
        "spec/gov-up.md",
        "Governed upstream, version TWO — drifted.\n",
    );
    let s = walk::status(root).expect("status");
    assert_eq!(s.edges_governed, 1, "the compiler edge is governed");
    assert_eq!(s.edges_stale, 0, "a governed edge never goes stale");
    // And it carries no sealed CID.
    let w = walk::walk(root, "app/gov.ts").expect("walk governed downstream");
    assert!(w
        .edges
        .outgoing
        .iter()
        .any(|e| e.governor == "compiler:tsc" && e.verdict == "governed"));
}

#[test]
fn cite_seal_on_missing_upstream_is_an_error() {
    let dir = fixture();
    let root = dir.path();
    let err = seal::seal(
        root,
        "app/foo.ts",
        "spec/does-not-exist.md",
        Some("cite-seal"),
        None,
    );
    assert!(
        err.is_err(),
        "cite-seal on a missing upstream is dangling-at-birth"
    );
}

/// `seal --on` must refuse a target that is not a readable file under the root, whichever
/// governor arm it lands in. A sealed edge to nothing is a record that reads as evidence.
#[test]
fn seal_refuses_an_on_target_that_does_not_exist_under_root() {
    let dir = fixture();
    let root = dir.path();

    // (1) explicit cite-seal — dangling at birth
    assert!(seal::seal(root, "app/foo.ts", "spec/nope.md", Some("cite-seal"), None).is_err());
    // (2) explicit governed — no seal to compute, still a user error
    assert!(seal::seal(
        root,
        "app/foo.ts",
        "spec/nope.md",
        Some("test:some-test"),
        None
    )
    .is_err());
    // (3) auto-derived governor — the arm a caller reaches by omitting --governor entirely
    assert!(seal::seal(root, "app/foo.ts", "spec/nope.md", None, None).is_err());
    // (4) a directory is not a file
    assert!(seal::seal(root, "app/foo.ts", "spec", Some("cite-seal"), None).is_err());

    // Nothing was appended by any of the four refusals.
    let s = walk::status(root).expect("status");
    assert_eq!(
        s.edges_sealed + s.edges_stale + s.edges_held + s.edges_dangling + s.edges_governed,
        0,
        "a refused seal leaves the sidecar byte-identical"
    );
}

#[test]
fn reseal_on_a_currently_ok_edge_is_an_error() {
    let dir = fixture();
    let root = dir.path();
    seal::seal(
        root,
        "app/foo.ts",
        "spec/bar.md",
        Some("cite-seal"),
        Some("foo conforms to bar".into()),
    )
    .expect("seal runs");
    // The edge is freshly sealed — Ok, not Stale. `reseal --on` must refuse a redundant
    // re-bless rather than silently appending a second identical-in-substance record.
    let s = walk::status(root).expect("status");
    assert_eq!(s.edges_sealed, 1);
    assert_eq!(s.edges_stale, 0);

    let err = seal::reseal(root, "app/foo.ts", Some("spec/bar.md"), false)
        .expect_err("reseal --on a currently-Ok edge must error, not silently re-append");
    let msg = err.to_string();
    assert!(
        msg.contains("not stale"),
        "error should explain the edge is not stale, got: {msg}"
    );

    // And no redundant record was appended — still exactly one sealed edge.
    let s = walk::status(root).expect("status");
    assert_eq!(s.edges_sealed, 1, "no redundant append happened");
    assert_eq!(s.edges_stale, 0);
}

#[test]
fn reseal_path_confinement_rejects_escaping_on_and_file_args() {
    let dir = fixture();
    let root = dir.path();
    seal::seal(
        root,
        "app/foo.ts",
        "spec/bar.md",
        Some("cite-seal"),
        Some("foo conforms to bar".into()),
    )
    .expect("seal runs");
    mutate(
        root,
        "spec/bar.md",
        "The upstream contract, version TWO — drifted.\n",
    );

    // A file created OUTSIDE the fixture root, in its parent directory.
    let outside = root.parent().expect("tempdir has a parent");
    std::fs::write(outside.join("outside.md"), "not part of this repo\n").unwrap();

    // `--on ../outside.md` must be rejected — it resolves outside the confined root.
    let err = seal::reseal(root, "app/foo.ts", Some("../outside.md"), false)
        .expect_err("--on escaping the root must error");
    assert!(
        err.to_string().contains("escapes"),
        "expected a path-confinement error, got: {err}"
    );

    // An absolute path outside the root must be rejected too.
    let abs_outside = outside.join("outside.md");
    let err = seal::reseal(
        root,
        "app/foo.ts",
        Some(abs_outside.to_str().unwrap()),
        false,
    )
    .expect_err("an absolute --on path outside the root must error");
    assert!(
        err.to_string().contains("escapes"),
        "expected a path-confinement error, got: {err}"
    );

    // The legitimate stale edge is still resealable after the rejected attempts.
    let resealed = seal::reseal(root, "app/foo.ts", Some("spec/bar.md"), false)
        .expect("the real, in-root edge still reseals fine");
    assert_eq!(resealed.len(), 1);
}

#[test]
fn seal_path_confinement_rejects_escaping_file_arg() {
    let dir = fixture();
    let root = dir.path();
    let outside = root.parent().expect("tempdir has a parent");
    std::fs::write(outside.join("escaper.ts"), "// outside the repo\n").unwrap();

    let err = seal::seal(
        root,
        "../escaper.ts",
        "spec/bar.md",
        Some("cite-seal"),
        None,
    )
    .expect_err("a <file> arg escaping the root must error");
    assert!(
        err.to_string().contains("escapes"),
        "expected a path-confinement error, got: {err}"
    );
}

#[test]
fn reseal_all_stale_supersedes_every_stale_outgoing_edge() {
    let dir = fixture();
    let root = dir.path();
    seal::seal(root, "app/foo.ts", "spec/bar.md", Some("cite-seal"), None).expect("seal 1");
    seal::seal(
        root,
        "app/foo.ts",
        "spec/gov-up.md",
        Some("cite-seal"),
        None,
    )
    .expect("seal 2");
    mutate(root, "spec/bar.md", "drifted one\n");
    mutate(root, "spec/gov-up.md", "drifted two\n");
    let s = walk::status(root).expect("status");
    assert_eq!(s.edges_stale, 2);

    let resealed = seal::reseal(root, "app/foo.ts", None, true).expect("reseal --all-stale");
    assert_eq!(resealed.len(), 2, "both stale outgoing edges resealed");
    let s = walk::status(root).expect("status");
    assert_eq!(s.edges_stale, 0);
    assert_eq!(s.edges_sealed, 2);
}

// ── Governor auto-derivation (spec §3): seal WITHOUT --governor ────────────────────────────

/// A fixture repo with a real cargo workspace at its root (two member crates) plus a doc
/// pair — proves governor auto-derivation end-to-end: `same-cargo-workspace` -> compiler,
/// `doc-doc` -> cite-seal. No `.claude/epr-meta/governors.yaml` here, so this also exercises
/// the fail-soft built-in default order.
fn fixture_with_cargo_workspace() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crate_a\", \"crate_b\"]\n",
    );
    write(
        root,
        "crate_a/Cargo.toml",
        "[package]\nname = \"crate_a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(root, "crate_a/src/lib.rs", "// crate_a lib, version one\n");
    write(
        root,
        "crate_b/Cargo.toml",
        "[package]\nname = \"crate_b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(root, "crate_b/src/lib.rs", "// crate_b lib, version one\n");
    write(root, "docs/downstream.md", "Downstream doc, version one.\n");
    write(root, "docs/upstream.md", "Upstream doc, version one.\n");
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "fixture"]);
    dir
}

#[test]
fn seal_without_governor_on_same_workspace_rs_pair_derives_compiler_and_never_goes_stale() {
    let dir = fixture_with_cargo_workspace();
    let root = dir.path();

    let outcome = seal::seal(root, "crate_a/src/lib.rs", "crate_b/src/lib.rs", None, None)
        .expect("seal derives a governor");
    assert!(outcome.derived, "no --governor given — must derive");
    assert_eq!(outcome.rule.as_deref(), Some("same-cargo-workspace"));
    assert!(
        outcome.governor.starts_with("compiler:"),
        "expected a compiler governor, got {}",
        outcome.governor
    );
    assert!(
        outcome.sealed_cid.is_none(),
        "a derived governed edge seals no CID"
    );

    let s = walk::status(root).expect("status");
    assert_eq!(s.edges_governed, 1, "the derived compiler edge is governed");
    assert_eq!(s.edges_stale, 0);

    // A governed edge never goes stale under upstream mutation.
    mutate(
        root,
        "crate_b/src/lib.rs",
        "// crate_b lib, version TWO — drifted\n",
    );
    let s = walk::status(root).expect("status");
    assert_eq!(s.edges_governed, 1);
    assert_eq!(
        s.edges_stale, 0,
        "a compiler-governed edge never goes stale, derived or explicit"
    );
}

#[test]
fn seal_without_governor_on_two_docs_derives_cite_seal_and_goes_stale_on_mutation() {
    let dir = fixture_with_cargo_workspace();
    let root = dir.path();

    let outcome = seal::seal(root, "docs/downstream.md", "docs/upstream.md", None, None)
        .expect("seal derives cite-seal");
    assert!(outcome.derived, "no --governor given — must derive");
    assert_eq!(outcome.rule.as_deref(), Some("doc-doc"));
    assert_eq!(outcome.governor, "cite-seal");
    assert!(
        outcome.sealed_cid.is_some(),
        "a derived cite-seal edge seals a CID"
    );

    let s = walk::status(root).expect("status");
    assert_eq!(s.edges_sealed, 1);
    assert_eq!(s.edges_stale, 0);

    // Existing behavior preserved: a cite-seal edge goes stale on upstream mutation, derived
    // or explicit alike.
    mutate(
        root,
        "docs/upstream.md",
        "Upstream doc, version TWO — drifted.\n",
    );
    let s = walk::status(root).expect("status");
    assert_eq!(s.edges_stale, 1, "the derived cite-seal edge is now stale");
    assert_eq!(s.edges_sealed, 0);
}

// ── valueflow authoring surface (claim · fulfill --on · context) ─────────────────────────
//
// Fixture shape follows `flow_note.rs`: a synthetic committed repo, `project`ed once so real
// `Intent` records exist to claim. Two plans on purpose — `plans/epic.md` scopes TWO gap items
// so the path arm of `--on` is genuinely ambiguous, and `plans/solo.md` scopes exactly one so
// the unambiguous path arm has something to resolve.

/// A committed, projected repo carrying two plans, their gap items, a brief, a task report, a
/// habit register and a gate manifest.
fn valueflow_fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    write(
        root,
        ".claude/epr-meta/recipes.yaml",
        r#"version: 1
recipes:
  - id: valueflow-fixture
    version: 1
    description: fixture plans and their gap items
    stages:
      - name: plan
        artifactKind: "doc:plan"
        paths:
          - "plans/**/*.md"
      - name: intent
        artifactKind: "gap:item"
        paths:
          - "gap-items/*.json"
    edges: []
"#,
    );
    write(
        root,
        "plans/epic.md",
        "---\nid: fixture-epic\n---\n\n# Fixture epic\n\nTwo tasks, both open.\n",
    );
    write(
        root,
        "gap-items/epic.json",
        r#"{"doc":"plans/epic.md","items":[{"id":"epic#1","state":"OPEN"},{"id":"epic#2","state":"OPEN"}]}"#,
    );
    write(
        root,
        "plans/solo.md",
        "---\nid: fixture-solo\n---\n\n# Fixture solo\n\nOne task.\n",
    );
    write(
        root,
        "gap-items/solo.json",
        r#"{"doc":"plans/solo.md","items":[{"id":"solo#1","state":"OPEN"}]}"#,
    );
    write(
        root,
        "briefs/task-1-brief.md",
        "# Task 1 brief\n\nDo the first thing, and prove it.\n",
    );
    write(
        root,
        "reports/task-1-report.md",
        "# Task 1 report\n\nstatus: DONE. The gate ran and exited zero.\n",
    );
    write(
        root,
        "genesis/manifests/habits.yaml",
        "version: 1\nhabits:\n  - id: dev-system-equilibrium\n    status: red\n    active: false\n    checks:\n      - \"epr flow stocks --check (plans/epic.md)\"\n    refs: []\n",
    );
    write(
        root,
        "build-manifest.json",
        r#"{
  "manifestVersion": "1.0",
  "gate": {
    "projects": {
      "fixture": {
        "dir": "plans",
        "run": { "kind": "root-just", "cargo": { "targetDir": "/tmp/fixture-target", "rustflags": "" } }
      }
    }
  }
}
"#,
    );

    // The covenant is what the WIP fence is scoped to — the document that declares the limit.
    write(
        root,
        ".epr-meta/habits-covenant.md",
        "# Habits covenant\n\nMax 2 active. Flips require evidence.\n",
    );
    write(
        root,
        ".epr-meta/dev-system-equilibrium.habit.md",
        "---\nid: dev-system-equilibrium\nstatus: red\n---\n\nThe evidence ledger.\n",
    );

    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "valueflow fixture"]);

    let recipes = root.join(".claude/epr-meta/recipes.yaml");
    project::project(root, &recipes).expect("project runs");
    dir
}

fn actor(as_ref: &str) -> NoteActor {
    NoteActor {
        as_ref: Some(as_ref.to_string()),
        session: None,
    }
}

fn request<'a>(on: &'a str, actor: &'a NoteActor) -> claim::ClaimRequest<'a> {
    claim::ClaimRequest {
        on,
        brief: None,
        serves: None,
        supersede: false,
        actor,
    }
}

fn records(root: &Path) -> Vec<(cid::Cid, FlowRecord)> {
    SidecarFlowStore::open(root)
        .expect("sidecar opens")
        .records()
        .expect("records read")
}

fn intent_cid_for(root: &Path, gap_id: &str) -> String {
    records(root)
        .into_iter()
        .find_map(|(cid, record)| match record {
            FlowRecord::Intent(intent)
                if intent
                    .resource_spec
                    .classified_as
                    .get(1)
                    .map(String::as_str)
                    == Some(gap_id) =>
            {
                Some(cid.to_string())
            }
            _ => None,
        })
        .expect("the projection minted an intent for this gap id")
}

fn commitments_satisfying(root: &Path, intent_cid: &str) -> usize {
    records(root)
        .into_iter()
        .filter(|(_, record)| match record {
            FlowRecord::Commitment(c) => c.satisfies.iter().any(|s| s.to_string() == intent_cid),
            _ => false,
        })
        .count()
}

#[test]
fn a_claim_mints_exactly_one_commitment_and_the_second_is_refused_until_superseded() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let intent = intent_cid_for(root, "epic#1");
    let implementer = actor("agent:implementer@claude-opus-5");

    let outcome = claim::claim(root, &request("epic#1", &implementer)).expect("claim runs");
    assert!(outcome.appended, "a first claim appends its commitment");
    assert_eq!(outcome.intent, intent);
    assert_eq!(outcome.gap_id, "epic#1");
    assert_eq!(outcome.provider, "agent:implementer@claude-opus-5");
    assert_eq!(
        outcome.steward.as_deref(),
        Some("author@example.test"),
        "an agent-provided claim always carries the tree's signer"
    );
    assert!(outcome.superseded_by.is_empty());
    assert_eq!(
        commitments_satisfying(root, &intent),
        1,
        "exactly one commitment satisfies the intent"
    );

    // Re-running the SAME claim against the SAME tree is a no-op, not a second claimant.
    let again = claim::claim(root, &request("epic#1", &implementer)).expect("re-claim runs");
    assert!(!again.appended, "identity is the atom address");
    assert_eq!(commitments_satisfying(root, &intent), 1);

    // A DIFFERENT actor is refused, and the refusal names the incumbent.
    let reviewer = actor("agent:reviewer@claude-opus-5");
    let err = claim::claim(root, &request("epic#1", &reviewer))
        .expect_err("a standing claim is not silently duplicated");
    assert!(
        err.to_string().contains("agent:implementer@claude-opus-5"),
        "the refusal must name the incumbent; got: {err}"
    );
    assert!(err.to_string().contains("--supersede"));
    assert_eq!(commitments_satisfying(root, &intent), 1, "nothing appended");

    // `--supersede` takes it over and reports what it took.
    let taken = claim::claim(
        root,
        &claim::ClaimRequest {
            supersede: true,
            ..request("epic#1", &reviewer)
        },
    )
    .expect("supersede runs");
    assert!(taken.appended);
    assert_eq!(
        taken.superseded_by.len(),
        1,
        "taking a task over is possible, never silent"
    );
    assert_eq!(commitments_satisfying(root, &intent), 2);
}

#[test]
fn on_resolves_an_address_a_gap_id_and_a_path_and_refuses_an_ambiguous_path() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let implementer = actor("agent:implementer@claude-opus-5");

    // (1) a content address
    let address = intent_cid_for(root, "epic#2");
    let by_address = claim::claim(root, &request(&address, &implementer)).expect("address arm");
    assert_eq!(by_address.gap_id, "epic#2");

    // (2) a gap id — already covered above; (3) a path that scopes exactly one intent
    let by_path = claim::claim(root, &request("plans/solo.md", &implementer)).expect("path arm");
    assert_eq!(by_path.gap_id, "solo#1");

    // A path scoping MORE than one intent refuses, naming every candidate.
    let err = claim::claim(root, &request("plans/epic.md", &implementer))
        .expect_err("guessing which task an author meant is the one thing a claim must not do");
    let message = err.to_string();
    assert!(
        message.contains("epic#1") && message.contains("epic#2"),
        "{message}"
    );

    // A well-formed address that names no record, and a target that is nothing at all.
    assert!(claim::claim(
        root,
        &request(
            "bafyreibzhpdchthmt3zlnhjjjsll6ji6p6fmhqlarcoymuhiprzof75jbq",
            &implementer
        )
    )
    .is_err());
    assert!(claim::claim(root, &request("not-a-thing", &implementer)).is_err());
}

#[test]
fn serves_is_checked_against_the_register_and_a_brief_is_carried_by_address() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let implementer = actor("agent:implementer@claude-opus-5");

    let err = claim::claim(
        root,
        &claim::ClaimRequest {
            serves: Some("no-such-habit"),
            ..request("epic#1", &implementer)
        },
    )
    .expect_err("an unknown habit id is a typo in the accounting");
    assert!(
        err.to_string().contains("genesis/manifests/habits.yaml"),
        "the refusal must name the register file; got: {err}"
    );

    let outcome = claim::claim(
        root,
        &claim::ClaimRequest {
            brief: Some("briefs/task-1-brief.md"),
            serves: Some("dev-system-equilibrium"),
            ..request("epic#1", &implementer)
        },
    )
    .expect("a declared habit and a readable brief are carried");
    assert_eq!(outcome.habit.as_deref(), Some("dev-system-equilibrium"));
    let brief = outcome.brief.expect("the brief is carried by address");

    let slots: Vec<String> = records(root)
        .into_iter()
        .find_map(|(cid, record)| match record {
            FlowRecord::Commitment(c) if cid.to_string() == outcome.commitment_cid => {
                Some(c.resource_spec.classified_as)
            }
            _ => None,
        })
        .expect("the commitment landed");
    assert_eq!(
        slots,
        vec![
            "gap:claimed".to_string(),
            "epic#1".to_string(),
            format!("brief:{brief}"),
            "habit:dev-system-equilibrium".to_string(),
            "steward:author@example.test".to_string(),
        ],
        "slot order is positional: tag, subject, brief, habit, steward LAST"
    );
}

// ── fulfill --on: the task-report arm ────────────────────────────────────────────────────

fn claimed(root: &Path, gap_id: &str, who: &NoteActor) -> String {
    claim::claim(root, &request(gap_id, who))
        .expect("claim runs")
        .commitment_cid
}

fn fulfil_request<'a>(
    on: &'a str,
    status: &'a str,
    who: &'a NoteActor,
) -> fulfill::FulfillOnRequest<'a> {
    fulfill::FulfillOnRequest {
        on,
        report: "reports/task-1-report.md",
        status,
        commits: &[],
        actor: who,
    }
}

fn discharged(root: &Path, commitment_cid: &str) -> bool {
    records(root).into_iter().any(|(_, record)| match record {
        FlowRecord::Event(e) => e.fulfills.iter().any(|c| c.to_string() == commitment_cid),
        _ => false,
    })
}

#[test]
fn a_done_report_discharges_the_commitment_and_the_three_other_statuses_are_refused() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let implementer = actor("agent:implementer@claude-opus-5");
    let commitment = claimed(root, "epic#1", &implementer);

    // The three non-discharging statuses refuse, and each names the record they belong in.
    for status in ["NEEDS_CONTEXT", "BLOCKED", "HOLD"] {
        let err = fulfill::fulfill_on(root, &fulfil_request("epic#1", status, &implementer))
            .expect_err("only DONE and DONE_WITH_CONCERNS discharge");
        assert!(
            err.to_string().contains("--kind observation"),
            "the refusal must name the record these three belong in; got: {err}"
        );
    }
    assert!(
        !discharged(root, &commitment),
        "a non-discharging status must never mark the commitment drained"
    );

    let commits = vec!["825a090df".to_string(), "4425bb6fb".to_string()];
    let outcome = fulfill::fulfill_on(
        root,
        &fulfill::FulfillOnRequest {
            commits: &commits,
            ..fulfil_request("epic#1", "DONE", &implementer)
        },
    )
    .expect("a DONE report discharges");
    assert!(outcome.appended);
    assert!(!outcome.already_fulfilled);
    assert_eq!(outcome.status, "DONE");
    assert_eq!(outcome.commitment, commitment);
    assert_eq!(outcome.commits, commits);
    assert!(discharged(root, &commitment));

    // The event's slots, positionally.
    let slots = records(root)
        .into_iter()
        .find_map(|(cid, record)| match record {
            FlowRecord::Event(e) if Some(cid.to_string()) == outcome.record_cid => {
                Some(e.classified_as)
            }
            _ => None,
        })
        .expect("the fulfilment event landed");
    assert_eq!(slots[0], "report:DONE");
    assert_eq!(slots[1], "epic#1");
    assert_eq!(slots[2], format!("evidence:{}", outcome.evidence));
    assert_eq!(slots[3], "commit:825a090df");
    assert_eq!(slots[4], "commit:4425bb6fb");
    assert_eq!(slots[5], "steward:author@example.test", "steward LAST");

    // A second fulfilment appends nothing and says so.
    let again = fulfill::fulfill_on(root, &fulfil_request("epic#1", "DONE", &implementer))
        .expect("a second fulfilment is a no-op, not an error");
    assert!(again.already_fulfilled);
    assert!(!again.appended);
    assert!(again.record_cid.is_none());
}

#[test]
fn the_status_gate_refuses_by_name_and_points_at_the_observation_note() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let implementer = actor("agent:implementer@claude-opus-5");
    claimed(root, "epic#1", &implementer);

    let err = fulfill::fulfill_on(root, &fulfil_request("epic#1", "BLOCKED", &implementer))
        .expect_err("BLOCKED does not discharge");
    let message = err.to_string();
    assert!(message.contains("BLOCKED"), "{message}");
    assert!(message.contains("--kind observation"), "{message}");

    let unknown = fulfill::fulfill_on(root, &fulfil_request("epic#1", "SHIPPED", &implementer))
        .expect_err("an unknown status is refused, never defaulted");
    assert!(unknown.to_string().contains("DONE_WITH_CONCERNS"));

    // Case and hyphens are forgiven; the vocabulary is not.
    let ok = fulfill::fulfill_on(
        root,
        &fulfil_request("epic#1", "done-with-concerns", &implementer),
    )
    .expect("a forgiving spelling of a legal status still discharges");
    assert_eq!(ok.status, "DONE_WITH_CONCERNS");
}

#[test]
fn a_gap_id_resolves_to_the_newest_active_commitment_and_an_unclaimed_one_refuses() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let implementer = actor("agent:implementer@claude-opus-5");
    let reviewer = actor("agent:reviewer@claude-opus-5");

    claimed(root, "epic#1", &implementer);
    let taken = claim::claim(
        root,
        &claim::ClaimRequest {
            supersede: true,
            ..request("epic#1", &reviewer)
        },
    )
    .expect("supersede runs")
    .commitment_cid;

    let outcome = fulfill::fulfill_on(root, &fulfil_request("epic#1", "DONE", &reviewer))
        .expect("the gap id resolves");
    assert_eq!(
        outcome.commitment, taken,
        "the newest active commitment carrying the id is the live promise"
    );

    let err = fulfill::fulfill_on(root, &fulfil_request("epic#2", "DONE", &implementer))
        .expect_err("an unclaimed gap item has no promise to discharge");
    assert!(err.to_string().contains("epr flow claim"));
}

// ── context: the one screen ──────────────────────────────────────────────────────────────

#[test]
fn context_shows_identity_intents_commitments_and_seals_for_a_path() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let implementer = actor("agent:implementer@claude-opus-5");
    let commitment = claimed(root, "epic#1", &implementer);

    let result = context::context(root, "plans/epic.md").expect("context runs");
    assert_eq!(result.identity.path.as_deref(), Some("plans/epic.md"));
    assert!(result.scope_note.is_none(), "a path computes every section");

    let gap_ids: Vec<&str> = result
        .intents
        .iter()
        .filter_map(|i| i.gap_id.as_deref())
        .collect();
    assert!(
        gap_ids.contains(&"epic#1") && gap_ids.contains(&"epic#2"),
        "{gap_ids:?}"
    );

    let claimed_row = result
        .commitments
        .iter()
        .find(|c| c.cid == commitment)
        .expect("the claim shows as an undischarged commitment");
    assert_eq!(claimed_row.provider, "agent:implementer@claude-opus-5");
    assert_eq!(claimed_row.gap_id.as_deref(), Some("epic#1"));
    assert!(
        claimed_row.latest_event.is_none(),
        "nothing has happened on this promise yet"
    );

    // Sections 5 and 8 are computed for a path, even when they are empty.
    assert!(result.seals.is_some());
    assert!(result.governance.is_some());

    // Discharging it moves the latest-event line, by the one shared ordering rule.
    fulfill::fulfill_on(root, &fulfil_request("epic#1", "DONE", &implementer))
        .expect("a DONE report discharges");
    let after = context::context(root, "plans/epic.md").expect("context runs");
    assert!(
        !after.commitments.iter().any(|c| c.cid == commitment),
        "a discharged commitment leaves the undischarged section"
    );
}

#[test]
fn a_ruling_and_a_verdict_are_readable_in_context_newest_first() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let author = NoteActor::default();

    note::note(
        root,
        "plans/epic.md",
        "ruling",
        "defer the puller to slice two",
        None,
        None,
        &author,
    )
    .expect("a ruling is a note");
    note::note(
        root,
        "plans/epic.md",
        "verdict",
        "the gate line is present and the diff conforms",
        None,
        Some("approved"),
        &actor("agent:reviewer@claude-opus-5"),
    )
    .expect("a verdict is a note");

    let result = context::context(root, "plans/epic.md").expect("context runs");
    assert_eq!(result.notes.len(), 2);
    assert_eq!(result.notes[0].kind, "run:verdict", "newest first");
    assert_eq!(result.notes[0].verdict.as_deref(), Some("approved"));
    assert_eq!(result.notes[0].actor, "agent:reviewer@claude-opus-5");
    assert_eq!(
        result.notes[0].steward.as_deref(),
        Some("author@example.test")
    );
    assert_eq!(result.notes[1].kind, "run:ruling");
    assert_eq!(
        result.notes[1].reason.as_deref(),
        Some("defer the puller to slice two")
    );

    // The window truncates rather than growing without bound.
    assert_eq!(
        context::context_with(root, "plans/epic.md", 1)
            .expect("context runs")
            .notes
            .len(),
        1
    );
}

/// The `--notes N` window: exactly N, taken off the NEWEST end, never a different set.
#[test]
fn the_notes_window_shows_exactly_n_taken_from_the_newest_end() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let author = NoteActor::default();
    for reason in ["first", "second", "third", "fourth"] {
        note::note(
            root,
            "plans/epic.md",
            "observation",
            reason,
            None,
            None,
            &author,
        )
        .expect("note runs");
    }

    let all = context::context_with(root, "plans/epic.md", 10).expect("context runs");
    let reasons: Vec<&str> = all
        .notes
        .iter()
        .filter_map(|n| n.reason.as_deref())
        .collect();
    assert_eq!(
        reasons,
        vec!["fourth", "third", "second", "first"],
        "newest first across the whole set"
    );

    let capped = context::context_with(root, "plans/epic.md", 2).expect("context runs");
    assert_eq!(capped.notes.len(), 2, "exactly N, never N+1");
    assert_eq!(capped.notes[0].reason.as_deref(), Some("fourth"));
    assert_eq!(
        capped.notes[1].reason.as_deref(),
        Some("third"),
        "the window truncates the OLD end, so the newest N survive"
    );

    // N larger than the set is not an error and does not pad.
    assert_eq!(
        context::context_with(root, "plans/epic.md", 99)
            .expect("context runs")
            .notes
            .len(),
        4
    );
    // And zero is a legal, if useless, window rather than a panic.
    assert!(context::context_with(root, "plans/epic.md", 0)
        .expect("context runs")
        .notes
        .is_empty());
}

#[test]
fn a_content_address_target_skips_the_path_only_sections_and_says_why() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let implementer = actor("agent:implementer@claude-opus-5");
    claimed(root, "epic#1", &implementer);

    let address = intent_cid_for(root, "epic#1");
    let scope = records(root)
        .into_iter()
        .find_map(|(_, record)| match record {
            FlowRecord::Intent(i)
                if i.resource_spec.classified_as.get(1).map(String::as_str) == Some("epic#1") =>
            {
                Some(i.in_scope_of.to_string())
            }
            _ => None,
        })
        .expect("the intent declares a scope");

    let result = context::context(root, &scope).expect("context on a bare address runs");
    assert!(result.identity.path.is_none());
    assert!(
        result.scope_note.is_some(),
        "an empty section printed without saying why reads as 'there are none'"
    );
    assert!(result.seals.is_none());
    assert!(result.governance.is_none());
    assert!(
        result.intents.iter().any(|i| i.cid == address),
        "the scoped intents are still derived for a bare address"
    );
    assert_eq!(result.commitments.len(), 1);

    // A well-formed address for something never recorded is refused, not rendered empty.
    assert!(context::context(
        root,
        "bafyreibzhpdchthmt3zlnhjjjsll6ji6p6fmhqlarcoymuhiprzof75jbq"
    )
    .is_err());
}

// ── context sections 6 and 7: the covering habit and the owning gate ─────────────────────

/// A review seat records its verdict ON THE COMMITMENT it reviewed. Notes are resource-scoped,
/// so before this roll-up the plan's own one screen missed exactly that record — the dogfood
/// finding this test exists for.
#[test]
fn a_verdict_on_a_commitment_rolls_up_onto_the_plan_it_is_scoped_to() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let implementer = actor("agent:implementer@claude-opus-5");
    let commitment = claimed(root, "epic#1", &implementer);

    note::note(
        root,
        &commitment,
        "verdict",
        "the gate line is present and the diff conforms",
        None,
        Some("approved"),
        &actor("agent:reviewer@claude-opus-5"),
    )
    .expect("a verdict on the commitment");
    note::note(
        root,
        "plans/epic.md",
        "ruling",
        "accepted, ship it",
        None,
        None,
        &NoteActor::default(),
    )
    .expect("a ruling on the plan itself");

    let result = context::context(root, "plans/epic.md").expect("context runs");
    assert_eq!(
        result.notes.len(),
        2,
        "the plan's own note and its commitment's note are one merged, newest-first set"
    );

    let verdict = result
        .notes
        .iter()
        .find(|n| n.kind == "run:verdict")
        .expect("the review seat's own record reaches the plan's screen");
    assert_eq!(verdict.verdict.as_deref(), Some("approved"));
    assert_eq!(
        verdict.via.as_deref(),
        Some("epic#1"),
        "and says which commitment it came via, so the marker is not a guess"
    );

    let ruling = result
        .notes
        .iter()
        .find(|n| n.kind == "run:ruling")
        .expect("the plan's own note is still there");
    assert!(
        ruling.via.is_none(),
        "a note on the atom itself carries no via marker"
    );

    // The merged set still obeys the window.
    assert_eq!(
        context::context_with(root, "plans/epic.md", 1)
            .expect("context runs")
            .notes
            .len(),
        1
    );
}

#[test]
fn context_names_the_covering_habit_and_the_owning_gate_for_a_path() {
    let dir = valueflow_fixture();
    let root = dir.path();

    let result = context::context(root, "plans/epic.md").expect("context runs");

    let covering = result
        .habits
        .iter()
        .find(|h| h.id == "dev-system-equilibrium")
        .expect("the register's check names this path");
    assert_eq!(covering.status, "red");
    assert!(!covering.active);
    assert_eq!(covering.source, "register");
    assert!(covering.first_check.is_some());

    let gate = result
        .gate
        .expect("build-manifest.json declares a gate for plans/");
    assert_eq!(gate.project.as_deref(), Some("fixture"));
    assert_eq!(gate.command.as_deref(), Some("just gate fixture"));
    assert!(gate.ambiguous.is_empty(), "one project, no tie");
    assert_eq!(gate.target_dir.as_deref(), Some("/tmp/fixture-target"));
    assert_eq!(gate.rustflags.as_deref(), Some(""));

    // A path no gate project declares is an honest absence, never a guessed command.
    let uncovered = context::context(root, "briefs/task-1-brief.md").expect("context runs");
    assert!(uncovered.gate.is_none());
    assert!(uncovered.habit_scope.is_none());
}

#[test]
fn a_habit_atom_renders_as_a_scope_with_the_work_accounted_to_it() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let implementer = actor("agent:implementer@claude-opus-5");

    claim::claim(
        root,
        &claim::ClaimRequest {
            serves: Some("dev-system-equilibrium"),
            ..request("epic#1", &implementer)
        },
    )
    .expect("a claim that serves the habit");

    let result = context::context(root, ".epr-meta/dev-system-equilibrium.habit.md")
        .expect("a habit atom is a legal target");
    let scope = result
        .habit_scope
        .expect("a .habit.md target renders the habit as a scope");
    assert_eq!(scope.id, "dev-system-equilibrium");
    assert_eq!(scope.status, "red");
    assert!(!scope.active);
    assert_eq!(scope.checks.len(), 1);
    assert_eq!(
        scope.open_commitments.len(),
        1,
        "the claim carrying habit:<id> is the work accounted to the standard"
    );
    assert_eq!(scope.open_commitments[0].gap_id.as_deref(), Some("epic#1"));
    assert_eq!(
        scope.open_commitments[0].habit.as_deref(),
        Some("dev-system-equilibrium")
    );
}

// ── ledger: the projection that makes progress.md and a status section generated ─────────

/// A ledger reads FORWARD. Everything else in this family answers "what stands now"; this one
/// answers "what happened, in order" — which is the only shape a pasted status section can have.
#[test]
fn the_ledger_lists_claim_fulfilment_and_notes_in_order_with_the_via_marker() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let implementer = actor("agent:implementer@claude-opus-5");
    let commitment = claimed(root, "epic#1", &implementer);

    let commits = vec!["825a090df".to_string()];
    fulfill::fulfill_on(
        root,
        &fulfill::FulfillOnRequest {
            commits: &commits,
            ..fulfil_request("epic#1", "DONE", &implementer)
        },
    )
    .expect("a DONE report discharges");

    note::note(
        root,
        "plans/epic.md",
        "ruling",
        "accepted, ship it",
        None,
        None,
        &NoteActor::default(),
    )
    .expect("a ruling on the plan");
    note::note(
        root,
        &commitment,
        "verdict",
        "the gate line is present",
        None,
        Some("approved"),
        &actor("agent:reviewer@claude-opus-5"),
    )
    .expect("a verdict on the commitment");

    let result = ledger::ledger(root, "plans/epic.md").expect("ledger runs");
    assert_eq!(result.label, "plans/epic.md");

    let kinds: Vec<&str> = result.entries.iter().map(|e| e.kind.as_str()).collect();
    assert_eq!(
        kinds,
        vec!["claim", "fulfilment", "ruling", "verdict"],
        "oldest first — a ledger reads forward, unlike every other reader in this family"
    );

    let claim_entry = &result.entries[0];
    assert_eq!(claim_entry.actor, "agent:implementer@claude-opus-5");
    assert_eq!(claim_entry.gap_id.as_deref(), Some("epic#1"));

    let fulfilment = &result.entries[1];
    assert_eq!(fulfilment.status.as_deref(), Some("DONE"));
    assert_eq!(fulfilment.commits, vec!["825a090df".to_string()]);
    assert!(
        fulfilment.report.is_some(),
        "the evidence is carried by address"
    );

    let ruling = &result.entries[2];
    assert!(
        ruling.via.is_none(),
        "a note on the atom itself has no via marker"
    );
    assert_eq!(ruling.reason.as_deref(), Some("accepted, ship it"));

    let verdict = &result.entries[3];
    assert_eq!(verdict.verdict.as_deref(), Some("approved"));
    assert_eq!(
        verdict.via.as_deref(),
        Some("epic#1"),
        "a record made on a commitment says which one"
    );
    assert_eq!(verdict.actor, "agent:reviewer@claude-opus-5");

    // Every entry carries the date its record was dated by, never a wall clock.
    assert!(result
        .entries
        .iter()
        .all(|e| e.occurred_at.len() >= 10 && e.occurred_at.starts_with("20")));
}

#[test]
fn a_ledger_on_an_atom_with_no_history_is_empty_rather_than_an_error() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let result = ledger::ledger(root, "plans/solo.md").expect("ledger runs");
    assert!(
        result.entries.is_empty(),
        "no history is an empty ledger, not a refusal"
    );
}

// ── the WIP fence as a bounded commitment ────────────────────────────────────────────────

fn write_register(root: &Path, active: usize) {
    let mut yaml = String::from("version: 1\nhabits:\n");
    for i in 0..3 {
        yaml.push_str(&format!(
            "  - id: habit-{i}\n    status: red\n    active: {}\n    checks: []\n    refs: []\n",
            i < active
        ));
    }
    write(root, "genesis/manifests/habits.yaml", &yaml);
}

fn fence_reading(root: &Path) -> stocks::StockReport {
    let window = stocks::parse_window("2026-08-29..2026-09-05", stocks::Period::Day)
        .expect("a declared window");
    let outcome =
        stocks::stocks(root, &window, &[stocks::StockName::ActiveHabits]).expect("stocks runs");
    outcome.stocks.into_iter().next().expect("one stock")
}

/// The fence is a PROMISE, not a rule in a script: `epr flow project` mints one Active
/// commitment carrying the bound, and the stock is judged against that commitment's own Bound.
#[test]
fn the_wip_fence_is_projected_as_one_bounded_commitment_and_is_idempotent() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let recipes = root.join(".claude/epr-meta/recipes.yaml");

    let fences: Vec<_> = records(root)
        .into_iter()
        .filter_map(|(cid, record)| match record {
            FlowRecord::Commitment(c)
                if c.resource_spec.classified_as.first().map(String::as_str)
                    == Some("register:wip-fence") =>
            {
                Some((cid, c))
            }
            _ => None,
        })
        .collect();
    assert_eq!(fences.len(), 1, "exactly one fence");
    let (_, fence) = &fences[0];
    assert_eq!(fence.provider.0, "tool:habits-register");
    assert_eq!(
        fence.resource_spec.classified_as,
        vec![
            "register:wip-fence".to_string(),
            "habit:attention".to_string()
        ]
    );
    let bound = fence.bound.as_ref().expect("the fence IS its bound");
    assert_eq!(bound.limit, 3.0);
    assert_eq!(bound.unit, "active-habit");
    assert_eq!(bound.threshold_pct, 50.0);
    // A ceiling and a declared source are the v1 DEFAULTS and must be encoded as absent —
    // `Bound::validate` refuses the redundant spellings, because one meaning with two encodings
    // gives one promise two addresses.
    assert!(bound.sense.is_none() && bound.source.is_none());
    assert!(bound.validate().is_ok());

    // Re-projecting appends nothing: identity is the atom address.
    project::project(root, &recipes).expect("re-project");
    assert_eq!(
        records(root)
            .into_iter()
            .filter(|(_, r)| matches!(r, FlowRecord::Commitment(c)
                if c.resource_spec.classified_as.first().map(String::as_str)
                    == Some("register:wip-fence")))
            .count(),
        1,
        "the fence is idempotent by atom CID"
    );
}

#[test]
fn the_active_habits_level_is_judged_against_the_fence_band_and_check_refuses_over_it() {
    let dir = valueflow_fixture();
    let root = dir.path();

    // Zero active: inside the band, nothing signalled.
    write_register(root, 0);
    let report = fence_reading(root);
    assert_eq!(report.verdict.word(), "WITHIN-BOUND");
    assert!(
        report.verdict.is_equilibrium(),
        "--check passes inside the band"
    );
    let bound = report
        .bound
        .as_ref()
        .expect("a bounded stock carries its reading");
    assert_eq!(bound.level, 0.0);
    // The covenant's "max 2 active" against a ceiling that breaches at `stock >= limit`: three
    // is the first FORBIDDEN level, so the limit is 3.0. 50% of 3 puts the band edge at 1.5.
    assert_eq!(bound.limit, 3.0);
    assert_eq!(bound.band_edge, 1.5);
    assert!(bound.signal.is_none());
    assert!(
        bound
            .level_basis
            .contains("max 2 active (fence breaches at 3)"),
        "the basis must state the covenant's number, not only the encoded limit: {}",
        bound.level_basis
    );
    assert!(
        bound.level_basis.contains("projected"),
        "the level is read from the register, not folded from events; the basis must say so: {}",
        bound.level_basis
    );
    assert!(
        report.measures.is_none(),
        "a projected level has no inflow or outflow to report, and inventing a rate is the \
         over-claim this whole vocabulary exists to prevent"
    );

    // One active: below the 1.5 band edge, plainly inside the band.
    write_register(root, 1);
    let report = fence_reading(root);
    assert_eq!(report.verdict.word(), "WITHIN-BOUND");
    assert!(report.verdict.is_equilibrium());
    assert!(report.bound.as_ref().unwrap().signal.is_none());

    // Two active: the covenant's MAXIMUM. Past the 1.5 band edge, so it warns — which is what
    // sitting exactly at the fence should feel like — and it does not refuse.
    write_register(root, 2);
    let report = fence_reading(root);
    assert_eq!(report.verdict.word(), "AT-BAND-EDGE");
    assert!(
        report.verdict.is_equilibrium(),
        "two active is what the covenant permits and must pass --check"
    );
    assert_eq!(
        report.bound.as_ref().unwrap().signal.as_deref(),
        Some("algedonic-approach")
    );

    // Three: the first forbidden level.
    write_register(root, 3);
    let report = fence_reading(root);
    assert_eq!(report.verdict.word(), "OVER-BOUND");
    assert!(
        !report.verdict.is_equilibrium(),
        "--check must exit non-zero at three active"
    );
    assert_eq!(
        report.bound.as_ref().unwrap().signal.as_deref(),
        Some("algedonic-breach")
    );
    assert_eq!(report.bound.as_ref().unwrap().level, 3.0);
}

/// The sidecar is append-only, so a fence whose limit is amended leaves the OLD promise standing
/// beside the new one. The reading must follow the newest, or an amended covenant never takes
/// effect and the superseded number wins forever — which is exactly what happened live the first
/// time this limit moved.
#[test]
fn the_newest_fence_commitment_wins_when_an_amended_limit_supersedes_an_older_one() {
    let dir = valueflow_fixture();
    let root = dir.path();

    // An AMENDED fence with a tighter limit, appended after the one `project` minted.
    let scope = elohim_epr_cli::flow::body_cid_of_file(&root.join(".epr-meta/habits-covenant.md"))
        .expect("the covenant is readable");
    let amended = elohim_epr_rea::Commitment {
        action: elohim_epr_rea::ReaVerb::Produce,
        provider: elohim_epr_rea::AgentRef("tool:habits-register".to_string()),
        receiver: elohim_epr_rea::AgentRef("repo:ethosengine/elohim".to_string()),
        resource_spec: elohim_epr_rea::ResourceSpec {
            classified_as: vec![
                "register:wip-fence".to_string(),
                "habit:attention".to_string(),
            ],
            quantity: None,
        },
        in_scope_of: scope,
        valid_from: None,
        valid_until: None,
        state: elohim_epr_rea::CommitmentState::Active,
        satisfies: Vec::new(),
        bound: Some(elohim_epr_rea::Bound {
            limit: 1.0,
            unit: "active-habit".to_string(),
            threshold_pct: 50.0,
            sense: None,
            source: None,
        }),
    };
    // Appended AFTER the fixture's projected fence, so it is the newest — the shape an amended
    // covenant leaves behind in an append-only log.
    let mut store = SidecarFlowStore::open(root).expect("sidecar opens");
    store
        .append(FlowRecord::Commitment(amended))
        .expect("append the amended fence");

    write_register(root, 2);
    let report = fence_reading(root);
    let bound = report.bound.as_ref().expect("a bounded reading");
    assert_eq!(
        bound.limit, 1.0,
        "the NEWEST fence decides; a superseded limit must not keep deciding forever"
    );
    assert_eq!(report.verdict.word(), "OVER-BOUND");
}

#[test]
fn an_unreadable_register_or_a_missing_fence_refuses_rather_than_reading_zero() {
    let dir = valueflow_fixture();
    let root = dir.path();

    std::fs::remove_file(root.join("genesis/manifests/habits.yaml")).unwrap();
    let report = fence_reading(root);
    assert_eq!(
        report.verdict.word(),
        "REFUSED",
        "an absent register is not a stock of zero"
    );
    assert!(!report.verdict.is_equilibrium());
}

/// `note --on <gap-id>` — the arm `claim` and `fulfill` had and `note` did not, which is why the
/// observer hook could not record a BLOCKED report against the item it was blocked on.
#[test]
fn a_note_resolves_a_gap_id_to_its_commitment_and_rolls_up_onto_the_plan() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let implementer = actor("agent:implementer@claude-opus-5");
    let commitment = claimed(root, "epic#1", &implementer);

    let outcome = note::note(
        root,
        "epic#1",
        "observation",
        "blocked on the cargo lease; nothing was appended",
        None,
        None,
        &implementer,
    )
    .expect("a gap id is a legal --on");
    assert_eq!(
        outcome.resource, commitment,
        "a gap id lands on the promise it names, not on the plan"
    );
    assert_eq!(
        outcome.on, "epic#1",
        "and the label is the id the caller used"
    );

    // And it reaches the plan's one screen, marked with the commitment it came via.
    let result = context::context(root, "plans/epic.md").expect("context runs");
    let observation = result
        .notes
        .iter()
        .find(|n| n.kind == "run:observation")
        .expect("the observation rolls up onto the plan");
    assert_eq!(observation.via.as_deref(), Some("epic#1"));

    // An unclaimed gap item still resolves — to the INTENT, which is the only thing that exists.
    let on_intent = note::note(
        root,
        "epic#2",
        "observation",
        "not claimed yet",
        None,
        None,
        &implementer,
    )
    .expect("an unclaimed gap id resolves to its intent");
    assert_eq!(on_intent.resource, intent_cid_for(root, "epic#2"));

    // A name that is neither refuses, and says so.
    let err = note::note(
        root,
        "epic#99",
        "observation",
        "nothing to attach to",
        None,
        None,
        &implementer,
    )
    .expect_err("an unknown gap id is not a silent no-op");
    assert!(err.to_string().contains("epic#99"), "{err}");
}

/// The two render falsities: a summary that speaks for a stock it cannot measure, and a line of
/// fold accounting printed for a stock that was never folded.
#[test]
fn the_summary_never_says_draining_for_a_stock_that_does_not_drain() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let window = stocks::parse_window("2026-08-29..2026-09-05", stocks::Period::Day)
        .expect("a declared window");

    // At the band edge: admissible, so `--check` still passes — and the summary must NOT claim
    // that every stock is draining, because a projected level never drains at all.
    write_register(root, 2);
    let outcome =
        stocks::stocks(root, &window, &[stocks::StockName::ActiveHabits]).expect("stocks runs");
    assert!(
        outcome.equilibrium,
        "two active is what the covenant permits"
    );
    let line = outcome.equilibrium_line();
    assert!(
        !line.contains("draining"),
        "a projected level does not drain and the summary must not say it does: {line}"
    );
    assert!(line.contains("n/a"), "{line}");
    assert!(line.contains("at its band edge"), "{line}");

    // Over the limit: still no claim of draining, and the band phrase changes.
    write_register(root, 3);
    let outcome =
        stocks::stocks(root, &window, &[stocks::StockName::ActiveHabits]).expect("stocks runs");
    assert!(!outcome.equilibrium);
    let line = outcome.equilibrium_line();
    assert!(!line.contains("draining"), "{line}");
    assert!(line.contains("over its limit"), "{line}");

    // Inside the band, the same discipline holds.
    write_register(root, 0);
    let line = stocks::stocks(root, &window, &[stocks::StockName::ActiveHabits])
        .expect("stocks runs")
        .equilibrium_line();
    assert!(line.contains("inside its band"), "{line}");
    assert!(!line.contains("draining"), "{line}");

    // A FOLDED stock keeps the sentence it has always had, byte for byte.
    let folded = stocks::stocks(root, &window, &[stocks::StockName::Commitments])
        .expect("stocks runs")
        .equilibrium_line();
    assert!(
        folded.starts_with("equilibrium (every stock draining): "),
        "the folded summary is unchanged: {folded}"
    );
}
