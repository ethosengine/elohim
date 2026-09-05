//! **Task 14b — the sunset arm** (Holochain Evolution Epic MVP, Station 8;
//! spec §4 step 5, §7 C2/C9/C14, §11.2's narrowed fence).
//!
//! # The one irreversible act
//!
//! Every other step of a crossing can be walked back. The apply installs
//! beside and carries; the revert (Task 13) returns authoring to v1 and leaves
//! v2 disabled and intact. The SUNSET cannot be walked back, because it closes
//! the v1 source chain — and a `CloseChain` cannot be un-authored. Probe B2
//! measured what a second one costs: the remote authority refuses the action
//! and issues a WARRANT against its author. So this arm's whole design is
//! about not doing it by accident.
//!
//! Three independent facts must all be true before a single byte is authored:
//!
//! 1. the role's window is OPEN (`LineageRoles::open_windows` — the same one
//!    definition the revert arm selects on, so a reverted or already-sunset
//!    role can never reach here);
//! 2. the `migrates-lineage` path that opened it still reads **active** on
//!    THIS peer's own DHT view (C5) — a revoked path reverts instead, and the
//!    revert arm runs first;
//! 3. a `sunsets-lineage` commitment **naming that migration** reads active on
//!    the same rail, and names the same role and the same DNA pair.
//!
//! Plus the convergence gate below. Anything unreadable holds; nothing here
//! ever seals on blindness (C4).
//!
//! # Fleet convergence is the operator's gate at MVP — said plainly
//!
//! The story's precondition is *"after fleet convergence"*, and this peer
//! cannot see the fleet. Storage knows only its OWN convergence: rung 5's
//! promote receipt lives in the ceremony, not in this process, and
//! `AppliedRelease` records what THIS peer applied. So the gate this arm
//! actually enforces is [`local_carry_converged`] — *this* peer carried its
//! whole v1 chain (`carried == v1_count`, as v1 itself reported the total).
//!
//! That is a necessary condition, not the sufficient one the story names.
//! **Fleet convergence remains the operator's gate**: the notarization of the
//! `sunsets-lineage` commitment IS the act that asserts it, and this arm treats
//! it as such. Naming the difference here rather than implying the stronger
//! check is the point — a peer that sealed while a neighbour was still mid-carry
//! would strand that neighbour behind a closed chain.
//!
//! # What this arm does NOT do: disable anything
//!
//! Station 8's story says each peer's runtime disables its v1 cell. This build
//! disables NOTHING, and that is a deliberate narrowing rather than an omission:
//! the conductor's `disable_app` is WHOLE-APP, and the base app carries every
//! other role (content, identity, governance). Disabling it to fence one role's
//! v1 chain would take the node off the air. Per-cell disable is the shape that
//! would make the story's line literal, and it does not exist at 0.7.
//!
//! What stands in its place is the fence the DNA already carries: `seal_close`
//! authors the `CloseChain` on v1 and carries it into v2 as a witness, and v2's
//! integrity `after close` rule refuses any carried proof authored above that
//! close — on every peer whose own witness history holds the close. The v1 cell
//! stays enabled and READABLE (C14: the closed chain is kept, never deleted),
//! and the role's writes go to v2 because [`crate::lineage_roles`] routes them
//! there. Nothing of this peer's is written to v1 again because nothing routes
//! there — not because the cell was taken away.
//!
//! # The mirror is the contract
//!
//! [`SealReceipt`] mirrors the v2 coordinator's own struct, which carries a
//! plain `#[derive(Serialize, Deserialize)]` and NO `rename_all` — so the wire
//! is snake_case, and every hash on it is a canonical base64 STRING the zome
//! rendered (a native `HoloHash` would arrive as a msgpack byte array and fail
//! this decode). Pinned by the round-trip test below.

use serde::Serialize;

use super::{AdoptionRefusal, PathEvidence, RefusalReason};
use crate::services::release_adoption::path_evidence::SunsetEvidence;
use seam_contracts::Answer;

/// The lifecycle state a commitment must read for this arm to act on it.
const ACTIVE_STATE: &str = "active";

/// The zome `seal_close` lives in — on the V2 (side) app, which is the only
/// end that can open a chain from the predecessor's close.
pub const SEAL_ZOME: &str = "node_registry_coordinator";
/// The coordinator function that performs the whole seal in one call.
pub const SEAL_FN: &str = "seal_close";

/// Wire mirror of the v2 cell's `seal_close` OUTPUT.
///
/// snake_case and `String` hashes, both because the zome says so — see the
/// module docs. `resumed` is `#[serde(default)]` on both sides, so a receipt
/// from a build predating the half-seal resume decodes as `false`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SealReceipt {
    /// The predecessor's `CloseChain` action, on the v1 chain.
    pub close_hash: String,
    /// The successor's `OpenChain` action, which names `close_hash`.
    pub open_hash: String,
    /// The witness that carried the close into v2, or the EMPTY STRING when a
    /// prior seal was found without one.
    pub witness_hash: String,
    /// The call found an existing seal for this lineage and authored NOTHING.
    pub already_sealed: bool,
    /// The call found a HALF-seal — v1 closed, v2 not yet opened — and RESUMED
    /// at the open step rather than closing v1 a second time. Worth reporting
    /// on its own line because a re-close is what Probe B2 measured earning a
    /// warrant.
    #[serde(default)]
    pub resumed: bool,
}

/// One `seal_close`, over a client connected to the V2 (side) app.
///
/// The work is bounded ON THE WASM SIDE before this call is made: the extern's
/// half-seal probe walks at most `HALF_SEAL_SCAN` (32) actions below v1's head
/// and the seal itself is three actions. No caller-side deadline is added —
/// one would abandon a conductor that keeps running (see the crate's
/// `conductor-call-is-uncancellable` rule).
pub async fn call_seal_close(
    client: &std::sync::Arc<crate::hc_client::HcClient>,
    v1_cell: &holochain_client::CellId,
) -> Result<SealReceipt, String> {
    let payload =
        rmp_serde::to_vec_named(v1_cell).map_err(|e| format!("encode seal_close v1_cell: {e}"))?;
    let bytes = client
        .call_zome(SEAL_ZOME, SEAL_FN, payload)
        .await
        .map_err(|e| e.to_string())?;
    rmp_serde::from_slice(&bytes).map_err(|e| format!("decode SealReceipt: {e}"))
}

/// Why this sweep is NOT sealing a window. Every variant is a fact about what
/// this peer could read, and every one of them is a HOLD rather than a refusal:
/// the next sweep asks again, and a sunset that never comes is the safe
/// outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SunsetHold {
    /// The migration path could not be read (C4 — blindness never seals).
    PathUnreadable,
    /// The path is readable and is not active — revoked, or never activated.
    /// A revoked path is the REVERT arm's business, and it runs first.
    PathNotActive,
    /// No `sunsets-lineage` commitment naming this migration is on this peer's
    /// DHT view. The ordinary state of every crossing before its sunset, and
    /// Station 8's first Then: *"no peer closes its v1 chain"*.
    NoSunsetCommitment,
    /// A sunset commitment was found but could not be read.
    SunsetUnreadable,
    /// The sunset commitment is not active — proposed, or itself revoked.
    SunsetNotActive,
    /// The sunset names a different role, or a different DNA pair, than the
    /// window it was matched to. Spec §3 binds both lineage arms to the same
    /// identity fields; a mismatch is somebody else's crossing.
    SunsetNamesAnotherCrossing,
    /// This peer has not finished carrying its own v1 chain into v2. Sealing
    /// now would close a chain whose facts are not all on the other side.
    NotConverged,
}

impl SunsetHold {
    pub fn label(self) -> &'static str {
        match self {
            SunsetHold::PathUnreadable => "path_unreadable",
            SunsetHold::PathNotActive => "path_not_active",
            SunsetHold::NoSunsetCommitment => "no_sunset_commitment",
            SunsetHold::SunsetUnreadable => "sunset_unreadable",
            SunsetHold::SunsetNotActive => "sunset_not_active",
            SunsetHold::SunsetNamesAnotherCrossing => "sunset_names_another_crossing",
            SunsetHold::NotConverged => "not_converged",
        }
    }
}

/// Seal, or hold and say why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SunsetVerdict {
    /// All three facts read true and this peer has carried its chain: seal.
    Seal,
    Hold(SunsetHold),
}

/// **The trigger, pure.** Given what this peer's own conductor said about the
/// migration path and about a sunset naming it, plus whether this peer's own
/// carry is complete, decide whether to seal.
///
/// The order of the checks is the order of specificity, and every unreadable
/// answer holds. There is deliberately no clause that seals on an assumption:
/// a sunset is the one act with no remedy, so `Seal` is reached only when three
/// positive readings and one local fact all line up.
///
/// A CLOSED window never reaches here at all — `LineageRoles::open_windows`
/// excludes it — which is what makes a second sweep over a sunset role a
/// no-op rather than a second seal.
pub fn sunset_decision(
    path: &Answer<PathEvidence>,
    sunset: &Answer<SunsetEvidence>,
    role: &str,
    carry_converged: bool,
) -> SunsetVerdict {
    let Answer::Present(path) = path else {
        return SunsetVerdict::Hold(SunsetHold::PathUnreadable);
    };
    if path.state != ACTIVE_STATE || path.revoked_at.is_some() {
        return SunsetVerdict::Hold(SunsetHold::PathNotActive);
    }
    let sunset = match sunset {
        Answer::Present(s) => s,
        Answer::Absent => return SunsetVerdict::Hold(SunsetHold::NoSunsetCommitment),
        Answer::Unreachable => return SunsetVerdict::Hold(SunsetHold::SunsetUnreadable),
    };
    if sunset.state != ACTIVE_STATE || sunset.revoked_at.is_some() {
        return SunsetVerdict::Hold(SunsetHold::SunsetNotActive);
    }
    // The sunset's own body must describe THIS crossing. `sunset_for` already
    // matched `migration_commitment_cid`; these are the identity fields spec §3
    // binds both lineage arms to, checked here so one commitment cannot close a
    // window it never named. An empty `role` on the sunset body is not a
    // wildcard — it fails this check, which is the fail-closed direction.
    if sunset.role != role
        || sunset.from_dna_hash != path.from_dna_hash
        || sunset.to_dna_hash != path.to_dna_hash
    {
        return SunsetVerdict::Hold(SunsetHold::SunsetNamesAnotherCrossing);
    }
    if !carry_converged {
        return SunsetVerdict::Hold(SunsetHold::NotConverged);
    }
    SunsetVerdict::Seal
}

/// Has THIS peer carried its whole v1 chain into v2?
///
/// `carried >= v1_count`, with `v1_count` being what **v1 itself reported** —
/// never a number this side derived, which is the same rule
/// [`super::LineageCarryReceipt::v1_count`] exists to keep. Three ways to be
/// false, all of them safe:
///
/// - no applied release on the channel (a restarted process: `AppliedRelease`
///   is ephemeral, so a peer that restarts after crossing holds no receipt and
///   will not seal until the release applies again — fail-closed, and worth
///   knowing);
/// - the release was applied by a vehicle that carries nothing;
/// - v1 never reported a total, so completeness is UNKNOWABLE and therefore not
///   established.
pub fn local_carry_converged(carry: Option<&super::LineageCarryReceipt>) -> bool {
    let Some(carry) = carry else {
        return false;
    };
    let Some(total) = carry.v1_count else {
        return false;
    };
    carry.carried >= total
}

/// What one sunset did. Recorded on the adoption report so `GET /admin/adoption`
/// shows it — the only place an operator can see that a chain was closed
/// without opening a conductor.
///
/// Every field is an observation of what THIS peer did to ITS OWN chains. C2
/// stands through the sunset too: nothing here moved a head, and nothing here
/// speaks for another peer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SunsetReceipt {
    /// The role whose window was closed.
    pub role: String,
    /// The side app that authors this role from now on. Named because after
    /// the sunset it is no longer "the side app" in any operational sense — it
    /// is simply where the role lives.
    pub lineage_app_id: String,
    /// The `sunsets-lineage` commitment that authorised this act.
    pub sunset_commitment_cid: String,
    /// The `CloseChain` on v1, base64.
    pub close_hash: String,
    /// The `OpenChain` on v2 naming that close, base64.
    pub open_hash: String,
    /// The witness that carried the close into v2, base64 — the EMPTY STRING
    /// when a prior seal was found without one.
    pub witness_hash: String,
    /// The seal found an existing one and authored nothing.
    pub already_sealed: bool,
    /// The seal found v1 closed with v2 unopened and RESUMED at the open step
    /// rather than closing v1 twice.
    pub resumed: bool,
    /// Unix seconds the sunset completed.
    pub at: i64,
}

/// The conductor seam the sunset path uses. ONE method, for the same structural
/// reason [`super::revert::SideAppAdmin`] has one: there is no `disable_app` and
/// no `uninstall_app` on this trait, so the sunset path cannot reach either
/// without a widening a reviewer sees.
#[async_trait::async_trait]
pub trait ChainSealer: Send + Sync {
    /// Run the v2 coordinator's own `seal_close` for `role`, against the side
    /// app that authors it. Returns the zome's receipt verbatim.
    async fn seal_close(&self, role: &str, side_app_id: &str) -> Result<SealReceipt, String>;
}

/// The sunset seam the controller's sweep calls. Implemented by
/// [`super::apply::HappLineageVehicle`] — the same object that opened the
/// window closes it for good, so the two halves cannot drift onto different
/// notions of which app id is the side app.
#[async_trait::async_trait]
pub trait LineageSunsetter: Send + Sync {
    async fn sunset_window(
        &self,
        role: &str,
        sunset_commitment_cid: &str,
    ) -> Result<SunsetReceipt, AdoptionRefusal>;
}

fn not_an_open_window(role: &str, detail: &str) -> AdoptionRefusal {
    AdoptionRefusal::new(
        RefusalReason::ApplyFailed,
        format!("cannot sunset role '{role}': {detail}"),
    )
}

/// **The sunset path.** A free function rather than a method, so the ceremony
/// is testable end to end against a fake conductor seam.
///
/// # The order IS the safety argument, and it is the apply's read forward
///
/// The apply's claim is *"any failure leaves the side app installed and the
/// window CLOSED"*. This one's is the mirror image:
///
/// **Any failure leaves the window OPEN.**
///
/// 1. **The guard**, on the same OPEN predicate the sweep selected with — so a
///    direct call and a swept call cannot disagree about what "open" means, and
///    a window already sunset (or reverted) refuses without touching a
///    conductor.
/// 2. **Seal FIRST.** `seal_close` closes v1 toward v2, opens v2 from that
///    close, and witnesses the close in v2 — in that order, atomically, in the
///    zome. If it fails, we return here and the window is STILL OPEN: the next
///    sweep asks again, and the zome's own idempotency (a completed seal reads
///    `already_sealed`, a half-seal `resumed`) means the retry never authors a
///    second `CloseChain`.
/// 3. **Then, and only then, close the window.** [`crate::lineage_roles::LineageRoles::sunset`]
///    sets `closed` and leaves authoring exactly where it is — on v2, which is
///    where it belongs from now on. After this line the role is no longer an
///    open window, so no further sweep, revert or sunset selects it. Station
///    8's last Then — *"a revocation after the sunset changes nothing"* — is
///    this flag and nothing more.
/// 4. **Disable nothing.** See the module docs: `disable_app` is whole-app and
///    the base app carries every other role.
pub async fn perform_sunset(
    lineage: &crate::lineage_roles::LineageRoles,
    sealer: &dyn ChainSealer,
    role: &str,
    sunset_commitment_cid: &str,
    now: i64,
) -> Result<SunsetReceipt, AdoptionRefusal> {
    let Some((_, window)) = lineage
        .open_windows()
        .into_iter()
        .find(|(open_role, _)| open_role == role)
    else {
        return Err(not_an_open_window(
            role,
            "this role has no OPEN lineage window (never crossed, already reverted, or already \
             SUNSET — the sunset is terminal and nothing reopens it)",
        ));
    };
    let lineage_app_id = window.authoring_app_id.clone();

    // (2) SEAL FIRST. A failure here returns with the window still open.
    let seal = sealer
        .seal_close(role, &lineage_app_id)
        .await
        .map_err(|e| {
            AdoptionRefusal::new(
                RefusalReason::ApplyFailed,
                format!(
                    "seal_close(role='{role}', app='{lineage_app_id}') failed: {e} — the window \
                     stays OPEN and the next sweep asks again"
                ),
            )
        })?;

    // (3) AND ONLY NOW is the window closed.
    lineage.sunset(role);

    tracing::info!(
        role,
        lineage_app_id = %lineage_app_id,
        sunset_commitment_cid,
        close_hash = %seal.close_hash,
        open_hash = %seal.open_hash,
        already_sealed = seal.already_sealed,
        resumed = seal.resumed,
        "release-adoption: lineage window SUNSET — v1 closed and still readable, v2 authors, \
         nothing disabled and nothing uninstalled"
    );

    Ok(SunsetReceipt {
        role: role.to_string(),
        lineage_app_id,
        sunset_commitment_cid: sunset_commitment_cid.to_string(),
        close_hash: seal.close_hash,
        open_hash: seal.open_hash,
        witness_hash: seal.witness_hash,
        already_sealed: seal.already_sealed,
        resumed: seal.resumed,
        at: now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::release_adoption::RosterEvidence;

    fn path(state: &str, revoked_at: Option<&str>) -> PathEvidence {
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

    fn sunset_ev(state: &str) -> SunsetEvidence {
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

    /// **The deliverable's precondition, asserted.** An active path plus an
    /// active sunset naming it plus a converged local carry — and nothing less.
    #[test]
    fn an_active_path_and_an_active_sunset_seal() {
        assert_eq!(
            sunset_decision(
                &Answer::Present(path("active", None)),
                &Answer::Present(sunset_ev("active")),
                "node_registry",
                true,
            ),
            SunsetVerdict::Seal
        );
    }

    /// **Station 8's first Then.** With no sunset commitment, no peer closes
    /// its v1 chain — and the hold says exactly that rather than falling
    /// through to a generic refusal.
    #[test]
    fn no_sunset_commitment_never_seals() {
        assert_eq!(
            sunset_decision(
                &Answer::Present(path("active", None)),
                &Answer::Absent,
                "node_registry",
                true,
            ),
            SunsetVerdict::Hold(SunsetHold::NoSunsetCommitment)
        );
    }

    /// **C4 — blindness never seals.** Neither an unreadable path nor an
    /// unreadable sunset establishes anything, and the one act with no remedy
    /// is the last place to guess.
    #[test]
    fn nothing_unreadable_ever_seals() {
        for blind in [Answer::Unreachable, Answer::Absent] {
            assert_eq!(
                sunset_decision(
                    &blind,
                    &Answer::Present(sunset_ev("active")),
                    "node_registry",
                    true
                ),
                SunsetVerdict::Hold(SunsetHold::PathUnreadable),
                "an unreadable path must never seal"
            );
        }
        assert_eq!(
            sunset_decision(
                &Answer::Present(path("active", None)),
                &Answer::Unreachable,
                "node_registry",
                true,
            ),
            SunsetVerdict::Hold(SunsetHold::SunsetUnreadable)
        );
    }

    /// A revoked path is the REVERT arm's business. Sealing it would close a
    /// chain the elohim just pulled the permission for.
    #[test]
    fn an_inactive_or_revoked_path_never_seals() {
        assert_eq!(
            sunset_decision(
                &Answer::Present(path("revoked", None)),
                &Answer::Present(sunset_ev("active")),
                "node_registry",
                true,
            ),
            SunsetVerdict::Hold(SunsetHold::PathNotActive)
        );
        assert_eq!(
            sunset_decision(
                &Answer::Present(path("active", Some("2026-09-04T10:00:00Z"))),
                &Answer::Present(sunset_ev("active")),
                "node_registry",
                true,
            ),
            SunsetVerdict::Hold(SunsetHold::PathNotActive)
        );
        assert_eq!(
            sunset_decision(
                &Answer::Present(path("proposed", None)),
                &Answer::Present(sunset_ev("active")),
                "node_registry",
                true,
            ),
            SunsetVerdict::Hold(SunsetHold::PathNotActive)
        );
    }

    /// A sunset commitment that is itself proposed or revoked is not authority.
    #[test]
    fn an_inactive_sunset_never_seals() {
        for state in ["proposed", "revoked"] {
            assert_eq!(
                sunset_decision(
                    &Answer::Present(path("active", None)),
                    &Answer::Present(sunset_ev(state)),
                    "node_registry",
                    true,
                ),
                SunsetVerdict::Hold(SunsetHold::SunsetNotActive)
            );
        }
        let mut revoked = sunset_ev("active");
        revoked.revoked_at = Some("2026-09-04T10:00:00Z".to_string());
        assert_eq!(
            sunset_decision(
                &Answer::Present(path("active", None)),
                &Answer::Present(revoked),
                "node_registry",
                true,
            ),
            SunsetVerdict::Hold(SunsetHold::SunsetNotActive)
        );
    }

    /// Spec §3 binds both lineage arms to the same identity fields — a sunset
    /// naming another role or another DNA pair closes nothing here.
    #[test]
    fn a_sunset_for_another_crossing_never_seals() {
        let mut other_role = sunset_ev("active");
        other_role.role = "lamad".to_string();
        assert_eq!(
            sunset_decision(
                &Answer::Present(path("active", None)),
                &Answer::Present(other_role),
                "node_registry",
                true,
            ),
            SunsetVerdict::Hold(SunsetHold::SunsetNamesAnotherCrossing)
        );

        let mut other_dna = sunset_ev("active");
        other_dna.to_dna_hash = "uhC0kELSEWHERE".to_string();
        assert_eq!(
            sunset_decision(
                &Answer::Present(path("active", None)),
                &Answer::Present(other_dna),
                "node_registry",
                true,
            ),
            SunsetVerdict::Hold(SunsetHold::SunsetNamesAnotherCrossing)
        );
    }

    /// The convergence gate. It is this peer's own carry, and the module docs
    /// say plainly that fleet convergence stays the operator's.
    #[test]
    fn an_unconverged_carry_never_seals() {
        assert_eq!(
            sunset_decision(
                &Answer::Present(path("active", None)),
                &Answer::Present(sunset_ev("active")),
                "node_registry",
                false,
            ),
            SunsetVerdict::Hold(SunsetHold::NotConverged)
        );
    }

    /// `carried == v1_count`, with the total read from v1 itself. An UNKNOWN
    /// total is not convergence — deriving one from `carried` would make the
    /// check true by construction.
    #[test]
    fn local_convergence_needs_v1s_own_total() {
        let receipt = |carried: u32, v1_count: Option<u32>| super::super::LineageCarryReceipt {
            role: "node_registry".to_string(),
            carried,
            v1_count,
            digest: "d".to_string(),
            v1_digest: "d".to_string(),
            witness_hashes: Vec::new(),
        };
        assert!(local_carry_converged(Some(&receipt(5, Some(5)))));
        assert!(local_carry_converged(Some(&receipt(6, Some(5)))));
        assert!(!local_carry_converged(Some(&receipt(4, Some(5)))));
        assert!(
            !local_carry_converged(Some(&receipt(5, None))),
            "an unknown total is not convergence"
        );
        assert!(
            !local_carry_converged(None),
            "no applied release — a restarted process holds no receipt and must not seal"
        );
    }

    /// The zome renders every hash as base64 and names its fields snake_case.
    /// A camelCase mirror would decode empty strings and report a seal with no
    /// close hash.
    #[test]
    fn the_seal_receipt_wire_is_snake_case_and_round_trips() {
        let receipt = SealReceipt {
            close_hash: "uhCkkCLOSE".to_string(),
            open_hash: "uhCkkOPEN".to_string(),
            witness_hash: "uhCkkWITNESS".to_string(),
            already_sealed: false,
            resumed: true,
        };
        let encoded = rmp_serde::to_vec_named(&receipt).unwrap();
        let as_json: serde_json::Value = rmp_serde::from_slice(&encoded).unwrap();
        for key in [
            "close_hash",
            "open_hash",
            "witness_hash",
            "already_sealed",
            "resumed",
        ] {
            assert!(
                as_json.get(key).is_some(),
                "the zome's field '{key}' must be on the wire verbatim: {as_json}"
            );
        }
        assert_eq!(
            rmp_serde::from_slice::<SealReceipt>(&encoded).unwrap(),
            receipt
        );

        // `resumed` is additive on both sides — a pre-resume zome decodes false.
        let without = rmp_serde::to_vec_named(&serde_json::json!({
            "close_hash": "uhCkkCLOSE",
            "open_hash": "uhCkkOPEN",
            "witness_hash": "",
            "already_sealed": true,
        }))
        .unwrap();
        let decoded: SealReceipt = rmp_serde::from_slice(&without).expect("additive decode");
        assert!(!decoded.resumed);
        assert!(decoded.already_sealed);
    }

    // ── the sunset path ────────────────────────────────────────────────────

    use crate::lineage_roles::{LineageRoles, WindowOrigin};
    use std::sync::{Arc, Mutex};

    /// A fake seal seam. Records what it was asked to seal AND what the
    /// resolver said at the moment of the call, which is how "seal BEFORE the
    /// window closes" is asserted without a clock.
    struct FakeSealer {
        lineage: Arc<LineageRoles>,
        calls: Mutex<Vec<(String, String, bool)>>,
        receipt: SealReceipt,
        fail_with: Option<String>,
    }

    impl FakeSealer {
        fn new(lineage: Arc<LineageRoles>) -> Self {
            Self {
                lineage,
                calls: Mutex::new(Vec::new()),
                receipt: SealReceipt {
                    close_hash: "uhCkkCLOSE".to_string(),
                    open_hash: "uhCkkOPEN".to_string(),
                    witness_hash: "uhCkkWITNESS".to_string(),
                    already_sealed: false,
                    resumed: false,
                },
                fail_with: None,
            }
        }
        fn failing(lineage: Arc<LineageRoles>, error: &str) -> Self {
            Self {
                fail_with: Some(error.to_string()),
                ..Self::new(lineage)
            }
        }
        fn resuming(lineage: Arc<LineageRoles>) -> Self {
            let base = Self::new(lineage);
            Self {
                receipt: SealReceipt {
                    resumed: true,
                    witness_hash: String::new(),
                    ..base.receipt.clone()
                },
                ..base
            }
        }
        /// `(role, side app id, whether the window was still OPEN at call time)`.
        fn calls(&self) -> Vec<(String, String, bool)> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl ChainSealer for FakeSealer {
        async fn seal_close(&self, role: &str, side_app_id: &str) -> Result<SealReceipt, String> {
            self.calls.lock().unwrap().push((
                role.to_string(),
                side_app_id.to_string(),
                !self.lineage.open_windows().is_empty(),
            ));
            match self.fail_with.as_ref() {
                Some(e) => Err(e.clone()),
                None => Ok(self.receipt.clone()),
            }
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

    /// **The deliverable's shape, on the storage side.** The chains seal, the
    /// window closes, authoring stays on v2, and nothing is disabled — the
    /// receipt names the commitment that authorised it.
    #[tokio::test]
    async fn a_sunset_seals_then_closes_the_window_and_leaves_authoring_on_v2() {
        let lineage = open_window();
        let sealer = FakeSealer::new(Arc::clone(&lineage));

        let receipt = perform_sunset(
            lineage.as_ref(),
            &sealer,
            "node_registry",
            "uhCEkSUNSET",
            1_700_000_000,
        )
        .await
        .expect("the window was open");

        assert_eq!(receipt.role, "node_registry");
        assert_eq!(receipt.lineage_app_id, "elohim@SIDE");
        assert_eq!(receipt.sunset_commitment_cid, "uhCEkSUNSET");
        assert_eq!(receipt.close_hash, "uhCkkCLOSE");
        assert_eq!(receipt.open_hash, "uhCkkOPEN");
        assert!(!receipt.already_sealed);
        assert!(!receipt.resumed);

        // The seal ran while the window was still OPEN — order asserted, not
        // commented.
        assert_eq!(
            sealer.calls(),
            vec![("node_registry".to_string(), "elohim@SIDE".to_string(), true)]
        );

        let snap = lineage.snapshot();
        let role = &snap["node_registry"];
        assert!(role.closed, "the window is closed for good");
        assert_eq!(
            role.authoring_app_id, "elohim@SIDE",
            "v2 authors from now on"
        );
        assert_eq!(
            role.reading_app_id, "elohim",
            "v1 stays the reading pointer"
        );
        assert!(lineage.open_windows().is_empty());
    }

    /// **Any failure leaves the window OPEN.** A seal that could not run must
    /// not close a window — the next sweep asks again, and the zome's own
    /// idempotency makes that retry safe.
    #[tokio::test]
    async fn a_failed_seal_leaves_the_window_open() {
        let lineage = open_window();
        let sealer = FakeSealer::failing(Arc::clone(&lineage), "websocket closed");

        let refused = perform_sunset(
            lineage.as_ref(),
            &sealer,
            "node_registry",
            "uhCEkSUNSET",
            1_700_000_000,
        )
        .await
        .expect_err("the seal failed");
        assert!(refused.detail.contains("websocket closed"), "{refused:?}");

        assert!(!lineage.snapshot()["node_registry"].closed);
        assert_eq!(
            lineage.open_windows().len(),
            1,
            "the window is still open, so the next sweep retries"
        );
    }

    /// **Idempotence.** A second sweep finds no open window and never reaches
    /// the seal; a direct second call refuses WITHOUT a conductor round trip.
    #[tokio::test]
    async fn a_second_sunset_does_nothing_and_costs_no_conductor_call() {
        let lineage = open_window();
        let sealer = FakeSealer::new(Arc::clone(&lineage));

        perform_sunset(
            lineage.as_ref(),
            &sealer,
            "node_registry",
            "uhCEkSUNSET",
            1_700_000_000,
        )
        .await
        .expect("first sunset");
        assert!(lineage.open_windows().is_empty());

        let again = perform_sunset(
            lineage.as_ref(),
            &sealer,
            "node_registry",
            "uhCEkSUNSET",
            1_700_000_001,
        )
        .await;
        assert!(again.is_err(), "the second call refuses");
        assert_eq!(sealer.calls().len(), 1, "and spent no second seal");
    }

    /// A `resumed` seal — v1 closed, v2 unopened, resumed at the open step — is
    /// REPORTED rather than hidden. It is the trace of a half-seal, and Probe
    /// B2 is why anyone would want to see it.
    #[tokio::test]
    async fn a_resumed_seal_is_reported_on_the_receipt() {
        let lineage = open_window();
        let sealer = FakeSealer::resuming(Arc::clone(&lineage));

        let receipt = perform_sunset(
            lineage.as_ref(),
            &sealer,
            "node_registry",
            "uhCEkSUNSET",
            1_700_000_000,
        )
        .await
        .expect("a resumed seal still sunsets");
        assert!(receipt.resumed);
        assert_eq!(receipt.witness_hash, "");
        assert!(lineage.snapshot()["node_registry"].closed);
    }

    /// A REVERTED role is not an open window either — the remedy and the
    /// sunset select on one predicate, so a peer that went back to v1 can never
    /// be sealed onto v2 by a late sunset commitment.
    #[tokio::test]
    async fn a_reverted_role_is_never_sunset() {
        let lineage = open_window();
        lineage.revert("node_registry");
        let sealer = FakeSealer::new(Arc::clone(&lineage));

        assert!(perform_sunset(
            lineage.as_ref(),
            &sealer,
            "node_registry",
            "uhCEkSUNSET",
            1_700_000_000,
        )
        .await
        .is_err());
        assert!(
            sealer.calls().is_empty(),
            "no conductor call on a reverted role"
        );
    }

    #[test]
    fn hold_labels_are_the_wire_words() {
        assert_eq!(
            SunsetHold::NoSunsetCommitment.label(),
            "no_sunset_commitment"
        );
        assert_eq!(
            serde_json::to_value(SunsetHold::NotConverged).unwrap(),
            serde_json::json!("not_converged")
        );
    }
}
