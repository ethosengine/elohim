//! Every governance manifest actually committed to this repo is integrity-clean
//! under the native evaluator.
//!
//! The golden-vector corpus proves correspondence on twelve SYNTHETIC cascades.
//! That is necessary and not sufficient: a vector suite is silent on what it has
//! no vector for, and the thing an integrity layer most easily gets wrong is
//! being subtly STRICTER than its twin on real input. A single over-strict check
//! here would make `epr check` route every write under the affected subtree —
//! a repo-wide false block that no synthetic vector would show.
//!
//! So this test runs the native `validate_meta` over the live corpus. Its Python
//! twin is `_lib.epr_meta.check_meta`. When the two layers were first brought
//! into correspondence (2026-08-12) they were run over the same 123 manifests and
//! agreed exactly — same files flagged, same rule indices, same message text.
//! That comparison was a one-off measurement, not a standing check: this test
//! asserts only the native side. A drift in the Python layer shows up in the
//! golden-vector corpus, which both hosts run.
//!
//! `.claude/worktrees/` is excluded for the same reason the seam census excludes
//! it: those are whole stale repo copies belonging to other agent sessions, not
//! this tree. Scanning them measures a snapshot of the past — and they are in
//! fact where the only two malformed manifests live, both predating the L6
//! `measure.kind` requirement.

use std::path::{Path, PathBuf};

/// Directory names that never contain governance this tree is responsible for.
const SKIP_DIRS: [&str; 5] = [".git", "node_modules", "target", ".angular", "dist"];

/// Repo-relative subtrees excluded by path (not by bare name, which would prune
/// any directory that happened to share it).
const SKIP_REL: [&str; 1] = [".claude/worktrees"];

fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if SKIP_REL.contains(&rel.as_str()) {
                continue;
            }
            walk(root, &path, out);
        } else if name == eprfs_meta::MANIFEST_NAME {
            out.push(path);
        }
    }
}

fn frontmatter_body(text: &str) -> Option<&str> {
    text.strip_prefix("---\n")
        .and_then(|rest| rest.split("\n---").next())
}

#[test]
fn live_manifests_are_integrity_clean() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repo root");
    let mut manifests = Vec::new();
    walk(&root, &root, &mut manifests);
    manifests.sort();

    assert!(
        !manifests.is_empty(),
        "found no .epr-meta manifests under {} — the walk is broken, not the repo clean",
        root.display()
    );

    let mut failures = Vec::new();
    for manifest in &manifests {
        let rel = manifest.strip_prefix(&root).unwrap_or(manifest).display();
        let Ok(text) = std::fs::read_to_string(manifest) else {
            failures.push(format!("{rel}: unreadable"));
            continue;
        };
        let Some(body) = frontmatter_body(&text) else {
            failures.push(format!("{rel}: no frontmatter block"));
            continue;
        };
        match serde_yaml::from_str::<serde_yaml::Value>(body) {
            Ok(cfg) => {
                let problems = eprfs_meta::validate_meta(&cfg);
                if !problems.is_empty() {
                    failures.push(format!("{rel}: {}", problems.join(" | ")));
                }
            }
            Err(error) => failures.push(format!("{rel}: unparseable YAML: {error}")),
        }
    }

    println!(
        "live manifest integrity: {} scanned, {} malformed",
        manifests.len(),
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "malformed governance manifests in the live tree:\n  {}",
        failures.join("\n  ")
    );
}
