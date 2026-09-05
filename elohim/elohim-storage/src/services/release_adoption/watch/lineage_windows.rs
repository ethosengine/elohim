//! The lineage-window sweep — the revert arm (Task 13a) and the sunset arm
//! (Task 14b) that run at the TOP of every [`AdoptionController`] tick.
//!
//! Lifted out of `watch.rs` verbatim (whole-branch review I6: `watch.rs` had
//! grown 2155 → 3532 lines on the lineage branch, past the 3000-line
//! `loc-soft` ceiling in `.claude/epr-meta/policies.yaml`; this move leaves it
//! at 2843, back under). Nothing here changed shape in the move
//! — same functions, same signatures, same order, same tests. It is a CHILD
//! module of `watch` rather than a sibling for one reason that is not
//! stylistic: every one of these methods reads `AdoptionController`'s private
//! fields (`hc`, `db`, `lineage`, `reverter`, `sunsetter`) and calls its
//! private `resolve_head`. A sibling would have had to widen all of that to
//! `pub(super)`; a child sees exactly what the code saw before the move, so
//! the extraction stays a move and not a visibility change.
//!
//! Only [`AdoptionController::sweep_lineage_windows`] is `pub(super)` — the
//! one entry point `watch::sweep_once` calls. The rest (`sweep_open_windows`,
//! `revert_open_windows`, `elected_away_from`) stay private to this module and
//! its tests, exactly as they were private to `watch` before.

use super::*;
use crate::services::release_adoption::{
    path_evidence, revert, sunset, LineageCarryReceipt, PathEvidence,
};

impl AdoptionController {
    /// **Task 13a — the revert arm (Station 7).** Runs at the TOP of every
    /// sweep, before any channel is checked.
    ///
    /// # Why it is its own arm and not part of `check_channel`
    ///
    /// The C6b idempotence exit returns `Verdict::Applied` for an
    /// already-applied release WITHOUT re-running `verify_path` — deliberately,
    /// because a converged fleet re-deriving the same verdict every minute is
    /// correct and unaffordable. But that is exactly the state a peer who
    /// CROSSED is in, so the peers who most need to hear about a revocation are
    /// the ones `check_channel` has stopped asking on behalf of. This arm asks
    /// on behalf of the WINDOW instead.
    ///
    /// # What it costs when nothing is happening
    ///
    /// One `RwLock` read and a filter. [`crate::lineage_roles::LineageRoles::open_windows`]
    /// returns empty on every peer that has not crossed, and this returns
    /// before touching the conductor. Crossings are rare and bounded, so the
    /// steady-state cost of the whole arm is that filter, once per sweep.
    ///
    /// The work is bounded BEFORE any conductor call: at most one window's
    /// worth of reads per open window, and open windows are at most one per
    /// role (and, at MVP, one per release — `sole_crossing`).
    pub(super) async fn sweep_lineage_windows(&self) {
        self.sweep_open_windows(
            |cid| async move {
                // (C5) The path, re-read through THIS peer's own conductor —
                // the same read `verify_path` consumes, so "revoked" means one
                // thing.
                path_evidence::fetch_path_evidence_for_cid(self.hc.as_ref(), self.db.as_ref(), &cid)
                    .await
            },
            |cid| async move {
                // **Task 14b.** The sunset that names that path, down the same
                // C5 rail. Skipped entirely when no sunsetter is wired — the
                // read costs a projection query and a zome call, and a
                // controller that could not act on the answer has no business
                // asking.
                if self.sunsetter.is_none() {
                    return Answer::Absent;
                }
                path_evidence::fetch_sunset_evidence_for(self.hc.as_ref(), self.db.as_ref(), &cid)
                    .await
            },
            // THIS peer's own carry, off the applied-release row the crossing
            // left behind. Threaded rather than read inline so the arm's
            // convergence gate is testable against a value instead of a
            // process-global registry.
            |channel_id| state::applied_release(&channel_id).and_then(|applied| applied.carry),
        )
        .await
    }

    /// The pre-Task-14b arm: revert only, with no sunset read. Retained as the
    /// name the revert tests drive, and as a statement that the sunset half is
    /// strictly additive — an `Absent` sunset holds every window exactly where
    /// the revert-only build left it.
    #[cfg(test)]
    async fn revert_open_windows<R, F>(&self, read_path: R)
    where
        R: Fn(String) -> F,
        F: std::future::Future<Output = Answer<PathEvidence>>,
    {
        self.sweep_open_windows(read_path, |_| async { Answer::Absent }, |_| None)
            .await
    }

    /// [`sweep_lineage_windows`](Self::sweep_lineage_windows) with both
    /// evidence reads threaded in, so the arm's control flow — which windows it
    /// looks at, what it does with an unreadable path, which remedy outranks
    /// which, and whether a second pass repeats itself — is testable without a
    /// conductor. Production passes the real C5 reads and nothing else differs.
    ///
    /// # Two arms, one loop, and the ORDER between them is a rule
    ///
    /// Both arms want the same fact — this window's path, re-read now — so
    /// reading it once and offering it to both is not merely cheaper, it is the
    /// only way they cannot disagree about what the path says in a single
    /// sweep.
    ///
    /// **Revert outranks sunset within a sweep.** A revoked path reverts and
    /// `continue`s; it never falls through to the seal. That is Station 8's
    /// own line read forward: a revocation before the sunset is the remedy, and
    /// a revocation after it changes nothing (because the window is closed and
    /// `open_windows` no longer selects it). The one ordering that would be
    /// wrong is sealing a window whose permission was just withdrawn.
    async fn sweep_open_windows<R, F, S, G, C>(&self, read_path: R, read_sunset: S, carry_of: C)
    where
        R: Fn(String) -> F,
        F: std::future::Future<Output = Answer<PathEvidence>>,
        S: Fn(String) -> G,
        G: std::future::Future<Output = Answer<path_evidence::SunsetEvidence>>,
        C: Fn(String) -> Option<LineageCarryReceipt>,
    {
        // Both halves or nothing — see `with_lineage_revert`.
        let (Some(lineage), Some(reverter)) = (self.lineage.as_ref(), self.reverter.as_ref())
        else {
            return;
        };
        let windows = lineage.open_windows();
        if windows.is_empty() {
            // THE IDLE EXIT, FIRST AND CHEAPEST. No conductor call.
            return;
        }

        for (role, window) in windows {
            // A window opened with no recorded origin (a fixture, or a build
            // predating `WindowOrigin`) names no path to re-read and no channel
            // to re-resolve. Skipping is the honest answer: this arm reverts on
            // EVIDENCE, and there is none to read.
            let Some(origin) = window.origin.as_ref() else {
                tracing::debug!(
                    role = role.as_str(),
                    "release-adoption: open lineage window records no origin — nothing to \
                     re-read, so the revert arm leaves it alone"
                );
                continue;
            };

            let evidence = read_path(origin.path_commitment_cid.clone()).await;

            // **13a review Minor — the head read is skipped when it cannot
            // matter.** `revert_decision` ranks `PathRevoked` strictly above
            // `PriorHeadReelected`, so once the path itself reads revoked the
            // channel-head resolve the re-election arm needs is a conductor
            // round trip whose answer cannot change the outcome. Ask the pure
            // decision first with `elected_away = false`; spend the read only
            // when it declines.
            let trigger = match revert::revert_decision(&evidence, false) {
                already @ Some(_) => already,
                None => {
                    let elected_away = self.elected_away_from(origin).await;
                    revert::revert_decision(&evidence, elected_away)
                }
            };

            if let Some(trigger) = trigger {
                match reverter.revert_window(&role, trigger).await {
                    Ok(receipt) => state::record_revert(receipt),
                    // A refusal here is a race the sweep lost (a reset, or a
                    // concurrent revert, between the snapshot and the call),
                    // not a claim about the path — so it is logged and the next
                    // sweep asks again, exactly like every other transient
                    // refusal.
                    Err(refusal) => tracing::warn!(
                        role = role.as_str(),
                        reason = trigger.label(),
                        refusal = %refusal.reason,
                        "release-adoption: revert refused — the window is unchanged and the next \
                         sweep asks again"
                    ),
                }
                // The remedy outranks the seal. A window that just reverted is
                // no longer open anyway; saying so here makes the precedence a
                // fact of control flow rather than an accident of state.
                continue;
            }

            // ── Task 14b: the sunset arm ────────────────────────────────────
            let Some(sunsetter) = self.sunsetter.as_ref() else {
                continue;
            };
            let sunset = read_sunset(origin.path_commitment_cid.clone()).await;
            // THIS peer's own carry, off the applied-release row the crossing
            // left behind. See `sunset::local_carry_converged` — and the module
            // docs, which say plainly that fleet convergence stays the
            // operator's gate at MVP.
            let converged =
                sunset::local_carry_converged(carry_of(origin.channel_id.clone()).as_ref());
            match sunset::sunset_decision(&evidence, &sunset, &role, converged) {
                sunset::SunsetVerdict::Hold(hold) => {
                    tracing::debug!(
                        role = role.as_str(),
                        hold = hold.label(),
                        "release-adoption: sunset held — the window stays open and the next \
                         sweep asks again"
                    );
                }
                sunset::SunsetVerdict::Seal => {
                    let cid = match &sunset {
                        Answer::Present(s) => s.commitment_cid.clone(),
                        // Unreachable by construction: `Seal` is returned only
                        // for a `Present` sunset. Named rather than unwrapped.
                        _ => continue,
                    };
                    match sunsetter.sunset_window(&role, &cid).await {
                        Ok(receipt) => state::record_sunset(receipt),
                        Err(refusal) => tracing::warn!(
                            role = role.as_str(),
                            sunset_commitment_cid = %cid,
                            refusal = %refusal.reason,
                            detail = %refusal.detail,
                            "release-adoption: sunset refused — the window stays OPEN and the \
                             next sweep asks again"
                        ),
                    }
                }
            }
        }
    }

    /// Has the channel that opened this window elected AWAY from the release
    /// that opened it, onto something that is not another crossing?
    ///
    /// That is rung 5's remedy reaching rung 6: the ceremony re-elects the
    /// prior `coordinator-bundle` / `happ-bundle` head, and a window left open
    /// behind it would keep authoring on v2 under a release nobody elected.
    ///
    /// Conservative in every direction that matters. `false` unless this peer
    /// positively READ an election naming a different, non-lineage release:
    /// a head it could not resolve, a head whose manifest it could not decode,
    /// and a head that is another `happ-lineage` crossing all answer `false`,
    /// because none of them is evidence the household went back.
    async fn elected_away_from(&self, origin: &crate::lineage_roles::WindowOrigin) -> bool {
        let Answer::Present(head) = self.resolve_head(&origin.channel_id).await else {
            return false;
        };
        if head.head_action_hash.0 == origin.release_cid {
            return false;
        }
        let Ok(Some(body)) = extract_release_body(&head.content.metadata_json) else {
            return false;
        };
        let Ok(manifest) = verify::verify_shape(&body) else {
            return false;
        };
        // Another crossing is a step FORWARD, not a revert. Only a
        // non-lineage head re-elected over an open window is the going-back
        // shape this arm acts on.
        manifest.artifact_class != ArtifactClass::HappLineage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::services::release_adoption::revert as adoption_revert;
    use crate::services::release_adoption::{PathEvidence, RosterEvidence};

    use crate::lineage_roles::{LineageRoles, WindowOrigin};
    use std::sync::Mutex as StdMutex;

    /// Records what the arm asked it to revert, and reverts for real (so the
    /// arm's idempotence claim is measured against the same OPEN predicate
    /// production uses, not a stub that never closes a window).
    struct FakeReverter {
        lineage: Arc<LineageRoles>,
        calls: StdMutex<Vec<(String, adoption_revert::RevertTrigger)>>,
    }

    impl FakeReverter {
        fn new(lineage: Arc<LineageRoles>) -> Arc<Self> {
            Arc::new(Self {
                lineage,
                calls: StdMutex::new(Vec::new()),
            })
        }
        fn calls(&self) -> Vec<(String, adoption_revert::RevertTrigger)> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl adoption_revert::LineageReverter for FakeReverter {
        async fn revert_window(
            &self,
            role: &str,
            trigger: adoption_revert::RevertTrigger,
        ) -> Result<adoption_revert::RevertReceipt, AdoptionRefusal> {
            self.calls.lock().unwrap().push((role.to_string(), trigger));
            self.lineage.revert(role);
            Ok(adoption_revert::RevertReceipt {
                role: role.to_string(),
                lineage_app_id: "elohim@SIDE".to_string(),
                reason: trigger,
                path_commitment_cid: Some("uhCEkPATH".to_string()),
                at: 1_700_000_000,
                disabled: true,
                disable_error: None,
                readopt: adoption_revert::readopt_not_attempted(role),
            })
        }
    }

    fn revert_evidence(state: &str, revoked_at: Option<&str>) -> PathEvidence {
        PathEvidence {
            commitment_cid: "uhCEkPATH".to_string(),
            state: state.to_string(),
            revoked_at: revoked_at.map(str::to_string),
            from_dna_hash: "uhC0kFROM".to_string(),
            to_dna_hash: "uhC0kTO".to_string(),
            constitution_root: "root".to_string(),
            signatures: 1,
            required_signatures: 1,
            roster_cid: "uhCEkROSTER".to_string(),
            signers: vec!["uhCAkONE".to_string()],
            roster: RosterEvidence::NotFound,
        }
    }

    fn crossed_window() -> Arc<LineageRoles> {
        let lineage = Arc::new(LineageRoles::new("elohim", &["node_registry"]));
        lineage.open_window(
            "node_registry",
            "elohim@SIDE",
            Some(WindowOrigin {
                channel_id: "runtime:coordinators:elohim:lineage".to_string(),
                release_cid: "uhCkkRELEASE".to_string(),
                path_commitment_cid: "uhCEkPATH".to_string(),
            }),
        );
        lineage
    }

    fn controller_with_revert(
        dir: &std::path::Path,
        lineage: Arc<LineageRoles>,
        reverter: Arc<FakeReverter>,
    ) -> AdoptionController {
        AdoptionController::new(dir).with_lineage_revert(lineage, reverter)
    }

    /// **The trigger fires on revoked.** The arm re-reads the path that opened
    /// the window — NOT the channel, which the C6b exit has stopped asking
    /// about — and a `revoked` answer reverts the role.
    #[tokio::test]
    async fn the_revert_arm_fires_on_a_revoked_path() {
        let dir = tempfile::tempdir().unwrap();
        let lineage = crossed_window();
        let reverter = FakeReverter::new(Arc::clone(&lineage));
        let controller =
            controller_with_revert(dir.path(), Arc::clone(&lineage), Arc::clone(&reverter));

        controller
            .revert_open_windows(|cid| {
                assert_eq!(cid, "uhCEkPATH", "the arm re-reads the WINDOW's own path");
                async { Answer::Present(revert_evidence("active", Some("2026-09-04T10:00:00Z"))) }
            })
            .await;

        assert_eq!(
            reverter.calls(),
            vec![(
                "node_registry".to_string(),
                adoption_revert::RevertTrigger::PathRevoked
            )]
        );
    }

    /// **…and not on active.** The ordinary state of an open window, swept
    /// every minute for as long as it stays open, must cost a read and change
    /// nothing.
    #[tokio::test]
    async fn the_revert_arm_leaves_an_active_path_alone() {
        let dir = tempfile::tempdir().unwrap();
        let lineage = crossed_window();
        let reverter = FakeReverter::new(Arc::clone(&lineage));
        let controller =
            controller_with_revert(dir.path(), Arc::clone(&lineage), Arc::clone(&reverter));

        controller
            .revert_open_windows(|_| async { Answer::Present(revert_evidence("active", None)) })
            .await;

        assert!(reverter.calls().is_empty());
        assert_eq!(lineage.app_id_for("node_registry"), "elohim@SIDE");
    }

    /// **C4 — blindness never reverts.** A path this peer could not read
    /// establishes nothing, and tearing a peer off v2 because of our own
    /// outage is the one direction of error that cannot self-heal.
    #[tokio::test]
    async fn the_revert_arm_never_reverts_on_an_unreadable_path() {
        let dir = tempfile::tempdir().unwrap();
        let lineage = crossed_window();
        let reverter = FakeReverter::new(Arc::clone(&lineage));
        let controller =
            controller_with_revert(dir.path(), Arc::clone(&lineage), Arc::clone(&reverter));

        for answer in [Answer::Unreachable, Answer::Absent] {
            let answer = answer.clone();
            controller
                .revert_open_windows(move |_| {
                    let answer = answer.clone();
                    async move { answer }
                })
                .await;
        }
        assert!(reverter.calls().is_empty());
    }

    /// **Idempotence.** The second sweep finds the window already closed (the
    /// OPEN predicate excludes the reverted shape) and does nothing — not even
    /// the path read.
    #[tokio::test]
    async fn a_second_sweep_over_an_already_reverted_window_does_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let lineage = crossed_window();
        let reverter = FakeReverter::new(Arc::clone(&lineage));
        let controller =
            controller_with_revert(dir.path(), Arc::clone(&lineage), Arc::clone(&reverter));

        let reads = Arc::new(StdMutex::new(0usize));
        for _ in 0..3 {
            let reads = Arc::clone(&reads);
            controller
                .revert_open_windows(move |_| {
                    *reads.lock().unwrap() += 1;
                    async { Answer::Present(revert_evidence("revoked", None)) }
                })
                .await;
        }

        assert_eq!(reverter.calls().len(), 1, "reverted exactly once");
        assert_eq!(
            *reads.lock().unwrap(),
            1,
            "and the later sweeps did not even read the path — the window is no longer open"
        );
    }

    /// A window with no recorded origin names no path to re-read, so the arm
    /// leaves it alone rather than guessing which commitment authorised it.
    #[tokio::test]
    async fn a_window_without_an_origin_is_skipped_by_the_arm() {
        let dir = tempfile::tempdir().unwrap();
        let lineage = Arc::new(LineageRoles::new("elohim", &["node_registry"]));
        lineage.open_window("node_registry", "elohim@SIDE", None);
        let reverter = FakeReverter::new(Arc::clone(&lineage));
        let controller =
            controller_with_revert(dir.path(), Arc::clone(&lineage), Arc::clone(&reverter));

        controller
            .revert_open_windows(|_| async {
                panic!("a window with no origin must never reach the path read")
            })
            .await;
        assert!(reverter.calls().is_empty());
    }

    /// A controller with no revert arm wired sweeps exactly as it did before
    /// Task 13a — no window is looked at, and no path is read.
    #[tokio::test]
    async fn an_unequipped_controller_has_no_revert_arm_at_all() {
        let dir = tempfile::tempdir().unwrap();
        let controller = AdoptionController::new(dir.path());
        controller
            .revert_open_windows(|_| async { panic!("no lineage resolver, no arm") })
            .await;
    }

    // ── Task 14b: the sunset arm ───────────────────────────────────────────

    use crate::services::release_adoption::path_evidence::SunsetEvidence;
    use crate::services::release_adoption::sunset as adoption_sunset;

    /// Records what the arm asked it to seal, and sunsets for real — so the
    /// idempotence claim is measured against the same OPEN predicate
    /// production uses.
    struct FakeSunsetter {
        lineage: Arc<LineageRoles>,
        calls: StdMutex<Vec<(String, String)>>,
    }

    impl FakeSunsetter {
        fn new(lineage: Arc<LineageRoles>) -> Arc<Self> {
            Arc::new(Self {
                lineage,
                calls: StdMutex::new(Vec::new()),
            })
        }
        fn calls(&self) -> Vec<(String, String)> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl adoption_sunset::LineageSunsetter for FakeSunsetter {
        async fn sunset_window(
            &self,
            role: &str,
            sunset_commitment_cid: &str,
        ) -> Result<adoption_sunset::SunsetReceipt, AdoptionRefusal> {
            self.calls
                .lock()
                .unwrap()
                .push((role.to_string(), sunset_commitment_cid.to_string()));
            self.lineage.sunset(role);
            Ok(adoption_sunset::SunsetReceipt {
                role: role.to_string(),
                lineage_app_id: "elohim@SIDE".to_string(),
                sunset_commitment_cid: sunset_commitment_cid.to_string(),
                close_hash: "uhCkkCLOSE".to_string(),
                open_hash: "uhCkkOPEN".to_string(),
                witness_hash: "uhCkkWITNESS".to_string(),
                already_sealed: false,
                resumed: false,
                at: 1_700_000_000,
            })
        }
    }

    fn sunset_evidence(state: &str) -> SunsetEvidence {
        SunsetEvidence {
            commitment_cid: "uhCEkSUNSET".to_string(),
            role: "node_registry".to_string(),
            migration_commitment_cid: "uhCEkPATH".to_string(),
            from_dna_hash: "uhC0kFROM".to_string(),
            to_dna_hash: "uhC0kTO".to_string(),
            state: state.to_string(),
            revoked_at: None,
        }
    }

    /// The carry the crossing left behind, carrying the ONE number the local
    /// convergence gate reads. Handed to the arm directly — production reads it
    /// off the applied-release row, which is a process-global registry no
    /// parallel test may safely reconcile.
    fn carry(
        carried: u32,
        v1_count: Option<u32>,
    ) -> crate::services::release_adoption::LineageCarryReceipt {
        crate::services::release_adoption::LineageCarryReceipt {
            role: "node_registry".to_string(),
            carried,
            v1_count,
            digest: "d".to_string(),
            v1_digest: "d".to_string(),
            witness_hashes: vec!["uhCkkW".to_string()],
        }
    }

    fn window_on(channel_id: &str) -> Arc<LineageRoles> {
        let lineage = Arc::new(LineageRoles::new("elohim", &["node_registry"]));
        lineage.open_window(
            "node_registry",
            "elohim@SIDE",
            Some(WindowOrigin {
                channel_id: channel_id.to_string(),
                release_cid: "uhCkkRELEASE".to_string(),
                path_commitment_cid: "uhCEkPATH".to_string(),
            }),
        );
        lineage
    }

    /// **Station 8's deliverable, at the arm.** An active path, an active
    /// sunset naming it, and a converged local carry — the window seals, once,
    /// and the receipt names the commitment that authorised it.
    #[tokio::test]
    async fn the_sunset_arm_seals_on_an_active_sunset_and_only_once() {
        let dir = tempfile::tempdir().unwrap();
        let lineage = window_on("runtime:coordinators:elohim:sunset-arm-seals");
        let reverter = FakeReverter::new(Arc::clone(&lineage));
        let sunsetter = FakeSunsetter::new(Arc::clone(&lineage));
        let controller = AdoptionController::new(dir.path())
            .with_lineage_revert(Arc::clone(&lineage), reverter.clone())
            .with_lineage_sunset(sunsetter.clone());

        for _ in 0..3 {
            controller
                .sweep_open_windows(
                    |_| async { Answer::Present(revert_evidence("active", None)) },
                    |cid| {
                        assert_eq!(cid, "uhCEkPATH", "the sunset is looked up BY the migration");
                        async { Answer::Present(sunset_evidence("active")) }
                    },
                    |_| Some(carry(5, Some(5))),
                )
                .await;
        }

        assert!(reverter.calls().is_empty(), "an active path never reverts");
        assert_eq!(
            sunsetter.calls(),
            vec![("node_registry".to_string(), "uhCEkSUNSET".to_string())],
            "sealed exactly once — the second sweep finds a CLOSED window"
        );
        assert!(lineage.snapshot()["node_registry"].closed);
        assert!(lineage.open_windows().is_empty());
    }

    /// **Station 8's first Then.** With no sunset commitment, no peer closes
    /// its v1 chain — and the window is left exactly as it was.
    #[tokio::test]
    async fn no_sunset_commitment_no_seal() {
        let dir = tempfile::tempdir().unwrap();
        let lineage = window_on("runtime:coordinators:elohim:sunset-arm-none");
        let sunsetter = FakeSunsetter::new(Arc::clone(&lineage));
        let controller = AdoptionController::new(dir.path())
            .with_lineage_revert(
                Arc::clone(&lineage),
                FakeReverter::new(Arc::clone(&lineage)),
            )
            .with_lineage_sunset(sunsetter.clone());

        controller
            .sweep_open_windows(
                |_| async { Answer::Present(revert_evidence("active", None)) },
                |_| async { Answer::Absent },
                |_| Some(carry(5, Some(5))),
            )
            .await;

        assert!(sunsetter.calls().is_empty());
        assert!(!lineage.snapshot()["node_registry"].closed);
        assert_eq!(lineage.open_windows().len(), 1);
    }

    /// **The remedy outranks the seal.** A revoked path reverts and never
    /// falls through to the sunset — even when a sunset commitment is sitting
    /// right there reading active.
    #[tokio::test]
    async fn a_revoked_path_reverts_and_never_seals() {
        let dir = tempfile::tempdir().unwrap();
        let lineage = window_on("runtime:coordinators:elohim:sunset-arm-revoked");
        let reverter = FakeReverter::new(Arc::clone(&lineage));
        let sunsetter = FakeSunsetter::new(Arc::clone(&lineage));
        let controller = AdoptionController::new(dir.path())
            .with_lineage_revert(Arc::clone(&lineage), reverter.clone())
            .with_lineage_sunset(sunsetter.clone());

        controller
            .sweep_open_windows(
                |_| async { Answer::Present(revert_evidence("revoked", None)) },
                |_| async { Answer::Present(sunset_evidence("active")) },
                |_| Some(carry(5, Some(5))),
            )
            .await;

        assert_eq!(reverter.calls().len(), 1);
        assert!(sunsetter.calls().is_empty(), "the seal is never reached");
        assert!(
            !lineage.snapshot()["node_registry"].closed,
            "a reverted role is not a sunset one"
        );
    }

    /// The convergence gate at the arm: this peer has not carried its whole v1
    /// chain, so it holds — with the sunset sitting there active.
    #[tokio::test]
    async fn an_unconverged_peer_holds_the_seal() {
        let dir = tempfile::tempdir().unwrap();
        let lineage = window_on("runtime:coordinators:elohim:sunset-arm-unconverged");
        let sunsetter = FakeSunsetter::new(Arc::clone(&lineage));
        let controller = AdoptionController::new(dir.path())
            .with_lineage_revert(
                Arc::clone(&lineage),
                FakeReverter::new(Arc::clone(&lineage)),
            )
            .with_lineage_sunset(sunsetter.clone());

        controller
            .sweep_open_windows(
                |_| async { Answer::Present(revert_evidence("active", None)) },
                |_| async { Answer::Present(sunset_evidence("active")) },
                // Three of five carried — this peer is not converged.
                |_| Some(carry(3, Some(5))),
            )
            .await;

        assert!(sunsetter.calls().is_empty());
        assert_eq!(lineage.open_windows().len(), 1);
    }

    /// A controller with a revert arm but NO sunsetter never reads a sunset at
    /// all — the read costs a projection query plus a zome call, and a
    /// controller that could not act on the answer has no business asking.
    #[tokio::test]
    async fn an_unequipped_controller_never_reads_a_sunset() {
        let dir = tempfile::tempdir().unwrap();
        let lineage = crossed_window();
        let controller = AdoptionController::new(dir.path()).with_lineage_revert(
            Arc::clone(&lineage),
            FakeReverter::new(Arc::clone(&lineage)),
        );

        controller
            .sweep_open_windows(
                |_| async { Answer::Present(revert_evidence("active", None)) },
                |_| async { panic!("no sunsetter wired, so no sunset read") },
                |_| Some(carry(5, Some(5))),
            )
            .await;
        assert!(!lineage.snapshot()["node_registry"].closed);
    }
}
