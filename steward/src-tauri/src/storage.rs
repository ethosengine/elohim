//! Managed elohim-storage sidecar process.
//!
//! Spawns elohim-storage as a child process with `kill_on_drop(true)` so it
//! dies automatically when the Tauri app exits. Health-checks the /health
//! endpoint before returning, matching the storage-start.sh pattern.

use std::path::PathBuf;
use tokio::process::{Child, Command};

/// Running elohim-storage child process.
///
/// Dropping this struct kills the child (via `kill_on_drop`).
pub struct StorageProcess {
    _child: Child,
    pub port: u16,
}

/// Configuration for spawning elohim-storage.
pub struct StorageConfig {
    pub binary_path: PathBuf,
    pub port: u16,
    pub storage_dir: PathBuf,
    pub enable_content_db: bool,
}

impl StorageProcess {
    /// Spawn elohim-storage and wait for it to become healthy.
    ///
    /// Polls GET /health up to 15 times (1s apart), matching storage-start.sh.
    /// Returns Err if the process exits early or never becomes healthy.
    pub async fn spawn(config: StorageConfig) -> Result<Self, String> {
        let mut cmd = Command::new(&config.binary_path);
        cmd.arg("--http-port")
            .arg(config.port.to_string())
            .arg("--storage-dir")
            .arg(&config.storage_dir);

        if config.enable_content_db {
            cmd.arg("--enable-content-db");
        }

        // Suppress stdin, let stdout/stderr flow to parent's log
        cmd.stdin(std::process::Stdio::null());
        cmd.kill_on_drop(true);

        let child = cmd.spawn().map_err(|e| {
            format!(
                "Failed to spawn elohim-storage at {}: {}",
                config.binary_path.display(),
                e
            )
        })?;

        log::info!(
            "Spawned elohim-storage (pid {}) on port {}",
            child.id().unwrap_or(0),
            config.port
        );

        // Health check loop — 15 attempts, 1s each
        let health_url = format!("http://localhost:{}/health", config.port);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        for attempt in 1..=15 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            match client.get(&health_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    log::info!(
                        "elohim-storage healthy after {} attempt(s)",
                        attempt
                    );
                    return Ok(Self {
                        _child: child,
                        port: config.port,
                    });
                }
                Ok(resp) => {
                    log::debug!(
                        "Health check attempt {}/15: status {}",
                        attempt,
                        resp.status()
                    );
                }
                Err(e) => {
                    log::debug!("Health check attempt {}/15: {}", attempt, e);
                }
            }
        }

        Err("elohim-storage did not become healthy within 15 seconds".to_string())
    }
}

/// Locate the elohim-storage binary.
///
/// - **Dev**: `CARGO_MANIFEST_DIR/../../holochain/target/release/elohim-storage`
/// - **Prod**: `{exe_dir}/../resources/bin/elohim-storage` (from bundle.resources)
pub fn find_binary() -> Result<PathBuf, String> {
    if tauri::is_dev() {
        // Dev build: binary is in the holochain workspace target dir
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let bin = manifest_dir
            .join("../../holochain/target/release/elohim-storage");
        let bin = bin
            .canonicalize()
            .map_err(|e| format!("elohim-storage binary not found at {}: {}", bin.display(), e))?;
        Ok(bin)
    } else {
        // Production build: bundled in resources/bin/
        let exe_dir = std::env::current_exe()
            .map_err(|e| format!("Cannot determine exe path: {}", e))?
            .parent()
            .ok_or("Cannot determine exe directory")?
            .to_path_buf();

        // Tauri bundle.resources places files relative to the executable.
        // On Linux deb: /usr/lib/<app>/bin/<exe> → resources at /usr/lib/<app>/resources/
        // On AppImage: resources are beside the executable
        let bin = exe_dir.join("../resources/bin/elohim-storage");
        if bin.exists() {
            // Ensure execute permission on first run
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(&bin) {
                    let mut perms = meta.permissions();
                    if perms.mode() & 0o111 == 0 {
                        perms.set_mode(perms.mode() | 0o755);
                        let _ = std::fs::set_permissions(&bin, perms);
                    }
                }
            }
            return Ok(bin);
        }

        Err(format!(
            "elohim-storage binary not found (checked {})",
            bin.display()
        ))
    }
}
