use std::{path::Path, process::Command};

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub fn run(program: &str, args: &[&str], cwd: &Path) -> Result<CommandOutput> {
    run_with_env(program, args, cwd, &[])
}

/// Environment variables by which git overrides repository discovery. git EXPORTS
/// these into every hook's environment, and `epr` runs from `.husky` hooks — so
/// inheriting them silently redirects a command away from the `cwd` the caller
/// named, at the outer repo. Cleared on every spawn so `cwd` is authoritative.
const GIT_LOCATION_ENV: [&str; 8] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_NAMESPACE",
    "GIT_PREFIX",
];

/// Build a child command with `cwd` made authoritative — the ONLY sanctioned way
/// to construct a git subprocess in this crate.
///
/// Reach for this instead of `Command::new("git")`: a bare `Command` inherits the
/// caller's `GIT_DIR`, and `epr` runs from git hooks, which is precisely where git
/// exports it.
pub fn build_command(program: &str, args: &[&str], cwd: &Path, env: &[(&str, &str)]) -> Command {
    let mut command = Command::new(program);
    command.args(args).current_dir(cwd);
    // Clear FIRST, so an explicitly-passed variable still wins below.
    for key in GIT_LOCATION_ENV {
        command.env_remove(key);
    }
    for (key, value) in env {
        command.env(key, value);
    }
    command
}

pub fn run_with_env(
    program: &str,
    args: &[&str],
    cwd: &Path,
    env: &[(&str, &str)],
) -> Result<CommandOutput> {
    let mut command = build_command(program, args, cwd, env);
    let output = command.output().map_err(|source| Error::Spawn {
        program: program.into(),
        source,
    })?;
    Ok(CommandOutput {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

pub fn require(program: &str, args: &[&str], cwd: &Path) -> Result<CommandOutput> {
    let output = run(program, args, cwd)?;
    if output.success {
        Ok(output)
    } else {
        Err(Error::Command {
            program: program.into(),
            stderr: output.stderr,
        })
    }
}

pub fn version(program: &str, cwd: &Path) -> Option<String> {
    run(program, &["--version"], cwd)
        .ok()
        .filter(|output| output.success)
        .map(|output| first_line(&output.stdout))
}

fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    /// Every git repo-location variable must be cleared, so the `cwd` we were
    /// handed is what git acts on.
    ///
    /// git EXPORTS `GIT_DIR` (and friends) into the environment of every hook it
    /// runs. `epr` is invoked from `.husky` hooks, and the eprfs test suite runs
    /// under the pre-push hook — so without this, a `git init` in a TempDir was
    /// silently redirected at the ambient repo and died on its config lock
    /// ("could not lock config file …/.git/config: File exists"). The gate could
    /// not pass on any push that touched eprfs, while the same test passed
    /// standalone.
    #[test]
    fn git_repo_location_env_is_cleared_so_cwd_is_authoritative() {
        let command = build_command("git", &["status"], Path::new("/tmp"), &[]);
        let cleared: Vec<&OsStr> = command
            .get_envs()
            .filter(|(_, value)| value.is_none())
            .map(|(key, _)| key)
            .collect();
        for key in GIT_LOCATION_ENV {
            assert!(
                cleared.contains(&OsStr::new(key)),
                "{key} must be cleared or an ambient value silently redirects git away from cwd"
            );
        }
    }

    /// An explicitly-passed variable still wins — clearing is a floor, not a cage.
    #[test]
    fn explicit_env_overrides_the_clearing() {
        let command = build_command("git", &["status"], Path::new("/tmp"), &[("GIT_DIR", "/x")]);
        let explicit = command
            .get_envs()
            .find(|(key, _)| *key == OsStr::new("GIT_DIR"))
            .expect("GIT_DIR entry present");
        assert_eq!(
            explicit.1,
            Some(OsStr::new("/x")),
            "an explicit GIT_DIR must survive the clearing"
        );
    }
}
