//! `epr lsp` — one shared, session-attributed language server per toolchain per checkout root
//! (contracts C1–C7 of the shared-toolchain-servers plan), resident on the device.
//!
//! - `attach <lang> [--root DIR]` is the editor's stdio endpoint. It connects to the device mux
//!   and proxies LSP both ways. It never starts a server: when the device socket is absent or
//!   refuses, it answers as a null server that says "device supervisor not running". It exits 0
//!   when the client leaves and 1 when the mux closes under it, so the editor's restart-on-crash
//!   reattaches.
//! - `serve --device [--root DIR]` is the device mux, run in the foreground by the device
//!   supervisor (ark). It prints `epr-lsp: listening <socket>` once the socket listens (the
//!   readiness probe), starts one language server per `(lang, root)` on that root's first
//!   attach, reaps each after the policy's idle TTL with no clients, and replaces one whose pin
//!   the policy moved. It moors berth as `lsp-device`, holds `toolchain:lsp` (class `lsp`) for its
//!   whole life, and heartbeats the mooring; a refused claim is fatal.
//! - `show [<lang>] [--root DIR] [--session ID] [--json]` prints what a session receives:
//!   `mine`, `caused-by-me`, and `others` folded per area, from the attribution snapshot.

use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use elohim_epr_lsp::attach::{null_server, proxy, ProxyEnd};
use elohim_epr_lsp::attrib::{rfc3339, SessionKey};
use elohim_epr_lsp::daemon::{bind, system_clock, termination, DeviceConfig, Resolver, ServerPlan};
use elohim_epr_lsp::deliver::{Delivered, Snapshot};
use elohim_epr_lsp::gatefeed::{self, GateFeedConfig, Placer};
use elohim_epr_lsp::mux::MuxConfig;
use elohim_epr_lsp::server::{
    build_spec, toolchain, LspPolicy, PolicyError, ProcessSpawner, SpecInputs, POLICY_REL,
};
use elohim_epr_lsp::socket::{connect, device_socket, DaemonRecord, Hello};
use elohim_epr_lsp::uri::{path_to_uri, uri_to_path};
use serde_json::{json, Value};

/// The reason an attach gives when the device mux does not answer.
pub const SUPERVISOR_DOWN: &str = "device supervisor not running — epr device supervise";
/// The device mux's berth mooring and the session its servers' flychecks declare.
pub const MOORING: &str = "lsp-device";
/// The one berth resource the device mux holds.
pub const RESOURCE: &str = "toolchain:lsp";
/// How often the mooring is heartbeat: well under berth's 10 800 s mooring TTL.
const HEARTBEAT: Duration = Duration::from_secs(15 * 60);
/// How long one berth call may run.
const BERTH_WAIT: Duration = Duration::from_secs(60);

/// Why an `epr lsp` verb could not run.
#[derive(Debug, thiserror::Error)]
pub enum LspCliError {
    /// The command line is not understood.
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),
    /// The `lsp` policy could not be read.
    #[error("{0}")]
    Policy(#[from] PolicyError),
    /// A filesystem or socket error.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// A snapshot or policy is not JSON.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// The device mux could not take its place: lock held, or berth refused.
    #[error("{0}")]
    Refused(String),
}

/// The result of an `epr lsp` verb.
pub type LspResult<T> = Result<T, LspCliError>;

/// The verbs and their options.
#[must_use]
pub fn usage() -> String {
    "usage: epr lsp <\n  \
     attach <lang> [--root DIR] (stdio LSP endpoint: joins the device's shared server for DIR; \
     with no device mux, answers with \"no diagnostics: <why>\")\n  \
     | serve --device [--root DIR] (the device mux, in the foreground under the device \
     supervisor; prints `epr-lsp: listening <socket>` when ready)\n  \
     | show [<lang>] [--root DIR] [--session ID] [--json] (mine, caused-by-me and others' \
     folded diagnostics for a session)>"
        .to_string()
}

/// Entry point for `epr lsp …`.
///
/// # Errors
///
/// Invalid arguments, an unreadable policy, a refused berth claim, or a filesystem error.
pub fn run(args: &[String]) -> LspResult<ExitCode> {
    match args.first().map(String::as_str) {
        Some("--help" | "-h" | "help") => {
            println!("{}", usage());
            Ok(ExitCode::SUCCESS)
        }
        Some("attach") => Ok(attach(&Options::parse(&args[1..])?)),
        Some("serve") => serve(&Options::parse(&args[1..])?),
        Some("show") => show(&Options::parse(&args[1..])?),
        Some(other) => Err(LspCliError::InvalidArguments(format!(
            "unknown lsp verb `{other}`{}\n{}",
            crate::hint::did_you_mean(other, "epr lsp", &["attach", "serve", "show"]),
            usage()
        ))),
        None => Err(LspCliError::InvalidArguments(usage())),
    }
}

#[derive(Debug, Default)]
struct Options {
    lang: Option<String>,
    root: Option<PathBuf>,
    session: Option<String>,
    json: bool,
    device: bool,
}

impl Options {
    fn parse(args: &[String]) -> LspResult<Options> {
        let mut options = Options::default();
        let mut index = 0;
        while index < args.len() {
            let arg = args[index].as_str();
            match arg {
                "--json" | "--device" => {
                    if arg == "--json" {
                        options.json = true;
                    } else {
                        options.device = true;
                    }
                    index += 1;
                }
                "--root" | "--session" => {
                    let value = args.get(index + 1).ok_or_else(|| {
                        LspCliError::InvalidArguments(format!("{arg} needs a value\n{}", usage()))
                    })?;
                    if arg == "--root" {
                        options.root = Some(PathBuf::from(value));
                    } else {
                        options.session = Some(value.clone());
                    }
                    index += 2;
                }
                flag if flag.starts_with("--") => {
                    return Err(LspCliError::InvalidArguments(format!(
                        "unknown lsp argument `{flag}`\n{}",
                        usage()
                    )))
                }
                lang if options.lang.is_none() => {
                    options.lang = Some(lang.to_owned());
                    index += 1;
                }
                extra => {
                    return Err(LspCliError::InvalidArguments(format!(
                        "unexpected argument `{extra}`\n{}",
                        usage()
                    )))
                }
            }
        }
        Ok(options)
    }

    /// `--root`, canonical, or the checkout around the working directory.
    fn root(&self) -> PathBuf {
        let start = self
            .root
            .clone()
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let start = start.canonicalize().unwrap_or(start);
        if self.root.is_some() {
            start
        } else {
            checkout_root(&start)
        }
    }
}

/// The git toplevel containing `start`, or `start` itself outside a checkout.
fn checkout_root(start: &Path) -> PathBuf {
    crate::git::discover_root(start)
        .ok()
        .and_then(|root| root.canonicalize().ok())
        .unwrap_or_else(|| start.to_path_buf())
}

/// The main checkout (`git rev-parse --git-common-dir`'s parent), so every worktree of the
/// repository shares one runtime directory, one edit log and one device socket (C2/C3).
fn main_checkout(root: &Path) -> PathBuf {
    crate::process::run(
        "git",
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        root,
    )
    .ok()
    .filter(|output| output.success && !output.stdout.is_empty())
    .and_then(|output| {
        PathBuf::from(output.stdout.trim())
            .parent()
            .map(Path::to_path_buf)
    })
    .unwrap_or_else(|| root.to_path_buf())
}

fn runtime_dir(root: &Path) -> PathBuf {
    main_checkout(root).join(".eprfs/status/lsp")
}

/// gate-live's run logs under the main checkout (`liveRoot` in genesis/orchestrator/gate-live.mjs).
const GATE_RUNS_REL: &str = ".eprfs/status/gate-live/runs";

/// Places a gate run from git: the run was gated in whichever checkout of the repository has
/// its candidate commit checked out, and a gate project's directory is its build manifest's
/// `gate.projects.<name>.dir`.
struct GitPlacer {
    main: PathBuf,
}

impl Placer for GitPlacer {
    fn checkouts(&self, sha: Option<&str>) -> Vec<PathBuf> {
        let Some(sha) = sha else {
            return Vec::new();
        };
        crate::process::run("git", &["worktree", "list", "--porcelain"], &self.main)
            .ok()
            .filter(|output| output.success)
            .map(|output| checkouts_at(&output.stdout, sha))
            .unwrap_or_default()
            .into_iter()
            .filter_map(|path| path.canonicalize().ok())
            .collect()
    }

    fn project_dir(&self, checkout: &Path, project: &str) -> Option<PathBuf> {
        let listed = crate::process::run(
            "git",
            &["ls-files", "-z", "--", "*build-manifest.json"],
            checkout,
        )
        .ok()
        .filter(|output| output.success)?;
        listed
            .stdout
            .split('\0')
            .filter(|rel| !rel.is_empty())
            .filter_map(|rel| std::fs::read(checkout.join(rel)).ok())
            .filter_map(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .find_map(|manifest| gate_project_dir(&manifest, project))
    }
}

/// The worktrees in `git worktree list --porcelain` output whose HEAD is `sha` (a full or
/// abbreviated hex id of at least 7).
fn checkouts_at(porcelain: &str, sha: &str) -> Vec<PathBuf> {
    if sha.len() < 7 {
        return Vec::new();
    }
    let mut found = Vec::new();
    let mut current: Option<&str> = None;
    for line in porcelain.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            current = Some(path);
        } else if let Some(head) = line.strip_prefix("HEAD ") {
            if let Some(path) = current.filter(|_| head.starts_with(sha)) {
                found.push(PathBuf::from(path));
            }
        }
    }
    found
}

/// `gate.projects.<project>.dir` in one build manifest.
fn gate_project_dir(manifest: &Value, project: &str) -> Option<PathBuf> {
    manifest
        .get("gate")?
        .get("projects")?
        .get(project)?
        .get("dir")?
        .as_str()
        .map(PathBuf::from)
}

/// This process's session (C1): `epr session`'s own resolver, else `anon-<pid>`, which is
/// delivered as its own session and never attributed to anyone else.
fn session_key() -> SessionKey {
    let identity = crate::session::resolve();
    match identity.session {
        Some(session) => SessionKey {
            session,
            owner_pid: identity.owner_pid,
            owner_start_time: identity.owner_start_time.map(|ticks| ticks.to_string()),
        },
        None => {
            let pid = std::process::id();
            SessionKey {
                session: format!("anon-{pid}"),
                owner_pid: Some(pid),
                owner_start_time: crate::session::process_start_time(Path::new("/proc"), pid)
                    .map(|ticks| ticks.to_string()),
            }
        }
    }
}

// ---- attach -------------------------------------------------------------------------------

fn attach(options: &Options) -> ExitCode {
    let root = options.root();
    let lang = options.lang.clone().unwrap_or_default();
    let anchor = path_to_uri(&root.join("Cargo.toml"));
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("epr lsp attach: no runtime: {error}");
            return ExitCode::SUCCESS;
        }
    };
    let code = runtime.block_on(async {
        let reached = if lang.is_empty() {
            Err("no language named: epr lsp attach <lang>".to_owned())
        } else {
            let hello = Hello {
                key: session_key(),
                root: root.clone(),
                lang,
            };
            connect(&device_socket(&runtime_dir(&root)), &hello)
                .await
                .map_err(|_| SUPERVISOR_DOWN.to_owned())
        };
        match reached {
            Ok(stream) => {
                match proxy(tokio::io::stdin(), tokio::io::stdout(), stream).await {
                    // The mux closed under this session: a crash, so the editor reattaches.
                    Ok(ProxyEnd::ServerClosed) => ExitCode::FAILURE,
                    Ok(ProxyEnd::ClientClosed | ProxyEnd::ClientExited) | Err(_) => {
                        ExitCode::SUCCESS
                    }
                }
            }
            Err(reason) => {
                eprintln!("epr lsp attach: no diagnostics: {reason}");
                let _ =
                    null_server(tokio::io::stdin(), tokio::io::stdout(), &reason, &anchor).await;
                ExitCode::SUCCESS
            }
        }
    });
    // A pending stdin read sits on a blocking thread; do not wait for it to return.
    runtime.shutdown_timeout(Duration::from_millis(100));
    code
}

// ---- serve --device -----------------------------------------------------------------------

/// Plans each root's server from the pool policy of the checkout holding it, read fresh at each
/// attach so a pin change in the policy reaches the next attach.
struct PolicyResolver {
    pid: u32,
    sock: PathBuf,
    session_env: Vec<String>,
}

impl Resolver for PolicyResolver {
    fn plan(&self, lang: &str, root: &Path) -> Result<ServerPlan, String> {
        let repo_root = checkout_root(root);
        let policy_path = repo_root.join(POLICY_REL);
        let policy: Value = std::fs::read(&policy_path)
            .map_err(|e| format!("{}: {e}", policy_path.display()))
            .and_then(|bytes| {
                serde_json::from_slice(&bytes).map_err(|e| format!("{POLICY_REL}: {e}"))
            })?;
        let policy = LspPolicy::from_pool_policy(&policy).map_err(|e| e.to_string())?;
        let server = policy.server(lang).map_err(|e| e.to_string())?;
        let target_dir = server
            .target_slot
            .as_deref()
            .map(|slot| lsp_slot(root, slot));
        let spec = build_spec(
            server,
            &SpecInputs {
                lang,
                root,
                repo_root: &repo_root,
                target_dir: target_dir.as_deref(),
                session_env: &self.session_env,
                mux_session: MOORING,
                mux_pid: self.pid,
            },
        );
        let pin = toolchain(&spec);
        let record = DaemonRecord {
            pid: self.pid,
            start_time: crate::session::process_start_time(Path::new("/proc"), self.pid)
                .map(|ticks| ticks.to_string()),
            root: root.to_string_lossy().into_owned(),
            lang: lang.to_owned(),
            toolchain: pin.clone(),
            target_dir: target_dir.map(|d| d.to_string_lossy().into_owned()),
            started_at: rfc3339(chrono::Utc::now().timestamp_millis()),
            sock: self.sock.to_string_lossy().into_owned(),
        };
        Ok(ServerPlan {
            mux: MuxConfig {
                root: root.to_path_buf(),
                initialization_options: spec.initialization_options.clone(),
                config_section: spec.config_section.clone(),
                mux_pid: self.pid,
                window_ms: policy.attribution_window_ms,
                lang: lang.to_owned(),
            },
            pin,
            idle_ttl: Duration::from_secs(policy.idle_ttl_s),
            spawner: Box::new(ProcessSpawner { spec }),
            record: Some(record),
        })
    }
}

fn serve(options: &Options) -> LspResult<ExitCode> {
    if !options.device {
        return Err(LspCliError::InvalidArguments(format!(
            "epr lsp serve runs only as the device mux: epr lsp serve --device\n{}",
            usage()
        )));
    }
    let checkout = options.root();
    let runtime_dir = runtime_dir(&checkout);
    let pid = std::process::id();
    let sock = device_socket(&runtime_dir);
    let resolver = PolicyResolver {
        pid,
        sock: sock.clone(),
        session_env: crate::session::registry()
            .runtimes
            .iter()
            .flat_map(|runtime| runtime.session_env.iter().cloned())
            .collect(),
    };
    let config = DeviceConfig {
        runtime_dir: runtime_dir.clone(),
        clock: system_clock(),
        resolver: Arc::new(resolver),
    };
    let executor = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let berth = Berth::new(&checkout, pid);
    executor.block_on(async {
        let Some(bound) = bind(config)? else {
            return Err(LspCliError::Refused(format!(
                "another device mux holds {}",
                runtime_dir.join("device.lock").display()
            )));
        };
        // The claim is the daemon row the device's build-slot policy reads; serving without it
        // would put an unaccounted toolchain on the device, so a refusal ends the mux here.
        if let Err(refusal) = berth.claim() {
            let _ = std::fs::remove_file(bound.socket());
            berth.down();
            return Err(LspCliError::Refused(refusal));
        }
        let mut stdout = std::io::stdout().lock();
        writeln!(stdout, "epr-lsp: listening {}", bound.socket().display())?;
        stdout.flush()?;
        drop(stdout);
        let heartbeat = berth.heartbeat();
        // Only the device mux tails gate-live runs: one reader per device, as for the C2 log.
        let gate_tail = tokio::spawn(gatefeed::tail(GateFeedConfig {
            runs_dir: main_checkout(&checkout).join(GATE_RUNS_REL),
            runtime_dir: runtime_dir.clone(),
            placer: Arc::new(GitPlacer {
                main: main_checkout(&checkout),
            }),
            feed: bound.external_feed(),
        }));
        let served = bound.run(termination()).await;
        gate_tail.abort();
        heartbeat.stop();
        berth.down();
        served.map_err(LspCliError::Io)
    })?;
    Ok(ExitCode::SUCCESS)
}

/// The pool slot `CARGO_TARGET_DIR` for the server (C5): the sibling `slot` of the root's
/// `dev` slot, `$POOL/family/<family>/<flat-ws>/<slot>`. This replicates `pool-lib.sh`
/// (`detect_family`, `flatten_path`, `slot_path`) and `cargo-pool key`'s workspace walk rather
/// than shelling out: `cargo-pool key` refuses a root with no Cargo.toml above it (the checkout
/// top, the default root), and a pure function is testable. A root that is itself the checkout
/// top has an empty workspace path, so its slot is `$POOL/family/<family>/<slot>`.
fn lsp_slot(root: &Path, slot: &str) -> PathBuf {
    let pool = std::env::var_os("CARGO_TARGET_POOL_ROOT")
        .filter(|v| !v.is_empty())
        .map_or_else(
            || PathBuf::from("/projects/.cargo-target-pool"),
            PathBuf::from,
        );
    let worktree = root
        .ancestors()
        .find(|dir| dir.join(".git").exists())
        .unwrap_or(root)
        .to_path_buf();
    let family = std::env::var("CARGO_TARGET_POOL_FAMILY")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| {
            std::fs::read_to_string(worktree.join(".family"))
                .ok()
                .map(|text| text.split_whitespace().collect::<String>())
                .filter(|v| !v.is_empty())
        })
        .unwrap_or_else(|| {
            let branch =
                crate::process::run("git", &["rev-parse", "--abbrev-ref", "HEAD"], &worktree)
                    .ok()
                    .filter(|o| o.success)
                    .map(|o| o.stdout.trim().to_owned());
            let basename = worktree
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            family_of(branch.as_deref(), &basename)
        });
    let ws_rel = workspace_rel(root, &worktree);
    let flat = flatten(&ws_rel);
    let family_dir = pool.join("family").join(family);
    if flat.is_empty() {
        family_dir.join(slot)
    } else {
        family_dir.join(flat).join(slot)
    }
}

/// `detect_family` past its overrides: the branch's first word, or the worktree's name.
fn family_of(branch: Option<&str>, basename: &str) -> String {
    match branch.filter(|b| !b.is_empty() && *b != "HEAD") {
        Some(branch) => {
            let branch = ["feat/", "feature/", "fix/", "chore/"]
                .iter()
                .find_map(|prefix| branch.strip_prefix(prefix))
                .or_else(|| branch.strip_prefix("worktree-"))
                .unwrap_or(branch);
            branch
                .split(['-', '/'])
                .next()
                .unwrap_or_default()
                .to_lowercase()
        }
        None => basename
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .trim_end_matches('-')
            .to_owned(),
    }
}

/// `cargo-pool key`'s walk: the nearest `[workspace]` manifest above `root`, else the nearest
/// manifest, relative to the worktree; empty when there is none.
fn workspace_rel(root: &Path, worktree: &Path) -> String {
    let mut first = None;
    for dir in root.ancestors() {
        if !dir.starts_with(worktree) {
            break;
        }
        if let Ok(text) = std::fs::read_to_string(dir.join("Cargo.toml")) {
            let rel = dir
                .strip_prefix(worktree)
                .unwrap_or(dir)
                .to_string_lossy()
                .into_owned();
            if text.lines().any(|line| line.starts_with("[workspace]")) {
                return rel;
            }
            first.get_or_insert(rel);
        }
    }
    first.unwrap_or_default()
}

/// `flatten_path`: `/` to `_`, every run of underscores to `__`, outer underscores stripped.
fn flatten(rel: &str) -> String {
    let replaced = rel.replace('/', "_");
    let mut out = String::new();
    let mut run = false;
    for c in replaced.chars() {
        if c == '_' {
            if !run {
                out.push_str("__");
            }
            run = true;
        } else {
            out.push(c);
            run = false;
        }
    }
    out.trim_matches('_').to_owned()
}

/// The device mux's berth presence: the `lsp-device` mooring and the `toolchain:lsp` lease.
struct Berth {
    program: PathBuf,
    pid: u32,
}

/// A running mooring heartbeat; stopped at exit.
struct Heartbeat {
    stop: Arc<AtomicBool>,
}

impl Heartbeat {
    fn stop(&self) {
        self.stop.store(true, Ordering::Release);
    }
}

impl Berth {
    fn new(checkout: &Path, pid: u32) -> Berth {
        Berth {
            program: checkout_root(checkout).join("genesis/agentic/bin/berth"),
            pid,
        }
    }

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    /// Moor `lsp-device` and claim `toolchain:lsp`. The mooring is best-effort (the claim moors
    /// on its own when the mooring is missing); the claim is not.
    fn claim(&self) -> Result<(), String> {
        let pid = self.pid.to_string();
        let moored = berth_call(
            &self.program,
            &Berth::args(&[
                "moor",
                "--session",
                MOORING,
                "--runtime",
                "lsp",
                "--pid",
                &pid,
                "--task",
                "device language-server mux",
            ]),
        );
        if let Err(why) = moored {
            eprintln!("epr lsp: warning: berth moor {MOORING}: {why}");
        }
        berth_call(
            &self.program,
            &Berth::args(&[
                "claim",
                RESOURCE,
                "--session",
                MOORING,
                "--class",
                "lsp",
                "--note",
                "device language-server mux",
            ]),
        )
        .map_err(|why| format!("berth refused {RESOURCE} for {MOORING}: {why}"))
    }

    /// `berth touch --session lsp-device --pid <pid>` every [`HEARTBEAT`].
    fn heartbeat(&self) -> Heartbeat {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let program = self.program.clone();
        let pid = self.pid.to_string();
        std::thread::spawn(move || {
            let mut last = Instant::now();
            while !flag.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_secs(1));
                if last.elapsed() < HEARTBEAT {
                    continue;
                }
                last = Instant::now();
                let touched = berth_call(
                    &program,
                    &Berth::args(&["touch", "--session", MOORING, "--pid", &pid]),
                );
                if let Err(why) = touched {
                    eprintln!("epr lsp: warning: berth touch {MOORING}: {why}");
                }
            }
        });
        Heartbeat { stop }
    }

    fn down(&self) {
        for args in [
            Berth::args(&["release", RESOURCE, "--session", MOORING]),
            Berth::args(&["unmoor", "--session", MOORING]),
        ] {
            if let Err(why) = berth_call(&self.program, &args) {
                eprintln!("epr lsp: warning: berth {}: {why}", args.join(" "));
            }
        }
    }
}

/// Run one berth command with a deadline: `Ok` on exit 0, else what berth said.
fn berth_call(program: &Path, args: &[String]) -> Result<(), String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("not run: {error}"))?;
    let deadline = Instant::now() + BERTH_WAIT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let mut detail = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_string(&mut detail);
    }
    match status {
        Some(status) if status.success() => Ok(()),
        Some(status) => Err(format!("exited {status}: {}", detail.trim())),
        None => Err(format!("timed out after {}s", BERTH_WAIT.as_secs())),
    }
}

// ---- show ---------------------------------------------------------------------------------

fn show(options: &Options) -> LspResult<ExitCode> {
    let root = options.root();
    let session = options
        .session
        .clone()
        .unwrap_or_else(|| session_key().session);
    let dir = runtime_dir(&root);
    let root12 = elohim_epr_lsp::socket::root12(&root);
    let mut snapshots = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(lang) = name
                .strip_prefix("attrib-")
                .and_then(|rest| rest.strip_suffix(&format!("-{root12}.json")))
            else {
                continue;
            };
            if options.lang.as_deref().is_some_and(|wanted| wanted != lang) {
                continue;
            }
            let snapshot: Snapshot = serde_json::from_slice(&std::fs::read(entry.path())?)?;
            snapshots.push(snapshot);
        }
    }
    snapshots.sort_by(|a, b| a.lang.cmp(&b.lang));
    if options.json {
        let servers: Vec<Value> = snapshots
            .iter()
            .map(|snapshot| show_json(snapshot, &session))
            .collect();
        let payload = json!({"root": root, "session": session, "servers": servers});
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(ExitCode::SUCCESS);
    }
    if snapshots.is_empty() {
        println!("epr lsp: no shared server is serving {}", root.display());
        return Ok(ExitCode::SUCCESS);
    }
    let mut stdout = std::io::stdout().lock();
    for snapshot in &snapshots {
        let view = snapshot.view(&session);
        writeln!(
            stdout,
            "epr lsp {} — {} — session {} (as of {})",
            snapshot.lang,
            snapshot.root,
            elohim_epr_lsp::attrib::short8(&session),
            snapshot.at
        )?;
        writeln!(stdout, "mine ({})", view.mine.len())?;
        for delivered in &view.mine {
            writeln!(stdout, "  {}", line(delivered, &root))?;
        }
        writeln!(stdout, "caused-by-me ({})", view.caused_by_me.len())?;
        for delivered in &view.caused_by_me {
            writeln!(stdout, "  {}", line(delivered, &root))?;
        }
        writeln!(stdout, "others ({})", view.others.len())?;
        for fold in &view.others {
            writeln!(stdout, "  {}", fold.message())?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn show_json(snapshot: &Snapshot, session: &str) -> Value {
    let view = snapshot.view(session);
    let others: Vec<Value> = view
        .others
        .iter()
        .map(|fold| {
            json!({
                "area": fold.area,
                "owners": fold.owners,
                "errors": fold.errors,
                "warnings": fold.warnings,
                "message": fold.message(),
            })
        })
        .collect();
    json!({
        "lang": snapshot.lang,
        "at": snapshot.at,
        "mine": view.mine,
        "causedByMe": view.caused_by_me,
        "others": others,
    })
}

fn line(delivered: &Delivered, root: &Path) -> String {
    let diagnostic = &delivered.diagnostic;
    let path = uri_to_path(&delivered.uri).map_or_else(
        || delivered.uri.clone(),
        |p| {
            p.strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .into_owned()
        },
    );
    let at = |key: &str| {
        diagnostic
            .pointer(&format!("/range/start/{key}"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
            + 1
    };
    let severity = match diagnostic.get("severity").and_then(Value::as_u64) {
        Some(2) => "warning",
        Some(3) => "info",
        Some(4) => "hint",
        _ => "error",
    };
    format!(
        "{path}:{}:{} {severity}: {} [{}]",
        at("line"),
        at("character"),
        diagnostic
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        delivered.label.as_str()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_follows_pool_lib_detect_family() {
        assert_eq!(
            family_of(Some("feat/shared-toolchain-servers"), "x"),
            "shared"
        );
        assert_eq!(family_of(Some("worktree-agent-abc"), "x"), "agent");
        assert_eq!(family_of(Some("dev"), "x"), "dev");
        assert_eq!(family_of(Some("Fix/Big-Thing"), "x"), "fix");
        assert_eq!(family_of(Some("HEAD"), "My Tree_2"), "my-tree-2");
        assert_eq!(family_of(None, "wt-"), "wt");
    }

    #[test]
    fn flatten_matches_pool_lib_flatten_path() {
        assert_eq!(flatten("elohim/eprfs"), "elohim__eprfs");
        assert_eq!(
            flatten("elohim/holochain/tools/hc_dbtool"),
            "elohim__holochain__tools__hc__dbtool"
        );
        assert_eq!(flatten(""), "");
        assert_eq!(flatten("crates"), "crates");
    }

    #[test]
    fn workspace_rel_follows_cargo_pool_key() {
        let dir = tempfile::tempdir().unwrap();
        let worktree = dir.path().join("wt");
        let crate_dir = worktree.join("elohim/eprfs/epr-lsp");
        std::fs::create_dir_all(&crate_dir).unwrap();
        std::fs::create_dir_all(worktree.join(".git")).unwrap();
        std::fs::write(worktree.join("elohim/eprfs/Cargo.toml"), "[workspace]\n").unwrap();
        std::fs::write(crate_dir.join("Cargo.toml"), "[package]\n").unwrap();
        assert_eq!(workspace_rel(&crate_dir, &worktree), "elohim/eprfs");
        assert_eq!(workspace_rel(&worktree, &worktree), "");
    }

    #[test]
    fn options_take_a_positional_language_and_flags() {
        let args: Vec<String> = [
            "rust",
            "--root",
            "/r",
            "--session",
            "s",
            "--json",
            "--device",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
        let options = Options::parse(&args).unwrap();
        assert_eq!(options.lang.as_deref(), Some("rust"));
        assert_eq!(options.root.as_deref(), Some(Path::new("/r")));
        assert_eq!(options.session.as_deref(), Some("s"));
        assert!(options.json && options.device);
        assert!(Options::parse(&["--bogus".to_owned()]).is_err());
        assert!(Options::parse(&["a".to_owned(), "b".to_owned()]).is_err());
    }

    #[test]
    fn serve_without_device_is_refused_and_attach_never_spawns() {
        let options = Options::parse(&["rust".to_owned()]).unwrap();
        assert!(matches!(
            serve(&options),
            Err(LspCliError::InvalidArguments(_))
        ));
        assert_eq!(
            SUPERVISOR_DOWN,
            "device supervisor not running — epr device supervise"
        );
    }

    #[test]
    fn a_refused_berth_call_names_what_berth_said() {
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("berth");
        std::fs::write(
            &fake,
            "#!/bin/sh\necho 'toolchain:lsp: REFUSED — held by lsp-device-old' >&2\nexit 3\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
        let berth = Berth {
            program: fake,
            pid: 1,
        };
        let refusal = berth.claim().unwrap_err();
        assert!(refusal.contains("held by lsp-device-old"), "{refusal}");
        assert!(refusal.contains("exit status: 3"), "{refusal}");
    }

    #[test]
    fn a_gate_run_is_placed_in_the_worktrees_at_its_candidate() {
        let porcelain = "worktree /r\nHEAD 1111111111111111111111111111111111111111\nbranch \
                         refs/heads/dev\n\nworktree /r/.worktrees/a\nHEAD \
                         2222222222222222222222222222222222222222\ndetached\n\nworktree \
                         /r/.worktrees/b\nHEAD 2222222222222222222222222222222222222222\n";
        let sha = "2222222222222222222222222222222222222222";
        assert_eq!(
            checkouts_at(porcelain, sha),
            vec![
                PathBuf::from("/r/.worktrees/a"),
                PathBuf::from("/r/.worktrees/b")
            ]
        );
        assert_eq!(
            checkouts_at(porcelain, "1111111"),
            vec![PathBuf::from("/r")]
        );
        assert!(
            checkouts_at(porcelain, "111").is_empty(),
            "too short to name one"
        );
        assert!(checkouts_at(porcelain, "3333333").is_empty());
    }

    #[test]
    fn a_gate_projects_dir_comes_from_its_manifest() {
        let manifest = json!({"gate": {"projects": {"eprfs": {"dir": "elohim/eprfs"}}}});
        assert_eq!(
            gate_project_dir(&manifest, "eprfs"),
            Some(PathBuf::from("elohim/eprfs"))
        );
        assert_eq!(gate_project_dir(&manifest, "app"), None);
        assert_eq!(gate_project_dir(&json!({"name": "x"}), "eprfs"), None);
    }
}
