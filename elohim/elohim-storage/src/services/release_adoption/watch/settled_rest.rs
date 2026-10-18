//! The settled rest — a channel with nothing left to do is not re-resolved
//! every sweep.
//!
//! # The cost this removes
//!
//! `sweep_once` resolves every followed channel's head once a tick. On a
//! converged peer the answer is the same head with the same verdict every
//! minute: one `resolve_content_head_local` per channel per sweep, forever, to
//! re-learn nothing. A channel whose last check ended at one of the two
//! SETTLED exits of `check_channel` — the C6b idempotence exit with its
//! pointers read and in place, or a head that carries no release — is instead
//! skipped for [`RESTED_SWEEPS`] sweeps and asked again on the next, so it
//! costs one resolve per `RESTED_SWEEPS + 1` sweeps.
//!
//! # The bound, and what it is counted in
//!
//! With no poke at all, a head that moves is read no later than the third
//! sweep after it moved: 180 s at the default cadence, which leaves 60 s of
//! the 240 s a fleet is given to converge for verify and apply. That is the
//! whole reason the rest is two sweeps and not more — the pokes below are the
//! fast path, never the guarantee.
//!
//! The bound is in SWEEPS, not seconds. The sweep timer is
//! `MissedTickBehavior::Skip` and channels are checked serially, so a stalled
//! runtime or a slow check stretches a sweep, and with it the rest — exactly
//! as it already stretched the every-sweep resolve.
//!
//! # What ends a rest at once
//!
//! - **a poke** ([`poke`]): a peer's head hint for the channel's content id
//!   reached this node's sync plane, or this node's own conductor signalled a
//!   head declaration for it. The channel is then ALERT for [`ALERT_SWEEPS`]
//!   sweeps: checked every sweep, and no rest may be recorded — a hint can
//!   outrun gossip, so the first resolve after a poke may still answer the old
//!   head. The alert is re-read when a rest is about to be recorded, so a poke
//!   that lands while a check is in flight leaves none. A poke never supplies
//!   an answer (I1): the resolve still goes through this node's own conductor.
//! - **the recorded row no longer names the rested head**: `reconcile_followed`
//!   resets a channel's row on a mode change and drops it on an unfollow, so
//!   both end the rest here with no clause of their own.
//! - **an open lineage window** whose origin is this channel: the revert arm is
//!   watching that head for a re-election, so the channel is not at rest and
//!   no rest is recorded for it while the window stands.
//! - **a pointer that drifted, or could not be read**: the C6b exit's own
//!   installed-reality check (a DB read) still runs every sweep against the
//!   metadata the rest carries. A moved pointer sends the channel straight
//!   back through `check_channel`; so does a read that failed, because an
//!   unread pointer is not a verified one.
//! - **a restart**: the memo is in-process.
//!
//! Every other verdict — a refusal (the ladder in
//! [`super::state::BACKOFF_LADDER_SECS`] paces it), `Waiting`, an observe-mode
//! `Ok` (an observer keeps re-deriving), a fresh apply, an absent head, a
//! canary whose candidate is already applied (that exit holds no pointer
//! metadata to keep checking) or could not be read — records nothing, so the
//! channel is asked again on the next sweep exactly as before.
//!
//! # What it costs
//!
//! A newly elected release on a channel that was settled is noticed on the
//! next sweep after a poke — the same minute it always was. When BOTH poke
//! paths miss (the author's sync doc has not reached this node and this
//! node's own conductor did not signal the declaration), it is noticed up to
//! `RESTED_SWEEPS` sweeps later than before. The C6b exit's installed-reality
//! refresh now runs on the asking sweeps only, so that snapshot on
//! `/admin/adoption` can be up to one rest older than its TTL.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, MutexGuard};

use super::*;

/// Sweeps skipped between two asks of a settled channel: it is asked every
/// third sweep. Sized against the fleet's 240 s convergence budget, not
/// against the cost it saves — see the module docs before raising it.
pub const RESTED_SWEEPS: u32 = 2;

/// Sweeps a poked channel stays alert: checked every sweep, never rested. Long
/// enough for the DHT record behind a hint to arrive by ordinary gossip.
pub const ALERT_SWEEPS: u32 = 5;

/// What a settled exit of `check_channel` hands the sweep.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct Settled {
    /// The applied head's `metadata_json`, from the C6b exit only: what the
    /// per-sweep pointer check needs while the head itself is not re-read.
    pub(super) applied_metadata_json: Option<String>,
}

/// One channel's rest: the head it settled on, and how much of it is left.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ChannelRest {
    head: ResolvedHead,
    skips_left: u32,
    applied_metadata_json: Option<String>,
}

/// Channels resting on a settled check, kept on the controller (a restart asks
/// everyone once) and bounded by the follow set
/// ([`AdoptionController::prune_rests`]).
#[derive(Debug, Default)]
pub(super) struct RestMemo {
    resting: Mutex<HashMap<String, ChannelRest>>,
}

impl RestMemo {
    /// The memo, recovered if a holder panicked: a map of counters has no
    /// invariant a panic can break, and the sweep must outlive one.
    fn lock(&self) -> MutexGuard<'_, HashMap<String, ChannelRest>> {
        self.resting.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// What the sweep does with a channel this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RestGate {
    /// Still resting: no resolve, no state transition.
    Rested,
    /// Check now. `alert` forbids recording a rest from this check.
    Check { alert: bool },
}

/// Channels on alert after a [`poke`]: channel id → alert sweeps left.
/// Process-wide, like the registry in [`super::state`], because the pokers
/// (the sync apply and the signal projection) hold no controller.
static ALERTS: LazyLock<Mutex<HashMap<String, u32>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// The alert map, recovered if a holder panicked — the pokers are on the sync
/// apply and signal paths and must never inherit a panic from here (the same
/// posture as `TriggerGate::claim`).
fn alerts() -> MutexGuard<'static, HashMap<String, u32>> {
    ALERTS.lock().unwrap_or_else(|e| e.into_inner())
}

/// A head for `content_id` may have moved: if it is a followed channel, end
/// its rest and hold it alert for [`ALERT_SWEEPS`] sweeps.
///
/// One map lookup and, for a followed channel, one insert. Never awaits, never
/// fails and never panics, so it is safe on the sync apply path. A content id
/// that is not a followed channel is a no-op and is not remembered. The poke
/// carries no head: it only decides WHEN this node asks its own conductor.
pub fn poke(content_id: &str) {
    #[cfg(test)]
    poke_capture::record(content_id);
    if !state::is_followed(content_id) {
        return;
    }
    alerts().insert(content_id.to_string(), ALERT_SWEEPS);
}

/// Is `channel_id` on alert this sweep? Consumes one sweep of the alert.
fn take_alert(channel_id: &str) -> bool {
    let mut alerts = alerts();
    match alerts.get_mut(channel_id) {
        Some(left) if *left > 1 => {
            *left -= 1;
            true
        }
        Some(_) => {
            alerts.remove(channel_id);
            true
        }
        None => false,
    }
}

/// Is an alert standing for `channel_id` right now? Reads, consumes nothing.
fn alert_standing(channel_id: &str) -> bool {
    alerts().contains_key(channel_id)
}

/// Test-only capture of [`poke`] calls. Thread local, so parallel tests never
/// see each other's pokes — the same shape as
/// `rea_projection::touch_capture`.
#[cfg(test)]
pub(crate) mod poke_capture {
    use std::cell::RefCell;

    thread_local! {
        static POKES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn record(id: &str) {
        POKES.with(|p| p.borrow_mut().push(id.to_string()));
    }

    /// The pokes made on this thread since the last drain.
    pub(crate) fn drain() -> Vec<String> {
        POKES.with(|p| std::mem::take(&mut *p.borrow_mut()))
    }
}

impl AdoptionController {
    /// Drop rests and alerts for channels that left the follow set, so neither
    /// map outlives the config that named its keys.
    pub(super) fn prune_rests(&self, followed: &FollowedChannels) {
        let keep = |id: &String| followed.channels.iter().any(|c| &c.channel_id == id);
        self.settled_rest.lock().retain(|id, _| keep(id));
        alerts().retain(|id, _| keep(id));
    }

    /// A tick with nothing followed: no sweep runs, but the registry and both
    /// maps must still let go of whatever was just unfollowed.
    pub(super) fn unfollowed_tick(&self, followed: &FollowedChannels) {
        state::reconcile_followed(followed);
        self.prune_rests(followed);
    }

    /// Rest this channel this sweep, or check it? Forgets the rest on every
    /// path that says the channel is no longer where the rest left it.
    pub(super) fn rest_gate(
        &self,
        channel: &FollowedChannel,
        existing: Option<&state::ChannelAdoptionState>,
    ) -> RestGate {
        let id = &channel.channel_id;
        if take_alert(id) {
            // Poked: something says the head may have moved.
            self.settled_rest.lock().remove(id);
            return RestGate::Check { alert: true };
        }
        // Copied out, so the DB read and the lineage read below run with the
        // memo unlocked. Only this sweep task writes the memo.
        let Some(rest) = self.settled_rest.lock().get(id).cloned() else {
            return RestGate::Check { alert: false };
        };
        // The rest is about the head the recorded row names. A row reset by a
        // mode change, or re-created by a re-follow, names none.
        let row_names_rested_head =
            existing.and_then(|row| row.resolved_head.as_ref()) == Some(&rest.head);
        // `None` is a pointer read that failed — not the same as "in place".
        let pointers_in_place = rest
            .applied_metadata_json
            .as_deref()
            .is_none_or(|metadata| {
                self.read_applied_pointer_drift(metadata)
                    .is_some_and(|drifted| drifted.is_empty())
            });
        let window_open = self.origin_of_open_window(id);
        let mut resting = self.settled_rest.lock();
        if !row_names_rested_head || window_open || !pointers_in_place || rest.skips_left == 0 {
            // The last clause is the rest simply running out: the check that
            // follows is the asking sweep, and it records a fresh rest only if
            // it settles again.
            resting.remove(id);
            return RestGate::Check { alert: false };
        }
        if let Some(rest) = resting.get_mut(id) {
            rest.skips_left -= 1;
        }
        RestGate::Rested
    }

    /// Record a rest after a completed check — only when the check ended at a
    /// settled exit with a resolved head, and never for a channel that is
    /// alert or is the origin of an open lineage window. Both are read NOW,
    /// not taken from the gate: a poke can land, and a window can open, while
    /// the check is in flight. Every other completed check leaves no rest
    /// behind: `rest_gate` removed any earlier one before the check ran.
    pub(super) fn record_rest(
        &self,
        channel_id: &str,
        head: Option<&ResolvedHead>,
        settled: Option<Settled>,
        alert: bool,
    ) {
        if alert || alert_standing(channel_id) || self.origin_of_open_window(channel_id) {
            return;
        }
        if let (Some(settled), Some(head)) = (settled, head) {
            self.settled_rest.lock().insert(
                channel_id.to_string(),
                ChannelRest {
                    head: head.clone(),
                    skips_left: RESTED_SWEEPS,
                    applied_metadata_json: settled.applied_metadata_json,
                },
            );
        }
    }

    /// Is `channel_id` the origin of an open lineage window? One lock read and
    /// a filter; empty on every peer that has not crossed.
    fn origin_of_open_window(&self, channel_id: &str) -> bool {
        self.lineage.as_ref().is_some_and(|lineage| {
            lineage.open_windows().iter().any(|(_, window)| {
                window
                    .origin
                    .as_ref()
                    .is_some_and(|origin| origin.channel_id == channel_id)
            })
        })
    }
}

/// Serialises every test that drives `sweep_once`: each sweep reconciles the
/// process-wide registry in [`super::state`] to ITS follow set, so two running
/// at once would drop each other's rows mid-check.
#[cfg(test)]
pub(super) fn sweep_test_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    &LOCK
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lineage_roles::{LineageRoles, WindowOrigin};

    /// A channel root: a head that is not a release. `Idle`, and settled.
    const ROOT: &str = r#"{"kind":"release-channel"}"#;
    /// Claims to be a release and carries none: refused, never settled.
    const BROKEN: &str = r#"{"kind":"release-manifest"}"#;

    /// The conductor, scripted per channel and counted per channel. A channel
    /// with no scripted answer is unreachable.
    #[derive(Default)]
    struct ScriptedHeads {
        answers: Mutex<HashMap<String, Option<ContentHeadWire>>>,
        calls: Mutex<HashMap<String, usize>>,
        /// Poke the channel being resolved from INSIDE the next resolve — a
        /// hint landing while the check is in flight.
        poke_during_next_resolve: Mutex<bool>,
    }

    impl ScriptedHeads {
        fn answer(&self, channel: &str, head: Option<ContentHeadWire>) {
            self.answers
                .lock()
                .unwrap()
                .insert(channel.to_string(), head);
        }

        fn calls(&self, channel: &str) -> usize {
            self.calls
                .lock()
                .unwrap()
                .get(channel)
                .copied()
                .unwrap_or(0)
        }
    }

    #[async_trait::async_trait]
    impl head_reader::HeadReader for ScriptedHeads {
        async fn resolve(&self, channel_id: &str) -> Answer<ContentHeadWire> {
            *self
                .calls
                .lock()
                .unwrap()
                .entry(channel_id.to_string())
                .or_insert(0) += 1;
            if std::mem::take(&mut *self.poke_during_next_resolve.lock().unwrap()) {
                poke(channel_id);
            }
            match self.answers.lock().unwrap().get(channel_id) {
                Some(head) => Answer::observed_absence(head.clone()),
                None => Answer::Unreachable,
            }
        }
    }

    /// A resolved head, built through `ContentHeadWire`'s real `Deserialize`
    /// impl so the fixture cannot drift from the wire shape.
    fn head(
        channel: &str,
        cid: &str,
        earned: Option<bool>,
        candidate: Option<&str>,
        metadata_json: &str,
    ) -> ContentHeadWire {
        let mut value = serde_json::json!({
            "content_id": channel,
            "head_action_hash": cid,
            "declared_at": 1_700_000_000_000_000i64,
            "canonical": true,
            "canonical_earned": earned,
            "content": {
                "id": channel,
                "content_type": "concept",
                "title": "t",
                "description": "d",
                "content_format": "markdown",
                "reach": "commons",
                "metadata_json": metadata_json,
            },
        });
        if let Some(candidate) = candidate {
            value["staging_candidate"] = serde_json::json!(candidate);
            value["staging_candidate_declared_at"] = serde_json::json!(1_700_000_001_000_000i64);
        }
        serde_json::from_value(value).expect("ContentHeadWire fixture must deserialize")
    }

    fn root_head(channel: &str, cid: &str) -> ContentHeadWire {
        head(channel, cid, Some(false), None, ROOT)
    }

    fn controller(dir: &tempfile::TempDir, heads: &Arc<ScriptedHeads>) -> AdoptionController {
        let mut controller = AdoptionController::new(dir.path());
        controller.heads = Some(heads.clone());
        controller
    }

    /// Run `n` sweeps; how many channels each one actually checked.
    async fn sweeps(
        controller: &AdoptionController,
        followed: &FollowedChannels,
        n: usize,
    ) -> Vec<usize> {
        let mut checked = Vec::with_capacity(n);
        for _ in 0..n {
            checked.push(controller.sweep_once(followed).await);
        }
        checked
    }

    fn resting(controller: &AdoptionController, channel: &str) -> bool {
        controller.settled_rest.lock().contains_key(channel)
    }

    /// Mark `cid` as the release this peer applied on `channel`, as a vehicle
    /// would have — the fact the C6b exit reads.
    fn applied(followed: &FollowedChannels, channel: &str, cid: &str) {
        state::reconcile_followed(followed);
        state::record_applied(
            channel,
            AppliedRelease {
                cid: cid.to_string(),
                at: 1_000,
                vehicle: "test_vehicle".to_string(),
                carry: None,
            },
            false,
        );
    }

    /// **The station.** A head that carries no release is settled: asked on
    /// the first sweep, skipped for `RESTED_SWEEPS`, asked again — one resolve
    /// per three sweeps, and a skipped sweep is not a check.
    #[tokio::test]
    async fn a_settled_idle_head_is_asked_every_third_sweep() {
        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:idle";
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(channel, Some(root_head(channel, "uhCkkRoot")));
        let controller = controller(&dir, &heads);
        let followed = state::parse_followed_channels(channel);

        let checked = sweeps(&controller, &followed, 7).await;
        assert_eq!(checked, vec![1, 0, 0, 1, 0, 0, 1]);
        assert_eq!(heads.calls(channel), 3);
        let row = state::channel_state(channel).unwrap();
        assert_eq!(
            row.sweeps, 3,
            "a rested sweep is no transition: lastCheckedAt names real looks only"
        );
    }

    /// The C6b idempotence exit — the converged-fleet case this exists for —
    /// is settled, on a staging head as much as an earned one.
    #[tokio::test]
    async fn a_converged_applying_channel_is_asked_every_third_sweep() {
        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:converged";
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(channel, Some(root_head(channel, "uhCkkApplied")));
        let controller = controller(&dir, &heads);
        let followed = state::parse_followed_channels(&format!("{channel}=canary"));
        applied(&followed, channel, "uhCkkApplied");

        let checked = sweeps(&controller, &followed, 4).await;
        assert_eq!(checked, vec![1, 0, 0, 1]);
        assert_eq!(heads.calls(channel), 2);
        assert_eq!(
            state::channel_state(channel).unwrap().verdict,
            Some(Verdict::Applied {
                release_cid: "uhCkkApplied".to_string(),
                vehicle: "test_vehicle".to_string(),
                already_current: true,
            })
        );
    }

    /// **The fallback bound, with no poke at all.** A head that moves right
    /// after a settled check is read no later than the third sweep after it
    /// moved — 180 s at the default cadence, inside the 240 s a fleet is given
    /// to converge.
    #[tokio::test]
    async fn an_unpoked_head_change_is_read_by_the_third_sweep_after_it() {
        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:unpoked-move";
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(channel, Some(root_head(channel, "uhCkkOld")));
        let controller = controller(&dir, &heads);
        let followed = state::parse_followed_channels(channel);

        // The worst phase: the head moves the instant the rest is recorded.
        assert_eq!(controller.sweep_once(&followed).await, 1);
        heads.answer(channel, Some(root_head(channel, "uhCkkNew")));

        let checked = sweeps(&controller, &followed, 3).await;
        assert_eq!(
            checked.iter().sum::<usize>(),
            1,
            "asked within three sweeps"
        );
        assert_eq!(
            state::channel_state(channel)
                .unwrap()
                .resolved_head
                .unwrap()
                .cid,
            "uhCkkNew"
        );
    }

    /// A poke ends the rest on the very next sweep and holds the channel
    /// alert: checked every sweep, and NO rest recorded although every one of
    /// those checks settles on the old head. When the alert lapses with the
    /// head unchanged, the channel rests again.
    #[tokio::test]
    async fn a_poke_checks_every_sweep_of_the_alert_and_then_the_channel_rests_again() {
        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:poked";
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(channel, Some(root_head(channel, "uhCkkOld")));
        let controller = controller(&dir, &heads);
        let followed = state::parse_followed_channels(channel);

        assert_eq!(sweeps(&controller, &followed, 2).await, vec![1, 0]);
        assert!(resting(&controller, channel));

        poke(channel);
        for sweep in 0..ALERT_SWEEPS {
            assert_eq!(
                controller.sweep_once(&followed).await,
                1,
                "alert sweep {sweep} must check"
            );
            assert!(
                !resting(&controller, channel),
                "alert sweep {sweep} settled on the old head and must not rest on it"
            );
        }
        assert_eq!(heads.calls(channel), 1 + ALERT_SWEEPS as usize);

        // The alert has lapsed: the next check settles and the rest returns.
        assert_eq!(sweeps(&controller, &followed, 3).await, vec![1, 0, 0]);
        assert!(resting(&controller, channel));
        assert_eq!(heads.calls(channel), 2 + ALERT_SWEEPS as usize);
    }

    /// A poke for a content id that is not a followed channel is not
    /// remembered — every applied content doc on the sync plane pokes.
    #[tokio::test]
    async fn a_poke_for_an_unfollowed_channel_is_inert() {
        let _serial = sweep_test_lock().lock().await;
        state::reconcile_followed(&state::parse_followed_channels("runtime:rest:followed"));
        poke("runtime:rest:not-followed");
        poke("runtime:rest:followed");
        // Copied out, so a failed assertion cannot poison the process-wide map.
        let alert = |id: &str| alerts().get(id).copied();
        assert_eq!(alert("runtime:rest:not-followed"), None);
        assert_eq!(alert("runtime:rest:followed"), Some(ALERT_SWEEPS));
    }

    /// The asking sweep IS a full check: a head that moved while the channel
    /// rested is judged in that same call, and its unsettled verdict leaves no
    /// rest behind.
    #[tokio::test]
    async fn a_new_head_on_the_asking_sweep_is_judged_at_once_and_the_rest_dropped() {
        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:moved";
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(channel, Some(root_head(channel, "uhCkkOld")));
        let controller = controller(&dir, &heads);
        let followed = state::parse_followed_channels(channel);

        assert_eq!(sweeps(&controller, &followed, 2).await, vec![1, 0]);
        heads.answer(
            channel,
            Some(head(channel, "uhCkkNew", Some(false), None, BROKEN)),
        );
        // One more rested sweep, then the asking sweep meets the new head.
        assert_eq!(sweeps(&controller, &followed, 2).await, vec![0, 1]);
        assert_eq!(heads.calls(channel), 2);
        let row = state::channel_state(channel).unwrap();
        assert_eq!(row.resolved_head.unwrap().cid, "uhCkkNew");
        assert!(matches!(row.verdict, Some(Verdict::Refused { .. })));
        assert!(!resting(&controller, channel));

        // Not resting: the ladder paces it now, and its gate resolves.
        controller.sweep_once(&followed).await;
        assert_eq!(heads.calls(channel), 3);
    }

    /// Nothing but a settled exit rests: not an absent head, not an
    /// unreachable conductor, not a canary whose candidate it could not read,
    /// and not a canary that already applied the candidate — that exit holds
    /// no pointer metadata for the rest to keep checking.
    #[tokio::test]
    async fn an_unsettled_verdict_never_rests() {
        let _serial = sweep_test_lock().lock().await;
        let (absent, unreachable, unread, candidate_applied) = (
            "runtime:rest:absent",
            "runtime:rest:unreachable",
            "runtime:rest:candidate-unread",
            "runtime:rest:candidate-applied",
        );
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(absent, None);
        // No conductor is wired, so the candidate's own record is unreadable
        // and the canary falls back to the winner — a root, which would settle.
        heads.answer(
            unread,
            Some(head(
                unread,
                "uhCkkWinner",
                Some(true),
                Some("uhCkkCandidate"),
                ROOT,
            )),
        );
        heads.answer(
            candidate_applied,
            Some(head(
                candidate_applied,
                "uhCkkWinner",
                Some(true),
                Some("uhCkkCandidate"),
                ROOT,
            )),
        );
        let controller = controller(&dir, &heads);
        let followed = state::parse_followed_channels(&format!(
            "{absent},{unreachable},{unread}=canary,{candidate_applied}=canary"
        ));
        applied(&followed, candidate_applied, "uhCkkCandidate");

        sweeps(&controller, &followed, 3).await;
        for channel in [absent, unreachable, unread, candidate_applied] {
            assert_eq!(
                heads.calls(channel),
                3,
                "{channel} must be asked every sweep"
            );
            assert!(!resting(&controller, channel));
        }
    }

    /// A mode change resets the recorded row, and an unfollow drops it: either
    /// way the row no longer names the rested head, so the next sweep checks.
    #[tokio::test]
    async fn a_mode_change_or_a_refollow_ends_the_rest() {
        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:reconfigured";
        let other = "runtime:rest:reconfigured-other";
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(channel, Some(root_head(channel, "uhCkkRoot")));
        heads.answer(other, None);
        let controller = controller(&dir, &heads);

        let observe = state::parse_followed_channels(channel);
        assert_eq!(sweeps(&controller, &observe, 2).await, vec![1, 0]);

        // Mode change, mid-rest.
        let apply = state::parse_followed_channels(&format!("{channel}=apply"));
        assert_eq!(sweeps(&controller, &apply, 2).await, vec![1, 0]);
        assert_eq!(heads.calls(channel), 2);

        // Unfollowed: the rest goes with the follow set that named it.
        poke(channel);
        let without = state::parse_followed_channels(other);
        controller.sweep_once(&without).await;
        assert!(!resting(&controller, channel));
        assert!(!alert_standing(channel));

        // Followed again: asked at once.
        assert_eq!(sweeps(&controller, &apply, 2).await, vec![1, 0]);
        assert_eq!(heads.calls(channel), 3);
    }

    /// The committed app-bundle manifest as a head's `metadata_json`, and the
    /// `(slug, pointer)` rows it expects once applied.
    fn app_bundle_metadata() -> (String, Vec<(String, String)>) {
        let body: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                "../../genesis/a2o/scripts/__tests__/fixtures/release-manifest-app-bundle.json",
            )
            .expect("the app-bundle fixture is committed"),
        )
        .unwrap();
        let manifest = verify::verify_shape(&body).unwrap();
        let pointers =
            crate::services::release_adoption::apply::expected_browser_pointers(&manifest);
        assert!(!pointers.is_empty(), "the fixture binds browser bundles");
        let metadata =
            serde_json::json!({"kind": RELEASE_MANIFEST_KIND, "manifest": body}).to_string();
        (metadata, pointers)
    }

    /// Rows for the bound slugs, each pointing at the release's own bytes.
    fn seed_pointers(pool: &crate::db::DbPool, pointers: &[(String, String)]) {
        let ctx = crate::db::AppContext::default_lamad();
        let mut conn = pool.get().unwrap();
        for (slug, hash) in pointers {
            crate::db::content_diesel::create_content(
                &mut conn,
                &ctx,
                crate::db::content_diesel::CreateContentInput {
                    id: slug.clone(),
                    title: slug.clone(),
                    description: None,
                    content_type: "concept".to_string(),
                    content_format: "markdown".to_string(),
                    blob_hash: Some(hash.clone()),
                    blob_cid: Some(hash.clone()),
                    content_size_bytes: None,
                    metadata_json: None,
                    reach: "commons".to_string(),
                    created_by: None,
                    tags: Vec::new(),
                    content_body: None,
                    dht_anchor_hash: None,
                },
            )
            .unwrap();
        }
    }

    /// The C6b exit's pointer check keeps running while the channel rests, and
    /// a pointer another writer moved ends the rest that same sweep.
    #[tokio::test]
    async fn a_drifted_pointer_ends_the_rest() {
        use crate::db::diesel_schema::content;
        use diesel::prelude::*;

        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:drift";
        let (metadata, pointers) = app_bundle_metadata();
        let pool = crate::test_util::test_pool();
        let ctx = crate::db::AppContext::default_lamad();
        seed_pointers(&pool, &pointers);

        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(
            channel,
            Some(head(channel, "uhCkkBundle", Some(false), None, &metadata)),
        );
        let controller = controller(&dir, &heads).with_db(pool.clone());
        let followed = state::parse_followed_channels(&format!("{channel}=canary"));
        applied(&followed, channel, "uhCkkBundle");

        assert_eq!(sweeps(&controller, &followed, 2).await, vec![1, 0]);
        assert_eq!(heads.calls(channel), 1);

        // Another writer moves one slug's pointer while the channel rests.
        diesel::update(
            content::table
                .filter(content::h_app_id.eq(&ctx.h_app_id))
                .filter(content::id.eq(&pointers[0].0)),
        )
        .set(content::blob_hash.eq(Some("sha256-moved-by-another-writer")))
        .execute(&mut pool.get().unwrap())
        .unwrap();

        controller.sweep_once(&followed).await;
        assert_eq!(
            heads.calls(channel),
            2,
            "a drifted pointer must send the channel back through check_channel at once"
        );
        assert!(!resting(&controller, channel));
    }

    /// The revert arm is watching this channel's head for a re-election, so
    /// the channel that opened an open window is checked every sweep.
    #[tokio::test]
    async fn the_origin_of_an_open_lineage_window_never_rests() {
        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:window-origin";
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(channel, Some(root_head(channel, "uhCkkRoot")));
        let mut controller = controller(&dir, &heads);
        let lineage = Arc::new(LineageRoles::new("elohim", &["node_registry"]));
        lineage.open_window(
            "node_registry",
            "elohim@SIDE",
            Some(WindowOrigin {
                channel_id: channel.to_string(),
                release_cid: "uhCkkRELEASE".to_string(),
                path_commitment_cid: "uhCEkPATH".to_string(),
            }),
        );
        // The resolver alone: with no reverter the window arm itself is dark,
        // so every resolve counted below is the channel loop's.
        controller.lineage = Some(lineage);
        let followed = state::parse_followed_channels(channel);

        for sweep in 0..3 {
            assert_eq!(controller.sweep_once(&followed).await, 1);
            assert!(
                !resting(&controller, channel),
                "sweep {sweep} settled, and must still leave no memo behind an open window"
            );
        }
        assert_eq!(heads.calls(channel), 3);
    }

    /// `MAX_CHANNELS_PER_SWEEP` is positional, so a rested channel keeps its
    /// place: it takes nothing from a neighbour that is still being asked
    /// every sweep, and it is asked again on schedule.
    #[tokio::test]
    async fn a_rested_channel_neither_takes_a_neighbours_slot_nor_starves() {
        let _serial = sweep_test_lock().lock().await;
        let (settled, busy) = ("runtime:rest:slot-settled", "runtime:rest:slot-busy");
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(settled, Some(root_head(settled, "uhCkkRoot")));
        heads.answer(busy, None);
        let controller = controller(&dir, &heads);
        let followed = state::parse_followed_channels(&format!("{settled},{busy}"));

        assert_eq!(
            sweeps(&controller, &followed, 6).await,
            vec![2, 1, 1, 2, 1, 1]
        );
        assert_eq!(heads.calls(settled), 2);
        assert_eq!(heads.calls(busy), 6);
    }

    /// A poke that lands while the check is in flight — after the gate read
    /// the alert, before the rest is recorded — leaves no rest: the alert is
    /// read again at record time.
    #[tokio::test]
    async fn a_poke_between_the_gate_and_the_record_leaves_no_rest() {
        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:poked-mid-check";
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(channel, Some(root_head(channel, "uhCkkOld")));
        let controller = controller(&dir, &heads);
        let followed = state::parse_followed_channels(channel);

        // Sweep 0 creates the row, so the channel is followed when poked.
        assert_eq!(sweeps(&controller, &followed, 3).await, vec![1, 0, 0]);
        *heads.poke_during_next_resolve.lock().unwrap() = true;
        assert_eq!(controller.sweep_once(&followed).await, 1);
        assert!(
            !resting(&controller, channel),
            "the check settled on the old head while a poke stood — no rest"
        );
        assert_eq!(controller.sweep_once(&followed).await, 1);
    }

    /// A pointer check that could not READ the rows is not "unchanged": the
    /// rest is forgotten, the channel is checked, and no new rest is recorded
    /// while the read keeps failing.
    #[tokio::test]
    async fn a_pointer_read_that_fails_ends_the_rest() {
        use diesel::r2d2::{ConnectionManager, Pool};

        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:pointer-unread";
        let (metadata, pointers) = app_bundle_metadata();
        // One connection and a short wait, so holding it makes `get()` fail.
        let url = format!(
            "file:rest_unread_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .connection_timeout(std::time::Duration::from_millis(50))
            .build(ConnectionManager::<diesel::SqliteConnection>::new(url))
            .unwrap();
        crate::db::run_migrations(&pool).unwrap();
        seed_pointers(&pool, &pointers);

        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(
            channel,
            Some(head(channel, "uhCkkBundle", Some(false), None, &metadata)),
        );
        let controller = controller(&dir, &heads).with_db(pool.clone());
        let followed = state::parse_followed_channels(&format!("{channel}=canary"));
        applied(&followed, channel, "uhCkkBundle");

        assert_eq!(sweeps(&controller, &followed, 2).await, vec![1, 0]);
        assert_eq!(heads.calls(channel), 1);

        let held = pool.get().unwrap();
        controller.sweep_once(&followed).await;
        assert_eq!(heads.calls(channel), 2, "an unread pointer must not rest");
        assert!(
            !resting(&controller, channel),
            "and the check that followed could not verify the pointers either"
        );
        std::mem::drop(held);
    }

    /// A tick with nothing followed runs no sweep — and must still let go of
    /// the rest and the alert of the channel that was just unfollowed.
    #[tokio::test]
    async fn a_tick_with_nothing_followed_prunes_rests_and_alerts() {
        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:unfollowed-all";
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(channel, Some(root_head(channel, "uhCkkRoot")));
        let controller = controller(&dir, &heads);
        let followed = state::parse_followed_channels(channel);

        controller.sweep_once(&followed).await;
        poke(channel);
        assert!(resting(&controller, channel) && alert_standing(channel));

        controller.unfollowed_tick(&FollowedChannels::default());
        assert!(!resting(&controller, channel));
        assert!(!alert_standing(channel));
        assert!(!state::is_followed(channel));
    }

    /// **The hint path, end to end.** A peer's doc for the channel's content
    /// id offered to the real `TriggerGate` wakes the resting controller on
    /// its next sweep — through the real membership check, not a capture.
    #[tokio::test]
    async fn a_sync_hint_for_a_followed_channel_wakes_its_resting_controller() {
        use crate::services::head_adoption_trigger::{TriggerGate, DEFAULT_TRIGGER_COOLDOWN};

        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:hinted";
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(channel, Some(root_head(channel, "uhCkkRoot")));
        let controller = controller(&dir, &heads);
        let followed = state::parse_followed_channels(channel);
        let (gate, _rx) = TriggerGate::new(DEFAULT_TRIGGER_COOLDOWN);

        assert_eq!(sweeps(&controller, &followed, 2).await, vec![1, 0]);
        gate.offer(
            crate::sync::projector::PROJECTION_NAMESPACE,
            &crate::sync::projector::content_doc_id(channel),
            "peer-a",
        );
        assert_eq!(controller.sweep_once(&followed).await, 1);
        assert_eq!(heads.calls(channel), 2);
    }

    /// **The author's path, end to end.** This cell's own head declaration
    /// for the channel, handled by the real signal projection, wakes the
    /// resting controller on its next sweep.
    #[tokio::test]
    async fn a_local_head_declaration_wakes_the_resting_controller() {
        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:declared";
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(channel, Some(root_head(channel, "uhCkkRoot")));
        let controller = controller(&dir, &heads);
        let followed = state::parse_followed_channels(channel);

        assert_eq!(sweeps(&controller, &followed, 2).await, vec![1, 0]);
        crate::rea_projection::handle_rea_signal(
            crate::rea_projection::ReaProjectionSignal::ContentHeadDeclared {
                content_id: channel.to_string(),
                head_action_hash: crate::signals::HoloHashB64("uhCkkDeclared".into()),
                entry_hash: None,
                author: None,
                canonical_declared_at: None,
                canonical_earned: None,
                canonical_link_hash: None,
                canonical_ordering_hash: None,
            },
            &crate::test_util::test_pool(),
            &crate::db::AppContext::default_lamad(),
        )
        .expect("a legacy declaration with no row is handled");
        assert_eq!(controller.sweep_once(&followed).await, 1);
        assert_eq!(heads.calls(channel), 2);
    }

    /// A panic while the alert map is held must not reach the sync apply or
    /// signal paths through `poke`, nor stop the sweep.
    #[tokio::test]
    async fn a_poisoned_alert_map_neither_panics_a_poke_nor_stops_the_sweep() {
        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:poisoned";
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(channel, Some(root_head(channel, "uhCkkRoot")));
        let controller = controller(&dir, &heads);
        let followed = state::parse_followed_channels(channel);
        assert_eq!(sweeps(&controller, &followed, 2).await, vec![1, 0]);

        let poisoner = std::thread::spawn(|| {
            let _held = ALERTS.lock().unwrap_or_else(|e| e.into_inner());
            panic!("poison the alert map");
        });
        assert!(poisoner.join().is_err());
        assert!(ALERTS.is_poisoned());

        poke(channel);
        assert_eq!(controller.sweep_once(&followed).await, 1);
    }
}
