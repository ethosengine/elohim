//! Observability for the Slice-2b provide-loop and the re-anchor backfill.
//!
//! ## Why this exists (the dark-card incident)
//!
//! The EPR resilience card (`GET /api/v1/resilience/{id}/household`) read all
//! zeros on alpha. Two app-layer gates, both invisible without log-scraping:
//!
//! 1. **The provide-loop was dormant.** It authors the `replicates-content`
//!    Commitment whose side-projection writes the `content:<reach>` provide rows
//!    the snapshot counts. It spawns only when `config.self_cid` is non-empty
//!    (main.rs), and `self_cid` was sourced solely from the `SELF_CID` env which
//!    is set in no manifest → permanently off, fleet-wide.
//! 2. **The reach circuit latched provenance-only.** The seed/import hit the
//!    in-pod conductor while its cells were still `CellDisabled`, so content rows
//!    landed with `dht_anchor_hash IS NULL` (never DHT-authored) → reach never
//!    re-notarized → no provide rows.
//!
//! This holder surfaces BOTH on `/p2p/status` so "card dark because loop off /
//! circuit latched" is one HTTP read, not a Loki query.
//!
//! ## Stuck vs draining (the second dark shape)
//!
//! Since the anchor-LIVENESS landing the `/p2p/status` pending count sums BOTH
//! re-anchor arms — never-authored (`remaining`) and dead-anchor
//! (`dead_remaining`). That tightening is correct and stays. But it made one
//! failure mode unreadable: a dead-anchor population that no sweep moves holds
//! `caughtUp=false` forever — indistinguishable, from `/p2p/status` alone, from
//! a healthy heal actively draining a large backlog.
//!
//! So this holder remembers the PREVIOUS sweep's unhealed dead residue and how
//! many consecutive sweeps it has sat unchanged. After
//! [`DEAD_REMAINING_STUCK_SWEEPS`] the surface says `deadRemainingStuck: true`
//! — "this is wedged", not "still healing". The verdict says THAT the residue
//! is wedged, not WHY; the per-arm counts beside it name the arm. Two causes
//! are known, and the first live instance (adam, 2026-09-11, both skip
//! counters at 0) was the second, not the first:
//!
//! - a stored `reach`/`content_type` outside the vocabulary — the sweep's
//!   skip-guards refuse the row every time (`reanchorSkippedReach` /
//!   `reanchorSkippedContentType` non-zero; the cure is a seed-data
//!   correction);
//! - the adopt-before-author pre-flight HOLDING the row every sweep
//!   (`reanchorHeld` + `reanchorHeldBackoff` covering the dead candidates).
//!
//! ## Settled by declaration is not unhealed (F3, 2026-10-07)
//!
//! A row the pre-flight holds because a declaration whose records ARE on this
//! network settles it is not waiting for anything this node can do; it is
//! obeying a canonical channel. Those rows are published as
//! `deadSettledByDeclaration` beside `reanchorDeadRemaining` and are NOT
//! laundered to `live` — their anchor stays dead and `reanchorDeadRemaining`
//! and `reanchorPending` still count them. Only `caughtUp` and the stuck
//! detector read the genuinely-unhealed residue (dead minus settled). A hold
//! that rests on an unanswered probe is never settled: it stays residue. A
//! declaration whose records answered ABSENT is not settled either — the
//! pre-flight releases it to the author path (F5) instead of holding it.
//!
//! ## Awaiting a release channel is not unhealed either
//!
//! A NEVER-AUTHORED row whose slug is bound to a release channel
//! (`metadata.releaseChannel`) is held at the pre-flight's first step: its
//! channel owns its serving pointer, and it authors when that channel adopts.
//! It is not work this loop can do. Those rows are published as
//! `reanchorAwaitingChannel` and leave the `caughtUp` residue exactly as
//! settled-by-declaration rows do; `reanchorPending` still counts them.
//!
//! Category C (operational): a per-process status snapshot, reconstructed in
//! memory, never persisted, never notarized. Mirrors
//! `p2p::projection_reconcile::ProjectionReconcileState`. The cross-sweep
//! memory is deliberately in-memory only — on restart the run counter resets to
//! 0 and the node re-earns the stuck verdict over the next
//! [`DEAD_REMAINING_STUCK_SWEEPS`] sweeps. That reconstruction is honest (a
//! genuinely wedged row is still wedged after the restart and re-trips the
//! verdict; a row healed by a seed correction never does) and it keeps the
//! signal off the DHT and out of any migration.

use std::sync::Arc;

use serde::Serialize;
use tokio::sync::RwLock;
use ts_rs::TS;

/// Consecutive sweeps a NON-ZERO `dead_remaining` must sit at the SAME value
/// before `/p2p/status` calls it stuck rather than draining.
///
/// Three, because the two arms share one sweep budget and the never-authored
/// arm takes first claim on it (`reanchor_backfill::run_once`): a large NULL
/// population can legitimately consume the whole budget for a sweep or two
/// while the dead arm goes untouched, and that is draining, not stuck. Three
/// consecutive identical observations (two no-progress transitions) clears that
/// window while still surfacing a genuine wedge inside a few sweep intervals —
/// long before an operator would otherwise conclude "still healing" from a
/// `caughtUp=false` that never moves.
pub const DEAD_REMAINING_STUCK_SWEEPS: u32 = 3;

/// One re-anchor sweep's publishable result — the input to
/// [`ProvideLoopState::publish_reanchor_sweep`].
///
/// Carries the two `remaining` arms SEPARATELY so the stuck detector can watch
/// the dead arm on its own, while the published `pending` stays their sum
/// (unchanged arithmetic — see the field docs on [`ProvideLoopStatus`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReanchorSweepResult {
    /// Rows brought to a settled state this sweep (re-authored + already
    /// anchored + adopted + held).
    pub completed: usize,
    /// Rows the pre-flight ADOPTED a canonical head for this sweep.
    pub adopted: usize,
    /// Rows the pre-flight HELD (or contested) this sweep.
    pub held: usize,
    /// Rows skipped this sweep on a standing held verdict (no probe paid).
    pub held_backoff: usize,
    /// Rows the pre-flight RELEASED to the author path this sweep because the
    /// records behind their declaration answered absent on this network (F5).
    pub held_unbacked: usize,
    /// Of `held` + `held_backoff`: holds that rest on an unanswered probe or a
    /// DB-pool error. Never settled.
    pub held_unanswered: usize,
    /// DEAD-arm rows whose last pre-flight held them on an ANSWER — settled by
    /// a declaration. Subtracted from `dead_remaining` for `caughtUp` and the
    /// stuck detector only.
    pub dead_settled_by_declaration: usize,
    /// NEVER-AUTHORED rows held because their release channel owns them —
    /// awaiting the channel's adoption, not re-anchor work. Subtracted from
    /// `remaining` for `caughtUp` only.
    pub awaiting_channel: usize,
    /// Rows that errored re-authoring this sweep (retryable).
    pub failed: usize,
    /// NULL-anchor (never-authored) rows remaining after the sweep.
    pub remaining: usize,
    /// DEAD-anchor (anchored-but-unprovable) rows remaining after the sweep.
    pub dead_remaining: usize,
    /// Rows skipped this sweep for a non-canonical `reach` — permanently
    /// un-re-authorable until the seed data is corrected.
    pub skipped_reach: usize,
    /// Rows skipped this sweep for a non-canonical `content_type` — same class,
    /// same fix.
    pub skipped_content_type: usize,
}

impl ReanchorSweepResult {
    /// The `/p2p/status` pending count: BOTH arms, every row. Unchanged
    /// arithmetic — rows settled by declaration are still dead-anchored and are
    /// still counted here.
    pub fn pending(&self) -> usize {
        self.remaining.saturating_add(self.dead_remaining)
    }

    /// Settled-by-declaration rows, clamped to the dead population they are a
    /// subset of (a ghost sweep may revive a row between the loop and the
    /// recount).
    pub fn settled_by_declaration(&self) -> usize {
        self.dead_settled_by_declaration.min(self.dead_remaining)
    }

    /// The genuinely-unhealed dead residue — see [`unhealed_dead_residue`].
    pub fn dead_residue(&self) -> usize {
        unhealed_dead_residue(self.dead_remaining, self.dead_settled_by_declaration)
    }

    /// Never-authored rows awaiting their release channel, clamped to
    /// `remaining` (they are a subset of it).
    pub fn awaiting_channel(&self) -> usize {
        self.awaiting_channel.min(self.remaining)
    }

    /// What `caughtUp` reads: never-authored rows NOT awaiting a release
    /// channel, plus the unhealed dead residue. A node whose only outstanding
    /// rows are settled by declaration or awaiting their channel is caught up;
    /// one with a single genuinely unhealed row is not.
    pub fn unhealed_pending(&self) -> usize {
        self.remaining
            .saturating_sub(self.awaiting_channel)
            .saturating_add(self.dead_residue())
    }
}

/// **Pure.** The dead rows that are genuinely unhealed: `dead_remaining` minus
/// those settled by a declaration (F3). Saturating, so a settled count that
/// overshoots the recount (a row revived between loop and recount) can never
/// underflow into a false "caught up" — it reads 0 only when every dead row is
/// settled.
pub fn unhealed_dead_residue(dead_remaining: usize, dead_settled_by_declaration: usize) -> usize {
    dead_remaining.saturating_sub(dead_settled_by_declaration)
}

/// Fold this sweep's `dead_remaining` into the consecutive-unchanged run
/// length. Pure + total, so the whole stuck-detection contract is unit-testable
/// with no sweep, no conductor, and no clock.
///
/// - `current == 0` resets the run to 0: nothing is outstanding, so nothing can
///   be stuck, and a later non-zero population starts its run clean.
/// - unchanged from `previous` extends the run.
/// - any change (progress OR growth) restarts the run at 1: the population
///   moved, which is evidence of a live loop either way.
pub fn next_unchanged_sweeps(previous: Option<usize>, previous_run: u32, current: usize) -> u32 {
    if current == 0 {
        0
    } else if previous == Some(current) {
        previous_run.saturating_add(1)
    } else {
        1
    }
}

/// Verdict: is the unhealed dead residue WEDGED rather than draining? Pure +
/// total. It says THAT, not WHY — see the module doc for the two known causes
/// and the counters that tell them apart.
///
/// True exactly when a non-zero residue has been observed at the same value
/// for at least [`DEAD_REMAINING_STUCK_SWEEPS`] consecutive sweeps. A zero
/// residue is never stuck (it is done, or settled by declaration), and a moving
/// one is never stuck (it is draining).
pub fn is_dead_remaining_stuck(dead_remaining: usize, unchanged_sweeps: u32) -> bool {
    dead_remaining > 0 && unchanged_sweeps >= DEAD_REMAINING_STUCK_SWEEPS
}

/// Where `self_cid` came from at boot — the load-bearing gate on whether the
/// provide-loop can spawn at all.
///
/// Serialized as the `selfCidSource` string on `/p2p/status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfCidSource {
    /// `SELF_CID` env was set and non-empty.
    Env,
    /// Derived at startup from the active transport's identity key — the libp2p
    /// `NodeIdentity` peer id OR the iroh `NodeId` (both are the join key the
    /// seeder resolves from `/p2p/status .peerId`). Resolved through the
    /// `NodeTransport` seam, so it covers BOTH backends.
    ///
    /// The wire string stays `"derived-libp2p-peer-id"` for back-compat: the
    /// `p2p-status-view` schema enum (in `elohim/sdk/schemas/`) is unchanged,
    /// and libp2p output is byte-identical. The string is historical; the
    /// meaning is "derived from the active transport's key."
    DerivedFromTransport,
    /// Neither — the provide-loop stays dormant (the dark-card cause).
    Unset,
}

impl SelfCidSource {
    pub fn as_str(self) -> &'static str {
        match self {
            SelfCidSource::Env => "env",
            // Wire value frozen for schema/back-compat (see variant docs).
            SelfCidSource::DerivedFromTransport => "derived-libp2p-peer-id",
            SelfCidSource::Unset => "unset",
        }
    }
}

/// Provide-loop + re-anchor-backfill status, exposed via `/p2p/status`.
///
/// Wire format: the `provideLoop` property of
/// `elohim/sdk/schemas/v1/views/p2p-status-view.schema.json` (an inline object,
/// mirroring the `projectionReconcile`/`pull` precedents). Schema contract test:
/// the `p2p_status_view_*` cases in `tests/schema_contract.rs`.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ProvideLoopStatus {
    /// Where `self_cid` came from: `env` | `derived-libp2p-peer-id` | `unset`.
    pub self_cid_source: String,
    /// True once the Slice-2b provide-loop authoring tick has spawned. False
    /// means dormant — the card cannot light its `content:<reach>` counts.
    pub active: bool,
    /// Re-anchor backfill: content rows still lacking `dht_anchor_hash` at the
    /// last sweep (candidates that could not yet be re-authored).
    #[ts(type = "number")]
    pub reanchor_pending: usize,
    /// Re-anchor backfill: rows successfully re-authored this process lifetime.
    #[ts(type = "number")]
    pub reanchor_completed: usize,
    /// Re-anchor backfill: rows that errored re-authoring (non-fatal, retried on
    /// a future boot's sweep).
    #[ts(type = "number")]
    pub reanchor_failed: usize,
    /// True when the re-anchor backfill has run AND nothing is left unhealed on
    /// either arm: no NULL-anchor row to author outside
    /// `reanchorAwaitingChannel`, and no dead-anchor row outside
    /// `deadSettledByDeclaration`. False before the first run or while
    /// unhealed candidates remain.
    pub reanchor_caught_up: bool,
    /// The DEAD-anchor arm of `reanchorPending` on its own: rows anchored under
    /// an action no living chain can present, remaining after the last sweep.
    /// `reanchorPending` is still the SUM of both arms — this field splits it so
    /// "nothing to author, something to re-adopt" is directly readable.
    #[ts(type = "number")]
    pub reanchor_dead_remaining: usize,
    /// Consecutive sweeps the unhealed dead residue (`reanchorDeadRemaining`
    /// minus `deadSettledByDeclaration`) has sat at the same non-zero value. 0
    /// while the residue is empty or moving. Watch it climb 1 → 2 → 3 to see a
    /// heal stall in progress.
    pub stuck_sweeps: u32,
    /// True when the unhealed dead residue is WEDGED rather than draining:
    /// non-zero for at least [`DEAD_REMAINING_STUCK_SWEEPS`] consecutive
    /// sweeps at the same value. Read this as *wedged*, not *still healing* —
    /// `caughtUp` will not move on its own. The verdict does not name the
    /// cause; the per-arm counts do: non-zero `reanchorSkippedReach` /
    /// `reanchorSkippedContentType` mean a seed-data correction is needed,
    /// while `reanchorHeld` / `reanchorHeldBackoff` covering the dead
    /// candidates (with `reanchorHeldUnanswered` beside them) mean the
    /// adopt-before-author pre-flight is holding the rows on probes nobody
    /// answered. Rows settled by a declaration never count toward it.
    pub dead_remaining_stuck: bool,
    /// Rows the LAST sweep skipped for a non-canonical `reach` (the DNA rejects
    /// them, so they are never re-authorable). Non-zero beside
    /// `deadRemainingStuck` names the wedge.
    #[ts(type = "number")]
    pub reanchor_skipped_reach: usize,
    /// Rows the LAST sweep skipped for a non-canonical `content_type`. Same
    /// class as `reanchorSkippedReach`, same fix (correct the seed data).
    #[ts(type = "number")]
    pub reanchor_skipped_content_type: usize,
    /// Rows the LAST sweep's pre-flight ADOPTED a canonical head for (no root
    /// minted).
    #[ts(type = "number")]
    pub reanchor_adopted: usize,
    /// Rows the LAST sweep's pre-flight HELD or contested — neither adopted nor
    /// authored.
    #[ts(type = "number")]
    pub reanchor_held: usize,
    /// Rows the LAST sweep skipped on a standing held verdict inside the
    /// held-backoff window (no probe paid). Still unhealed unless settled.
    #[ts(type = "number")]
    pub reanchor_held_backoff: usize,
    /// Rows the LAST sweep's pre-flight released to the author path because
    /// the records behind their declaration answered ABSENT on this network —
    /// a declaration pointing at a network that no longer exists (F5).
    #[ts(type = "number")]
    pub reanchor_held_unbacked: usize,
    /// Of `reanchorHeld` + `reanchorHeldBackoff`: holds that rest on an
    /// unanswered probe or a DB-pool error. Never counted as settled.
    #[ts(type = "number")]
    pub reanchor_held_unanswered: usize,
    /// Dead-anchor rows (of `reanchorDeadRemaining`) whose last pre-flight held
    /// them on an ANSWER — settled by a declaration whose records are on this
    /// network. Their anchor stays dead (never laundered to live) and they stay
    /// in `reanchorDeadRemaining` / `reanchorPending`; `caughtUp` and the stuck
    /// detector read only the residue beside them.
    #[ts(type = "number")]
    pub dead_settled_by_declaration: usize,
    /// Never-authored rows (of `reanchorPending`'s NULL arm) the LAST sweep held
    /// because their slug is bound to a release channel
    /// (`metadata.releaseChannel`), which owns their serving pointer. Not
    /// re-anchor work: each authors when its channel adopts. They stay in
    /// `reanchorPending`; `caughtUp` reads only the rows beside them.
    #[ts(type = "number")]
    pub reanchor_awaiting_channel: usize,
}

impl Default for ProvideLoopStatus {
    fn default() -> Self {
        Self {
            self_cid_source: SelfCidSource::Unset.as_str().to_string(),
            active: false,
            reanchor_pending: 0,
            reanchor_completed: 0,
            reanchor_failed: 0,
            reanchor_caught_up: false,
            reanchor_dead_remaining: 0,
            stuck_sweeps: 0,
            dead_remaining_stuck: false,
            reanchor_skipped_reach: 0,
            reanchor_skipped_content_type: 0,
            reanchor_adopted: 0,
            reanchor_held: 0,
            reanchor_held_backoff: 0,
            reanchor_held_unbacked: 0,
            reanchor_held_unanswered: 0,
            dead_settled_by_declaration: 0,
            reanchor_awaiting_channel: 0,
        }
    }
}

/// The published status plus the cross-sweep memory the stuck detector needs.
///
/// Ephemeral (Category C), in-memory only: `prev_dead_remaining` is node-local
/// operational state, reconstructible by observation, never persisted and never
/// notarized. It lives under the SAME lock as the status it derives, so a
/// reader can never catch a verdict that disagrees with the count it was
/// computed from.
#[derive(Debug, Default)]
struct ProvideLoopInner {
    status: ProvideLoopStatus,
    /// The previous sweep's unhealed dead residue. `None` before the first
    /// sweep.
    prev_dead_remaining: Option<usize>,
}

/// Thread-safe holder for the provide-loop status snapshot. Created in the
/// composition root (main.rs), written by the boot path (self_cid derive +
/// loop spawn) and the re-anchor backfill, read by `/p2p/status`.
#[derive(Debug, Clone, Default)]
pub struct ProvideLoopState {
    inner: Arc<RwLock<ProvideLoopInner>>,
}

impl ProvideLoopState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot for `/p2p/status`.
    pub async fn status(&self) -> ProvideLoopStatus {
        self.inner.read().await.status.clone()
    }

    /// Record where `self_cid` resolved from (boot path).
    pub async fn set_self_cid_source(&self, source: SelfCidSource) {
        self.inner.write().await.status.self_cid_source = source.as_str().to_string();
    }

    /// Mark the provide-loop spawned (or confirm it stayed dormant).
    pub async fn set_active(&self, active: bool) {
        self.inner.write().await.status.active = active;
    }

    /// Publish the result of one re-anchor backfill sweep.
    ///
    /// `pending` is BOTH remaining arms summed, every row. `caught_up` reads
    /// the UNHEALED pending only — never-authored rows outside
    /// `awaiting_channel`, plus the dead residue outside
    /// `dead_settled_by_declaration` (F3). `completed`/`failed` advance
    /// the cumulative counters; the remaining/skip/per-arm counts reflect the
    /// LATEST sweep only.
    ///
    /// Additionally folds the dead residue into the cross-sweep stuck detector:
    /// the unchanged-run counter advances via [`next_unchanged_sweeps`] and the
    /// verdict via [`is_dead_remaining_stuck`], both pure.
    pub async fn publish_reanchor_sweep(&self, result: ReanchorSweepResult) {
        let pending = result.pending();
        let residue = result.dead_residue();
        let mut inner = self.inner.write().await;

        let run = next_unchanged_sweeps(
            inner.prev_dead_remaining,
            inner.status.stuck_sweeps,
            residue,
        );
        inner.prev_dead_remaining = Some(residue);

        let s = &mut inner.status;
        s.reanchor_completed = s.reanchor_completed.saturating_add(result.completed);
        s.reanchor_failed = s.reanchor_failed.saturating_add(result.failed);
        s.reanchor_pending = pending;
        s.reanchor_caught_up = result.unhealed_pending() == 0;
        s.reanchor_dead_remaining = result.dead_remaining;
        s.stuck_sweeps = run;
        s.dead_remaining_stuck = is_dead_remaining_stuck(residue, run);
        s.reanchor_skipped_reach = result.skipped_reach;
        s.reanchor_skipped_content_type = result.skipped_content_type;
        s.reanchor_adopted = result.adopted;
        s.reanchor_held = result.held;
        s.reanchor_held_backoff = result.held_backoff;
        s.reanchor_held_unbacked = result.held_unbacked;
        s.reanchor_held_unanswered = result.held_unanswered;
        s.dead_settled_by_declaration = result.settled_by_declaration();
        s.reanchor_awaiting_channel = result.awaiting_channel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_cid_source_strings_are_stable_wire_values() {
        assert_eq!(SelfCidSource::Env.as_str(), "env");
        assert_eq!(
            SelfCidSource::DerivedFromTransport.as_str(),
            "derived-libp2p-peer-id"
        );
        assert_eq!(SelfCidSource::Unset.as_str(), "unset");
    }

    #[test]
    fn default_status_reads_dormant_unset() {
        let s = ProvideLoopStatus::default();
        assert_eq!(s.self_cid_source, "unset");
        assert!(!s.active);
        assert!(!s.reanchor_caught_up);
        assert_eq!(s.reanchor_completed, 0);
        // Before the first sweep nothing is known to be wedged — the stuck
        // signal starts silent, never asserting a verdict it has not earned.
        assert!(!s.dead_remaining_stuck);
        assert_eq!(s.stuck_sweeps, 0);
        assert_eq!(s.reanchor_dead_remaining, 0);
    }

    /// Terse builder so the sweep-sequence tests read as the sequence they are.
    fn sweep(remaining: usize, dead_remaining: usize) -> ReanchorSweepResult {
        ReanchorSweepResult {
            remaining,
            dead_remaining,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn set_and_snapshot_roundtrip() {
        let state = ProvideLoopState::new();
        state
            .set_self_cid_source(SelfCidSource::DerivedFromTransport)
            .await;
        state.set_active(true).await;
        state
            .publish_reanchor_sweep(ReanchorSweepResult {
                completed: 3,
                failed: 1,
                ..Default::default()
            })
            .await;

        let snap = state.status().await;
        assert_eq!(snap.self_cid_source, "derived-libp2p-peer-id");
        assert!(snap.active);
        assert_eq!(snap.reanchor_completed, 3);
        assert_eq!(snap.reanchor_failed, 1);
        assert_eq!(snap.reanchor_pending, 0);
        assert!(snap.reanchor_caught_up);
    }

    #[tokio::test]
    async fn reanchor_sweeps_accumulate_completed_and_failed() {
        let state = ProvideLoopState::new();
        state
            .publish_reanchor_sweep(ReanchorSweepResult {
                completed: 2,
                remaining: 5,
                ..Default::default()
            })
            .await;
        state
            .publish_reanchor_sweep(ReanchorSweepResult {
                completed: 5,
                failed: 1,
                ..Default::default()
            })
            .await;

        let snap = state.status().await;
        // Cumulative across sweeps.
        assert_eq!(snap.reanchor_completed, 7);
        assert_eq!(snap.reanchor_failed, 1);
        // pending + caught_up reflect the LATEST sweep.
        assert_eq!(snap.reanchor_pending, 0);
        assert!(snap.reanchor_caught_up);
    }

    // ── stuck vs draining ───────────────────────────────────────────

    #[test]
    fn pending_is_still_the_sum_of_both_arms() {
        // The tightening this observability split must NOT relax: a node with
        // nothing left to author and a standing dead population is not caught up.
        assert_eq!(sweep(0, 5).pending(), 5);
        assert_eq!(sweep(4, 5).pending(), 9);
        assert_eq!(sweep(0, 0).pending(), 0);
    }

    #[test]
    fn unchanged_run_advances_only_while_the_value_holds() {
        // First observation of a non-zero population starts the run at 1.
        assert_eq!(next_unchanged_sweeps(None, 0, 7), 1);
        // Same value again extends it.
        assert_eq!(next_unchanged_sweeps(Some(7), 1, 7), 2);
        assert_eq!(next_unchanged_sweeps(Some(7), 2, 7), 3);
        // Progress restarts the run — the loop is alive.
        assert_eq!(next_unchanged_sweeps(Some(7), 9, 6), 1);
        // So does GROWTH: the population moved either way, which is not a wedge.
        assert_eq!(next_unchanged_sweeps(Some(7), 9, 8), 1);
        // Drained to zero resets, so a later population starts its run clean.
        assert_eq!(next_unchanged_sweeps(Some(7), 9, 0), 0);
        assert_eq!(next_unchanged_sweeps(Some(0), 0, 3), 1);
    }

    #[test]
    fn stuck_verdict_needs_a_nonzero_population_held_past_the_threshold() {
        // Empty is DONE, never stuck — however long it has been empty.
        assert!(!is_dead_remaining_stuck(0, DEAD_REMAINING_STUCK_SWEEPS));
        assert!(!is_dead_remaining_stuck(0, 99));
        // Below the threshold reads as draining (the shared-budget window).
        for run in 0..DEAD_REMAINING_STUCK_SWEEPS {
            assert!(
                !is_dead_remaining_stuck(5, run),
                "run {run} is still inside the draining window"
            );
        }
        // At and past the threshold: wedged.
        assert!(is_dead_remaining_stuck(5, DEAD_REMAINING_STUCK_SWEEPS));
        assert!(is_dead_remaining_stuck(1, DEAD_REMAINING_STUCK_SWEEPS + 4));
    }

    #[tokio::test]
    async fn a_wedged_dead_population_reads_stuck_after_n_sweeps() {
        // The skip-guarded row's shape: nothing left to author, the same dead
        // rows every sweep, both skip counters naming why.
        let state = ProvideLoopState::new();
        let wedged = ReanchorSweepResult {
            dead_remaining: 2,
            skipped_reach: 1,
            skipped_content_type: 1,
            ..Default::default()
        };

        for expected_run in 1..DEAD_REMAINING_STUCK_SWEEPS {
            state.publish_reanchor_sweep(wedged).await;
            let snap = state.status().await;
            assert_eq!(snap.stuck_sweeps, expected_run);
            assert!(
                !snap.dead_remaining_stuck,
                "must still read as draining at run {expected_run}"
            );
            // The tightening holds the whole time.
            assert_eq!(snap.reanchor_pending, 2);
            assert!(!snap.reanchor_caught_up);
        }

        state.publish_reanchor_sweep(wedged).await;
        let snap = state.status().await;
        assert_eq!(snap.stuck_sweeps, DEAD_REMAINING_STUCK_SWEEPS);
        assert!(
            snap.dead_remaining_stuck,
            "an unmoved dead population past the threshold is a seed-data fix, \
             not a heal in progress"
        );
        // caughtUp is UNCHANGED by the verdict — this is an observability
        // split, not a gate loosening.
        assert!(!snap.reanchor_caught_up);
        assert_eq!(snap.reanchor_pending, 2);
        assert_eq!(snap.reanchor_dead_remaining, 2);
        // …and the skip counts name the wedge.
        assert_eq!(snap.reanchor_skipped_reach, 1);
        assert_eq!(snap.reanchor_skipped_content_type, 1);
    }

    #[tokio::test]
    async fn a_draining_dead_population_never_reads_stuck() {
        // Same total sweep count as the wedged case, but the number moves —
        // this is the healthy heal the stuck signal must not slander.
        let state = ProvideLoopState::new();
        for dead in [9usize, 7, 4, 2, 0] {
            state.publish_reanchor_sweep(sweep(0, dead)).await;
            let snap = state.status().await;
            assert!(
                !snap.dead_remaining_stuck,
                "a moving population is draining, not stuck (dead={dead})"
            );
        }
        let snap = state.status().await;
        assert_eq!(snap.stuck_sweeps, 0);
        assert!(snap.reanchor_caught_up);
    }

    #[tokio::test]
    async fn healing_clears_a_standing_stuck_verdict() {
        // A seed-data correction lands mid-run: the verdict must drop the moment
        // the population moves, not linger until restart.
        let state = ProvideLoopState::new();
        for _ in 0..=DEAD_REMAINING_STUCK_SWEEPS {
            state.publish_reanchor_sweep(sweep(0, 3)).await;
        }
        assert!(state.status().await.dead_remaining_stuck);

        state.publish_reanchor_sweep(sweep(0, 0)).await;
        let snap = state.status().await;
        assert!(!snap.dead_remaining_stuck);
        assert_eq!(snap.stuck_sweeps, 0);
        assert_eq!(snap.reanchor_dead_remaining, 0);
        assert!(snap.reanchor_caught_up);
    }

    #[tokio::test]
    async fn a_never_authored_backlog_alone_is_never_stuck() {
        // `remaining` (first authorship) has its own retry story; the stuck
        // detector watches ONLY the dead arm, so a frozen NULL population must
        // not trip it.
        let state = ProvideLoopState::new();
        for _ in 0..(DEAD_REMAINING_STUCK_SWEEPS + 2) {
            state.publish_reanchor_sweep(sweep(11, 0)).await;
        }
        let snap = state.status().await;
        assert!(!snap.dead_remaining_stuck);
        assert_eq!(snap.stuck_sweeps, 0);
        assert_eq!(snap.reanchor_pending, 11);
        assert!(!snap.reanchor_caught_up);
    }

    // ── F3: settled by declaration is not unhealed ──────────────────

    #[test]
    fn residue_is_dead_minus_settled_and_never_underflows() {
        assert_eq!(unhealed_dead_residue(5, 0), 5);
        assert_eq!(unhealed_dead_residue(5, 3), 2);
        assert_eq!(unhealed_dead_residue(5, 5), 0);
        // A row revived between the loop and the recount can leave the settled
        // count above the recount: saturate, never wrap into a huge residue.
        assert_eq!(unhealed_dead_residue(5, 7), 0);
        assert_eq!(unhealed_dead_residue(0, 0), 0);

        let r = ReanchorSweepResult {
            remaining: 2,
            dead_remaining: 5,
            dead_settled_by_declaration: 7,
            ..Default::default()
        };
        assert_eq!(
            r.pending(),
            7,
            "pending still counts every row on both arms"
        );
        assert_eq!(
            r.settled_by_declaration(),
            5,
            "clamped to the dead population"
        );
        assert_eq!(r.dead_residue(), 0);
        assert_eq!(
            r.unhealed_pending(),
            2,
            "the NULL arm is never settled away"
        );
    }

    #[tokio::test]
    async fn an_all_settled_dead_population_reads_caught_up_and_never_stuck() {
        // The household after F5: every dead row that remains is held by a
        // declaration whose records ARE here. Not laundered — still dead, still
        // pending — but nothing is waiting on this node.
        let state = ProvideLoopState::new();
        let settled = ReanchorSweepResult {
            dead_remaining: 4,
            held_backoff: 4,
            dead_settled_by_declaration: 4,
            ..Default::default()
        };
        for _ in 0..(DEAD_REMAINING_STUCK_SWEEPS + 2) {
            state.publish_reanchor_sweep(settled).await;
        }
        let snap = state.status().await;
        assert!(snap.reanchor_caught_up);
        assert!(!snap.dead_remaining_stuck);
        assert_eq!(snap.stuck_sweeps, 0);
        assert_eq!(snap.reanchor_dead_remaining, 4, "never laundered to live");
        assert_eq!(snap.reanchor_pending, 4, "pending still counts them");
        assert_eq!(snap.dead_settled_by_declaration, 4);
        assert_eq!(snap.reanchor_held_backoff, 4);
    }

    #[tokio::test]
    async fn an_unsettled_residue_beside_settled_rows_still_reads_stuck() {
        // Two settled rows must not hide three genuinely unhealed ones.
        let state = ProvideLoopState::new();
        let mixed = ReanchorSweepResult {
            dead_remaining: 5,
            held: 5,
            held_unanswered: 3,
            dead_settled_by_declaration: 2,
            ..Default::default()
        };
        for _ in 0..DEAD_REMAINING_STUCK_SWEEPS {
            state.publish_reanchor_sweep(mixed).await;
        }
        let snap = state.status().await;
        assert!(!snap.reanchor_caught_up);
        assert!(snap.dead_remaining_stuck);
        assert_eq!(snap.stuck_sweeps, DEAD_REMAINING_STUCK_SWEEPS);
        assert_eq!(snap.dead_settled_by_declaration, 2);
        assert_eq!(snap.reanchor_held_unanswered, 3);
    }

    /// Refinement B: never-authored rows awaiting their release channel leave
    /// the `caughtUp` residue exactly as settled-by-declaration rows do, while
    /// `reanchorPending` stays raw and the stuck detector (dead arm only) is
    /// untouched.
    #[tokio::test]
    async fn rows_awaiting_their_channel_are_not_unhealed_work() {
        let r = ReanchorSweepResult {
            remaining: 13,
            awaiting_channel: 13,
            ..Default::default()
        };
        assert_eq!(r.pending(), 13);
        assert_eq!(r.unhealed_pending(), 0);
        // One genuinely unhealed NULL row beside them still blocks caughtUp.
        let r = ReanchorSweepResult {
            remaining: 14,
            awaiting_channel: 13,
            ..Default::default()
        };
        assert_eq!(r.unhealed_pending(), 1);
        // Overshoot clamps to the NULL population, never underflows.
        let r = ReanchorSweepResult {
            remaining: 2,
            awaiting_channel: 5,
            dead_remaining: 3,
            dead_settled_by_declaration: 1,
            ..Default::default()
        };
        assert_eq!(r.awaiting_channel(), 2);
        assert_eq!(r.unhealed_pending(), 2, "only the dead residue is left");

        // The household shape: 13 channel-bound NULL rows, every dead row
        // settled — caught up, not stuck, pending raw.
        let state = ProvideLoopState::new();
        let household = ReanchorSweepResult {
            remaining: 13,
            awaiting_channel: 13,
            dead_remaining: 2,
            dead_settled_by_declaration: 2,
            ..Default::default()
        };
        for _ in 0..(DEAD_REMAINING_STUCK_SWEEPS + 1) {
            state.publish_reanchor_sweep(household).await;
        }
        let snap = state.status().await;
        assert!(snap.reanchor_caught_up);
        assert!(!snap.dead_remaining_stuck);
        assert_eq!(snap.reanchor_pending, 15);
        assert_eq!(snap.reanchor_awaiting_channel, 13);
    }

    #[tokio::test]
    async fn per_arm_counts_are_the_latest_sweep_not_cumulative() {
        let state = ProvideLoopState::new();
        state
            .publish_reanchor_sweep(ReanchorSweepResult {
                adopted: 3,
                held: 2,
                held_backoff: 1,
                held_unbacked: 4,
                held_unanswered: 1,
                ..Default::default()
            })
            .await;
        state
            .publish_reanchor_sweep(ReanchorSweepResult {
                adopted: 1,
                ..Default::default()
            })
            .await;
        let snap = state.status().await;
        assert_eq!(snap.reanchor_adopted, 1);
        assert_eq!(snap.reanchor_held, 0);
        assert_eq!(snap.reanchor_held_backoff, 0);
        assert_eq!(snap.reanchor_held_unbacked, 0);
        assert_eq!(snap.reanchor_held_unanswered, 0);
    }
}
