use super::{files, Refusal, Result, Store};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// One bounded external operation; credentials belong only in env.
#[derive(Clone)]
pub struct CommandSpec {
    /// Program name or absolute path.
    pub program: String,
    /// Arguments, excluding argv[0].
    pub args: Vec<String>,
    /// Optional working directory.
    pub cwd: Option<PathBuf>,
    /// Environment overrides; None removes a variable.
    pub env: BTreeMap<String, Option<String>>,
    /// Maximum elapsed runtime before the process group is killed.
    pub timeout: Duration,
}
impl CommandSpec {
    pub(super) fn new(
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
            cwd: None,
            env: BTreeMap::new(),
            timeout: Duration::from_secs(60),
        }
    }
}
/// Output at the subprocess boundary.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// True only for exit zero.
    pub success: bool,
    /// Standard output.
    pub stdout: String,
    /// Standard error, captured rather than printed (may contain external credentials).
    pub stderr: String,
}
/// External operations needed for source checks, health, builds and attestations.
pub trait Runner: Send + Sync {
    /// Execute one bounded command. Implementations must not print environment credentials.
    fn run(&self, command: &CommandSpec) -> Result<CommandOutput>;
}
/// Native subprocess implementation with a process-group timeout.
pub struct SystemRunner;
impl Runner for SystemRunner {
    fn run(&self, spec: &CommandSpec) -> Result<CommandOutput> {
        use std::os::unix::process::CommandExt;
        let mut command = Command::new(&spec.program);
        command
            .args(&spec.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
        if let Some(cwd) = &spec.cwd {
            command.current_dir(cwd);
        }
        for (k, v) in &spec.env {
            if let Some(v) = v {
                command.env(k, v);
            } else {
                command.env_remove(k);
            }
        }
        let mut child = command
            .spawn()
            .map_err(|e| Refusal::new("command-unavailable", format!("{}: {e}", spec.program)))?;
        // Drain both pipes concurrently: a compiler can fill either pipe before exiting.
        let out = child.stdout.take();
        let err = child.stderr.take();
        let read = |pipe: Option<Box<dyn std::io::Read + Send>>| {
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                if let Some(mut pipe) = pipe {
                    let _ = pipe.read_to_end(&mut bytes);
                }
                bytes
            })
        };
        let out = read(out.map(|p| Box::new(p) as Box<dyn std::io::Read + Send>));
        let err = read(err.map(|p| Box::new(p) as Box<dyn std::io::Read + Send>));
        let start = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break Some(status);
            }
            if start.elapsed() >= spec.timeout {
                // SAFETY: the child was placed in its own group at spawn; the negative pid targets only that group.
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.wait();
                break None;
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        let stdout = String::from_utf8_lossy(&out.join().unwrap_or_default()).into_owned();
        let stderr = String::from_utf8_lossy(&err.join().unwrap_or_default()).into_owned();
        let status = status.ok_or_else(|| {
            Refusal::new(
                "command-timeout",
                format!("{} exceeded {}s", spec.program, spec.timeout.as_secs()),
            )
        })?;
        Ok(CommandOutput {
            success: status.success(),
            stdout,
            stderr,
        })
    }
}
impl Store {
    pub(super) fn run(&self, spec: &CommandSpec) -> Result<CommandOutput> {
        self.runner.run(spec)
    }
    pub(super) fn git(&self, repo: &Path, args: &[&str]) -> Result<String> {
        let out = self.git_output(repo, args)?;
        if !out.success {
            return Err(Refusal::new(
                "git-unavailable",
                format!("git {} failed", args.first().unwrap_or(&"")),
            ));
        }
        Ok(out.stdout.trim().into())
    }
    pub(super) fn git_output(&self, repo: &Path, args: &[&str]) -> Result<CommandOutput> {
        let mut spec = CommandSpec::new("git", args.iter().copied());
        spec.cwd = Some(repo.into());
        // A hook's git plumbing must not redirect source checks into the superproject.
        for key in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_PREFIX"] {
            spec.env.insert(key.into(), None);
        }
        spec.timeout = Duration::from_secs(15);
        self.run(&spec)
    }
    pub(super) fn git_ok(&self, repo: &Path, args: &[&str]) -> bool {
        self.git_output(repo, args).is_ok_and(|r| r.success)
    }
    /// Resolve the current repository root, honoring EPR_REPO_ROOT.
    pub fn repo_root(&self, repo: Option<&Path>) -> Result<PathBuf> {
        let start = files::env("EPR_REPO_ROOT")
            .map(PathBuf::from)
            .or_else(|| repo.map(Path::to_owned))
            .unwrap_or(std::env::current_dir()?);
        Ok(PathBuf::from(
            self.git(&start, &["rev-parse", "--show-toplevel"])?,
        ))
    }
    /// Source closure from Cargo metadata, falling back conservatively to the entire checkout.
    pub fn source_paths(&self, repo: &Path) -> Vec<String> {
        let manifest = repo.join("elohim/eprfs/Cargo.toml");
        let mut spec = CommandSpec::new(
            "cargo",
            [
                "metadata",
                "--format-version",
                "1",
                "--offline",
                "--manifest-path",
                &manifest.to_string_lossy(),
            ],
        );
        spec.env.insert("RUSTFLAGS".into(), Some(String::new()));
        let result = self
            .run(&spec)
            .ok()
            .filter(|o| o.success)
            .and_then(|o| serde_json::from_str::<Value>(&o.stdout).ok());
        let Some(packages) = result
            .as_ref()
            .and_then(|v| v["packages"].as_array())
            .filter(|v| !v.is_empty())
        else {
            return vec![".".into()];
        };
        let mut paths = std::collections::BTreeSet::from(["elohim/eprfs".into()]);
        let root = repo.canonicalize().unwrap_or(repo.into());
        for package in packages.iter().filter(|v| v["source"].is_null()) {
            let Some(manifest) = package["manifest_path"].as_str() else {
                return vec![".".into()];
            };
            let Some(dir) = Path::new(manifest).parent() else {
                return vec![".".into()];
            };
            let canonical = dir.canonicalize().unwrap_or(dir.into());
            let Ok(relative) = canonical.strip_prefix(&root) else {
                return vec![".".into()];
            };
            paths.insert(relative.to_string_lossy().into_owned());
        }
        paths.into_iter().collect()
    }
    /// Committed source identity and cleanliness; hook ledgers outside the build closure do not count.
    pub fn source_state(&self, repo: &Path) -> Result<Value> {
        if self.config.source["kind"] == "gitlink" {
            let path = self.config.source["path"]
                .as_str()
                .ok_or_else(|| Refusal::new("gitlink-invalid", "missing source.path"))?;
            let line = self.git(repo, &["ls-tree", "HEAD", "--", path])?;
            let sha = line
                .strip_prefix("160000 commit ")
                .and_then(|s| s.split('\t').next())
                .filter(|s| s.len() == 40)
                .ok_or_else(|| Refusal::new("gitlink-invalid", path))?;
            return Ok(json!({"head":sha,"clean":true,"status":""}));
        }
        let head = self.git(repo, &["rev-parse", "HEAD"])?;
        let paths = self.source_paths(repo);
        let mut args = vec!["status", "--porcelain", "--untracked-files=normal", "--"];
        args.extend(paths.iter().map(String::as_str));
        let dirty = self.git(repo, &args)?;
        Ok(json!({"head":head,"clean":dirty.is_empty(),"status":dirty}))
    }
    pub(super) fn descends(&self, repo: &Path, ancestor: &str, descendant: &str) -> Result<bool> {
        if self.tool != "brit" {
            return Ok(self.git_ok(repo, &["merge-base", "--is-ancestor", ancestor, descendant]));
        }
        let dir = self.brit_git_dir(repo)?;
        let missing: Vec<_> = [ancestor, descendant]
            .into_iter()
            .filter(|sha| !self.git_ok(&dir, &["cat-file", "-e", &format!("{sha}^{{commit}}")]))
            .collect();
        if !missing.is_empty() {
            let mut args = vec!["fetch", "origin"];
            args.extend(missing);
            if !self.git_ok(&dir, &args) {
                return Err(Refusal::new(
                    "brit-fetch-failed",
                    "fetch missing brit commits with git fetch origin <sha>",
                ));
            }
        }
        Ok(self.git_ok(&dir, &["merge-base", "--is-ancestor", ancestor, descendant]))
    }
    pub(super) fn brit_git_dir(&self, repo: &Path) -> Result<PathBuf> {
        if let Some(dir) = files::env("BRIT_GIT_DIR") {
            return files::absolute(Path::new(&dir));
        }
        let common = PathBuf::from(self.git(repo, &["rev-parse", "--git-common-dir"])?);
        Ok(if common.is_absolute() {
            common
        } else {
            repo.join(common)
        }
        .join("modules/elohim/brit"))
    }
    pub(super) fn build_target(&self, repo: &Path) -> Result<PathBuf> {
        if let Some(target) = files::env("CARGO_TARGET_DIR") {
            return files::absolute(Path::new(&target));
        }
        let mut spec = CommandSpec::new(
            repo.join("genesis/agentic/bin/cargo-pool")
                .to_string_lossy(),
            ["key"],
        );
        spec.cwd = Some(repo.join("elohim/eprfs"));
        let output = self.run(&spec)?;
        output
            .stdout
            .lines()
            .find_map(|l| l.strip_prefix("slot_dev="))
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| {
                Refusal::new(
                    "bootstrap-pool-unavailable",
                    "cargo-pool key did not return slot_dev",
                )
            })
    }
    pub(super) fn build_environment(
        &self,
        repo: &Path,
        target: &Path,
    ) -> BTreeMap<String, Option<String>> {
        let policy: Value =
            files::read_json(&repo.join("genesis/agentic/pool-policy.json")).unwrap_or(Value::Null);
        let mut env = BTreeMap::new();
        for project in ["*", "eprfs"] {
            if let Some(values) = policy["cargo_env_overrides"][project].as_object() {
                for (k, v) in values {
                    if let Some(v) = v.as_str() {
                        env.insert(k.clone(), Some(v.into()));
                    }
                }
            }
        }
        env.insert("RUSTFLAGS".into(), Some(String::new()));
        env.insert(
            "CARGO_TARGET_DIR".into(),
            Some(target.to_string_lossy().into()),
        );
        env
    }
}

pub(super) fn registry_credential(
    env: &dyn Fn(&str) -> Option<String>,
) -> BTreeMap<String, Option<String>> {
    let token = env("CARGO_REGISTRIES_ELOHIM_TOKEN")
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_owned())
        .or_else(|| {
            env("NPM_TOKEN")
                .filter(|s| !s.trim().is_empty())
                .map(|s| format!("Bearer {}", s.trim()))
        });
    let Some(token) = token else {
        return BTreeMap::new();
    };
    BTreeMap::from([
        ("CARGO_REGISTRIES_ELOHIM_TOKEN".into(), Some(token)),
        (
            "CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS".into(),
            Some("cargo:token".into()),
        ),
        (
            "CARGO_REGISTRIES_ELOHIM_INDEX".into(),
            Some(env("CARGO_REGISTRIES_ELOHIM_INDEX").unwrap_or_else(|| {
                "sparse+https://nexus.ethosengine.com/repository/cargo-internal/".into()
            })),
        ),
    ])
}

pub(super) fn copy_executable(source: &Path, dest: &Path) -> Result<()> {
    files::atomic_bytes(dest, &fs::read(source)?, 0o555)
}
