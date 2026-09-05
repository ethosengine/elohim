//! `LineageBridge` — the trailing bridge sweep (Holochain Evolution Epic,
//! Station 6; spec §5 bridge, §7 C3/C6).
//!
//! # What this is for
//!
//! A lineage window is a period in which ONE peer authors on v2 while its
//! neighbours are still authoring on v1. Task 7's apply vehicle crosses that
//! peer's OWN chain once, at the moment the window opens — and then stops. But
//! v1 does not stop. A neighbour keeps writing records the crossed peer can
//! see (they gossip to it as v1 agent activity) and cannot read from v2,
//! because v2 is a different DNA and holds none of them.
//!
//! This ticker closes that gap while the window is open: every
//! [`LINEAGE_SWEEP_SECS`] it asks the v2 cell to HELD-CARRY one page of each
//! neighbour's v1 records — `carry_from { source: Held(agent) }` — so a record
//! jessica authors on v1 during the window becomes readable on james's v2
//! within one sweep interval, held with jessica's own signature. James is a
//! COURIER: he authors the witness, never the content.
//!
//! # The three disciplines this module is built around
//!
//! **C3 liveness — one page per tick per neighbour, never a loop.**
//! `HcClient::call_zome` has no cancellation path, so the only honest bound is
//! a small batch the extern can always finish ([`HELD_PAGE_LIMIT`]). The apply
//! vehicle can afford `release_adoption::carry::fold_carry`'s
//! walk-to-the-end because it runs once, inside an apply that is allowed to
//! take minutes. A ticker cannot: a loop here would hold a tick hostage to a
//! chain of uncancellable calls. So the cursor advances one page per tick and
//! the sweep is a *trailing* process by design.
//!
//! **The held cursor is an ordinal into a list gossip can grow underneath.**
//! `export_held_records` walks the COURIER'S OWN integrated view of a
//! neighbour's chain — a subset, possibly gapped. When that view grows, every
//! ordinal past the insertion point shifts. So a `v1_digest` that changes
//! mid-walk restarts the walk at cursor 0 rather than trusting a stale
//! position.
//!
//! **But re-walking depends on Task 20's `already_carried`, which is in flight
//! in the zome and is NOT landed.** Entry-hash idempotency lives on the v2
//! side; until it lands, a caught-up sweep that re-walks a view re-creates
//! every entry and authors a duplicate witness EVERY TICK. So the bridge does
//! not assume it: a page whose receipt omits `already_carried` is a v2 that
//! cannot state its own idempotency, and the sweep HALTS that neighbour at the
//! moment it would have rewound rather than re-walking blind
//! ([`AgentSweep::halted`]). Forward progress up to the end of the local view
//! is kept — it is only the repeat that is refused. Do not read this module as
//! evidence that idempotency has landed.
//!
//! **Neighbours are enumerated from the READING cell.** `known_agents` is a v1
//! question — who is on the PREDECESSOR DHT — so it is asked of the app id the
//! window pins reads to, connected explicitly. Asking the role's supervised
//! client instead would be wrong the moment the bridge supervisor re-mints it
//! mid-window: that client resolves through `LineageRoles::app_id_for`, which
//! returns the AUTHORING app, and the sweep would enumerate v2's DHT and find
//! nobody to carry.
//!
//! **Storage never claims completeness.** A held page is not self-evidencing —
//! `carried == v1_total` on it says the courier carried everything IT HAD, not
//! everything the neighbour had. So this module reports what it observed
//! ([`AgentSweep`], projected onto the passport) and asserts nothing. Station
//! 6's a2o step compares james's sweep view of jessica against jessica's OWN
//! `export_records`; that cross-view check lives in the harness, and storage
//! never calls a neighbour's HTTP to manufacture it.
//!
//! # Cost when nothing is happening
//!
//! An idle tick makes NO conductor call. The very first thing
//! [`LineageBridge::tick`] does is read the in-process
//! [`crate::lineage_roles::LineageRoles`] snapshot; with no window open it
//! returns immediately. A lineage window is a rare, bounded state, so on a
//! normal peer this task costs one `BTreeMap` clone every 30 seconds forever.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::hc_client_registry::HcClientRegistry;
use crate::lineage_roles::LineageRoles;
use crate::services::release_adoption::carry::{
    self, call_carry_from, CarryInput, CarryReceipt, CarrySource, ExportResume, CARRY_ZOME,
};

/// How often the bridge sweeps, in seconds. Overridable with
/// `LINEAGE_SWEEP_SECS` for a mesh run that wants a tighter loop than the
/// deliverable's "within one sweep interval" wording implies.
pub const LINEAGE_SWEEP_SECS: u64 = 30;

/// Environment variable that overrides [`LINEAGE_SWEEP_SECS`].
pub const LINEAGE_SWEEP_SECS_ENV: &str = "LINEAGE_SWEEP_SECS";

/// Records ONE held page moves. Half the apply path's batch, deliberately:
/// this call runs on a repeating tick beside live traffic rather than inside a
/// one-shot apply, and the sweep catches up over ticks rather than in one.
pub const HELD_PAGE_LIMIT: u32 = 16;

/// The v1 extern that lists who is on the predecessor DHT.
pub const KNOWN_AGENTS_FN: &str = "known_agents";

/// What this peer has observed while sweeping ONE neighbour, for ONE role.
///
/// Every field is an OBSERVATION, never a claim: `total` and `digest` describe
/// the courier's own integrated view of that neighbour's chain, and
/// `observed_head` is the single number that reaches past it. Nothing here
/// says the neighbour's chain was carried whole, because a held sweep cannot
/// know that.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentSweep {
    /// Where the next tick resumes. `None` means "start at the beginning" —
    /// which is the state after a fresh start, after the walk reached the end
    /// of the courier's view, and after a digest change forced a restart. All
    /// three are the same instruction, so they are the same value.
    pub cursor: Option<u32>,
    /// The digest the last page reported. Compared against the next page's to
    /// notice the courier's view growing underneath an in-progress walk.
    pub last_digest: Option<String>,
    /// The [`ExportResume`] the last page returned, handed back on the next
    /// tick so the predecessor pins this walk instead of re-deriving its digest
    /// and total per page (Task 24/28, G8).
    ///
    /// A pin is only ever as good as the walk it describes, so it is dropped
    /// wherever the walk is: cleared by [`next_sweep`] on the restart a
    /// mid-walk digest change forces, and cleared by the ticker when the export
    /// refuses it by name ([`carry::CHAIN_MOVED`]). Losing it is never an
    /// error — an unpinned page is exactly what this sweep sent before the
    /// field existed.
    pub resume: Option<ExportResume>,
    /// How many action rows the predecessor's POSITION scan read on the last
    /// page — **risk row R1's metric**, projected per neighbour so the cost of
    /// finding a page is readable without a log dive.
    ///
    /// Last non-`None` wins, the same discipline `observed_head` keeps: a page
    /// that cannot state it must not erase what an earlier page did. `None`
    /// only from a v2 cell whose `carry_from` predates Task 29 — the producer
    /// half landed there, so this is populated on a current mesh.
    pub scanned: Option<u32>,
    /// **WHICH STORE ANSWERED the last page of this neighbour's walk** —
    /// `"authority"`, `"local-only"`, `"unreachable"`, or `None` from a zome
    /// predating Task 29. Projected per neighbour because it is the one field
    /// that says whether the numbers beside it can be trusted as the
    /// NEIGHBOUR's chain or only as this peer's arc-scoped slice of it.
    ///
    /// Station 6's root cause, made readable: a held view frozen at 212 records
    /// against an `observed_head` of 732 looks identical to a short chain
    /// unless the page says a partial-arc LOCAL read produced it. With
    /// `local-only` beside `total`, a driver knows the number is a floor, not a
    /// count — and `unreachable` says nobody confirmed anything at all.
    ///
    /// Last non-`None` wins, like its siblings: a page that cannot state which
    /// store answered must not erase what an earlier page did. Kept as an OPEN
    /// string all the way through — see [`carry::CarryReceipt::view`] for why a
    /// closed enum here would break older peers on a future fourth label.
    pub view: Option<String>,
    /// The highest action sequence the predecessor observed for this chain,
    /// last non-`None` wins. Never fabricated as 0.
    pub observed_head: Option<u32>,
    /// The record count of the courier's view, as the export stated it, last
    /// non-`None` wins.
    pub total: Option<u32>,
    /// Records this sweep has NEWLY moved into v2 for this neighbour,
    /// accumulated over ticks — [`CarryReceipt::newly_carried`], so re-walking
    /// the same view adds nothing rather than a multiple of the truth.
    pub carried: u32,
    /// RFC3339 stamp of the last tick that touched this neighbour, success or
    /// failure. Stamped by the ticker, not by [`next_sweep`], which stays pure.
    pub last_sweep: Option<String>,
    /// The last thing worth an operator's attention about this neighbour: a
    /// page failure, the `restarted:` note a mid-walk digest change leaves, or
    /// the `halted:` reason below. Cleared by the next clean page.
    pub last_error: Option<String>,
    /// **Terminal for this neighbour until a reset.** Set when the sweep
    /// reached the point where it would REWIND (end of the local view, or a
    /// digest change mid-walk) against a v2 that does not report
    /// `already_carried` — i.e. a zome predating Task 20's entry-hash
    /// idempotency, where a re-walk re-creates every entry and authors a
    /// duplicate witness every tick.
    ///
    /// Halting rather than rewinding keeps the forward progress already made
    /// and refuses only the repeat. It is deliberately NOT cleared by a later
    /// page: the property is about the v2 cell's code, which does not change
    /// under a running window. `POST /admin/lineage/reset` clears it with the
    /// rest of the state, which is the right granularity — a new crossing gets
    /// a fresh judgement.
    pub halted: bool,
}

/// Fold ONE held page into a neighbour's sweep state. Pure — no clock, no
/// conductor, no lock — so the cursor bookkeeping, which is where an
/// off-by-one silently truncates a neighbour, is unit-testable outright.
///
/// `last_sweep` is carried through untouched; the ticker stamps it, because a
/// function that reads the clock is not a function you can assert on.
///
/// The rules, in the order they matter:
///
/// 1. **Digest changed mid-walk → restart.** If the walk was in progress
///    (`state.cursor.is_some()`) and this page's `v1_digest` differs from the
///    last one, the courier's view of this neighbour changed underneath the
///    ordinal. The cursor goes back to `None` and a `restarted:` note lands on
///    `last_error` — the page's own carriage still counts, because those
///    records did move.
/// 2. **Otherwise the cursor follows the page.** `next_cursor: None` on a HELD
///    page means end-of-LOCAL-VIEW, never end-of-chain, so it also resolves to
///    "start again at the beginning next tick" — which is precisely what a
///    trailing sweep should do, and is safe because a re-carried record comes
///    back as `already_carried`.
/// 3. **A rewind against a pre-Task-20 v2 HALTS instead.** Both cases above
///    resolve to "start again at the beginning next tick", which is a re-walk.
///    A re-walk is only safe when the v2 cell reports `already_carried`
///    ([`CarryReceipt::reports_idempotency`]); a cell that cannot state it
///    would re-create every entry and author a duplicate witness every tick, so
///    the neighbour is halted with a `halted:` reason and the forward progress
///    already made is kept.
/// 4. **`carried` accumulates NEW carriage only.**
/// 5. **`total` / `observed_head` / `scanned` / `view` keep the last
///    non-`None`.** A momentarily blind authority must not erase what an
///    earlier page established. `view` joins them rather than tracking only the
///    latest page because it QUALIFIES those same numbers: a `local-only` label
///    dropped on a silent page would leave an arc-scoped `total` reading as if
///    an authority had confirmed it.
/// 6. **The resume pin follows the page, and dies with the walk.** A forward
///    page's pin is kept and handed back next tick; a restart clears it,
///    because a pin describes the walk that is being abandoned. The pin is
///    never a correctness input here — losing one costs the predecessor a
///    re-derivation and nothing else.
pub fn next_sweep(state: &AgentSweep, page: &CarryReceipt) -> AgentSweep {
    let mid_walk = state.cursor.is_some();
    let digest_changed = state
        .last_digest
        .as_deref()
        .is_some_and(|prev| prev != page.v1_digest);
    let restarted = mid_walk && digest_changed;
    // Both ways the cursor can land back at the beginning — which is the ONLY
    // thing idempotency is needed for. A forward page never needs it.
    let would_rewind = restarted || page.next_cursor.is_none();
    let halted = state.halted || (would_rewind && !page.reports_idempotency());

    AgentSweep {
        cursor: if restarted { None } else { page.next_cursor },
        last_digest: Some(page.v1_digest.clone()),
        // A pin describes ONE walk. The restart abandons that walk, so it
        // abandons the pin with it — handing it back would earn the named
        // `chain moved` refusal on the very next tick.
        resume: if restarted { None } else { page.resume.clone() },
        scanned: page.scanned.or(state.scanned),
        view: page.view.clone().or_else(|| state.view.clone()),
        observed_head: page.v1_observed_head.or(state.observed_head),
        total: page.v1_total.or(state.total),
        carried: state.carried.saturating_add(page.newly_carried()),
        last_sweep: state.last_sweep.clone(),
        last_error: if halted {
            Some(
                "halted: this v2 cell does not report `already_carried`, so it predates Task 20's \
                 entry-hash idempotency — re-walking the local view would re-create every entry \
                 and author a duplicate witness on every tick. Forward carriage up to here is \
                 kept; the repeat is refused. Clears on POST /admin/lineage/reset."
                    .to_string(),
            )
        } else if restarted {
            Some(format!(
                "restarted: the courier's view of this chain changed mid-walk (digest {} → {}) — \
                 the held cursor is an ordinal into a list gossip can grow, so the walk resumes at \
                 the beginning rather than at a stale position",
                state.last_digest.as_deref().unwrap_or("?"),
                page.v1_digest
            ))
        } else {
            None
        },
        halted,
    }
}

/// Key into the sweep state: `(role, agent)`, where `agent` is the neighbour's
/// canonical `uhCAk…` rendering. A `String` and not an `AgentPubKey` so the
/// state is `Ord`-keyed, cheap to snapshot, and directly projectable.
pub type SweepKey = (String, String);

/// One role with a lineage window currently OPEN, carrying BOTH app ids the
/// sweep needs — because it asks two different cells two different questions.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OpenWindow {
    role: String,
    /// The v1 app id reads stay pinned to. `known_agents` is asked HERE.
    reading_app_id: String,
    /// The v2 side app id authoring for this role. `carry_from` runs HERE.
    authoring_app_id: String,
}

/// **Task 13a.** The revert ceremony's view of this ticker: one method, so a
/// revert can drop a reverted role's cursors without holding — or being able
/// to reach — anything else about the sweep.
impl crate::services::release_adoption::revert::RoleSweepState for LineageBridge {
    fn clear_role(&self, role: &str) {
        LineageBridge::clear_role(self, role);
    }
}

/// The ticker. One tokio task, spawned once at startup, idle while no lineage
/// window is open.
pub struct LineageBridge {
    lineage: Arc<LineageRoles>,
    registry: Arc<HcClientRegistry>,
    interval: Duration,
    state: RwLock<BTreeMap<SweepKey, AgentSweep>>,
}

impl LineageBridge {
    /// **Task 33.** No `base_app_id`: the sweep names its v1 cell from the
    /// window's own `reading_app_id` — the app it CONNECTS to — so the two can
    /// never describe different chains, and a run-scoped predecessor is swept
    /// exactly like the household's own. The base app id remains available on
    /// [`LineageRoles::base_app_id`] for anything that genuinely needs it.
    pub fn new(lineage: Arc<LineageRoles>, registry: Arc<HcClientRegistry>) -> Self {
        Self {
            lineage,
            registry,
            interval: sweep_interval(),
            state: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// A point-in-time copy of every neighbour sweep, for the passport. The
    /// passport never holds this `Arc` — only the snapshot, taken fresh on each
    /// `/version` call (the same discipline `LineageRoles::snapshot` follows).
    pub fn snapshot(&self) -> BTreeMap<SweepKey, AgentSweep> {
        self.state.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Forget every sweep. Wired into `POST /admin/lineage/reset` (Task 10):
    /// a mesh run must START and END at the v1 baseline, and a cursor left
    /// pointing into a view of a side app that reset just uninstalled is a
    /// position with nothing behind it.
    ///
    /// Deliberately NOT called by `LineageRoles::revert`/`sunset` — those are
    /// Task 13/14's ceremonies, and what a reverted window's sweep record
    /// should become is their decision to make, not this module's.
    pub fn reset(&self) {
        self.state
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    /// Forget every neighbour sweep for ONE role — [`reset`](Self::reset)'s
    /// granularity, narrowed.
    ///
    /// **Task 13a.** A revert disables the side app this role's cursors are
    /// ordinals into, so they must go for exactly the reason the reset route
    /// clears them: a position with nothing behind it. Only that role's keys
    /// move — another role's window may still be open, and clearing its
    /// cursors would restart a walk that is making progress.
    ///
    /// Note the doc on `reset` above still holds: the BRIDGE does not decide
    /// this. `LineageRoles::revert` does not call it either. The revert
    /// ceremony (`release_adoption::apply::HappLineageVehicle`) calls it,
    /// which is Task 13's decision to make and is why this method exists
    /// rather than a hook inside `revert`.
    pub fn clear_role(&self, role: &str) {
        self.state
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|(swept_role, _), _| swept_role != role);
    }

    /// The roles with an OPEN lineage window, each paired with BOTH app ids the
    /// sweep needs. Pure over a snapshot, so the idle-tick exit and the app-id
    /// selection are both testable without a conductor.
    ///
    /// Both ids are returned rather than just the authoring one because the
    /// sweep asks two DIFFERENT cells two different questions: `known_agents`
    /// goes to the READING (v1) app — who is on the predecessor DHT — and
    /// `carry_from` goes to the AUTHORING (v2) side app. Collapsing them to one
    /// id is the mistake this signature exists to make impossible.
    ///
    /// # The predicate names the OPEN shape, it does not negate the closed ones
    ///
    /// An open window is exactly what `LineageRoles::open_window` builds:
    /// **reads pinned to the base app, authoring moved off it, not closed.**
    /// All three clauses are load-bearing, and the tempting shorthand
    /// `authoring != reading && !closed` is WRONG — `revert` leaves the two ids
    /// still different but INVERTED (it moves the side app into
    /// `reading_app_id` as a historical pointer and puts authoring back on
    /// base). A sweep run over a reverted role would ask `known_agents` of v2
    /// and `carry_from` of v1: backwards on both cells, and noisy rather than
    /// silent, but never something a courier should attempt.
    ///
    /// A SUNSET role is excluded by `!closed`: the crossing is over, and a
    /// courier that kept carrying into a closed window would be writing into a
    /// cell the ceremony has already finished with.
    ///
    /// The predicate itself now lives on [`LineageRoles::open_windows`] —
    /// **Task 13a**, because the revert sweep needs the identical answer and
    /// two copies of a three-clause rule is exactly how the inverted-ids case
    /// gets re-introduced. This method keeps its own name and its own return
    /// type (the two app ids, un-collapsed), and adds nothing to the rule.
    fn open_windows(&self) -> Vec<OpenWindow> {
        self.lineage
            .open_windows()
            .into_iter()
            .map(|(role, lineage)| OpenWindow {
                role,
                reading_app_id: lineage.reading_app_id,
                authoring_app_id: lineage.authoring_app_id,
            })
            .collect()
    }

    /// One tick. Never returns an error: a sweep that could not run is a
    /// missing observation, not a fault in the node, and the next tick asks
    /// again.
    pub async fn tick(&self) {
        // THE IDLE EXIT, FIRST AND CHEAPEST. No conductor call, no admin
        // round trip, no lock held past this line.
        let windows = self.open_windows();
        if windows.is_empty() {
            return;
        }
        for window in windows {
            self.sweep_role(&window).await;
        }
    }

    /// Sweep every known neighbour for ONE role, one page each.
    async fn sweep_role(&self, window: &OpenWindow) {
        let role = window.role.as_str();
        let Some(admin) = self.registry.any_admin_websocket() else {
            tracing::debug!(
                role,
                "lineage bridge: no admin connection — skipping this tick"
            );
            return;
        };
        let v1_cell = match self.v1_cell(&admin, role, &window.reading_app_id).await {
            Ok(cell) => cell,
            Err(e) => {
                tracing::warn!(role, error = %e, "lineage bridge: no v1 cell to carry FROM");
                return;
            }
        };

        // `known_agents` is a V1 QUESTION — who is on the PREDECESSOR DHT — so
        // it is asked of the READING app id, connected explicitly.
        //
        // **The hazard this avoids.** `registry.client(role)` looks like the
        // normal path, and it is the wrong one here: that supervised client is
        // built from `LineageRoles::app_id_for(role)`, which resolves to the
        // AUTHORING app while a window is open. It happens to still hold a v1
        // connection only because it was dialled before the window opened — so
        // the first time the bridge supervisor re-mints that role mid-window
        // (any conductor restart), it silently reconnects to v2 and this call
        // starts enumerating the SUCCESSOR's DHT. The failure is quiet: an
        // empty or foreign neighbour list, and a sweep that carries nothing
        // while reporting no error.
        let reading = match self
            .registry
            .connect_app(&window.reading_app_id, role)
            .await
        {
            Ok(client) => client,
            Err(e) => {
                tracing::warn!(
                    role,
                    reading_app_id = %window.reading_app_id,
                    error = %e,
                    "lineage bridge: could not connect to the reading (v1) app this tick"
                );
                return;
            }
        };
        let agents = match self.neighbours(&reading).await {
            Ok(agents) => agents,
            Err(e) => {
                tracing::warn!(role, error = %e, "lineage bridge: could not list neighbours");
                return;
            }
        };
        if agents.is_empty() {
            return;
        }

        // ONE connection to the side app per tick, shared by every neighbour's
        // page — never cached on `self`, for the same reason the apply vehicle
        // does not cache it: a handle held across ticks outlives the window it
        // belongs to, and a reverted or sunset window would still have a live
        // authoring path into v2.
        let side = match self
            .registry
            .connect_app(&window.authoring_app_id, role)
            .await
        {
            Ok(client) => client,
            Err(e) => {
                tracing::warn!(
                    role,
                    lineage_app_id = %window.authoring_app_id,
                    error = %e,
                    "lineage bridge: could not connect to the side app this tick"
                );
                return;
            }
        };

        for agent in agents {
            // Per-agent failure NEVER aborts the tick: one unreachable
            // neighbour must not stop the sweep of the others.
            self.sweep_agent(role, &side, &v1_cell, agent).await;
        }
    }

    /// One page for one neighbour. Snapshot the state, drop the guard, call,
    /// then write — no lock is ever held across an `.await`.
    async fn sweep_agent(
        &self,
        role: &str,
        side: &Arc<crate::hc_client::HcClient>,
        v1_cell: &holochain_client::CellId,
        agent: holochain_types::prelude::AgentPubKey,
    ) {
        let key: SweepKey = (role.to_string(), agent.to_string());
        let before = self
            .state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&key)
            .cloned()
            .unwrap_or_default();
        // A halted neighbour is not swept again. The reason is on its
        // `last_error` and was logged once, at the transition below — logging
        // it every tick forever would be the noise a halt exists to avoid.
        if before.halted {
            return;
        }

        let input = CarryInput {
            v1_cell: v1_cell.clone(),
            cursor: before.cursor,
            limit: HELD_PAGE_LIMIT,
            source: CarrySource::Held(agent),
            // The pin the last page returned, so the predecessor walks this
            // page's window rather than re-deriving the whole view's digest
            // every tick (Task 24/28, G8).
            resume: before.resume.clone(),
        };
        let outcome = call_carry_from(side, input).await;

        let mut after = match &outcome {
            Ok(page) => next_sweep(&before, page),
            // **The pin was refused by name — drop it, keep the cursor.**
            // Deliberately NOT a restart here, unlike the apply fold. A restart
            // is a REWIND, and rule 3 above only permits one against a v2 that
            // reports `already_carried` — which an error carries no receipt to
            // tell us. Dropping the pin alone puts the next tick back on the
            // un-pinned path this sweep used before the field existed: it is
            // always correct, merely slower, and the page it returns states the
            // changed digest so `next_sweep`'s landed restart-or-halt rules
            // make that judgement with a receipt in hand.
            Err(e) if carry::is_chain_moved(e) => AgentSweep {
                resume: None,
                last_error: Some(format!(
                    "unpinned: the predecessor refused the resume token ({} …) — its view of this \
                     chain moved, so the next page is walked without a pin and the digest it \
                     reports decides whether the walk restarts",
                    carry::CHAIN_MOVED
                )),
                ..before.clone()
            },
            Err(e) => AgentSweep {
                last_error: Some(e.clone()),
                ..before.clone()
            },
        };
        after.last_sweep = Some(now_rfc3339());
        let carried_this_page = outcome.as_ref().map_or(0, |p| p.newly_carried());
        let witnessed = outcome.as_ref().is_ok_and(|p| !p.witness_hash.is_empty());

        self.state
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key.clone(), after.clone());

        if after.halted && !before.halted {
            tracing::warn!(
                role,
                agent = %key.1,
                carried = after.carried,
                "lineage bridge: HALTED this neighbour — the v2 cell does not report \
                 `already_carried`, so it predates Task 20's entry-hash idempotency and a re-walk \
                 would re-create every entry and author a duplicate witness each tick. Forward \
                 carriage is kept; the repeat is refused until POST /admin/lineage/reset."
            );
            return;
        }

        match &outcome {
            Ok(_) if carried_this_page > 0 => tracing::info!(
                role,
                agent = %key.1,
                carried = carried_this_page,
                total_carried = after.carried,
                witnessed,
                cursor = ?after.cursor,
                observed_head = ?after.observed_head,
                // WHICH STORE ANSWERED (Task 29). Without it, a `local-only`
                // page — one peer's arc-scoped slice of the neighbour's chain —
                // logs identically to an authority-confirmed one, and a driver
                // reading these numbers cannot tell a short chain from a
                // truncated view of a long one.
                view = ?after.view,
                scanned = ?after.scanned,
                "lineage bridge: held-carried a page of a neighbour's v1 records into v2"
            ),
            // `witness_hash == ""` and nothing new: the page re-walked records
            // already carried. Debug, not info — this is the steady state of a
            // caught-up sweep and would otherwise log forever.
            Ok(_) => tracing::debug!(
                role,
                agent = %key.1,
                cursor = ?after.cursor,
                "lineage bridge: nothing new for this neighbour"
            ),
            Err(e) => tracing::warn!(
                role,
                agent = %key.1,
                error = %e,
                "lineage bridge: held carry failed for this neighbour — recorded, tick continues"
            ),
        }
    }

    /// The PREDECESSOR app's provisioned cell for `role` — the held carry's v1
    /// source.
    ///
    /// **Task 33.** `reading_app_id` comes off the window the sweep is already
    /// holding, not from `base_app_id`: the sweep CONNECTS to that app (see the
    /// hazard named at the call site), so naming the cell from anywhere else is
    /// how the two would come to describe different chains. On an unbound
    /// crossing the window's reading id IS the base app id, so nothing moves.
    async fn v1_cell(
        &self,
        admin: &holochain_client::AdminWebsocket,
        role: &str,
        reading_app_id: &str,
    ) -> Result<holochain_client::CellId, String> {
        let apps = admin
            .list_apps(None)
            .await
            .map_err(|e| format!("list_apps: {e}"))?;
        let v1 = apps
            .into_iter()
            .find(|a| a.installed_app_id == reading_app_id)
            .ok_or_else(|| format!("v1 app '{reading_app_id}' is not installed"))?;
        v1.cell_info
            .get(role)
            .and_then(|cells| {
                cells.iter().find_map(|c| match c {
                    holochain_client::CellInfo::Provisioned(p) => Some(p.cell_id.clone()),
                    _ => None,
                })
            })
            .ok_or_else(|| {
                format!("v1 app '{reading_app_id}' has no provisioned cell for role '{role}'")
            })
    }

    /// Who is on the v1 DHT, MINUS ourselves.
    ///
    /// `client` MUST be connected to the READING app id — see the hazard named
    /// at the call site. This function cannot check that for itself, which is
    /// why `OpenWindow` carries the id rather than the caller re-deriving one.
    ///
    /// `known_agents` deliberately INCLUDES the caller (it reports who is on
    /// the DHT, not who is foreign), and `carry_from` refuses
    /// `Held(<our own key>)` rather than silently mis-labelling a self-carry.
    /// So the filter is the caller's job and it is done here, once.
    async fn neighbours(
        &self,
        client: &Arc<crate::hc_client::HcClient>,
    ) -> Result<Vec<holochain_types::prelude::AgentPubKey>, String> {
        let payload = rmp_serde::to_vec_named(&()).map_err(|e| format!("encode (): {e}"))?;
        let bytes = client
            .call_zome(CARRY_ZOME, KNOWN_AGENTS_FN, payload)
            .await
            .map_err(|e| e.to_string())?;
        let agents: Vec<holochain_types::prelude::AgentPubKey> = rmp_serde::from_slice(&bytes)
            .map_err(|e| format!("decode {KNOWN_AGENTS_FN} response: {e}"))?;
        let me = client.agent_key_uhcak();
        Ok(agents.into_iter().filter(|a| a.to_string() != me).collect())
    }

    /// Spawn the ticker. One task for the life of the process.
    pub fn spawn(self: Arc<Self>, mut shutdown: tokio::sync::broadcast::Receiver<()>) {
        let interval = self.interval;
        tokio::spawn(async move {
            tracing::info!(
                interval_secs = interval.as_secs(),
                page_limit = HELD_PAGE_LIMIT,
                "lineage bridge sweep armed (idle until a lineage window opens)"
            );
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {}
                    _ = shutdown.recv() => {
                        tracing::info!("lineage bridge sweep exiting (shutdown)");
                        return;
                    }
                }
                self.tick().await;
            }
        });
    }
}

/// The sweep cadence, read ONCE at construction. Never read on the hot path —
/// an `std::env::var` a tick calls is the parallel-test flake class, and a
/// cadence that can change under a running loop is a cadence nothing can
/// assert.
fn sweep_interval() -> Duration {
    let secs = std::env::var(LINEAGE_SWEEP_SECS_ENV)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|secs| *secs > 0)
        .unwrap_or(LINEAGE_SWEEP_SECS);
    Duration::from_secs(secs)
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A page from a Task-20 v2 — one that reports its own idempotency.
    fn page(
        carried: u32,
        already: u32,
        next_cursor: Option<u32>,
        digest: &str,
        total: Option<u32>,
        head: Option<u32>,
    ) -> CarryReceipt {
        CarryReceipt {
            carried,
            next_cursor,
            v1_digest: digest.to_string(),
            witness_hash: if carried > already {
                "uhCEkWitness".to_string()
            } else {
                String::new()
            },
            v1_total: total,
            // A held carry is never a self-carry: the courier authors the
            // witness, never the content.
            self_carried: 0,
            v1_observed_head: head,
            already_carried: Some(already),
            // The pin a Task-24 predecessor returns: named after the digest it
            // belongs to, so a test can see WHICH walk a kept pin describes.
            resume: Some(ExportResume {
                head: format!("uhCkkHead-{digest}"),
                digest: digest.to_string(),
                total: total.unwrap_or(0),
                observed_head: head,
                cursor_seq: next_cursor.map(|c| (c, c * 6)),
            }),
            scanned: Some(8),
            // A held page names WHICH store answered (Task 29). `authority`
            // here so the fixture's default is the confident case and a test
            // that cares about the arc-scoped one says so explicitly.
            view: Some("authority".to_string()),
        }
    }

    /// The same page from a v2 whose zome PREDATES Task 20 — `already_carried`
    /// simply is not on the wire.
    fn pre_task_20_page(carried: u32, next_cursor: Option<u32>, digest: &str) -> CarryReceipt {
        CarryReceipt {
            already_carried: None,
            ..page(carried, 0, next_cursor, digest, None, None)
        }
    }

    /// FRESH: nothing swept yet, the first page lands whole.
    #[test]
    fn a_fresh_sweep_takes_the_page_verbatim() {
        let after = next_sweep(
            &AgentSweep::default(),
            &page(16, 0, Some(16), "digest-a", Some(41), Some(57)),
        );
        assert_eq!(after.cursor, Some(16));
        assert_eq!(after.last_digest.as_deref(), Some("digest-a"));
        assert_eq!(after.carried, 16);
        assert_eq!(after.total, Some(41));
        assert_eq!(after.observed_head, Some(57));
        assert!(after.last_error.is_none());
        // The ticker stamps the clock, not the fold.
        assert!(after.last_sweep.is_none());
    }

    /// MID-WALK, SAME DIGEST: the cursor advances and carriage accumulates.
    #[test]
    fn a_mid_walk_page_on_the_same_digest_advances_the_cursor() {
        let before = AgentSweep {
            cursor: Some(16),
            last_digest: Some("digest-a".into()),
            total: Some(41),
            observed_head: Some(57),
            carried: 16,
            ..Default::default()
        };
        let after = next_sweep(&before, &page(16, 0, Some(32), "digest-a", Some(41), None));
        assert_eq!(after.cursor, Some(32));
        assert_eq!(after.carried, 32, "16 + 16, accumulated across ticks");
        assert_eq!(
            after.observed_head,
            Some(57),
            "a page that observed nothing must not ERASE what an earlier page established"
        );
        assert!(after.last_error.is_none());
    }

    /// MID-WALK, DIGEST CHANGED: the courier's view grew underneath the
    /// ordinal, so the walk restarts at the beginning with a note — and the
    /// page's own carriage is still counted, because those records did move.
    #[test]
    fn a_digest_change_mid_walk_restarts_the_cursor_with_a_note() {
        let before = AgentSweep {
            cursor: Some(16),
            last_digest: Some("digest-a".into()),
            carried: 16,
            ..Default::default()
        };
        let after = next_sweep(&before, &page(4, 0, Some(32), "digest-b", None, None));
        assert_eq!(
            after.cursor, None,
            "a stale ordinal is never trusted — the next tick starts at the beginning"
        );
        assert_eq!(after.carried, 20);
        assert_eq!(after.last_digest.as_deref(), Some("digest-b"));
        let note = after.last_error.expect("a restart leaves a note");
        assert!(note.starts_with("restarted:"), "got {note}");
        assert!(
            note.contains("digest-a") && note.contains("digest-b"),
            "{note}"
        );
    }

    /// A digest change on the FIRST page of a walk is not a restart — there is
    /// no position to invalidate, so it is simply the digest of the walk we
    /// are starting.
    #[test]
    fn a_digest_change_before_the_walk_started_is_not_a_restart() {
        let before = AgentSweep {
            cursor: None,
            last_digest: Some("digest-a".into()),
            carried: 40,
            ..Default::default()
        };
        let after = next_sweep(&before, &page(16, 0, Some(16), "digest-b", None, None));
        assert_eq!(after.cursor, Some(16));
        assert!(after.last_error.is_none());
    }

    /// PAGE END: `next_cursor: None` on a held page is end-of-LOCAL-VIEW, so
    /// the sweep resolves to "start again next tick" — a trailing sweep, which
    /// is exactly the intent.
    #[test]
    fn the_end_of_the_local_view_rewinds_rather_than_stopping() {
        let before = AgentSweep {
            cursor: Some(32),
            last_digest: Some("digest-a".into()),
            carried: 32,
            ..Default::default()
        };
        let after = next_sweep(&before, &page(9, 0, None, "digest-a", Some(41), Some(57)));
        assert_eq!(after.cursor, None);
        assert_eq!(after.carried, 41);
        assert!(after.last_error.is_none());
    }

    /// EMPTY PAGE: a caught-up sweep re-walks a view whose every record is
    /// already carried. `witness_hash == ""`, nothing new, and the running
    /// total does NOT inflate — which is the whole reason the accumulator is
    /// `newly_carried` and not `carried`.
    #[test]
    fn a_re_walk_of_an_already_carried_view_adds_nothing() {
        let before = AgentSweep {
            cursor: None,
            last_digest: Some("digest-a".into()),
            carried: 41,
            total: Some(41),
            ..Default::default()
        };
        let re_walk = page(16, 16, Some(16), "digest-a", Some(41), Some(57));
        assert!(
            re_walk.witness_hash.is_empty(),
            "a page that carried nothing new authors no witness"
        );
        let after = next_sweep(&before, &re_walk);
        assert_eq!(after.carried, 41, "a re-walk must not multiply the truth");
        assert_eq!(after.cursor, Some(16));
    }

    /// The window filter: only an OPEN window sweeps. A single-cell role and a
    /// SUNSET role are both skipped, so the idle tick makes no conductor call.
    #[test]
    fn only_an_open_window_is_swept() {
        let lineage = Arc::new(LineageRoles::new("elohim", &["node_registry", "lamad"]));
        let bridge = LineageBridge::new(Arc::clone(&lineage), Arc::new(HcClientRegistry::empty()));
        assert!(
            bridge.open_windows().is_empty(),
            "a node with no window open sweeps nothing"
        );

        lineage.open_window("node_registry", "elohim@EKiIscIk5BDd", None);
        assert_eq!(
            bridge.open_windows(),
            vec![OpenWindow {
                role: "node_registry".to_string(),
                // `known_agents` is asked HERE — the v1 app reads stay pinned
                // to, never the authoring one. Asking the role's supervised
                // client instead would resolve to the authoring app after a
                // mid-window re-mint and enumerate v2's DHT.
                reading_app_id: "elohim".to_string(),
                authoring_app_id: "elohim@EKiIscIk5BDd".to_string(),
            }]
        );

        // Sunset closes the crossing — a courier must not keep carrying into a
        // window the ceremony has finished with.
        lineage.sunset("node_registry");
        assert!(bridge.open_windows().is_empty());
    }

    /// An idle tick is a no-op even with a registry that would panic-by-503 on
    /// any call: the window filter runs FIRST, so nothing downstream is
    /// reached.
    #[tokio::test]
    async fn an_idle_tick_touches_no_conductor() {
        let bridge = LineageBridge::new(
            Arc::new(LineageRoles::new("elohim", &["node_registry"])),
            // `empty()` has no conductor URLs at all — `connect_app` on it
            // fails closed, and `any_admin_websocket` is `None`. Reaching
            // either would be observable; the tick returns before both.
            Arc::new(HcClientRegistry::empty()),
        );
        bridge.tick().await;
        assert!(bridge.snapshot().is_empty());
    }

    /// Reset clears the sweep record. The mesh fixture converges to baseline
    /// through `POST /admin/lineage/reset`, and a cursor pointing into the view
    /// of a side app that reset just uninstalled is a position with nothing
    /// behind it.
    #[test]
    fn reset_forgets_every_sweep() {
        let bridge = LineageBridge::new(
            Arc::new(LineageRoles::new("elohim", &["node_registry"])),
            Arc::new(HcClientRegistry::empty()),
        );
        bridge.state.write().unwrap().insert(
            ("node_registry".into(), "uhCAkJessica".into()),
            AgentSweep {
                carried: 7,
                ..Default::default()
            },
        );
        assert_eq!(bridge.snapshot().len(), 1);
        bridge.reset();
        assert!(bridge.snapshot().is_empty());
    }

    /// **Task 13a.** A revert clears ONE role's cursors and leaves every other
    /// role's alone — another window may still be open, and restarting a walk
    /// that is making progress is not what a revert asks for.
    #[test]
    fn clear_role_forgets_one_roles_sweeps_and_no_others() {
        let bridge = LineageBridge::new(
            Arc::new(LineageRoles::new("elohim", &["node_registry", "lamad"])),
            Arc::new(HcClientRegistry::empty()),
        );
        {
            let mut state = bridge.state.write().unwrap();
            for key in [
                ("node_registry".to_string(), "uhCAkJessica".to_string()),
                ("node_registry".to_string(), "uhCAkMatthew".to_string()),
                ("lamad".to_string(), "uhCAkJessica".to_string()),
            ] {
                state.insert(
                    key,
                    AgentSweep {
                        carried: 3,
                        ..Default::default()
                    },
                );
            }
        }
        assert_eq!(bridge.snapshot().len(), 3);

        bridge.clear_role("node_registry");

        let left: Vec<SweepKey> = bridge.snapshot().into_keys().collect();
        assert_eq!(
            left,
            vec![("lamad".to_string(), "uhCAkJessica".to_string())],
            "only the reverted role's cursors go"
        );
        // Idempotent: a second clear on a role with nothing left is a no-op,
        // which is what a re-run sweep does after the window is already gone.
        bridge.clear_role("node_registry");
        assert_eq!(bridge.snapshot().len(), 1);
    }

    /// **The two app ids an open window carries are DIFFERENT and are never
    /// collapsed.** This is the whole guard behind Minor 1: the sweep asks
    /// `known_agents` of the reading (v1) app and `carry_from` of the authoring
    /// (v2) side app, and a window whose ids were equal would not be an open
    /// window at all.
    #[test]
    fn an_open_windows_two_app_ids_are_never_the_same_cell() {
        let lineage = Arc::new(LineageRoles::new("elohim", &["node_registry"]));
        lineage.open_window("node_registry", "elohim@EKiIscIk5BDd", None);
        let bridge = LineageBridge::new(Arc::clone(&lineage), Arc::new(HcClientRegistry::empty()));
        let [window] = bridge.open_windows().try_into().expect("one open window");
        assert_ne!(window.reading_app_id, window.authoring_app_id);
        assert_eq!(window.reading_app_id, "elohim", "the PREDECESSOR is asked");
        // …and a REVERT ends the window. This is the case the naive predicate
        // `authoring != reading && !closed` gets wrong: revert leaves the two
        // ids different but INVERTED (the side app becomes the historical
        // `reading_app_id`, authoring goes back to base), so a sweep would ask
        // `known_agents` of v2 and `carry_from` of v1 — backwards on both.
        lineage.revert("node_registry");
        let reverted = lineage.snapshot();
        assert_ne!(
            reverted["node_registry"].reading_app_id, reverted["node_registry"].authoring_app_id,
            "the ids ARE still different after a revert — which is exactly why the \
             predicate names the open shape instead of negating the closed ones"
        );
        assert!(bridge.open_windows().is_empty());
    }

    /// **A pre-Task-20 v2 halts at the rewind rather than re-walking.** The
    /// forward page is taken (that carriage is real); the moment the walk would
    /// return to the beginning, the sweep refuses — because on a cell that
    /// cannot state its own idempotency a re-walk re-creates every entry and
    /// authors a duplicate witness on every tick.
    #[test]
    fn a_pre_task_20_v2_halts_instead_of_re_walking() {
        // Forward page: NOT halted, and the records really moved.
        let forward = next_sweep(
            &AgentSweep::default(),
            &pre_task_20_page(16, Some(16), "digest-a"),
        );
        assert!(!forward.halted, "forward carriage is never refused");
        assert_eq!(forward.carried, 16);

        // End of the local view — the rewind point. Halt.
        let at_end = next_sweep(&forward, &pre_task_20_page(9, None, "digest-a"));
        assert!(at_end.halted);
        assert_eq!(at_end.carried, 25, "the last page's carriage is still kept");
        let why = at_end.last_error.clone().expect("a halt states its reason");
        assert!(why.starts_with("halted:"), "got {why}");
        assert!(why.contains("already_carried"), "{why}");

        // Terminal: a LATER page — even a well-formed Task-20 one — does not
        // un-halt it. The property is about the cell's code, and only a reset
        // re-judges it.
        let later = next_sweep(&at_end, &page(1, 0, Some(1), "digest-a", None, None));
        assert!(later.halted);
    }

    /// A digest change mid-walk is also a rewind, so it halts too on a
    /// pre-Task-20 cell — restarting there would re-carry from zero.
    #[test]
    fn a_digest_change_on_a_pre_task_20_v2_halts_rather_than_restarting() {
        let before = AgentSweep {
            cursor: Some(16),
            last_digest: Some("digest-a".into()),
            carried: 16,
            ..Default::default()
        };
        let after = next_sweep(&before, &pre_task_20_page(4, Some(32), "digest-b"));
        assert!(after.halted);
        assert!(after.last_error.unwrap().starts_with("halted:"));
    }

    /// A Task-20 cell that reports `already_carried: Some(0)` is NOT halted —
    /// absent and zero are different claims, and only absence is a refusal.
    #[test]
    fn a_reported_zero_is_not_an_absent_field() {
        let after = next_sweep(
            &AgentSweep::default(),
            &page(9, 0, None, "digest-a", Some(9), Some(33)),
        );
        assert!(
            !after.halted,
            "'nothing was already here' is a claim; 'I cannot tell you' is not"
        );
        assert_eq!(
            after.cursor, None,
            "and it rewinds, as a trailing sweep should"
        );
    }

    /// The page batch is bounded BEFORE the call is made, and the cadence is
    /// read once rather than per tick.
    #[test]
    fn the_sweep_is_bounded_and_its_cadence_is_read_once() {
        const {
            assert!(HELD_PAGE_LIMIT > 0);
            assert!(LINEAGE_SWEEP_SECS > 0);
        }
        assert_eq!(HELD_PAGE_LIMIT, 16);
        assert_eq!(LINEAGE_SWEEP_SECS, 30);
        assert_eq!(KNOWN_AGENTS_FN, "known_agents");
        let bridge = LineageBridge::new(
            Arc::new(LineageRoles::new("elohim", &[])),
            Arc::new(HcClientRegistry::empty()),
        );
        // Whatever the environment says, the interval is captured at
        // construction and is a positive duration.
        assert!(bridge.interval() > Duration::ZERO);
    }

    /// **The pin round-trips through the sweep state.** A forward page's
    /// [`ExportResume`] survives onto the state the next tick reads, verbatim —
    /// that state IS the channel by which the pin reaches the zome, so a fold
    /// that dropped it would leave every page paying the first-page cost with
    /// nothing failing to say so.
    #[test]
    fn a_forward_page_keeps_its_resume_pin_for_the_next_tick() {
        let after = next_sweep(
            &AgentSweep::default(),
            &page(16, 0, Some(16), "digest-a", Some(41), Some(57)),
        );
        let pin = after.resume.as_ref().expect("a forward page keeps its pin");
        assert_eq!(pin.head, "uhCkkHead-digest-a");
        assert_eq!(pin.digest, "digest-a");
        assert_eq!(pin.total, 41);
        assert_eq!(pin.observed_head, Some(57));
        assert_eq!(
            pin.cursor_seq,
            Some((16, 96)),
            "the field that makes the next page cost its own window survives too"
        );

        // …and the NEXT page's pin replaces it rather than accumulating.
        let later = next_sweep(
            &after,
            &page(16, 0, Some(32), "digest-a", Some(41), Some(57)),
        );
        assert_eq!(
            later.resume.as_ref().map(|r| r.cursor_seq),
            Some(Some((32, 192)))
        );
    }

    /// **A restart drops the pin.** The mid-walk digest change abandons the
    /// walk; handing its pin back next tick would earn the predecessor's named
    /// `chain moved` refusal on every subsequent tick, which is a sweep that
    /// never moves again.
    #[test]
    fn a_restart_clears_the_resume_pin_with_the_cursor() {
        let before = AgentSweep {
            cursor: Some(16),
            last_digest: Some("digest-a".into()),
            resume: Some(ExportResume {
                head: "uhCkkHead-digest-a".into(),
                digest: "digest-a".into(),
                total: 41,
                observed_head: Some(57),
                cursor_seq: Some((16, 96)),
            }),
            ..AgentSweep::default()
        };
        let after = next_sweep(&before, &page(4, 0, Some(32), "digest-b", None, None));
        assert_eq!(after.cursor, None, "the landed restart still restarts");
        assert_eq!(
            after.resume, None,
            "a pin describes ONE walk — the abandoned walk's pin is abandoned with it"
        );
        assert!(after
            .last_error
            .as_deref()
            .is_some_and(|note| note.starts_with("restarted:")));
    }

    /// **WHICH STORE ANSWERED reaches the state, and survives a silent page.**
    ///
    /// Station 6's root cause in one assertion: a held page that read short
    /// because a partial-arc LOCAL store answered must be distinguishable from
    /// one an authority confirmed, or a permanent truncation is banked as a
    /// crossing. `view` rides the same last-non-`None` rule as its siblings
    /// BECAUSE it qualifies them — a dropped `local-only` would leave an
    /// arc-scoped `total` reading as authority-confirmed.
    #[test]
    fn the_answering_view_reaches_the_sweep_state_and_qualifies_the_numbers() {
        let after = next_sweep(
            &AgentSweep::default(),
            &page(16, 0, Some(16), "d", None, None),
        );
        assert_eq!(after.view.as_deref(), Some("authority"));

        // An arc-scoped page relabels the state — that IS the signal.
        let local_only = CarryReceipt {
            view: Some("local-only".into()),
            ..page(16, 0, Some(32), "d", None, None)
        };
        let after = next_sweep(&after, &local_only);
        assert_eq!(after.view.as_deref(), Some("local-only"));

        // …and a page from a zome predating Task 29 must not ERASE it: an
        // unqualified `total` is worse than a stale qualification.
        let silent = CarryReceipt {
            view: None,
            ..page(16, 0, Some(48), "d", None, None)
        };
        assert_eq!(
            next_sweep(&after, &silent).view.as_deref(),
            Some("local-only")
        );

        // A fresh sweep whose first page cannot state it stays honestly unset.
        let unlabelled = CarryReceipt {
            view: None,
            ..page(16, 0, Some(16), "d", None, None)
        };
        assert_eq!(next_sweep(&AgentSweep::default(), &unlabelled).view, None);
    }

    /// **R1's metric reaches the state.** `scanned` is what says the resumed
    /// page cost its own window rather than the whole chain; it keeps the last
    /// non-`None`, so a page that cannot state it does not erase what an earlier
    /// page did.
    #[test]
    fn the_scanned_count_reaches_the_sweep_state_and_survives_a_silent_page() {
        let after = next_sweep(
            &AgentSweep::default(),
            &page(16, 0, Some(16), "digest-a", Some(41), Some(57)),
        );
        assert_eq!(after.scanned, Some(8));

        let silent = CarryReceipt {
            scanned: None,
            ..page(16, 0, Some(32), "digest-a", Some(41), Some(57))
        };
        assert_eq!(
            next_sweep(&after, &silent).scanned,
            Some(8),
            "last non-`None` wins — a silent page reports nothing, it does not report zero"
        );
    }

    /// A page from a v2 that predates the pin leaves the state unpinned, and
    /// nothing about that is an error: it is the pre-Task-24 sweep, exactly.
    #[test]
    fn a_pre_pin_page_simply_leaves_the_sweep_unpinned() {
        let unpinned = CarryReceipt {
            resume: None,
            scanned: None,
            ..page(16, 0, Some(16), "digest-a", Some(41), Some(57))
        };
        let after = next_sweep(&AgentSweep::default(), &unpinned);
        assert_eq!(after.resume, None);
        assert_eq!(after.scanned, None);
        assert_eq!(after.cursor, Some(16), "and the walk advances regardless");
        assert!(after.last_error.is_none(), "an absent pin is not a fault");
    }

    /// The named refusal is matched as the zome writes it — including the
    /// wrapping a `call_zome` error puts around a Guest message, which is why
    /// this is a substring test and never an equality one.
    #[test]
    fn the_chain_moved_refusal_is_recognised_through_its_call_zome_wrapping() {
        assert!(carry::is_chain_moved(&format!(
            "zome call failed: Guest(\"{}: the resume token pins the chain head uhCkkOld, but \
             this walk now sees uhCkkNew.\")",
            carry::CHAIN_MOVED
        )));
        assert!(
            !carry::is_chain_moved("Websocket closed: No connection"),
            "a transport failure is NOT an instruction to drop the pin"
        );
    }
}
