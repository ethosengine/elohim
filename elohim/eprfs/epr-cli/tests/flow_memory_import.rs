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
/// First taken 2026-09-10 from `.claude/scripts/memory-kit/memory-index-projector.py --apply` run
/// over `tests/fixtures/memory-entries/` — 98 indexed rows of 229 entries, 23,993 bytes. It is the
/// harness's non-vacuous half: the oracle proves the RULE, this constant proves the BYTES, and a
/// fixture edit that changes either one has to be re-baselined deliberately.
///
/// RE-BASELINED 2026-09-11 to 24,056 bytes: the generated-file header named the deleted kit script
/// as the way to regenerate the index, so every projection reproduced an instruction nobody could
/// follow. It now names `epr flow memory project --index`. Only that header line moved — the row
/// rule, the row count and every rendered row are unchanged, which is why the oracle leg (when a
/// Python projector is present at all) is expected to differ on the header alone.
const EXPECTED_INDEX_SHA256: &str =
    "841513f54186efc6192c04aaf53b0f4b19c16fb079f27486959700062fb8cd05";
const EXPECTED_INDEX_BYTES: usize = 24_056;
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
    // The human who wrote the bytes is provenance on the source — by display name only, the
    // identity reserve — and neither an author nor an agent.
    assert_eq!(c["imported"]["gitName"], "Fixture Author");
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

// ── the identity reserve (station 5) ─────────────────────────────────────────────────────────

const SECOND_SESSION: &str = "station-five-second";
const SECOND_AUTHOR: &str = "agent:librarian@fixture";

/// Three small importable entries, committed: enough to exercise every leg without the 229-entry
/// corpus's import time.
fn plant_small(root: &Path) -> Vec<&'static str> {
    let names = ["feedback_alpha", "project_beta", "reference_gamma"];
    for name in names {
        write(
            root,
            &format!(".claude/memory/{name}.md"),
            &format!(
                "---\nname: {name}\ntitle: Title {name}\ndescription: The one-line claim of {name}.\nmetadata:\n  type: feedback\n---\n\nbody of {name}\n"
            ),
        );
    }
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "small corpus"]);
    names.to_vec()
}

fn contributions(root: &Path) -> Vec<(String, Vec<u8>)> {
    let dir = root.join(".eprfs/status/memory/contributions");
    let mut out: Vec<(String, Vec<u8>)> = std::fs::read_dir(&dir)
        .expect("contributions")
        .map(|e| {
            let path = e.expect("entry").path();
            (
                path.file_name()
                    .expect("name")
                    .to_string_lossy()
                    .to_string(),
                std::fs::read(&path).expect("read"),
            )
        })
        .filter(|(name, _)| name.ends_with(".json"))
        .collect();
    out.sort();
    out
}

/// Whether `text` carries a `Name <x@y>` shaped identity — the habit's own check-2 pattern.
fn carries_bracketed_email(text: &str) -> bool {
    text.split('<').skip(1).any(|tail| {
        tail.split('>')
            .next()
            .is_some_and(|inner| inner.contains('@'))
    })
}

#[test]
fn imported_git_name_carries_no_email() {
    let dir = repo();
    let root = dir.path();
    plant_small(root);
    import(root, ".claude/memory");
    for (name, bytes) in contributions(root) {
        let text = String::from_utf8(bytes).expect("utf8");
        let c: serde_json::Value = serde_json::from_str(&text).expect("json");
        let imported = &c["imported"];
        assert_eq!(imported["gitName"], "Fixture Author", "{name}");
        assert!(
            imported.get("gitAuthor").is_none(),
            "{name}: the retired gitAuthor field survived"
        );
        let serialized = serde_json::to_string(imported).expect("imported serializes");
        assert!(
            !serialized.contains('@'),
            "{name}: the imported provenance carries an `@`: {serialized}"
        );
        assert!(!carries_bracketed_email(&text), "{name}: {text}");
    }
}

#[test]
fn steward_slot_names_the_collective_never_an_email() {
    let dir = repo();
    let root = dir.path();
    plant_small(root);
    import(root, ".claude/memory");
    let declared: serde_json::Value =
        serde_json::from_str(include_str!("../../../../.epr-meta/collective.json"))
            .expect("collective declaration");
    for (name, bytes) in contributions(root) {
        let c: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(
            c["steward"], declared["steward"],
            "{name}: the steward slot names the collective's declared steward"
        );
        assert!(
            !c["steward"].as_str().expect("steward").contains('@'),
            "{name}: steward carries an `@`"
        );
    }
}

#[test]
fn memory_import_freezes_author() {
    let dir = repo();
    let root = dir.path();
    plant_small(root);
    import(root, ".claude/memory");
    let before = contributions(root);
    let events_before = events(root);

    // A different participant re-imports the SAME bytes under its own session.
    actor::claim(root, SECOND_AUTHOR, SECOND_SESSION).expect("second claim");
    let again = memory::execute_with(
        root,
        "import",
        &Options {
            target: Some(".claude/memory"),
            session: Some(SECOND_SESSION),
            ..Options::default()
        },
    )
    .expect("re-import");
    assert_eq!(again["counts"]["skipped"], before.len());
    assert_eq!(again["counts"]["contributed"], 0);
    assert_eq!(again["counts"]["eventsAppended"], 0);
    assert_eq!(
        contributions(root),
        before,
        "a re-import by another participant rewrote a contribution — author must stay frozen"
    );
    for (name, bytes) in &before {
        let c: serde_json::Value = serde_json::from_slice(bytes).expect("json");
        assert_eq!(c["author"], AUTHOR, "{name}");
    }
    // An identical re-run appends nothing.
    assert_eq!(events(root), events_before);
}

#[test]
fn w5_reimport_in_a_fresh_checkout_keeps_authors() {
    let dir = repo();
    let root = dir.path();
    plant_small(root);
    import(root, ".claude/memory");
    let before = contributions(root);
    assert!(!before.is_empty());

    // A fresh checkout: the tracked contributions arrive, the gitignored flow plane does not.
    std::fs::remove_file(root.join(".eprfs/status/flows.jsonl")).expect("drop the private plane");
    actor::claim(root, SECOND_AUTHOR, SECOND_SESSION).expect("second claim");
    let again = memory::execute_with(
        root,
        "import",
        &Options {
            target: Some(".claude/memory"),
            session: Some(SECOND_SESSION),
            ..Options::default()
        },
    )
    .expect("re-import");
    assert_eq!(again["counts"]["skipped"], before.len(), "{again}");
    assert_eq!(again["counts"]["contributed"], 0, "{again}");
    assert_eq!(
        contributions(root),
        before,
        "the tracked bytes alone freeze the author — no plane, no flip"
    );
    for (name, bytes) in &before {
        let c: serde_json::Value = serde_json::from_slice(bytes).expect("json");
        assert_eq!(c["author"], AUTHOR, "{name}");
    }
}

// ── the migration act ─────────────────────────────────────────────────────────────────────────

const WITNESS: &str = "agent:orchestrator@fixture";
const HUMAN: &str = "human:matthew";
const MIGRATION_SESSION: &str = "identity-reserve-session";

/// Plant the store as it stood BEFORE the identity reserve: contributions carrying
/// `gitAuthor: Name <email>`, each with its attributed contribution act in the flow plane — the
/// exact state 250 live files were in. Built from the typed contribution so the layout is the
/// serializer's own, then the one field renamed back to its retired spelling.
fn plant_legacy(root: &Path) -> Vec<String> {
    use eprfs_agent::memory::{Contribution, FileRef, Imported, Reach, Source};
    let collective_bytes =
        std::fs::read(root.join(".epr-meta/collective.json")).expect("collective");
    let collective = FileRef {
        path: ".epr-meta/collective.json".into(),
        cid: eprfs_core::BlobCid::compute_raw(&collective_bytes).to_string(),
    };
    let mut names = Vec::new();
    for name in plant_small(root) {
        let rel = format!(".claude/memory/{name}.md");
        let bytes = std::fs::read(root.join(&rel)).expect("entry");
        let contribution = Contribution {
            version: 1,
            collective: collective.clone(),
            author: AUTHOR.into(),
            steward: "repo:ethosengine/elohim".into(),
            scope: "repository".into(),
            reach: Reach::Repository,
            concern: name.into(),
            claim: format!("The one-line claim of {name}."),
            // Exactly what `import` writes, so a later re-import of these entries is a re-import of
            // the SAME bytes — the lineage, not a content change, is what the tests exercise.
            uncertainty: vec![format!(
                "Imported verbatim from {rel}; the claim is the entry's own one-line description \
                 and the entry body is the pinned source, not restated here."
            )],
            sources: vec![Source {
                resource: FileRef {
                    path: rel.clone(),
                    cid: eprfs_core::BlobCid::compute_raw(&bytes).to_string(),
                },
                reach: Reach::Repository,
            }],
            supersedes: vec![],
            contradicts: vec![],
            imported: Some(Imported {
                display: format!("Title {name}"),
                file: format!("{name}.md"),
                scope_path: rel.clone(),
                git_name: "Fixture Author <fixture@example.test>".into(),
                entry_type: "feedback".into(),
                indexed: true,
            }),
        };
        let text = format!(
            "{}\n",
            serde_json::to_string_pretty(&contribution).expect("serialize")
        )
        .replace("\"gitName\":", "\"gitAuthor\":");
        let request = format!(".eprfs/status/memory/contributions/{name}.json");
        write(root, &request, &text);
        let raw = eprfs_core::BlobCid::compute_raw(text.as_bytes()).to_string();
        elohim_epr_cli::flow::note::note(
            root,
            &request,
            "observation",
            &format!(
                "Collective contribution {raw} under {}; unreviewed, no acceptance",
                collective.cid
            ),
            None,
            None,
            &elohim_epr_cli::flow::note::NoteActor {
                as_ref: None,
                session: Some(SESSION.into()),
            },
        )
        .expect("the legacy contribution act");
        names.push(format!("{name}.json"));
    }
    names
}

fn epr(root: &Path, key_file: &Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(args)
        .arg("--root")
        .arg(root)
        .env(elohim_epr_cli::device_key::DEVICE_KEY_ENV, key_file)
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env_remove("CLAUDE_SESSION_ID")
        .env_remove("ELOHIM_SESSION_ID")
        .output()
        .expect("epr runs")
}

fn migrate(root: &Path, key_file: &Path) -> serde_json::Value {
    let out = epr(
        root,
        key_file,
        &[
            "flow",
            "memory",
            "migrate-identity-reserve",
            "--session",
            MIGRATION_SESSION,
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "migrate failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("migrate json")
}

/// A witnessed device: the fixture repository gains a roster whose genesis is signed by a temp key.
fn witnessed(root: &Path, keys: &TempDir) -> PathBuf {
    let key_file = keys.path().join("ed25519.seed");
    let key = elohim_epr_cli::device_key::DeviceKey::load_or_generate(&key_file).expect("key");
    actor::witness(
        root,
        HUMAN,
        WITNESS,
        "witness-session",
        "operator of this fixture device",
        false,
        &key,
    )
    .expect("witness");
    key_file
}

fn last_event(root: &Path) -> serde_json::Value {
    let flows = std::fs::read_to_string(root.join(".eprfs/status/flows.jsonl")).expect("flows");
    serde_json::from_str(flows.lines().last().expect("a record")).expect("record json")
}

#[test]
fn migration_rewrites_only_the_identity_field_in_one_attributed_act() {
    let dir = repo();
    let root = dir.path();
    let names = plant_legacy(root);
    let keys = TempDir::new().expect("keys");
    let key_file = witnessed(root, &keys);
    let before = contributions(root);
    let events_before = events(root);

    let outcome = migrate(root, &key_file);
    assert_eq!(outcome["counts"]["migrated"], names.len());
    assert_eq!(outcome["counts"]["refused"], 0);
    assert_eq!(
        events(root),
        events_before + 1,
        "the migration is ONE attributed act, however many files it rewrites"
    );

    // The act is the standing human's, signed through the device's standing claim.
    let record = last_event(root);
    assert_eq!(record["record"]["provider"], HUMAN, "{record}");
    let slots: Vec<String> =
        serde_json::from_value(record["record"]["classifiedAs"].clone()).expect("slots");
    assert!(
        slots.contains(&"source:claim-signed".to_string()),
        "{slots:?}"
    );
    assert_eq!(slots.last().expect("steward"), &format!("steward:{HUMAN}"));
    assert!(
        !slots.iter().any(|s| s.contains("example.test")),
        "no email anywhere in the act: {slots:?}"
    );

    // Every other byte is untouched: the only differing line is the identity field.
    let after = contributions(root);
    assert_eq!(after.len(), before.len());
    for ((name, old), (_, new)) in before.iter().zip(after.iter()) {
        let old = String::from_utf8_lossy(old);
        let new = String::from_utf8_lossy(new);
        let changed: Vec<(&str, &str)> = old
            .lines()
            .zip(new.lines())
            .filter(|(a, b)| a != b)
            .collect();
        assert_eq!(old.lines().count(), new.lines().count(), "{name}");
        assert_eq!(
            changed,
            vec![(
                "    \"gitAuthor\": \"Fixture Author <fixture@example.test>\",",
                "    \"gitName\": \"Fixture Author\","
            )],
            "{name}"
        );
        assert!(!carries_bracketed_email(&new), "{name}");
    }

    // The author's act carries forward onto the migrated bytes: the index still counts every
    // contribution as attributed (the lineage is the migration act, not a re-authoring).
    let projected = project(root, Options::default()).expect("index projects");
    assert_eq!(projected["population"]["unattributed"], 0, "{projected}");
    assert_eq!(projected["entries"], names.len(), "{projected}");
}

#[test]
fn migration_is_idempotent() {
    let dir = repo();
    let root = dir.path();
    plant_legacy(root);
    let keys = TempDir::new().expect("keys");
    let key_file = witnessed(root, &keys);
    migrate(root, &key_file);
    let settled = contributions(root);
    let events_settled = events(root);

    let second = migrate(root, &key_file);
    assert_eq!(second["counts"]["migrated"], 0);
    assert_eq!(
        contributions(root),
        settled,
        "a second migration changed a file"
    );
    assert_eq!(
        events(root),
        events_settled,
        "a second migration appended an act for nothing"
    );

    // And a re-import after the migration by ANOTHER participant leaves the store as it is:
    // the migrated bytes are attributed through the lineage, and the author stays frozen.
    actor::claim(root, SECOND_AUTHOR, SECOND_SESSION).expect("second claim");
    let again = memory::execute_with(
        root,
        "import",
        &Options {
            target: Some(".claude/memory"),
            session: Some(SECOND_SESSION),
            ..Options::default()
        },
    )
    .expect("re-import");
    assert_eq!(again["counts"]["eventsAppended"], 0, "{again}");
    assert_eq!(contributions(root), settled);
}

/// A session with no claim, on a device standing for no one: nobody can be attributed the act, and
/// the note leg would fall back to the commit author's email — so nothing is written.
#[test]
fn migration_refuses_without_any_attributable_actor() {
    let dir = repo();
    let root = dir.path();
    plant_legacy(root);
    let keys = TempDir::new().expect("keys");
    // A key, but no witness: this device stands for no one. And MIGRATION_SESSION claims nothing.
    let key_file = keys.path().join("ed25519.seed");
    elohim_epr_cli::device_key::DeviceKey::load_or_generate(&key_file).expect("key");
    let before = contributions(root);
    let events_before = events(root);
    let out = epr(
        root,
        &key_file,
        &[
            "flow",
            "memory",
            "migrate-identity-reserve",
            "--session",
            MIGRATION_SESSION,
            "--json",
        ],
    );
    assert!(!out.status.success(), "an unattributable migration ran");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("registered no actor claim") && stderr.contains("witnessed"),
        "the refusal names both missing arms: {stderr}"
    );
    assert_eq!(
        contributions(root),
        before,
        "a refused migration wrote a file"
    );
    assert_eq!(
        events(root),
        events_before,
        "a refused migration wrote an act"
    );
    assert!(
        !root.join(".eprfs/status/identity-reserve").exists(),
        "a refused migration pinned a manifest"
    );
}

/// Ruling R-P11: the executing session's claim provides the act; the standing human stewards it.
#[test]
fn migration_provider_is_the_session_claim_when_one_exists() {
    const EXECUTOR: &str = "agent:implementer@fixture";
    let dir = repo();
    let root = dir.path();
    let names = plant_legacy(root);
    let keys = TempDir::new().expect("keys");
    let key_file = witnessed(root, &keys);
    actor::claim(root, EXECUTOR, MIGRATION_SESSION).expect("executor claim");

    let out = epr(
        root,
        &key_file,
        &[
            "flow",
            "memory",
            "migrate-identity-reserve",
            "--session",
            MIGRATION_SESSION,
            "--basis",
            "run on the orchestrator's behalf",
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let outcome: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(outcome["counts"]["migrated"], names.len());

    let record = last_event(root);
    assert_eq!(record["record"]["provider"], EXECUTOR, "{record}");
    let slots: Vec<String> =
        serde_json::from_value(record["record"]["classifiedAs"].clone()).expect("slots");
    assert_eq!(slots.last().expect("steward"), &format!("steward:{HUMAN}"));
    assert!(
        slots.iter().any(|s| s.starts_with("actor-claim:")),
        "the session claim is pinned: {slots:?}"
    );
    assert!(
        !slots.contains(&"source:claim-signed".to_string()),
        "the standing arm did not attribute it: {slots:?}"
    );
    assert!(
        slots.iter().any(|s| s.contains(&format!(
            "executed in session {MIGRATION_SESSION}; basis: run on the orchestrator's behalf"
        ))),
        "{slots:?}"
    );
    assert!(
        !slots.iter().any(|s| s.contains("example.test")),
        "no email anywhere in the act: {slots:?}"
    );
    // The lineage still carries every author's act forward.
    let projected = project(root, Options::default()).expect("index projects");
    assert_eq!(projected["population"]["unattributed"], 0, "{projected}");
}

/// A migration act attributed to the standing human although its executing session held a claim
/// is corrected by a re-run: no byte moves, one corrected act names the one it corrects, and a
/// further re-run appends nothing.
#[test]
fn a_misattributed_migration_act_is_corrected_by_a_re_run() {
    const EXECUTOR: &str = "agent:implementer@fixture";
    let dir = repo();
    let root = dir.path();
    plant_legacy(root);
    let keys = TempDir::new().expect("keys");
    let key_file = witnessed(root, &keys);
    // The session held no claim when it migrated, so the standing human provided the act; once
    // the session is shown to be the executor's, that act names the wrong provider.
    migrate(root, &key_file);
    let wrong = last_event(root);
    assert_eq!(wrong["record"]["provider"], HUMAN);
    actor::claim(root, EXECUTOR, MIGRATION_SESSION).expect("executor claim");
    let settled = contributions(root);
    let events_settled = events(root);

    let corrected = migrate(root, &key_file);
    assert_eq!(corrected["counts"]["migrated"], 0);
    assert_eq!(contributions(root), settled, "a correction moved a byte");
    assert_eq!(events(root), events_settled + 1, "one corrected act");
    let record = last_event(root);
    assert_eq!(record["record"]["provider"], EXECUTOR, "{record}");
    let slots: Vec<String> =
        serde_json::from_value(record["record"]["classifiedAs"].clone()).expect("slots");
    assert_eq!(slots.last().expect("steward"), &format!("steward:{HUMAN}"));
    let wrong_cid = wrong["cid"].as_str().expect("cid");
    assert!(
        slots
            .iter()
            .any(|s| s.contains(&format!("corrects {wrong_cid}"))),
        "{slots:?}"
    );
    assert_eq!(corrected["corrections"][0]["corrects"][0], wrong_cid);

    // Settled: a correctly attributed act stands for the lineage, so nothing more is appended.
    let again = migrate(root, &key_file);
    assert!(again.get("corrections").is_none(), "{again}");
    assert_eq!(events(root), events_settled + 1);
    let projected = project(root, Options::default()).expect("index projects");
    assert_eq!(projected["population"]["unattributed"], 0, "{projected}");
}
