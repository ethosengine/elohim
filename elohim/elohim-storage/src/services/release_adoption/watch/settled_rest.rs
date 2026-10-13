//! The settled rest — a channel with nothing left to do is not re-resolved
//! every sweep.
//!
//! # The cost this removes
//!
//! `sweep_once` resolves every followed channel's head once a tick. On a
//! converged peer the answer is the same head with the same verdict every
//! minute: one `resolve_content_head_local` per channel per sweep, forever, to
//! re-learn nothing. A channel whose last check ended at one of the three
//! SETTLED exits of `check_channel` — the C6b idempotence exit, the canary's
//! candidate-already-applied exit, or a head that carries no release — is
//! instead skipped for [`RESTED_SWEEPS`] sweeps and asked again on the next,
//! so it costs one resolve per `RESTED_SWEEPS + 1` sweeps (five minutes at the
//! default cadence, proportionally at any other: sweeps are counted, not
//! timed).
//!
//! # What ends a rest at once
//!
//! - **a poke** ([`poke`]): a peer's head hint for the channel's content id
//!   reached this node's sync plane, or this node's own conductor signalled a
//!   head declaration for it. The channel is then ALERT for [`ALERT_SWEEPS`]
//!   sweeps: checked every sweep, and no rest may be recorded — a hint can
//!   outrun gossip, so the first resolve after a poke may still answer the old
//!   head, and resting on that would lose the release for a whole rest. A poke
//!   never supplies an answer (I1): the resolve still goes through this node's
//!   own conductor.
//! - **the recorded row no longer names the rested head**: `reconcile_followed`
//!   resets a channel's row on a mode change and drops it on an unfollow, so
//!   both end the rest here with no clause of their own.
//! - **an open lineage window** whose origin is this channel: the revert arm is
//!   watching that head for a re-election, so the channel is not at rest.
//! - **a drifted pointer**: the C6b exit's own installed-reality check
//!   (`applied_pointers_drifted`, a DB read) still runs every sweep against the
//!   metadata the rest carries, and a moved pointer sends the channel straight
//!   back through `check_channel`.
//! - **a restart**: the memo is in-process.
//!
//! Every other verdict — a refusal (the ladder in
//! [`super::state::BACKOFF_LADDER_SECS`] paces it), `Waiting`, an observe-mode
//! `Ok` (an observer keeps re-deriving), a fresh apply, an absent head, a
//! candidate this canary could not read — records nothing, so the channel is
//! asked again on the next sweep exactly as before.
//!
//! # What it costs
//!
//! A newly elected release on a channel that was settled is noticed on the
//! next sweep after a poke — the same minute it always was. Only when BOTH
//! poke paths miss (the author's sync doc never reaches this node and this
//! node's own conductor never signals the declaration) is it noticed up to
//! `RESTED_SWEEPS` sweeps later than before. A hint that outruns gossip by
//! more than `ALERT_SWEEPS` sweeps costs one further rest on top. The C6b
//! exit's installed-reality refresh (its TTL equals one rest) now runs on the
//! asking sweeps only, so that snapshot on `/admin/adoption` can be one rest
//! older than its TTL.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use super::*;

/// Sweeps skipped between two asks of a settled channel. With the asking sweep
/// that is one resolve per five sweeps.
pub const RESTED_SWEEPS: u32 = 4;

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

/// A head for `content_id` may have moved: if it is a followed channel, end
/// its rest and hold it alert for [`ALERT_SWEEPS`] sweeps.
///
/// One map lookup and, for a followed channel, one insert. Never blocks,
/// awaits or fails, so it is safe on the sync apply path. A content id that is
/// not a followed channel is a no-op and is not remembered. The poke carries no
/// head: it only decides WHEN this node asks its own conductor.
pub fn poke(content_id: &str) {
    #[cfg(test)]
    poke_capture::record(content_id);
    if !state::is_followed(content_id) {
        return;
    }
    ALERTS
        .lock()
        .unwrap()
        .insert(content_id.to_string(), ALERT_SWEEPS);
}

/// Is `channel_id` on alert this sweep? Consumes one sweep of the alert.
fn take_alert(channel_id: &str) -> bool {
    let mut alerts = ALERTS.lock().unwrap();
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
        self.settled_rest
            .resting
            .lock()
            .unwrap()
            .retain(|id, _| keep(id));
        ALERTS.lock().unwrap().retain(|id, _| keep(id));
    }

    /// Rest this channel this sweep, or check it? Forgets the rest on every
    /// path that says the channel is no longer where the rest left it.
    pub(super) fn rest_gate(
        &self,
        channel: &FollowedChannel,
        existing: Option<&state::ChannelAdoptionState>,
    ) -> RestGate {
        let id = &channel.channel_id;
        let alert = take_alert(id);
        let mut resting = self.settled_rest.resting.lock().unwrap();
        if alert {
            // Poked: something says the head may have moved.
            resting.remove(id);
            return RestGate::Check { alert: true };
        }
        let Some(rest) = resting.get_mut(id) else {
            return RestGate::Check { alert: false };
        };
        // The rest is about the head the recorded row names. A row reset by a
        // mode change, or re-created by a re-follow, names none.
        let row_names_rested_head =
            existing.and_then(|row| row.resolved_head.as_ref()) == Some(&rest.head);
        if !row_names_rested_head
            || self.origin_of_open_window(id)
            || rest
                .applied_metadata_json
                .as_deref()
                .is_some_and(|metadata| !self.applied_pointers_drifted(metadata).is_empty())
            || rest.skips_left == 0
        {
            // The last clause is the rest simply running out: the check that
            // follows is the asking sweep, and it records a fresh rest only if
            // it settles again.
            resting.remove(id);
            return RestGate::Check { alert: false };
        }
        rest.skips_left -= 1;
        RestGate::Rested
    }

    /// Record a rest after a completed check — only when the check ended at a
    /// settled exit with a resolved head, and never while the channel is alert.
    /// Every other completed check leaves no rest behind: `rest_gate` removed
    /// any earlier one before the check ran.
    pub(super) fn record_rest(
        &self,
        channel_id: &str,
        head: Option<&ResolvedHead>,
        settled: Option<Settled>,
        alert: bool,
    ) {
        if alert {
            return;
        }
        if let (Some(settled), Some(head)) = (settled, head) {
            self.settled_rest.resting.lock().unwrap().insert(
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
        controller
            .settled_rest
            .resting
            .lock()
            .unwrap()
            .contains_key(channel)
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
    /// per five sweeps, and a skipped sweep is not a check.
    #[tokio::test]
    async fn a_settled_idle_head_is_asked_once_in_five_sweeps() {
        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:idle";
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(channel, Some(root_head(channel, "uhCkkRoot")));
        let controller = controller(&dir, &heads);
        let followed = state::parse_followed_channels(channel);

        let checked = sweeps(&controller, &followed, 11).await;
        assert_eq!(checked, vec![1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]);
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
    async fn a_converged_applying_channel_is_asked_once_in_five_sweeps() {
        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:converged";
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(channel, Some(root_head(channel, "uhCkkApplied")));
        let controller = controller(&dir, &heads);
        let followed = state::parse_followed_channels(&format!("{channel}=canary"));
        applied(&followed, channel, "uhCkkApplied");

        let checked = sweeps(&controller, &followed, 6).await;
        assert_eq!(checked, vec![1, 0, 0, 0, 0, 1]);
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

    /// A canary that already applied the candidate beneath an earned head has
    /// nothing to do until promotion.
    #[tokio::test]
    async fn a_canary_that_applied_the_candidate_is_asked_once_in_five_sweeps() {
        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:candidate-applied";
        let dir = tempfile::tempdir().unwrap();
        let heads = Arc::new(ScriptedHeads::default());
        heads.answer(
            channel,
            Some(head(
                channel,
                "uhCkkWinner",
                Some(true),
                Some("uhCkkCandidate"),
                ROOT,
            )),
        );
        let controller = controller(&dir, &heads);
        let followed = state::parse_followed_channels(&format!("{channel}=canary"));
        applied(&followed, channel, "uhCkkCandidate");

        let checked = sweeps(&controller, &followed, 6).await;
        assert_eq!(checked, vec![1, 0, 0, 0, 0, 1]);
        assert_eq!(heads.calls(channel), 2);
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
        let alert = |id: &str| ALERTS.lock().unwrap().get(id).copied();
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

        assert_eq!(sweeps(&controller, &followed, 3).await, vec![1, 0, 0]);
        heads.answer(
            channel,
            Some(head(channel, "uhCkkNew", Some(false), None, BROKEN)),
        );
        // Two more rested sweeps, then the asking sweep meets the new head.
        assert_eq!(sweeps(&controller, &followed, 3).await, vec![0, 0, 1]);
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
    /// unreachable conductor, not a canary whose candidate it could not read.
    #[tokio::test]
    async fn an_unsettled_verdict_never_rests() {
        let _serial = sweep_test_lock().lock().await;
        let (absent, unreachable, unread) = (
            "runtime:rest:absent",
            "runtime:rest:unreachable",
            "runtime:rest:candidate-unread",
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
        let controller = controller(&dir, &heads);
        let followed =
            state::parse_followed_channels(&format!("{absent},{unreachable},{unread}=canary"));

        sweeps(&controller, &followed, 3).await;
        for channel in [absent, unreachable, unread] {
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
        let alert_left = ALERTS.lock().unwrap().contains_key(channel);
        assert!(!alert_left);

        // Followed again: asked at once.
        assert_eq!(sweeps(&controller, &apply, 2).await, vec![1, 0]);
        assert_eq!(heads.calls(channel), 3);
    }

    /// The C6b exit's pointer check keeps running while the channel rests, and
    /// a pointer another writer moved ends the rest that same sweep.
    #[tokio::test]
    async fn a_drifted_pointer_ends_the_rest() {
        use crate::db::diesel_schema::content;
        use diesel::prelude::*;

        let _serial = sweep_test_lock().lock().await;
        let channel = "runtime:rest:drift";
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

        let pool = crate::test_util::test_pool();
        let ctx = crate::db::AppContext::default_lamad();
        {
            let mut conn = pool.get().unwrap();
            for (slug, hash) in &pointers {
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

        assert_eq!(sweeps(&controller, &followed, 3).await, vec![1, 1, 1]);
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
            vec![2, 1, 1, 1, 1, 2]
        );
        assert_eq!(heads.calls(settled), 2);
        assert_eq!(heads.calls(busy), 6);
    }
}
