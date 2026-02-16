//! Node bootstrap - mDNS discovery and registration
//!
//! Listens for mDNS announcements from new nodes and handles:
//! 1. Discovery via mDNS (service type: _elohim-node._tcp)
//! 2. Inventory collection from node
//! 3. Registration with Node Registry DNA
//! 4. Triggering NATS credential provisioning
//!
//! ## mDNS Protocol
//!
//! Nodes announce themselves with:
//! - Service type: `_elohim-node._tcp.local`
//! - TXT records containing JSON inventory
//! - Service name: node ID
//!
//! ## Bootstrap Flow
//!
//! ```text
//! 1. Node starts, broadcasts mDNS announcement
//! 2. Doorway receives announcement, parses inventory
//! 3. Doorway calls register_node() on Node Registry DNA
//! 4. Doorway provisions NATS JWT credentials
//! 5. Doorway sends credentials to node via NATS INBOX
//! 6. Node starts heartbeat loop
//! ```

use super::{
    inventory::{InventoryAnnouncement, NodeInventory},
    nats_provisioning::NatsProvisioner,
    OrchestratorState,
};
use crate::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Configuration for node bootstrap
#[derive(Debug, Clone)]
pub struct NodeBootstrapConfig {
    /// mDNS service type to listen for
    pub mdns_service_type: String,
    /// Holochain admin websocket port for DNA calls
    pub admin_port: u16,
    /// Region for this orchestrator
    pub region: String,
}

impl Default for NodeBootstrapConfig {
    fn default() -> Self {
        Self {
            mdns_service_type: "elohim-node".to_string(),
            admin_port: 8888,
            region: "default".to_string(),
        }
    }
}

/// Node bootstrap service
pub struct NodeBootstrap {
    config: NodeBootstrapConfig,
    state: Arc<OrchestratorState>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl NodeBootstrap {
    /// Create new node bootstrap service
    pub fn new(config: NodeBootstrapConfig, state: Arc<OrchestratorState>) -> Self {
        Self {
            config,
            state,
            shutdown_tx: None,
        }
    }

    /// Start the mDNS listener
    pub async fn start(&self) -> Result<()> {
        info!(
            service_type = %self.config.mdns_service_type,
            "Starting mDNS listener for node discovery"
        );

        let state = Arc::clone(&self.state);
        let config = self.config.clone();

        // Spawn mDNS listener task
        tokio::spawn(async move {
            if let Err(e) = run_mdns_listener(config, state).await {
                error!(error = %e, "mDNS listener failed");
            }
        });

        Ok(())
    }

    /// Stop the mDNS listener
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping mDNS listener");
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(()).await;
        }
        Ok(())
    }

    /// Manually process a node announcement (for testing or NATS-based discovery)
    pub async fn process_announcement(&self, announcement: InventoryAnnouncement) -> Result<()> {
        process_node_announcement(announcement, Arc::clone(&self.state), &self.config).await
    }
}

/// Run the mDNS listener loop
async fn run_mdns_listener(
    _config: NodeBootstrapConfig,
    _state: Arc<OrchestratorState>,
) -> Result<()> {
    // Note: In production, this would use mdns-sd or similar crate
    // For now, we'll create a placeholder that can be replaced with actual mDNS

    info!("mDNS listener started - waiting for node announcements");

    // Placeholder: In production, use:
    // let mdns = ServiceDaemon::new()?;
    // let receiver = mdns.browse(&format!("_{}._{}.local", config.mdns_service_type, "tcp"))?;

    // For now, simulate waiting for announcements
    // Real implementation would:
    // while let Ok(event) = receiver.recv_async().await {
    //     match event {
    //         ServiceEvent::ServiceResolved(info) => {
    //             let inventory = parse_txt_records(&info);
    //             let announcement = InventoryAnnouncement::new_announce(inventory);
    //             process_node_announcement(announcement, Arc::clone(&state), &config).await?;
    //         }
    //         _ => {}
    //     }
    // }

    // Keep the task alive
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

/// Process a node announcement
async fn process_node_announcement(
    announcement: InventoryAnnouncement,
    state: Arc<OrchestratorState>,
    config: &NodeBootstrapConfig,
) -> Result<()> {
    let node_id = &announcement.inventory.node_id;

    match announcement.msg_type.as_str() {
        "announce" => {
            info!(node_id = %node_id, "Processing new node announcement");

            // Check if node already registered
            if let Some(existing) = state.get_node(node_id).await {
                warn!(
                    node_id = %node_id,
                    status = ?existing.status,
                    "Node already registered, updating inventory"
                );
                // Could update inventory here
                return Ok(());
            }

            // Register with orchestrator state
            state
                .register_node(node_id.clone(), announcement.inventory.clone())
                .await?;

            // Register with Node Registry DNA
            if let Err(e) = register_with_dna(&announcement.inventory, config.admin_port).await {
                error!(
                    node_id = %node_id,
                    error = %e,
                    "Failed to register node with DNA"
                );
                return Err(e);
            }

            // Provision NATS credentials
            let provisioner = NatsProvisioner::new(state.config().nats_url.clone());
            match provisioner.provision_node(node_id).await {
                Ok(creds) => {
                    info!(
                        node_id = %node_id,
                        "NATS credentials provisioned"
                    );

                    // Send credentials to node via NATS INBOX
                    if let Err(e) = send_credentials_to_node(node_id, &creds, &state).await {
                        error!(
                            node_id = %node_id,
                            error = %e,
                            "Failed to send NATS credentials to node"
                        );
                    }

                    // Mark node as provisioned
                    state.mark_provisioned(node_id).await?;
                }
                Err(e) => {
                    error!(
                        node_id = %node_id,
                        error = %e,
                        "Failed to provision NATS credentials"
                    );
                }
            }

            // Auto-assign custodian responsibilities if enabled
            if state.config().auto_assign_custodians && announcement.inventory.custodian_opt_in {
                if let Err(e) =
                    auto_assign_custodian(&announcement.inventory, config.admin_port).await
                {
                    warn!(
                        node_id = %node_id,
                        error = %e,
                        "Failed to auto-assign custodian responsibilities"
                    );
                }
            }

            info!(node_id = %node_id, "Node bootstrap complete");
        }

        "update" => {
            debug!(node_id = %node_id, "Processing inventory update");
            // Update node inventory in state
            // Could call update_node_capacity() on DNA
        }

        "deregister" => {
            info!(node_id = %node_id, "Processing node deregistration");
            // Handle graceful deregistration
            // Call deregister_node() on DNA
        }

        _ => {
            warn!(
                node_id = %node_id,
                msg_type = %announcement.msg_type,
                "Unknown announcement type"
            );
        }
    }

    Ok(())
}

/// Register node with Node Registry DNA via Holochain admin websocket
async fn register_with_dna(inventory: &NodeInventory, _admin_port: u16) -> Result<()> {
    // Convert inventory to DNA input
    let input = inventory.to_registration_input();

    info!(
        node_id = %input.node_id,
        region = %input.region,
        tier = %input.steward_tier,
        "Registering node with Node Registry DNA"
    );

    // In production, this would:
    // 1. Connect to Holochain admin websocket
    // 2. Get the app websocket port
    // 3. Call register_node() zome function
    //
    // let admin_ws = AdminWebsocket::connect(format!("localhost:{}", admin_port)).await?;
    // let app_ws = admin_ws.connect_app_ws().await?;
    // let cell_id = get_node_registry_cell(&admin_ws).await?;
    // app_ws.call_zome({
    //     cell_id,
    //     zome_name: "node_registry_coordinator",
    //     fn_name: "register_node",
    //     payload: input,
    // }).await?;

    debug!(node_id = %input.node_id, "DNA registration simulated");
    Ok(())
}

/// Send NATS credentials to node
async fn send_credentials_to_node(
    node_id: &str,
    _creds: &super::nats_provisioning::NatsCredentials,
    _state: &OrchestratorState,
) -> Result<()> {
    // In production, publish to node's NATS inbox
    // let nats_client = state.nats_client.as_ref().ok_or("No NATS client")?;
    // let inbox = format!("_HPOS_INBOX.{}.credentials", node_id);
    // nats_client.publish(&inbox, &serde_json::to_vec(creds)?).await?;

    debug!(node_id = %node_id, "Credentials delivery simulated");
    Ok(())
}

/// Auto-assign custodian responsibilities to new node
async fn auto_assign_custodian(inventory: &NodeInventory, _admin_port: u16) -> Result<()> {
    // In production, this would:
    // 1. Query for content needing custodians in this region
    // 2. Calculate optimal assignments based on node capacity
    // 3. Call assign_custodian() for each assignment
    //
    // let available_content = get_content_needing_custodians(
    //     &inventory.region,
    //     inventory.max_custody_gb,
    // ).await?;
    //
    // for content in available_content {
    //     let assignment = CustodianAssignment {
    //         content_id: content.id,
    //         custodian_node_id: inventory.node_id.clone(),
    //         strategy: "full_replica".to_string(),
    //         ...
    //     };
    //     assign_custodian(assignment).await?;
    // }

    info!(
        node_id = %inventory.node_id,
        region = %inventory.region,
        max_custody_gb = ?inventory.max_custody_gb,
        "Auto-assign custodian simulated"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::inventory::NodeCapacity;

    #[tokio::test]
    async fn test_node_bootstrap_creation() {
        let config = NodeBootstrapConfig::default();
        let state = Arc::new(OrchestratorState::new(Default::default()));
        let _bootstrap = NodeBootstrap::new(config, state);
    }

    #[tokio::test]
    async fn test_process_announcement() {
        let state = Arc::new(OrchestratorState::new(Default::default()));
        let config = NodeBootstrapConfig::default();

        let inventory = NodeInventory::new(
            "test-node".to_string(),
            "uhCAk...".to_string(),
            NodeCapacity {
                cpu_cores: 4,
                memory_gb: 16,
                storage_tb: 1.0,
                bandwidth_mbps: 500,
            },
            "us-west-2".to_string(),
        );

        let announcement = InventoryAnnouncement::new_announce(inventory);

        // This will fail DNA registration but should not panic
        let result = process_node_announcement(announcement, state.clone(), &config).await;
        // In test environment without Holochain, this is expected to succeed
        // because DNA registration is simulated
        assert!(result.is_ok());

        // Verify node was registered with state
        let node = state.get_node("test-node").await;
        assert!(node.is_some());
    }
}
