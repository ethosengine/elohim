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
use tokio::process::{Child, Command};
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

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

        let child = Command::new(&self.conductor_binary)
            .arg("--config-path")
            .arg(&self.config_path)
            .arg("--piped")
            .env("HOLOCHAIN_DATA_DIR", &self.data_dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| ConductorError::SpawnFailed(e.to_string()))?;

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
