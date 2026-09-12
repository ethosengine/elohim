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
use tracing::{debug, info, warn};

use super::agent_key::{canonical_agent_key, lookup_forms};
use super::registry::ConductorRegistry;
use super::typed_admin::TypedAdminClient;

/// The EXACT string a freshly provisioned agent is known by — the one value
/// that becomes [`ProvisionedAgent::agent_pub_key`], and from there the JWT
/// claim, the account response, and the `recipient` of the hosted-cell compute
/// grant.
///
/// Split out as a pure function precisely so that last hop can be pinned by a
/// test without a conductor: `elohim-storage`'s grant surface does
/// `AgentPubKey::try_from(recipient)` and refuses anything that is not the
/// canonical HoloHash form. This used to be bare base64 (`hCAk…`), which made
/// every hosted registration's promise leg a 500 while every other reader —
/// all of them pass-through — stayed silent about it.
pub fn provisioned_agent_key(raw: &[u8]) -> String {
    canonical_agent_key(raw)
}

/// Default app ID prefix for provisioned agents.
const DEFAULT_APP_ID: &str = "elohim";

/// Default bundle path for the Holochain app.
const DEFAULT_BUNDLE_PATH: &str = "/app/elohim.happ";

/// How many conductors a single provisioning call may be re-offered to when the
/// admin socket drops mid-call.
///
/// Bounded deliberately: a registration is an interactive request behind a
/// hosted human's sign-up, so the budget is "ask a couple of siblings", never
/// "walk the pool". Each attempt targets a DIFFERENT conductor (the failing one
/// is excluded), so three attempts is three distinct members, not three tries at
/// the same sick one.
const PROVISION_TRANSPORT_ATTEMPTS: u32 = 3;

/// Is this provisioning error a TRANSPORT fault rather than a verdict?
///
/// A transport fault means the admin websocket died before the conductor
/// answered — the work was never performed *or* its outcome was never reported,
/// and in both cases nothing is established about whether it can succeed
/// elsewhere. A verdict (at capacity, cell genesis timed out, the conductor
/// refused) is a real answer and must not be retried into a second side effect.
///
/// Matched on the rendered error string because that is the only shape the
/// `holochain_client` admin errors reach this layer in: they arrive as
/// `External API wire error: InternalError("… Other({\"error\":\"BrokenPipe\"})")`,
/// with the discriminant already flattened into text upstream. Widening this
/// set is safe in one direction only — a false POSITIVE costs one extra
/// provisioning attempt on a different conductor, a false NEGATIVE costs a human
/// their registration.
fn is_transport_fault(error: &str) -> bool {
    const TRANSPORT_MARKERS: [&str; 8] = [
        "BrokenPipe",
        "ConnectionAborted",
        "ConnectionReset",
        "ConnectionRefused",
        "connection closed",
        "Connection closed",
        "websocket closed",
        "Failed to connect to admin",
    ];
    TRANSPORT_MARKERS
        .iter()
        .any(|marker| error.contains(marker))
}

/// Result of successful agent provisioning.
#[derive(Debug, Clone)]
pub struct ProvisionedAgent {
    /// Base64-encoded 39-byte agent public key from the conductor.
    pub agent_pub_key: String,
    /// Conductor identifier (e.g., "conductor-0").
    pub conductor_id: String,
    /// Conductor WebSocket URL (app interface).
    pub conductor_url: String,
    /// Conductor ADMIN WebSocket URL, taken from the registry rather than
    /// derived from `conductor_url`. The `app_port - 1` derivation is a socat
    /// convention that does not hold for `hc sandbox`, which picks a random
    /// admin port and pins the app interface separately — so callers that
    /// re-derived it were dialling a port nothing listens on.
    pub admin_url: String,
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

    /// Provision an agent for the given user (idempotent).
    ///
    /// Searches ALL conductors for an existing app before installing a new one.
    /// The app ID is deterministic per (app_id, conductor_id, user_identifier),
    /// so we check each conductor's expected app ID. This handles the logout→
    /// re-login case where `find_least_loaded()` returns a different conductor
    /// than the one the app was originally installed on.
    ///
    /// Flow, per attempt:
    /// 1. Search all conductors for existing app (idempotency)
    /// 2. If not found, pick the least loaded conductor not already excluded
    /// 3. Generate agent key, install, enable, register
    ///
    /// ## A dropped admin socket is not a provisioning verdict
    ///
    /// `elohim-genesis/dev` #1576 refused two hosted registrations on a
    /// seven-peer-healthy alpha with
    /// `Failed to generate agent key on conductor-3: … Other: {"error":"BrokenPipe"}`
    /// and
    /// `Failed to install app on conductor-3: … Other({"error":"ConnectionAborted"})`,
    /// each surfacing to the human as `503 PROVISIONING_FAILED`. Both are
    /// TRANSPORT faults on the admin websocket: the conductor never answered,
    /// so neither establishes anything about the work — exactly the class the
    /// substrate trust contract says must be re-offered rather than verdicted
    /// (the same lesson `conductor_admission`'s shed carries in elohim-storage,
    /// and the same lesson the seed-blob forward learned on 2026-09-02).
    ///
    /// Only `conductor-3` appeared anywhere in that build's log: the pool held
    /// other members and none was ever asked. The re-offer is bounded at
    /// [`PROVISION_TRANSPORT_ATTEMPTS`] and EXCLUDES the conductor that dropped,
    /// so a single sick pool member cannot absorb every attempt — and the
    /// idempotency search re-runs at the top of each attempt, because an
    /// `install_app` whose socket died may have landed before it died.
    ///
    /// A non-transport error (at capacity, cell genesis timed out, a real
    /// conductor refusal) is returned immediately and unchanged: those ARE
    /// verdicts.
    pub async fn provision_agent(&self, user_identifier: &str) -> Result<ProvisionedAgent, String> {
        let mut excluded: Vec<String> = Vec::new();
        let mut last_transport_error: Option<String> = None;

        for attempt in 1..=PROVISION_TRANSPORT_ATTEMPTS {
            // 1. Search ALL conductors for an existing app for this user.
            // Re-run per attempt, not once: a previous attempt's aborted
            // install may have completed on the conductor that stopped talking.
            if let Some(result) = self.find_existing_app(user_identifier).await {
                return Ok(result);
            }

            // 2. No existing app found — provision on the least loaded
            // conductor that has not already dropped on us in this call.
            let Some(conductor) = self.registry.find_least_loaded_excluding(&excluded) else {
                break;
            };

            match self.provision_on(&conductor, user_identifier).await {
                Ok(agent) => return Ok(agent),
                Err(e) if is_transport_fault(&e) => {
                    warn!(
                        conductor = %conductor.conductor_id,
                        attempt,
                        max_attempts = PROVISION_TRANSPORT_ATTEMPTS,
                        error = %e,
                        "conductor admin socket dropped mid-provision — re-offering to \
                         another conductor; a dropped socket says nothing about the work"
                    );
                    excluded.push(conductor.conductor_id.clone());
                    last_transport_error = Some(e);
                }
                Err(e) => return Err(e),
            }
        }

        Err(last_transport_error
            .unwrap_or_else(|| "No conductors available for provisioning".to_string()))
    }

    /// One provisioning attempt against ONE named conductor.
    async fn provision_on(
        &self,
        conductor: &super::registry::ConductorInfo,
        user_identifier: &str,
    ) -> Result<ProvisionedAgent, String> {
        let conductor = conductor.clone();
        if conductor.capacity_used >= conductor.capacity_max {
            return Err(format!(
                "Conductor {} at capacity ({}/{})",
                conductor.conductor_id, conductor.capacity_used, conductor.capacity_max
            ));
        }

        let installed_app_id =
            generate_app_id(&self.app_id, &conductor.conductor_id, user_identifier);
        let admin = TypedAdminClient::connect(&conductor.admin_url)
            .await
            .map_err(|e| {
                format!(
                    "Failed to connect to admin on {}: {}",
                    conductor.conductor_id, e
                )
            })?;

        info!(
            conductor = %conductor.conductor_id,
            admin_url = %conductor.admin_url,
            installed_app_id = %installed_app_id,
            user = %user_identifier,
            "Provisioning new agent on conductor"
        );

        // 3. Generate agent key
        let agent_key = admin.generate_agent_pub_key().await.map_err(|e| {
            format!(
                "Failed to generate agent key on {}: {}",
                conductor.conductor_id, e
            )
        })?;

        let agent_pub_key_b64 = provisioned_agent_key(&agent_key);

        // 4. Install app
        if let Err(e) = admin
            .install_app(&installed_app_id, &agent_key, &self.bundle_path)
            .await
        {
            return Err(format!(
                "Failed to install app on {}: {}",
                conductor.conductor_id, e
            ));
        }

        // 5. Enable app
        if let Err(e) = admin.enable_app(&installed_app_id).await {
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

        // 6. Verify cells are ready before returning (cell genesis can take seconds)
        {
            let max_polls = 30;
            let poll_interval = std::time::Duration::from_millis(500);
            let mut cells_ready = false;

            for attempt in 1..=max_polls {
                match admin.get_app_info(&installed_app_id).await {
                    Ok(info) if !info.cell_ids.is_empty() => {
                        let role_names: Vec<&str> =
                            info.cell_ids.iter().map(|(r, _)| r.as_str()).collect();
                        info!(
                            conductor = %conductor.conductor_id,
                            app_id = %installed_app_id,
                            cell_count = info.cell_ids.len(),
                            roles = ?role_names,
                            polls = attempt,
                            "Cell genesis complete — cells ready"
                        );
                        cells_ready = true;
                        break;
                    }
                    Ok(_) => {
                        debug!(
                            app_id = %installed_app_id,
                            attempt,
                            max_polls,
                            "Waiting for cell genesis..."
                        );
                    }
                    Err(e) => {
                        debug!(
                            app_id = %installed_app_id,
                            attempt,
                            error = %e,
                            "get_app_info not ready yet during genesis poll"
                        );
                    }
                }
                tokio::time::sleep(poll_interval).await;
            }

            if !cells_ready {
                warn!(
                    conductor = %conductor.conductor_id,
                    app_id = %installed_app_id,
                    "Cell genesis timed out after {}s — cleaning up",
                    max_polls as f64 * poll_interval.as_secs_f64()
                );
                if let Err(cleanup_err) = admin.uninstall_app(&installed_app_id).await {
                    warn!(
                        "Cleanup uninstall after genesis timeout failed: {}",
                        cleanup_err
                    );
                }
                return Err(format!(
                    "Cell genesis timed out on {} for app '{}' after {}s",
                    conductor.conductor_id,
                    installed_app_id,
                    max_polls as f64 * poll_interval.as_secs_f64()
                ));
            }
        }

        // 7. Register agent→conductor mapping under EVERY string form a reader
        // might hold — canonical first, then the legacy bare encodings a JWT or
        // a pre-canonical Mongo row can still carry. These are lookup aliases
        // for ONE agent, not one agent each: the registry counts capacity by
        // distinct install, so the alias fan-out cannot inflate the count.
        let key_forms = lookup_forms(&agent_key);
        let (canonical_form, alias_forms) = key_forms
            .split_first()
            .expect("lookup_forms always yields at least the canonical form");
        if let Err(e) = self
            .registry
            .register_agent(canonical_form, &conductor.conductor_id, &installed_app_id)
            .await
        {
            warn!(
                "Agent registration failed, attempting uninstall cleanup for {}: {}",
                installed_app_id, e
            );
            if let Err(cleanup_err) = admin.uninstall_app(&installed_app_id).await {
                warn!("Cleanup uninstall also failed: {}", cleanup_err);
            }
            return Err(format!("Failed to register agent mapping: {e}"));
        }

        for alias in alias_forms {
            let _ = self
                .registry
                .register_agent(alias, &conductor.conductor_id, &installed_app_id)
                .await;
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
            admin_url: conductor.admin_url,
            installed_app_id,
        })
    }

    /// Search all conductors for an existing app for this user.
    ///
    /// Since `generate_app_id` includes the conductor_id, we check each conductor
    /// with its own deterministic app ID. Returns the first match found.
    async fn find_existing_app(&self, user_identifier: &str) -> Option<ProvisionedAgent> {
        let conductors = self.registry.list_conductors();
        if conductors.is_empty() {
            return None;
        }

        for conductor in &conductors {
            let app_id = generate_app_id(&self.app_id, &conductor.conductor_id, user_identifier);
            let admin = match TypedAdminClient::connect(&conductor.admin_url).await {
                Ok(a) => a,
                Err(_) => continue,
            };

            match admin.get_app_info(&app_id).await {
                Ok(existing) => {
                    // SAME form as the fresh-provision path above — an
                    // idempotent re-provision must hand back a key the grant
                    // surface accepts, or a returning human's promise leg fails
                    // where a newcomer's succeeds.
                    let agent_pub_key_b64 = provisioned_agent_key(&existing.agent_pub_key);

                    info!(
                        conductor = %conductor.conductor_id,
                        agent = %agent_pub_key_b64,
                        app_id = %app_id,
                        user = %user_identifier,
                        "Reusing existing app installation (idempotent provision)"
                    );

                    // Re-register in case the registry lost the mapping, under
                    // every lookup form (aliases for ONE agent — capacity is
                    // counted by distinct install, never by form count).
                    for form in lookup_forms(&existing.agent_pub_key) {
                        let _ = self
                            .registry
                            .register_agent(&form, &conductor.conductor_id, &app_id)
                            .await;
                    }

                    return Some(ProvisionedAgent {
                        agent_pub_key: agent_pub_key_b64,
                        conductor_id: conductor.conductor_id.clone(),
                        conductor_url: conductor.conductor_url.clone(),
                        admin_url: conductor.admin_url.clone(),
                        installed_app_id: app_id,
                    });
                }
                Err(_) => continue,
            }
        }

        None
    }

    /// Deprovision an agent — uninstall the app from its conductor.
    pub async fn deprovision_agent(&self, agent_pub_key: &str) -> Result<(), String> {
        // Look up conductor assignment
        let entry = self
            .registry
            .get_conductor_for_agent(agent_pub_key)
            .ok_or_else(|| format!("Agent {agent_pub_key} not found in registry"))?;

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

        let admin = TypedAdminClient::connect(&conductor.admin_url)
            .await
            .map_err(|e| {
                format!(
                    "Failed to connect to admin on {}: {}",
                    entry.conductor_id, e
                )
            })?;
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

    format!("{app_id}-{conductor_id}-{short_hash}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two error strings `elohim-genesis/dev` #1576 put in front of a human
    /// as `503 PROVISIONING_FAILED`, verbatim from the build log. Both must
    /// classify as transport faults — the conductor never answered.
    #[test]
    fn the_two_build_1576_refusals_are_transport_faults_not_verdicts() {
        let generate_key = "Failed to generate agent key on conductor-3: Admin error \
             (generate_agent_pub_key): External API wire error: \
             InternalError(\"Other: {\\\"error\\\":\\\"BrokenPipe\\\"}\")";
        let install = "Failed to install app on conductor-3: Admin error (install_app): \
             External API wire error: InternalError(\"Conductor returned an error while \
             using a ConductorApi: Other({\\\"error\\\":\\\"ConnectionAborted\\\"})\")";

        assert!(
            is_transport_fault(generate_key),
            "a BrokenPipe on the admin socket says nothing about the work"
        );
        assert!(
            is_transport_fault(install),
            "a ConnectionAborted on the admin socket says nothing about the work"
        );
    }

    /// The other half of the contract: a real answer must never be retried into
    /// a second side effect.
    #[test]
    fn a_conductor_verdict_is_never_reoffered() {
        for verdict in [
            "Conductor conductor-1 at capacity (50/50)",
            "Cell genesis timed out on conductor-2 for app 'elohim-conductor-2-ab12cd' after 15s",
            "Failed to enable app on conductor-0: AppNotFound",
            "Failed to register agent mapping: mongo write failed",
        ] {
            assert!(
                !is_transport_fault(verdict),
                "a verdict must be returned unchanged, got a retry for: {verdict}"
            );
        }
    }

    /// The exclusion is what makes the re-offer meaningful: without it, the
    /// pool's least-loaded member is handed back on every attempt and the
    /// budget is spent on the one conductor already known to be dropping.
    #[tokio::test]
    async fn a_dropped_conductor_is_excluded_from_the_next_offer() {
        use crate::conductor::registry::ConductorInfo;

        let registry = Arc::new(ConductorRegistry::new(None).await);
        for id in ["conductor-3", "conductor-4"] {
            registry.register_conductor(ConductorInfo {
                conductor_id: id.to_string(),
                conductor_url: format!("ws://{id}:8888"),
                admin_url: format!("ws://{id}:4444"),
                capacity_used: 0,
                capacity_max: 10,
            });
        }

        let first = registry
            .find_least_loaded()
            .expect("a two-member pool has a least-loaded member");
        let second = registry
            .find_least_loaded_excluding(std::slice::from_ref(&first.conductor_id))
            .expect("excluding one member of a two-member pool still leaves one");

        assert_ne!(
            first.conductor_id, second.conductor_id,
            "the re-offer must reach a DIFFERENT conductor"
        );
        assert!(
            registry
                .find_least_loaded_excluding(&[
                    first.conductor_id.clone(),
                    second.conductor_id.clone()
                ])
                .is_none(),
            "excluding every member must yield None, not wrap around"
        );
    }

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
