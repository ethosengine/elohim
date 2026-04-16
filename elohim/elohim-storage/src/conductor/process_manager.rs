//! Conductor Process Manager
//!
//! Spawns the Holochain conductor as a child process, monitors it,
//! and provides readiness checks via AdminWebsocket connection.
//!
//! Part of the elohim-node consolidation — merging 4 k8s containers into 1.
//! Instead of running the conductor as a separate container, elohim-storage
//! spawns and manages it as a child process.

use holochain_client::AdminWebsocket;
use std::path::PathBuf;
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
    ) -> Self {
        Self {
            conductor_binary,
            config_path,
            data_dir,
            admin_port,
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

        for attempt in 1..=max_retries {
            match AdminWebsocket::connect(&addr, None).await {
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
}
