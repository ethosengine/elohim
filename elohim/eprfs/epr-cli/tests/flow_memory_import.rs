//! Station four: authored memory entries become collective contributions, and `MEMORY.md` becomes
//! a projection of them.
//!
//! Every assertion here is on SHIPPED OUTPUT — the bytes a caller receives, the events the sidecar
//! holds, the files on disk — never on an intermediate the production path does not itself use.
//! Two of them are deliberately adversarial:
//!
//! * The privacy-line test is **mutation-checked**: the same bytes that are refused under a private
//!   path are imported under an allowed one, so deleting the gate turns the test red rather than
//!   leaving it passing for an unrelated reason (missing frontmatter, an unreadable file, an empty
//!   directory). A refusal test that cannot tell WHY it refused proves nothing.
//! * The index-parity test pins the expected bytes by digest **as well as** running the Python
//!   oracle, so a machine without `python3` still fails on drift instead of silently skipping.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use elohim_epr_cli::{
    actor,
    flow::memory::{self, entries, Options},
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

/// The digest of the index this fixture corpus projects.
///
/// Taken 2026-09-10 from `.claude/scripts/memory-kit/memory-index-projector.py --apply` run over
/// `tests/fixtures/memory-entries/` — 98 indexed rows of 229 entries, 23,993 bytes. It is the
/// harness's non-vacuous half: the oracle proves the RULE, this constant proves the BYTES, and a
/// fixture edit that changes either one has to be re-baselined deliberately.
const EXPECTED_INDEX_SHA256: &str =
    "8ee2e07eac63f45594cf18b6ab5f996594c5c25b0e511fd20bba198a627a83dd";
const EXPECTED_INDEX_BYTES: usize = 23_993;
const EXPECTED_INDEX_ROWS: usize = 98;

/// The fixture corpus is the live corpus's frontmatter, snapshotted. 226 of its 229 entries carry
/// the required frontmatter; three declare no kind in either dialect and are refused by name.
const FIXTURE_ENTRIES: usize = 229;
const FIXTURE_IMPORTABLE: usize = 226;
const FIXTURE_REFUSED: usize = 3;

const SESSION: &str = "station-four";
const AUTHOR: &str = "agent:investigator@fixture";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root")
}

fn git(root: &Path, args: &[&str]) {
    let out = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture Author")
        .env("GIT_COMMITTER_NAME", "Fixture Author")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A synthetic repository carrying the REAL collective declaration and the REAL measure registry.
///
/// Both are `include_str!`ed rather than hand-written: the source-rule that admits `.claude/memory`
/// and the watermark that refuses an oversized index are the very declarations under test, and a
/// local copy of them would let the registry drift while the tests stayed green.
fn repo() -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    write(
        root,
        ".epr-meta/collective.json",
        include_str!("../../../../.epr-meta/collective.json"),
    );
    write(
        root,
        ".claude/epr-meta/measures.yaml",
        include_str!("../../../../.claude/epr-meta/measures.yaml"),
    );
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "fixture"]);
    actor::claim(root, AUTHOR, SESSION).expect("actor claim");
    dir
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, text).expect("write");
}

/// Copy the committed frontmatter snapshot of the live memory corpus into `<root>/.claude/memory`.
fn plant_corpus(root: &Path) -> usize {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/memory-entries");
    let mut planted = 0;
    for item in std::fs::read_dir(&src).expect("fixture corpus") {
        let path = item.expect("entry").path();
        if path.extension().is_some_and(|e| e == "md") {
            let name = path.file_name().expect("name");
            std::fs::create_dir_all(root.join(".claude/memory")).expect("mkdir");
            std::fs::copy(&path, root.join(".claude/memory").join(name)).expect("copy");
            planted += 1;
        }
    }
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "corpus"]);
    planted
}

fn import(root: &Path, dir: &str) -> serde_json::Value {
    memory::execute_with(
        root,
        "import",
        &Options {
            target: Some(dir),
            session: Some(SESSION),
            ..Options::default()
        },
    )
    .expect("import")
}

fn project(root: &Path, opts: Options) -> elohim_epr_cli::flow::FlowResult<serde_json::Value> {
    memory::execute_with(
        root,
        "project",
        &Options {
            index: true,
            session: Some(SESSION),
            ..opts
        },
    )
}

fn events(root: &Path) -> usize {
    std::fs::read_to_string(root.join(".eprfs/status/flows.jsonl"))
        .map(|t| t.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0)
}

fn folds(root: &Path, measure: &str) -> usize {
    std::fs::read_to_string(root.join(".eprfs/status/flows.jsonl"))
        .map(|t| {
            t.lines()
                .filter(|l| l.contains(&format!("measure:{measure}")))
                .count()
        })
        .unwrap_or(0)
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

// ── import ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn importing_the_corpus_twice_appends_exactly_one_round_of_events() {
    let dir = repo();
    let root = dir.path();
    assert_eq!(plant_corpus(root), FIXTURE_ENTRIES);

    let first = import(root, ".claude/memory");
    assert_eq!(first["counts"]["entries"], FIXTURE_ENTRIES);
    assert_eq!(first["counts"]["contributed"], FIXTURE_IMPORTABLE);
    assert_eq!(first["counts"]["refused"], FIXTURE_REFUSED);
    assert_eq!(first["counts"]["eventsAppended"], FIXTURE_IMPORTABLE);
    // Never widened: the contribution takes the NARROWER of the two policies that govern the bytes
    // it pins and the bytes that record it. The RELATION is asserted, not a declared value — the
    // collective owns its source rules and may legitimately move one (it did: `.eprfs/status/memory`
    // was raised to `repository` after this leg landed), while this leg owns only the refusal to
    // exceed whatever they say.
    let rung = |value: &serde_json::Value| match value.as_str().expect("a declared reach") {
        "private" => 0,
        "workspace" => 1,
        "repository" => 2,
        other => panic!("undeclared reach `{other}`"),
    };
    assert_eq!(
        rung(&first["declaredReach"]["effective"]),
        rung(&first["declaredReach"]["source"]).min(rung(&first["declaredReach"]["request"])),
        "effective reach must be the minimum of the two declared policies"
    );

    let after_first = events(root);
    let requests: BTreeSet<String> =
        std::fs::read_dir(root.join(".eprfs/status/memory/contributions"))
            .expect("contributions written")
            .map(|e| e.expect("entry").file_name().to_string_lossy().to_string())
            .collect();
    assert_eq!(requests.len(), FIXTURE_IMPORTABLE);
    let sample =
        root.join(".eprfs/status/memory/contributions/project_holochain_evolution_epic.json");
    let sample_bytes = std::fs::read(&sample).expect("a request file");

    let second = import(root, ".claude/memory");
    assert_eq!(second["counts"]["skipped"], FIXTURE_IMPORTABLE);
    assert_eq!(second["counts"]["contributed"], 0);
    assert_eq!(second["counts"]["eventsAppended"], 0);
    assert_eq!(
        events(root),
        after_first,
        "a second import appended events — idempotence is by content, not by luck"
    );
    assert_eq!(
        std::fs::read(&sample).expect("request survives"),
        sample_bytes,
        "a second import rewrote a request file, changing the CID the record names"
    );
}

#[test]
fn an_imported_contribution_carries_provenance_without_minting_a_persona() {
    let dir = repo();
    let root = dir.path();
    plant_corpus(root);
    import(root, ".claude/memory");

    let text = std::fs::read_to_string(
        root.join(".eprfs/status/memory/contributions/project_holochain_evolution_epic.json"),
    )
    .expect("request");
    let c: serde_json::Value = serde_json::from_str(&text).expect("contribution json");
    // The ACTING participant is the registered agent claim — never a persona invented for a human.
    assert_eq!(c["author"], AUTHOR);
    assert_eq!(c["steward"], "repo:ethosengine/elohim");
    // The human who wrote the bytes is provenance on the source, and the git-signing human is the
    // note's steward slot; neither is an author and neither is an agent.
    assert_eq!(
        c["imported"]["gitAuthor"],
        "Fixture Author <fixture@example.test>"
    );
    assert_eq!(c["imported"]["file"], "project_holochain_evolution_epic.md");
    assert_eq!(
        c["imported"]["scopePath"],
        ".claude/memory/project_holochain_evolution_epic.md"
    );
    assert!(!c["imported"]["display"]
        .as_str()
        .expect("display")
        .is_empty());
    // The entry's original bytes are pinned by RAW cid as the contribution's source.
    let raw = eprfs_core::BlobCid::compute_raw(
        &std::fs::read(root.join(".claude/memory/project_holochain_evolution_epic.md"))
            .expect("entry"),
    )
    .to_string();
    assert_eq!(c["sources"][0]["resource"]["cid"], raw);
}

#[test]
fn a_refusal_reports_whether_it_would_cost_the_index_a_row() {
    let dir = repo();
    let root = dir.path();
    // Two entries with the same defect — no declared kind — differing only in whether the index
    // carries them today. The report has to tell those two apart, because one is free to refuse
    // and the other silently drops a row every session used to load.
    write(
        root,
        ".claude/memory/kindless-indexed.md",
        "---\nname: kindless-indexed\ntitle: Indexed\ndescription: In the index today.\nmetadata:\n  node_type: memory\n---\nbody\n",
    );
    write(
        root,
        ".claude/memory/kindless-opted-out.md",
        "---\nindex: false\nname: kindless-opted-out\ntitle: Opted out\ndescription: Not in the index.\nmetadata:\n  node_type: memory\n---\nbody\n",
    );
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "kindless"]);

    let outcome = import(root, ".claude/memory");
    assert_eq!(outcome["counts"]["refused"], 2);
    assert_eq!(
        outcome["counts"]["refusedIndexedToday"], 1,
        "the blast radius of a refusal is reported, not left to be inferred"
    );
    let by_entry: Vec<(&str, Option<bool>)> = outcome["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .map(|e| {
            (
                e["entry"].as_str().expect("path"),
                e["indexedToday"].as_bool(),
            )
        })
        .collect();
    assert!(by_entry.contains(&(".claude/memory/kindless-indexed.md", Some(true))));
    assert!(by_entry.contains(&(".claude/memory/kindless-opted-out.md", Some(false))));
}

#[test]
fn a_dry_run_plans_and_writes_nothing() {
    let dir = repo();
    let root = dir.path();
    plant_corpus(root);
    let before = events(root);
    let planned = memory::execute_with(
        root,
        "import",
        &Options {
            target: Some(".claude/memory"),
            session: Some(SESSION),
            dry_run: true,
            ..Options::default()
        },
    )
    .expect("dry run");
    assert_eq!(planned["counts"]["contributed"], FIXTURE_IMPORTABLE);
    assert_eq!(planned["counts"]["eventsAppended"], 0);
    assert_eq!(events(root), before);
    assert!(!root.join(".eprfs/status/memory/contributions").exists());
}

// ── the privacy line ──────────────────────────────────────────────────────────────────────────

#[test]
fn a_private_store_is_refused_and_the_same_bytes_are_importable_elsewhere() {
    let dir = repo();
    let root = dir.path();
    plant_corpus(root);

    // Identical bytes, two homes. The ONLY difference is the path.
    let entry = "---\nname: private-working-note\ntitle: A working note\ndescription: Bytes that would import fine anywhere the privacy line does not name.\nmetadata:\n  type: feedback\n---\n\nbody\n";
    write(root, ".claude/memory-kit/private-working-note.md", entry);
    write(root, ".claude/memory/private-working-note.md", entry);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "twins"]);

    let before = events(root);
    let refusal = memory::execute_with(
        root,
        "import",
        &Options {
            target: Some(".claude/memory-kit"),
            session: Some(SESSION),
            ..Options::default()
        },
    )
    .expect_err("a private store must be refused, not imported");
    let message = refusal.to_string();
    assert!(
        message.contains("private store") && message.contains("private report tier"),
        "the refusal must NAME the rule that refused: {message}"
    );
    assert_eq!(
        events(root),
        before,
        "a refused import still appended to the flow plane"
    );
    assert!(!root.join(".eprfs/status/memory/contributions").exists());

    // Mutation check: with the gate removed, this half would still pass — so the pair is the test.
    let allowed = import(root, ".claude/memory");
    assert_eq!(allowed["counts"]["contributed"], FIXTURE_IMPORTABLE + 1);
    assert!(root
        .join(".eprfs/status/memory/contributions/private-working-note.json")
        .exists());
}

#[test]
fn a_private_file_inside_an_allowed_directory_is_refused_by_name() {
    let dir = repo();
    let root = dir.path();
    let entry = "---\nname: n\ntitle: T\ndescription: D\nmetadata:\n  type: feedback\n---\nbody\n";
    write(root, ".claude/memory/session.continuation.md", entry);
    // The twin is a DOCUMENT about continuations, which is fruit, not a private record.
    write(root, ".claude/memory/session-continuation-design.md", entry);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "entries"]);

    let outcome = import(root, ".claude/memory");
    let states: Vec<(&str, &str)> = outcome["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .map(|e| {
            (
                e["entry"].as_str().expect("entry path"),
                e["state"].as_str().expect("state"),
            )
        })
        .collect();
    assert!(states.contains(&(".claude/memory/session.continuation.md", "refused")));
    assert!(states.contains(&(
        ".claude/memory/session-continuation-design.md",
        "contributed"
    )));
    let reason = outcome["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .find(|e| e["state"] == "refused")
        .expect("a refusal")["reason"]
        .as_str()
        .expect("reason")
        .to_string();
    assert!(reason.contains("continuation"), "unnamed refusal: {reason}");
}

// ── the projected index ───────────────────────────────────────────────────────────────────────

#[test]
fn the_projected_index_is_byte_identical_to_the_python_projector() {
    let dir = repo();
    let root = dir.path();
    plant_corpus(root);
    import(root, ".claude/memory");

    let projected = project(
        root,
        Options {
            out: Some(".claude/memory/MEMORY.md"),
            ..Options::default()
        },
    )
    .expect("index projects");
    assert_eq!(projected["entries"], EXPECTED_INDEX_ROWS);
    assert_eq!(projected["bytes"], EXPECTED_INDEX_BYTES);

    let native = std::fs::read(root.join(".claude/memory/MEMORY.md")).expect("index written");
    // The non-vacuous half: the bytes are pinned whether or not the oracle can be run here.
    assert_eq!(
        sha256(&native),
        EXPECTED_INDEX_SHA256,
        "projected index bytes drifted from the recorded baseline"
    );

    // The oracle half: the RULE, re-derived from frontmatter by the script this leg replaces.
    let script = repo_root().join(".claude/scripts/memory-kit/memory-index-projector.py");
    if !script.is_file() {
        eprintln!("skipped oracle leg: {} is absent (the kit has been cleared); the digest pin above still holds", script.display());
        return;
    }
    let oracle = tempfile::tempdir().expect("oracle root");
    let oracle_dir = oracle.path().join(".claude/memory");
    std::fs::create_dir_all(&oracle_dir).expect("mkdir");
    for item in std::fs::read_dir(root.join(".claude/memory")).expect("corpus") {
        let path = item.expect("entry").path();
        if path.file_name().is_some_and(|n| n == "MEMORY.md") {
            continue;
        }
        std::fs::copy(&path, oracle_dir.join(path.file_name().expect("name"))).expect("copy");
    }
    let run = std::process::Command::new("python3")
        .arg(&script)
        .arg("--apply")
        .arg("--quiet")
        .env("MEMORY_INDEX_ROOT", oracle.path())
        .output();
    let Ok(run) = run else {
        eprintln!("skipped oracle leg: python3 is not on PATH; the digest pin above still holds");
        return;
    };
    assert!(
        run.status.success(),
        "oracle failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let expected = std::fs::read(oracle_dir.join("MEMORY.md")).expect("oracle output");
    assert_eq!(
        String::from_utf8_lossy(&native),
        String::from_utf8_lossy(&expected),
        "native projection diverged from the frontmatter projector"
    );
}

#[test]
fn without_out_the_projection_writes_no_index() {
    let dir = repo();
    let root = dir.path();
    plant_corpus(root);
    import(root, ".claude/memory");
    let reported = project(root, Options::default()).expect("index projects");
    assert_eq!(reported["bytes"], EXPECTED_INDEX_BYTES);
    assert_eq!(reported["wrote"], serde_json::Value::Null);
    assert!(!root.join(".claude/memory/MEMORY.md").exists());
}

// ── the declared budget ───────────────────────────────────────────────────────────────────────

/// Plant importable ASCII entries whose projected index is EXACTLY `target` bytes.
///
/// Every row is `- [<title>](<file>) — <desc>\n`: 12 bytes of fixed shape plus three ASCII fields,
/// so the arithmetic is exact rather than tuned. Descriptions stay at or under the 200-character
/// render bound, because a longer one would be truncated and the row would not be the size it was
/// asked for.
fn corpus_of_exactly(root: &Path, target: usize) -> usize {
    const TITLE: &str = "Budget T"; // 8 ASCII bytes
    let mut remaining = target - entries::HEADER.len();
    let mut planted = 0usize;
    while remaining > 0 {
        let file = format!("e{planted:03}.md"); // 7 ASCII bytes
        let fixed = 12 + TITLE.len() + file.len();
        assert!(remaining > fixed, "target leaves an unfillable remainder");
        let desc = if remaining <= fixed + entries::DESC_MAX {
            remaining - fixed
        } else if remaining - (fixed + entries::DESC_MAX) <= fixed {
            // Taking a full-width row here would strand a remainder too small to be a row of its
            // own; shorten this one so the LAST row still clears the fixed overhead.
            entries::DESC_MAX - (fixed + 1)
        } else {
            entries::DESC_MAX
        };
        write(
            root,
            &format!(".claude/memory/{file}"),
            &format!(
                "---\nname: {}\ntitle: {TITLE}\ndescription: {}\nmetadata:\n  type: project\n---\n\nbody\n",
                file.trim_end_matches(".md"),
                "x".repeat(desc)
            ),
        );
        remaining -= fixed + desc;
        planted += 1;
    }
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "budget corpus"]);
    planted
}

#[test]
fn the_declared_budget_refuses_one_byte_over_the_watermark_and_not_at_it() {
    let at = repo();
    corpus_of_exactly(at.path(), 24_000);
    import(at.path(), ".claude/memory");
    let ok = project(
        at.path(),
        Options {
            budget: Some("memory-index-bytes@1"),
            out: Some(".claude/memory/MEMORY.md"),
            ..Options::default()
        },
    )
    .expect("exactly at the watermark is within it");
    assert_eq!(ok["bytes"], 24_000);
    // The watermark arrives as the registry declared it — a float, because a bound is a magnitude.
    assert_eq!(ok["budget"]["hard"].as_f64(), Some(24_000.0));
    assert_eq!(ok["budget"]["state"], "over-soft");
    assert!(at.path().join(".claude/memory/MEMORY.md").is_file());

    let over = repo();
    corpus_of_exactly(over.path(), 24_001);
    import(over.path(), ".claude/memory");
    let refusal = project(
        over.path(),
        Options {
            budget: Some("memory-index-bytes@1"),
            out: Some(".claude/memory/MEMORY.md"),
            ..Options::default()
        },
    )
    .expect_err("one byte over the hard watermark must refuse");
    let message = refusal.to_string();
    assert!(
        message.contains("24001") && message.contains("memory-index-bytes-ceiling@1"),
        "the refusal must quote the observation and the bound that refused: {message}"
    );
    assert!(
        !over.path().join(".claude/memory/MEMORY.md").exists(),
        "a refused projection still wrote the index"
    );
}

#[test]
fn an_undeclared_budget_pin_is_refused_naming_the_registry() {
    let dir = repo();
    let root = dir.path();
    plant_corpus(root);
    import(root, ".claude/memory");
    let refusal = project(
        root,
        Options {
            budget: Some("no-such-measure@1"),
            ..Options::default()
        },
    )
    .expect_err("an undeclared pin is not a budget");
    assert!(refusal.to_string().contains("measures.yaml"));
}

// ── the drift fold ────────────────────────────────────────────────────────────────────────────

#[test]
fn the_unloaded_fold_is_appended_once_per_distinct_set() {
    let dir = repo();
    let root = dir.path();
    // Comfortably past the declared 24,000-byte cap, so rows fall off the end.
    corpus_of_exactly(root, 26_000);
    import(root, ".claude/memory");

    let first = project(root, Options::default()).expect("index projects");
    let unloaded = first["indexUnloaded"].as_u64().expect("unloaded count");
    assert!(unloaded > 0, "a 26,000-byte index must have unloaded rows");
    assert_eq!(folds(root, "memory-index-drift@1"), 1);

    // The same drift, asked twice, is one record.
    let again = project(root, Options::default()).expect("index projects");
    assert_eq!(again["indexUnloaded"], first["indexUnloaded"]);
    assert_eq!(
        folds(root, "memory-index-drift@1"),
        1,
        "an unchanged unloaded set minted a second fold"
    );

    // A DIFFERENT unloaded set is a different observation, even at the same count.
    write(
        root,
        ".claude/memory/zzz-tail.md",
        "---\nname: zzz-tail\ntitle: Tail\ndescription: One more row past the cap.\nmetadata:\n  type: project\n---\n\nbody\n",
    );
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "tail"]);
    import(root, ".claude/memory");
    let changed = project(root, Options::default()).expect("index projects");
    assert!(changed["indexUnloaded"].as_u64().expect("count") >= unloaded);
    assert_eq!(
        folds(root, "memory-index-drift@1"),
        2,
        "a changed unloaded set must mint its own fold"
    );
}

#[test]
fn a_drained_index_witnesses_its_zero_exactly_once() {
    let dir = repo();
    let root = dir.path();
    corpus_of_exactly(root, 2_000);
    import(root, ".claude/memory");
    let first = project(root, Options::default()).expect("index projects");
    assert_eq!(first["indexUnloaded"], 0);
    project(root, Options::default()).expect("index projects");
    assert_eq!(
        folds(root, "memory-index-drift@1"),
        1,
        "a witnessed zero is a record, and it is one record"
    );
}
