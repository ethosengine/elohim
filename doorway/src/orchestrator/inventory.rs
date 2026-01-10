//! Node inventory collection
//!
//! Collects capacity information from nodes:
//! - CPU cores
//! - Memory (GB)
//! - Storage (TB)
//! - Bandwidth (Mbps)
//! - Custodian opt-in settings

use serde::{Deserialize, Serialize};

/// Node capacity information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeCapacity {
    /// Available CPU cores
    pub cpu_cores: u32,
    /// Memory in GB
    pub memory_gb: u32,
    /// Storage in TB
    pub storage_tb: f64,
    /// Network bandwidth in Mbps
    pub bandwidth_mbps: u32,
}

/// Complete node inventory
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInventory {
    /// Node identifier
    pub node_id: String,
    /// Agent public key (base64)
    pub agent_pub_key: String,
    /// Hardware capacity
    pub capacity: NodeCapacity,
    /// Geographic region
    pub region: String,
    /// Steward tier: caretaker, guardian, steward, pioneer
    pub steward_tier: String,
    /// Whether node opts in to custodian responsibilities
    pub custodian_opt_in: bool,
    /// Maximum storage to dedicate to custody (GB)
    pub max_custody_gb: Option<f64>,
    /// Maximum bandwidth to dedicate (Mbps)
    pub max_bandwidth_mbps: Option<u32>,
    /// Maximum CPU percentage to dedicate
    pub max_cpu_percent: Option<f64>,
    /// mDNS address where node was discovered
    pub discovered_address: Option<String>,
}

impl NodeInventory {
    /// Create inventory for a new node with default settings
    pub fn new(
        node_id: String,
        agent_pub_key: String,
        capacity: NodeCapacity,
        region: String,
    ) -> Self {
        Self {
            node_id,
            agent_pub_key,
            capacity,
            region,
            steward_tier: "caretaker".to_string(),
            custodian_opt_in: true,
            max_custody_gb: None,
            max_bandwidth_mbps: None,
            max_cpu_percent: None,
            discovered_address: None,
        }
    }

    /// Set custodian limits
    pub fn with_custodian_limits(
        mut self,
        max_custody_gb: f64,
        max_bandwidth_mbps: u32,
        max_cpu_percent: f64,
    ) -> Self {
        self.max_custody_gb = Some(max_custody_gb);
        self.max_bandwidth_mbps = Some(max_bandwidth_mbps);
        self.max_cpu_percent = Some(max_cpu_percent);
        self
    }

    /// Set steward tier
    pub fn with_steward_tier(mut self, tier: &str) -> Self {
        self.steward_tier = tier.to_string();
        self
    }

    /// Convert to NodeRegistration for DNA call
    pub fn to_registration_input(&self) -> NodeRegistrationInput {
        NodeRegistrationInput {
            node_id: self.node_id.clone(),
            agent_pub_key: self.agent_pub_key.clone(),
            region: self.region.clone(),
            steward_tier: self.steward_tier.clone(),
            cpu_cores: self.capacity.cpu_cores,
            memory_gb: self.capacity.memory_gb,
            storage_tb: self.capacity.storage_tb,
            bandwidth_mbps: self.capacity.bandwidth_mbps,
            custodian_opt_in: self.custodian_opt_in,
            max_custody_gb: self.max_custody_gb,
            max_bandwidth_mbps: self.max_bandwidth_mbps,
            max_cpu_percent: self.max_cpu_percent,
        }
    }
}

/// Input for node_registry DNA register_node() call
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeRegistrationInput {
    pub node_id: String,
    pub agent_pub_key: String,
    pub region: String,
    pub steward_tier: String,
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub storage_tb: f64,
    pub bandwidth_mbps: u32,
    pub custodian_opt_in: bool,
    pub max_custody_gb: Option<f64>,
    pub max_bandwidth_mbps: Option<u32>,
    pub max_cpu_percent: Option<f64>,
}

/// Input for node_registry DNA heartbeat() call
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatInput {
    pub node_id: String,
    pub timestamp: String,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub storage_usage_tb: f64,
    pub active_connections: u32,
    pub custodied_content_gb: f64,
}

/// Parse inventory from mDNS TXT records or JSON payload
pub fn parse_inventory_from_json(json_str: &str) -> Result<NodeInventory, serde_json::Error> {
    serde_json::from_str(json_str)
}

/// Inventory announcement message (sent by nodes via mDNS/NATS)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryAnnouncement {
    /// Message type: "announce" | "update" | "deregister"
    pub msg_type: String,
    /// Node inventory
    pub inventory: NodeInventory,
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    /// Signature over message (optional)
    pub signature: Option<String>,
}

impl InventoryAnnouncement {
    /// Create new announcement
    pub fn new_announce(inventory: NodeInventory) -> Self {
        Self {
            msg_type: "announce".to_string(),
            inventory,
            timestamp: chrono::Utc::now().to_rfc3339(),
            signature: None,
        }
    }

    /// Create update announcement
    pub fn new_update(inventory: NodeInventory) -> Self {
        Self {
            msg_type: "update".to_string(),
            inventory,
            timestamp: chrono::Utc::now().to_rfc3339(),
            signature: None,
        }
    }

    /// Create deregister announcement
    pub fn new_deregister(inventory: NodeInventory) -> Self {
        Self {
            msg_type: "deregister".to_string(),
            inventory,
            timestamp: chrono::Utc::now().to_rfc3339(),
            signature: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inventory_creation() {
        let capacity = NodeCapacity {
            cpu_cores: 8,
            memory_gb: 32,
            storage_tb: 2.0,
            bandwidth_mbps: 1000,
        };

        let inventory = NodeInventory::new(
            "node-001".to_string(),
            "uhCAk...".to_string(),
            capacity,
            "us-west-2".to_string(),
        )
        .with_custodian_limits(500.0, 500, 50.0)
        .with_steward_tier("guardian");

        assert_eq!(inventory.node_id, "node-001");
        assert_eq!(inventory.steward_tier, "guardian");
        assert_eq!(inventory.max_custody_gb, Some(500.0));
    }

    #[test]
    fn test_inventory_serialization() {
        let capacity = NodeCapacity {
            cpu_cores: 4,
            memory_gb: 16,
            storage_tb: 1.0,
            bandwidth_mbps: 500,
        };

        let inventory = NodeInventory::new(
            "node-002".to_string(),
            "uhCAk...".to_string(),
            capacity,
            "eu-west-1".to_string(),
        );

        let json = serde_json::to_string(&inventory).unwrap();
        let parsed: NodeInventory = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.node_id, inventory.node_id);
        assert_eq!(parsed.capacity.cpu_cores, 4);
    }
}
