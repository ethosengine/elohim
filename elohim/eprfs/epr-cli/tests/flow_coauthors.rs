//! Plural authorship in the derived valueflow — `Co-Authored-By` trailers lifted onto the
//! `Produce` events `epr flow project` mints.
//!
//! The projection has always credited exactly one person per artifact: the commit's signing
//! author. Every collaborator the author named in the commit message was visible in `git log` and
//! invisible in the value chain, so the repository's own model of how it gets built asserted solo
//! authorship of work that was plural. Git already parses the trailer block; all that was missing
//! was carrying it across.
//!
//! Two invariants are load-bearing here and both are asserted below rather than assumed.
//!
//! **A trailer-less commit must encode exactly as it always did.** `classified_as` is skipped
//! when empty, so a solo produce event's bytes — and therefore its CID — are untouched by this
//! vocabulary existing. The sidecar re-verifies every stored CID on read, so the alternative is
//! not a quiet re-address but an integrity error across the whole append-only log. Pinned by a
//! golden computed against the projection BEFORE trailers were read.
//!
//! **An already-recorded act must not be recorded twice.** Enrichment changes what a re-derived
//! event says, which changes its CID, which a CID-only guard would read as a brand-new
//! production — the same artifact, the same author, the same instant, counted twice by every
//! stock that folds produce against consume. The key guard is what refuses that, and it refuses
//! it without rewriting the older record: history is not retro-attributed.

use std::path::Path;

use elohim_epr_cli::flow::{body_cid, project};
use elohim_epr_rea::{FlowEvent, FlowRecord, FlowStore, ReaVerb, SidecarFlowStore};
use tempfile::TempDir;

/// Every fixture commit is dated identically, so a golden CID is a statement about the ENCODING
/// rather than about when the suite happened to run.
const FIXED_DATE: &str = "2026-08-15T12:00:00+00:00";

/// The produce-event CID for `docs/solo.md` under this fixture, computed against the projection
/// as it stood before `Co-Authored-By` was read at all. It is here to fail loudly if the empty
/// roster ever starts encoding as something other than nothing.
const SOLO_PRODUCE_CID: &str = "bafyreidisfhnlnyo3gkbh7wc27cmli63fychzxori2leyisyb7swoav4du";

const RECIPES: &str = r#"version: 1
recipes:
  - id: elohim-dev-pipeline
    version: 1
    description: fixture pipeline
    stages:
      - name: spec
        artifactKind: "doc:spec"
        paths:
          - "docs/*.md"
    edges: []
"#;

/// A commit message whose trailer block names three collaborators — deliberately NOT in sorted
/// order, so the sorting the slots get is proven rather than coincidental.
const PLURAL_MESSAGE: &str = "author the plural doc

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
Co-Authored-By: ethosengine <noreply@ethosengine.com>
Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
";

fn git(root: &Path, args: &[&str]) {
    // build_command, not a bare Command — git exports GIT_DIR into hook environments and this
    // suite runs under the pre-push hook, where a bare spawn hits the ambient repo.
    let out = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture Author")
        .env("GIT_AUTHOR_EMAIL", "author@example.test")
        .env("GIT_COMMITTER_NAME", "Fixture Author")
        .env("GIT_COMMITTER_EMAIL", "author@example.test")
        .env("GIT_AUTHOR_DATE", FIXED_DATE)
        .env("GIT_COMMITTER_DATE", FIXED_DATE)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?} failed");
}

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn doc(id: &str) -> String {
    format!("---\nid: {id}\ntitle: {id}\n---\n\nBody of {id}.\n")
}

fn recipes(root: &Path) -> std::path::PathBuf {
    root.join(".claude/epr-meta/recipes.yaml")
}

fn sidecar_lines(root: &Path) -> usize {
    std::fs::read_to_string(root.join(".eprfs/status/flows.jsonl"))
        .map(|t| t.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0)
}

/// The single `Produce` event whose resource is the artifact at `rel`. Located by content
/// address, the same way the projection identifies it.
fn produce_event_for(root: &Path, rel: &str) -> FlowEvent {
    let want = body_cid(&std::fs::read_to_string(root.join(rel)).unwrap());
    let mut found: Vec<FlowEvent> = SidecarFlowStore::open(root)
        .unwrap()
        .records()
        .unwrap()
        .into_iter()
        .filter_map(|(_, record)| match record {
            FlowRecord::Event(e) if e.action == ReaVerb::Produce && e.resource == want => Some(e),
            _ => None,
        })
        .collect();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one produce event for {rel}, got {}",
        found.len()
    );
    found.pop().unwrap()
}

fn co_authors_of(event: &FlowEvent) -> Vec<String> {
    event
        .classified_as
        .iter()
        .filter(|slot| slot.starts_with("co-author:"))
        .cloned()
        .collect()
}

/// One commit whose message names three collaborators, then a second that names none — so both
/// arms are observed in one history rather than in two fixtures that might differ elsewhere.
fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    write(root, ".claude/epr-meta/recipes.yaml", RECIPES);
    write(root, "docs/plural.md", &doc("plural"));
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", PLURAL_MESSAGE]);

    write(root, "docs/solo.md", &doc("solo"));
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "author the solo doc"]);
    dir
}

#[test]
fn a_plural_commit_credits_every_collaborator_sorted_and_deduped_while_a_solo_one_credits_none() {
    let dir = fixture();
    let root = dir.path();
    project::project(root, &recipes(root)).expect("project runs");

    let plural = produce_event_for(root, "docs/plural.md");
    assert_eq!(
        co_authors_of(&plural),
        vec![
            "co-author:agent:claude-fable-5".to_string(),
            "co-author:agent:claude-opus-4-6".to_string(),
            "co-author:collective:ethosengine".to_string(),
        ],
        "the roster is sorted, not in the order the author typed it — the slot list is part of \
         the event's address, and two commits naming the same people describe the same \
         collaboration"
    );
    assert_eq!(
        plural.provider.0, "author@example.test",
        "the signing author stays the provider: plurality is additive, never a transfer of who \
         is answerable for the commit"
    );

    let solo = produce_event_for(root, "docs/solo.md");
    assert!(
        solo.classified_as.is_empty(),
        "a commit that named nobody must classify as nothing, not as an empty roster"
    );
}

#[test]
fn a_solo_produce_event_keeps_the_cid_it_had_before_plural_authorship_was_readable() {
    // The golden was computed against the projection BEFORE trailers were read. If reading them
    // had changed the encoding of a commit that has none, every sidecar written under the old
    // code would fail its own CID re-verification on the next read.
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, ".claude/epr-meta/recipes.yaml", RECIPES);
    write(root, "docs/solo.md", &doc("solo"));
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "no trailers here"]);

    project::project(root, &recipes(root)).expect("project runs");

    let log = std::fs::read_to_string(root.join(".eprfs/status/flows.jsonl")).unwrap();
    assert!(
        log.contains(SOLO_PRODUCE_CID),
        "the trailer-less produce event must still address as {SOLO_PRODUCE_CID}; got:\n{log}"
    );
}

#[test]
fn projecting_twice_over_unchanged_history_mints_nothing_the_second_time() {
    let dir = fixture();
    let root = dir.path();
    let path = recipes(root);

    project::project(root, &path).expect("first run");
    let before = sidecar_lines(root);
    let second = project::project(root, &path).expect("second run");

    assert_eq!(second.counts["event"].new, 0);
    assert!(second.counts["event"].present >= 2);
    assert_eq!(
        sidecar_lines(root),
        before,
        "a projection that responds to being run again is not a measure"
    );
}

#[test]
fn an_act_already_recorded_without_its_roster_is_not_appended_a_second_time_with_one() {
    // The fixture is the real migration shape, built out of history rather than by hand-writing
    // a record: project a commit that names nobody, so the sidecar holds the un-enriched event;
    // then amend that commit's MESSAGE to name collaborators. The tree is untouched, so the
    // artifact's body CID, its process, its author and its author date are all unchanged — the
    // act is identical in every respect the key names. Only what the event SAYS about it grew,
    // which is exactly the divergence a CID-only guard cannot see.
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, ".claude/epr-meta/recipes.yaml", RECIPES);
    write(root, "docs/solo.md", &doc("solo"));
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(
        root,
        &["commit", "-q", "-m", "recorded before anyone was credited"],
    );

    let path = recipes(root);
    project::project(root, &path).expect("first run");
    let before_lines = sidecar_lines(root);
    let recorded = produce_event_for(root, "docs/solo.md");
    assert!(recorded.classified_as.is_empty());

    git(
        root,
        &[
            "commit",
            "--amend",
            "-q",
            "-m",
            "recorded before anyone was credited\n\nCo-Authored-By: Claude Fable 5 \
             <noreply@anthropic.com>\nCo-Authored-By: ethosengine <noreply@ethosengine.com>\n",
        ],
    );

    let second = project::project(root, &path).expect("second run");

    assert_eq!(
        second.counts["event"].new, 0,
        "the re-derived produce event addresses differently now that it carries a roster, but it \
         is the same production — appending it would report the artifact as produced twice"
    );
    assert!(second.counts["event"].present >= 1);
    assert_eq!(
        sidecar_lines(root),
        before_lines,
        "not one line was appended"
    );

    // And the older record still says exactly what it said when it was witnessed. Retro-
    // attribution is a migration with a decision behind it, never a side effect of re-projecting.
    let after = produce_event_for(root, "docs/solo.md");
    assert!(
        after.classified_as.is_empty(),
        "history is append-only and is not rewritten in place"
    );
    assert_eq!(after.occurred_at, recorded.occurred_at);
}
