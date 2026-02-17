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

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// ---------- Doorway Bootstrap Client ----------

/// Peer info returned from bootstrap discovery.
#[derive(Debug, Clone)]
pub struct BootstrapPeerInfo {
    pub agent: Vec<u8>,
    pub urls: Vec<String>,
    pub signed_at_ms: u64,
}

/// Put agent info into the doorway bootstrap service.
///
/// Constructs a `SignedAgentInfo` envelope with Ed25519 signature and sends it
/// as MessagePack to `POST /bootstrap/put`.
///
/// Uses `rmpv` manual encoding to match doorway's expected binary format
/// (string-keyed maps, not struct serialization).
pub async fn bootstrap_put(
    doorway_url: &str,
    space: &[u8; 36],
    agent: &[u8; 36],
    urls: &[String],
    signing_key: &ed25519_dalek::SigningKey,
) -> Result<(), RegistrationError> {
    use ed25519_dalek::Signer;
    use rmpv::Value;

    let signed_at_ms = now_ms() as i64;
    let expires_after_ms: i64 = 20 * 60 * 1000; // 20 minutes

    // Build inner agent_info as MessagePack bytes
    let url_values: Vec<Value> = urls
        .iter()
        .map(|u| Value::String(u.clone().into()))
        .collect();

    let agent_info_map = Value::Map(vec![
        (Value::String("space".into()), Value::Binary(space.to_vec())),
        (Value::String("agent".into()), Value::Binary(agent.to_vec())),
        (Value::String("urls".into()), Value::Array(url_values)),
        (
            Value::String("signed_at_ms".into()),
            Value::Integer(signed_at_ms.into()),
        ),
        (
            Value::String("expires_after_ms".into()),
            Value::Integer(expires_after_ms.into()),
        ),
    ]);

    let mut agent_info_bytes = Vec::new();
    rmpv::encode::write_value(&mut agent_info_bytes, &agent_info_map)
        .map_err(|e| RegistrationError::Network(format!("msgpack encode agent_info: {}", e)))?;

    // Sign the agent_info bytes
    let signature = signing_key.sign(&agent_info_bytes);

    // Build outer SignedAgentInfo envelope
    let envelope = Value::Map(vec![
        (Value::String("agent".into()), Value::Binary(agent.to_vec())),
        (
            Value::String("signature".into()),
            Value::Binary(signature.to_bytes().to_vec()),
        ),
        (
            Value::String("agent_info".into()),
            Value::Binary(agent_info_bytes),
        ),
    ]);

    let mut body = Vec::new();
    rmpv::encode::write_value(&mut body, &envelope)
        .map_err(|e| RegistrationError::Network(format!("msgpack encode envelope: {}", e)))?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/bootstrap/put", doorway_url))
        .header("Content-Type", "application/msgpack")
        .body(body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| RegistrationError::Network(e.to_string()))?;

    if !response.status().is_success() {
        return Err(RegistrationError::Network(format!(
            "bootstrap_put failed: HTTP {}",
            response.status()
        )));
    }

    info!("Bootstrap put successful");
    Ok(())
}

/// Discover random peers from the doorway bootstrap service.
///
/// Sends `POST /bootstrap/random` with a MessagePack query and parses the
/// response array of `SignedAgentInfo` envelopes.
pub async fn bootstrap_random(
    doorway_url: &str,
    space: &[u8; 36],
    limit: u32,
) -> Result<Vec<BootstrapPeerInfo>, RegistrationError> {
    use rmpv::Value;

    // Build request body
    let request = Value::Map(vec![
        (Value::String("space".into()), Value::Binary(space.to_vec())),
        (
            Value::String("limit".into()),
            Value::Integer((limit as i64).into()),
        ),
    ]);

    let mut body = Vec::new();
    rmpv::encode::write_value(&mut body, &request)
        .map_err(|e| RegistrationError::Network(format!("msgpack encode: {}", e)))?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/bootstrap/random", doorway_url))
        .header("Content-Type", "application/msgpack")
        .body(body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| RegistrationError::Network(e.to_string()))?;

    if !response.status().is_success() {
        return Err(RegistrationError::Network(format!(
            "bootstrap_random failed: HTTP {}",
            response.status()
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| RegistrationError::Network(e.to_string()))?;

    // Parse response as MessagePack array of SignedAgentInfo
    let value = rmpv::decode::read_value(&mut bytes.as_ref())
        .map_err(|e| RegistrationError::InvalidResponse(format!("msgpack decode: {}", e)))?;

    let peers = match value {
        Value::Array(items) => {
            let mut peers = Vec::new();
            for item in items {
                if let Some(peer) = parse_signed_agent_info(&item) {
                    peers.push(peer);
                }
            }
            peers
        }
        _ => {
            warn!("bootstrap_random: unexpected response format");
            vec![]
        }
    };

    info!(num_peers = peers.len(), "Bootstrap random discovery complete");
    Ok(peers)
}

/// Get the current server timestamp from doorway bootstrap.
pub async fn bootstrap_now(
    doorway_url: &str,
) -> Result<u64, RegistrationError> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/bootstrap/now", doorway_url))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| RegistrationError::Network(e.to_string()))?;

    if !response.status().is_success() {
        return Err(RegistrationError::Network(format!(
            "bootstrap_now failed: HTTP {}",
            response.status()
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| RegistrationError::Network(e.to_string()))?;

    let value = rmpv::decode::read_value(&mut bytes.as_ref())
        .map_err(|e| RegistrationError::InvalidResponse(format!("msgpack decode: {}", e)))?;

    match value {
        rmpv::Value::Integer(n) => n
            .as_u64()
            .ok_or_else(|| RegistrationError::InvalidResponse("timestamp not u64".into())),
        _ => Err(RegistrationError::InvalidResponse(
            "expected integer timestamp".into(),
        )),
    }
}

/// Parse a SignedAgentInfo MessagePack value into BootstrapPeerInfo.
fn parse_signed_agent_info(value: &rmpv::Value) -> Option<BootstrapPeerInfo> {
    let map = value.as_map()?;

    let agent_info_bytes = map.iter().find_map(|(k, v)| {
        if k.as_str()? == "agent_info" {
            v.as_slice()
        } else {
            None
        }
    })?;

    // Parse inner agent_info
    let inner = rmpv::decode::read_value(&mut &agent_info_bytes[..]).ok()?;
    let inner_map = inner.as_map()?;

    let agent = inner_map.iter().find_map(|(k, v)| {
        if k.as_str()? == "agent" {
            Some(v.as_slice()?.to_vec())
        } else {
            None
        }
    })?;

    let urls = inner_map.iter().find_map(|(k, v)| {
        if k.as_str()? == "urls" {
            let arr = v.as_array()?;
            Some(
                arr.iter()
                    .filter_map(|u| u.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        }
    })?;

    let signed_at_ms = inner_map.iter().find_map(|(k, v)| {
        if k.as_str()? == "signed_at_ms" {
            v.as_i64().map(|n| n as u64)
        } else {
            None
        }
    })?;

    Some(BootstrapPeerInfo {
        agent,
        urls,
        signed_at_ms,
    })
}
