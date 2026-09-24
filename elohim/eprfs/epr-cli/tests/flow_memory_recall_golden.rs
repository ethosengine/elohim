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
    redact_accounting_elapsed(&redact_measured(
        &redact_measured(&text, "elapsed_seconds "),
        "semantic_query_ms ",
    ))
}

/// Replaces the rounded wall-clock segment of the `Accounting:` line (`… native bytes · 0.1s ·
/// …`) with the same placeholder: it is the same measured duration as `elapsed_seconds`, rounded
/// to a tenth, and a loaded machine turns `0.0s` into `0.1s` without the rendering changing.
fn redact_accounting_elapsed(text: &str) -> String {
    let is_seconds = |segment: &str| {
        segment
            .strip_suffix('s')
            .is_some_and(|n| !n.is_empty() && n.parse::<f64>().is_ok())
    };
    text.split_inclusive('\n')
        .map(|line| {
            if !line.starts_with("Accounting:") {
                return line.to_string();
            }
            let (body, newline) = match line.strip_suffix('\n') {
                Some(body) => (body, "\n"),
                None => (line, ""),
            };
            let segments: Vec<String> = body
                .split(" · ")
                .map(|segment| {
                    if is_seconds(segment) {
                        "<FIXTURE_ELAPSED>s".to_string()
                    } else {
                        segment.to_string()
                    }
                })
                .collect();
            segments.join(" · ") + newline
        })
        .collect()
}

/// Replaces the numeric value following every `needle` occurrence with a fixed placeholder — a
/// measured wall-clock duration, not part of the pinned rendering: `elapsed_seconds`, and (task
/// 4.5) `semantic_query_ms`, the first screen's semantic call when its route answers.
fn redact_measured(text: &str, needle: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find(needle) {
        out.push_str(&rest[..pos + needle.len()]);
        rest = &rest[pos + needle.len()..];
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
const GOLDEN_FOCUSED: &str = "f4b05a76057fdeb31ea5fd8df1c428ff6df7ff45b0520ee8917efcfa09292124";
const GOLDEN_WHOLE: &str = "431796e6858934c0691ce6fcaa2cde82947af0bb7addb8967a6ea060340bd34d";
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
