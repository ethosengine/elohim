//! Release soak/build attestations — the evidence leg that moves a release
//! from *staging* to *earned*.
//!
//! Spec: `genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md`
//! §5 (BuildAttestation / SoakAttestation). Atom:
//! `genesis/data/timeline/backlog/task-release-soak-attestation-rail.md`.
//!
//! Two public functions, both consumed by the adoption controller: T3 calls
//! [`count_qualifying_attestations`] in its threshold arm, T4 calls
//! [`author_soak_attestation`] post-apply. Both compose against the structs in
//! this module, never against storage tables.
//!
//! ## The DNA-hash constraint this module is shaped by
//!
//! The generated `ATTESTATION_KINDS` list is compiled INTO the integrity zome
//! (`content_store_integrity::attestation_validator` floor 1 +
//! `generated_attestation_kinds.rs`). A NEW attestation kind is therefore a
//! DNA-hash move — a network event, not a deploy. Proven live on the household
//! mesh 2026-09-01: `attestation:release-soak` is refused with
//! `unknown_attestation_subtype`.
//!
//! So release attestations RIDE an existing kind. The chosen kind is
//! [`RIDDEN_ATTESTATION_KIND`] = `attestation:device-health` (infrastructure
//! pillar). Rationale:
//!
//! - **The payload axis fits exactly.** device-health's declared metadata is a
//!   health metric summarised over an observation window
//!   (`device_id`, `health_metric`, `period_start`, `period_end`,
//!   `sample_count`, `summary_value`) — which is precisely a soak window.
//! - **The reading it produces is honest**: "device D reports availability over
//!   window [start,end], while running the release this attestation is anchored
//!   to." The subject anchor is the release; `device_id` names the device.
//! - **Its live reader cannot be poisoned.** The only consumer of
//!   `attestation:device-health` (`infrastructure` zome, ~lib.rs:821) reads
//!   `get_attestations_for_subject(doorway_id, …)` — keyed on a DEVICE cid.
//!   Release attestations anchor on a RELEASE cid, so the two streams never
//!   intersect.
//! - **The rejected alternative** was `attestation:content-quality`: its subject
//!   axis is right (a release manifest is a `Content` entry) but it is a
//!   *reach-grant* attestation with a closed `quality_dimension` enum, and it
//!   HAS a live consumer that renders verification badges
//!   (`app/lamad/.../data-loader.service.ts` `listBySubject(contentId,
//!   'attestation:content-quality')`). Riding it would make release soak
//!   evidence surface as content-verification badges to learners. That is
//!   semantic violence with a user-visible blast radius; device-health has
//!   none.
//!
//! The residual strain: the infrastructure manifest declares
//! `subject_kinds: ["device"]` while we anchor on a release CID. Nothing
//! enforces `subject_kinds` today (floors 2/3/4 are `TODO(C.3)`), and the
//! §11.1 constitutional DNA batch is where first-class
//! `attestation:soak` / `attestation:build-provenance` kinds belong.
//!
//! ## Where the discriminator and context live
//!
//! Inside `metadata_json`, under `proof_evidence` — NOT under
//! `evidence_json.summary_metric`. Three reasons, in order of force:
//!
//! 1. `proof_evidence` is *semantically* the right home: probe results are the
//!    evidence, `buildInfo` is the provenance of what was proven, `outcome` is
//!    the verdict.
//! 2. Integrity floor 8 validates `proof_evidence.class` and its required
//!    material, so our shape passes a LIVE validator floor rather than an
//!    unenforced one (proven on the mesh: `class: "audit"` without
//!    `merkle_root` is refused with `floor8_failed`).
//! 3. It keeps `evidence_json.summary_metric` conformant to the ridden kind's
//!    declared metadata schema (`device-health-metadata.schema.json`, which is
//!    `additionalProperties: false`), so a real device-health reader that ever
//!    encounters one of these rows sees a well-formed device-health summary.
//!
//! ## Why this module reads through the CONDUCTOR, not the local projection
//!
//! The `attestations` SQL table looks like the obvious source — it carries
//! `proof_evidence_json` verbatim and is indexed by `subject_cid`. It is NOT
//! usable as the threshold reader's source, for two defects measured on the
//! household mesh on 2026-09-01 (both live in files this atom must not edit;
//! see the atom's "Blockers for the integrator"):
//!
//! - **Identity collapse.** The coordinator stamps
//!   `Content.id = format!("attest-{kind}-{issuer}")`
//!   (`content_store/src/attestation.rs`). That id is the projection's PRIMARY
//!   KEY, so one issuer can hold at most ONE row per kind across ALL subjects,
//!   forever — a second release's soak attestation silently REPLACES the first.
//! - **Provenance laundering.** `reanchor_backfill::is_canonical_content_type`
//!   (`:51`) returns TRUE for `attestation:` prefixes, so
//!   `p2p::projection_reconcile` feeds peer-discovered attestation rows through
//!   `reanchor_backfill::run_once` → `ContentService::update_via_conductor` →
//!   `conductor_writes::call_create_content`, minting a NEW DHT entry authored
//!   by the LOCAL agent. Correct for ordinary content; for an attestation the
//!   author IS the claim. Measured: on `jessica`, all three of the probe's
//!   attestations projected with `issuer_cid = jessica`.
//!
//! C1 (exclude the builder) is unanswerable against a laundered issuer column.
//! So the reader goes to the conductor for BOTH halves and cross-checks them:
//!
//! 1. `content_store::get_attestations_for_subject(release_cid)` walks the
//!    `AttestationToSubject` links and yields the AUTHENTICATED `(cid, issuer)`
//!    pairs — the links are only ever created inside `issue_attestation`, so a
//!    re-authored copy has no link and cannot appear here.
//! 2. `content_store::get_content_by_id("attest-{kind}-{issuer}")` yields the
//!    full `Content` — `author_id` plus `metadata_json` with the context.
//! 3. The two must AGREE (`content.author_id == link issuer`) and the entry's
//!    own `subject_cid` / `proof_evidence.releaseCid` must both name the
//!    release under test.
//!
//! Step 3 makes the reader **fail-closed against both defects**: a laundered
//! copy fails the author check, and an id-collision (the issuer's row now
//! resolving to a *different* release) fails the release check. Neither can
//! inflate a count; both can only deflate one — which is why
//! [`QualifyingEvidence`] reports `provenance_mismatched` and `unresolved`
//! alongside `qualifying`, so a caller can never read an under-count as a real
//! evidence deficit.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::conductor_admission::AdmissionClass;
use crate::hc_client::HcClient;

/// Zome hosting the attestation coordinator (the `lamad` role's cell — the same
/// zome [`crate::services::conductor_writes`] addresses).
const ZOME_NAME: &str = "content_store";

/// The existing generated attestation kind release attestations ride.
/// See the module docs for the choice rationale; a NEW kind moves the DNA hash.
pub const RIDDEN_ATTESTATION_KIND: &str = "attestation:device-health";

/// `metadata_json.proof_evidence.kind` discriminator for a soak attestation.
pub const SOAK_DISCRIMINATOR: &str = "release-soak";

/// `metadata_json.proof_evidence.kind` discriminator for a build attestation.
/// Authored by the same rail (the builder's own evidence), and deliberately
/// NOT counted toward a promotion threshold — see [`count_qualifying_attestations`].
pub const BUILD_DISCRIMINATOR: &str = "release-build";

/// The `health_metric` a soak window reports on the ridden kind's metadata
/// schema. `availability` is the enum member that means "did it stay up".
const SOAK_HEALTH_METRIC: &str = "availability";

/// Ceiling on the per-issuer entry resolves one threshold read will make.
///
/// A `call_zome` cannot be cancelled — a caller-side timeout abandons the call
/// while the conductor keeps executing it, still holding the read permit. So
/// the work is SIZED before any call is made: attesters beyond this ceiling are
/// reported as `unresolved` (an honest partial, never a failure) and the next
/// sweep picks them up. A release's canary set is single-digit by design
/// (spec §5 head-plane cost), so this ceiling is slack, not a cap on ambition.
const MAX_RESOLVES_PER_SWEEP: usize = 32;

// ---------------------------------------------------------------------------
// Typed refusals (spec §6 step 5 — every arm carries a typed reason)
// ---------------------------------------------------------------------------

/// Why a release-attestation operation refused. Every variant is a *reason*, so
/// the adoption controller can attach a per-decision metric without re-deriving
/// the cause from a string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TypedRefusal {
    #[error("release_cid_empty: a release attestation must name the release it attests")]
    ReleaseCidEmpty,

    #[error("soak_context_incomplete: {field} is required to author a soak attestation")]
    SoakContextIncomplete { field: &'static str },

    #[error(
        "proof_evidence_incomplete: class '{class}' requires '{material}' \
         (integrity floor 8 would refuse this commit)"
    )]
    ProofEvidenceIncomplete {
        class: &'static str,
        material: &'static str,
    },

    #[error("conductor_unavailable: {0}")]
    ConductorUnavailable(String),

    #[error("conductor_refused: {0}")]
    ConductorRefused(String),

    #[error("wire_decode_failed: {0}")]
    WireDecodeFailed(String),
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

/// Handles the two public functions need. T3/T4 already hold an [`HcClient`];
/// both halves of this module are conductor-sourced (see the module docs on why
/// the local projection is not the reader's source), so this carries nothing
/// else.
pub struct ReleaseAttestationCtx {
    pub hc: Arc<HcClient>,
}

impl ReleaseAttestationCtx {
    pub fn new(hc: Arc<HcClient>) -> Self {
        Self { hc }
    }
}

/// One probe run inside the soak window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    pub name: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ProbeResult {
    pub fn green(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ok: true,
            detail: None,
        }
    }
    pub fn red(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ok: false,
            detail: Some(detail.into()),
        }
    }
}

/// The context that makes a release attestation worth reading — the reason the
/// spec calls this evidence *context-bearing*. Two peers' different experiences
/// of the same release are information (rakia stage-2-canopy), and archetype +
/// region are what let a regional channel elect the head that FITS while the
/// commons head holds the envelope.
///
/// Populate with [`SoakContext::from_runtime`], which reads the same sources
/// boot registration does (`Config.device_archetype` / `household_id` /
/// `node_role` / `region`) plus the runtime passport's own build info.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoakContext {
    /// `runtime:<artifact-class>:<network>:<channel-name>` — the channel whose
    /// head this release is a version under.
    pub channel_id: String,
    /// This node's own device/agent cid. Lands in the ridden kind's required
    /// `device_id` metadata field.
    pub device_id: String,
    /// `Config.device_archetype` — the diversity axis a later threshold can
    /// require (spec §11.2), carried now so that becomes additive.
    pub device_archetype: String,
    /// Archetype capability level (`devices.json`), 0 when unknown.
    pub capability_level: i32,
    /// `Config.region` — the other diversity axis.
    pub region: Option<String>,
    pub household_id: Option<String>,
    pub node_role: Option<String>,
    /// The RUNNING binary's own build info — never the pin tag. Closes the
    /// base-vs-fork axis the conductor-pin-ships-base-binary incident opened.
    pub build_info: BuildInfoRef,
    /// RFC3339. Floor 5's parser accepts `YYYY-MM-DDTHH:MM:SS` + `Z`/offset.
    pub window_start: String,
    pub window_end: String,
}

/// The subset of the runtime passport's build info a release attestation
/// carries. A local mirror rather than a dependency, so the seam stays explicit
/// (the convention `conductor_writes::CollectiveWire` uses).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfoRef {
    pub version: String,
    pub commit: String,
    pub service: String,
}

impl From<&elohim_compute::BuildInfo> for BuildInfoRef {
    fn from(b: &elohim_compute::BuildInfo) -> Self {
        Self {
            version: b.version.clone(),
            commit: b.commit.clone(),
            service: b.service.clone(),
        }
    }
}

impl SoakContext {
    /// Populate from the same runtime sources boot registration reads.
    ///
    /// `capability_level` is passed in rather than re-derived: the archetype
    /// fixture loader in
    /// [`crate::services::boot_registration`] is private, and the value it
    /// resolved is already on this node's `stewarded_nodes` row. Callers that
    /// have neither pass `0` — the field is a diversity hint, never a gate.
    #[allow(clippy::too_many_arguments)]
    pub fn from_runtime(
        config: &crate::config::Config,
        channel_id: impl Into<String>,
        device_id: impl Into<String>,
        capability_level: i32,
        build: &elohim_compute::BuildInfo,
        window_start: impl Into<String>,
        window_end: impl Into<String>,
    ) -> Self {
        Self {
            channel_id: channel_id.into(),
            device_id: device_id.into(),
            device_archetype: config
                .device_archetype
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            capability_level,
            region: config.region.clone(),
            household_id: config.household_id.clone(),
            node_role: config.node_role.clone(),
            build_info: BuildInfoRef::from(build),
            window_start: window_start.into(),
            window_end: window_end.into(),
        }
    }

    fn validate(&self) -> Result<(), TypedRefusal> {
        for (field, value) in [
            ("channelId", self.channel_id.as_str()),
            ("deviceId", self.device_id.as_str()),
            ("deviceArchetype", self.device_archetype.as_str()),
            ("windowStart", self.window_start.as_str()),
            ("windowEnd", self.window_end.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(TypedRefusal::SoakContextIncomplete { field });
            }
        }
        Ok(())
    }
}

/// The verdict a soak window reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SoakVerdict {
    Pass,
    Fail,
}

impl SoakVerdict {
    fn as_str(self) -> &'static str {
        match self {
            SoakVerdict::Pass => "pass",
            SoakVerdict::Fail => "fail",
        }
    }
}

/// What the soak observed. A FAILING soak is still authored — the refusal and
/// the evidence behind it are what feed contest/revert (spec §6 step 5) — it
/// simply never qualifies toward a promotion threshold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoakOutcome {
    pub verdict: SoakVerdict,
    pub probe_results: Vec<ProbeResult>,
    pub note: Option<String>,
}

impl SoakOutcome {
    pub fn pass(probe_results: Vec<ProbeResult>) -> Self {
        Self {
            verdict: SoakVerdict::Pass,
            probe_results,
            note: None,
        }
    }
    pub fn fail(probe_results: Vec<ProbeResult>, note: impl Into<String>) -> Self {
        Self {
            verdict: SoakVerdict::Fail,
            probe_results,
            note: Some(note.into()),
        }
    }
    fn green_count(&self) -> usize {
        self.probe_results.iter().filter(|p| p.ok).count()
    }
}

/// What an authored attestation is addressed by. Mirrors the coordinator's
/// `AttestationOutput`; `cid` is the entry's `EntryHash`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestationRef {
    pub cid: String,
    pub attestation_kind: String,
    pub subject_cid: String,
    pub issuer_cid: String,
}

// ---------------------------------------------------------------------------
// Adoption discipline — C1 is enforced by the type, not by remembering
// ---------------------------------------------------------------------------

/// The channel's declared discipline, exactly as it appears in a release
/// manifest's `adoptionDiscipline` block (T1's schema). Deserializes straight
/// from that JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelAdoptionDiscipline {
    /// How long a canary must run clean before its attestation counts.
    pub soak_secs: u64,
    /// How many QUALIFYING attestations earn promotion.
    pub attestation_threshold: u32,
    /// Canary ordering — carried through for T3; not read by this module.
    #[serde(default)]
    pub canary_order: Vec<String>,
}

impl ChannelAdoptionDiscipline {
    /// **C1.** Bind this channel discipline to ONE release by naming that
    /// release's builder agent (the manifest's `provenance.builderAgent`).
    ///
    /// This is the only way to construct an [`AdoptionDiscipline`], and the
    /// reader takes nothing else — so the builder exclusion cannot be forgotten
    /// at a call site. A builder's own attestation never suffices to earn its
    /// release; making that a type obligation rather than a remembered step is
    /// the whole point.
    pub fn for_release(&self, builder_agent: impl Into<String>) -> AdoptionDiscipline {
        AdoptionDiscipline {
            soak_secs: self.soak_secs,
            attestation_threshold: self.attestation_threshold,
            canary_order: self.canary_order.clone(),
            excluded_agents: vec![builder_agent.into()],
        }
    }
}

/// A channel discipline bound to one release. Construct via
/// [`ChannelAdoptionDiscipline::for_release`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptionDiscipline {
    pub soak_secs: u64,
    pub attestation_threshold: u32,
    pub canary_order: Vec<String>,
    /// Agents whose attestations never qualify. Always contains the release's
    /// builder (C1); a channel ceremony may add more later.
    excluded_agents: Vec<String>,
}

impl AdoptionDiscipline {
    pub fn excluded_agents(&self) -> &[String] {
        &self.excluded_agents
    }
    fn excludes(&self, agent: &str) -> bool {
        self.excluded_agents.iter().any(|a| a == agent)
    }
    /// Add a further exclusion (e.g. a contested peer). The builder exclusion
    /// established at construction is never removable.
    pub fn also_excluding(mut self, agent: impl Into<String>) -> Self {
        self.excluded_agents.push(agent.into());
        self
    }
}

/// What the threshold reader found. `qualifying` is the number the controller
/// gates on; every other field exists so an under-count is never mistaken for a
/// real evidence deficit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualifyingEvidence {
    /// Distinct non-excluded agents with a PASSING soak attestation naming this
    /// release. This is the promotion number.
    pub qualifying: u32,
    /// Every release attestation seen for this release, qualifying or not.
    pub total: u32,
    /// Diversity axis — `qualifying` broken down by attester archetype. Carried
    /// now so a diversity threshold (spec §11.2) is purely additive.
    pub by_archetype: BTreeMap<String, u32>,
    /// Excluded by C1 (or a further channel exclusion).
    pub builder_excluded: u32,
    /// Attestations that failed their soak. Not a deficit — evidence.
    pub failed: u32,
    /// Link-walk issuer disagreed with the entry's `author_id`: a re-authored /
    /// laundered copy. Never counted; surfaced so the deficit is legible.
    pub provenance_mismatched: u32,
    /// The link walk named an attestation this conductor could not resolve to a
    /// release-bearing entry (id collision, or not yet gossiped in).
    pub unresolved: u32,
    /// The threshold this evidence was measured against.
    pub threshold: u32,
}

impl QualifyingEvidence {
    /// Whether promotion is earned. Deliberately a method and not a stored
    /// field: the comparison is the controller's decision, not the reader's.
    pub fn threshold_met(&self) -> bool {
        self.qualifying >= self.threshold
    }
    /// True when the count is provably incomplete — the honest signal that a
    /// "not enough evidence" verdict may be a substrate problem, not a real one.
    pub fn is_degraded(&self) -> bool {
        self.provenance_mismatched > 0 || self.unresolved > 0
    }
}

// ---------------------------------------------------------------------------
// Wire mirrors of the coordinator's shapes
// ---------------------------------------------------------------------------

/// Local mirror of `content_store::attestation::IssueAttestationInput`.
///
/// A mirror rather than a dependency on the DNA crate: elohim-storage does not
/// link the zome crates (the convention `conductor_writes::CollectiveWire`
/// establishes). Field names and order are the coordinator's — snake_case, and
/// every field is required by the zome's `Deserialize`.
#[derive(Debug, Clone, Serialize)]
struct IssueAttestationWire {
    attestation_kind: String,
    subject_cid: String,
    subject_kind: String,
    title: String,
    description: Option<String>,
    reach: String,
    metadata: serde_json::Value,
    parent_governance_action_cid: Option<String>,
    vote_value: Option<String>,
    proof_class: String,
    proof_evidence: serde_json::Value,
    expires_at: Option<String>,
}

/// Local mirror of `content_store::attestation::AttestationOutput`.
#[derive(Debug, Clone, Deserialize)]
struct AttestationOutputWire {
    cid: String,
    attestation_kind: String,
    subject_cid: String,
    issuer_cid: String,
}

impl From<AttestationOutputWire> for AttestationRef {
    fn from(w: AttestationOutputWire) -> Self {
        Self {
            cid: w.cid,
            attestation_kind: w.attestation_kind,
            subject_cid: w.subject_cid,
            issuer_cid: w.issuer_cid,
        }
    }
}

/// The coordinator's deterministic content id for an attestation:
/// `format!("attest-{kind}-{issuer}")` (`content_store/src/attestation.rs`).
///
/// Reproduced here because it is the ONLY handle by which a context-bearing
/// attestation entry can be read back through an existing extern — the link
/// walk yields an `EntryHash` and no extern resolves a `Content` from one. That
/// this id is not unique per attestation is the identity-collapse defect the
/// module docs name; the reader is fail-closed against it.
fn attestation_content_id(kind: &str, issuer_cid: &str) -> String {
    format!("attest-{kind}-{issuer_cid}")
}

// ---------------------------------------------------------------------------
// Authoring
// ---------------------------------------------------------------------------

/// Build the `metadata` block — kept conformant to the ridden kind's declared
/// metadata schema (`device-health-metadata.schema.json`), which is
/// `additionalProperties: false`. Pure; the authoring test asserts the shape.
fn soak_metadata(soak: &SoakContext, outcome: &SoakOutcome) -> serde_json::Value {
    serde_json::json!({
        "device_id": soak.device_id,
        "health_metric": SOAK_HEALTH_METRIC,
        "period_start": soak.window_start,
        "period_end": soak.window_end,
        "sample_count": outcome.probe_results.len().max(1),
        "summary_value": format!(
            "{SOAK_DISCRIMINATOR} {} {}/{}",
            outcome.verdict.as_str(),
            outcome.green_count(),
            outcome.probe_results.len()
        ),
    })
}

/// Build the `proof_evidence` block — the discriminator plus the context the
/// threshold reader keys on. `class: "witness"` because the attesting peer is a
/// witness to its own soak window; witness is the one floor-8 class that
/// requires no additional material, so the block cannot be refused for missing
/// proof material it does not have.
fn soak_proof_evidence(
    release_cid: &str,
    soak: &SoakContext,
    outcome: &SoakOutcome,
) -> serde_json::Value {
    serde_json::json!({
        "class": "witness",
        "kind": SOAK_DISCRIMINATOR,
        "releaseCid": release_cid,
        "channelId": soak.channel_id,
        "deviceArchetype": soak.device_archetype,
        "capabilityLevel": soak.capability_level,
        "region": soak.region,
        "householdId": soak.household_id,
        "nodeRole": soak.node_role,
        "outcome": outcome.verdict.as_str(),
        "note": outcome.note,
        "probeResults": outcome.probe_results,
        "buildInfo": soak.build_info,
        "soakWindow": { "start": soak.window_start, "end": soak.window_end },
    })
}

/// Integrity floor 8, pre-checked locally so a malformed block is a typed
/// refusal here rather than an `InvalidCommit` from the conductor.
fn check_floor8(proof_evidence: &serde_json::Value) -> Result<(), TypedRefusal> {
    let class = proof_evidence.get("class").and_then(|v| v.as_str());
    let has = |k: &str| proof_evidence.get(k).and_then(|v| v.as_str()).is_some();
    match class {
        Some("witness") => Ok(()),
        Some("audit") if has("merkle_root") => Ok(()),
        Some("audit") => Err(TypedRefusal::ProofEvidenceIncomplete {
            class: "audit",
            material: "merkle_root",
        }),
        Some("proof") if has("proof_blob") => Ok(()),
        Some("proof") => Err(TypedRefusal::ProofEvidenceIncomplete {
            class: "proof",
            material: "proof_blob",
        }),
        Some("confirmation") if has("confirmer_signature") => Ok(()),
        Some("confirmation") => Err(TypedRefusal::ProofEvidenceIncomplete {
            class: "confirmation",
            material: "confirmer_signature",
        }),
        _ => Err(TypedRefusal::ProofEvidenceIncomplete {
            class: "unknown",
            material: "class",
        }),
    }
}

/// Pure builder for the coordinator input. Separated from the zome call so the
/// entire wire shape is unit-testable without a conductor.
fn build_soak_input(
    release_cid: &str,
    soak: &SoakContext,
    outcome: &SoakOutcome,
) -> Result<IssueAttestationWire, TypedRefusal> {
    if release_cid.trim().is_empty() {
        return Err(TypedRefusal::ReleaseCidEmpty);
    }
    soak.validate()?;
    let proof_evidence = soak_proof_evidence(release_cid, soak, outcome);
    check_floor8(&proof_evidence)?;
    Ok(IssueAttestationWire {
        attestation_kind: RIDDEN_ATTESTATION_KIND.to_string(),
        subject_cid: release_cid.to_string(),
        // The subject is the release manifest — a `Content` entry (spec §5).
        subject_kind: "content".to_string(),
        title: format!("Release soak: {release_cid} on {}", soak.device_archetype),
        description: Some(format!(
            "soak {} — {}/{} probes green over {}..{}",
            outcome.verdict.as_str(),
            outcome.green_count(),
            outcome.probe_results.len(),
            soak.window_start,
            soak.window_end
        )),
        reach: "community".to_string(),
        metadata: soak_metadata(soak, outcome),
        parent_governance_action_cid: None,
        vote_value: None,
        proof_class: "witness".to_string(),
        proof_evidence,
        expires_at: None,
    })
}

/// Author this peer's soak attestation for `release_cid` through its own
/// conductor.
///
/// Composes the existing consolidated-attestation rail
/// (`content_store::issue_attestation`) — the same coordinator the
/// infrastructure DNA's device-health bridge calls. No parallel rail, no new
/// entry type, no DNA-hash move.
///
/// The T4 (post-apply) call site: apply a release, run the soak window, then
/// author whatever the probes actually observed. A FAILING soak is authored too
/// — that evidence is what feeds contest/revert.
pub async fn author_soak_attestation(
    ctx: &ReleaseAttestationCtx,
    release_cid: &str,
    soak: SoakContext,
    outcome: SoakOutcome,
) -> Result<AttestationRef, TypedRefusal> {
    let input = build_soak_input(release_cid, &soak, &outcome)?;
    let payload = rmp_serde::to_vec_named(&input).map_err(|e| {
        TypedRefusal::WireDecodeFailed(format!("encode IssueAttestationInput: {e}"))
    })?;
    let bytes = ctx
        .hc
        .call_zome(ZOME_NAME, "issue_attestation", payload)
        .await
        .map_err(|e| TypedRefusal::ConductorRefused(e.to_string()))?;
    let out: AttestationOutputWire = rmp_serde::from_slice(&bytes)
        .map_err(|e| TypedRefusal::WireDecodeFailed(format!("decode AttestationOutput: {e}")))?;
    tracing::info!(
        release_cid = %release_cid,
        cid = %out.cid,
        archetype = %soak.device_archetype,
        outcome = %outcome.verdict.as_str(),
        "release soak attestation authored"
    );
    Ok(out.into())
}

// ---------------------------------------------------------------------------
// Reading — the threshold arm
// ---------------------------------------------------------------------------

/// One attestation as the link walk reports it: an AUTHENTICATED `(cid, issuer)`
/// pair. The `AttestationToSubject` link is only ever created inside
/// `issue_attestation`, so a re-authored copy cannot appear here.
#[derive(Debug, Clone, Deserialize)]
struct SubjectAttestationWire {
    #[allow(dead_code)]
    cid: String,
    #[allow(dead_code)]
    attestation_kind: String,
    #[allow(dead_code)]
    subject_cid: String,
    issuer_cid: String,
}

/// How one candidate attestation was classified. Every non-`Qualifies` variant
/// is a reason, so [`QualifyingEvidence`] can report the shape of a shortfall.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Verdict {
    /// Counts. Carries the attester and the diversity axis.
    Qualifies { agent: String, archetype: String },
    /// Excluded by C1 (or a further channel exclusion).
    Excluded,
    /// A real, well-formed attestation whose soak did not pass.
    Failed,
    /// The entry's `author_id` disagreed with the link walk's issuer — a
    /// re-authored / laundered copy.
    ProvenanceMismatch,
    /// Could not be resolved to a release-bearing entry on this conductor.
    Unresolved,
    /// Resolved, but it is not release evidence at all (a genuine
    /// device-health row, or a `release-build` attestation, which never counts
    /// toward a soak threshold).
    NotReleaseSoakEvidence,
}

/// Classify one resolved candidate. PURE — the entire decision surface of the
/// threshold reader lives here, so it is unit-testable without a conductor.
///
/// `entry` is the `Content` the conductor resolved for the issuer's attestation
/// id; `link_issuer` is the authenticated issuer the link walk reported.
fn classify(
    link_issuer: &str,
    entry: Option<&ResolvedEntry>,
    release_cid: &str,
    discipline: &AdoptionDiscipline,
) -> Verdict {
    let Some(entry) = entry else {
        return Verdict::Unresolved;
    };

    // Provenance FIRST: a laundered copy must never be read for its content.
    if entry.author_id.as_deref() != Some(link_issuer) {
        return Verdict::ProvenanceMismatch;
    }

    // The entry the (non-unique) id resolved to must actually be about THIS
    // release — on both the entry's own subject and the attested payload.
    if entry.subject_cid != release_cid || entry.release_cid.as_deref() != Some(release_cid) {
        return Verdict::Unresolved;
    }

    if entry.discriminator.as_deref() != Some(SOAK_DISCRIMINATOR) {
        return Verdict::NotReleaseSoakEvidence;
    }

    if entry.revoked {
        return Verdict::NotReleaseSoakEvidence;
    }

    if discipline.excludes(link_issuer) {
        return Verdict::Excluded;
    }

    if entry.outcome.as_deref() != Some(SoakVerdict::Pass.as_str()) {
        return Verdict::Failed;
    }

    Verdict::Qualifies {
        agent: link_issuer.to_string(),
        archetype: entry
            .device_archetype
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
    }
}

/// The fields the reader needs out of a resolved `Content` entry, decoded once.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ResolvedEntry {
    author_id: Option<String>,
    subject_cid: String,
    release_cid: Option<String>,
    discriminator: Option<String>,
    outcome: Option<String>,
    device_archetype: Option<String>,
    revoked: bool,
}

impl ResolvedEntry {
    /// Decode from the entry's `metadata_json` + `author_id`. PURE.
    fn from_metadata(author_id: Option<String>, metadata_json: &str) -> Self {
        let meta: serde_json::Value = serde_json::from_str(metadata_json).unwrap_or_default();
        let pe = meta.get("proof_evidence");
        let s = |v: Option<&serde_json::Value>, k: &str| {
            v.and_then(|o| o.get(k))
                .and_then(|x| x.as_str())
                .map(str::to_string)
        };
        Self {
            author_id,
            subject_cid: meta
                .get("subject_cid")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            release_cid: s(pe, "releaseCid"),
            discriminator: s(pe, "kind"),
            outcome: s(pe, "outcome"),
            device_archetype: s(pe, "deviceArchetype"),
            revoked: meta
                .get("revocation")
                .map(|r| !r.is_null())
                .unwrap_or(false),
        }
    }
}

/// Fold classified verdicts into the evidence report. PURE. One qualifying
/// voice per agent: a peer that attests twice does not become two peers.
fn tally(verdicts: Vec<Verdict>, threshold: u32) -> QualifyingEvidence {
    let mut ev = QualifyingEvidence {
        threshold,
        ..Default::default()
    };
    let mut counted: Vec<String> = Vec::new();
    for v in verdicts {
        ev.total += 1;
        match v {
            Verdict::Qualifies { agent, archetype } => {
                if counted.contains(&agent) {
                    continue;
                }
                counted.push(agent);
                ev.qualifying += 1;
                *ev.by_archetype.entry(archetype).or_insert(0) += 1;
            }
            Verdict::Excluded => ev.builder_excluded += 1,
            Verdict::Failed => ev.failed += 1,
            Verdict::ProvenanceMismatch => ev.provenance_mismatched += 1,
            Verdict::Unresolved => ev.unresolved += 1,
            Verdict::NotReleaseSoakEvidence => {
                // Not evidence about this release at all — do not even count it
                // in `total`, which is scoped to release attestations.
                ev.total -= 1;
            }
        }
    }
    ev
}

/// Count the attestations that qualify `release_cid` for promotion under
/// `discipline`.
///
/// Reads through THIS peer's own conductor (I1: never adopt from a payload) in
/// two legs — the authenticated link walk, then a context-bearing entry read
/// per attester — and cross-checks them. See the module docs for why the local
/// SQL projection is not the source, and what the cross-check protects against.
///
/// Bounded: `1 + N` zome calls per read, `N` = attesters for this release.
/// Admitted as [`AdmissionClass::Background`] — a controller sweep must never
/// occupy the lane a person is standing in.
///
/// The T3 (verify) call site: gate promotion on
/// [`QualifyingEvidence::threshold_met`], and treat
/// [`QualifyingEvidence::is_degraded`] as "this count is provably incomplete" —
/// a reason to retry the sweep, never to promote.
pub async fn count_qualifying_attestations(
    ctx: &ReleaseAttestationCtx,
    release_cid: &str,
    discipline: &AdoptionDiscipline,
) -> Result<QualifyingEvidence, TypedRefusal> {
    if release_cid.trim().is_empty() {
        return Err(TypedRefusal::ReleaseCidEmpty);
    }

    // Leg 1 — the authenticated (cid, issuer) set from the link walk.
    let payload = rmp_serde::to_vec_named(&release_cid.to_string())
        .map_err(|e| TypedRefusal::WireDecodeFailed(format!("encode subject_cid: {e}")))?;
    let bytes = ctx
        .hc
        .call_zome_timed(
            ZOME_NAME,
            "get_attestations_for_subject",
            payload,
            AdmissionClass::Background,
        )
        .await
        .map(|(b, _timing)| b)
        .map_err(|e| TypedRefusal::ConductorUnavailable(e.to_string()))?;
    let linked: Vec<SubjectAttestationWire> = rmp_serde::from_slice(&bytes).map_err(|e| {
        TypedRefusal::WireDecodeFailed(format!("decode Vec<AttestationOutput>: {e}"))
    })?;

    // Leg 2 — resolve each attester's entry for its context, once per issuer.
    let mut issuers: Vec<String> = Vec::new();
    for l in &linked {
        if !issuers.contains(&l.issuer_cid) {
            issuers.push(l.issuer_cid.clone());
        }
    }

    // Size the work BEFORE any resolve: a conductor call cannot be cancelled,
    // so the bound has to be on how many we start, never on abandoning one.
    let (resolvable, deferred) = if issuers.len() > MAX_RESOLVES_PER_SWEEP {
        let (head, tail) = issuers.split_at(MAX_RESOLVES_PER_SWEEP);
        tracing::warn!(
            release_cid = %release_cid,
            attesters = issuers.len(),
            ceiling = MAX_RESOLVES_PER_SWEEP,
            "release attestation attesters exceed the per-sweep resolve ceiling — \
             remainder reported as unresolved and picked up next sweep"
        );
        (head.to_vec(), tail.len())
    } else {
        (issuers.clone(), 0)
    };

    let mut verdicts = Vec::with_capacity(issuers.len());
    for _ in 0..deferred {
        verdicts.push(Verdict::Unresolved);
    }
    for issuer in &resolvable {
        let id = attestation_content_id(RIDDEN_ATTESTATION_KIND, issuer);
        let resolved =
            match crate::services::conductor_writes::get_content_by_id(&ctx.hc, &id).await {
                Ok(Some(out)) => Some(ResolvedEntry::from_metadata(
                    out.content.author_id.clone(),
                    &out.content.metadata_json,
                )),
                Ok(None) => None,
                Err(e) => {
                    // A single unreadable entry deflates the count; it must never
                    // fail the whole sweep, or one cold entry blocks every release.
                    tracing::warn!(
                        release_cid = %release_cid,
                        issuer = %issuer,
                        error = %e,
                        "release attestation entry unreadable — counted as unresolved"
                    );
                    None
                }
            };
        verdicts.push(classify(issuer, resolved.as_ref(), release_cid, discipline));
    }

    let evidence = tally(verdicts, discipline.attestation_threshold);
    tracing::info!(
        release_cid = %release_cid,
        qualifying = evidence.qualifying,
        total = evidence.total,
        excluded = evidence.builder_excluded,
        unresolved = evidence.unresolved,
        provenance_mismatched = evidence.provenance_mismatched,
        threshold = evidence.threshold,
        met = evidence.threshold_met(),
        "release attestation threshold measured"
    );
    Ok(evidence)
}

// ---------------------------------------------------------------------------
// Tests — every pure decision surface, no conductor required
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_for(archetype: &str) -> SoakContext {
        SoakContext {
            channel_id: "runtime:coordinator-bundle:household:test".to_string(),
            device_id: "uhCAk-device".to_string(),
            device_archetype: archetype.to_string(),
            capability_level: 3,
            region: Some("household".to_string()),
            household_id: Some("household-test".to_string()),
            node_role: Some("steward".to_string()),
            build_info: BuildInfoRef {
                version: "0.1.0".to_string(),
                commit: "abc1234".to_string(),
                service: "elohim-storage".to_string(),
            },
            window_start: "2026-09-01T10:00:00Z".to_string(),
            window_end: "2026-09-01T10:30:00Z".to_string(),
        }
    }

    fn discipline(threshold: u32, builder: &str) -> AdoptionDiscipline {
        ChannelAdoptionDiscipline {
            soak_secs: 1800,
            attestation_threshold: threshold,
            canary_order: vec![],
        }
        .for_release(builder)
    }

    /// An entry as it would decode from a genuine attestation by `issuer`.
    fn entry(issuer: &str, release: &str, outcome: &str, archetype: &str) -> ResolvedEntry {
        let meta = serde_json::json!({
            "attestation_kind": RIDDEN_ATTESTATION_KIND,
            "subject_cid": release,
            "subject_kind": "content",
            "proof_evidence": {
                "class": "witness",
                "kind": SOAK_DISCRIMINATOR,
                "releaseCid": release,
                "deviceArchetype": archetype,
                "outcome": outcome,
            },
        });
        ResolvedEntry::from_metadata(Some(issuer.to_string()), &meta.to_string())
    }

    // ---- authoring shape -------------------------------------------------

    #[test]
    fn ridden_kind_is_a_generated_kind_not_a_new_one() {
        // The constraint the whole module is shaped by: authoring must name a
        // kind the integrity zome already compiles in. This asserts we never
        // drift back to a bespoke `attestation:release-*`.
        assert!(!RIDDEN_ATTESTATION_KIND.contains("release"));
        assert_eq!(RIDDEN_ATTESTATION_KIND, "attestation:device-health");
    }

    #[test]
    fn soak_input_rides_the_generated_kind_and_anchors_on_the_release() {
        let input = build_soak_input(
            "release-abc",
            &ctx_for("home-server"),
            &SoakOutcome::pass(vec![]),
        )
        .expect("input");
        assert_eq!(input.attestation_kind, RIDDEN_ATTESTATION_KIND);
        assert_eq!(input.subject_cid, "release-abc");
        assert_eq!(input.subject_kind, "content");
    }

    #[test]
    fn metadata_stays_conformant_to_the_ridden_kinds_schema() {
        // device-health-metadata.schema.json is additionalProperties:false with
        // device_id / health_metric / period_start / period_end required.
        let input = build_soak_input(
            "release-abc",
            &ctx_for("workstation"),
            &SoakOutcome::pass(vec![
                ProbeResult::green("health"),
                ProbeResult::green("p2p"),
            ]),
        )
        .expect("input");
        let m = input.metadata.as_object().expect("object");
        let allowed = [
            "device_id",
            "health_metric",
            "period_start",
            "period_end",
            "sample_count",
            "summary_value",
        ];
        for key in m.keys() {
            assert!(allowed.contains(&key.as_str()), "extra metadata key {key}");
        }
        for key in ["device_id", "health_metric", "period_start", "period_end"] {
            assert!(m.contains_key(key), "missing required {key}");
        }
        assert_eq!(m["health_metric"], serde_json::json!(SOAK_HEALTH_METRIC));
        assert_eq!(m["sample_count"], serde_json::json!(2));
    }

    #[test]
    fn discriminator_and_context_ride_in_proof_evidence() {
        let input = build_soak_input(
            "release-abc",
            &ctx_for("workstation"),
            &SoakOutcome::pass(vec![ProbeResult::green("health")]),
        )
        .expect("input");
        let pe = &input.proof_evidence;
        assert_eq!(pe["kind"], serde_json::json!(SOAK_DISCRIMINATOR));
        assert_eq!(pe["releaseCid"], serde_json::json!("release-abc"));
        assert_eq!(pe["deviceArchetype"], serde_json::json!("workstation"));
        assert_eq!(pe["outcome"], serde_json::json!("pass"));
        assert_eq!(pe["class"], serde_json::json!("witness"));
        assert!(pe["buildInfo"]["commit"].is_string());
        assert!(pe["probeResults"].is_array());
    }

    #[test]
    fn floor8_accepts_witness_and_refuses_incomplete_material() {
        assert!(check_floor8(&serde_json::json!({"class": "witness"})).is_ok());
        assert_eq!(
            check_floor8(&serde_json::json!({"class": "audit"})),
            Err(TypedRefusal::ProofEvidenceIncomplete {
                class: "audit",
                material: "merkle_root"
            })
        );
        assert!(check_floor8(&serde_json::json!({"class": "audit", "merkle_root": "ab"})).is_ok());
        assert!(check_floor8(&serde_json::json!({"class": "nope"})).is_err());
        assert!(check_floor8(&serde_json::json!({})).is_err());
    }

    #[test]
    fn authoring_refuses_an_empty_release_or_incomplete_context() {
        assert_eq!(
            build_soak_input("  ", &ctx_for("x"), &SoakOutcome::pass(vec![])).unwrap_err(),
            TypedRefusal::ReleaseCidEmpty
        );
        let mut bad = ctx_for("x");
        bad.channel_id = String::new();
        assert_eq!(
            build_soak_input("r", &bad, &SoakOutcome::pass(vec![])).unwrap_err(),
            TypedRefusal::SoakContextIncomplete { field: "channelId" }
        );
    }

    #[test]
    fn a_failing_soak_is_still_authored_and_says_so() {
        let input = build_soak_input(
            "release-abc",
            &ctx_for("home-server"),
            &SoakOutcome::fail(
                vec![ProbeResult::red("p2p", "no peers")],
                "arc never filled",
            ),
        )
        .expect("input");
        assert_eq!(input.proof_evidence["outcome"], serde_json::json!("fail"));
        assert_eq!(
            input.proof_evidence["note"],
            serde_json::json!("arc never filled")
        );
    }

    #[test]
    fn attestation_content_id_matches_the_coordinators_format() {
        assert_eq!(
            attestation_content_id("attestation:device-health", "uhCAkAgent"),
            "attest-attestation:device-health-uhCAkAgent"
        );
    }

    // ---- C1: the builder exclusion cannot be forgotten --------------------

    #[test]
    fn discipline_cannot_be_built_without_naming_the_builder() {
        let d = discipline(2, "uhCAkBuilder");
        assert_eq!(d.excluded_agents(), ["uhCAkBuilder".to_string()]);
        assert!(d.excludes("uhCAkBuilder"));
        assert!(!d.excludes("uhCAkPeer"));
    }

    #[test]
    fn channel_discipline_deserialises_from_the_manifest_block() {
        let d: ChannelAdoptionDiscipline = serde_json::from_str(
            r#"{"soakSecs":1800,"attestationThreshold":2,"canaryOrder":["a","b"]}"#,
        )
        .expect("discipline");
        assert_eq!(d.soak_secs, 1800);
        assert_eq!(d.attestation_threshold, 2);
        assert_eq!(d.canary_order, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn the_builders_own_attestation_never_qualifies() {
        let d = discipline(2, "uhCAkBuilder");
        let e = entry("uhCAkBuilder", "release-abc", "pass", "builder-node");
        assert_eq!(
            classify("uhCAkBuilder", Some(&e), "release-abc", &d),
            Verdict::Excluded
        );
    }

    // ---- classification --------------------------------------------------

    #[test]
    fn a_passing_peer_soak_qualifies_with_its_archetype() {
        let d = discipline(2, "uhCAkBuilder");
        let e = entry("uhCAkPeer", "release-abc", "pass", "home-server");
        assert_eq!(
            classify("uhCAkPeer", Some(&e), "release-abc", &d),
            Verdict::Qualifies {
                agent: "uhCAkPeer".to_string(),
                archetype: "home-server".to_string()
            }
        );
    }

    #[test]
    fn a_laundered_copy_is_refused_before_its_content_is_read() {
        // The measured defect: a receiving peer re-authors the attestation
        // under its own key. The link walk still names the REAL issuer, so the
        // disagreement is detectable — and must never be counted.
        let d = discipline(2, "uhCAkBuilder");
        let laundered = entry("uhCAkLocalPeer", "release-abc", "pass", "workstation");
        assert_eq!(
            classify("uhCAkRealIssuer", Some(&laundered), "release-abc", &d),
            Verdict::ProvenanceMismatch
        );
    }

    #[test]
    fn an_id_collision_resolving_to_another_release_is_unresolved_not_counted() {
        // `attest-{kind}-{issuer}` is not unique per attestation, so the id can
        // resolve to the issuer's soak of a DIFFERENT release. Fail closed.
        let d = discipline(2, "uhCAkBuilder");
        let other = entry("uhCAkPeer", "release-OTHER", "pass", "home-server");
        assert_eq!(
            classify("uhCAkPeer", Some(&other), "release-abc", &d),
            Verdict::Unresolved
        );
    }

    #[test]
    fn a_missing_entry_is_unresolved() {
        let d = discipline(2, "uhCAkBuilder");
        assert_eq!(
            classify("uhCAkPeer", None, "release-abc", &d),
            Verdict::Unresolved
        );
    }

    #[test]
    fn a_failing_soak_is_evidence_but_never_qualifies() {
        let d = discipline(2, "uhCAkBuilder");
        let e = entry("uhCAkPeer", "release-abc", "fail", "home-server");
        assert_eq!(
            classify("uhCAkPeer", Some(&e), "release-abc", &d),
            Verdict::Failed
        );
    }

    #[test]
    fn a_genuine_device_health_row_is_not_release_evidence() {
        let d = discipline(2, "uhCAkBuilder");
        let meta = serde_json::json!({
            "subject_cid": "release-abc",
            "proof_evidence": { "class": "witness" },
        });
        let e = ResolvedEntry::from_metadata(Some("uhCAkPeer".into()), &meta.to_string());
        // No releaseCid at all → not about this release.
        assert_eq!(
            classify("uhCAkPeer", Some(&e), "release-abc", &d),
            Verdict::Unresolved
        );
    }

    #[test]
    fn a_build_attestation_does_not_count_toward_a_soak_threshold() {
        let d = discipline(2, "uhCAkBuilder");
        let meta = serde_json::json!({
            "subject_cid": "release-abc",
            "proof_evidence": {
                "class": "witness",
                "kind": BUILD_DISCRIMINATOR,
                "releaseCid": "release-abc",
                "outcome": "pass",
            },
        });
        let e = ResolvedEntry::from_metadata(Some("uhCAkPeer".into()), &meta.to_string());
        assert_eq!(
            classify("uhCAkPeer", Some(&e), "release-abc", &d),
            Verdict::NotReleaseSoakEvidence
        );
    }

    #[test]
    fn a_revoked_attestation_stops_counting() {
        let d = discipline(2, "uhCAkBuilder");
        let meta = serde_json::json!({
            "subject_cid": "release-abc",
            "revocation": { "reason": "probe was wrong", "supersedes_cid": "uhCEkx" },
            "proof_evidence": {
                "class": "witness",
                "kind": SOAK_DISCRIMINATOR,
                "releaseCid": "release-abc",
                "outcome": "pass",
            },
        });
        let e = ResolvedEntry::from_metadata(Some("uhCAkPeer".into()), &meta.to_string());
        assert_eq!(
            classify("uhCAkPeer", Some(&e), "release-abc", &d),
            Verdict::NotReleaseSoakEvidence
        );
    }

    // ---- tally -----------------------------------------------------------

    #[test]
    fn two_peers_meet_a_threshold_of_two_and_the_builder_is_excluded() {
        let ev = tally(
            vec![
                Verdict::Qualifies {
                    agent: "a".into(),
                    archetype: "workstation".into(),
                },
                Verdict::Qualifies {
                    agent: "b".into(),
                    archetype: "home-server".into(),
                },
                Verdict::Excluded,
            ],
            2,
        );
        assert_eq!(ev.qualifying, 2);
        assert_eq!(ev.total, 3);
        assert_eq!(ev.builder_excluded, 1);
        assert_eq!(ev.by_archetype["workstation"], 1);
        assert_eq!(ev.by_archetype["home-server"], 1);
        assert!(ev.threshold_met());
        assert!(!ev.is_degraded());
    }

    #[test]
    fn one_peer_attesting_twice_is_still_one_voice() {
        let ev = tally(
            vec![
                Verdict::Qualifies {
                    agent: "a".into(),
                    archetype: "workstation".into(),
                },
                Verdict::Qualifies {
                    agent: "a".into(),
                    archetype: "workstation".into(),
                },
            ],
            2,
        );
        assert_eq!(ev.qualifying, 1);
        assert_eq!(ev.by_archetype["workstation"], 1);
        assert!(!ev.threshold_met());
    }

    #[test]
    fn a_degraded_count_is_flagged_so_it_is_never_read_as_a_real_deficit() {
        let ev = tally(vec![Verdict::ProvenanceMismatch, Verdict::Unresolved], 2);
        assert_eq!(ev.qualifying, 0);
        assert_eq!(ev.provenance_mismatched, 1);
        assert_eq!(ev.unresolved, 1);
        assert!(!ev.threshold_met());
        assert!(ev.is_degraded());
    }

    #[test]
    fn non_release_rows_do_not_inflate_the_total() {
        let ev = tally(
            vec![
                Verdict::Qualifies {
                    agent: "a".into(),
                    archetype: "workstation".into(),
                },
                Verdict::NotReleaseSoakEvidence,
            ],
            1,
        );
        assert_eq!(ev.total, 1);
        assert_eq!(ev.qualifying, 1);
        assert!(ev.threshold_met());
    }
}
