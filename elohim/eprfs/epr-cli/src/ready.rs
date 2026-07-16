use std::path::Path;

use serde_json::json;

use crate::{
    check, doctor,
    error::Result,
    git::{self, ChangedPath},
    process,
    report::{Finding, FindingStatus, Report},
};

pub fn run(root: &Path, target: &str, changes: &[ChangedPath], deep: bool) -> Result<Report> {
    let doctor = doctor::run(root)?;
    let base = git::merge_base(root, target)?;
    let outcome = check::run_at(root, changes, "HEAD", &base)?;
    let mut report = Report::new("ready", root);
    report.extend(doctor.findings);
    report.extend(outcome.report.findings);

    if outcome.needs_package_verify {
        report.remove_code("package.verify.pending");
        if run_package_verify(&mut report, root) {
            // A generated projection changing is locally suspicious, but a
            // green fidelity proof establishes that it matches its package
            // authority for the reach candidate.
            report.remove_code("authority.projection-edited");
        }
    }

    if outcome.touches_eprfs {
        if deep {
            run_native_tests(&mut report, root);
        } else {
            report.push(
                Finding::new(
                    "quality.eprfs-deferred",
                    FindingStatus::Info,
                    "EPRFS Rust quality gates remain for pre-push/CI",
                )
                .remediation(
                    "run `epr ready --deep` for focused fmt + clippy + tests before requesting reach",
                ),
            );
        }
    }

    report.data = json!({
        "target": target,
        "mergeBase": base,
        "changedPaths": changes,
        "deep": deep,
        "meaning": "local governance-floor readiness; push/remote quality gates remain authoritative for shared reach"
    });
    report.recompute();
    Ok(report)
}

fn run_package_verify(report: &mut Report, root: &Path) -> bool {
    let script = "elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs";
    match process::run("node", &[script, "verify"], root) {
        Ok(output) if output.success => {
            report.push(Finding::new(
                "package.verify",
                FindingStatus::Pass,
                "agentic package projections are fidelity-green",
            ));
            true
        }
        Ok(output) => {
            report.push(
                Finding::new(
                    "package.verify",
                    FindingStatus::Fail,
                    "agentic package projection verification failed",
                )
                .detail(trim_output(&output.stderr, &output.stdout))
                .remediation("repair package/projection drift, then rerun `epr ready`")
                .blocking(),
            );
            false
        }
        Err(error) => {
            report.push(
                Finding::new(
                    "package.verify",
                    FindingStatus::Fail,
                    format!("could not run package verifier: {error}"),
                )
                .remediation("install Node dependencies with `pnpm install`")
                .blocking(),
            );
            false
        }
    }
}

fn run_native_tests(report: &mut Report, root: &Path) {
    let workspace = root.join("elohim/eprfs");
    let manifest = workspace.join("Cargo.toml");
    let manifest = manifest.to_string_lossy();
    let args = [
        "fmt",
        "--manifest-path",
        manifest.as_ref(),
        "--all",
        "--",
        "--check",
    ];
    let env = [
        ("RUSTFLAGS", ""),
        ("CARGO_TARGET_DIR", "/tmp/eprfs-gate-target"),
    ];
    if let Some(finding) = native_gate("quality.eprfs-fmt", "format", &args, &env, root) {
        report.push(finding);
        return;
    }

    let args = [
        "clippy",
        "--manifest-path",
        manifest.as_ref(),
        "-p",
        "eprfs-meta",
        "-p",
        "elohim-epr-cli",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ];
    if let Some(finding) = native_gate("quality.eprfs-clippy", "clippy", &args, &env, root) {
        report.push(finding);
        return;
    }

    let args = [
        "test",
        "--manifest-path",
        manifest.as_ref(),
        "-p",
        "eprfs-meta",
        "-p",
        "elohim-epr-cli",
    ];
    match process::run_with_env("cargo", &args, root, &env) {
        Ok(output) if output.success => report.push(Finding::new(
            "quality.eprfs-focused",
            FindingStatus::Pass,
            "focused native governance tests pass",
        )),
        Ok(output) => report.push(
            Finding::new(
                "quality.eprfs-focused",
                FindingStatus::Fail,
                "focused native governance tests failed",
            )
            .detail(trim_output(&output.stderr, &output.stdout))
            .blocking(),
        ),
        Err(error) => report.push(
            Finding::new(
                "quality.eprfs-focused",
                FindingStatus::Fail,
                format!("could not run focused Rust tests: {error}"),
            )
            .blocking(),
        ),
    }
}

fn native_gate(
    code: &str,
    name: &str,
    args: &[&str],
    env: &[(&str, &str)],
    root: &Path,
) -> Option<Finding> {
    match process::run_with_env("cargo", args, root, env) {
        Ok(output) if output.success => None,
        Ok(output) => Some(
            Finding::new(
                code,
                FindingStatus::Fail,
                format!("focused Rust {name} gate failed"),
            )
            .detail(trim_output(&output.stderr, &output.stdout))
            .blocking(),
        ),
        Err(error) => Some(
            Finding::new(
                code,
                FindingStatus::Fail,
                format!("could not run Rust {name}: {error}"),
            )
            .blocking(),
        ),
    }
}

fn trim_output(stderr: &str, stdout: &str) -> String {
    let value = if stderr.is_empty() { stdout } else { stderr };
    const LIMIT: usize = 2_000;
    if value.len() <= LIMIT {
        value.to_string()
    } else {
        let start = value
            .char_indices()
            .map(|(index, _)| index)
            .rev()
            .find(|index| value.len() - index <= LIMIT)
            .unwrap_or(0);
        format!("…{}", &value[start..])
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    use super::*;
    use crate::{git, process, setup};

    #[test]
    fn committed_deny_finding_blocks_reach_but_not_local_authoring() {
        let dir = TempDir::new().unwrap();
        process::require("git", &["init", "-q"], dir.path()).unwrap();
        process::require(
            "git",
            &["config", "user.email", "epr@test.invalid"],
            dir.path(),
        )
        .unwrap();
        process::require("git", &["config", "user.name", "EPR Test"], dir.path()).unwrap();
        fs::write(dir.path().join("AGENTS.md"), "governance\n").unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "governance\n").unwrap();
        fs::create_dir_all(dir.path().join(".epr-meta/elohim/packages")).unwrap();
        fs::write(
            dir.path().join(".epr-meta/manifest.md"),
            "---\nepr-meta-version: 1\nroot: true\nrules:\n  - id: born-linked\n    class: deny\n    when: { write: \"*.md\", new: true }\n    require-frontmatter: [id]\n---\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join(".husky")).unwrap();
        let hook = dir.path().join(".husky/pre-push");
        fs::write(&hook, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
        setup::run(dir.path()).unwrap();
        process::require(
            "git",
            &["add", "AGENTS.md", "CLAUDE.md", ".epr-meta", ".husky"],
            dir.path(),
        )
        .unwrap();
        process::require("git", &["commit", "-q", "-m", "baseline"], dir.path()).unwrap();
        let base = process::require("git", &["rev-parse", "HEAD"], dir.path())
            .unwrap()
            .stdout;

        fs::write(dir.path().join("bad.md"), "no frontmatter\n").unwrap();
        process::require("git", &["add", "bad.md"], dir.path()).unwrap();
        process::require("git", &["commit", "-q", "-m", "candidate"], dir.path()).unwrap();
        let changes = git::changes_against(dir.path(), &base).unwrap();

        let report = run(dir.path(), &base, &changes, false).unwrap();

        assert!(!report.ready);
        assert!(report.findings.iter().any(|finding| {
            finding.code == "governance.rule.born-linked" && finding.blocks_reach
        }));
    }
}
