//! Agent Provisioner — orchestrates Holochain agent provisioning
//!
//! When a user registers, the provisioner:
//! 1. Establishes that the user has no app on ANY conductor of the pool
//! 2. Finds the least loaded conductor
//! 3. Generates a new agent key on that conductor
//! 4. Installs the app with that agent key
//! 5. Enables the app
//! 6. Registers the agent→conductor mapping
//!
//! Every failure of an admin call arrives typed
//! ([`FaultClass`](super::typed_admin::FaultClass)), and what happens next is
//! decided on that type alone: an attempt whose install was never sent may be
//! offered to another conductor; one whose install was sent is settled on the
//! conductor it was sent to. See [`AgentProvisioner::provision_agent`].

use std::sync::Arc;
use tracing::{debug, info, warn};

use super::agent_key::{canonical_agent_key, lookup_forms};
use super::registry::{ConductorInfo, ConductorRegistry};
use super::typed_admin::{AdminConnector, FaultClass, LiveAdminConnector, ProvisionAdmin};

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

/// How many conductors a single provisioning call may be re-offered to when an
/// attempt fails before its install was sent.
///
/// Bounded deliberately: a registration is an interactive request behind a
/// hosted human's sign-up, so the budget is "ask a couple of siblings", never
/// "walk the pool". Each attempt targets a DIFFERENT conductor (the failing one
/// is excluded), so three attempts is three distinct members, not three tries at
/// the same sick one.
const PROVISION_TRANSPORT_ATTEMPTS: u32 = 3;

/// How long a freshly enabled app is given to finish cell genesis.
const GENESIS_MAX_POLLS: u32 = 30;
const GENESIS_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);

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

/// Why provisioning did not produce an agent.
///
/// Two outcomes a caller must be able to tell apart, because they ask
/// different things of the person waiting:
///
/// - [`ProvisionError::Failed`] — nothing was left behind. Trying again is a
///   fresh start.
/// - [`ProvisionError::Unresolved`] — this person may ALREADY have an app on
///   the named conductor(s) and the doorway could not find out. Provisioning
///   them anywhere else would risk a second agent key for one person, so the
///   doorway refuses instead. Trying again later resolves it, once the
///   conductor answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvisionError {
    /// An install may have landed — or an earlier one may exist — on
    /// `conductors`, and that could not be resolved there.
    Unresolved {
        conductors: Vec<String>,
        detail: String,
    },
    /// Provisioning failed and established that nothing was installed.
    Failed(String),
}

impl ProvisionError {
    /// The machine-readable code a route puts in front of a client.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unresolved { .. } => "PROVISIONING_UNRESOLVED",
            Self::Failed(_) => "PROVISIONING_FAILED",
        }
    }

    fn unresolved(conductor_id: &str, detail: impl Into<String>) -> Self {
        Self::Unresolved {
            conductors: vec![conductor_id.to_string()],
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for ProvisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unresolved { conductors, detail } => write!(
                f,
                "provisioning is unresolved on {}: {detail}",
                conductors.join(", ")
            ),
            Self::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ProvisionError {}

/// One failed attempt on one conductor, classed by what it establishes about
/// THE INSTALL — the only step that gives a person an identity.
///
/// `install` is not always the class of the admin call that failed. A key
/// generation whose socket dropped is `SentOutcomeUnknown` as an admin call,
/// but the install was never written, so for the install it is `NotSent`: an
/// orphaned key on a conductor is not an identity. See [`before_install`].
struct AttemptFault {
    install: FaultClass,
    message: String,
}

impl AttemptFault {
    /// The doorway's or the conductor's own answer. Never re-offered.
    fn verdict(message: String) -> Self {
        Self {
            install: FaultClass::Refused,
            message,
        }
    }
}

/// What a fault in a step that runs BEFORE `install_app` establishes about the
/// install: a refusal is still a verdict; any transport fault means the install
/// was never written.
fn before_install(class: FaultClass) -> FaultClass {
    match class {
        FaultClass::Refused => FaultClass::Refused,
        FaultClass::NotSent | FaultClass::SentOutcomeUnknown => FaultClass::NotSent,
    }
}

/// What the search for an existing app established.
enum Existence {
    /// The person already has an app; this is it.
    Found(ProvisionedAgent),
    /// EVERY conductor answered, and none has the app.
    NotInstalled,
    /// No reachable conductor has the app, but these could not be asked.
    /// "Not found" is not a conclusion that is available here.
    Unresolved {
        conductors: Vec<String>,
        detail: String,
    },
}

/// Orchestrates agent provisioning on a conductor.
pub struct AgentProvisioner {
    registry: Arc<ConductorRegistry>,
    app_id: String,
    bundle_path: String,
    connector: Arc<dyn AdminConnector>,
}

impl AgentProvisioner {
    /// Create a new provisioner backed by the given conductor registry.
    pub fn new(registry: Arc<ConductorRegistry>) -> Self {
        Self {
            registry,
            app_id: DEFAULT_APP_ID.to_string(),
            bundle_path: DEFAULT_BUNDLE_PATH.to_string(),
            connector: Arc::new(LiveAdminConnector),
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

    /// Drive the provisioner against a scripted conductor pool.
    #[cfg(test)]
    fn with_connector(mut self, connector: Arc<dyn AdminConnector>) -> Self {
        self.connector = connector;
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
    /// Flow:
    /// 1. Search all conductors for an existing app (idempotency). If any could
    ///    not be asked, stop: see below.
    /// 2. Pick the least loaded conductor not already excluded
    /// 3. Generate agent key, install, enable, register
    ///
    /// ## One person, one agent key
    ///
    /// Each attempt generates a FRESH agent key, and the app id embeds the
    /// conductor id. So "try the next conductor" is only safe when the previous
    /// attempt cannot have installed anything — otherwise one person ends up
    /// with two agent keys on two conductors, and nothing reconciles them.
    ///
    /// The rule, decided on the typed [`FaultClass`] of the INSTALL:
    ///
    /// - `NotSent` — the install was never written (the connect failed, or key
    ///   generation dropped). Re-offer to another conductor, bounded at
    ///   [`PROVISION_TRANSPORT_ATTEMPTS`], excluding the one that dropped.
    /// - `SentOutcomeUnknown` — the install was written and its outcome is not
    ///   known. Resolve it on the SAME conductor: if the app is there, finish
    ///   it and use it; if it is not, or the conductor cannot be asked, fail
    ///   with [`ProvisionError::Unresolved`]. Never a second key elsewhere.
    /// - `Refused` — the conductor (or the doorway's own capacity check)
    ///   answered. Returned unchanged; a verdict is not retried into a second
    ///   side effect.
    ///
    /// The same reasoning binds the search in step 1: a conductor that cannot
    /// be reached might hold this person's app, so "found nothing" is only a
    /// conclusion when every conductor answered.
    ///
    /// ## History: a dropped admin socket is not a provisioning verdict
    ///
    /// `elohim-genesis/dev` #1576 refused two hosted registrations on a
    /// seven-peer-healthy alpha with
    /// `Failed to generate agent key on conductor-3: … Other: {"error":"BrokenPipe"}`
    /// and
    /// `Failed to install app on conductor-3: … Other({"error":"ConnectionAborted"})`,
    /// each surfacing to the human as `503 PROVISIONING_FAILED`. The first is
    /// still re-offered (no install was sent). The second no longer is: an
    /// install that was sent is resolved where it was sent.
    pub async fn provision_agent(
        &self,
        user_identifier: &str,
    ) -> Result<ProvisionedAgent, ProvisionError> {
        // 1. Search ALL conductors for an existing app for this user — ONCE.
        // Not per attempt: the only thing a later attempt could newly find is
        // an install this call sent, and that is resolved on its own conductor
        // below rather than rediscovered by a pool walk.
        match self.find_existing_app(user_identifier).await {
            Existence::Found(agent) => return Ok(agent),
            Existence::Unresolved { conductors, detail } => {
                warn!(
                    user = %user_identifier,
                    unreachable = ?conductors,
                    "Cannot establish whether this user already has an app — refusing to \
                     provision a second one elsewhere"
                );
                return Err(ProvisionError::Unresolved { conductors, detail });
            }
            Existence::NotInstalled => {}
        }

        let mut excluded: Vec<String> = Vec::new();
        let mut last_not_sent: Option<String> = None;

        for attempt in 1..=PROVISION_TRANSPORT_ATTEMPTS {
            // 2. Provision on the least loaded conductor that has not already
            // dropped on us in this call.
            let Some(conductor) = self.registry.find_least_loaded_excluding(&excluded) else {
                break;
            };

            let fault = match self.provision_on(&conductor, user_identifier).await {
                Ok(agent) => return Ok(agent),
                Err(fault) => fault,
            };

            match fault.install {
                FaultClass::NotSent => {
                    warn!(
                        conductor = %conductor.conductor_id,
                        attempt,
                        max_attempts = PROVISION_TRANSPORT_ATTEMPTS,
                        error = %fault.message,
                        "provisioning attempt failed before its install was sent — \
                         re-offering to another conductor"
                    );
                    excluded.push(conductor.conductor_id.clone());
                    last_not_sent = Some(fault.message);
                }
                FaultClass::SentOutcomeUnknown => {
                    warn!(
                        conductor = %conductor.conductor_id,
                        error = %fault.message,
                        "install was sent and its outcome is unknown — resolving on the \
                         SAME conductor; it will not be re-offered"
                    );
                    return self
                        .resolve_sent_install(&conductor, user_identifier, &fault.message)
                        .await;
                }
                FaultClass::Refused => return Err(ProvisionError::Failed(fault.message)),
            }
        }

        Err(ProvisionError::Failed(last_not_sent.unwrap_or_else(|| {
            "No conductors available for provisioning".to_string()
        })))
    }

    /// One provisioning attempt against ONE named conductor.
    async fn provision_on(
        &self,
        conductor: &ConductorInfo,
        user_identifier: &str,
    ) -> Result<ProvisionedAgent, AttemptFault> {
        if conductor.capacity_used >= conductor.capacity_max {
            return Err(AttemptFault::verdict(format!(
                "Conductor {} at capacity ({}/{})",
                conductor.conductor_id, conductor.capacity_used, conductor.capacity_max
            )));
        }

        let installed_app_id =
            generate_app_id(&self.app_id, &conductor.conductor_id, user_identifier);
        let admin = self
            .connector
            .connect(&conductor.admin_url)
            .await
            .map_err(|fault| AttemptFault {
                install: before_install(fault.class),
                message: format!(
                    "Failed to connect to admin on {}: {}",
                    conductor.conductor_id, fault
                ),
            })?;

        info!(
            conductor = %conductor.conductor_id,
            admin_url = %conductor.admin_url,
            installed_app_id = %installed_app_id,
            user = %user_identifier,
            "Provisioning new agent on conductor"
        );

        // 3. Generate agent key. Nothing identity-bearing exists until the
        // install below, so a transport fault here leaves the install unsent.
        let agent_key = admin
            .generate_agent_pub_key()
            .await
            .map_err(|fault| AttemptFault {
                install: before_install(fault.class),
                message: format!(
                    "Failed to generate agent key on {}: {}",
                    conductor.conductor_id, fault
                ),
            })?;

        // 4. Install app. From the moment this is written, the person may have
        // an identity on THIS conductor — the fault's own class is the answer.
        if let Err(fault) = admin
            .install_app(&installed_app_id, &agent_key, &self.bundle_path)
            .await
        {
            return Err(AttemptFault {
                install: fault.class,
                message: format!(
                    "Failed to install app on {}: {}",
                    conductor.conductor_id, fault
                ),
            });
        }

        // 5. Enable app. The app IS installed here now.
        if let Err(fault) = admin.enable_app(&installed_app_id).await {
            let message = format!(
                "Failed to enable app on {}: {}",
                conductor.conductor_id, fault
            );
            return Err(match fault.class {
                // The conductor answered: clean up what we just installed.
                FaultClass::Refused => {
                    warn!(
                        "Enable failed, attempting uninstall cleanup for {}: {}",
                        installed_app_id, fault
                    );
                    if let Err(cleanup_err) = admin.uninstall_app(&installed_app_id).await {
                        warn!("Cleanup uninstall also failed: {}", cleanup_err);
                    }
                    AttemptFault::verdict(message)
                }
                // The socket went: an installed app in an unknown state. No
                // cleanup over a dead socket, and above all no second install.
                FaultClass::NotSent | FaultClass::SentOutcomeUnknown => AttemptFault {
                    install: FaultClass::SentOutcomeUnknown,
                    message,
                },
            });
        }

        // 6. Verify cells are ready before returning (cell genesis can take seconds)
        if !self
            .await_cells(&*admin, conductor, &installed_app_id)
            .await
        {
            let waited = f64::from(GENESIS_MAX_POLLS) * GENESIS_POLL_INTERVAL.as_secs_f64();
            warn!(
                conductor = %conductor.conductor_id,
                app_id = %installed_app_id,
                "Cell genesis timed out after {}s — cleaning up",
                waited
            );
            if let Err(cleanup_err) = admin.uninstall_app(&installed_app_id).await {
                warn!(
                    "Cleanup uninstall after genesis timeout failed: {}",
                    cleanup_err
                );
            }
            return Err(AttemptFault::verdict(format!(
                "Cell genesis timed out on {} for app '{}' after {}s",
                conductor.conductor_id, installed_app_id, waited
            )));
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
            return Err(AttemptFault::verdict(format!(
                "Failed to register agent mapping: {e}"
            )));
        }

        for alias in alias_forms {
            let _ = self
                .registry
                .register_agent(alias, &conductor.conductor_id, &installed_app_id)
                .await;
        }

        let agent_pub_key_b64 = provisioned_agent_key(&agent_key);
        info!(
            conductor = %conductor.conductor_id,
            agent = %agent_pub_key_b64,
            app_id = %installed_app_id,
            "Agent provisioned successfully"
        );

        Ok(ProvisionedAgent {
            agent_pub_key: agent_pub_key_b64,
            conductor_id: conductor.conductor_id.clone(),
            conductor_url: conductor.conductor_url.clone(),
            admin_url: conductor.admin_url.clone(),
            installed_app_id,
        })
    }

    /// Poll until the app reports at least one provisioned cell, or the genesis
    /// budget runs out. `true` = cells ready.
    async fn await_cells(
        &self,
        admin: &dyn ProvisionAdmin,
        conductor: &ConductorInfo,
        installed_app_id: &str,
    ) -> bool {
        for attempt in 1..=GENESIS_MAX_POLLS {
            match admin.find_app(installed_app_id).await {
                Ok(Some(info)) if !info.cell_ids.is_empty() => {
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
                    return true;
                }
                Ok(_) => {
                    debug!(
                        app_id = %installed_app_id,
                        attempt,
                        max_polls = GENESIS_MAX_POLLS,
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
            tokio::time::sleep(GENESIS_POLL_INTERVAL).await;
        }
        false
    }

    /// An install was written to `conductor` and its outcome is unknown.
    /// Settle it THERE.
    ///
    /// Three things can be true, and only one of them yields an agent:
    ///
    /// - the app is installed → finish it (enable if needed, wait for cells)
    ///   and use it, with the agent key the CONDUCTOR reports for it;
    /// - the conductor answers that it has no such app → still unresolved. A
    ///   conductor does not cancel an install because its client went away, so
    ///   "not there yet" is not "never will be"; a retry finds it once it lands;
    /// - the conductor cannot be asked → unresolved.
    ///
    /// Never uninstalls: by the time anyone looks again, `create_human` may
    /// have run on this app.
    async fn resolve_sent_install(
        &self,
        conductor: &ConductorInfo,
        user_identifier: &str,
        sent_fault: &str,
    ) -> Result<ProvisionedAgent, ProvisionError> {
        let conductor_id = conductor.conductor_id.as_str();
        let installed_app_id = generate_app_id(&self.app_id, conductor_id, user_identifier);

        let admin = self
            .connector
            .connect(&conductor.admin_url)
            .await
            .map_err(|fault| {
                ProvisionError::unresolved(
                    conductor_id,
                    format!(
                        "{sent_fault}; the install of '{installed_app_id}' may have landed and \
                         {conductor_id} could not be reached to find out ({fault})"
                    ),
                )
            })?;

        let existing = match admin.find_app(&installed_app_id).await {
            Ok(Some(existing)) => existing,
            Ok(None) => {
                return Err(ProvisionError::unresolved(
                    conductor_id,
                    format!(
                        "{sent_fault}; '{installed_app_id}' is not installed on {conductor_id} \
                         yet, but the install was sent and may still land"
                    ),
                ));
            }
            Err(fault) => {
                return Err(ProvisionError::unresolved(
                    conductor_id,
                    format!(
                        "{sent_fault}; the install of '{installed_app_id}' may have landed and \
                         {conductor_id} did not answer the lookup ({fault})"
                    ),
                ));
            }
        };

        info!(
            conductor = %conductor_id,
            app_id = %installed_app_id,
            status = ?existing.status,
            "The install whose outcome was unknown DID land — finishing it in place"
        );

        // Installed is not ready. Check what state it is in.
        match existing.status.as_deref() {
            Some("running") => {}
            Some("disabled") => {
                admin.enable_app(&installed_app_id).await.map_err(|fault| {
                    ProvisionError::unresolved(
                        conductor_id,
                        format!(
                            "'{installed_app_id}' is installed on {conductor_id} but disabled, \
                             and could not be enabled ({fault})"
                        ),
                    )
                })?;
            }
            other => {
                return Err(ProvisionError::unresolved(
                    conductor_id,
                    format!(
                        "'{installed_app_id}' is installed on {conductor_id} in state {} and \
                         cannot be used as it stands",
                        other.unwrap_or("unknown")
                    ),
                ));
            }
        }

        if existing.cell_ids.is_empty()
            && !self
                .await_cells(&*admin, conductor, &installed_app_id)
                .await
        {
            return Err(ProvisionError::unresolved(
                conductor_id,
                format!(
                    "'{installed_app_id}' is installed on {conductor_id} but its cells did not \
                     become ready"
                ),
            ));
        }

        Ok(self
            .adopt_existing(conductor, installed_app_id, &existing.agent_pub_key)
            .await)
    }

    /// Hand back an app that already exists on `conductor`, registering its
    /// agent under every lookup form.
    async fn adopt_existing(
        &self,
        conductor: &ConductorInfo,
        installed_app_id: String,
        raw_agent_key: &[u8],
    ) -> ProvisionedAgent {
        // SAME form as the fresh-provision path — an idempotent re-provision
        // must hand back a key the grant surface accepts, or a returning
        // human's promise leg fails where a newcomer's succeeds.
        let agent_pub_key_b64 = provisioned_agent_key(raw_agent_key);

        // Re-register in case the registry lost the mapping, under every lookup
        // form (aliases for ONE agent — capacity is counted by distinct
        // install, never by form count).
        for form in lookup_forms(raw_agent_key) {
            let _ = self
                .registry
                .register_agent(&form, &conductor.conductor_id, &installed_app_id)
                .await;
        }

        ProvisionedAgent {
            agent_pub_key: agent_pub_key_b64,
            conductor_id: conductor.conductor_id.clone(),
            conductor_url: conductor.conductor_url.clone(),
            admin_url: conductor.admin_url.clone(),
            installed_app_id,
        }
    }

    /// Search all conductors for an existing app for this user.
    ///
    /// Since `generate_app_id` includes the conductor_id, we check each conductor
    /// with its own deterministic app ID. Returns the first match found.
    ///
    /// A conductor that cannot be reached, or does not answer the lookup, is
    /// NOT a conductor without the app. It used to be skipped, which let "no
    /// existing app" be concluded from a pool that had not all been asked —
    /// and a person whose app sat on the silent conductor was then given a
    /// second one. Now it is reported: [`Existence::Unresolved`].
    async fn find_existing_app(&self, user_identifier: &str) -> Existence {
        let mut unreachable: Vec<String> = Vec::new();
        let mut reasons: Vec<String> = Vec::new();

        for conductor in &self.registry.list_conductors() {
            let app_id = generate_app_id(&self.app_id, &conductor.conductor_id, user_identifier);

            let lookup = match self.connector.connect(&conductor.admin_url).await {
                Ok(admin) => admin.find_app(&app_id).await.map(|found| (admin, found)),
                Err(fault) => Err(fault),
            };

            match lookup {
                Ok((admin, Some(existing))) => {
                    info!(
                        conductor = %conductor.conductor_id,
                        app_id = %app_id,
                        user = %user_identifier,
                        status = ?existing.status,
                        "Reusing existing app installation (idempotent provision)"
                    );

                    // An app left disabled is usually an install whose enable
                    // never arrived. Enable it rather than hand back an app no
                    // zome call can reach. Best-effort: on failure it is handed
                    // back as it always was, and the caller's own readiness
                    // handling (the chaperone re-enables and polls) still runs.
                    if existing.status.as_deref() == Some("disabled") {
                        if let Err(fault) = admin.enable_app(&app_id).await {
                            warn!(
                                conductor = %conductor.conductor_id,
                                app_id = %app_id,
                                error = %fault,
                                "Existing app is disabled and could not be enabled"
                            );
                        }
                    }

                    return Existence::Found(
                        self.adopt_existing(conductor, app_id, &existing.agent_pub_key)
                            .await,
                    );
                }
                Ok((_, None)) => {}
                Err(fault) => {
                    warn!(
                        conductor = %conductor.conductor_id,
                        error = %fault,
                        "Could not ask this conductor whether the user already has an app"
                    );
                    reasons.push(format!("{}: {}", conductor.conductor_id, fault));
                    unreachable.push(conductor.conductor_id.clone());
                }
            }
        }

        if unreachable.is_empty() {
            Existence::NotInstalled
        } else {
            unreachable.sort();
            Existence::Unresolved {
                conductors: unreachable,
                detail: format!(
                    "could not establish whether this user already has an app there ({})",
                    reasons.join("; ")
                ),
            }
        }
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

        let admin = self
            .connector
            .connect(&conductor.admin_url)
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
    use crate::conductor::typed_admin::{AdminFault, AppInfoDetailed};
    use async_trait::async_trait;
    use std::collections::{HashMap, VecDeque};
    use std::sync::Mutex;

    // ── a scripted conductor pool ───────────────────────────────────────────
    //
    // Stands in for the admin websockets of a pool, behind the same
    // `AdminConnector` / `ProvisionAdmin` seam the live client implements. Each
    // conductor counts what reached it, so a test can assert not only what the
    // provisioner returned but what it DID — above all, how many agent keys it
    // generated and where it sent an install.

    const USER: &str = "jane@example.com";

    /// What a conductor does with an `install_app`.
    #[derive(Clone)]
    enum Install {
        /// Installs (disabled, until enabled) and answers.
        Succeeds,
        /// Answers with an error; nothing is installed.
        Refuses(&'static str),
        /// The request is written, then the socket drops. `lands` is the state
        /// the app is in afterwards on the conductor — `None` if it has not
        /// (yet) been installed.
        SentThenDropped { lands: Option<&'static str> },
    }

    struct FakeApp {
        agent_key: Vec<u8>,
        status: &'static str,
        cells_ready: bool,
    }

    struct FakeConductor {
        /// Scripted connect outcomes (`true` = accepted), consumed in order.
        /// Once exhausted, connects succeed unless `dark`.
        connects: VecDeque<bool>,
        /// Refuses every connect.
        dark: bool,
        /// Goes dark the moment an install is written to it.
        dark_after_install: bool,
        keygen_fault: Option<AdminFault>,
        install: Install,
        /// Returned by the NEXT `enable_app`, once.
        enable_fault: Option<AdminFault>,
        /// Returned by every `find_app`.
        lookup_fault: Option<AdminFault>,
        apps: HashMap<String, FakeApp>,
        keys_generated: u32,
        installs_received: u32,
        enables_received: u32,
        uninstalls_received: u32,
    }

    impl FakeConductor {
        fn healthy() -> Self {
            Self {
                connects: VecDeque::new(),
                dark: false,
                dark_after_install: false,
                keygen_fault: None,
                install: Install::Succeeds,
                enable_fault: None,
                lookup_fault: None,
                apps: HashMap::new(),
                keys_generated: 0,
                installs_received: 0,
                enables_received: 0,
                uninstalls_received: 0,
            }
        }
    }

    #[derive(Default)]
    struct FakePool {
        conductors: Mutex<HashMap<String, FakeConductor>>,
        keys_minted: Mutex<u8>,
    }

    impl FakePool {
        fn with<R>(&self, conductor_id: &str, f: impl FnOnce(&mut FakeConductor) -> R) -> R {
            let mut conductors = self.conductors.lock().unwrap();
            f(conductors
                .get_mut(&admin_url(conductor_id))
                .expect("a conductor of this pool"))
        }

        fn total_keys_generated(&self) -> u32 {
            self.conductors
                .lock()
                .unwrap()
                .values()
                .map(|c| c.keys_generated)
                .sum()
        }

        fn total_installs_received(&self) -> u32 {
            self.conductors
                .lock()
                .unwrap()
                .values()
                .map(|c| c.installs_received)
                .sum()
        }
    }

    fn admin_url(conductor_id: &str) -> String {
        format!("ws://{conductor_id}:4444")
    }

    fn raw_agent_key(seed: u8) -> Vec<u8> {
        holochain_client::AgentPubKey::from_raw_36(vec![seed; 36])
            .get_raw_39()
            .to_vec()
    }

    struct FakeConnector(Arc<FakePool>);

    struct FakeAdmin {
        pool: Arc<FakePool>,
        admin_url: String,
    }

    impl FakeAdmin {
        fn with<R>(&self, f: impl FnOnce(&mut FakeConductor) -> R) -> R {
            let mut conductors = self.pool.conductors.lock().unwrap();
            f(conductors
                .get_mut(&self.admin_url)
                .expect("a conductor of this pool"))
        }
    }

    #[async_trait]
    impl AdminConnector for FakeConnector {
        async fn connect(&self, admin_url: &str) -> Result<Box<dyn ProvisionAdmin>, AdminFault> {
            let accepted = {
                let mut conductors = self.0.conductors.lock().unwrap();
                let conductor = conductors
                    .get_mut(admin_url)
                    .expect("a conductor of this pool");
                !conductor.dark && conductor.connects.pop_front().unwrap_or(true)
            };
            if !accepted {
                return Err(AdminFault::not_sent(format!(
                    "Admin WebSocket connect to {admin_url} failed: IO error: Connection \
                     refused (os error 111)"
                )));
            }
            Ok(Box::new(FakeAdmin {
                pool: Arc::clone(&self.0),
                admin_url: admin_url.to_string(),
            }))
        }
    }

    #[async_trait]
    impl ProvisionAdmin for FakeAdmin {
        async fn generate_agent_pub_key(&self) -> Result<Vec<u8>, AdminFault> {
            if let Some(fault) = self.with(|c| c.keygen_fault.clone()) {
                return Err(fault);
            }
            let seed = {
                let mut minted = self.pool.keys_minted.lock().unwrap();
                *minted += 1;
                *minted
            };
            self.with(|c| c.keys_generated += 1);
            Ok(raw_agent_key(seed))
        }

        async fn install_app(
            &self,
            installed_app_id: &str,
            agent_key: &[u8],
            _bundle_path: &str,
        ) -> Result<(), AdminFault> {
            self.with(|c| {
                c.installs_received += 1;
                let land = |status: &'static str, c: &mut FakeConductor| {
                    c.apps.insert(
                        installed_app_id.to_string(),
                        FakeApp {
                            agent_key: agent_key.to_vec(),
                            status,
                            cells_ready: status == "running",
                        },
                    );
                };
                match c.install.clone() {
                    Install::Succeeds => {
                        land("disabled", c);
                        Ok(())
                    }
                    Install::Refuses(why) => Err(AdminFault::refused(format!(
                        "Admin error (install_app): External API wire error: \
                         InternalError(\"{why}\")"
                    ))),
                    Install::SentThenDropped { lands } => {
                        if let Some(status) = lands {
                            land(status, c);
                        }
                        if c.dark_after_install {
                            c.dark = true;
                        }
                        Err(AdminFault::sent_outcome_unknown(
                            "Admin error (install_app): Websocket error: Websocket closed: \
                             ConnectionClosed",
                        ))
                    }
                }
            })
        }

        async fn enable_app(&self, installed_app_id: &str) -> Result<(), AdminFault> {
            self.with(|c| {
                c.enables_received += 1;
                if let Some(fault) = c.enable_fault.take() {
                    return Err(fault);
                }
                match c.apps.get_mut(installed_app_id) {
                    Some(app) => {
                        app.status = "running";
                        app.cells_ready = true;
                        Ok(())
                    }
                    None => Err(AdminFault::refused(
                        "Admin error (enable_app): External API wire error: AppNotFound",
                    )),
                }
            })
        }

        async fn find_app(
            &self,
            installed_app_id: &str,
        ) -> Result<Option<AppInfoDetailed>, AdminFault> {
            self.with(|c| {
                if let Some(fault) = c.lookup_fault.clone() {
                    return Err(fault);
                }
                Ok(c.apps.get(installed_app_id).map(|app| AppInfoDetailed {
                    installed_app_id: installed_app_id.to_string(),
                    agent_pub_key: app.agent_key.clone(),
                    cell_ids: if app.cells_ready {
                        vec![(
                            "imagodei".to_string(),
                            (vec![0u8; 39], app.agent_key.clone()),
                        )]
                    } else {
                        Vec::new()
                    },
                    status: Some(app.status.to_string()),
                    unrecoverable_cell_id: None,
                    unrecoverable_reason: None,
                }))
            })
        }

        async fn uninstall_app(&self, installed_app_id: &str) -> Result<(), AdminFault> {
            self.with(|c| {
                c.uninstalls_received += 1;
                c.apps.remove(installed_app_id);
                Ok(())
            })
        }
    }

    /// A two-conductor pool where `conductor-a` is the least loaded — so it is
    /// always the first offer — and `conductor-b` is the sibling a wrongly
    /// re-offered install would land on.
    async fn two_conductor_pool() -> (AgentProvisioner, Arc<FakePool>) {
        let registry = Arc::new(ConductorRegistry::new(None).await);
        let pool = Arc::new(FakePool::default());
        for (id, used) in [("conductor-a", 0), ("conductor-b", 5)] {
            registry.register_conductor(ConductorInfo {
                conductor_id: id.to_string(),
                conductor_url: format!("ws://{id}:8888"),
                admin_url: admin_url(id),
                capacity_used: used,
                capacity_max: 10,
            });
            pool.conductors
                .lock()
                .unwrap()
                .insert(admin_url(id), FakeConductor::healthy());
        }
        let provisioner = AgentProvisioner::new(registry)
            .with_connector(Arc::new(FakeConnector(Arc::clone(&pool))));
        (provisioner, pool)
    }

    fn app_on(conductor_id: &str) -> String {
        generate_app_id(DEFAULT_APP_ID, conductor_id, USER)
    }

    fn assert_untouched(pool: &FakePool, conductor_id: &str) {
        pool.with(conductor_id, |c| {
            assert_eq!(
                c.keys_generated, 0,
                "{conductor_id} must not have been asked for an agent key"
            );
            assert_eq!(
                c.installs_received, 0,
                "{conductor_id} must not have been sent an install"
            );
            assert!(c.apps.is_empty());
        });
    }

    fn unresolved_on(error: &ProvisionError) -> Vec<String> {
        match error {
            ProvisionError::Unresolved { conductors, .. } => conductors.clone(),
            ProvisionError::Failed(message) => {
                panic!("expected an unresolved result, got a plain failure: {message}")
            }
        }
    }

    // ── the happy path, through the seam ────────────────────────────────────

    #[tokio::test]
    async fn a_healthy_pool_provisions_once_on_the_least_loaded_conductor() {
        let (provisioner, pool) = two_conductor_pool().await;

        let agent = provisioner.provision_agent(USER).await.expect("provisions");

        assert_eq!(agent.conductor_id, "conductor-a");
        assert_eq!(agent.installed_app_id, app_on("conductor-a"));
        assert_eq!(pool.total_keys_generated(), 1);
        assert_untouched(&pool, "conductor-b");

        // …and asking again finds it instead of installing a second.
        let again = provisioner.provision_agent(USER).await.expect("idempotent");
        assert_eq!(again.agent_pub_key, agent.agent_pub_key);
        assert_eq!(pool.total_keys_generated(), 1);
        assert_eq!(pool.total_installs_received(), 1);
    }

    // ── never a second identity for one person ──────────────────────────────

    /// THE CASE. The install is written to conductor-a and the socket drops.
    /// It landed. The provisioner must finish THAT install — one agent key in
    /// the whole pool, nothing sent to the sibling — and hand back the key the
    /// conductor holds for it.
    #[tokio::test]
    async fn an_install_sent_then_dropped_is_finished_on_the_same_conductor() {
        let (provisioner, pool) = two_conductor_pool().await;
        pool.with("conductor-a", |c| {
            c.install = Install::SentThenDropped {
                lands: Some("disabled"),
            }
        });

        let agent = provisioner
            .provision_agent(USER)
            .await
            .expect("an install that landed must be used, not abandoned");

        assert_eq!(agent.conductor_id, "conductor-a");
        assert_eq!(
            pool.total_keys_generated(),
            1,
            "no second generate_agent_pub_key — one person, one agent key"
        );
        assert_untouched(&pool, "conductor-b");
        pool.with("conductor-a", |c| {
            let app = &c.apps[&app_on("conductor-a")];
            assert_eq!(
                agent.agent_pub_key,
                provisioned_agent_key(&app.agent_key),
                "the agent handed back must be the one the conductor installed"
            );
            assert_eq!(app.status, "running", "a disabled app is not ready");
            assert_eq!(c.enables_received, 1);
            assert_eq!(c.installs_received, 1);
            assert_eq!(c.uninstalls_received, 0);
        });
    }

    /// Same drop, but the install is not visible on the conductor when we look.
    /// A conductor does not cancel an install because its client went away, so
    /// this is NOT "nothing happened": the answer is an honest unresolved, and
    /// still no second key and no install anywhere else.
    #[tokio::test]
    async fn an_install_sent_then_dropped_that_is_not_visible_is_unresolved_not_reoffered() {
        let (provisioner, pool) = two_conductor_pool().await;
        pool.with("conductor-a", |c| {
            c.install = Install::SentThenDropped { lands: None }
        });

        let error = provisioner
            .provision_agent(USER)
            .await
            .expect_err("an install of unknown outcome must not be papered over");

        assert_eq!(unresolved_on(&error), vec!["conductor-a".to_string()]);
        assert_eq!(error.code(), "PROVISIONING_UNRESOLVED");
        assert_eq!(pool.total_keys_generated(), 1);
        assert_eq!(pool.total_installs_received(), 1);
        assert_untouched(&pool, "conductor-b");
    }

    /// The install is written, lands, and the conductor then stops answering
    /// entirely. It cannot be resolved there — so registration fails saying
    /// exactly that, rather than quietly minting a second identity next door.
    #[tokio::test]
    async fn an_install_sent_to_a_conductor_that_then_goes_dark_is_unresolved_not_reoffered() {
        let (provisioner, pool) = two_conductor_pool().await;
        pool.with("conductor-a", |c| {
            c.install = Install::SentThenDropped {
                lands: Some("running"),
            };
            c.dark_after_install = true;
        });

        let error = provisioner
            .provision_agent(USER)
            .await
            .expect_err("an unreachable conductor cannot resolve its own install");

        assert_eq!(unresolved_on(&error), vec!["conductor-a".to_string()]);
        assert!(
            error.to_string().contains("conductor-a"),
            "the error must name the conductor the identity may be on: {error}"
        );
        assert_eq!(pool.total_keys_generated(), 1);
        assert_untouched(&pool, "conductor-b");
    }

    /// The install answers OK and the socket drops on the ENABLE. The app is
    /// installed there: finish it in place. No uninstall over a dead socket,
    /// no second install.
    #[tokio::test]
    async fn an_enable_whose_socket_drops_is_finished_in_place() {
        let (provisioner, pool) = two_conductor_pool().await;
        pool.with("conductor-a", |c| {
            c.enable_fault = Some(AdminFault::sent_outcome_unknown(
                "Admin error (enable_app): Websocket error: Websocket closed: ConnectionClosed",
            ))
        });

        let agent = provisioner
            .provision_agent(USER)
            .await
            .expect("an installed app must be finished, not re-provisioned");

        assert_eq!(agent.conductor_id, "conductor-a");
        assert_eq!(pool.total_keys_generated(), 1);
        assert_untouched(&pool, "conductor-b");
        pool.with("conductor-a", |c| {
            assert_eq!(
                c.enables_received, 2,
                "the dropped enable, then the real one"
            );
            assert_eq!(c.uninstalls_received, 0);
        });
    }

    /// A landed install that is not usable is REPORTED — never treated as
    /// ready, never uninstalled (create_human may already have run on it by
    /// the time anyone looks), never replaced elsewhere.
    #[tokio::test]
    async fn a_landed_install_in_an_unusable_state_is_reported_not_used() {
        for state in ["unrecoverable", "paused"] {
            let (provisioner, pool) = two_conductor_pool().await;
            pool.with("conductor-a", |c| {
                c.install = Install::SentThenDropped { lands: Some(state) }
            });

            let error = provisioner.provision_agent(USER).await.expect_err(state);

            assert_eq!(unresolved_on(&error), vec!["conductor-a".to_string()]);
            assert!(error.to_string().contains(state), "{error}");
            pool.with("conductor-a", |c| assert_eq!(c.uninstalls_received, 0));
            assert_untouched(&pool, "conductor-b");
        }

        // Disabled, and the enable is refused: reported, not used.
        let (provisioner, pool) = two_conductor_pool().await;
        pool.with("conductor-a", |c| {
            c.install = Install::SentThenDropped {
                lands: Some("disabled"),
            };
            c.enable_fault = Some(AdminFault::refused(
                "Admin error (enable_app): External API wire error: InternalError(\"CellDisabled\")",
            ));
        });
        let error = provisioner
            .provision_agent(USER)
            .await
            .expect_err("a disabled app that cannot be enabled is not ready");
        assert_eq!(unresolved_on(&error), vec!["conductor-a".to_string()]);
        pool.with("conductor-a", |c| assert_eq!(c.uninstalls_received, 0));
        assert_untouched(&pool, "conductor-b");
    }

    // ── what MAY be re-offered ──────────────────────────────────────────────

    /// The connect is refused before anything is written. Nothing reached
    /// conductor-a, so the sibling is asked — the re-offer still happens.
    #[tokio::test]
    async fn a_connect_refused_before_anything_is_sent_is_reoffered() {
        let (provisioner, pool) = two_conductor_pool().await;
        // Answers the existence check, then refuses the provisioning connect.
        pool.with("conductor-a", |c| {
            c.connects = VecDeque::from([true, false])
        });

        let agent = provisioner
            .provision_agent(USER)
            .await
            .expect("a connect that never sent anything must be re-offered");

        assert_eq!(agent.conductor_id, "conductor-b");
        assert_untouched(&pool, "conductor-a");
        assert_eq!(pool.total_keys_generated(), 1);
    }

    /// `elohim-genesis/dev` #1576: key generation fails with a dropped pipe.
    /// As an admin call its outcome is unknown — but no install was written, so
    /// for the install it is `NotSent`, and the sibling is asked.
    #[tokio::test]
    async fn a_key_generation_that_drops_is_reoffered_because_no_install_was_sent() {
        let (provisioner, pool) = two_conductor_pool().await;
        pool.with("conductor-a", |c| {
            c.keygen_fault = Some(AdminFault::sent_outcome_unknown(
                "Admin error (generate_agent_pub_key): External API wire error: \
                 InternalError(\"Other: {\\\"error\\\":\\\"BrokenPipe\\\"}\")",
            ))
        });

        let agent = provisioner
            .provision_agent(USER)
            .await
            .expect("a dropped key generation says nothing about the work");

        assert_eq!(agent.conductor_id, "conductor-b");
        pool.with("conductor-a", |c| assert_eq!(c.installs_received, 0));
        assert_eq!(pool.total_installs_received(), 1);
    }

    #[test]
    fn a_fault_before_the_install_never_makes_the_install_unknown() {
        assert_eq!(before_install(FaultClass::NotSent), FaultClass::NotSent);
        assert_eq!(
            before_install(FaultClass::SentOutcomeUnknown),
            FaultClass::NotSent
        );
        assert_eq!(before_install(FaultClass::Refused), FaultClass::Refused);
    }

    // ── verdicts ────────────────────────────────────────────────────────────

    /// The other half of the contract: a real answer must never be retried into
    /// a second side effect.
    #[tokio::test]
    async fn a_refused_install_is_a_verdict_and_is_not_reoffered() {
        let (provisioner, pool) = two_conductor_pool().await;
        pool.with("conductor-a", |c| {
            c.install = Install::Refuses("bundle not found at /app/elohim.happ")
        });

        let error = provisioner
            .provision_agent(USER)
            .await
            .expect_err("a refusal is an answer");

        assert_eq!(error.code(), "PROVISIONING_FAILED");
        assert!(
            error
                .to_string()
                .starts_with("Failed to install app on conductor-a: "),
            "{error}"
        );
        assert_untouched(&pool, "conductor-b");
    }

    /// A refused key generation is a verdict too — class decides, not stage.
    #[tokio::test]
    async fn a_refused_key_generation_is_a_verdict_and_is_not_reoffered() {
        let (provisioner, pool) = two_conductor_pool().await;
        pool.with("conductor-a", |c| {
            c.keygen_fault = Some(AdminFault::refused(
                "Admin error (generate_agent_pub_key): External API wire error: \
                 InternalError(\"keystore locked\")",
            ))
        });

        let error = provisioner
            .provision_agent(USER)
            .await
            .expect_err("verdict");

        assert!(matches!(error, ProvisionError::Failed(_)), "{error}");
        assert_untouched(&pool, "conductor-b");
    }

    /// The doorway's own capacity check is a verdict as well.
    #[tokio::test]
    async fn a_full_pool_is_a_verdict() {
        let registry = Arc::new(ConductorRegistry::new(None).await);
        let pool = Arc::new(FakePool::default());
        registry.register_conductor(ConductorInfo {
            conductor_id: "conductor-a".to_string(),
            conductor_url: "ws://conductor-a:8888".to_string(),
            admin_url: admin_url("conductor-a"),
            capacity_used: 10,
            capacity_max: 10,
        });
        pool.conductors
            .lock()
            .unwrap()
            .insert(admin_url("conductor-a"), FakeConductor::healthy());
        let provisioner = AgentProvisioner::new(registry)
            .with_connector(Arc::new(FakeConnector(Arc::clone(&pool))));

        let error = provisioner.provision_agent(USER).await.expect_err("full");

        assert_eq!(
            error,
            ProvisionError::Failed("Conductor conductor-a at capacity (10/10)".to_string())
        );
        assert_eq!(pool.total_keys_generated(), 0);
    }

    // ── the existence check ─────────────────────────────────────────────────

    /// A conductor that cannot be reached is not a conductor without the app.
    /// The search says so explicitly, and provisioning stops: no key, no
    /// install, anywhere.
    #[tokio::test]
    async fn an_unreachable_conductor_during_the_existence_check_is_unresolved_not_absent() {
        let (provisioner, pool) = two_conductor_pool().await;
        pool.with("conductor-b", |c| c.dark = true);

        match provisioner.find_existing_app(USER).await {
            Existence::Unresolved { conductors, .. } => {
                assert_eq!(conductors, vec!["conductor-b".to_string()]);
            }
            Existence::NotInstalled => panic!(
                "\"no existing app\" must not be concluded from a pool that was not all asked"
            ),
            Existence::Found(agent) => panic!("nothing is installed, found {agent:?}"),
        }

        let error = provisioner
            .provision_agent(USER)
            .await
            .expect_err("the user may already have an app on the silent conductor");

        assert_eq!(unresolved_on(&error), vec!["conductor-b".to_string()]);
        assert_eq!(error.code(), "PROVISIONING_UNRESOLVED");
        assert_eq!(pool.total_keys_generated(), 0);
        assert_eq!(pool.total_installs_received(), 0);
    }

    /// Reaching a conductor is not enough — it has to ANSWER the lookup.
    #[tokio::test]
    async fn a_conductor_that_does_not_answer_the_lookup_is_unresolved_not_absent() {
        let (provisioner, pool) = two_conductor_pool().await;
        pool.with("conductor-a", |c| {
            c.lookup_fault = Some(AdminFault::sent_outcome_unknown(
                "Admin error (list_apps for get_app_info): Websocket error: Timeout",
            ))
        });

        assert!(matches!(
            provisioner.find_existing_app(USER).await,
            Existence::Unresolved { conductors, .. } if conductors == vec!["conductor-a".to_string()]
        ));
        assert_eq!(pool.total_keys_generated(), 0);
    }

    /// Only a pool in which EVERY conductor answered "no" is "not installed".
    #[tokio::test]
    async fn every_conductor_answering_no_is_the_only_not_installed() {
        let (provisioner, _pool) = two_conductor_pool().await;
        assert!(matches!(
            provisioner.find_existing_app(USER).await,
            Existence::NotInstalled
        ));
    }

    /// An app that IS found settles the question, whatever else is dark: the
    /// person has an identity and it is on a conductor that answered.
    #[tokio::test]
    async fn an_app_found_on_a_reachable_conductor_is_used_even_if_another_is_dark() {
        let (provisioner, pool) = two_conductor_pool().await;
        let key = raw_agent_key(200);
        pool.with("conductor-a", |c| c.dark = true);
        pool.with("conductor-b", |c| {
            c.apps.insert(
                app_on("conductor-b"),
                FakeApp {
                    agent_key: key.clone(),
                    status: "running",
                    cells_ready: true,
                },
            );
        });

        let agent = provisioner
            .provision_agent(USER)
            .await
            .expect("an existing app is the answer");

        assert_eq!(agent.conductor_id, "conductor-b");
        assert_eq!(agent.agent_pub_key, provisioned_agent_key(&key));
        assert_eq!(pool.total_keys_generated(), 0);
        assert_eq!(pool.total_installs_received(), 0);
    }

    /// An existing app left disabled — typically an install whose enable never
    /// arrived — is enabled when it is found, not handed back as if ready.
    #[tokio::test]
    async fn an_existing_disabled_app_is_enabled_when_found() {
        let (provisioner, pool) = two_conductor_pool().await;
        pool.with("conductor-a", |c| {
            c.apps.insert(
                app_on("conductor-a"),
                FakeApp {
                    agent_key: raw_agent_key(201),
                    status: "disabled",
                    cells_ready: false,
                },
            );
        });

        provisioner.provision_agent(USER).await.expect("found");

        pool.with("conductor-a", |c| {
            assert_eq!(c.apps[&app_on("conductor-a")].status, "running");
            assert_eq!(c.enables_received, 1);
        });
        assert_eq!(pool.total_keys_generated(), 0);
    }

    // ── the error a route reports ───────────────────────────────────────────

    #[test]
    fn an_unresolved_provision_has_its_own_code_and_names_the_conductor() {
        let error = ProvisionError::unresolved("conductor-3", "the install may have landed");
        assert_eq!(error.code(), "PROVISIONING_UNRESOLVED");
        assert_eq!(
            error.to_string(),
            "provisioning is unresolved on conductor-3: the install may have landed"
        );
        assert_eq!(
            ProvisionError::Failed("x".into()).code(),
            "PROVISIONING_FAILED"
        );
    }

    /// The exclusion is what makes the re-offer meaningful: without it, the
    /// pool's least-loaded member is handed back on every attempt and the
    /// budget is spent on the one conductor already known to be dropping.
    #[tokio::test]
    async fn a_dropped_conductor_is_excluded_from_the_next_offer() {
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
        assert_eq!(
            result.unwrap_err(),
            ProvisionError::Failed("No conductors available for provisioning".to_string())
        );
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
