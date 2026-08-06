//! Conductor Process Manager
//!
//! Spawns the Holochain conductor as a child process, monitors it,
//! and provides readiness checks via AdminWebsocket connection.
//!
//! Part of the elohim-node consolidation — merging 4 k8s containers into 1.
//! Instead of running the conductor as a separate container, elohim-storage
//! spawns and manages it as a child process.

use holochain_client::{AdminWebsocket, WebsocketConfig};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

const DB_POOL_SATURATION_MARKER: &str = "Database read connection is saturated. Util ";

#[derive(Debug, PartialEq)]
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

/// Drain one conductor output pipe, preserving its bytes on the parent output
/// while projecting the existing saturation event into Prometheus. A broken
/// parent output must not stop draining the child pipe (which would deadlock a
/// log-heavy conductor), so forwarding errors are warned once and then ignored.
async fn forward_conductor_output<R, W>(reader: R, mut writer: W, stream: &'static str)
where
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
                }
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

        let mut child = Command::new(&self.conductor_binary)
            .arg("--config-path")
            .arg(&self.config_path)
            .arg("--piped")
            .env("HOLOCHAIN_DATA_DIR", &self.data_dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| ConductorError::SpawnFailed(e.to_string()))?;

        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(forward_conductor_output(
                stdout,
                tokio::io::stdout(),
                "stdout",
            ));
        }
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(forward_conductor_output(
                stderr,
                tokio::io::stderr(),
                "stderr",
            ));
        }

        let pid = child.id().unwrap_or(0);
        info!(pid = pid, "Conductor process spawned");

        self.child = Some(child);
        Ok(())
    }

    /// Wait for the conductor to become ready by connecting to the AdminWebsocket.
    ///
    /// Retries up to `max_retries` times with a 2-second delay between attempts.
    /// Returns the connected AdminWebsocket on success.
    pub async fn wait_for_ready(&self, max_retries: u32) -> Result<AdminWebsocket, ConductorError> {
        let addr = format!("localhost:{}", self.admin_port);
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
            match AdminWebsocket::connect_with_config(&addr, ws_config.clone(), None).await {
                Ok(ws) => {
                    info!(
                        attempt = attempt,
                        "Conductor is ready — AdminWebsocket connected"
                    );
                    return Ok(ws);
                }
                Err(e) => {
                    if attempt < max_retries {
                        warn!(
                            attempt = attempt,
                            max_retries = max_retries,
                            error = %e,
                            "Conductor not ready yet, retrying in 2s"
                        );
                        sleep(Duration::from_secs(2)).await;
                    } else {
                        error!(
                            attempts = max_retries,
                            error = %e,
                            "Conductor failed to become ready"
                        );
                        return Err(ConductorError::NotReady(format!(
                            "Failed after {} attempts: {}",
                            max_retries, e
                        )));
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

    #[error("Conductor not ready: {0}")]
    NotReady(String),

    #[error("Failed to stop conductor: {0}")]
    StopFailed(String),

    #[error("Conductor genesis-heal failed: {0}")]
    HealFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let forward = tokio::spawn(forward_conductor_output(
            parent_reader,
            parent_writer,
            "test",
        ));
        let expected =
            b"kind=Dht Database read connection is saturated. Util 500.00%\nraw-\xff-byte\n";

        child_output.write_all(expected).await.unwrap();
        child_output.shutdown().await.unwrap();

        let mut actual = Vec::new();
        captured_output.read_to_end(&mut actual).await.unwrap();
        forward.await.unwrap();
        assert_eq!(actual, expected);
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
}
