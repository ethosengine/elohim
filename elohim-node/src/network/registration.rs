//! Node registration with doorways
//!
//! Handles the registration lifecycle:
//! 1. Parse join key to get operator + cluster info
//! 2. Connect to doorway
//! 3. Register node with doorway
//! 4. Receive cluster membership
//! 5. Maintain heartbeat

use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};

use super::{RegisteredDoorway, DoorwayStatus, ClusterInfo, ClusterRole};
use crate::update::CURRENT_VERSION;

/// Registration status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RegistrationStatus {
    /// Not yet registered with any network
    Unregistered,
    /// Registration in progress
    Registering,
    /// Registered but not yet synced
    Registered,
    /// Registered and actively syncing
    Active,
    /// Registration failed
    Failed { error: String },
    /// Suspended by operator
    Suspended { reason: String },
}

/// Registration request sent to doorway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationRequest {
    /// Node's unique ID
    pub node_id: String,
    /// Hostname
    pub hostname: String,
    /// Node version
    pub version: String,
    /// Join key (encrypted operator + cluster info)
    pub join_key: String,
    /// Node's public key for verification
    pub pub_key: String,
    /// Node capabilities
    pub capabilities: Vec<String>,
    /// Node hardware info
    pub hardware: HardwareInfo,
}

/// Hardware information for capacity planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cpu_cores: usize,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub arch: String,
    pub os: String,
}

/// Registration response from doorway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationResponse {
    /// Whether registration succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Cluster information
    pub cluster: Option<ClusterInfo>,
    /// Operator public key for verification
    pub operator_pub_key: Option<String>,
    /// Other doorways in the network
    pub doorways: Vec<DoorwayInfo>,
    /// Initial sync cursor
    pub sync_cursor: Option<String>,
}

/// Information about a doorway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorwayInfo {
    pub url: String,
    pub pub_key: String,
    pub region: Option<String>,
    pub capabilities: Vec<String>,
}

/// Network registration service
pub struct NetworkRegistration {
    node_id: String,
    hostname: String,
    status: RegistrationStatus,
    doorways: Vec<RegisteredDoorway>,
}

impl NetworkRegistration {
    pub fn new(node_id: String, hostname: String) -> Self {
        Self {
            node_id,
            hostname,
            status: RegistrationStatus::Unregistered,
            doorways: Vec::new(),
        }
    }

    /// Register with a doorway using a join key
    pub async fn register(
        &mut self,
        doorway_url: &str,
        join_key: &str,
    ) -> Result<RegistrationResponse, RegistrationError> {
        info!("Registering with doorway {}", doorway_url);
        self.status = RegistrationStatus::Registering;

        // Build registration request
        let request = RegistrationRequest {
            node_id: self.node_id.clone(),
            hostname: self.hostname.clone(),
            version: CURRENT_VERSION.to_string(),
            join_key: join_key.to_string(),
            pub_key: self.generate_node_key()?,
            capabilities: self.get_capabilities(),
            hardware: self.collect_hardware_info(),
        };

        // Send registration to doorway
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/nodes/register", doorway_url))
            .json(&request)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| RegistrationError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = format!("Registration failed: HTTP {}", response.status());
            self.status = RegistrationStatus::Failed { error: error.clone() };
            return Err(RegistrationError::Rejected(error));
        }

        let reg_response: RegistrationResponse = response
            .json()
            .await
            .map_err(|e| RegistrationError::InvalidResponse(e.to_string()))?;

        if !reg_response.success {
            let error = reg_response.error.unwrap_or_else(|| "Unknown error".to_string());
            self.status = RegistrationStatus::Failed { error: error.clone() };
            return Err(RegistrationError::Rejected(error));
        }

        // Add the primary doorway
        self.doorways.push(RegisteredDoorway {
            url: doorway_url.to_string(),
            pub_key: reg_response.operator_pub_key.clone(),
            is_primary: true,
            status: DoorwayStatus::Connected,
            last_contact: now(),
            capabilities: vec!["sync".to_string(), "updates".to_string()],
        });

        // Add secondary doorways
        for dw in &reg_response.doorways {
            if dw.url != doorway_url {
                self.doorways.push(RegisteredDoorway {
                    url: dw.url.clone(),
                    pub_key: Some(dw.pub_key.clone()),
                    is_primary: false,
                    status: DoorwayStatus::Disconnected,
                    last_contact: 0,
                    capabilities: dw.capabilities.clone(),
                });
            }
        }

        self.status = RegistrationStatus::Registered;
        info!("Registration successful");

        Ok(reg_response)
    }

    /// Send heartbeat to primary doorway
    pub async fn heartbeat(&mut self) -> Result<(), RegistrationError> {
        let primary = self.doorways
            .iter_mut()
            .find(|d| d.is_primary)
            .ok_or(RegistrationError::NoDoorway)?;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/nodes/{}/heartbeat", primary.url, self.node_id))
            .json(&HeartbeatRequest {
                node_id: self.node_id.clone(),
                version: CURRENT_VERSION.to_string(),
                status: "active".to_string(),
                sync_position: None, // TODO: Get from sync engine
            })
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await;

        match response {
            Ok(r) if r.status().is_success() => {
                primary.status = DoorwayStatus::Connected;
                primary.last_contact = now();
                Ok(())
            }
            Ok(r) => {
                warn!("Heartbeat failed: HTTP {}", r.status());
                primary.status = DoorwayStatus::Degraded;
                Err(RegistrationError::HeartbeatFailed)
            }
            Err(e) => {
                error!("Heartbeat error: {}", e);
                primary.status = DoorwayStatus::Disconnected;
                Err(RegistrationError::Network(e.to_string()))
            }
        }
    }

    /// Unregister from doorway
    pub async fn unregister(&mut self) -> Result<(), RegistrationError> {
        if let Some(primary) = self.doorways.iter().find(|d| d.is_primary) {
            let client = reqwest::Client::new();
            let _ = client
                .delete(format!("{}/api/nodes/{}", primary.url, self.node_id))
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await;
        }

        self.doorways.clear();
        self.status = RegistrationStatus::Unregistered;
        Ok(())
    }

    fn generate_node_key(&self) -> Result<String, RegistrationError> {
        // TODO: Generate Ed25519 keypair
        Ok(format!("node-pub-key-{}", self.node_id))
    }

    fn get_capabilities(&self) -> Vec<String> {
        vec![
            "sync".to_string(),
            "storage".to_string(),
            "relay".to_string(),
        ]
    }

    fn collect_hardware_info(&self) -> HardwareInfo {
        HardwareInfo {
            cpu_cores: num_cpus::get(),
            memory_bytes: 0, // TODO: Get from sysinfo
            storage_bytes: 0, // TODO: Get from sysinfo
            arch: std::env::consts::ARCH.to_string(),
            os: std::env::consts::OS.to_string(),
        }
    }
}

/// Heartbeat request
#[derive(Debug, Serialize)]
struct HeartbeatRequest {
    node_id: String,
    version: String,
    status: String,
    sync_position: Option<String>,
}

/// Registration errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum RegistrationError {
    #[error("No doorway configured")]
    NoDoorway,

    #[error("Network error: {0}")]
    Network(String),

    #[error("Registration rejected: {0}")]
    Rejected(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Heartbeat failed")]
    HeartbeatFailed,

    #[error("Key generation failed")]
    KeyGeneration,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
