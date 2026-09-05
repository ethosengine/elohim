//! Integration tests over a synthetic tempdir repo for `epr actor` — the in-flight identity
//! plane's foundation leg.
//!
//! Fixture shape is `flow_note.rs`'s: a committed tempdir repo, so `head_commit_provenance` has
//! a real HEAD to date claims against. Two fixtures on purpose — one WITH an agent package on
//! disk and one without — because `definition_cid`'s honest absence is the half most likely to
//! rot into a placeholder, and only a fixture that genuinely lacks the file can pin it.
//!
//! The record-level goldens (CID stability, tamper detection, `current_for` latest-wins) live
//! where the model does, in `elohim/epr-rea/src/actor.rs::tests`. What is asserted here is the
//! CLI's own contract: what it refuses, what it writes, and what its payload says.

#[path = "support/concurrent_actors.rs"]
mod concurrent_actors;

use std::path::Path;

use elohim_epr_cli::actor::{claim, current, ActorError};
use elohim_epr_rea::{ActorStore, SidecarActorStore};
use tempfile::TempDir;

const SESSION: &str = "session-abc123";
const CLAIMED: &str = "agent:scribe@opus-5";

#[test]
fn governance_pins_the_exact_claim_across_workers_and_supersession() {
    use elohim_epr_rea::{ActorClaim, ActorRecord};
    let dir = fixture();
    let root = dir.path();
    let (_, date) = elohim_epr_cli::flow::head_commit_provenance(root).unwrap();
    let expected = |session: &str, identity: &str| {
        ActorRecord::Claim(ActorClaim::new(identity, session, &date, None).unwrap())
            .cid()
            .unwrap()
            .to_string()
    };
    let decision = |session: Option<&str>| {
        let mut args = vec!["--path".into(), "README.md".into()];
        if let Some(session) = session {
            args.extend(["--session".into(), session.into()]);
        }
        elohim_epr_cli::govern::evaluate(root, &args).unwrap()
    };
    claim(root, CLAIMED, "worker-a").unwrap();
    claim(root, CLAIMED, "worker-b").unwrap();
    let first = decision(Some("worker-a"));
    assert_eq!(first["actor"]["claimCid"], expected("worker-a", CLAIMED));
    assert_eq!(
        decision(Some("worker-b"))["actor"]["claimCid"],
        expected("worker-b", CLAIMED)
    );
    assert_ne!(expected("worker-a", CLAIMED), expected("worker-b", CLAIMED));
    claim(root, "agent:scribe@gpt-6", "worker-a").unwrap();
    assert_eq!(
        decision(Some("worker-a"))["actor"]["claimCid"],
        expected("worker-a", "agent:scribe@gpt-6")
    );
    assert_eq!(first["actor"]["claimCid"], expected("worker-a", CLAIMED));
    assert_eq!(
        decision(Some("worker-b"))["actor"]["claimCid"],
        expected("worker-b", CLAIMED)
    );
    assert!(decision(Some("unclaimed"))["actor"]["claimCid"].is_null());
    assert!(decision(None).get("actor").is_none());
    write(root, ".eprfs/status/actors.jsonl", "corrupt\n");
    let corrupt = decision(Some("worker-a"));
    assert_eq!(corrupt["actor"]["source"], "unclaimed");
    assert!(corrupt["actor"]["claimCid"].is_null());
    let missing = fixture();
    let absent = elohim_epr_cli::govern::evaluate(
        missing.path(),
        &[
            "--path".into(),
            "README.md".into(),
            "--session".into(),
            "worker-a".into(),
        ],
    )
    .unwrap();
    assert_eq!(absent["actor"]["source"], "unclaimed");
    assert!(absent["actor"]["claimCid"].is_null());
    assert!(!missing.path().join(".eprfs/status/actors.jsonl").exists());
}

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

/// A committed tempdir repo with NO agent packages — the honest-absence case.
fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "README.md", "fixture\n");
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "fixture"]);
    dir
}

/// The same repo carrying a package for the `scribe` role.
fn fixture_with_package() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "README.md", "fixture\n");
    write(
        root,
        ".epr-meta/elohim/packages/agents/scribe.json",
        "{\n  \"kind\": \"AgentPackage\",\n  \"metadata\": { \"id\": \"scribe\" }\n}\n",
    );
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "fixture"]);
    dir
}

/// An uncommitted tempdir repo — git is initialized but has no HEAD to date a claim against.
fn fixture_without_head() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "README.md", "fixture\n");
    git(dir.path(), &["init", "-q"]);
    dir
}

fn sidecar_lines(root: &Path) -> usize {
    std::fs::read_to_string(root.join(".eprfs/status/actors.jsonl"))
        .map(|t| t.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0)
}

// (a) claim ───────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_claim_appends_one_record_dated_by_the_tree_it_was_made_against() {
    let dir = fixture();
    let root = dir.path();

    let outcome = claim(root, CLAIMED, SESSION).expect("claim runs");

    assert!(outcome.appended);
    assert_eq!(outcome.claimed, CLAIMED);
    assert_eq!(outcome.session, SESSION);
    assert!(outcome.superseded.is_none(), "nothing to supersede");
    assert!(
        !outcome.claimed_at.is_empty(),
        "claimed_at is the git HEAD author date, never empty"
    );
    assert!(
        outcome.claimed_at.starts_with("20"),
        "an RFC3339 date, not a placeholder: {}",
        outcome.claimed_at
    );
    assert_eq!(sidecar_lines(root), 1, "exactly one line appended");

    // `records()` re-verifies every stored CID on read, so a successful read IS the round trip.
    let records = SidecarActorStore::open(root).unwrap().records().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].0.to_string(), outcome.record_cid);

    // The claim lands in its OWN sidecar, never interleaved with the value-chain log.
    assert!(root.join(".eprfs/status/actors.jsonl").exists());
    assert!(!root.join(".eprfs/status/flows.jsonl").exists());
}

#[test]
fn a_claim_without_a_package_on_disk_carries_no_definition_cid() {
    let dir = fixture();
    let outcome = claim(dir.path(), CLAIMED, SESSION).expect("claim runs");
    assert_eq!(
        outcome.definition_cid, None,
        "a missing package is honest absence, never a placeholder address"
    );
}

#[test]
fn a_claim_addresses_the_package_the_role_names() {
    let dir = fixture_with_package();
    let root = dir.path();
    let outcome = claim(root, CLAIMED, SESSION).expect("claim runs");

    let cid = outcome
        .definition_cid
        .as_deref()
        .expect("the scribe package is on disk");
    assert!(cid.starts_with("sha256:"), "got: {cid}");
    assert_eq!(cid.len(), "sha256:".len() + 64, "a full digest: {cid}");

    // It addresses the RAW BYTES of that exact file: edit the package, get a different address.
    write(
        root,
        ".epr-meta/elohim/packages/agents/scribe.json",
        "{\n  \"kind\": \"AgentPackage\",\n  \"metadata\": { \"id\": \"scribe\", \"v\": 2 }\n}\n",
    );
    let rebuilt = claim(root, CLAIMED, SESSION).expect("claim runs");
    assert_ne!(
        rebuilt.definition_cid.as_deref(),
        Some(cid),
        "a rebuilt package is a different definition"
    );
    // …and therefore a different record, which supersedes the first.
    assert!(rebuilt.appended);
    assert_eq!(
        rebuilt.superseded.as_ref().map(|p| p.claimed.as_str()),
        Some(CLAIMED)
    );

    // A role with no package file falls back to honest absence rather than erroring.
    let other = claim(root, "agent:storyteller@opus-5", SESSION).expect("claim runs");
    assert_eq!(other.definition_cid, None);
}

// (b) idempotence + supersession ──────────────────────────────────────────────────────────────

#[test]
fn an_identical_re_claim_is_a_no_op() {
    let dir = fixture();
    let root = dir.path();

    let first = claim(root, CLAIMED, SESSION).expect("first");
    let second = claim(root, CLAIMED, SESSION).expect("second");

    assert!(first.appended);
    assert!(
        !second.appended,
        "re-claiming what is already current is a no-op"
    );
    assert_eq!(first.record_cid, second.record_cid);
    assert!(
        second.superseded.is_none(),
        "a claim does not supersede itself"
    );
    assert_eq!(sidecar_lines(root), 1, "no second line");
}

#[test]
fn a_switched_persona_supersedes_the_prior_claim_without_rewriting_it() {
    let dir = fixture();
    let root = dir.path();

    let first = claim(root, CLAIMED, SESSION).expect("first");
    let second = claim(root, "agent:rust-architect@opus-5", SESSION).expect("second");

    assert!(second.appended);
    let superseded = second.superseded.expect("the prior claim is named");
    assert_eq!(superseded.claimed, CLAIMED);
    assert_eq!(superseded.record_cid, first.record_cid);

    assert_eq!(
        sidecar_lines(root),
        2,
        "supersession appends, never rewrites"
    );
    let now = current(root, SESSION).expect("current runs");
    assert_eq!(
        now.claim.expect("a claim").claimed,
        "agent:rust-architect@opus-5",
        "latest wins"
    );
}

#[test]
fn a_claim_in_another_session_neither_supersedes_nor_is_seen_by_the_first() {
    let dir = fixture();
    let root = dir.path();

    claim(root, CLAIMED, SESSION).expect("first");
    let other = claim(root, "agent:rust-architect@opus-5", "session-other").expect("second");
    assert!(
        other.superseded.is_none(),
        "sessions are separate runs, not a shared stack"
    );

    assert_eq!(
        current(root, SESSION).unwrap().claim.unwrap().claimed,
        CLAIMED
    );
    assert_eq!(
        current(root, "session-other")
            .unwrap()
            .claim
            .unwrap()
            .claimed,
        "agent:rust-architect@opus-5"
    );
}

// (c) refusals ────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_malformed_as_is_refused_and_nothing_is_written() {
    let dir = fixture();
    let root = dir.path();

    for bad in [
        "scribe@opus-5",     // no `agent:` prefix
        "agent:scribe",      // no model half
        "agent:Scribe@opus", // case-only variant of a real role
        "agent:@opus-5",     // empty role
        "agent:../../etc@x", // a path, wearing a role's clothes
    ] {
        let err = claim(root, bad, SESSION).expect_err("a malformed --as must refuse");
        assert!(matches!(err, ActorError::Fabric(_)), "got: {err:?}");
        assert!(
            err.to_string().contains("agent:<role>@<model>"),
            "the refusal must name the legal shape; got: {err}"
        );
    }

    assert!(
        !root.join(".eprfs").exists(),
        "the store must not even be opened — SidecarActorStore::open creates the tree"
    );
}

#[test]
fn a_blank_session_is_refused_before_the_store_is_opened() {
    let dir = fixture();
    let root = dir.path();
    let err = claim(root, CLAIMED, "  \n\t ").expect_err("an unscoped claim must refuse");
    assert!(
        matches!(err, ActorError::InvalidArguments(_)),
        "got: {err:?}"
    );
    assert!(!root.join(".eprfs").exists());
}

#[test]
fn a_tree_with_no_head_cannot_date_a_claim_and_is_refused() {
    // The load-bearing refusal: a claim dated by wall clock would be undated in every fold and
    // unorderable against its own supersessions, so absence of history is a refusal, not a
    // fallback.
    let dir = fixture_without_head();
    let root = dir.path();

    let err = claim(root, CLAIMED, SESSION).expect_err("no HEAD must refuse");
    assert!(
        matches!(err, ActorError::InvalidArguments(_)),
        "got: {err:?}"
    );
    assert!(
        err.to_string().contains("wall clock"),
        "the refusal must say why; got: {err}"
    );
    assert_eq!(sidecar_lines(root), 0, "nothing written");
}

// (d) current ─────────────────────────────────────────────────────────────────────────────────

#[test]
fn current_answers_with_a_null_claim_rather_than_failing_when_nobody_registered() {
    let dir = fixture();
    let root = dir.path();

    // No claim has ever been made, and no sidecar exists yet — still an ANSWER, not a failure.
    let outcome = current(root, SESSION).expect("an unregistered session is exit-0");
    assert_eq!(outcome.session, SESSION);
    assert!(outcome.claim.is_none());

    let json: serde_json::Value = serde_json::to_value(&outcome).unwrap();
    assert!(
        json.get("claim").is_some_and(serde_json::Value::is_null),
        "the key must be present and null, never omitted: {json}"
    );
}

#[test]
fn current_reads_back_every_field_of_the_registered_claim() {
    let dir = fixture_with_package();
    let root = dir.path();
    let claimed = claim(root, CLAIMED, SESSION).expect("claim runs");

    let outcome = current(root, SESSION).expect("current runs");
    let found = outcome.claim.expect("a claim");
    assert_eq!(found.claimed, CLAIMED);
    assert_eq!(found.session, SESSION);
    assert_eq!(found.claimed_at, claimed.claimed_at);
    assert_eq!(found.definition_cid, claimed.definition_cid);
    assert_eq!(found.record_cid, claimed.record_cid);

    // A session nobody claimed under stays absent even though the store is readable.
    assert!(current(root, "session-never").unwrap().claim.is_none());
}

#[test]
fn the_outcomes_serialize_camel_cased_for_json_consumers() {
    let dir = fixture_with_package();
    let root = dir.path();
    let outcome = claim(root, CLAIMED, SESSION).expect("claim runs");

    let json = serde_json::to_string(&outcome).expect("serializes");
    assert!(json.contains("\"claimedAt\""), "got: {json}");
    assert!(json.contains("\"definitionCid\""), "got: {json}");
    assert!(json.contains("\"recordCid\""), "got: {json}");
    assert!(json.contains(&outcome.record_cid));

    let json = serde_json::to_string(&current(root, SESSION).unwrap()).expect("serializes");
    assert!(json.contains("\"claimedAt\""), "got: {json}");
}
