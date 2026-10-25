//! Explicit, local-only repository onboarding.

use std::path::Path;

use crate::{
    doctor,
    error::{Error, Result},
    process,
    report::{Finding, FindingStatus, Report},
};

/// Activate the repository's checked-in reach gate without installing a
/// package manager or writing outside this clone. This changes only local Git
/// configuration and is therefore both explicit and reversible.
pub fn run(root: &Path) -> Result<Report> {
    let existing = doctor::configured_hooks_path(root)?;
    let configured = existing.clone().unwrap_or_else(|| {
        if root.join(".husky/pre-push").is_file() {
            ".husky".into()
        } else {
            ".githooks".into()
        }
    });
    let hook = root.join(&configured).join("pre-push");
    if !hook.is_file() || !doctor::is_executable(&hook) {
        return Err(Error::InvalidArguments(
            format!("no executable pre-push reach gate at {}; repair the declared hook or provide .husky/pre-push or .githooks/pre-push", hook.display()),
        ));
    }
    if existing.is_none() {
        process::require(
            "git",
            &["config", "--local", "core.hooksPath", &configured],
            root,
        )?;
    }

    let mut report = doctor::run(root)?;
    report.command = "setup".into();
    report.push(
        Finding::new(
            "git.hooks-activated",
            FindingStatus::Pass,
            "checked-in reach gate activated for this clone",
        )
        .detail(if existing.is_none() {
            "Only local Git configuration changed; working-tree authoring remains private."
        } else {
            "Existing hook configuration preserved; no Git configuration changed."
        }),
    );
    report.recompute();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use std::fs;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn activates_only_the_clone_local_checked_in_hook() {
        // `git -c …` exports its overrides to every child as GIT_CONFIG_PARAMETERS, so a
        // test run under `git -c core.hooksPath=… push` would read that override instead
        // of the clone-local setting. Re-run this test alone without it.
        if std::env::var_os("GIT_CONFIG_PARAMETERS").is_some() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "setup::tests::activates_only_the_clone_local_checked_in_hook",
                ])
                .env_remove("GIT_CONFIG_PARAMETERS")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stdout)
            );
            return;
        }
        let dir = TempDir::new().unwrap();
        process::require("git", &["init", "-q"], dir.path()).unwrap();
        for path in ["AGENTS.md", "CLAUDE.md"] {
            fs::write(dir.path().join(path), "governance\n").unwrap();
        }
        fs::create_dir_all(dir.path().join(".epr-meta/elohim/packages")).unwrap();
        let verifier = dir.path().join(crate::authority::PACKAGE_VERIFIER);
        fs::create_dir_all(verifier.parent().unwrap()).unwrap();
        fs::write(verifier, "// package verifier fixture\n").unwrap();
        fs::write(
            dir.path().join(".epr-meta/manifest.md"),
            "---\nepr-meta-version: 1\nroot: true\n---\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join(".husky")).unwrap();
        let hook = dir.path().join(".husky/pre-push");
        fs::write(&hook, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();

        let report = run(dir.path()).unwrap();
        let configured = process::require(
            "git",
            &["config", "--local", "--get", "core.hooksPath"],
            dir.path(),
        )
        .unwrap();

        assert_eq!(configured.stdout, ".husky");
        assert!(report.ready);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "git.hooks-activated"));
    }
}
