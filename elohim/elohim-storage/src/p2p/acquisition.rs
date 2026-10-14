//! Acquisition reconcile stream state (spec §4) — the sibling of the
//! replication stream. Per-pin GapTrackers over the DECLARED desired set;
//! all state Category C (in-memory, recomputed on restart from active pins
//! × local inventory). Wire vocabulary is the unified set (spec §4.3):
//! {total, fetched, pending, failed, caughtUp}, plus the closed `state`
//! derivation (`PullState`, story 4.1 of the serving-edge campaign) that lets
//! a reader tell idle-by-design apart from dead without re-deriving it from
//! caughtUp.

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use ts_rs::TS;

use super::reconcile_rails::GapTracker;
use elohim_views::acquisition::EprPullStatusView;

/// FLOOR on the per-item retry budget, not the budget itself.
///
/// `drain_acquisition_queue` rotates each retry to a different connected peer,
/// so the budget is really "how many providers do we probe before calling an
/// item unsatisfiable". Three retries on a 6-peer fabric probes at most half of
/// it — and then RETIRES the pin as if the bytes were proven unavailable, which
/// is a false negative, not a bounded one. The live budget is
/// [`retry_budget_for_peers`]: `max(MIN_RETRIES, connected peers)`. Retirement
/// must mean actually-unsatisfiable.
const MIN_RETRIES: u32 = 3;

/// Default cadence (seconds) of the acquisition AND provide reconcile ticks in
/// `P2PNode::run`. Before this constant the tick was a hardcoded
/// `Duration::from_secs(60)` with no operator lever, so a chapter-11 a2o
/// scenario that can only WAIT for the budget to exhaust spent ~5 minutes on a
/// 3-peer loopback mesh. The mesh's dev-tier pacing profile (`hc-mesh.sh`)
/// now declares `ACQUISITION_RECONCILE_SECS` the same way it declares
/// `PROJECTION_RECONCILE_SECS`; prod keeps this default.
pub const DEFAULT_RECONCILE_SECS: u64 = 60;

/// The reconcile tick duration for a configured cadence. `None` and `Some(0)`
/// both fall back to [`DEFAULT_RECONCILE_SECS`] — tokio's ticker panics on a
/// zero period, and "reconcile never" is not a cadence this loop offers.
pub fn reconcile_cadence(secs: Option<u64>) -> std::time::Duration {
    std::time::Duration::from_secs(match secs {
        Some(0) | None => DEFAULT_RECONCILE_SECS,
        Some(n) => n,
    })
}

/// The per-item retry budget for a fabric of `connected_peers` peers: enough
/// attempts that the rotation can reach EVERY connected provider at least once,
/// never fewer than [`MIN_RETRIES`].
pub fn retry_budget_for_peers(connected_peers: usize) -> u32 {
    MIN_RETRIES.max(u32::try_from(connected_peers).unwrap_or(u32::MAX))
}

/// Device-paced: deliberately below replication's 50 (R-E — the acquisition
/// stream serves a person's wants, not node policy; it must not starve the
/// node-level stream).
pub const MAX_ACQUISITION_INFLIGHT: usize = 25;

/// Longest the FIRST acquisition drain after boot waits for the iroh peer
/// book to learn a peer. Measured 2026-08-29 (dual household mesh, warm
/// recovery jessica<-matthew): the 5 s dispatch cadence drained all 11 pulls
/// in the first seconds while the book learned its first peer at +8..10 s —
/// so the transport selector only ever saw single-plane peers and libp2p
/// carried every pull with no decision to make (backlog
/// `pull-leg-drains-before-iroh-book-warms`). Bounded, never indefinite: a
/// node whose book never warms drains on this deadline and pays it once.
///
/// NOT A TUNABLE. Measured 2026-08-29 (household mesh, warm recovery): this
/// expired at 10.0 s and the first peer was learned at +28 s, because the book
/// fills from the transport-manifest gossip round, which lands every 30 s. A
/// wall-clock hold waits on a cadence-delivered event: any value under one
/// round expires before the round can land, and any value over it is "wait
/// for a round" with extra steps and a 30 s tax on every boot. The release is
/// therefore an EVENT, not this deadline: the doorway bootstrap's boot seed
/// (`p2p_iroh::doorway_bootstrap`, one GET of `/p2p/manifests` at T0) fills
/// the book from the survivors' signed manifests before the first tick can
/// expire. This constant is the bounded floor underneath that — the boot with
/// no doorway, or an unreachable one — never the lever.
pub const FIRST_DRAIN_HOLD: std::time::Duration = std::time::Duration::from_secs(10);

/// Why the first drain was released — one per boot, counted in
/// `elohim_acquisition_first_drain_total{outcome}` (C8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstDrainRelease {
    /// The iroh book holds at least one peer: dual-plane decisions are possible.
    BookWarm,
    /// No iroh leg is running — there is no second plane to wait for.
    NoIrohLeg,
    /// [`FIRST_DRAIN_HOLD`] elapsed with the book still empty.
    Expired,
}

impl FirstDrainRelease {
    pub const ALL: [FirstDrainRelease; 3] = [
        FirstDrainRelease::BookWarm,
        FirstDrainRelease::NoIrohLeg,
        FirstDrainRelease::Expired,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FirstDrainRelease::BookWarm => "book_warm",
            FirstDrainRelease::NoIrohLeg => "no_iroh_leg",
            FirstDrainRelease::Expired => "expired",
        }
    }
}

/// The first-drain gate's answer for one dispatch tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstDrain {
    /// Keep the queue for the next tick: the iroh book is still empty and the
    /// hold has time left. Counted as `outcome="held"` per tick.
    Hold,
    /// Drain now, and never consult this gate again this process.
    Release(FirstDrainRelease),
}

/// Decide whether the first acquisition drain may run. `iroh_book_empty` is
/// `None` when no iroh leg exists (release at once), `Some(true)` while the
/// book has learned nobody. Pure: the loop supplies `since_boot` and `hold`.
pub fn first_drain(
    iroh_book_empty: Option<bool>,
    since_boot: std::time::Duration,
    hold: std::time::Duration,
) -> FirstDrain {
    match iroh_book_empty {
        None => FirstDrain::Release(FirstDrainRelease::NoIrohLeg),
        Some(false) => FirstDrain::Release(FirstDrainRelease::BookWarm),
        Some(true) if since_boot >= hold => FirstDrain::Release(FirstDrainRelease::Expired),
        Some(true) => FirstDrain::Hold,
    }
}

#[cfg(test)]
mod first_drain_tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn holds_while_the_book_is_empty_and_time_remains() {
        assert_eq!(
            first_drain(Some(true), Duration::from_secs(0), FIRST_DRAIN_HOLD),
            FirstDrain::Hold
        );
        assert_eq!(
            first_drain(Some(true), Duration::from_secs(9), FIRST_DRAIN_HOLD),
            FirstDrain::Hold
        );
    }

    #[test]
    fn releases_the_moment_the_book_warms() {
        assert_eq!(
            first_drain(Some(false), Duration::from_secs(1), FIRST_DRAIN_HOLD),
            FirstDrain::Release(FirstDrainRelease::BookWarm)
        );
    }

    #[test]
    fn releases_on_the_deadline_never_indefinitely() {
        assert_eq!(
            first_drain(Some(true), FIRST_DRAIN_HOLD, FIRST_DRAIN_HOLD),
            FirstDrain::Release(FirstDrainRelease::Expired)
        );
    }

    #[test]
    fn releases_at_once_without_an_iroh_leg() {
        assert_eq!(
            first_drain(None, Duration::ZERO, FIRST_DRAIN_HOLD),
            FirstDrain::Release(FirstDrainRelease::NoIrohLeg)
        );
    }

    #[test]
    fn every_release_reason_has_a_distinct_label() {
        let labels: std::collections::BTreeSet<_> =
            FirstDrainRelease::ALL.iter().map(|r| r.label()).collect();
        assert_eq!(labels.len(), FirstDrainRelease::ALL.len());
    }
}

/// Pin status for a pin whose every wanted item exhausted its retry budget
/// against every connected provider.
///
/// NOT a new column: `acquisition_pins.status` is TEXT and
/// [`crate::views::PinView`]`::status` is a `String`, so this value is
/// additive on the wire. It DOES need a migration, though: `status` carries a
/// CHECK constraint enumerating the admitted values, and the original
/// migration (`2026-06-07-000000_acquisition_pins/up.sql`) did not include
/// `'retired'` — writing it failed with a CheckViolation at the SQLite engine
/// until `2026-07-31-200000_acquisition_pins_admit_retired` widened the CHECK.
/// `updated_at` (already stamped by `set_pin_status`) is the retirement clock.
///
/// Distinct from `removed`, which is a PERSON's revocation. `retired` is the
/// SUBSTRATE saying "no peer can supply these bytes right now" — a pin the
/// person still wants, held back so it stops pinning `pull.caughtUp` false
/// forever (alpha-A: 73 total / 3 fetched / 70 failed, caughtUp false for 12h+).
/// It is re-admitted the moment a peer advertises the content again, or after
/// its hold ([`pin_hold`]) — [`PIN_READMIT_COOLDOWN`] doubled per consecutive
/// retirement, capped at [`PIN_READMIT_MAX_HOLD`] (6 h, 12 h, then 24 h).
pub const PIN_STATUS_RETIRED: &str = "retired";

// The hold and its ladder live beside the row they are
// stored on (`db::acquisition_pins`); re-exported so this module stays the one
// place a reader of the acquisition stream looks.
pub use crate::db::acquisition_pins::{
    pin_hold, pin_readmit_at, PIN_READMIT_COOLDOWN, PIN_READMIT_MAX_HOLD,
};

use crate::db::acquisition_pins::{HoldPass, NamedPass, ReadmitCause};
use crate::metrics::{PinBackoffReset, PinHoldSkip, PinReadmitCause};

type Now = chrono::DateTime<chrono::Utc>;

/// Republish `elohim_acquisition_pins_retired` from the DB (the durable truth),
/// never from an incremented counter — a counter drifts across restarts and
/// concurrent flips; a recount cannot.
pub fn publish_retired_pin_gauge(conn: &mut diesel::SqliteConnection) {
    if let Ok(n) = crate::db::acquisition_pins::count_pins_by_status(conn, PIN_STATUS_RETIRED) {
        crate::metrics::set_acquisition_pins_retired(u64::try_from(n).unwrap_or(0));
    }
}

/// The per-tick pass over the retired set, shared by the libp2p node and the
/// pure-iroh pull core (`db::acquisition_pins::retired_hold_pass`): settle pins
/// whose content has become local and re-admit EVERY pin that is due (its own
/// hold ended, or a read shortened it) — as before the hold ladder, there is
/// no per-tick bound — and fail a pin with no readable retirement clock closed
/// onto the first rung.
///
/// Every count below is of rows the pass committed; the pass is one
/// transaction, so an error means nothing changed and nothing is counted.
///
/// bounded-work: one indexed read of the due rows (none at rest), one chunked
/// presence query over them, one write per due row.
pub fn run_hold_pass(conn: &mut diesel::SqliteConnection, now: Now) -> HoldPass {
    let pass = match crate::db::acquisition_pins::retired_hold_pass(
        conn,
        // Slice 1: all pinned content lives in the lamad app.
        &crate::db::AppContext::default_lamad(),
        now,
    ) {
        Ok(pass) => pass,
        // A failed pass leaves every retired pin exactly where it was — never
        // invisible.
        Err(e) => {
            tracing::warn!(error = %e, "acquisition: retired-pin hold pass failed");
            return HoldPass::default();
        }
    };
    for pin in &pass.settled {
        tracing::info!(
            target: "elohim_storage::acquisition",
            pin_id = pin.id,
            head_ref = %pin.head_ref,
            consecutive_retirements = pin.consecutive_retirements,
            "acquisition pin SETTLED — its content became local while it was retired; \
             active again, backoff reset"
        );
    }
    for (pin, cause) in &pass.readmitted {
        tracing::info!(
            target: "elohim_storage::acquisition",
            pin_id = pin.id,
            head_ref = %pin.head_ref,
            consecutive_retirements = pin.consecutive_retirements,
            cause = readmit_cause(*cause).label(),
            demanded_while_held = pin.held_demand_count,
            "acquisition pin RE-ADMITTED for a re-probe — retirement is a hold, never a delete"
        );
    }
    for pin in &pass.clocks_repaired {
        tracing::warn!(
            target: "elohim_storage::acquisition",
            pin_id = pin.id,
            head_ref = %pin.head_ref,
            retired_at = ?pin.retired_at,
            consecutive_retirements = pin.consecutive_retirements,
            "acquisition pin is retired with no readable retirement time — not re-admitted; \
             holding on the first rung from now"
        );
    }
    let count = |cause| pass.readmitted.iter().filter(|(_, c)| *c == cause).count() as u64;
    let (by_hold, by_demand, by_read) = (
        count(ReadmitCause::Hold),
        count(ReadmitCause::Demand),
        count(ReadmitCause::Read),
    );
    let settled = pass.settled.len() as u64;
    let reprobed = by_hold + by_demand + by_read;
    crate::metrics::inc_acquisition_pin_retirements("readmitted", reprobed + settled);
    crate::metrics::inc_acquisition_pin_readmissions(PinReadmitCause::Hold, by_hold);
    crate::metrics::inc_acquisition_pin_readmissions(PinReadmitCause::Demand, by_demand);
    crate::metrics::inc_acquisition_pin_readmissions(PinReadmitCause::Read, by_read);
    crate::metrics::inc_acquisition_pin_readmissions(PinReadmitCause::Fetched, settled);
    crate::metrics::inc_acquisition_pin_backoff_resets(
        PinBackoffReset::Fetched,
        pass.settled
            .iter()
            .filter(|p| p.consecutive_retirements > 0)
            .count() as u64,
    );
    crate::metrics::inc_acquisition_pin_hold_skips(
        PinHoldSkip::HoldNotElapsed,
        (pass.held - pass.clocks_repaired.len()) as u64,
    );
    crate::metrics::inc_acquisition_pin_hold_skips(
        PinHoldSkip::ClockUnreadable,
        pass.clocks_repaired.len() as u64,
    );
    if reprobed + settled > 0 {
        publish_retired_pin_gauge(conn);
    }
    pass
}

fn readmit_cause(cause: ReadmitCause) -> PinReadmitCause {
    match cause {
        ReadmitCause::Hold => PinReadmitCause::Hold,
        ReadmitCause::Demand => PinReadmitCause::Demand,
        ReadmitCause::Read => PinReadmitCause::Read,
    }
}

/// Primary re-admission arm, shared by both transports: a peer's content
/// inventory naming a RETIRED pin's `head_ref` is fresh evidence that the bytes
/// exist somewhere, so the pin goes straight back to `active` with its backoff
/// ladder reset and a new round (a fresh retry budget on the next reconcile).
/// A named pin that is already active keeps its round and loses any retained
/// retirement count.
pub fn readmit_pins_named_by_inventory(
    conn: &mut diesel::SqliteConnection,
    remote_ids: &[String],
    now: Now,
) -> NamedPass {
    let advertised: HashSet<&str> = remote_ids.iter().map(String::as_str).collect();
    let pass = match crate::db::acquisition_pins::inventory_named_pins(conn, &advertised, now) {
        Ok(pass) => pass,
        // A failed pass would silently strand the pin in `retired` — never
        // invisible.
        Err(e) => {
            tracing::warn!(error = %e, "acquisition: inventory-driven re-admission failed");
            return NamedPass::default();
        }
    };
    for pin in &pass.readmitted {
        tracing::info!(
            target: "elohim_storage::acquisition",
            pin_id = pin.id,
            head_ref = %pin.head_ref,
            consecutive_retirements = pin.consecutive_retirements,
            "acquisition pin RE-ADMITTED — a peer advertised its content again; backoff reset"
        );
    }
    let readmitted = pass.readmitted.len() as u64;
    let resets = pass
        .readmitted
        .iter()
        .chain(&pass.reset_active)
        .filter(|p| p.consecutive_retirements > 0)
        .count() as u64;
    if readmitted > 0 {
        crate::metrics::inc_acquisition_pin_retirements("readmitted", readmitted);
        crate::metrics::inc_acquisition_pin_readmissions(PinReadmitCause::Inventory, readmitted);
        publish_retired_pin_gauge(conn);
    }
    crate::metrics::inc_acquisition_pin_backoff_resets(PinBackoffReset::Inventory, resets);
    pass
}

/// A peer's content inventory arrived — the one handler both transports call
/// with the advertised ids: re-admit any retired pin it names
/// ([`readmit_pins_named_by_inventory`]), then hand the ids to the replication
/// stream. Returns the replication gaps `discover` found.
///
/// Inventory is metadata, not bytes — this re-opens the ASK, it never marks
/// anything fetched.
pub async fn inventory_received(
    pool: Option<&crate::db::DbPool>,
    replication: &super::replication::ReplicationState,
    remote_ids: Vec<String>,
) -> Vec<String> {
    if !remote_ids.is_empty() {
        if let Some(mut conn) = pool.and_then(|p| p.get().ok()) {
            readmit_pins_named_by_inventory(&mut conn, &remote_ids, chrono::Utc::now());
        }
    }
    replication.discover(remote_ids).await
}

/// Flip `exhausted` pins (from [`AcquisitionState::exhausted_pins`], each with
/// the round it exhausted in) to [`PIN_STATUS_RETIRED`], one rung up the
/// backoff ladder each, and republish the retired gauge. A pin re-activated
/// since that exhaustion was measured is in a later round and is left alone.
/// Returns how many were retired.
pub fn retire_exhausted_pins(
    conn: &mut diesel::SqliteConnection,
    exhausted: &[(i32, i32)],
    now: Now,
) -> usize {
    let retired = match crate::db::acquisition_pins::retire_pins(conn, exhausted, now) {
        Ok(retired) => retired,
        // A rejected status write (e.g. a CHECK-constraint violation) means
        // no pin left `active` — the exact failure mode that made retirement
        // inert until the 2026-07-31-200000_acquisition_pins_admit_retired
        // migration. That must never be invisible again.
        Err(e) => {
            tracing::warn!(error = %e, "acquisition: pin retirement failed");
            Vec::new()
        }
    };
    for &(pin_id, consecutive_retirements) in &retired {
        crate::metrics::inc_acquisition_pin_backoff_step(
            crate::db::acquisition_pins::pin_hold_label(consecutive_retirements),
        );
        tracing::info!(
            target: "elohim_storage::acquisition",
            pin_id,
            consecutive_retirements,
            hold_hours = pin_hold(consecutive_retirements).num_hours(),
            "acquisition pin RETIRED — no connected peer could supply its bytes; \
             held back until a peer advertises it again or its hold elapses"
        );
    }
    crate::metrics::inc_acquisition_pin_retirements("exhausted", retired.len() as u64);
    // Materialise the sibling series even when nothing revived this cycle, so
    // "no pin has ever been re-admitted" is distinguishable from "the emitter
    // never ran".
    crate::metrics::inc_acquisition_pin_retirements("readmitted", 0);
    publish_retired_pin_gauge(conn);
    retired.len()
}

/// Record on the row that an active pin's bytes are local, resetting its
/// backoff ladder. `pins` is the tick's active census and `local_has` its
/// local-presence set, both already in hand — no extra read, and no write once
/// every fetched pin is settled.
pub fn settle_fetched_pins(
    conn: &mut diesel::SqliteConnection,
    pins: &[crate::db::models::AcquisitionPin],
    local_has: &HashSet<String>,
    now: Now,
) {
    let ids = crate::db::acquisition_pins::pins_needing_fetched_settle(pins, local_has);
    if ids.is_empty() {
        return;
    }
    match crate::db::acquisition_pins::settle_fetched_pins(conn, &ids, now) {
        // Counted from the rows the write touched, as they stood before it.
        Ok(settled) => crate::metrics::inc_acquisition_pin_backoff_resets(
            PinBackoffReset::Fetched,
            settled
                .iter()
                .filter(|p| p.consecutive_retirements > 0)
                .count() as u64,
        ),
        Err(e) => tracing::warn!(error = %e, "acquisition: fetched-pin settle failed"),
    }
}

/// Whether a transport retires a pin whose retry budget is spent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetirePolicy {
    /// Retire on a spent budget of [`retry_budget_for_peers`] — what the
    /// libp2p node has always done. This is BOUNDED PROBING, not proof that
    /// every connected peer was asked: the drain's rotation advances once per
    /// drain, so a want's retries can land on the same peer (two peers, two
    /// stable batches of 25). That is why the hold ladder stops at 24 h.
    EveryPeerProbed,
    /// The transport has never retired (pure-iroh). Exhausted pins are left
    /// active — held open, not retired on unproven coverage.
    CoverageUnproven,
}

/// What one [`reconcile_tick`] did.
#[derive(Debug, Default)]
pub struct ReconcileTick {
    /// No reconcile had completed before this one (first pass since boot).
    pub first_pass: bool,
    pub active_pins: usize,
    pub desired_items: usize,
    pub local_items: usize,
    /// Newly-wanted content ids, pin-priority order — the caller queues these.
    pub to_dispatch: Vec<String>,
    /// Retired pins put back to `active` this tick (re-probe or settled).
    pub readmitted: usize,
    /// Pins retired at the end of this tick.
    pub retired: usize,
}

/// One acquisition reconcile tick (spec §4.2) — the whole of it, for both
/// transports: hold pass over the retired set, active-pin census, local
/// presence, fetched settlement, tracker reconcile, retirement. Pure local
/// computation — no network here (R-H); the caller owns the dispatch queue.
///
/// `connected_peers` sizes the retry budget ([`retry_budget_for_peers`]).
/// `Err` is the outcome to record when the tick stopped early.
pub async fn reconcile_tick(
    pool: &crate::db::DbPool,
    state: &AcquisitionState,
    connected_peers: usize,
    retire: RetirePolicy,
    now: Now,
) -> Result<ReconcileTick, crate::metrics::AcquisitionReconcileOutcome> {
    use crate::metrics::AcquisitionReconcileOutcome;

    let first_pass = state.rollup_if_initialized().await.is_none();
    let mut conn = pool.get().map_err(|e| {
        tracing::warn!(
            target: "elohim_storage::acquisition",
            error = %e,
            first_pass,
            "acquisition reconcile stopped before pin census: DB connection unavailable"
        );
        AcquisitionReconcileOutcome::DbPoolUnavailable
    })?;

    // (0) The retired set. Retirement is a HOLD, not a delete: a pin whose
    // content turned up some other way is settled, and a pin whose hold has
    // ended is re-admitted for a re-probe — every due pin, in this pass. The
    // primary re-admission arm is inventory-driven (`inventory_received`).
    let hold = run_hold_pass(&mut conn, now);

    let pins = crate::db::acquisition_pins::list_active_pins(&mut conn).map_err(|e| {
        tracing::warn!(
            target: "elohim_storage::acquisition",
            error = %e,
            first_pass,
            "acquisition reconcile stopped: active-pin census failed"
        );
        AcquisitionReconcileOutcome::PinLoadFailed
    })?;
    let active_pins = pins.len();
    let pin_wants: Vec<(i32, i32, Vec<String>)> = pins
        .iter()
        .filter(|p| p.kind == "item")
        .map(|p| (p.id, p.round, vec![p.head_ref.clone()]))
        .collect();
    let want_ids: Vec<String> = pin_wants
        .iter()
        .flat_map(|(_, _, ids)| ids.iter().cloned())
        .collect();
    // Slice 1: all pinned content lives in the lamad app — revisit the
    // app-scope here if cross-app pins land.
    let app_ctx = crate::db::AppContext::default_lamad();
    let local_has = crate::db::content_diesel::content_ids_present(&mut conn, &app_ctx, &want_ids)
        .map_err(|e| {
            tracing::warn!(
                target: "elohim_storage::acquisition",
                error = %e,
                first_pass,
                active_pins,
                "acquisition reconcile stopped: local-presence query failed"
            );
            AcquisitionReconcileOutcome::PresenceQueryFailed
        })?;
    // A pin whose bytes are local has its answer: reset its backoff ladder on
    // the row. Writes nothing once every fetched pin is settled.
    settle_fetched_pins(&mut conn, &pins, &local_has, now);
    // DB work is done; release the connection before the reconcile await so it
    // isn't held across the acquisition write-lock.
    drop(conn);

    // Peer-breadth budget: the dispatch rotation walks a different provider
    // per retry, so the budget IS the number of providers probed before an
    // item is called unsatisfiable.
    let to_dispatch = state
        .reconcile_rounds(
            pin_wants,
            &local_has,
            retry_budget_for_peers(connected_peers),
        )
        .await;

    // Retire pins the fabric cannot satisfy. Held BACK, not deleted: the pin
    // row survives (a person still wants it). Without this an exhausted pin's
    // tracker keeps `fetched < total` forever and `pull.caughtUp` can never go
    // true (alpha-A: 73 total / 3 fetched / 70 failed, caughtUp false for 12h+).
    let exhausted = match retire {
        RetirePolicy::EveryPeerProbed => state.exhausted_pins().await,
        RetirePolicy::CoverageUnproven => Vec::new(),
    };
    let retired = match pool.get() {
        Ok(mut conn) => retire_exhausted_pins(&mut conn, &exhausted, now),
        Err(_) => 0,
    };

    Ok(ReconcileTick {
        first_pass,
        active_pins,
        desired_items: want_ids.len(),
        local_items: local_has.len(),
        to_dispatch,
        readmitted: hold.readmitted.len() + hold.settled.len(),
        retired,
    })
}

/// Closed states for `/p2p/status.pull.state` (story 4.1 of the serving-edge
/// campaign). Derived from the SAME `total`/`fetched` pair `caught_up` uses,
/// in the ONE place [`PullState::from_counts`] — so the two fields can never
/// disagree. Lets a CI reader (`substrate-verify.sh` `cmd_projection`)
/// distinguish "idle by design" (nobody pinned content on this node) from
/// "dead" without guessing from `caughtUp:false` alone (backlog
/// `ci-substrate-projection-pull-stream-dark`).
///
/// A permanently-failed item (one that exhausted its retry budget — see
/// [`AcquisitionState::exhausted_pin_ids`]) sits inside `failed` without ever
/// moving `fetched`, so it reads as `Active`, never `CaughtUp` (spec R-A:
/// never false-complete). There is deliberately no fourth "stalled"/"failed"
/// state here — that finer distinction belongs to `exhausted_pin_ids` and the
/// per-pin surfaces, not this three-state rollup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum PullState {
    /// `total == 0`: a completed reconcile observed an empty desired set.
    /// NOT a synonym for "uninitialized" — that stays `None` on the outer
    /// `Option<PullStatusInfo>`, never this variant.
    #[default]
    Idle,
    /// `total > 0 && fetched < total`: still fetching (or holding a
    /// permanently-failed item — see the type doc above).
    Active,
    /// `total > 0 && fetched == total`: byte-arrival complete.
    CaughtUp,
}

impl PullState {
    /// The single derivation point for `state` — computed from the same
    /// `total`/`fetched` pair `caught_up` uses (see `rollup_inner`), so the
    /// two fields can never read differently for the same rollup.
    pub fn from_counts(total: i32, fetched: i32) -> Self {
        if total == 0 {
            PullState::Idle
        } else if fetched >= total {
            PullState::CaughtUp
        } else {
            PullState::Active
        }
    }
}

/// Pull-queue rollup. None on the wire means "cannot compute" = keep waiting
/// (the wait-for-drain tri-state contract, spec §4.3) — never caught-up.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PullStatusInfo {
    pub total: i32,
    pub fetched: i32,
    pub pending: i32,
    pub failed: i32,
    pub caught_up: bool,
    pub state: PullState,
}

/// Per-pin progress, served on GET /api/v1/pins (own node only).
#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PinPullStatus {
    pub pin_id: i32,
    pub total: i32,
    pub fetched: i32,
    pub pending: i32,
    pub failed: i32,
    pub caught_up: bool,
    // The fields below were added after the first release of this view. A
    // reader of an older node gets none of them, so they are optional on the
    // wire (views CONVENTIONS: additive fields never become required).
    /// Fetch attempts the pin's most-tried want has failed in this round.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub attempts_used: Option<u32>,
    /// Attempts a want gets before the pin is retired
    /// ([`retry_budget_for_peers`]); `attemptsUsed == retryBudget` with
    /// `pending == 0` is a pin the next reconcile retires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub retry_budget: Option<u32>,
}

#[derive(Debug, Default)]
struct AcquisitionInner {
    /// True after at least one local pin/presence reconcile completed.
    ///
    /// Category C operational state: reconstructed from `acquisition_pins` ×
    /// local content presence after every process restart. Until this flips,
    /// `/p2p/status.pull` must stay null — zero is an observation, not a default.
    initialized: bool,
    /// pin row id → tracker over that pin's resolved item set
    trackers: HashMap<i32, GapTracker>,
    /// pin row id → resolved desired-set size (Slice 1: 1 for item pins)
    totals: HashMap<i32, usize>,
    /// content id → pin ids wanting it (completion fan-out)
    wanted_by: HashMap<String, Vec<i32>>,
    /// pin row id → the row's `round` its tracker was built for. A pin that
    /// comes back with a different round was re-activated (cooldown, demand,
    /// inventory, explicit re-pin): its tracker — and the retry budget spent in
    /// it — belongs to the round before and is discarded.
    rounds: HashMap<i32, i32>,
}

/// True if any tracker still wants this id.
fn wanted(inner: &AcquisitionInner, content_id: &str) -> bool {
    inner.wanted_by.get(content_id).is_some_and(|pins| {
        pins.iter()
            .any(|p| inner.trackers.get(p).is_some_and(|t| t.wants(content_id)))
    })
}

#[derive(Debug, Clone, Default)]
pub struct AcquisitionState {
    inner: Arc<RwLock<AcquisitionInner>>,
}

impl AcquisitionState {
    pub fn new() -> Self {
        Self::default()
    }

    /// [`Self::reconcile_rounds`] for callers that hold no pin rows (every pin
    /// in round 0, so a tracker is only ever discarded by its pin leaving the
    /// active set). The live loop goes through [`reconcile_tick`], which
    /// passes each row's real round.
    pub async fn reconcile(
        &self,
        pin_wants: Vec<(i32, Vec<String>)>,
        local_has: &std::collections::HashSet<String>,
        max_retries: u32,
    ) -> Vec<String> {
        let pin_wants = pin_wants
            .into_iter()
            .map(|(pin_id, want_ids)| (pin_id, 0, want_ids))
            .collect();
        self.reconcile_rounds(pin_wants, local_has, max_retries)
            .await
    }

    /// Reconcile the declared wants (spec §4.2 step 1–2). `pin_wants` maps
    /// each active pin — with the `round` its row is in — to its resolved item
    /// ids; `local_has` is the set of ids already present in the local
    /// projection. Returns newly-pending content ids in pin-priority order
    /// (caller passes pins pre-sorted).
    /// `max_retries` is the live per-item budget (see [`retry_budget_for_peers`])
    /// — applied to every tracker each cycle, so a fabric that grows re-admits
    /// items that were exhausted against a smaller one.
    ///
    /// A pin whose round differs from the one its tracker was built for starts
    /// with a fresh tracker. That is what makes a re-activation real when no
    /// reconcile ran between the retirement and it (an inventory naming the
    /// head in the same minute): without it the pin would come back holding
    /// its exhausted tracker, dispatch nothing, and be retired again at once.
    pub async fn reconcile_rounds(
        &self,
        pin_wants: Vec<(i32, i32, Vec<String>)>,
        local_has: &std::collections::HashSet<String>,
        max_retries: u32,
    ) -> Vec<String> {
        let mut inner = self.inner.write().await;
        // Drop trackers for pins no longer active (un-pinned → queue drains)
        let live: HashSet<i32> = pin_wants.iter().map(|(id, _, _)| *id).collect();
        inner.trackers.retain(|id, _| live.contains(id));
        inner.totals.retain(|id, _| live.contains(id));
        inner.rounds.retain(|id, _| live.contains(id));
        inner.wanted_by.retain(|_, pins| {
            pins.retain(|p| live.contains(p));
            !pins.is_empty()
        });

        let mut to_dispatch = Vec::new();
        for (pin_id, round, want_ids) in pin_wants {
            if inner
                .rounds
                .insert(pin_id, round)
                .is_some_and(|was| was != round)
            {
                inner.trackers.remove(&pin_id);
            }
            inner.totals.insert(pin_id, want_ids.len());
            for id in &want_ids {
                let entry = inner.wanted_by.entry(id.clone()).or_default();
                if !entry.contains(&pin_id) {
                    entry.push(pin_id);
                }
            }
            let tracker = inner
                .trackers
                .entry(pin_id)
                .or_insert_with(|| GapTracker::new(max_retries));
            // Re-apply every cycle: the fabric's peer count moves, and a tracker
            // that outlives the cycle must not keep probing against a stale
            // (smaller) budget.
            tracker.set_max_retries(max_retries);
            // TODO(cluster-pins): clones local_has once per pin — fine for
            // Slice-1 item pins (tiny sets); revisit with a shared Arc or a
            // borrow variant when cluster closures bring large desired sets.
            tracker.set_local_ids(local_has.clone());
            let want_ids_for_local = want_ids.clone();
            let gaps = tracker.reconcile_desired(want_ids);
            // Already-local wants are satisfied (bytes present), not pending —
            // count them as fetched so the rollup can reach caught_up on a
            // content-holder node. See GapTracker::mark_local_wants_satisfied.
            tracker.mark_local_wants_satisfied(&want_ids_for_local);
            to_dispatch.extend(gaps);
        }
        // C4 honest absence: only a completed DB-backed reconcile earns an
        // observed pull rollup. Early returns before this method leave the state
        // uninitialized, so callers publish null rather than fabricated zeroes.
        inner.initialized = true;
        to_dispatch
    }

    /// Byte-arrival done-signal (R-A): called from the ContentData completion
    /// path AFTER bulk_create_content succeeds — never on inventory receipt.
    pub async fn mark_completed(&self, content_id: &str) {
        let mut inner = self.inner.write().await;
        if let Some(pin_ids) = inner.wanted_by.get(content_id).cloned() {
            for pin_id in pin_ids {
                if let Some(t) = inner.trackers.get_mut(&pin_id) {
                    t.mark_completed(content_id);
                    // keeps the tracker's own bit fresh for symmetry with replication;
                    // the acquisition rollup recomputes independently (see rollup()).
                    t.update_caught_up();
                }
            }
        }
    }

    pub async fn mark_failed(&self, content_id: &str) {
        // No update_caught_up here (unlike mark_completed): a failed item
        // removed from pending does not satisfy the drain contract — only a
        // byte-arrival completion earns caught_up.
        let mut inner = self.inner.write().await;
        if let Some(pin_ids) = inner.wanted_by.get(content_id).cloned() {
            for pin_id in pin_ids {
                if let Some(t) = inner.trackers.get_mut(&pin_id) {
                    t.mark_failed(content_id);
                }
            }
        }
    }

    /// True if any tracker still wants this id (dispatch filter).
    pub async fn wants(&self, content_id: &str) -> bool {
        wanted(&*self.inner.read().await, content_id)
    }

    /// Pins whose desired set is UNSATISFIABLE against the current fabric: every
    /// remaining want spent its retry budget (`exhausted > 0`), nothing is still
    /// in flight (`pending == 0`), and the pin never completed (`completed <
    /// total`).
    ///
    /// The caller flips these to [`PIN_STATUS_RETIRED`]. All three conditions are
    /// load-bearing:
    /// - `exhausted > 0` — a budget was actually SPENT. With
    ///   [`retry_budget_for_peers`] that means every connected provider was
    ///   asked, so this is a measured absence, not an unprobed one.
    /// - `pending == 0` — nothing is still being fetched; retiring an in-flight
    ///   pin would abandon bytes that are arriving.
    /// - `completed < total` — a satisfied pin is caught-up, not exhausted.
    ///   (`total == 0`, the resolved-empty case, is excluded for the same reason
    ///   `rollup` refuses to call it caught_up: nothing was ever asked.)
    ///
    /// No eviction code is needed on the tracker side: once the pin leaves
    /// `list_active_pins`, the `trackers.retain` at the top of [`Self::reconcile`]
    /// drops it on the next cycle, which is what removes it from BOTH `total` and
    /// `fetched` in [`Self::rollup`].
    pub async fn exhausted_pin_ids(&self) -> Vec<i32> {
        self.exhausted_pins()
            .await
            .into_iter()
            .map(|(pin_id, _)| pin_id)
            .collect()
    }

    /// [`Self::exhausted_pin_ids`], each with the round its budget was spent
    /// in. Retirement names that round, so a snapshot taken before the pin was
    /// re-activated cannot retire it.
    pub async fn exhausted_pins(&self) -> Vec<(i32, i32)> {
        let inner = self.inner.read().await;
        let mut out: Vec<(i32, i32)> = inner
            .trackers
            .iter()
            .filter(|(pin_id, t)| {
                let c = t.counts();
                let total = *inner.totals.get(pin_id).unwrap_or(&0);
                total > 0 && c.exhausted > 0 && c.pending == 0 && c.completed < total
            })
            .map(|(pin_id, _)| (*pin_id, *inner.rounds.get(pin_id).unwrap_or(&0)))
            .collect();
        out.sort_unstable();
        out
    }

    fn rollup_inner(inner: &AcquisitionInner) -> PullStatusInfo {
        let mut s = PullStatusInfo::default();
        for (pin_id, t) in &inner.trackers {
            let c = t.counts();
            s.total += *inner.totals.get(pin_id).unwrap_or(&0) as i32;
            s.fetched += c.completed as i32;
            s.pending += c.pending as i32;
            s.failed += c.failed as i32;
        }
        // caught_up is recomputed here from byte-arrival (fetched == total), NOT read from
        // the per-tracker GapTracker.caught_up bit (which the replication stream maintains
        // on a pending-based formula). The two are intentionally independent.
        // Caught-up means BYTE-ARRIVAL complete: every wanted item fetched
        // (spec R-A — never false-complete). A failed item (fetched < total
        // with pending transiently 0 before re-queue) must NOT report caught_up.
        // total == 0 (resolved-empty) is likewise not caught_up.
        s.caught_up = s.total > 0 && s.fetched == s.total;
        // Same total/fetched pair as the line above, in the ONE derivation
        // point — caught_up and state can never disagree (PullState doc).
        s.state = PullState::from_counts(s.total, s.fetched);
        s
    }

    pub async fn rollup(&self) -> PullStatusInfo {
        let inner = self.inner.read().await;
        Self::rollup_inner(&inner)
    }

    /// Honest status boundary for `/p2p/status.pull`.
    ///
    /// `None` has exactly one provenance: no local acquisition reconcile has
    /// completed since process start. Once initialized, `Some(total=0)` is a
    /// positive observation that the active desired set is empty, never a
    /// default standing in for unmeasured state.
    pub async fn rollup_if_initialized(&self) -> Option<PullStatusInfo> {
        let inner = self.inner.read().await;
        inner.initialized.then(|| Self::rollup_inner(&inner))
    }

    pub async fn per_pin(&self) -> Vec<PinPullStatus> {
        let inner = self.inner.read().await;
        let mut out: Vec<PinPullStatus> = inner
            .trackers
            .iter()
            .map(|(pin_id, t)| {
                let c = t.counts();
                let total = *inner.totals.get(pin_id).unwrap_or(&0) as i32;
                PinPullStatus {
                    pin_id: *pin_id,
                    total,
                    fetched: c.completed as i32,
                    pending: c.pending as i32,
                    failed: c.failed as i32,
                    caught_up: total > 0 && (c.completed as i32) == total,
                    attempts_used: Some(t.attempts_used()),
                    retry_budget: Some(t.max_retries()),
                }
            })
            .collect();
        out.sort_by_key(|p| p.pin_id);
        out
    }

    /// Per-EPR rollup grouped by `head_ref`. `pin_heads` maps each live pin id
    /// to its `head_ref` (supplied by the HTTP handler from the DB — the
    /// AcquisitionState never learns head_ref itself). Returns `None` when no
    /// active tracker belongs to `epr_id` (handler maps that to 404).
    ///
    /// Shared content is counted ONCE: ids are deduped across all pins of the
    /// EPR before counting. `total == 0` (resolved-empty) surfaces as None
    /// total / None caughtUp — never false-complete (spec R-A, §4.3).
    pub async fn per_epr(
        &self,
        epr_id: &str,
        pin_heads: &HashMap<i32, String>,
    ) -> Option<EprPullStatusView> {
        let inner = self.inner.read().await;
        // Which live pins belong to this EPR?
        let group: Vec<i32> = inner
            .trackers
            .keys()
            .copied()
            .filter(|pid| pin_heads.get(pid).map(|h| h == epr_id).unwrap_or(false))
            .collect();
        if group.is_empty() {
            return None;
        }
        // Union the desired set and per-status id sets across the group's pins.
        let mut desired: HashSet<String> = HashSet::new();
        let mut completed: HashSet<String> = HashSet::new();
        let mut pending: HashSet<String> = HashSet::new();
        let mut failed: HashSet<String> = HashSet::new();
        for pid in &group {
            let t = match inner.trackers.get(pid) {
                Some(t) => t,
                None => continue,
            };
            for id in t.completed_ids() {
                desired.insert(id.clone());
                completed.insert(id);
            }
            for id in t.pending_ids() {
                desired.insert(id.clone());
                pending.insert(id);
            }
            for id in t.failed_ids() {
                desired.insert(id.clone());
                failed.insert(id);
            }
        }
        // Status precedence: a completed id is fetched even if another pin's
        // tracker still has it pending/failed. Subtract higher-precedence sets.
        pending.retain(|id| !completed.contains(id));
        failed.retain(|id| !completed.contains(id) && !pending.contains(id));

        let total_n = desired.len() as u64;
        let fetched = completed.len() as u64;
        // total == 0 resolved-empty → tri-state None (never caught up); matches
        // rollup()'s `s.total > 0` guard in this same file.
        let (total, caught_up) = if total_n == 0 {
            (None, None)
        } else {
            (Some(total_n), Some(fetched == total_n))
        };
        Some(EprPullStatusView {
            epr_id: epr_id.to_string(),
            total,
            fetched,
            pending: pending.len() as u64,
            failed: failed.len() as u64,
            caught_up,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Story 4.1 (serving-edge campaign): `state` is derived from the SAME
    /// total/fetched pair `caught_up` uses, in one function
    /// ([`PullState::from_counts`]) — pin the full truth table (0/0, n/0,
    /// n/k, n/n) so the two can never silently disagree.
    #[test]
    fn pull_state_truth_table_matches_caught_up() {
        let cases = [
            (0, 0, PullState::Idle),
            (5, 0, PullState::Active),
            (5, 3, PullState::Active),
            (5, 5, PullState::CaughtUp),
        ];
        for (total, fetched, expected) in cases {
            assert_eq!(
                PullState::from_counts(total, fetched),
                expected,
                "total={total} fetched={fetched}"
            );
            let caught_up = total > 0 && fetched == total;
            assert_eq!(
                expected == PullState::CaughtUp,
                caught_up,
                "state must agree with caught_up for total={total} fetched={fetched}"
            );
        }
    }

    #[tokio::test]
    async fn reconcile_diffs_wants_and_rolls_up() {
        let acq = AcquisitionState::new();
        let local: HashSet<String> = ["have-1".to_string()].into_iter().collect();
        let dispatch = acq
            .reconcile(
                vec![
                    (1, vec!["have-1".into(), "need-1".into()]),
                    (2, vec!["need-2".into()]),
                ],
                &local,
                MIN_RETRIES,
            )
            .await;
        assert_eq!(dispatch.len(), 2);
        let r = acq.rollup().await;
        // have-1 is already local → now counted as fetched (satisfied), so
        // fetched=1; need-1/need-2 are still pending. Not caught_up (3 != 1).
        assert_eq!((r.total, r.fetched, r.pending), (3, 1, 2));
        assert!(!r.caught_up);
        assert_eq!(
            r.state,
            PullState::Active,
            "total>0, fetched<total is active"
        );
    }

    #[tokio::test]
    async fn all_pinned_items_already_local_is_caught_up() {
        // The content-holder case (matthew holds the seeded corpus): a pin
        // whose every item is already present locally must report caught_up.
        // Today total counts already-local items but fetched does not, so the
        // pull stream is pull=false forever and Verify Projection Sync fails.
        let acq = AcquisitionState::new();
        let local: HashSet<String> = ["have-1".to_string()].into_iter().collect();
        let dispatch = acq
            .reconcile(vec![(1, vec!["have-1".into()])], &local, MIN_RETRIES)
            .await;
        assert!(
            dispatch.is_empty(),
            "already-local item needs no fetch dispatch"
        );
        let r = acq.rollup().await;
        assert_eq!((r.total, r.fetched, r.pending), (1, 1, 0));
        assert!(
            r.caught_up,
            "a pin whose every item is already local must be caught_up"
        );
        assert_eq!(r.state, PullState::CaughtUp);
    }

    #[tokio::test]
    async fn byte_arrival_completes_every_wanting_pin() {
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        acq.reconcile(
            vec![(1, vec!["shared".into()]), (2, vec!["shared".into()])],
            &local,
            MIN_RETRIES,
        )
        .await;
        acq.mark_completed("shared").await;
        let pins = acq.per_pin().await;
        assert!(pins.iter().all(|p| p.caught_up && p.fetched == 1));
    }

    #[tokio::test]
    async fn resolved_empty_desired_set_is_not_caught_up() {
        // A pin whose desired set resolves to zero items (e.g. an empty
        // cluster closure) must surface as resolved-empty, NOT caught_up
        // (spec §4.3/§10 — never silently false-complete).
        let acq = AcquisitionState::new();
        let local = std::collections::HashSet::new();
        acq.reconcile(vec![(1, vec![])], &local, MIN_RETRIES).await;
        let r = acq.rollup().await;
        assert_eq!(r.total, 0);
        assert!(!r.caught_up, "zero-item desired set must not be caught_up");
        assert_eq!(
            r.state,
            PullState::Idle,
            "total==0 is idle by design, never a dead-stream guess"
        );
        let pins = acq.per_pin().await;
        assert_eq!(pins.len(), 1);
        assert!(!pins[0].caught_up);
    }

    #[tokio::test]
    async fn pull_is_null_until_a_reconcile_observes_the_desired_set() {
        let acq = AcquisitionState::new();
        assert!(
            acq.rollup_if_initialized().await.is_none(),
            "an unmeasured pull set must be null, never fabricated zeroes"
        );

        acq.reconcile(vec![], &HashSet::new(), MIN_RETRIES).await;
        let observed = acq
            .rollup_if_initialized()
            .await
            .expect("a completed empty reconcile is an observed status");
        assert_eq!(
            (
                observed.total,
                observed.fetched,
                observed.pending,
                observed.failed
            ),
            (0, 0, 0, 0)
        );
        assert!(
            !observed.caught_up,
            "an observed empty desired set is still not byte-arrival caught up"
        );
        assert_eq!(observed.state, PullState::Idle);
    }

    #[tokio::test]
    async fn failed_item_is_not_caught_up_despite_empty_pending() {
        // R-A: an item that failed its fetch (removed from pending until the
        // next reconcile re-queues it) leaves fetched < total — the pull must
        // NOT report caught_up even though pending is transiently 0.
        let acq = AcquisitionState::new();
        let local = std::collections::HashSet::new();
        acq.reconcile(vec![(1, vec!["need".into()])], &local, MIN_RETRIES)
            .await;
        acq.mark_failed("need").await;
        let r = acq.rollup().await;
        assert_eq!((r.total, r.fetched, r.pending, r.failed), (1, 0, 0, 1));
        assert!(
            !r.caught_up,
            "a failed (unfetched) item must not be caught_up (R-A)"
        );
        assert_eq!(
            r.state,
            PullState::Active,
            "a permanently-failed item sits in `failed` without moving `fetched` — \
             it reads as active, never caughtUp, and there is no separate \
             stalled/failed state in this 3-state rollup"
        );
        let pins = acq.per_pin().await;
        assert!(!pins[0].caught_up);
    }

    #[tokio::test]
    async fn unpinned_pin_drains_out_of_state() {
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        acq.reconcile(
            vec![(1, vec!["a".into()]), (2, vec!["b".into()])],
            &local,
            MIN_RETRIES,
        )
        .await;
        acq.reconcile(vec![(1, vec!["a".into()])], &local, MIN_RETRIES)
            .await;
        let pins = acq.per_pin().await;
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].pin_id, 1);
        assert!(!acq.wants("b").await);
    }

    /// Spend a pin's whole retry budget the way the live loop does: fail, then
    /// re-reconcile (retry-on-NEXT-cycle, R-E) until `enqueue_missing` refuses.
    async fn exhaust(acq: &AcquisitionState, pin_wants: Vec<(i32, Vec<String>)>, budget: u32) {
        let local = HashSet::new();
        for _ in 0..budget {
            acq.reconcile(pin_wants.clone(), &local, budget).await;
            for (_, ids) in &pin_wants {
                for id in ids {
                    acq.mark_failed(id).await;
                }
            }
        }
        acq.reconcile(pin_wants, &local, budget).await;
    }

    #[tokio::test]
    async fn exhausted_pin_retires_and_rollup_reaches_caught_up() {
        // The live alpha-A shape: 73 total / 3 fetched / 70 failed, and
        // `pull.caughtUp` false for 12h+. Nothing in the runtime ever flipped a
        // pin's status (the only status-flip caller was the HTTP DELETE route),
        // so an unsatisfiable want held the queue open forever.
        let acq = AcquisitionState::new();
        let local: HashSet<String> = ["have-1".to_string()].into_iter().collect();

        // pin 1 is satisfiable (already local); pin 2 is not.
        acq.reconcile(vec![(1, vec!["have-1".into()])], &local, MIN_RETRIES)
            .await;
        for _ in 0..MIN_RETRIES {
            acq.reconcile(
                vec![(1, vec!["have-1".into()]), (2, vec!["gone".into()])],
                &local,
                MIN_RETRIES,
            )
            .await;
            acq.mark_failed("gone").await;
        }
        acq.reconcile(
            vec![(1, vec!["have-1".into()]), (2, vec!["gone".into()])],
            &local,
            MIN_RETRIES,
        )
        .await;

        let r = acq.rollup().await;
        assert!(
            !r.caught_up,
            "before retirement the unsatisfiable want holds the queue open"
        );
        assert_eq!(
            r.state,
            PullState::Active,
            "an exhausted-but-not-yet-retired pin still reads as active, not a distinct stalled state"
        );

        assert_eq!(
            acq.exhausted_pin_ids().await,
            vec![2],
            "only the unsatisfiable pin is exhausted — the satisfied one is caught-up"
        );

        // The caller flips pin 2 to `retired`, so it leaves `list_active_pins`
        // and the NEXT reconcile's `trackers.retain` drops it — from BOTH total
        // and fetched. No separate eviction code exists, or is needed.
        acq.reconcile(vec![(1, vec!["have-1".into()])], &local, MIN_RETRIES)
            .await;
        let r = acq.rollup().await;
        assert_eq!((r.total, r.fetched), (1, 1));
        assert!(
            r.caught_up,
            "with the unsatisfiable pin retired the pull queue can finally drain"
        );
        assert_eq!(r.state, PullState::CaughtUp);
    }

    #[tokio::test]
    async fn retired_pin_readmitted_on_new_inventory() {
        // Retirement is a HOLD, not a delete. When the pin comes back as active
        // (a peer advertised its head_ref again), the tracker is rebuilt and the
        // item re-enters the pull queue — caught_up correctly goes false again,
        // because the bytes still have not arrived.
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        exhaust(&acq, vec![(1, vec!["want".into()])], MIN_RETRIES).await;
        assert_eq!(acq.exhausted_pin_ids().await, vec![1]);

        // Retired: absent from `list_active_pins` → tracker dropped, rollup empty.
        acq.reconcile(vec![], &local, MIN_RETRIES).await;
        assert!(acq.exhausted_pin_ids().await.is_empty());

        // Re-admitted: back in `list_active_pins` with a CLEAN budget.
        let dispatch = acq
            .reconcile(vec![(1, vec!["want".into()])], &local, MIN_RETRIES)
            .await;
        assert_eq!(
            dispatch,
            vec!["want".to_string()],
            "a re-admitted pin re-enters the pull queue"
        );
        let r = acq.rollup().await;
        assert_eq!((r.total, r.fetched, r.pending), (1, 0, 1));
        assert!(
            !r.caught_up,
            "re-admission must not false-complete — the bytes still have not arrived (R-A)"
        );
    }

    #[tokio::test]
    async fn all_pins_retired_is_not_caught_up() {
        // The failure mode retirement could EASILY introduce: retire every pin,
        // the tracker map empties, `total` goes 0 — and a naive `pending == 0`
        // rollup would call that caught-up. `rollup`'s `s.total > 0` guard is
        // what stops "we gave up on everything" from reading as "we have
        // everything"; this test is that guard's regression fence.
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        exhaust(
            &acq,
            vec![(1, vec!["a".into()]), (2, vec!["b".into()])],
            MIN_RETRIES,
        )
        .await;
        assert_eq!(acq.exhausted_pin_ids().await, vec![1, 2]);

        acq.reconcile(vec![], &local, MIN_RETRIES).await;
        let r = acq.rollup().await;
        assert_eq!((r.total, r.fetched), (0, 0));
        assert!(
            !r.caught_up,
            "every pin retired is NOT caught up — an empty desired set never false-completes"
        );
        assert!(acq.per_pin().await.is_empty());
    }

    #[test]
    fn the_hold_is_timed_from_retired_at() {
        use chrono::TimeZone;
        let retired = chrono::Utc
            .with_ymd_and_hms(2026, 10, 13, 12, 0, 0)
            .unwrap();
        let at = Some("2026-10-13T12:00:00Z");
        let hours = |n| retired + chrono::Duration::hours(n);
        assert_eq!(pin_readmit_at(at, 1), Some(hours(6)));
        assert_eq!(pin_readmit_at(at, 2), Some(hours(12)));
        assert_eq!(pin_readmit_at(at, 3), Some(hours(24)));
        assert_eq!(pin_readmit_at(at, 6), Some(hours(24)));
        // An offset is normalised to UTC.
        assert_eq!(
            pin_readmit_at(Some("2026-10-13T14:00:00+02:00"), 1),
            Some(hours(6))
        );
        // No readable clock is NOT "due": the hold pass repairs the row and
        // holds it, it never re-admits on this answer.
        assert_eq!(pin_readmit_at(None, 1), None);
        assert_eq!(pin_readmit_at(Some("not-a-timestamp"), 1), None);
        assert_eq!(pin_readmit_at(Some("2026-07-31 12:00:00"), 6), None);
    }

    #[test]
    fn hold_doubles_from_six_hours_and_caps_at_a_day() {
        let hours = |n| pin_hold(n).num_hours();
        assert_eq!((1..=4).map(hours).collect::<Vec<_>>(), vec![6, 12, 24, 24]);
        assert_eq!(hours(i32::MAX), 24, "no overflow at the far end");
        // A retired row with no backoff state serves the first rung.
        assert_eq!(hours(0), 6);
        assert_eq!(hours(-3), 6);
        assert_eq!(PIN_READMIT_MAX_HOLD, chrono::Duration::hours(24));
    }

    #[tokio::test]
    async fn per_pin_reports_attempts_used_against_the_budget() {
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        let wants = || vec![(1, vec!["often".to_string(), "once".to_string()])];
        acq.reconcile(wants(), &local, 4).await;
        let p = &acq.per_pin().await[0];
        assert_eq!((p.attempts_used, p.retry_budget), (Some(0), Some(4)));

        acq.mark_failed("often").await;
        acq.mark_failed("once").await;
        acq.reconcile(wants(), &local, 4).await;
        acq.mark_failed("often").await;
        let p = &acq.per_pin().await[0];
        // The MOST-tried want: that is the one closest to retiring the pin.
        assert_eq!((p.attempts_used, p.retry_budget), (Some(2), Some(4)));
        assert!(!p.caught_up);
    }

    // ── The reconcile tick, end to end ──────────────────────────────────────
    //
    // These drive `reconcile_tick` — the function both transports' loops call —
    // against a real migrated database, with a fake fetcher: every want the
    // tick dispatches is reported failed, as if no peer had the bytes.

    use crate::db::acquisition_pins as pin_db;
    use crate::db::models::{AcquisitionPin, NewAcquisitionPin};
    use chrono::{Duration, TimeZone};

    const PEERS: usize = 3;

    fn t0() -> Now {
        chrono::Utc
            .with_ymd_and_hms(2026, 10, 13, 12, 0, 0)
            .unwrap()
    }

    fn pin(pool: &crate::db::DbPool, head: &str) -> AcquisitionPin {
        pin_db::upsert_pin(
            &mut pool.get().expect("conn"),
            NewAcquisitionPin {
                agent_pub_key: "local-device".into(),
                head_ref: head.into(),
                kind: "item".into(),
                closure_rule_json: None,
                priority: 1,
            },
        )
        .expect("upsert")
    }

    fn row(pool: &crate::db::DbPool, id: i32) -> AcquisitionPin {
        pin_db::get_pin_by_id(&mut pool.get().expect("conn"), id)
            .expect("query")
            .expect("row")
    }

    fn retire_now(pool: &crate::db::DbPool, id: i32, at: Now) {
        let round = row(pool, id).round;
        pin_db::retire_pin(&mut pool.get().expect("conn"), id, round, at)
            .expect("retire")
            .expect("retired");
    }

    fn make_local(pool: &crate::db::DbPool, id: &str) {
        let result = crate::db::content_diesel::bulk_create_content(
            &mut pool.get().expect("conn"),
            &crate::db::AppContext::default_lamad(),
            vec![crate::db::content_diesel::CreateContentInput {
                id: id.to_string(),
                title: id.to_string(),
                description: None,
                content_type: "concept".to_string(),
                content_format: "markdown".to_string(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: "public".to_string(),
                created_by: None,
                tags: vec![],
                dht_anchor_hash: None,
                content_body: Some("body".to_string()),
            }],
        )
        .expect("bulk_create_content");
        assert_eq!(result.inserted, 1);
    }

    async fn tick(pool: &crate::db::DbPool, acq: &AcquisitionState, now: Now) -> ReconcileTick {
        reconcile_tick(pool, acq, PEERS, RetirePolicy::EveryPeerProbed, now)
            .await
            .expect("tick")
    }

    /// The fake fetcher: no peer has the bytes.
    async fn fail_dispatched(acq: &AcquisitionState, tick: &ReconcileTick) {
        for id in &tick.to_dispatch {
            acq.mark_failed(id).await;
        }
    }

    /// From a tick that dispatched, fail every fetch and tick again until a
    /// tick retires something. Returns the fetch rounds that took.
    async fn fail_until_retired(
        pool: &crate::db::DbPool,
        acq: &AcquisitionState,
        now: Now,
        mut t: ReconcileTick,
    ) -> u32 {
        let mut rounds = 0;
        while t.retired == 0 {
            assert!(
                !t.to_dispatch.is_empty(),
                "a live pin with budget left is dispatched every round"
            );
            fail_dispatched(acq, &t).await;
            rounds += 1;
            assert!(rounds <= 16, "retirement never came");
            t = tick(pool, acq, now).await;
        }
        rounds
    }

    #[tokio::test]
    async fn the_tick_retires_holds_readmits_dispatches_and_retires_one_rung_up() {
        let pool = crate::test_util::test_pool();
        let acq = AcquisitionState::new();
        let p = pin(&pool, "e2e-nobody-holds-this");

        // Round one: dispatched, every peer asked, retired onto the first rung.
        let first = tick(&pool, &acq, t0()).await;
        assert_eq!(first.to_dispatch, vec!["e2e-nobody-holds-this".to_string()]);
        assert_eq!((first.active_pins, first.retired), (1, 0));
        let rounds = fail_until_retired(&pool, &acq, t0(), first).await;
        assert_eq!(
            rounds,
            retry_budget_for_peers(PEERS),
            "one attempt per peer"
        );
        let retired = row(&pool, p.id);
        assert_eq!(retired.status, "retired");
        assert_eq!(retired.consecutive_retirements, 1);
        assert_eq!(retired.retired_at.as_deref(), Some("2026-10-13T12:00:00Z"));
        assert_eq!(retired.last_outcome.as_deref(), Some("exhausted"));

        // Held: 5 h 59 m in, nothing is re-admitted and nothing dispatched,
        // and the pin no longer counts against the pull queue.
        let held = tick(&pool, &acq, t0() + Duration::minutes(359)).await;
        assert_eq!((held.readmitted, held.active_pins), (0, 0));
        assert!(held.to_dispatch.is_empty());
        assert_eq!(acq.rollup().await.total, 0);
        assert_eq!(row(&pool, p.id).status, "retired");

        // 6 h 1 m: re-admitted and dispatched again in the same tick, with a
        // whole budget.
        let second_at = t0() + Duration::minutes(361);
        let readmit = tick(&pool, &acq, second_at).await;
        assert_eq!(readmit.readmitted, 1);
        assert_eq!(
            readmit.to_dispatch,
            vec!["e2e-nobody-holds-this".to_string()]
        );
        let r = acq.rollup().await;
        assert_eq!((r.total, r.fetched), (1, 0));
        assert!(!r.caught_up, "a re-probe is not caught up (unchanged)");
        let reprobing = row(&pool, p.id);
        assert_eq!(reprobing.status, "active");
        assert_eq!(reprobing.round, retired.round + 1);

        // Still nobody has it: retired again, one rung UP.
        let rounds = fail_until_retired(&pool, &acq, second_at, readmit).await;
        assert_eq!(rounds, retry_budget_for_peers(PEERS));
        let again = row(&pool, p.id);
        assert_eq!(
            (again.status.as_str(), again.consecutive_retirements),
            ("retired", 2)
        );

        // The second rung is 12 h, not 6.
        let held = tick(&pool, &acq, second_at + Duration::minutes(719)).await;
        assert_eq!(held.readmitted, 0);
        let third = tick(&pool, &acq, second_at + Duration::minutes(721)).await;
        assert_eq!(third.readmitted, 1);
        assert_eq!(third.to_dispatch.len(), 1);
    }

    /// The ordering the earlier test avoided: an inventory names the head
    /// BEFORE any reconcile has run since the retirement, so the pin's
    /// exhausted tracker is still in memory when it comes back.
    #[tokio::test]
    async fn an_inventory_right_after_retirement_gets_a_fresh_budget_not_a_second_retirement() {
        let pool = crate::test_util::test_pool();
        let acq = AcquisitionState::new();
        let p = pin(&pool, "named-next");
        let first = tick(&pool, &acq, t0()).await;
        fail_until_retired(&pool, &acq, t0(), first).await;
        let retired = row(&pool, p.id);
        assert_eq!(retired.status, "retired");
        assert_eq!(
            acq.exhausted_pins().await,
            vec![(p.id, retired.round)],
            "the exhausted tracker is still there — no reconcile has dropped it"
        );

        // The inventory handler both transports call.
        let replication = crate::p2p::replication::ReplicationState::new();
        let gaps = inventory_received(
            Some(&pool),
            &replication,
            vec!["named-next".to_string(), "unpinned-elsewhere".to_string()],
        )
        .await;
        assert_eq!(gaps.len(), 2, "the ids still reach the replication stream");
        let named = row(&pool, p.id);
        assert_eq!(
            (named.status.as_str(), named.consecutive_retirements),
            ("active", 0)
        );
        assert_eq!(named.round, retired.round + 1);

        // The very next reconcile: dispatched with a fresh budget, not retired.
        let next = tick(&pool, &acq, t0() + Duration::minutes(1)).await;
        assert_eq!(next.to_dispatch, vec!["named-next".to_string()]);
        assert_eq!(next.retired, 0);
        assert_eq!(row(&pool, p.id).status, "active");
        assert_eq!(acq.per_pin().await[0].attempts_used, Some(0));

        // And it takes the whole budget again to retire it — onto the FIRST rung.
        let rounds = fail_until_retired(&pool, &acq, t0() + Duration::minutes(1), next).await;
        assert_eq!(rounds, retry_budget_for_peers(PEERS));
        assert_eq!(row(&pool, p.id).consecutive_retirements, 1);
    }

    /// A person re-pins a RETIRED pin before any reconcile has run since the
    /// retirement. The re-pin opens a new round: neither the exhaustion
    /// snapshot taken before it nor the tracker still in memory retires it.
    #[tokio::test]
    async fn a_repin_of_a_retired_pin_is_not_retired_by_the_exhaustion_that_preceded_it() {
        let pool = crate::test_util::test_pool();
        let acq = AcquisitionState::new();
        let p = pin(&pool, "repinned");
        let first = tick(&pool, &acq, t0()).await;
        fail_until_retired(&pool, &acq, t0(), first).await;
        assert_eq!(row(&pool, p.id).status, "retired");
        let stale = acq.exhausted_pins().await;
        assert_eq!(stale, vec![(p.id, p.round)]);

        pin(&pool, "repinned");
        let retired = retire_exhausted_pins(&mut pool.get().expect("conn"), &stale, t0());
        assert_eq!(
            retired, 0,
            "a stale snapshot cannot retire a re-activated pin"
        );

        let after = tick(&pool, &acq, t0() + Duration::minutes(1)).await;
        assert_eq!(after.retired, 0);
        assert_eq!(after.to_dispatch, vec!["repinned".to_string()]);
        let r = row(&pool, p.id);
        assert_eq!(
            (r.status.as_str(), r.consecutive_retirements),
            ("active", 1)
        );
        assert_eq!(r.round, p.round + 1);
    }

    /// A client that re-asserts an ACTIVE, unfillable pin on every tick does
    /// not hand it a fresh retry budget each time: it retires on schedule.
    #[tokio::test]
    async fn reasserting_an_active_pin_every_tick_does_not_stop_it_retiring() {
        let pool = crate::test_util::test_pool();
        let acq = AcquisitionState::new();
        let p = pin(&pool, "reasserted");
        let mut rounds = 0;
        let mut t = tick(&pool, &acq, t0()).await;
        while t.retired == 0 {
            fail_dispatched(&acq, &t).await;
            rounds += 1;
            assert!(rounds <= 16, "the re-assertion kept the pin from retiring");
            // The client POSTs the same pin again before every reconcile.
            assert_eq!(pin(&pool, "reasserted").round, p.round);
            t = tick(&pool, &acq, t0()).await;
        }
        assert_eq!(rounds, retry_budget_for_peers(PEERS));
        let r = row(&pool, p.id);
        assert_eq!(
            (r.status.as_str(), r.consecutive_retirements),
            ("retired", 1)
        );
    }

    /// Review round 4, blocker 2: 301 pins come due at once, and X — the
    /// highest-priority pin, read while held but not admitted — would have
    /// sorted last behind a 300-row bound. Every due pin is re-admitted in the
    /// same pass, and priority orders the dispatch, as before the ladder.
    #[tokio::test]
    async fn every_due_pin_is_readmitted_in_one_pass_and_priority_orders_the_dispatch() {
        let pool = crate::test_util::test_pool();
        let acq = AcquisitionState::new();
        for i in 0..300 {
            let p = pin(&pool, &format!("e2e-{i:03}"));
            retire_now(&pool, p.id, t0());
        }
        let x = pin_db::upsert_pin(
            &mut pool.get().expect("conn"),
            NewAcquisitionPin {
                agent_pub_key: "local-device".into(),
                head_ref: "x-highest-priority".into(),
                kind: "item".into(),
                closure_rule_json: None,
                priority: 9,
            },
        )
        .expect("upsert");
        retire_now(&pool, x.id, t0());
        // A window with no room: X's read is not admitted, only marked.
        let window_full = crate::services::demand_autopin::DemandAutoPin::with_limits(
            true,
            std::time::Duration::from_secs(300),
            0,
            2048,
        );
        assert!(window_full
            .record_demand(&mut pool.get().expect("conn"), "x-highest-priority")
            .expect("demand")
            .is_none());
        assert_eq!(row(&pool, x.id).read_since_retired, 1);

        let early = tick(&pool, &acq, t0() + Duration::minutes(359)).await;
        assert_eq!(early.readmitted, 0);
        let due = tick(&pool, &acq, t0() + Duration::minutes(361)).await;
        assert_eq!(due.readmitted, 301, "all of them, in one pass");
        assert_eq!(due.to_dispatch.len(), 301);
        assert_eq!(
            due.to_dispatch[0], "x-highest-priority",
            "and the highest-priority pin is dispatched first"
        );
        assert_eq!(acq.rollup().await.total, 301);
    }

    /// A read through the production demand writer, then the tick: a pin one
    /// minute into a 24 h hold is re-probed by the next tick and stays on its
    /// rung; a pin nobody read serves its hold.
    #[tokio::test]
    async fn a_read_miss_on_a_retired_pin_is_reprobed_by_the_next_tick() {
        let pool = crate::test_util::test_pool();
        let acq = AcquisitionState::new();
        let read = pin(&pool, "read-while-held");
        let unread = pin(&pool, "nobody-reads");
        // Three retirements each: the 24 h rung.
        for (i, at) in [0, 10, 30].into_iter().enumerate() {
            let at = t0() + Duration::hours(at);
            retire_now(&pool, read.id, at);
            retire_now(&pool, unread.id, at);
            if i < 2 {
                run_hold_pass(&mut pool.get().expect("conn"), at + Duration::hours(13));
            }
        }
        let retired_at = t0() + Duration::hours(30);
        assert_eq!(row(&pool, read.id).consecutive_retirements, 3);

        let quiet = tick(&pool, &acq, retired_at + Duration::minutes(1)).await;
        assert_eq!(quiet.readmitted, 0);

        let autopin = crate::services::demand_autopin::DemandAutoPin::new(
            true,
            std::time::Duration::from_secs(300),
        );
        let demanded = autopin
            .record_demand(&mut pool.get().expect("conn"), "read-while-held")
            .expect("demand");
        assert!(
            demanded.is_none(),
            "the read does not re-activate the pin itself"
        );
        assert_eq!(row(&pool, read.id).status, "retired");

        let next = tick(&pool, &acq, retired_at + Duration::minutes(2)).await;
        assert_eq!(next.readmitted, 1);
        assert_eq!(next.to_dispatch, vec!["read-while-held".to_string()]);
        assert_eq!(
            row(&pool, read.id).consecutive_retirements,
            3,
            "ladder not reset"
        );
        assert_eq!(
            row(&pool, unread.id).status,
            "retired",
            "unread: its whole 24 h"
        );

        // Unfillable still: the fourth rung, not the first.
        fail_until_retired(&pool, &acq, retired_at + Duration::minutes(2), next).await;
        assert_eq!(row(&pool, read.id).consecutive_retirements, 4);
    }

    /// Put a pin on the top rung without walking the ladder.
    fn set_rung(pool: &crate::db::DbPool, id: i32, rung: i32) {
        use diesel::connection::SimpleConnection;
        pool.get()
            .expect("conn")
            .batch_execute(&format!(
                "UPDATE acquisition_pins SET consecutive_retirements = {rung} WHERE id = {id}"
            ))
            .expect("set rung");
    }

    /// Review round 3, blocker 2. A head is read while its re-probe is active
    /// (the autopin's per-id throttle now holds it for five minutes), it then
    /// retires onto the top rung, and it is read again inside that window. The
    /// second read is one dev's autopin would NOT have admitted — so it does
    /// not re-admit the pin at once — but it must not leave the pin waiting out
    /// 24 h either: dev would have re-probed it by `retired_at + 6 h`.
    #[tokio::test]
    async fn a_throttled_read_after_retirement_is_reprobed_by_six_hours_as_on_dev() {
        let pool = crate::test_util::test_pool();
        let acq = AcquisitionState::new();
        let autopin = crate::services::demand_autopin::DemandAutoPin::new(
            true,
            std::time::Duration::from_secs(300),
        );
        let p = pin(&pool, "read-then-retired");
        let first = tick(&pool, &acq, t0()).await;

        // Read while active: admitted, the id enters the throttle window.
        let read = autopin
            .record_demand(&mut pool.get().expect("conn"), "read-then-retired")
            .expect("demand");
        assert!(read.is_some(), "an active pin already wants it");

        // It retires — onto the top rung.
        set_rung(&pool, p.id, 2);
        fail_until_retired(&pool, &acq, t0(), first).await;
        let retired = row(&pool, p.id);
        assert_eq!(
            (retired.status.as_str(), retired.consecutive_retirements),
            ("retired", 3)
        );
        assert_eq!(pin_hold(3), PIN_READMIT_MAX_HOLD);

        // Read again, seconds later: throttled.
        for _ in 0..3 {
            let read = autopin
                .record_demand(&mut pool.get().expect("conn"), "read-then-retired")
                .expect("demand");
            assert!(read.is_none());
        }
        let marked = row(&pool, p.id);
        assert_eq!(marked.status, "retired");
        assert_eq!(
            marked.held_demand_count, 0,
            "not admitted: no demand recorded"
        );
        assert_eq!(
            marked.read_since_retired, 1,
            "but the row knows it was read"
        );
        assert_eq!(marked.retired_at, retired.retired_at);
        assert_eq!(
            marked.updated_at, retired.updated_at,
            "one write, no clock moved"
        );

        // Not re-admitted early (dev would not have re-armed it either)…
        let early = tick(&pool, &acq, t0() + Duration::minutes(359)).await;
        assert_eq!(early.readmitted, 0);
        // …and not later than dev's flat 6 h.
        let due = tick(&pool, &acq, t0() + Duration::minutes(361)).await;
        assert_eq!(due.readmitted, 1);
        assert_eq!(due.to_dispatch, vec!["read-then-retired".to_string()]);
        let back = row(&pool, p.id);
        assert_eq!(
            (back.status.as_str(), back.consecutive_retirements),
            ("active", 3)
        );
        assert_eq!(back.read_since_retired, 0);
    }

    /// Review round 3, blocker 3. With the autopin's window of distinct ids
    /// full, a sweep of retired heads must not re-admit anything dev would not
    /// have: nothing before each pin's 6 h mark. And read-driven re-admission
    /// does not take the active set past the ceiling any more than on dev.
    #[tokio::test]
    async fn a_sweep_of_retired_heads_past_the_budgets_readmits_nothing_dev_would_not() {
        let pool = crate::test_util::test_pool();
        let acq = AcquisitionState::new();
        let heads: Vec<String> = (0..6).map(|i| format!("retired-{i}")).collect();
        let mut ids = Vec::new();
        for head in &heads {
            let p = pin(&pool, head);
            set_rung(&pool, p.id, 2);
            retire_now(&pool, p.id, t0());
            ids.push(p.id);
        }
        let active = |pool: &crate::db::DbPool| {
            pin_db::count_active_pins(&mut pool.get().expect("conn")).expect("count")
        };

        // A window of one distinct id, already spent.
        let window_full = crate::services::demand_autopin::DemandAutoPin::with_limits(
            true,
            std::time::Duration::from_secs(300),
            1,
            64,
        );
        assert!(window_full
            .record_demand(&mut pool.get().expect("conn"), "fills-the-window")
            .expect("demand")
            .is_some());
        assert_eq!(active(&pool), 1);

        // The sweep: four retired heads, read repeatedly.
        for _ in 0..3 {
            for head in &heads[..4] {
                assert!(window_full
                    .record_demand(&mut pool.get().expect("conn"), head)
                    .expect("demand")
                    .is_none());
            }
        }
        for id in &ids[..4] {
            let r = row(&pool, *id);
            assert_eq!((r.held_demand_count, r.read_since_retired), (0, 1));
        }
        // A new head past the window is still refused outright.
        assert!(window_full
            .record_demand(&mut pool.get().expect("conn"), "another-new-head")
            .expect("demand")
            .is_none());
        assert_eq!(
            pin_db::list_all_pins(&mut pool.get().expect("conn"))
                .expect("list")
                .len(),
            7
        );

        // The ceiling. A window with room, a ceiling of two active rows, one
        // already active: the first read of a retired head is admitted (dev
        // would re-activate it: 1 < 2); the second finds the ceiling reached
        // by the first — counted before it is re-admitted — and is not.
        let at_ceiling = crate::services::demand_autopin::DemandAutoPin::with_limits(
            true,
            std::time::Duration::from_secs(300),
            16,
            2,
        );
        for head in &heads[4..] {
            assert!(at_ceiling
                .record_demand(&mut pool.get().expect("conn"), head)
                .expect("demand")
                .is_none());
        }
        let (admitted, refused) = (row(&pool, ids[4]), row(&pool, ids[5]));
        assert_eq!(
            (admitted.held_demand_count, admitted.read_since_retired),
            (1, 0)
        );
        assert_eq!(
            (refused.held_demand_count, refused.read_since_retired),
            (0, 1)
        );

        // The next tick: only the admitted read is re-admitted. The active set
        // is at the ceiling, not over it.
        let next = tick(&pool, &acq, t0() + Duration::minutes(1)).await;
        assert_eq!(next.readmitted, 1);
        assert_eq!(row(&pool, ids[4]).status, "active");
        assert_eq!(active(&pool), 2);

        // Nothing else before the 6 h mark…
        let early = tick(&pool, &acq, t0() + Duration::minutes(359)).await;
        assert_eq!(early.readmitted, 0);
        // …and at it, the five marked pins — dev's cooldown, no sooner. (Dev
        // re-admitted every retired pin at 6 h whatever the ceiling.)
        let due = tick(&pool, &acq, t0() + Duration::minutes(361)).await;
        assert_eq!(due.readmitted, 5);
        for id in &ids {
            let r = row(&pool, *id);
            assert_eq!(
                (r.status.as_str(), r.consecutive_retirements),
                ("active", 3)
            );
        }
    }

    #[tokio::test]
    async fn content_that_arrives_some_other_way_settles_a_retired_pin_when_it_comes_due() {
        let pool = crate::test_util::test_pool();
        let acq = AcquisitionState::new();
        let p = pin(&pool, "fetched-in-the-foreground");
        let first = tick(&pool, &acq, t0()).await;
        fail_until_retired(&pool, &acq, t0(), first).await;
        assert_eq!(row(&pool, p.id).status, "retired");

        make_local(&pool, "fetched-in-the-foreground");

        // One minute into the hold: not due, not looked at.
        let early = tick(&pool, &acq, t0() + Duration::minutes(1)).await;
        assert_eq!(early.readmitted, 0);
        assert_eq!(row(&pool, p.id).status, "retired");

        // Due: settled, not re-probed.
        let t = tick(&pool, &acq, t0() + Duration::minutes(361)).await;
        assert_eq!(t.readmitted, 1);
        assert!(t.to_dispatch.is_empty(), "nothing to fetch — it is here");
        let r = row(&pool, p.id);
        assert_eq!(
            (r.status.as_str(), r.consecutive_retirements),
            ("active", 0)
        );
        assert_eq!(r.last_outcome.as_deref(), Some("fetched"));
        let rollup = acq.rollup().await;
        assert_eq!((rollup.total, rollup.fetched), (1, 1));
        assert!(rollup.caught_up);
    }

    /// A transport that has never retired (pure-iroh) still does not: the pin
    /// stays active with its budget spent.
    #[tokio::test]
    async fn a_transport_without_proven_peer_coverage_does_not_retire() {
        let pool = crate::test_util::test_pool();
        let acq = AcquisitionState::new();
        let p = pin(&pool, "asked-somebody-thrice");
        for _ in 0..=retry_budget_for_peers(PEERS) + 1 {
            let t = reconcile_tick(&pool, &acq, PEERS, RetirePolicy::CoverageUnproven, t0())
                .await
                .expect("tick");
            assert_eq!(t.retired, 0);
            fail_dispatched(&acq, &t).await;
        }
        assert_eq!(acq.exhausted_pin_ids().await, vec![p.id]);
        let r = row(&pool, p.id);
        assert_eq!(
            (r.status.as_str(), r.consecutive_retirements),
            ("active", 0)
        );
    }

    #[test]
    fn reconcile_cadence_guards_zero_and_absent() {
        use std::time::Duration;
        assert_eq!(
            reconcile_cadence(None),
            Duration::from_secs(DEFAULT_RECONCILE_SECS)
        );
        assert_eq!(
            reconcile_cadence(Some(0)),
            Duration::from_secs(DEFAULT_RECONCILE_SECS)
        );
        assert_eq!(reconcile_cadence(Some(10)), Duration::from_secs(10));
    }

    #[test]
    fn retry_budget_follows_the_fabric_never_below_the_floor() {
        // A 6-peer fabric must probe 6 providers before calling an item
        // unsatisfiable — retirement is only honest if every peer was asked.
        assert_eq!(retry_budget_for_peers(6), 6);
        assert_eq!(retry_budget_for_peers(25), 25);
        // Small/empty fabrics keep the floor (a 1-peer fabric still deserves
        // retries against transient failures).
        assert_eq!(retry_budget_for_peers(0), MIN_RETRIES);
        assert_eq!(retry_budget_for_peers(1), MIN_RETRIES);
        assert_eq!(retry_budget_for_peers(3), MIN_RETRIES);
    }

    #[tokio::test]
    async fn an_in_flight_pin_is_never_retired() {
        // `pending == 0` is load-bearing: retiring a pin whose bytes are still
        // arriving would abandon a fetch in progress.
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        acq.reconcile(vec![(1, vec!["a".into(), "b".into()])], &local, MIN_RETRIES)
            .await;
        // "a" exhausts; "b" stays pending.
        for _ in 0..MIN_RETRIES {
            acq.mark_failed("a").await;
            acq.reconcile(vec![(1, vec!["a".into(), "b".into()])], &local, MIN_RETRIES)
                .await;
        }
        assert!(
            acq.exhausted_pin_ids().await.is_empty(),
            "a pin with an item still in flight must not retire"
        );
    }

    #[tokio::test]
    async fn per_epr_groups_by_head_ref_counting_shared_content_once() {
        let acq = AcquisitionState::new();
        // pin 1 and pin 2 both belong to EPR "head-A" and both want "shared";
        // pin 1 additionally wants "only1", pin 2 additionally wants "only2".
        // pin 3 belongs to a different EPR "head-B".
        let local = HashSet::new();
        acq.reconcile(
            vec![
                (1, vec!["shared".into(), "only1".into()]),
                (2, vec!["shared".into(), "only2".into()]),
                (3, vec!["b1".into()]),
            ],
            &local,
            MIN_RETRIES,
        )
        .await;
        // byte-arrival completes the shared id (fans out to BOTH pins 1 & 2).
        acq.mark_completed("shared").await;

        let mut heads: HashMap<i32, String> = HashMap::new();
        heads.insert(1, "head-A".into());
        heads.insert(2, "head-A".into());
        heads.insert(3, "head-B".into());

        let a = acq.per_epr("head-A", &heads).await.expect("head-A present");
        // distinct desired ids for head-A = {shared, only1, only2} = 3, NOT 4.
        assert_eq!(a.total, Some(3), "shared id counted once across pins");
        // "shared" fetched once; "only1"/"only2" still pending.
        assert_eq!(a.fetched, 1);
        assert_eq!(a.pending, 2);
        assert_eq!(a.failed, 0);
        assert_eq!(a.caught_up, Some(false));
        assert_eq!(a.epr_id, "head-A");

        let b = acq.per_epr("head-B", &heads).await.expect("head-B present");
        assert_eq!(b.total, Some(1));
        assert_eq!(b.pending, 1);

        // Unknown EPR → None (the handler maps None → 404).
        assert!(acq.per_epr("head-Z", &heads).await.is_none());
    }

    #[tokio::test]
    async fn per_epr_all_shared_fetched_is_caught_up() {
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        acq.reconcile(
            vec![(1, vec!["x".into()]), (2, vec!["x".into()])],
            &local,
            MIN_RETRIES,
        )
        .await;
        acq.mark_completed("x").await;
        let mut heads: HashMap<i32, String> = HashMap::new();
        heads.insert(1, "head-A".into());
        heads.insert(2, "head-A".into());
        let a = acq.per_epr("head-A", &heads).await.unwrap();
        assert_eq!(a.total, Some(1)); // single distinct id across both pins
        assert_eq!(a.fetched, 1);
        assert_eq!(a.caught_up, Some(true));
    }

    #[tokio::test]
    async fn per_epr_resolved_empty_is_none_total_not_caught_up() {
        // A pin whose desired set resolves to zero items → tri-state None total
        // / None caughtUp (never false-complete), matching rollup()'s guard.
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        acq.reconcile(vec![(1, vec![])], &local, MIN_RETRIES).await;
        let mut heads: HashMap<i32, String> = HashMap::new();
        heads.insert(1, "head-A".into());
        let a = acq.per_epr("head-A", &heads).await.expect("head-A present");
        assert_eq!(a.total, None, "zero-item desired set → None total");
        assert_eq!(a.caught_up, None, "None total must never be caught_up");
        assert_eq!(a.fetched, 0);
    }

    #[tokio::test]
    async fn per_epr_failed_item_is_not_caught_up() {
        // R-A: a failed (unfetched) item leaves fetched < total — the per-EPR
        // rollup must not report caught_up even though pending is transiently 0.
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        acq.reconcile(vec![(1, vec!["need".into()])], &local, MIN_RETRIES)
            .await;
        acq.mark_failed("need").await;
        let mut heads: HashMap<i32, String> = HashMap::new();
        heads.insert(1, "head-A".into());
        let a = acq.per_epr("head-A", &heads).await.expect("head-A present");
        assert_eq!(a.total, Some(1));
        assert_eq!(a.fetched, 0);
        assert_eq!(a.failed, 1);
        assert_eq!(
            a.caught_up,
            Some(false),
            "failed item must not be caught_up"
        );
    }
}
