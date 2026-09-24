//! Federation Service
//!
//! Core federation engine for doorway-to-doorway cooperation:
//! - DHT registration via infrastructure DNA
//! - Periodic heartbeat reporting
//! - Name-routed one-hop relay inputs (see `crate::services::name_routing`)
//! - Runtime-mutable peer URL list for admin-managed federation topology
//!
//! This is the capstone of the 5-stage agency model, enabling community
//! doorway stewards to register, publish, and participate in the network.
//!
//! # Doorway Lifecycle & Steward Failure (Future Work)
//!
//! Doorway operators are **custodians, not owners**. Hosted humans own their
//! identity (Holochain agent keys) and must never be stranded by steward
//! failure. The following scenarios must be handled:
//!
//! - **Steward death/incapacitation**: Dead-man's switch — if heartbeats stop
//!   for N days, federation peers treat the doorway as implicitly draining.
//! - **Capture/compromise**: Federation peers can flag a doorway as compromised,
//!   triggering automatic migration warnings to hosted humans.
//! - **Voluntary shutdown (drain protocol)**: Steward signals "shuttering" to
//!   the federation. Peer doorways absorb hosted humans. Humans who haven't
//!   exported keys get escalating urgency warnings with a deadline.
//! - **Admin capability constraints**: Admin APIs must never allow actions that
//!   strand humans without recourse. All admin mutations are runtime-only
//!   (reset on restart) as a safety measure.
//!
//! The human-scale P2P network (not the federation itself) is the ground truth.
//! Doorways serve humans; humans do not belong to doorways.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::config::Args;
use crate::server::AppState;
use crate::services::zome_caller::ZomeCaller;

// =============================================================================
// Configuration
// =============================================================================

/// Federation configuration — derived from CLI args
#[derive(Debug, Clone)]
pub struct FederationConfig {
    /// Whether federation is enabled (requires doorway_id + doorway_url)
    pub enabled: bool,
    /// Unique doorway identifier (e.g., "alpha-elohim-host")
    pub doorway_id: String,
    /// Public URL of this doorway (e.g., "https://alpha.elohim.host")
    pub doorway_url: String,
    /// Pkarr-shaped, signed candidate addresses advertised by this doorway.
    pub endpoints: Vec<infrastructure_types::DoorwayEndpoint>,
    /// Geographic region for routing
    pub region: Option<String>,
    /// Heartbeat interval in seconds (default: 60)
    pub heartbeat_interval_secs: u64,
    /// Role name in the hApp for infrastructure DNA
    pub infrastructure_role: String,
    /// Zome name within infrastructure DNA
    pub zome_name: String,
}

impl FederationConfig {
    /// Build federation config from CLI args.
    /// Returns None if doorway_id or doorway_url not configured (federation disabled).
    pub fn from_args(args: &Args) -> Option<Self> {
        let doorway_id = args.doorway_id.as_ref()?;
        let doorway_url = args.doorway_url.as_ref()?;
        let ttl_secs = args.doorway_endpoint_ttl_secs;
        let mut endpoints = vec![infrastructure_types::DoorwayEndpoint {
            service: "gateway".to_string(),
            url: doorway_url.clone(),
            priority: 0,
            ttl_secs,
        }];
        for (index, url) in args.doorway_urls.iter().enumerate() {
            if url != doorway_url && !endpoints.iter().any(|endpoint| endpoint.url == *url) {
                endpoints.push(infrastructure_types::DoorwayEndpoint {
                    service: "gateway".to_string(),
                    url: url.clone(),
                    priority: u16::try_from(index + 1).unwrap_or(u16::MAX),
                    ttl_secs,
                });
            }
        }
        if let Some(url) = &args.bootstrap_url {
            endpoints.push(infrastructure_types::DoorwayEndpoint {
                service: "bootstrap".to_string(),
                url: url.clone(),
                priority: 0,
                ttl_secs,
            });
        }
        if let Some(url) = &args.signal_url {
            endpoints.push(infrastructure_types::DoorwayEndpoint {
                service: "signal".to_string(),
                url: url.clone(),
                priority: 0,
                ttl_secs,
            });
        }

        Some(Self {
            enabled: true,
            doorway_id: doorway_id.clone(),
            doorway_url: doorway_url.clone(),
            endpoints,
            region: args.region.clone(),
            heartbeat_interval_secs: 60,
            infrastructure_role: "infrastructure".to_string(),
            zome_name: "infrastructure".to_string(),
        })
    }
}

// =============================================================================
// Zome Input/Output Types (shared with infrastructure coordinator zome)
// =============================================================================

pub use infrastructure_types::{
    DoorwayOutput, DoorwayRegistration, RecordHealthAttestationInput, RegisterDoorwayInput,
};

// =============================================================================
// Registration
// =============================================================================

/// Register this doorway in the infrastructure DNA's DHT.
///
/// Called at startup. On "already exists" error, falls back to update_doorway
/// for idempotent restarts.
pub async fn register_doorway_in_dht(
    config: &FederationConfig,
    zome_caller: &ZomeCaller,
    capabilities: Vec<String>,
) -> Result<(), String> {
    let caps_json = serde_json::to_string(&capabilities)
        .map_err(|e| format!("Failed to serialize capabilities: {e}"))?;

    let input = RegisterDoorwayInput {
        id: config.doorway_id.clone(),
        url: config.doorway_url.clone(),
        identity_root: None,
        endpoints: config.endpoints.clone(),
        capabilities_json: caps_json,
        reach: "public".to_string(),
        region: config.region.clone(),
        bandwidth_mbps: None,
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    info!(
        doorway_id = %config.doorway_id,
        doorway_url = %config.doorway_url,
        "Registering doorway in infrastructure DHT"
    );

    match zome_caller
        .call::<RegisterDoorwayInput, DoorwayOutput>(
            &config.infrastructure_role,
            &config.zome_name,
            "register_doorway",
            &input,
        )
        .await
    {
        Ok(_output) => {
            info!(
                doorway_id = %config.doorway_id,
                "Doorway registered in DHT successfully"
            );
            Ok(())
        }
        Err(e) if e.contains("already exists") => {
            // Doorway already registered — update instead (idempotent startup)
            info!(
                doorway_id = %config.doorway_id,
                "Doorway already registered, updating..."
            );
            match zome_caller
                .call::<RegisterDoorwayInput, DoorwayOutput>(
                    &config.infrastructure_role,
                    &config.zome_name,
                    "update_doorway",
                    &input,
                )
                .await
            {
                Ok(_) => {
                    info!("Doorway registration updated successfully");
                    Ok(())
                }
                Err(e) => {
                    warn!("Failed to update doorway registration: {}", e);
                    Err(e)
                }
            }
        }
        Err(e) => {
            warn!("Failed to register doorway in DHT: {}", e);
            Err(e)
        }
    }
}

/// Take THIS doorway out of the infrastructure DHT's federation roster.
///
/// Calls the coordinator's `deregister_doorway` with this doorway's own id —
/// self-deregistration only, the mirror of [`register_doorway_in_dht`]; the
/// zome deletes only index links this agent authored. Returns the number of
/// links deleted (0 = nothing to remove; the verb is idempotent).
///
/// The registration verb had no inverse, so every a2o scenario doorway stayed
/// on the roster forever and every sibling probed and attested it every
/// round (conductor-store growth report 2026-09-24 §3, §7.1).
pub async fn deregister_doorway_in_dht(
    config: &FederationConfig,
    zome_caller: &ZomeCaller,
) -> Result<u32, String> {
    let deleted: u32 = zome_caller
        .call::<String, u32>(
            &config.infrastructure_role,
            &config.zome_name,
            "deregister_doorway",
            &config.doorway_id,
        )
        .await?;
    info!(
        doorway_id = %config.doorway_id,
        links_deleted = deleted,
        "Doorway deregistered from the infrastructure DHT roster"
    );
    Ok(deleted)
}

// =============================================================================
// Registration retry (bounded)
// =============================================================================

/// First retry gap after the boot attempt fails.
pub const REGISTRATION_RETRY_BASE_SECS: u64 = 15;
/// Ceiling on a single gap — the same "a few discovery ticks" horizon the
/// name-route shed window uses. Five ticks is as long as this doorway can
/// usefully stay silent about its own existence.
pub const REGISTRATION_RETRY_MAX_SECS: u64 = 300;
/// Bounded, never endless. 15+30+60+120+240 then 300×7 ≈ 42 minutes of
/// patience — comfortably past this conductor line's ~11-minute cell-startup
/// window (interfaces accept calls while cells are still initialising and
/// answer `CellDisabled`), and short enough that a doorway which still cannot
/// register says so LOUDLY rather than retrying into the void.
pub const REGISTRATION_MAX_RETRIES: u32 = 12;

/// Gap before retry number `attempt` (1-based; attempt 0 is the boot try).
/// `None` once the bounded budget is spent.
///
/// Pure — the crate's established clock-injection discipline (see
/// `routes::admin_dev::apply_shed_request`): the schedule is a decision, the
/// sleeping is the caller's.
pub fn registration_retry_delay_secs(attempt: u32) -> Option<u64> {
    if attempt == 0 || attempt > REGISTRATION_MAX_RETRIES {
        return None;
    }
    Some(
        REGISTRATION_RETRY_BASE_SECS
            .saturating_mul(2u64.saturating_pow(attempt - 1))
            .min(REGISTRATION_RETRY_MAX_SECS),
    )
}

/// How a bounded registration run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistrationOutcome {
    /// `attempts` includes the boot try, so `1` means it worked first time.
    Registered { attempts: u32 },
    /// The bounded budget was spent and this doorway is NOT in the registry.
    GaveUp { attempts: u32, last_error: String },
}

/// Drive `attempt` until it succeeds or the bounded backoff budget is spent.
///
/// # Why this exists
///
/// Registration used to be ONE try, 5s after boot, whose failure was a WARN
/// and nothing else (`main.rs`: "Federation registration failed
/// (non-fatal)"). Measured on the household mesh 2026-09-21T03:22:42Z: the
/// gamma doorway's single attempt hit `CellDisabled` — the conductor's
/// interfaces were already accepting calls while its cells were still
/// initialising — and `gamma-elohim-host` was then absent from every
/// doorway's `/api/v1/federation/doorways` for the whole life of the process.
/// Since "the registry IS the DHT" (2026-09-12 ruling) and the discovery loop
/// re-reads `get_all_doorways` every tick, a doorway missing from that set is
/// invisible to every sibling's coherence probe and therefore can never be a
/// holder in anyone's name-route fold — no relay to it is possible, for as
/// long as it runs. The steward-peer registration sitting right beside it in
/// `main.rs` already retried in the background and registered 10s later; this
/// gives its sibling the same courtesy, bounded.
///
/// # Shape
///
/// ONE call in flight (the loop awaits sequentially), never on a request
/// path (a background task), and NO caller-side timeout around the zome
/// call — `conductor-call-is-uncancellable`: abandoning a call the conductor
/// is still running does not stop it, it only loses the answer.
///
/// `sleep` is injected so the schedule is testable without a test that
/// actually waits 42 minutes.
pub async fn drive_registration_with_retry<A, AFut, S, SFut>(
    doorway_id: &str,
    mut attempt: A,
    mut sleep: S,
) -> RegistrationOutcome
where
    A: FnMut(u32) -> AFut,
    AFut: std::future::Future<Output = Result<(), String>>,
    S: FnMut(u64) -> SFut,
    SFut: std::future::Future<Output = ()>,
{
    let mut attempts = 0u32;
    loop {
        attempts += 1;
        match attempt(attempts).await {
            Ok(()) => return RegistrationOutcome::Registered { attempts },
            Err(last_error) => match registration_retry_delay_secs(attempts) {
                Some(delay) => {
                    warn!(
                        doorway_id = %doorway_id,
                        attempt = attempts,
                        retry_in_secs = delay,
                        error = %last_error,
                        "Federation registration failed — retrying (this doorway is \
                         undiscoverable to every sibling until it registers)"
                    );
                    sleep(delay).await;
                }
                None => {
                    return RegistrationOutcome::GaveUp {
                        attempts,
                        last_error,
                    }
                }
            },
        }
    }
}

/// Background task: register this doorway in the DHT, retrying on a bounded
/// backoff. See [`drive_registration_with_retry`] for why the boot-once
/// version was a defect.
pub fn spawn_doorway_registration_task(
    config: FederationConfig,
    zome_caller: Arc<ZomeCaller>,
    capabilities: Vec<String>,
    initial_delay: Duration,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        tokio::time::sleep(initial_delay).await;
        info!(
            "Federation: registering doorway '{}' in DHT...",
            config.doorway_id
        );
        let outcome = drive_registration_with_retry(
            &config.doorway_id,
            |_attempt| {
                let config = &config;
                let zome_caller = &zome_caller;
                let capabilities = capabilities.clone();
                async move { register_doorway_in_dht(config, zome_caller, capabilities).await }
            },
            |secs| tokio::time::sleep(Duration::from_secs(secs)),
        )
        .await;
        match outcome {
            RegistrationOutcome::Registered { attempts } => {
                info!(
                    doorway_id = %config.doorway_id,
                    attempts,
                    "Federation: doorway registration complete"
                );
            }
            RegistrationOutcome::GaveUp {
                attempts,
                last_error,
            } => {
                // Loud, not a debug crumb: a doorway that is not in the
                // registry is invisible to every sibling's fold for the rest
                // of this process's life.
                tracing::error!(
                    doorway_id = %config.doorway_id,
                    attempts,
                    error = %last_error,
                    "Federation: doorway NEVER registered in the DHT after the bounded retry \
                     budget — no sibling can discover it or relay a name to it until this \
                     process is restarted"
                );
            }
        }
    })
}

// =============================================================================
// Heartbeat
// =============================================================================

/// Probe rounds a roster member may go without liveness evidence before the
/// peer-health probe stops dialing it.
///
/// A probe round is every 5th heartbeat tick: 5 x `heartbeat_interval_secs`
/// (60 s from `FederationConfig::from_args`, so 300 s; the 2026-09-24 store
/// report measured 150 s). Three rounds is 15 minutes at 300 s and 7.5 at
/// 150 s, so a scenario doorway that lived four minutes leaves the roster
/// within ~15 minutes of its last sign of life, while a sibling that is merely
/// slow keeps being probed as long as it answers at all.
///
/// Why this exists: every probe writes an immutable
/// `attestation:device-health` Content node, and the roster is every doorway
/// ever registered. 27,317 of 30,787 lamad entries on the household were
/// these, 25,973 of them `unreachable` about doorways torn down minutes after
/// birth (`genesis/docs/content/elohim-protocol/architecture/
/// 2026-09-24-conductor-store-growth-report.md` §3 and §7.1 option (a)).
/// This bounds WHO is probed; what one probe writes is unchanged.
pub const PROBE_ROSTER_EXPIRY_ROUNDS: u64 = 3;

/// Every this-many probe rounds, expired roster members get a grace re-probe.
///
/// Expiry alone was a one-way ratchet: an expired peer came back only when its
/// `record_serial` changed (a reboot), so a live sibling that stalled WITHOUT
/// restarting — a partition, a parked worker, a certificate lapse — was
/// skipped forever. A grace probe that gets any answer is ordinary liveness
/// evidence ([`ProbeRoster::observe`]) and the peer is live again from the
/// next round.
///
/// 12 rounds is one hour at the configured 300 s round (5 x 60 s heartbeat).
/// With [`PROBE_ROSTER_GRACE_BATCH`] this bounds the write rate a fully dead
/// roster costs one attestor to `GRACE_BATCH / GRACE_ROUNDS` attestations per
/// round — 16 per hour, whatever the roster's size — against one per member
/// per round before H1 (the store report's ~648/h across three attestors).
pub const PROBE_ROSTER_GRACE_ROUNDS: u64 = 12;

/// Most expired members one grace round re-probes, least recently tried first.
///
/// The order rotates through every expired member: a peer's turn comes from
/// the round it expired or was last grace-probed, whichever is later, so a
/// silent peer goes to the back and the whole expired set is revisited every
/// `ceil(expired / GRACE_BATCH)` grace rounds (179 dead registrations — the
/// household's measured count — are all revisited within 12 grace rounds).
pub const PROBE_ROSTER_GRACE_BATCH: usize = 16;

/// Per-attestor liveness book for the peer-health probe roster.
///
/// Liveness evidence for a peer is any of: first appearing in the roster;
/// answering a probe at all (`online` or `degraded` — only a transport failure
/// is `unreachable`); or its registration's `record_serial` changing, which a
/// doorway does on every boot. There is no `active|<ts>` link on doorway
/// registrations; these three are the freshness markers that exist. A peer
/// with no evidence for [`PROBE_ROSTER_EXPIRY_ROUNDS`] rounds is skipped, and
/// re-enters the moment it re-registers — or when it answers a grace re-probe
/// (every [`PROBE_ROSTER_GRACE_ROUNDS`], at most [`PROBE_ROSTER_GRACE_BATCH`]).
///
/// Pure bookkeeping on round numbers (no clock), so the expiry is testable
/// without sleeping. Entries for ids that leave the roster are dropped, so the
/// book never outgrows the roster itself.
#[derive(Debug, Default)]
pub struct ProbeRoster {
    round: u64,
    peers: std::collections::HashMap<String, ProbeRosterEntry>,
}

#[derive(Debug, Clone)]
struct ProbeRosterEntry {
    last_live_round: u64,
    record_serial: Option<u64>,
    /// Round of this peer's last grace re-probe, if any since it last lived.
    last_grace_round: Option<u64>,
}

impl ProbeRosterEntry {
    fn expired_at(&self, round: u64) -> bool {
        round.saturating_sub(self.last_live_round) >= PROBE_ROSTER_EXPIRY_ROUNDS
    }

    /// Grace-queue position: the later of when it expired and when it was
    /// last grace-probed. Lower goes first.
    fn grace_turn(&self) -> u64 {
        let expired_round = self.last_live_round + PROBE_ROSTER_EXPIRY_ROUNDS;
        self.last_grace_round
            .map_or(expired_round, |g| g.max(expired_round))
    }
}

impl ProbeRoster {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a probe round over `roster`: returns `(to_probe, skipped_ids)`.
    ///
    /// `to_probe` keeps roster order and includes this round's grace
    /// re-probes; `skipped_ids` is every expired member NOT probed this round,
    /// so the skip counter keeps counting the rounds a peer goes unprobed.
    pub fn begin_round(&mut self, roster: &[PeerDoorway]) -> (Vec<PeerDoorway>, Vec<String>) {
        self.round += 1;
        let round = self.round;
        let present: std::collections::HashSet<&str> =
            roster.iter().map(|peer| peer.id.as_str()).collect();
        self.peers.retain(|id, _| present.contains(id.as_str()));

        for peer in roster {
            let entry = self
                .peers
                .entry(peer.id.clone())
                .or_insert_with(|| ProbeRosterEntry {
                    last_live_round: round,
                    record_serial: peer.record_serial,
                    last_grace_round: None,
                });
            if peer.record_serial.is_some() && peer.record_serial != entry.record_serial {
                entry.record_serial = peer.record_serial;
                entry.last_live_round = round;
                entry.last_grace_round = None;
            }
        }

        let grace = self.grace_selection(round);

        let mut to_probe = Vec::new();
        let mut skipped = Vec::new();
        for peer in roster {
            let expired = self
                .peers
                .get(&peer.id)
                .is_some_and(|entry| entry.expired_at(round));
            if !expired || grace.contains(peer.id.as_str()) {
                to_probe.push(peer.clone());
            } else {
                skipped.push(peer.id.clone());
            }
        }
        (to_probe, skipped)
    }

    /// On a grace round, pick up to [`PROBE_ROSTER_GRACE_BATCH`] expired ids,
    /// least recently tried first (ties by id, so the order is deterministic),
    /// and stamp them as tried this round.
    fn grace_selection(&mut self, round: u64) -> std::collections::HashSet<String> {
        if !round.is_multiple_of(PROBE_ROSTER_GRACE_ROUNDS) {
            return std::collections::HashSet::new();
        }
        let mut expired: Vec<(u64, String)> = self
            .peers
            .iter()
            .filter(|(_, entry)| entry.expired_at(round))
            .map(|(id, entry)| (entry.grace_turn(), id.clone()))
            .collect();
        expired.sort_unstable();
        expired.truncate(PROBE_ROSTER_GRACE_BATCH);
        expired
            .into_iter()
            .map(|(_, id)| {
                if let Some(entry) = self.peers.get_mut(&id) {
                    entry.last_grace_round = Some(round);
                }
                id
            })
            .collect()
    }

    /// Record one probe's outcome in the current round. `answered` is true for
    /// any HTTP response (`online` / `degraded`), false for `unreachable`.
    pub fn observe(&mut self, peer_id: &str, answered: bool) {
        if answered {
            if let Some(entry) = self.peers.get_mut(peer_id) {
                entry.last_live_round = self.round;
                entry.last_grace_round = None;
            }
        }
    }
}

/// Spawn periodic peer-health-probe task (every heartbeat_interval_secs).
///
/// On every Nth tick it probes cached federation peers' `/health` endpoints and
/// records peer-witnessed health attestations via the infrastructure zome.
/// Only the LIVE roster is probed: [`ProbeRoster`] skips a peer with no
/// liveness evidence for [`PROBE_ROSTER_EXPIRY_ROUNDS`] rounds, counted as
/// `doorway_federation_doorbell_total{side="probe",outcome="skipped_expired"}`
/// for every round it goes unprobed; expired peers get a bounded grace
/// re-probe every [`PROBE_ROSTER_GRACE_ROUNDS`].
/// Logs warnings on failure but does not crash.
///
/// NOTE: This task no longer reports a *self*-heartbeat to the conductor. The
/// `record_heartbeat` coordinator fn was retired from the infrastructure DNA
/// (observation-event-layer spec `2026-05-11-observation-event-layer-design.md`
/// §10 Stage 6) — doorway self-liveness now flows through
/// `infrastructure:doorway-heartbeat` observations on the Track 2 observation
/// substrate (not yet wired here; Stages 4/7 of that spec). Calling the removed
/// fn every interval failed unconditionally, and a zome-call `Err` clears the
/// conductor connection (`zome_caller::call_zome`), so the dead call drove a
/// reconnect-churn loop that wedged the gateway under load (2026-06-13 freeze).
/// The peer-health-probe path below uses `record_health_attestation`, which
/// still exists (bridges to elohim `issue_attestation`), so it is retained.
pub fn spawn_heartbeat_task(
    config: FederationConfig,
    zome_caller: Arc<ZomeCaller>,
    state: Arc<AppState>,
) -> JoinHandle<()> {
    let interval = std::time::Duration::from_secs(config.heartbeat_interval_secs);

    tokio::spawn(async move {
        info!(
            doorway_id = %config.doorway_id,
            interval_secs = config.heartbeat_interval_secs,
            "Federation peer-health-probe task started"
        );

        let mut probe_counter: u32 = 0;
        let probe_interval: u32 = 5; // Every 5th interval (~5 minutes)
        let mut roster = ProbeRoster::new();
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        loop {
            tokio::time::sleep(interval).await;

            // Peer health probing — every Nth tick
            probe_counter += 1;
            if probe_counter >= probe_interval {
                probe_counter = 0;

                let cached_peers = get_cached_peers(&state.peer_cache).await;
                let (live_roster, expired) = roster.begin_round(&cached_peers);
                if !expired.is_empty() {
                    for _ in &expired {
                        crate::metrics::record_doorbell(
                            crate::services::federation_doorbell::SIDE_PROBE,
                            crate::services::federation_doorbell::OUTCOME_SKIPPED_EXPIRED,
                        );
                    }
                    debug!(
                        probed = live_roster.len(),
                        skipped = expired.len(),
                        "Peer-health probe skipped roster members with no liveness evidence"
                    );
                }
                for peer in &live_roster {
                    let probe_start = std::time::Instant::now();
                    let health_url = format!("{}/health", peer.url.trim_end_matches('/'));

                    let (observed_status, response_time_ms, conductor_healthy) =
                        match http_client.get(&health_url).send().await {
                            Ok(resp) => {
                                let elapsed =
                                    probe_start.elapsed().as_millis().min(u32::MAX as u128) as u32;
                                if resp.status().is_success() {
                                    let conductor_ok =
                                        resp.json::<serde_json::Value>().await.ok().and_then(|v| {
                                            v.get("conductor")?.get("connected")?.as_bool()
                                        });
                                    let status = if conductor_ok == Some(true) {
                                        "online"
                                    } else {
                                        "degraded"
                                    };
                                    (status.to_string(), Some(elapsed), conductor_ok)
                                } else {
                                    ("degraded".to_string(), Some(elapsed), None)
                                }
                            }
                            Err(_) => ("unreachable".to_string(), None, None),
                        };
                    roster.observe(&peer.id, observed_status != "unreachable");

                    let attestation_input = RecordHealthAttestationInput {
                        attestor_doorway_id: config.doorway_id.clone(),
                        subject_doorway_id: peer.id.clone(),
                        observed_status: observed_status.clone(),
                        response_time_ms,
                        conductor_healthy,
                    };

                    match rmp_serde::to_vec(&attestation_input) {
                        Ok(payload) => {
                            match zome_caller
                                .call_zome(
                                    &config.infrastructure_role,
                                    &config.zome_name,
                                    "record_health_attestation",
                                    payload,
                                )
                                .await
                            {
                                Ok(_) => {
                                    debug!(
                                        peer = %peer.id,
                                        status = %observed_status,
                                        "Health attestation recorded"
                                    );
                                }
                                Err(e) => {
                                    warn!(
                                        peer = %peer.id,
                                        error = %e,
                                        "Failed to record health attestation"
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to serialize attestation input");
                        }
                    }
                }
            }
        }
    })
}

// =============================================================================
// Cross-Doorway Content Fetch — RETIRED
// =============================================================================
//
// `fetch_from_remote_doorway` lived here: a DHT `find_publishers` query that
// iterated remote publishers for one blob hash, resolved each one's `did:web`
// document, and pulled bytes from whichever answered. It was NEVER CALLED by
// anything, while its own doc comment claimed DeliveryRelay invoked it as a
// final fallback tier — dead code with a lying doc comment, the worst state.
//
// It is deleted rather than wired, per the 2026-09-12 operator ruling (WS3 /
// Task 3.3): the federated fallback the doorway actually owes is a **name**
// route, not a blob route. Publisher iteration for bytes is the shape
// `doorway/CLAUDE.md` §"No Blob Fan-Out" forbids — byte mobility belongs to
// the substrate's replication, not to the web2 projection. What replaced it:
// `crate::services::name_routing` — a registry fold over sibling doorways'
// live `project-epr` contracts plus ONE forwarded hop, carrying the same
// `x-federation-hop` loop-prevention header this function introduced.

// =============================================================================
// Doorway List (for federation routes)
// =============================================================================

/// Get all registered doorways from the DHT.
/// Used by the /api/v1/federation/doorways endpoint.
///
/// DHT-native federation discovery: enumerates every registered doorway via the
/// infrastructure zome's `get_all_doorways` list-all anchor. This rides the
/// conductor DHT gossip plane (WAN-NAT-correct) — a registration written through
/// any doorway's conductor gossips to every peer's DHT, so each doorway sees the
/// whole federation, not just itself. The FEDERATION_PEERS HTTP loop is a bootstrap
/// fallback; this is the canonical path.
pub async fn get_all_doorways(
    zome_caller: &ZomeCaller,
    config: &FederationConfig,
) -> Result<Vec<DoorwayRegistration>, String> {
    // FAILOVER-ELIGIBLE: `get_all_doorways` is an agent-agnostic DHT read — the
    // registration set gossips to every peer, so any conductor in the pool answers
    // identically and no signing identity is embedded in the result. Pinning this
    // to one conductor is what made a single sick peer (adam, sqlite write-guard
    // pressure, 2026-08-05) take `GET /api/v1/federation/doorways` down on
    // doorway-B for 5.5 hours.
    match zome_caller
        .call_failover::<(), Vec<DoorwayOutput>>(
            &config.infrastructure_role,
            &config.zome_name,
            "get_all_doorways",
            &(),
        )
        .await
    {
        Ok(outputs) => Ok(outputs.into_iter().map(|o| o.doorway).collect()),
        Err(e) => {
            warn!("Failed to query doorways from DHT: {}", e);
            Err(e)
        }
    }
}

// =============================================================================
// Peer Discovery (HTTP-based federation)
// =============================================================================

/// Cached peer doorway info from HTTP federation queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerDoorway {
    pub id: String,
    pub url: String,
    pub region: Option<String>,
    pub capabilities: Vec<String>,
    pub source_peer: String,
    /// The registration's monotonic signed serial, when the reporting peer
    /// read it from the DHT (`None` for a peer-cache echo). It bumps on every
    /// (re-)registration, so a change is liveness evidence for [`ProbeRoster`]:
    /// a doorway that restarts re-registers and re-enters the probe roster.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_serial: Option<u64>,
}

/// Shared cache of known peer doorways, refreshed periodically
pub type PeerCache = Arc<tokio::sync::RwLock<Vec<PeerDoorway>>>;

/// Create a new empty peer cache
pub fn new_peer_cache() -> PeerCache {
    Arc::new(tokio::sync::RwLock::new(Vec::new()))
}

// =============================================================================
// Peer JWKS Cache (cross-doorway EdDSA trust — Task 2.1)
// =============================================================================
//
// A sibling doorway's Ed25519 public key, keyed by `kid` (== that doorway's
// `doorway_id`). Populated out-of-band by `refresh_peer_jwks_cache` /
// `ensure_peer_key_cached` below — `auth::jwt::JwtValidator::verify_token`
// itself NEVER touches the network; it only reads this cache synchronously
// (via the `PeerKeyLookup` trait it implements).

/// TTL for a positively-cached peer public key before it's considered stale
/// and eligible for re-fetch.
pub const PEER_JWKS_TTL: Duration = Duration::from_secs(600);

/// Cooldown after a failed on-demand fetch attempt for a given `kid` — keeps
/// a burst of foreign-`kid` verify calls from becoming a fetch storm. Only
/// ONE attempt happens per cooldown window per `kid`.
pub const PEER_JWKS_NEGATIVE_COOLDOWN: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
enum JwksCacheEntry {
    /// A verified Ed25519 public key, fetched at `fetched_at`.
    Positive {
        pubkey: [u8; 32],
        fetched_at: Instant,
    },
    /// The last on-demand fetch attempt for this `kid` failed at this
    /// instant; no further attempt until `PEER_JWKS_NEGATIVE_COOLDOWN`
    /// elapses.
    NegativeCooldown { attempted_at: Instant },
}

/// Shared `kid` -> Ed25519 public key cache, sourced from peers'
/// `/.well-known/doorway-keys` JWKS. Cheap to clone (Arc-backed). Implements
/// `auth::jwt::PeerKeyLookup` so a `JwtValidator` can be handed read access
/// via `.with_peer_key_lookup(...)`.
#[derive(Clone)]
pub struct PeerJwksCache(Arc<dashmap::DashMap<String, JwksCacheEntry>>);

impl PeerJwksCache {
    /// Synchronous, network-free lookup, TTL-bounded.
    pub fn get(&self, kid: &str) -> Option<[u8; 32]> {
        match self.0.get(kid).map(|entry| entry.clone()) {
            Some(JwksCacheEntry::Positive { pubkey, fetched_at })
                if fetched_at.elapsed() < PEER_JWKS_TTL =>
            {
                Some(pubkey)
            }
            _ => None,
        }
    }

    /// Anchor `kid` to `pubkey`. Returns `false` when the insert was
    /// REFUSED: a live positive entry already anchors this `kid` to a
    /// DIFFERENT key, and a JWKS document that disagrees with a live anchor
    /// is a takeover attempt, not a refresh. Re-asserting the SAME pubkey is
    /// a legitimate refresh and succeeds (sliding the TTL); once the anchor
    /// has aged past `PEER_JWKS_TTL` it is no longer live and a rotated key
    /// may take its place.
    fn insert_positive(&self, kid: String, pubkey: [u8; 32]) -> bool {
        if let Some(JwksCacheEntry::Positive {
            pubkey: anchored,
            fetched_at,
        }) = self.0.get(&kid).map(|entry| entry.clone())
        {
            if anchored != pubkey && fetched_at.elapsed() < PEER_JWKS_TTL {
                warn!(
                    kid = %kid,
                    "Refusing to replace a live peer JWKS trust anchor with a different pubkey"
                );
                return false;
            }
        }
        self.0.insert(
            kid,
            JwksCacheEntry::Positive {
                pubkey,
                fetched_at: Instant::now(),
            },
        );
        true
    }

    fn mark_negative(&self, kid: &str) {
        self.0.insert(
            kid.to_string(),
            JwksCacheEntry::NegativeCooldown {
                attempted_at: Instant::now(),
            },
        );
    }

    /// True when an on-demand fetch attempt for `kid` is warranted right
    /// now: no live positive entry, and either no prior failed attempt or
    /// its cooldown has elapsed. Guards `ensure_peer_key_cached` from a
    /// per-request fetch storm.
    fn should_attempt_fetch(&self, kid: &str) -> bool {
        match self.0.get(kid).map(|entry| entry.clone()) {
            None => true,
            Some(JwksCacheEntry::Positive { fetched_at, .. }) => {
                fetched_at.elapsed() >= PEER_JWKS_TTL
            }
            Some(JwksCacheEntry::NegativeCooldown { attempted_at }) => {
                attempted_at.elapsed() >= PEER_JWKS_NEGATIVE_COOLDOWN
            }
        }
    }
}

/// Create a new empty peer JWKS cache.
pub fn new_peer_jwks_cache() -> PeerJwksCache {
    PeerJwksCache(Arc::new(dashmap::DashMap::new()))
}

impl crate::auth::jwt::PeerKeyLookup for PeerJwksCache {
    fn lookup_key(&self, kid: &str) -> Option<[u8; 32]> {
        self.get(kid)
    }
}

/// Wire shape of `GET /.well-known/doorway-keys` — mirrors
/// `routes::federation::JwksResponse` / `JwkKey` (kept independent here so
/// this module has no route-layer dependency).
#[derive(Deserialize)]
struct JwksWire {
    keys: Vec<JwkWire>,
}

#[derive(Deserialize)]
struct JwkWire {
    kty: String,
    crv: String,
    kid: String,
    x: String,
}

fn decode_base64url_pubkey(x: &str) -> Option<[u8; 32]> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let bytes = URL_SAFE_NO_PAD.decode(x).ok()?;
    bytes.try_into().ok()
}

/// Fetches a single peer's JWKS document. Never errors outward — a down or
/// non-conformant peer just yields an empty list (mirrors `fetch_single_peer`
/// above).
async fn fetch_peer_jwks(client: &reqwest::Client, peer_url: &str) -> Vec<(String, [u8; 32])> {
    let url = format!(
        "{}/.well-known/doorway-keys",
        peer_url.trim_end_matches('/')
    );
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.json::<JwksWire>().await {
            Ok(parsed) => parsed
                .keys
                .into_iter()
                .filter(|k| k.kty == "OKP" && k.crv == "Ed25519")
                .filter_map(|k| decode_base64url_pubkey(&k.x).map(|pubkey| (k.kid, pubkey)))
                .collect(),
            Err(e) => {
                warn!(peer = %peer_url, error = %e, "Failed to parse peer JWKS document");
                Vec::new()
            }
        },
        Ok(resp) => {
            debug!(peer = %peer_url, status = %resp.status(), "Peer JWKS query returned non-success");
            Vec::new()
        }
        Err(e) => {
            debug!(peer = %peer_url, error = %e, "Failed to reach peer JWKS endpoint");
            Vec::new()
        }
    }
}

/// Refreshes the peer JWKS cache by querying every known peer's
/// `/.well-known/doorway-keys`. Intended to run on the SAME cadence as
/// `refresh_peer_cache` — piggybacking the peer list the existing discovery
/// task already maintains — never per-request.
pub async fn refresh_peer_jwks_cache(
    peers: &[PeerDoorway],
    client: &reqwest::Client,
    cache: &PeerJwksCache,
) {
    for peer in peers {
        if peer.url.trim().is_empty() {
            continue;
        }
        let keys = fetch_peer_jwks(client, &peer.url).await;
        for (kid, pubkey) in keys {
            cache.insert_positive(kid, pubkey);
        }
    }
}

/// On-demand, cooldown-guarded single-`kid` fetch for a verify-time cache
/// miss. `should_attempt_fetch` enforces the negative cooldown BEFORE any
/// network I/O happens, so a burst of requests for the same unresolvable
/// `kid` triggers at most one fetch per cooldown window.
pub async fn ensure_peer_key_cached(
    cache: &PeerJwksCache,
    kid: &str,
    peer_url: &str,
    client: &reqwest::Client,
) -> Option<[u8; 32]> {
    if let Some(key) = cache.get(kid) {
        return Some(key);
    }
    if !cache.should_attempt_fetch(kid) {
        return None;
    }
    let keys = fetch_peer_jwks(client, peer_url).await;
    let mut found = None;
    for (fetched_kid, pubkey) in keys {
        let is_target = fetched_kid == kid;
        let accepted = cache.insert_positive(fetched_kid, pubkey);
        if is_target {
            // A REFUSED insert means a live anchor already holds this `kid`
            // with a different key — serve the incumbent, never the
            // challenger, and never fall through to `mark_negative` (which
            // would let a challenger evict a live anchor).
            found = if accepted {
                Some(pubkey)
            } else {
                cache.get(kid)
            };
        }
    }
    if found.is_none() {
        cache.mark_negative(kid);
    }
    found
}

/// The doorways a JWKS refresh may take a trust anchor from: a registration
/// the infrastructure zome holds AND that carries a `signing_key`.
///
/// A registration with an empty `signing_key` has not yet bound an identity,
/// so whatever answers its URL could claim any `kid` it likes. It is filtered
/// out here rather than at insert time so `refresh_peer_jwks_cache` is never
/// handed one at all — no request, no chance to answer.
fn jwks_trust_anchors(registrations: &[DoorwayRegistration]) -> Vec<PeerDoorway> {
    registrations
        .iter()
        .filter(|reg| !reg.signing_key.trim().is_empty() && !reg.url.trim().is_empty())
        .map(|reg| PeerDoorway {
            id: reg.id.clone(),
            url: reg.url.clone(),
            region: reg.region.clone(),
            capabilities: Vec::new(),
            source_peer: "dht-registration".to_string(),
            record_serial: Some(reg.record_serial),
        })
        .collect()
}

/// Spawns a background task that periodically refreshes the peer JWKS cache
/// from the DHT doorway registry — NOT from `PeerCache`.
///
/// `PeerCache` is populated entirely by what peers report over HTTP
/// (`fetch_single_peer`), carries no signature material, and is never
/// verified against anything; sourcing trust anchors from it let ANY doorway
/// that appears in gossip publish a `kid` and overwrite a sibling's JWT
/// verification key. `get_all_doorways` reads the registrations the
/// infrastructure zome admitted to the DHT instead, and only those carrying a
/// `signing_key` are fetched. `initial_delay`/`interval` mirror
/// `spawn_peer_discovery_task`'s cadence.
pub fn spawn_peer_jwks_refresh_task(
    zome_caller: Arc<ZomeCaller>,
    config: FederationConfig,
    jwks_cache: PeerJwksCache,
    initial_delay: Duration,
    interval: Duration,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        tokio::time::sleep(initial_delay).await;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        loop {
            match get_all_doorways(&zome_caller, &config).await {
                Ok(registrations) => {
                    let anchors = jwks_trust_anchors(&registrations);
                    if anchors.is_empty() {
                        debug!(
                            registration_count = registrations.len(),
                            "Peer JWKS refresh: no registered doorway carries a signing key"
                        );
                    } else {
                        refresh_peer_jwks_cache(&anchors, &client, &jwks_cache).await;
                    }
                }
                Err(e) => {
                    debug!(
                        error = %e,
                        "Peer JWKS refresh: doorway registry unavailable this tick"
                    );
                }
            }
            tokio::time::sleep(interval).await;
        }
    })
}

// =============================================================================
// Cross-Edge Coherence Probe (F-COHERENCE)
// =============================================================================

/// Probe results from sibling doorways' `/api/v1/federation/coherence`.
///
/// Cat-C node-local read-model (transport-only here; the DATA type +
/// comparison logic live in `crate::routes::coherence`). Refreshed inside the
/// existing peer-discovery loop — observability-only, NEVER reconciled.
pub type PeerCoherenceCache =
    Arc<tokio::sync::RwLock<Vec<crate::routes::coherence::PeerCoherence>>>;

/// Create a new empty peer-coherence cache.
pub fn new_peer_coherence_cache() -> PeerCoherenceCache {
    Arc::new(tokio::sync::RwLock::new(Vec::new()))
}

/// Fetch one peer's `CoherenceManifest`, returning a TRI-STATE
/// `(reachable, Option<CoherenceManifest>)` so a 200-with-garbage-body is
/// distinguished from a down / not-yet-deployed peer (fix 2):
///
///   - transport error, 404, or ANY non-200 (incl. 5xx during a rolling deploy)
///     → `(false, None)`: down or not-yet-deployed; never fires a false
///     divergence alarm while a sibling is still deploying.
///   - HTTP 200 but the body fails to deserialize → `(true, None)`: a peer
///     serving a 200 garbage/incompatible body IS a real (mixed-version)
///     divergence to surface, NOT "still deploying".
///   - HTTP 200 + a parseable manifest → `(true, Some(manifest))`.
///
/// The 5xx → `(false, None)` mapping is deliberate: a 503 during a rolling
/// deploy mapping to `reachable=true` would manufacture exactly the deploy-noise
/// WARN this feature exists to avoid. ONLY `200 && body-parse-fails` is the
/// reachable-but-divergent case.
///
/// NO per-request timeout here — the `coherence_client` already carries a
/// client-level 5s timeout (fix 10: one timeout, not two).
///
/// (X-COH-DEF: retry cadence DEFERS to `elohim_compute::backoff::jittered` when
/// present — until then the existing loop interval is the cadence.)
pub(crate) async fn fetch_peer_coherence(
    client: &reqwest::Client,
    peer_url: &str,
) -> (bool, Option<crate::routes::coherence::CoherenceManifest>) {
    let url = format!(
        "{}/api/v1/federation/coherence",
        peer_url.trim_end_matches('/')
    );
    match client.get(&url).send().await {
        // 200 OK → reachable; body may or may not deserialize.
        Ok(r) if r.status().is_success() => match r.json().await {
            Ok(manifest) => (true, Some(manifest)),
            // 200 but the body is garbage/incompatible → reachable, no manifest.
            // This IS a divergence (mixed version), so reachable=true is correct.
            Err(_) => (true, None),
        },
        // Any non-200 (404 not-yet-deployed, 5xx rolling deploy) → not reachable.
        _ => (false, None),
    }
}

/// Probe each known peer's coherence manifest CONCURRENTLY, compare each to OUR
/// self-manifest, store the per-peer verdicts in `cache`, and emit a structured
/// WARN alarm ONLY for peers whose divergence is NEW or CHANGED since the prior
/// tick (edge-triggered — fix 5).
///
/// Fix 1 — concurrency: the per-peer probes run via `join_all`, so a tick's
/// latency is ~max(timeout), not the sum of N serial 5s probes. This keeps the
/// 60s discovery cadence (which also feeds `PeerCache` / `/p2p-peers`) from
/// drifting when several siblings are slow.
///
/// Fix 6 — empty `peer_url`s are skipped (an empty url builds a relative URL
/// reqwest rejects, swallowed as `(false, None)` forever).
///
/// Fix 5 — the WARN is edge-triggered: we read the PRIOR cache snapshot before
/// overwriting it and warn only on newly/changed-divergent peers
/// (`divergences_to_warn`). This kills the per-tick log-spam AND makes
/// `coherence_cache` a real READER, not a write-only dead store.
///
/// `self_manifest` is supplied by the caller each tick (the caller gen-gates the
/// recompute). `peers` is a snapshot of the federation peer cache — `(id, url)`
/// pairs to probe.
pub async fn refresh_coherence(
    self_manifest: &crate::routes::coherence::CoherenceManifest,
    peers: &[(String, String)],
    client: &reqwest::Client,
    cache: &PeerCoherenceCache,
    name_routes: Option<&crate::services::name_routing::NameRouteTable>,
) {
    use crate::routes::coherence::{compare_to_peer, divergences_to_warn};

    // Story 4.2 slice 1: the round's START, captured BEFORE any probe fires —
    // this is the `fetch_started` `install_name_routes` hands to
    // `NameRouteTable::replace_all_at` so a holder the doorbell installed
    // WHILE this round was in flight is never clobbered by this round's
    // (now-stale) copy of that holder (C2 monotonic).
    let fetch_started = doorbell_now_secs();

    // Fix 1 + 6: probe every non-empty-url peer CONCURRENTLY; each future does
    // fetch + compare and yields its `PeerCoherence` verdict.
    //
    // The probe's manifest is RETAINED (not dropped after the compare) because
    // a peer's head set IS its live `project-epr` contract list — exactly the
    // registry the name-route fold needs. Populating `name_routes` here costs
    // ZERO additional I/O: same tick, same response, second reader.
    let probes = peers
        .iter()
        .filter(|(_, peer_url)| !peer_url.trim().is_empty())
        .map(|(peer_id, peer_url)| async move {
            let (reachable, peer_manifest) = fetch_peer_coherence(client, peer_url).await;
            (peer_id.clone(), peer_url.clone(), reachable, peer_manifest)
        });
    let probed: Vec<(
        String,
        String,
        bool,
        Option<crate::routes::coherence::CoherenceManifest>,
    )> = futures::future::join_all(probes).await;

    if let Some(table) = name_routes {
        install_name_routes(table, &probed, fetch_started);
    }

    let results: Vec<crate::routes::coherence::PeerCoherence> = probed
        .iter()
        .map(|(peer_id, _, reachable, manifest)| {
            compare_to_peer(self_manifest, peer_id, *reachable, manifest.as_ref())
        })
        .collect();

    // Fix 5 — edge-triggered divergence ALARM. Read the PRIOR verdicts BEFORE
    // overwriting the cache, and warn only on the newly/changed-divergent subset.
    // Make the degraded state LOUD (FallbackOutcome precedent: the EPR-router
    // degraded state hid for days at DEBUG). Naming BOTH doorway_ids + the build
    // SHAs lets the operator instantly tell content-skew (digests differ, builds
    // equal) from deploy-skew (builds differ — the actual "two EPR heads" symptom).
    let prior = cache.read().await.clone();
    for pc in divergences_to_warn(&prior, &results) {
        warn!(
            self_doorway = %self_manifest.doorway_id,
            peer_doorway = %pc.doorway_id,
            self_digest = %self_manifest.digest,
            peer_digest = ?pc.digest,
            self_build = ?self_manifest.build_id,
            peer_build = ?pc.build_id,
            divergent_paths = ?pc.divergent_paths,
            "CROSS-EDGE EPR HEAD DIVERGENCE — two doorways serving different heads"
        );
    }

    let mut cache_write = cache.write().await;
    *cache_write = results;
}

/// Project a coherence-probe round into the name-route table.
///
/// Each probed peer contributes one [`HolderContract`] per mounted root it
/// reports — that is the peer's own projection of its `project-epr`
/// commitments, self-labelled with the `doorway_id` its manifest asserts (the
/// discovery-advertised id can lag). Liveness is the probe's tri-state:
/// manifest present → `Serving`; 200 with an unreadable body → `Uncertain`;
/// anything else (transport error, 404, 5xx — a shed included) → `Unreachable`.
///
/// The probe genuinely cannot tell a shed from a death; only a relay attempt
/// can, and `NameRouteTable::note_shed` is where that observation lands. An
/// `Unreachable` holder is still folded (ordered last), because this verdict is
/// up to one discovery interval stale.
///
/// Honest edge: an unreachable peer has no manifest, so its liveness is keyed
/// by the DISCOVERY-advertised `peer_id`. If that id ever differs from the
/// `doorway_id` a retained last-good contract carries, the stale holder folds
/// as `Uncertain` rather than `Unreachable` — it is tried first and fails, and
/// the next holder serves. One wasted attempt, never a wrong answer.
///
/// [`HolderContract`]: crate::services::name_routing::HolderContract
///
/// Build ONE peer's [`HolderContract`]s + liveness verdict from a probe
/// result. Shared by the whole-batch fold ([`install_name_routes`]) and the
/// single-holder install path ([`install_holder_snapshot`]), so both agree on
/// exactly what "this peer's contracts" means (story 4.2 slice 1, design
/// §3.2). Returns the RESOLVED doorway_id (the manifest's self-report when
/// present, the probe's discovery id otherwise) alongside the built rows.
fn holder_contracts_and_liveness(
    peer_id: &str,
    peer_url: &str,
    reachable: bool,
    manifest: Option<&crate::routes::coherence::CoherenceManifest>,
) -> (
    String,
    Vec<crate::services::name_routing::HolderContract>,
    crate::services::name_routing::HolderLiveness,
) {
    use crate::services::name_routing::{HolderContract, HolderLiveness};

    match manifest {
        Some(m) => {
            let doorway_id = if m.doorway_id.trim().is_empty() {
                peer_id.to_string()
            } else {
                m.doorway_id.clone()
            };
            let mut contracts = Vec::new();
            for head in &m.heads {
                if head.hostnames.is_empty() {
                    contracts.push(
                        HolderContract::any_host(&doorway_id, peer_url, &head.url_path)
                            .with_projection(head.commitment_id.clone(), Some(head.epr_id.clone())),
                    );
                } else {
                    for hostname in &head.hostnames {
                        let Some(host) = crate::services::name_routing::RouteKey::new(
                            Some(hostname),
                            &head.url_path,
                        )
                        .host
                        else {
                            continue;
                        };
                        contracts.push(HolderContract {
                            doorway_id: doorway_id.clone(),
                            origin: peer_url.to_string(),
                            url_path: head.url_path.clone(),
                            host: Some(host),
                            commitment_id: head.commitment_id.clone(),
                            epr_id: Some(head.epr_id.clone()),
                        });
                    }
                }
            }
            (doorway_id, contracts, HolderLiveness::Serving)
        }
        None => (
            peer_id.to_string(),
            Vec::new(),
            if reachable {
                HolderLiveness::Uncertain
            } else {
                HolderLiveness::Unreachable
            },
        ),
    }
}

/// One shared sort key for the name-route table: doorway_id, then url_path,
/// then host. The ONE definition — [`install_name_routes`],
/// [`install_holder_snapshot`], and `NameRouteTable::replace_holder`/
/// `replace_all_at` must all agree on the OwnerOrder tiebreak, or the fold's
/// "stable final tiebreak" (see `selector_rank`) would depend on which path
/// last touched a given holder.
fn sort_contracts_by_owner_order(contracts: &mut [crate::services::name_routing::HolderContract]) {
    contracts.sort_by(|left, right| {
        left.doorway_id
            .cmp(&right.doorway_id)
            .then_with(|| left.url_path.cmp(&right.url_path))
            .then_with(|| left.host.cmp(&right.host))
    });
}

fn install_name_routes(
    table: &crate::services::name_routing::NameRouteTable,
    probed: &[(
        String,
        String,
        bool,
        Option<crate::routes::coherence::CoherenceManifest>,
    )],
    fetch_started: u64,
) {
    use crate::services::name_routing::{HolderContract, HolderLiveness};
    use std::collections::HashMap;

    let mut contracts: Vec<HolderContract> = Vec::new();
    let mut liveness: HashMap<String, HolderLiveness> = HashMap::new();
    let mut digests: HashMap<String, String> = HashMap::new();

    for (peer_id, peer_url, reachable, manifest) in probed {
        let (doorway_id, peer_contracts, peer_liveness) =
            holder_contracts_and_liveness(peer_id, peer_url, *reachable, manifest.as_ref());
        if let Some(m) = manifest {
            digests.insert(doorway_id.clone(), m.digest.clone());
        }
        liveness.insert(doorway_id, peer_liveness);
        contracts.extend(peer_contracts);
    }

    // OwnerOrder — the selector's FINAL TIEBREAK — is the first-appearance order
    // of same-key contracts in this vector (`fold_candidate_holders` enumerates
    // them). `selector_rank` documents that term as a "stable final tiebreak",
    // and until this sort it was not stable at all: the order arrived from
    // `probed`, i.e. `refresh_peer_cache`'s cache, which is the union of what
    // each seed's own `GET /api/v1/federation/doorways` returned — so the FIRST
    // reachable seed's DHT link order silently became every doorway's holder
    // priority. `merge_discovery_seeds` already sorts DHT registrations "by id
    // for determinism"; `refresh_peer_cache`'s per-seed fan-out then threw that
    // away, and nothing downstream restored it.
    //
    // Measured on the household mesh 2026-09-21 (run R1): with "garden" held by
    // alpha and gamma, beta's fold put the MOST RECENTLY REGISTERED doorway
    // (gamma) first, so every relay went to gamma and the busy holder alpha was
    // never dialled at all — which made the balance scenario's first two claims
    // pass for the wrong reason (gamma served because it was first, not because
    // alpha was set aside: zero `name_route_shed` lines, both demotion counters
    // 0) and its recovery claim unreachable.
    //
    // Sorting by doorway_id extends the module's OWN existing determinism rule
    // to the place that discarded it. It is arbitrary-but-stable, which is
    // exactly what a final tiebreak must be — the meaningful terms
    // (Liveness, ReachStanding, Nearest, Weight) all rank above it.
    sort_contracts_by_owner_order(&mut contracts);

    debug!(
        contracts = contracts.len(),
        peers = probed.len(),
        "name-route table refreshed from the coherence probe"
    );
    table.replace_all_at(contracts, liveness, digests, fetch_started);
}

/// Install ONE holder's manifest into `table` — the single-holder counterpart
/// to [`install_name_routes`]'s whole-batch fold, sharing the same contract
/// builder ([`holder_contracts_and_liveness`]) and the same final sort so both
/// paths agree on exactly what "this peer's contracts" means (story 4.2
/// slice 1, design §3.2). Used by the doorbell receiver
/// (`services::federation_doorbell::receive_doorbell`) and the admin refresh
/// verb (`routes::federation::handle_admin_refresh_federation_peers`).
pub(crate) fn install_holder_snapshot(
    table: &crate::services::name_routing::NameRouteTable,
    peer_id: &str,
    peer_url: &str,
    manifest: &crate::routes::coherence::CoherenceManifest,
    now: u64,
) {
    let (doorway_id, mut contracts, liveness) =
        holder_contracts_and_liveness(peer_id, peer_url, true, Some(manifest));
    sort_contracts_by_owner_order(&mut contracts);
    table.replace_holder(
        &doorway_id,
        contracts,
        liveness,
        manifest.digest.clone(),
        now,
    );
}

/// Outcome of pulling and installing one sibling's manifest under an EXPECTED
/// doorway_id (the id the caller resolved from `peer_cache` or a doorbell
/// body) — shared by the doorbell receiver and the admin refresh verb (design
/// §3.2/§3.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PullOutcome {
    Installed,
    /// Reachable, but the manifest's self-reported `doorwayId` differs from
    /// what the caller expected (risk 2 / C1 anti-self-election) — never
    /// installed under the wrong key. The next discovery poll catches this
    /// peer instead; the skip is logged, not hidden.
    IdMismatch {
        manifest_doorway_id: String,
    },
    /// Transport error, 404, or any non-200 (5xx included).
    Unreachable,
    /// 200 OK but the body did not deserialize.
    GarbageBody,
}

/// Fetch `expected_doorway_id`'s own coherence manifest at `peer_url` and, if
/// it checks out, install it as that ONE holder's snapshot via
/// [`install_holder_snapshot`]. Shared by `routes::coherence::handle_doorbell`
/// (through `services::federation_doorbell::receive_doorbell`) and
/// `routes::federation::handle_admin_refresh_federation_peers`.
pub(crate) async fn pull_and_install_holder(
    table: &crate::services::name_routing::NameRouteTable,
    expected_doorway_id: &str,
    peer_url: &str,
    client: &reqwest::Client,
) -> PullOutcome {
    let (reachable, manifest) = fetch_peer_coherence(client, peer_url).await;
    match (reachable, manifest) {
        (true, Some(m)) => {
            if !m.doorway_id.trim().is_empty() && m.doorway_id != expected_doorway_id {
                return PullOutcome::IdMismatch {
                    manifest_doorway_id: m.doorway_id,
                };
            }
            install_holder_snapshot(
                table,
                expected_doorway_id,
                peer_url,
                &m,
                doorbell_now_secs(),
            );
            PullOutcome::Installed
        }
        (true, None) => PullOutcome::GarbageBody,
        (false, _) => PullOutcome::Unreachable,
    }
}

/// Wall-clock seconds — the one clock boundary the doorbell/refresh pull path
/// needs. Mirrors `services::name_routing`'s private `now_secs` (duplicated
/// here rather than exposed across the module boundary, since that module
/// deliberately keeps its clock read private so its pure logic stays
/// clock-free in tests).
pub(crate) fn doorbell_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Shared mutable list of federation peer URLs.
/// Seeded from FEDERATION_PEERS env var, mutable via admin API at runtime.
pub type PeerUrlList = Arc<tokio::sync::RwLock<Vec<String>>>;

/// Create a new peer URL list from initial seeds
pub fn new_peer_url_list(initial: Vec<String>) -> PeerUrlList {
    Arc::new(tokio::sync::RwLock::new(initial))
}

/// Add a peer URL to the list. Returns true if added (not a duplicate).
pub async fn add_peer_url(list: &PeerUrlList, url: String) -> bool {
    let mut urls = list.write().await;
    if urls.contains(&url) {
        false
    } else {
        urls.push(url);
        true
    }
}

/// Remove a peer URL from the list. Returns true if removed.
pub async fn remove_peer_url(list: &PeerUrlList, url: &str) -> bool {
    let mut urls = list.write().await;
    let len_before = urls.len();
    urls.retain(|u| u != url);
    urls.len() < len_before
}

/// Get a snapshot of the current peer URL list.
pub async fn get_peer_urls(list: &PeerUrlList) -> Vec<String> {
    list.read().await.clone()
}

/// Fetch federation info from a single peer doorway.
/// Queries `{peer_url}/api/v1/federation/doorways` and returns parsed doorways.
async fn fetch_single_peer(client: &reqwest::Client, peer_url: &str) -> Vec<PeerDoorway> {
    let url = format!(
        "{}/api/v1/federation/doorways",
        peer_url.trim_end_matches('/')
    );

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            // Parse the FederationDoorwaysResponse shape
            #[derive(Deserialize)]
            struct PeerResponse {
                doorways: Vec<PeerEntry>,
            }
            #[derive(Deserialize)]
            struct PeerEntry {
                id: String,
                url: String,
                region: Option<String>,
                capabilities: Vec<String>,
                #[allow(dead_code)]
                status: Option<String>,
                #[serde(default)]
                record_serial: Option<u64>,
            }

            match resp.json::<PeerResponse>().await {
                Ok(parsed) => parsed
                    .doorways
                    .into_iter()
                    .map(|d| PeerDoorway {
                        id: d.id,
                        url: d.url,
                        region: d.region,
                        capabilities: d.capabilities,
                        source_peer: peer_url.to_string(),
                        record_serial: d.record_serial,
                    })
                    .collect(),
                Err(e) => {
                    warn!(peer = %peer_url, error = %e, "Failed to parse peer federation response");
                    Vec::new()
                }
            }
        }
        Ok(resp) => {
            debug!(peer = %peer_url, status = %resp.status(), "Peer federation query returned non-success");
            Vec::new()
        }
        Err(e) => {
            debug!(peer = %peer_url, error = %e, "Failed to reach peer doorway");
            Vec::new()
        }
    }
}

/// Refresh the peer cache by querying all configured peers.
/// Deduplicates by doorway id — if multiple peers report the same doorway,
/// the first occurrence wins.
pub async fn refresh_peer_cache(peer_urls: &[String], self_id: Option<&str>, cache: &PeerCache) {
    if peer_urls.is_empty() {
        // Fix 12: no configured peers → clear the cache rather than stranding a
        // stale set. A stale entry left here would be a phantom peer the
        // coherence probe alarms against even after federation was emptied.
        let mut cache_write = cache.write().await;
        cache_write.clear();
        return;
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    let mut all_peers = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    // Exclude self from results
    if let Some(id) = self_id {
        seen_ids.insert(id.to_string());
    }

    for peer_url in peer_urls {
        let discovered = fetch_single_peer(&client, peer_url).await;
        for peer in discovered {
            if seen_ids.insert(peer.id.clone()) {
                all_peers.push(peer);
            }
        }
    }

    let count = all_peers.len();
    {
        let mut cache_write = cache.write().await;
        *cache_write = all_peers;
    }

    if count > 0 {
        info!(
            peers = count,
            sources = peer_urls.len(),
            "Federation peer cache refreshed"
        );
    }
}

/// Normalize a candidate seed URL to a comparable origin form, or `None` if it
/// is not an http(s) URL we could ever fetch from.
fn normalize_seed(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let parsed = reqwest::Url::parse(trimmed).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return None;
    }
    Some(trimmed.to_string())
}

/// **Where peer discovery gets its seeds.** Static `FEDERATION_PEERS` UNION the
/// DHT-registered doorway set, deduped by origin, self excluded.
///
/// This is the doorway-side half of "the registry IS the DHT" (2026-09-12
/// operator ruling). Peer discovery — and therefore the coherence probe, and
/// therefore the name-route table the one-hop relay folds over — used to be
/// GATED on a non-empty static peer list, so a doorway pair that knew each
/// other perfectly well through `register_doorway_in_dht` + heartbeat (both
/// visible on `GET /api/v1/federation/doorways`) never probed each other and
/// the name-route registry stayed empty forever. The static list is now an
/// ADDITIONAL SEED, never a gate.
///
/// Ordering: static seeds first (an operator's explicit list keeps its
/// priority and its meaning for existing telemetry), then DHT registrations by
/// id for determinism, and within one registration its `gateway` endpoints by
/// signed priority before the registration's own `url`.
///
/// Self-exclusion is belt AND braces: by registration id, and by origin
/// (`self_urls` = this doorway's own advertised URLs). A doorway that probed
/// itself would coherence-compare against its own manifest and, worse, could
/// fold itself in as a relay candidate.
pub fn merge_discovery_seeds(
    static_peers: &[String],
    registrations: &[DoorwayRegistration],
    self_doorway_id: Option<&str>,
    self_urls: &[String],
) -> Vec<String> {
    let mut seeds: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Never dial ourselves, whichever of our own addresses is offered.
    for own in self_urls {
        if let Some(normalized) = normalize_seed(own) {
            seen.insert(normalized);
        }
    }

    let push =
        |raw: &str, seeds: &mut Vec<String>, seen: &mut std::collections::HashSet<String>| {
            if let Some(normalized) = normalize_seed(raw) {
                if seen.insert(normalized.clone()) {
                    seeds.push(normalized);
                }
            }
        };

    for peer in static_peers {
        push(peer, &mut seeds, &mut seen);
    }

    let mut registered: Vec<&DoorwayRegistration> = registrations
        .iter()
        .filter(|reg| match self_doorway_id {
            Some(id) => reg.id != id,
            None => true,
        })
        .collect();
    registered.sort_by(|left, right| left.id.cmp(&right.id));

    for reg in registered {
        let mut gateways: Vec<(usize, &infrastructure_types::DoorwayEndpoint)> = reg
            .endpoints
            .iter()
            .enumerate()
            .filter(|(_, endpoint)| endpoint.service == "gateway")
            .collect();
        gateways.sort_by_key(|(signed_order, endpoint)| (endpoint.priority, *signed_order));
        for (_, endpoint) in gateways {
            push(&endpoint.url, &mut seeds, &mut seen);
        }
        push(&reg.url, &mut seeds, &mut seen);
    }

    seeds
}

/// One discovery tick's peer-cache half: merge the seeds, then refresh the
/// cache from them. Returns the seeds actually used so the caller (and tests)
/// can see what the tick dialed.
///
/// Extracted from the loop body so the DHT-seeded path is testable without a
/// conductor: a test supplies `registrations` directly, exactly as
/// `get_all_doorways` would have.
pub async fn refresh_peer_cache_from_seeds(
    static_peers: &[String],
    registrations: &[DoorwayRegistration],
    self_id: Option<&str>,
    self_urls: &[String],
    cache: &PeerCache,
) -> Vec<String> {
    let seeds = merge_discovery_seeds(static_peers, registrations, self_id, self_urls);
    refresh_peer_cache(&seeds, self_id, cache).await;
    seeds
}

/// Spawn a background task that periodically refreshes the peer cache.
/// Initial fetch happens after `initial_delay`, then every `interval`.
/// Reads peer URLs from the shared mutable list on each iteration.
///
/// Also runs the F-COHERENCE cross-edge probe on each tick (additive): it
/// recomputes THIS edge's self-manifest from `epr_router` and compares it
/// against every discovered peer's coherence manifest, storing the verdicts in
/// `coherence_cache` and alarming on divergence. Detection-only — never
/// reconciles.
#[allow(clippy::too_many_arguments)]
pub fn spawn_peer_discovery_task(
    peer_urls: PeerUrlList,
    self_id: Option<String>,
    cache: PeerCache,
    epr_router: Arc<crate::projection::EprRouter>,
    coherence_cache: PeerCoherenceCache,
    name_routes: Arc<crate::services::name_routing::NameRouteTable>,
    // DHT registry reader. `Some` means each tick also seeds from
    // `get_all_doorways` — the same source `GET /api/v1/federation/doorways`
    // serves — so a doorway pair that knows each other only through the DHT
    // still discovers, probes, and name-routes. `None` falls back to the
    // static list alone.
    dht_registry: Option<(Arc<ZomeCaller>, FederationConfig)>,
    // This doorway's own advertised URLs — never dialled as a peer.
    self_urls: Vec<String>,
    initial_delay: std::time::Duration,
    interval: std::time::Duration,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // Wait for startup to settle (peer doorways may still be booting)
        tokio::time::sleep(initial_delay).await;

        // BuildInfo is process-constant — construct once, reuse the commit SHA
        // for every per-tick self-manifest.
        let build = elohim_compute::BuildInfo::new("elohim-doorway");
        // Fix 3: cross-edge coherence is meaningless without a self-identity. If
        // `doorway_id` is unknown, the self-exclusion in `refresh_peer_cache`
        // (keyed on the real id) can't drop a self-echo, so the doorway would
        // coherence-probe ITSELF. `Some(id)` only when we have a real id (never
        // the "unknown" fallback).
        let coherence_self_id: Option<String> = match self_id.as_deref() {
            Some(id) if !id.is_empty() && id != "unknown" => Some(id.to_string()),
            _ => None,
        };
        let coherence_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        // Fix 8 — generation-gate cache: last `(generation, CoherenceManifest)`.
        // The self-manifest is only re-minted when the router generation changed.
        let mut cached_self_manifest: Option<(u64, crate::routes::coherence::CoherenceManifest)> =
            None;

        loop {
            // ALWAYS refresh the peer cache — it feeds /p2p-peers and the 60s
            // discovery cadence, independent of coherence. (`self_id`, including
            // the "unknown" fallback, is still used for self-exclusion here.)
            let static_urls = peer_urls.read().await.clone();
            // The registry IS the DHT: read the registered doorway set every
            // tick so a registration that appears (or an endpoint that moves)
            // changes the probe set without a doorway restart. A conductor
            // that is not ready yet yields an empty set and the static list
            // still seeds — the next tick retries, no operator step.
            let registrations = match dht_registry.as_ref() {
                Some((zome_caller, config)) => get_all_doorways(zome_caller, config)
                    .await
                    .unwrap_or_else(|e| {
                        debug!(
                            error = %e,
                            "Peer discovery: doorway registry unavailable this tick"
                        );
                        Vec::new()
                    }),
                None => Vec::new(),
            };
            let urls = refresh_peer_cache_from_seeds(
                &static_urls,
                &registrations,
                self_id.as_deref(),
                &self_urls,
                &cache,
            )
            .await;
            debug!(
                seeds = urls.len(),
                static_seeds = static_urls.len(),
                dht_registrations = registrations.len(),
                "Federation peer discovery tick"
            );

            // F-COHERENCE cross-edge probe — runs only when we have a real
            // self-identity (fix 3).
            if let Some(coherence_id) = coherence_self_id.as_deref() {
                // Fix 8: recompute the self-manifest only when the generation moved.
                let gen = epr_router.generation();
                if crate::routes::coherence::should_recompute_self_manifest(
                    cached_self_manifest.as_ref(),
                    gen,
                ) {
                    let minted = crate::routes::coherence::router_fingerprint(
                        epr_router.as_ref(),
                        coherence_id,
                        Some(&build.commit),
                    );
                    cached_self_manifest = Some((gen, minted));
                }
                // Safe: just set above when None.
                let self_manifest = &cached_self_manifest.as_ref().unwrap().1;

                // Fix 4: before the EPR router populates (boot / pre-first
                // projection) the self-manifest has zero heads → its digest is the
                // empty-set CID, which mismatches any populated peer → a spurious
                // divergence WARN. Skip the probe/compare/WARN entirely this tick.
                if !self_manifest.heads.is_empty() {
                    let peers: Vec<(String, String)> = get_cached_peers(&cache)
                        .await
                        .into_iter()
                        .map(|p| (p.id, p.url))
                        .collect();
                    refresh_coherence(
                        self_manifest,
                        &peers,
                        &coherence_client,
                        &coherence_cache,
                        Some(name_routes.as_ref()),
                    )
                    .await;
                }
            }

            tokio::time::sleep(interval).await;
        }
    })
}

/// Get the current list of known peers from the cache
pub async fn get_cached_peers(cache: &PeerCache) -> Vec<PeerDoorway> {
    cache.read().await.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    // ── Bounded DHT self-registration retry ──────────────────────────────────
    //
    // The defect these pin, measured on the household mesh
    // 2026-09-21T03:22:42Z (`logs/doorway-c.log`): gamma's ONE registration
    // attempt hit the conductor's cell-startup window (`CellDisabled`), logged
    // "Federation registration failed (non-fatal)", and never tried again —
    // so `gamma-elohim-host` was absent from every doorway's
    // `/api/v1/federation/doorways` for the life of the process and could
    // never be a holder in a sibling's name-route fold.

    /// A conductor still initialising its cells answers `CellDisabled` for
    /// minutes; the doorway must keep asking until it is ready. This is the
    /// exact shape of the live failure, replayed without the wait.
    #[tokio::test]
    async fn registration_retries_through_the_conductor_startup_window() {
        use std::cell::RefCell;
        let slept: RefCell<Vec<u64>> = RefCell::new(Vec::new());
        let outcome = drive_registration_with_retry(
            "gamma-elohim-host",
            |attempt| async move {
                if attempt < 4 {
                    Err("authorize_signing_credentials failed for role 'mishpat': \
                         CellDisabled(...)"
                        .to_string())
                } else {
                    Ok(())
                }
            },
            |secs| {
                slept.borrow_mut().push(secs);
                async {}
            },
        )
        .await;
        assert_eq!(outcome, RegistrationOutcome::Registered { attempts: 4 });
        assert_eq!(
            *slept.borrow(),
            vec![15, 30, 60],
            "one bounded backoff gap per failed attempt, none after the success"
        );
    }

    /// Stop on success — a doorway that registers first time must not sleep
    /// or re-register.
    #[tokio::test]
    async fn registration_that_succeeds_at_boot_never_retries() {
        use std::cell::RefCell;
        let slept: RefCell<Vec<u64>> = RefCell::new(Vec::new());
        let outcome = drive_registration_with_retry(
            "alpha-elohim-host",
            |_attempt| async { Ok(()) },
            |secs| {
                slept.borrow_mut().push(secs);
                async {}
            },
        )
        .await;
        assert_eq!(outcome, RegistrationOutcome::Registered { attempts: 1 });
        assert!(
            slept.borrow().is_empty(),
            "no backoff after a clean boot try"
        );
    }

    /// Bounded, never endless: a conductor that is simply never coming back
    /// must end in a loud, terminal verdict rather than an infinite loop.
    #[tokio::test]
    async fn registration_gives_up_after_the_bounded_budget() {
        use std::cell::RefCell;
        let slept: RefCell<Vec<u64>> = RefCell::new(Vec::new());
        let outcome = drive_registration_with_retry(
            "gamma-elohim-host",
            |_attempt| async { Err("conductor unreachable".to_string()) },
            |secs| {
                slept.borrow_mut().push(secs);
                async {}
            },
        )
        .await;
        match outcome {
            RegistrationOutcome::GaveUp {
                attempts,
                last_error,
            } => {
                assert_eq!(attempts, REGISTRATION_MAX_RETRIES + 1);
                assert_eq!(last_error, "conductor unreachable");
            }
            other => panic!("expected GaveUp, got {other:?}"),
        }
        assert_eq!(slept.borrow().len(), REGISTRATION_MAX_RETRIES as usize);
    }

    /// The schedule itself: monotonic, capped, and bounded.
    #[test]
    fn the_retry_schedule_is_monotonic_capped_and_bounded() {
        assert_eq!(
            registration_retry_delay_secs(0),
            None,
            "attempt 0 is the boot try"
        );
        let mut previous = 0u64;
        let mut total = 0u64;
        for attempt in 1..=REGISTRATION_MAX_RETRIES {
            let delay = registration_retry_delay_secs(attempt)
                .unwrap_or_else(|| panic!("attempt {attempt} is inside the budget"));
            assert!(
                delay >= previous,
                "gap {attempt} shrank: {previous} -> {delay}"
            );
            assert!(
                delay <= REGISTRATION_RETRY_MAX_SECS,
                "gap {attempt} exceeds the cap"
            );
            previous = delay;
            total += delay;
        }
        assert_eq!(
            registration_retry_delay_secs(REGISTRATION_MAX_RETRIES + 1),
            None,
            "the budget is bounded"
        );
        // The window this exists for: this conductor line can answer
        // `CellDisabled` for ~11 minutes after its interfaces come up.
        assert!(
            total >= 11 * 60,
            "the total patience ({total}s) must outlast the conductor's cell-startup window"
        );
    }

    #[test]
    fn test_federation_config_from_args_none() {
        let args = Args::parse_from(["doorway"]);
        let config = FederationConfig::from_args(&args);
        assert!(
            config.is_none(),
            "Should be None when doorway_id/url not set"
        );
    }

    #[test]
    fn test_federation_config_from_args_some() {
        let args = Args::parse_from([
            "doorway",
            "--doorway-id",
            "alpha-elohim-host",
            "--doorway-url",
            "https://alpha.elohim.host",
            "--doorway-urls",
            "https://alpha-backup.example,https://alpha.elohim.host",
            "--doorway-endpoint-ttl-secs",
            "120",
            "--bootstrap-url",
            "https://bootstrap.elohim.host",
            "--signal-url",
            "wss://signal.elohim.host",
            "--region",
            "us-west",
        ]);
        let config = FederationConfig::from_args(&args).unwrap();
        assert_eq!(config.doorway_id, "alpha-elohim-host");
        assert_eq!(config.doorway_url, "https://alpha.elohim.host");
        assert_eq!(config.region, Some("us-west".to_string()));
        assert_eq!(config.heartbeat_interval_secs, 60);
        assert_eq!(
            config.endpoints,
            vec![
                infrastructure_types::DoorwayEndpoint {
                    service: "gateway".to_string(),
                    url: "https://alpha.elohim.host".to_string(),
                    priority: 0,
                    ttl_secs: 120,
                },
                infrastructure_types::DoorwayEndpoint {
                    service: "gateway".to_string(),
                    url: "https://alpha-backup.example".to_string(),
                    priority: 1,
                    ttl_secs: 120,
                },
                infrastructure_types::DoorwayEndpoint {
                    service: "bootstrap".to_string(),
                    url: "https://bootstrap.elohim.host".to_string(),
                    priority: 0,
                    ttl_secs: 120,
                },
                infrastructure_types::DoorwayEndpoint {
                    service: "signal".to_string(),
                    url: "wss://signal.elohim.host".to_string(),
                    priority: 0,
                    ttl_secs: 120,
                },
            ]
        );
    }

    #[test]
    fn test_register_doorway_input_serialization() {
        let input = RegisterDoorwayInput {
            id: "test-doorway".to_string(),
            url: "https://test.elohim.host".to_string(),
            identity_root: None,
            endpoints: vec![infrastructure_types::DoorwayEndpoint {
                service: "gateway".to_string(),
                url: "https://test.elohim.host".to_string(),
                priority: 0,
                ttl_secs: 300,
            }],
            capabilities_json: r#"["gateway","bootstrap"]"#.to_string(),
            reach: "public".to_string(),
            region: Some("us-west".to_string()),
            bandwidth_mbps: None,
            version: "0.1.0".to_string(),
        };

        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: RegisterDoorwayInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "test-doorway");
    }

    // ── F-COHERENCE: tri-state fetch + edge-trigger + empty-clear ─────────────
    mod coherence_probe {
        use super::super::{
            fetch_peer_coherence, install_name_routes, merge_discovery_seeds, new_peer_cache,
            new_peer_coherence_cache, refresh_coherence, refresh_peer_cache,
            refresh_peer_cache_from_seeds, DoorwayRegistration, PeerDoorway,
        };
        use crate::projection::EprRouter;
        use crate::routes::coherence::{router_fingerprint, CoherenceManifest, EprHeadFingerprint};
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

        /// Build this edge's self-manifest from a head set (real digest path).
        fn self_manifest(heads: &[(&str, &str)]) -> CoherenceManifest {
            let router = EprRouter::new();
            router.replace_all(
                heads
                    .iter()
                    .map(|(p, e)| crate::routes::coherence::sample_projection(p, e))
                    .collect(),
            );
            router_fingerprint(&router, "alpha", Some("build-alpha"))
        }

        /// Serialize a CoherenceManifest body for a mock peer to return.
        fn manifest_body(id: &str, heads: &[(&str, &str)]) -> CoherenceManifest {
            let mut hv: Vec<EprHeadFingerprint> = heads
                .iter()
                .map(|(p, e)| EprHeadFingerprint {
                    url_path: (*p).to_string(),
                    epr_id: (*e).to_string(),
                    commitment_id: Some(format!("test-{e}")),
                    hostnames: Vec::new(),
                })
                .collect();
            let digest = crate::routes::coherence::mint_head_set_digest(&mut hv);
            CoherenceManifest {
                doorway_id: id.to_string(),
                generation: 1,
                heads: hv,
                digest,
                build_id: Some("build-peer".to_string()),
            }
        }

        async fn mount_coherence(server: &MockServer, body: &CoherenceManifest) {
            Mock::given(matchers::method("GET"))
                .and(matchers::path("/api/v1/federation/coherence"))
                .respond_with(ResponseTemplate::new(200).set_body_json(body))
                .mount(server)
                .await;
        }

        #[test]
        fn coherence_hostnames_install_exact_holder_contracts() {
            let table = crate::services::name_routing::NameRouteTable::new();
            let mut heads = vec![EprHeadFingerprint {
                url_path: "/".into(),
                epr_id: "candidate-epr".into(),
                commitment_id: Some("project-epr-candidate".into()),
                hostnames: vec!["Candidate.Example:443".into()],
            }];
            let manifest = CoherenceManifest {
                doorway_id: "alpha".into(),
                generation: 1,
                digest: crate::routes::coherence::mint_head_set_digest(&mut heads),
                heads,
                build_id: None,
            };
            install_name_routes(
                &table,
                &[(
                    "alpha".into(),
                    "https://alpha.example".into(),
                    true,
                    Some(manifest),
                )],
                0,
            );

            let exact = table.holders_for(
                &crate::services::name_routing::RouteKey::new(Some("candidate.example"), "/"),
                "local",
            );
            assert_eq!(exact.len(), 1);
            assert_eq!(exact[0].host.as_deref(), Some("candidate.example"));
            assert_eq!(
                exact[0].commitment_id.as_deref(),
                Some("project-epr-candidate")
            );
            assert_eq!(exact[0].epr_id.as_deref(), Some("candidate-epr"));
            assert!(table
                .holders_for(
                    &crate::services::name_routing::RouteKey::new(Some("other.example"), "/"),
                    "local",
                )
                .is_empty());
        }

        #[tokio::test]
        async fn fetch_404_is_unreachable() {
            let server = MockServer::start().await;
            // No mock mounted → 404 on the coherence path.
            let client = reqwest::Client::new();
            let (reachable, manifest) = fetch_peer_coherence(&client, &server.uri()).await;
            assert!(!reachable, "404 → not reachable (not-yet-deployed)");
            assert!(manifest.is_none());
        }

        #[tokio::test]
        async fn fetch_transport_error_is_unreachable() {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(200))
                .build()
                .unwrap();
            // Dead port → transport error.
            let (reachable, manifest) =
                fetch_peer_coherence(&client, "http://127.0.0.1:1/never").await;
            assert!(!reachable);
            assert!(manifest.is_none());
        }

        #[tokio::test]
        async fn fetch_200_garbage_body_is_reachable_no_manifest() {
            // Fix 2: a peer that answers 200 with an undeserializable body is a
            // reachable divergence (reachable=true, manifest=None), NOT "down".
            let server = MockServer::start().await;
            Mock::given(matchers::method("GET"))
                .and(matchers::path("/api/v1/federation/coherence"))
                .respond_with(ResponseTemplate::new(200).set_body_string("not json at all"))
                .mount(&server)
                .await;
            let client = reqwest::Client::new();
            let (reachable, manifest) = fetch_peer_coherence(&client, &server.uri()).await;
            assert!(reachable, "200 garbage body IS reachable (a divergence)");
            assert!(manifest.is_none(), "garbage body yields no manifest");
        }

        #[tokio::test]
        async fn fetch_200_5xx_is_unreachable() {
            // A 5xx (rolling-deploy state) must map to not-reachable so it never
            // manufactures deploy-noise WARNs.
            let server = MockServer::start().await;
            Mock::given(matchers::method("GET"))
                .and(matchers::path("/api/v1/federation/coherence"))
                .respond_with(ResponseTemplate::new(503))
                .mount(&server)
                .await;
            let client = reqwest::Client::new();
            let (reachable, _m) = fetch_peer_coherence(&client, &server.uri()).await;
            assert!(!reachable, "503 → not reachable (deploy state, no alarm)");
        }

        #[tokio::test]
        async fn fetch_200_valid_manifest_is_reachable_with_manifest() {
            let server = MockServer::start().await;
            mount_coherence(&server, &manifest_body("apex", &[("/lamad", "A")])).await;
            let client = reqwest::Client::new();
            let (reachable, manifest) = fetch_peer_coherence(&client, &server.uri()).await;
            assert!(reachable);
            let m = manifest.expect("valid manifest parses");
            assert_eq!(m.doorway_id, "apex");
        }

        #[tokio::test]
        async fn refresh_coherence_writes_verdicts_into_cache() {
            // The cache must become a real reader/writer: a divergent peer lands
            // a verdict with reachable=true, agrees=false.
            let server = MockServer::start().await;
            mount_coherence(&server, &manifest_body("apex", &[("/lamad", "DIFFERENT")])).await;
            let me = self_manifest(&[("/lamad", "MINE")]);
            let cache = new_peer_coherence_cache();
            let client = reqwest::Client::new();
            let peers = vec![("apex".to_string(), server.uri())];

            // The SAME probe round also installs the name-route table — one
            // fetch, two readers (WS3 Task 3.3).
            let name_routes = crate::services::name_routing::NameRouteTable::new();
            refresh_coherence(&me, &peers, &client, &cache, Some(&name_routes)).await;

            let stored = cache.read().await.clone();
            assert_eq!(stored.len(), 1);
            assert!(stored[0].reachable);
            assert!(!stored[0].agrees, "divergent peer → !agrees");
            // Fix 7: labeled from the manifest's self-report.
            assert_eq!(stored[0].doorway_id, "apex");

            // …and the peer's head set became a name-route contract, labelled
            // by the manifest's self-reported doorway_id and pointing at the
            // origin we probed.
            let holders = name_routes.holders_for(
                &crate::services::name_routing::RouteKey::path_only("/lamad/deep"),
                "me",
            );
            assert_eq!(holders.len(), 1, "the peer holds /lamad");
            assert_eq!(holders[0].doorway_id, "apex");
            assert_eq!(holders[0].origin, server.uri().trim_end_matches('/'));
            assert_eq!(
                holders[0].liveness,
                crate::services::name_routing::HolderLiveness::Serving
            );
            assert!(
                name_routes
                    .holders_for(
                        &crate::services::name_routing::RouteKey::path_only("/shefa"),
                        "me",
                    )
                    .is_empty(),
                "a root the peer does not mount yields no holder"
            );
        }

        #[tokio::test]
        async fn refresh_coherence_skips_empty_peer_url() {
            // Fix 6: an empty url must be skipped, not produce a dead verdict.
            let me = self_manifest(&[("/lamad", "MINE")]);
            let cache = new_peer_coherence_cache();
            let client = reqwest::Client::new();
            let peers = vec![("ghost".to_string(), "".to_string())];

            refresh_coherence(&me, &peers, &client, &cache, None).await;

            let stored = cache.read().await.clone();
            assert!(stored.is_empty(), "empty-url peer must not yield a verdict");
        }

        #[tokio::test]
        async fn refresh_peer_cache_clears_on_empty_urls() {
            // Fix 12: an empty peer-url list clears any stale cache entry.
            let cache = new_peer_cache();
            // Seed a stale peer directly.
            cache.write().await.push(PeerDoorway {
                id: "stale".into(),
                url: "https://stale.example".into(),
                region: None,
                capabilities: vec![],
                source_peer: "seed".into(),
                record_serial: None,
            });
            assert_eq!(cache.read().await.len(), 1);

            refresh_peer_cache(&[], Some("alpha"), &cache).await;
            assert!(
                cache.read().await.is_empty(),
                "empty peer-url list must clear the stale cache"
            );
        }

        // ── DHT-seeded discovery: the static list is a seed, never a gate ─────

        /// A `DoorwayRegistration` shaped like what `get_all_doorways` returns.
        fn registration(id: &str, url: &str, gateways: &[(&str, u16)]) -> DoorwayRegistration {
            DoorwayRegistration {
                id: id.to_string(),
                url: url.to_string(),
                identity_root: format!("{id}-identity-root"),
                signing_key: format!("{id}-key"),
                endpoints: gateways
                    .iter()
                    .map(|(u, priority)| infrastructure_types::DoorwayEndpoint {
                        service: "gateway".to_string(),
                        url: (*u).to_string(),
                        priority: *priority,
                        ttl_secs: 300,
                    })
                    .collect(),
                record_serial: 1,
                record_signature: vec![1; 64],
                operator_agent: format!("{id}-operator"),
                operator_human: None,
                capabilities_json: r#"["gateway"]"#.to_string(),
                reach: "public".to_string(),
                region: None,
                bandwidth_mbps: None,
                version: "test".to_string(),
                tier: "Emerging".to_string(),
                registered_at: "test".to_string(),
                updated_at: "test".to_string(),
            }
        }

        #[test]
        fn seeds_come_from_the_dht_when_no_static_peers_are_configured() {
            // The household shape: no FEDERATION_PEERS at all, one sibling
            // registered in the DHT. Before this fix the discovery task never
            // even spawned, so this seed set was empty forever.
            let registrations = vec![registration(
                "apex-elohim-host",
                "http://localhost:8889",
                &[("http://localhost:8889", 0)],
            )];
            let seeds = merge_discovery_seeds(
                &[],
                &registrations,
                Some("alpha-elohim-host"),
                &["http://localhost:8888".to_string()],
            );
            assert_eq!(seeds, vec!["http://localhost:8889".to_string()]);
        }

        #[test]
        fn static_and_dht_seeds_merge_and_dedupe_by_origin() {
            let registrations = vec![
                registration(
                    "apex",
                    "http://localhost:8889/",
                    &[("http://localhost:8889", 0)],
                ),
                registration("zeta", "https://zeta.example", &[]),
            ];
            let seeds = merge_discovery_seeds(
                &[
                    "https://static-one.example/".to_string(),
                    "http://localhost:8889".to_string(), // also in the DHT
                ],
                &registrations,
                Some("alpha"),
                &[],
            );
            assert_eq!(
                seeds,
                vec![
                    // static first, order preserved, trailing slash normalised
                    "https://static-one.example".to_string(),
                    "http://localhost:8889".to_string(),
                    // DHT registrations by id; the duplicate origin is dropped
                    "https://zeta.example".to_string(),
                ]
            );
        }

        #[test]
        fn self_is_never_seeded_by_id_or_by_origin() {
            let registrations = vec![
                registration(
                    "alpha",
                    "http://localhost:8888",
                    &[("http://localhost:8888", 0)],
                ),
                // A stale registration under another id still advertising OUR origin.
                registration("ghost", "http://localhost:8888", &[]),
                registration("apex", "http://localhost:8889", &[]),
            ];
            let seeds = merge_discovery_seeds(
                &[],
                &registrations,
                Some("alpha"),
                &["http://localhost:8888/".to_string()],
            );
            assert_eq!(
                seeds,
                vec!["http://localhost:8889".to_string()],
                "self excluded by id AND by origin"
            );
        }

        #[test]
        fn non_http_and_empty_seeds_are_dropped() {
            let registrations = vec![registration("bad", "wss://not-a-gateway.example", &[])];
            let seeds = merge_discovery_seeds(
                &["".to_string(), "   ".to_string(), "not a url".to_string()],
                &registrations,
                None,
                &[],
            );
            assert!(seeds.is_empty(), "only http(s) origins are dialable seeds");
        }

        #[tokio::test]
        async fn dht_registered_sibling_populates_the_name_route_table() {
            // END-TO-END for the live finding: NO static peers, ONE
            // DHT-registered sibling. The tick seeds from the registration,
            // discovers the peer over HTTP, coherence-probes it, and the
            // name-route table fills from that peer's head set — which is what
            // the one-hop relay folds over.
            let sibling = MockServer::start().await;

            // The sibling's federation surface (what refresh_peer_cache reads).
            Mock::given(matchers::method("GET"))
                .and(matchers::path("/api/v1/federation/doorways"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "doorways": [{
                        "id": "apex-elohim-host",
                        "url": sibling.uri(),
                        "region": null,
                        "capabilities": ["gateway"],
                        "status": "online",
                    }]
                })))
                .mount(&sibling)
                .await;
            // …and its coherence manifest (what the name-route table folds).
            mount_coherence(
                &sibling,
                &manifest_body("apex-elohim-host", &[("/nrt-garden", "EPR-GARDEN")]),
            )
            .await;

            let registrations = vec![registration(
                "apex-elohim-host",
                &sibling.uri(),
                &[(&sibling.uri(), 0)],
            )];

            let cache = new_peer_cache();
            let seeds = refresh_peer_cache_from_seeds(
                &[], // NO static FEDERATION_PEERS — the old gate
                &registrations,
                Some("alpha-elohim-host"),
                &["http://localhost:8888".to_string()],
                &cache,
            )
            .await;
            assert_eq!(seeds.len(), 1, "the DHT registration alone seeded the tick");

            let peers: Vec<(String, String)> = cache
                .read()
                .await
                .iter()
                .map(|p| (p.id.clone(), p.url.clone()))
                .collect();
            assert_eq!(peers.len(), 1, "the sibling was discovered");

            let name_routes = crate::services::name_routing::NameRouteTable::new();
            let me = self_manifest(&[("/lamad", "MINE")]);
            let client = reqwest::Client::new();
            refresh_coherence(
                &me,
                &peers,
                &client,
                &new_peer_coherence_cache(),
                Some(&name_routes),
            )
            .await;

            let holders = name_routes.holders_for(
                &crate::services::name_routing::RouteKey::path_only("/nrt-garden/"),
                "alpha-elohim-host",
            );
            assert_eq!(holders.len(), 1, "the sibling holds /nrt-garden");
            assert_eq!(holders[0].doorway_id, "apex-elohim-host");
            assert_eq!(holders[0].origin, sibling.uri().trim_end_matches('/'));
            assert_eq!(
                holders[0].liveness,
                crate::services::name_routing::HolderLiveness::Serving
            );
        }

        /// OwnerOrder — `selector_rank`'s documented "stable final tiebreak" —
        /// must not depend on the order the coherence probe happened to return.
        ///
        /// THE DEFECT THIS PINS (household mesh, 2026-09-21 run R1): the probe
        /// order is `refresh_peer_cache`'s cache, which is the union of what each
        /// seed's own `GET /api/v1/federation/doorways` returned — so the first
        /// reachable seed's DHT link order became every doorway's holder
        /// priority. With "garden" held by alpha and gamma, beta put the
        /// most-recently-registered doorway (gamma) first and relayed EVERY
        /// request to it, so the holder that had declared itself busy (alpha) was
        /// never dialled: zero `name_route_shed` lines, both demotion counters 0,
        /// and the balance scenario's "set aside" claims passing for the wrong
        /// reason. `merge_discovery_seeds` already sorts registrations "by id for
        /// determinism"; this restores that rule where the per-seed fan-out
        /// discarded it.
        #[test]
        fn owner_order_is_stable_by_doorway_id_whatever_order_the_probe_returned() {
            use crate::services::name_routing::{NameRouteTable, RouteKey};

            let probed = vec![
                (
                    "gamma-elohim-host".to_string(),
                    "http://localhost:8890".to_string(),
                    true,
                    Some(manifest_body(
                        "gamma-elohim-host",
                        &[("/nrt-garden", "EPR-GARDEN")],
                    )),
                ),
                (
                    "alpha-elohim-host".to_string(),
                    "http://localhost:8888".to_string(),
                    true,
                    Some(manifest_body(
                        "alpha-elohim-host",
                        &[("/nrt-garden", "EPR-GARDEN")],
                    )),
                ),
            ];
            let expected = vec!["alpha-elohim-host", "gamma-elohim-host"];

            // gamma first in the probe — the live household order.
            let forward = NameRouteTable::new();
            install_name_routes(&forward, &probed, 0);
            let forward_order: Vec<String> = forward
                .holders_for(&RouteKey::path_only("/nrt-garden/"), "apex-elohim-host")
                .into_iter()
                .map(|holder| holder.doorway_id)
                .collect();
            assert_eq!(
                forward_order, expected,
                "owner order must be the stable id order, not the probe's arrival order"
            );

            // alpha first in the probe — the SAME fold must come out.
            let reversed: Vec<_> = probed.into_iter().rev().collect();
            let backward = NameRouteTable::new();
            install_name_routes(&backward, &reversed, 0);
            let backward_order: Vec<String> = backward
                .holders_for(&RouteKey::path_only("/nrt-garden/"), "apex-elohim-host")
                .into_iter()
                .map(|holder| holder.doorway_id)
                .collect();
            assert_eq!(
                backward_order, forward_order,
                "the fold changed when only the probe's arrival order changed — OwnerOrder is \
                 not a stable tiebreak"
            );
        }
    }

    // ── Peer JWKS cache: fetch, refresh, TTL/cooldown (Task 2.1) ──────────
    mod peer_jwks {
        use super::super::{
            ensure_peer_key_cached, fetch_peer_jwks, jwks_trust_anchors, new_peer_jwks_cache,
            refresh_peer_jwks_cache, DoorwayRegistration, PeerDoorway,
        };
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

        fn jwks_body(kid: &str, pubkey: &[u8; 32]) -> serde_json::Value {
            serde_json::json!({
                "keys": [{
                    "kty": "OKP",
                    "crv": "Ed25519",
                    "use": "sig",
                    "kid": kid,
                    "x": URL_SAFE_NO_PAD.encode(pubkey),
                }]
            })
        }

        async fn mount_jwks(server: &MockServer, kid: &str, pubkey: &[u8; 32]) {
            Mock::given(matchers::method("GET"))
                .and(matchers::path("/.well-known/doorway-keys"))
                .respond_with(ResponseTemplate::new(200).set_body_json(jwks_body(kid, pubkey)))
                .mount(server)
                .await;
        }

        #[tokio::test]
        async fn fetch_peer_jwks_parses_okp_ed25519_entries() {
            let server = MockServer::start().await;
            let pubkey = [7u8; 32];
            mount_jwks(&server, "sibling", &pubkey).await;

            let client = reqwest::Client::new();
            let keys = fetch_peer_jwks(&client, &server.uri()).await;

            assert_eq!(keys.len(), 1);
            assert_eq!(keys[0].0, "sibling");
            assert_eq!(keys[0].1, pubkey);
        }

        #[tokio::test]
        async fn fetch_peer_jwks_404_is_empty_not_an_error() {
            let server = MockServer::start().await;
            // No mock mounted → 404, mirroring an undeployed/older peer.
            let client = reqwest::Client::new();
            let keys = fetch_peer_jwks(&client, &server.uri()).await;
            assert!(keys.is_empty());
        }

        #[tokio::test]
        async fn refresh_peer_jwks_cache_populates_from_known_peers() {
            let server = MockServer::start().await;
            let pubkey = [9u8; 32];
            mount_jwks(&server, "sibling", &pubkey).await;

            let cache = new_peer_jwks_cache();
            let client = reqwest::Client::new();
            let peers = vec![PeerDoorway {
                id: "sibling".into(),
                url: server.uri(),
                region: None,
                capabilities: vec![],
                source_peer: "seed".into(),
                record_serial: None,
            }];

            refresh_peer_jwks_cache(&peers, &client, &cache).await;

            assert_eq!(cache.get("sibling"), Some(pubkey));
            assert_eq!(cache.get("unknown-doorway"), None);
        }

        #[tokio::test]
        async fn ensure_peer_key_cached_fetches_once_on_miss() {
            let server = MockServer::start().await;
            let pubkey = [3u8; 32];
            mount_jwks(&server, "sibling", &pubkey).await;

            let cache = new_peer_jwks_cache();
            let client = reqwest::Client::new();

            let found = ensure_peer_key_cached(&cache, "sibling", &server.uri(), &client).await;
            assert_eq!(found, Some(pubkey));
            // Now served from cache without another fetch attempt needed.
            assert_eq!(cache.get("sibling"), Some(pubkey));
        }

        #[tokio::test]
        async fn ensure_peer_key_cached_negative_cooldown_guards_against_a_fetch_storm() {
            let server = MockServer::start().await;
            // Peer never publishes this kid → every fetch 404s. Expect
            // EXACTLY ONE outbound request across two on-demand calls.
            Mock::given(matchers::method("GET"))
                .and(matchers::path("/.well-known/doorway-keys"))
                .respond_with(ResponseTemplate::new(404))
                .expect(1)
                .mount(&server)
                .await;

            let cache = new_peer_jwks_cache();
            let client = reqwest::Client::new();

            let first = ensure_peer_key_cached(&cache, "ghost", &server.uri(), &client).await;
            assert_eq!(first, None);

            // Immediately retrying must NOT issue a second network call —
            // the negative cooldown guards it. wiremock's `.expect(1)`
            // panics on drop if a second request landed.
            let second = ensure_peer_key_cached(&cache, "ghost", &server.uri(), &client).await;
            assert_eq!(second, None);
        }

        #[test]
        fn insert_positive_refuses_conflicting_pubkey_for_same_kid() {
            let cache = new_peer_jwks_cache();
            let incumbent = [1u8; 32];
            let challenger = [2u8; 32];

            assert!(cache.insert_positive("sibling".to_string(), incumbent));
            assert!(
                !cache.insert_positive("sibling".to_string(), challenger),
                "a live kid must never be re-anchored to a different pubkey"
            );
            assert_eq!(
                cache.get("sibling"),
                Some(incumbent),
                "the incumbent anchor survives the takeover attempt"
            );
        }

        #[test]
        fn insert_positive_accepts_identical_pubkey_as_refresh() {
            let cache = new_peer_jwks_cache();
            let pubkey = [5u8; 32];

            assert!(cache.insert_positive("sibling".to_string(), pubkey));
            assert!(
                cache.insert_positive("sibling".to_string(), pubkey),
                "re-asserting the SAME key is a legitimate refresh, not a takeover"
            );
            assert_eq!(cache.get("sibling"), Some(pubkey));
        }

        fn registration(id: &str, url: &str, signing_key: &str) -> DoorwayRegistration {
            DoorwayRegistration {
                id: id.to_string(),
                url: url.to_string(),
                identity_root: format!("{id}-identity-root"),
                signing_key: signing_key.to_string(),
                endpoints: vec![],
                record_serial: 1,
                record_signature: vec![1; 64],
                operator_agent: format!("{id}-operator"),
                operator_human: None,
                capabilities_json: r#"["gateway"]"#.to_string(),
                reach: "public".to_string(),
                region: None,
                bandwidth_mbps: None,
                version: "test".to_string(),
                tier: "Emerging".to_string(),
                registered_at: "test".to_string(),
                updated_at: "test".to_string(),
            }
        }

        #[tokio::test]
        async fn jwks_refresh_never_reaches_a_doorway_without_a_signing_key() {
            let anchored = MockServer::start().await;
            let pubkey = [11u8; 32];
            mount_jwks(&anchored, "anchored", &pubkey).await;

            // A registration with no signing key must never be contacted —
            // if it were, this body would claim the sibling's `kid`.
            let keyless = MockServer::start().await;
            Mock::given(matchers::method("GET"))
                .and(matchers::path("/.well-known/doorway-keys"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(jwks_body("anchored", &[99u8; 32])),
                )
                .expect(0)
                .mount(&keyless)
                .await;

            let registrations = vec![
                registration("anchored", &anchored.uri(), "anchored-signing-key"),
                registration("keyless", &keyless.uri(), ""),
            ];

            let anchors = jwks_trust_anchors(&registrations);
            assert_eq!(anchors.len(), 1);
            assert_eq!(anchors[0].id, "anchored");

            let cache = new_peer_jwks_cache();
            refresh_peer_jwks_cache(&anchors, &reqwest::Client::new(), &cache).await;

            assert_eq!(cache.get("anchored"), Some(pubkey));
            // wiremock's `.expect(0)` panics on drop if the keyless doorway
            // was contacted at all.
        }
    }

    // ── Probe roster expiry (conductor-store growth report §3 / §7.1 (a)) ────
    //
    // The heartbeat wrote one immutable attestation per registered doorway
    // per round, forever, including every a2o scenario doorway that lived four
    // minutes. These pin that only the live roster is probed.
    mod probe_roster {
        use super::*;

        fn peer(id: &str, serial: Option<u64>) -> PeerDoorway {
            PeerDoorway {
                id: id.into(),
                url: format!("http://{id}.invalid"),
                region: None,
                capabilities: vec![],
                source_peer: "seed".into(),
                record_serial: serial,
            }
        }

        fn ids(peers: &[PeerDoorway]) -> Vec<&str> {
            peers.iter().map(|p| p.id.as_str()).collect()
        }

        #[test]
        fn a_short_lived_doorway_leaves_the_roster_after_the_expiry_rounds() {
            let mut roster = ProbeRoster::new();
            let members = vec![peer("sibling", Some(1)), peer("scenario", Some(7))];

            // Round 1: both are new, both probed; both answer.
            let (probe, skipped) = roster.begin_round(&members);
            assert_eq!(ids(&probe), vec!["sibling", "scenario"]);
            assert!(skipped.is_empty());
            roster.observe("sibling", true);
            roster.observe("scenario", true);

            // The scenario doorway is torn down: it stops answering.
            for round in 2..=PROBE_ROSTER_EXPIRY_ROUNDS {
                let (probe, skipped) = roster.begin_round(&members);
                assert_eq!(ids(&probe), vec!["sibling", "scenario"], "round {round}");
                assert!(skipped.is_empty(), "round {round}");
                roster.observe("sibling", true);
                roster.observe("scenario", false);
            }

            // Expiry: no liveness evidence for PROBE_ROSTER_EXPIRY_ROUNDS rounds.
            for _ in 0..5 {
                let (probe, skipped) = roster.begin_round(&members);
                assert_eq!(ids(&probe), vec!["sibling"]);
                assert_eq!(skipped, vec!["scenario".to_string()]);
                roster.observe("sibling", true);
            }
        }

        #[test]
        fn a_degraded_answer_is_liveness_only_unreachable_is_not() {
            let mut roster = ProbeRoster::new();
            let members = vec![peer("slow", None)];
            for _ in 0..(PROBE_ROSTER_EXPIRY_ROUNDS * 3) {
                let (probe, skipped) = roster.begin_round(&members);
                assert_eq!(ids(&probe), vec!["slow"]);
                assert!(skipped.is_empty());
                // `degraded` (HTTP answered, conductor down) is still an answer.
                roster.observe("slow", true);
            }
        }

        #[test]
        fn a_doorway_that_was_never_alive_is_probed_only_for_the_expiry_rounds() {
            // A freshly booted attestor finds 170-odd dead registrations: each
            // gets PROBE_ROSTER_EXPIRY_ROUNDS probes, then silence until the
            // first grace round.
            let mut roster = ProbeRoster::new();
            let members = vec![peer("dead", Some(3))];
            let mut probed = 0;
            for _ in 0..(PROBE_ROSTER_GRACE_ROUNDS - 1) {
                let (probe, _) = roster.begin_round(&members);
                probed += probe.len() as u64;
                roster.observe("dead", false);
            }
            assert_eq!(probed, PROBE_ROSTER_EXPIRY_ROUNDS);
        }

        /// Run rounds until the next grace round is the one about to begin.
        fn advance_to_just_before_grace(roster: &mut ProbeRoster, members: &[PeerDoorway]) {
            while !(roster.round + 1).is_multiple_of(PROBE_ROSTER_GRACE_ROUNDS) {
                let (probe, _) = roster.begin_round(members);
                for peer in &probe {
                    roster.observe(&peer.id, false);
                }
            }
        }

        #[test]
        fn an_expired_peer_that_answers_its_grace_probe_is_live_again() {
            // A sibling that stalled without restarting (same record_serial).
            let mut roster = ProbeRoster::new();
            let members = vec![peer("stalled", Some(5))];
            // It expires, and is still stalled at the first grace round.
            advance_to_just_before_grace(&mut roster, &members);
            let (probe, _) = roster.begin_round(&members);
            assert_eq!(ids(&probe), vec!["stalled"]);
            roster.observe("stalled", false);
            let (probe, skipped) = roster.begin_round(&members);
            assert!(probe.is_empty());
            assert_eq!(skipped, vec!["stalled".to_string()]);
            advance_to_just_before_grace(&mut roster, &members);

            // Next grace round: it is re-probed, and this time it answers (healed).
            let (probe, skipped) = roster.begin_round(&members);
            assert_eq!(ids(&probe), vec!["stalled"]);
            assert!(skipped.is_empty(), "a grace-probed peer is not a skip");
            roster.observe("stalled", true);

            // Live again from the very next round, every round.
            for _ in 0..(PROBE_ROSTER_EXPIRY_ROUNDS * 2) {
                let (probe, skipped) = roster.begin_round(&members);
                assert_eq!(ids(&probe), vec!["stalled"]);
                assert!(skipped.is_empty());
                roster.observe("stalled", true);
            }
        }

        #[test]
        fn a_silent_peer_stays_expired_between_grace_rounds() {
            let mut roster = ProbeRoster::new();
            let members = vec![peer("silent", Some(9))];
            advance_to_just_before_grace(&mut roster, &members);
            for grace_round in 0..3 {
                let (probe, _) = roster.begin_round(&members);
                assert_eq!(ids(&probe), vec!["silent"], "grace round {grace_round}");
                roster.observe("silent", false);
                // Every round until the next grace round: skipped and counted.
                for _ in 1..PROBE_ROSTER_GRACE_ROUNDS {
                    let (probe, skipped) = roster.begin_round(&members);
                    assert!(probe.is_empty(), "grace round {grace_round}");
                    assert_eq!(skipped, vec!["silent".to_string()]);
                    roster.observe("silent", false);
                }
            }
        }

        #[test]
        fn grace_rotates_through_the_expired_set_oldest_first() {
            // More expired members than one batch: every one is revisited, and
            // the batch never exceeds PROBE_ROSTER_GRACE_BATCH.
            let n = PROBE_ROSTER_GRACE_BATCH * 3 + 1;
            let members: Vec<PeerDoorway> = (0..n)
                .map(|i| peer(&format!("dead-{i:03}"), Some(1)))
                .collect();
            let mut roster = ProbeRoster::new();
            let mut grace_probed = std::collections::HashSet::new();
            let grace_rounds_needed = n.div_ceil(PROBE_ROSTER_GRACE_BATCH);
            let mut grace_rounds_seen = 0;
            while grace_rounds_seen < grace_rounds_needed {
                let (probe, _) = roster.begin_round(&members);
                if roster.round.is_multiple_of(PROBE_ROSTER_GRACE_ROUNDS) {
                    grace_rounds_seen += 1;
                    assert_eq!(probe.len(), PROBE_ROSTER_GRACE_BATCH, "a full batch");
                    let untried_before = n - grace_probed.len();
                    let fresh = probe
                        .iter()
                        .filter(|peer| grace_probed.insert(peer.id.clone()))
                        .count();
                    // Untried peers always go before any repeat.
                    assert_eq!(fresh, untried_before.min(PROBE_ROSTER_GRACE_BATCH));
                }
                for peer in &probe {
                    roster.observe(&peer.id, false);
                }
            }
            assert_eq!(grace_probed.len(), n, "every expired peer got a grace turn");
        }

        /// The point of H1: a fully dead roster of N costs one attestor at most
        /// ceil(N / PROBE_ROSTER_GRACE_ROUNDS) probe writes per round on
        /// average once it has expired — never N per round.
        #[test]
        fn a_fully_dead_roster_costs_a_bounded_write_rate_not_n_per_round() {
            for n in [1usize, 3, 16, 50, 179, 400] {
                let members: Vec<PeerDoorway> = (0..n)
                    .map(|i| peer(&format!("gone-{i}"), Some(1)))
                    .collect();
                let mut roster = ProbeRoster::new();
                // Let every member expire (they never answer).
                for _ in 0..PROBE_ROSTER_EXPIRY_ROUNDS {
                    let (probe, _) = roster.begin_round(&members);
                    for peer in &probe {
                        roster.observe(&peer.id, false);
                    }
                }
                let window = PROBE_ROSTER_GRACE_ROUNDS * 10;
                let mut writes = 0u64;
                for _ in 0..window {
                    let (probe, skipped) = roster.begin_round(&members);
                    assert!(probe.len() <= PROBE_ROSTER_GRACE_BATCH, "n={n}");
                    assert_eq!(
                        probe.len() + skipped.len(),
                        n,
                        "n={n}: every member accounted"
                    );
                    writes += probe.len() as u64;
                    for peer in &probe {
                        roster.observe(&peer.id, false);
                    }
                }
                let bound_per_round = (n as u64).div_ceil(PROBE_ROSTER_GRACE_ROUNDS);
                assert!(
                    writes <= bound_per_round * window,
                    "n={n}: {writes} writes in {window} rounds exceeds ceil(N/G)={bound_per_round}/round"
                );
                assert!(
                    writes
                        <= (PROBE_ROSTER_GRACE_BATCH as u64) * (window / PROBE_ROSTER_GRACE_ROUNDS),
                    "n={n}: more than one batch per grace period"
                );
                if n > 1 {
                    assert!(
                        writes < (n as u64) * window,
                        "n={n}: N per round is the pre-H1 cost"
                    );
                }
            }
        }

        #[test]
        fn re_registration_readmits_an_expired_doorway() {
            let mut roster = ProbeRoster::new();
            let dead = vec![peer("restarting", Some(10))];
            for _ in 0..(PROBE_ROSTER_EXPIRY_ROUNDS + 2) {
                roster.begin_round(&dead);
                roster.observe("restarting", false);
            }
            let (probe, skipped) = roster.begin_round(&dead);
            assert!(probe.is_empty());
            assert_eq!(skipped, vec!["restarting".to_string()]);

            // It boots again: register/update_doorway bumps record_serial.
            let back = vec![peer("restarting", Some(11))];
            let (probe, skipped) = roster.begin_round(&back);
            assert_eq!(ids(&probe), vec!["restarting"]);
            assert!(skipped.is_empty());

            // A peer-cache echo with no serial is not evidence either way.
            roster.observe("restarting", false);
            let echo = vec![peer("restarting", None)];
            let (probe, _) = roster.begin_round(&echo);
            assert_eq!(ids(&probe), vec!["restarting"]);
        }

        #[test]
        fn ids_that_leave_the_roster_are_forgotten() {
            let mut roster = ProbeRoster::new();
            for _ in 0..(PROBE_ROSTER_EXPIRY_ROUNDS + 1) {
                roster.begin_round(&[peer("gone", Some(1))]);
                roster.observe("gone", false);
            }
            roster.begin_round(&[]);
            assert!(
                roster.peers.is_empty(),
                "the book never outgrows the roster"
            );
            // Seen again later (e.g. the registry answered again): a new member.
            let (probe, _) = roster.begin_round(&[peer("gone", Some(1))]);
            assert_eq!(ids(&probe), vec!["gone"]);
        }

        #[test]
        fn expiry_lands_within_fifteen_minutes_at_the_configured_cadence() {
            // Probe round = 5 heartbeat ticks (spawn_heartbeat_task).
            let round_secs = 5 * 60; // FederationConfig::from_args heartbeat
            assert!(PROBE_ROSTER_EXPIRY_ROUNDS * round_secs <= 15 * 60);
            assert!(
                PROBE_ROSTER_EXPIRY_ROUNDS >= 2,
                "one missed probe is not death"
            );
        }

        #[test]
        fn a_skip_is_counted_in_the_doorbell_family() {
            use crate::services::federation_doorbell::{OUTCOME_SKIPPED_EXPIRED, SIDE_PROBE};
            let series = crate::metrics::DOORWAY_FEDERATION_DOORBELL_TOTAL
                .with_label_values(&[SIDE_PROBE, OUTCOME_SKIPPED_EXPIRED]);
            let before = series.get();
            crate::metrics::record_doorbell(SIDE_PROBE, OUTCOME_SKIPPED_EXPIRED);
            assert!(series.get() > before);
        }
    }
}
