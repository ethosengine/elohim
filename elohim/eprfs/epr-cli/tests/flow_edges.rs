//! Seal-aware walk integration — the slice's end-to-end proof (plan Task 3/4). A synthetic
//! git repo: seal an edge → status shows it sealed; mutate the upstream → status shows it
//! stale AND walk-forward from the upstream lists the downstream in `stale_edges`; reseal →
//! stale clears; a held edge stays held (excluded from stale); a compiler-governed edge
//! never goes stale under upstream mutation.

use std::path::Path;
use std::process::Command;

use elohim_epr_cli::flow::{seal, walk};
use tempfile::TempDir;

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
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
        "cite-seal",
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
    seal::seal(root, "app/gov.ts", "spec/gov-up.md", "compiler:tsc", None).expect("seal governed");
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
        "cite-seal",
        None,
    );
    assert!(
        err.is_err(),
        "cite-seal on a missing upstream is dangling-at-birth"
    );
}

#[test]
fn reseal_all_stale_supersedes_every_stale_outgoing_edge() {
    let dir = fixture();
    let root = dir.path();
    seal::seal(root, "app/foo.ts", "spec/bar.md", "cite-seal", None).expect("seal 1");
    seal::seal(root, "app/foo.ts", "spec/gov-up.md", "cite-seal", None).expect("seal 2");
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
