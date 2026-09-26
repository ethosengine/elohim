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
    // The collective's Stewards on record, verbatim: a declaration names none of its own.
    write(
        root,
        ".eprfs/status/affiliations.jsonl",
        include_str!("../../../../.eprfs/status/affiliations.jsonl"),
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
    assert!(
        declared.get("steward").is_none(),
        "a declaration names no steward; Stewards are affiliation records"
    );
    for (name, bytes) in contributions(root) {
        let c: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(
            c["steward"], "repo:ethosengine/elohim",
            "{name}: the steward slot names the root collective's default acts_for"
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

/// An amended declaration re-pins the collective's CID. A re-import after the amendment must not
/// re-pin — and so re-author — every contribution recorded under the earlier declaration.
#[test]
fn an_amended_declaration_does_not_reauthor_earlier_contributions() {
    let dir = repo();
    let root = dir.path();
    plant_small(root);
    import(root, ".claude/memory");
    let before = contributions(root);
    let events_before = events(root);
    let path = root.join(".epr-meta/collective.json");
    let mut declaration: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("declaration")).expect("json");
    declaration["charter"] = serde_json::json!("An amended charter.");
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&declaration).expect("json"),
    )
    .expect("amend");
    actor::claim(root, SECOND_AUTHOR, SECOND_SESSION).expect("second claim");
    for session in [SESSION, SECOND_SESSION] {
        let again = memory::execute_with(
            root,
            "import",
            &Options {
                target: Some(".claude/memory"),
                session: Some(session),
                ..Options::default()
            },
        )
        .expect("re-import");
        assert_eq!(again["counts"]["skipped"], before.len(), "{session}");
        assert_eq!(again["counts"]["eventsAppended"], 0, "{session}");
    }
    assert_eq!(contributions(root), before);
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
    use eprfs_agent::memory::{Contribution, FileRef, Imported, Locality, Source};
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
            reach: Locality::Repository,
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
                reach: Locality::Repository,
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

// ── the entry form: per entry, never per importer ───────────────────────────────────────────────

fn import_entries(
    root: &Path,
    session: &str,
    files: &[&str],
) -> elohim_epr_cli::flow::FlowResult<serde_json::Value> {
    memory::execute_with(
        root,
        "import",
        &Options {
            target: files.first().copied(),
            more_targets: files[1..].to_vec(),
            session: Some(session),
            ..Options::default()
        },
    )
}

fn author_of(root: &Path, stem: &str) -> String {
    let text = std::fs::read_to_string(
        root.join(format!(".eprfs/status/memory/contributions/{stem}.json")),
    )
    .expect("contribution");
    let c: serde_json::Value = serde_json::from_str(&text).expect("json");
    c["author"].as_str().expect("author").to_string()
}

/// The harness imports what ONE session wrote under THAT session. Two sessions wrote into the same
/// directory; each import names only its own entries, and each entry lands under its writer —
/// even when the writer is a session that has since been followed by another claim.
#[test]
fn the_entry_form_imports_only_the_named_entries_under_their_writers_claim() {
    let dir = repo();
    let root = dir.path();
    plant_small(root);
    // A LATER session claims; SESSION (the fixture's first) is now a past session.
    actor::claim(root, SECOND_AUTHOR, SECOND_SESSION).expect("second claim");
    // The harness witnessed each write: beta by the second session, the rest by the first.
    witness(
        root,
        ".claude/memory/project_beta.md",
        SECOND_SESSION,
        "2099-01-01T00:00:00Z",
    );
    for file in [
        ".claude/memory/feedback_alpha.md",
        ".claude/memory/reference_gamma.md",
    ] {
        witness(root, file, SESSION, "2099-01-01T00:00:00Z");
    }

    let beta = import_entries(root, SECOND_SESSION, &[".claude/memory/project_beta.md"])
        .expect("the second session imports its own entry");
    assert_eq!(beta["form"], "entries");
    assert_eq!(beta["directory"], ".claude/memory");
    assert_eq!(beta["counts"]["entries"], 1, "{beta}");
    assert_eq!(beta["counts"]["contributed"], 1, "{beta}");
    assert_eq!(beta["counts"]["eventsAppended"], 1, "{beta}");
    let names: Vec<String> = contributions(root).into_iter().map(|(n, _)| n).collect();
    assert_eq!(
        names,
        vec!["project_beta.json".to_string()],
        "an entry-form import swept up entries it did not name"
    );
    assert_eq!(author_of(root, "project_beta"), SECOND_AUTHOR);

    // The past session's entries, imported under the past session: its own claim, not the latest.
    let rest = import_entries(
        root,
        SESSION,
        &[
            ".claude/memory/reference_gamma.md",
            ".claude/memory/feedback_alpha.md",
            ".claude/memory/feedback_alpha.md",
        ],
    )
    .expect("the past session imports its own entries");
    assert_eq!(rest["counts"]["entries"], 2, "duplicates collapse: {rest}");
    assert_eq!(rest["counts"]["contributed"], 2, "{rest}");
    assert_eq!(author_of(root, "feedback_alpha"), AUTHOR);
    assert_eq!(author_of(root, "reference_gamma"), AUTHOR);
    assert_eq!(author_of(root, "project_beta"), SECOND_AUTHOR);

    // A directory sweep afterwards finds nothing to do and re-authors nothing.
    let events_before = events(root);
    let sweep = import(root, ".claude/memory");
    assert_eq!(sweep["form"], "directory");
    assert_eq!(sweep["counts"]["skipped"], 3, "{sweep}");
    assert_eq!(events(root), events_before);
    assert_eq!(author_of(root, "project_beta"), SECOND_AUTHOR);

    let projected = project(root, Options::default()).expect("index projects");
    assert_eq!(projected["population"]["unattributed"], 0, "{projected}");
    assert_eq!(projected["entries"], 3, "{projected}");
}

/// No fallback: a session that never claimed has no author to offer, and the entry form will not
/// borrow one — not even this device's witnessed standing human, which the directory form accepts.
#[test]
fn the_entry_form_refuses_a_session_that_never_claimed_and_writes_nothing() {
    let dir = repo();
    let root = dir.path();
    plant_small(root);
    let keys = TempDir::new().expect("keys");
    let key_file = witnessed(root, &keys);
    let events_before = events(root);

    let out = epr(
        root,
        &key_file,
        &[
            "flow",
            "memory",
            "import",
            ".claude/memory/feedback_alpha.md",
            "--session",
            "never-claimed-session",
            "--json",
        ],
    );
    assert!(
        !out.status.success(),
        "an entry-form import borrowed an author for an unclaimed session: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("registered no actor claim") && stderr.contains("unattributable"),
        "the refusal names why and what the entry stays: {stderr}"
    );
    assert!(
        !root.join(".eprfs/status/memory/contributions").exists(),
        "a refused entry-form import wrote a request"
    );
    assert_eq!(events(root), events_before, "a refused import wrote an act");
}

#[test]
fn the_entry_form_names_entries_of_one_directory_and_never_the_index() {
    let dir = repo();
    let root = dir.path();
    plant_small(root);
    write(
        root,
        ".claude/memory/MEMORY.md",
        "- [x](feedback_alpha.md) - row\n",
    );
    write(
        root,
        ".claude/other/feedback_delta.md",
        "---\nname: feedback_delta\ndescription: d.\nmetadata:\n  type: feedback\n---\n",
    );

    let index = import_entries(root, SESSION, &[".claude/memory/MEMORY.md"])
        .expect_err("the projection is never an entry");
    assert!(index.to_string().contains("MEMORY.md"), "{index}");

    let spread = import_entries(
        root,
        SESSION,
        &[
            ".claude/memory/feedback_alpha.md",
            ".claude/other/feedback_delta.md",
        ],
    )
    .expect_err("one import names one directory's entries");
    assert!(spread.to_string().contains("span"), "{spread}");

    let missing = import_entries(
        root,
        SESSION,
        &[
            ".claude/memory/absent.md",
            ".claude/memory/feedback_alpha.md",
        ],
    )
    .expect_err("a missing entry is refused, not skipped");
    assert!(missing.to_string().contains("not a file"), "{missing}");
    assert!(!root.join(".eprfs/status/memory/contributions").exists());
}

#[test]
fn a_second_positional_is_still_refused_outside_import() {
    let dir = repo();
    let root = dir.path();
    let err = memory::execute_with(
        root,
        "pin",
        &Options {
            input: Some(".epr-meta/collective.json"),
            target: Some("a"),
            more_targets: vec!["b"],
            ..Options::default()
        },
    )
    .expect_err("only import takes more than one positional");
    assert!(
        err.to_string().contains("unexpected second argument b"),
        "{err}"
    );
}

// ── as of the writing: the claim current WHEN the entry was written ─────────────────────────────

const ORCHESTRATOR: &str = "agent:orchestrator@fixture";
const SUBAGENT: &str = "agent:scribe@fixture";
const SHARED_SESSION: &str = "one-session-two-personas";

/// Append a claim dated `at` (a claim is dated by its caller; the fixture dates it outright).
fn claim_at(root: &Path, claimed: &str, session: &str, at: &str) {
    use elohim_epr_rea::{ActorClaim, ActorRecord, ActorStore, SidecarActorStore};
    let claim = ActorClaim::new(claimed, session, at, None).expect("claim shape");
    SidecarActorStore::open(root)
        .expect("actor store")
        .append(ActorRecord::Claim(claim))
        .expect("append");
}

/// The harness's write witness for an entry's CURRENT bytes: what the PostToolUse hook appends at
/// the edit moment. Written here exactly as the hook writes it.
fn witness(root: &Path, file: &str, session: &str, observed_at: &str) {
    use std::io::Write as _;
    let bytes = std::fs::read(root.join(file)).expect("entry bytes");
    let line = serde_json::json!({
        "path": file,
        "sha256": sha256(&bytes),
        "session": session,
        "observedAt": observed_at,
    });
    let log = root.join(".eprfs/status/memory-writes.jsonl");
    std::fs::create_dir_all(log.parent().expect("parent")).expect("mkdir");
    let mut fh = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .expect("witness log");
    writeln!(fh, "{line}").expect("append");
}

fn import_as_of(
    root: &Path,
    session: &str,
    at: &str,
    files: &[&str],
) -> elohim_epr_cli::flow::FlowResult<serde_json::Value> {
    for file in files {
        witness(root, file, session, at);
    }
    memory::execute_with(
        root,
        "import",
        &Options {
            target: files.first().copied(),
            more_targets: files[1..].to_vec(),
            session: Some(session),
            as_of: Some(at),
            ..Options::default()
        },
    )
}

/// The act's provider in the flow plane, not just the request's author slot: the note leg must
/// agree with the request, or the projection would not count the contribution at all.
fn last_provider(root: &Path) -> String {
    last_event(root)["record"]["provider"]
        .as_str()
        .expect("provider")
        .to_string()
}

#[test]
fn a_subagent_claiming_later_never_takes_the_orchestrators_earlier_entry() {
    let dir = repo();
    let root = dir.path();
    plant_small(root);
    claim_at(root, ORCHESTRATOR, SHARED_SESSION, "2026-09-25T10:00:00Z");
    // The subagent claims LATER in the same session: it is now the session's latest claim.
    claim_at(root, SUBAGENT, SHARED_SESSION, "2026-09-25T12:00:00Z");

    let out = import_as_of(
        root,
        SHARED_SESSION,
        "2026-09-25T11:30:00.250Z",
        &[".claude/memory/feedback_alpha.md"],
    )
    .expect("the orchestrator's entry imports under the orchestrator");
    assert_eq!(out["counts"]["contributed"], 1, "{out}");
    assert_eq!(author_of(root, "feedback_alpha"), ORCHESTRATOR);
    assert_eq!(
        last_provider(root),
        ORCHESTRATOR,
        "the act fell forward to the later claim"
    );
    let projected = project(root, Options::default()).expect("index projects");
    assert_eq!(projected["population"]["unattributed"], 0, "{projected}");
}

#[test]
fn an_entry_written_before_any_claim_is_unattributable() {
    let dir = repo();
    let root = dir.path();
    plant_small(root);
    claim_at(root, ORCHESTRATOR, SHARED_SESSION, "2026-09-25T10:00:00Z");
    let events_before = events(root);

    let err = import_as_of(
        root,
        SHARED_SESSION,
        "2026-09-25T09:59:59.999Z",
        &[".claude/memory/feedback_alpha.md"],
    )
    .expect_err("no claim existed yet; the later one must not be borrowed");
    let message = err.to_string();
    assert!(
        message.contains("no actor claim when it was written")
            && message.contains("unattributable"),
        "{message}"
    );
    assert!(
        message.contains("2026-09-25T10:00:00"),
        "names the first claim: {message}"
    );
    assert!(!root.join(".eprfs/status/memory/contributions").exists());
    assert_eq!(events(root), events_before);
}

#[test]
fn two_claims_bracketing_two_entries_give_each_entry_its_own_claim() {
    let dir = repo();
    let root = dir.path();
    plant_small(root);
    claim_at(root, ORCHESTRATOR, SHARED_SESSION, "2026-09-25T10:00:00Z");
    claim_at(root, SUBAGENT, SHARED_SESSION, "2026-09-25T12:00:00Z");

    import_as_of(
        root,
        SHARED_SESSION,
        "2026-09-25T13:00:00+01:00", // 12:00Z — exactly at the second claim: it counts
        &[".claude/memory/project_beta.md"],
    )
    .expect("the later entry imports under the later claim");
    assert_eq!(author_of(root, "project_beta"), SUBAGENT);
    assert_eq!(last_provider(root), SUBAGENT);

    import_as_of(
        root,
        SHARED_SESSION,
        "2026-09-25T11:59:59Z",
        &[".claude/memory/feedback_alpha.md"],
    )
    .expect("the earlier entry imports under the earlier claim");
    assert_eq!(author_of(root, "feedback_alpha"), ORCHESTRATOR);
    assert_eq!(last_provider(root), ORCHESTRATOR);

    // And the directory form refuses an instant it cannot honour per entry.
    let err = memory::execute_with(
        root,
        "import",
        &Options {
            target: Some(".claude/memory"),
            session: Some(SHARED_SESSION),
            as_of: Some("2026-09-25T11:00:00Z"),
            ..Options::default()
        },
    )
    .expect_err("--as-of is the entry form's");
    assert!(err.to_string().contains("entry form"), "{err}");
    let projected = project(root, Options::default()).expect("index projects");
    assert_eq!(projected["population"]["unattributed"], 0, "{projected}");
}

// ── recorded_at: ordering two claims made against ONE tree ──────────────────────────────────────

fn actor_lines(root: &Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(root.join(".eprfs/status/actors.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("actor line"))
        .collect()
}

#[test]
fn two_claims_against_one_head_are_ordered_by_when_they_were_recorded() {
    let dir = repo();
    let root = dir.path();
    plant_small(root);
    actor::claim_recorded_at(root, ORCHESTRATOR, "same-head", "2026-09-25T20:10:00Z")
        .expect("first claim");
    actor::claim_recorded_at(root, SUBAGENT, "same-head", "2026-09-25T20:30:00Z")
        .expect("second claim");
    let claims: Vec<serde_json::Value> = actor_lines(root)
        .into_iter()
        .filter(|l| l["record"]["session"] == "same-head")
        .collect();
    assert_eq!(claims.len(), 2);
    assert_eq!(
        claims[0]["record"]["claimedAt"], claims[1]["record"]["claimedAt"],
        "both claims were made against the same HEAD — claimedAt alone cannot order them"
    );
    assert_eq!(claims[1]["record"]["recordedAt"], "2026-09-25T20:30:00Z");

    // Written at 20:20, between the two recordings: the EARLIER claim authors it.
    import_as_of(
        root,
        "same-head",
        "2026-09-25T20:20:00Z",
        &[".claude/memory/feedback_alpha.md"],
    )
    .expect("the entry between the claims imports under the earlier one");
    assert_eq!(author_of(root, "feedback_alpha"), ORCHESTRATOR);
    assert_eq!(last_provider(root), ORCHESTRATOR);
}

#[test]
fn re_claiming_the_current_identity_appends_nothing() {
    let dir = repo();
    let root = dir.path();
    let before = actor_lines(root).len();
    // `repo()` claimed AUTHOR under SESSION a moment ago; the same claim again is a no-op, even
    // though its recording instant would differ.
    let again = actor::claim(root, AUTHOR, SESSION).expect("re-claim");
    assert!(
        !again.appended,
        "an unchanged re-claim was re-stamped and appended"
    );
    assert_eq!(actor_lines(root).len(), before);
    // A CLI claim records its instant.
    let claim = actor_lines(root)
        .into_iter()
        .find(|l| l["record"]["session"] == SESSION)
        .expect("the claim");
    assert!(
        claim["record"]["recordedAt"].is_string(),
        "a new claim carries recordedAt: {claim}"
    );
}

// ── steward of record (operator ruling) ─────────────────────────────────────────────────────────

const STANDING_HUMAN: &str = "human:matthew"; // Standing Steward in the real affiliations sidecar
const FIXTURE_HUMAN: &str = "human:adam"; // Fixture Steward in the same sidecar

fn plant_entry(root: &Path, stem: &str, origin: Option<&str>, modified: &str) {
    let origin = origin
        .map(|o| format!("  originSessionId: {o}\n"))
        .unwrap_or_default();
    write(
        root,
        &format!(".claude/memory/{stem}.md"),
        &format!(
            "---\nname: {stem}\ndescription: The operator's ruling {stem}.\nmetadata:\n  type: feedback\n{origin}  modified: {modified}\n---\n\nbody\n"
        ),
    );
}

fn stand(
    root: &Path,
    session: &str,
    files: &[&str],
) -> elohim_epr_cli::flow::FlowResult<serde_json::Value> {
    memory::execute_with(
        root,
        "import",
        &Options {
            target: files.first().copied(),
            more_targets: files[1..].to_vec(),
            session: Some(session),
            steward_of_record: true,
            ..Options::default()
        },
    )
}

#[test]
fn steward_of_record_refuses_an_agent_session() {
    let dir = repo();
    let root = dir.path();
    plant_entry(root, "feedback_ruling", None, "2026-09-25T09:00:00Z");
    let err = stand(root, SESSION, &[".claude/memory/feedback_ruling.md"])
        .expect_err("an agent never stands as steward of record");
    assert!(err.to_string().contains("not a human participant"), "{err}");
    assert!(!root.join(".eprfs/status/memory/contributions").exists());
}

#[test]
fn steward_of_record_refuses_a_fixture_steward() {
    let dir = repo();
    let root = dir.path();
    plant_entry(root, "feedback_ruling", None, "2026-09-25T09:00:00Z");
    actor::claim(root, FIXTURE_HUMAN, "fixture-human-session").expect("claim");
    let err = stand(
        root,
        "fixture-human-session",
        &[".claude/memory/feedback_ruling.md"],
    )
    .expect_err("a fixture human cannot stand for real fruit");
    assert!(err.to_string().contains("FIXTURE"), "{err}");
    assert!(!root.join(".eprfs/status/memory/contributions").exists());
}

#[test]
fn steward_of_record_refuses_an_entry_with_a_witnessable_author() {
    let dir = repo();
    let root = dir.path();
    // SESSION claimed AUTHOR in `repo()`; this entry was written (per its frontmatter) after.
    plant_entry(
        root,
        "feedback_authored",
        Some(SESSION),
        "2099-01-01T00:00:00Z",
    );
    actor::claim(root, STANDING_HUMAN, "human-session").expect("claim");
    let events_before = events(root);
    let err = stand(
        root,
        "human-session",
        &[".claude/memory/feedback_authored.md"],
    )
    .expect_err("a real author is never overridden");
    let message = err.to_string();
    assert!(
        message.contains("witnessable author")
            && message.contains(SESSION)
            && message.contains(AUTHOR),
        "names the session and its claim: {message}"
    );
    assert!(!root.join(".eprfs/status/memory/contributions").exists());
    assert_eq!(events(root), events_before);
}

#[test]
fn steward_of_record_admits_an_unattributable_entry_and_labels_it() {
    let dir = repo();
    let root = dir.path();
    plant_entry(
        root,
        "feedback_ruling",
        Some("never-claimed-session"),
        "2026-09-25T09:00:00Z",
    );
    plant_entry(root, "project_no_origin", None, "2026-09-25T09:00:00Z");
    actor::claim(root, STANDING_HUMAN, "human-session").expect("claim");

    let out = stand(
        root,
        "human-session",
        &[
            ".claude/memory/feedback_ruling.md",
            ".claude/memory/project_no_origin.md",
        ],
    )
    .expect("the standing human stands for unattributable entries");
    assert_eq!(out["counts"]["contributed"], 2, "{out}");

    let text = std::fs::read_to_string(
        root.join(".eprfs/status/memory/contributions/feedback_ruling.json"),
    )
    .expect("contribution");
    let c: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(c["author"], STANDING_HUMAN);
    let slot = c["uncertainty"]
        .as_array()
        .expect("uncertainty")
        .iter()
        .filter_map(|v| v.as_str())
        .find(|s| s.starts_with("authorship: steward-of-record"))
        .expect("the authorship slot")
        .to_string();
    assert!(
        slot.contains("does not say human:matthew wrote it"),
        "{slot}"
    );
    assert!(
        slot.contains("Origin session: never-claimed-session"),
        "{slot}"
    );
    let no_origin = std::fs::read_to_string(
        root.join(".eprfs/status/memory/contributions/project_no_origin.json"),
    )
    .expect("contribution");
    assert!(no_origin.contains("Origin session: none recorded"));
    assert_eq!(last_provider(root), STANDING_HUMAN);

    // The label is shown where authorship is read: the projected index row.
    project(
        root,
        Options {
            out: Some(".claude/memory/MEMORY.md"),
            ..Options::default()
        },
    )
    .expect("index projects");
    let index = std::fs::read_to_string(root.join(".claude/memory/MEMORY.md")).expect("index");
    let row = index
        .lines()
        .find(|l| l.contains("](feedback_ruling.md)"))
        .expect("a row");
    assert!(
        row.contains("[steward of record: human:matthew]"),
        "the row does not say who stands for it: {row}"
    );
}

// ── one parser, the harness's write witness, a conservative steward of record ──────────────────

fn attribution_of(root: &Path, file: &str) -> serde_json::Value {
    let out = memory::execute_with(
        root,
        "attribution",
        &Options {
            target: Some(file),
            ..Options::default()
        },
    )
    .expect("attribution report");
    out["entries"][0].clone()
}

fn import_plain(
    root: &Path,
    file: &str,
    session: Option<&str>,
    as_of: Option<&str>,
) -> elohim_epr_cli::flow::FlowResult<serde_json::Value> {
    memory::execute_with(
        root,
        "import",
        &Options {
            target: Some(file),
            session,
            as_of,
            ..Options::default()
        },
    )
}

/// The review's finding: a `|` block quoting `originSessionId:`/`modified:` read as the entry's
/// own keys by a second, naive parser. There is one parser now, and it skips a block scalar's
/// body at the top level AND inside `metadata:` — so the import follows the real keys.
#[test]
fn a_block_scalar_quoting_the_keys_is_ignored_and_the_import_follows_the_real_keys() {
    let dir = repo();
    let root = dir.path();
    actor::claim_recorded_at(root, ORCHESTRATOR, "real-session", "2026-09-25T10:00:00Z")
        .expect("real writer's claim");
    actor::claim_recorded_at(root, SUBAGENT, "quoted-session", "2026-09-25T09:00:00Z")
        .expect("quoted session's claim");
    write(
        root,
        ".claude/memory/feedback_block.md",
        "---\nname: feedback_block\ndescription: An entry that quotes attribution keys.\n\
         details: |\n  originSessionId: quoted-session\n  modified: 2026-09-25T09:30:00Z\n\
         metadata:\n  type: feedback\n  originSessionId: real-session\n  modified: 2026-09-25T11:00:00Z\n\
         \x20 note: |\n    originSessionId: quoted-session\n    modified: 2026-09-25T09:30:00Z\n\
         ---\n\nbody\n",
    );
    let file = ".claude/memory/feedback_block.md";
    let report = attribution_of(root, file);
    assert_eq!(report["session"], "real-session", "{report}");
    assert_eq!(report["sessionSource"], "frontmatter");
    assert_eq!(report["writtenAt"], "2026-09-25T11:00:00.000Z", "{report}");
    assert_eq!(report["writtenBasis"], "modified");

    // A caller naming the quoted writer is refused, naming both.
    let err = import_plain(root, file, Some("quoted-session"), None)
        .expect_err("the entry's own origin governs");
    let message = err.to_string();
    assert!(
        message.contains("real-session") && message.contains("quoted-session"),
        "{message}"
    );
    // A caller's instant that disagrees with the entry's `modified` is refused.
    let err = import_plain(root, file, None, Some("2026-09-25T09:30:00Z"))
        .expect_err("the entry's own modified governs");
    assert!(err.to_string().contains("disagrees"), "{err}");
    assert!(!root.join(".eprfs/status/memory/contributions").exists());

    // No caller assertions at all: the entry's real keys decide the author.
    import_plain(root, file, None, None).expect("imports under its real writer");
    assert_eq!(author_of(root, "feedback_block"), ORCHESTRATOR);
    assert_eq!(last_provider(root), ORCHESTRATOR);
}

/// Live mtime is never evidence: with no `modified` and no witness of its bytes, an entry under a
/// session that DID claim is unattributable, and no one may stand for it either.
#[test]
fn an_ambiguous_write_under_a_claimed_session_is_unattributable_and_never_stood_for() {
    let dir = repo();
    let root = dir.path();
    actor::claim(root, STANDING_HUMAN, "human-session").expect("claim");
    // SESSION claimed AUTHOR in `repo()`. No `modified`, and no witness of these bytes.
    let entry = |stem: &str, desc: &str| {
        format!(
            "---\nname: {stem}\ndescription: {desc}\nmetadata:\n  type: feedback\n  originSessionId: {SESSION}\n---\n"
        )
    };
    let file = ".claude/memory/feedback_unwitnessed.md";
    write(
        root,
        file,
        &entry("feedback_unwitnessed", "No modified, no witness."),
    );
    let report = attribution_of(root, file);
    assert_eq!(report["writtenBasis"], "ambiguous", "{report}");
    assert_eq!(report["attributable"], false);
    assert_eq!(report["stewardOfRecordAdmissible"], false);
    assert!(
        report["reason"].as_str().unwrap().contains("ambiguous"),
        "{report}"
    );

    let err = import_plain(root, file, None, None).expect_err("never guessed");
    assert!(err.to_string().contains("unattributable"), "{err}");
    let err = stand(root, "human-session", &[file]).expect_err("never stood for");
    let message = err.to_string();
    assert!(
        message.contains(SESSION) && message.contains("ambiguous"),
        "{message}"
    );

    // A witness of these exact bytes proves the write, and the import follows it.
    witness(root, file, SESSION, "2099-01-01T00:00:00Z");
    import_plain(root, file, None, None).expect("witnessed, so attributable");
    assert_eq!(author_of(root, "feedback_unwitnessed"), AUTHOR);

    // A witness of DIFFERENT bytes proves nothing about these.
    let edited = ".claude/memory/feedback_reedited.md";
    write(root, edited, &entry("feedback_reedited", "Witnessed once."));
    witness(root, edited, SESSION, "2099-01-01T00:00:00Z");
    write(
        root,
        edited,
        &entry("feedback_reedited", "Edited again, unwitnessed."),
    );
    let report = attribution_of(root, edited);
    assert_eq!(report["writtenBasis"], "ambiguous", "{report}");
}

/// The steward of record is admitted only when no agent author could possibly be witnessed —
/// including a PROVEN write that predates the origin session's first claim.
#[test]
fn steward_of_record_admits_a_write_proven_to_predate_the_first_claim() {
    let dir = repo();
    let root = dir.path();
    actor::claim(root, STANDING_HUMAN, "human-session").expect("claim");
    actor::claim_recorded_at(root, ORCHESTRATOR, "late-session", "2026-09-25T10:00:00Z")
        .expect("late claim");
    plant_entry(
        root,
        "feedback_early",
        Some("late-session"),
        "2026-09-25T09:00:00Z",
    );
    let file = ".claude/memory/feedback_early.md";
    let report = attribution_of(root, file);
    assert_eq!(report["attributable"], false, "{report}");
    assert_eq!(report["stewardOfRecordAdmissible"], true, "{report}");
    import_plain(root, file, None, None).expect_err("no claim when it was written");

    stand(root, "human-session", &[file]).expect("proven to predate: admissible");
    assert_eq!(author_of(root, "feedback_early"), STANDING_HUMAN);

    // The same session, a write AFTER its first claim: an author is witnessable; refused.
    plant_entry(
        root,
        "feedback_after",
        Some("late-session"),
        "2026-09-25T11:00:00Z",
    );
    let err = stand(root, "human-session", &[".claude/memory/feedback_after.md"])
        .expect_err("a witnessable author is never overridden");
    let message = err.to_string();
    assert!(
        message.contains("late-session") && message.contains(ORCHESTRATOR),
        "{message}"
    );
}

// ── the final round: column-0 block bodies, the earliest witness ────────────────────────────────

/// A top-level block scalar with an EMPTY body, followed by a column-0 line that reads like a key:
/// YAML says new key, an author who forgot to indent meant body. Attribution rests on which, so
/// the entry is refused as malformed — never read either way. With a real indented body, the
/// column-0 line that follows it is unambiguously a key, and is followed.
#[test]
fn a_column_zero_line_after_an_empty_block_scalar_is_malformed_never_guessed() {
    let dir = repo();
    let root = dir.path();
    actor::claim(root, STANDING_HUMAN, "human-session").expect("claim");
    actor::claim_recorded_at(root, ORCHESTRATOR, "real-session", "2026-09-25T10:00:00Z")
        .expect("claim");
    let ambiguous = ".claude/memory/feedback_ambiguous_block.md";
    write(
        root,
        ambiguous,
        "---\nname: feedback_ambiguous_block\ndescription: A block with no indented body.\n\
         details: |\noriginSessionId: real-session\nmetadata:\n  type: feedback\n  \
         modified: 2026-09-25T11:00:00Z\n---\n",
    );
    let report = attribution_of(root, ambiguous);
    assert_eq!(report["attributable"], false, "{report}");
    assert_eq!(report["stewardOfRecordAdmissible"], false, "{report}");
    assert!(
        report["reason"].as_str().unwrap().contains("malformed"),
        "{report}"
    );
    let err = import_plain(root, ambiguous, None, None).expect_err("never guessed");
    assert!(err.to_string().contains("malformed"), "{err}");
    let err = stand(root, "human-session", &[ambiguous]).expect_err("never stood for either");
    assert!(err.to_string().contains("malformed"), "{err}");
    // The directory form refuses it by name too.
    let swept = memory::execute_with(
        root,
        "import",
        &Options {
            target: Some(".claude/memory"),
            session: Some(SESSION),
            ..Options::default()
        },
    )
    .expect("the directory form reports per entry");
    let refused = swept["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["entry"] == ambiguous)
        .expect("the entry is reported");
    assert_eq!(refused["state"], "refused", "{refused}");
    assert!(refused["reason"].as_str().unwrap().contains("malformed"));

    // An indented body ends at the column-0 line — which is then a real, unambiguous key.
    let clear = ".claude/memory/feedback_clear_block.md";
    write(
        root,
        clear,
        "---\nname: feedback_clear_block\ndescription: A block with a body.\n\
         details: |\n  originSessionId: quoted-session\noriginSessionId: real-session\n\
         metadata:\n  type: feedback\n  modified: 2026-09-25T11:00:00Z\n---\n",
    );
    let report = attribution_of(root, clear);
    assert_eq!(report["session"], "real-session", "{report}");
    import_plain(root, clear, None, None).expect("follows the real key");
    assert_eq!(author_of(root, "feedback_clear_block"), ORCHESTRATOR);
}

/// The first session to produce an entry's bytes authored them. A later IDENTICAL re-save by
/// another session (a no-op write the hook no longer even witnesses) never takes them over.
#[test]
fn the_earliest_witness_of_the_bytes_is_the_author_not_a_later_identical_resave() {
    let dir = repo();
    let root = dir.path();
    actor::claim_recorded_at(root, ORCHESTRATOR, "session-a", "2026-09-25T09:00:00Z")
        .expect("claim a");
    actor::claim_recorded_at(root, SUBAGENT, "session-b", "2026-09-25T09:00:00Z").expect("claim b");
    let file = ".claude/memory/feedback_resaved.md";
    write(
        root,
        file,
        "---\nname: feedback_resaved\ndescription: Written by A, re-saved unchanged by B.\n\
         metadata:\n  type: feedback\n---\n",
    );
    witness(root, file, "session-a", "2026-09-25T10:00:00Z");
    witness(root, file, "session-b", "2026-09-25T11:00:00Z");
    let report = attribution_of(root, file);
    assert_eq!(report["session"], "session-a", "{report}");
    assert_eq!(report["writtenAt"], "2026-09-25T10:00:00.000Z", "{report}");
    import_plain(root, file, None, None).expect("imports under its first writer");
    assert_eq!(author_of(root, "feedback_resaved"), ORCHESTRATOR);
    assert_eq!(last_provider(root), ORCHESTRATOR);
}

// ── the writer of the current bytes: the harness's witness before the entry's self-report ──────

const CURATOR: &str = "agent:librarian@fixture";
const CURATOR_SESSION: &str = "curator-session";
const WRITER_SESSION: &str = "writer-session";

/// A memory entry whose frontmatter names `origin` as its writer, with extra top-level lines.
fn memory_entry(root: &Path, stem: &str, desc: &str, origin: Option<&str>, extra: &str) -> String {
    let file = format!(".claude/memory/{stem}.md");
    let origin = origin
        .map(|o| format!("  originSessionId: {o}\n"))
        .unwrap_or_default();
    write(
        root,
        &file,
        &format!(
            "---\nname: {stem}\ndescription: {desc}\n{extra}metadata:\n  type: feedback\n{origin}---\n\nbody of {stem}\n"
        ),
    );
    file
}

/// The librarian's defect (2026-09-26): S2 edited an entry whose frontmatter still named its first
/// writer S1, and the edit was imported under S1's claim. The harness witnessed S2 writing the
/// current bytes; that witness names the writer, and the edit is reported as S2's — never S1's.
#[test]
fn an_edit_by_another_session_is_attributed_to_its_editor_never_the_frontmatter_origin() {
    let dir = repo();
    let root = dir.path();
    actor::claim_recorded_at(root, ORCHESTRATOR, WRITER_SESSION, "2026-09-25T09:00:00Z")
        .expect("S1's claim");
    actor::claim_recorded_at(root, CURATOR, CURATOR_SESSION, "2026-09-25T09:00:00Z")
        .expect("S2's claim");
    let file = memory_entry(
        root,
        "feedback_edited",
        "Written by S1.",
        Some(WRITER_SESSION),
        "",
    );
    witness(root, &file, WRITER_SESSION, "2099-01-01T00:00:00Z");
    import_plain(root, &file, None, None).expect("S1 contributes its own entry");
    assert_eq!(author_of(root, "feedback_edited"), ORCHESTRATOR);
    let contributed =
        std::fs::read(root.join(".eprfs/status/memory/contributions/feedback_edited.json"))
            .expect("contribution");

    // S2 edits; the frontmatter still names S1.
    memory_entry(
        root,
        "feedback_edited",
        "Rewritten by S2.",
        Some(WRITER_SESSION),
        "",
    );
    witness(root, &file, CURATOR_SESSION, "2099-01-01T01:00:00Z");
    let report = attribution_of(root, &file);
    assert_eq!(report["session"], CURATOR_SESSION, "{report}");
    assert_eq!(report["sessionSource"], "witness", "{report}");
    assert_eq!(report["claim"], CURATOR, "{report}");
    assert_eq!(report["writtenAt"], "2099-01-01T01:00:00.000Z", "{report}");
    assert_eq!(report["contributedBy"], ORCHESTRATOR, "{report}");
    assert_eq!(
        report["editedByAnotherParticipant"],
        serde_json::json!({"writer": CURATOR, "contributor": ORCHESTRATOR}),
        "{report}"
    );
    assert_eq!(report["importable"], false, "{report}");

    // Neither S1's claim nor S2's may re-author the contribution through the entry form.
    let err = import_plain(root, &file, None, None).expect_err("never re-authored");
    assert!(err.to_string().contains("never re-authored"), "{err}");
    let err = import_plain(root, &file, Some(WRITER_SESSION), None)
        .expect_err("the frontmatter origin is not the writer of these bytes");
    assert!(err.to_string().contains(CURATOR_SESSION), "{err}");
    assert_eq!(
        std::fs::read(root.join(".eprfs/status/memory/contributions/feedback_edited.json"))
            .expect("contribution"),
        contributed,
        "the contribution was rewritten"
    );
}

/// An entry predating the witness log has only its frontmatter: its `originSessionId` and
/// `modified` still name its writer and instant.
#[test]
fn a_legacy_entry_with_no_witness_still_resolves_by_its_frontmatter() {
    let dir = repo();
    let root = dir.path();
    actor::claim_recorded_at(root, ORCHESTRATOR, WRITER_SESSION, "2026-09-25T09:00:00Z")
        .expect("claim");
    let file = memory_entry(
        root,
        "feedback_legacy",
        "Written before the witness log existed.",
        Some(WRITER_SESSION),
        "",
    );
    // `modified` sits in metadata beside the origin.
    let text = std::fs::read_to_string(root.join(&file)).expect("entry");
    write(
        root,
        &file,
        &text.replace(
            "  originSessionId:",
            "  modified: 2026-09-25T11:00:00Z\n  originSessionId:",
        ),
    );
    let report = attribution_of(root, &file);
    assert_eq!(report["session"], WRITER_SESSION, "{report}");
    assert_eq!(report["sessionSource"], "frontmatter", "{report}");
    assert_eq!(report["writtenBasis"], "modified", "{report}");
    assert_eq!(report["importable"], true, "{report}");
    import_plain(root, &file, None, None).expect("imports by its frontmatter");
    assert_eq!(author_of(root, "feedback_legacy"), ORCHESTRATOR);
}

/// The earliest witness of the bytes wins — over a later identical re-save, and over a frontmatter
/// origin naming the re-saver.
#[test]
fn the_earliest_witness_wins_over_a_later_resave_and_over_the_frontmatter() {
    let dir = repo();
    let root = dir.path();
    actor::claim_recorded_at(root, ORCHESTRATOR, "session-a", "2026-09-25T09:00:00Z")
        .expect("claim a");
    actor::claim_recorded_at(root, SUBAGENT, "session-b", "2026-09-25T09:00:00Z").expect("claim b");
    let file = memory_entry(
        root,
        "feedback_first",
        "Written by A; B's frontmatter and re-save do not take it.",
        Some("session-b"),
        "",
    );
    witness(root, &file, "session-a", "2099-01-01T00:00:00Z");
    witness(root, &file, "session-b", "2099-01-01T01:00:00Z");
    let report = attribution_of(root, &file);
    assert_eq!(report["session"], "session-a", "{report}");
    assert_eq!(report["writtenAt"], "2099-01-01T00:00:00.000Z", "{report}");
    import_plain(root, &file, None, None).expect("imports under its first writer");
    assert_eq!(author_of(root, "feedback_first"), ORCHESTRATOR);
}

// ── cross-author supersession, governed ────────────────────────────────────────────────────────

const WITNESSED_AT: &str = "2099-01-01T00:00:00Z";

/// Write an entry as `session`, witness it, and import it through the entry form.
fn contribute_entry(root: &Path, stem: &str, session: &str, extra: &str) -> serde_json::Value {
    let file = memory_entry(root, stem, &format!("The claim of {stem}."), None, extra);
    witness(root, &file, session, WITNESSED_AT);
    let out = import_plain(root, &file, None, None).expect("import");
    out["entries"][0].clone()
}

fn fold_claims(root: &Path) {
    actor::claim_recorded_at(root, ORCHESTRATOR, WRITER_SESSION, "2026-09-25T09:00:00Z")
        .expect("orchestrator");
    actor::claim_recorded_at(root, CURATOR, CURATOR_SESSION, "2026-09-25T09:00:00Z")
        .expect("curator");
}

/// Project and install the index; return (report, index text).
fn project_index(root: &Path) -> (serde_json::Value, String) {
    let out = project(
        root,
        Options {
            out: Some(".claude/memory/MEMORY.md"),
            ..Options::default()
        },
    )
    .expect("index projects");
    let text = std::fs::read_to_string(root.join(".claude/memory/MEMORY.md")).expect("index");
    (out, text)
}

fn row<'a>(index: &'a str, file: &str) -> Option<&'a str> {
    index.lines().find(|l| l.contains(&format!("]({file})")))
}

/// Every member's entry file and contribution file, byte for byte.
fn member_bytes(root: &Path, stems: &[&str]) -> Vec<Vec<u8>> {
    stems
        .iter()
        .flat_map(|s| {
            [
                std::fs::read(root.join(format!(".claude/memory/{s}.md"))).expect("entry"),
                std::fs::read(root.join(format!(".eprfs/status/memory/contributions/{s}.json")))
                    .expect("contribution"),
            ]
        })
        .collect()
}

fn verdict_on(root: &Path, resource: &str, session: &str, verdict: &str) {
    elohim_epr_cli::flow::note::note(
        root,
        resource,
        "verdict",
        "the umbrella keeps what its members said",
        None,
        Some(verdict),
        &elohim_epr_cli::flow::note::NoteActor {
            as_ref: None,
            session: Some(session.into()),
        },
    )
    .expect("verdict note");
}

fn fold_of<'a>(report: &'a serde_json::Value, umbrella: &str) -> &'a serde_json::Value {
    report["supersession"]
        .as_array()
        .expect("supersession")
        .iter()
        .find(|f| f["umbrella"] == umbrella)
        .expect("the umbrella's fold")
}

#[test]
fn a_same_author_fold_hides_its_members_at_once_and_touches_none_of_their_bytes() {
    let dir = repo();
    let root = dir.path();
    fold_claims(root);
    contribute_entry(root, "feedback_one", CURATOR_SESSION, "");
    contribute_entry(root, "feedback_two", CURATOR_SESSION, "");
    let before = member_bytes(root, &["feedback_one", "feedback_two"]);

    let umbrella = contribute_entry(
        root,
        "feedback_umbrella",
        CURATOR_SESSION,
        "supersedes: [feedback_one, feedback_two.md]\n",
    );
    assert_eq!(umbrella["state"], "contributed", "{umbrella}");
    assert_eq!(umbrella["supersession"]["state"], "effective", "{umbrella}");
    assert_eq!(umbrella["supersession"]["basis"], "same-author");
    assert_eq!(author_of(root, "feedback_umbrella"), CURATOR);
    let recorded: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            root.join(".eprfs/status/memory/contributions/feedback_umbrella.json"),
        )
        .expect("umbrella"),
    )
    .expect("json");
    let pinned = recorded["supersedes"].as_array().expect("supersedes");
    assert_eq!(pinned.len(), 2, "{recorded}");
    assert_eq!(
        pinned[0]["path"],
        ".eprfs/status/memory/contributions/feedback_one.json"
    );

    let (report, index) = project_index(root);
    assert!(row(&index, "feedback_umbrella.md").is_some(), "{index}");
    assert!(row(&index, "feedback_one.md").is_none(), "{index}");
    assert!(row(&index, "feedback_two.md").is_none(), "{index}");
    assert_eq!(
        report["supersededRows"],
        serde_json::json!(["feedback_one.md", "feedback_two.md"])
    );
    assert_eq!(report["population"]["superseded"], 2, "{report}");
    assert_eq!(
        member_bytes(root, &["feedback_one", "feedback_two"]),
        before
    );

    // Attribution still sees the member, and names who folded it.
    let member = attribution_of(root, ".claude/memory/feedback_one.md");
    assert_eq!(member["supersededBy"], "feedback_umbrella.md", "{member}");
    assert_eq!(member["contributedBy"], CURATOR);
}

#[test]
fn another_build_of_the_curators_own_role_is_another_author_so_the_fold_waits() {
    // The same-author bypass grants a fold with no Steward review, so it is exact identity:
    // a sibling build of the curator's role is a different participant (review of 98decb42f).
    let dir = repo();
    let root = dir.path();
    fold_claims(root);
    actor::claim_recorded_at(
        root,
        "agent:librarian@another-build",
        "sibling-session",
        "2026-09-25T09:00:00Z",
    )
    .expect("sibling build");
    contribute_entry(root, "feedback_sibling", "sibling-session", "");
    let umbrella = contribute_entry(
        root,
        "feedback_sibling_umbrella",
        CURATOR_SESSION,
        "supersedes: [feedback_sibling]\n",
    );
    assert_eq!(umbrella["supersession"]["state"], "pending", "{umbrella}");
    let (_, index) = project_index(root);
    assert!(row(&index, "feedback_sibling.md").is_some(), "{index}");
}

#[test]
fn a_same_author_fold_cannot_bury_a_pending_cross_author_umbrella() {
    let dir = repo();
    let root = dir.path();
    fold_claims(root);
    contribute_entry(root, "feedback_theirs", WRITER_SESSION, "");
    let pending = contribute_entry(
        root,
        "feedback_pending_umbrella",
        CURATOR_SESSION,
        "supersedes: [feedback_theirs]\n",
    );
    assert_eq!(pending["supersession"]["state"], "pending", "{pending}");
    // The curator folds its OWN pending umbrella: same author, effective at once — but the
    // pending umbrella's row (and its member's) must stay visible until a Steward rules.
    let outer = contribute_entry(
        root,
        "feedback_outer_umbrella",
        CURATOR_SESSION,
        "supersedes: [feedback_pending_umbrella]\n",
    );
    assert_eq!(outer["supersession"]["state"], "effective", "{outer}");
    let (_, index) = project_index(root);
    assert!(
        row(&index, "feedback_pending_umbrella.md").is_some(),
        "{index}"
    );
    assert!(row(&index, "feedback_theirs.md").is_some(), "{index}");
}

#[test]
fn a_cross_author_fold_stays_pending_until_a_distinct_steward_approves() {
    let dir = repo();
    let root = dir.path();
    fold_claims(root);
    actor::claim(root, STANDING_HUMAN, "human-session").expect("steward claim");
    contribute_entry(root, "feedback_theirs", WRITER_SESSION, "");
    contribute_entry(root, "feedback_mine", CURATOR_SESSION, "");
    let before = member_bytes(root, &["feedback_theirs", "feedback_mine"]);

    let umbrella = contribute_entry(
        root,
        "feedback_umbrella",
        CURATOR_SESSION,
        "supersedes: [feedback_theirs, feedback_mine]\n",
    );
    let resource = umbrella["supersession"]["resource"]
        .as_str()
        .expect("resource")
        .to_string();
    assert_eq!(umbrella["supersession"]["state"], "pending", "{umbrella}");
    let command = format!("epr flow note --on {resource} --kind verdict --verdict approved");
    assert!(
        umbrella["supersession"]["approve"]
            .as_str()
            .unwrap()
            .starts_with(&command),
        "{umbrella}"
    );

    // Pending: umbrella and members all indexed; the umbrella's row says so; the advisory names
    // the command.
    let (report, index) = project_index(root);
    for file in ["feedback_theirs.md", "feedback_mine.md"] {
        assert!(
            row(&index, file).is_some(),
            "{file} was hidden early: {index}"
        );
    }
    assert!(
        row(&index, "feedback_umbrella.md")
            .unwrap()
            .contains("[supersession pending a Steward's verdict]"),
        "{index}"
    );
    assert_eq!(fold_of(&report, "feedback_umbrella.md")["state"], "pending");
    assert!(
        report["advisory"][0].as_str().unwrap().contains(&command),
        "{report}"
    );
    let member = attribution_of(root, ".claude/memory/feedback_theirs.md");
    assert_eq!(member["supersededBy"], serde_json::Value::Null, "{member}");
    assert_eq!(member["supersessionPending"], "feedback_umbrella.md");

    // The curator's own approval counts for nothing.
    verdict_on(root, &resource, CURATOR_SESSION, "approved");
    let (report, index) = project_index(root);
    assert_eq!(fold_of(&report, "feedback_umbrella.md")["state"], "pending");
    assert!(row(&index, "feedback_theirs.md").is_some());

    // A distinct, standing Steward approves: the members fold out.
    verdict_on(root, &resource, "human-session", "approved");
    let (report, index) = project_index(root);
    let fold = fold_of(&report, "feedback_umbrella.md");
    assert_eq!(fold["state"], "effective", "{fold}");
    assert_eq!(fold["basis"], "steward-verdict");
    assert_eq!(fold["approval"]["approver"], STANDING_HUMAN);
    assert_eq!(fold["approval"]["validatedAt"], "local (steward on record)");
    assert!(row(&index, "feedback_theirs.md").is_none(), "{index}");
    assert!(row(&index, "feedback_mine.md").is_none(), "{index}");
    let umbrella_row = row(&index, "feedback_umbrella.md").expect("umbrella row");
    assert!(!umbrella_row.contains("pending"), "{umbrella_row}");
    assert_eq!(
        member_bytes(root, &["feedback_theirs", "feedback_mine"]),
        before,
        "a superseded member's bytes moved"
    );
    let member = attribution_of(root, ".claude/memory/feedback_theirs.md");
    assert_eq!(member["supersededBy"], "feedback_umbrella.md", "{member}");
    assert_eq!(member["contributedBy"], ORCHESTRATOR, "never re-authored");
}

#[test]
fn a_steward_curator_cannot_approve_its_own_fold_and_a_fixture_approval_reads_bootstrap() {
    let dir = repo();
    let root = dir.path();
    fold_claims(root);
    actor::claim_recorded_at(
        root,
        STANDING_HUMAN,
        "human-session",
        "2026-09-25T09:00:00Z",
    )
    .expect("steward curator");
    actor::claim(root, FIXTURE_HUMAN, "fixture-session").expect("fixture steward");
    contribute_entry(root, "feedback_theirs", WRITER_SESSION, "");
    let umbrella = contribute_entry(
        root,
        "feedback_umbrella",
        "human-session",
        "supersedes: [feedback_theirs]\n",
    );
    assert_eq!(author_of(root, "feedback_umbrella"), STANDING_HUMAN);
    let resource = umbrella["supersession"]["resource"]
        .as_str()
        .unwrap()
        .to_string();

    verdict_on(root, &resource, "human-session", "approved");
    let (report, index) = project_index(root);
    let fold = fold_of(&report, "feedback_umbrella.md");
    assert_eq!(fold["state"], "pending", "{fold}");
    assert!(
        fold["reason"].as_str().unwrap().contains("is the author"),
        "{fold}"
    );
    assert!(row(&index, "feedback_theirs.md").is_some());

    verdict_on(root, &resource, "fixture-session", "approved");
    let (report, index) = project_index(root);
    let fold = fold_of(&report, "feedback_umbrella.md");
    assert_eq!(fold["state"], "effective", "{fold}");
    assert_eq!(fold["approval"]["approver"], FIXTURE_HUMAN);
    assert_eq!(
        fold["approval"]["validatedAt"],
        "bootstrap (fixture co-steward)"
    );
    assert!(row(&index, "feedback_theirs.md").is_none(), "{index}");

    // A later contrary verdict from an active Steward withdraws the fold.
    verdict_on(root, &resource, "fixture-session", "changes-requested");
    let (report, index) = project_index(root);
    assert_eq!(fold_of(&report, "feedback_umbrella.md")["state"], "pending");
    assert!(row(&index, "feedback_theirs.md").is_some(), "{index}");
}

#[test]
fn a_stewardless_collective_never_makes_a_cross_author_fold_effective() {
    use eprfs_agent::memory::{Affiliation, AffiliationStanding, MemberKind, MembershipRole};
    let dir = repo();
    let root = dir.path();
    fold_claims(root);
    actor::claim(root, STANDING_HUMAN, "human-session").expect("steward claim");
    contribute_entry(root, "feedback_theirs", WRITER_SESSION, "");
    let umbrella = contribute_entry(
        root,
        "feedback_umbrella",
        CURATOR_SESSION,
        "supersedes: [feedback_theirs]\n",
    );
    let resource = umbrella["supersession"]["resource"]
        .as_str()
        .unwrap()
        .to_string();
    verdict_on(root, &resource, "human-session", "approved");

    // Both Stewards leave: the fixture co-steward (sponsored by matthew), then matthew himself.
    let declaration = std::fs::read(root.join(".epr-meta/collective.json")).expect("declaration");
    let withdrawal = |member: &str, standing: AffiliationStanding, sponsor: &str| Affiliation {
        version: 1,
        collective: eprfs_agent::memory::FileRef {
            path: ".epr-meta/collective.json".into(),
            cid: eprfs_core::BlobCid::compute_raw(&declaration).to_string(),
        },
        member: member.into(),
        member_kind: MemberKind::Person,
        role: MembershipRole::Steward,
        sponsor: Some(sponsor.into()),
        acts_for: None,
        standing,
        since: "2026-09-26T00:00:00Z".into(),
        withdrawn: Some("2026-09-26T00:00:00Z".into()),
    };
    let mut lines =
        std::fs::read_to_string(root.join(memory::AFFILIATIONS_PATH)).expect("affiliations");
    for record in [
        withdrawal(FIXTURE_HUMAN, AffiliationStanding::Fixture, STANDING_HUMAN),
        withdrawal(
            STANDING_HUMAN,
            AffiliationStanding::Standing,
            STANDING_HUMAN,
        ),
    ] {
        lines.push_str(&memory::affiliation_line(&record).expect("line"));
        lines.push('\n');
    }
    write(root, memory::AFFILIATIONS_PATH, &lines);
    let collective = memory::execute(root, "collective", None, None).expect("collective");
    assert_eq!(collective["stewardship"], "stewardless", "{collective}");

    let (report, index) = project_index(root);
    let fold = fold_of(&report, "feedback_umbrella.md");
    assert_eq!(fold["state"], "pending", "{fold}");
    assert!(
        fold["reason"].as_str().unwrap().contains("stewardless"),
        "{fold}"
    );
    assert!(row(&index, "feedback_theirs.md").is_some(), "{index}");
}

#[test]
fn a_fold_over_an_uncontributed_entry_itself_or_a_cycle_is_refused() {
    let dir = repo();
    let root = dir.path();
    fold_claims(root);

    // Not contributed: the entry exists, but no contribution does.
    memory_entry(root, "feedback_loose", "Never imported.", None, "");
    let refused = contribute_entry(
        root,
        "feedback_umbrella",
        CURATOR_SESSION,
        "supersedes: [feedback_loose]\n",
    );
    assert_eq!(refused["state"], "refused", "{refused}");
    assert!(
        refused["reason"]
            .as_str()
            .unwrap()
            .contains("not contributed"),
        "{refused}"
    );

    // Itself.
    let refused = contribute_entry(
        root,
        "feedback_selfish",
        CURATOR_SESSION,
        "supersedes: [feedback_selfish]\n",
    );
    assert_eq!(refused["state"], "refused", "{refused}");
    assert!(
        refused["reason"].as_str().unwrap().contains("itself"),
        "{refused}"
    );

    // A cycle: a folds b; then b is re-imported folding a.
    contribute_entry(root, "feedback_b", CURATOR_SESSION, "");
    let a = contribute_entry(
        root,
        "feedback_a",
        CURATOR_SESSION,
        "supersedes: [feedback_b]\n",
    );
    assert_eq!(a["state"], "contributed", "{a}");
    let file = memory_entry(
        root,
        "feedback_b",
        "The claim of feedback_b, now folding a.",
        None,
        "supersedes: [feedback_a]\n",
    );
    witness(root, &file, CURATOR_SESSION, "2099-01-01T01:00:00Z");
    let out = import_plain(root, &file, None, None).expect("reports per entry");
    let refused = &out["entries"][0];
    assert_eq!(refused["state"], "refused", "{refused}");
    assert!(
        refused["reason"].as_str().unwrap().contains("cycle"),
        "{refused}"
    );

    // A block list is refused by name rather than read as nothing.
    let refused = contribute_entry(
        root,
        "feedback_block_list",
        CURATOR_SESSION,
        "supersedes:\n  - feedback_b\n",
    );
    assert_eq!(refused["state"], "refused", "{refused}");
    assert!(
        refused["reason"].as_str().unwrap().contains("inline list"),
        "{refused}"
    );
}
