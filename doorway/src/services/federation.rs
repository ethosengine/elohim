//! Federation Service
//!
//! Core federation engine for doorway-to-doorway cooperation:
//! - DHT registration via infrastructure DNA
//! - Periodic heartbeat reporting
//! - Cross-doorway content fetching via DID resolution
//!
//! This is the capstone of the 5-stage agency model, enabling community
//! doorway stewards to register, publish, and participate in the network.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::config::Args;
use crate::server::AppState;
use crate::services::did_resolver::DIDResolver;
use crate::services::zome_caller::ZomeCaller;

// =============================================================================
// Configuration
// =============================================================================

/// Federation configuration — derived from CLI args
#[derive(Debug, Clone)]
pub struct FederationConfig {
    /// Whether federation is enabled (requires doorway_id + doorway_url)
    pub enabled: bool,
    /// Unique doorway identifier (e.g., "alpha-elohim-host")
    pub doorway_id: String,
    /// Public URL of this doorway (e.g., "https://alpha.elohim.host")
    pub doorway_url: String,
    /// Geographic region for routing
    pub region: Option<String>,
    /// Heartbeat interval in seconds (default: 60)
    pub heartbeat_interval_secs: u64,
    /// Role name in the hApp for infrastructure DNA
    pub infrastructure_role: String,
    /// Zome name within infrastructure DNA
    pub zome_name: String,
}

impl FederationConfig {
    /// Build federation config from CLI args.
    /// Returns None if doorway_id or doorway_url not configured (federation disabled).
    pub fn from_args(args: &Args) -> Option<Self> {
        let doorway_id = args.doorway_id.as_ref()?;
        let doorway_url = args.doorway_url.as_ref()?;

        Some(Self {
            enabled: true,
            doorway_id: doorway_id.clone(),
            doorway_url: doorway_url.clone(),
            region: args.region.clone(),
            heartbeat_interval_secs: 60,
            infrastructure_role: "infrastructure".to_string(),
            zome_name: "infrastructure".to_string(),
        })
    }
}

// =============================================================================
// Zome Input/Output Types (match infrastructure coordinator zome exactly)
// =============================================================================

/// Input for registering a doorway (matches infrastructure zome RegisterDoorwayInput)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterDoorwayInput {
    pub id: String,
    pub url: String,
    pub capabilities_json: String,
    pub reach: String,
    pub region: Option<String>,
    pub bandwidth_mbps: Option<u32>,
    pub version: String,
}

/// Input for recording a heartbeat (matches infrastructure zome RecordHeartbeatInput)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordHeartbeatInput {
    pub doorway_id: String,
    pub status: String,
    pub uptime_ratio: f32,
    pub active_connections: u32,
    pub content_served: u64,
}

/// Input for finding content publishers (matches infrastructure zome FindPublishersInput)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindPublishersInput {
    pub content_hash: String,
    pub capability: Option<String>,
    pub prefer_region: Option<String>,
    pub limit: Option<usize>,
    pub online_only: Option<bool>,
}

/// Output from doorway registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorwayOutput {
    pub action_hash: Vec<u8>,
    pub doorway: DoorwayInfo,
}

/// Doorway info (matches DoorwayRegistration from infrastructure integrity zome)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorwayInfo {
    pub id: String,
    pub url: String,
    pub operator_agent: String,
    pub operator_human: Option<String>,
    pub capabilities_json: String,
    pub reach: String,
    pub region: Option<String>,
    pub bandwidth_mbps: Option<u32>,
    pub version: String,
    pub tier: String,
    pub registered_at: String,
    pub updated_at: String,
}

/// Federation error types
#[derive(Debug)]
pub enum FederationError {
    /// Zome call failed
    ZomeCallFailed(String),
    /// DID resolution failed
    DIDResolutionFailed(String),
    /// No publishers found for content
    NoPublishers(String),
    /// Remote doorway fetch failed
    RemoteFetchFailed(String),
}

impl std::fmt::Display for FederationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FederationError::ZomeCallFailed(e) => write!(f, "Zome call failed: {}", e),
            FederationError::DIDResolutionFailed(e) => write!(f, "DID resolution failed: {}", e),
            FederationError::NoPublishers(h) => write!(f, "No publishers for content: {}", h),
            FederationError::RemoteFetchFailed(e) => write!(f, "Remote fetch failed: {}", e),
        }
    }
}

// =============================================================================
// Registration
// =============================================================================

/// Register this doorway in the infrastructure DNA's DHT.
///
/// Called at startup. On "already exists" error, falls back to update_doorway
/// for idempotent restarts.
pub async fn register_doorway_in_dht(
    config: &FederationConfig,
    zome_caller: &ZomeCaller,
    capabilities: Vec<String>,
) -> Result<(), String> {
    let caps_json = serde_json::to_string(&capabilities)
        .map_err(|e| format!("Failed to serialize capabilities: {}", e))?;

    let input = RegisterDoorwayInput {
        id: config.doorway_id.clone(),
        url: config.doorway_url.clone(),
        capabilities_json: caps_json,
        reach: "public".to_string(),
        region: config.region.clone(),
        bandwidth_mbps: None,
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    info!(
        doorway_id = %config.doorway_id,
        doorway_url = %config.doorway_url,
        "Registering doorway in infrastructure DHT"
    );

    match zome_caller.call::<RegisterDoorwayInput, DoorwayOutput>(
        &config.infrastructure_role,
        &config.zome_name,
        "register_doorway",
        &input,
    ).await {
        Ok(_output) => {
            info!(
                doorway_id = %config.doorway_id,
                "Doorway registered in DHT successfully"
            );
            Ok(())
        }
        Err(e) if e.contains("already exists") => {
            // Doorway already registered — update instead (idempotent startup)
            info!(
                doorway_id = %config.doorway_id,
                "Doorway already registered, updating..."
            );
            match zome_caller.call::<RegisterDoorwayInput, DoorwayOutput>(
                &config.infrastructure_role,
                &config.zome_name,
                "update_doorway",
                &input,
            ).await {
                Ok(_) => {
                    info!("Doorway registration updated successfully");
                    Ok(())
                }
                Err(e) => {
                    warn!("Failed to update doorway registration: {}", e);
                    Err(e)
                }
            }
        }
        Err(e) => {
            warn!("Failed to register doorway in DHT: {}", e);
            Err(e)
        }
    }
}

// =============================================================================
// Heartbeat
// =============================================================================

/// Spawn periodic heartbeat task (every heartbeat_interval_secs).
///
/// Gathers live metrics from AppState and reports to infrastructure DHT.
/// Logs warnings on failure but does not crash.
pub fn spawn_heartbeat_task(
    config: FederationConfig,
    zome_caller: Arc<ZomeCaller>,
    state: Arc<AppState>,
) -> JoinHandle<()> {
    let interval = std::time::Duration::from_secs(config.heartbeat_interval_secs);

    tokio::spawn(async move {
        info!(
            doorway_id = %config.doorway_id,
            interval_secs = config.heartbeat_interval_secs,
            "Federation heartbeat task started"
        );

        let mut content_served_total: u64 = 0;

        loop {
            tokio::time::sleep(interval).await;

            // Gather live metrics from AppState
            let active_connections = state.pool.as_ref()
                .map(|p| p.connected_count() as u32)
                .unwrap_or(0);

            // Increment a rough content served counter based on cache stats
            let cache_hits = state.cache.stats().hits;
            content_served_total = content_served_total.wrapping_add(cache_hits as u64);

            let input = RecordHeartbeatInput {
                doorway_id: config.doorway_id.clone(),
                status: "online".to_string(),
                uptime_ratio: 1.0, // Running = online
                active_connections,
                content_served: content_served_total,
            };

            debug!(
                doorway_id = %config.doorway_id,
                active_connections = active_connections,
                "Recording heartbeat"
            );

            match zome_caller.call::<RecordHeartbeatInput, Vec<u8>>(
                &config.infrastructure_role,
                &config.zome_name,
                "record_heartbeat",
                &input,
            ).await {
                Ok(_) => {
                    debug!("Heartbeat recorded successfully");
                }
                Err(e) => {
                    warn!(
                        doorway_id = %config.doorway_id,
                        error = %e,
                        "Failed to record heartbeat (will retry next interval)"
                    );
                }
            }
        }
    })
}

// =============================================================================
// Cross-Doorway Content Fetch
// =============================================================================

/// Fetch content from a remote doorway via DHT publisher discovery + DID resolution.
///
/// Called by DeliveryRelay as final fallback tier when local storage returns 404.
///
/// Flow:
/// 1. Query infrastructure DHT for publishers of this content hash
/// 2. For each publisher, extract doorway URL from ContentServer endpoints
/// 3. Resolve doorway's DID document via DIDResolver
/// 4. Extract ElohimBlobStore service endpoint
/// 5. Fetch blob via HTTP GET
/// 6. Add X-Federation-Hop header to prevent infinite loops
pub async fn fetch_from_remote_doorway(
    content_hash: &str,
    zome_caller: &ZomeCaller,
    did_resolver: &DIDResolver,
    config: &FederationConfig,
) -> Result<Vec<u8>, FederationError> {
    debug!(
        content_hash = %content_hash,
        "Searching for remote publishers via infrastructure DHT"
    );

    // Step 1: Find publishers in DHT
    let input = FindPublishersInput {
        content_hash: content_hash.to_string(),
        capability: Some("blob".to_string()),
        prefer_region: config.region.clone(),
        limit: Some(5),
        online_only: Some(true),
    };

    let publishers_result = zome_caller.call_zome(
        &config.infrastructure_role,
        &config.zome_name,
        "find_publishers",
        rmp_serde::to_vec(&input).map_err(|e| FederationError::ZomeCallFailed(e.to_string()))?,
    ).await.map_err(|e| FederationError::ZomeCallFailed(e))?;

    // Parse publishers response
    #[derive(Deserialize)]
    struct FindPublishersOutput {
        #[allow(dead_code)]
        content_hash: String,
        publishers: Vec<PublisherInfo>,
    }
    #[derive(Deserialize)]
    struct PublisherInfo {
        #[allow(dead_code)]
        action_hash: Vec<u8>,
        server: ServerInfo,
    }
    #[derive(Deserialize)]
    struct ServerInfo {
        endpoints: Vec<EndpointInfo>,
        #[allow(dead_code)]
        online: bool,
        #[allow(dead_code)]
        priority: u8,
        #[allow(dead_code)]
        region: Option<String>,
    }
    #[derive(Deserialize)]
    struct EndpointInfo {
        url: String,
        #[allow(dead_code)]
        protocol: String,
    }

    let output: FindPublishersOutput = rmp_serde::from_slice(&publishers_result)
        .map_err(|e| FederationError::ZomeCallFailed(format!("Failed to parse publishers: {}", e)))?;

    if output.publishers.is_empty() {
        return Err(FederationError::NoPublishers(content_hash.to_string()));
    }

    // Step 2-5: Try each publisher
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    for publisher in &output.publishers {
        for endpoint in &publisher.server.endpoints {
            // Try direct endpoint fetch first
            let blob_url = format!("{}/{}", endpoint.url.trim_end_matches('/'), content_hash);

            debug!(
                blob_url = %blob_url,
                "Attempting federation fetch from remote publisher"
            );

            match http_client
                .get(&blob_url)
                .header("X-Federation-Hop", "1")
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    match resp.bytes().await {
                        Ok(bytes) => {
                            info!(
                                content_hash = %content_hash,
                                source = %endpoint.url,
                                size = bytes.len(),
                                "Federation fetch successful"
                            );
                            return Ok(bytes.to_vec());
                        }
                        Err(e) => {
                            warn!("Failed to read response body from {}: {}", blob_url, e);
                            continue;
                        }
                    }
                }
                Ok(resp) => {
                    debug!(
                        status = %resp.status(),
                        url = %blob_url,
                        "Remote publisher returned non-success"
                    );
                    continue;
                }
                Err(e) => {
                    debug!(
                        error = %e,
                        url = %blob_url,
                        "Failed to reach remote publisher"
                    );
                    continue;
                }
            }
        }

        // If direct endpoint failed, try DID resolution for the doorway URL
        // Extract domain from the first endpoint URL to construct a DID
        if let Some(endpoint) = publisher.server.endpoints.first() {
            if let Some(domain) = extract_domain_from_url(&endpoint.url) {
                let did = format!("did:web:{}", domain);
                match did_resolver.resolve(&did).await {
                    Ok(doc) => {
                        // Find ElohimBlobStore service endpoint
                        if let Some(blob_service) = doc.service.iter()
                            .find(|s| s.service_type == "ElohimBlobStore")
                        {
                            let blob_url = format!(
                                "{}/{}",
                                blob_service.service_endpoint.trim_end_matches('/'),
                                content_hash
                            );

                            match http_client
                                .get(&blob_url)
                                .header("X-Federation-Hop", "1")
                                .send()
                                .await
                            {
                                Ok(resp) if resp.status().is_success() => {
                                    if let Ok(bytes) = resp.bytes().await {
                                        info!(
                                            content_hash = %content_hash,
                                            source = %blob_url,
                                            "Federation fetch via DID resolution successful"
                                        );
                                        return Ok(bytes.to_vec());
                                    }
                                }
                                _ => continue,
                            }
                        }
                    }
                    Err(e) => {
                        debug!(did = %did, error = ?e, "DID resolution failed for remote publisher");
                    }
                }
            }
        }
    }

    Err(FederationError::RemoteFetchFailed(format!(
        "All {} publishers failed for content {}",
        output.publishers.len(),
        content_hash
    )))
}

/// Extract domain from a URL (e.g., "https://alpha.elohim.host/store" -> "alpha.elohim.host")
fn extract_domain_from_url(url: &str) -> Option<String> {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    let domain = without_scheme
        .split('/')
        .next()?
        .split(':')
        .next()?;

    if domain.is_empty() {
        None
    } else {
        Some(domain.to_string())
    }
}

// =============================================================================
// Doorway List (for federation routes)
// =============================================================================

/// Get all registered doorways from the DHT.
/// Used by the /api/v1/federation/doorways endpoint.
pub async fn get_all_doorways(
    zome_caller: &ZomeCaller,
    config: &FederationConfig,
) -> Result<Vec<DoorwayInfo>, String> {
    // Use get_doorways_by_region with empty region to get all,
    // or use find_publishers with wildcard. Since the zome doesn't have
    // a "get_all_doorways" function, we'll query by the current doorway's operator
    // and the known regions. For now, return the self doorway as a starting point.
    // A full implementation would use a "list all" anchor pattern.

    // Try to get our own doorway registration as proof of concept
    match zome_caller.call::<String, Option<DoorwayOutput>>(
        &config.infrastructure_role,
        &config.zome_name,
        "get_doorway_by_id",
        &config.doorway_id,
    ).await {
        Ok(Some(output)) => Ok(vec![output.doorway]),
        Ok(None) => Ok(vec![]),
        Err(e) => {
            warn!("Failed to query doorways from DHT: {}", e);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_federation_config_from_args_none() {
        let args = Args::parse_from(["doorway"]);
        let config = FederationConfig::from_args(&args);
        assert!(config.is_none(), "Should be None when doorway_id/url not set");
    }

    #[test]
    fn test_federation_config_from_args_some() {
        let args = Args::parse_from([
            "doorway",
            "--doorway-id", "alpha-elohim-host",
            "--doorway-url", "https://alpha.elohim.host",
            "--region", "us-west",
        ]);
        let config = FederationConfig::from_args(&args).unwrap();
        assert_eq!(config.doorway_id, "alpha-elohim-host");
        assert_eq!(config.doorway_url, "https://alpha.elohim.host");
        assert_eq!(config.region, Some("us-west".to_string()));
        assert_eq!(config.heartbeat_interval_secs, 60);
    }

    #[test]
    fn test_extract_domain_from_url() {
        assert_eq!(
            extract_domain_from_url("https://alpha.elohim.host/store"),
            Some("alpha.elohim.host".to_string())
        );
        assert_eq!(
            extract_domain_from_url("http://localhost:8080/api"),
            Some("localhost".to_string())
        );
        assert_eq!(
            extract_domain_from_url(""),
            None
        );
    }

    #[test]
    fn test_register_doorway_input_serialization() {
        let input = RegisterDoorwayInput {
            id: "test-doorway".to_string(),
            url: "https://test.elohim.host".to_string(),
            capabilities_json: r#"["gateway","bootstrap"]"#.to_string(),
            reach: "public".to_string(),
            region: Some("us-west".to_string()),
            bandwidth_mbps: None,
            version: "0.1.0".to_string(),
        };

        let bytes = rmp_serde::to_vec(&input).unwrap();
        let decoded: RegisterDoorwayInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "test-doorway");
    }
}
