//! Agent Provisioner — orchestrates Holochain agent provisioning
//!
//! When a user registers, the provisioner:
//! 1. Reads its own record of this person ([`super::provision_intent`]). If
//!    one exists, it settles THAT attempt on the conductor it names.
//! 2. Otherwise records the intent, on the least loaded conductor
//! 3. Generates a new agent key on that conductor, and records it
//! 4. Installs the app with that agent key
//! 5. Enables the app
//! 6. Registers the agent→conductor mapping
//!
//! The record is a saga log, not an authority: cells exist if the conductor
//! says so, a human exists if the DHT says so, and the log only remembers what
//! this doorway asked for.
//!
//! Every failure of an admin call arrives typed
//! ([`FaultClass`](super::typed_admin::FaultClass)), and what happens next is
//! decided on that type alone: an attempt whose install was never sent may be
//! offered to another conductor; one whose install was sent is settled on the
//! conductor it was sent to. See [`AgentProvisioner::provision_agent`].

use std::sync::Arc;
use tracing::{debug, info, warn};

use super::agent_key::{canonical_agent_key, lookup_forms};
use super::provision_intent::{IntentState, ProvisionIntent};
use super::registry::{ConductorInfo, ConductorRegistry};
use super::typed_admin::{
    AdminConnector, AppInfoDetailed, FaultClass, LiveAdminConnector, ProvisionAdmin,
};

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
    /// The attempt installed an app and could not confirm it removed it. The
    /// record must not be abandoned over an app that may still be there.
    app_may_remain: bool,
}

impl AttemptFault {
    fn new(install: FaultClass, message: String) -> Self {
        Self {
            install,
            message,
            app_may_remain: false,
        }
    }

    /// The doorway's or the conductor's own answer. Never re-offered.
    fn verdict(message: String) -> Self {
        Self::new(FaultClass::Refused, message)
    }
}

/// How one attempt on one conductor ended, when it did not end in an agent.
enum AttemptError {
    /// A typed fault; the caller decides re-offer / resolve / stop on it.
    Fault(AttemptFault),
    /// Another writer moved this person's record before the install was sent.
    /// Nothing was sent; their record stands.
    LostClaim,
    /// The record could not be written before the install was sent. Nothing
    /// was sent.
    Log(String),
    /// Already settled as far as it can be; report it as it is.
    Final(ProvisionError),
}

impl From<AttemptFault> for AttemptError {
    fn from(fault: AttemptFault) -> Self {
        Self::Fault(fault)
    }
}

/// One turn of resolving a person's provisioning from the record.
enum Step {
    Done(Result<ProvisionedAgent, ProvisionError>),
    /// The record changed under us (or was just abandoned): read it again.
    Again,
}

/// How many times a call re-reads the record after losing it to another
/// writer before reporting that someone else is provisioning this person.
const CLAIM_ROUNDS: u32 = 3;

/// The intent log could not be read or written, at a point where nothing had
/// been sent to any conductor. Provisioning blind would be provisioning
/// without the one thing that prevents a second identity, so it stops.
fn log_unavailable(error: String) -> ProvisionError {
    ProvisionError::Failed(format!(
        "provisioning intent log unavailable — nothing was sent to any conductor: {error}"
    ))
}

/// How long ago `then` was. A clock that ran backwards reads as "just now".
fn age_of(then: chrono::DateTime<chrono::Utc>) -> std::time::Duration {
    (chrono::Utc::now() - then)
        .to_std()
        .unwrap_or(std::time::Duration::ZERO)
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

/// What the recovery scan ([`AgentProvisioner::recover_by_pool_scan`])
/// established.
#[derive(Debug)]
pub enum Existence {
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
    /// ## The doorway asks its own record first, not the pool
    ///
    /// The doorway is the one that starts a provisioning attempt, so it writes
    /// the attempt down ([`super::provision_intent`]) BEFORE asking a conductor
    /// for anything, and every later call starts from that record:
    ///
    /// 1. **A record (or a legacy agent mapping) exists** → settle it on THAT
    ///    conductor and no other. Reachable: finish it forward. Unreachable:
    ///    [`ProvisionError::Unresolved`] naming it — this person waits for that
    ///    conductor, and nobody else does.
    /// 2. **No record and no mapping** → by this doorway's records the person
    ///    was never attempted: record the intent, then provision on the
    ///    least-loaded conductor. The pool is NOT scanned, so a conductor that
    ///    is down elsewhere blocks nothing.
    ///
    /// The pool scan survives only as [`Self::recover_by_pool_scan`], a named
    /// recovery path for a lost record.
    ///
    /// ## One person, one agent key
    ///
    /// Each attempt generates a FRESH agent key, and the app id embeds the
    /// conductor id. So "try the next conductor" is only safe when the previous
    /// attempt cannot have installed anything. The rule, decided on the typed
    /// [`FaultClass`] of the INSTALL:
    ///
    /// - `NotSent` — the install was never written (the connect failed, or key
    ///   generation dropped). The record is moved to another conductor and the
    ///   attempt re-offered, bounded at [`PROVISION_TRANSPORT_ATTEMPTS`].
    /// - `SentOutcomeUnknown` — the install was written and its outcome is not
    ///   known. Resolved on the SAME conductor, in this call and in every later
    ///   one (the record says an install was sent, to which conductor, with
    ///   which key). Never a second key elsewhere.
    /// - `Refused` — the conductor (or the doorway's own capacity check)
    ///   answered. A verdict; the record is abandoned if nothing was left
    ///   behind, and a later registration starts fresh.
    ///
    /// ## An install that is still in flight
    ///
    /// A conductor does not stop installing because its client stopped
    /// listening. A record that says "install sent" whose app is not on the
    /// conductor is therefore `Unresolved` until [`install_in_flight_bound`]
    /// has passed since it was sent; after that the SAME install is re-sent —
    /// same conductor, same app id, same recorded key. The re-send is safe at
    /// any age (a conductor refuses a second app under one id, and that refusal
    /// is read as "the original landed"); the bound only keeps the doorway from
    /// stacking a second install on a conductor still working on the first.
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
        // One call per person at a time in this process. The log's
        // compare-and-set is what holds across processes; this only spares a
        // second in-process call a wasted key generation. The wait is bounded by
        // the same bound an in-flight install is given; past it the caller
        // proceeds unserialized and the log decides.
        let _serial = self
            .registry
            .provision_locks()
            .acquire_within(user_identifier, install_in_flight_bound())
            .await;

        let result = self.provision_from_record(user_identifier).await;
        self.publish_intent_counts().await;
        result
    }

    async fn provision_from_record(
        &self,
        user_identifier: &str,
    ) -> Result<ProvisionedAgent, ProvisionError> {
        for _ in 0..CLAIM_ROUNDS {
            let recorded = self
                .registry
                .intents()
                .get(user_identifier)
                .await
                .map_err(log_unavailable)?;

            let step = match recorded {
                // An install was written, or seen installed: only the conductor
                // the record names can say what became of it.
                Some(intent) if intent.may_have_landed() => self.resolve_recorded(intent).await,
                // A conductor was chosen and nothing sent. If that is recent,
                // another call (another process — this process is serialized
                // above) is between choosing and sending: its conductor is the
                // one to wait on. If it is old, that call died; take it over.
                Some(intent) if intent.is_live() => {
                    if age_of(intent.updated_at) < install_in_flight_bound() {
                        Step::Done(Err(ProvisionError::unresolved(
                            &intent.conductor_id,
                            format!(
                                "another provisioning attempt for this person chose {} and is \
                                 still in progress",
                                intent.conductor_id
                            ),
                        )))
                    } else {
                        self.offer(user_identifier, Some(intent)).await
                    }
                }
                // Abandoned: the last attempt left nothing behind. Start over
                // on top of the record.
                Some(abandoned) => self.offer(user_identifier, Some(abandoned)).await,
                // No record. A person provisioned before the log existed still
                // has an agent mapping; failing that, they were never attempted.
                None => match self.resolve_legacy_mapping(user_identifier).await {
                    Some(step) => step,
                    None => self.offer(user_identifier, None).await,
                },
            };

            match step {
                Step::Done(result) => return result,
                Step::Again => continue,
            }
        }

        // Every round lost the record to another writer. Whoever keeps winning
        // is provisioning this person; say where.
        let conductors = match self.registry.intents().get(user_identifier).await {
            Ok(Some(intent)) => vec![intent.conductor_id],
            _ => Vec::new(),
        };
        Err(ProvisionError::Unresolved {
            conductors,
            detail: "another provisioning attempt for this person kept changing the record"
                .to_string(),
        })
    }

    /// Record the intent, then provision — on the least-loaded conductor, and
    /// on a sibling only while no install has been sent.
    ///
    /// `prior` is the record this attempt writes over (an abandoned one, or an
    /// unsent one whose writer died), or `None` for a person never attempted.
    async fn offer(&self, user_identifier: &str, prior: Option<ProvisionIntent>) -> Step {
        let store = self.registry.intents();
        let mut held = prior;
        let mut excluded: Vec<String> = Vec::new();
        let mut last_not_sent: Option<String> = None;

        for attempt in 1..=PROVISION_TRANSPORT_ATTEMPTS {
            let Some(conductor) = self.registry.find_least_loaded_excluding(&excluded) else {
                break;
            };

            if conductor.capacity_used >= conductor.capacity_max {
                let message = format!(
                    "Conductor {} at capacity ({}/{})",
                    conductor.conductor_id, conductor.capacity_used, conductor.capacity_max
                );
                self.abandon_if_live(held.as_mut(), &message).await;
                return Step::Done(Err(ProvisionError::Failed(message)));
            }

            // WRITE IT DOWN FIRST. Before the key is generated, before the
            // install is sent, before the conductor is even dialled.
            let installed_app_id =
                generate_app_id(&self.app_id, &conductor.conductor_id, user_identifier);
            let mut intent = ProvisionIntent::intended(
                user_identifier,
                &conductor.conductor_id,
                &installed_app_id,
            );
            let claimed = match &held {
                None => store.insert_new(&intent).await,
                Some(previous) => {
                    intent.version = previous.version + 1;
                    intent.created_at = previous.created_at;
                    store.replace(previous.version, &intent).await
                }
            };
            match claimed {
                Ok(true) => {}
                // Someone else wrote a record for this person first. Theirs
                // stands; go and resolve on it.
                Ok(false) => return Step::Again,
                Err(e) => return Step::Done(Err(log_unavailable(e))),
            }

            let fault = match self.provision_on(&conductor, &mut intent).await {
                Ok(agent) => return Step::Done(Ok(agent)),
                Err(AttemptError::LostClaim) => return Step::Again,
                Err(AttemptError::Log(e)) => return Step::Done(Err(log_unavailable(e))),
                Err(AttemptError::Final(error)) => return Step::Done(Err(error)),
                Err(AttemptError::Fault(fault)) => fault,
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
                    held = Some(intent);
                }
                FaultClass::SentOutcomeUnknown => {
                    warn!(
                        conductor = %conductor.conductor_id,
                        error = %fault.message,
                        "install was sent and its outcome is unknown — resolving on the \
                         SAME conductor; it will not be re-offered"
                    );
                    return Step::Done(
                        self.resolve_sent_install(&conductor, &mut intent, &fault.message)
                            .await,
                    );
                }
                FaultClass::Refused => {
                    // A definite refusal. If nothing was left on the conductor
                    // the attempt is over and the person is free to start again.
                    if !fault.app_may_remain {
                        self.abandon_if_live(Some(&mut intent), &fault.message)
                            .await;
                    }
                    return Step::Done(Err(ProvisionError::Failed(fault.message)));
                }
            }
        }

        // Nothing was ever sent to any conductor. Do not leave a record pinning
        // the person to one.
        let message =
            last_not_sent.unwrap_or_else(|| "No conductors available for provisioning".to_string());
        self.abandon_if_live(
            held.as_mut(),
            &format!("no install was ever sent: {message}"),
        )
        .await;
        Step::Done(Err(ProvisionError::Failed(message)))
    }

    /// One provisioning attempt against ONE named conductor, under `intent`.
    async fn provision_on(
        &self,
        conductor: &ConductorInfo,
        intent: &mut ProvisionIntent,
    ) -> Result<ProvisionedAgent, AttemptError> {
        let installed_app_id = intent.installed_app_id.clone();
        let admin = self
            .connector
            .connect(&conductor.admin_url)
            .await
            .map_err(|fault| {
                AttemptFault::new(
                    before_install(fault.class),
                    format!(
                        "Failed to connect to admin on {}: {}",
                        conductor.conductor_id, fault
                    ),
                )
            })?;

        info!(
            conductor = %conductor.conductor_id,
            admin_url = %conductor.admin_url,
            installed_app_id = %installed_app_id,
            user = %intent.identifier,
            "Provisioning new agent on conductor"
        );

        // 3. Generate agent key. Nothing identity-bearing exists until the
        // install below, so a transport fault here leaves the install unsent.
        let agent_key = admin.generate_agent_pub_key().await.map_err(|fault| {
            AttemptFault::new(
                before_install(fault.class),
                format!(
                    "Failed to generate agent key on {}: {}",
                    conductor.conductor_id, fault
                ),
            )
        })?;

        // Record the key and that the install is about to be written — BEFORE
        // writing it. If the doorway dies on the next line, the record already
        // says an install may be on this conductor, and with which key.
        intent.agent_pub_key = Some(provisioned_agent_key(&agent_key));
        intent.install_sent_at = Some(chrono::Utc::now());
        match self.save(intent).await {
            Ok(true) => {}
            Ok(false) => return Err(AttemptError::LostClaim),
            Err(e) => return Err(AttemptError::Log(e)),
        }

        // 4. Install app. From the moment this is written, the person may have
        // an identity on THIS conductor — the fault's own class is the answer.
        if let Err(fault) = admin
            .install_app(&installed_app_id, &agent_key, &self.bundle_path)
            .await
        {
            let message = format!(
                "Failed to install app on {}: {}",
                conductor.conductor_id, fault
            );
            match fault.class {
                // The socket was gone before the request was written. Take the
                // "sent" back — unless the record cannot be corrected, in which
                // case it stays, and is resolved here like any sent install.
                FaultClass::NotSent => {
                    intent.install_sent_at = None;
                    intent.agent_pub_key = None;
                    return Err(match self.save(intent).await {
                        Ok(true) => AttemptFault::new(FaultClass::NotSent, message),
                        _ => AttemptFault::new(FaultClass::SentOutcomeUnknown, message),
                    }
                    .into());
                }
                FaultClass::SentOutcomeUnknown => {
                    return Err(AttemptFault::new(FaultClass::SentOutcomeUnknown, message).into());
                }
                // The conductor answered. One refusal is not a failure: "an app
                // with this id is already installed" — the deterministic id is
                // this person's, so that app is theirs (a record this doorway
                // lost). Use it, with the key the CONDUCTOR holds for it.
                FaultClass::Refused => {
                    if let Ok(Some(existing)) = admin.find_app(&installed_app_id).await {
                        info!(
                            conductor = %conductor.conductor_id,
                            app_id = %installed_app_id,
                            "Install refused because the app already exists — adopting it"
                        );
                        return self
                            .finish_forward(&*admin, conductor, intent, existing)
                            .await
                            .map_err(AttemptError::Final);
                    }
                    return Err(AttemptFault::verdict(message).into());
                }
            }
        }
        intent.state = IntentState::Installed;
        self.note(intent).await;

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
                    self.cleanup_fresh_install(&*admin, &installed_app_id, message)
                        .await
                }
                // The socket went: an installed app in an unknown state. No
                // cleanup over a dead socket, and above all no second install.
                FaultClass::NotSent | FaultClass::SentOutcomeUnknown => {
                    AttemptFault::new(FaultClass::SentOutcomeUnknown, message)
                }
            }
            .into());
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
            return Err(self
                .cleanup_fresh_install(
                    &*admin,
                    &installed_app_id,
                    format!(
                        "Cell genesis timed out on {} for app '{}' after {}s",
                        conductor.conductor_id, installed_app_id, waited
                    ),
                )
                .await
                .into());
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
            return Err(self
                .cleanup_fresh_install(
                    &*admin,
                    &installed_app_id,
                    format!("Failed to register agent mapping: {e}"),
                )
                .await
                .into());
        }

        for alias in alias_forms {
            let _ = self
                .registry
                .register_agent(alias, &conductor.conductor_id, &installed_app_id)
                .await;
        }

        intent.state = IntentState::Enabled;
        self.note(intent).await;

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

    /// Uninstall an app THIS attempt installed moments ago, after a verdict
    /// that makes it unusable. Only ever reached inside `provision_on`, before
    /// the app has been handed to anyone — `create_human` cannot have run.
    ///
    /// The fault says whether the app may still be there: if the uninstall did
    /// not succeed, the record must NOT be abandoned over it.
    async fn cleanup_fresh_install(
        &self,
        admin: &dyn ProvisionAdmin,
        installed_app_id: &str,
        message: String,
    ) -> AttemptFault {
        let mut fault = AttemptFault::verdict(message);
        if let Err(cleanup_err) = admin.uninstall_app(installed_app_id).await {
            warn!("Cleanup uninstall failed: {}", cleanup_err);
            fault.app_may_remain = true;
        }
        fault
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

    /// An install was written to `conductor` in THIS call and its outcome is
    /// unknown. Settle it there.
    ///
    /// - the app is installed → finish it forward;
    /// - the conductor answers that it has no such app → unresolved. The record
    ///   already says "sent"; a later call finds the app once it lands, or
    ///   re-sends the same install once it is implausibly old;
    /// - the conductor cannot be asked → unresolved.
    async fn resolve_sent_install(
        &self,
        conductor: &ConductorInfo,
        intent: &mut ProvisionIntent,
        sent_fault: &str,
    ) -> Result<ProvisionedAgent, ProvisionError> {
        let conductor_id = conductor.conductor_id.as_str();
        let installed_app_id = intent.installed_app_id.clone();

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

        match admin.find_app(&installed_app_id).await {
            Ok(Some(existing)) => {
                info!(
                    conductor = %conductor_id,
                    app_id = %installed_app_id,
                    status = ?existing.status,
                    "The install whose outcome was unknown DID land — finishing it in place"
                );
                self.finish_forward(&*admin, conductor, intent, existing)
                    .await
            }
            Ok(None) => Err(ProvisionError::unresolved(
                conductor_id,
                format!(
                    "{sent_fault}; '{installed_app_id}' is not installed on {conductor_id} \
                     yet, but the install was sent and may still land"
                ),
            )),
            Err(fault) => Err(ProvisionError::unresolved(
                conductor_id,
                format!(
                    "{sent_fault}; the install of '{installed_app_id}' may have landed and \
                     {conductor_id} did not answer the lookup ({fault})"
                ),
            )),
        }
    }

    /// A record from an earlier call says something may be on a conductor.
    /// Settle it on THAT conductor, and only there.
    async fn resolve_recorded(&self, mut intent: ProvisionIntent) -> Step {
        let Some(conductor) = self.registry.get_conductor_info(&intent.conductor_id) else {
            return Step::Done(self.recover_unplaceable_record(intent).await);
        };
        let conductor_id = conductor.conductor_id.as_str();
        let installed_app_id = intent.installed_app_id.clone();

        let admin = match self.connector.connect(&conductor.admin_url).await {
            Ok(admin) => admin,
            Err(fault) => {
                return Step::Done(Err(ProvisionError::unresolved(
                    conductor_id,
                    format!(
                        "this person's provisioning is recorded on {conductor_id} \
                         ('{installed_app_id}', {}), which could not be reached ({fault})",
                        intent.state.as_str()
                    ),
                )));
            }
        };

        let existing = match admin.find_app(&installed_app_id).await {
            Ok(existing) => existing,
            Err(fault) => {
                return Step::Done(Err(ProvisionError::unresolved(
                    conductor_id,
                    format!(
                        "this person's provisioning is recorded on {conductor_id} \
                         ('{installed_app_id}'), which did not answer the lookup ({fault})"
                    ),
                )));
            }
        };

        if let Some(existing) = existing {
            return Step::Done(
                self.finish_forward(&*admin, &conductor, &mut intent, existing)
                    .await,
            );
        }

        // The conductor ANSWERED: no such app.
        if intent.state >= IntentState::Installed {
            // The log saw it installed and the conductor no longer has it —
            // uninstalled, or the conductor was reset. Cells exist only if the
            // conductor says so; the record gives way and the person starts
            // fresh.
            warn!(
                conductor = %conductor_id,
                app_id = %installed_app_id,
                recorded_state = intent.state.as_str(),
                "The conductor no longer has an app this doorway recorded — the record \
                 gives way to the conductor"
            );
            self.abandon_if_live(
                Some(&mut intent),
                &format!("{conductor_id} no longer has '{installed_app_id}'"),
            )
            .await;
            return Step::Again;
        }

        // Sent, never seen. Still in flight?
        let sent = intent
            .install_sent_at
            .map(age_of)
            .unwrap_or(std::time::Duration::MAX);
        let bound = install_in_flight_bound();
        if sent < bound {
            return Step::Done(Err(ProvisionError::unresolved(
                conductor_id,
                format!(
                    "the install of '{installed_app_id}' was sent to {conductor_id} {}s ago \
                     and is not there yet; it may still land (it is re-sent after {}s)",
                    sent.as_secs(),
                    bound.as_secs()
                ),
            )));
        }

        Step::Done(self.resend_install(&*admin, &conductor, &mut intent).await)
    }

    /// Re-send an install that has been "in flight" longer than the doorway's
    /// own admin request is ever waited for — to the SAME conductor, under the
    /// SAME app id, with the SAME recorded key. Never a new key: a new key
    /// would be a second identity if the original lands after all.
    async fn resend_install(
        &self,
        admin: &dyn ProvisionAdmin,
        conductor: &ConductorInfo,
        intent: &mut ProvisionIntent,
    ) -> Result<ProvisionedAgent, ProvisionError> {
        let conductor_id = conductor.conductor_id.as_str();
        let installed_app_id = intent.installed_app_id.clone();
        let unresolved = |detail: String| ProvisionError::unresolved(conductor_id, detail);

        let Some(agent_key) = intent
            .agent_pub_key
            .as_deref()
            .and_then(super::agent_key::decode_key_bytes)
        else {
            return Err(unresolved(format!(
                "an install of '{installed_app_id}' was sent to {conductor_id} but the record \
                 holds no usable agent key to re-send it with"
            )));
        };

        intent.install_sent_at = Some(chrono::Utc::now());
        match self.save(intent).await {
            Ok(true) => {}
            Ok(false) => {
                return Err(unresolved(
                    "another provisioning attempt for this person is re-sending the install"
                        .to_string(),
                ))
            }
            Err(e) => return Err(log_unavailable(e)),
        }

        warn!(
            conductor = %conductor_id,
            app_id = %installed_app_id,
            "Re-sending an install that never appeared — same conductor, same app id, same key"
        );
        let resent = admin
            .install_app(&installed_app_id, &agent_key, &self.bundle_path)
            .await;

        // Whatever the re-send answered, the conductor's app list is the
        // truth. A refusal here most often means the ORIGINAL landed.
        match admin.find_app(&installed_app_id).await {
            Ok(Some(existing)) => {
                self.finish_forward(admin, conductor, intent, existing)
                    .await
            }
            Ok(None) => Err(unresolved(match resent {
                Ok(()) => format!(
                    "the re-sent install of '{installed_app_id}' was accepted by {conductor_id} \
                     but the app is not listed yet"
                ),
                Err(fault) => format!(
                    "the re-sent install of '{installed_app_id}' on {conductor_id} failed \
                     ({fault}) and the app is not there"
                ),
            })),
            Err(fault) => Err(unresolved(format!(
                "the install of '{installed_app_id}' was re-sent to {conductor_id}, which did \
                 not answer the lookup ({fault})"
            ))),
        }
    }

    /// An app exists on `conductor` for this record. Bring it to ready and hand
    /// it back — with the agent key the CONDUCTOR reports for it.
    ///
    /// Installed is not ready: a disabled app is enabled, an app in any other
    /// non-running state is reported, and cells are waited for. Never
    /// uninstalls — `create_human` may already have run on this app.
    async fn finish_forward(
        &self,
        admin: &dyn ProvisionAdmin,
        conductor: &ConductorInfo,
        intent: &mut ProvisionIntent,
        existing: AppInfoDetailed,
    ) -> Result<ProvisionedAgent, ProvisionError> {
        let conductor_id = conductor.conductor_id.as_str();
        let installed_app_id = intent.installed_app_id.clone();

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
            && !self.await_cells(admin, conductor, &installed_app_id).await
        {
            return Err(ProvisionError::unresolved(
                conductor_id,
                format!(
                    "'{installed_app_id}' is installed on {conductor_id} but its cells did not \
                     become ready"
                ),
            ));
        }

        let agent = self
            .adopt_existing(conductor, installed_app_id, &existing.agent_pub_key)
            .await;

        intent.agent_pub_key = Some(agent.agent_pub_key.clone());
        if intent.state < IntentState::Enabled {
            intent.state = IntentState::Enabled;
        }
        self.note(intent).await;
        Ok(agent)
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

    /// A person provisioned before the intent log existed: no record, but this
    /// doorway holds an agent mapping for their deterministic app id on some
    /// conductor. That mapping names the conductor to resolve on.
    ///
    /// A lookup in the doorway's OWN records — no conductor is dialled to find
    /// the mapping, and only the mapped conductor is dialled to resolve it.
    /// `None` = no mapping (or a stale one the conductor disowned): the person
    /// was never attempted, as far as this doorway knows.
    async fn resolve_legacy_mapping(&self, user_identifier: &str) -> Option<Step> {
        let (conductor, installed_app_id) =
            self.registry.list_conductors().into_iter().find_map(|c| {
                let app_id = generate_app_id(&self.app_id, &c.conductor_id, user_identifier);
                self.registry
                    .find_agent_by_app(&c.conductor_id, &app_id)
                    .map(|_| (c, app_id))
            })?;
        let conductor_id = conductor.conductor_id.as_str();
        let unresolved = |detail: String| {
            Some(Step::Done(Err(ProvisionError::unresolved(
                conductor_id,
                detail,
            ))))
        };

        let admin = match self.connector.connect(&conductor.admin_url).await {
            Ok(admin) => admin,
            Err(fault) => {
                return unresolved(format!(
                    "this person is mapped to '{installed_app_id}' on {conductor_id}, which \
                     could not be reached ({fault})"
                ));
            }
        };
        let existing = match admin.find_app(&installed_app_id).await {
            Ok(Some(existing)) => existing,
            Ok(None) => {
                warn!(
                    conductor = %conductor_id,
                    app_id = %installed_app_id,
                    "A persisted agent mapping names an app its conductor does not have — \
                     treating the person as never attempted"
                );
                return None;
            }
            Err(fault) => {
                return unresolved(format!(
                    "this person is mapped to '{installed_app_id}' on {conductor_id}, which \
                     did not answer the lookup ({fault})"
                ));
            }
        };

        info!(
            conductor = %conductor_id,
            app_id = %installed_app_id,
            user = %user_identifier,
            status = ?existing.status,
            "Reusing existing app installation (agent mapping, no intent record) — backfilling"
        );

        // An app left disabled is enabled rather than handed back unusable.
        // Best-effort, as it always was on this path: the caller's own
        // readiness handling (the chaperone re-enables and polls) still runs.
        if existing.status.as_deref() == Some("disabled") {
            if let Err(fault) = admin.enable_app(&installed_app_id).await {
                warn!(
                    conductor = %conductor_id,
                    app_id = %installed_app_id,
                    error = %fault,
                    "Existing app is disabled and could not be enabled"
                );
            }
        }

        let agent = self
            .adopt_existing(
                &conductor,
                installed_app_id.clone(),
                &existing.agent_pub_key,
            )
            .await;
        self.backfill(user_identifier, &agent).await;
        Some(Step::Done(Ok(agent)))
    }

    /// Write the record for an app found without one. If another writer got
    /// there first, theirs stands.
    async fn backfill(&self, user_identifier: &str, agent: &ProvisionedAgent) {
        let mut intent = ProvisionIntent::intended(
            user_identifier,
            &agent.conductor_id,
            &agent.installed_app_id,
        );
        intent.agent_pub_key = Some(agent.agent_pub_key.clone());
        intent.state = IntentState::Enabled;
        if let Err(e) = self.registry.intents().insert_new(&intent).await {
            warn!(user = %user_identifier, error = %e, "Could not backfill the provisioning intent");
        }
    }

    /// A record whose conductor is not in this doorway's pool cannot be
    /// resolved "on that conductor". This is the lost-record case, and the one
    /// place the request path uses the recovery scan.
    async fn recover_unplaceable_record(
        &self,
        mut intent: ProvisionIntent,
    ) -> Result<ProvisionedAgent, ProvisionError> {
        warn!(
            user = %intent.identifier,
            recorded_conductor = %intent.conductor_id,
            "Provisioning record names a conductor that is not in the pool — running the \
             recovery scan"
        );
        match self.recover_by_pool_scan(&intent.identifier).await {
            Existence::Found(agent) => {
                intent.conductor_id = agent.conductor_id.clone();
                intent.installed_app_id = agent.installed_app_id.clone();
                intent.agent_pub_key = Some(agent.agent_pub_key.clone());
                if intent.state < IntentState::Enabled {
                    intent.state = IntentState::Enabled;
                }
                self.note(&mut intent).await;
                Ok(agent)
            }
            Existence::NotInstalled => Err(ProvisionError::unresolved(
                &intent.conductor_id,
                format!(
                    "this person's provisioning is recorded on {}, which is not in this \
                     doorway's pool, and no conductor in the pool has their app",
                    intent.conductor_id
                ),
            )),
            Existence::Unresolved { conductors, detail } => {
                Err(ProvisionError::Unresolved { conductors, detail })
            }
        }
    }

    /// RECOVERY PATH — not on the request path. Ask EVERY conductor in the pool
    /// whether it holds this person's app.
    ///
    /// This is how the doorway used to answer "does this person already have an
    /// app?" on every registration, which made every registration depend on
    /// every conductor. It is kept for the case the intent log cannot answer: a
    /// record that was lost, or that names a conductor no longer in the pool.
    /// It is for an operator or an admin tool to invoke; the only caller on the
    /// request path is [`Self::recover_unplaceable_record`].
    ///
    /// A conductor that cannot be reached, or does not answer the lookup, is
    /// NOT a conductor without the app: [`Existence::Unresolved`] names it.
    pub async fn recover_by_pool_scan(&self, user_identifier: &str) -> Existence {
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
                        "Recovery scan found an existing app installation"
                    );

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

    // ── the log ─────────────────────────────────────────────────────────────

    /// Write `intent` against the version it was read at. `Ok(false)` = another
    /// writer moved the record; `intent` is left as it was.
    async fn save(&self, intent: &mut ProvisionIntent) -> Result<bool, String> {
        let expected = intent.version;
        let mut next = intent.clone();
        next.version = expected + 1;
        next.updated_at = chrono::Utc::now();
        let written = self.registry.intents().replace(expected, &next).await?;
        if written {
            *intent = next;
        }
        Ok(written)
    }

    /// Record progress that nothing is decided on. A failure here is logged and
    /// provisioning carries on: the log is rebuilt from the conductor on the
    /// next resolution, so a missed mark costs a lookup, never an identity.
    async fn note(&self, intent: &mut ProvisionIntent) {
        match self.save(intent).await {
            Ok(true) => {}
            Ok(false) => warn!(
                user = %intent.identifier,
                state = intent.state.as_str(),
                "Provisioning record was changed by another writer; progress not recorded"
            ),
            Err(e) => warn!(
                user = %intent.identifier,
                state = intent.state.as_str(),
                error = %e,
                "Provisioning progress could not be recorded"
            ),
        }
    }

    /// End an attempt that left nothing behind.
    async fn abandon_if_live(&self, intent: Option<&mut ProvisionIntent>, reason: &str) {
        let Some(intent) = intent else { return };
        if !intent.is_live() {
            return;
        }
        intent.state = IntentState::Abandoned;
        intent.abandoned_reason = Some(reason.to_string());
        intent.install_sent_at = None;
        intent.agent_pub_key = None;
        self.note(intent).await;
    }

    /// Mark how far the registration got AFTER provisioning: the route calls
    /// this when its `create_human` returns and when the account row is
    /// written. Forward-only and best-effort; a person with no record is left
    /// without one. Nothing is decided on these marks — whether a human exists
    /// is the DHT's to say.
    pub async fn record_progress(&self, user_identifier: &str, state: IntentState) {
        if state == IntentState::Abandoned {
            return;
        }
        let Ok(Some(mut intent)) = self.registry.intents().get(user_identifier).await else {
            return;
        };
        if !intent.is_live() || intent.state >= state {
            return;
        }
        intent.state = state;
        self.note(&mut intent).await;
        self.publish_intent_counts().await;
    }

    /// Publish the log's counts so abandoned and stale records are visible
    /// debt. Nothing reaps them.
    async fn publish_intent_counts(&self) {
        let bound = chrono::Duration::from_std(install_in_flight_bound())
            .unwrap_or_else(|_| chrono::Duration::zero());
        match self
            .registry
            .intents()
            .counts(chrono::Utc::now() - bound)
            .await
        {
            Ok(counts) => {
                for (state, n) in &counts.by_state {
                    crate::metrics::set_provision_intents(state.as_str(), *n);
                }
                crate::metrics::set_provision_intents_stale(counts.stale);
            }
            Err(e) => debug!(error = %e, "Provisioning intent counts unavailable"),
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

/// How long after an install was written it is still presumed in flight.
///
/// DERIVED, NOT CHOSEN: it is the admin client's own request timeout, read from
/// the library that enforces it ([`super::typed_admin::admin_request_timeout`],
/// i.e. `holochain_websocket::WebsocketConfig::CLIENT_DEFAULT
/// .default_request_timeout`). That is the longest the doorway itself ever
/// waits for an `install_app` to answer; an install older than that is one the
/// doorway has, by its own client's rule, stopped expecting an answer to.
///
/// It is not a guarantee about the conductor — the conductor has no deadline
/// for an install and does not cancel one. So nothing unsafe may hang on this
/// bound, and nothing does: past it the SAME install is re-sent (same
/// conductor, same app id, same key), which a conductor that already has the
/// app refuses.
fn install_in_flight_bound() -> std::time::Duration {
    super::typed_admin::admin_request_timeout()
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
    use tokio::sync::Notify;

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
        /// `(entered, release)`: an install signals `entered` once it has been
        /// received, then waits for `release` before the conductor acts on it —
        /// so a test can hold an install provably in flight.
        install_gate: Option<(Arc<Notify>, Arc<Notify>)>,
        /// The next install finds that an earlier, identical one landed first:
        /// the app appears (with the key being sent) and the conductor refuses
        /// the duplicate.
        lands_before_next_install: bool,
        apps: HashMap<String, FakeApp>,
        /// Connect ATTEMPTS, accepted or not.
        connects_received: u32,
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
                install_gate: None,
                lands_before_next_install: false,
                apps: HashMap::new(),
                connects_received: 0,
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
                conductor.connects_received += 1;
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
            let gate = self.with(|c| {
                c.installs_received += 1;
                c.install_gate.take()
            });
            if let Some((entered, release)) = gate {
                entered.notify_one();
                release.notified().await;
            }
            self.with(|c| {
                // A conductor refuses a second app under an id it already has.
                let already_installed = || {
                    Err(AdminFault::refused(format!(
                        "Admin error (install_app): External API wire error: \
                         InternalError(\"AppAlreadyInstalled({installed_app_id})\")"
                    )))
                };
                if c.lands_before_next_install {
                    c.lands_before_next_install = false;
                    c.apps.insert(
                        installed_app_id.to_string(),
                        FakeApp {
                            agent_key: agent_key.to_vec(),
                            status: "disabled",
                            cells_ready: false,
                        },
                    );
                    return already_installed();
                }
                if c.apps.contains_key(installed_app_id) {
                    return already_installed();
                }
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
        // Refuses the provisioning connect, the first thing it is asked.
        pool.with("conductor-a", |c| c.connects = VecDeque::from([false]));

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

    /// THE RECOVERY SCAN. A conductor that cannot be reached is not a conductor
    /// without the app, and the scan says so explicitly. (It is no longer on
    /// the request path — see `a_fresh_registration_does_not_scan_the_pool`.)
    #[tokio::test]
    async fn an_unreachable_conductor_during_the_recovery_scan_is_unresolved_not_absent() {
        let (provisioner, pool) = two_conductor_pool().await;
        pool.with("conductor-b", |c| c.dark = true);

        match provisioner.recover_by_pool_scan(USER).await {
            Existence::Unresolved { conductors, .. } => {
                assert_eq!(conductors, vec!["conductor-b".to_string()]);
            }
            Existence::NotInstalled => panic!(
                "\"no existing app\" must not be concluded from a pool that was not all asked"
            ),
            Existence::Found(agent) => panic!("nothing is installed, found {agent:?}"),
        }

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
            provisioner.recover_by_pool_scan(USER).await,
            Existence::Unresolved { conductors, .. } if conductors == vec!["conductor-a".to_string()]
        ));
        assert_eq!(pool.total_keys_generated(), 0);
    }

    /// Only a pool in which EVERY conductor answered "no" is "not installed".
    #[tokio::test]
    async fn every_conductor_answering_no_is_the_only_not_installed() {
        let (provisioner, _pool) = two_conductor_pool().await;
        assert!(matches!(
            provisioner.recover_by_pool_scan(USER).await,
            Existence::NotInstalled
        ));
    }

    /// An app that IS found by the recovery scan settles the question,
    /// whatever else is dark.
    #[tokio::test]
    async fn the_recovery_scan_uses_an_app_found_on_a_reachable_conductor() {
        let (provisioner, pool) = two_conductor_pool().await;
        let key = raw_agent_key(200);
        pool.with("conductor-a", |c| c.dark = true);
        pool.with("conductor-b", |c| {
            c.apps.insert(
                app_on("conductor-b"),
                FakeApp {
                    agent_key: key.clone(),
                    status: "disabled",
                    cells_ready: false,
                },
            );
        });

        match provisioner.recover_by_pool_scan(USER).await {
            Existence::Found(agent) => {
                assert_eq!(agent.conductor_id, "conductor-b");
                assert_eq!(agent.agent_pub_key, provisioned_agent_key(&key));
            }
            other => panic!("the app is on conductor-b: {other:?}"),
        }
        pool.with("conductor-b", |c| {
            assert_eq!(
                c.apps[&app_on("conductor-b")].status,
                "running",
                "a disabled app is enabled when found"
            );
        });
        assert_eq!(pool.total_keys_generated(), 0);
        assert_eq!(pool.total_installs_received(), 0);
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

    // ── the intent log: what this doorway asked for ─────────────────────────
    //
    // The provisioner remembers its own attempt, so "does this person already
    // have an app?" is answered from the doorway's record and settled on the
    // ONE conductor that record names — never by asking the whole pool.

    use crate::conductor::provision_intent::{IntentState, IntentStore, MemoryIntentStore};

    /// A doorway process over `pool`, with its intent log in `store`.
    /// `loads` sets which conductor is least loaded for THIS process — the one
    /// a fresh attempt would be offered to.
    async fn doorway_over(
        pool: &Arc<FakePool>,
        store: &Arc<MemoryIntentStore>,
        loads: [(&str, usize); 2],
    ) -> AgentProvisioner {
        let registry = ConductorRegistry::new(None)
            .await
            .with_intent_store(Arc::clone(store) as Arc<dyn IntentStore>);
        for (id, used) in loads {
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
                .entry(admin_url(id))
                .or_insert_with(FakeConductor::healthy);
        }
        AgentProvisioner::new(Arc::new(registry))
            .with_connector(Arc::new(FakeConnector(Arc::clone(pool))))
    }

    const A_FIRST: [(&str, usize); 2] = [("conductor-a", 0), ("conductor-b", 5)];
    const B_FIRST: [(&str, usize); 2] = [("conductor-a", 9), ("conductor-b", 0)];

    async fn intent_of(store: &MemoryIntentStore, who: &str) -> ProvisionIntent {
        store
            .get(who)
            .await
            .unwrap()
            .unwrap_or_else(|| panic!("no intent recorded for {who}"))
    }

    /// Make the recorded install look as if it was sent `ago` in the past.
    async fn age_install(store: &MemoryIntentStore, who: &str, ago: std::time::Duration) {
        let mut intent = intent_of(store, who).await;
        let expected = intent.version;
        let then = chrono::Utc::now() - chrono::Duration::from_std(ago).unwrap();
        intent.install_sent_at = Some(then);
        intent.updated_at = then;
        intent.version += 1;
        assert!(store.replace(expected, &intent).await.unwrap());
    }

    /// The first call's install is written and its socket drops; the app is not
    /// (yet) visible. Both later tests start here.
    async fn an_install_in_flight_on_conductor_a(
        pool: &Arc<FakePool>,
        store: &Arc<MemoryIntentStore>,
    ) {
        let first = doorway_over(pool, store, A_FIRST).await;
        pool.with("conductor-a", |c| {
            c.install = Install::SentThenDropped { lands: None }
        });
        let error = first
            .provision_agent(USER)
            .await
            .expect_err("the install's outcome is unknown");
        assert_eq!(unresolved_on(&error), vec!["conductor-a".to_string()]);
        assert_eq!(pool.total_keys_generated(), 1);
        pool.with("conductor-a", |c| c.install = Install::Succeeds);
    }

    /// The in-flight install lands, with the key the doorway generated for it.
    async fn the_in_flight_install_lands(pool: &FakePool, store: &MemoryIntentStore) {
        let intent = intent_of(store, USER).await;
        let key = crate::conductor::decode_key_bytes(
            intent
                .agent_pub_key
                .as_deref()
                .expect("the key is recorded before the install is sent"),
        )
        .expect("the recorded key decodes");
        pool.with("conductor-a", |c| {
            c.apps.insert(
                intent.installed_app_id.clone(),
                FakeApp {
                    agent_key: key,
                    status: "disabled",
                    cells_ready: false,
                },
            );
        });
    }

    /// THE CROSS-CALL CASE. A retry while the first attempt's install is still
    /// in flight finds the intent and waits on THAT conductor: no second key, no
    /// second install — even though a fresh attempt would now be offered to the
    /// sibling. When the install lands, the next retry finishes it.
    #[tokio::test]
    async fn a_retry_while_the_install_is_in_flight_yields_one_key_and_one_install() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        an_install_in_flight_on_conductor_a(&pool, &store).await;

        let intent = intent_of(&store, USER).await;
        assert_eq!(intent.conductor_id, "conductor-a");
        assert_eq!(intent.state, IntentState::Intended);
        assert!(intent.install_sent_at.is_some());

        // Same process, but conductor-b is now the least loaded.
        let doorway = doorway_over(&pool, &store, B_FIRST).await;
        let error = doorway
            .provision_agent(USER)
            .await
            .expect_err("still in flight: the answer is to wait on conductor-a");
        assert_eq!(unresolved_on(&error), vec!["conductor-a".to_string()]);
        assert_eq!(pool.total_keys_generated(), 1, "no second agent key");
        assert_eq!(pool.total_installs_received(), 1, "no second install");
        assert_untouched(&pool, "conductor-b");

        the_in_flight_install_lands(&pool, &store).await;
        let agent = doorway
            .provision_agent(USER)
            .await
            .expect("the install landed: finish it forward");
        assert_eq!(agent.conductor_id, "conductor-a");
        assert_eq!(pool.total_keys_generated(), 1);
        assert_eq!(pool.total_installs_received(), 1);
        assert_untouched(&pool, "conductor-b");
        assert_eq!(intent_of(&store, USER).await.state, IntentState::Enabled);
        assert_eq!(store.len(), 1);
    }

    /// The same, across a doorway restart: a NEW process (new registry, nothing
    /// in memory) over the same durable log resolves on the same conductor.
    #[tokio::test]
    async fn a_retry_after_a_doorway_restart_yields_one_key_and_one_install() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        an_install_in_flight_on_conductor_a(&pool, &store).await;
        the_in_flight_install_lands(&pool, &store).await;

        let restarted = doorway_over(&pool, &store, B_FIRST).await;
        let agent = restarted
            .provision_agent(USER)
            .await
            .expect("the restarted doorway finishes what the old one asked for");

        assert_eq!(agent.conductor_id, "conductor-a");
        assert_eq!(pool.total_keys_generated(), 1);
        assert_eq!(pool.total_installs_received(), 1);
        assert_untouched(&pool, "conductor-b");
    }

    /// An unreachable conductor blocks the ONE person whose intent names it.
    /// Everyone else provisions on another conductor without it being asked.
    #[tokio::test]
    async fn an_unreachable_conductor_blocks_only_the_identifier_whose_intent_names_it() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        an_install_in_flight_on_conductor_a(&pool, &store).await;
        pool.with("conductor-a", |c| c.dark = true);

        let doorway = doorway_over(&pool, &store, B_FIRST).await;

        let error = doorway
            .provision_agent(USER)
            .await
            .expect_err("jane's install may be on conductor-a; she waits for it");
        assert_eq!(unresolved_on(&error), vec!["conductor-a".to_string()]);
        assert_eq!(error.code(), "PROVISIONING_UNRESOLVED");

        let connects_before = pool.with("conductor-a", |c| c.connects_received);
        let bob = doorway
            .provision_agent("bob@example.com")
            .await
            .expect("bob was never attempted: a dark conductor elsewhere blocks nothing");
        assert_eq!(bob.conductor_id, "conductor-b");
        assert_eq!(
            pool.with("conductor-a", |c| c.connects_received),
            connects_before,
            "provisioning bob must not ask conductor-a anything"
        );
    }

    /// With no record at all, a registration is offered to the least-loaded
    /// conductor and no other conductor is dialled — the pool is not scanned.
    #[tokio::test]
    async fn a_fresh_registration_does_not_scan_the_pool() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        pool.with("conductor-b", |c| c.dark = true);

        let agent = doorway
            .provision_agent(USER)
            .await
            .expect("a dark conductor that was never asked for anything blocks nothing");

        assert_eq!(agent.conductor_id, "conductor-a");
        assert_eq!(pool.with("conductor-b", |c| c.connects_received), 0);
        let intent = intent_of(&store, USER).await;
        assert_eq!(intent.state, IntentState::Enabled);
        assert_eq!(intent.installed_app_id, app_on("conductor-a"));
        assert_eq!(
            intent.agent_pub_key.as_deref(),
            Some(agent.agent_pub_key.as_str())
        );
    }

    /// Two first registrations of one identifier at once, in one process: one
    /// intent, one key, one install. The first install PARKS inside the
    /// conductor so both calls are provably in flight together.
    #[tokio::test]
    async fn two_concurrent_first_registrations_of_one_identifier_yield_one_intent() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = Arc::new(doorway_over(&pool, &store, A_FIRST).await);
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        pool.with("conductor-a", |c| {
            c.install_gate = Some((Arc::clone(&entered), Arc::clone(&release)))
        });

        let first = {
            let doorway = Arc::clone(&doorway);
            tokio::spawn(async move { doorway.provision_agent(USER).await })
        };
        entered.notified().await;

        let second = {
            let doorway = Arc::clone(&doorway);
            tokio::spawn(async move { doorway.provision_agent(USER).await })
        };
        for _ in 0..20 {
            tokio::task::yield_now().await;
        }
        assert_eq!(
            pool.total_keys_generated(),
            1,
            "the second call generated a key while the first was mid-install"
        );
        assert_eq!(store.len(), 1);

        release.notify_one();
        let first = first.await.unwrap().expect("the first call provisions");
        let second = second
            .await
            .unwrap()
            .expect("the second finds the first's app");

        assert_eq!(first.agent_pub_key, second.agent_pub_key);
        assert_eq!(pool.total_keys_generated(), 1);
        assert_eq!(pool.total_installs_received(), 1);
        assert_eq!(store.len(), 1);
        assert_untouched(&pool, "conductor-b");
    }

    /// The same race from TWO provisioner instances on one log (two doorway
    /// processes on one database): the log's compare-and-set gives one intent,
    /// and the loser resolves on the winner's conductor — here, by waiting.
    #[tokio::test]
    async fn two_provisioner_instances_on_one_log_yield_one_intent() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let winner = Arc::new(doorway_over(&pool, &store, A_FIRST).await);
        // The other process would offer a fresh attempt to conductor-b.
        let loser = doorway_over(&pool, &store, B_FIRST).await;
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        pool.with("conductor-a", |c| {
            c.install_gate = Some((Arc::clone(&entered), Arc::clone(&release)))
        });

        let first = {
            let winner = Arc::clone(&winner);
            tokio::spawn(async move { winner.provision_agent(USER).await })
        };
        entered.notified().await;

        let error = loser
            .provision_agent(USER)
            .await
            .expect_err("the other process is mid-install on conductor-a");
        assert_eq!(unresolved_on(&error), vec!["conductor-a".to_string()]);
        assert_untouched(&pool, "conductor-b");
        assert_eq!(pool.total_keys_generated(), 1);

        release.notify_one();
        let agent = first.await.unwrap().expect("the winner provisions");
        assert_eq!(agent.conductor_id, "conductor-a");
        assert_eq!(store.len(), 1);
        assert_eq!(pool.total_installs_received(), 1);

        // …and once it has, the other process simply finds it.
        let found = loser.provision_agent(USER).await.expect("found");
        assert_eq!(found.agent_pub_key, agent.agent_pub_key);
        assert_untouched(&pool, "conductor-b");
    }

    /// A definite refusal before anything landed ends the attempt: the intent is
    /// abandoned (and stays countable), and a later registration starts fresh.
    #[tokio::test]
    async fn a_refused_install_abandons_the_intent_and_a_later_registration_starts_fresh() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        pool.with("conductor-a", |c| {
            c.install = Install::Refuses("bundle not found at /app/elohim.happ")
        });

        let error = doorway.provision_agent(USER).await.expect_err("refused");
        assert_eq!(error.code(), "PROVISIONING_FAILED");

        let abandoned = intent_of(&store, USER).await;
        assert_eq!(abandoned.state, IntentState::Abandoned);
        assert!(
            abandoned
                .abandoned_reason
                .as_deref()
                .unwrap_or_default()
                .contains("bundle not found"),
            "{abandoned:?}"
        );
        let counts = store.counts(chrono::Utc::now()).await.unwrap();
        assert!(counts.by_state.contains(&(IntentState::Abandoned, 1)));

        pool.with("conductor-a", |c| c.install = Install::Succeeds);
        let agent = doorway
            .provision_agent(USER)
            .await
            .expect("an abandoned intent does not hold the person");
        assert_eq!(agent.conductor_id, "conductor-a");
        let fresh = intent_of(&store, USER).await;
        assert_eq!(fresh.state, IntentState::Enabled);
        assert_eq!(fresh.abandoned_reason, None);
        assert_eq!(store.len(), 1, "one record per identifier, reused");
        pool.with("conductor-a", |c| assert_eq!(c.apps.len(), 1));
    }

    /// Attempts that never sent an install leave no claim on anyone: once the
    /// re-offers are exhausted the intent is abandoned, not left pinning the
    /// person to a conductor nothing was ever sent to.
    #[tokio::test]
    async fn an_attempt_that_never_sent_an_install_does_not_pin_the_person() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        pool.with("conductor-a", |c| c.dark = true);
        pool.with("conductor-b", |c| c.dark = true);

        let error = doorway.provision_agent(USER).await.expect_err("all dark");
        assert!(matches!(error, ProvisionError::Failed(_)), "{error}");
        assert_eq!(intent_of(&store, USER).await.state, IntentState::Abandoned);
        assert_eq!(pool.total_keys_generated(), 0);

        pool.with("conductor-b", |c| c.dark = false);
        let agent = doorway.provision_agent(USER).await.expect("b is back");
        assert_eq!(agent.conductor_id, "conductor-b");
    }

    /// A person provisioned before the log existed has a mapping and no intent.
    /// They resolve on the MAPPED conductor — nobody else is asked — and the log
    /// is backfilled.
    #[tokio::test]
    async fn a_legacy_mapping_with_no_intent_resolves_on_the_mapped_conductor() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let key = raw_agent_key(210);
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
        doorway
            .registry
            .register_agent(
                &provisioned_agent_key(&key),
                "conductor-b",
                &app_on("conductor-b"),
            )
            .await
            .unwrap();
        pool.with("conductor-a", |c| c.dark = true);

        let agent = doorway
            .provision_agent(USER)
            .await
            .expect("the mapping says where this person lives");

        assert_eq!(agent.conductor_id, "conductor-b");
        assert_eq!(agent.agent_pub_key, provisioned_agent_key(&key));
        assert_eq!(pool.total_keys_generated(), 0);
        assert_eq!(pool.total_installs_received(), 0);
        assert_eq!(
            pool.with("conductor-a", |c| c.connects_received),
            0,
            "the mapped conductor is the only one asked"
        );
        let backfilled = intent_of(&store, USER).await;
        assert_eq!(backfilled.conductor_id, "conductor-b");
        assert_eq!(backfilled.state, IntentState::Enabled);
    }

    /// …and if the mapped conductor cannot be reached, that person waits for it;
    /// they are not given a second identity on the conductor that is up.
    #[tokio::test]
    async fn a_legacy_mapping_on_an_unreachable_conductor_is_unresolved() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        doorway
            .registry
            .register_agent(
                &provisioned_agent_key(&raw_agent_key(211)),
                "conductor-b",
                &app_on("conductor-b"),
            )
            .await
            .unwrap();
        pool.with("conductor-b", |c| c.dark = true);

        let error = doorway.provision_agent(USER).await.expect_err("b is dark");

        assert_eq!(unresolved_on(&error), vec!["conductor-b".to_string()]);
        assert_eq!(pool.total_keys_generated(), 0);
        assert_untouched(&pool, "conductor-a");
    }

    /// An install sent long enough ago that the doorway's own admin request
    /// would have timed out, and still not on the conductor: re-send it — to
    /// the SAME conductor, under the SAME app id, with the SAME recorded key.
    #[tokio::test]
    async fn a_stale_in_flight_install_is_resent_on_the_same_conductor_with_the_same_key() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        an_install_in_flight_on_conductor_a(&pool, &store).await;
        let recorded_key = intent_of(&store, USER).await.agent_pub_key.unwrap();

        // Just inside the bound: still treated as in flight.
        let bound = install_in_flight_bound();
        age_install(&store, USER, bound - std::time::Duration::from_secs(5)).await;
        let doorway = doorway_over(&pool, &store, B_FIRST).await;
        let error = doorway.provision_agent(USER).await.expect_err("in flight");
        assert_eq!(unresolved_on(&error), vec!["conductor-a".to_string()]);
        assert_eq!(pool.total_installs_received(), 1);

        // Past it: re-sent.
        age_install(&store, USER, bound + std::time::Duration::from_secs(1)).await;
        let agent = doorway
            .provision_agent(USER)
            .await
            .expect("the stale install is re-sent where it was first sent");

        assert_eq!(agent.conductor_id, "conductor-a");
        assert_eq!(
            agent.agent_pub_key, recorded_key,
            "the SAME key, not a new one"
        );
        assert_eq!(
            pool.total_keys_generated(),
            1,
            "no second generate_agent_pub_key"
        );
        pool.with("conductor-a", |c| {
            assert_eq!(c.installs_received, 2, "the original and the re-send");
            assert_eq!(c.apps.len(), 1);
        });
        assert_untouched(&pool, "conductor-b");
        assert_eq!(intent_of(&store, USER).await.state, IntentState::Enabled);
    }

    /// The bound is the admin client's own request timeout, read from the
    /// library — not a number written here.
    #[test]
    fn the_in_flight_bound_is_the_admin_request_timeout() {
        assert_eq!(
            install_in_flight_bound(),
            holochain_websocket::WebsocketConfig::CLIENT_DEFAULT.default_request_timeout
        );
    }

    /// A re-send the conductor refuses because the ORIGINAL landed after all
    /// ("already installed") is not a failure: the app is there, with the
    /// recorded key. Adopt it.
    #[tokio::test]
    async fn a_resend_refused_because_the_original_landed_adopts_it() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        an_install_in_flight_on_conductor_a(&pool, &store).await;
        age_install(
            &store,
            USER,
            install_in_flight_bound() + std::time::Duration::from_secs(1),
        )
        .await;
        // The original lands between our lookup and our re-send.
        pool.with("conductor-a", |c| c.lands_before_next_install = true);

        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let agent = doorway.provision_agent(USER).await.expect("adopted");

        assert_eq!(agent.conductor_id, "conductor-a");
        assert_eq!(pool.total_keys_generated(), 1);
        pool.with("conductor-a", |c| {
            assert_eq!(c.apps.len(), 1);
            assert_eq!(c.uninstalls_received, 0);
        });
    }

    /// The doorway's record was lost (no intent, no mapping) but the person's
    /// app is on the conductor a fresh attempt is offered to: the conductor
    /// refuses the duplicate app id, and the existing app is adopted — with ITS
    /// key — rather than reported as a failure.
    #[tokio::test]
    async fn a_lost_record_whose_app_is_on_the_offered_conductor_is_adopted() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let key = raw_agent_key(212);
        pool.with("conductor-a", |c| {
            c.apps.insert(
                app_on("conductor-a"),
                FakeApp {
                    agent_key: key.clone(),
                    status: "disabled",
                    cells_ready: false,
                },
            );
        });

        let agent = doorway.provision_agent(USER).await.expect("adopted");

        assert_eq!(agent.agent_pub_key, provisioned_agent_key(&key));
        pool.with("conductor-a", |c| {
            assert_eq!(c.apps[&app_on("conductor-a")].status, "running");
            assert_eq!(c.uninstalls_received, 0);
        });
        let intent = intent_of(&store, USER).await;
        assert_eq!(intent.state, IntentState::Enabled);
        assert_eq!(
            intent.agent_pub_key.as_deref(),
            Some(agent.agent_pub_key.as_str())
        );
        assert_untouched(&pool, "conductor-b");
    }

    /// The route's progress marks only ever move the record forward.
    #[tokio::test]
    async fn progress_marks_move_the_intent_forward_only() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        doorway.provision_agent(USER).await.unwrap();

        doorway
            .record_progress(USER, IntentState::AccountRecorded)
            .await;
        doorway
            .record_progress(USER, IntentState::HumanCreated)
            .await;

        assert_eq!(
            intent_of(&store, USER).await.state,
            IntentState::AccountRecorded
        );
        // A person the doorway never provisioned has no record to mark.
        doorway
            .record_progress("nobody@example.com", IntentState::HumanCreated)
            .await;
        assert_eq!(store.len(), 1);
    }

    /// A record naming a conductor that is no longer in the pool cannot be
    /// resolved "on that conductor". This — a record that names no usable
    /// conductor — is the one place the request path falls back to the named
    /// recovery scan.
    #[tokio::test]
    async fn an_intent_naming_a_conductor_outside_the_pool_uses_the_recovery_scan() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let mut intent = ProvisionIntent::intended(USER, "conductor-9", "elohim-conductor-9-xx");
        intent.install_sent_at = Some(chrono::Utc::now());
        store.insert_new(&intent).await.unwrap();
        let key = raw_agent_key(213);
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

        let agent = doorway.provision_agent(USER).await.expect("recovered");

        assert_eq!(agent.conductor_id, "conductor-b");
        assert_eq!(pool.total_keys_generated(), 0);
        assert_eq!(intent_of(&store, USER).await.conductor_id, "conductor-b");
    }
}
