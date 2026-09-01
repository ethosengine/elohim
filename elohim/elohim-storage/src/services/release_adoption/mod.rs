//! Release adoption controller — **observe mode**.
//!
//! Rung 5 of the upgrade-velocity debt snowball (backlog
//! `upgrade-propagation-p2p-design-arc`; design
//! `genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md`
//! §5/§6). The one genuinely new component of that spec, landed in its safe
//! half: this module **sees and judges** releases. It never applies one.
//!
//! # Why a controller, and why observe first
//!
//! "Bytes hash right" is transport; adoption is consent. The spec's P1 posture
//! is a k8s-shaped reconciliation loop — the DHT is the manifest, the peer's
//! own conductor is the only authority it obeys, and the controller eagerly
//! reconciles toward the head that authority elected. Every prior arm of this
//! snowball landed dry-run-first (`POST /admin/coordinators/sync?apply=false`
//! is always allowed because *reporting drift is diagnostics, not mutation*);
//! this one does the same thing at module granularity. `apply.rs` is T4's
//! (`task-release-apply-vehicles`) and the only thing this module leaves it is
//! the [`ApplyVehicle`] seam — declared here so the currency T4 implements
//! against is fixed before any vehicle exists, and **deliberately with no
//! implementations in this tree**.
//!
//! # The shape of one sweep
//!
//! ```text
//!   followed channels (runtime-config: ELOHIM_RELEASE_CHANNELS)
//!        │
//!   [watch]   resolve THIS node's conductor for the channel's canonical head
//!        │    (I1 — a peer hint may trigger a resolve, never supply the answer)
//!        │    no earned/staging head → idle, tier: none, NEVER a guess (C4)
//!        │
//!   [fetch]   the release manifest rides the winning version's `metadata_json`
//!        │    as {"kind":"release-manifest","manifest":{…}} (T2's ceremony
//!        │    driver); artifact bytes come by CID off the blob plane
//!        │
//!   [verify]  the floor (§6.3, floor-protected — never stage-priced):
//!        │      · manifest shape against T1's schema
//!        │      · blob CID + byte-length + digest match
//!        │      · compatibility envelope against the runtime passport's
//!        │        INSTALLED reality — the same per-role lineage refusal
//!        │        happ_manager enforces, moved to verify time
//!        │      · lineage parent against the channel's L2 version chain
//!        │        (the body field is a HINT that must match)
//!        │      · attestation threshold per the manifest's adoptionDiscipline
//!        │
//!   [apply]   ── T4. Not in this tree. ──
//! ```
//!
//! # P2P design gate (spec §5, carried)
//!
//! `AdoptionState` is **Ephemeral (C)**: process-local, reconstructable from
//! one sweep, surfaced on a node-local admin route, never notarized and never
//! gossiped as authority. The controller adds **no entity and no
//! `build_manifest()` route** — `GET /admin/adoption` is node-local exactly as
//! `POST /admin/coordinators/sync` is. The manifest itself adds no DHT entry
//! type: it rides the existing `Content` entry's `metadata_json` discriminator
//! valve, so this whole rung is DNA-hash-NEUTRAL.
//!
//! # Concern-canon disposition (registered in `seam-registry.yaml` at birth)
//!
//! - **C4 honest absence** — [`state::HeadTier::None`] is reported as `none`,
//!   never as "latest". A channel whose head this conductor cannot resolve
//!   reports `resolvedHead: null` with an `unreachable` verdict; *unreachable
//!   is never absence*. The head resolve is typed [`Answer`] for exactly this.
//! - **C6a bounded work** — [`watch::MAX_CHANNELS_PER_SWEEP`],
//!   [`watch::MAX_ARTIFACT_BYTES_PER_SWEEP`] and a **finite** backoff ladder
//!   ([`state::BACKOFF_LADDER_SECS`]). Work is sized BEFORE any conductor call
//!   is made, never abandoned after — a `call_zome` cannot be cancelled, so a
//!   caller-side timeout only abandons a conductor that keeps running while
//!   still holding the read permit.
//! - **C6b idempotent effect** — a sweep is a pure function of
//!   `(channel, releaseCid, installed reality)`; re-running it re-derives the
//!   same verdict and re-uses already-staged artifact bytes rather than
//!   re-fetching them.
//! - **C8 observability-per-decision** — every arm carries a typed
//!   [`RefusalReason`] (a [`ReasonLabel`] implementor, so a typo cannot mint a
//!   silent extra series) and increments
//!   `elohim_release_adoption_decisions_total{arm, reason}`.
//! - **C1 anti-self-election** — never re-derived here: the threshold arm goes
//!   through [`crate::services::release_attestation::count_qualifying_attestations`],
//!   whose `AdoptionDiscipline` can only be constructed by naming the release's
//!   builder, so the builder exclusion is a type obligation rather than a
//!   remembered step.
//! - **C9 identity/lineage continuity** — the per-role DNA-hash guard is the
//!   refusal that makes crossing the DNA line structurally impossible on this
//!   rung; crossing it is rung 6's migration ceremony.
//!
//! # Schema vendoring
//!
//! T1's schema (`elohim/rakia/schemas/v1/release-manifest.schema.json`) is
//! **OPEN by design** — a release manifest crosses the wire between
//! mixed-version peers, so the additive-wire floor says consumers MUST tolerate
//! unknown properties. The Rust mirror below therefore carries **no
//! `deny_unknown_fields`** anywhere, and every optional field is
//! `serde(default)`. The mirror is pinned to the schema by
//! `release_manifest_mirror_accepts_every_committed_fixture` and
//! `release_manifest_mirror_agrees_with_the_rakia_schema`, which load the
//! schema file and T2's fixtures from disk — two independent sources, so the
//! test cannot be measuring its own mirror.

pub mod state;
pub mod verify;
pub mod watch;

use std::path::PathBuf;

use elohim_epr::Reach;
use seam_contracts::ReasonLabel;
use serde::{Deserialize, Serialize};

pub use state::{AdoptionMode, FollowedChannel, HeadTier};

// ---------------------------------------------------------------------------
// The manifest — a vendored, deliberately OPEN mirror of T1's schema
// ---------------------------------------------------------------------------

/// `metadata_json.kind` discriminator T2's ceremony driver publishes a release
/// under. A channel ROOT carries `release-channel`; a release VERSION carries
/// this.
pub const RELEASE_MANIFEST_KIND: &str = "release-manifest";

/// What kind of runtime artifact a release carries. Determines which apply
/// vehicle T4 routes it to; this module only reads it for the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactClass {
    /// → `sync_coordinators` hot-swap (no re-key, no DHT churn).
    CoordinatorBundle,
    /// → runtime-config reload (seconds).
    ConfigEpr,
    /// → exe-slot swap. LOCAL and MESH rungs only (spec §9).
    StorageBinary,
    /// → the install path for already-joined or re-installing peers.
    HappBundle,
}

impl ArtifactClass {
    pub fn label(self) -> &'static str {
        match self {
            ArtifactClass::CoordinatorBundle => "coordinator-bundle",
            ArtifactClass::ConfigEpr => "config-epr",
            ArtifactClass::StorageBinary => "storage-binary",
            ArtifactClass::HappBundle => "happ-bundle",
        }
    }
}

/// One artifact blob in a release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    /// CIDv1, raw codec, sha2-256, base32-lower. The ADDRESS. A bare
    /// `sha256-<hex>` here is a schema violation, not a legacy tolerance.
    pub blob_cid: String,
    /// Exact byte length. A fetch that yields a different length is a typed
    /// refusal, never a retry.
    pub bytes: u64,
    /// sha2-256 hex of the same bytes — byte-equality verification only.
    pub sha256: String,
    /// Base name the artifact is applied under. Never a path.
    pub filename: String,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
}

/// The installed reality one role of a release binds to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleBinding {
    /// Crossing this line is rung 6's migration ceremony — refused at verify.
    pub dna_hash: String,
    /// The role's coordinator wasm hashes, sorted. The NORMATIVE field.
    pub coordinator_wasm_hashes: Vec<String>,
    /// Optional zome name → wasm hash map, the shape the runtime passport
    /// reports. Additive detail over `coordinator_wasm_hashes`; when present it
    /// is cross-checked against it rather than trusted alongside it.
    #[serde(default)]
    pub coordinator_zomes: Option<std::collections::BTreeMap<String, String>>,
}

/// Role name → the installed reality this release binds to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliesTo {
    pub roles: std::collections::BTreeMap<String, RoleBinding>,
}

/// The compatibility envelope — where unity is enforced (spec §8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    /// Protocol wire-format epochs this release SPEAKS. A release that speaks
    /// more than one epoch is what lets mixed-version peers keep talking.
    pub wire_epochs: Vec<u32>,
    /// The previous release's CID on this channel, or null for a first release.
    /// A HINT: verified against the channel's L2 version chain.
    #[serde(default)]
    pub lineage_parent_cid: Option<String>,
    /// The `serde(default)` additive-wire floor assertion.
    pub additive_only: bool,
}

/// Where the artifact came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltFrom {
    pub git_commit: String,
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub dirty: Option<bool>,
    #[serde(default)]
    pub repository: Option<String>,
}

/// Who built this, with what, from where.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    /// A builder's own attestation never suffices to EARN its release (C1).
    /// This field is what the threshold arm excludes by.
    pub builder_agent: String,
    pub toolchain: String,
    /// The artifact's OWN self-reported build info. Never the pin tag — the
    /// conductor-pin-ships-base-binary incident proved that untrustworthy.
    pub build_info: serde_json::Value,
    pub built_from: BuiltFrom,
}

/// A runtime release published as an EPR object on the content dataplane.
///
/// Mirror of `epr:schema:rakia:release-manifest:v1`. **Open by construction**
/// (see the module docs): no `deny_unknown_fields`, every optional field
/// defaulted, so a peer one build behind still reads a release that ADDED a
/// field within the lineage window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseManifest {
    /// Always `release-manifest` — the `metadata_json` discriminator.
    pub kind: String,
    /// Absent means `1.0`. Bumps only on a NON-additive shape change.
    #[serde(default)]
    pub manifest_version: Option<String>,
    pub channel_id: String,
    pub artifact_class: ArtifactClass,
    pub artifacts: Vec<Artifact>,
    pub applies_to: AppliesTo,
    pub envelope: Envelope,
    pub provenance: Provenance,
    /// The narrow-never-widen law applies to runtime heads verbatim.
    pub declared_reach: Reach,
    /// Constitutional artifact of the ceremony, notarized alongside the
    /// release. Reused verbatim from the attestation module (T5) so C1's
    /// builder exclusion is one type obligation, not two spellings.
    pub adoption_discipline: crate::services::release_attestation::ChannelAdoptionDiscipline,
    #[serde(default)]
    pub notes: Option<String>,
}

// ---------------------------------------------------------------------------
// The currency T4 implements against (names normative — task atom §Interface)
// ---------------------------------------------------------------------------

/// A release that passed the whole verify floor on THIS peer, with its bytes on
/// disk. The **only** thing an [`ApplyVehicle`] is ever handed.
///
/// Construction is private to [`verify`]: there is no way to mint one of these
/// without having run the floor, which is what makes "verified" a type fact
/// rather than a call-order convention.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedRelease {
    /// The channel this release is a version of.
    pub channel_id: String,
    /// The winning version's action hash (base64) — what the channel's
    /// canonical head declaration points at. This is the idempotency key: a
    /// sweep that sees the same `(channel_id, release_cid)` must be a no-op.
    pub release_cid: String,
    pub manifest: ReleaseManifest,
    /// Verified bytes on disk, **one per `manifest.artifacts` entry, in
    /// manifest order**. Each file is named by its artifact's `filename` and
    /// its digest has been checked against `sha256`; the vehicle owns
    /// placement from here.
    pub artifact_paths: Vec<PathBuf>,
}

/// What an apply vehicle returns when it succeeded. T4 fills the shape in; it
/// is declared here so the receipt T6 reads is fixed before any vehicle exists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedReceipt {
    pub channel_id: String,
    pub release_cid: String,
    /// Which vehicle acted (e.g. `sync_coordinators`, `runtime_config_reload`).
    pub vehicle: String,
    /// Unix seconds the apply completed.
    pub applied_at_unix: i64,
    /// Vehicle-specific evidence (per-role applied/skipped, config keys
    /// changed, exe-slot before/after). Additive by construction.
    #[serde(default)]
    pub detail: serde_json::Value,
}

/// The apply seam. **Deliberately unimplemented in this tree.**
///
/// T4 (`task-release-apply-vehicles`) implements one per artifact class. The
/// `apply` signature is normative and fixed by the task atom;
/// [`handles`](ApplyVehicle::handles) is additive with a default so a vehicle
/// written against the bare signature still compiles.
///
/// A vehicle takes a [`VerifiedRelease`] and nothing else — it cannot re-read
/// the channel, re-fetch bytes, or second-guess the floor. That is the point:
/// verification is floor-protected and happens exactly once, here.
pub trait ApplyVehicle {
    /// Apply a verified release locally. Refuse with a typed reason; never
    /// panic, never partially apply without saying so in the receipt.
    fn apply(&self, v: &VerifiedRelease) -> Result<AppliedReceipt, AdoptionRefusal>;

    /// Which artifact classes this vehicle handles. The default is **none** —
    /// a vehicle that does not declare its classes is never routed to, which
    /// is the safe direction for a seam whose implementations do not exist yet.
    fn handles(&self) -> &'static [ArtifactClass] {
        &[]
    }
}

// ---------------------------------------------------------------------------
// Typed refusals (C8 — every arm carries a reason)
// ---------------------------------------------------------------------------

/// Which arm of the loop produced a decision. The metric's `arm` label.
///
/// [`DecisionArm::Apply`] exists because the vocabulary is T4's too, and a
/// reason enum that cannot name the arm it will be used from is a rename
/// waiting to happen. It is deliberately **not** pre-touched at registration:
/// this build has no vehicle, so a zero on `{arm="apply"}` would read as "the
/// apply arm ran and did nothing", which is exactly the false-green the
/// pre-touch discipline exists to deny.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionArm {
    Watch,
    Fetch,
    Verify,
    Apply,
}

impl ReasonLabel for DecisionArm {
    const ALL: &'static [Self] = &[
        DecisionArm::Watch,
        DecisionArm::Fetch,
        DecisionArm::Verify,
        DecisionArm::Apply,
    ];

    fn label(&self) -> &'static str {
        match self {
            DecisionArm::Watch => "watch",
            DecisionArm::Fetch => "fetch",
            DecisionArm::Verify => "verify",
            DecisionArm::Apply => "apply",
        }
    }
}

/// The `reason` label emitted when an arm ended WITHOUT a refusal.
pub const REASON_OK: &str = "ok";

/// The `reason` label for the honest-absence exit: the channel resolved, and
/// there is no head to judge. Not a refusal — C3/C4's idle.
pub const REASON_IDLE: &str = "idle";

/// Why the controller refused. One variant per genuinely distinct cause: the
/// metric is only as useful as its ability to tell a correct refusal from a
/// substrate failure, and folding those two is the documented way a dashboard
/// stops being readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalReason {
    /// The channel's head resolved, but the winning version's `metadata_json`
    /// carries no `kind: "release-manifest"` envelope. A channel root, or a
    /// non-release version.
    ManifestAbsent,
    /// The envelope is there but the JSON did not decode.
    ManifestUndecodable,
    /// The manifest decoded but violates T1's schema (a required field, or a
    /// field whose pattern the schema pins).
    ManifestSchemaInvalid,
    /// The manifest's `channelId` is not the channel we resolved it from — a
    /// laundered or misfiled release.
    ChannelIdMismatch,
    /// A followed channel declares a mode this build refuses. Only `observe`
    /// is legal until T4 lands.
    ModeNotPermitted,
    /// No source served the artifact's bytes. **Not** absence — the bytes may
    /// simply not have replicated yet; the next sweep asks again.
    ArtifactUnavailable,
    /// Bytes arrived at a different length than the manifest declares.
    ArtifactLengthMismatch,
    /// Bytes arrived with a different digest than the manifest declares.
    ArtifactDigestMismatch,
    /// This peer could not read its own installed reality (no conductor admin
    /// connection, or the inventory timed out). C4: this is *unreachable*, not
    /// "the envelope does not match" — the envelope was never checked.
    InstalledRealityUnknown,
    /// The release binds to a role this peer does not run.
    RoleNotInstalled,
    /// **The DNA line.** The release binds to a different integrity lineage
    /// than the installed cell. The same per-role refusal
    /// `happ_manager::lineage_mismatch_error` enforces at apply time, moved to
    /// verify time. Crossing it is rung 6's ceremony.
    DnaLineageMismatch,
    /// The release supersedes coordinator wasm hashes this peer does not run.
    CoordinatorLineageMismatch,
    /// The release speaks no wire epoch this build speaks.
    WireEpochUnsupported,
    /// The release does not assert the additive-wire floor. A removal or
    /// repurposing is a declared fork or a rung-6 migration — never an
    /// accepted envelope.
    AdditiveFloorBroken,
    /// The manifest's declared lineage parent disagrees with the channel's L2
    /// version chain. The body field is a hint that MUST match.
    LineageParentMismatch,
    /// The L2 version chain could not be read, so the lineage hint establishes
    /// nothing in either direction (C5).
    LineageUnverifiable,
    /// The attestation threshold could not be read. **Not a pass** — the
    /// spec's explicit instruction for the pre-T5 world, kept as a live arm
    /// because a conductor that cannot answer must not be read as consent.
    ThresholdUnchecked,
    /// The threshold was read and is not met yet. The ordinary staging state.
    ThresholdUnmet,
    /// The threshold count is provably incomplete (`is_degraded`): a reason to
    /// sweep again, never to promote.
    ThresholdEvidenceDegraded,
    /// This peer has no conductor to resolve through. Unreachable, never
    /// absence — a topology fact about us, not a fact about the channel.
    ConductorUnavailable,
}

impl ReasonLabel for RefusalReason {
    const ALL: &'static [Self] = &[
        RefusalReason::ManifestAbsent,
        RefusalReason::ManifestUndecodable,
        RefusalReason::ManifestSchemaInvalid,
        RefusalReason::ChannelIdMismatch,
        RefusalReason::ModeNotPermitted,
        RefusalReason::ArtifactUnavailable,
        RefusalReason::ArtifactLengthMismatch,
        RefusalReason::ArtifactDigestMismatch,
        RefusalReason::InstalledRealityUnknown,
        RefusalReason::RoleNotInstalled,
        RefusalReason::DnaLineageMismatch,
        RefusalReason::CoordinatorLineageMismatch,
        RefusalReason::WireEpochUnsupported,
        RefusalReason::AdditiveFloorBroken,
        RefusalReason::LineageParentMismatch,
        RefusalReason::LineageUnverifiable,
        RefusalReason::ThresholdUnchecked,
        RefusalReason::ThresholdUnmet,
        RefusalReason::ThresholdEvidenceDegraded,
        RefusalReason::ConductorUnavailable,
    ];

    fn label(&self) -> &'static str {
        match self {
            RefusalReason::ManifestAbsent => "manifest_absent",
            RefusalReason::ManifestUndecodable => "manifest_undecodable",
            RefusalReason::ManifestSchemaInvalid => "manifest_schema_invalid",
            RefusalReason::ChannelIdMismatch => "channel_id_mismatch",
            RefusalReason::ModeNotPermitted => "mode_not_permitted",
            RefusalReason::ArtifactUnavailable => "artifact_unavailable",
            RefusalReason::ArtifactLengthMismatch => "artifact_length_mismatch",
            RefusalReason::ArtifactDigestMismatch => "artifact_digest_mismatch",
            RefusalReason::InstalledRealityUnknown => "installed_reality_unknown",
            RefusalReason::RoleNotInstalled => "role_not_installed",
            RefusalReason::DnaLineageMismatch => "dna_lineage_mismatch",
            RefusalReason::CoordinatorLineageMismatch => "coordinator_lineage_mismatch",
            RefusalReason::WireEpochUnsupported => "wire_epoch_unsupported",
            RefusalReason::AdditiveFloorBroken => "additive_floor_broken",
            RefusalReason::LineageParentMismatch => "lineage_parent_mismatch",
            RefusalReason::LineageUnverifiable => "lineage_unverifiable",
            RefusalReason::ThresholdUnchecked => "threshold_unchecked",
            RefusalReason::ThresholdUnmet => "threshold_unmet",
            RefusalReason::ThresholdEvidenceDegraded => "threshold_evidence_degraded",
            RefusalReason::ConductorUnavailable => "conductor_unavailable",
        }
    }
}

impl RefusalReason {
    /// The arm this reason can be emitted from. Used to pre-touch exactly the
    /// series a real branch can reach — a pre-touch of an unreachable
    /// `(arm, reason)` pair would be the same false-green as an absent series,
    /// pointing the other way.
    pub fn arm(self) -> DecisionArm {
        match self {
            RefusalReason::ConductorUnavailable
            | RefusalReason::ModeNotPermitted
            | RefusalReason::ManifestAbsent
            | RefusalReason::ManifestUndecodable => DecisionArm::Watch,

            RefusalReason::ArtifactUnavailable
            | RefusalReason::ArtifactLengthMismatch
            | RefusalReason::ArtifactDigestMismatch => DecisionArm::Fetch,

            RefusalReason::ManifestSchemaInvalid
            | RefusalReason::ChannelIdMismatch
            | RefusalReason::InstalledRealityUnknown
            | RefusalReason::RoleNotInstalled
            | RefusalReason::DnaLineageMismatch
            | RefusalReason::CoordinatorLineageMismatch
            | RefusalReason::WireEpochUnsupported
            | RefusalReason::AdditiveFloorBroken
            | RefusalReason::LineageParentMismatch
            | RefusalReason::LineageUnverifiable
            | RefusalReason::ThresholdUnchecked
            | RefusalReason::ThresholdUnmet
            | RefusalReason::ThresholdEvidenceDegraded => DecisionArm::Verify,
        }
    }

    /// Whether a later sweep could plausibly change this verdict WITHOUT a new
    /// release being published.
    ///
    /// This is the retry axis, and it is deliberately separate from the reason
    /// itself: `dna_lineage_mismatch` is a correct, terminal refusal (only a
    /// new release changes it), while `artifact_unavailable` is a substrate
    /// state that heals on its own. Backing off the same on both is how a
    /// controller either hammers a wall or sleeps through a cure.
    pub fn is_transient(self) -> bool {
        matches!(
            self,
            RefusalReason::ArtifactUnavailable
                | RefusalReason::InstalledRealityUnknown
                | RefusalReason::LineageUnverifiable
                | RefusalReason::ThresholdUnchecked
                | RefusalReason::ThresholdUnmet
                | RefusalReason::ThresholdEvidenceDegraded
                | RefusalReason::ConductorUnavailable
        )
    }
}

/// A refusal with its evidence. The `reason` is what the metric counts; the
/// payload is what the operator reads on `/admin/adoption`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionRefusal {
    /// Metric-safe label — `reason.label()`, flattened so the JSON is readable
    /// without a lookup table.
    pub reason: String,
    /// Human-readable evidence. Never load-bearing for a decision.
    pub detail: String,
    /// Which arm refused.
    pub arm: String,
    /// Whether a later sweep could change this without a new release.
    pub transient: bool,
    #[serde(skip)]
    reason_code: RefusalReason,
}

impl AdoptionRefusal {
    pub fn new(reason: RefusalReason, detail: impl Into<String>) -> Self {
        Self {
            reason: reason.label().to_string(),
            detail: detail.into(),
            arm: reason.arm().label().to_string(),
            transient: reason.is_transient(),
            reason_code: reason,
        }
    }

    /// The typed reason. Read this, never the `reason` string — the string is
    /// the wire/metric projection, the code is the decision.
    pub fn reason_code(&self) -> RefusalReason {
        self.reason_code
    }
}

impl std::fmt::Display for AdoptionRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.reason, self.detail)
    }
}

impl std::error::Error for AdoptionRefusal {}

#[cfg(test)]
mod tests {
    use super::*;
    use seam_contracts::{assert_reason_labels_conformant, assert_reason_labels_stable};

    /// **C8.** Every reason projects to a distinct, metric-safe label. A
    /// duplicate would silently merge two outcomes into one series — how a
    /// correct refusal and a substrate failure end up in the same bucket.
    #[test]
    fn refusal_reason_labels_are_conformant() {
        assert_reason_labels_conformant::<RefusalReason>();
        assert_reason_labels_conformant::<DecisionArm>();
    }

    /// The label set is a dashboard contract: pinning it makes a rename a
    /// deliberate act with a failing test attached, not a silent series break.
    #[test]
    fn refusal_reason_labels_are_stable() {
        assert_reason_labels_stable::<RefusalReason>(&[
            "manifest_absent",
            "manifest_undecodable",
            "manifest_schema_invalid",
            "channel_id_mismatch",
            "mode_not_permitted",
            "artifact_unavailable",
            "artifact_length_mismatch",
            "artifact_digest_mismatch",
            "installed_reality_unknown",
            "role_not_installed",
            "dna_lineage_mismatch",
            "coordinator_lineage_mismatch",
            "wire_epoch_unsupported",
            "additive_floor_broken",
            "lineage_parent_mismatch",
            "lineage_unverifiable",
            "threshold_unchecked",
            "threshold_unmet",
            "threshold_evidence_degraded",
            "conductor_unavailable",
        ]);
        assert_reason_labels_stable::<DecisionArm>(&["watch", "fetch", "verify", "apply"]);
    }

    /// A terminal refusal and a transient one must not share a retry posture.
    /// The DNA-line refusal is the canonical terminal case: only a NEW release
    /// can change it, so a controller that keeps re-fetching for it is burning
    /// a conductor on a decided question.
    #[test]
    fn the_dna_line_refusal_is_terminal_and_absence_is_not() {
        assert!(!RefusalReason::DnaLineageMismatch.is_transient());
        assert!(!RefusalReason::AdditiveFloorBroken.is_transient());
        assert!(RefusalReason::ArtifactUnavailable.is_transient());
        assert!(RefusalReason::InstalledRealityUnknown.is_transient());
        assert!(RefusalReason::ThresholdUnchecked.is_transient());
    }

    /// A refusal carries its arm so the metric cannot be labelled at the call
    /// site — the pairing is a property of the reason, not of who emitted it.
    #[test]
    fn every_refusal_names_the_arm_it_can_come_from() {
        for reason in RefusalReason::ALL {
            let refusal = AdoptionRefusal::new(*reason, "fixture");
            assert_eq!(refusal.arm, reason.arm().label());
            assert_eq!(refusal.reason_code(), *reason);
            // No refusal may claim the apply arm: this build has no vehicle.
            assert_ne!(reason.arm(), DecisionArm::Apply);
        }
    }
}
