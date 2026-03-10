//! Setup wizard - Configure node to join network or become doorway
//!
//! Two setup modes:
//! 1. Join existing network: Provide join key + doorway URL
//! 2. Become doorway: Configure hostname, DNS, ddclient

use serde::{Deserialize, Serialize};

/// Setup mode selection
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum SetupMode {
    /// Join an existing Elohim network
    JoinNetwork(JoinNetworkConfig),
    /// Become a doorway (bootstrap) node
    BecomeDoorway(DoorwayConfig),
}

/// Configuration for joining an existing network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinNetworkConfig {
    /// Join key from operator (contains agent key + cluster info)
    pub join_key: String,
    /// Doorway URL for bootstrapping
    pub doorway_url: String,
    /// Optional: Override cluster name
    pub cluster_name: Option<String>,
}

/// Configuration for becoming a doorway node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorwayConfig {
    /// Public hostname (e.g., doorway.myfamily.net)
    pub hostname: String,
    /// DNS provider for dynamic DNS
    pub dns_provider: DnsProvider,
    /// Enable HTTPS with Let's Encrypt
    pub enable_https: bool,
    /// Admin email for Let's Encrypt
    pub admin_email: Option<String>,
    /// Operator agent key (will be generated if not provided)
    pub operator_key: Option<String>,
}

/// Supported DNS providers for dynamic DNS
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider")]
pub enum DnsProvider {
    /// No dynamic DNS (static IP or manual)
    None,
    /// Cloudflare DNS
    Cloudflare { api_token: String, zone_id: String },
    /// DuckDNS
    DuckDns { token: String, domain: String },
    /// No-IP
    NoIp {
        username: String,
        password: String,
        hostname: String,
    },
    /// Generic ddclient config
    Ddclient { config: String },
}

/// Setup wizard state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SetupState {
    /// Fresh node, no configuration
    Fresh,
    /// Mode selected, awaiting configuration
    ModeSelected { mode: String },
    /// Configuration provided, validating
    Validating,
    /// Connecting to network
    Connecting,
    /// Setup complete
    Complete,
    /// Setup failed
    Failed { error: String },
}

/// Result of setup attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupResult {
    pub success: bool,
    pub state: SetupState,
    pub message: String,
    pub details: Option<SetupDetails>,
}

/// Details after successful setup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupDetails {
    pub node_id: String,
    pub cluster_name: String,
    pub cluster_role: String,
    pub connected_peers: usize,
    pub doorway_url: Option<String>,
}

/// Validate and execute join network setup
pub async fn setup_join_network(config: JoinNetworkConfig) -> SetupResult {
    tracing::info!("Setting up node to join network via {}", config.doorway_url);

    // 1. Validate join key format
    if let Err(e) = validate_join_key(&config.join_key) {
        return SetupResult {
            success: false,
            state: SetupState::Failed {
                error: format!("Invalid join key: {}", e),
            },
            message: "Invalid join key format".to_string(),
            details: None,
        };
    }

    // 2. Validate doorway URL
    if let Err(e) = validate_doorway_url(&config.doorway_url).await {
        return SetupResult {
            success: false,
            state: SetupState::Failed {
                error: format!("Cannot reach doorway: {}", e),
            },
            message: "Cannot connect to doorway".to_string(),
            details: None,
        };
    }

    // 3. Decode join key to get operator info and cluster key
    let (_operator_key, _cluster_key, cluster_name) = match decode_join_key(&config.join_key) {
        Ok(info) => info,
        Err(e) => {
            return SetupResult {
                success: false,
                state: SetupState::Failed {
                    error: format!("Failed to decode join key: {}", e),
                },
                message: "Invalid join key".to_string(),
                details: None,
            };
        }
    };

    // 4. Check for updates before syncing
    tracing::info!("Checking for updates before sync...");
    let update_result = check_and_apply_updates(&config.doorway_url).await;
    if let Err(e) = &update_result {
        tracing::warn!("Update check failed (continuing anyway): {}", e);
    }
    let update_applied = update_result.unwrap_or(false);

    // 5. Connect to doorway and register
    // TODO: Implement actual connection

    // 6. Join cluster
    // TODO: Implement cluster join

    let mut message = "Successfully joined network".to_string();
    if update_applied {
        message = "Successfully joined network. Update applied - restart required.".to_string();
    }

    SetupResult {
        success: true,
        state: SetupState::Complete,
        message,
        details: Some(SetupDetails {
            node_id: "node-1".to_string(),
            cluster_name: config.cluster_name.unwrap_or(cluster_name),
            cluster_role: "replica".to_string(),
            connected_peers: 0,
            doorway_url: Some(config.doorway_url),
        }),
    }
}

/// Check for updates from doorway and apply if available
async fn check_and_apply_updates(doorway_url: &str) -> Result<bool, String> {
    use crate::update::{UpdateConfig, UpdateService, UpdateStatus};

    let config = UpdateConfig {
        enabled: true,
        auto_apply: true, // Auto-apply during setup
        ..Default::default()
    };

    let mut update_service = UpdateService::new(config);
    update_service.set_doorway(doorway_url.to_string());

    // Check for updates
    let status = update_service
        .check_for_updates()
        .await
        .map_err(|e| e.to_string())?;

    match status {
        UpdateStatus::UpdateAvailable {
            current, latest, ..
        } => {
            tracing::info!("Update available: {} -> {}", current, latest);

            // Apply the update
            update_service
                .apply_update()
                .await
                .map_err(|e| e.to_string())?;

            Ok(true)
        }
        UpdateStatus::UpToDate => {
            tracing::info!("Already running latest version");
            Ok(false)
        }
        _ => Ok(false),
    }
}

/// Validate and execute doorway setup
pub async fn setup_doorway(config: DoorwayConfig) -> SetupResult {
    tracing::info!("Setting up node as doorway at {}", config.hostname);

    // 1. Validate hostname
    if config.hostname.is_empty() {
        return SetupResult {
            success: false,
            state: SetupState::Failed {
                error: "Hostname is required".to_string(),
            },
            message: "Hostname is required".to_string(),
            details: None,
        };
    }

    // 2. Configure dynamic DNS if needed
    if let Err(e) = configure_ddns(&config.dns_provider).await {
        return SetupResult {
            success: false,
            state: SetupState::Failed {
                error: format!("Failed to configure DNS: {}", e),
            },
            message: "DNS configuration failed".to_string(),
            details: None,
        };
    }

    // 3. Configure HTTPS if enabled
    if config.enable_https {
        if let Err(e) = configure_https(&config.hostname, config.admin_email.as_deref()).await {
            tracing::warn!("HTTPS setup failed: {}. Continuing without HTTPS.", e);
        }
    }

    // 4. Generate operator key if not provided
    let _operator_key = config.operator_key.unwrap_or_else(generate_operator_key);

    // 5. Start doorway services
    // TODO: Implement doorway startup

    SetupResult {
        success: true,
        state: SetupState::Complete,
        message: "Successfully configured as doorway".to_string(),
        details: Some(SetupDetails {
            node_id: "doorway-1".to_string(),
            cluster_name: "doorway-cluster".to_string(),
            cluster_role: "doorway".to_string(),
            connected_peers: 0,
            doorway_url: Some(format!("https://{}", config.hostname)),
        }),
    }
}

fn validate_join_key(key: &str) -> Result<(), String> {
    // Join key format: base64(operator_key:cluster_key:cluster_name)
    if key.len() < 20 {
        return Err("Key too short".to_string());
    }
    // TODO: Proper validation
    Ok(())
}

async fn validate_doorway_url(url: &str) -> Result<(), String> {
    // TODO: Actually check if doorway is reachable
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }
    Ok(())
}

fn decode_join_key(_key: &str) -> Result<(String, String, String), String> {
    // TODO: Implement proper decoding
    Ok((
        "operator-key".to_string(),
        "cluster-key".to_string(),
        "my-family".to_string(),
    ))
}

async fn configure_ddns(provider: &DnsProvider) -> Result<(), String> {
    match provider {
        DnsProvider::None => Ok(()),
        DnsProvider::Cloudflare {
            api_token: _,
            zone_id: _,
        } => {
            tracing::info!("Configuring Cloudflare DNS...");
            // TODO: Implement Cloudflare API calls
            Ok(())
        }
        DnsProvider::DuckDns { token: _, domain } => {
            tracing::info!("Configuring DuckDNS for {}...", domain);
            // TODO: Implement DuckDNS update
            Ok(())
        }
        DnsProvider::NoIp {
            username: _,
            hostname,
            ..
        } => {
            tracing::info!("Configuring No-IP for {}...", hostname);
            // TODO: Implement No-IP update
            Ok(())
        }
        DnsProvider::Ddclient { config: _ } => {
            tracing::info!("Writing ddclient configuration...");
            // TODO: Write ddclient config file
            Ok(())
        }
    }
}

async fn configure_https(hostname: &str, _email: Option<&str>) -> Result<(), String> {
    tracing::info!("Requesting Let's Encrypt certificate for {}...", hostname);
    // TODO: Implement ACME/Let's Encrypt
    Ok(())
}

fn generate_operator_key() -> String {
    // TODO: Generate Ed25519 keypair
    "generated-operator-key".to_string()
}
