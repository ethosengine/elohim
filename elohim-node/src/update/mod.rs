//! Auto-update module
//!
//! Handles automatic updates when connecting to a doorway:
//! 1. Check doorway for latest version manifest
//! 2. Compare with current version
//! 3. Download and verify update
//! 4. Apply atomically with rollback capability
//! 5. Restart if needed
//!
//! Updates happen BEFORE sync to ensure version compatibility.

pub mod apply;
pub mod download;
pub mod manifest;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

pub use apply::UpdateApplier;
pub use download::UpdateDownloader;
pub use manifest::{ReleaseChannel, UpdateManifest};

/// Current version of this binary
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Update check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateStatus {
    /// No update available, current version is latest
    UpToDate,
    /// Update available
    UpdateAvailable {
        current: String,
        latest: String,
        release_notes: Option<String>,
        size_bytes: u64,
    },
    /// Update is being downloaded
    Downloading { progress_percent: u8 },
    /// Update downloaded, ready to apply
    ReadyToApply { version: String },
    /// Update is being applied
    Applying,
    /// Update failed
    Failed { error: String },
    /// Requires restart to complete
    PendingRestart { version: String },
}

/// Update configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// Enable automatic updates
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Release channel (stable, beta, nightly)
    #[serde(default)]
    pub channel: ReleaseChannel,

    /// Check for updates on startup
    #[serde(default = "default_true")]
    pub check_on_startup: bool,

    /// Check interval in seconds (0 = disabled)
    #[serde(default = "default_check_interval")]
    pub check_interval_secs: u64,

    /// Auto-apply updates without prompting
    #[serde(default)]
    pub auto_apply: bool,

    /// Directory for storing update files
    #[serde(default = "default_update_dir")]
    pub update_dir: PathBuf,

    /// Keep N previous versions for rollback
    #[serde(default = "default_keep_versions")]
    pub keep_versions: usize,
}

fn default_true() -> bool {
    true
}
fn default_check_interval() -> u64 {
    3600
} // 1 hour
fn default_update_dir() -> PathBuf {
    PathBuf::from("/var/lib/elohim/updates")
}
fn default_keep_versions() -> usize {
    2
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            channel: ReleaseChannel::Stable,
            check_on_startup: true,
            check_interval_secs: default_check_interval(),
            auto_apply: false,
            update_dir: default_update_dir(),
            keep_versions: default_keep_versions(),
        }
    }
}

/// Update service that manages the update lifecycle
pub struct UpdateService {
    config: UpdateConfig,
    doorway_url: Option<String>,
    status: UpdateStatus,
    downloader: UpdateDownloader,
    applier: UpdateApplier,
}

impl UpdateService {
    pub fn new(config: UpdateConfig) -> Self {
        let update_dir = config.update_dir.clone();
        Self {
            config: config.clone(),
            doorway_url: None,
            status: UpdateStatus::UpToDate,
            downloader: UpdateDownloader::new(update_dir.clone()),
            applier: UpdateApplier::new(update_dir, config.keep_versions),
        }
    }

    /// Set the doorway URL to check for updates
    pub fn set_doorway(&mut self, url: String) {
        self.doorway_url = Some(url);
    }

    /// Get current update status
    pub fn status(&self) -> &UpdateStatus {
        &self.status
    }

    /// Check for updates from doorway
    pub async fn check_for_updates(&mut self) -> Result<UpdateStatus, UpdateError> {
        let doorway_url = self.doorway_url.as_ref().ok_or(UpdateError::NoDoorway)?;

        info!("Checking for updates from {}", doorway_url);

        // Fetch manifest from doorway
        let manifest_url = format!("{}/api/updates/manifest", doorway_url);
        let manifest = self.fetch_manifest(&manifest_url).await?;

        // Find release for our channel and architecture
        let arch = std::env::consts::ARCH;
        let os = std::env::consts::OS;

        let release = manifest
            .find_release(&self.config.channel, arch, os)
            .ok_or(UpdateError::NoReleaseForPlatform)?;

        // Compare versions
        if is_newer_version(&release.version, CURRENT_VERSION) {
            info!(
                "Update available: {} -> {} ({})",
                CURRENT_VERSION, release.version, release.channel
            );

            self.status = UpdateStatus::UpdateAvailable {
                current: CURRENT_VERSION.to_string(),
                latest: release.version.clone(),
                release_notes: release.release_notes.clone(),
                size_bytes: release.size_bytes,
            };
        } else {
            info!("Already running latest version: {}", CURRENT_VERSION);
            self.status = UpdateStatus::UpToDate;
        }

        Ok(self.status.clone())
    }

    /// Download and apply update
    pub async fn apply_update(&mut self) -> Result<(), UpdateError> {
        let doorway_url = self.doorway_url.as_ref().ok_or(UpdateError::NoDoorway)?;

        // Get the manifest again to get download URL
        let manifest_url = format!("{}/api/updates/manifest", doorway_url);
        let manifest = self.fetch_manifest(&manifest_url).await?;

        let arch = std::env::consts::ARCH;
        let os = std::env::consts::OS;

        let release = manifest
            .find_release(&self.config.channel, arch, os)
            .ok_or(UpdateError::NoReleaseForPlatform)?;

        // Download update
        info!(
            "Downloading update {} ({} bytes)",
            release.version, release.size_bytes
        );
        self.status = UpdateStatus::Downloading {
            progress_percent: 0,
        };

        let download_url = format!(
            "{}/api/updates/download/{}/{}/{}",
            doorway_url, release.version, os, arch
        );

        let update_path = self
            .downloader
            .download(
                &download_url,
                &release.version,
                &release.checksum,
                |progress| {
                    // Update progress (TODO: need async status updates)
                    info!("Download progress: {}%", progress);
                },
            )
            .await?;

        self.status = UpdateStatus::ReadyToApply {
            version: release.version.clone(),
        };

        // Verify signature
        if let Some(ref signature) = release.signature {
            info!("Verifying update signature");
            self.downloader.verify_signature(&update_path, signature)?;
        }

        // Apply update atomically
        info!("Applying update");
        self.status = UpdateStatus::Applying;

        let version = release.version.clone();
        self.applier.apply(&update_path, &version)?;

        info!("Update applied successfully, restart required");
        self.status = UpdateStatus::PendingRestart { version };

        Ok(())
    }

    /// Rollback to previous version
    pub fn rollback(&mut self) -> Result<(), UpdateError> {
        info!("Rolling back to previous version");
        self.applier.rollback()?;
        Ok(())
    }

    /// Check and auto-apply if configured
    #[allow(dead_code)]
    pub async fn check_and_auto_apply(&mut self) -> Result<bool, UpdateError> {
        if !self.config.enabled {
            return Ok(false);
        }

        self.check_for_updates().await?;

        if let UpdateStatus::UpdateAvailable { .. } = &self.status {
            if self.config.auto_apply {
                self.apply_update().await?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn fetch_manifest(&self, url: &str) -> Result<UpdateManifest, UpdateError> {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| UpdateError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(UpdateError::Network(format!(
                "Failed to fetch manifest: {}",
                response.status()
            )));
        }

        let manifest: UpdateManifest = response
            .json()
            .await
            .map_err(|e| UpdateError::InvalidManifest(e.to_string()))?;

        Ok(manifest)
    }
}

/// Update errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum UpdateError {
    #[error("No doorway URL configured")]
    NoDoorway,

    #[error("Network error: {0}")]
    Network(String),

    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("No release available for this platform")]
    NoReleaseForPlatform,

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Checksum mismatch")]
    ChecksumMismatch,

    #[allow(dead_code)]
    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Apply failed: {0}")]
    ApplyFailed(String),

    #[error("Rollback failed: {0}")]
    RollbackFailed(String),

    #[error("IO error: {0}")]
    Io(String),
}

/// Compare semantic versions, returns true if `new` is newer than `current`
fn is_newer_version(new: &str, current: &str) -> bool {
    let parse = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<&str> = v.trim_start_matches('v').split('.').collect();
        (
            parts.first().and_then(|s| s.parse().ok()).unwrap_or(0),
            parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
            parts
                .get(2)
                .and_then(|s| s.split('-').next()?.parse().ok())
                .unwrap_or(0),
        )
    };

    let new_v = parse(new);
    let cur_v = parse(current);

    new_v > cur_v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("0.2.0", "0.1.0"));
        assert!(is_newer_version("1.0.0", "0.9.9"));
        assert!(is_newer_version("0.1.1", "0.1.0"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
        assert!(!is_newer_version("0.0.9", "0.1.0"));
        assert!(is_newer_version("v0.2.0", "0.1.0"));
        assert!(is_newer_version("0.2.0-beta", "0.1.0"));
    }
}
