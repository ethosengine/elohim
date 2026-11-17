//! The build-input view (`elohim/eprfs/bin-build-view.json`) covers what a build of `epr` reads.
//!
//! A mesh-build method keys a binary on a view of the repository. Keyed on the whole eprfs
//! workspace, the view held ignored generated outputs (a ts-rs `bindings/`, a package's `dist/`)
//! and the generated-output scan refused every key (2026-11-17). The view file lists only what
//! `cargo build --bin` reads, so those directories are outside it and the scan has nothing to
//! refuse. That is sound only while the list is complete: a build that reads a tracked file the
//! view does not name would be recorded under a key that does not cover it.
//!
//! Cargo writes the binary's dep-info (`epr.d`) beside the binary it builds for these integration
//! tests. This test reads it and refuses any file inside the repository that the view leaves out.
//! The first hand-written list missed two (the device manifest the binary embeds, and the
//! rust-style `scan` crate); this is the check that would have named them.

use std::path::{Path, PathBuf};

const VIEW_REL: &str = "elohim/eprfs/bin-build-view.json";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root")
}

fn view_sources(root: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(root.join(VIEW_REL)).expect("the view file reads");
    let doc: serde_json::Value = serde_json::from_str(&text).expect("the view file is JSON");
    doc["steps"]["bin-build"]["inputs"]["sources"]
        .as_array()
        .expect("steps.bin-build.inputs.sources is a list")
        .iter()
        .map(|entry| entry.as_str().expect("a source is a string").to_owned())
        .collect()
}

/// A source is an exact path, or a subtree when it ends in `/**`.
fn covers(sources: &[String], path: &str) -> bool {
    sources
        .iter()
        .any(|source| match source.strip_suffix("/**") {
            Some(dir) => path
                .strip_prefix(dir)
                .is_some_and(|rest| rest.starts_with('/')),
            None => source == path,
        })
}

/// Dep-info is one Make rule per output: `target: dep dep …`, a space in a path escaped as `\ `.
fn dep_info_inputs(text: &str) -> Vec<PathBuf> {
    let mut inputs = Vec::new();
    for line in text.lines() {
        let Some((_, deps)) = line.split_once(": ") else {
            continue;
        };
        let mut current = String::new();
        let mut chars = deps.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\\' if chars.peek() == Some(&' ') => {
                    current.push(' ');
                    chars.next();
                }
                ' ' => {
                    if !current.is_empty() {
                        inputs.push(PathBuf::from(std::mem::take(&mut current)));
                    }
                }
                other => current.push(other),
            }
        }
        if !current.is_empty() {
            inputs.push(PathBuf::from(current));
        }
    }
    inputs
}

#[test]
fn the_view_names_no_test_bench_or_example_directory() {
    // gate-reads: elohim/eprfs/bin-build-view.json
    let sources = view_sources(&repo_root());
    assert!(!sources.is_empty(), "the view lists nothing");
    for source in &sources {
        for part in source.split('/') {
            assert!(
                !matches!(part, "tests" | "benches" | "examples" | "bindings" | "dist"),
                "{source} brings `{part}` into the build-input view; a binary build does not read \
                 it, and an ignored generated output inside the view makes every key refuse"
            );
        }
    }
}

#[test]
fn every_tracked_file_a_build_of_epr_reads_is_inside_the_view() {
    // The view file is the one repository file read here. The dep-info lies in the target
    // directory; the paths it names are compared as strings and never opened.
    // gate-reads: elohim/eprfs/bin-build-view.json
    let root = repo_root();
    let sources = view_sources(&root);
    let dep_info = PathBuf::from(format!("{}.d", env!("CARGO_BIN_EXE_epr")));
    let text = std::fs::read_to_string(&dep_info).unwrap_or_else(|error| {
        panic!(
            "no dep-info at {}: {error}. Cargo writes it beside the epr binary it builds for \
             these tests; without it this test cannot say what the build read",
            dep_info.display()
        )
    });
    let inputs = dep_info_inputs(&text);
    let mut inside = 0_usize;
    let mut uncovered = Vec::new();
    for input in &inputs {
        let Ok(resolved) = input.canonicalize() else {
            continue;
        };
        let Ok(rel) = resolved.strip_prefix(&root) else {
            continue; // the registry, the toolchain, the target directory outside the checkout
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        if rel.starts_with("target/") || rel.contains("/target/") {
            continue; // build output written under an in-tree target directory
        }
        inside += 1;
        if !covers(&sources, &rel) {
            uncovered.push(rel);
        }
    }
    assert!(
        inside > 100,
        "the dep-info at {} names only {inside} files inside the repository; it is not the epr \
         binary's, or the parse is broken",
        dep_info.display()
    );
    uncovered.sort();
    uncovered.dedup();
    assert!(
        uncovered.is_empty(),
        "a build of epr read tracked files outside {VIEW_REL}; add each (a crate as its \
         Cargo.toml and src/**, an embedded file by its exact path):\n  {}",
        uncovered.join("\n  ")
    );
}
