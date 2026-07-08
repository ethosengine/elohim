//! Host filesystem capability profiles for EPRFS projection surfaces.
//!
//! This crate describes what a local target can preserve. It deliberately does
//! not mount anything and does not know about a specific storage backend.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostProfile {
    pub name: String,
    pub symlinks: SymlinkMode,
    pub executable_bits: Capability,
    pub case_sensitive_paths: Capability,
    pub extended_attributes: Capability,
    pub atomic_rename: Capability,
    pub peer_sync: PeerSync,
}

impl HostProfile {
    pub fn current_platform() -> Self {
        #[cfg(target_os = "linux")]
        {
            return Self::linux_native();
        }

        #[cfg(target_os = "macos")]
        {
            return Self::macos_native();
        }

        #[cfg(target_os = "windows")]
        {
            return Self::windows_native();
        }

        #[allow(unreachable_code)]
        Self::portable_directory()
    }

    pub fn linux_native() -> Self {
        Self {
            name: "linux-native".into(),
            symlinks: SymlinkMode::Native,
            executable_bits: Capability::Supported,
            case_sensitive_paths: Capability::Supported,
            extended_attributes: Capability::Unknown,
            atomic_rename: Capability::Supported,
            peer_sync: PeerSync::None,
        }
    }

    pub fn macos_native() -> Self {
        Self {
            name: "macos-native".into(),
            symlinks: SymlinkMode::Native,
            executable_bits: Capability::Supported,
            case_sensitive_paths: Capability::Unknown,
            extended_attributes: Capability::Unknown,
            atomic_rename: Capability::Supported,
            peer_sync: PeerSync::None,
        }
    }

    pub fn windows_native() -> Self {
        Self {
            name: "windows-native".into(),
            symlinks: SymlinkMode::Marker,
            executable_bits: Capability::Unsupported,
            case_sensitive_paths: Capability::Unknown,
            extended_attributes: Capability::Unknown,
            atomic_rename: Capability::Supported,
            peer_sync: PeerSync::None,
        }
    }

    pub fn portable_directory() -> Self {
        Self {
            name: "portable-directory".into(),
            symlinks: SymlinkMode::Marker,
            executable_bits: Capability::Unsupported,
            case_sensitive_paths: Capability::Unknown,
            extended_attributes: Capability::Unsupported,
            atomic_rename: Capability::Unknown,
            peer_sync: PeerSync::Unknown,
        }
    }

    pub fn peer_managed_directory(profile: impl Into<String>) -> Self {
        Self {
            name: format!("peer-managed-directory:{}", profile.into()),
            symlinks: SymlinkMode::Marker,
            executable_bits: Capability::Unsupported,
            case_sensitive_paths: Capability::Unknown,
            extended_attributes: Capability::Unsupported,
            atomic_rename: Capability::Unknown,
            peer_sync: PeerSync::ManagedProjection,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    Supported,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SymlinkMode {
    Native,
    Marker,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PeerSync {
    None,
    ManagedProjection,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_managed_profile_uses_marker_safe_semantics() {
        let profile = HostProfile::peer_managed_directory("lab");

        assert_eq!(profile.name, "peer-managed-directory:lab");
        assert_eq!(profile.symlinks, SymlinkMode::Marker);
        assert_eq!(profile.executable_bits, Capability::Unsupported);
        assert_eq!(profile.peer_sync, PeerSync::ManagedProjection);
    }
}
