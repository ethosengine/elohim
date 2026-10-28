//! Agent Provisioner — orchestrates Holochain agent provisioning
//!
//! When a user registers, the provisioner:
//! 1. Reads its own record of this person ([`super::provision_intent`]). If
//!    one exists, it settles THAT attempt on the conductor it names.
//! 2. Otherwise records the intent, on the least loaded conductor
//! 3. Asks that conductor whether anything is under the app id, and records
//!    that the install is about to be sent after a verified absence
//! 4. Installs the app, letting the conductor mint the agent key, and records
//!    the key it minted
//! 5. Enables the app
//! 6. Registers the agent→conductor mapping
//!
//! The record is a saga log, not an authority: cells exist if the conductor
//! says so, a human exists if the DHT says so, and the log only remembers what
//! this doorway asked for.
//!
//! # An app id is not a person
//!
//! Nothing here treats "an app exists under this id" as "this is that person's
//! app". The id is a string anyone can compute from an identifier, and an
//! identifier is whatever an unauthenticated registration typed. An existing
//! app is this person's only when the conductor's agent key for it equals a key
//! this doorway holds for exactly that identifier: the key in their intent
//! record, or the key on their own account row. Otherwise an app found under
//! the id is a CONFLICT, answered `PROVISIONING_UNRESOLVED` and left for an
//! operator.
//!
//! Apps installed under the old six-hex id are not migrated; they are
//! recognised only by `AgentProvisioner::resolve_account_mapping`, which
//! retires at the recast (plane-separation §11).
//!
//! Every failure of an admin call arrives typed
//! ([`FaultClass`](super::typed_admin::FaultClass)), and what happens next is
//! decided on that type alone: an attempt whose install was never sent may be
//! offered to another conductor; one whose install was sent is settled on the
//! conductor it was sent to. See [`AgentProvisioner::provision_agent`].

use std::sync::Arc;
use tracing::{debug, info, warn};

use super::agent_key::{canonical_agent_key, lookup_forms};
use super::provision_intent::{IntentState, ProvisionIntent, StuckReason, ABANDONED_RECORD_TTL};
use super::registry::{ConductorInfo, ConductorRegistry, StoreVerdict};
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
    /// The doorway cannot provision anyone right now: its intent log cannot be
    /// read or written, so an attempt could not be remembered. Nothing was
    /// asked of any conductor.
    NotReady(String),
}

impl ProvisionError {
    /// The machine-readable code a route puts in front of a client.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unresolved { .. } => "PROVISIONING_UNRESOLVED",
            Self::Failed(_) => "PROVISIONING_FAILED",
            Self::NotReady(_) => "PROVISIONING_NOT_READY",
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
            Self::Failed(message) | Self::NotReady(message) => f.write_str(message),
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
/// without the one thing that prevents a second identity, so it stops — and
/// says the doorway is not ready, which is a different thing from a failure.
fn log_unavailable(error: String) -> ProvisionError {
    ProvisionError::NotReady(format!(
        "the provisioning intent log is not available, so no attempt can be recorded and \
         nothing was sent to any conductor: {error}"
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
    /// Each install has the conductor mint a FRESH agent key, and the app id
    /// embeds the conductor id. So "try the next conductor" is only safe when the previous
    /// attempt cannot have installed anything. The rule, decided on the typed
    /// [`FaultClass`] of the INSTALL:
    ///
    /// - `NotSent` — the install was never written (the connect failed, or the
    ///   pre-install lookup dropped). The record is moved to another conductor and the
    ///   attempt re-offered, bounded at [`PROVISION_TRANSPORT_ATTEMPTS`].
    /// - `SentOutcomeUnknown` — the install was written and its outcome is not
    ///   known. Resolved on the SAME conductor, in this call and in every later
    ///   one (the record says an install was sent, to which conductor, after a
    ///   verified absence of its id there). Never a second key elsewhere.
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
    /// same conductor, same app id, after a fresh verified absence. The re-send
    /// is safe at
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
        self.provision_agent_for_account(user_identifier, None)
            .await
    }

    /// [`Self::provision_agent`], for a caller that already holds an ACCOUNT
    /// for this identifier.
    ///
    /// `account_agent_key` is the agent key on the doorway's own account row
    /// for EXACTLY this identifier (or in a session token this doorway minted
    /// from that row). It is the one piece of evidence, besides an intent
    /// record, that an existing app belongs to this person — see
    /// [`Self::resolve_account_mapping`]. Pass `None` for a registration: a
    /// person with no account has no such evidence.
    pub async fn provision_agent_for_account(
        &self,
        user_identifier: &str,
        account_agent_key: Option<&str>,
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

        self.provision_from_record(user_identifier, account_agent_key)
            .await
    }

    async fn provision_from_record(
        &self,
        user_identifier: &str,
        account_agent_key: Option<&str>,
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
                // An app conflict holds until an operator's recovery proves
                // whose the app is: never adopted, never re-offered elsewhere.
                Some(intent)
                    if intent.is_live() && intent.stuck == Some(StuckReason::AppConflict) =>
                {
                    Step::Done(Err(ProvisionError::unresolved(
                        &intent.conductor_id,
                        format!(
                            "an app under '{}' on {} is not one this doorway can show is this \
                             person's (app conflict); an operator must resolve it",
                            intent.installed_app_id, intent.conductor_id
                        ),
                    )))
                }
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
                // No record, and the store gate is off
                // (`DOORWAY_PROVISIONING_STORE_GATE`, the default): resolve the
                // person as this doorway did before the log existed — an app
                // under their legacy id anywhere in the pool is theirs — and
                // otherwise provision them under a record. Nothing is refused
                // for want of a record.
                None if !self.registry.store_gate() => {
                    match self.legacy_find_existing_app(user_identifier).await {
                        Some(agent) => Step::Done(Ok(agent)),
                        None => self.offer(user_identifier, None).await,
                    }
                }
                // No record, store gate on.
                None => {
                    // An account row's own agent key may name an app from
                    // before the log existed.
                    let from_account = match account_agent_key {
                        Some(key) => self.resolve_account_mapping(user_identifier, key).await,
                        None => None,
                    };
                    match from_account {
                        Some(step) => step,
                        // "No record" means "new" only if the doorway knows its
                        // records are intact. Empty at boot over conductors that
                        // host people is WIPED; empty at boot with a conductor
                        // not yet asked is UNKNOWN. Guessing "new" in either
                        // mints a second identity for everyone who comes back.
                        None => match self.store_refusal() {
                            Some(refusal) => Step::Done(Err(refusal)),
                            // By this doorway's records the person was never
                            // attempted.
                            None => self.offer(user_identifier, None).await,
                        },
                    }
                }
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

    /// Why a person with no record may not be provisioned as new right now, if
    /// there is a reason.
    ///
    /// The two refusals are deliberately different answers:
    ///
    /// - `Wiped` → `PROVISIONING_UNRESOLVED`. This person may have an app the
    ///   doorway lost its record of; an OPERATOR must run recovery.
    /// - `Unknown` → `PROVISIONING_NOT_READY`. The doorway has not finished
    ///   learning its own state: some conductor has not yet answered what it
    ///   hosts. Nothing is known to be unresolved for this person, no operator
    ///   action is needed, and it clears by itself once the named conductors
    ///   answer.
    ///
    /// Neither refuses while the store gate is off
    /// (`DOORWAY_PROVISIONING_STORE_GATE`): the verdict is then observed and
    /// reported, never enforced.
    fn store_refusal(&self) -> Option<ProvisionError> {
        if !self.registry.store_gate() {
            return None;
        }
        let state = self.registry.store_state();
        match state.verdict {
            StoreVerdict::NotWiped => None,
            StoreVerdict::Wiped => Some(ProvisionError::Unresolved {
                conductors: self.pool_conductor_ids(),
                detail: format!(
                    "this doorway's provisioning records were empty at startup while its \
                     conductors already host {} app(s), so it cannot tell whether this person \
                     is new; an operator must run POST /admin/provisioning/recover",
                    state.hosted_apps_seen
                ),
            }),
            StoreVerdict::Unknown => Some(ProvisionError::NotReady(format!(
                "this doorway's provisioning records were empty at startup and it has not yet \
                 been able to ask every conductor what it hosts (not yet answered: {}), so it \
                 cannot tell whether this person is new; nothing was sent to any conductor",
                if state.unwalked.is_empty() {
                    "none — the intent log itself could not be read".to_string()
                } else {
                    state.unwalked.join(", ")
                }
            ))),
        }
    }

    /// Ask every pool conductor that has not yet answered since boot what it
    /// hosts, and record the answer. Returns the conductors STILL unwalked.
    ///
    /// Only a successful listing counts: a conductor that cannot be reached
    /// stays unwalked, and the wiped-store verdict stays `Unknown` because of
    /// it. Agent mappings are registered from what a conductor reports — a
    /// conductor that was down at boot would otherwise never have its hosted
    /// agents routed.
    pub async fn walk_unwalked_conductors(&self) -> Vec<String> {
        let unwalked = self.registry.store_state().unwalked;
        for conductor_id in &unwalked {
            let Some(conductor) = self.registry.get_conductor_info(conductor_id) else {
                continue;
            };
            let listed = match self.connector.connect(&conductor.admin_url).await {
                Ok(admin) => admin.list_apps().await,
                Err(fault) => Err(fault),
            };
            match listed {
                Ok(apps) => {
                    let hosted: Vec<_> = apps
                        .into_iter()
                        .filter(|app| app.installed_app_id != self.app_id)
                        .collect();
                    for app in &hosted {
                        for form in lookup_forms(&app.agent_pub_key) {
                            if self.registry.get_conductor_for_agent(&form).is_none() {
                                let _ = self
                                    .registry
                                    .register_agent(&form, conductor_id, &app.installed_app_id)
                                    .await;
                            }
                        }
                    }
                    self.registry.mark_walked(conductor_id, hosted.len());
                    info!(
                        conductor = %conductor_id,
                        hosted_apps = hosted.len(),
                        verdict = self.registry.store_state().verdict.as_str(),
                        "Conductor walked: it answered what it hosts"
                    );
                }
                Err(fault) => debug!(
                    conductor = %conductor_id,
                    error = %fault,
                    "Conductor not walked yet — it could not be asked what it hosts"
                ),
            }
        }
        self.registry.store_state().unwalked
    }

    /// THE STORE-GATE-OFF LOOKUP — how this doorway resolved a person before
    /// the intent log existed, kept so that turning the log on changes nothing
    /// for people provisioned before it.
    ///
    /// Asks each pool conductor for an app under the person's id — the current
    /// full-hash id, then the LEGACY short id — and adopts the first it finds.
    /// The full-hash id matters on a doorway with no Mongo (the log is process
    /// memory): after a restart its records are gone, and the apps it installed
    /// under full-hash ids must still be found, on whichever conductor holds
    /// them, as dev found its short-id apps. A conductor that cannot be asked
    /// is skipped, as it always was. This is adoption by app id — the very thing
    /// [`Self::finish_forward`] refuses — and it runs only while the store gate
    /// is off; with the gate on, an app without a record is adopted only on
    /// proof ([`Self::resolve_account_mapping`]). No record is written for what
    /// it finds: an app id is not proof, and a record is.
    async fn legacy_find_existing_app(&self, user_identifier: &str) -> Option<ProvisionedAgent> {
        for conductor in &self.registry.list_conductors() {
            let Ok(admin) = self.connector.connect(&conductor.admin_url).await else {
                continue;
            };
            for app_id in [
                generate_app_id(&self.app_id, &conductor.conductor_id, user_identifier),
                legacy_short_app_id(&self.app_id, &conductor.conductor_id, user_identifier),
            ] {
                if let Ok(Some(existing)) = admin.find_app(&app_id).await {
                    info!(
                        conductor = %conductor.conductor_id,
                        app_id = %app_id,
                        user = %user_identifier,
                        "Reusing an app under this person's id with no record (store gate off)"
                    );
                    return Some(
                        self.adopt_existing(conductor, app_id, &existing.agent_pub_key)
                            .await,
                    );
                }
            }
        }
        None
    }

    /// Whether the store gate (`DOORWAY_PROVISIONING_STORE_GATE`) is on.
    pub fn store_gate(&self) -> bool {
        self.registry.store_gate()
    }

    /// Whether a person with no record is refused until the records are
    /// recovered: the store gate is on and the verdict reads `wiped`.
    pub fn recovery_required(&self) -> bool {
        self.registry.recovery_required()
    }

    fn pool_conductor_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .registry
            .list_conductors()
            .into_iter()
            .map(|c| c.conductor_id)
            .collect();
        ids.sort();
        ids
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

            // WRITE IT DOWN FIRST. Before the install is sent, before the
            // conductor is even dialled.
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

        // 3. Ask the conductor whether anything is already under this id —
        // BEFORE writing anything identity-bearing. The conductor mints the
        // agent key at install (`ProvisionAdmin::install_app`), so this doorway
        // holds no key until the install answers; this verified absence is what
        // lets an install whose answer was lost be recognised later as the one
        // this record sent (`finish_forward`). A transport fault here leaves the
        // install unsent.
        match admin.find_app(&installed_app_id).await {
            Ok(None) => {}
            Ok(Some(_)) => {
                // Not installed by this attempt, and an id is not a person.
                self.mark_stuck(intent, StuckReason::AppConflict).await;
                return Err(AttemptError::Final(ProvisionError::unresolved(
                    &conductor.conductor_id,
                    format!(
                        "an app already exists on {} under '{installed_app_id}' and this \
                         doorway has no record that it belongs to this person; an operator \
                         must resolve it",
                        conductor.conductor_id
                    ),
                )));
            }
            Err(fault) => {
                return Err(AttemptFault::new(
                    before_install(fault.class),
                    format!(
                        "Failed to ask {} whether '{installed_app_id}' exists: {}",
                        conductor.conductor_id, fault
                    ),
                )
                .into());
            }
        }

        // Record that the install is about to be written, after a verified
        // absence — BEFORE writing it. If the doorway dies on the next line, the
        // record already says an install may be on this conductor.
        intent.install_sent_at = Some(intent_now());
        intent.absent_before_send = true;
        match self.save(intent).await {
            Ok(true) => {}
            Ok(false) => return Err(AttemptError::LostClaim),
            Err(e) => return Err(AttemptError::Log(e)),
        }

        // 4. Install app; the conductor mints the key. From the moment this is
        // written, the person may have an identity on THIS conductor — the
        // fault's own class is the answer.
        let agent_key = match admin
            .install_app(&installed_app_id, &self.bundle_path)
            .await
        {
            Ok(agent_key) => agent_key,
            Err(fault) => {
                let message = format!(
                    "Failed to install app on {}: {}",
                    conductor.conductor_id, fault
                );
                return Err(match fault.class {
                    // The socket was gone before the request was written. Take
                    // the "sent" back — unless the record cannot be corrected,
                    // in which case it stays, and is resolved here like any
                    // sent install.
                    FaultClass::NotSent => {
                        intent.install_sent_at = None;
                        intent.absent_before_send = false;
                        intent.agent_pub_key = None;
                        match self.save(intent).await {
                            Ok(true) => AttemptFault::new(FaultClass::NotSent, message),
                            _ => AttemptFault::new(FaultClass::SentOutcomeUnknown, message),
                        }
                        .into()
                    }
                    FaultClass::SentOutcomeUnknown => {
                        AttemptFault::new(FaultClass::SentOutcomeUnknown, message).into()
                    }
                    // The conductor answered. What is on it now decides the
                    // rest, and only the conductor can say.
                    FaultClass::Refused => {
                        let found = admin.find_app(&installed_app_id).await;
                        // A DEFINITE refusal: this record's install did not
                        // land. The absence it recorded before sending is no
                        // evidence about anything under the id from now on, so
                        // it is withdrawn — persisted, so a later request can
                        // never adopt what is there on the strength of it.
                        intent.absent_before_send = false;
                        match found {
                            // Nothing there: a definite refusal that left nothing.
                            Ok(None) => AttemptFault::verdict(message).into(),
                            // An app exists under this id now. THIS install was
                            // refused, so it is not the one this record sent, and
                            // an id is not a person: a conflict — never an
                            // adoption, on this request or any later one, until an
                            // operator's recovery proves whose it is.
                            Ok(Some(_)) => {
                                intent.stuck = Some(StuckReason::AppConflict);
                                self.note(intent).await;
                                AttemptError::Final(ProvisionError::unresolved(
                                    &conductor.conductor_id,
                                    format!(
                                    "an app appeared on {} under '{installed_app_id}' while this \
                                     doorway's own install of it was refused, so this doorway \
                                     has no evidence it belongs to this person; an operator \
                                     must resolve it",
                                    conductor.conductor_id
                                ),
                                ))
                            }
                            // The lookup itself failed: nothing is known about what
                            // is on the conductor. Never abandon on an unknown.
                            Err(lookup) => {
                                self.note(intent).await;
                                AttemptError::Final(ProvisionError::unresolved(
                                    &conductor.conductor_id,
                                    format!(
                                        "{message}; and {} did not answer whether \
                                     '{installed_app_id}' exists ({lookup})",
                                        conductor.conductor_id
                                    ),
                                ))
                            }
                        }
                    }
                });
            }
        };
        // The key the conductor minted is this person's from here on: it is
        // the proof every later lookup checks an app against.
        intent.agent_pub_key = Some(provisioned_agent_key(&agent_key));
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
    /// SAME app id. Only ever called right after that conductor answered that
    /// no app exists under the id, so the re-send is again preceded by a
    /// verified absence (`absent_before_send`). The conductor mints the key, as
    /// on the first send; a conductor refuses a second app under one id, so
    /// whichever of the two installs lands is the only identity there.
    ///
    /// This is also where a record that can never be finished ENDS. Each time a
    /// reachable conductor answers "that app is not here" after a re-send it
    /// refused, the record counts one verified absence. The checks are at
    /// least one in-flight bound apart by construction. At
    /// [`ABSENT_CHECKS_BEFORE_ABANDON`] the record is abandoned: nothing landed.
    /// A re-send whose own outcome is unknown, or a conductor that does not
    /// answer, never counts.
    async fn resend_install(
        &self,
        admin: &dyn ProvisionAdmin,
        conductor: &ConductorInfo,
        intent: &mut ProvisionIntent,
    ) -> Result<ProvisionedAgent, ProvisionError> {
        let conductor_id = conductor.conductor_id.as_str();
        let installed_app_id = intent.installed_app_id.clone();
        let unresolved = |detail: String| ProvisionError::unresolved(conductor_id, detail);

        // Stamp the attempt first, as on the first send. This is also what
        // spaces the absence checks one bound apart. The caller has just seen
        // the id absent on this conductor.
        intent.install_sent_at = Some(intent_now());
        intent.absent_before_send = true;
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
            "Re-sending an install that never appeared — same conductor, same app id"
        );
        // `Some(refused)` only for a DEFINITE refusal; an accepted re-send is
        // `None`.
        let refused: Option<String> = match admin
            .install_app(&installed_app_id, &self.bundle_path)
            .await
        {
            Ok(minted) => {
                // Only a record whose earlier install never answered reaches a
                // re-send, so it holds no key yet; this one is the proof now.
                intent.agent_pub_key = Some(provisioned_agent_key(&minted));
                None
            }
            Err(fault) if fault.class == FaultClass::Refused => Some(fault.message),
            Err(fault) => {
                // Sent, outcome unknown — or never written. Either way not a
                // verified absence; look, and report.
                return match admin.find_app(&installed_app_id).await {
                    Ok(Some(existing)) => {
                        self.finish_forward(admin, conductor, intent, existing)
                            .await
                    }
                    _ => Err(unresolved(format!(
                        "the re-sent install of '{installed_app_id}' on {conductor_id} \
                         has an unknown outcome ({fault})"
                    ))),
                };
            }
        };

        // Whatever the re-send answered, the conductor's app list is the
        // truth. A refusal here most often means the ORIGINAL landed.
        match admin.find_app(&installed_app_id).await {
            Ok(Some(existing)) => {
                self.finish_forward(admin, conductor, intent, existing)
                    .await
            }
            Ok(None) => match refused {
                // Accepted but not listed yet: wait for it.
                None => {
                    self.note(intent).await;
                    Err(unresolved(format!(
                        "the re-sent install of '{installed_app_id}' was accepted by \
                         {conductor_id} but the app is not listed yet"
                    )))
                }
                // Refused, and the conductor confirms nothing is there: one
                // verified absence.
                Some(reason) => {
                    intent.absent_past_bound += 1;
                    if intent.absent_past_bound >= ABSENT_CHECKS_BEFORE_ABANDON {
                        let ended = format!(
                            "'{installed_app_id}' was verifiably absent on {conductor_id} at \
                             {} checks, each at least {}s apart, and could not be installed \
                             ({reason}); nothing landed",
                            intent.absent_past_bound,
                            install_in_flight_bound().as_secs()
                        );
                        self.abandon_if_live(Some(intent), &ended).await;
                        return Err(ProvisionError::Failed(ended));
                    }
                    self.note(intent).await;
                    Err(unresolved(format!(
                        "the install of '{installed_app_id}' is not on {conductor_id} and could \
                         not be made there ({reason}); verified absent {} of {} times before \
                         this record is given up",
                        intent.absent_past_bound, ABSENT_CHECKS_BEFORE_ABANDON
                    )))
                }
            },
            Err(fault) => Err(unresolved(format!(
                "the install of '{installed_app_id}' was re-sent to {conductor_id}, which did \
                 not answer the lookup ({fault})"
            ))),
        }
    }

    /// An app exists on `conductor` under this record's id. Prove it is this
    /// person's, bring it to ready, and hand it back.
    ///
    /// PROOF FIRST. The app is this person's only if the conductor's agent key
    /// for it EQUALS the key this doorway recorded for them (the key the
    /// conductor minted when this record's install answered). The app id proves
    /// nothing on its own — an id is a string anyone can compute, and it used
    /// to be 24 bits wide. On a mismatch the record is left exactly as it is
    /// (the recorded key is never overwritten with the conductor's) and marked
    /// stuck.
    ///
    /// One case has no recorded key: an install whose answer was lost, so the
    /// key the conductor minted never reached the doorway. That install is
    /// recognised only if the record says the conductor had answered "nothing
    /// under this id" immediately before it was sent (`absent_before_send`);
    /// the key found is then recorded and is the proof from then on. Without
    /// that evidence the record is stuck (`NoUsableKey`), never adopted.
    ///
    /// Then: installed is not ready. A disabled app is enabled, an app in any
    /// other non-running state is reported, and cells are waited for. Never
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

        let conductors_key = provisioned_agent_key(&existing.agent_pub_key);
        let recorded_key = match intent.agent_pub_key.clone() {
            Some(recorded_key) => recorded_key,
            // The install's answer — and with it the key the conductor minted —
            // was lost. The record says the conductor answered "nothing under
            // this id" immediately before this record's install was written, so
            // the app now under the id on this conductor is that install. Record
            // its key; from here on the key is the proof.
            None if intent.absent_before_send
                && intent.install_sent_at.is_some()
                && intent.stuck != Some(StuckReason::AppConflict) =>
            {
                info!(
                    conductor = %conductor_id,
                    app_id = %installed_app_id,
                    "Recording the agent key the conductor minted for an install whose answer \
                     was lost (the id was verified absent before it was sent)"
                );
                intent.agent_pub_key = Some(conductors_key.clone());
                conductors_key.clone()
            }
            None => {
                if intent.stuck != Some(StuckReason::AppConflict) {
                    self.mark_stuck(intent, StuckReason::NoUsableKey).await;
                }
                return Err(ProvisionError::unresolved(
                    conductor_id,
                    format!(
                        "'{installed_app_id}' exists on {conductor_id} but this doorway recorded \
                         no agent key for this person's install, and no verified absence before \
                         it was sent, so it cannot be shown to be theirs"
                    ),
                ));
            }
        };
        if conductors_key != recorded_key {
            // Keep the more specific name if this record is already known to be
            // a first-send conflict.
            if intent.stuck != Some(StuckReason::AppConflict) {
                self.mark_stuck(intent, StuckReason::KeyMismatch).await;
            }
            return Err(ProvisionError::unresolved(
                conductor_id,
                format!(
                    "'{installed_app_id}' exists on {conductor_id} with an agent key that is \
                     not the agent key this doorway recorded for this person; it is not \
                     treated as theirs"
                ),
            ));
        }

        match existing.status.as_deref() {
            Some("running") => {}
            Some("disabled") => {
                if let Err(fault) = admin.enable_app(&installed_app_id).await {
                    // Only a refusal is a property of the app; a dropped socket
                    // says nothing about it.
                    if fault.class == FaultClass::Refused {
                        self.mark_stuck(intent, StuckReason::AppUnusable).await;
                    }
                    return Err(ProvisionError::unresolved(
                        conductor_id,
                        format!(
                            "'{installed_app_id}' is installed on {conductor_id} but disabled, \
                             and could not be enabled ({fault})"
                        ),
                    ));
                }
            }
            other => {
                self.mark_stuck(intent, StuckReason::AppUnusable).await;
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
            self.mark_stuck(intent, StuckReason::CellsNotReady).await;
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

        intent.stuck = None;
        intent.absent_past_bound = 0;
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

    /// THE ONE LEGACY LOOKUP — retires at the recast (plane-separation §11).
    ///
    /// A person provisioned before the intent log existed has no record, and
    /// their app may sit under an OLD short app id (six hex digits of the
    /// identifier's hash — 24 bits, shared by unrelated identifiers). That id is
    /// never recomputed and never matched on. The only way such an app is
    /// recognised is through a mapping that already names the person exactly:
    ///
    /// - `account_agent_key` — the agent key on the doorway's own account row
    ///   for EXACTLY this identifier (the caller supplies it from that row);
    /// - the doorway's persisted agent→conductor mapping for THAT key, which
    ///   gives the conductor and the app id as they were installed;
    /// - and the conductor confirming that the app's agent key IS that key.
    ///
    /// A lookup in the doorway's OWN records — no conductor is dialled to find
    /// the mapping, and only the mapped conductor is dialled to confirm it.
    /// `None` = no mapping for that key (or a stale one the conductor
    /// disowned): by this doorway's records the person was never attempted.
    async fn resolve_account_mapping(
        &self,
        user_identifier: &str,
        account_agent_key: &str,
    ) -> Option<Step> {
        let entry = self.registry.get_conductor_for_agent(account_agent_key)?;
        let installed_app_id = entry.app_id.clone();
        let unresolved = |conductor_id: &str, detail: String| {
            Some(Step::Done(Err(ProvisionError::unresolved(
                conductor_id,
                detail,
            ))))
        };
        let Some(conductor) = self.registry.get_conductor_info(&entry.conductor_id) else {
            return unresolved(
                &entry.conductor_id,
                format!(
                    "this person's account is mapped to '{installed_app_id}' on {}, which is \
                     not in this doorway's pool",
                    entry.conductor_id
                ),
            );
        };
        let conductor_id = conductor.conductor_id.as_str();

        let admin = match self.connector.connect(&conductor.admin_url).await {
            Ok(admin) => admin,
            Err(fault) => {
                return unresolved(
                    conductor_id,
                    format!(
                        "this person's account is mapped to '{installed_app_id}' on \
                         {conductor_id}, which could not be reached ({fault})"
                    ),
                );
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
                return unresolved(
                    conductor_id,
                    format!(
                        "this person's account is mapped to '{installed_app_id}' on \
                         {conductor_id}, which did not answer the lookup ({fault})"
                    ),
                );
            }
        };

        let account_key = super::agent_key::normalize_agent_key(account_agent_key);
        if provisioned_agent_key(&existing.agent_pub_key) != account_key {
            return unresolved(
                conductor_id,
                format!(
                    "this person's account is mapped to '{installed_app_id}' on {conductor_id}, \
                     but that app's agent key is not the account's agent key; it is not \
                     treated as theirs"
                ),
            );
        }

        info!(
            conductor = %conductor_id,
            app_id = %installed_app_id,
            user = %user_identifier,
            status = ?existing.status,
            "Reusing existing app installation (account mapping, no intent record) — backfilling"
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
    /// resolved "on that conductor". The pool is scanned for an app that
    /// carries the RECORDED key; if none does, the record is stuck by name.
    ///
    /// It is not abandoned: the conductor it names cannot be asked, so whether
    /// anything landed there is unverified.
    async fn recover_unplaceable_record(
        &self,
        mut intent: ProvisionIntent,
    ) -> Result<ProvisionedAgent, ProvisionError> {
        warn!(
            user = %intent.identifier,
            recorded_conductor = %intent.conductor_id,
            "Provisioning record names a conductor that is not in the pool"
        );
        let named = intent.conductor_id.clone();
        let found = match intent.agent_pub_key.clone() {
            Some(recorded_key) => {
                self.recover_by_pool_scan(&intent.identifier, &recorded_key)
                    .await
            }
            None => Existence::NotInstalled,
        };
        match found {
            Existence::Found(agent) => {
                intent.conductor_id = agent.conductor_id.clone();
                intent.installed_app_id = agent.installed_app_id.clone();
                intent.stuck = None;
                if intent.state < IntentState::Enabled {
                    intent.state = IntentState::Enabled;
                }
                self.note(&mut intent).await;
                Ok(agent)
            }
            Existence::NotInstalled | Existence::Unresolved { .. } => {
                self.mark_stuck(&mut intent, StuckReason::ConductorNotInPool)
                    .await;
                Err(ProvisionError::unresolved(
                    &named,
                    format!(
                        "this person's provisioning is recorded on {named}, which is not in \
                         this doorway's pool, and no conductor in the pool holds an app with \
                         the agent key recorded for them; an operator must resolve it"
                    ),
                ))
            }
        }
    }

    /// RECOVERY PATH — not on the request path. Ask EVERY conductor in the pool
    /// whether it holds this person's app.
    ///
    /// This is how the doorway used to answer "does this person already have an
    /// app?" on every registration, which made every registration depend on
    /// every conductor. It is kept for a record that names a conductor no
    /// longer in the pool ([`Self::recover_unplaceable_record`], its only
    /// caller on the request path).
    ///
    /// `proof_key` is the agent key recorded for this person. An app found
    /// under the person's id is adopted ONLY if the conductor's key for it
    /// equals `proof_key` — the scan never adopts on an id alone.
    ///
    /// A conductor that cannot be reached, or does not answer the lookup, is
    /// NOT a conductor without the app: [`Existence::Unresolved`] names it.
    pub async fn recover_by_pool_scan(&self, user_identifier: &str, proof_key: &str) -> Existence {
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
                    if provisioned_agent_key(&existing.agent_pub_key) != proof_key {
                        warn!(
                            conductor = %conductor.conductor_id,
                            app_id = %app_id,
                            "Recovery scan found an app under this id whose agent key is not \
                             the recorded key — not adopted"
                        );
                        continue;
                    }
                    info!(
                        conductor = %conductor.conductor_id,
                        app_id = %app_id,
                        user = %user_identifier,
                        status = ?existing.status,
                        "Recovery scan found this person's app (agent key confirmed)"
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

    /// OPERATOR RECOVERY — rebuild this doorway's records from the conductors.
    ///
    /// For a doorway whose database was wiped or restored while its conductors
    /// kept their apps. Invoked by `POST /admin/provisioning/recover`, never by
    /// a registration.
    ///
    /// - **Mappings** (agent key → conductor, app) are rebuilt from what each
    ///   conductor reports. Those are facts about the conductor and need no
    ///   proof.
    /// - **Records** (identifier → app) are rebuilt ONLY where an account row
    ///   this doorway holds carries an agent key a conductor confirms.
    ///   `accounts` is `(identifier, agent key)` from the doorway's own account
    ///   rows. An account whose key no conductor holds gets no record; an app
    ///   no account claims is reported and assigned to nobody — the doorway has
    ///   no evidence whose it is.
    ///
    /// The walk clears `recovery_required` only if it is COMPLETE: every
    /// conductor answered and every record an account proved was written.
    pub async fn rebuild_from_conductors(&self, accounts: &[(String, String)]) -> RecoveryReport {
        let mut report = self.rebuild_records(accounts, RecordWrite::Replace).await;
        if report.complete {
            self.registry.mark_recovered();
            self.registry.mark_records_backfilled();
        }
        self.fill_store_state(&mut report);
        self.fill_log_state(&mut report).await;
        info!(
            complete = report.complete,
            apps_seen = report.apps_seen,
            records_rebuilt = report.records_rebuilt.len(),
            unreachable = ?report.conductors_unreachable,
            "Operator recovery walked the conductor pool"
        );
        report
    }

    /// AUTOMATIC BACKFILL — the same walk as the operator's recovery, run by
    /// the doorway itself at every boot with the store gate on, whatever the
    /// verdict, and before the record-dependent node/device login check is
    /// enforced (`ConductorRegistry::records_backfilled`). It only INSERTS a
    /// record where none exists.
    ///
    /// On a normal upgrade the agent mappings survive and the verdict reads
    /// `not_wiped`, yet no account has a record; without this, every legacy
    /// node/device account whose key names a hosted app would be refused at
    /// login the moment the gate is on.
    ///
    /// `Wiped` means "no records at boot, and the conductors host apps". That
    /// is also exactly what the FIRST boot with the intent log looks like on a
    /// doorway that already hosts people, so the verdict alone cannot tell a
    /// new log from a lost one. The account rows can: an account whose agent
    /// key a conductor confirms is positive evidence that this doorway's
    /// records of that person survived, and it is the proof the operator's
    /// recovery already rebuilds from. So the migration:
    ///
    /// - rebuilds a record for every account a conductor confirms (account key
    ///   EQUALS the conductor's key for an app — never an app id);
    /// - clears the verdict only if the walk is COMPLETE (every conductor
    ///   answered, every proven record written) AND at least one account was
    ///   proven. A database that lost its accounts too proves nobody: it stays
    ///   `Wiped`, and that is a genuine loss for an operator.
    ///
    /// An app no account claims is assigned to nobody, as in the operator's
    /// recovery.
    pub async fn migrate_from_accounts(&self, accounts: &[(String, String)]) -> RecoveryReport {
        let mut report = self
            .rebuild_records(accounts, RecordWrite::InsertOnly)
            .await;
        // Records now exist for every account a conductor confirms: from here
        // the record-dependent node/device login check may be enforced.
        if report.complete {
            self.registry.mark_records_backfilled();
        }
        let cleared = report.complete && report.accounts_proven > 0;
        if cleared {
            self.registry.mark_recovered();
        }
        self.fill_store_state(&mut report);
        self.fill_log_state(&mut report).await;
        info!(
            complete = report.complete,
            accounts_proven = report.accounts_proven,
            records_rebuilt = report.records_rebuilt.len(),
            records_not_written = report.records_not_written.len(),
            apps_without_an_account = report.apps_without_an_account.len(),
            unreachable = ?report.conductors_unreachable,
            cleared,
            "First-use provisioning migration walked the conductor pool"
        );
        report
    }

    /// The walk both recoveries share: rebuild mappings from every conductor,
    /// and records from the accounts a conductor confirms. Marks nothing.
    async fn rebuild_records(
        &self,
        accounts: &[(String, String)],
        write: RecordWrite,
    ) -> RecoveryReport {
        let mut report = RecoveryReport::default();
        // canonical agent key → (conductor, app id)
        let mut seen: std::collections::HashMap<String, (String, String)> =
            std::collections::HashMap::new();

        let mut conductors = self.registry.list_conductors();
        conductors.sort_by(|a, b| a.conductor_id.cmp(&b.conductor_id));
        for conductor in &conductors {
            let listed = match self.connector.connect(&conductor.admin_url).await {
                Ok(admin) => admin.list_apps().await,
                Err(fault) => Err(fault),
            };
            let apps = match listed {
                Ok(apps) => apps,
                Err(fault) => {
                    warn!(
                        conductor = %conductor.conductor_id,
                        error = %fault,
                        "Recovery could not list this conductor's apps"
                    );
                    report
                        .conductors_unreachable
                        .push(conductor.conductor_id.clone());
                    continue;
                }
            };
            report
                .conductors_walked
                .push(conductor.conductor_id.clone());
            self.registry.mark_walked(
                &conductor.conductor_id,
                apps.iter()
                    .filter(|app| app.installed_app_id != self.app_id)
                    .count(),
            );
            for app in apps {
                // The doorway's own app is not a hosted person's.
                if app.installed_app_id == self.app_id {
                    continue;
                }
                report.apps_seen += 1;
                for form in lookup_forms(&app.agent_pub_key) {
                    if self
                        .registry
                        .register_agent(&form, &conductor.conductor_id, &app.installed_app_id)
                        .await
                        .is_ok()
                    {
                        report.mappings_written += 1;
                    }
                }
                seen.insert(
                    provisioned_agent_key(&app.agent_pub_key),
                    (conductor.conductor_id.clone(), app.installed_app_id),
                );
            }
        }

        let mut claimed: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (identifier, account_key) in accounts {
            let key = super::agent_key::normalize_agent_key(account_key);
            let Some((conductor_id, installed_app_id)) = seen.get(&key) else {
                report.accounts_without_an_app.push(identifier.clone());
                continue;
            };
            claimed.insert(key.clone());
            report.accounts_proven += 1;

            let mut rebuilt = ProvisionIntent::intended(identifier, conductor_id, installed_app_id);
            rebuilt.agent_pub_key = Some(key.clone());
            rebuilt.state = IntentState::AccountRecorded;
            let store = self.registry.intents();
            let written = match store.get(identifier).await {
                Ok(None) => store.insert_new(&rebuilt).await,
                // Already the same proven fact: leave it.
                Ok(Some(existing))
                    if existing.is_live()
                        && existing.stuck.is_none()
                        && existing.agent_pub_key.as_deref() == Some(key.as_str())
                        && existing.installed_app_id == *installed_app_id =>
                {
                    Ok(true)
                }
                // The automatic backfill writes only where nothing is
                // recorded: it runs at every boot with the gate on, over
                // records that are live and moving.
                Ok(Some(_)) if write == RecordWrite::InsertOnly => {
                    report.records_kept.push(identifier.clone());
                    continue;
                }
                // Abandoned, stuck, or pointing elsewhere: the conductor-
                // confirmed account key wins (the operator's recovery only).
                Ok(Some(existing)) => {
                    rebuilt.version = existing.version + 1;
                    rebuilt.created_at = existing.created_at;
                    store.replace(existing.version, &rebuilt).await
                }
                Err(e) => Err(e),
            };
            match written {
                Ok(true) => report.records_rebuilt.push(identifier.clone()),
                Ok(false) => report.records_not_written.push(format!(
                    "{identifier}: the record changed while it was being rebuilt"
                )),
                Err(e) => report
                    .records_not_written
                    .push(format!("{identifier}: {e}")),
            }
        }

        let mut unclaimed: Vec<String> = seen
            .iter()
            .filter(|(key, _)| !claimed.contains(*key))
            .map(|(_, (conductor_id, app_id))| format!("{conductor_id}/{app_id}"))
            .collect();
        unclaimed.sort();
        report.apps_without_an_account = unclaimed;

        // Only a walk that reached EVERY conductor AND wrote every record an
        // account proved is a recovery. A partial one clears nothing: `Wiped`
        // stays wiped, and `Unknown` stays unknown for the conductors that did
        // not answer — recovery cannot clear what it could not ask, nor what it
        // could not write down.
        report.complete =
            report.conductors_unreachable.is_empty() && report.records_not_written.is_empty();
        report
    }

    /// The operator's explicit decision that a stuck record is to be given up.
    ///
    /// The doorway never abandons a record while something may be on a
    /// conductor; an operator who has looked may. Returns the identifiers
    /// actually abandoned.
    pub async fn operator_abandon(&self, identifiers: &[String]) -> Vec<String> {
        let mut abandoned = Vec::new();
        for identifier in identifiers {
            if let Ok(Some(mut intent)) = self.registry.intents().get(identifier).await {
                if intent.is_live() {
                    self.abandon_if_live(Some(&mut intent), "abandoned by an operator")
                        .await;
                    if !intent.is_live() {
                        abandoned.push(identifier.clone());
                    }
                }
            }
        }
        abandoned
    }

    /// Add the log's own state to a recovery report: what is stuck, and why.
    async fn fill_log_state(&self, report: &mut RecoveryReport) {
        let store = self.registry.intents();
        report.intent_store_available = store.available();
        match store.stuck_records().await {
            Ok(stuck) => {
                report.stuck = stuck
                    .into_iter()
                    .map(|intent| StuckRecord {
                        identifier: intent.identifier,
                        conductor_id: intent.conductor_id,
                        installed_app_id: intent.installed_app_id,
                        reason: intent.stuck.map(StuckReason::as_str).unwrap_or("unknown"),
                        state: intent.state.as_str(),
                        absent_past_bound: intent.absent_past_bound,
                    })
                    .collect();
            }
            Err(e) => report.log_errors.push(e),
        }
        match store.counts(intent_now() - in_flight_bound_chrono()).await {
            Ok(counts) => {
                report.stale = counts.stale;
                report.abandoned = counts
                    .by_state
                    .iter()
                    .find(|(state, _)| *state == IntentState::Abandoned)
                    .map(|(_, n)| *n)
                    .unwrap_or(0);
            }
            Err(e) => report.log_errors.push(e),
        }
    }

    /// The log's state without walking any conductor.
    pub async fn recovery_status(&self) -> RecoveryReport {
        let mut report = RecoveryReport::default();
        self.fill_store_state(&mut report);
        self.fill_log_state(&mut report).await;
        report
    }

    fn fill_store_state(&self, report: &mut RecoveryReport) {
        let state = self.registry.store_state();
        report.store_gate = self.registry.store_gate();
        report.recovery_required = self.registry.recovery_required();
        report.store_verdict = state.verdict.as_str();
        report.conductors_unwalked = state.unwalked;
        report.hosted_apps_seen = state.hosted_apps_seen;
    }

    // ── the log ─────────────────────────────────────────────────────────────

    /// Write `intent` against the version it was read at. `Ok(false)` = another
    /// writer moved the record; `intent` is left as it was.
    async fn save(&self, intent: &mut ProvisionIntent) -> Result<bool, String> {
        let expected = intent.version;
        let mut next = intent.clone();
        next.version = expected + 1;
        next.updated_at = intent_now();
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

    /// End an attempt that left nothing behind. The record stays, countable,
    /// until it expires.
    async fn abandon_if_live(&self, intent: Option<&mut ProvisionIntent>, reason: &str) {
        let Some(intent) = intent else { return };
        if !intent.is_live() {
            return;
        }
        intent.state = IntentState::Abandoned;
        intent.abandoned_reason = Some(reason.to_string());
        intent.install_sent_at = None;
        intent.absent_before_send = false;
        intent.agent_pub_key = None;
        intent.stuck = None;
        intent.expire_at = chrono::Duration::from_std(ABANDONED_RECORD_TTL)
            .ok()
            .map(|ttl| intent_now() + ttl);
        self.note(intent).await;
    }

    /// Name why a live record cannot be finished without an operator. Written
    /// only when it changes, so a record that stays stuck is not rewritten on
    /// every call.
    async fn mark_stuck(&self, intent: &mut ProvisionIntent, reason: StuckReason) {
        if intent.stuck == Some(reason) {
            return;
        }
        intent.stuck = Some(reason);
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
    }

    /// Did THIS doorway provision `agent` for EXACTLY `user_identifier`?
    ///
    /// True only when a live record for that identifier names the same
    /// conductor, the same app id and the same agent key. This is the evidence
    /// a route needs before it attaches something that already exists on that
    /// agent — an existing Human — to a registration. The app id alone, or an
    /// agent some other identifier was provisioned, is not evidence.
    pub async fn provisioned_for(&self, user_identifier: &str, agent: &ProvisionedAgent) -> bool {
        match self.registry.intents().get(user_identifier).await {
            Ok(Some(intent)) => {
                intent.is_live()
                    && intent.conductor_id == agent.conductor_id
                    && intent.installed_app_id == agent.installed_app_id
                    && intent.agent_pub_key.as_deref() == Some(agent.agent_pub_key.as_str())
            }
            _ => false,
        }
    }

    /// Does THIS doorway host anyone under `user_identifier`? Never installs,
    /// never scans the pool, never reads an app id as a person.
    ///
    /// The `node`/`device` register arms ask this instead of provisioning: a
    /// human whose identity lives on their own node or device is not hosted
    /// here, and installing an app for them minted a second agent on the
    /// operator's conductor (household 2026-10-14: James and Jessica each held a
    /// second 5-cell app on matthew's conductor). Those arms never ADOPT what
    /// they find either: a registration body is unauthenticated, so an agent
    /// this doorway hosts is never handed to it on the strength of an
    /// identifier.
    ///
    /// Three answers, and the third is not folded into the first:
    ///
    /// - `Ok(None)` — this doorway's records are intact and hold no live
    ///   record for the identifier: it hosts nobody under it.
    /// - `Ok(Some(conductor_id))` — a live record says this doorway provisioned
    ///   (or is provisioning) a hosted identity for the identifier on that
    ///   conductor.
    /// - `Err(_)` — unknown: the log cannot be read, or it was empty at boot
    ///   over conductors that host people (wiped) or that have not all answered
    ///   (unknown). "No record" is not "nobody" in either.
    pub async fn hosted_here(
        &self,
        user_identifier: &str,
    ) -> Result<Option<String>, ProvisionError> {
        match self.registry.intents().get(user_identifier).await {
            Err(e) => Err(log_unavailable(e)),
            Ok(Some(intent)) if intent.is_live() => Ok(Some(intent.conductor_id)),
            Ok(_) => match self.store_refusal() {
                Some(refusal) => Err(refusal),
                None => Ok(None),
            },
        }
    }

    /// Does a live record for EXACTLY `user_identifier` name `agent_pub_key` on
    /// the conductor and app this doorway's registry maps that key to?
    ///
    /// The one piece of evidence that lets a node/device account whose key names
    /// a hosted app log in (an operator's recovery re-bound it). Anything that
    /// cannot be read is `false`: the login is refused, never waved through.
    pub async fn records_agent_for(&self, user_identifier: &str, agent_pub_key: &str) -> bool {
        let Some(entry) = self.registry.get_conductor_for_agent(agent_pub_key) else {
            return false;
        };
        let canonical = super::agent_key::normalize_agent_key(agent_pub_key);
        match self.registry.intents().get(user_identifier).await {
            Ok(Some(intent)) => {
                intent.is_live()
                    && intent.conductor_id == entry.conductor_id
                    && intent.installed_app_id == entry.app_id
                    && intent
                        .agent_pub_key
                        .as_deref()
                        .map(super::agent_key::normalize_agent_key)
                        == Some(canonical)
            }
            _ => false,
        }
    }

    /// Forget this person's record — their account was closed. Best-effort: a
    /// record that could not be deleted is logged, and expires or is found
    /// stale; it holds an identifier, not an identity.
    pub async fn forget(&self, user_identifier: &str) {
        if let Err(e) = self.registry.intents().delete(user_identifier).await {
            warn!(user = %user_identifier, error = %e, "Could not delete the provisioning record");
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
        if let Err(e) = admin.uninstall_app(&entry.app_id).await {
            // A busy conductor (just restarted, re-syncing) can finish an uninstall after the
            // admin call has timed out. Ask it what is installed before calling this a failure:
            // keeping a gone app registered pins its capacity slot until the doorway restarts,
            // and the pool then refuses hosting at a ceiling it is not at.
            let installed = match self.connector.connect(&conductor.admin_url).await {
                Ok(fresh) => fresh.list_apps().await.ok(),
                Err(_) => None,
            };
            if !uninstall_settled(&entry.app_id, installed.as_deref()) {
                return Err(format!(
                    "Failed to uninstall app {} on {}: {}",
                    entry.app_id, entry.conductor_id, e
                ));
            }
            warn!(
                conductor = %entry.conductor_id,
                app_id = %entry.app_id,
                error = %e,
                "uninstall answered with an error but the conductor no longer lists the app"
            );
        }

        // Remove from registry
        self.registry.unregister_agent(agent_pub_key);

        // …and from the intent log: the app is gone, and the record holds the
        // person's identifier.
        let canonical = super::agent_key::normalize_agent_key(agent_pub_key);
        if let Err(e) = self.registry.intents().delete_by_agent(&canonical).await {
            warn!(agent = %agent_pub_key, error = %e, "Could not delete the provisioning record");
        }

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
/// conductor, same app id), which a conductor that already has the app
/// refuses.
fn install_in_flight_bound() -> std::time::Duration {
    super::typed_admin::admin_request_timeout()
}

/// How a recovery walk writes the records it has proof for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordWrite {
    /// The automatic backfill: only where there is no record at all.
    InsertOnly,
    /// The operator's recovery: the conductor-confirmed account key replaces
    /// an abandoned, stuck or differing record.
    Replace,
}

/// What the operator's recovery found and did.
#[derive(Debug, Default, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryReport {
    /// Every conductor in the pool answered AND every record an account proved
    /// was written. Only then is recovery complete.
    pub complete: bool,
    /// Whether registration of people without a record is still refused
    /// because the store was found wiped. Never while `store_gate` is off.
    pub recovery_required: bool,
    /// `DOORWAY_PROVISIONING_STORE_GATE`: whether the verdict may refuse.
    pub store_gate: bool,
    /// Accounts whose agent key a conductor confirmed — the positive evidence
    /// the automatic migration needs before it clears a `wiped` verdict.
    pub accounts_proven: usize,
    /// `unknown` | `not_wiped` | `wiped` — see `conductor::registry::store_verdict`.
    pub store_verdict: &'static str,
    /// Pool conductors that have not answered what they host since boot.
    pub conductors_unwalked: Vec<String>,
    /// Hosted apps reported by the conductors walked so far.
    pub hosted_apps_seen: usize,
    pub intent_store_available: bool,
    pub conductors_walked: Vec<String>,
    pub conductors_unreachable: Vec<String>,
    /// Hosted apps the conductors reported (the doorway's own app excluded).
    pub apps_seen: usize,
    pub mappings_written: usize,
    /// Identifiers whose record was rebuilt from an account row whose agent
    /// key a conductor confirmed.
    pub records_rebuilt: Vec<String>,
    pub records_not_written: Vec<String>,
    /// Accounts whose agent key no conductor reported.
    pub accounts_without_an_app: Vec<String>,
    /// Accounts a conductor confirmed whose existing record the automatic
    /// backfill left as it was (it never replaces a record).
    pub records_kept: Vec<String>,
    /// `conductor/app id` of hosted apps no account row claims. Assigned to
    /// nobody: the doorway has no evidence whose they are.
    pub apps_without_an_account: Vec<String>,
    /// Live records the doorway cannot finish on its own, with the reason.
    pub stuck: Vec<StuckRecord>,
    /// Live records not yet enabled and unchanged past the in-flight bound.
    pub stale: u64,
    pub abandoned: u64,
    pub operator_abandoned: Vec<String>,
    pub log_errors: Vec<String>,
}

/// One stuck record, as an operator sees it.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StuckRecord {
    pub identifier: String,
    pub conductor_id: String,
    pub installed_app_id: String,
    pub reason: &'static str,
    pub state: &'static str,
    pub absent_past_bound: u32,
}

/// How many verified absences, each at least one in-flight bound apart, end a
/// record whose install can never be made.
///
/// Three is the same budget a single call spends re-offering
/// ([`PROVISION_TRANSPORT_ATTEMPTS`]) — "ask a few times, then stop" — spread
/// over at least three in-flight bounds. It bounds how long a person is held
/// by an attempt that left nothing; it is not a safety margin, because every
/// check is the conductor itself answering that the app is not there.
const ABSENT_CHECKS_BEFORE_ABANDON: u32 = PROVISION_TRANSPORT_ATTEMPTS;

/// How often the intent log's counts are published. One in-flight bound: the
/// stale count's own resolution.
pub const INTENT_COUNTS_PERIOD: std::time::Duration =
    holochain_websocket::WebsocketConfig::CLIENT_DEFAULT.default_request_timeout;

fn intent_now() -> chrono::DateTime<chrono::Utc> {
    super::provision_intent::now()
}

fn in_flight_bound_chrono() -> chrono::Duration {
    chrono::Duration::from_std(install_in_flight_bound())
        .unwrap_or_else(|_| chrono::Duration::zero())
}

/// Publish the intent log's counts and the doorway's provisioning readiness,
/// and drop abandoned records whose time has come.
///
/// Runs on a timer ([`INTENT_COUNTS_PERIOD`]), never on a request: it is
/// several counts over the collection and must not sit on a registration.
pub async fn publish_intent_counts(registry: &ConductorRegistry) {
    let store = registry.intents();
    if let Err(e) = store.purge_expired(intent_now()).await {
        debug!(error = %e, "Expired provisioning intents could not be purged");
    }
    match store.counts(intent_now() - in_flight_bound_chrono()).await {
        Ok(counts) => {
            for (state, n) in &counts.by_state {
                crate::metrics::set_provision_intents(state.as_str(), *n);
            }
            for (reason, n) in &counts.stuck {
                crate::metrics::set_provision_intents_stuck(reason.as_str(), *n);
            }
            crate::metrics::set_provision_intents_stale(counts.stale);
        }
        Err(e) => debug!(error = %e, "Provisioning intent counts unavailable"),
    }
    let state = registry.store_state();
    // With the store gate off nothing is refused on the verdict, so readiness
    // is the log's availability alone.
    crate::metrics::set_provisioning_readiness(
        store.available() && (state.verdict == StoreVerdict::NotWiped || !registry.store_gate()),
        registry.recovery_required(),
    );
    crate::metrics::set_store_verdict(state.verdict.as_str(), state.unwalked.len());
    // Hosted apps the conductors report against live records in the log. A
    // DIFFERENCE OF COUNTS, not a join: it says "at least this many hosted apps
    // have no record here" (apps installed before the log existed, or records
    // lost). The exact list, joined on agent key against the account rows, is
    // the recovery report's `appsWithoutAnAccount`.
    if let Ok(total) = store.total().await {
        crate::metrics::set_hosted_apps(
            state.hosted_apps_seen,
            (state.hosted_apps_seen as u64).saturating_sub(total),
        );
    }
}

/// Say, once, how the store verdict settled.
pub fn log_store_verdict(walker: &AgentProvisioner) {
    let state = walker.registry.store_state();
    match state.verdict {
        StoreVerdict::Wiped if !walker.registry.store_gate() => warn!(
            hosted_apps = state.hosted_apps_seen,
            "This doorway's provisioning records are empty while its conductors host apps \
             (a first boot with the intent log, or a lost database). The store gate is off \
             (DOORWAY_PROVISIONING_STORE_GATE), so nothing is refused on it"
        ),
        StoreVerdict::Wiped => tracing::error!(
            hosted_apps = state.hosted_apps_seen,
            "This doorway's provisioning records are EMPTY but its conductors already host apps \
             — a wiped or restored database. Registration of anyone without a record answers \
             PROVISIONING_UNRESOLVED until an operator runs POST /admin/provisioning/recover"
        ),
        verdict => info!(
            verdict = verdict.as_str(),
            hosted_apps = state.hosted_apps_seen,
            "Every conductor has answered what it hosts — provisioning store verdict settled"
        ),
    }
}

/// How long to wait before asking the unwalked conductors again: doubling from
/// one second, capped at the counts period. A conductor that starts after the
/// doorway is asked within seconds of coming up, and one that stays down costs
/// one connect attempt per period.
pub fn walk_backoff(attempt: u32) -> std::time::Duration {
    std::time::Duration::from_secs(1u64 << attempt.min(6)).min(INTENT_COUNTS_PERIOD)
}

/// The OLD app id: six hex digits of the identifier's hash. Recomputed only by
/// [`AgentProvisioner::legacy_find_existing_app`], while the store gate is off;
/// with the gate on it is never recomputed (see `resolve_account_mapping`).
fn legacy_short_app_id(app_id: &str, conductor_id: &str, user_identifier: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(user_identifier.as_bytes());
    format!("{app_id}-{conductor_id}-{}", hex::encode(&hash[..3]))
}

/// Domain separation for [`generate_app_id`]'s preimage.
const APP_ID_DOMAIN: &[u8] = b"elohim/doorway/hosted-app-id/v1\0";

/// Generate the installed app ID for a user on a conductor.
///
/// Format: `{app_id}-{conductor_id}-{hex(sha256(DOMAIN || identifier))}` — the
/// WHOLE digest, 256 bits.
///
/// The id used to carry three bytes of the hash. Twenty-four bits do not name
/// a person: unrelated identifiers shared an id, and code that read "an app
/// with this id exists" as "this person's app" could hand one person another's
/// agent. The full digest of a domain-separated preimage removes the
/// collision. It does not, by itself, make an id proof of anything — the proof
/// is the recorded agent key (`AgentProvisioner::finish_forward`).
///
/// Apps installed under the old short id are not migrated (the dataplane is
/// replaceable at the recast, plane-separation §11) and are reachable only
/// through `AgentProvisioner::resolve_account_mapping`.
fn generate_app_id(app_id: &str, conductor_id: &str, user_identifier: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(APP_ID_DOMAIN);
    hasher.update(user_identifier.as_bytes());
    let digest = hex::encode(hasher.finalize());

    format!("{app_id}-{conductor_id}-{digest}")
}

/// Did an uninstall that answered with an error actually take effect? Pure. Only a
/// successful `list_apps` that no longer names the app says yes; an unreadable conductor
/// (`None`) leaves the app counted, because a still-installed cell must keep its slot.
pub(crate) fn uninstall_settled(
    app_id: &str,
    installed: Option<&[super::typed_admin::InstalledAppInfo]>,
) -> bool {
    installed.is_some_and(|apps| apps.iter().all(|app| app.installed_app_id != app_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conductor::typed_admin::{AdminFault, AppInfoDetailed};
    use async_trait::async_trait;
    use std::collections::{HashMap, VecDeque};
    use std::sync::Mutex;
    use tokio::sync::Notify;

    /// An uninstall that timed out is settled only when the conductor, asked afresh, no
    /// longer lists the app; an unreadable conductor keeps the slot counted.
    #[test]
    fn a_late_uninstall_settles_only_on_a_listing_without_the_app() {
        use super::super::typed_admin::InstalledAppInfo;
        let app = |id: &str| InstalledAppInfo {
            installed_app_id: id.to_string(),
            agent_pub_key: vec![0; 39],
        };
        let others = [app("elohim"), app("elohim-conductor-0-aaaaaa")];
        assert!(uninstall_settled(
            "elohim-conductor-0-bbbbbb",
            Some(&others)
        ));
        assert!(!uninstall_settled(
            "elohim-conductor-0-aaaaaa",
            Some(&others)
        ));
        assert!(!uninstall_settled("elohim-conductor-0-bbbbbb", None));
        assert!(uninstall_settled("elohim-conductor-0-bbbbbb", Some(&[])));
    }

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
        install: Install,
        /// Returned by the NEXT `enable_app`, once.
        enable_fault: Option<AdminFault>,
        /// Returned by every `find_app`.
        lookup_fault: Option<AdminFault>,
        /// Becomes `lookup_fault` the moment an install is received: the
        /// conductor answers the pre-install lookup, then stops answering.
        lookup_fault_after_install: Option<AdminFault>,
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
                install: Install::Succeeds,
                enable_fault: None,
                lookup_fault: None,
                lookup_fault_after_install: None,
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
        /// Mints the agent key itself, as a real conductor does for an install
        /// with no key: one key per app that LANDS (`keys_generated`).
        async fn install_app(
            &self,
            installed_app_id: &str,
            _bundle_path: &str,
        ) -> Result<Vec<u8>, AdminFault> {
            let gate = self.with(|c| {
                c.installs_received += 1;
                if let Some(fault) = c.lookup_fault_after_install.take() {
                    c.lookup_fault = Some(fault);
                }
                c.install_gate.take()
            });
            if let Some((entered, release)) = gate {
                entered.notify_one();
                release.notified().await;
            }
            let mint = || {
                let mut minted = self.pool.keys_minted.lock().unwrap();
                *minted += 1;
                raw_agent_key(*minted)
            };
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
                    c.keys_generated += 1;
                    c.apps.insert(
                        installed_app_id.to_string(),
                        FakeApp {
                            agent_key: mint(),
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
                    let agent_key = mint();
                    c.keys_generated += 1;
                    c.apps.insert(
                        installed_app_id.to_string(),
                        FakeApp {
                            agent_key: agent_key.clone(),
                            status,
                            cells_ready: status == "running",
                        },
                    );
                    agent_key
                };
                match c.install.clone() {
                    Install::Succeeds => Ok(land("disabled", c)),
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

        async fn list_apps(
            &self,
        ) -> Result<Vec<crate::conductor::typed_admin::InstalledAppInfo>, AdminFault> {
            self.with(|c| {
                if let Some(fault) = c.lookup_fault.clone() {
                    return Err(fault);
                }
                let mut apps: Vec<_> = c
                    .apps
                    .iter()
                    .map(
                        |(id, app)| crate::conductor::typed_admin::InstalledAppInfo {
                            installed_app_id: id.clone(),
                            agent_pub_key: app.agent_key.clone(),
                        },
                    )
                    .collect();
                apps.sort_by(|a, b| a.installed_app_id.cmp(&b.installed_app_id));
                Ok(apps)
            })
        }
    }

    /// A two-conductor pool where `conductor-a` is the least loaded — so it is
    /// always the first offer — and `conductor-b` is the sibling a wrongly
    /// re-offered install would land on.
    async fn two_conductor_pool() -> (AgentProvisioner, Arc<FakePool>) {
        let registry = Arc::new(ConductorRegistry::new(None).await.with_store_gate(true));
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
            ProvisionError::Failed(message) | ProvisionError::NotReady(message) => {
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
            "no second minted key — one person, one agent key"
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
        assert_eq!(pool.total_keys_generated(), 0, "nothing has landed yet");
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

    /// `elohim-genesis/dev` #1576's shape on the step before the install: the
    /// pre-install lookup fails with a dropped pipe. As an admin call its
    /// outcome is unknown — but no install was written, so for the install it
    /// is `NotSent`, and the sibling is asked.
    #[tokio::test]
    async fn a_pre_install_lookup_that_drops_is_reoffered_because_no_install_was_sent() {
        let (provisioner, pool) = two_conductor_pool().await;
        pool.with("conductor-a", |c| {
            c.lookup_fault = Some(AdminFault::sent_outcome_unknown(
                "Admin error (list_apps for get_app_info): External API wire error: \
                 InternalError(\"Other: {\\\"error\\\":\\\"BrokenPipe\\\"}\")",
            ))
        });

        let agent = provisioner
            .provision_agent(USER)
            .await
            .expect("a dropped lookup says nothing about the work");

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

    /// A refused pre-install lookup is a verdict too — class decides, not
    /// stage.
    #[tokio::test]
    async fn a_refused_pre_install_lookup_is_a_verdict_and_is_not_reoffered() {
        let (provisioner, pool) = two_conductor_pool().await;
        pool.with("conductor-a", |c| {
            c.lookup_fault = Some(AdminFault::refused(
                "Admin error (list_apps for get_app_info): External API wire error: \
                 InternalError(\"keystore locked\")",
            ))
        });

        let error = provisioner
            .provision_agent(USER)
            .await
            .expect_err("verdict");

        assert!(matches!(error, ProvisionError::Failed(_)), "{error}");
        pool.with("conductor-a", |c| assert_eq!(c.installs_received, 0));
        assert_untouched(&pool, "conductor-b");
    }

    /// The conductor mints the agent key: the doorway never asks for one, so a
    /// hosting conductor's keystore is never listed seed by seed (the lair
    /// frame overflow `TypedAdminClient::install_app_minting_key` names), and
    /// the key handed back is the key the conductor minted.
    #[tokio::test]
    async fn the_conductor_mints_the_key_and_the_record_holds_it() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let provisioner = doorway_over(&pool, &store, A_FIRST).await;

        let agent = provisioner.provision_agent(USER).await.expect("provisions");

        pool.with("conductor-a", |c| {
            let app = &c.apps[&app_on("conductor-a")];
            assert_eq!(agent.agent_pub_key, provisioned_agent_key(&app.agent_key));
        });
        let intent = store.get(USER).await.unwrap().expect("a record");
        assert_eq!(
            intent.agent_pub_key.as_deref(),
            Some(agent.agent_pub_key.as_str())
        );
        assert!(intent.absent_before_send);
    }

    /// The doorway's own capacity check is a verdict as well.
    #[tokio::test]
    async fn a_full_pool_is_a_verdict() {
        let registry = Arc::new(ConductorRegistry::new(None).await.with_store_gate(true));
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

        match provisioner
            .recover_by_pool_scan(USER, &provisioned_agent_key(&raw_agent_key(200)))
            .await
        {
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
            provisioner
            .recover_by_pool_scan(USER, &provisioned_agent_key(&raw_agent_key(200)))
            .await,
            Existence::Unresolved { conductors, .. } if conductors == vec!["conductor-a".to_string()]
        ));
        assert_eq!(pool.total_keys_generated(), 0);
    }

    /// Only a pool in which EVERY conductor answered "no" is "not installed".
    #[tokio::test]
    async fn every_conductor_answering_no_is_the_only_not_installed() {
        let (provisioner, _pool) = two_conductor_pool().await;
        assert!(matches!(
            provisioner
                .recover_by_pool_scan(USER, &provisioned_agent_key(&raw_agent_key(200)))
                .await,
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

        match provisioner
            .recover_by_pool_scan(USER, &provisioned_agent_key(&raw_agent_key(200)))
            .await
        {
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
        let registry = Arc::new(ConductorRegistry::new(None).await.with_store_gate(true));
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
        assert_eq!(id.len(), "elohim-conductor-0-".len() + 64); // a whole SHA-256

        // Deterministic
        let id2 = generate_app_id("elohim", "conductor-0", "test@example.com");
        assert_eq!(id, id2);

        // Different input → different hash
        let id3 = generate_app_id("elohim", "conductor-0", "other@example.com");
        assert_ne!(id, id3);
    }

    #[tokio::test]
    async fn test_provisioner_creation() {
        let registry = Arc::new(ConductorRegistry::new(None).await.with_store_gate(true));
        let provisioner = AgentProvisioner::new(Arc::clone(&registry))
            .with_app_id("my-app".to_string())
            .with_bundle_path("/path/to/bundle.happ".to_string());

        assert_eq!(provisioner.app_id, "my-app");
        assert_eq!(provisioner.bundle_path, "/path/to/bundle.happ");
    }

    #[tokio::test]
    async fn test_provision_no_conductors() {
        let registry = Arc::new(ConductorRegistry::new(None).await.with_store_gate(true));
        let provisioner = AgentProvisioner::new(Arc::clone(&registry));

        let result = provisioner.provision_agent("test@example.com").await;
        assert_eq!(
            result.unwrap_err(),
            ProvisionError::Failed("No conductors available for provisioning".to_string())
        );
    }

    #[tokio::test]
    async fn test_deprovision_unknown_agent() {
        let registry = Arc::new(ConductorRegistry::new(None).await.with_store_gate(true));
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
            .with_store_gate(true)
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
        let then =
            crate::conductor::provision_intent::now() - chrono::Duration::from_std(ago).unwrap();
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
        assert_eq!(pool.total_keys_generated(), 0, "nothing has landed yet");
        pool.with("conductor-a", |c| c.install = Install::Succeeds);
    }

    /// The in-flight install lands, under the key the conductor mints for it —
    /// a key the doorway never saw, because the install's answer was lost.
    async fn the_in_flight_install_lands(pool: &FakePool, store: &MemoryIntentStore) {
        let intent = intent_of(store, USER).await;
        assert!(
            intent.agent_pub_key.is_none() && intent.absent_before_send,
            "an install whose answer was lost holds no key, only the verified absence"
        );
        let key = {
            let mut minted = pool.keys_minted.lock().unwrap();
            *minted += 1;
            raw_agent_key(*minted)
        };
        pool.with("conductor-a", |c| {
            c.keys_generated += 1;
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
        assert_eq!(
            pool.total_keys_generated(),
            0,
            "nothing has landed, nothing minted"
        );
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
            pool.total_installs_received(),
            1,
            "the second call sent an install while the first was mid-install"
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
        assert_eq!(pool.total_installs_received(), 1);

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
        let counts = store
            .counts(crate::conductor::provision_intent::now())
            .await
            .unwrap();
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

    /// A person provisioned before the log existed has an account, a mapping
    /// for the account's agent key, and no intent. They resolve on the MAPPED
    /// conductor — nobody else is asked — and the log is backfilled.
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
            .provision_agent_for_account(USER, Some(&provisioned_agent_key(&key)))
            .await
            .expect("the account's own agent key says where this person lives");

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

        let error = doorway
            .provision_agent_for_account(USER, Some(&provisioned_agent_key(&raw_agent_key(211))))
            .await
            .expect_err("b is dark");

        assert_eq!(unresolved_on(&error), vec!["conductor-b".to_string()]);
        assert_eq!(pool.total_keys_generated(), 0);
        assert_untouched(&pool, "conductor-a");
    }

    /// An install sent long enough ago that the doorway's own admin request
    /// would have timed out, and still not on the conductor: re-send it — to
    /// the SAME conductor, under the SAME app id, after it answers the id is
    /// absent. The conductor mints the key; the record holds the key it
    /// minted, and only one app (one identity) is ever under the id.
    #[tokio::test]
    async fn a_stale_in_flight_install_is_resent_on_the_same_conductor_with_one_key() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        an_install_in_flight_on_conductor_a(&pool, &store).await;

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
            intent_of(&store, USER).await.agent_pub_key.as_deref(),
            Some(agent.agent_pub_key.as_str()),
            "the record holds the key the conductor minted"
        );
        assert_eq!(pool.total_keys_generated(), 1, "one identity");
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
    /// recovery scan, and the scan adopts only an app carrying the RECORDED key.
    #[tokio::test]
    async fn an_intent_naming_a_conductor_outside_the_pool_uses_the_recovery_scan() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let key = raw_agent_key(213);
        let mut intent = ProvisionIntent::intended(USER, "conductor-9", "elohim-conductor-9-xx");
        intent.install_sent_at = Some(crate::conductor::provision_intent::now());
        intent.agent_pub_key = Some(provisioned_agent_key(&key));
        store.insert_new(&intent).await.unwrap();
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

    // ── an app id is not a person ───────────────────────────────────────────
    //
    // The app id used to carry 24 bits of the identifier's hash, and several
    // paths read "an app exists under this id" as "this is that person's app".
    // Two identifiers that share the old short hash stand in for the whole
    // class here: on no path may one of them be handed the other's agent.
    //
    // The proof the doorway accepts is the agent key IT recorded for the
    // identifier (or the key on that identifier's own account row). Never the
    // id alone.

    use crate::conductor::provision_intent::StuckReason;

    /// Two identifiers whose SHA-256 share their first three bytes (`f791bc`):
    /// under the old id scheme they named the SAME app on any conductor.
    const ALICE: &str = "person963@alpha.elohim.host";
    const MALLORY: &str = "person2087@alpha.elohim.host";

    fn app_for(conductor_id: &str, who: &str) -> String {
        generate_app_id(DEFAULT_APP_ID, conductor_id, who)
    }

    fn place_app(
        pool: &FakePool,
        conductor_id: &str,
        app_id: &str,
        key: &[u8],
        status: &'static str,
    ) {
        pool.with(conductor_id, |c| {
            c.apps.insert(
                app_id.to_string(),
                FakeApp {
                    agent_key: key.to_vec(),
                    status,
                    cells_ready: status == "running",
                },
            );
        });
    }

    #[test]
    fn the_two_test_identifiers_shared_one_app_id_under_the_old_scheme() {
        assert_eq!(
            legacy_short_app_id(DEFAULT_APP_ID, "conductor-a", ALICE),
            legacy_short_app_id(DEFAULT_APP_ID, "conductor-a", MALLORY),
        );
        assert_ne!(ALICE, MALLORY);
    }

    /// NEW IDS ARE FULL-WIDTH. The id carries the whole digest of a
    /// domain-separated preimage, so two identifiers cannot share one.
    #[test]
    fn new_app_ids_carry_the_full_identity() {
        let alice = app_for("conductor-a", ALICE);
        let mallory = app_for("conductor-a", MALLORY);

        assert_ne!(alice, mallory, "the old collision must not survive");
        let digest = alice
            .strip_prefix("elohim-conductor-a-")
            .expect("the id still names the app and the conductor");
        assert_eq!(digest.len(), 64, "a whole SHA-256, in hex: {alice}");
        assert!(digest.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(alice, app_for("conductor-a", ALICE), "deterministic");
        assert_ne!(alice, app_for("conductor-b", ALICE));
        assert_ne!(
            digest,
            {
                use sha2::{Digest, Sha256};
                hex::encode(Sha256::digest(ALICE.as_bytes()))
            },
            "the preimage is domain-separated, not the bare identifier"
        );
    }

    /// LEGACY MAPPING. Alice was provisioned under the old short id and the
    /// doorway holds her agent mapping. Mallory, whose identifier shares that
    /// short hash, registers. She must get an app of her own — never Alice's.
    #[tokio::test]
    async fn a_short_hash_twin_never_resolves_to_the_others_app_through_an_agent_mapping() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let alice_key = raw_agent_key(220);
        let legacy_id = legacy_short_app_id(DEFAULT_APP_ID, "conductor-a", ALICE);
        place_app(&pool, "conductor-a", &legacy_id, &alice_key, "running");
        doorway
            .registry
            .register_agent(
                &provisioned_agent_key(&alice_key),
                "conductor-a",
                &legacy_id,
            )
            .await
            .unwrap();

        let mallory = doorway
            .provision_agent(MALLORY)
            .await
            .expect("mallory is a new person and is provisioned as one");

        assert_ne!(
            mallory.agent_pub_key,
            provisioned_agent_key(&alice_key),
            "mallory was handed alice's agent"
        );
        assert_ne!(mallory.installed_app_id, legacy_id);
        pool.with("conductor-a", |c| {
            assert_eq!(
                c.apps[&legacy_id].agent_key, alice_key,
                "alice's app untouched"
            );
        });
    }

    /// THE ONE LEGACY LOOKUP. A person provisioned under an old short id is
    /// found only through the agent key on THEIR OWN account row — never by
    /// recomputing the short hash.
    #[tokio::test]
    async fn a_legacy_app_is_found_only_through_the_account_rows_own_agent_key() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let alice_key = raw_agent_key(221);
        let alice_account_key = provisioned_agent_key(&alice_key);
        let legacy_id = legacy_short_app_id(DEFAULT_APP_ID, "conductor-b", ALICE);
        place_app(&pool, "conductor-b", &legacy_id, &alice_key, "running");
        doorway
            .registry
            .register_agent(&alice_account_key, "conductor-b", &legacy_id)
            .await
            .unwrap();

        // Alice, with the key her account row holds: her own app, no new key.
        let alice = doorway
            .provision_agent_for_account(ALICE, Some(&alice_account_key))
            .await
            .expect("alice's account row names her agent");
        assert_eq!(alice.agent_pub_key, alice_account_key);
        assert_eq!(alice.installed_app_id, legacy_id);
        assert_eq!(pool.total_keys_generated(), 0);
        let backfilled = intent_of(&store, ALICE).await;
        assert_eq!(
            backfilled.agent_pub_key.as_deref(),
            Some(alice_account_key.as_str())
        );
        assert_eq!(backfilled.installed_app_id, legacy_id);

        // Mallory, with HER account's key (which maps to nothing): a new app.
        let mallory_account_key = provisioned_agent_key(&raw_agent_key(222));
        let mallory = doorway
            .provision_agent_for_account(MALLORY, Some(&mallory_account_key))
            .await
            .expect("mallory is provisioned fresh");
        assert_ne!(mallory.agent_pub_key, alice_account_key);
        assert_ne!(mallory.installed_app_id, legacy_id);
    }

    /// …and a key that maps to an app whose conductor reports a DIFFERENT key
    /// is not proof either.
    #[tokio::test]
    async fn an_account_key_the_conductor_does_not_confirm_is_unresolved() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let mapped_key = provisioned_agent_key(&raw_agent_key(223));
        let legacy_id = legacy_short_app_id(DEFAULT_APP_ID, "conductor-a", ALICE);
        place_app(
            &pool,
            "conductor-a",
            &legacy_id,
            &raw_agent_key(224),
            "running",
        );
        doorway
            .registry
            .register_agent(&mapped_key, "conductor-a", &legacy_id)
            .await
            .unwrap();

        let error = doorway
            .provision_agent_for_account(ALICE, Some(&mapped_key))
            .await
            .expect_err("the conductor's key for that app is not the account's");

        assert_eq!(unresolved_on(&error), vec!["conductor-a".to_string()]);
        assert_eq!(pool.total_keys_generated(), 0);
        assert!(
            store.is_empty(),
            "nothing is recorded on an unproven adoption"
        );
    }

    /// REFUSED INSTALL. An app already exists under the id this registration
    /// would install. With no record and no account mapping the doorway has no
    /// evidence it is this person's: a conflict, never an adoption.
    #[tokio::test]
    async fn an_app_already_under_the_id_is_a_conflict_not_this_persons_app() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let someone_elses_key = raw_agent_key(225);
        let id = app_for("conductor-a", MALLORY);
        place_app(&pool, "conductor-a", &id, &someone_elses_key, "disabled");

        let error = doorway
            .provision_agent(MALLORY)
            .await
            .expect_err("an existing app under the id is not hers to take");

        assert_eq!(error.code(), "PROVISIONING_UNRESOLVED");
        assert_eq!(unresolved_on(&error), vec!["conductor-a".to_string()]);
        pool.with("conductor-a", |c| {
            assert_eq!(c.apps[&id].agent_key, someone_elses_key);
            assert_eq!(c.apps[&id].status, "disabled", "not even enabled");
            assert_eq!(c.uninstalls_received, 0);
        });
        let record = intent_of(&store, MALLORY).await;
        assert!(
            record.is_live(),
            "a conflict is not abandoned: something is there"
        );
        assert_eq!(record.stuck, Some(StuckReason::AppConflict));
        assert_ne!(
            record.agent_pub_key.as_deref(),
            Some(provisioned_agent_key(&someone_elses_key).as_str())
        );
        assert_untouched(&pool, "conductor-b");

        // Asking again stays a conflict and generates no further keys.
        let keys = pool.total_keys_generated();
        let again = doorway.provision_agent(MALLORY).await.expect_err("still");
        assert_eq!(again.code(), "PROVISIONING_UNRESOLVED");
        assert_eq!(pool.total_keys_generated(), keys);
    }

    /// RESEND and RESOLVE. The record holds the key the conductor minted for
    /// this person's install. An app under the id whose conductor key DIFFERS
    /// is not this person's: the answer is Unresolved, and the record is left
    /// exactly as it was. A verified absence before the send never outranks a
    /// recorded key.
    #[tokio::test]
    async fn a_recorded_key_that_differs_from_the_conductors_is_unresolved_and_unchanged() {
        for aged_past_the_bound in [false, true] {
            let pool = Arc::new(FakePool::default());
            let store = Arc::new(MemoryIntentStore::new());
            an_install_in_flight_on_conductor_a(&pool, &store).await;
            let mut recorded = intent_of(&store, USER).await;
            let recorded_key = provisioned_agent_key(&raw_agent_key(225));
            let expected = recorded.version;
            recorded.agent_pub_key = Some(recorded_key.clone());
            recorded.version += 1;
            assert!(store.replace(expected, &recorded).await.unwrap());
            if aged_past_the_bound {
                age_install(
                    &store,
                    USER,
                    install_in_flight_bound() + std::time::Duration::from_secs(1),
                )
                .await;
            }
            // An app appears under the id — with somebody else's key.
            let other = raw_agent_key(226);
            place_app(
                &pool,
                "conductor-a",
                &recorded.installed_app_id,
                &other,
                "disabled",
            );

            let doorway = doorway_over(&pool, &store, A_FIRST).await;
            let error = doorway
                .provision_agent(USER)
                .await
                .expect_err("the app under the id carries a different key");

            assert_eq!(unresolved_on(&error), vec!["conductor-a".to_string()]);
            assert!(
                error.to_string().contains("agent key"),
                "the reason must be distinct: {error}"
            );
            let after = intent_of(&store, USER).await;
            assert_eq!(
                after.agent_pub_key.as_deref(),
                Some(recorded_key.as_str()),
                "the recorded key must never be overwritten with the conductor's"
            );
            assert_eq!(after.stuck, Some(StuckReason::KeyMismatch));
            assert!(after.is_live());
            pool.with("conductor-a", |c| {
                assert_eq!(c.apps[&recorded.installed_app_id].status, "disabled");
                assert_eq!(c.installs_received, 1, "no re-send onto someone else's app");
            });
        }
    }

    /// POOL-SCAN RECOVERY. The scan adopts only an app whose conductor key is
    /// the recorded key.
    #[tokio::test]
    async fn the_recovery_scan_never_adopts_an_app_on_its_id_alone() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let recorded_key = provisioned_agent_key(&raw_agent_key(227));
        let mut intent =
            ProvisionIntent::intended(MALLORY, "conductor-9", "elohim-conductor-9-gone");
        intent.install_sent_at = Some(crate::conductor::provision_intent::now());
        intent.agent_pub_key = Some(recorded_key.clone());
        store.insert_new(&intent).await.unwrap();
        let other = raw_agent_key(228);
        place_app(
            &pool,
            "conductor-b",
            &app_for("conductor-b", MALLORY),
            &other,
            "running",
        );

        let error = doorway
            .provision_agent(MALLORY)
            .await
            .expect_err("the app found carries a key this doorway never recorded");

        assert_eq!(error.code(), "PROVISIONING_UNRESOLVED");
        let after = intent_of(&store, MALLORY).await;
        assert_eq!(
            after.conductor_id, "conductor-9",
            "the record is not rewritten"
        );
        assert_eq!(after.agent_pub_key.as_deref(), Some(recorded_key.as_str()));
        assert_eq!(after.stuck, Some(StuckReason::ConductorNotInPool));
        assert!(
            after.is_live(),
            "never abandoned while absence is unverified"
        );
        assert_eq!(pool.total_keys_generated(), 0);
    }

    /// ALREADY-HAS-A-HUMAN. The route may recover an existing Human for a
    /// registration only on an app this doorway recorded for EXACTLY that
    /// identifier, with that agent key.
    #[tokio::test]
    async fn only_an_app_recorded_for_exactly_this_identifier_may_recover_its_existing_human() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let alice = doorway.provision_agent(ALICE).await.expect("alice");

        assert!(doorway.provisioned_for(ALICE, &alice).await);
        assert!(
            !doorway.provisioned_for(MALLORY, &alice).await,
            "another identifier presenting alice's agent is not alice"
        );
        let mut forged = alice.clone();
        forged.agent_pub_key = provisioned_agent_key(&raw_agent_key(229));
        assert!(!doorway.provisioned_for(ALICE, &forged).await);
        let mut elsewhere = alice.clone();
        elsewhere.installed_app_id = app_for("conductor-a", MALLORY);
        assert!(!doorway.provisioned_for(ALICE, &elsewhere).await);
    }

    // ── never abandon on an unknown; every stuck record has a name ──────────

    /// A refused install whose follow-up lookup FAILS establishes nothing about
    /// what is on the conductor. Unresolved — the record and its key stay.
    #[tokio::test]
    async fn a_refused_install_whose_lookup_fails_is_unresolved_and_never_abandoned() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        pool.with("conductor-a", |c| {
            c.install = Install::Refuses("AppAlreadyInstalled");
            c.lookup_fault_after_install = Some(AdminFault::sent_outcome_unknown(
                "Admin error (list_apps for get_app_info): Websocket error: Timeout",
            ));
        });

        let error = doorway.provision_agent(USER).await.expect_err("unknown");

        assert_eq!(error.code(), "PROVISIONING_UNRESOLVED");
        assert_eq!(unresolved_on(&error), vec!["conductor-a".to_string()]);
        let record = intent_of(&store, USER).await;
        assert!(record.is_live(), "never abandon on an unknown");
        assert!(
            record.absent_before_send,
            "the evidence for the sent install is kept"
        );
        assert!(record.install_sent_at.is_some());
        assert_untouched(&pool, "conductor-b");
    }

    /// 4a. A re-send the conductor keeps refusing, with the app verifiably
    /// absent on a reachable conductor each time, ends: after the bound the
    /// record is abandoned (nothing landed) and the person can start again.
    #[tokio::test]
    async fn a_resend_refused_with_the_app_verifiably_absent_is_abandoned_after_a_bound() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        an_install_in_flight_on_conductor_a(&pool, &store).await;
        pool.with("conductor-a", |c| {
            c.install = Install::Refuses("bundle not found at /app/elohim.happ")
        });
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let past = install_in_flight_bound() + std::time::Duration::from_secs(1);

        for round in 1..ABSENT_CHECKS_BEFORE_ABANDON {
            age_install(&store, USER, past).await;
            let error = doorway.provision_agent(USER).await.expect_err("refused");
            assert_eq!(error.code(), "PROVISIONING_UNRESOLVED", "round {round}");
            let record = intent_of(&store, USER).await;
            assert!(record.is_live(), "round {round}: not yet");
            assert_eq!(record.absent_past_bound, round);
        }

        age_install(&store, USER, past).await;
        let error = doorway.provision_agent(USER).await.expect_err("the last");
        assert_eq!(error.code(), "PROVISIONING_FAILED");
        let record = intent_of(&store, USER).await;
        assert_eq!(record.state, IntentState::Abandoned);
        assert!(
            record
                .abandoned_reason
                .as_deref()
                .unwrap_or_default()
                .contains("verifiably absent"),
            "{record:?}"
        );
        assert!(record.expire_at.is_some(), "an abandoned record expires");
        assert_eq!(pool.total_keys_generated(), 0, "nothing ever landed");

        pool.with("conductor-a", |c| c.install = Install::Succeeds);
        doorway
            .provision_agent(USER)
            .await
            .expect("an abandoned record does not hold the person");
    }

    /// …but a re-send whose outcome is UNKNOWN is not a verified absence and
    /// never counts toward abandonment.
    #[tokio::test]
    async fn a_resend_of_unknown_outcome_never_counts_toward_abandonment() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        an_install_in_flight_on_conductor_a(&pool, &store).await;
        pool.with("conductor-a", |c| {
            c.install = Install::SentThenDropped { lands: None }
        });
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let past = install_in_flight_bound() + std::time::Duration::from_secs(1);

        for _ in 0..(ABSENT_CHECKS_BEFORE_ABANDON + 2) {
            age_install(&store, USER, past).await;
            let error = doorway.provision_agent(USER).await.expect_err("unknown");
            assert_eq!(error.code(), "PROVISIONING_UNRESOLVED");
        }
        let record = intent_of(&store, USER).await;
        assert!(record.is_live());
        assert_eq!(record.absent_past_bound, 0);
    }

    /// 4b. A record naming a conductor that is not in the pool, with nothing in
    /// the pool to prove: stuck BY NAME, visible, and not abandoned — the named
    /// conductor cannot be asked, so absence is unverified.
    #[tokio::test]
    async fn a_record_naming_a_conductor_outside_the_pool_is_stuck_by_name_not_abandoned() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let mut intent = ProvisionIntent::intended(USER, "conductor-9", "elohim-conductor-9-xx");
        intent.install_sent_at = Some(crate::conductor::provision_intent::now());
        intent.agent_pub_key = Some(provisioned_agent_key(&raw_agent_key(230)));
        store.insert_new(&intent).await.unwrap();

        let error = doorway.provision_agent(USER).await.expect_err("stuck");

        assert_eq!(unresolved_on(&error), vec!["conductor-9".to_string()]);
        let record = intent_of(&store, USER).await;
        assert!(record.is_live());
        assert_eq!(record.stuck, Some(StuckReason::ConductorNotInPool));
        let counts = store
            .counts(crate::conductor::provision_intent::now())
            .await
            .unwrap();
        assert!(counts.stuck.contains(&(StuckReason::ConductorNotInPool, 1)));
        assert_eq!(pool.total_keys_generated(), 0);
    }

    /// 4c. An installed app the doorway cannot bring to ready is stuck by name.
    #[tokio::test]
    async fn an_installed_app_that_cannot_be_made_ready_is_stuck_by_name() {
        for state in ["unrecoverable", "paused"] {
            let pool = Arc::new(FakePool::default());
            let store = Arc::new(MemoryIntentStore::new());
            let doorway = doorway_over(&pool, &store, A_FIRST).await;
            pool.with("conductor-a", |c| {
                c.install = Install::SentThenDropped { lands: Some(state) }
            });

            let error = doorway.provision_agent(USER).await.expect_err(state);

            assert_eq!(error.code(), "PROVISIONING_UNRESOLVED");
            let record = intent_of(&store, USER).await;
            assert!(record.is_live(), "{state}: the app is THERE");
            assert_eq!(record.stuck, Some(StuckReason::AppUnusable), "{state}");
            pool.with("conductor-a", |c| assert_eq!(c.uninstalls_received, 0));
        }
    }

    /// 4d. "Install sent" with no recorded key and no verified absence before
    /// the send cannot be proven. If an app is there it is stuck by name; if a
    /// reachable conductor keeps answering that nothing is there and refusing
    /// the re-send, it ends like 4a.
    #[tokio::test]
    async fn a_sent_install_with_no_usable_key_is_stuck_or_ends_but_never_adopts() {
        let past = install_in_flight_bound() + std::time::Duration::from_secs(1);

        // An app IS there: not provable, not adopted, stuck.
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let mut intent = ProvisionIntent::intended(USER, "conductor-a", &app_on("conductor-a"));
        intent.install_sent_at = Some(crate::conductor::provision_intent::now());
        store.insert_new(&intent).await.unwrap();
        place_app(
            &pool,
            "conductor-a",
            &app_on("conductor-a"),
            &raw_agent_key(231),
            "running",
        );
        let error = doorway.provision_agent(USER).await.expect_err("unprovable");
        assert_eq!(error.code(), "PROVISIONING_UNRESOLVED");
        assert_eq!(
            intent_of(&store, USER).await.stuck,
            Some(StuckReason::NoUsableKey)
        );
        assert_eq!(pool.total_keys_generated(), 0);

        // Nothing is there and every re-send is refused, verified repeatedly
        // past the bound: it ends.
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        pool.with("conductor-a", |c| {
            c.install = Install::Refuses("bundle not found at /app/elohim.happ")
        });
        let mut intent = ProvisionIntent::intended(USER, "conductor-a", &app_on("conductor-a"));
        intent.install_sent_at = Some(crate::conductor::provision_intent::now());
        store.insert_new(&intent).await.unwrap();
        for _ in 1..ABSENT_CHECKS_BEFORE_ABANDON {
            age_install(&store, USER, past).await;
            let error = doorway.provision_agent(USER).await.expect_err("absent");
            assert_eq!(error.code(), "PROVISIONING_UNRESOLVED");
        }
        age_install(&store, USER, past).await;
        let error = doorway.provision_agent(USER).await.expect_err("ended");
        assert_eq!(error.code(), "PROVISIONING_FAILED");
        assert_eq!(intent_of(&store, USER).await.state, IntentState::Abandoned);
        assert_eq!(
            pool.total_installs_received(),
            ABSENT_CHECKS_BEFORE_ABANDON,
            "one refused re-send per verified absence"
        );
        assert_eq!(pool.total_keys_generated(), 0, "nothing landed");
    }

    // ── refusing to guess ───────────────────────────────────────────────────

    /// A doorway whose records were empty at boot while its conductors host
    /// apps cannot tell a returning person from a new one. Until an operator
    /// has run recovery, a person with no record is Unresolved — not "new".
    #[tokio::test]
    async fn a_wiped_doorway_does_not_treat_everyone_as_new() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        wiped(&doorway);

        let error = doorway
            .provision_agent(USER)
            .await
            .expect_err("cannot know");

        assert_eq!(error.code(), "PROVISIONING_UNRESOLVED");
        assert!(error.to_string().contains("recover"), "{error}");
        assert_eq!(pool.total_keys_generated(), 0);
        assert!(store.is_empty());

        doorway.registry.mark_recovered();
        doorway.provision_agent(USER).await.expect("after recovery");
    }

    /// …while a person the doorway DOES hold a record for is unaffected.
    #[tokio::test]
    async fn recovery_required_does_not_block_a_person_with_a_record() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let first = doorway.provision_agent(USER).await.expect("provisioned");
        wiped(&doorway);

        let again = doorway.provision_agent(USER).await.expect("has a record");
        assert_eq!(again.agent_pub_key, first.agent_pub_key);
    }

    /// The operator's recovery: rebuild mappings from what the conductors
    /// report, and records from the account rows whose agent key a conductor
    /// confirms. An account whose key no conductor holds gets no record; an app
    /// no account claims is reported, not assigned.
    #[tokio::test]
    async fn recovery_rebuilds_records_only_where_an_account_key_matches_a_conductor() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        wiped(&doorway);
        let alice_key = raw_agent_key(232);
        let orphan_key = raw_agent_key(233);
        let alice_app = legacy_short_app_id(DEFAULT_APP_ID, "conductor-b", ALICE);
        place_app(&pool, "conductor-b", &alice_app, &alice_key, "running");
        place_app(
            &pool,
            "conductor-a",
            "elohim-conductor-a-orphan",
            &orphan_key,
            "running",
        );

        let report = doorway
            .rebuild_from_conductors(&[
                (ALICE.to_string(), provisioned_agent_key(&alice_key)),
                (
                    MALLORY.to_string(),
                    provisioned_agent_key(&raw_agent_key(234)),
                ),
            ])
            .await;

        assert!(report.complete, "{report:?}");
        assert_eq!(report.apps_seen, 2);
        assert_eq!(report.records_rebuilt, vec![ALICE.to_string()]);
        assert_eq!(report.accounts_without_an_app, vec![MALLORY.to_string()]);
        assert_eq!(report.apps_without_an_account.len(), 1);
        assert!(
            !doorway.registry.recovery_required(),
            "a complete walk clears it"
        );

        let alice = intent_of(&store, ALICE).await;
        assert_eq!(alice.conductor_id, "conductor-b");
        assert_eq!(alice.installed_app_id, alice_app);
        assert_eq!(alice.state, IntentState::AccountRecorded);
        assert!(store.get(MALLORY).await.unwrap().is_none());
        assert!(doorway
            .registry
            .get_conductor_for_agent(&provisioned_agent_key(&orphan_key))
            .is_some());

        // And now alice resolves to HER app without a new key.
        let found = doorway.provision_agent(ALICE).await.expect("recovered");
        assert_eq!(found.agent_pub_key, provisioned_agent_key(&alice_key));
        assert_eq!(pool.total_keys_generated(), 0);
    }

    /// A walk that could not reach every conductor is not a recovery: the flag
    /// stays, and the report says which conductor was not asked.
    #[tokio::test]
    async fn an_incomplete_recovery_walk_does_not_clear_the_flag() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        wiped(&doorway);
        place_app(
            &pool,
            "conductor-a",
            "elohim-conductor-a-someone",
            &raw_agent_key(235),
            "running",
        );
        pool.with("conductor-b", |c| c.dark = true);

        let report = doorway.rebuild_from_conductors(&[]).await;

        assert!(!report.complete);
        assert_eq!(
            report.conductors_unreachable,
            vec!["conductor-b".to_string()]
        );
        assert!(doorway.registry.recovery_required());
    }

    /// The intent store being unavailable refuses provisioning; it is never
    /// replaced by memory.
    #[tokio::test]
    async fn an_unavailable_intent_store_refuses_provisioning() {
        struct Down;
        #[async_trait]
        impl IntentStore for Down {
            async fn get(&self, _: &str) -> Result<Option<ProvisionIntent>, String> {
                Err("connection refused".into())
            }
            async fn insert_new(&self, _: &ProvisionIntent) -> Result<bool, String> {
                Err("connection refused".into())
            }
            async fn replace(&self, _: i64, _: &ProvisionIntent) -> Result<bool, String> {
                Err("connection refused".into())
            }
            async fn delete(&self, _: &str) -> Result<(), String> {
                Err("connection refused".into())
            }
            async fn delete_by_agent(&self, _: &str) -> Result<u64, String> {
                Err("connection refused".into())
            }
            async fn purge_expired(&self, _: chrono::DateTime<chrono::Utc>) -> Result<u64, String> {
                Err("connection refused".into())
            }
            async fn stuck_records(&self) -> Result<Vec<ProvisionIntent>, String> {
                Err("connection refused".into())
            }
            async fn total(&self) -> Result<u64, String> {
                Err("connection refused".into())
            }
            async fn counts(
                &self,
                _: chrono::DateTime<chrono::Utc>,
            ) -> Result<crate::conductor::provision_intent::IntentCounts, String> {
                Err("connection refused".into())
            }
            fn available(&self) -> bool {
                false
            }
        }

        let pool = Arc::new(FakePool::default());
        let registry = ConductorRegistry::new(None)
            .await
            .with_store_gate(true)
            .with_intent_store(Arc::new(Down));
        registry.register_conductor(ConductorInfo {
            conductor_id: "conductor-a".to_string(),
            conductor_url: "ws://conductor-a:8888".to_string(),
            admin_url: admin_url("conductor-a"),
            capacity_used: 0,
            capacity_max: 10,
        });
        pool.conductors
            .lock()
            .unwrap()
            .insert(admin_url("conductor-a"), FakeConductor::healthy());
        let doorway = AgentProvisioner::new(Arc::new(registry))
            .with_connector(Arc::new(FakeConnector(Arc::clone(&pool))));

        let error = doorway.provision_agent(USER).await.expect_err("no store");

        assert_eq!(error.code(), "PROVISIONING_NOT_READY");
        assert!(error.to_string().contains("connection refused"), "{error}");
        assert_eq!(
            pool.total_keys_generated(),
            0,
            "nothing is asked of a conductor"
        );
        assert_eq!(pool.with("conductor-a", |c| c.connects_received), 0);
    }

    /// Deprovisioning forgets the person's record.
    #[tokio::test]
    async fn deprovisioning_deletes_the_record() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let agent = doorway.provision_agent(USER).await.unwrap();
        assert_eq!(store.len(), 1);

        doorway
            .deprovision_agent(&agent.agent_pub_key)
            .await
            .expect("deprovisioned");

        assert!(
            store.is_empty(),
            "the identifier is not kept after deprovision"
        );
        doorway.forget(USER).await;
        assert!(store.is_empty());
    }

    // ── an unknown is never a negative: the wiped-store verdict ─────────────

    use crate::conductor::registry::StoreVerdict as Verdict;

    /// A doorway whose store was empty at boot and whose conductor-a has
    /// answered that it hosts three apps: wiped.
    /// A doorway over the pool with the store gate OFF — the default.
    async fn ungated_doorway_over(
        pool: &Arc<FakePool>,
        store: &Arc<MemoryIntentStore>,
    ) -> AgentProvisioner {
        ungated_doorway_loaded(pool, store, A_FIRST).await
    }

    async fn ungated_doorway_loaded(
        pool: &Arc<FakePool>,
        store: &Arc<MemoryIntentStore>,
        loads: [(&str, usize); 2],
    ) -> AgentProvisioner {
        let gated = doorway_over(pool, store, loads).await;
        let registry = Arc::new(
            ConductorRegistry::new(None)
                .await
                .with_intent_store(Arc::clone(store) as Arc<dyn IntentStore>),
        );
        for conductor in gated.registry.list_conductors() {
            registry.register_conductor(conductor);
        }
        AgentProvisioner::new(registry).with_connector(Arc::new(FakeConnector(Arc::clone(pool))))
    }

    /// STORE GATE OFF (the default): the same wiped verdict that refuses
    /// above is observed and refuses nobody. A person whose app sits under
    /// their legacy id is handed it, as before the intent log; anyone else is
    /// provisioned under a record.
    #[tokio::test]
    async fn with_the_store_gate_off_a_wiped_verdict_refuses_nobody() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = ungated_doorway_over(&pool, &store).await;
        wiped(&doorway);
        assert!(!doorway.recovery_required());

        let alice_key = raw_agent_key(240);
        let alice_app = legacy_short_app_id(DEFAULT_APP_ID, "conductor-b", ALICE);
        place_app(&pool, "conductor-b", &alice_app, &alice_key, "running");
        let alice = doorway.provision_agent(ALICE).await.expect("as before");
        assert_eq!(alice.agent_pub_key, provisioned_agent_key(&alice_key));
        assert_eq!(alice.installed_app_id, alice_app);
        assert!(
            store.get(ALICE).await.unwrap().is_none(),
            "an app id is not proof: no record is written for it"
        );

        let fresh = doorway.provision_agent(USER).await.expect("new person");
        assert_eq!(fresh.installed_app_id, app_on("conductor-a"));
        assert!(intent_of(&store, USER).await.agent_pub_key.is_some());
        assert_eq!(pool.total_keys_generated(), 1);
    }

    /// FIRST USE OF THE LOG, store gate on: the verdict reads wiped because the
    /// log is new, and the account rows a conductor confirms prove it is not a
    /// loss. The migration backfills their records and clears the verdict.
    #[tokio::test]
    async fn the_first_use_migration_clears_wiped_on_accounts_a_conductor_confirms() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        wiped(&doorway);
        let alice_key = raw_agent_key(241);
        let alice_app = legacy_short_app_id(DEFAULT_APP_ID, "conductor-b", ALICE);
        place_app(&pool, "conductor-b", &alice_app, &alice_key, "running");

        let report = doorway
            .migrate_from_accounts(&[(ALICE.to_string(), provisioned_agent_key(&alice_key))])
            .await;

        assert!(report.complete, "{report:?}");
        assert_eq!(report.accounts_proven, 1);
        assert!(!doorway.recovery_required(), "proven accounts clear it");
        assert_eq!(intent_of(&store, ALICE).await.installed_app_id, alice_app);
        let found = doorway.provision_agent(ALICE).await.expect("her record");
        assert_eq!(found.agent_pub_key, provisioned_agent_key(&alice_key));
        doorway
            .provision_agent(USER)
            .await
            .expect("a new person is new");
    }

    /// A database that lost its accounts too proves nobody: the migration
    /// completes its walk and leaves the verdict wiped for an operator.
    #[tokio::test]
    async fn the_first_use_migration_does_not_clear_a_loss_that_proves_nobody() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        wiped(&doorway);
        place_app(
            &pool,
            "conductor-a",
            "elohim-conductor-a-someone",
            &raw_agent_key(242),
            "running",
        );

        let report = doorway.migrate_from_accounts(&[]).await;

        assert!(report.complete);
        assert_eq!(report.accounts_proven, 0);
        assert!(doorway.recovery_required());
        assert_eq!(
            doorway.provision_agent(USER).await.unwrap_err().code(),
            "PROVISIONING_UNRESOLVED"
        );
    }

    /// A record that could not be written is not a recovery: `complete` needs
    /// the writes, not only the conductors' answers.
    #[tokio::test]
    async fn a_walk_whose_record_writes_fail_is_not_complete() {
        struct ReadsButCannotWrite(MemoryIntentStore);
        #[async_trait]
        impl IntentStore for ReadsButCannotWrite {
            async fn get(&self, who: &str) -> Result<Option<ProvisionIntent>, String> {
                self.0.get(who).await
            }
            async fn insert_new(&self, _: &ProvisionIntent) -> Result<bool, String> {
                Err("write refused".into())
            }
            async fn replace(&self, _: i64, _: &ProvisionIntent) -> Result<bool, String> {
                Err("write refused".into())
            }
            async fn delete(&self, who: &str) -> Result<(), String> {
                self.0.delete(who).await
            }
            async fn delete_by_agent(&self, key: &str) -> Result<u64, String> {
                self.0.delete_by_agent(key).await
            }
            async fn purge_expired(
                &self,
                now: chrono::DateTime<chrono::Utc>,
            ) -> Result<u64, String> {
                self.0.purge_expired(now).await
            }
            async fn stuck_records(&self) -> Result<Vec<ProvisionIntent>, String> {
                self.0.stuck_records().await
            }
            async fn total(&self) -> Result<u64, String> {
                self.0.total().await
            }
            async fn counts(
                &self,
                stale_before: chrono::DateTime<chrono::Utc>,
            ) -> Result<crate::conductor::provision_intent::IntentCounts, String> {
                self.0.counts(stale_before).await
            }
            fn available(&self) -> bool {
                true
            }
        }

        let pool = Arc::new(FakePool::default());
        let registry = ConductorRegistry::new(None)
            .await
            .with_store_gate(true)
            .with_intent_store(Arc::new(ReadsButCannotWrite(MemoryIntentStore::new())));
        for (id, used) in A_FIRST {
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
        let doorway = AgentProvisioner::new(Arc::new(registry))
            .with_connector(Arc::new(FakeConnector(Arc::clone(&pool))));
        wiped(&doorway);
        let alice_key = raw_agent_key(243);
        place_app(
            &pool,
            "conductor-a",
            "elohim-conductor-a-alice",
            &alice_key,
            "running",
        );

        let accounts = [(ALICE.to_string(), provisioned_agent_key(&alice_key))];
        let migrated = doorway.migrate_from_accounts(&accounts).await;
        assert!(!migrated.complete, "{migrated:?}");
        assert_eq!(migrated.records_not_written.len(), 1);
        assert!(doorway.recovery_required(), "nothing was written down");

        let operator = doorway.rebuild_from_conductors(&accounts).await;
        assert!(!operator.complete);
        assert!(doorway.recovery_required());
    }

    /// BLOCKER (review rev-5): the conductor answers the id absent, a
    /// competing install lands under it, and ours is refused. That app is not
    /// this person's — and stays not theirs on the NEXT request, which must not
    /// adopt it on the absence the first request recorded. Gate on and off.
    #[tokio::test]
    async fn a_refused_first_install_never_adopts_the_competing_app_on_any_request() {
        for gated in [true, false] {
            let pool = Arc::new(FakePool::default());
            let store = Arc::new(MemoryIntentStore::new());
            let doorway = if gated {
                doorway_over(&pool, &store, A_FIRST).await
            } else {
                ungated_doorway_over(&pool, &store).await
            };
            pool.with("conductor-a", |c| c.lands_before_next_install = true);

            for request in 1..=2 {
                let error = doorway
                    .provision_agent(USER)
                    .await
                    .expect_err("the app under the id is not this person's");
                assert_eq!(
                    unresolved_on(&error),
                    vec!["conductor-a".to_string()],
                    "gated={gated} request {request}"
                );
                let record = intent_of(&store, USER).await;
                assert_eq!(record.stuck, Some(StuckReason::AppConflict), "{request}");
                assert!(record.agent_pub_key.is_none(), "{request}: nothing adopted");
                assert!(!record.absent_before_send, "{request}: refusal persisted");
            }
            assert_eq!(pool.total_installs_received(), 1, "no re-send onto it");
            assert_eq!(pool.total_keys_generated(), 1, "only the competitor's key");
            assert_untouched(&pool, "conductor-b");
        }
    }

    /// MAJOR (review rev-5): store gate off and no Mongo — the log is process
    /// memory. After a restart the records are gone; a person provisioned
    /// under a full-hash id must get their ORIGINAL identity back, whichever
    /// conductor is now least loaded, as dev found its short-id apps.
    #[tokio::test]
    async fn without_mongo_a_restart_keeps_the_identity_with_the_gate_off() {
        for loads in [A_FIRST, B_FIRST] {
            let pool = Arc::new(FakePool::default());
            let before = ungated_doorway_over(&pool, &Arc::new(MemoryIntentStore::new())).await;
            let original = before.provision_agent(USER).await.expect("first");
            assert_eq!(original.conductor_id, "conductor-a");

            // Restart: a fresh process-memory log over the same conductors.
            let after =
                ungated_doorway_loaded(&pool, &Arc::new(MemoryIntentStore::new()), loads).await;
            let again = after.provision_agent(USER).await.expect("found again");

            assert_eq!(again.agent_pub_key, original.agent_pub_key, "{loads:?}");
            assert_eq!(again.installed_app_id, original.installed_app_id);
            assert_eq!(
                pool.total_installs_received(),
                1,
                "{loads:?}: no new install"
            );
            assert_untouched(&pool, "conductor-b");
        }
    }

    /// MAJOR (review rev-5): a normal upgrade keeps the agent mappings, so the
    /// verdict reads not_wiped and no account has a record. With the gate on,
    /// the boot backfill still writes a record for every account a conductor
    /// confirms — including a legacy node/device row carrying a hosted key —
    /// and only then is the record-dependent login check enforceable. It never
    /// replaces a record that exists.
    #[tokio::test]
    async fn the_backfill_runs_on_a_normal_upgrade_before_login_needs_a_record() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let node_key = raw_agent_key(250);
        let node_app = legacy_short_app_id(DEFAULT_APP_ID, "conductor-a", ALICE);
        place_app(&pool, "conductor-a", &node_app, &node_key, "running");
        let mallory_key = raw_agent_key(251);
        place_app(
            &pool,
            "conductor-b",
            "elohim-conductor-b-mallory",
            &mallory_key,
            "running",
        );
        // Mappings survived the upgrade: the verdict is not_wiped.
        let upgraded = AgentProvisioner::new(Arc::new(
            ConductorRegistry::new(None)
                .await
                .with_store_gate(true)
                .with_intent_store(Arc::clone(&store) as Arc<dyn IntentStore>),
        ))
        .with_connector(Arc::new(FakeConnector(Arc::clone(&pool))));
        for conductor in doorway.registry.list_conductors() {
            upgraded.registry.register_conductor(conductor);
        }
        upgraded
            .registry
            .register_agent(&provisioned_agent_key(&node_key), "conductor-a", &node_app)
            .await
            .unwrap();
        upgraded.registry.track_store_loss(Some(1));
        assert_eq!(upgraded.registry.store_state().verdict, Verdict::NotWiped);

        // A live record for a confirmed account that differs from what the
        // conductor holds: the automatic backfill must not replace it.
        let mallory =
            ProvisionIntent::intended(MALLORY, "conductor-b", &app_for("conductor-b", MALLORY));
        store.insert_new(&mallory).await.unwrap();

        assert!(!upgraded.registry.records_backfilled());
        assert!(
            !upgraded
                .records_agent_for(ALICE, &provisioned_agent_key(&node_key))
                .await
        );

        let report = upgraded
            .migrate_from_accounts(&[
                (ALICE.to_string(), provisioned_agent_key(&node_key)),
                (MALLORY.to_string(), provisioned_agent_key(&mallory_key)),
            ])
            .await;

        assert!(report.complete, "{report:?}");
        assert!(upgraded.registry.records_backfilled());
        assert!(
            upgraded
                .records_agent_for(ALICE, &provisioned_agent_key(&node_key))
                .await
        );
        assert_eq!(report.records_kept, vec![MALLORY.to_string()]);
        assert_eq!(intent_of(&store, MALLORY).await, mallory, "never replaced");
    }

    fn wiped(doorway: &AgentProvisioner) {
        doorway.registry.track_store_loss(Some(0));
        doorway.registry.mark_walked("conductor-a", 3);
        assert_eq!(doorway.registry.store_state().verdict, Verdict::Wiped);
    }

    /// THE LIVE DEFECT. The doorway boots before its conductors: every one is
    /// unreachable. That is UNKNOWN, not "they hold nothing". When a conductor
    /// becomes reachable holding hosted apps over an empty store, the verdict
    /// settles to WIPED. In BOTH states a person with no record is refused and
    /// is never offered to a conductor.
    #[tokio::test]
    async fn a_doorway_that_boots_before_its_conductors_does_not_treat_everyone_as_new() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        pool.with("conductor-a", |c| c.dark = true);
        pool.with("conductor-b", |c| c.dark = true);
        doorway.registry.track_store_loss(Some(0));

        // Boot: nothing reachable.
        let still = doorway.walk_unwalked_conductors().await;
        assert_eq!(
            still,
            vec!["conductor-a".to_string(), "conductor-b".to_string()]
        );
        assert_eq!(doorway.registry.store_state().verdict, Verdict::Unknown);

        let error = doorway.provision_agent(USER).await.expect_err("unknown");
        assert_eq!(error.code(), "PROVISIONING_NOT_READY");
        assert!(
            error.to_string().contains("conductor-a") && error.to_string().contains("conductor-b"),
            "the refusal must name the conductors not yet asked: {error}"
        );
        assert!(store.is_empty());

        // conductor-a comes up. It hosts people this doorway has no record of.
        pool.with("conductor-a", |c| c.dark = false);
        place_app(
            &pool,
            "conductor-a",
            "elohim-conductor-a-someone",
            &raw_agent_key(240),
            "running",
        );
        place_app(
            &pool,
            "conductor-a",
            "elohim-conductor-a-someone-else",
            &raw_agent_key(241),
            "running",
        );
        let still = doorway.walk_unwalked_conductors().await;
        assert_eq!(still, vec!["conductor-b".to_string()]);
        let state = doorway.registry.store_state();
        assert_eq!(
            state.verdict,
            Verdict::Wiped,
            "hosted apps over an empty store"
        );
        assert_eq!(state.hosted_apps_seen, 2);

        let error = doorway.provision_agent(USER).await.expect_err("wiped");
        assert_eq!(error.code(), "PROVISIONING_UNRESOLVED");
        assert!(error.to_string().contains("recover"), "{error}");

        // Never offered to a conductor, in either state.
        assert_eq!(pool.total_keys_generated(), 0);
        assert_eq!(pool.total_installs_received(), 0);
        assert!(store.is_empty());
        // …and the walk registered the agents it found, so they are routable.
        assert!(doorway
            .registry
            .get_conductor_for_agent(&provisioned_agent_key(&raw_agent_key(240)))
            .is_some());
    }

    // ── the node/device question: does this doorway host this identifier? ──

    /// `hosted_here` answers from the doorway's own record only: nobody, a
    /// named conductor, or (an abandoned record) nobody again. Asking it never
    /// dials a conductor, generates a key or sends an install.
    #[tokio::test]
    async fn hosted_here_answers_from_the_record_and_never_installs() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;

        assert_eq!(doorway.hosted_here(USER).await, Ok(None));
        assert_eq!(pool.total_keys_generated(), 0);
        assert_eq!(pool.total_installs_received(), 0);

        let agent = doorway.provision_agent(USER).await.expect("provisions");
        let (keys, installs) = (pool.total_keys_generated(), pool.total_installs_received());
        assert_eq!(
            doorway.hosted_here(USER).await,
            Ok(Some(agent.conductor_id.clone())),
            "a person this doorway hosts is named by their conductor"
        );
        assert_eq!(
            doorway.hosted_here("someone-else@example.com").await,
            Ok(None),
            "another identifier is not answered by this person's record"
        );
        assert_eq!(pool.total_keys_generated(), keys);
        assert_eq!(pool.total_installs_received(), installs);

        let mut gone =
            ProvisionIntent::intended("left@example.com", "conductor-a", "elohim-conductor-a-left");
        gone.state = IntentState::Abandoned;
        assert!(store.insert_new(&gone).await.unwrap());
        assert_eq!(
            doorway.hosted_here("left@example.com").await,
            Ok(None),
            "an abandoned record left nothing behind and speaks for nobody"
        );
    }

    /// A node/device login onto a hosted app is let through only by this
    /// doorway's record for exactly that identifier naming that agent and app.
    #[tokio::test]
    async fn records_agent_for_needs_the_identifiers_own_record_naming_the_agent() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        let agent = doorway.provision_agent(USER).await.expect("provisions");

        assert!(doorway.records_agent_for(USER, &agent.agent_pub_key).await);
        assert!(
            !doorway
                .records_agent_for("someone-else@example.com", &agent.agent_pub_key)
                .await,
            "another identifier's record is not evidence"
        );
        assert!(
            !doorway.records_agent_for(USER, "uhCAkunmapped").await,
            "a key the registry does not map is not evidence"
        );

        doorway.forget(USER).await;
        assert!(!doorway.records_agent_for(USER, &agent.agent_pub_key).await);
    }

    /// With the records empty at boot, "no record" is not "nobody": an
    /// unanswered conductor is NOT_READY and a wiped store is UNRESOLVED.
    #[tokio::test]
    async fn hosted_here_keeps_unknown_apart_from_nobody() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        doorway.registry.track_store_loss(Some(0));
        assert_eq!(doorway.registry.store_state().verdict, Verdict::Unknown);
        assert_eq!(
            doorway.hosted_here(USER).await.expect_err("unknown").code(),
            "PROVISIONING_NOT_READY"
        );

        wiped(&doorway);
        assert_eq!(
            doorway.hosted_here(USER).await.expect_err("wiped").code(),
            "PROVISIONING_UNRESOLVED"
        );
        assert_eq!(pool.total_keys_generated(), 0);
        assert_eq!(pool.total_installs_received(), 0);
    }

    /// An empty store over conductors that have ALL answered "nobody" is a new
    /// deployment: registration proceeds.
    #[tokio::test]
    async fn an_empty_store_over_verifiably_empty_conductors_is_a_new_deployment() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        doorway.registry.track_store_loss(Some(0));

        assert!(doorway.walk_unwalked_conductors().await.is_empty());
        assert_eq!(doorway.registry.store_state().verdict, Verdict::NotWiped);

        doorway
            .provision_agent(USER)
            .await
            .expect("everyone IS new here");
    }

    /// One of two conductors never becomes reachable: the verdict stays
    /// Unknown, and the refusal names it.
    #[tokio::test]
    async fn a_conductor_that_never_answers_keeps_the_verdict_unknown_and_is_named() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        pool.with("conductor-b", |c| c.dark = true);
        doorway.registry.track_store_loss(Some(0));

        for _ in 0..3 {
            assert_eq!(
                doorway.walk_unwalked_conductors().await,
                vec!["conductor-b".to_string()]
            );
        }
        assert_eq!(doorway.registry.store_state().verdict, Verdict::Unknown);
        // The conductor that answered is not asked again.
        assert_eq!(pool.with("conductor-a", |c| c.connects_received), 1);

        let error = doorway.provision_agent(USER).await.expect_err("unknown");
        assert_eq!(error.code(), "PROVISIONING_NOT_READY");
        assert!(error.to_string().contains("conductor-b"), "{error}");
        assert!(!error.to_string().contains("conductor-a,"), "{error}");
        assert_eq!(pool.total_keys_generated(), 0);
    }

    /// Unknown is not clearable by recovery: a recovery walk that cannot reach
    /// the unanswered conductor is incomplete and leaves the verdict where it
    /// was.
    #[tokio::test]
    async fn recovery_cannot_clear_an_unknown_it_could_not_ask() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        pool.with("conductor-b", |c| c.dark = true);
        doorway.registry.track_store_loss(Some(0));
        doorway.walk_unwalked_conductors().await;
        assert_eq!(doorway.registry.store_state().verdict, Verdict::Unknown);

        let report = doorway.rebuild_from_conductors(&[]).await;

        assert!(!report.complete);
        assert_eq!(report.store_verdict, "unknown");
        assert_eq!(report.conductors_unwalked, vec!["conductor-b".to_string()]);
        assert_eq!(doorway.registry.store_state().verdict, Verdict::Unknown);
        let error = doorway
            .provision_agent(USER)
            .await
            .expect_err("still unknown");
        assert_eq!(error.code(), "PROVISIONING_NOT_READY");
    }

    /// A store that HAD records at boot is not wiped, and a conductor that is
    /// down does not stop anyone registering.
    #[tokio::test]
    async fn a_store_with_records_registers_people_while_a_conductor_is_down() {
        let pool = Arc::new(FakePool::default());
        let store = Arc::new(MemoryIntentStore::new());
        let doorway = doorway_over(&pool, &store, A_FIRST).await;
        pool.with("conductor-b", |c| c.dark = true);
        doorway.registry.track_store_loss(Some(5));
        doorway.walk_unwalked_conductors().await;

        let state = doorway.registry.store_state();
        assert_eq!(state.verdict, Verdict::NotWiped);
        assert_eq!(state.unwalked, vec!["conductor-b".to_string()]);
        doorway.provision_agent(USER).await.expect("not blocked");
    }

    /// The walk retries quickly at first and never slower than the counts
    /// period.
    #[test]
    fn the_walk_backs_off_to_the_counts_period() {
        assert_eq!(walk_backoff(0), std::time::Duration::from_secs(1));
        assert_eq!(walk_backoff(3), std::time::Duration::from_secs(8));
        assert_eq!(walk_backoff(40), INTENT_COUNTS_PERIOD);
        assert!(walk_backoff(6) <= INTENT_COUNTS_PERIOD);
    }
}
