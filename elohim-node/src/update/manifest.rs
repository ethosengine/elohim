//! Update manifest format
//!
//! The manifest is served by doorways at `/api/updates/manifest`
//! and contains information about available releases.

use serde::{Deserialize, Serialize};

/// Update manifest served by doorway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    /// Schema version for forward compatibility
    pub schema_version: u32,

    /// Timestamp when manifest was generated
    pub generated_at: u64,

    /// Available releases
    pub releases: Vec<Release>,

    /// Minimum supported version (older versions must update)
    pub minimum_version: Option<String>,

    /// Public key for signature verification (base64 encoded)
    pub signing_key: Option<String>,
}

/// A release in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    /// Version string (semver)
    pub version: String,

    /// Release channel
    pub channel: ReleaseChannel,

    /// Target OS (linux, macos, windows)
    pub os: String,

    /// Target architecture (x86_64, aarch64, armv7)
    pub arch: String,

    /// Download size in bytes
    pub size_bytes: u64,

    /// SHA256 checksum of the download
    pub checksum: String,

    /// Ed25519 signature of the checksum (base64 encoded)
    pub signature: Option<String>,

    /// Release notes (markdown)
    pub release_notes: Option<String>,

    /// When this release was published
    pub published_at: u64,

    /// Whether this is a critical security update
    #[serde(default)]
    pub critical: bool,
}

/// Release channels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseChannel {
    /// Stable releases (default)
    #[default]
    Stable,
    /// Beta releases for testing
    Beta,
    /// Nightly builds for development
    Nightly,
}

impl std::fmt::Display for ReleaseChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReleaseChannel::Stable => write!(f, "stable"),
            ReleaseChannel::Beta => write!(f, "beta"),
            ReleaseChannel::Nightly => write!(f, "nightly"),
        }
    }
}

impl UpdateManifest {
    /// Create a new empty manifest
    pub fn new() -> Self {
        Self {
            schema_version: 1,
            generated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            releases: Vec::new(),
            minimum_version: None,
            signing_key: None,
        }
    }

    /// Find the latest release for a channel and platform
    pub fn find_release(&self, channel: &ReleaseChannel, arch: &str, os: &str) -> Option<&Release> {
        self.releases
            .iter()
            .filter(|r| r.channel == *channel && r.arch == arch && r.os == os)
            .max_by(|a, b| compare_versions(&a.version, &b.version))
    }

    /// Check if a version is below minimum supported
    #[allow(dead_code)]
    pub fn is_version_supported(&self, version: &str) -> bool {
        match &self.minimum_version {
            Some(min) => !is_older_version(version, min),
            None => true,
        }
    }
}

/// Compare two semver strings
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
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

    parse(a).cmp(&parse(b))
}

#[allow(dead_code)]
fn is_older_version(version: &str, than: &str) -> bool {
    compare_versions(version, than) == std::cmp::Ordering::Less
}

/// Example manifest for a doorway to serve
impl Default for UpdateManifest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_release() {
        let manifest = UpdateManifest {
            schema_version: 1,
            generated_at: 0,
            releases: vec![
                Release {
                    version: "0.1.0".to_string(),
                    channel: ReleaseChannel::Stable,
                    os: "linux".to_string(),
                    arch: "x86_64".to_string(),
                    size_bytes: 1000,
                    checksum: "abc".to_string(),
                    signature: None,
                    release_notes: None,
                    published_at: 0,
                    critical: false,
                },
                Release {
                    version: "0.2.0".to_string(),
                    channel: ReleaseChannel::Stable,
                    os: "linux".to_string(),
                    arch: "x86_64".to_string(),
                    size_bytes: 1100,
                    checksum: "def".to_string(),
                    signature: None,
                    release_notes: None,
                    published_at: 0,
                    critical: false,
                },
            ],
            minimum_version: None,
            signing_key: None,
        };

        let release = manifest.find_release(&ReleaseChannel::Stable, "x86_64", "linux");
        assert!(release.is_some());
        assert_eq!(release.unwrap().version, "0.2.0");
    }
}
