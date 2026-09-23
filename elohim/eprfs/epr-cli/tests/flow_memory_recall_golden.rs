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

// Every re-baseline of these pins, and why, is recorded in the dated history of
// tests/fixtures/recall-golden/README.md — that file, not a count here, is the record. Two
// kinds recur: a rendering change (a new `lens:` or honesty-floor line), and a contract byte
// change — `contract_value()` inherits the live contract, and every rendered view prints its
// `recipe` CID (`Contract::method_cid()` over the WHOLE contract's bytes), so any contract edit
// moves GOLDEN_FOCUSED and GOLDEN_WHOLE. GOLDEN_REFUSAL has never moved: a refusal never reaches
// `render()`'s orientation/lens/floor preamble, so it never prints a `recipe` line.
const GOLDEN_FOCUSED: &str = "46e2d00a401baa78dd2af568958c92c3af2b2ca4012bbf0c7e07cd493a83499f";
const GOLDEN_WHOLE: &str = "562a9cd85cf9df15c7ec2bfa8289bb58e2a0fd204e17f9c67563f1750fe6c6b5";
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
