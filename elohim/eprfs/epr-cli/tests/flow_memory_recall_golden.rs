//! Station zero of governed discovery: the split must be byte-identical. Three renderings are
//! pinned by digest BEFORE any function moves; the constants are re-baselined only by a task
//! that says why.
use std::path::Path;
use std::process::Command;

use sha2::{Digest, Sha256};

mod common; // re-use tests/common/mod.rs helpers: repo(), begin_focused()

/// Runs the shipped binary and returns its stdout with two ambient, non-algorithmic fields
/// normalized out.
///
/// The renderer legitimately echoes `--root <path>` back into its own "next command"
/// suggestions and reports a measured `elapsed_seconds` in its Usage line — both real product
/// behaviour, not test artifacts, but both vary run to run (`tempfile::tempdir()`'s random
/// suffix; actual wall-clock duration) in a way that has nothing to do with the recall
/// algorithm's output shape. Pinning the RENDERING, not incidentals a given run happened to
/// produce, is what "byte-identical" means here.
fn run_text(root: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(["flow", "memory", "recall"])
        .args(args)
        .args([
            "--root",
            &root.to_string_lossy(),
            "--contract",
            "contract.json",
        ])
        .output()
        .expect("epr runs");
    let raw = String::from_utf8(out.stdout).expect("utf8");
    let text = raw.replace(&*root.to_string_lossy(), "<FIXTURE_ROOT>");
    redact_elapsed_seconds(&text)
}

/// Replaces the numeric value following every `elapsed_seconds ` occurrence with a fixed
/// placeholder — a measured wall-clock duration, not part of the pinned rendering.
fn redact_elapsed_seconds(text: &str) -> String {
    const NEEDLE: &str = "elapsed_seconds ";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find(NEEDLE) {
        out.push_str(&rest[..pos + NEEDLE.len()]);
        rest = &rest[pos + NEEDLE.len()..];
        let end = rest
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(rest.len());
        out.push_str("<FIXTURE_ELAPSED>");
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn digest(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}

// Re-baselined station 1 (Task 1.1): a `lens:` line is now printed after `Guiding context` on
// every rendered view — see tests/fixtures/recall-golden/README.md. Re-baselined AGAIN in fix
// round 1 of Task 1.1's review: the `lens:` line gained a short CID and a `renew:` slot, and the
// fixture contract now declares an explicit `lens_table` (see tests/common/mod.rs
// `contract_value`) rather than only inheriting the live one. Re-baselined a THIRD time for
// Task 1.2: the honesty floor adds one `recipe … · lens … · selection: … · omissions: … ·
// receipts: …` line immediately after `lens:` on every view, at every lens — see
// tests/fixtures/recall-golden/README.md's "station 1, Task 1.2" entry. GOLDEN_REFUSAL is
// unchanged across all three rounds: a refusal never reaches `render()`'s orientation/lens/floor
// preamble.
const GOLDEN_FOCUSED: &str = "2b830dac46a98cfb06c865063e365f45e800a0cadcf2a640361133a1625feddf";
const GOLDEN_WHOLE: &str = "9b5477e4eeac2a003a13fd51e3895bb868fc81ed10ff6349ef66a8c2fd09482e";
const GOLDEN_REFUSAL: &str = "882890b4af2e5f60f4d6fbc322377fe4eb9bc12ad495fe4b435fb8d251b1a061";

#[test]
fn focused_open_is_byte_identical() {
    let dir = common::repo();
    common::begin_focused(dir.path(), "which command rebuilds the stale index");
    let text = run_text(
        dir.path(),
        &[
            "open",
            "--session",
            "golden",
            "--need",
            "which command rebuilds the stale index",
            "--scope",
            "tooling",
        ],
    );
    assert_eq!(digest(&text), GOLDEN_FOCUSED, "{text}");
}

#[test]
fn whole_open_is_byte_identical() {
    let dir = common::repo();
    let text = run_text(
        dir.path(),
        &["open", "--session", "golden-whole", "--need", "orient"],
    );
    assert_eq!(digest(&text), GOLDEN_WHOLE, "{text}");
}

#[test]
fn refusal_is_byte_identical() {
    let dir = common::repo();
    let text = run_text(
        dir.path(),
        &[
            "read",
            "--session",
            "golden-refuse",
            "--path",
            "worktrees/x.md",
            "--lines",
            "1:2",
        ],
    );
    assert_eq!(digest(&text), GOLDEN_REFUSAL, "{text}");
}
