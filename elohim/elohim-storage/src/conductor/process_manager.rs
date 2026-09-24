//! Conductor Process Manager
//!
//! Spawns the Holochain conductor as a child process, monitors it,
//! and provides readiness checks via AdminWebsocket connection.
//!
//! Part of the elohim-node consolidation — merging 4 k8s containers into 1.
//! Instead of running the conductor as a separate container, elohim-storage
//! spawns and manages it as a child process.

use holochain_client::{AdminWebsocket, WebsocketConfig};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

const DB_POOL_SATURATION_MARKER: &str = "Database read connection is saturated. Util ";

/// Lines kept per forwarded output stream (stdout / stderr) so a dead child's
/// last words survive past the moment they scrolled off the parent's own
/// stdout/stderr — the death-witness evidence `wait_for_ready` attaches to a
/// `ConductorError::Exited`.
const OUTPUT_RING_CAPACITY: usize = 200;

/// How many trailing lines of stderr-then-stdout ride along on a death-witness
/// error. Bounded independently of the ring capacity so the log line and the
/// error payload stay a manageable size even when the ring itself is fuller.
const DEATH_WITNESS_TAIL_LINES: usize = 40;

/// Default CPU niceness for the spawned conductor child.
///
/// Storage and the conductor share one container cgroup (elohim-node
/// consolidation), so a conductor-side DHT storm competes head-to-head with
/// storage's HTTP runtime for the same CPU quota. The 2026-08-17 alpha outage
/// aftermath proved the failure shape: a sustained kitsune2 op-fetch storm
/// pinned the shared quota at 100% CFS throttle, storage's read handlers
/// stopped getting scheduled, the doorway breaker opened on never-answered,
/// and every /db read shed catching-up 503s for hours while the humans rows
/// themselves were correct. Niceness only matters under contention: an idle
/// node still gives the conductor everything, but when the quota saturates,
/// storage's short read handlers outweigh the conductor's churn threads and
/// verify-locally-then-serve keeps holding.
const CONDUCTOR_NICE_DEFAULT: i32 = 10;

/// Resolve the conductor niceness from an `ELOHIM_CONDUCTOR_NICE` value.
/// Clamped to [0, 19] — the conductor is never boosted ABOVE storage (a
/// negative nice would need CAP_SYS_NICE anyway); unparseable input keeps the
/// safe default.
fn resolve_conductor_nice(raw: Option<&str>) -> i32 {
    match raw.map(str::parse::<i32>) {
        Some(Ok(n)) => n.clamp(0, 19),
        Some(Err(_)) => CONDUCTOR_NICE_DEFAULT,
        None => CONDUCTOR_NICE_DEFAULT,
    }
}

/// Read a live process's scheduling niceness from `/proc/<pid>/stat`
/// (field 19 — the 17th field after the `)` closing the comm name, which may
/// itself contain spaces/parens). `None` when the read/parse fails.
#[cfg(target_os = "linux")]
fn read_proc_nice(pid: u32) -> Option<i32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after_comm = stat.rsplit_once(')')?.1;
    after_comm.split_whitespace().nth(16)?.parse().ok()
}

/// The in-proc lair keystore type string, exactly as written by every
/// conductor-config.yaml this codebase generates (edgenode template,
/// household mesh, `hc sandbox`).
const LAIR_SERVER_IN_PROC: &str = "lair_server_in_proc";

/// Read `keystore.lair_root` from a conductor config, but ONLY when the
/// keystore type is the in-proc lair server (`lair_server_in_proc`) — an
/// external/remote lair-keystore process is not this node's to touch.
///
/// This is a small line scan, not a YAML parser — a full YAML dependency is
/// unwarranted for reading two scalar fields out of a flat `keystore:`
/// mapping. See [`keystore_block_lines`] / [`yaml_scalar_value`].
///
/// Returns `None` — a safe no-op, never an error — for a missing file, no
/// top-level `keystore:` block, a different keystore type, or a `keystore`
/// block with no `lair_root`. A guard that cannot determine the lair root
/// must never block conductor spawn.
fn read_in_proc_lair_root(config_path: &std::path::Path) -> Option<PathBuf> {
    let contents = std::fs::read_to_string(config_path).ok()?;
    let block = keystore_block_lines(&contents);
    if yaml_scalar_value(&block, "type").as_deref() != Some(LAIR_SERVER_IN_PROC) {
        return None;
    }
    yaml_scalar_value(&block, "lair_root").map(PathBuf::from)
}

/// The lines belonging to a top-level `keystore:` mapping — every line
/// starting at column 0 with `keystore:` is Holochain's convention here
/// (verified against every conductor-config.yaml this codebase writes: the
/// edgenode template, the household mesh, `hc sandbox`). Collects lines
/// until the next column-0 line (the start of the next top-level key) or
/// EOF. Empty (not `None`) when there is no top-level `keystore:` key at
/// all — `yaml_scalar_value` over an empty block already yields `None` for
/// every key, so the two "nothing to read" cases don't need to be told
/// apart by the caller.
fn keystore_block_lines(contents: &str) -> Vec<&str> {
    let mut lines = contents.lines();
    let found = lines.by_ref().any(|line| line == "keystore:");
    if !found {
        return Vec::new();
    }
    let mut block = Vec::new();
    for line in lines {
        let indented = line.starts_with(' ') || line.starts_with('\t');
        if indented || line.trim().is_empty() {
            block.push(line);
        } else {
            break; // the next top-level key — the keystore mapping ended.
        }
    }
    block
}

/// Find a flat `key: value` pair within a block of lines already scoped to
/// one YAML mapping (see [`keystore_block_lines`]). Ignores comment-only
/// lines; strips a trailing `# comment` and surrounding quotes from the
/// value. `None` when the key is absent or its value is empty.
fn yaml_scalar_value(block: &[&str], key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    for line in block {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        let Some(rest) = trimmed.strip_prefix(&prefix) else {
            continue;
        };
        let value = rest.split('#').next().unwrap_or(rest).trim();
        let value = value.trim_matches('"').trim_matches('\'').trim();
        if value.is_empty() {
            return None;
        }
        return Some(value.to_string());
    }
    None
}

/// Whether a pid is a currently-live process, checked via `/proc/<pid>`.
#[cfg(target_os = "linux")]
fn pid_is_alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

/// Off Linux there is no `/proc` to consult. Conservatively report "alive" so
/// the guard never removes a live lair's files on a platform it cannot
/// verify liveness on.
#[cfg(not(target_os = "linux"))]
fn pid_is_alive(_pid: u32) -> bool {
    true
}

/// Heal a stale Lair keystore left behind by a prior crash (the 2026-09-23
/// ethosengine incident: the node crashed, an EMPTY `pid_file` and an
/// orphaned unix socket were left in the Lair root, and the next conductor
/// spawn crash-looped against them until an operator deleted both by hand).
///
/// Only inspects and possibly removes two files, `pid_file` and the paired
/// `socket` (Lair's fixed layout — verified against every running household
/// `ks/` directory: `pid_file`, `socket`, `store_file*`,
/// `lair-keystore-config.yaml`), and only in the stale-pid case:
///
/// - `pid_file` missing → no-op (nothing to heal).
/// - `pid_file` empty, unparseable as a pid, or names a pid that is not
///   alive → remove `pid_file`, then remove `socket` if present.
/// - `pid_file` names a LIVE pid → leave everything alone; another lair
///   server owns this root.
///
/// HARD RULE: never touches `store_file*` or `lair-keystore-config.yaml` —
/// those hold the agent keys, and deleting them re-keys the node.
fn heal_stale_lair_socket(lair_root: &std::path::Path) {
    let pid_file = lair_root.join("pid_file");
    let Ok(contents) = std::fs::read_to_string(&pid_file) else {
        // Missing (or unreadable) pid_file — nothing to heal.
        return;
    };

    let trimmed = contents.trim();
    let stale_reason: &str = if trimmed.is_empty() {
        "pid_file is empty"
    } else {
        match trimmed.parse::<u32>() {
            Ok(pid) if pid_is_alive(pid) => {
                // A live process owns this lair root — leave pid_file and
                // socket untouched.
                return;
            }
            Ok(_) => "pid_file names a pid that is not alive",
            Err(_) => "pid_file is not a valid pid",
        }
    };

    warn!(
        lair_root = %lair_root.display(),
        pid_file = %pid_file.display(),
        reason = stale_reason,
        "Removing stale Lair pid_file left by a prior crash"
    );
    if let Err(e) = std::fs::remove_file(&pid_file) {
        warn!(
            path = %pid_file.display(),
            error = %e,
            "Failed to remove stale Lair pid_file"
        );
    }

    let socket_path = lair_root.join("socket");
    match std::fs::remove_file(&socket_path) {
        Ok(()) => {
            warn!(
                lair_root = %lair_root.display(),
                socket = %socket_path.display(),
                reason = stale_reason,
                "Removing stale Lair unix socket left by a prior crash"
            );
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // No socket to clean up — the crash left only the pid_file.
        }
        Err(e) => {
            warn!(
                path = %socket_path.display(),
                error = %e,
                "Failed to remove stale Lair unix socket"
            );
        }
    }
}

/// Entry point called before every conductor spawn: resolve the in-proc lair
/// root from the conductor config the process manager is about to use, and
/// heal a stale pid_file/socket pair in it. A config with no in-proc lair
/// root (different keystore type, unreadable/unparseable config) is a
/// deliberate no-op — see [`read_in_proc_lair_root`].
fn heal_stale_lair_state(config_path: &std::path::Path) {
    if let Some(lair_root) = read_in_proc_lair_root(config_path) {
        heal_stale_lair_socket(&lair_root);
    }
}

#[derive(Debug, Clone, PartialEq)]
struct DbPoolSaturation {
    kind: &'static str,
    utilization_ratio: f64,
}

/// Parse Holochain's existing read-pool saturation event without admitting
/// per-DNA identifiers into Prometheus labels.
fn parse_db_pool_saturation(line: &str) -> Option<DbPoolSaturation> {
    let (prefix, utilization) = line.split_once(DB_POOL_SATURATION_MARKER)?;
    let percent = utilization.split_once('%')?.0.trim().parse::<f64>().ok()?;
    if !percent.is_finite() || percent < 0.0 {
        return None;
    }

    // Plain tracing renders `kind=Dht(...)`; structured tracing renders the
    // same value inside JSON. Restrict the result to a fixed vocabulary so a
    // DNA hash can never become a high-cardinality metric label.
    let prefix = prefix.to_ascii_lowercase();
    let kind_context = prefix
        .rfind("kind")
        .map(|index| &prefix[index..])
        .unwrap_or("");
    let kind = if kind_context.contains("peermetastore") || kind_context.contains("peer_meta_store")
    {
        "peer_meta_store"
    } else if kind_context.contains("authored") {
        "authored"
    } else if kind_context.contains("conductor") {
        "conductor"
    } else if kind_context.contains("cache") {
        "cache"
    } else if kind_context.contains("wasm") {
        "wasm"
    } else if kind_context.contains("dht") {
        "dht"
    } else {
        "unknown"
    };

    Some(DbPoolSaturation {
        kind,
        utilization_ratio: percent / 100.0,
    })
}

/// Bounded FIFO of the most recently forwarded lines from one conductor
/// output stream. Pure ring-buffer semantics (push evicts the oldest line
/// once at capacity) so the death-witness eviction rule is unit-testable
/// without a real child process.
#[derive(Debug)]
struct RingBuffer {
    capacity: usize,
    lines: VecDeque<String>,
}

impl RingBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            lines: VecDeque::with_capacity(capacity),
        }
    }

    fn push(&mut self, line: String) {
        if self.lines.len() >= self.capacity {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    /// The last `n` lines, oldest first. Fewer than `n` when the buffer has
    /// not yet filled.
    fn last_n(&self, n: usize) -> Vec<String> {
        let skip = self.lines.len().saturating_sub(n);
        self.lines.iter().skip(skip).cloned().collect()
    }
}

impl Default for RingBuffer {
    fn default() -> Self {
        Self::new(OUTPUT_RING_CAPACITY)
    }
}

/// Shared, mutex-guarded state fed by `forward_conductor_output` and read by
/// `wait_for_ready` — the bridge that lets the readiness probe see what the
/// child has been saying on its own output pipes without disturbing the
/// existing byte-for-byte forwarding to the parent's stdout/stderr.
#[derive(Debug, Default)]
struct ConductorOutputState {
    stdout: Mutex<RingBuffer>,
    stderr: Mutex<RingBuffer>,
    last_db_pool_saturation: Mutex<Option<DbPoolSaturation>>,
}

impl ConductorOutputState {
    /// Last `n` lines of stderr followed by the last `n` lines of stdout —
    /// stderr first because that is where a Rust panic (and Holochain's own
    /// FATAL PANIC banner) lands.
    fn last_lines(&self, n: usize) -> Vec<String> {
        let mut lines = self
            .stderr
            .lock()
            .expect("stderr ring buffer mutex poisoned")
            .last_n(n);
        lines.extend(
            self.stdout
                .lock()
                .expect("stdout ring buffer mutex poisoned")
                .last_n(n),
        );
        lines
    }
}

/// Drain one conductor output pipe, preserving its bytes on the parent output
/// while projecting the existing saturation event into Prometheus. A broken
/// parent output must not stop draining the child pipe (which would deadlock a
/// log-heavy conductor), so forwarding errors are warned once and then ignored.
async fn forward_conductor_output<R, W>(
    reader: R,
    mut writer: W,
    stream: &'static str,
    output_state: Arc<ConductorOutputState>,
) where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut reader = BufReader::new(reader);
    let mut bytes = Vec::new();
    let mut write_failed = false;

    // bounded-work: one child-pipe lifetime; each iteration drains one
    // newline-delimited record and terminates on EOF or the first read error.
    loop {
        bytes.clear();
        match reader.read_until(b'\n', &mut bytes).await {
            Ok(0) => break,
            Ok(_) => {
                let line = String::from_utf8_lossy(&bytes);
                if let Some(sample) = parse_db_pool_saturation(&line) {
                    crate::metrics::observe_db_read_pool_saturation(
                        sample.kind,
                        sample.utilization_ratio,
                    );
                    *output_state
                        .last_db_pool_saturation
                        .lock()
                        .expect("db-pool-saturation mutex poisoned") = Some(sample);
                }
                let ring = match stream {
                    "stderr" => &output_state.stderr,
                    _ => &output_state.stdout,
                };
                ring.lock()
                    .expect("output ring buffer mutex poisoned")
                    .push(line.trim_end_matches(['\n', '\r']).to_string());
                if !write_failed {
                    if let Err(err) = writer.write_all(&bytes).await {
                        write_failed = true;
                        warn!(%stream, %err, "Failed to forward conductor output; continuing to drain child pipe");
                    }
                }
            }
            Err(err) => {
                warn!(%stream, %err, "Failed to read conductor output pipe");
                break;
            }
        }
    }
}

/// Stand-in for a child's exit status in the readiness decision below — only
/// whether the child has exited matters to the decision, never the status's
/// platform-specific bits, so tests can drive `classify_readiness_outcome`
/// without spawning a real process (there is no public constructor for
/// `std::process::ExitStatus`).
type ExitStatusLike = ();

/// Outcome of one `wait_for_ready` polling attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadinessOutcome {
    /// The child has already exited — stop polling immediately, regardless
    /// of how many attempts remain.
    ChildExited,
    /// The child is alive and attempts remain — sleep and retry.
    Retry,
    /// The child is alive but attempts are exhausted — give up as NotReady.
    GiveUp,
}

/// Pure decision: given whether the child has exited and how many attempts
/// remain, what should `wait_for_ready` do next? A dead child always wins
/// immediately — a slow child and a dead child must never be conflated by
/// burning the rest of the retry budget on a process that is no longer
/// running.
fn classify_readiness_outcome(
    child_exited: Option<ExitStatusLike>,
    attempt: u32,
    max_retries: u32,
) -> ReadinessOutcome {
    if child_exited.is_some() {
        return ReadinessOutcome::ChildExited;
    }
    if attempt < max_retries {
        ReadinessOutcome::Retry
    } else {
        ReadinessOutcome::GiveUp
    }
}

/// Manages a Holochain conductor child process.
///
/// Spawns the conductor binary with the given config, monitors its lifecycle,
/// and provides readiness checks via AdminWebsocket connection.
pub struct ConductorManager {
    conductor_binary: PathBuf,
    config_path: PathBuf,
    data_dir: PathBuf,
    admin_port: u16,
    /// Admin-websocket request timeout. Must cover the cold first hApp install
    /// (single-threaded wasm compile + genesis), which on slower per-core nodes
    /// exceeds holochain_client's 60s default. See `HAPP_INSTALL_TIMEOUT_SECS`.
    admin_request_timeout: Duration,
    child: Option<Child>,
    /// Ring-buffered tail of the child's forwarded stdout/stderr plus the
    /// last-seen DB-pool-saturation sample — the death-witness evidence
    /// `wait_for_ready` attaches to a `ConductorError::Exited`/`NotReady`.
    output_state: Arc<ConductorOutputState>,
}

impl ConductorManager {
    /// Create a new ConductorManager.
    ///
    /// Does not start the conductor — call [`start`] to spawn the process.
    pub fn new(
        conductor_binary: PathBuf,
        config_path: PathBuf,
        data_dir: PathBuf,
        admin_port: u16,
        admin_request_timeout: Duration,
    ) -> Self {
        Self {
            conductor_binary,
            config_path,
            data_dir,
            admin_port,
            admin_request_timeout,
            child: None,
            output_state: Arc::new(ConductorOutputState::default()),
        }
    }

    /// Spawn the Holochain conductor as a child process.
    ///
    /// Uses `--config-path` and `--piped` arguments. The process is configured
    /// with `kill_on_drop(true)` so it is terminated if the manager is dropped.
    pub fn start(&mut self) -> Result<(), ConductorError> {
        if self.is_running() {
            return Err(ConductorError::AlreadyRunning);
        }

        info!(
            binary = %self.conductor_binary.display(),
            config = %self.config_path.display(),
            data_dir = %self.data_dir.display(),
            admin_port = self.admin_port,
            "Starting Holochain conductor"
        );

        // Heal a stale Lair keystore left by a prior crash BEFORE spawning —
        // see `heal_stale_lair_state`. Never touches store_file*/
        // lair-keystore-config.yaml; a pid_file naming a live process is
        // left untouched.
        heal_stale_lair_state(&self.config_path);

        let nice = resolve_conductor_nice(std::env::var("ELOHIM_CONDUCTOR_NICE").ok().as_deref());
        let mut cmd = Command::new(&self.conductor_binary);
        cmd.arg("--config-path")
            .arg(&self.config_path)
            .arg("--piped")
            .env("HOLOCHAIN_DATA_DIR", &self.data_dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        // CPU-deprioritize the conductor relative to storage's HTTP runtime in
        // the shared cgroup (see CONDUCTOR_NICE_DEFAULT). Best-effort: a failed
        // setpriority must never fail the spawn.
        #[cfg(unix)]
        if nice != 0 {
            unsafe {
                cmd.pre_exec(move || {
                    libc::setpriority(libc::PRIO_PROCESS, 0, nice);
                    Ok(())
                });
            }
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| ConductorError::SpawnFailed(e.to_string()))?;

        // Fresh output state per spawned child — a restart must not let a
        // prior life's tail lines bleed into this child's death-witness
        // evidence.
        self.output_state = Arc::new(ConductorOutputState::default());

        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(forward_conductor_output(
                stdout,
                tokio::io::stdout(),
                "stdout",
                Arc::clone(&self.output_state),
            ));
        }
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(forward_conductor_output(
                stderr,
                tokio::io::stderr(),
                "stderr",
                Arc::clone(&self.output_state),
            ));
        }

        let pid = child.id().unwrap_or(0);
        // Log the EFFECTIVE niceness, not the intended one: setpriority in
        // pre_exec is best-effort and can fail (EPERM when lowering the nice
        // VALUE below the parent's without CAP_SYS_NICE / under RLIMIT_NICE),
        // and a silently un-deprioritized conductor is exactly the
        // participation-vs-protection tradeoff that must be LOUD (this
        // crate's .epr-meta: reduced-participation paths never go quiet).
        #[cfg(target_os = "linux")]
        match read_proc_nice(pid) {
            Some(effective) if effective == nice => {
                info!(pid = pid, nice = effective, "Conductor process spawned");
            }
            Some(effective) => {
                warn!(
                    pid = pid,
                    intended_nice = nice,
                    effective_nice = effective,
                    "Conductor spawned at UNEXPECTED niceness — setpriority likely failed                      (EPERM/RLIMIT_NICE); storage-read protection under CPU contention is                      NOT in effect at the intended level"
                );
            }
            None => {
                info!(
                    pid = pid,
                    nice = nice,
                    "Conductor process spawned (niceness unverified — /proc read failed)"
                );
            }
        }
        #[cfg(not(target_os = "linux"))]
        info!(pid = pid, nice = nice, "Conductor process spawned");

        self.child = Some(child);
        Ok(())
    }

    /// Wait for the conductor to become ready by connecting to the AdminWebsocket.
    ///
    /// Retries up to `max_retries` times with a 2-second delay between attempts.
    /// Returns the connected AdminWebsocket on success.
    ///
    /// Every attempt `try_wait()`s the child BEFORE touching the websocket at
    /// all: a child that panicked seconds after spawn and a child that is
    /// merely slow to bind its admin socket both present as connection
    /// refused, so without consulting the child directly this loop would
    /// spend the full `max_retries * 2s` window logging "not ready yet"
    /// against a process that was never coming back — the 2026-09-02 alpha
    /// incident this method now closes.
    pub async fn wait_for_ready(
        &mut self,
        max_retries: u32,
    ) -> Result<AdminWebsocket, ConductorError> {
        let addr = format!("localhost:{}", self.admin_port);
        let started_at = std::time::Instant::now();
        info!(
            addr = %addr,
            max_retries = max_retries,
            "Waiting for conductor to become ready"
        );

        // Widen the admin-WS request timeout from holochain_client's 60s default.
        // The cold first hApp install compiles wasm single-threaded; on slower
        // per-core nodes (e.g. the shem apex) that compile exceeds 60s, so
        // `install_app` returns `Websocket error: Timeout`, the boot fails, and the
        // node crash-loops re-attempting the same cold compile forever (its wasm
        // cache never warms). Warm-cache installs are sub-second, so this budget
        // only ever bounds the first cold install. See `HAPP_INSTALL_TIMEOUT_SECS`.
        let ws_config = {
            let mut c = WebsocketConfig::CLIENT_DEFAULT;
            c.default_request_timeout = self.admin_request_timeout;
            Arc::new(c)
        };

        for attempt in 1..=max_retries {
            let exit_status = match self.child.as_mut() {
                Some(child) => child.try_wait().ok().flatten(),
                None => None,
            };

            if classify_readiness_outcome(exit_status.as_ref().map(|_| ()), attempt, max_retries)
                == ReadinessOutcome::ChildExited
            {
                let status = exit_status
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown (status unavailable)".to_string());
                let uptime_secs = started_at.elapsed().as_secs();
                let last_lines = self.last_lines(DEATH_WITNESS_TAIL_LINES);
                error!(
                    status = %status,
                    uptime_secs = uptime_secs,
                    attempt = attempt,
                    "Conductor child exited before becoming ready — stopping the readiness \
                     poll immediately instead of burning the remaining retry window"
                );
                for line in &last_lines {
                    error!("conductor[stderr]: {line}");
                }
                return Err(ConductorError::Exited {
                    status,
                    uptime_secs,
                    last_lines,
                });
            }

            match AdminWebsocket::connect_with_config(&addr, ws_config.clone(), None).await {
                Ok(ws) => {
                    info!(
                        attempt = attempt,
                        "Conductor is ready — AdminWebsocket connected"
                    );
                    return Ok(ws);
                }
                Err(e) => {
                    match classify_readiness_outcome(None, attempt, max_retries) {
                        ReadinessOutcome::Retry => {
                            warn!(
                                attempt = attempt,
                                max_retries = max_retries,
                                error = %e,
                                "Conductor not ready yet, retrying in 2s"
                            );
                            sleep(Duration::from_secs(2)).await;
                        }
                        // GiveUp: attempts exhausted but the child is still alive per
                        // try_wait above — keep the window as-is, just report honestly
                        // that this is a slow/stuck conductor, not a dead one.
                        ReadinessOutcome::GiveUp | ReadinessOutcome::ChildExited => {
                            let last_saturation = self
                                .output_state
                                .last_db_pool_saturation
                                .lock()
                                .expect("db-pool-saturation mutex poisoned")
                                .clone();
                            error!(
                                attempts = max_retries,
                                error = %e,
                                child_alive = true,
                                last_db_pool_saturation = ?last_saturation,
                                "Conductor failed to become ready"
                            );
                            return Err(ConductorError::NotReady {
                                message: format!("Failed after {} attempts: {}", max_retries, e),
                                child_alive: true,
                                last_db_pool_saturation: last_saturation
                                    .map(|s| format!("{}={:.2}", s.kind, s.utilization_ratio)),
                            });
                        }
                    }
                }
            }
        }

        unreachable!()
    }

    /// Check if the conductor child process is still running.
    pub fn is_running(&mut self) -> bool {
        match self.child.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(None) => true,     // Still running
                Ok(Some(_)) => false, // Exited
                Err(_) => false,      // Error checking — assume not running
            },
            None => false,
        }
    }

    /// Stop the conductor process gracefully.
    ///
    /// Sends a kill signal to the child process and waits for it to exit.
    pub async fn stop(&mut self) -> Result<(), ConductorError> {
        if let Some(ref mut child) = self.child {
            info!("Stopping conductor process");
            child
                .kill()
                .await
                .map_err(|e| ConductorError::StopFailed(e.to_string()))?;
            info!("Conductor process stopped");
            self.child = None;
            Ok(())
        } else {
            Ok(())
        }
    }

    /// Returns the admin port this conductor is configured to listen on.
    pub fn admin_port(&self) -> u16 {
        self.admin_port
    }

    /// PID of the live conductor child process, or `None` if not started or
    /// already reaped. The memory-attribution sampler reads this each tick to
    /// attribute the fused-cgroup working set to the conductor child vs the
    /// storage parent (`std::process::id()`).
    pub fn child_pid(&self) -> Option<u32> {
        self.child.as_ref().and_then(|c| c.id())
    }

    /// Path to the conductor-config the process is (re)spawned with.
    pub fn config_path(&self) -> &std::path::Path {
        &self.config_path
    }

    /// Last `n` lines of the current child's forwarded stderr, followed by
    /// the last `n` lines of its forwarded stdout — the death-witness tail
    /// consumed by the boot-reconcile loop and downstream self-heal work.
    /// Empty before the first `start()` or once the ring has no history.
    pub fn last_lines(&self, n: usize) -> Vec<String> {
        self.output_state.last_lines(n)
    }

    /// Restart the conductor: stop, then start again with the (possibly
    /// rewritten) config at `config_path`.
    ///
    /// This is the ONLY way to apply a changed `network.target_arc_factor` —
    /// there is no runtime arc-resize API (spike verdict, spec §2). The caller
    /// (the authority-arc actuator) MUST stagger restarts across the mesh so
    /// coverage holds during this node's reconvergence (spec §4); this method
    /// restarts only the local conductor. Disruptive: drops all conductor
    /// connections until `wait_for_ready` succeeds again.
    pub async fn restart(&mut self) -> Result<(), ConductorError> {
        info!("Restarting conductor (config change — e.g. authority-arc actuation)");
        self.stop().await?;
        self.start()?;
        Ok(())
    }

    /// Clear the conductor data dir (chain databases + lair keystore) so the next
    /// spawn boots clean — the **node-repair primitive**.
    ///
    /// This is the self-repair a node performs to recover from a genesis-less /
    /// DNA-drifted cell (the alpha CellWithoutGenesis incident, 2026-06-22): a clean
    /// data dir boots with no cell, so `ensure_happ_installed` runs `install_fresh`
    /// → genesis against the assigned bundle. **Destructive + RE-KEYS** — the lair
    /// keystore lives under the data dir, so a new agent key is minted; only invoke
    /// where a re-key is acceptable (a re-seedable node — its own
    /// `GENESIS_SELF_HEAL_IDENTITY` policy), never a lineage-bearing node (which must
    /// migrate, not wipe).
    ///
    /// Public so it can be driven by the boot reconcile loop (main.rs) today and by a
    /// human-facilitated "Update / Repair" trigger on the peer menu later — the node,
    /// not an operator with kubectl, owns its DNA-lifecycle repair (P1 reconciliation
    /// controller). Guarded against unsafe paths (refuses a relative path, the
    /// filesystem root, or a path with no final component) so a misconfigured
    /// `data_dir` can never wipe `/`.
    pub fn clear_conductor_state(data_dir: &std::path::Path) -> Result<(), ConductorError> {
        if !data_dir.is_absolute()
            || data_dir == std::path::Path::new("/")
            || data_dir.file_name().is_none()
        {
            return Err(ConductorError::HealFailed(format!(
                "refusing to clear unsafe conductor data_dir: {}",
                data_dir.display()
            )));
        }
        // Remove the tree, then recreate the empty root for the conductor to write
        // fresh state into. Already-absent is success (nothing to clear).
        match std::fs::remove_dir_all(data_dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(ConductorError::HealFailed(format!(
                    "failed to clear conductor data_dir {}: {e}",
                    data_dir.display()
                )))
            }
        }
        std::fs::create_dir_all(data_dir).map_err(|e| {
            ConductorError::HealFailed(format!(
                "failed to recreate conductor data_dir {}: {e}",
                data_dir.display()
            ))
        })?;
        info!(
            data_dir = %data_dir.display(),
            "Conductor data dir cleared for genesis re-heal (new agent key will be minted)"
        );
        Ok(())
    }
}

impl Drop for ConductorManager {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            // kill_on_drop(true) handles async cleanup, but we also attempt
            // a synchronous start_kill for immediate signal delivery.
            if let Err(e) = child.start_kill() {
                error!(error = %e, "Failed to kill conductor on drop");
            }
        }
    }
}

/// Errors from conductor process management.
#[derive(Debug, thiserror::Error)]
pub enum ConductorError {
    #[error("Conductor is already running")]
    AlreadyRunning,

    #[error("Failed to spawn conductor: {0}")]
    SpawnFailed(String),

    #[error("Conductor not ready: {message}")]
    NotReady {
        message: String,
        /// `try_wait()` confirmed the child was still alive at give-up time —
        /// distinguishes a slow/stuck conductor from a dead one (that case is
        /// `Exited` instead, returned immediately rather than waiting out the
        /// full retry budget).
        child_alive: bool,
        /// The most recent DB-read-pool saturation sample seen on the
        /// child's output, if any — `kind=ratio` (e.g. `dht=2.27`).
        last_db_pool_saturation: Option<String>,
    },

    /// The conductor child process exited before the admin websocket ever
    /// answered. Returned immediately on the first `try_wait()` that
    /// observes it — never after burning the rest of `max_retries`.
    #[error("Conductor exited before becoming ready: {status} (alive {uptime_secs}s)")]
    Exited {
        status: String,
        uptime_secs: u64,
        /// Last ~`DEATH_WITNESS_TAIL_LINES` lines of stderr then stdout from
        /// the output ring buffer.
        last_lines: Vec<String>,
    },

    #[error("Failed to stop conductor: {0}")]
    StopFailed(String),

    #[error("Conductor genesis-heal failed: {0}")]
    HealFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conductor_nice_defaults_to_deprioritized_and_env_overrides() {
        // Default: the conductor runs CPU-deprioritized so storage's HTTP reads
        // stay schedulable inside the shared cgroup quota during DHT churn.
        assert_eq!(resolve_conductor_nice(None), CONDUCTOR_NICE_DEFAULT);
        // Operator override, including full disable (0) and clamping to the
        // valid nice range.
        assert_eq!(resolve_conductor_nice(Some("0")), 0);
        assert_eq!(resolve_conductor_nice(Some("19")), 19);
        assert_eq!(resolve_conductor_nice(Some("40")), 19);
        assert_eq!(resolve_conductor_nice(Some("-7")), 0);
        assert_eq!(
            resolve_conductor_nice(Some("bogus")),
            CONDUCTOR_NICE_DEFAULT
        );
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn spawned_conductor_child_runs_at_the_resolved_niceness() {
        use std::os::unix::fs::PermissionsExt;

        // Stand-in conductor binary: a script that just sleeps, so we can read
        // the live child's scheduling niceness from /proc.
        let dir = std::env::temp_dir().join(format!("elohim_nice_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("fake-conductor.sh");
        std::fs::write(&script, "#!/bin/sh\nsleep 30\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let mut mgr = ConductorManager::new(
            script,
            dir.join("conductor-config.yaml"),
            dir.clone(),
            4499,
            Duration::from_secs(5),
        );
        mgr.start().unwrap();
        let pid = mgr.child_pid().expect("child pid after start");

        // pre_exec runs between fork and exec; give the child a moment.
        sleep(Duration::from_millis(200)).await;

        let nice = read_proc_nice(pid).expect("child /proc stat readable");
        // The spawn resolves the AMBIENT env var (a CI shell exporting the
        // documented ELOHIM_CONDUCTOR_NICE override must not red this test),
        // and setpriority cannot LOWER the nice value below the parent's
        // without CAP_SYS_NICE — a parent already running nicer than the
        // target leaves the child at the inherited value.
        let resolved =
            resolve_conductor_nice(std::env::var("ELOHIM_CONDUCTOR_NICE").ok().as_deref());
        let parent_nice = read_proc_nice(std::process::id()).expect("parent /proc stat readable");
        let expected = resolved.max(parent_nice);
        assert_eq!(
            nice, expected,
            "conductor child must run at the resolved (env-aware) niceness,              floored by the parent's own niceness (EPERM rule)"
        );

        mgr.stop().await.unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn child_pid_is_none_before_start() {
        // A fresh manager has no spawned child, so the memory-attribution sampler
        // gets None and samples only the storage parent until the conductor starts.
        let mgr = ConductorManager::new(
            PathBuf::from("/nonexistent/holochain"),
            PathBuf::from("/nonexistent/conductor-config.yaml"),
            PathBuf::from("/tmp"),
            4444,
            Duration::from_secs(180),
        );
        assert_eq!(mgr.child_pid(), None, "no child before start()");
    }

    #[test]
    fn parses_db_pool_saturation_ratio_and_bounded_kind() {
        let sample = parse_db_pool_saturation(
            "INFO holochain_perf{kind=Dht(uhC0k-secret-dna-id)}: \
             Database read connection is saturated. Util 22662.00%",
        )
        .expect("saturation event");

        assert_eq!(sample.kind, "dht");
        assert!((sample.utilization_ratio - 226.62).abs() < f64::EPSILON);
    }

    #[test]
    fn parses_structured_kind_without_using_identifier_as_label() {
        let sample = parse_db_pool_saturation(
            r#"{"spans":[{"kind":"Authored(CellId(uhC0k-private, uhCAk-private))"}],"fields":{"message":"Database read connection is saturated. Util 350.00%"}}"#,
        )
        .expect("structured saturation event");

        assert_eq!(sample.kind, "authored");
        assert!((sample.utilization_ratio - 3.5).abs() < f64::EPSILON);
    }

    #[test]
    fn ignores_non_saturation_and_invalid_utilization() {
        assert!(parse_db_pool_saturation("Database connection ready").is_none());
        assert!(parse_db_pool_saturation(
            "kind=Dht Database read connection is saturated. Util NaN%"
        )
        .is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn forwards_conductor_output_bytes_unchanged() {
        use tokio::io::AsyncReadExt;

        let (mut child_output, parent_reader) = tokio::io::duplex(1024);
        let (parent_writer, mut captured_output) = tokio::io::duplex(1024);
        let output_state = Arc::new(ConductorOutputState::default());
        let forward = tokio::spawn(forward_conductor_output(
            parent_reader,
            parent_writer,
            "stderr",
            Arc::clone(&output_state),
        ));
        let expected =
            b"kind=Dht Database read connection is saturated. Util 500.00%\nraw-\xff-byte\n";

        child_output.write_all(expected).await.unwrap();
        child_output.shutdown().await.unwrap();

        let mut actual = Vec::new();
        captured_output.read_to_end(&mut actual).await.unwrap();
        forward.await.unwrap();
        assert_eq!(actual, expected);

        // The ring buffer captured both forwarded lines (trailing newline
        // stripped) alongside the byte-for-byte forward.
        assert_eq!(
            output_state.last_lines(10),
            vec![
                "kind=Dht Database read connection is saturated. Util 500.00%".to_string(),
                "raw-\u{fffd}-byte".to_string(),
            ]
        );
    }

    #[test]
    fn clear_conductor_state_refuses_unsafe_paths() {
        // The destructive genesis-heal must NEVER wipe the filesystem root or a
        // relative/footless path if data_dir is misconfigured.
        assert!(ConductorManager::clear_conductor_state(std::path::Path::new("/")).is_err());
        assert!(
            ConductorManager::clear_conductor_state(std::path::Path::new("relative/dir")).is_err()
        );
        assert!(ConductorManager::clear_conductor_state(std::path::Path::new("")).is_err());
    }

    #[test]
    fn clear_conductor_state_empties_a_real_dir() {
        // A genuine data dir is emptied (stale state gone) but the root remains, so
        // the next conductor spawn boots clean.
        let root = std::env::temp_dir().join("elohim_clear_state_test");
        let data = root.join("a/b/holochain");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(data.join("stale.sqlite3"), b"x").unwrap();
        assert!(data.join("stale.sqlite3").exists());

        ConductorManager::clear_conductor_state(&data).unwrap();

        assert!(data.exists(), "data dir root recreated for fresh state");
        assert!(
            !data.join("stale.sqlite3").exists(),
            "genesis-less stale state cleared"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ring_buffer_keeps_the_last_n_and_drops_the_oldest() {
        let mut ring = RingBuffer::new(3);
        assert_eq!(ring.last_n(10), Vec::<String>::new(), "empty ring");

        ring.push("a".to_string());
        ring.push("b".to_string());
        assert_eq!(ring.last_n(10), vec!["a".to_string(), "b".to_string()]);

        // Pushing past capacity evicts the oldest line first (FIFO), never a
        // middle or newest one.
        ring.push("c".to_string());
        ring.push("d".to_string());
        assert_eq!(
            ring.last_n(10),
            vec!["b".to_string(), "c".to_string(), "d".to_string()],
            "oldest ('a') dropped once capacity was exceeded"
        );

        // last_n caps the tail even when the ring holds more.
        assert_eq!(ring.last_n(2), vec!["c".to_string(), "d".to_string()]);
        assert_eq!(ring.last_n(0), Vec::<String>::new());
    }

    // --- Lair pid_file / socket crash-recovery guard ---------------------

    /// Fresh tempdir standing in for a lair_root (`ks/`), pre-populated with
    /// the sensitive files the guard must NEVER touch — every test asserts
    /// these survive.
    fn fresh_lair_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "elohim_lair_guard_test_{tag}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("store_file"), b"sqlcipher-secrets").unwrap();
        std::fs::write(dir.join("store_file-shm"), b"shm").unwrap();
        std::fs::write(dir.join("store_file-wal"), b"wal").unwrap();
        std::fs::write(
            dir.join("lair-keystore-config.yaml"),
            b"connectionUrl: unix://x",
        )
        .unwrap();
        dir
    }

    fn assert_sensitive_lair_files_untouched(lair_root: &std::path::Path) {
        assert_eq!(
            std::fs::read(lair_root.join("store_file")).unwrap(),
            b"sqlcipher-secrets",
            "store_file must never be touched by the guard"
        );
        assert_eq!(
            std::fs::read(lair_root.join("store_file-shm")).unwrap(),
            b"shm"
        );
        assert_eq!(
            std::fs::read(lair_root.join("store_file-wal")).unwrap(),
            b"wal"
        );
        assert!(
            lair_root.join("lair-keystore-config.yaml").exists(),
            "lair-keystore-config.yaml must never be touched by the guard"
        );
    }

    #[test]
    fn heal_stale_lair_socket_removes_empty_pid_file_and_socket() {
        let dir = fresh_lair_root("empty_pid");
        std::fs::write(dir.join("pid_file"), b"").unwrap();
        std::fs::write(dir.join("socket"), b"").unwrap();

        heal_stale_lair_socket(&dir);

        assert!(!dir.join("pid_file").exists(), "empty pid_file removed");
        assert!(!dir.join("socket").exists(), "paired socket removed");
        assert_sensitive_lair_files_untouched(&dir);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn heal_stale_lair_socket_removes_garbage_pid_file_and_socket() {
        let dir = fresh_lair_root("garbage_pid");
        std::fs::write(dir.join("pid_file"), b"not-a-pid\n").unwrap();
        std::fs::write(dir.join("socket"), b"").unwrap();

        heal_stale_lair_socket(&dir);

        assert!(!dir.join("pid_file").exists(), "garbage pid_file removed");
        assert!(!dir.join("socket").exists(), "paired socket removed");
        assert_sensitive_lair_files_untouched(&dir);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn heal_stale_lair_socket_removes_dead_pid_file_and_socket() {
        let dir = fresh_lair_root("dead_pid");
        // A pid extremely unlikely to be alive in this container, and never
        // equal to our own pid or any pid we spawn in these tests.
        std::fs::write(dir.join("pid_file"), b"999999\n").unwrap();
        std::fs::write(dir.join("socket"), b"").unwrap();
        assert!(!pid_is_alive(999_999), "precondition: 999999 must be dead");

        heal_stale_lair_socket(&dir);

        assert!(!dir.join("pid_file").exists(), "dead-pid pid_file removed");
        assert!(!dir.join("socket").exists(), "stale socket removed");
        assert_sensitive_lair_files_untouched(&dir);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn heal_stale_lair_socket_leaves_live_pid_file_and_socket_intact() {
        let dir = fresh_lair_root("live_pid");
        let own_pid = std::process::id();
        std::fs::write(dir.join("pid_file"), own_pid.to_string()).unwrap();
        std::fs::write(dir.join("socket"), b"live-socket-bytes").unwrap();

        heal_stale_lair_socket(&dir);

        assert!(
            dir.join("pid_file").exists(),
            "live-pid pid_file must be left intact — another lair owns it"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("pid_file")).unwrap(),
            own_pid.to_string()
        );
        assert_eq!(
            std::fs::read(dir.join("socket")).unwrap(),
            b"live-socket-bytes",
            "socket must be left intact alongside a live pid_file"
        );
        assert_sensitive_lair_files_untouched(&dir);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn heal_stale_lair_socket_is_a_noop_with_no_pid_file() {
        let dir = fresh_lair_root("no_pid_file");
        std::fs::write(dir.join("socket"), b"orphan-but-no-pid-file").unwrap();

        heal_stale_lair_socket(&dir);

        assert!(
            dir.join("socket").exists(),
            "a socket with no pid_file at all is left alone — nothing to conclude staleness from"
        );
        assert_sensitive_lair_files_untouched(&dir);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn heal_stale_lair_state_is_a_noop_for_missing_lair_root() {
        // A config path that doesn't exist, or that parses to a keystore
        // type this guard doesn't own, must never panic or error — just do
        // nothing. Exercised via the full entry point (config -> lair_root
        // resolution -> heal) rather than the lower-level heal function.
        let missing_config = std::env::temp_dir().join(format!(
            "elohim_lair_guard_test_missing_config_{}.yaml",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&missing_config);
        heal_stale_lair_state(&missing_config); // must not panic

        let dir = fresh_lair_root("other_keystore_type");
        let config_path = dir.join("conductor-config.yaml");
        std::fs::write(
            &config_path,
            format!(
                "keystore:\n  type: lair_server\n  connection_url: unix://{}/socket\n",
                dir.display()
            ),
        )
        .unwrap();
        // Leave a pid_file/socket pair that WOULD be healed under
        // lair_server_in_proc, to prove the non-matching keystore type is
        // what stops it, not an accidental empty lair_root.
        std::fs::write(dir.join("pid_file"), b"").unwrap();
        std::fs::write(dir.join("socket"), b"").unwrap();

        heal_stale_lair_state(&config_path);

        assert!(
            dir.join("pid_file").exists(),
            "a non-in-proc keystore type must never be touched"
        );
        assert!(dir.join("socket").exists());
        assert_sensitive_lair_files_untouched(&dir);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_in_proc_lair_root_parses_type_and_path() {
        let dir = std::env::temp_dir().join(format!(
            "elohim_lair_guard_test_config_parse_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("conductor-config.yaml");
        std::fs::write(
            &config_path,
            format!(
                "data_root_path: \"/var/local/lib/holochain\"\nkeystore:\n  type: lair_server_in_proc\n  lair_root: \"{}/ks\"\n",
                dir.display()
            ),
        )
        .unwrap();

        let resolved = read_in_proc_lair_root(&config_path);
        assert_eq!(resolved, Some(dir.join("ks")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_in_proc_lair_root_none_for_unreadable_config() {
        let missing = std::path::Path::new("/nonexistent/conductor-config.yaml");
        assert_eq!(read_in_proc_lair_root(missing), None);
    }

    #[test]
    fn read_in_proc_lair_root_parses_the_edgenode_template_snippet() {
        // The exact shape emitted by genesis/orchestrator/manifests/humans/
        // _edgenode-conductor.template.yaml (and matched by every real
        // household conductor-config.yaml inspected during this fix).
        let dir = std::env::temp_dir().join(format!(
            "elohim_lair_guard_test_template_snippet_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("conductor-config.yaml");
        std::fs::write(
            &config_path,
            "network:\n  bootstrap_url: \"https://example\"\nkeystore:\n  type: lair_server_in_proc\n  lair_root: \"/var/local/lib/holochain/ks\"\n",
        )
        .unwrap();

        assert_eq!(
            read_in_proc_lair_root(&config_path),
            Some(PathBuf::from("/var/local/lib/holochain/ks"))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_in_proc_lair_root_none_for_config_with_no_keystore_block() {
        let dir = std::env::temp_dir().join(format!(
            "elohim_lair_guard_test_no_keystore_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("conductor-config.yaml");
        std::fs::write(
            &config_path,
            "network:\n  bootstrap_url: \"https://example\"\nadmin_interfaces: []\n",
        )
        .unwrap();

        assert_eq!(read_in_proc_lair_root(&config_path), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_in_proc_lair_root_none_for_non_in_proc_keystore_type() {
        let dir = std::env::temp_dir().join(format!(
            "elohim_lair_guard_test_remote_keystore_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("conductor-config.yaml");
        std::fs::write(
            &config_path,
            format!(
                "keystore:\n  type: lair_server\n  connection_url: \"unix://{}/socket\"\n",
                dir.display()
            ),
        )
        .unwrap();

        assert_eq!(
            read_in_proc_lair_root(&config_path),
            None,
            "a non-in-proc keystore type is not this node's lair root to touch"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn heal_stale_lair_state_end_to_end_via_real_config_shape() {
        // Wires the full path: a real conductor-config.yaml (the shape this
        // codebase's templates generate) -> lair_root resolution -> the
        // heal itself — the exact call `start()` makes.
        let dir = fresh_lair_root("end_to_end");
        let config_path = dir.parent().unwrap().join(format!(
            "{}_conductor-config.yaml",
            dir.file_name().unwrap().to_string_lossy()
        ));
        std::fs::write(
            &config_path,
            format!(
                "network:\n  target_arc_factor: 1\ndata_root_path: \"/var/local/lib/holochain\"\nkeystore:\n  type: lair_server_in_proc\n  lair_root: \"{}\"\nadmin_interfaces: []\n",
                dir.display()
            ),
        )
        .unwrap();
        std::fs::write(dir.join("pid_file"), b"999999\n").unwrap();
        std::fs::write(dir.join("socket"), b"").unwrap();

        heal_stale_lair_state(&config_path);

        assert!(!dir.join("pid_file").exists());
        assert!(!dir.join("socket").exists());
        assert_sensitive_lair_files_untouched(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&config_path);
    }

    #[test]
    fn readiness_outcome_prefers_child_death_over_attempt_budget() {
        // Table test: (child_exited, attempt, max_retries) -> expected outcome.
        // A dead child always wins immediately, even on attempt 1 of a large
        // budget — that is the entire point of the fix (never wait out a
        // dead child's retry window).
        let cases: &[(Option<ExitStatusLike>, u32, u32, ReadinessOutcome)] = &[
            // Child alive, attempts remain: retry.
            (None, 1, 60, ReadinessOutcome::Retry),
            (None, 59, 60, ReadinessOutcome::Retry),
            // Child alive, attempts exhausted: give up (but child_alive is
            // reported true by the caller).
            (None, 60, 60, ReadinessOutcome::GiveUp),
            // Child exited: stop immediately regardless of how many attempts
            // remain — including the very first attempt.
            (Some(()), 1, 60, ReadinessOutcome::ChildExited),
            (Some(()), 30, 60, ReadinessOutcome::ChildExited),
            (Some(()), 60, 60, ReadinessOutcome::ChildExited),
        ];

        for (child_exited, attempt, max_retries, expected) in cases.iter().copied() {
            assert_eq!(
                classify_readiness_outcome(child_exited, attempt, max_retries),
                expected,
                "child_exited={child_exited:?} attempt={attempt} max_retries={max_retries}"
            );
        }
    }
}
