//! Build metadata captured at compile time.

use serde::{Deserialize, Serialize};

/// Build-time metadata for a service binary.
///
/// Values are injected via environment variables set by the CI pipeline.
/// When built locally without those variables, fields default to `"unknown"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    pub version: String,
    pub commit: String,
    pub commit_full: String,
    pub build_time: String,
    pub rustc_version: String,
    pub service: String,
}

impl BuildInfo {
    /// Create a new `BuildInfo` for the given service name.
    ///
    /// Reads `CARGO_PKG_VERSION` at compile time and optional CI env vars
    /// (`GIT_COMMIT_SHORT`, `GIT_COMMIT_FULL`, `BUILD_TIMESTAMP`, `RUSTC_VERSION`).
    pub fn new(service: &str) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            commit: option_env!("GIT_COMMIT_SHORT")
                .unwrap_or("unknown")
                .to_string(),
            commit_full: option_env!("GIT_COMMIT_FULL")
                .unwrap_or("unknown")
                .to_string(),
            build_time: option_env!("BUILD_TIMESTAMP")
                .unwrap_or("unknown")
                .to_string(),
            rustc_version: option_env!("RUSTC_VERSION")
                .unwrap_or("unknown")
                .to_string(),
            service: service.to_string(),
        }
    }

    /// HTTP `User-Agent`-style string: `service/version` or `service/version+commit`.
    pub fn user_agent(&self) -> String {
        if self.commit == "unknown" {
            format!("{}/{}", self.service, self.version)
        } else {
            format!("{}/{}+{}", self.service, self.version, self.commit)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_populates_version_from_cargo() {
        let info = BuildInfo::new("doorway");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.service, "doorway");
    }

    #[test]
    fn test_user_agent_without_commit() {
        let info = BuildInfo {
            version: "1.2.3".to_string(),
            commit: "unknown".to_string(),
            commit_full: "unknown".to_string(),
            build_time: "unknown".to_string(),
            rustc_version: "unknown".to_string(),
            service: "doorway".to_string(),
        };
        assert_eq!(info.user_agent(), "doorway/1.2.3");
    }

    #[test]
    fn test_user_agent_with_commit() {
        let info = BuildInfo {
            version: "1.2.3".to_string(),
            commit: "abc1234".to_string(),
            commit_full: "abc1234567890".to_string(),
            build_time: "2026-03-29T12:00:00Z".to_string(),
            rustc_version: "1.78.0".to_string(),
            service: "doorway".to_string(),
        };
        assert_eq!(info.user_agent(), "doorway/1.2.3+abc1234");
    }

    #[test]
    fn test_serializes_camel_case() {
        let info = BuildInfo::new("test-svc");
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"commitFull\""));
        assert!(json.contains("\"buildTime\""));
        assert!(json.contains("\"rustcVersion\""));
    }

    #[test]
    fn test_roundtrip() {
        let info = BuildInfo::new("storage");
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: BuildInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.version, info.version);
        assert_eq!(deserialized.service, "storage");
    }
}
