//! PRESENCE RECOVERY — when this node observes a conductor's cells present
//! again, every observed role of that conductor is probed in ONE pass.
//!
//! The bridge supervisor ([`crate::hc_client_registry`]) owns the slots, the
//! re-mint and the steady [`BRIDGE_PROBE_INTERVAL`](crate::hc_client_registry::BRIDGE_PROBE_INTERVAL) tick. This module owns the
//! one thing that tick could not do on its own: act in the wake of the
//! membership reading that finds the cells back, for every role at once, and
//! take those readings while a re-mint's cells are coming back. The supervisor
//! touches it in exactly two places — it spawns [`drive_presence_recovery`]
//! beside its per-role loops, and a re-mint calls
//! [`PresenceRecovery::reopen_at`]. The promise, its bounds and what the seam
//! deliberately does not do are on [`PresenceRecovery`].

use std::sync::Arc;
use std::time::Duration;

use tracing::{debug, info};

use crate::hc_client_registry::{
    HcClientRegistry, CROSS_CELL_DRIVER_ROLE, CROSS_CELL_ROLES, OBSERVED_ROLES,
};

/// How often the presence watch re-reads `ListCellIds` while it is open.
///
/// This IS the "immediately" of presence recovery after a re-mint: a cell that
/// enters the conductor's running map inside an open watch is seen within one
/// cadence, and every observed role of that conductor is probed in the same
/// wake — so `/health/serving` turns within this bound plus one `ListCellIds`
/// round trip (itself capped by
/// [`crate::services::cell_membership::MEMBERSHIP_READ_DEADLINE`]) plus the
/// probes' own round trips. Short because the read is a lock-and-clone of an
/// in-memory map (see `cell_membership`'s module doc); not shorter, because
/// nothing a person waits on is faster than a couple of seconds and every read
/// is an admin round trip the conductor serves while it is still starting.
pub(crate) const PRESENCE_WATCH_CADENCE: Duration = Duration::from_secs(2);

/// How long after a bridge re-mint the presence watch stays open.
///
/// A re-mint is the one moment this node KNOWS cells are about to come back:
/// the conductor answers app sockets before its cells are in the running map,
/// so the read that follows a re-mint routinely proves them absent (measured on
/// the household 2026-10-18: absent at re-mint, running seconds later, seen only
/// on the next 20s tick — once more for `mishpat`). Six probe intervals covers
/// that startup with room to spare, and caps the watch at
/// `PRESENCE_WATCH_WINDOW / PRESENCE_WATCH_CADENCE` = 60 reads per re-mint,
/// taken only while some observed role is in a not-running episode. A conductor
/// still starting after the window (the fleet has measured hours) falls back to
/// the steady cadence: the next ordinary membership read wakes the same pass.
pub(crate) const PRESENCE_WATCH_WINDOW: Duration = Duration::from_secs(120);

/// How long shutdown waits for the presence driver to DRAIN the probes it
/// offered before the runtime is torn down.
///
/// The driver never drops a probe it offered — a conductor call cannot be
/// cancelled, only abandoned — so at shutdown it finishes them, and `main`
/// awaits that drain. A drain holds at most one probe per observed role, each
/// one read-only extern with no DHT access, issued to a conductor that is
/// still running while storage stops; the same order as
/// [`crate::services::cell_membership::MEMBERSHIP_READ_DEADLINE`], the
/// longest this crate waits on any single admin answer. Past it, the drain is
/// abandoned and said so: `call_zome` has no deadline of its own, and a
/// conductor that never answers must not hold the process open.
pub const PRESENCE_SHUTDOWN_DRAIN_BOUND: Duration = Duration::from_secs(10);

/// One observed role, as the presence pass sees it: which client TENDS it and
/// the `CellId` that client carries for it.
///
/// `tended_by` is the role itself for a role with a slot of its own, and
/// [`CROSS_CELL_DRIVER_ROLE`] for a [`CROSS_CELL_ROLES`] entry — the same
/// client the supervisor already tends it through, so a probe of `mishpat`
/// crosses the socket a `mishpat` zome call would.
#[derive(Debug, Clone)]
pub(crate) struct ObservedCell {
    pub role: &'static str,
    pub tended_by: &'static str,
    /// `None` while the tending role's slot is empty (mid re-mint) or its client
    /// resolved no cell for this role. Not presence, so never probed.
    pub cell_id: Option<holochain_client::CellId>,
}

/// The client that tends `role`: its own, or the cross-cell driver's.
pub(crate) fn tender_of(role: &'static str) -> &'static str {
    if CROSS_CELL_ROLES.contains(&role) {
        CROSS_CELL_DRIVER_ROLE
    } else {
        role
    }
}

/// What the presence seam needs from a conductor: the roles to look at, a
/// membership source to read through, and the probe.
///
/// A trait for the reason [`crate::hc_client_registry::NotRunningActuator`] is one: the pass and its
/// driver are production code driven by the tests against a fake conductor,
/// so deleting a wire in them turns a test red rather than leaving it green.
/// There is deliberately no `enable_app` here — the presence seam never
/// mutates the conductor and never spends a ladder rung.
#[async_trait::async_trait]
pub(crate) trait PresenceConductor: Send + Sync {
    /// Every [`OBSERVED_ROLES`] entry on this conductor.
    fn observed_cells(&self) -> Vec<ObservedCell>;
    /// An admin handle to read `ListCellIds` through; `None` when no role holds
    /// a client (boot ramp, or every slot mid re-mint).
    fn membership_source(
        &self,
    ) -> Option<Arc<dyn crate::services::cell_membership::RunningCellSource>>;
    /// Ask `role`'s cell its one read-only question through `tended_by`'s
    /// client — [`crate::services::cell_probe::probe_cell`] in production, so a
    /// probe that returns lands in `record_role_success` like any zome call.
    /// The verdict says whether a probe was ASKED at all: only an admitted
    /// probe spends the role's presence run.
    async fn probe_cell(
        &self,
        role: &'static str,
        tended_by: &'static str,
    ) -> crate::services::cell_probe::ProbeVerdict;
    /// The gate [`Self::probe_cell`] admits through — whose release wakes an
    /// opportunity that found the role's slot taken.
    fn probe_gate(&self) -> &crate::services::cell_probe::ProbeGate;
}

#[async_trait::async_trait]
impl PresenceConductor for HcClientRegistry {
    fn observed_cells(&self) -> Vec<ObservedCell> {
        OBSERVED_ROLES
            .iter()
            .map(|&role| {
                let tended_by = tender_of(role);
                ObservedCell {
                    role,
                    tended_by,
                    cell_id: self
                        .client(tended_by)
                        .and_then(|hc| hc.cell_id_for_role(role).cloned()),
                }
            })
            .collect()
    }

    fn membership_source(
        &self,
    ) -> Option<Arc<dyn crate::services::cell_membership::RunningCellSource>> {
        self.any_admin_websocket().map(|admin| {
            Arc::new(admin) as Arc<dyn crate::services::cell_membership::RunningCellSource>
        })
    }

    async fn probe_cell(
        &self,
        role: &'static str,
        tended_by: &'static str,
    ) -> crate::services::cell_probe::ProbeVerdict {
        match self.client(tended_by) {
            Some(hc) => HcClientRegistry::probe_the_cell(role, &hc).await,
            None => crate::services::cell_probe::ProbeVerdict::Unreachable,
        }
    }

    fn probe_gate(&self) -> &crate::services::cell_probe::ProbeGate {
        crate::services::cell_probe::probe_gate()
    }
}

/// What one pass did. Tests only: production reads the offered probes off
/// [`PresenceRecovery::plan_at`] directly.
#[cfg(test)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PresencePass {
    /// The roles probed in this pass, in [`PresenceConductor::observed_cells`]
    /// order.
    pub probed: Vec<&'static str>,
}

/// A probe the driver offered, landing: which role, for which presence run,
/// under which re-mint epoch, and what the gate and the conductor said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Landed {
    pub role: &'static str,
    pub run: u64,
    pub epoch: u64,
    pub verdict: crate::services::cell_probe::ProbeVerdict,
}

/// One probe a pass offers: the role to ask, and the run it is asked for.
#[derive(Debug, Clone)]
pub(crate) struct Offer {
    pub cell: ObservedCell,
    pub run: u64,
}

/// The seam's bookkeeping, under ONE lock so no reader sees half a transition.
#[derive(Debug, Default)]
struct SeamState {
    /// When the post-re-mint watch closes; `None` = never opened.
    watch_until: Option<std::time::Instant>,
    /// Bumped by every re-mint, and carried by every offered probe, so a probe
    /// offered before a re-mint is recognised when it lands after one.
    epoch: u64,
    /// The presence run ([`crate::services::cell_membership::MembershipCache::presence_run_at`])
    /// each role was last offered a probe for. A role is owed once per RUN, so
    /// a run that began after this one — however the readings in between were
    /// coalesced — is owed again.
    probed: std::collections::BTreeMap<&'static str, u64>,
    /// Roles with a probe the driver offered and has not seen land.
    in_flight: std::collections::BTreeSet<&'static str>,
    /// Roles whose offered probe found the gate already holding a probe of
    /// theirs (the supervisor's): the opportunity is KEPT, and re-offered when
    /// that probe releases the gate.
    pending: std::collections::BTreeSet<&'static str>,
}

/// THE PRESENCE SEAM: when this node observes a conductor's cells present
/// again, it probes EVERY observed role of that conductor in one pass.
///
/// ## The promise
///
/// A role in a not-running episode whose cell a membership reading shows in
/// the conductor's running map is probed **in the wake of that reading** — not
/// on its own supervisor's next tick, and not on its driver's tick a second
/// time round for a cross-cell role. Every such role of the conductor,
/// `mishpat` included, is offered its probe in the SAME pass, and the result
/// lands through the existing paths: a probe that returns reaches
/// [`crate::conductor_bridge_health::record_role_success`], one that fails
/// lands in the responsive class and ends nothing.
///
/// "Immediately" is bounded by WHEN a reading happens, so the seam also makes
/// readings happen when they matter:
///
/// * **After a bridge re-mint** — the moment the cells are about to come back
///   — a watch opens for [`PRESENCE_WATCH_WINDOW`] and re-reads at most every
///   [`PRESENCE_WATCH_CADENCE`] while any observed role is not running. A cell
///   that rejoins inside the window is probed within one cadence of rejoining.
/// * **Otherwise** the readings are the ones the node already takes (each
///   supervisor tick, the diagnostics handler), and the pass rides every one of
///   them through [`crate::services::cell_membership::MembershipCache::read_landed`].
///
/// ## What it does not do
///
/// It loosens no predicate: presence is never recovery, only the probe's
/// returned call is. It never calls `enable_app`, never consumes or retracts a
/// ladder rung and never wakes a supervisor, so the [`BRIDGE_PROBE_INTERVAL`](crate::hc_client_registry::BRIDGE_PROBE_INTERVAL)
/// tick, its ping and its ladder run exactly as before. A STRANDED role
/// (membership PROVES its cell absent) is not probed — the watch only reads —
/// which is the observe-only contract unchanged: "keep watching `ListCellIds`
/// and probe the cell the moment membership says it is running", now kept.
///
/// ## Probes run beside the driver, never in front of it
///
/// A pass only OFFERS probes; [`drive_presence_recovery`] keeps every offered
/// probe in a set it polls in the same `select!` as readings, re-mints, the
/// watch cadence and shutdown. One slow cell therefore delays only its own
/// verdict: the watch keeps reading, a later-returning cell is probed in its
/// own reading's wake, and a re-mint is acted on as it lands. The set is
/// bounded — at most one probe per observed role, since a role with a probe
/// still in flight is not offered another — and the driver never drops one it
/// offered: a re-mint leaves it running (its verdict lands like any call's),
/// and shutdown stops taking events and DRAINS the set before returning,
/// because a conductor call cannot be cancelled, only abandoned.
///
/// ## Not a probe storm
///
/// ONE ADMITTED probe per role per PRESENCE RUN — an unbroken stretch of readings that
/// show its cell running, identified by the reading that began it and recorded
/// by the cache as each reading LANDS
/// ([`crate::services::cell_membership::MembershipCache::presence_run_at`]),
/// not inferred from whichever reading the driver happens to wake on. A role
/// whose probe was refused while its cell stays present is left to its own
/// supervisor tick (the steady cadence it always had), not re-probed on every
/// watch read. A reading that shows the cell absent, or a re-mint's revocation,
/// ends the run — even when the driver never sees that reading because a later
/// one coalesced it — so the next return of the cells earns exactly one more
/// pass. A re-mint also forgets which runs were probed. A run is spent only by
/// a probe the [`crate::services::cell_probe::ProbeGate`] admitted: an offer
/// that finds the supervisor's probe of the same role already in flight keeps
/// the run, waits on the gate's release (no polling) and is re-offered then —
/// so a supervisor probe that fails does not cost the run its own.
pub struct PresenceRecovery {
    cadence: Duration,
    window: Duration,
    state: std::sync::Mutex<SeamState>,
    /// Wakes the driver when a re-mint opens the watch.
    reopened: tokio::sync::Notify,
    passes: std::sync::atomic::AtomicU64,
}

impl Default for PresenceRecovery {
    fn default() -> Self {
        Self::with_timing(PRESENCE_WATCH_CADENCE, PRESENCE_WATCH_WINDOW)
    }
}

impl PresenceRecovery {
    /// A seam with the production bounds.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// A seam with its two bounds named by the caller. Production uses
    /// [`Self::new`]; tests shrink both so a watch can be driven in real time.
    pub(crate) fn with_timing(cadence: Duration, window: Duration) -> Self {
        Self {
            cadence,
            window,
            state: std::sync::Mutex::new(SeamState::default()),
            reopened: tokio::sync::Notify::new(),
            passes: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, SeamState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A bridge was re-minted at `now`: the cells on the far side are coming
    /// back. Opens (or extends) the watch, starts a new epoch, releases every
    /// role's presence run — the reading that ran it described the previous
    /// conductor — and wakes the driver. A probe already in flight is left to
    /// land.
    pub(crate) fn reopen_at(&self, now: std::time::Instant) {
        let until = now + self.window;
        {
            let mut state = self.state();
            state.watch_until = Some(state.watch_until.map_or(until, |open| open.max(until)));
            state.epoch = state.epoch.wrapping_add(1);
            state.probed.clear();
            state.pending.clear();
        }
        self.reopened.notify_one();
    }

    /// Is the post-re-mint watch open at `now`?
    pub(crate) fn watching_at(&self, now: std::time::Instant) -> bool {
        self.state().watch_until.is_some_and(|until| now < until)
    }

    /// How many passes have run. Observability and tests.
    pub fn passes(&self) -> u64 {
        self.passes.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// ONE PASS over every observed role of the conductor, against the cache's
    /// CURRENT reading: which probes are owed now. Offers them; does not run
    /// them — [`drive_presence_recovery`] does, beside its event loop.
    ///
    /// For each role: a cell the reading does not show running is left alone.
    /// A cell it shows running, on a role in a not-running episode with no
    /// probe of this seam in flight and a presence run not yet probed, is
    /// published through [`HcClientRegistry::publish_cell_state_from`] — the
    /// supervisor's own publication path, contradiction gate included, minus
    /// its refresh — and offered a probe when that classification owes one
    /// now. It asks the conductor nothing: the only reads this seam originates
    /// are the watch's. The per-role
    /// [`crate::services::cell_probe::ProbeGate`] keeps a probe the supervisor
    /// already has in flight from becoming two.
    pub(crate) fn plan_at(
        &self,
        cells: &[ObservedCell],
        cache: &crate::services::cell_membership::MembershipCache,
        at: crate::services::cell_membership::ObservedAt,
    ) -> (u64, Vec<Offer>) {
        self.passes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut offered = Vec::new();
        for cell in cells {
            let Some(run) = cell
                .cell_id
                .as_ref()
                .and_then(|id| cache.presence_run_at(id, at))
            else {
                continue;
            };
            if !crate::conductor_bridge_health::role_is_not_running(cell.role) {
                continue;
            }
            {
                let state = self.state();
                if state.in_flight.contains(cell.role)
                    || state.pending.contains(cell.role)
                    || state.probed.get(cell.role) == Some(&run)
                {
                    continue;
                }
            }
            // Published from the reading in hand. The planner never reads: its
            // only reads are the watch's, under the ceiling no revocation resets.
            let tick = HcClientRegistry::publish_cell_state_from(
                cell.role,
                cell.cell_id.as_ref(),
                cache,
                at,
            );
            if !tick.state.probe_is_owed_now() {
                continue;
            }
            let mut state = self.state();
            // Re-checked under the lock the claim is made in: the seam lock is
            // not held across the publish, which takes the role's own locks.
            if state.in_flight.contains(cell.role)
                || state.pending.contains(cell.role)
                || state.probed.get(cell.role) == Some(&run)
            {
                continue;
            }
            // OFFERED, not spent: the run is spent by `settle`, and only if
            // the gate admitted the probe.
            state.in_flight.insert(cell.role);
            offered.push(Offer {
                cell: cell.clone(),
                run,
            });
        }
        if !offered.is_empty() {
            info!(
                roles = ?offered.iter().map(|o| o.cell.role).collect::<Vec<_>>(),
                "cells present again on this conductor — probing every not-running role in ONE \
                 pass rather than on each role's next tick; a call that returns is still the \
                 only recovery"
            );
        }
        (self.state().epoch, offered)
    }

    /// An offered probe landed: the role may be offered another.
    ///
    /// A probe offered before the latest re-mint lands like any other — its
    /// verdict is already recorded by the zome-call path — and frees its role;
    /// the run it was offered for ended with that re-mint, so the role is
    /// re-planned against the readings of the new conductor.
    ///
    /// What the verdict does to the run, current epoch only:
    ///
    /// * a probe was ASKED (`Served`, `Refused`, `EncodeFailed`) — the run is
    ///   spent;
    /// * nothing in this run can ask one (`NoProbeAvailable`, `Unreachable`) —
    ///   the run is closed rather than re-offered, since re-offering would
    ///   answer the same at once; a re-mint opens the next;
    /// * the gate already holds a probe of this role (`AlreadyInFlight`, the
    ///   supervisor's) — NOT spent: the role goes pending and is re-offered
    ///   when that probe releases the gate, so if it fails this run still gets
    ///   its own probe;
    /// * the role is serving (`NotInEpisode`) — nothing is owed.
    pub(crate) fn settle(&self, landed: Landed) {
        use crate::services::cell_probe::ProbeVerdict;
        let mut state = self.state();
        state.in_flight.remove(landed.role);
        if landed.epoch != state.epoch {
            debug!(
                role = landed.role,
                offered_in_epoch = landed.epoch,
                epoch = state.epoch,
                "a presence probe offered before the latest re-mint landed after it — its \
                 verdict stands; the role is re-planned against the new conductor's readings"
            );
            return;
        }
        match landed.verdict {
            ProbeVerdict::Served
            | ProbeVerdict::Refused { .. }
            | ProbeVerdict::EncodeFailed { .. }
            | ProbeVerdict::NoProbeAvailable
            | ProbeVerdict::Unreachable => {
                state.probed.insert(landed.role, landed.run);
            }
            ProbeVerdict::AlreadyInFlight => {
                state.pending.insert(landed.role);
            }
            ProbeVerdict::NotInEpisode => {}
        }
    }

    /// Release every pending role whose gate slot is free again; `true` when
    /// any was. The caller arms the gate's release wake BEFORE calling this.
    fn release_pending(&self, gate: &crate::services::cell_probe::ProbeGate) -> bool {
        let mut state = self.state();
        let before = state.pending.len();
        state.pending.retain(|role| gate.in_flight(role));
        state.pending.len() < before
    }

    fn has_pending(&self) -> bool {
        !self.state().pending.is_empty()
    }

    /// [`Self::plan_at`], then every offered probe to completion, then
    /// [`Self::settle`] — the driver's schedule with the event loop taken out,
    /// for tests that pin what one pass decides.
    #[cfg(test)]
    pub(crate) async fn run_pass_at(
        &self,
        cells: &[ObservedCell],
        cache: &crate::services::cell_membership::MembershipCache,
        at: crate::services::cell_membership::ObservedAt,
        conductor: &dyn PresenceConductor,
    ) -> PresencePass {
        let (epoch, offered) = self.plan_at(cells, cache, at);
        let verdicts = futures_util::future::join_all(
            offered
                .iter()
                .map(|o| conductor.probe_cell(o.cell.role, o.cell.tended_by)),
        )
        .await;
        for (offer, verdict) in offered.iter().zip(verdicts) {
            self.settle(Landed {
                role: offer.cell.role,
                run: offer.run,
                epoch,
                verdict,
            });
        }
        PresencePass {
            probed: offered.iter().map(|o| o.cell.role).collect(),
        }
    }
}

/// The ONE process-wide presence seam. Tests build their own.
pub(crate) fn presence_recovery() -> &'static PresenceRecovery {
    static SEAM: std::sync::OnceLock<PresenceRecovery> = std::sync::OnceLock::new();
    SEAM.get_or_init(PresenceRecovery::new)
}

/// One offered probe, as the driver holds it.
type OfferedProbe<'a> = std::pin::Pin<Box<dyn std::future::Future<Output = Landed> + Send + 'a>>;

/// The presence seam's driver.
///
/// ONE `select!` over: a landed reading, a re-mint, the watch cadence (only
/// while the watch is open), the probe gate's release (only while a role is
/// pending behind someone else's probe), an offered probe landing, and
/// shutdown. After any
/// of the first four it EVALUATES — if watching and some observed role is not
/// running, read; then plan one pass and add its offered probes to the set the
/// same `select!` polls. Nothing it awaits waits on a probe, so a slow cell
/// holds up only its own verdict.
///
/// No timer of its own outside the watch: with the watch shut it sleeps until
/// a reading lands, a re-mint opens it or a probe lands. The reads it takes
/// itself go through
/// [`crate::services::cell_membership::MembershipCache::refresh_if_older_than_at`],
/// whose ceiling no revocation resets, so however often it wakes it asks the
/// conductor at most once per cadence.
///
/// On shutdown it takes no further events and DRAINS the probes it offered:
/// each is one read-only extern, and dropping one would abandon a call the
/// conductor goes on running.
pub(crate) async fn drive_presence_recovery(
    seam: &PresenceRecovery,
    cache: &crate::services::cell_membership::MembershipCache,
    conductor: &dyn PresenceConductor,
    mut shutdown: tokio::sync::broadcast::Receiver<()>,
) {
    use futures_util::stream::{FuturesUnordered, StreamExt};
    let gate = conductor.probe_gate();
    let mut probes: FuturesUnordered<OfferedProbe<'_>> = FuturesUnordered::new();
    loop {
        // ARM the gate's release wake before looking at it, so a release that
        // lands between the look and the `select!` is not missed. A pending
        // role whose slot is already free is re-offered now; one still held
        // waits for a release — never a poll.
        let released = gate.released().notified();
        tokio::pin!(released);
        released.as_mut().enable();
        if !seam.release_pending(gate) {
            let watching = seam.watching_at(std::time::Instant::now());
            let pending = seam.has_pending();
            tokio::select! {
                _ = cache.read_landed().notified() => {}
                _ = seam.reopened.notified() => {}
                _ = tokio::time::sleep(seam.cadence), if watching => {}
                _ = &mut released, if pending => {}
                Some(landed) = probes.next(), if !probes.is_empty() => seam.settle(landed),
                _ = shutdown.recv() => {
                    while let Some(landed) = probes.next().await {
                        seam.settle(landed);
                    }
                    return;
                }
            }
        }
        let cells = conductor.observed_cells();
        let at = crate::services::cell_membership::ObservedAt::now();
        if seam.watching_at(at.mono)
            && cells
                .iter()
                .any(|cell| crate::conductor_bridge_health::role_is_not_running(cell.role))
        {
            // The ONE read this seam originates, under the ceiling.
            if let Some(src) = conductor.membership_source() {
                cache
                    .refresh_if_older_than_at(src.as_ref(), at, seam.cadence)
                    .await;
            }
        }
        let (epoch, offered) = seam.plan_at(
            &cells,
            cache,
            crate::services::cell_membership::ObservedAt::now(),
        );
        for offer in offered {
            probes.push(Box::pin(async move {
                let verdict = conductor
                    .probe_cell(offer.cell.role, offer.cell.tended_by)
                    .await;
                Landed {
                    role: offer.cell.role,
                    run: offer.run,
                    epoch,
                    verdict,
                }
            }));
        }
    }
}

#[cfg(test)]
mod presence_recovery_tests {
    //! The presence seam, driven against a fake conductor: the pass and its
    //! driver are the production functions; only the conductor is supplied.
    //! Every role name is this module's own so no parallel test can read
    //! these synthetic outages through the process-wide aggregate.
    use super::*;
    use crate::conductor_bridge_health::{
        record_role_app_disabled, record_role_success, role_bridge_health, role_is_not_running,
        ZomePathStatus,
    };
    use crate::hc_client_registry::{BRIDGE_PROBE_INTERVAL, SUPERVISED_ROLES};
    use crate::services::cell_membership::{
        fake::FakeAdmin, MembershipCache, ObservedAt, RefreshOutcome, RunningCellSource,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    fn cell(seed: u8) -> holochain_client::CellId {
        use holochain_types::prelude::{AgentPubKey, DnaHash};
        holochain_client::CellId::new(
            DnaHash::from_raw_32(vec![seed; 32]),
            AgentPubKey::from_raw_32(vec![seed.wrapping_add(23); 32]),
        )
    }

    fn observed(
        role: &'static str,
        tended_by: &'static str,
        cell_id: &holochain_client::CellId,
    ) -> ObservedCell {
        ObservedCell {
            role,
            tended_by,
            cell_id: Some(cell_id.clone()),
        }
    }

    /// A conductor whose running map the test sets (through the shared
    /// [`FakeAdmin`]) and whose probes are recorded. A probe that "returns"
    /// reaches `record_role_success`, exactly where a real one lands.
    struct FakeConductor {
        cells: Vec<ObservedCell>,
        admin: Arc<FakeAdmin>,
        serves: AtomicBool,
        probes: Mutex<Vec<(&'static str, &'static str)>>,
        /// A role listed here PARKS its probe until [`Self::unpark`] — the slow
        /// cell a single uncancellable call can be.
        parked: Mutex<std::collections::BTreeMap<&'static str, Arc<tokio::sync::Semaphore>>>,
        landed: Mutex<Vec<&'static str>>,
        /// The REAL gate production admits through, owned by this test.
        gate: crate::services::cell_probe::ProbeGate,
    }

    /// A probe that asks nothing real; the gate's admission is what is tested.
    const TEST_PROBE: crate::services::cell_probe::CellProbe =
        crate::services::cell_probe::CellProbe {
            zome: "presence_test",
            fn_name: "presence_test_probe",
            payload: crate::services::cell_probe::ProbePayload::Unit,
        };

    impl FakeConductor {
        fn new(cells: Vec<ObservedCell>, admin: Arc<FakeAdmin>, serves: bool) -> Self {
            Self {
                cells,
                admin,
                serves: AtomicBool::new(serves),
                probes: Mutex::new(Vec::new()),
                parked: Mutex::new(std::collections::BTreeMap::new()),
                landed: Mutex::new(Vec::new()),
                gate: crate::services::cell_probe::ProbeGate::new(),
            }
        }

        fn park(&self, role: &'static str) {
            self.parked
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(role, Arc::new(tokio::sync::Semaphore::new(0)));
        }

        fn unpark(&self, role: &'static str) {
            if let Some(gate) = self
                .parked
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(role)
            {
                gate.add_permits(tokio::sync::Semaphore::MAX_PERMITS);
            }
        }

        fn landed_of(&self, role: &str) -> usize {
            self.landed
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .filter(|r| **r == role)
                .count()
        }

        fn probes(&self) -> Vec<(&'static str, &'static str)> {
            self.probes
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        }

        fn probes_of(&self, role: &str) -> usize {
            self.probes().iter().filter(|(r, _)| *r == role).count()
        }
    }

    #[async_trait::async_trait]
    impl PresenceConductor for FakeConductor {
        fn observed_cells(&self) -> Vec<ObservedCell> {
            self.cells.clone()
        }

        fn membership_source(&self) -> Option<Arc<dyn RunningCellSource>> {
            Some(Arc::clone(&self.admin) as Arc<dyn RunningCellSource>)
        }

        async fn probe_cell(
            &self,
            role: &'static str,
            tended_by: &'static str,
        ) -> crate::services::cell_probe::ProbeVerdict {
            use crate::services::cell_probe::{probe_admission_with, ProbeVerdict};
            // Through the REAL admission: never-while-healthy, then the gate.
            let (_probe, _slot) = match probe_admission_with(role, Some(TEST_PROBE), &self.gate) {
                Ok(admitted) => admitted,
                Err(verdict) => return verdict,
            };
            self.probes
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push((role, tended_by));
            let parked = self
                .parked
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(role)
                .cloned();
            if let Some(gate) = parked {
                let _ = gate.acquire().await;
            }
            let served = self.serves.load(Ordering::SeqCst);
            if served {
                record_role_success(role);
            }
            self.landed
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(role);
            if served {
                ProbeVerdict::Served
            } else {
                ProbeVerdict::Refused {
                    msg: "CellDisabled (presence-seam test)".to_string(),
                }
            }
        }

        fn probe_gate(&self) -> &crate::services::cell_probe::ProbeGate {
            &self.gate
        }
    }

    /// Start the PRODUCTION driver over test-owned state.
    fn spawn_driver(
        seam: &Arc<PresenceRecovery>,
        cache: &Arc<MembershipCache>,
        conductor: &Arc<FakeConductor>,
    ) -> (
        tokio::sync::broadcast::Sender<()>,
        tokio::task::JoinHandle<()>,
    ) {
        let (tx, rx) = tokio::sync::broadcast::channel(1);
        let (seam, cache, conductor) = (Arc::clone(seam), Arc::clone(cache), Arc::clone(conductor));
        let handle = tokio::spawn(async move {
            drive_presence_recovery(&seam, &cache, conductor.as_ref(), rx).await;
        });
        (tx, handle)
    }

    async fn stop(tx: tokio::sync::broadcast::Sender<()>, handle: tokio::task::JoinHandle<()>) {
        let _ = tx.send(());
        handle.await.expect("the driver exits on shutdown");
    }

    /// Wait for `cond`, failing loudly rather than hanging.
    async fn until(what: &str, cond: impl Fn() -> bool) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !cond() {
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for: {what}"));
    }

    /// A reading of "now", taken the way a supervisor tick or the diagnostics
    /// handler takes one — through the shared cache, by someone other than the
    /// presence seam.
    async fn someone_reads(cache: &MembershipCache, admin: &FakeAdmin) -> RefreshOutcome {
        cache
            .refresh_if_older_than_at(admin, ObservedAt::now(), Duration::ZERO)
            .await
    }

    fn open_episode(role: &str) {
        record_role_app_disabled(role, "CellDisabled (presence-seam test)");
        assert!(
            role_is_not_running(role),
            "precondition: {role} is not running"
        );
    }

    fn is_live(role: &str) -> bool {
        role_bridge_health().for_role(role).status() == ZomePathStatus::Live
    }

    /// (a) A membership flip absent→present probes EVERY role of the conductor
    /// in the wake of the reading that saw it — no tick involved anywhere.
    #[tokio::test]
    async fn a_reading_that_finds_the_cells_back_probes_every_role_without_a_tick() {
        const A: &str = "test-presence-flip-driver";
        const B: &str = "test-presence-flip-own";
        const X: &str = "test-presence-flip-cross";
        let (ca, cb, cx) = (cell(81), cell(82), cell(83));
        let admin = Arc::new(FakeAdmin::new(Ok(Vec::new())));
        let conductor = Arc::new(FakeConductor::new(
            vec![
                observed(A, A, &ca),
                observed(B, B, &cb),
                observed(X, A, &cx),
            ],
            Arc::clone(&admin),
            true,
        ));
        for role in [A, B, X] {
            open_episode(role);
        }
        let seam = Arc::new(PresenceRecovery::new());
        let cache = Arc::new(MembershipCache::new());
        let (tx, handle) = spawn_driver(&seam, &cache, &conductor);

        // A reading lands while the cells are still ABSENT: a pass runs and
        // asks nothing, because a probe would fail by construction.
        assert_eq!(
            someone_reads(&cache, &admin).await,
            RefreshOutcome::Read { running: 0 }
        );
        until("the pass over the absent reading", || seam.passes() >= 1).await;
        assert!(conductor.probes().is_empty(), "absent cells are not probed");

        // The cells rejoin the running map, and the next reading says so.
        admin.set_tail(Ok(vec![ca.clone(), cb.clone(), cx.clone()]));
        let started = std::time::Instant::now();
        assert_eq!(
            someone_reads(&cache, &admin).await,
            RefreshOutcome::Read { running: 3 }
        );
        until("every role probed", || conductor.probes().len() == 3).await;
        let elapsed = started.elapsed();

        let mut probed = conductor.probes();
        probed.sort();
        let mut expected = vec![(A, A), (B, B), (X, A)];
        expected.sort();
        assert_eq!(
            probed, expected,
            "every observed role, the cross-cell one through its driver's client"
        );
        assert!(
            elapsed < BRIDGE_PROBE_INTERVAL / 10,
            "in the reading's own wake, not on a tick: {elapsed:?}"
        );
        for role in [A, B, X] {
            assert!(
                is_live(role),
                "{role}: the probe's returned call is the recovery"
            );
        }
        assert_eq!(
            admin.call_count(),
            2,
            "with no watch open the seam takes no reading of its own"
        );
        stop(tx, handle).await;
    }

    /// (a, after a re-mint) The watch is the reader: with nobody else asking,
    /// cells that rejoin are seen within a few cadences and every role probed.
    #[tokio::test]
    async fn after_a_remint_the_watch_sees_the_cells_rejoin_with_no_other_reader() {
        const A: &str = "test-presence-watch-driver";
        const B: &str = "test-presence-watch-own";
        const X: &str = "test-presence-watch-cross";
        let cadence = Duration::from_millis(20);
        let (ca, cb, cx) = (cell(84), cell(85), cell(86));
        let admin = Arc::new(FakeAdmin::new(Ok(Vec::new())));
        let conductor = Arc::new(FakeConductor::new(
            vec![
                observed(A, A, &ca),
                observed(B, B, &cb),
                observed(X, A, &cx),
            ],
            Arc::clone(&admin),
            true,
        ));
        for role in [A, B, X] {
            open_episode(role);
        }
        let seam = Arc::new(PresenceRecovery::with_timing(
            cadence,
            Duration::from_secs(5),
        ));
        let cache = Arc::new(MembershipCache::new());
        let (tx, handle) = spawn_driver(&seam, &cache, &conductor);

        seam.reopen_at(std::time::Instant::now());
        until("the watch reading on its own", || admin.call_count() >= 2).await;
        assert!(
            conductor.probes().is_empty(),
            "absent cells are watched, not probed"
        );

        admin.set_tail(Ok(vec![ca.clone(), cb.clone(), cx.clone()]));
        let started = std::time::Instant::now();
        until("every role probed", || conductor.probes().len() == 3).await;
        let elapsed = started.elapsed();
        assert!(
            elapsed < cadence * 10,
            "seen within a few cadences of rejoining, not on a 20s tick: {elapsed:?}"
        );
        for role in [A, B, X] {
            assert_eq!(conductor.probes_of(role), 1, "{role}: one probe");
            assert!(is_live(role), "{role} recovered");
        }
        stop(tx, handle).await;
    }

    /// (b) The cross-cell role is probed in the SAME pass as its driver — not a
    /// tick later on the driver's loop — and through the driver's client.
    #[tokio::test]
    async fn the_cross_cell_role_recovers_in_the_same_pass_as_its_driver() {
        const A: &str = "test-presence-same-pass-driver";
        const X: &str = "test-presence-same-pass-cross";
        let (ca, cx) = (cell(87), cell(88));
        let admin = Arc::new(FakeAdmin::new(Ok(vec![ca.clone(), cx.clone()])));
        let conductor = FakeConductor::new(
            vec![observed(A, A, &ca), observed(X, A, &cx)],
            Arc::clone(&admin),
            true,
        );
        open_episode(A);
        open_episode(X);
        let cache = MembershipCache::new();
        someone_reads(&cache, &admin).await;

        let seam = PresenceRecovery::new();
        let pass = seam
            .run_pass_at(
                &conductor.observed_cells(),
                &cache,
                ObservedAt::now(),
                &conductor,
            )
            .await;
        assert_eq!(pass.probed, vec![A, X], "ONE pass, both roles");
        assert_eq!(conductor.probes(), vec![(A, A), (X, A)]);
        assert!(is_live(A) && is_live(X), "both recovered by that one pass");
        assert_eq!(seam.passes(), 1);
    }

    /// The production conductor names every observed role with the client that
    /// carries its cell, and a registry with no clients offers nothing to read.
    #[test]
    fn every_observed_role_is_tended_by_the_client_that_carries_it() {
        for role in OBSERVED_ROLES {
            let tender = tender_of(role);
            assert!(
                SUPERVISED_ROLES.contains(&tender),
                "{role} tended by a slot"
            );
            if CROSS_CELL_ROLES.contains(&role) {
                assert_eq!(tender, CROSS_CELL_DRIVER_ROLE);
            } else {
                assert_eq!(tender, role);
            }
        }
        let registry = HcClientRegistry::empty();
        let cells = registry.observed_cells();
        assert_eq!(
            cells.iter().map(|c| c.role).collect::<Vec<_>>(),
            OBSERVED_ROLES.to_vec()
        );
        assert!(cells.iter().all(|c| c.cell_id.is_none()));
        assert!(registry.membership_source().is_none());
    }

    /// (c) The steady state is untouched: the pass leaves healthy roles alone,
    /// the seam never mutates the conductor or the ladder, never wakes a
    /// supervisor, and the supervisor still ticks on `BRIDGE_PROBE_INTERVAL`.
    #[tokio::test]
    async fn the_steady_tick_is_unchanged_and_healthy_roles_are_left_alone() {
        const HEALTHY: &str = "test-presence-steady-healthy";
        const QUIET: &str = "test-presence-steady-never-observed";
        let (ch, cq) = (cell(89), cell(90));
        let admin = Arc::new(FakeAdmin::new(Ok(vec![ch.clone(), cq.clone()])));
        let conductor = FakeConductor::new(
            vec![observed(HEALTHY, HEALTHY, &ch), observed(QUIET, QUIET, &cq)],
            Arc::clone(&admin),
            true,
        );
        record_role_success(HEALTHY);
        let cache = MembershipCache::new();
        someone_reads(&cache, &admin).await;
        let seam = PresenceRecovery::new();
        for _ in 0..3 {
            let pass = seam
                .run_pass_at(
                    &conductor.observed_cells(),
                    &cache,
                    ObservedAt::now(),
                    &conductor,
                )
                .await;
            assert!(pass.probed.is_empty(), "no episode, nothing to prove");
        }
        assert!(conductor.probes().is_empty());
        assert_eq!(admin.call_count(), 1, "the pass reads nothing itself");

        assert_eq!(
            BRIDGE_PROBE_INTERVAL,
            Duration::from_secs(20),
            "the steady cadence"
        );
        let seam_src = include_str!("presence_recovery.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("the seam, above its tests");
        let source = include_str!("../hc_client_registry.rs");
        for forbidden in [
            "enable_app(",
            "enable_ledger",
            "note_attempt",
            "transport_ledger()",
            "raise_doubt",
            ".invalidate(",
        ] {
            assert!(
                !seam_src.contains(forbidden),
                "the presence seam must not reach `{forbidden}`"
            );
        }
        let supervisor = source
            .split("pub fn spawn_bridge_supervisor(")
            .nth(1)
            .expect("supervisor")
            .split("#[cfg(test)]")
            .next()
            .expect("supervisor boundary");
        assert!(supervisor.contains("_ = tokio::time::sleep(BRIDGE_PROBE_INTERVAL) => {}"));
        assert!(supervisor.contains("drive_presence_recovery("));
        let refill = supervisor
            .find("registry.set_client(role, Some(Arc::clone(&serving)))")
            .expect("the re-minted handle goes back in its slot");
        let reopen = supervisor
            .find(".reopen_at(std::time::Instant::now())")
            .expect("a re-mint opens the presence watch");
        assert!(
            refill < reopen,
            "the watch opens once the fresh handle is servable"
        );
    }

    /// (c, the watch's own cost) With the watch shut the seam reads nothing,
    /// however often it is woken; once its window closes it stops reading.
    #[tokio::test]
    async fn the_watch_reads_only_inside_its_window_and_never_when_shut() {
        const ROLE: &str = "test-presence-watch-window";
        let cadence = Duration::from_millis(10);
        let window = Duration::from_millis(150);
        let c = cell(91);
        let admin = Arc::new(FakeAdmin::new(Ok(Vec::new())));
        let conductor = Arc::new(FakeConductor::new(
            vec![observed(ROLE, ROLE, &c)],
            Arc::clone(&admin),
            true,
        ));
        open_episode(ROLE);
        let seam = Arc::new(PresenceRecovery::with_timing(cadence, window));
        let cache = Arc::new(MembershipCache::new());
        let (tx, handle) = spawn_driver(&seam, &cache, &conductor);

        // Shut: woken by five readings, it adds none of its own.
        for n in 1..=5 {
            someone_reads(&cache, &admin).await;
            until("the pass over that reading", || seam.passes() >= n).await;
        }
        assert_eq!(admin.call_count(), 5, "a shut watch reads nothing");

        // Open: it reads on its own, at most once per cadence, then stops.
        let opened = std::time::Instant::now();
        seam.reopen_at(opened);
        tokio::time::sleep(window + cadence * 10).await;
        let after_window = admin.call_count();
        let watch_reads = after_window - 5;
        let ceiling = (window.as_millis() / cadence.as_millis()) as usize + 1;
        assert!(watch_reads >= 1, "the open watch read");
        assert!(
            watch_reads <= ceiling,
            "{watch_reads} reads inside a {window:?} window at a {cadence:?} cadence"
        );
        tokio::time::sleep(cadence * 10).await;
        assert_eq!(
            admin.call_count(),
            after_window,
            "a closed watch stops reading"
        );
        stop(tx, handle).await;
    }

    /// (d) No probe storm: one presence run earns one probe per role however
    /// many readings follow it; only the cells leaving and returning (or a
    /// re-mint) earns another.
    #[tokio::test]
    async fn one_presence_run_earns_one_probe_however_many_readings_follow() {
        const A: &str = "test-presence-storm-a";
        const B: &str = "test-presence-storm-b";
        let cadence = Duration::from_millis(10);
        let (ca, cb) = (cell(92), cell(93));
        let present = vec![ca.clone(), cb.clone()];
        let admin = Arc::new(FakeAdmin::new(Ok(present.clone())));
        // Probes are REFUSED: the cells stay present and the roles stay not
        // running across every reading — the shape that would storm.
        let conductor = Arc::new(FakeConductor::new(
            vec![observed(A, A, &ca), observed(B, B, &cb)],
            Arc::clone(&admin),
            false,
        ));
        open_episode(A);
        open_episode(B);
        let seam = Arc::new(PresenceRecovery::with_timing(
            cadence,
            Duration::from_secs(5),
        ));
        let cache = Arc::new(MembershipCache::new());
        let (tx, handle) = spawn_driver(&seam, &cache, &conductor);

        seam.reopen_at(std::time::Instant::now());
        until("twenty watch readings", || admin.call_count() >= 20).await;
        let passes = seam.passes();
        assert!(passes >= 20, "many passes ran ({passes})");
        assert_eq!(conductor.probes_of(A), 1, "one probe for one presence run");
        assert_eq!(conductor.probes_of(B), 1, "one probe for one presence run");
        assert!(role_is_not_running(A) && role_is_not_running(B));

        // The cells leave the running map, then return: a NEW run, one more.
        admin.set_tail(Ok(Vec::new()));
        let seen = admin.call_count();
        until("a reading of the absence", || admin.call_count() > seen + 1).await;
        admin.set_tail(Ok(present.clone()));
        until("the return probed", || {
            conductor.probes_of(A) == 2 && conductor.probes_of(B) == 2
        })
        .await;

        // A re-mint releases the run too.
        seam.reopen_at(std::time::Instant::now());
        until("the re-mint's run probed", || {
            conductor.probes_of(A) == 3 && conductor.probes_of(B) == 3
        })
        .await;
        let settled = conductor.probes().len();
        let reads = admin.call_count();
        until("ten more watch readings", || {
            admin.call_count() >= reads + 10
        })
        .await;
        assert_eq!(
            conductor.probes().len(),
            settled,
            "and nothing more after it"
        );
        stop(tx, handle).await;
    }

    /// FINDING 2 — one slow probe stalls nothing but itself.
    ///
    /// A's probe parks (a slow cell; the call cannot be cancelled). While it is
    /// parked the watch goes on reading, B's cell returns and B is probed and
    /// recovered in its own reading's wake, and a re-mint is acted on. A is
    /// never offered a second probe while its first is in flight, and shutdown
    /// DRAINS A's probe rather than dropping it.
    #[tokio::test]
    async fn a_parked_probe_holds_up_only_its_own_verdict() {
        const A: &str = "test-presence-parked-slow";
        const B: &str = "test-presence-parked-later";
        let cadence = Duration::from_millis(10);
        let (ca, cb) = (cell(94), cell(95));
        let admin = Arc::new(FakeAdmin::new(Ok(vec![ca.clone()])));
        let conductor = Arc::new(FakeConductor::new(
            vec![observed(A, A, &ca), observed(B, B, &cb)],
            Arc::clone(&admin),
            true,
        ));
        conductor.park(A);
        open_episode(A);
        open_episode(B);
        let seam = Arc::new(PresenceRecovery::with_timing(
            cadence,
            Duration::from_secs(5),
        ));
        let cache = Arc::new(MembershipCache::new());
        let (tx, handle) = spawn_driver(&seam, &cache, &conductor);

        seam.reopen_at(std::time::Instant::now());
        until("A's probe offered and parked", || {
            conductor.probes_of(A) == 1
        })
        .await;
        let reads = admin.call_count();
        until("the watch reading on behind the parked probe", || {
            admin.call_count() >= reads + 5
        })
        .await;

        // B's cell returns while A is still parked.
        admin.set_tail(Ok(vec![ca.clone(), cb.clone()]));
        until("B probed and recovered", || is_live(B)).await;
        assert_eq!(conductor.landed_of(A), 0, "A is still parked");
        assert!(role_is_not_running(A));

        // A re-mint lands while A is parked: acted on, and A is still not
        // offered a second probe while its first is in flight.
        let passes = seam.passes();
        seam.reopen_at(std::time::Instant::now());
        until("the re-mint acted on", || seam.passes() > passes).await;
        let reads = admin.call_count();
        until("more readings", || admin.call_count() >= reads + 5).await;
        assert_eq!(conductor.probes_of(A), 1, "one probe in flight per role");

        // Shutdown waits for the offered call instead of abandoning it.
        let _ = tx.send(());
        tokio::time::sleep(cadence * 5).await;
        assert!(
            !handle.is_finished(),
            "the parked probe is drained, not dropped"
        );
        conductor.unpark(A);
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .expect("the driver returns once its offered probe lands")
            .expect("the driver task did not panic");
        assert_eq!(conductor.landed_of(A), 1);
        assert!(is_live(A), "A's verdict landed through the zome-call path");
    }

    /// FINDING 1 — a burst of readings cannot hide a new presence run.
    ///
    /// A's run is probed (refused) and A stays not running. Then, while B's
    /// probe is parked, the cells go ABSENT and come back in two readings that
    /// land back to back — one wake for the driver, which only ever sees the
    /// second. The run recorded at landing still differs, so A is owed again.
    #[tokio::test]
    async fn two_readings_behind_a_parked_probe_still_start_a_new_run() {
        const A: &str = "test-presence-burst-a";
        const B: &str = "test-presence-burst-parked";
        let (ca, cb) = (cell(96), cell(97));
        let present = vec![ca.clone(), cb.clone()];
        let admin = Arc::new(FakeAdmin::new(Ok(present.clone())));
        let conductor = Arc::new(FakeConductor::new(
            vec![observed(A, A, &ca), observed(B, B, &cb)],
            Arc::clone(&admin),
            false,
        ));
        conductor.park(B);
        open_episode(A);
        open_episode(B);
        let seam = Arc::new(PresenceRecovery::new());
        let cache = Arc::new(MembershipCache::new());
        let (tx, handle) = spawn_driver(&seam, &cache, &conductor);

        someone_reads(&cache, &admin).await;
        until("A's first probe landed", || conductor.landed_of(A) == 1).await;
        until("B's probe offered and parked", || {
            conductor.probes_of(B) == 1
        })
        .await;

        // Absent, then running again — two readings with no yield between
        // them, so the driver wakes once and sees only the second.
        admin.set_tail(Ok(Vec::new()));
        assert_eq!(
            someone_reads(&cache, &admin).await,
            RefreshOutcome::Read { running: 0 }
        );
        admin.set_tail(Ok(present.clone()));
        assert_eq!(
            someone_reads(&cache, &admin).await,
            RefreshOutcome::Read { running: 2 }
        );
        until("the new run probed", || conductor.probes_of(A) == 2).await;
        assert_eq!(conductor.probes_of(B), 1, "B's parked probe is not doubled");

        conductor.unpark(B);
        stop(tx, handle).await;
    }

    /// The run is recorded by the cache as each reading lands: continuous
    /// presence keeps it, an absence or a revocation ends it.
    #[tokio::test]
    async fn the_cache_records_a_presence_run_per_cell_at_landing() {
        let c = cell(98);
        let admin = FakeAdmin::new(Ok(vec![c.clone()]));
        let cache = MembershipCache::new();
        someone_reads(&cache, &admin).await;
        let first = cache
            .presence_run_at(&c, ObservedAt::now())
            .expect("running");
        someone_reads(&cache, &admin).await;
        assert_eq!(cache.presence_run_at(&c, ObservedAt::now()), Some(first));

        admin.set_tail(Ok(Vec::new()));
        someone_reads(&cache, &admin).await;
        assert_eq!(cache.presence_run_at(&c, ObservedAt::now()), None);
        admin.set_tail(Ok(vec![c.clone()]));
        someone_reads(&cache, &admin).await;
        let second = cache
            .presence_run_at(&c, ObservedAt::now())
            .expect("running");
        assert!(second > first, "absence ended the run");

        cache.invalidate("re-mint (test)");
        someone_reads(&cache, &admin).await;
        let third = cache
            .presence_run_at(&c, ObservedAt::now())
            .expect("running");
        assert!(third > second, "a revocation ended the run too");
    }

    /// FINDING 3 — an offer that collides with the supervisor's probe keeps
    /// its run.
    ///
    /// The supervisor holds A's gate slot. The presence pass offers A, the gate
    /// answers `AlreadyInFlight`, and NOTHING is spent: the driver waits on the
    /// gate's release without polling (no passes while it waits), and when the
    /// supervisor's probe ends without recovering A, A gets its own probe.
    #[tokio::test]
    async fn an_offer_refused_by_the_gate_waits_for_its_release_and_keeps_the_run() {
        const A: &str = "test-presence-collision";
        let c = cell(99);
        let admin = Arc::new(FakeAdmin::new(Ok(vec![c.clone()])));
        let conductor = Arc::new(FakeConductor::new(
            vec![observed(A, A, &c)],
            Arc::clone(&admin),
            true,
        ));
        open_episode(A);
        let supervisors_probe = conductor.gate.claim(A).expect("the supervisor's slot");
        let seam = Arc::new(PresenceRecovery::new());
        let cache = Arc::new(MembershipCache::new());
        let (tx, handle) = spawn_driver(&seam, &cache, &conductor);

        someone_reads(&cache, &admin).await;
        until("the offer met the supervisor's probe", || {
            seam.has_pending()
        })
        .await;
        let passes = seam.passes();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(
            seam.passes(),
            passes,
            "pending waits on the gate — no spinning"
        );
        assert_eq!(conductor.probes_of(A), 0, "no probe was asked");

        // The supervisor's probe ends and A is still not running.
        drop(supervisors_probe);
        until("A's own probe", || conductor.probes_of(A) == 1).await;
        until("A recovered", || is_live(A)).await;
        assert!(!seam.has_pending());
        stop(tx, handle).await;
    }

    /// RE-REVIEW A — the planner originates no read at all.
    ///
    /// The supervisor's 15 s window has lapsed (a revocation clears it just
    /// the same) while the reading still holds authority. Publishing through
    /// the supervisor's refresh would have read here, inside the watch's
    /// ceiling; the planner publishes from the reading in hand and asks the
    /// conductor nothing.
    #[tokio::test]
    async fn the_planner_publishes_from_the_reading_in_hand_and_reads_nothing() {
        use crate::services::cell_membership::{
            MEMBERSHIP_AUTHORITY_TTL, MEMBERSHIP_REFRESH_INTERVAL,
        };
        const A: &str = "test-presence-planner-no-read";
        let c = cell(100);
        let admin = FakeAdmin::new(Ok(vec![c.clone()]));
        let cache = MembershipCache::new();
        let t0: u64 = 1_700_000_000_000;
        cache
            .refresh_if_older_than_at(&admin, ObservedAt::from(t0), Duration::ZERO)
            .await;
        open_episode(A);

        let later = t0 + MEMBERSHIP_REFRESH_INTERVAL.as_millis() as u64 + 1_000;
        assert!(later < t0 + MEMBERSHIP_AUTHORITY_TTL.as_millis() as u64);
        assert!(cache.is_stale_at(ObservedAt::from(later)), "precondition");
        let seam = PresenceRecovery::new();
        let (_, offered) = seam.plan_at(&[observed(A, A, &c)], &cache, ObservedAt::from(later));
        assert_eq!(
            offered.iter().map(|o| o.cell.role).collect::<Vec<_>>(),
            vec![A],
            "owed, and published from the reading in hand"
        );
        assert_eq!(
            admin.call_count(),
            1,
            "the planner asked the conductor nothing"
        );
    }
}
