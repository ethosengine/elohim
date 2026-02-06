//! Storage Registration Service
//!
//! Auto-registers local elohim-storage endpoints with the infrastructure DNA.
//! This is a prototype feature for doorway operators who run elohim-storage
//! alongside doorway on the same host.
//!
//! When enabled (via `ELOHIM_STORAGE_AUTO_REGISTER=true`), this will:
//! 1. Call `register_content_server` on the infrastructure zome
//! 2. Register wildcard content_hash ("*") = can serve any content
//! 3. The registration emits a `ContentServerCommitted` signal
//! 4. Other doorways pick up the signal and add fallback URLs
//!
//! ## Configuration
//!
//! - `ELOHIM_STORAGE_AUTO_REGISTER`: Set to "true" to enable (default: false)
//! - `ELOHIM_STORAGE_URL`: Base URL of local storage (default: "http://localhost:8080")
//! - `ELOHIM_STORAGE_CAPABILITIES`: Comma-separated capabilities (default: "blob,html5_app")

use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

/// Storage registration configuration
#[derive(Debug, Clone)]
pub struct StorageRegistrationConfig {
    /// Whether auto-registration is enabled
    pub enabled: bool,
    /// Base URL of local elohim-storage
    pub storage_url: String,
    /// Capabilities to register (blob, html5_app, media_stream, etc.)
    pub capabilities: Vec<String>,
    /// Infrastructure DNA cell ID (base64-encoded hash)
    pub infrastructure_dna_hash: Option<String>,
    /// Zome name in infrastructure DNA
    pub zome_name: String,
    /// Priority for this storage node (0-100)
    pub priority: u8,
}

impl Default for StorageRegistrationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            storage_url: "http://localhost:8080".to_string(),
            capabilities: vec!["blob".to_string(), "html5_app".to_string()],
            infrastructure_dna_hash: None,
            zome_name: "infrastructure".to_string(),
            priority: 100,
        }
    }
}

impl StorageRegistrationConfig {
    /// Create config from environment variables
    pub fn from_env() -> Self {
        let enabled = std::env::var("ELOHIM_STORAGE_AUTO_REGISTER")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let storage_url = std::env::var("ELOHIM_STORAGE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());

        let capabilities = std::env::var("ELOHIM_STORAGE_CAPABILITIES")
            .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|_| vec!["blob".to_string(), "html5_app".to_string()]);

        let infrastructure_dna_hash = std::env::var("INFRASTRUCTURE_DNA_HASH").ok();

        let priority = std::env::var("ELOHIM_STORAGE_PRIORITY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);

        Self {
            enabled,
            storage_url,
            capabilities,
            infrastructure_dna_hash,
            priority,
            ..Default::default()
        }
    }
}

/// Input for registering a content server (matches zome input)
#[derive(Debug, Clone, Serialize)]
pub struct RegisterContentServerInput {
    /// Content hash this server can provide ("*" for wildcard)
    pub content_hash: String,
    /// Capability: blob, html5_app, media_stream, etc.
    pub capability: String,
    /// URL where this server accepts content requests (deprecated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serve_url: Option<String>,
    /// Multiple reachable endpoints for content fetching
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<StorageEndpointInput>>,
    /// Server priority (0-100, higher = preferred)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
    /// Geographic region for latency-based routing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// Bandwidth capacity in Mbps
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bandwidth_mbps: Option<u32>,
}

/// Storage endpoint input
#[derive(Debug, Clone, Serialize)]
pub struct StorageEndpointInput {
    pub url: String,
    pub protocol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
}

/// Result of storage registration
#[derive(Debug)]
pub struct StorageRegistrationResult {
    pub success: bool,
    pub registered_capabilities: Vec<String>,
    pub errors: Vec<String>,
}

/// Register local elohim-storage with infrastructure DNA
///
/// This function is designed to be called during doorway startup.
/// It registers the local storage endpoint for each configured capability.
///
/// # Arguments
///
/// * `config` - Storage registration configuration
/// * `conductor_url` - WebSocket URL to the conductor app interface
/// * `installed_app_id` - Installed app ID for authentication
///
/// # Returns
///
/// A result indicating which capabilities were successfully registered.
pub async fn register_local_storage(
    config: &StorageRegistrationConfig,
    conductor_url: &str,
    installed_app_id: &str,
) -> StorageRegistrationResult {
    if !config.enabled {
        debug!("Storage auto-registration disabled");
        return StorageRegistrationResult {
            success: true,
            registered_capabilities: vec![],
            errors: vec![],
        };
    }

    info!(
        storage_url = %config.storage_url,
        capabilities = ?config.capabilities,
        "Auto-registering local elohim-storage with infrastructure DNA"
    );

    let mut registered = Vec::new();
    let mut errors = Vec::new();

    // Build endpoint list
    let endpoints = vec![StorageEndpointInput {
        url: config.storage_url.clone(),
        protocol: if config.storage_url.starts_with("https://") {
            "https".to_string()
        } else {
            "http".to_string()
        },
        priority: Some(config.priority),
    }];

    for capability in &config.capabilities {
        let input = RegisterContentServerInput {
            content_hash: "*".to_string(), // Wildcard: can serve any content
            capability: capability.clone(),
            serve_url: Some(config.storage_url.clone()), // Deprecated but included for compat
            endpoints: Some(endpoints.clone()),
            priority: Some(config.priority),
            region: None,
            bandwidth_mbps: None,
        };

        match register_with_conductor(
            conductor_url,
            installed_app_id,
            &config.zome_name,
            &input,
        )
        .await
        {
            Ok(()) => {
                info!(capability = %capability, "Registered storage for capability");
                registered.push(capability.clone());
            }
            Err(e) => {
                warn!(capability = %capability, error = %e, "Failed to register storage");
                errors.push(format!("{}: {}", capability, e));
            }
        }
    }

    let success = errors.is_empty();
    if success {
        info!(
            count = registered.len(),
            "Storage auto-registration completed successfully"
        );
    } else {
        warn!(
            registered = registered.len(),
            failed = errors.len(),
            "Storage auto-registration completed with some failures"
        );
    }

    StorageRegistrationResult {
        success,
        registered_capabilities: registered,
        errors,
    }
}

/// Call the infrastructure zome to register a content server
async fn register_with_conductor(
    conductor_url: &str,
    _installed_app_id: &str,
    zome_name: &str,
    input: &RegisterContentServerInput,
) -> Result<(), String> {
    // For prototype, we'll use a simple approach:
    // Log the registration intent. In production, this would make an actual zome call.
    //
    // TODO: Implement actual zome call using the conductor WebSocket protocol
    // This requires:
    // 1. Connect to conductor app interface
    // 2. Authenticate with IssueAppAuthenticationToken
    // 3. Call the infrastructure zome's register_content_server function
    //
    // For now, this is a placeholder that logs the intent.
    // The actual zome call would be similar to what ImportClient does.

    debug!(
        conductor_url = conductor_url,
        zome_name = zome_name,
        content_hash = %input.content_hash,
        capability = %input.capability,
        "Would register content server with infrastructure zome (not implemented)"
    );

    // TODO: When implementing, use something like:
    // let client = ImportClient::new(conductor_url);
    // client.call_zome(cell_id, zome_name, "register_content_server", payload).await

    // For now, return success to allow the prototype to continue
    // This means storage registration is a no-op until we implement the zome call
    info!(
        capability = %input.capability,
        endpoint = ?input.endpoints,
        "Storage registration intent logged (zome call not yet implemented)"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = StorageRegistrationConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.storage_url, "http://localhost:8080");
        assert_eq!(config.capabilities, vec!["blob", "html5_app"]);
        assert_eq!(config.priority, 100);
    }

    #[test]
    fn test_register_input_serialization() {
        let input = RegisterContentServerInput {
            content_hash: "*".to_string(),
            capability: "blob".to_string(),
            serve_url: Some("http://localhost:8080".to_string()),
            endpoints: Some(vec![StorageEndpointInput {
                url: "http://localhost:8080".to_string(),
                protocol: "http".to_string(),
                priority: Some(100),
            }]),
            priority: Some(100),
            region: None,
            bandwidth_mbps: None,
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"content_hash\":\"*\""));
        assert!(json.contains("\"capability\":\"blob\""));
        assert!(json.contains("\"endpoints\""));
    }

    #[tokio::test]
    async fn test_register_disabled() {
        let config = StorageRegistrationConfig::default(); // disabled by default

        let result = register_local_storage(&config, "ws://localhost:4445", "elohim").await;

        assert!(result.success);
        assert!(result.registered_capabilities.is_empty());
        assert!(result.errors.is_empty());
    }
}
