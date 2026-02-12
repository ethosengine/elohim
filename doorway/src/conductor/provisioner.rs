//! Agent Provisioner — orchestrates Holochain agent provisioning
//!
//! When a user registers, the provisioner:
//! 1. Finds the least loaded conductor
//! 2. Generates a new agent key on that conductor
//! 3. Installs the app with that agent key
//! 4. Enables the app
//! 5. Registers the agent→conductor mapping
//!
//! Provisioning failure is non-fatal — registration falls back to local key generation.

use std::sync::Arc;
use tracing::{info, warn};

use super::admin_client::AdminClient;
use super::registry::ConductorRegistry;

/// Default app ID prefix for provisioned agents.
const DEFAULT_APP_ID: &str = "elohim";

/// Default bundle path for the Holochain app.
const DEFAULT_BUNDLE_PATH: &str = "/app/elohim.happ";

/// Result of successful agent provisioning.
#[derive(Debug, Clone)]
pub struct ProvisionedAgent {
    /// Base64-encoded 39-byte agent public key from the conductor.
    pub agent_pub_key: String,
    /// Conductor identifier (e.g., "conductor-0").
    pub conductor_id: String,
    /// Conductor WebSocket URL.
    pub conductor_url: String,
    /// Installed app ID on the conductor.
    pub installed_app_id: String,
}

/// Orchestrates agent provisioning on a conductor.
pub struct AgentProvisioner {
    registry: Arc<ConductorRegistry>,
    app_id: String,
    bundle_path: String,
}

impl AgentProvisioner {
    /// Create a new provisioner backed by the given conductor registry.
    pub fn new(registry: Arc<ConductorRegistry>) -> Self {
        Self {
            registry,
            app_id: DEFAULT_APP_ID.to_string(),
            bundle_path: DEFAULT_BUNDLE_PATH.to_string(),
        }
    }

    /// Override the base app ID used for installed apps.
    pub fn with_app_id(mut self, app_id: String) -> Self {
        self.app_id = app_id;
        self
    }

    /// Override the bundle path used for app installation.
    pub fn with_bundle_path(mut self, path: String) -> Self {
        self.bundle_path = path;
        self
    }

    /// Provision a new agent for the given user.
    ///
    /// Flow:
    /// 1. `registry.find_least_loaded()` — fail if no conductors or all at capacity
    /// 2. `AdminClient::new(admin_url)` — short-lived connection
    /// 3. `admin.generate_agent_pub_key()` — 39-byte agent key
    /// 4. `admin.install_app(app_id, agent_key, bundle_path)` — if fails, no cleanup needed
    /// 5. `admin.enable_app(app_id)` — if fails, attempt `uninstall_app` cleanup
    /// 6. `registry.register_agent(...)` — persist to DashMap + MongoDB
    pub async fn provision_agent(&self, user_identifier: &str) -> Result<ProvisionedAgent, String> {
        // 1. Find least loaded conductor
        let conductor = self
            .registry
            .find_least_loaded()
            .ok_or("No conductors available for provisioning")?;

        // Check capacity
        if conductor.capacity_used >= conductor.capacity_max {
            return Err(format!(
                "Conductor {} at capacity ({}/{})",
                conductor.conductor_id, conductor.capacity_used, conductor.capacity_max
            ));
        }

        let installed_app_id =
            generate_app_id(&self.app_id, &conductor.conductor_id, user_identifier);

        info!(
            conductor = %conductor.conductor_id,
            admin_url = %conductor.admin_url,
            installed_app_id = %installed_app_id,
            user = %user_identifier,
            "Provisioning agent on conductor"
        );

        // 2. Create admin client
        let admin = AdminClient::new(conductor.admin_url.clone());

        // 3. Generate agent key
        let agent_key = admin.generate_agent_pub_key().await.map_err(|e| {
            format!(
                "Failed to generate agent key on {}: {}",
                conductor.conductor_id, e
            )
        })?;

        let agent_pub_key_b64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &agent_key);

        // 4. Install app
        if let Err(e) = admin
            .install_app(&installed_app_id, &agent_key, &self.bundle_path)
            .await
        {
            // Install failed — orphaned key is harmless, no cleanup needed
            return Err(format!(
                "Failed to install app on {}: {}",
                conductor.conductor_id, e
            ));
        }

        // 5. Enable app
        if let Err(e) = admin.enable_app(&installed_app_id).await {
            // Enable failed — attempt uninstall cleanup
            warn!(
                "Enable failed, attempting uninstall cleanup for {}: {}",
                installed_app_id, e
            );
            if let Err(cleanup_err) = admin.uninstall_app(&installed_app_id).await {
                warn!("Cleanup uninstall also failed: {}", cleanup_err);
            }
            return Err(format!(
                "Failed to enable app on {}: {}",
                conductor.conductor_id, e
            ));
        }

        // 6. Register agent→conductor mapping
        if let Err(e) = self
            .registry
            .register_agent(
                &agent_pub_key_b64,
                &conductor.conductor_id,
                &installed_app_id,
            )
            .await
        {
            // Registration failed — attempt uninstall cleanup
            warn!(
                "Agent registration failed, attempting uninstall cleanup for {}: {}",
                installed_app_id, e
            );
            if let Err(cleanup_err) = admin.uninstall_app(&installed_app_id).await {
                warn!("Cleanup uninstall also failed: {}", cleanup_err);
            }
            return Err(format!("Failed to register agent mapping: {}", e));
        }

        info!(
            conductor = %conductor.conductor_id,
            agent = %agent_pub_key_b64,
            app_id = %installed_app_id,
            "Agent provisioned successfully"
        );

        Ok(ProvisionedAgent {
            agent_pub_key: agent_pub_key_b64,
            conductor_id: conductor.conductor_id,
            conductor_url: conductor.conductor_url,
            installed_app_id,
        })
    }

    /// Deprovision an agent — uninstall the app from its conductor.
    pub async fn deprovision_agent(&self, agent_pub_key: &str) -> Result<(), String> {
        // Look up conductor assignment
        let entry = self
            .registry
            .get_conductor_for_agent(agent_pub_key)
            .ok_or_else(|| format!("Agent {} not found in registry", agent_pub_key))?;

        // Get conductor info for admin URL
        let conductors = self.registry.list_conductors();
        let conductor = conductors
            .iter()
            .find(|c| c.conductor_id == entry.conductor_id)
            .ok_or_else(|| format!("Conductor {} not found in pool", entry.conductor_id))?;

        info!(
            conductor = %entry.conductor_id,
            agent = %agent_pub_key,
            app_id = %entry.app_id,
            "Deprovisioning agent"
        );

        let admin = AdminClient::new(conductor.admin_url.clone());
        admin.uninstall_app(&entry.app_id).await.map_err(|e| {
            format!(
                "Failed to uninstall app {} on {}: {}",
                entry.app_id, entry.conductor_id, e
            )
        })?;

        // Remove from registry
        self.registry.unregister_agent(agent_pub_key);

        info!(
            conductor = %entry.conductor_id,
            agent = %agent_pub_key,
            "Agent deprovisioned successfully"
        );

        Ok(())
    }
}

/// Generate the installed app ID for a user on a conductor.
///
/// Format: `{app_id}-{conductor_id}-{hash(user_identifier)[0:6]}`
fn generate_app_id(app_id: &str, conductor_id: &str, user_identifier: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(user_identifier.as_bytes());
    let hash = hasher.finalize();
    let short_hash = hex::encode(&hash[..3]); // 6 hex chars from 3 bytes

    format!("{}-{}-{}", app_id, conductor_id, short_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_app_id() {
        let id = generate_app_id("elohim", "conductor-0", "test@example.com");
        assert!(id.starts_with("elohim-conductor-0-"));
        assert_eq!(id.len(), "elohim-conductor-0-".len() + 6); // 6 hex chars

        // Deterministic
        let id2 = generate_app_id("elohim", "conductor-0", "test@example.com");
        assert_eq!(id, id2);

        // Different input → different hash
        let id3 = generate_app_id("elohim", "conductor-0", "other@example.com");
        assert_ne!(id, id3);
    }

    #[tokio::test]
    async fn test_provisioner_creation() {
        let registry = Arc::new(ConductorRegistry::new(None).await);
        let provisioner = AgentProvisioner::new(Arc::clone(&registry))
            .with_app_id("my-app".to_string())
            .with_bundle_path("/path/to/bundle.happ".to_string());

        assert_eq!(provisioner.app_id, "my-app");
        assert_eq!(provisioner.bundle_path, "/path/to/bundle.happ");
    }

    #[tokio::test]
    async fn test_provision_no_conductors() {
        let registry = Arc::new(ConductorRegistry::new(None).await);
        let provisioner = AgentProvisioner::new(Arc::clone(&registry));

        let result = provisioner.provision_agent("test@example.com").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No conductors available"));
    }

    #[tokio::test]
    async fn test_deprovision_unknown_agent() {
        let registry = Arc::new(ConductorRegistry::new(None).await);
        let provisioner = AgentProvisioner::new(Arc::clone(&registry));

        let result = provisioner.deprovision_agent("unknown_key").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found in registry"));
    }
}
