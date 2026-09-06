//! The same fixture pair and variants as capacity_ratification_test.py, through the native CLI.
use std::{fs, path::Path, process::Command};

use serde_json::Value;
use tempfile::TempDir;

const LEDGER: &str = "genesis/data/rakia/compute-capacity.json";
const DEPLOYMENTS: &str = "genesis/orchestrator/data/deployments.json";
const MANIFEST: &str = "---\nepr-meta-version: 1\nroot: true\nrules:\n  - id: test-bench-aggregate-capacity\n    class: ask\n    validator: epr:validator-test-bench-aggregate-capacity\n---\n";

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn shared_capacity_ratification_vectors() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/capacity");
    let base: Value =
        serde_json::from_str(&fs::read_to_string(fixtures.join("compute-capacity.json")).unwrap())
            .unwrap();
    let deployments = fs::read_to_string(fixtures.join("deployments.json")).unwrap();
    let cases: Vec<Value> =
        serde_json::from_str(&fs::read_to_string(fixtures.join("cases.json")).unwrap()).unwrap();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let mut ledger = base.clone();
        for (pointer, replacement) in case["patches"].as_object().unwrap() {
            let (parent, key) = pointer.rsplit_once('/').unwrap();
            let obj = ledger.pointer_mut(parent).unwrap();
            if let Some(array) = obj.as_array_mut() {
                array[key.parse::<usize>().unwrap()] = replacement.clone();
            } else {
                obj[key] = replacement.clone();
            }
        }
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        // A git hook (pre-push) exports GIT_DIR/GIT_WORK_TREE/GIT_INDEX_FILE to every child;
        // inherited, they would make `git init` initialise the HOOK's repo instead of the temp
        // dir, and `epr govern` would then fail to find a worktree from /tmp (push #9,
        // 2026-09-06). The temp repo must be discovered by path, never by ambient env.
        assert!(Command::new("git")
            .args(["init", "-q"])
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .current_dir(root)
            .status()
            .unwrap()
            .success());
        write(root, ".epr-meta", MANIFEST);
        write(root, LEDGER, &ledger.to_string());
        write(root, DEPLOYMENTS, &deployments);
        for path in [LEDGER, DEPLOYMENTS] {
            let output = Command::new(env!("CARGO_BIN_EXE_epr"))
                .args(["govern", "--repo", root.to_str().unwrap(), "--path", path])
                .arg("--content-file")
                .arg(root.join(path))
                .env("EPR_META_NOW", case["now"].as_str().unwrap())
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .env_remove("GIT_INDEX_FILE")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{name}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let result: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(
                result["decision"], case["decision"],
                "{name} {path}: {result}"
            );
            assert_eq!(
                result["winningClass"], case["cls"],
                "{name} {path}: {result}"
            );
            let reason = result["reason"].as_str().unwrap_or("");
            for text in case["contains"].as_array().unwrap() {
                assert!(
                    reason.contains(text.as_str().unwrap()),
                    "{name} {path}: missing {text} in {reason}"
                );
            }
            if case["cls"] == "inject" {
                assert_eq!(result["referReason"], "stale-evidence", "{name}");
            }
        }
        if [
            "stale-31-days",
            "fresh-30-days",
            "stale-invalid-still-refuses",
        ]
        .contains(&name)
        {
            let output = Command::new(env!("CARGO_BIN_EXE_epr"))
                .args(["check", "--repo", root.to_str().unwrap(), "--json", LEDGER])
                .env("EPR_META_NOW", case["now"].as_str().unwrap())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{name}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let report: Value = serde_json::from_slice(&output.stdout).unwrap();
            let finding = report["findings"]
                .as_array()
                .unwrap()
                .iter()
                .find(|finding| finding["code"] == "governance.rule.test-bench-aggregate-capacity")
                .unwrap();
            assert_eq!(
                finding["blocksReach"],
                case["cls"] != "inject",
                "{name}: {finding}"
            );
            if case["cls"] == "inject" {
                assert_eq!(finding["status"], "refer", "{name}: {finding}");
            }
        }
        println!("GREEN {name} (both write directions)");
    }
}
