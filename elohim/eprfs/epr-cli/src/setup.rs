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
    let hook = root.join(".husky/pre-push");
    if !hook.is_file() {
        return Err(Error::InvalidArguments(
            "the repository has no checked-in `.husky/pre-push` reach gate".into(),
        ));
    }

    process::require(
        "git",
        &["config", "--local", "core.hooksPath", ".husky"],
        root,
    )?;

    let mut report = doctor::run(root)?;
    report.command = "setup".into();
    report.push(
        Finding::new(
            "git.hooks-activated",
            FindingStatus::Pass,
            "checked-in reach gate activated for this clone",
        )
        .detail("Only local Git configuration changed; working-tree authoring remains private."),
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
        let dir = TempDir::new().unwrap();
        process::require("git", &["init", "-q"], dir.path()).unwrap();
        for path in ["AGENTS.md", "CLAUDE.md"] {
            fs::write(dir.path().join(path), "governance\n").unwrap();
        }
        fs::create_dir_all(dir.path().join(".epr-meta/elohim/packages")).unwrap();
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
