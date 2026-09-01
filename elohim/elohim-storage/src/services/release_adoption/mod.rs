//! Release adoption controller — observe **and apply**.
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
//! this one did the same thing at module granularity, landing observe-only.
//!
//! **T4 (`task-release-apply-vehicles`, 2026-09-01) landed the second half.**
//! [`apply`] now implements one [`ApplyVehicle`] per artifact class, and
//! `mode: apply` is a legal declaration in `ELOHIM_RELEASE_CHANNELS`. What did
//! NOT change is the direction of the safety argument: observe is still the
//! default; a vehicle is still handed a [`VerifiedRelease`] and nothing else;
//! and every vehicle routes to machinery that already existed and was already
//! proven (`happ_manager::sync_coordinators_report`, the rung-4 runtime-config
//! reload, the mesh exe-slot). Apply invents no mechanics — it routes.
//!
//! **The canary-first-adoption fix (2026-09-01) landed the same day.** The
//! attestation threshold gates PROMOTION — an EARNED head — never staging
//! adoption; `mode: apply` had been refusing every STAGING head on
//! `threshold_unmet`, which meant a canary in apply mode could never adopt
//! the very head its own soak was supposed to attest, a deadlock by
//! construction. `mode: canary` is now the third legal declaration: it
//! adopts a verified STAGING head (threshold read and reported, never
//! enforced — the soak IS the evidence) as well as an EARNED one (threshold
//! enforced, same as `apply`). `mode: apply` still adopts EARNED heads only;
//! a verified STAGING head there is [`state::Verdict::Waiting`], not a
//! refusal.
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
//!        │      · attestation threshold per the manifest's adoptionDiscipline —
//!        │        EARNED heads only; a STAGING head's threshold is read and
//!        │        reported, never enforced (the threshold gates PROMOTION, not
//!        │        staging adoption)
//!        │
//!   [apply]   observe → report and stop, on either tier. canary → route a
//!        │    verified release of EITHER tier by artifact class to the EXISTING
//!        │    vehicle (coordinator hot-swap · runtime-config reload · exe-slot
//!        │    staging). apply → route only an EARNED release the same way; a
//!        │    verified STAGING release there is `waiting`, not applied and not
//!        │    refused. Idempotent on (channel, releaseCid), deferred under
//!        │    readable pressure, never self-exec'ing.
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

pub mod apply;
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

/// The apply seam. Implemented per artifact class in [`apply`] (T4).
///
/// A vehicle takes a [`VerifiedRelease`] and nothing else — it cannot re-read
/// the channel, re-fetch bytes, or second-guess the floor. That is the point:
/// verification is floor-protected and happens exactly once, here.
///
/// # Why this is `async` (a T4 amendment to T3's declared shape)
///
/// T3 declared `fn apply(&self, …)` — synchronous — before any vehicle existed.
/// Two of the four vehicles (`coordinator-bundle`, `happ-bundle`) route to
/// `happ_manager::sync_coordinators_report`, which is an `async` conductor
/// admin call, and there is no honest way to reach it from a sync fn on this
/// runtime: `Handle::block_on` panics inside a tokio worker and
/// `block_in_place` parks a worker thread for the whole of a multi-second
/// hot-swap. Widening to `async` is therefore the minimum change that lets the
/// seam mean what it says. Everything else about the currency is unchanged —
/// the argument, the return type, the typed refusal, and the additive
/// [`handles`](ApplyVehicle::handles) default are all exactly as declared.
#[async_trait::async_trait]
pub trait ApplyVehicle: Send + Sync {
    /// Apply a verified release locally. Refuse with a typed reason; never
    /// panic, never partially apply without saying so in the receipt.
    async fn apply(&self, v: &VerifiedRelease) -> Result<AppliedReceipt, AdoptionRefusal>;

    /// Which artifact classes this vehicle handles. The default is **none** —
    /// a vehicle that does not declare its classes is never routed to, which
    /// is the safe direction: a vehicle that forgets to declare its classes is
    /// inert rather than promiscuous.
    fn handles(&self) -> &'static [ArtifactClass] {
        &[]
    }

    /// A short, stable name for the receipt's `vehicle` field. Also the name an
    /// operator reads on `/admin/adoption`.
    fn name(&self) -> &'static str;
}

// ---------------------------------------------------------------------------
// Typed refusals (C8 — every arm carries a reason)
// ---------------------------------------------------------------------------

/// Which arm of the loop produced a decision. The metric's `arm` label.
///
/// [`DecisionArm::Apply`] was named by T3 before any vehicle existed and was
/// deliberately left un-pre-touched then — a zero on `{arm="apply"}` would have
/// read as "the apply arm ran and did nothing". **T4 landed the vehicles, so it
/// is now pre-touched like every other arm** (`watch::pretouch_metrics`): a
/// zero there is a measured zero.
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

/// **C6b.** The `reason` label for the idempotent apply exit: this peer has
/// already applied exactly this `(channelId, releaseCid)` pair, so a re-sweep
/// is a no-op.
///
/// Deliberately NOT a [`RefusalReason`]: nothing refused, and nothing needs to
/// change. It is also deliberately not folded into [`REASON_OK`] — a run of
/// `already_current` is the shape of a CONVERGED peer, and reading that as a
/// run of fresh applies is how a dashboard reports a stable fleet as a churning
/// one. It is emitted from the apply arm before ANY conductor call beyond the
/// head resolve.
pub const REASON_ALREADY_CURRENT: &str = "already_current";

/// **Design 2026-09-01 (canary-first adoption).** The `reason` label for
/// [`state::Verdict::Waiting`]: a peer in `apply` mode verified a STAGING
/// head and is doing nothing wrong — `apply` adopts EARNED heads only, and
/// only a `canary` soaks a staging one. Deliberately NOT a [`RefusalReason`]
/// (nothing refused) and deliberately not folded into [`REASON_OK`]
/// (dashboarding "waiting for promotion" as "verified and stopped" would hide
/// the exact deadlock this mode exists to avoid: a fleet stuck here forever
/// with no canary configured looks identical to a fleet in healthy observe
/// mode unless the reason is its own series).
pub const REASON_AWAITING_PROMOTION: &str = "awaiting_promotion";

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

    // ── the apply arm (T4) ────────────────────────────────────────────────
    //
    // Every reason below belongs to [`DecisionArm::Apply`]. They exist as
    // separate variants rather than one `apply_failed` because the CURES
    // differ, and a metric that cannot tell an operator-gate refusal from a
    // conductor failure from a stakes refusal is a metric that sends every
    // investigation down the same wrong path.
    /// The release verified, this peer is in `apply` mode, and no compiled
    /// vehicle declares the release's `artifactClass`. Terminal: only a new
    /// release (of a class this build handles) changes it.
    NoVehicleForClass,
    /// The vehicle's operator gate refuses. Today that is
    /// `ALLOW_COORDINATOR_UPDATE` (inheriting `ALLOW_DNA_REINSTALL`) for the
    /// coordinator/hApp classes — the SAME gate the boot path and
    /// `POST /admin/coordinators/sync` honour, so the three can never drift.
    ApplyNotPermitted,
    /// **C11.** The node is under a pressure signal it can read cheaply, so
    /// the apply is deferred to a later sweep — lag-within-window rather than
    /// churn. Transient by definition.
    DeferredBackpressure,
    /// The vehicle ran and its mechanism failed (a conductor error, an IO
    /// error, a drifted role the hot-swap could not heal). Transient: the
    /// substrate may be different next sweep. The detail carries what broke.
    ApplyFailed,
    /// **Spec §9.** A `storage-binary` release, on a peer whose DECLARED
    /// network stakes are not `Simulacra`. Fleet binaries stay out of this
    /// rung; the local/mesh developer trust context is the only one where a
    /// binary may be staged. Fail-closed: the default declaration is
    /// `Bootstrap`, so a node that declares nothing refuses.
    BinaryStakesNotSimulacra,
    /// **Spec §6.4.** A `happ-bundle` release on a peer that is not joined —
    /// it has no cell on this network's DNA, so it structurally cannot have
    /// performed the verified local resolve for the very channel that supplies
    /// the DNA. Its first bundle is seeded out of band. Transient: once the
    /// peer joins, the same release applies.
    BootstrapOutOfBand,
    /// A `config-epr` release names a knob this process captures once at boot
    /// (`runtime_config::BOOT_ONLY`). Writing it would report `applied` and
    /// change nothing — the exact lie that list exists to prevent. Terminal:
    /// only a new release, naming different keys, changes it.
    ConfigKnobBootOnly,
    /// A `config-epr` release on a node with no watched runtime-config path
    /// (`ELOHIM_RUNTIME_CONFIG_PATH` unset). Writing a file nothing reads is
    /// the same lie in a different costume.
    RuntimeConfigUnwatched,
    /// The verified artifact bytes are not usable by the vehicle its class
    /// routes to (no artifact at all, or a payload the vehicle cannot parse).
    /// Terminal — the bytes verified, so they will not become different.
    ApplyPayloadUnusable,
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
        RefusalReason::NoVehicleForClass,
        RefusalReason::ApplyNotPermitted,
        RefusalReason::DeferredBackpressure,
        RefusalReason::ApplyFailed,
        RefusalReason::BinaryStakesNotSimulacra,
        RefusalReason::BootstrapOutOfBand,
        RefusalReason::ConfigKnobBootOnly,
        RefusalReason::RuntimeConfigUnwatched,
        RefusalReason::ApplyPayloadUnusable,
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
            RefusalReason::NoVehicleForClass => "no_vehicle_for_class",
            RefusalReason::ApplyNotPermitted => "apply_not_permitted",
            RefusalReason::DeferredBackpressure => "deferred_backpressure",
            RefusalReason::ApplyFailed => "apply_failed",
            RefusalReason::BinaryStakesNotSimulacra => "binary_stakes_not_simulacra",
            RefusalReason::BootstrapOutOfBand => "bootstrap_out_of_band",
            RefusalReason::ConfigKnobBootOnly => "config_knob_boot_only",
            RefusalReason::RuntimeConfigUnwatched => "runtime_config_unwatched",
            RefusalReason::ApplyPayloadUnusable => "apply_payload_unusable",
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

            RefusalReason::NoVehicleForClass
            | RefusalReason::ApplyNotPermitted
            | RefusalReason::DeferredBackpressure
            | RefusalReason::ApplyFailed
            | RefusalReason::BinaryStakesNotSimulacra
            | RefusalReason::BootstrapOutOfBand
            | RefusalReason::ConfigKnobBootOnly
            | RefusalReason::RuntimeConfigUnwatched
            | RefusalReason::ApplyPayloadUnusable => DecisionArm::Apply,
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
                // The apply arm's transient half. `deferred_backpressure` is
                // transient BY DEFINITION (it is a statement about us, not the
                // release); `apply_failed` because the mechanism may work next
                // sweep; `bootstrap_out_of_band` because a peer that joins has
                // changed the fact the refusal was about — the same release
                // then applies. Everything else on this arm is a standing
                // declaration (a stakes level, an operator gate, a key the
                // manifest names) that only a NEW release or a deliberate
                // operator act reopens, so it parks at the ladder's ceiling
                // rather than climbing to it — still re-checked every half
                // hour, never hammered.
                | RefusalReason::DeferredBackpressure
                | RefusalReason::ApplyFailed
                | RefusalReason::BootstrapOutOfBand
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
            "no_vehicle_for_class",
            "apply_not_permitted",
            "deferred_backpressure",
            "apply_failed",
            "binary_stakes_not_simulacra",
            "bootstrap_out_of_band",
            "config_knob_boot_only",
            "runtime_config_unwatched",
            "apply_payload_unusable",
        ]);
        assert_reason_labels_stable::<DecisionArm>(&["watch", "fetch", "verify", "apply"]);
    }

    /// The two non-refusal reason labels are distinct from each other and from
    /// every refusal label. `already_current` sharing a bucket with `ok` would
    /// report a CONVERGED fleet as a continuously-applying one — which is
    /// exactly the reading that makes an apply-rate panel useless.
    #[test]
    fn the_non_refusal_reason_labels_never_collide_with_a_refusal() {
        assert_ne!(REASON_OK, REASON_IDLE);
        assert_ne!(REASON_OK, REASON_ALREADY_CURRENT);
        assert_ne!(REASON_IDLE, REASON_ALREADY_CURRENT);
        assert_ne!(REASON_OK, REASON_AWAITING_PROMOTION);
        assert_ne!(REASON_IDLE, REASON_AWAITING_PROMOTION);
        assert_ne!(REASON_ALREADY_CURRENT, REASON_AWAITING_PROMOTION);
        for reason in RefusalReason::ALL {
            let label = reason.label();
            assert_ne!(label, REASON_OK);
            assert_ne!(label, REASON_IDLE);
            assert_ne!(label, REASON_ALREADY_CURRENT);
            assert_ne!(label, REASON_AWAITING_PROMOTION);
        }
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
        }
    }

    /// **T4.** The apply arm is now REACHABLE — the inverse of T3's assertion
    /// that no refusal could claim it. Every one of the four arms must have at
    /// least one reason that can be emitted from it, or the arm vocabulary
    /// carries a name with no code behind it (the false-green the pre-touch
    /// discipline exists to deny, pointing the other way).
    #[test]
    fn every_decision_arm_has_at_least_one_reachable_refusal() {
        for arm in DecisionArm::ALL {
            assert!(
                RefusalReason::ALL.iter().any(|r| r.arm() == *arm),
                "arm {:?} names no refusal that can be emitted from it",
                arm
            );
        }
    }

    /// The apply arm's transient/terminal split is a COST decision, and it is
    /// deliberately asymmetric: a pressure deferral or a failed mechanism may
    /// cure itself, while a stakes declaration, an operator gate, or a key the
    /// manifest names does not — those park at the ladder's ceiling instead of
    /// spending four more sweeps climbing to it.
    #[test]
    fn the_apply_arms_retry_posture_splits_substrate_from_declaration() {
        assert!(RefusalReason::DeferredBackpressure.is_transient());
        assert!(RefusalReason::ApplyFailed.is_transient());
        assert!(RefusalReason::BootstrapOutOfBand.is_transient());

        assert!(!RefusalReason::BinaryStakesNotSimulacra.is_transient());
        assert!(!RefusalReason::ApplyNotPermitted.is_transient());
        assert!(!RefusalReason::ConfigKnobBootOnly.is_transient());
        assert!(!RefusalReason::NoVehicleForClass.is_transient());
        assert!(!RefusalReason::RuntimeConfigUnwatched.is_transient());
        assert!(!RefusalReason::ApplyPayloadUnusable.is_transient());
    }
}
