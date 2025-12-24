//! Orchestrator - Plug-n-play node management for Elohim clusters
//!
//! ## Overview
//!
//! The orchestrator enables k8s-like node management:
//! - Plug node into rack, turn it on, supply a key
//! - Orchestrator detects node via mDNS, collects inventory
//! - Registers node with Node Registry DNA
//! - Provisions NATS JWT credentials
//! - Starts heartbeat loop for liveness
//!
//! ## Flow
//!
//! ```text
//! New Node (Tauri) --mDNS--> Doorway --register_node()--> Node Registry DNA
//!                                    <--NATS JWT--
//!                           --heartbeat()-->
//! ```
//!
//! ## Modules
//!
//! - `node_bootstrap` - mDNS discovery and node registration
//! - `nats_provisioning` - JWT credential provisioning for NATS
//! - `heartbeat` - Liveness heartbeat loop
//! - `disaster_recovery` - Content replication coordination
//! - `inventory` - Node capacity collection

pub mod disaster_recovery;
pub mod heartbeat;
pub mod inventory;
pub mod nats_provisioning;
pub mod node_bootstrap;

pub use disaster_recovery::DisasterRecoveryCoordinator;
pub use heartbeat::{HeartbeatConfig, HeartbeatLoop};
pub use inventory::{NodeCapacity, NodeInventory};
pub use nats_provisioning::{NatsCredentials, NatsProvisioner};
pub use node_bootstrap::{NodeBootstrap, NodeBootstrapConfig};

// Note: NodeHealthStatus, NodeStatus, OrchestratorConfig, OrchestratorState, Orchestrator
// are defined below and are public by default in this module

use crate::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Central orchestrator state shared across components
pub struct OrchestratorState {
    /// Known nodes and their status
    nodes: RwLock<std::collections::HashMap<String, NodeStatus>>,
    /// NATS client for messaging
    nats_client: Option<Arc<crate::nats::NatsClient>>,
    /// Configuration
    config: OrchestratorConfig,
}

/// Node status tracked by orchestrator
#[derive(Debug, Clone)]
pub struct NodeStatus {
    pub node_id: String,
    pub status: NodeHealthStatus,
    pub last_heartbeat: Option<std::time::Instant>,
    pub inventory: Option<NodeInventory>,
    pub nats_provisioned: bool,
}

/// Health status of a node
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeHealthStatus {
    Discovered,
    Registering,
    Online,
    Degraded,
    Offline,
    Failed,
}

/// Orchestrator configuration
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// mDNS service type to listen for
    pub mdns_service_type: String,
    /// Holochain admin websocket port
    pub admin_port: u16,
    /// NATS server URL
    pub nats_url: String,
    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,
    /// Failure threshold (missed heartbeats before marking failed)
    pub failure_threshold: u32,
    /// Auto-assign custodian responsibilities
    pub auto_assign_custodians: bool,
    /// Region for this orchestrator
    pub region: String,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            mdns_service_type: "elohim-node".to_string(),
            admin_port: 8888,
            nats_url: "nats://localhost:4222".to_string(),
            heartbeat_interval_secs: 30,
            failure_threshold: 3,
            auto_assign_custodians: true,
            region: "default".to_string(),
        }
    }
}

impl OrchestratorState {
    /// Create new orchestrator state
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            nodes: RwLock::new(std::collections::HashMap::new()),
            nats_client: None,
            config,
        }
    }

    /// Set NATS client
    pub fn with_nats_client(mut self, client: Arc<crate::nats::NatsClient>) -> Self {
        self.nats_client = Some(client);
        self
    }

    /// Register a discovered node
    pub async fn register_node(&self, node_id: String, inventory: NodeInventory) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        let status = NodeStatus {
            node_id: node_id.clone(),
            status: NodeHealthStatus::Discovered,
            last_heartbeat: None,
            inventory: Some(inventory),
            nats_provisioned: false,
        };
        nodes.insert(node_id.clone(), status);
        info!(node_id = %node_id, "Node discovered and registered");
        Ok(())
    }

    /// Update node heartbeat
    pub async fn update_heartbeat(&self, node_id: &str) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.get_mut(node_id) {
            node.last_heartbeat = Some(std::time::Instant::now());
            if node.status == NodeHealthStatus::Offline {
                node.status = NodeHealthStatus::Online;
                info!(node_id = %node_id, "Node came back online");
            }
        }
        Ok(())
    }

    /// Mark node as provisioned
    pub async fn mark_provisioned(&self, node_id: &str) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.get_mut(node_id) {
            node.nats_provisioned = true;
            node.status = NodeHealthStatus::Online;
            info!(node_id = %node_id, "Node provisioned and online");
        }
        Ok(())
    }

    /// Get nodes that missed heartbeats
    pub async fn get_failed_nodes(&self) -> Vec<String> {
        let nodes = self.nodes.read().await;
        let threshold = std::time::Duration::from_secs(
            self.config.heartbeat_interval_secs * self.config.failure_threshold as u64,
        );

        nodes
            .iter()
            .filter(|(_, status)| {
                if let Some(last_hb) = status.last_heartbeat {
                    last_hb.elapsed() > threshold
                } else {
                    // No heartbeat ever received after registration
                    false
                }
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get all online nodes
    pub async fn get_online_nodes(&self) -> Vec<NodeStatus> {
        let nodes = self.nodes.read().await;
        nodes
            .values()
            .filter(|s| s.status == NodeHealthStatus::Online)
            .cloned()
            .collect()
    }

    /// Get node by ID
    pub async fn get_node(&self, node_id: &str) -> Option<NodeStatus> {
        let nodes = self.nodes.read().await;
        nodes.get(node_id).cloned()
    }

    /// Get orchestrator config
    pub fn config(&self) -> &OrchestratorConfig {
        &self.config
    }

    /// Get all nodes (for status reporting)
    pub async fn all_nodes(&self) -> Vec<NodeStatus> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().collect()
    }
}

/// Orchestrator service that coordinates all components
pub struct Orchestrator {
    state: Arc<OrchestratorState>,
    bootstrap: Option<NodeBootstrap>,
    heartbeat: Option<HeartbeatLoop>,
    disaster_recovery: Option<DisasterRecoveryCoordinator>,
}

impl Orchestrator {
    /// Create new orchestrator with fresh state
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            state: Arc::new(OrchestratorState::new(config)),
            bootstrap: None,
            heartbeat: None,
            disaster_recovery: None,
        }
    }

    /// Create orchestrator with existing state (for sharing state with AppState)
    pub fn with_state(state: Arc<OrchestratorState>) -> Self {
        Self {
            state,
            bootstrap: None,
            heartbeat: None,
            disaster_recovery: None,
        }
    }

    /// Get shared state
    pub fn state(&self) -> Arc<OrchestratorState> {
        Arc::clone(&self.state)
    }

    /// Start all orchestrator components
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting orchestrator...");

        // Start node bootstrap (mDNS listener)
        let bootstrap_config = NodeBootstrapConfig {
            mdns_service_type: self.state.config.mdns_service_type.clone(),
            admin_port: self.state.config.admin_port,
            region: self.state.config.region.clone(),
        };
        let bootstrap = NodeBootstrap::new(bootstrap_config, Arc::clone(&self.state));
        bootstrap.start().await?;
        self.bootstrap = Some(bootstrap);

        // Start heartbeat loop
        let hb_config = HeartbeatConfig {
            interval_secs: self.state.config.heartbeat_interval_secs,
            failure_threshold: self.state.config.failure_threshold,
        };
        let heartbeat = HeartbeatLoop::new(hb_config, Arc::clone(&self.state));
        heartbeat.start().await?;
        self.heartbeat = Some(heartbeat);

        // Start disaster recovery coordinator
        let dr = DisasterRecoveryCoordinator::new(Arc::clone(&self.state));
        dr.start().await?;
        self.disaster_recovery = Some(dr);

        info!("Orchestrator started successfully");
        Ok(())
    }

    /// Graceful shutdown
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down orchestrator...");

        if let Some(hb) = self.heartbeat.take() {
            hb.stop().await?;
        }
        if let Some(bootstrap) = self.bootstrap.take() {
            bootstrap.stop().await?;
        }
        if let Some(dr) = self.disaster_recovery.take() {
            dr.stop().await?;
        }

        info!("Orchestrator shutdown complete");
        Ok(())
    }
}
