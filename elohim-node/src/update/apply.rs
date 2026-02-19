//! Atomic update application with rollback
//!
//! Updates are applied atomically using a staging approach:
//! 1. Current binary is backed up to versions/
//! 2. New binary is staged in staging/
//! 3. Atomic rename from staging to target
//! 4. On failure, rollback from versions/
//!
//! Directory structure:
//!   /var/lib/elohim/updates/
//!     ├── elohim-node-0.2.0      # Downloaded update
//!     ├── staging/
//!     │   └── elohim-node        # Staged before atomic swap
//!     └── versions/
//!         ├── elohim-node.0.1.0  # Previous version (for rollback)
//!         └── elohim-node.0.0.9  # Older version

use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

use super::UpdateError;

/// Handles atomic update application
pub struct UpdateApplier {
    update_dir: PathBuf,
    staging_dir: PathBuf,
    versions_dir: PathBuf,
    keep_versions: usize,
}

impl UpdateApplier {
    pub fn new(update_dir: PathBuf, keep_versions: usize) -> Self {
        let staging_dir = update_dir.join("staging");
        let versions_dir = update_dir.join("versions");

        Self {
            update_dir,
            staging_dir,
            versions_dir,
            keep_versions,
        }
    }

    /// Apply an update atomically
    pub fn apply(&self, update_path: &Path, version: &str) -> Result<(), UpdateError> {
        info!("Applying update {} from {:?}", version, update_path);

        // Ensure directories exist
        fs::create_dir_all(&self.staging_dir).map_err(|e| UpdateError::Io(e.to_string()))?;
        fs::create_dir_all(&self.versions_dir).map_err(|e| UpdateError::Io(e.to_string()))?;

        // Get current binary path
        let current_exe = std::env::current_exe().map_err(|e| UpdateError::Io(e.to_string()))?;

        info!("Current executable: {:?}", current_exe);

        // Step 1: Backup current version
        let current_version = super::CURRENT_VERSION;
        let backup_path = self
            .versions_dir
            .join(format!("elohim-node.{}", current_version));

        if current_exe.exists() && !backup_path.exists() {
            info!("Backing up current version to {:?}", backup_path);
            fs::copy(&current_exe, &backup_path)
                .map_err(|e| UpdateError::ApplyFailed(format!("Backup failed: {}", e)))?;
        }

        // Step 2: Stage new binary
        let staged_path = self.staging_dir.join("elohim-node");
        info!("Staging new binary to {:?}", staged_path);

        fs::copy(update_path, &staged_path)
            .map_err(|e| UpdateError::ApplyFailed(format!("Staging failed: {}", e)))?;

        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&staged_path)
                .map_err(|e| UpdateError::Io(e.to_string()))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&staged_path, perms).map_err(|e| UpdateError::Io(e.to_string()))?;
        }

        // Step 3: Atomic swap
        // On Linux, we can't replace a running executable directly
        // Instead, we use the self-replace pattern:
        // 1. Rename current to .old
        // 2. Rename staged to current
        // 3. Delete .old on next start

        let old_path = current_exe.with_extension("old");

        // Clean up any previous .old file
        if old_path.exists() {
            fs::remove_file(&old_path).ok();
        }

        info!("Performing atomic swap");

        // Move current to .old (if current exists and is different from staged)
        if current_exe.exists() {
            fs::rename(&current_exe, &old_path)
                .map_err(|e| UpdateError::ApplyFailed(format!(
                    "Failed to move current binary: {}. You may need to restart with the new binary manually.", e
                )))?;
        }

        // Move staged to current
        match fs::rename(&staged_path, &current_exe) {
            Ok(_) => {
                info!("Update applied successfully");
            }
            Err(e) => {
                // Rollback: restore old binary
                error!("Failed to install new binary: {}", e);
                if old_path.exists() {
                    fs::rename(&old_path, &current_exe).ok();
                }
                return Err(UpdateError::ApplyFailed(format!(
                    "Failed to install new binary: {}",
                    e
                )));
            }
        }

        // Clean up old versions, keeping only the last N
        self.cleanup_old_versions()?;

        // Mark update complete - create a marker file
        let marker_path = self.update_dir.join(".update-pending-restart");
        fs::write(&marker_path, version).map_err(|e| UpdateError::Io(e.to_string()))?;

        info!("Update to {} complete, restart required", version);
        Ok(())
    }

    /// Rollback to the previous version
    pub fn rollback(&self) -> Result<(), UpdateError> {
        info!("Rolling back to previous version");

        // Find the most recent backup
        let backups = self.list_backups()?;
        let latest_backup = backups.first().ok_or(UpdateError::RollbackFailed(
            "No backup available".to_string(),
        ))?;

        info!("Rolling back to {:?}", latest_backup);

        let current_exe = std::env::current_exe().map_err(|e| UpdateError::Io(e.to_string()))?;

        // Same atomic swap pattern
        let old_path = current_exe.with_extension("old");

        if old_path.exists() {
            fs::remove_file(&old_path).ok();
        }

        if current_exe.exists() {
            fs::rename(&current_exe, &old_path)
                .map_err(|e| UpdateError::RollbackFailed(e.to_string()))?;
        }

        fs::copy(latest_backup, &current_exe)
            .map_err(|e| UpdateError::RollbackFailed(e.to_string()))?;

        info!("Rollback complete, restart required");
        Ok(())
    }

    /// List available backup versions, newest first
    pub fn list_backups(&self) -> Result<Vec<PathBuf>, UpdateError> {
        if !self.versions_dir.exists() {
            return Ok(Vec::new());
        }

        let mut backups: Vec<(PathBuf, std::time::SystemTime)> = fs::read_dir(&self.versions_dir)
            .map_err(|e| UpdateError::Io(e.to_string()))?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                let modified = entry.metadata().ok()?.modified().ok()?;
                Some((path, modified))
            })
            .collect();

        // Sort by modification time, newest first
        backups.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(backups.into_iter().map(|(p, _)| p).collect())
    }

    /// Clean up old versions, keeping only the last N
    fn cleanup_old_versions(&self) -> Result<(), UpdateError> {
        let backups = self.list_backups()?;

        if backups.len() > self.keep_versions {
            info!("Cleaning up old versions, keeping {}", self.keep_versions);

            for old_backup in backups.iter().skip(self.keep_versions) {
                info!("Removing old backup: {:?}", old_backup);
                fs::remove_file(old_backup).ok();
            }
        }

        Ok(())
    }

    /// Check if an update is pending restart
    #[allow(dead_code)]
    pub fn is_restart_pending(&self) -> Option<String> {
        let marker_path = self.update_dir.join(".update-pending-restart");
        fs::read_to_string(&marker_path).ok()
    }

    /// Clear the restart pending marker (called after successful restart)
    #[allow(dead_code)]
    pub fn clear_restart_pending(&self) {
        let marker_path = self.update_dir.join(".update-pending-restart");
        fs::remove_file(&marker_path).ok();

        // Also clean up any .old files
        if let Ok(current_exe) = std::env::current_exe() {
            let old_path = current_exe.with_extension("old");
            if old_path.exists() {
                fs::remove_file(&old_path).ok();
            }
        }
    }
}

/// Request a restart of the service
#[allow(dead_code)]
pub fn request_restart() -> Result<(), UpdateError> {
    info!("Requesting service restart");

    #[cfg(target_os = "linux")]
    {
        // Try systemd first
        let status = std::process::Command::new("systemctl")
            .args(["restart", "elohim-node"])
            .status();

        match status {
            Ok(s) if s.success() => {
                info!("Restart requested via systemd");
                return Ok(());
            }
            _ => {
                warn!("systemctl restart failed, trying SIGHUP");
            }
        }

        // Fall back to SIGHUP (if the process handles it for reload)
        unsafe {
            libc::raise(libc::SIGHUP);
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        warn!("Auto-restart not implemented for this platform");
    }

    Ok(())
}
