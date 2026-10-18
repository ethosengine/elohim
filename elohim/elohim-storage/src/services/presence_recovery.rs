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

use tracing::info;

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
    async fn probe_cell(&self, role: &'static str, tended_by: &'static str);
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

    async fn probe_cell(&self, role: &'static str, tended_by: &'static str) {
        if let Some(hc) = self.client(tended_by) {
            HcClientRegistry::probe_the_cell(role, &hc).await;
        }
    }
}

/// What one presence pass did. Observability and tests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PresencePass {
    /// The roles probed in this pass, in [`PresenceConductor::observed_cells`]
    /// order. Every probe of a pass is in flight at once.
    pub probed: Vec<&'static str>,
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
/// `mishpat` included, is probed in the SAME pass, concurrently, and the result
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
/// ## Not a probe storm
///
/// ONE probe per role per PRESENCE RUN — a stretch of readings that show its
/// cell running. The pass spends the role's run when it probes; a role whose
/// probe was refused while its cell stays present is left to its own
/// supervisor tick (the steady cadence it always had), not re-probed on every
/// watch read. The run is released when a reading no longer shows the cell
/// (absent, unknown, revoked) and by a re-mint, so the next return of the cells
/// earns exactly one more pass.
pub struct PresenceRecovery {
    cadence: Duration,
    window: Duration,
    /// When the post-re-mint watch closes; `None` = never opened.
    watch_until: std::sync::Mutex<Option<std::time::Instant>>,
    /// Roles whose current presence run has already been probed by a pass.
    spent: std::sync::Mutex<std::collections::BTreeSet<&'static str>>,
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
            watch_until: std::sync::Mutex::new(None),
            spent: std::sync::Mutex::new(std::collections::BTreeSet::new()),
            reopened: tokio::sync::Notify::new(),
            passes: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// A bridge was re-minted at `now`: the cells on the far side are coming
    /// back. Opens (or extends) the watch, releases every role's presence run —
    /// the reading that ran it described the previous conductor — and wakes
    /// the driver.
    pub(crate) fn reopen_at(&self, now: std::time::Instant) {
        let until = now + self.window;
        {
            let mut watch = self.watch_until.lock().unwrap_or_else(|e| e.into_inner());
            *watch = Some(watch.map_or(until, |open| open.max(until)));
        }
        self.spent.lock().unwrap_or_else(|e| e.into_inner()).clear();
        self.reopened.notify_one();
    }

    /// Is the post-re-mint watch open at `now`?
    pub(crate) fn watching_at(&self, now: std::time::Instant) -> bool {
        self.watch_until
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some_and(|until| now < until)
    }

    /// How many passes have run. Observability and tests.
    pub fn passes(&self) -> u64 {
        self.passes.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Claim `role`'s current presence run; `false` when a pass already has.
    fn spend(&self, role: &'static str) -> bool {
        self.spent
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(role)
    }

    fn release(&self, role: &'static str) {
        self.spent
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(role);
    }

    /// ONE PASS over every observed role of the conductor, against the cache's
    /// CURRENT reading.
    ///
    /// For each role: a cell the reading does not show running releases the
    /// role's presence run and is left alone. A cell it shows running, on a
    /// role in a not-running episode whose run is unspent, is published through
    /// [`HcClientRegistry::observe_and_publish_cell_state_from`] — the
    /// supervisor's own publication path, contradiction gate included — and
    /// probed when that classification owes a probe now. All of a pass's
    /// probes are in flight together; the per-role
    /// [`crate::services::cell_probe::ProbeGate`] keeps a probe the supervisor
    /// already has in flight from becoming two.
    pub(crate) async fn run_pass_at(
        &self,
        cells: &[ObservedCell],
        src: &dyn crate::services::cell_membership::RunningCellSource,
        cache: &crate::services::cell_membership::MembershipCache,
        at: crate::services::cell_membership::ObservedAt,
        conductor: &dyn PresenceConductor,
    ) -> PresencePass {
        self.passes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut owed = Vec::new();
        for cell in cells {
            let present = cell
                .cell_id
                .as_ref()
                .and_then(|id| cache.cell_running_at(id, at))
                == Some(true);
            if !present {
                self.release(cell.role);
                continue;
            }
            if !crate::conductor_bridge_health::role_is_not_running(cell.role) {
                continue;
            }
            let tick = HcClientRegistry::observe_and_publish_cell_state_from(
                cell.role,
                src,
                cell.cell_id.as_ref(),
                cache,
                at,
            )
            .await;
            if tick.state.probe_is_owed_now() && self.spend(cell.role) {
                owed.push(cell);
            }
        }
        if !owed.is_empty() {
            info!(
                roles = ?owed.iter().map(|c| c.role).collect::<Vec<_>>(),
                "cells present again on this conductor — probing every not-running role in ONE \
                 pass rather than on each role's next tick; a call that returns is still the \
                 only recovery"
            );
        }
        futures_util::future::join_all(
            owed.iter()
                .map(|cell| conductor.probe_cell(cell.role, cell.tended_by)),
        )
        .await;
        PresencePass {
            probed: owed.iter().map(|cell| cell.role).collect(),
        }
    }
}

/// The ONE process-wide presence seam. Tests build their own.
pub(crate) fn presence_recovery() -> &'static PresenceRecovery {
    static SEAM: std::sync::OnceLock<PresenceRecovery> = std::sync::OnceLock::new();
    SEAM.get_or_init(PresenceRecovery::new)
}

/// The presence seam's driver: wake on a landed reading, a re-mint, or (while
/// the watch is open) the watch cadence — then, if watching and some observed
/// role is not running, read; then run one pass.
///
/// No timer of its own outside the watch: with the watch shut it sleeps until
/// a reading lands or a re-mint opens it. The reads it takes itself go through
/// [`crate::services::cell_membership::MembershipCache::refresh_if_older_than_at`]
/// at the seam's cadence, so however often it wakes it asks the conductor at
/// most once per cadence — the cache's claim is the bound, not this loop.
pub(crate) async fn drive_presence_recovery(
    seam: &PresenceRecovery,
    cache: &crate::services::cell_membership::MembershipCache,
    conductor: &dyn PresenceConductor,
    mut shutdown: tokio::sync::broadcast::Receiver<()>,
) {
    loop {
        let watching = seam.watching_at(std::time::Instant::now());
        tokio::select! {
            _ = cache.read_landed().notified() => {}
            _ = seam.reopened.notified() => {}
            _ = tokio::time::sleep(seam.cadence), if watching => {}
            _ = shutdown.recv() => return,
        }
        let Some(src) = conductor.membership_source() else {
            continue;
        };
        let cells = conductor.observed_cells();
        let at = crate::services::cell_membership::ObservedAt::now();
        if seam.watching_at(at.mono)
            && cells
                .iter()
                .any(|cell| crate::conductor_bridge_health::role_is_not_running(cell.role))
        {
            cache
                .refresh_if_older_than_at(src.as_ref(), at, seam.cadence)
                .await;
        }
        seam.run_pass_at(
            &cells,
            src.as_ref(),
            cache,
            crate::services::cell_membership::ObservedAt::now(),
            conductor,
        )
        .await;
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
    }

    impl FakeConductor {
        fn new(cells: Vec<ObservedCell>, admin: Arc<FakeAdmin>, serves: bool) -> Self {
            Self {
                cells,
                admin,
                serves: AtomicBool::new(serves),
                probes: Mutex::new(Vec::new()),
            }
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

        async fn probe_cell(&self, role: &'static str, tended_by: &'static str) {
            self.probes
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push((role, tended_by));
            if self.serves.load(Ordering::SeqCst) {
                record_role_success(role);
            }
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
                admin.as_ref(),
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
                    admin.as_ref(),
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
}
