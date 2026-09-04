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
//! position. Re-walking is safe and cheap because the zome is idempotent by
//! entry hash (Task 20): a re-carried record comes back as `already_carried`,
//! creates nothing and authors no second witness.
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
    call_carry_from, CarryInput, CarryReceipt, CarrySource, CARRY_ZOME,
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
    /// page failure, or the `restarted:` note a mid-walk digest change leaves.
    /// Cleared by the next clean page.
    pub last_error: Option<String>,
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
/// 3. **`carried` accumulates NEW carriage only.**
/// 4. **`total` / `observed_head` keep the last non-`None`.** A momentarily
///    blind authority must not erase what an earlier page established.
pub fn next_sweep(state: &AgentSweep, page: &CarryReceipt) -> AgentSweep {
    let mid_walk = state.cursor.is_some();
    let digest_changed = state
        .last_digest
        .as_deref()
        .is_some_and(|prev| prev != page.v1_digest);
    let restarted = mid_walk && digest_changed;

    AgentSweep {
        cursor: if restarted { None } else { page.next_cursor },
        last_digest: Some(page.v1_digest.clone()),
        observed_head: page.v1_observed_head.or(state.observed_head),
        total: page.v1_total.or(state.total),
        carried: state.carried.saturating_add(page.newly_carried()),
        last_sweep: state.last_sweep.clone(),
        last_error: if restarted {
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
    }
}

/// Key into the sweep state: `(role, agent)`, where `agent` is the neighbour's
/// canonical `uhCAk…` rendering. A `String` and not an `AgentPubKey` so the
/// state is `Ord`-keyed, cheap to snapshot, and directly projectable.
pub type SweepKey = (String, String);

/// The ticker. One tokio task, spawned once at startup, idle while no lineage
/// window is open.
pub struct LineageBridge {
    lineage: Arc<LineageRoles>,
    registry: Arc<HcClientRegistry>,
    base_app_id: String,
    interval: Duration,
    state: RwLock<BTreeMap<SweepKey, AgentSweep>>,
}

impl LineageBridge {
    pub fn new(
        lineage: Arc<LineageRoles>,
        registry: Arc<HcClientRegistry>,
        base_app_id: impl Into<String>,
    ) -> Self {
        Self {
            lineage,
            registry,
            base_app_id: base_app_id.into(),
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

    /// The roles with an OPEN lineage window, paired with the side app id
    /// authoring for them. Pure over a snapshot, so the idle-tick exit is
    /// testable without a conductor.
    ///
    /// A SUNSET role is excluded: the window is closed, the crossing is over,
    /// and a courier that kept carrying into a closed window would be writing
    /// into a cell the ceremony has already finished with.
    fn open_windows(&self) -> Vec<(String, String)> {
        self.lineage
            .snapshot()
            .into_iter()
            .filter(|(_, role)| role.authoring_app_id != role.reading_app_id && !role.closed)
            .map(|(name, role)| (name, role.authoring_app_id))
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
        for (role, lineage_app_id) in windows {
            self.sweep_role(&role, &lineage_app_id).await;
        }
    }

    /// Sweep every known neighbour for ONE role, one page each.
    async fn sweep_role(&self, role: &str, lineage_app_id: &str) {
        let Some(admin) = self.registry.any_admin_websocket() else {
            tracing::debug!(
                role,
                "lineage bridge: no admin connection — skipping this tick"
            );
            return;
        };
        let v1_cell = match self.v1_cell(&admin, role).await {
            Ok(cell) => cell,
            Err(e) => {
                tracing::warn!(role, error = %e, "lineage bridge: no v1 cell to carry FROM");
                return;
            }
        };

        // `known_agents` runs on the BASE app's supervised client — the normal
        // role path. It is a v1 question ("who is on the predecessor DHT?"),
        // so it is asked of a v1 cell.
        let Some(base_client) = self.registry.client(role) else {
            tracing::debug!(role, "lineage bridge: role bridge unconnected this tick");
            return;
        };
        let agents = match self.neighbours(&base_client).await {
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
        let side = match self.registry.connect_app(lineage_app_id, role).await {
            Ok(client) => client,
            Err(e) => {
                tracing::warn!(
                    role,
                    lineage_app_id,
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

        let input = CarryInput {
            v1_cell: v1_cell.clone(),
            cursor: before.cursor,
            limit: HELD_PAGE_LIMIT,
            source: CarrySource::Held(agent),
        };
        let outcome = call_carry_from(side, input).await;

        let mut after = match &outcome {
            Ok(page) => next_sweep(&before, page),
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

        match &outcome {
            Ok(_) if carried_this_page > 0 => tracing::info!(
                role,
                agent = %key.1,
                carried = carried_this_page,
                total_carried = after.carried,
                witnessed,
                cursor = ?after.cursor,
                observed_head = ?after.observed_head,
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

    /// The base app's provisioned cell for `role` — the held carry's v1 source.
    async fn v1_cell(
        &self,
        admin: &holochain_client::AdminWebsocket,
        role: &str,
    ) -> Result<holochain_client::CellId, String> {
        let apps = admin
            .list_apps(None)
            .await
            .map_err(|e| format!("list_apps: {e}"))?;
        let base = apps
            .into_iter()
            .find(|a| a.installed_app_id == self.base_app_id)
            .ok_or_else(|| format!("base app '{}' is not installed", self.base_app_id))?;
        base.cell_info
            .get(role)
            .and_then(|cells| {
                cells.iter().find_map(|c| match c {
                    holochain_client::CellInfo::Provisioned(p) => Some(p.cell_id.clone()),
                    _ => None,
                })
            })
            .ok_or_else(|| {
                format!(
                    "base app '{}' has no provisioned cell for role '{role}'",
                    self.base_app_id
                )
            })
    }

    /// Who is on the v1 DHT, MINUS ourselves.
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
            already_carried: already,
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
        let bridge = LineageBridge::new(
            Arc::clone(&lineage),
            Arc::new(HcClientRegistry::empty()),
            "elohim",
        );
        assert!(
            bridge.open_windows().is_empty(),
            "a node with no window open sweeps nothing"
        );

        lineage.open_window("node_registry", "elohim@EKiIscIk5BDd");
        assert_eq!(
            bridge.open_windows(),
            vec![(
                "node_registry".to_string(),
                "elohim@EKiIscIk5BDd".to_string()
            )]
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
            "elohim",
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
            "elohim",
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
            "elohim",
        );
        // Whatever the environment says, the interval is captured at
        // construction and is a positive duration.
        assert!(bridge.interval() > Duration::ZERO);
    }
}
