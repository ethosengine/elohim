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

pub fn run_with_env(
    program: &str,
    args: &[&str],
    cwd: &Path,
    env: &[(&str, &str)],
) -> Result<CommandOutput> {
    let mut command = Command::new(program);
    command.args(args).current_dir(cwd);
    for (key, value) in env {
        command.env(key, value);
    }
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
