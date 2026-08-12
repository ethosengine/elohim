//! Rust runner for the governance golden-vector corpus — the cross-runtime
//! correspondence theorem (`elohim/sdk/schemas/v1/registries/governance-parity-vectors.json`).
//!
//! The Python PreToolUse hook, the git-gate, and this native evaluator must all
//! derive the SAME `{decision, winning_class, rule_id}` (plus `witnessed` and,
//! where asserted, `refer_reason`) from the SAME manifest cascades. Two runtimes
//! deriving different verdicts is a forked governance plane.
//!
//! Parity law (Global Constraint 5): a vector this runtime genuinely cannot
//! execute is an EXPLICIT skip with a printed reason — never silent green.
//!
//! Provider note — WHY THIS LIVES IN `epr-cli` AND NOT `eprfs-meta`. It used to
//! run against `NoValidators`, on the reasoning that the corpus's only validator
//! reference was deliberately nonexistent so a null provider and the real one
//! agreed. That reasoning was circular: a null provider returns `Unavailable` for
//! EVERY reference, so the harness could not have detected a validator-provider
//! divergence of any kind, and the corpus had been shaped around exactly that
//! blind spot. The evaluator whose parity actually matters is the COMPOSED one —
//! library + `ElohimRepositoryValidators` — which is what the gate runs and what
//! the hook calls, and it exists here at the composition root.
//!
//! Consequence for fixtures: each vector's temp root is a faithful mini-repo, so
//! the shared validator-scope registry is copied into it. The provider resolves
//! scope from `repo_root` exactly as it does in production — no test-only path.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use elohim_epr_cli::repository_validators::ElohimRepositoryValidators;
use eprfs_meta::{evaluate_path_with, resolve_decision, GovernanceWrite};
use serde::Deserialize;
use tempfile::TempDir;

/// Vectors this runtime cannot execute, with the reason (printed, never green).
///
/// Empty since 2026-08-12. `malformed-manifest-refers` used to live here: serde
/// silently ignores unknown rule keys, so eprfs-meta could not detect a
/// rule-integrity failure and the vector was skipped as "the Python `check_meta`
/// surface". That skip was load-bearing in the wrong direction — it meant the
/// native evaluator PERMITTED writes that Python routed to judgment, and the
/// corpus recorded the gap as out-of-scope rather than as the fork it was.
/// `eprfs_meta::integrity` closes it.
fn skip_reason(_name: &str) -> Option<&'static str> {
    None
}

#[derive(Debug, Deserialize)]
struct Corpus {
    vectors: Vec<Vector>,
}

#[derive(Debug, Deserialize)]
struct Vector {
    name: String,
    #[serde(default)]
    manifests: BTreeMap<String, String>,
    write: WriteSpec,
    expect: Expect,
}

#[derive(Debug, Deserialize)]
struct WriteSpec {
    path: String,
    #[serde(default)]
    new: bool,
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Expect {
    decision: String,
    winning_class: Option<String>,
    rule_id: Option<String>,
    #[serde(default)]
    witnessed: Option<bool>,
    #[serde(default)]
    refer_reason: Option<String>,
    #[serde(default)]
    directive: Option<String>,
}

/// CARGO_MANIFEST_DIR = elohim/eprfs/epr-cli → up two to the elohim/ crate root,
/// then into the SDK registries.
fn registry_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../sdk/schemas/v1/registries")
        .join(name)
}

fn vectors_path() -> PathBuf {
    registry_path("governance-parity-vectors.json")
}

/// Repo-relative home of the validator-scope map, mirrored into each fixture root
/// so the provider's own resolution path is what gets exercised.
const VALIDATOR_SCOPE_REL: &str = "elohim/sdk/schemas/v1/registries/governance-validators.json";

fn seed_validator_scope(root: &std::path::Path) {
    let source = registry_path("governance-validators.json");
    let Ok(body) = fs::read_to_string(&source) else {
        return; // absent registry → provider sees no declarations → routes. Safe direction.
    };
    let target = root.join(VALIDATOR_SCOPE_REL);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(target, body).unwrap();
}

/// Wrap a vector's bare YAML rules body into a valid `.epr-meta` frontmatter
/// document. The repo-root manifest is marked `root: true`; nested manifests are
/// children of their directory.
fn wrap_manifest(rel_path: &str, body: &str) -> String {
    let root_line = if rel_path == ".epr-meta" {
        "root: true\n"
    } else {
        ""
    };
    format!("---\nepr-meta-version: 1\n{root_line}{body}\n---\n")
}

#[test]
fn governance_parity_vectors_hold() {
    let path = vectors_path();
    if !path.is_file() {
        println!(
            "SKIP-ALL governance parity: vectors file absent at {} — nothing to verify.",
            path.display()
        );
        return;
    }

    let corpus: Corpus = serde_json::from_str(&fs::read_to_string(&path).expect("read vectors"))
        .expect("parse vectors");

    let mut passed = Vec::new();
    let mut skipped = Vec::new();
    let mut failures = Vec::new();

    for vector in &corpus.vectors {
        if let Some(reason) = skip_reason(&vector.name) {
            skipped.push((vector.name.clone(), reason));
            continue;
        }

        let dir = TempDir::new().unwrap();
        seed_validator_scope(dir.path());
        for (rel, body) in &vector.manifests {
            let full = dir.path().join(rel);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&full, wrap_manifest(rel, body)).unwrap();
        }

        let target = dir.path().join(&vector.write.path);
        let mut write = GovernanceWrite::new(vector.write.path.clone());
        write.is_new = vector.write.new;
        write.content = vector.write.content.clone();

        let evaluation =
            match evaluate_path_with(dir.path(), &target, &write, &ElohimRepositoryValidators) {
                Ok(evaluation) => evaluation,
                Err(error) => {
                    failures.push(format!("{}: evaluation errored: {error}", vector.name));
                    continue;
                }
            };
        let resolved = resolve_decision(&evaluation.verdicts);

        let mut mismatches = Vec::new();
        if resolved.decision != vector.expect.decision {
            mismatches.push(format!(
                "decision expected {:?} got {:?}",
                vector.expect.decision, resolved.decision
            ));
        }
        if resolved.winning_class != vector.expect.winning_class {
            mismatches.push(format!(
                "winning_class expected {:?} got {:?}",
                vector.expect.winning_class, resolved.winning_class
            ));
        }
        if resolved.rule_id != vector.expect.rule_id {
            mismatches.push(format!(
                "rule_id expected {:?} got {:?}",
                vector.expect.rule_id, resolved.rule_id
            ));
        }
        if let Some(witnessed) = vector.expect.witnessed {
            let got = resolved.winning_class.is_some();
            if got != witnessed {
                mismatches.push(format!("witnessed expected {witnessed} got {got}"));
            }
        }
        if let Some(refer_reason) = &vector.expect.refer_reason {
            if resolved.refer_reason.as_deref() != Some(refer_reason.as_str()) {
                mismatches.push(format!(
                    "refer_reason expected {:?} got {:?}",
                    refer_reason, resolved.refer_reason
                ));
            }
        }
        // `directive` (dispatch self-dispatch emission) is a hook-surface concern,
        // not part of the library verdict triple; the dispatch class IS carried in
        // winning_class, which we do assert. Note it rather than assert it.
        let directive_note = vector
            .expect
            .directive
            .as_deref()
            .map(|d| format!(" (directive `{d}` is hook-surface, not asserted)"))
            .unwrap_or_default();

        if mismatches.is_empty() {
            passed.push(format!("{}{directive_note}", vector.name));
        } else {
            failures.push(format!("{}: {}", vector.name, mismatches.join("; ")));
        }
    }

    println!("\n=== governance parity vectors ===");
    println!("PASSED ({}):", passed.len());
    for name in &passed {
        println!("  ✓ {name}");
    }
    println!("SKIPPED ({}):", skipped.len());
    for (name, reason) in &skipped {
        println!("  → {name}: {reason}");
    }
    if !failures.is_empty() {
        println!("FAILED ({}):", failures.len());
        for failure in &failures {
            println!("  ✗ {failure}");
        }
    }
    println!("=================================\n");

    assert!(
        failures.is_empty(),
        "governance parity mismatches: {failures:?}"
    );
}
