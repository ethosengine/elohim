//! Help is a QUESTION, not an error.
//!
//! An agent discovering this CLI walks down four layers — `epr`, `epr flow`, `epr flow memory`,
//! `epr flow memory recall` — and before 2026-09-11 three of them answered "what do you offer" with
//! usage on STDERR and a non-zero exit. A refusal per layer is not a discovery surface: a caller
//! that treats a non-zero exit as failure (a script, a hook, an agent with a bounded retry budget)
//! learns that asking was a mistake. Each layer is asserted separately because each one is a
//! different dispatcher, and a shared helper would prove only that the helper works.

use std::process::Command;

fn answer(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(args)
        .output()
        .expect("epr runs");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert_eq!(
        out.status.code(),
        Some(0),
        "`epr {}` exited non-zero\nstdout: {stdout}\nstderr: {stderr}",
        args.join(" ")
    );
    assert!(
        stderr.is_empty(),
        "`epr {}` answered on stderr: {stderr}",
        args.join(" ")
    );
    assert!(
        stdout.contains("usage:"),
        "`epr {}` printed no usage: {stdout}",
        args.join(" ")
    );
    stdout
}

#[test]
fn the_root_command_answers_help_on_stdout() {
    let out = answer(&["--help"]);
    assert!(out.contains("flow"), "{out}");
    assert_eq!(answer(&["-h"]), out);
}

#[test]
fn the_flow_family_answers_help_on_stdout() {
    let out = answer(&["flow", "--help"]);
    assert!(out.contains("memory"), "{out}");
}

#[test]
fn the_memory_shell_answers_help_and_names_its_operations() {
    let out = answer(&["flow", "memory", "--help"]);
    assert!(out.contains("recall"), "{out}");
    // A caller who named NO operation asked the same question and gets the same answer.
    let bare = answer(&["flow", "memory"]);
    assert!(bare.contains("collective"), "{bare}");
    assert!(bare.contains("recall"), "{bare}");
}

#[test]
fn the_recall_executor_answers_help_on_stdout() {
    let out = answer(&["flow", "memory", "recall", "--help"]);
    assert!(out.contains("--session"), "{out}");
    assert!(out.contains("open"), "{out}");
}
