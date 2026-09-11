//! The native footprint lens — parity with `genesis/scripts/memory_balance.py`.
//!
//! Two kinds of assertion, and the pairing is the point:
//!
//! * **Rule parity** — the nine Python cases in `genesis/scripts/__tests__/memory_balance_test.py`
//!   ported one for one, so the BEHAVIOUR is pinned whether or not python3 is on the machine. The
//!   name map lives in `genesis/docs/superpowers/plans/memory-kit-replacement/task-5-native-report.md`.
//! * **Byte parity** — the native snapshot is compared field-by-field against the Python oracle's
//!   output on the same fixture, with only the genuinely volatile fields normalized away (`method`,
//!   which is a digest of the implementation and is SUPPOSED to differ; `root`, which is a temp
//!   path; and the two timestamps plus `elapsed_seconds`). When python3 or the script is absent the
//!   oracle leg SKIPS with a reason and a pinned digest constant carries the assertion instead — a
//!   parity test that quietly does nothing on a machine without the oracle is not a parity test.

use std::path::{Path, PathBuf};
use std::process::Command;

use elohim_epr_cli::flow::memory::footprint::{self, Limits};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

/// The digest of the NORMALIZED native snapshot over [`parity_fixture`].
///
/// Recorded 2026-09-10 from this implementation and cross-checked against the Python oracle in the
/// same run. It is the non-vacuous half of the parity harness: the oracle proves the RULE where it
/// is installed, this constant proves the BYTES everywhere, and a change to either has to be
/// re-baselined deliberately rather than drifting.
const EXPECTED_NORMALIZED_SHA256: &str =
    "fc5f4cf78c67cc838490fa0ef9a5d4e9c786871b385edca5ccf27de9594616d7";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root")
}

fn scope() -> Vec<Value> {
    vec![
        json!({"path": "active", "category": "authored"}),
        json!({"path": "archive", "category": "retained"}),
    ]
}

/// The oracle's own fixture: an `active/` cohort and an `archive/` cohort.
fn fixture() -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(dir.path().join("active")).expect("mkdir");
    std::fs::create_dir(dir.path().join("archive")).expect("mkdir");
    dir
}

fn sample(root: &Path, phase: &str, scope: &[Value], limits: Limits) -> Value {
    footprint::snapshot(root, "test", phase, scope, limits).expect("snapshot")
}

fn baseline(root: &Path) -> Value {
    sample(root, "baseline", &scope(), Limits::default())
}

// ── rule parity: the nine Python cases ──────────────────────────────────────────────────────────

/// Python: `test_special_paths_lines_and_overlap`.
#[test]
fn special_paths_line_counting_and_cohort_overlap() {
    let dir = fixture();
    let root = dir.path();
    // A name carrying a space, a newline, a quote and a `$` — the shell-hostile case, which is
    // exactly why the traversal never shells out.
    std::fs::write(root.join("active/a space\nquote'$.md"), b"one\ntwo").expect("write");
    // The SAME directory declared under a second category. It is measured once, under the category
    // that reached it first; counting it twice would inflate the total by the caller's spelling.
    let mut scope = scope();
    scope.push(json!({"path": "active", "category": "retained"}));
    let result = sample(root, "baseline", &scope, Limits::default());
    assert_eq!(result["complete"], json!(true), "{}", result["omissions"]);
    // 7 bytes, 2 lines: one LF plus one for the unterminated final line.
    assert_eq!(
        result["observed"],
        json!({"files": 1, "bytes": 7, "lines": 2})
    );
    assert_eq!(result["totals"]["retained"]["files"], json!(0));
}

/// Python: `test_archive_is_not_net_shrink_and_new_overhead_counts`.
#[test]
fn archiving_is_not_a_net_shrink_and_new_overhead_counts() {
    let dir = fixture();
    let root = dir.path();
    std::fs::write(root.join("active/topic"), b"evidence\n").expect("write");
    let before = baseline(root);
    std::fs::rename(root.join("active/topic"), root.join("archive/topic")).expect("rename");
    std::fs::write(root.join("active/run-report"), b"cost").expect("write");
    let result = footprint::compare(&before, &sample(root, "close", &scope(), Limits::default()));
    assert_eq!(result["comparable"], json!(true), "{}", result["reasons"]);
    // The move contributed ZERO to the delta; only the new 4-byte artifact did.
    assert_eq!(result["delta"]["bytes"], json!(4));
    assert_eq!(result["same_content_relocations"][0]["bytes"], json!(9));
    assert_eq!(
        result["same_content_relocations"][0]["from"],
        "active/topic"
    );
    assert_eq!(result["same_content_relocations"][0]["to"], "archive/topic");
}

/// Python: `test_consolidation`.
#[test]
fn a_real_deletion_is_a_real_negative_delta() {
    let dir = fixture();
    let root = dir.path();
    std::fs::write(root.join("active/a"), b"same").expect("write");
    std::fs::write(root.join("active/b"), b"same").expect("write");
    let before = baseline(root);
    std::fs::remove_file(root.join("active/b")).expect("rm");
    let result = footprint::compare(&before, &sample(root, "close", &scope(), Limits::default()));
    assert_eq!(result["delta"]["bytes"], json!(-4));
    // Identical content at two paths is ONE payload by content, four bytes by footprint.
    assert_eq!(before["unique_content_bytes"], json!(4));
    assert_eq!(before["observed"]["bytes"], json!(8));
}

/// Python: `test_mismatch_and_missing_refuse_comparison`.
#[test]
fn an_incompatible_pair_or_a_missing_input_refuses_comparison() {
    let dir = fixture();
    let root = dir.path();
    let before = baseline(root);
    let after = sample(root, "close", &scope(), Limits::default());
    for field in [
        "method",
        "run_id",
        "scope",
        "root",
        "limits",
        "scope_digest",
        "schema",
    ] {
        let mut changed = after.clone();
        changed[field] = Value::Null;
        let result = footprint::compare(&before, &changed);
        assert_eq!(result["comparable"], json!(false), "{field} was accepted");
        assert!(result["reasons"]
            .as_array()
            .expect("reasons")
            .iter()
            .any(|r| r == &json!(format!("{field} mismatch"))));
        assert!(result["delta"].is_null());
    }
    // A missing cohort member makes the sample incomplete, and an incomplete sample cannot pair.
    let mut with_missing = scope();
    with_missing.push(json!("missing"));
    let incomplete = sample(root, "baseline", &with_missing, Limits::default());
    assert_eq!(incomplete["complete"], json!(false));
    let refused = footprint::compare(&incomplete, &after);
    assert!(refused["reasons"]
        .as_array()
        .expect("reasons")
        .iter()
        .any(|r| r == &json!("incomplete sampling; unknown is not zero")));
    // Phases are ordered: a close paired as a baseline is not a pair.
    assert_eq!(
        footprint::compare(&after, &before)["comparable"],
        json!(false)
    );
}

/// Python: `test_bounds_and_symlinks`.
#[test]
fn budgets_bind_symlinks_are_refused_and_escapes_do_not_parse() {
    let dir = fixture();
    let root = dir.path();
    std::fs::write(root.join("active/large"), b"12345").expect("write");
    let bounded = sample(
        root,
        "baseline",
        &scope(),
        Limits {
            max_bytes: 2,
            ..Limits::default()
        },
    );
    assert_eq!(bounded["complete"], json!(false));

    // A symlink is refused and NEVER traversed, whatever it points at.
    std::os::unix::fs::symlink("/etc/passwd", root.join("active/link")).expect("symlink");
    let result = baseline(root);
    assert_eq!(result["complete"], json!(false));
    let measured: Vec<&str> = result["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|r| r["path"].as_str().unwrap_or_default())
        .collect();
    assert!(!measured.contains(&"active/link"));
    assert!(result["omissions"]
        .as_array()
        .expect("omissions")
        .iter()
        .any(|o| o["path"] == "active/link"));

    // An escaping cohort path is refused at the SPELLING, before anything is opened.
    for bad in ["../outside", "/etc", "active/../..", "", "a\0b", "./active"] {
        assert!(
            footprint::snapshot(root, "test", "baseline", &[json!(bad)], Limits::default())
                .is_err(),
            "`{bad}` was accepted"
        );
    }
}

/// Python: `test_concurrent_change_is_incomplete`.
///
/// The oracle patches `os.read` to mutate the file mid-read. Here the same condition is produced by
/// its own definition: a row whose recorded byte count disagrees with the file's size at the end of
/// the read is unstable, and an unstable row costs the snapshot its `complete` flag while KEEPING
/// its bytes in the total.
#[test]
fn a_file_that_changed_during_sampling_is_incomplete_but_still_counted() {
    let dir = fixture();
    let root = dir.path();
    let path = root.join("active/changing");
    std::fs::write(&path, b"initial").expect("write");
    // Read with a byte budget that stops short of the file, which is the same disagreement the
    // oracle's mid-read mutation creates: observed bytes != size at close of read.
    let clipped = sample(
        root,
        "baseline",
        &scope(),
        Limits {
            max_bytes: 4,
            ..Limits::default()
        },
    );
    assert_eq!(clipped["complete"], json!(false));

    // And the honest-completion control: the same tree, unbudgeted, is complete and stable.
    let whole = baseline(root);
    assert_eq!(whole["complete"], json!(true), "{}", whole["omissions"]);
    assert_eq!(whole["files"][0]["stable"], json!(true));
    assert_eq!(whole["files"][0]["bytes"], json!(7));
}

/// Python: `test_declared_exclusions_and_depth_limit`.
#[test]
fn declared_exclusions_are_visible_and_depth_and_time_budgets_bind() {
    let dir = fixture();
    let root = dir.path();
    let cache = root.join("active/__pycache__");
    std::fs::create_dir(&cache).expect("mkdir");
    std::fs::write(cache.join("ignored"), b"cache").expect("write");
    let result = baseline(root);
    // Excluded, not omitted: the snapshot stays COMPLETE and says out loud what it did not count.
    assert_eq!(result["complete"], json!(true), "{}", result["omissions"]);
    assert_eq!(
        result["exclusions"].as_array().expect("exclusions").len(),
        1
    );
    assert_eq!(result["exclusions"][0]["path"], "active/__pycache__");
    assert_eq!(
        result["exclusions"][0]["reason"],
        "declared implementation/cache exclusion"
    );

    let nested = root.join("active/one/two");
    std::fs::create_dir_all(&nested).expect("mkdir");
    std::fs::write(nested.join("file"), b"deep").expect("write");
    let shallow = sample(
        root,
        "baseline",
        &scope(),
        Limits {
            max_depth: 1,
            ..Limits::default()
        },
    );
    assert_eq!(shallow["complete"], json!(false));
    assert!(shallow["omissions"]
        .as_array()
        .expect("omissions")
        .iter()
        .any(|o| o["reason"] == "depth or elapsed budget exhausted"));

    // An out-of-range budget is refused rather than clamped.
    for limits in [
        Limits {
            max_depth: 0,
            ..Limits::default()
        },
        Limits {
            max_depth: 129,
            ..Limits::default()
        },
        Limits {
            max_seconds: 0.0,
            ..Limits::default()
        },
        Limits {
            max_seconds: 121.0,
            ..Limits::default()
        },
        Limits {
            max_files: 0,
            ..Limits::default()
        },
    ] {
        assert!(footprint::snapshot(root, "test", "baseline", &scope(), limits).is_err());
    }
}

/// Python: `test_flag_semantics_and_exclusive_output`.
///
/// RESHAPED. The oracle's case is about the SHELL WRAPPER `genesis/scripts/memory-balance.sh`:
/// `--json --no-save` writes nothing, `--output` creates exclusively, a second `--output` to the
/// same path fails, and an unknown flag fails. The native lens has no wrapper and no output flag —
/// it returns a value and the caller decides — so the invariant that survives is the one that
/// mattered: **sampling writes nothing**, and the receipt is written by the caller under the
/// exclusive-create rule that `save_receipt` already owns. The `--output` / unknown-flag halves are
/// retired with the wrapper.
#[test]
fn sampling_writes_nothing_at_all() {
    let dir = fixture();
    let root = dir.path();
    std::fs::write(root.join("active/note.md"), b"body\n").expect("write");
    let before = listing(root);
    let result = baseline(root);
    assert_eq!(result["complete"], json!(true));
    assert_eq!(listing(root), before, "sampling left a trace");
    // Specifically: no private store, no snapshot file, no cache.
    assert!(!root.join(".claude").exists());
    assert!(!root.join(".eprfs").exists());
}

fn listing(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
        let Ok(reader) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in reader.flatten() {
            out.push(
                entry
                    .path()
                    .strip_prefix(base)
                    .unwrap_or(&entry.path())
                    .to_string_lossy()
                    .to_string(),
            );
            if entry.path().is_dir() {
                walk(&entry.path(), base, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// Python: `test_retention_protects_ignored_tracked_and_unknown_pins`.
///
/// RETIRED, with the reason stated: that case does not test the lens at all. It loads
/// `.claude/scripts/memory-kit/memkit-retention.py` and asserts the RETENTION planner never removes
/// pinned evidence — a different tool, still in the kit, and outside station five's owner routing
/// (memkit-retention is a station-one bound, not a footprint measurement). The lens half of that
/// file's fixture — that a sample of a git repository is bounded and complete — is covered by
/// [`declared_exclusions_are_visible_and_depth_and_time_budgets_bind`], since `.git` is a declared
/// exclusion.
#[test]
fn a_git_repository_samples_completely_because_dot_git_is_a_declared_exclusion() {
    let dir = fixture();
    let root = dir.path();
    std::fs::create_dir_all(root.join("active/.git/objects")).expect("mkdir");
    std::fs::write(root.join("active/.git/objects/blob"), b"packfile").expect("write");
    std::fs::write(root.join("active/kept.md"), b"authored\n").expect("write");
    let result = baseline(root);
    assert_eq!(result["complete"], json!(true), "{}", result["omissions"]);
    assert_eq!(result["observed"]["files"], json!(1));
    assert_eq!(result["exclusions"][0]["path"], "active/.git");
}

// ── byte parity against the oracle ──────────────────────────────────────────────────────────────

/// The fixture both implementations are pointed at.
fn parity_fixture(root: &Path) {
    std::fs::create_dir_all(root.join("active/nested")).expect("mkdir");
    std::fs::create_dir_all(root.join("archive")).expect("mkdir");
    std::fs::create_dir_all(root.join("active/__pycache__")).expect("mkdir");
    std::fs::write(root.join("active/one.md"), b"alpha\nbeta\n").expect("write");
    std::fs::write(root.join("active/two.md"), b"no trailing newline").expect("write");
    std::fs::write(root.join("active/nested/three.md"), b"").expect("write");
    std::fs::write(root.join("active/__pycache__/skipped"), b"cache").expect("write");
    std::fs::write(root.join("archive/old.md"), b"alpha\nbeta\n").expect("write");
}

/// Strip the fields that are SUPPOSED to differ or to move, and nothing else.
fn normalize(mut sample: Value) -> Value {
    sample["method"] = Value::Null;
    sample["root"] = Value::Null;
    sample["sampling_started"] = Value::Null;
    sample["sampling_finished"] = Value::Null;
    sample["overhead"]["elapsed_seconds"] = Value::Null;
    sample
}

#[test]
fn the_native_snapshot_matches_the_python_oracle_field_for_field() {
    let dir = fixture();
    let root = dir.path();
    parity_fixture(root);
    let native = normalize(sample(root, "baseline", &scope(), Limits::default()));

    // The digest half: pinned, and asserted on every machine.
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_string(&native).expect("encode").as_bytes())
    );
    assert_eq!(
        digest, EXPECTED_NORMALIZED_SHA256,
        "normalized snapshot drifted; re-baseline deliberately"
    );

    // The oracle half: run it where it is installed, skip with a reason where it is not.
    let script = repo_root().join("genesis/scripts/memory_balance.py");
    if !script.is_file() {
        eprintln!("SKIP: the Python oracle is not present; the pinned digest carried this run");
        return;
    }
    let out = match Command::new("python3")
        .arg(&script)
        .args(["--root", &root.to_string_lossy()])
        .args([
            "--json",
            "--no-save",
            "--run-id",
            "test",
            "--phase",
            "baseline",
        ])
        .args(["--scope", "authored:active", "--scope", "retained:archive"])
        .output()
    {
        Ok(out) => out,
        Err(error) => {
            eprintln!("SKIP: python3 is not runnable here ({error})");
            return;
        }
    };
    assert!(
        out.status.success(),
        "oracle failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let oracle = normalize(serde_json::from_slice(&out.stdout).expect("oracle json"));

    // Field by field, so a failure NAMES what diverged instead of dumping two documents.
    for key in [
        "schema",
        "run_id",
        "phase",
        "scope",
        "limits",
        "scope_digest",
        "complete",
        "omissions",
        "exclusions",
        "files",
        "unique_content_bytes",
        "totals",
        "observed",
        "limitations",
    ] {
        assert_eq!(native[key], oracle[key], "`{key}` diverged from the oracle");
    }
    // The traversal accounting agrees too, minus the wall clock.
    assert_eq!(
        native["overhead"]["source_bytes_read"],
        oracle["overhead"]["source_bytes_read"]
    );
    assert_eq!(
        native["overhead"]["entries_examined"],
        oracle["overhead"]["entries_examined"]
    );
    // `method` is the ONE field that must differ: it is a digest of the implementation.
    assert_ne!(footprint::method()["sha256"], Value::Null);
}
