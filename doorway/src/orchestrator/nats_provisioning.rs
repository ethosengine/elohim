//! NATS JWT credential provisioning
//!
//! Provisions NATS credentials for new nodes using the HOLO operator
//! chain of trust pattern (Operator → Account → User).
//!
//! ## Credential Structure
//!
//! Each node receives:
//! - User JWT scoped to HPOS account
//! - Signing key with workload_role permissions
//! - Access to WORKLOAD.{node_id}.* subjects
//!
//! ## Permission Model
//!
//! Nodes can:
//! - Publish: WORKLOAD.orchestrator.>, WORKLOAD.{pubkey}.>, INVENTORY.*.{pubkey}.update.>
//! - Subscribe: WORKLOAD.{pubkey}.>, INVENTORY.*.{pubkey}.>
//!
//! This uses template substitution ({{tag(pubkey)}}) for per-node scoping.

use crate::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// NATS credentials for a provisioned node
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NatsCredentials {
    /// User credentials file content (.creds format)
    pub creds_content: String,
    /// User public key
    pub user_pub_key: String,
    /// Account name (HPOS)
    pub account: String,
    /// Role assigned (workload_role)
    pub role: String,
    /// NATS server URLs
    pub server_urls: Vec<String>,
    /// Expiration timestamp (ISO 8601)
    pub expires_at: Option<String>,
}

/// NATS credential provisioner
pub struct NatsProvisioner {
    /// NATS server URL
    nats_url: String,
    /// Path to operator signing key
    operator_sk_path: Option<String>,
    /// Path to HPOS account signing key
    hpos_sk_path: Option<String>,
}

impl NatsProvisioner {
    /// Create new provisioner
    pub fn new(nats_url: String) -> Self {
        Self {
            nats_url,
            operator_sk_path: None,
            hpos_sk_path: None,
        }
    }

    /// Configure signing key paths
    pub fn with_signing_keys(mut self, operator_sk_path: String, hpos_sk_path: String) -> Self {
        self.operator_sk_path = Some(operator_sk_path);
        self.hpos_sk_path = Some(hpos_sk_path);
        self
    }

    /// Provision NATS credentials for a node
    pub async fn provision_node(&self, node_id: &str) -> Result<NatsCredentials> {
        info!(node_id = %node_id, "Provisioning NATS credentials");

        // In production, this would use nkeys + nsc libraries to:
        // 1. Generate user keypair
        // 2. Create user JWT signed by HPOS account signing key
        // 3. Scope permissions to node's pubkey using templates
        // 4. Set expiration
        //
        // Example nsc commands equivalent:
        // nsc add user --name {node_id} --account HPOS -K workload_role --tag pubkey:{node_pubkey}
        // nsc generate creds --name {node_id} --account HPOS

        // For now, create placeholder credentials
        let creds = NatsCredentials {
            creds_content: format!(
                "-----BEGIN NATS USER JWT-----\n\
                placeholder_jwt_for_{node_id}\n\
                ------END NATS USER JWT------\n\n\
                ************************* IMPORTANT *************************\n\
                NKEY Seed printed below can be used to sign and prove identity.\n\
                NKEYs are sensitive and should be treated as secrets.\n\n\
                -----BEGIN USER NKEY SEED-----\n\
                SUAIBDPBAUTWCWBKIO6XHQNINK5FWJW4OHLXC3HQ2KFE4PEJUA44CNHTAM\n\
                ------END USER NKEY SEED------\n\n\
                *************************************************************\n"
            ),
            user_pub_key: format!("U{}", generate_placeholder_key()),
            account: "HPOS".to_string(),
            role: "workload_role".to_string(),
            server_urls: vec![self.nats_url.clone()],
            expires_at: Some((chrono::Utc::now() + chrono::Duration::days(365)).to_rfc3339()),
        };

        debug!(
            node_id = %node_id,
            user_pub_key = %creds.user_pub_key,
            "Credentials generated"
        );

        Ok(creds)
    }

    /// Revoke credentials for a node
    pub async fn revoke_node(&self, node_id: &str) -> Result<()> {
        warn!(node_id = %node_id, "Revoking NATS credentials");

        // In production:
        // nsc revocations add-user --account HPOS --name {node_id}
        // nsc push --account HPOS

        Ok(())
    }

    /// Renew credentials for a node
    pub async fn renew_node(&self, node_id: &str) -> Result<NatsCredentials> {
        info!(node_id = %node_id, "Renewing NATS credentials");

        // In production:
        // 1. Check current credentials validity
        // 2. Generate new JWT with extended expiration
        // 3. Push to resolver

        self.provision_node(node_id).await
    }
}

/// Generate a placeholder key (for development)
fn generate_placeholder_key() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{timestamp:040X}")
}

/// NATS permission template for workload role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadPermissions {
    pub publish: WorkloadPublishPerms,
    pub subscribe: WorkloadSubscribePerms,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadPublishPerms {
    /// Patterns the node can publish to
    pub allow: Vec<String>,
    /// Patterns the node cannot publish to
    pub deny: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadSubscribePerms {
    /// Patterns the node can subscribe to
    pub allow: Vec<String>,
    /// Patterns the node cannot subscribe to
    pub deny: Vec<String>,
}

impl Default for WorkloadPermissions {
    fn default() -> Self {
        // These templates use {{tag(pubkey)}} which gets substituted
        // with the node's actual public key at runtime by NATS
        Self {
            publish: WorkloadPublishPerms {
                allow: vec![
                    "WORKLOAD.orchestrator.>".to_string(),
                    "WORKLOAD.{{tag(pubkey)}}.>".to_string(),
                    "INVENTORY.*.{{tag(pubkey)}}.update.>".to_string(),
                    "$JS.API.>".to_string(),
                    "_HPOS_INBOX.{{tag(pubkey)}}.>".to_string(),
                    "_ADMIN_INBOX.orchestrator.>".to_string(),
                ],
                deny: vec![],
            },
            subscribe: WorkloadSubscribePerms {
                allow: vec![
                    "WORKLOAD.{{tag(pubkey)}}.>".to_string(),
                    "INVENTORY.*.{{tag(pubkey)}}.>".to_string(),
                    "$JS.API.>".to_string(),
                    "_HPOS_INBOX.{{tag(pubkey)}}.>".to_string(),
                ],
                deny: vec![],
            },
        }
    }
}

/// Subject patterns for orchestrator communication
pub struct OrchestratorSubjects;

impl OrchestratorSubjects {
    /// Workload commands from orchestrator to node
    pub fn workload_to_node(node_pubkey: &str) -> String {
        format!("WORKLOAD.{node_pubkey}.>")
    }

    /// Workload responses from node to orchestrator
    pub fn workload_from_node() -> &'static str {
        "WORKLOAD.orchestrator.>"
    }

    /// Inventory updates from node
    pub fn inventory_update(node_pubkey: &str) -> String {
        format!("INVENTORY.*.{node_pubkey}.update.>")
    }

    /// Node's inbox for receiving credentials and commands
    pub fn node_inbox(node_pubkey: &str) -> String {
        format!("_HPOS_INBOX.{node_pubkey}.>")
    }

    /// Orchestrator's inbox for responses
    pub fn orchestrator_inbox() -> &'static str {
        "_ADMIN_INBOX.orchestrator.>"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provision_node() {
        let provisioner = NatsProvisioner::new("nats://localhost:4222".to_string());
        let creds = provisioner.provision_node("test-node-001").await.unwrap();

        assert_eq!(creds.account, "HPOS");
        assert_eq!(creds.role, "workload_role");
        assert!(!creds.creds_content.is_empty());
        assert!(creds.user_pub_key.starts_with("U"));
    }

    #[test]
    fn test_workload_permissions() {
        let perms = WorkloadPermissions::default();

        assert!(perms
            .publish
            .allow
            .contains(&"WORKLOAD.orchestrator.>".to_string()));
        assert!(perms
            .publish
            .allow
            .contains(&"WORKLOAD.{{tag(pubkey)}}.>".to_string()));
        assert!(perms
            .subscribe
            .allow
            .contains(&"WORKLOAD.{{tag(pubkey)}}.>".to_string()));
    }

    #[test]
    fn test_subject_patterns() {
        let node_key = "UABC123";

        assert_eq!(
            OrchestratorSubjects::workload_to_node(node_key),
            "WORKLOAD.UABC123.>"
        );
        assert_eq!(
            OrchestratorSubjects::node_inbox(node_key),
            "_HPOS_INBOX.UABC123.>"
        );
    }
}
