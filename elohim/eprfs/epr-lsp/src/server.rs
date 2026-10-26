//! The server child: its policy (contract C5), the spec it is spawned from, and the spawner seam.
//!
//! [`Spawner`] is the seam between the daemon and a real process, so the daemon's tests can run
//! an in-process fake server over pipes and count how many servers were spawned.

use std::collections::BTreeMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;

use serde::Deserialize;
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncWrite};

/// Where the `lsp` policy lives, relative to the checkout.
pub const POLICY_REL: &str = "genesis/agentic/pool-policy.json";

/// A root under this directory is a WASM root (C5): DNA zomes keep their own toolchain.
pub const WASM_ROOTS: &str = "elohim/holochain/dna";

/// Why the policy could not be read.
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    /// The pool policy has no `lsp` key.
    #[error("{POLICY_REL} declares no `lsp` policy")]
    Missing,
    /// The `lsp` policy does not have the C5 shape.
    #[error("{POLICY_REL} `lsp` is malformed: {0}")]
    Malformed(#[from] serde_json::Error),
    /// The language has no server.
    #[error("{POLICY_REL} `lsp.servers` declares no `{0}` server")]
    UnknownLanguage(String),
}

fn default_idle_ttl_s() -> u64 {
    900
}

fn default_window_ms() -> i64 {
    30_000
}

/// The `lsp` key of the pool policy.
#[derive(Debug, Clone, Deserialize)]
pub struct LspPolicy {
    /// Seconds with zero clients before the daemon reaps its server.
    #[serde(default = "default_idle_ttl_s")]
    pub idle_ttl_s: u64,
    /// The attribution window.
    #[serde(default = "default_window_ms")]
    pub attribution_window_ms: i64,
    /// One server per language.
    pub servers: BTreeMap<String, ServerPolicy>,
}

/// One language's server.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerPolicy {
    /// The program and its arguments.
    pub command: Vec<String>,
    /// Environment for the child; `null` unsets.
    #[serde(default)]
    pub env: BTreeMap<String, Option<String>>,
    /// Overrides applied on WASM roots; `null` unsets.
    #[serde(default)]
    pub env_wasm_roots: BTreeMap<String, Option<String>>,
    /// The pool slot name for `CARGO_TARGET_DIR`, when the toolchain builds.
    #[serde(default)]
    pub target_slot: Option<String>,
    /// The server's `initializationOptions`, owned by the daemon.
    #[serde(default)]
    pub initialization_options: Value,
}

impl LspPolicy {
    /// Read the `lsp` key out of a parsed pool policy.
    ///
    /// # Errors
    ///
    /// [`PolicyError::Missing`] or [`PolicyError::Malformed`].
    pub fn from_pool_policy(policy: &Value) -> Result<LspPolicy, PolicyError> {
        let lsp = policy.get("lsp").ok_or(PolicyError::Missing)?;
        Ok(serde_json::from_value(lsp.clone())?)
    }

    /// The server for `lang`.
    ///
    /// # Errors
    ///
    /// [`PolicyError::UnknownLanguage`].
    pub fn server(&self, lang: &str) -> Result<&ServerPolicy, PolicyError> {
        self.servers
            .get(lang)
            .ok_or_else(|| PolicyError::UnknownLanguage(lang.to_owned()))
    }
}

/// Everything needed to start one server.
#[derive(Debug, Clone, PartialEq)]
pub struct ServerSpec {
    /// The program.
    pub program: String,
    /// Its arguments.
    pub args: Vec<String>,
    /// Its working directory (the root).
    pub cwd: PathBuf,
    /// Environment changes: `Some` sets, `None` unsets.
    pub env: BTreeMap<String, Option<String>>,
    /// The daemon's `initializationOptions`, with wrapper paths resolved.
    pub initialization_options: Value,
    /// The configuration section the server pulls (`rust-analyzer`).
    pub config_section: Option<String>,
}

/// The facts a spec is built from besides the policy.
#[derive(Debug, Clone)]
pub struct SpecInputs<'a> {
    /// The language.
    pub lang: &'a str,
    /// The checkout root the server analyses.
    pub root: &'a Path,
    /// The repository checkout containing it (where the wrapper scripts live).
    pub repo_root: &'a Path,
    /// The pool slot for `CARGO_TARGET_DIR`, when the policy names one.
    pub target_dir: Option<&'a Path>,
    /// Session environment variables a client's environment may carry (from the runtime
    /// registry); the shared server must not inherit one session's identity.
    pub session_env: &'a [String],
    /// The daemon's synthetic session (`lsp-<lang>-<root12>`), declared to the child so its
    /// flycheck berth claims are the daemon's, never an attached session's.
    pub mux_session: &'a str,
    /// The daemon's pid, the declared session's owner.
    pub mux_pid: u32,
}

/// Whether `root` is a WASM root of `repo_root` (C5).
#[must_use]
pub fn is_wasm_root(repo_root: &Path, root: &Path) -> bool {
    root.strip_prefix(repo_root)
        .is_ok_and(|rel| rel.starts_with(WASM_ROOTS))
}

/// Build the spec for one server.
#[must_use]
pub fn build_spec(policy: &ServerPolicy, inputs: &SpecInputs<'_>) -> ServerSpec {
    let mut env = policy.env.clone();
    if is_wasm_root(inputs.repo_root, inputs.root) {
        env.extend(policy.env_wasm_roots.clone());
    }
    if let (Some(_), Some(dir)) = (&policy.target_slot, inputs.target_dir) {
        env.insert(
            "CARGO_TARGET_DIR".into(),
            Some(dir.to_string_lossy().into_owned()),
        );
    }
    for name in inputs.session_env {
        env.insert(name.clone(), None);
    }
    env.insert(
        "ELOHIM_SESSION_ID".into(),
        Some(inputs.mux_session.to_owned()),
    );
    env.insert(
        "ELOHIM_SESSION_OWNER_PID".into(),
        Some(inputs.mux_pid.to_string()),
    );
    let mut initialization_options = policy.initialization_options.clone();
    resolve_override_commands(&mut initialization_options, inputs.root, inputs.repo_root);
    let (program, args) = policy.command.split_first().map_or_else(
        || (String::new(), Vec::new()),
        |(p, a)| (p.clone(), a.to_vec()),
    );
    let config_section = (inputs.lang == "rust").then(|| "rust-analyzer".to_owned());
    ServerSpec {
        program,
        args,
        cwd: inputs.root.to_path_buf(),
        env,
        initialization_options,
        config_section,
    }
}

/// Resolve the program of every `overrideCommand` (check, buildScripts, any future one) that is
/// a relative path: against the root (C5), or against the checkout holding the root when the
/// wrapper is not under the root itself (a root below the checkout top has no `genesis/`).
fn resolve_override_commands(value: &mut Value, root: &Path, repo_root: &Path) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if key == "overrideCommand" {
                    if let Some(Value::String(program)) =
                        child.as_array_mut().and_then(|command| command.first_mut())
                    {
                        if program.contains('/') && Path::new(program.as_str()).is_relative() {
                            let under_root = root.join(&*program);
                            let base = if under_root.exists() { root } else { repo_root };
                            *program = base.join(&*program).to_string_lossy().into_owned();
                        }
                    }
                } else {
                    resolve_override_commands(child, root, repo_root);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                resolve_override_commands(item, root, repo_root);
            }
        }
        _ => {}
    }
}

/// The toolchain a spec pins, for the C3 json: `RUSTUP_TOOLCHAIN`, or `default` when unset.
#[must_use]
pub fn toolchain(spec: &ServerSpec) -> String {
    match spec.env.get("RUSTUP_TOOLCHAIN") {
        Some(Some(pin)) => pin.clone(),
        Some(None) => "default".into(),
        None => std::env::var("RUSTUP_TOOLCHAIN").unwrap_or_else(|_| "default".into()),
    }
}

/// A running server's pipes and its exit.
pub struct ServerIo {
    /// The server's stdin.
    pub stdin: Box<dyn AsyncWrite + Send + Unpin>,
    /// The server's stdout.
    pub stdout: Box<dyn AsyncRead + Send + Unpin>,
    /// Resolves with a description when the server exits. Dropping it stops the server.
    pub exited: Pin<Box<dyn Future<Output = String> + Send>>,
}

impl std::fmt::Debug for ServerIo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerIo").finish_non_exhaustive()
    }
}

/// Starts the server.
pub trait Spawner: Send {
    /// Start one server.
    ///
    /// # Errors
    ///
    /// The spawn failed.
    fn spawn(&mut self) -> std::io::Result<ServerIo>;
}

/// Spawns the real child from a [`ServerSpec`]; its stderr goes wherever the daemon's goes.
#[derive(Debug, Clone)]
pub struct ProcessSpawner {
    /// The spec.
    pub spec: ServerSpec,
}

impl Spawner for ProcessSpawner {
    fn spawn(&mut self) -> std::io::Result<ServerIo> {
        let spec = &self.spec;
        let mut command = tokio::process::Command::new(&spec.program);
        command
            .args(&spec.args)
            .current_dir(&spec.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);
        for (key, value) in &spec.env {
            match value {
                Some(value) => command.env(key, value),
                None => command.env_remove(key),
            };
        }
        let mut child = command.spawn()?;
        let missing = || std::io::Error::other("server pipes were not captured");
        let stdin = child.stdin.take().ok_or_else(missing)?;
        let stdout = child.stdout.take().ok_or_else(missing)?;
        Ok(ServerIo {
            stdin: Box::new(stdin),
            stdout: Box::new(stdout),
            exited: Box::pin(async move {
                match child.wait().await {
                    Ok(status) => status.to_string(),
                    Err(error) => format!("wait failed: {error}"),
                }
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn policy() -> LspPolicy {
        LspPolicy::from_pool_policy(&json!({"lsp": {
            "idle_ttl_s": 900,
            "attribution_window_ms": 30000,
            "servers": {"rust": {
                "command": ["rust-analyzer"],
                "env": {"RUSTUP_TOOLCHAIN": "1.96.1", "CARGO_BUILD_JOBS": "1"},
                "env_wasm_roots": {"RUSTUP_TOOLCHAIN": null},
                "target_slot": "lsp",
                "initialization_options": {
                    "check": {"overrideCommand": ["genesis/agentic/bin/lsp-flycheck", "rust", "$saved_file"]},
                    "cargo": {"targetDir": false, "buildScripts": {"overrideCommand": ["genesis/agentic/bin/lsp-flycheck", "build-scripts"]}}
                }
            }}
        }}))
        .unwrap()
    }

    fn inputs<'a>(root: &'a Path, target: &'a Path, env: &'a [String]) -> SpecInputs<'a> {
        SpecInputs {
            lang: "rust",
            root,
            repo_root: Path::new("/repo"),
            target_dir: Some(target),
            session_env: env,
            mux_session: "lsp-rust-abc",
            mux_pid: 7,
        }
    }

    #[test]
    fn native_root_pins_the_toolchain_and_the_lsp_slot() {
        let policy = policy();
        let env = vec!["CLAUDE_SESSION_ID".to_owned()];
        let spec = build_spec(
            policy.server("rust").unwrap(),
            &inputs(Path::new("/repo"), Path::new("/pool/fam/lsp"), &env),
        );
        assert_eq!(spec.program, "rust-analyzer");
        assert_eq!(spec.env["RUSTUP_TOOLCHAIN"].as_deref(), Some("1.96.1"));
        assert_eq!(
            spec.env["CARGO_TARGET_DIR"].as_deref(),
            Some("/pool/fam/lsp")
        );
        assert_eq!(
            spec.env["CLAUDE_SESSION_ID"], None,
            "a client's session is scrubbed"
        );
        assert_eq!(
            spec.env["ELOHIM_SESSION_ID"].as_deref(),
            Some("lsp-rust-abc")
        );
        assert_eq!(
            spec.initialization_options["check"]["overrideCommand"][0],
            "/repo/genesis/agentic/bin/lsp-flycheck"
        );
        assert_eq!(
            spec.initialization_options["check"]["overrideCommand"][2],
            "$saved_file"
        );
        assert_eq!(
            spec.initialization_options["cargo"]["buildScripts"]["overrideCommand"][0],
            "/repo/genesis/agentic/bin/lsp-flycheck",
            "every overrideCommand is resolved, not only check's"
        );
        assert_eq!(spec.config_section.as_deref(), Some("rust-analyzer"));
        assert_eq!(toolchain(&spec), "1.96.1");
    }

    #[test]
    fn wasm_root_unsets_the_pin() {
        let policy = policy();
        let root = Path::new("/repo/elohim/holochain/dna/elohim");
        let spec = build_spec(
            policy.server("rust").unwrap(),
            &inputs(root, Path::new("/t"), &[]),
        );
        assert_eq!(spec.env["RUSTUP_TOOLCHAIN"], None);
        assert_eq!(toolchain(&spec), "default");
        assert!(!is_wasm_root(
            Path::new("/repo"),
            Path::new("/repo/elohim/eprfs")
        ));
    }

    #[test]
    fn missing_policy_and_language_are_named() {
        assert!(matches!(
            LspPolicy::from_pool_policy(&json!({})),
            Err(PolicyError::Missing)
        ));
        assert!(matches!(
            policy().server("cobol"),
            Err(PolicyError::UnknownLanguage(_))
        ));
    }
}
