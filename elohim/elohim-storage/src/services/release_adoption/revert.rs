//! **Task 13a — revert before sunset (Holochain Evolution Epic MVP,
//! Station 7).** The storage side of the remedy: the trigger that notices a
//! path has been revoked, and the receipt the revert leaves behind.
//!
//! # The claim this module keeps
//!
//! *"Revoking the migration commitment inside its horizon returns every peer
//! to v1 authoring, leaves v2 cells disabled and intact"* — with a bright line
//! under **intact**. Nothing here uninstalls, and nothing here can: the
//! conductor seam this module hands the vehicle
//! ([`SideAppAdmin`]) declares `disable_app` and nothing else, so
//! "never uninstall" is a property of the type rather than a rule someone has
//! to remember. Uninstall stays where an operator has to ask for it by name
//! (`POST /admin/lineage/reset` with `{"uninstall": true}`).
//!
//! # Why the trigger cannot live in `check_channel`
//!
//! The controller's C6b idempotence exit returns `Verdict::Applied` for an
//! ALREADY-APPLIED release without re-running `verify_path` — deliberately,
//! because re-deriving a converged verdict every minute forever is the shape
//! of a controller that is correct and unaffordable. But it means the peers
//! who actually crossed (james, matthew) never re-read the path they crossed
//! under, and so would never notice its revocation. The revert sweep is
//! therefore its own arm, keyed on the WINDOW rather than on the channel:
//! while a role's window is open, the path that opened it is re-read every
//! sweep, and that read is the ONLY work the arm does. With no window open it
//! costs zero conductor calls ([`LineageRoles::open_windows`] is a lock and a
//! filter).
//!
//! # The revert horizon at MVP
//!
//! The story's revocation is "inside its revert horizon", and the horizon is
//! declared on the COMMITMENT body (`window.revert_until`), not on the release
//! manifest: `release_attestation::PathRef` carries `commitmentCid` and
//! nothing else. So at MVP **the manifest carries no `revertUntil`, and a
//! revert is therefore always allowed while the window is open** — stated here
//! rather than silently assumed. What bounds it instead is the SUNSET: a
//! closed window is excluded from [`LineageRoles::open_windows`], so a
//! revocation notarized after the sunset reaches no window and changes
//! nothing, which is exactly Station 8's Then. Projecting
//! `window.revert_until` into `PathEvidence` and comparing it here is a
//! tightening, not a correction, and it belongs with the commitment-body work
//! rather than with this arm.

use serde::Serialize;

use super::{AdoptionRefusal, PathEvidence, RefusalReason};
use seam_contracts::Answer;

/// The lifecycle state a quorum-checked revocation records on the anchor it
/// revokes. Mirrors `path_evidence`'s own constant; kept here so the pure
/// decision below has no dependency on that module's private items.
const REVOKED_STATE: &str = "revoked";

/// Why a window is being reverted. Two causes, kept apart because an operator
/// reading `/admin/adoption` needs to know whether the elohim REVOKED the path
/// or whether the household simply re-elected the prior head — the first is a
/// governance act about this crossing, the second is an ordinary election that
/// happens to strand an open window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RevertTrigger {
    /// The `migrates-lineage` commitment this window crossed under now reads
    /// `revoked` on this peer's own DHT view (Task 19's lifecycle links, read
    /// down the peer's own conductor — never the author's say-so).
    PathRevoked,
    /// Rung 5's remedy applied to rung 6: the channel that elected the
    /// crossing has elected a different, non-lineage head (a
    /// `coordinator-bundle` / `happ-bundle` release re-electing the base).
    /// The ceremony said "go back", and a window left open behind it would
    /// keep authoring on v2 under a release nobody elected.
    PriorHeadReelected,
}

impl RevertTrigger {
    pub fn label(self) -> &'static str {
        match self {
            RevertTrigger::PathRevoked => "path_revoked",
            RevertTrigger::PriorHeadReelected => "prior_head_reelected",
        }
    }
}

/// Whether this evidence says the path is revoked.
///
/// Both clauses, deliberately. `path_evidence::resolve_lifecycle` already
/// stamps `revoked_at` whenever it resolves a `revoked` link (falling back to
/// the state word when the link carries no time), so in practice the first
/// clause suffices — but `verify_path` checks `revoked_at` and `state`
/// separately, and a trigger that agreed with only half of that rule would be
/// a place for the two to drift apart.
pub fn is_revoked(ev: &PathEvidence) -> bool {
    ev.revoked_at.is_some() || ev.state == REVOKED_STATE
}

/// **The trigger, pure.** Given what this peer's own conductor said about the
/// path that opened a window, and whether the channel has elected away from
/// the release that opened it, decide whether to revert.
///
/// The ordering is not arbitrary: `PathRevoked` wins when both are true,
/// because the governance act is the more specific fact and it is the one the
/// operator has to be told about.
///
/// **C4 — unreachable is never absence, and neither is a revert.** An
/// [`Answer::Unreachable`] path (a conductor that could not answer) and an
/// [`Answer::Absent`] one (a commitment not yet gossiped to this peer) both
/// establish NOTHING about revocation, so neither reverts on its own. They do
/// not veto the re-election arm either: that arm's evidence is the channel's
/// own election, which was read separately.
pub fn revert_decision(
    evidence: &Answer<PathEvidence>,
    elected_away: bool,
) -> Option<RevertTrigger> {
    if let Answer::Present(ev) = evidence {
        if is_revoked(ev) {
            return Some(RevertTrigger::PathRevoked);
        }
    }
    if elected_away {
        return Some(RevertTrigger::PriorHeadReelected);
    }
    None
}

/// What became of the window-time v2 records — the other half of Station 7.
///
/// The story asks for james's v2-authored record to be *re-authored* on v1
/// with the same entry hash, and for anything not yet re-authored to be
/// reported as `pending`, **never as lost**. Task 13b put the act on the v1
/// coordinator (`readopt_from(v2_cell, cursor, limit) -> ReadoptReceipt`:
/// `export_records` cross-cell from v2, `create_entry` natively on v1,
/// idempotent by entry hash); Task 13c drives it from here.
///
/// Three answers, and the difference between the last two is the whole point:
/// a walk that FAILED still says what the pages before it brought home, so a
/// half-finished readopt reads as partial rather than as nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ReadoptStatus {
    /// No readopt was attempted, and why. A node with no client registry (and
    /// so no way to dial its own base app) is the only production shape that
    /// reaches this; the reason says so rather than leaving a silent zero.
    NotAttempted { reason: String },
    /// The walk ran to the end of the successor's export.
    Readopted(super::readopt::ReadoptSummary),
    /// The walk died partway. `partial` is what the pages before it DID
    /// re-author — the story's "pending, never lost", with numbers.
    Failed {
        reason: String,
        partial: super::readopt::ReadoptSummary,
    },
}

/// The answer when no [`Readopter`] is wired at all.
pub fn readopt_not_attempted(role: &str) -> ReadoptStatus {
    ReadoptStatus::NotAttempted {
        reason: format!(
            "role '{role}': no readopt seam is wired on this node, so the window-time v2 records \
             were not re-authored on v1 by this revert. The v2 cell is DISABLED, never \
             uninstalled, so every one of them is intact and readable in it — pending \
             re-authoring, never lost."
        ),
    }
}

/// The 13c seam: re-author this agent's window-time v2 facts back onto v1.
///
/// Implemented by [`super::apply::HappLineageVehicle`] — the same object that
/// opened the window — so the v1 cell the readopt runs on and the v2 cell it
/// reads from are derived once, from the app ids the crossing itself installed.
#[async_trait::async_trait]
pub trait Readopter: Send + Sync {
    /// `side_app_id` is passed rather than looked up because by the time this
    /// runs the revert has already moved that id out of `authoring_app_id`;
    /// re-deriving it here is how the two halves would come to disagree.
    async fn readopt(&self, role: &str, side_app_id: &str) -> ReadoptStatus;
}

/// What one reverted window did. Recorded on the adoption report so
/// `GET /admin/adoption` shows it.
///
/// Every field is an observation of what THIS peer did to ITSELF. Nothing here
/// is an authority claim, and nothing here moved a head — **C2 stands through
/// the revert too**: the ceremony re-elected, this peer converged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevertReceipt {
    /// The role whose window was reverted.
    pub role: String,
    /// The side app that was authoring, and is now the historical reading
    /// pointer. Disabled by this revert; still installed.
    pub lineage_app_id: String,
    /// `path_revoked` | `prior_head_reelected`.
    pub reason: RevertTrigger,
    /// The `migrates-lineage` commitment the window crossed under, when the
    /// window recorded one. Absent for a window opened with no origin.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_commitment_cid: Option<String>,
    /// Unix seconds the revert completed.
    pub at: i64,
    /// Whether `disable_app` on the side app succeeded. **A failure here does
    /// not fail the revert**: authoring has already returned to base by that
    /// point, which is the safe direction, and a side app left enabled but
    /// unrouted is inert. Same discipline `POST /admin/lineage/reset` follows
    /// — one stuck side app never blocks the rest.
    pub disabled: bool,
    /// What `disable_app` said when it failed. Absent on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_error: Option<String>,
    /// The window-time v2 records (13b).
    pub readopt: ReadoptStatus,
}

/// The conductor seam the revert path uses — **and the reason "never
/// uninstall" is structural.**
///
/// One method. There is no `uninstall_app` here to call, so a future edit to
/// the revert path cannot reach one without first widening this trait, which
/// is a change a reviewer sees. Implemented for
/// `holochain_client::AdminWebsocket` in [`super::apply`]; faked in tests.
#[async_trait::async_trait]
pub trait SideAppAdmin: Send + Sync {
    async fn disable_app(&self, app_id: &str) -> Result<(), String>;
}

/// The trailing-sweep state seam. `LineageBridge` implements it; the vehicle
/// holds it as a `dyn` so the revert path is testable without a conductor
/// registry.
pub trait RoleSweepState: Send + Sync {
    /// Forget every neighbour sweep for `role`. The bridge's cursors are
    /// ordinals into a view of a side app this revert has just disabled, so a
    /// cursor left behind would resume at a position with nothing behind it —
    /// the same reasoning `POST /admin/lineage/reset` clears them for, at the
    /// same granularity but for ONE role.
    fn clear_role(&self, role: &str);
}

/// The revert seam the controller's sweep calls. Implemented by
/// [`super::apply::HappLineageVehicle`] — the same object that opened the
/// window closes it, so the two halves of the ceremony cannot drift onto
/// different notions of which app id is the side app.
#[async_trait::async_trait]
pub trait LineageReverter: Send + Sync {
    async fn revert_window(
        &self,
        role: &str,
        trigger: RevertTrigger,
    ) -> Result<RevertReceipt, AdoptionRefusal>;
}

/// The refusal a revert raises when the role it was asked about is not an open
/// window. Not an error condition so much as a race the sweep can lose (a
/// reset, or a concurrent revert, between the snapshot and the call) — so it
/// is `ApplyFailed` and transient, never a claim about the path.
fn not_an_open_window(role: &str, detail: &str) -> AdoptionRefusal {
    AdoptionRefusal::new(
        RefusalReason::ApplyFailed,
        format!("cannot revert role '{role}': {detail}"),
    )
}

/// **The revert path.** A free function rather than a method, so the ceremony
/// is testable end to end against a fake conductor seam — the vehicle that
/// owns it holds a real `AdminWebsocket`, which no unit test can build.
///
/// # The order IS the safety argument, and it is the mirror of the apply's
///
/// The apply vehicle's claim is "any failure leaves the side app INSTALLED and
/// the window CLOSED". This is the same claim read backwards:
///
/// 1. **Routing first.** [`LineageRoles::revert`] flips authoring back to base
///    and parks the side app id in `reading_app_id` as a historical pointer.
///    After this line no new write can reach v2 — which is the whole point,
///    and it is why it happens BEFORE the conductor is touched. A revert that
///    disabled the cell first would leave a window in which the role is still
///    routed to a cell that no longer answers.
/// 2. **Then bring the window-time records home** ([`Readopter`], Task 13c) —
///    and this step is HERE, before the disable, for a reason that is easy to
///    get wrong: `readopt_from` runs on v1 and reaches ACROSS to the v2 cell
///    for its export. A disabled app's cells answer no cross-cell call, so a
///    readopt placed after the disable would fail on every revert while
///    looking like an ordering detail. Its failure is reported on the receipt,
///    never fatal — the revert itself has already succeeded by this line.
/// 3. **Then disable, and NEVER uninstall.** [`SideAppAdmin`] has one method,
///    so this is structural rather than remembered. A failure here does NOT
///    fail the revert: authoring is already home, and a side app left enabled
///    but unrouted is inert. It is recorded on the receipt instead — the same
///    discipline `POST /admin/lineage/reset` follows.
/// 4. **Then drop the trailing sweep's cursors for this role.** They are
///    ordinals into a view of the cell just disabled.
///
/// # Idempotence
///
/// The guard is the OPEN predicate itself. After step 1 the role no longer
/// matches [`crate::lineage_roles::LineageRoles::open_windows`], so a second
/// sweep never reaches this function for the same window; and if it is called
/// directly anyway, the guard below refuses without touching the conductor. A
/// SUNSET window is refused by the same guard, which is Station 8's *"a
/// revocation after the sunset changes nothing"* — the sunset is the only
/// irreversible act, and it outranks the remedy.
pub async fn perform_revert(
    lineage: &crate::lineage_roles::LineageRoles,
    side_admin: &dyn SideAppAdmin,
    bridge: Option<&dyn RoleSweepState>,
    readopter: Option<&dyn Readopter>,
    role: &str,
    trigger: RevertTrigger,
    now: i64,
) -> Result<RevertReceipt, AdoptionRefusal> {
    // THE GUARD, and it reads the SAME predicate the sweep selected on — so a
    // direct call and a swept call cannot disagree about what "open" means.
    let Some((_, window)) = lineage
        .open_windows()
        .into_iter()
        .find(|(open_role, _)| open_role == role)
    else {
        return Err(not_an_open_window(
            role,
            "this role has no OPEN lineage window (never crossed, already reverted, or SUNSET — \
             a sunset is terminal and no revocation reopens it)",
        ));
    };
    let lineage_app_id = window.authoring_app_id.clone();
    let path_commitment_cid = window
        .origin
        .as_ref()
        .map(|o| o.path_commitment_cid.clone());

    // (1) ROUTING FIRST. No further write can reach v2 after this line.
    lineage.revert(role);

    // (2) THE WINDOW-TIME RECORDS, while the v2 cell can still answer. See the
    // ordering note above: this cannot move below the disable.
    let readopt = match readopter {
        Some(readopter) => readopter.readopt(role, &lineage_app_id).await,
        None => readopt_not_attempted(role),
    };
    if let ReadoptStatus::Failed { reason, .. } = &readopt {
        tracing::warn!(
            role,
            lineage_app_id = %lineage_app_id,
            error = %reason,
            "release-adoption: revert stands, but the window-time v2 records were only partly \
             re-authored on v1 — they remain intact in the disabled side app, never lost"
        );
    }

    // (3) DISABLE, never uninstall. Non-fatal by design.
    let (disabled, disable_error) = match side_admin.disable_app(&lineage_app_id).await {
        Ok(()) => (true, None),
        Err(e) => {
            tracing::warn!(
                role,
                lineage_app_id = %lineage_app_id,
                error = %e,
                "release-adoption: revert flipped authoring back to v1 but could not disable the \
                 side app — it stays installed and unrouted, which is inert; the revert stands"
            );
            (false, Some(e))
        }
    };

    // (4) The trailing sweep's cursors point into a view of the cell above.
    if let Some(bridge) = bridge {
        bridge.clear_role(role);
    }

    tracing::info!(
        role,
        lineage_app_id = %lineage_app_id,
        reason = trigger.label(),
        disabled,
        "release-adoption: lineage window REVERTED — v1 authoring, v2 reading, cell disabled and \
         intact"
    );

    Ok(RevertReceipt {
        role: role.to_string(),
        lineage_app_id,
        reason: trigger,
        path_commitment_cid,
        at: now,
        disabled,
        disable_error,
        readopt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::release_adoption::RosterEvidence;

    fn evidence(state: &str, revoked_at: Option<&str>) -> PathEvidence {
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

    #[test]
    fn an_active_path_never_reverts() {
        assert_eq!(
            revert_decision(&Answer::Present(evidence("active", None)), false),
            None
        );
    }

    #[test]
    fn a_revoked_path_reverts() {
        assert_eq!(
            revert_decision(
                &Answer::Present(evidence("active", Some("2026-09-04T10:00:00Z"))),
                false
            ),
            Some(RevertTrigger::PathRevoked)
        );
        // The state word alone, with no time on the link, is the same finding.
        assert_eq!(
            revert_decision(&Answer::Present(evidence("revoked", None)), false),
            Some(RevertTrigger::PathRevoked)
        );
    }

    /// **C4.** A conductor that could not answer, and a commitment that has
    /// not gossiped here yet, establish nothing about revocation. Reverting on
    /// either would tear a peer off v2 because of our own blindness.
    #[test]
    fn neither_unreachable_nor_absent_reverts_on_its_own() {
        assert_eq!(revert_decision(&Answer::Unreachable, false), None);
        assert_eq!(revert_decision(&Answer::Absent, false), None);
    }

    /// The re-election arm rides on the channel's own election, which is read
    /// separately — so a path this peer could not read does not veto it.
    #[test]
    fn re_election_reverts_even_when_the_path_read_failed() {
        assert_eq!(
            revert_decision(&Answer::Unreachable, true),
            Some(RevertTrigger::PriorHeadReelected)
        );
        assert_eq!(
            revert_decision(&Answer::Present(evidence("active", None)), true),
            Some(RevertTrigger::PriorHeadReelected)
        );
    }

    /// Both true: the governance act is the more specific fact and the one an
    /// operator has to be told about.
    #[test]
    fn revocation_outranks_re_election() {
        assert_eq!(
            revert_decision(&Answer::Present(evidence("revoked", None)), true),
            Some(RevertTrigger::PathRevoked)
        );
    }

    /// With no seam wired the revert still SAYS what became of the records,
    /// and says they are intact — the one thing the story forbids reporting as
    /// lost.
    #[test]
    fn the_unwired_readopt_answer_is_typed_and_names_its_reason() {
        let ReadoptStatus::NotAttempted { reason } = readopt_not_attempted("node_registry") else {
            panic!("an unwired seam must report NotAttempted");
        };
        assert!(reason.contains("node_registry"), "{reason}");
        assert!(reason.contains("never lost"), "{reason}");
    }

    // ── the revert path ────────────────────────────────────────────────────

    use crate::lineage_roles::{LineageRoles, WindowOrigin};
    use std::sync::{Arc, Mutex};

    /// A fake conductor seam. Records the app ids it was asked to disable AND
    /// what the resolver looked like at the moment of the call — which is how
    /// the ordering claim ("routing first, then disable") is asserted without
    /// a clock.
    struct FakeAdmin {
        lineage: Arc<LineageRoles>,
        calls: Mutex<Vec<(String, String)>>,
        fail_with: Option<String>,
    }

    impl FakeAdmin {
        fn new(lineage: Arc<LineageRoles>) -> Self {
            Self {
                lineage,
                calls: Mutex::new(Vec::new()),
                fail_with: None,
            }
        }
        fn failing(lineage: Arc<LineageRoles>, error: &str) -> Self {
            Self {
                lineage,
                calls: Mutex::new(Vec::new()),
                fail_with: Some(error.to_string()),
            }
        }
        /// `(app id disabled, the role's authoring app id AT THAT MOMENT)`.
        fn calls(&self) -> Vec<(String, String)> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl SideAppAdmin for FakeAdmin {
        async fn disable_app(&self, app_id: &str) -> Result<(), String> {
            self.calls
                .lock()
                .unwrap()
                .push((app_id.to_string(), self.lineage.app_id_for("node_registry")));
            match self.fail_with.as_ref() {
                Some(e) => Err(e.clone()),
                None => Ok(()),
            }
        }
    }

    #[derive(Default)]
    struct FakeSweep {
        cleared: Mutex<Vec<String>>,
    }

    impl RoleSweepState for FakeSweep {
        fn clear_role(&self, role: &str) {
            self.cleared.lock().unwrap().push(role.to_string());
        }
    }

    /// A fake readopt seam. Records what it was asked to readopt AND whether
    /// the side app had already been disabled at that moment — which is how
    /// the "readopt BEFORE the disable" ordering is asserted rather than
    /// commented. `readopt_from` reaches into the v2 cell, and a disabled app
    /// answers nothing.
    struct FakeReadopter {
        admin: Arc<FakeAdmin>,
        calls: Mutex<Vec<(String, String, bool)>>,
        summary: super::super::readopt::ReadoptSummary,
    }

    impl FakeReadopter {
        fn new(admin: Arc<FakeAdmin>) -> Self {
            Self {
                admin,
                calls: Mutex::new(Vec::new()),
                summary: super::super::readopt::ReadoptSummary {
                    readopted: 1,
                    already_present: 4,
                    foreign: 1,
                    pages: 1,
                    v2_digest: "v2digest".to_string(),
                    v2_total: Some(6),
                },
            }
        }
        /// `(role, side app id, whether disable_app had already run)`.
        fn calls(&self) -> Vec<(String, String, bool)> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl Readopter for FakeReadopter {
        async fn readopt(&self, role: &str, side_app_id: &str) -> ReadoptStatus {
            self.calls.lock().unwrap().push((
                role.to_string(),
                side_app_id.to_string(),
                !self.admin.calls().is_empty(),
            ));
            ReadoptStatus::Readopted(self.summary.clone())
        }
    }

    fn open_window() -> Arc<LineageRoles> {
        let lineage = Arc::new(LineageRoles::new("elohim", &["node_registry"]));
        lineage.open_window(
            "node_registry",
            "elohim@SIDE",
            Some(WindowOrigin {
                channel_id: "runtime:coordinators:elohim:commons".to_string(),
                release_cid: "uhCkkRELEASE".to_string(),
                path_commitment_cid: "uhCEkPATH".to_string(),
            }),
        );
        lineage
    }

    /// **The deliverable's shape, on the storage side.** v1 authoring, v2 as
    /// the reading pointer, the cell disabled, nothing uninstalled, and the
    /// window-time records reported — never claimed lost.
    #[tokio::test]
    async fn a_revert_marks_v1_authoring_disables_the_side_app_and_reports_the_records() {
        let lineage = open_window();
        let admin = Arc::new(FakeAdmin::new(Arc::clone(&lineage)));
        let sweep = FakeSweep::default();

        let receipt = perform_revert(
            lineage.as_ref(),
            admin.as_ref(),
            Some(&sweep),
            None,
            "node_registry",
            RevertTrigger::PathRevoked,
            1_700_000_000,
        )
        .await
        .expect("the window was open");

        assert_eq!(receipt.role, "node_registry");
        assert_eq!(receipt.lineage_app_id, "elohim@SIDE");
        assert_eq!(receipt.reason, RevertTrigger::PathRevoked);
        assert_eq!(receipt.path_commitment_cid.as_deref(), Some("uhCEkPATH"));
        assert!(receipt.disabled);
        assert!(receipt.disable_error.is_none());
        assert!(
            matches!(receipt.readopt, ReadoptStatus::NotAttempted { .. }),
            "with no readopt seam wired the revert still SAYS so — never a silent zero"
        );

        let snap = lineage.snapshot();
        let role = &snap["node_registry"];
        assert_eq!(role.authoring_app_id, "elohim", "v1 authoring");
        assert_eq!(role.reading_app_id, "elohim@SIDE", "v2 reading");
        assert!(!role.closed, "a revert is not a sunset");

        assert_eq!(sweep.cleared.lock().unwrap().as_slice(), ["node_registry"]);
    }

    /// **The ordering claim, asserted rather than commented.** `disable_app`
    /// must be reached only AFTER authoring has returned to base — the fake
    /// records the resolver's answer at call time, so a reordered
    /// implementation fails here. "Never uninstall" is not asserted because it
    /// is unrepresentable: [`SideAppAdmin`] has one method.
    #[tokio::test]
    async fn routing_returns_to_base_before_the_cell_is_disabled() {
        let lineage = open_window();
        assert_eq!(lineage.app_id_for("node_registry"), "elohim@SIDE");
        let admin = Arc::new(FakeAdmin::new(Arc::clone(&lineage)));

        perform_revert(
            lineage.as_ref(),
            admin.as_ref(),
            None,
            None,
            "node_registry",
            RevertTrigger::PriorHeadReelected,
            1_700_000_000,
        )
        .await
        .expect("the window was open");

        assert_eq!(
            admin.calls(),
            vec![("elohim@SIDE".to_string(), "elohim".to_string())],
            "disable_app was called exactly once, on the side app, and authoring was ALREADY \
             back at base when it ran"
        );
    }

    /// A conductor that will not disable the cell does not fail the revert:
    /// authoring is already home (the safe direction) and an unrouted side app
    /// is inert. The receipt says so rather than the log alone.
    #[tokio::test]
    async fn a_failed_disable_is_reported_not_fatal() {
        let lineage = open_window();
        let admin = Arc::new(FakeAdmin::failing(Arc::clone(&lineage), "websocket closed"));

        let receipt = perform_revert(
            lineage.as_ref(),
            admin.as_ref(),
            None,
            None,
            "node_registry",
            RevertTrigger::PathRevoked,
            1_700_000_000,
        )
        .await
        .expect("a stuck side app never blocks the revert");

        assert!(!receipt.disabled);
        assert_eq!(receipt.disable_error.as_deref(), Some("websocket closed"));
        assert_eq!(lineage.app_id_for("node_registry"), "elohim");
    }

    /// **Idempotence.** The second sweep finds no open window (the OPEN
    /// predicate excludes the reverted shape), so nothing happens — and if the
    /// path is called directly anyway it refuses WITHOUT touching the
    /// conductor.
    #[tokio::test]
    async fn a_second_revert_does_nothing_and_costs_no_conductor_call() {
        let lineage = open_window();
        let admin = Arc::new(FakeAdmin::new(Arc::clone(&lineage)));
        let sweep = FakeSweep::default();

        perform_revert(
            lineage.as_ref(),
            admin.as_ref(),
            Some(&sweep),
            None,
            "node_registry",
            RevertTrigger::PathRevoked,
            1_700_000_000,
        )
        .await
        .expect("first revert");

        assert!(
            lineage.open_windows().is_empty(),
            "a reverted window is no longer open, so a second sweep never selects it"
        );

        let again = perform_revert(
            lineage.as_ref(),
            admin.as_ref(),
            Some(&sweep),
            None,
            "node_registry",
            RevertTrigger::PathRevoked,
            1_700_000_001,
        )
        .await;
        assert!(again.is_err(), "the second call refuses");
        assert_eq!(admin.calls().len(), 1, "and spent no second conductor call");
        assert_eq!(
            sweep.cleared.lock().unwrap().len(),
            1,
            "and cleared the sweep exactly once"
        );
        // The routing is unchanged by the refused second call.
        let snap = lineage.snapshot();
        assert_eq!(snap["node_registry"].authoring_app_id, "elohim");
        assert_eq!(snap["node_registry"].reading_app_id, "elohim@SIDE");
    }

    /// **Station 8: the sunset is the only irreversible act.** A revocation
    /// notarized after the sunset reaches a CLOSED window, which the OPEN
    /// predicate excludes — so nothing changes and no cell is touched.
    #[tokio::test]
    async fn a_sunset_window_is_never_reverted() {
        let lineage = open_window();
        lineage.sunset("node_registry");
        let admin = Arc::new(FakeAdmin::new(Arc::clone(&lineage)));

        let refused = perform_revert(
            lineage.as_ref(),
            admin.as_ref(),
            None,
            None,
            "node_registry",
            RevertTrigger::PathRevoked,
            1_700_000_000,
        )
        .await;

        assert!(refused.is_err());
        assert!(
            admin.calls().is_empty(),
            "no conductor call on a sunset role"
        );
        let snap = lineage.snapshot();
        assert!(snap["node_registry"].closed);
        assert_eq!(
            snap["node_registry"].authoring_app_id, "elohim@SIDE",
            "the closed chain stays closed and v2 keeps authoring"
        );
    }

    /// A role that was never crossed refuses without touching anything.
    #[tokio::test]
    async fn a_role_with_no_window_refuses() {
        let lineage = Arc::new(LineageRoles::new("elohim", &["node_registry"]));
        let admin = Arc::new(FakeAdmin::new(Arc::clone(&lineage)));
        assert!(perform_revert(
            lineage.as_ref(),
            admin.as_ref(),
            None,
            None,
            "node_registry",
            RevertTrigger::PathRevoked,
            1_700_000_000,
        )
        .await
        .is_err());
        assert!(admin.calls().is_empty());
    }

    /// A window with no recorded origin still reverts — the receipt just has
    /// no path to name. (The controller's sweep skips such a window because it
    /// has no evidence to re-read; a direct call is a different question.)
    #[tokio::test]
    async fn a_window_without_an_origin_reverts_and_names_no_path() {
        let lineage = Arc::new(LineageRoles::new("elohim", &["node_registry"]));
        lineage.open_window("node_registry", "elohim@SIDE", None);
        let admin = Arc::new(FakeAdmin::new(Arc::clone(&lineage)));

        let receipt = perform_revert(
            lineage.as_ref(),
            admin.as_ref(),
            None,
            None,
            "node_registry",
            RevertTrigger::PriorHeadReelected,
            1_700_000_000,
        )
        .await
        .expect("the window was open");
        assert!(receipt.path_commitment_cid.is_none());
    }

    /// **Task 13c, and the ordering that makes it possible.** The readopt runs
    /// BEFORE `disable_app`, because `readopt_from` reaches into the v2 cell
    /// and a disabled app answers nothing. The fake records whether the disable
    /// had already happened, so a reordered implementation fails here rather
    /// than at 3am on a live revert.
    #[tokio::test]
    async fn the_readopt_runs_while_the_side_app_can_still_answer() {
        let lineage = open_window();
        let admin = Arc::new(FakeAdmin::new(Arc::clone(&lineage)));
        let readopter = FakeReadopter::new(Arc::clone(&admin));

        let receipt = perform_revert(
            lineage.as_ref(),
            admin.as_ref(),
            None,
            Some(&readopter),
            "node_registry",
            RevertTrigger::PathRevoked,
            1_700_000_000,
        )
        .await
        .expect("the window was open");

        assert_eq!(
            readopter.calls(),
            vec![(
                "node_registry".to_string(),
                "elohim@SIDE".to_string(),
                false
            )],
            "the readopt named the SIDE app and ran while it was still enabled"
        );
        let ReadoptStatus::Readopted(summary) = &receipt.readopt else {
            panic!("expected a counted readopt, got {:?}", receipt.readopt);
        };
        assert_eq!(summary.readopted, 1);
        assert_eq!(summary.already_present, 4);
        assert_eq!(summary.foreign, 1);
        assert_eq!(summary.complete(), Some(true));
        // …and the disable still happened, after.
        assert_eq!(admin.calls().len(), 1);
        assert!(receipt.disabled);
    }

    /// A readopt that DIED still reports what it moved, and never fails the
    /// revert: authoring is already home, and the records are intact in the
    /// disabled cell. "Pending, never lost", with numbers.
    #[tokio::test]
    async fn a_failed_readopt_is_reported_partial_never_fatal() {
        struct DyingReadopter;
        #[async_trait::async_trait]
        impl Readopter for DyingReadopter {
            async fn readopt(&self, _role: &str, _side_app_id: &str) -> ReadoptStatus {
                ReadoptStatus::Failed {
                    reason: "readopt_from(role='node_registry', cursor=Some(16)) failed on page 1"
                        .to_string(),
                    partial: super::super::readopt::ReadoptSummary {
                        readopted: 2,
                        pages: 1,
                        v2_digest: "v2digest".to_string(),
                        ..Default::default()
                    },
                }
            }
        }

        let lineage = open_window();
        let admin = Arc::new(FakeAdmin::new(Arc::clone(&lineage)));

        let receipt = perform_revert(
            lineage.as_ref(),
            admin.as_ref(),
            None,
            Some(&DyingReadopter),
            "node_registry",
            RevertTrigger::PathRevoked,
            1_700_000_000,
        )
        .await
        .expect("a failed readopt never fails the revert");

        let ReadoptStatus::Failed { reason, partial } = &receipt.readopt else {
            panic!("expected a partial readopt, got {:?}", receipt.readopt);
        };
        assert!(reason.contains("page 1"), "{reason}");
        assert_eq!(partial.readopted, 2, "what landed is still reported");
        assert_eq!(
            partial.complete(),
            None,
            "an unknown total is never rendered as completeness"
        );
        assert_eq!(
            lineage.app_id_for("node_registry"),
            "elohim",
            "v1 authoring"
        );
    }

    #[test]
    fn trigger_labels_are_the_wire_words() {
        assert_eq!(RevertTrigger::PathRevoked.label(), "path_revoked");
        assert_eq!(
            RevertTrigger::PriorHeadReelected.label(),
            "prior_head_reelected"
        );
        assert_eq!(
            serde_json::to_value(RevertTrigger::PathRevoked).unwrap(),
            serde_json::json!("path_revoked")
        );
    }
}
