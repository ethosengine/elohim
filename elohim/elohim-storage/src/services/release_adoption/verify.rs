//! The verify floor (spec §6.3, §8) — **floor-protected, never stage-priced**.
//!
//! Everything in this file is a pure function. No I/O, no conductor, no clock:
//! a caller assembles the evidence (the manifest bytes, this peer's installed
//! reality, the channel's L2 lineage, the fetched artifact digests, the
//! attestation count) and this module decides. That split is deliberate —
//! it is what lets the whole floor be contract-tested without a mesh, and it is
//! what makes "verified" a property of the evidence rather than of the call
//! order in `watch.rs`.
//!
//! # The five checks, in the order a refusal is cheapest
//!
//! 1. **Shape** — the manifest against T1's schema. Serde alone is not enough:
//!    the schema pins `pattern`s (a `blobCid` must be a raw-codec CIDv1, a
//!    `dnaHash` must be `uhC0k…`) that a `String` field happily accepts. A
//!    `sha256-<hex>` in a `cid` field is the exact legacy-address confusion the
//!    addressing canon exists to refuse, so it is refused here.
//! 2. **Channel identity** — the manifest's own `channelId` must be the channel
//!    we resolved it from. A release found on the wrong channel is laundered,
//!    whatever else is true about it.
//! 3. **Envelope** — against the runtime passport's INSTALLED reality. The
//!    per-role DNA-hash guard is `happ_manager::lineage_mismatch_error`'s
//!    refusal moved to verify time: `update_coordinators` matches integrity
//!    dependencies BY NAME, so a cross-lineage bundle would splice coordinators
//!    onto integrity zomes they were never compiled against. Crossing the DNA
//!    line is rung 6's ceremony; here it is structurally refused.
//! 4. **Lineage** — the manifest's `envelope.lineageParentCid` against the
//!    channel's L2 version chain (the resolved head's `supersedes`). The body
//!    field is a HINT that must match; a mismatch is a typed refusal, never an
//!    accepted envelope.
//! 5. **Threshold** — per the manifest's own `adoptionDiscipline`, read through
//!    T5's `count_qualifying_attestations`. When it cannot be read the verdict
//!    is `threshold_unchecked`, which is **not a pass**.
//! 6. **Artifacts, then already-current, then coordinator lineage** — the
//!    envelope's coordinator-wasm (SUPERSEDES) leg is the one check that cannot
//!    be settled before the bytes are on disk, because the question it should
//!    ask first is *does this peer already run what these bytes install?*
//!    ([`already_runs_target`]). A channel re-authored over the SAME bytes — a
//!    threshold-0 revert — produces a new release CID for coordinator wasm the
//!    peer is already running, and the supersedes check alone can only see that
//!    the peer no longer runs the OLD bytes. That is convergence being reported
//!    as `coordinator_lineage_mismatch`, forever, which is what this ordering
//!    fixes (measured live on the household mesh, 2026-09-04).
//!
//! # Why the artifact check is byte-exact and not "close enough"
//!
//! The schema says a fetch that yields a different length is a typed refusal,
//! never a retry. Both length and digest are checked, and separately: a length
//! mismatch is a truncation/wrong-object story, a digest mismatch with the right
//! length is a corruption/substitution story, and folding them loses the only
//! signal that tells those apart.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use seam_contracts::Answer;

use super::{
    AdoptionRefusal, Artifact, ArtifactClass, HeadTier, PathEvidence, RefusalReason,
    ReleaseManifest, VerifiedRelease, RELEASE_MANIFEST_KIND,
};
use crate::services::release_attestation::QualifyingEvidence;

/// Wire-format epochs this build SPEAKS.
///
/// The sync-state contract's epoch-before-position axis (spec §8.1). A release
/// that declares no epoch in this set speaks a protocol this binary does not,
/// and adopting it would be the big-bang roll the whole rung exists to retire.
///
/// NOT the publisher boot epoch from `p2p/sync_state.rs`, which is a per-process
/// runtime value with an unrelated meaning.
pub const SUPPORTED_WIRE_EPOCHS: &[u32] = &[0, 1];

// ---------------------------------------------------------------------------
// Installed reality — the passport, projected into what the envelope needs
// ---------------------------------------------------------------------------

/// One role as this peer actually runs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledRole {
    pub role: String,
    pub dna_hash: String,
    /// zome name → wasm hash, exactly the shape the runtime passport reports.
    pub coordinator_zomes: BTreeMap<String, String>,
    /// **Rung 6.** The constitutional root this role's DNA declares, when the
    /// passport exposes one. `None` when the role declares no root — which is
    /// every role today (`HappRolePassport` carries no such field yet); wired
    /// through as `None` at [`InstalledReality::from_happ_passport`] rather
    /// than invented here. [`verify_path`] only compares roots when this is
    /// `Some` — a role with no declared root imposes no root constraint.
    pub constitution_root: Option<String>,
}

impl InstalledRole {
    /// The role's coordinator wasm hashes, sorted and deduplicated — the shape
    /// the manifest's NORMATIVE `coordinatorWasmHashes` field is in.
    pub fn wasm_hashes(&self) -> BTreeSet<&str> {
        self.coordinator_zomes
            .values()
            .map(String::as_str)
            .collect()
    }
}

/// What this peer has installed, as the envelope check needs to see it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstalledReality {
    pub app_id: String,
    pub roles: BTreeMap<String, InstalledRole>,
}

impl InstalledReality {
    /// Project the runtime passport's hApp leg.
    ///
    /// **C4.** A passport whose `happ.error` is set, or whose role list is
    /// empty, is *unreachable*, not "no roles installed" — so this returns an
    /// [`Answer`], and the caller may not flatten it. Reading a failed conductor
    /// inventory as "this peer runs nothing" would make every envelope check
    /// trivially refuse for a reason that is a fact about us, not the release.
    /// A per-role `error` demotes that ONE role, not the whole answer.
    pub fn from_happ_passport(happ: &crate::runtime_passport::HappPassport) -> Answer<Self> {
        if happ.error.is_some() {
            return Answer::Unreachable;
        }
        let mut roles = BTreeMap::new();
        for role in &happ.roles {
            if role.error.is_some() || role.dna_hash == "unknown" {
                continue;
            }
            roles.insert(
                role.role.clone(),
                InstalledRole {
                    role: role.role.clone(),
                    dna_hash: role.dna_hash.clone(),
                    coordinator_zomes: role.coordinator_wasm_hashes.clone(),
                    // The runtime passport does not expose a per-role
                    // constitution root yet — every role declares none.
                    constitution_root: None,
                },
            );
        }
        if roles.is_empty() {
            // The app is installed but nothing about its roles could be read.
            // Not an installed reality; not evidence of absence either.
            return Answer::Unreachable;
        }
        Answer::Present(Self {
            app_id: happ.app_id.clone(),
            roles,
        })
    }
}

/// The channel's L2 version chain, as far as the resolve could see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineageEvidence {
    /// The prior RELEASE the chain proves — never the manifest's own
    /// self-report. Proven by ORDER when the superseded record is itself a
    /// release (release→release); proven by EXISTENCE on this channel when
    /// it is not (a star chain around the channel root, which is what
    /// `update_content` targets for every non-first version). `None` for a
    /// channel's first release: the head's declaration may still
    /// structurally supersede an EARLIER action (the channel root, or an
    /// ordinary content version), but that action is not itself a release,
    /// so it is never reported here as one.
    pub supersedes: Option<String>,
}

// ---------------------------------------------------------------------------
// Shape — T1's schema, enforced where serde cannot reach
// ---------------------------------------------------------------------------

fn refuse(reason: RefusalReason, detail: impl Into<String>) -> AdoptionRefusal {
    AdoptionRefusal::new(reason, detail)
}

/// `^runtime:[a-z0-9][a-z0-9-]*:[a-z0-9][a-z0-9-]*:[a-z0-9][a-z0-9-]*$`
pub fn is_channel_id(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("runtime:") else {
        return false;
    };
    let parts: Vec<&str> = rest.split(':').collect();
    parts.len() == 3 && parts.iter().all(|p| is_lower_slug(p))
}

fn is_lower_slug(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// `^[a-z][a-z0-9_-]*$` — the schema's role-name shape.
fn is_role_name(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// `^bafkrei[a-z2-7]{52}$` — CIDv1, raw codec, sha2-256, base32-lower.
///
/// The address form the blob plane is canonical on. A bare `sha256-<hex>` is
/// the LEGACY blob-path form and is refused here by construction: sha2-256 is
/// the multihash INSIDE a CID, never a standalone address.
pub fn is_blob_cid(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("bafkrei") else {
        return false;
    };
    rest.len() == 52
        && rest
            .chars()
            .all(|c| c.is_ascii_lowercase() || ('2'..='7').contains(&c))
}

/// `^[0-9a-f]{64}$`
pub fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64
        && s.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// `^uhC0k[A-Za-z0-9_-]{48}$` — a Holochain DnaHash.
pub fn is_dna_hash(s: &str) -> bool {
    is_holo_hash(s, "uhC0k")
}

/// `^uhCok[A-Za-z0-9_-]{48}$` — a Holochain WasmHash.
pub fn is_wasm_hash(s: &str) -> bool {
    is_holo_hash(s, "uhCok")
}

fn is_holo_hash(s: &str, prefix: &str) -> bool {
    let Some(rest) = s.strip_prefix(prefix) else {
        return false;
    };
    rest.len() == 48
        && rest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// `^[0-9a-f]{7,40}$`
fn is_git_commit(s: &str) -> bool {
    (7..=40).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// Decode and shape-check a manifest body against T1's schema.
///
/// The mirror is OPEN (unknown properties pass, per the additive-wire floor);
/// this function enforces the half serde cannot: required fields with pinned
/// patterns, and the cross-field agreement the schema describes in prose.
pub fn verify_shape(body: &serde_json::Value) -> Result<ReleaseManifest, AdoptionRefusal> {
    let manifest: ReleaseManifest = serde_json::from_value(body.clone())
        .map_err(|e| refuse(RefusalReason::ManifestSchemaInvalid, format!("decode: {e}")))?;

    if manifest.kind != RELEASE_MANIFEST_KIND {
        return Err(refuse(
            RefusalReason::ManifestSchemaInvalid,
            format!(
                "kind must be '{RELEASE_MANIFEST_KIND}', got '{}'",
                manifest.kind
            ),
        ));
    }
    if let Some(v) = manifest.manifest_version.as_deref() {
        if v != "1.0" {
            return Err(refuse(
                RefusalReason::ManifestSchemaInvalid,
                format!("manifestVersion must be '1.0' (absent means 1.0), got '{v}'"),
            ));
        }
    }
    if !is_channel_id(&manifest.channel_id) {
        return Err(refuse(
            RefusalReason::ManifestSchemaInvalid,
            format!(
                "channelId '{}' is not `runtime:<class>:<network>:<name>`",
                manifest.channel_id
            ),
        ));
    }
    if manifest.artifacts.is_empty() {
        return Err(refuse(
            RefusalReason::ManifestSchemaInvalid,
            "artifacts must carry at least one entry",
        ));
    }
    for artifact in &manifest.artifacts {
        verify_artifact_shape(artifact)?;
    }
    if manifest.applies_to.roles.is_empty() {
        return Err(refuse(
            RefusalReason::ManifestSchemaInvalid,
            "appliesTo.roles must name at least one role",
        ));
    }
    for (role, binding) in &manifest.applies_to.roles {
        if !is_role_name(role) {
            return Err(refuse(
                RefusalReason::ManifestSchemaInvalid,
                format!("appliesTo.roles key '{role}' is not a role name"),
            ));
        }
        if !is_dna_hash(&binding.dna_hash) {
            return Err(refuse(
                RefusalReason::ManifestSchemaInvalid,
                format!(
                    "appliesTo.roles.{role}.dnaHash '{}' is not a DnaHash (uhC0k…)",
                    binding.dna_hash
                ),
            ));
        }
        for hash in &binding.coordinator_wasm_hashes {
            if !is_wasm_hash(hash) {
                return Err(refuse(
                    RefusalReason::ManifestSchemaInvalid,
                    format!(
                        "appliesTo.roles.{role}.coordinatorWasmHashes carries '{hash}', \
                         which is not a WasmHash (uhCok…)"
                    ),
                ));
            }
        }
        let mut seen = BTreeSet::new();
        for hash in &binding.coordinator_wasm_hashes {
            if !seen.insert(hash) {
                return Err(refuse(
                    RefusalReason::ManifestSchemaInvalid,
                    format!("appliesTo.roles.{role}.coordinatorWasmHashes repeats '{hash}'"),
                ));
            }
        }
        // The additive `coordinatorZomes` map is DETAIL over the normative
        // array — so it is cross-checked against it, never trusted alongside
        // it. A map naming a hash the array omits is two declarations of the
        // same fact that disagree, which is exactly the drift the normative
        // field exists to settle.
        if let Some(zomes) = &binding.coordinator_zomes {
            for (zome, hash) in zomes {
                if !is_wasm_hash(hash) {
                    return Err(refuse(
                        RefusalReason::ManifestSchemaInvalid,
                        format!(
                            "appliesTo.roles.{role}.coordinatorZomes.{zome} '{hash}' \
                             is not a WasmHash (uhCok…)"
                        ),
                    ));
                }
                if !binding.coordinator_wasm_hashes.iter().any(|h| h == hash) {
                    return Err(refuse(
                        RefusalReason::ManifestSchemaInvalid,
                        format!(
                            "appliesTo.roles.{role}.coordinatorZomes.{zome} names '{hash}', \
                             absent from the normative coordinatorWasmHashes array"
                        ),
                    ));
                }
            }
        }
        // **Rung 6.** `migrateFrom` and `lineage` are cross-field agreement
        // the vendored schema mirror does not pin (see module docs — it is
        // OPEN by design). Enforced here rather than left as an implicit
        // manifest-author convention: a `happ-lineage` release whose
        // `migrateFrom` names a hash absent from its own `lineage` chain
        // could never establish [`ArtifactClass::HappLineage`]'s crossing_ok
        // in [`verify_envelope`] for THAT hash — better to refuse it at shape
        // time, naming the role, than let it silently fall through to a
        // `dna_lineage_mismatch` that gives no hint why.
        if manifest.artifact_class == ArtifactClass::HappLineage {
            if let Some(from) = &binding.migrate_from {
                let in_lineage = binding
                    .lineage
                    .as_ref()
                    .is_some_and(|l| l.iter().any(|h| h == from));
                if !in_lineage {
                    return Err(refuse(
                        RefusalReason::ManifestSchemaInvalid,
                        format!(
                            "appliesTo.roles.{role}.migrateFrom '{from}' is not a member of its \
                             own lineage chain"
                        ),
                    ));
                }
            }
        }
    }
    if manifest.envelope.wire_epochs.is_empty() {
        return Err(refuse(
            RefusalReason::ManifestSchemaInvalid,
            "envelope.wireEpochs must declare at least one epoch",
        ));
    }
    if manifest.provenance.builder_agent.trim().is_empty() {
        return Err(refuse(
            RefusalReason::ManifestSchemaInvalid,
            "provenance.builderAgent must not be empty — C1 excludes by it",
        ));
    }
    if manifest.provenance.toolchain.trim().is_empty() {
        return Err(refuse(
            RefusalReason::ManifestSchemaInvalid,
            "provenance.toolchain must not be empty",
        ));
    }
    if !is_git_commit(&manifest.provenance.built_from.git_commit) {
        return Err(refuse(
            RefusalReason::ManifestSchemaInvalid,
            format!(
                "provenance.builtFrom.gitCommit '{}' is not a 7-40 char hex commit",
                manifest.provenance.built_from.git_commit
            ),
        ));
    }
    Ok(manifest)
}

fn verify_artifact_shape(artifact: &Artifact) -> Result<(), AdoptionRefusal> {
    if !is_blob_cid(&artifact.blob_cid) {
        return Err(refuse(
            RefusalReason::ManifestSchemaInvalid,
            format!(
                "artifact blobCid '{}' is not a raw-codec CIDv1 (bafkrei…). A bare \
                 sha256-<hex> is the LEGACY blob-path form and is never an address.",
                artifact.blob_cid
            ),
        ));
    }
    if !is_sha256_hex(&artifact.sha256) {
        return Err(refuse(
            RefusalReason::ManifestSchemaInvalid,
            format!(
                "artifact sha256 '{}' is not 64 lowercase hex",
                artifact.sha256
            ),
        ));
    }
    if artifact.filename.is_empty()
        || artifact.filename.contains('/')
        || artifact.filename.contains('\\')
    {
        return Err(refuse(
            RefusalReason::ManifestSchemaInvalid,
            format!(
                "artifact filename '{}' must be a bare base name — the apply vehicle owns \
                 placement",
                artifact.filename
            ),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Envelope — against this peer's installed reality
// ---------------------------------------------------------------------------

/// **The compatibility envelope (spec §8).** Variety lives above it; unity is
/// enforced AT it.
///
/// Since 2026-09-04 this covers the additive floor, the wire epochs, role
/// presence and **the DNA line** — everything that can be, and therefore is,
/// decided before a byte is fetched. The coordinator-wasm (SUPERSEDES) leg
/// moved to [`verify_coordinator_lineage`], which runs after the artifacts so
/// [`already_runs_target`] can answer first. Watch's early call to this function
/// is unchanged and still pays for the DNA-line refusal up front.
pub fn verify_envelope(
    manifest: &ReleaseManifest,
    installed: &Answer<InstalledReality>,
) -> Result<(), AdoptionRefusal> {
    if !manifest.envelope.additive_only {
        return Err(refuse(
            RefusalReason::AdditiveFloorBroken,
            "envelope.additiveOnly is false — a removal or repurposing beyond the additive \
             wire floor is a declared fork or a rung-6 migration, never an accepted envelope",
        ));
    }

    let spoken: Vec<u32> = manifest
        .envelope
        .wire_epochs
        .iter()
        .copied()
        .filter(|e| SUPPORTED_WIRE_EPOCHS.contains(e))
        .collect();
    if spoken.is_empty() {
        return Err(refuse(
            RefusalReason::WireEpochUnsupported,
            format!(
                "release speaks wire epochs {:?}; this build speaks {SUPPORTED_WIRE_EPOCHS:?} — \
                 no common epoch, so mixed-version peers could not keep talking across it",
                manifest.envelope.wire_epochs
            ),
        ));
    }

    let installed = match installed {
        Answer::Present(reality) => reality,
        // C4: we could not READ our own installed reality. The envelope was
        // never checked; saying "mismatch" would be a claim we have no
        // evidence for, in the direction that looks like diligence.
        Answer::Unreachable | Answer::Absent => {
            return Err(refuse(
                RefusalReason::InstalledRealityUnknown,
                "this peer's own installed reality could not be read (no conductor admin \
                 connection, or the hApp inventory did not answer) — the envelope was not \
                 checked, which is not the same as failing it",
            ));
        }
    };

    for (role, binding) in &manifest.applies_to.roles {
        let Some(installed_role) = installed.roles.get(role) else {
            return Err(refuse(
                RefusalReason::RoleNotInstalled,
                format!(
                    "release binds role '{role}', which this peer does not run (installed: {:?})",
                    installed.roles.keys().collect::<Vec<_>>()
                ),
            ));
        };

        // THE DNA LINE. `update_coordinators` matches integrity dependencies by
        // NAME, so a cross-lineage bundle would splice coordinators onto
        // integrity zomes they were never compiled against. This is
        // `happ_manager::lineage_mismatch_error`'s refusal, moved to verify
        // time. **Rung 6 (2026-09-04):** a `happ-lineage` release may cross
        // this line — but ONLY by declaring the installed hash as BOTH its
        // `migrateFrom` and a member of its `lineage` chain. Declaring intent
        // is not evidence: [`verify_path`] still has to find a notarized,
        // unrevoked, quorum-met commitment naming this exact crossing before
        // the release is verified — this check just lets the manifest SAY it
        // wants to cross, structurally refusing every other class the same
        // as before.
        if binding.dna_hash != installed_role.dna_hash {
            let crossing_ok = manifest.artifact_class == ArtifactClass::HappLineage
                && binding.migrate_from.as_deref() == Some(installed_role.dna_hash.as_str())
                && binding
                    .lineage
                    .as_ref()
                    .is_some_and(|l| l.iter().any(|h| h == &installed_role.dna_hash));
            if !crossing_ok {
                return Err(refuse(
                    RefusalReason::DnaLineageMismatch,
                    format!(
                        "role '{role}': release binds DNA {} but this peer runs {} — crossing \
                         the DNA line needs a happ-lineage release whose migrateFrom names the \
                         installed hash and whose lineage contains it (spec \
                         2026-09-03-holochain-evolution-epic-design §4)",
                        binding.dna_hash, installed_role.dna_hash
                    ),
                ));
            }
        }

        // The coordinator-wasm (SUPERSEDES) leg is deliberately NOT here — see
        // [`verify_coordinator_lineage`]. It has to run AFTER the artifact
        // bytes are on disk, because the by-bytes exit that precedes it can
        // only be decided from those bytes.
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Already current BY BYTES — the exit that precedes the supersedes check
// ---------------------------------------------------------------------------

/// What a release's artifact bytes WOULD install, per role: role name → (zome
/// name → coordinator wasm hash). Derived from the staged bundle itself
/// (`happ_manager::bundle_coordinator_wasm_hashes`), because a release manifest
/// declares only the hashes it SUPERSEDES and never the ones it provides.
pub type TargetCoordinators = BTreeMap<String, BTreeMap<String, String>>;

/// **Already current by BYTES.** Does this peer's installed reality ALREADY
/// equal what this release would install, for every role the release touches?
///
/// Why this is not the same question as `appliedRelease.cid == resolvedHead.cid`
/// (the C6b idempotence exit in `watch`): a channel can be re-authored as a NEW
/// manifest over the SAME bytes — a threshold-0 revert does exactly that — and
/// then the CIDs differ while the bytes do not. Measured live on the household
/// mesh 2026-09-04: james ran the release's target coordinator wasm and refused
/// the re-authored manifest for it on every sweep with
/// `coordinator_lineage_mismatch`, because the only thing the supersedes check
/// can see is that james does not run the OLD bytes — which is precisely what
/// having already adopted the new ones means.
///
/// Three states, and only the first is an exit:
///
/// - every touched role's installed coordinator map equals the target's → the
///   peer is CURRENT; applying would be a byte-for-byte no-op.
/// - some touched role differs (including a role the target does not name) →
///   NOT current: fall through to [`verify_coordinator_lineage`], which decides
///   whether this peer is one this release was cut for.
/// - the target is unknown (a non-bundle artifact class, unreadable bytes, or
///   an installed reality we could not read) → NOT current, same fall-through.
///   Absence of evidence never takes an exit that STOPS work.
///
/// Exactness is the safety property: the comparison is the whole per-role zome
/// map, which is the same drift unit `happ_manager::role_report` computes, so
/// "already current" here means exactly "a hot-swap of this bundle would report
/// zero drift for this role". A peer running NEITHER the superseded nor the
/// target bytes fails it and is refused, as before.
pub fn already_runs_target(
    manifest: &ReleaseManifest,
    installed: &Answer<InstalledReality>,
    target: &Answer<TargetCoordinators>,
) -> bool {
    let (Answer::Present(installed), Answer::Present(target)) = (installed, target) else {
        return false;
    };
    // A release that touches no role can never be "already current by bytes":
    // there is nothing to have equalled.
    if manifest.applies_to.roles.is_empty() {
        return false;
    }
    manifest.applies_to.roles.keys().all(|role| {
        match (installed.roles.get(role), target.get(role)) {
            // An empty target map is not equality evidence — it is a bundle
            // that resolved no coordinator zomes for the role at all.
            (Some(running), Some(want)) if !want.is_empty() => running.coordinator_zomes == *want,
            _ => false,
        }
    })
}

/// The coordinator-wasm leg of the envelope: **who this release was cut for.**
///
/// For a coordinator-bundle release the manifest's declared hashes are what the
/// release SUPERSEDES — what it applies ONTO. A peer that does not run them is
/// not the peer this release was cut for.
///
/// Split out of [`verify_envelope`] so the by-bytes exit
/// ([`already_runs_target`]) can be decided first: the target hashes live in the
/// artifact bytes, so this check is the one part of the envelope that cannot be
/// settled before those bytes are staged and verified. Everything the envelope
/// decides about corruption — the DNA line above all — stays in
/// `verify_envelope`, where it is still paid for before a byte is fetched.
pub fn verify_coordinator_lineage(
    manifest: &ReleaseManifest,
    installed: &Answer<InstalledReality>,
) -> Result<(), AdoptionRefusal> {
    let installed = match installed {
        Answer::Present(reality) => reality,
        Answer::Unreachable | Answer::Absent => {
            return Err(refuse(
                RefusalReason::InstalledRealityUnknown,
                "this peer's own installed reality could not be read — the coordinator lineage \
                 was not checked, which is not the same as failing it",
            ));
        }
    };
    for (role, binding) in &manifest.applies_to.roles {
        let Some(installed_role) = installed.roles.get(role) else {
            return Err(refuse(
                RefusalReason::RoleNotInstalled,
                format!(
                    "release binds role '{role}', which this peer does not run (installed: {:?})",
                    installed.roles.keys().collect::<Vec<_>>()
                ),
            ));
        };
        let running = installed_role.wasm_hashes();
        let missing: Vec<&String> = binding
            .coordinator_wasm_hashes
            .iter()
            .filter(|h| !running.contains(h.as_str()))
            .collect();
        if !missing.is_empty() {
            return Err(refuse(
                RefusalReason::CoordinatorLineageMismatch,
                format!(
                    "role '{role}': release supersedes coordinator wasm {missing:?}, which this \
                     peer does not run (running: {running:?})"
                ),
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Lineage — the body field is a HINT that must match the L2 chain
// ---------------------------------------------------------------------------

/// Verify the manifest's declared lineage parent against the channel's L2
/// version chain.
///
/// **C5.** Unreadable lineage establishes nothing in either direction, so it is
/// its own refusal (`lineage_unverifiable`) rather than a pass or a mismatch.
pub fn verify_lineage(
    manifest: &ReleaseManifest,
    chain: &Answer<LineageEvidence>,
) -> Result<(), AdoptionRefusal> {
    let chain = match chain {
        Answer::Present(evidence) => evidence,
        Answer::Absent | Answer::Unreachable => {
            return Err(refuse(
                RefusalReason::LineageUnverifiable,
                "the channel's L2 version chain could not be read, so the manifest's declared \
                 lineage parent proves nothing in either direction",
            ));
        }
    };
    let declared = manifest.envelope.lineage_parent_cid.as_deref();
    let actual = chain.supersedes.as_deref();
    if declared != actual {
        return Err(refuse(
            RefusalReason::LineageParentMismatch,
            format!(
                "envelope.lineageParentCid declares {declared:?} but the channel's release \
                 chain supersedes {actual:?} — the body field is a hint that MUST match the L2 \
                 chain; when the channel's version chain does not order releases (a star chain \
                 around the root, which `update_content` targets for every non-first version), \
                 the declared parent must exist as a release on this channel"
            ),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Path — rung 6's notarized crossing, checked before the threshold
// ---------------------------------------------------------------------------

/// **Rung 6.** Verify the notarized migrates-lineage commitment a
/// `happ-lineage` release crosses the DNA line under.
///
/// A no-op — `Ok(())` — for every artifact class but [`ArtifactClass::HappLineage`];
/// every other class never touches `adoptionDiscipline.path`, so there is
/// nothing for this check to do on their behalf. `path` is caller-fetched
/// evidence (module docs: this module does no I/O), typed [`Answer`] for the
/// same C4 reason every other floor input is: `Answer::Absent` means "not
/// notarized (yet)", never "refused"; `Answer::Unreachable` means this peer's
/// own conductor could not answer, which establishes nothing about the
/// commitment either way.
pub fn verify_path(
    manifest: &ReleaseManifest,
    installed: &InstalledReality,
    path: &Answer<PathEvidence>,
) -> Result<(), AdoptionRefusal> {
    if manifest.artifact_class != ArtifactClass::HappLineage {
        return Ok(());
    }
    let wanted = manifest.adoption_discipline.path.as_ref().ok_or_else(|| {
        refuse(
            RefusalReason::ManifestSchemaInvalid,
            "artifactClass is happ-lineage without adoptionDiscipline.path — a lineage release \
             with no notarized commitment to cross under is not a legal manifest",
        )
    })?;

    let ev = match path {
        Answer::Present(ev) => ev,
        // C4: not notarized YET is not the same as refused. The commitment
        // may simply not have replicated to this peer's conductor.
        Answer::Absent => {
            return Err(refuse(
                RefusalReason::PathNotNotarized,
                format!(
                    "no migrates-lineage commitment {} is notarized on this peer's conductor",
                    wanted.commitment_cid
                ),
            ));
        }
        // C4: this peer could not read the commitment at all — a fact about
        // us, not about the commitment. Not the same refusal as "read it and
        // it did not match."
        Answer::Unreachable => {
            return Err(refuse(
                RefusalReason::ConductorUnavailable,
                "path evidence unreadable — establishes nothing in either direction (C4)",
            ));
        }
    };

    if ev.commitment_cid != wanted.commitment_cid {
        return Err(refuse(
            RefusalReason::PathNotNotarized,
            format!(
                "commitment {} is not the manifest's path {}",
                ev.commitment_cid, wanted.commitment_cid
            ),
        ));
    }
    if ev.revoked_at.is_some() {
        return Err(refuse(
            RefusalReason::PathRevoked,
            format!(
                "path {} revoked at {}",
                ev.commitment_cid,
                ev.revoked_at.clone().unwrap()
            ),
        ));
    }
    if ev.state != "active" {
        return Err(refuse(
            RefusalReason::PathNotNotarized,
            format!("path {} is {}, not active", ev.commitment_cid, ev.state),
        ));
    }
    if ev.signatures < ev.required_signatures {
        return Err(refuse(
            RefusalReason::QuorumUnmet,
            format!("{} of {} signatures", ev.signatures, ev.required_signatures),
        ));
    }

    // **Task 16 — the roster check (epic §4.1).** The count above says HOW
    // MANY signed; this says WHOSE signature was ever allowed to count. Until
    // it existed, a `migrates-lineage` commitment notarized through a
    // household peer whose key sits on no roster at all was accepted by every
    // peer on the mesh (measured, Station 10, `cucumber-stations-mvp-r14`) —
    // the quorum was a headcount with no electorate.
    //
    // EVIDENCE, NEVER AUTHORITY (C5). `roster_members` was read through this
    // peer's OWN conductor from the commitment the body's `roster_cid` names,
    // never taken from the body's word for who its signers are. What is NOT
    // checked here is the roster's own chain back to `constitution_root`:
    // that is the integrity-side arm, hash-moving on mishpat, and named as
    // such rather than faked here.
    let Some(members) = ev.roster_members.as_ref() else {
        // C4 applied to the ROSTER: this is the conductor's answer of
        // absence, not our failure to read (that is `Unreachable` above, and
        // refuses as `conductor_unavailable`). A quorum this peer cannot
        // check is not a quorum it may assume — so it refuses, and self-heals
        // the moment the roster gossips here.
        return Err(refuse(
            RefusalReason::QuorumUnmet,
            format!("roster {} not found", ev.roster_cid),
        ));
    };
    for signer in &ev.signers {
        if !members.iter().any(|m| m == signer) {
            return Err(refuse(
                RefusalReason::QuorumUnmet,
                format!("signer {} is not on roster {}", signer, ev.roster_cid),
            ));
        }
    }
    // The rule the refusal above is only the loud half of: an off-roster
    // signature NEVER counts toward `required_signatures`. Kept as its own
    // check because it is the invariant — the loop is one policy over it (any
    // stranger at all refuses), and a body whose `signatures` array carries
    // more elements than it does readable `agent`s reaches here with a
    // headcount the roster cannot back.
    let on_roster = ev
        .signers
        .iter()
        .filter(|s| members.iter().any(|m| m == *s))
        .count();
    if on_roster < ev.required_signatures {
        return Err(refuse(
            RefusalReason::QuorumUnmet,
            format!(
                "{} of {} signatures from roster {}",
                on_roster, ev.required_signatures, ev.roster_cid
            ),
        ));
    }

    for (role, binding) in &manifest.applies_to.roles {
        let Some(inst) = installed.roles.get(role) else {
            // Already refused as RoleNotInstalled by verify_envelope, which
            // runs before this in `verify`. Nothing further to say here.
            continue;
        };
        if ev.from_dna_hash != inst.dna_hash || ev.to_dna_hash != binding.dna_hash {
            return Err(refuse(
                RefusalReason::PathNotNotarized,
                format!(
                    "path {} names {}→{}, release is {}→{}",
                    ev.commitment_cid,
                    ev.from_dna_hash,
                    ev.to_dna_hash,
                    inst.dna_hash,
                    binding.dna_hash
                ),
            ));
        }
        if let Some(root) = inst.constitution_root.as_deref() {
            if root != ev.constitution_root {
                return Err(refuse(
                    RefusalReason::RootMismatch,
                    format!("path root {} ≠ installed root {root}", ev.constitution_root),
                ));
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Threshold — read through T5, never re-derived
// ---------------------------------------------------------------------------

/// Verify the attestation threshold the manifest's own `adoptionDiscipline`
/// declares.
///
/// `evidence` is `None` when the count could not be taken at all — the reader
/// refused, or T5's rail was unavailable. That is `threshold_unchecked`, and
/// **it is not a pass**: a conductor that cannot answer must never be read as
/// consent.
///
/// C1 is not re-derived here. The builder exclusion is a construction
/// obligation on `AdoptionDiscipline` (only `ChannelAdoptionDiscipline::for_release`
/// mints one, and it takes the builder), so by the time evidence exists the
/// builder's own attestation has already been excluded by type.
pub fn verify_threshold(
    manifest: &ReleaseManifest,
    evidence: Option<&QualifyingEvidence>,
) -> Result<(), AdoptionRefusal> {
    let Some(evidence) = evidence else {
        return Err(refuse(
            RefusalReason::ThresholdUnchecked,
            format!(
                "adoptionDiscipline requires {} qualifying attestation(s); this peer could not \
                 read the count. Unchecked is NOT a pass.",
                manifest.adoption_discipline.attestation_threshold
            ),
        ));
    };
    if evidence.is_degraded() {
        return Err(refuse(
            RefusalReason::ThresholdEvidenceDegraded,
            format!(
                "attestation count is provably incomplete ({} provenance-mismatched, {} \
                 unresolved) — a reason to sweep again, never to promote",
                evidence.provenance_mismatched, evidence.unresolved
            ),
        ));
    }
    if !evidence.threshold_met() {
        return Err(refuse(
            RefusalReason::ThresholdUnmet,
            format!(
                "{} qualifying attestation(s) of {} required ({} total seen, {} builder-excluded, \
                 {} failed soak)",
                evidence.qualifying,
                evidence.threshold,
                evidence.total,
                evidence.builder_excluded,
                evidence.failed
            ),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Artifacts — byte-exact, and the two failure stories kept apart
// ---------------------------------------------------------------------------

/// One artifact's bytes, as a source produced them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedArtifact {
    /// The address the source was asked for.
    pub blob_cid: String,
    /// Where the verified bytes now live.
    pub path: PathBuf,
    /// Actual byte length.
    pub bytes: u64,
    /// Actual sha2-256, lowercase hex.
    pub sha256: String,
}

/// Match fetched bytes to the manifest, in manifest order.
///
/// Length and digest are separate refusals on purpose: a length mismatch is a
/// truncation or a wrong object, a digest mismatch at the right length is
/// corruption or substitution, and one bucket for both loses the only signal
/// that tells those apart.
pub fn verify_artifacts(
    manifest: &ReleaseManifest,
    fetched: &[FetchedArtifact],
) -> Result<Vec<PathBuf>, AdoptionRefusal> {
    let mut paths = Vec::with_capacity(manifest.artifacts.len());
    for declared in &manifest.artifacts {
        let Some(actual) = fetched.iter().find(|f| f.blob_cid == declared.blob_cid) else {
            return Err(refuse(
                RefusalReason::ArtifactUnavailable,
                format!(
                    "no source served bytes for '{}' ({}) — the bytes may simply not have \
                     replicated yet; this is not absence",
                    declared.filename, declared.blob_cid
                ),
            ));
        };
        if actual.bytes != declared.bytes {
            return Err(refuse(
                RefusalReason::ArtifactLengthMismatch,
                format!(
                    "'{}' ({}): manifest declares {} bytes, source served {} — a different \
                     length is a typed refusal, never a retry",
                    declared.filename, declared.blob_cid, declared.bytes, actual.bytes
                ),
            ));
        }
        if !actual.sha256.eq_ignore_ascii_case(&declared.sha256) {
            return Err(refuse(
                RefusalReason::ArtifactDigestMismatch,
                format!(
                    "'{}' ({}): manifest declares sha256 {}, source served {}",
                    declared.filename, declared.blob_cid, declared.sha256, actual.sha256
                ),
            ));
        }
        paths.push(actual.path.clone());
    }
    Ok(paths)
}

// ---------------------------------------------------------------------------
// The composed floor
// ---------------------------------------------------------------------------

/// Everything a verify needs, assembled by the caller.
pub struct VerifyInput<'a> {
    /// The channel we resolved this manifest from.
    pub channel_id: &'a str,
    /// The winning version's action hash (base64).
    pub release_cid: &'a str,
    /// The `manifest` object out of the head's `metadata_json` envelope.
    pub body: &'a serde_json::Value,
    pub installed: &'a Answer<InstalledReality>,
    pub lineage: &'a Answer<LineageEvidence>,
    pub artifacts: &'a [FetchedArtifact],
    /// `None` when the attestation count could not be taken.
    pub attestations: Option<&'a QualifyingEvidence>,
    /// **Design 2026-09-01 (canary-first adoption).** The election tier
    /// behind the head this manifest rode in on. Gates WHETHER the threshold
    /// is enforced: `Earned` (and `None`, unchanged from before this design)
    /// enforce it exactly as `verify_threshold` always has; `Staging` does
    /// not — the threshold gates PROMOTION, never staging adoption, so a
    /// canary soaking a staging head must be able to verify it with zero
    /// qualifying attestations.
    pub tier: HeadTier,
    /// **The by-bytes evidence (2026-09-04).** What the STAGED artifact would
    /// install, per role — `Answer::Absent` for an artifact class that installs
    /// no coordinators, `Answer::Unreachable` when the bytes could not be read.
    /// Read only by [`already_runs_target`]; absence never takes the exit.
    pub target_coordinators: &'a Answer<TargetCoordinators>,
    /// **Rung 6 (2026-09-04).** Caller-fetched evidence for the manifest's
    /// `adoptionDiscipline.path` commitment — this module does no I/O (module
    /// docs), so the fetch lives entirely on the caller's side. Read only by
    /// [`verify_path`], and only when `manifest.artifactClass` is
    /// `happ-lineage`; every other class ignores it. `Answer::Absent` for
    /// every existing caller today (the fetch site is a later task's), which
    /// is exactly what `verify_path` needs to be a no-op for the four
    /// existing artifact classes.
    pub path: Answer<PathEvidence>,
}

/// What the floor decided. Two ways to pass, and they are not the same fact.
#[derive(Debug, Clone, PartialEq)]
pub enum VerifyOutcome {
    /// The release passed the whole floor and is ready for a vehicle.
    Verified(Box<VerifiedRelease>),
    /// This peer ALREADY runs exactly what the release would install, for every
    /// role it touches ([`already_runs_target`]). Convergence, reached without
    /// spending a vehicle — and never a refusal, however the manifest was
    /// authored. The roles are carried for the report.
    AlreadyCurrent { roles: Vec<String> },
}

/// Run the whole floor. The ONLY constructor of a [`VerifiedRelease`].
///
/// Order matters for cost, not correctness: shape and channel identity are free
/// and reject the largest class of garbage before any installed-reality read.
///
/// **The one place order is load-bearing** is the tail. The by-bytes exit
/// ([`already_runs_target`]) sits AFTER [`verify_artifacts`] and BEFORE
/// [`verify_coordinator_lineage`]:
///
/// - after the artifacts, because the target hashes are read out of the staged
///   bytes, and trusting bytes whose length and digest have not been checked
///   would move the floor from "verify then apply" to "believe what is on disk";
/// - before the supersedes check, because a peer that already runs the target
///   is exactly the peer the supersedes check reports as running neither.
pub fn verify(input: VerifyInput<'_>) -> Result<VerifyOutcome, AdoptionRefusal> {
    let manifest = verify_shape(input.body)?;

    if manifest.channel_id != input.channel_id {
        return Err(refuse(
            RefusalReason::ChannelIdMismatch,
            format!(
                "manifest declares channel '{}' but was resolved from '{}' — a release found \
                 on the wrong channel is laundered, whatever else is true about it",
                manifest.channel_id, input.channel_id
            ),
        ));
    }

    verify_envelope(&manifest, input.installed)?;
    verify_lineage(&manifest, input.lineage)?;

    // **Rung 6.** `verify_envelope` above already required `input.installed`
    // to be `Answer::Present` (any other answer is a refusal it already
    // returned), so this match is exhaustive-but-unreachable on the other two
    // arms rather than a second unwrap of an already-proven fact.
    let installed_reality = match input.installed {
        Answer::Present(reality) => reality,
        Answer::Absent | Answer::Unreachable => {
            unreachable!("verify_envelope already required Answer::Present or returned Err")
        }
    };
    verify_path(&manifest, installed_reality, &input.path)?;

    // The threshold gates PROMOTION (an EARNED head), never staging adoption
    // — a STAGING head's evidence is read and reported elsewhere (the
    // sweep's `attestations` field on `/admin/adoption`), never enforced
    // here. `None` is UNCHANGED from before this design: it enforces exactly
    // as `Earned` does.
    if input.tier != HeadTier::Staging {
        verify_threshold(&manifest, input.attestations)?;
    }
    let artifact_paths = verify_artifacts(&manifest, input.artifacts)?;

    // ALREADY CURRENT BY BYTES — decided from bytes that just proved out, and
    // decided BEFORE the supersedes check that would otherwise refuse exactly
    // the peers this exit is about.
    if already_runs_target(&manifest, input.installed, input.target_coordinators) {
        return Ok(VerifyOutcome::AlreadyCurrent {
            roles: manifest.applies_to.roles.keys().cloned().collect(),
        });
    }

    verify_coordinator_lineage(&manifest, input.installed)?;

    Ok(VerifyOutcome::Verified(Box::new(VerifiedRelease {
        channel_id: input.channel_id.to_string(),
        release_cid: input.release_cid.to_string(),
        manifest,
        artifact_paths,
    })))
}

#[cfg(test)]
mod tests {
    use super::super::{AppliesTo, RoleBinding};
    use super::*;
    use crate::runtime_passport::{HappPassport, HappRolePassport};
    use crate::services::release_attestation::PathRef;
    use std::path::Path;

    const FIXTURE_DIR: &str = "../../genesis/a2o/scripts/__tests__/fixtures";
    const SCHEMA_PATH: &str = "../rakia/schemas/v1/release-manifest.schema.json";

    fn fixture(name: &str) -> serde_json::Value {
        let path = Path::new(FIXTURE_DIR).join(name);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("fixture {} unreadable: {e}", path.display()));
        serde_json::from_str(&text).expect("fixture is JSON")
    }

    fn all_fixtures() -> Vec<(&'static str, serde_json::Value)> {
        [
            "release-manifest-coordinator-bundle.json",
            "release-manifest-config-epr.json",
            "release-manifest-storage-binary.json",
            "release-manifest-happ-bundle.json",
            "release-manifest-envelope-broken.json",
        ]
        .into_iter()
        .map(|n| (n, fixture(n)))
        .collect()
    }

    /// Unwrap the floor's PASS outcome. `AlreadyCurrent` is a pass too, but a
    /// different fact — a test that meant to exercise the vehicle path must
    /// fail loudly rather than silently measure the by-bytes exit.
    fn expect_verified(outcome: VerifyOutcome) -> VerifiedRelease {
        match outcome {
            VerifyOutcome::Verified(v) => *v,
            VerifyOutcome::AlreadyCurrent { roles } => {
                panic!("expected a verified release; the by-bytes exit fired for roles {roles:?}")
            }
        }
    }

    fn installed_from(role: &str, dna: &str, zome: &str, wasm: &str) -> Answer<InstalledReality> {
        InstalledReality::from_happ_passport(&HappPassport {
            app_id: "elohim".to_string(),
            roles: vec![HappRolePassport {
                role: role.to_string(),
                dna_hash: dna.to_string(),
                coordinator_wasm_hashes: [(zome.to_string(), wasm.to_string())]
                    .into_iter()
                    .collect(),
                error: None,
                lineage: None,
            }],
            error: None,
            lineage_apps: Vec::new(),
        })
    }

    /// The vendored mirror accepts every manifest T2's packager actually
    /// produces. Reads the committed fixtures from disk, so the mirror is
    /// measured against an independent artifact rather than against itself.
    #[test]
    fn release_manifest_mirror_accepts_every_committed_fixture() {
        for (name, body) in all_fixtures() {
            let manifest = verify_shape(&body)
                .unwrap_or_else(|e| panic!("fixture {name} refused by the shape floor: {e}"));
            assert_eq!(manifest.kind, RELEASE_MANIFEST_KIND, "{name}");
            assert!(!manifest.artifacts.is_empty(), "{name}");
        }
    }

    /// The mirror and T1's schema agree on the same fixtures. Two genuinely
    /// independent sources — the schema is loaded from disk and applied by a
    /// real JSON-Schema validator — so neither can be measuring the other.
    #[test]
    fn release_manifest_mirror_agrees_with_the_rakia_schema() {
        let schema_text = std::fs::read_to_string(SCHEMA_PATH)
            .unwrap_or_else(|e| panic!("T1 schema unreadable at {SCHEMA_PATH}: {e}"));
        let schema: serde_json::Value =
            serde_json::from_str(&schema_text).expect("T1 schema is JSON");
        let validator = jsonschema::validator_for(&schema).expect("T1 schema compiles");

        for (name, body) in all_fixtures() {
            assert!(
                validator.is_valid(&body),
                "fixture {name} does not satisfy T1's schema"
            );
            assert!(
                verify_shape(&body).is_ok(),
                "fixture {name} satisfies the schema but the Rust mirror refused it — the \
                 mirror has drifted from `elohim/rakia/schemas/v1/release-manifest.schema.json`"
            );
        }
    }

    /// The schema is OPEN by design: a peer one build behind must still read a
    /// release that ADDED a field within the lineage window. If this ever
    /// fails, someone has closed the mirror and broken the mixed-version floor.
    #[test]
    fn an_unknown_property_is_tolerated_because_the_wire_floor_is_additive() {
        let mut body = fixture("release-manifest-coordinator-bundle.json");
        body["someFieldFromANewerRelease"] = serde_json::json!({"nested": true});
        body["artifacts"][0]["futureHint"] = serde_json::json!("ok");
        assert!(
            verify_shape(&body).is_ok(),
            "the mirror must tolerate unknown properties (spec §8.2 additive floor)"
        );
    }

    /// A `sha256-<hex>` in a `cid` field is the legacy-address confusion the
    /// addressing canon exists to refuse. Serde would accept it happily.
    #[test]
    fn a_legacy_sha256_address_is_not_a_blob_cid() {
        let mut body = fixture("release-manifest-coordinator-bundle.json");
        body["artifacts"][0]["blobCid"] = serde_json::json!(
            "sha256-fdecf95c65d9aa4d4f87da0813d7134ec4552388d6e621f7019563b64021ac90"
        );
        let refusal = verify_shape(&body).expect_err("a legacy address is not a CID");
        assert_eq!(refusal.reason_code(), RefusalReason::ManifestSchemaInvalid);
    }

    /// The additive `coordinatorZomes` map is DETAIL over the normative array.
    /// A map naming a hash the array omits is two declarations of one fact that
    /// disagree.
    #[test]
    fn coordinator_zomes_may_not_contradict_the_normative_array() {
        let mut body = fixture("release-manifest-coordinator-bundle.json");
        body["appliesTo"]["roles"]["lamad"]["coordinatorZomes"]["content_store"] =
            serde_json::json!("uhCokZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ");
        let refusal = verify_shape(&body).expect_err("contradiction must refuse");
        assert_eq!(refusal.reason_code(), RefusalReason::ManifestSchemaInvalid);
    }

    /// **DoD arm 1 — envelope mismatch.** T2's deliberately-broken fixture
    /// binds a DNA hash no peer runs. This is the refusal that makes crossing
    /// the DNA line structurally impossible on this rung.
    #[test]
    fn the_envelope_broken_fixture_refuses_on_the_dna_line() {
        let body = fixture("release-manifest-envelope-broken.json");
        let manifest = verify_shape(&body).expect("the broken fixture is well-SHAPED");
        let installed = installed_from(
            "lamad",
            "uhC0kSCgQh19oJMsMmZpirEuCK2NvtW0ULJYQ_Rmpx6AzH8H2Saco",
            "content_store",
            "uhCokJ38rRzUyb_lejmSZVryqTqJ8xqccMhErjIMB22210eSKRcNd",
        );
        let refusal =
            verify_envelope(&manifest, &installed).expect_err("a foreign DNA must be refused");
        assert_eq!(refusal.reason_code(), RefusalReason::DnaLineageMismatch);
        assert!(
            !refusal.transient,
            "the DNA line is terminal, not transient"
        );
    }

    /// Packaging is not verification: the same bytes verify on the peer they
    /// were cut for. Without this the DNA-line test above would only prove that
    /// verify refuses everything.
    #[test]
    fn the_matching_peer_accepts_the_same_envelope() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let manifest = verify_shape(&body).unwrap();
        let binding = &manifest.applies_to.roles["lamad"];
        let installed = installed_from(
            "lamad",
            &binding.dna_hash,
            "content_store",
            &binding.coordinator_wasm_hashes[0],
        );
        verify_envelope(&manifest, &installed).expect("the peer this release was cut for");
    }

    // -------------------------------------------------------------------
    // Rung 6 — the happ-lineage positive branch and verify_path (Task 4)
    //
    // No `happ-lineage` fixture is committed on disk (Task 3 landed the
    // schema, not a packager fixture for it), so `manifest_for` is the one
    // builder in this module that returns a struct literal rather than a
    // disk fixture — it still starts from a REAL fixture (the coordinator-
    // bundle one) so every field this suite does not care about
    // (provenance, envelope, artifacts) stays schema-legal.
    // -------------------------------------------------------------------

    /// A DNA hash this peer runs before crossing. Distinct from [`V2_NR`].
    const INSTALLED_NR: &str = "uhC0kiK2ZWeqhFWCEPyYngFb51yBMWXaSCrUZoL8g5ubbbPIa84yR";
    /// The DNA hash a `happ-lineage` release crosses TO.
    const V2_NR: &str = "uhC0knBUbHoWC8FJowoRoWD8s7bA16J7PglOU3shVv5UTG79BG16Q";
    /// The one notarized commitment [`lineage_manifest`]'s path names.
    const LINEAGE_PATH_CID: &str = "uhCkkLineagePathCommitment";
    /// The constitutional root both [`lineage_manifest`]'s installed role and
    /// [`path_evidence_ok`] agree on by default.
    const LINEAGE_ROOT: &str = "bafyLineageConstitutionRoot";

    /// A manifest of `class`, with one role (`node_registry`) bound to
    /// [`V2_NR`] and no `migrateFrom`/`lineage` declared yet — the caller
    /// mutates those for the case under test, exactly as
    /// `m.applies_to.roles.get_mut("node_registry")` does in the tests below.
    fn manifest_for(class: ArtifactClass) -> ReleaseManifest {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let mut m = verify_shape(&body).unwrap();
        m.artifact_class = class;
        let mut roles = BTreeMap::new();
        roles.insert(
            "node_registry".to_string(),
            RoleBinding {
                dna_hash: V2_NR.to_string(),
                coordinator_wasm_hashes: vec![TARGET_WASM.to_string()],
                coordinator_zomes: None,
                migrate_from: None,
                lineage: None,
            },
        );
        m.applies_to = AppliesTo { roles };
        m
    }

    /// A `happ-lineage` manifest whose `node_registry` binding declares the
    /// crossing FROM [`INSTALLED_NR`] (both `migrateFrom` and `lineage`) TO
    /// [`V2_NR`], with `adoptionDiscipline.path` naming [`LINEAGE_PATH_CID`]
    /// — exactly the manifest [`path_evidence_ok`] is evidence FOR.
    fn lineage_manifest() -> ReleaseManifest {
        let mut m = manifest_for(ArtifactClass::HappLineage);
        {
            let binding = m.applies_to.roles.get_mut("node_registry").unwrap();
            binding.migrate_from = Some(INSTALLED_NR.to_string());
            binding.lineage = Some(vec![INSTALLED_NR.to_string()]);
        }
        m.adoption_discipline.path = Some(PathRef {
            commitment_cid: LINEAGE_PATH_CID.to_string(),
        });
        m
    }

    /// This peer's installed reality for one `role`, running DNA `dna` —
    /// built directly (not through [`installed_from`]'s `HappPassport`
    /// round-trip) because [`verify_path`]'s root check needs an installed
    /// role that DOES declare a `constitutionRoot`, which the runtime
    /// passport does not expose yet (see [`InstalledRole::constitution_root`]
    /// docs). Returns the unwrapped reality — [`verify_path`] takes
    /// `&InstalledReality`, never an `Answer`.
    fn installed_with(role: &str, dna: &str) -> InstalledReality {
        let mut roles = BTreeMap::new();
        roles.insert(
            role.to_string(),
            InstalledRole {
                role: role.to_string(),
                dna_hash: dna.to_string(),
                coordinator_zomes: [("node_registry_zome".to_string(), TARGET_WASM.to_string())]
                    .into_iter()
                    .collect(),
                constitution_root: Some(LINEAGE_ROOT.to_string()),
            },
        );
        InstalledReality {
            app_id: "elohim".to_string(),
            roles,
        }
    }

    /// Evidence that makes [`verify_path`] PASS for [`lineage_manifest`]
    /// against `installed_with("node_registry", INSTALLED_NR)` — the same
    /// commitment, active, unrevoked, quorum exactly met (1 of 1, so a single
    /// mutation to `signatures` alone can swing the quorum check), and the
    /// crossing/root the manifest and installed reality actually declare.
    fn path_evidence_ok() -> PathEvidence {
        PathEvidence {
            commitment_cid: LINEAGE_PATH_CID.to_string(),
            state: "active".to_string(),
            revoked_at: None,
            from_dna_hash: INSTALLED_NR.to_string(),
            to_dna_hash: V2_NR.to_string(),
            constitution_root: LINEAGE_ROOT.to_string(),
            signatures: 1,
            required_signatures: 1,
            roster_cid: LINEAGE_ROSTER_CID.to_string(),
            signers: vec![ROSTER_MEMBER.to_string()],
            roster_members: Some(vec![ROSTER_MEMBER.to_string()]),
        }
    }

    /// The roster the lineage path names, and the one key on it — a declared
    /// 1-of-1 progenitor roster, exactly the household-rehearsal shape epic
    /// §4.1 describes.
    const LINEAGE_ROSTER_CID: &str = "uhCEkLineageCouncilRoster";
    const ROSTER_MEMBER: &str = "uhCAkBootstrapStewardKey";
    /// A household peer's key — a real agent, a real signature, and on no
    /// roster. This is the agent the live Station 10 run notarized a path
    /// through, and every peer accepted it.
    const OFF_ROSTER_SIGNER: &str = "uhCAkJessicaHouseholdKey";

    /// **Task 16 deliverable.** A commitment signed by an agent who is not on
    /// the earned roster is `quorum_unmet` — even though the COUNT is
    /// satisfied, which is precisely what the mesh accepted before this check
    /// existed. The refusal names the signer and the roster, so an operator
    /// can go and look at both.
    #[test]
    fn verify_path_refuses_a_signer_who_is_not_on_the_roster() {
        let m = lineage_manifest();
        let inst = installed_with("node_registry", INSTALLED_NR);

        let mut ev = path_evidence_ok();
        ev.signers = vec![OFF_ROSTER_SIGNER.to_string()];
        // The count is MET — one signature, one required. Only the roster
        // makes this a refusal.
        assert_eq!(ev.signatures, 1);
        assert_eq!(ev.required_signatures, 1);

        let r = verify_path(&m, &inst, &Answer::Present(ev)).unwrap_err();
        assert_eq!(r.reason_code(), RefusalReason::QuorumUnmet);
        assert!(
            r.detail.contains(OFF_ROSTER_SIGNER) && r.detail.contains(LINEAGE_ROSTER_CID),
            "the refusal must name the stranger AND the roster they are absent from; got {:?}",
            r.detail
        );
    }

    /// An off-roster signature never COUNTS, which is the invariant the
    /// stranger-refusal above is one policy over. Here the quorum needs two
    /// and the body carries two — one member, one stranger — so a check that
    /// counted signatures rather than members would pass.
    #[test]
    fn only_roster_members_count_toward_the_quorum() {
        let m = lineage_manifest();
        let inst = installed_with("node_registry", INSTALLED_NR);

        let mut ev = path_evidence_ok();
        ev.required_signatures = 2;
        ev.signatures = 2;
        ev.signers = vec![ROSTER_MEMBER.to_string(), OFF_ROSTER_SIGNER.to_string()];
        let r = verify_path(&m, &inst, &Answer::Present(ev.clone())).unwrap_err();
        assert_eq!(
            r.reason_code(),
            RefusalReason::QuorumUnmet,
            "2 of 2 signatures, but only 1 of them from the roster"
        );

        // …and the same commitment with the SECOND signer on the roster
        // passes, so what is pinned is the membership rule and not a blanket
        // refusal of every two-signature path.
        ev.roster_members = Some(vec![
            ROSTER_MEMBER.to_string(),
            OFF_ROSTER_SIGNER.to_string(),
        ]);
        verify_path(&m, &inst, &Answer::Present(ev)).expect("both signers on the roster");
    }

    /// **C4 on the roster.** A roster this peer's conductor answered "no such
    /// entry" for refuses `quorum_unmet` — a quorum we cannot check is not a
    /// quorum we may assume. (A roster we could not READ never reaches
    /// `verify_path` at all: that is `Answer::Unreachable` on the whole
    /// evidence, pinned in `path_evidence`.)
    #[test]
    fn a_roster_this_peer_cannot_find_refuses_rather_than_assuming_a_quorum() {
        let m = lineage_manifest();
        let inst = installed_with("node_registry", INSTALLED_NR);

        let mut ev = path_evidence_ok();
        ev.roster_members = None;
        let r = verify_path(&m, &inst, &Answer::Present(ev)).unwrap_err();
        assert_eq!(r.reason_code(), RefusalReason::QuorumUnmet);
        assert!(
            r.detail.contains(LINEAGE_ROSTER_CID),
            "the refusal must name the roster that could not be found; got {:?}",
            r.detail
        );
    }

    /// **Task 4 deliverable, positive half.** A `happ-lineage` release whose
    /// binding names the installed hash as both `migrateFrom` and a member of
    /// `lineage` is NOT a `dna_lineage_mismatch` — the declaration alone is
    /// enough for `verify_envelope` to let it through; the notarized evidence
    /// is [`verify_path`]'s job, checked separately.
    #[test]
    fn happ_lineage_positive_branch_accepts_migrate_from_equal_to_installed() {
        let mut m = manifest_for(ArtifactClass::HappLineage);
        {
            let binding = m.applies_to.roles.get_mut("node_registry").unwrap();
            binding.migrate_from = Some(INSTALLED_NR.to_string());
            binding.dna_hash = V2_NR.to_string();
            binding.lineage = Some(vec![INSTALLED_NR.to_string()]);
        }
        let installed = installed_with("node_registry", INSTALLED_NR);
        assert!(verify_envelope(&m, &Answer::Present(installed)).is_ok());
    }

    /// **Task 4 deliverable, negative half.** `migrateFrom` naming a hash
    /// this peer does NOT run is the ordinary DNA-line refusal — a
    /// `happ-lineage` release does not get a free pass, it gets a NARROWER
    /// one.
    #[test]
    fn happ_lineage_refuses_when_migrate_from_is_not_installed() {
        let mut m = manifest_for(ArtifactClass::HappLineage);
        m.applies_to
            .roles
            .get_mut("node_registry")
            .unwrap()
            .migrate_from =
            Some("uhC0kNotTheInstalledHashAtAll00000000000000000000000".to_string());
        let r = verify_envelope(
            &m,
            &Answer::Present(installed_with("node_registry", INSTALLED_NR)),
        )
        .unwrap_err();
        assert_eq!(r.reason_code(), RefusalReason::DnaLineageMismatch);
    }

    // `coordinator_bundle_still_refuses_dna_line`: the existing
    // `the_envelope_broken_fixture_refuses_on_the_dna_line` test above is
    // exactly this case (a `coordinator-bundle` release with a foreign DNA
    // hash, still refused) — `crossing_ok` is `false` by construction for
    // any class but `HappLineage`, so it needed no change and is left
    // unchanged per the brief.

    /// No evidence at all is `path_not_notarized`, never absence read as
    /// "refused" — C4: the commitment may simply not have replicated yet.
    #[test]
    fn verify_path_absent_is_path_not_notarized() {
        let m = lineage_manifest();
        let r = verify_path(
            &m,
            &installed_with("node_registry", INSTALLED_NR),
            &Answer::Absent,
        )
        .unwrap_err();
        assert_eq!(r.reason_code(), RefusalReason::PathNotNotarized);
    }

    /// One evidence value, mutated one field at a time: revoked (terminal),
    /// quorum unmet (substrate lag), root mismatch (a declaration only a new
    /// release cures). Each assertion checks the NAMED reason, not merely
    /// `is_err()` — a wrong reason here would still turn the test green.
    #[test]
    fn verify_path_revoked_quorum_root() {
        let m = lineage_manifest();
        let inst = installed_with("node_registry", INSTALLED_NR);

        let mut ev = path_evidence_ok();
        ev.revoked_at = Some("2026-09-04T00:00:00Z".to_string());
        assert_eq!(
            verify_path(&m, &inst, &Answer::Present(ev.clone()))
                .unwrap_err()
                .reason_code(),
            RefusalReason::PathRevoked
        );

        ev.revoked_at = None;
        ev.signatures = 0;
        assert_eq!(
            verify_path(&m, &inst, &Answer::Present(ev.clone()))
                .unwrap_err()
                .reason_code(),
            RefusalReason::QuorumUnmet
        );

        ev.signatures = 1;
        ev.constitution_root = "bafyOTHERConstitutionRoot".to_string();
        assert_eq!(
            verify_path(&m, &inst, &Answer::Present(ev))
                .unwrap_err()
                .reason_code(),
            RefusalReason::RootMismatch
        );
    }

    /// The baseline evidence actually PASSES — without this, the four
    /// refusal tests above would only prove that `verify_path` refuses
    /// everything, the same packaging-is-not-verification gap
    /// `the_matching_peer_accepts_the_same_envelope` exists to close for the
    /// envelope check.
    #[test]
    fn verify_path_accepts_the_matching_evidence() {
        let m = lineage_manifest();
        let inst = installed_with("node_registry", INSTALLED_NR);
        verify_path(&m, &inst, &Answer::Present(path_evidence_ok()))
            .expect("evidence cut for exactly this crossing");
    }

    /// A `happ-lineage` manifest whose `adoptionDiscipline.path` is absent is
    /// not a legal manifest — refused at `verify_path` time (the vendored
    /// schema mirror is OPEN by design and does not itself pin this).
    #[test]
    fn happ_lineage_without_a_declared_path_is_schema_invalid() {
        let m = manifest_for(ArtifactClass::HappLineage);
        // `manifest_for` does not set `adoptionDiscipline.path` — confirm the
        // fixture matches the assumption this test is making.
        assert!(m.adoption_discipline.path.is_none());
        let r = verify_path(
            &m,
            &installed_with("node_registry", INSTALLED_NR),
            &Answer::Absent,
        )
        .unwrap_err();
        assert_eq!(r.reason_code(), RefusalReason::ManifestSchemaInvalid);
    }

    /// **The cross-field check `verify_shape` enforces (the brief's "enforce
    /// in Rust what the schema cannot").** A `happ-lineage` binding whose
    /// `migrateFrom` names a hash absent from its own `lineage` chain is
    /// refused at shape time, naming the role — never silently accepted only
    /// to fail `crossing_ok` later with no hint why.
    #[test]
    fn migrate_from_must_be_a_member_of_its_own_lineage() {
        let mut m = manifest_for(ArtifactClass::HappLineage);
        {
            let binding = m.applies_to.roles.get_mut("node_registry").unwrap();
            binding.migrate_from = Some(INSTALLED_NR.to_string());
            binding.lineage = Some(vec![V2_NR.to_string()]); // does NOT contain INSTALLED_NR
        }
        let body = serde_json::to_value(&m).expect("manifest re-serializes");
        let r = verify_shape(&body).unwrap_err();
        assert_eq!(r.reason_code(), RefusalReason::ManifestSchemaInvalid);
    }

    /// **C4 / DoD arm 4 — honest absence.** An unreadable passport is
    /// `installed_reality_unknown`, never `dna_lineage_mismatch`. Reading a
    /// failed conductor inventory as a mismatch is a claim we have no evidence
    /// for, in the direction that looks like diligence.
    #[test]
    fn an_unreadable_passport_is_unknown_reality_never_a_mismatch() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let manifest = verify_shape(&body).unwrap();

        let errored = InstalledReality::from_happ_passport(&HappPassport {
            app_id: "elohim".to_string(),
            roles: Vec::new(),
            error: Some("conductor admin connection unavailable".to_string()),
            lineage_apps: Vec::new(),
        });
        assert!(matches!(errored, Answer::Unreachable));
        let refusal = verify_envelope(&manifest, &errored).expect_err("unknown reality refuses");
        assert_eq!(
            refusal.reason_code(),
            RefusalReason::InstalledRealityUnknown
        );
        assert!(refusal.transient, "a conductor that comes back cures this");

        // A role whose get_dna_definition failed demotes THAT role, and when it
        // is the only one the whole answer is unreachable — never a silent
        // "this peer runs nothing".
        let role_errored = InstalledReality::from_happ_passport(&HappPassport {
            app_id: "elohim".to_string(),
            roles: vec![HappRolePassport {
                role: "lamad".to_string(),
                dna_hash: "uhC0kSCgQh19oJMsMmZpirEuCK2NvtW0ULJYQ_Rmpx6AzH8H2Saco".to_string(),
                coordinator_wasm_hashes: BTreeMap::new(),
                error: Some("get_dna_definition failed".to_string()),
                lineage: None,
            }],
            error: None,
            lineage_apps: Vec::new(),
        });
        assert!(matches!(role_errored, Answer::Unreachable));
    }

    /// A peer that does not run the role at all is its own refusal — distinct
    /// from running it on a different lineage.
    #[test]
    fn a_role_this_peer_does_not_run_is_not_a_lineage_mismatch() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let manifest = verify_shape(&body).unwrap();
        let installed = installed_from(
            "imagodei",
            "uhC0kSCgQh19oJMsMmZpirEuCK2NvtW0ULJYQ_Rmpx6AzH8H2Saco",
            "identity",
            "uhCokJ38rRzUyb_lejmSZVryqTqJ8xqccMhErjIMB22210eSKRcNd",
        );
        let refusal = verify_envelope(&manifest, &installed).expect_err("role absent");
        assert_eq!(refusal.reason_code(), RefusalReason::RoleNotInstalled);
    }

    /// The right DNA on the wrong coordinator generation. Kept distinct from
    /// the DNA line because the cures are different: this one a hot-swap can
    /// reach, the DNA line only a rung-6 migration can.
    ///
    /// Lives on [`verify_coordinator_lineage`] rather than [`verify_envelope`]
    /// since 2026-09-04: the supersedes leg runs after the artifact bytes are
    /// staged, so the by-bytes exit gets to answer first.
    #[test]
    fn a_superseded_coordinator_this_peer_does_not_run_refuses_separately() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let manifest = verify_shape(&body).unwrap();
        let binding = &manifest.applies_to.roles["lamad"];
        let installed = installed_from(
            "lamad",
            &binding.dna_hash,
            "content_store",
            "uhCokQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQ",
        );
        // The envelope proper no longer carries this leg — the DNA line and
        // the epoch/role checks pass, and only the supersedes check refuses.
        verify_envelope(&manifest, &installed)
            .expect("the envelope's own checks pass on this peer");
        let refusal =
            verify_coordinator_lineage(&manifest, &installed).expect_err("wrong generation");
        assert_eq!(
            refusal.reason_code(),
            RefusalReason::CoordinatorLineageMismatch
        );
    }

    // -----------------------------------------------------------------------
    // Already current BY BYTES (2026-09-04) — the live james refusal
    // -----------------------------------------------------------------------

    /// Target evidence for one role, in the shape
    /// `happ_manager::bundle_coordinator_wasm_hashes` returns.
    fn target_of(role: &str, zome: &str, wasm: &str) -> Answer<TargetCoordinators> {
        Answer::Present(
            [(
                role.to_string(),
                [(zome.to_string(), wasm.to_string())]
                    .into_iter()
                    .collect::<BTreeMap<String, String>>(),
            )]
            .into_iter()
            .collect(),
        )
    }

    const TARGET_WASM: &str = "uhCokomHUyeMYCMYlWYPFzsepfRjgOi50RXulOl5MQ4STwULXJ7Wb";
    const THIRD_WASM: &str = "uhCokZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ";

    /// **The measured case.** A channel re-authored over the SAME bytes mints a
    /// new release CID, so the CID-equality idempotence exit cannot see it —
    /// and the peer that already adopted those bytes no longer runs what the
    /// manifest says it supersedes. Refusing that peer forever is the bug;
    /// running the target bytes IS being current.
    #[test]
    fn a_peer_running_the_target_bytes_is_already_current() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let manifest = verify_shape(&body).unwrap();
        let binding = &manifest.applies_to.roles["lamad"];
        let installed = installed_from("lamad", &binding.dna_hash, "content_store", TARGET_WASM);
        let target = target_of("lamad", "content_store", TARGET_WASM);

        assert!(already_runs_target(&manifest, &installed, &target));
        // And the supersedes check ALONE would have refused exactly this peer.
        assert_eq!(
            verify_coordinator_lineage(&manifest, &installed)
                .expect_err("the supersedes check cannot see convergence")
                .reason_code(),
            RefusalReason::CoordinatorLineageMismatch
        );
    }

    /// The peer the release WAS cut for still proceeds: it runs the superseded
    /// bytes, not the target ones, so there is nothing already-current about
    /// it and the floor must hand it to a vehicle.
    #[test]
    fn a_peer_running_the_superseded_bytes_proceeds() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let manifest = verify_shape(&body).unwrap();
        let binding = &manifest.applies_to.roles["lamad"];
        let installed = installed_from(
            "lamad",
            &binding.dna_hash,
            "content_store",
            &binding.coordinator_wasm_hashes[0],
        );
        let target = target_of("lamad", "content_store", TARGET_WASM);

        assert!(!already_runs_target(&manifest, &installed, &target));
        verify_coordinator_lineage(&manifest, &installed)
            .expect("the peer this release was cut for");
    }

    /// Running NEITHER is still `coordinator_lineage_mismatch`. The exit must
    /// not become a way for any peer to claim convergence.
    #[test]
    fn a_peer_running_neither_is_still_a_coordinator_lineage_mismatch() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let manifest = verify_shape(&body).unwrap();
        let binding = &manifest.applies_to.roles["lamad"];
        let installed = installed_from("lamad", &binding.dna_hash, "content_store", THIRD_WASM);
        let target = target_of("lamad", "content_store", TARGET_WASM);

        assert!(!already_runs_target(&manifest, &installed, &target));
        assert_eq!(
            verify_coordinator_lineage(&manifest, &installed)
                .expect_err("runs neither")
                .reason_code(),
            RefusalReason::CoordinatorLineageMismatch
        );
    }

    /// Absence of target evidence never takes an exit that stops work: an
    /// unreadable bundle, an artifact class that installs no coordinators, and
    /// an unreadable passport all fall through to the supersedes check.
    #[test]
    fn unknown_evidence_never_takes_the_by_bytes_exit() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let manifest = verify_shape(&body).unwrap();
        let binding = &manifest.applies_to.roles["lamad"];
        let installed = installed_from("lamad", &binding.dna_hash, "content_store", TARGET_WASM);
        let target = target_of("lamad", "content_store", TARGET_WASM);

        assert!(!already_runs_target(&manifest, &installed, &Answer::Absent));
        assert!(!already_runs_target(
            &manifest,
            &installed,
            &Answer::Unreachable
        ));
        assert!(!already_runs_target(
            &manifest,
            &Answer::Unreachable,
            &target
        ));
        // An empty per-role target map is not equality evidence either.
        let empty_role: Answer<TargetCoordinators> = Answer::Present(
            [("lamad".to_string(), BTreeMap::new())]
                .into_iter()
                .collect(),
        );
        assert!(!already_runs_target(&manifest, &installed, &empty_role));
    }

    /// EVERY touched role must match. A release that touches two roles and
    /// finds one of them drifted is not already current — applying it would
    /// still change something.
    #[test]
    fn every_touched_role_must_equal_the_target() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let mut manifest = verify_shape(&body).unwrap();
        let lamad = manifest.applies_to.roles["lamad"].clone();
        manifest
            .applies_to
            .roles
            .insert("imagodei".to_string(), lamad.clone());

        let installed = Answer::Present(InstalledReality {
            app_id: "elohim".to_string(),
            roles: [
                (
                    "lamad".to_string(),
                    InstalledRole {
                        role: "lamad".to_string(),
                        dna_hash: lamad.dna_hash.clone(),
                        coordinator_zomes: [("content_store".to_string(), TARGET_WASM.to_string())]
                            .into_iter()
                            .collect(),
                        constitution_root: None,
                    },
                ),
                (
                    "imagodei".to_string(),
                    InstalledRole {
                        role: "imagodei".to_string(),
                        dna_hash: lamad.dna_hash.clone(),
                        coordinator_zomes: [("identity".to_string(), THIRD_WASM.to_string())]
                            .into_iter()
                            .collect(),
                        constitution_root: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        });

        let target: Answer<TargetCoordinators> = Answer::Present(
            [
                (
                    "lamad".to_string(),
                    [("content_store".to_string(), TARGET_WASM.to_string())]
                        .into_iter()
                        .collect::<BTreeMap<String, String>>(),
                ),
                (
                    "imagodei".to_string(),
                    [("identity".to_string(), TARGET_WASM.to_string())]
                        .into_iter()
                        .collect::<BTreeMap<String, String>>(),
                ),
            ]
            .into_iter()
            .collect(),
        );

        assert!(
            !already_runs_target(&manifest, &installed, &target),
            "one drifted role means the release still has work to do"
        );
    }

    #[test]
    fn a_release_speaking_no_epoch_this_build_speaks_is_refused() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let mut manifest = verify_shape(&body).unwrap();
        manifest.envelope.wire_epochs = vec![97, 98];
        let installed = installed_from(
            "lamad",
            &manifest.applies_to.roles["lamad"].dna_hash,
            "content_store",
            &manifest.applies_to.roles["lamad"].coordinator_wasm_hashes[0],
        );
        let refusal = verify_envelope(&manifest, &installed).expect_err("no common epoch");
        assert_eq!(refusal.reason_code(), RefusalReason::WireEpochUnsupported);
    }

    #[test]
    fn a_broken_additive_floor_is_refused_before_anything_else() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let mut manifest = verify_shape(&body).unwrap();
        manifest.envelope.additive_only = false;
        let refusal =
            verify_envelope(&manifest, &Answer::Unreachable).expect_err("floor break refuses");
        assert_eq!(refusal.reason_code(), RefusalReason::AdditiveFloorBroken);
        assert!(!refusal.transient);
    }

    /// **DoD arm 2 — lineage-hint mismatch.** The body field is a hint; the L2
    /// chain is the authority.
    #[test]
    fn a_lineage_hint_that_disagrees_with_the_l2_chain_is_refused() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let manifest = verify_shape(&body).unwrap();
        // The fixture declares a FIRST release (null parent). A chain that says
        // otherwise must refuse, not be overwritten by the body's claim.
        let chain = Answer::Present(LineageEvidence {
            supersedes: Some("uhCkkSomeEarlierRelease".to_string()),
        });
        let refusal = verify_lineage(&manifest, &chain).expect_err("hint must match the chain");
        assert_eq!(refusal.reason_code(), RefusalReason::LineageParentMismatch);
        assert!(!refusal.transient);

        // And the agreeing case passes, so the test is not just asserting that
        // lineage always refuses.
        verify_lineage(
            &manifest,
            &Answer::Present(LineageEvidence { supersedes: None }),
        )
        .expect("a first release with a null parent agrees with an empty chain");
    }

    /// The first-release defect, at the verify boundary: a fresh channel's
    /// first release declares a null parent, and the L2 chain evidence for a
    /// first release is ALSO `supersedes: None` — never the channel root's
    /// cid — so the two agree and the release is not refused.
    #[test]
    fn a_first_release_with_no_prior_release_evidence_passes() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let manifest = verify_shape(&body).unwrap();
        verify_lineage(
            &manifest,
            &Answer::Present(LineageEvidence { supersedes: None }),
        )
        .expect("a first release agrees with L2 evidence that names no prior release");
    }

    /// **C5.** An unreadable chain establishes nothing in either direction.
    #[test]
    fn an_unreadable_lineage_chain_is_unverifiable_never_a_pass() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let manifest = verify_shape(&body).unwrap();
        let refusal =
            verify_lineage(&manifest, &Answer::Unreachable).expect_err("unreadable refuses");
        assert_eq!(refusal.reason_code(), RefusalReason::LineageUnverifiable);
        assert!(refusal.transient);
    }

    /// **DoD arm 3 — threshold-unchecked.** An unreadable count is NOT a pass.
    #[test]
    fn an_unreadable_attestation_count_is_unchecked_never_a_pass() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let manifest = verify_shape(&body).unwrap();
        let refusal = verify_threshold(&manifest, None).expect_err("unchecked is not a pass");
        assert_eq!(refusal.reason_code(), RefusalReason::ThresholdUnchecked);
        assert!(refusal.transient, "a working reader cures this");
    }

    #[test]
    fn threshold_unmet_degraded_and_met_are_three_different_answers() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let manifest = verify_shape(&body).unwrap();

        let unmet = QualifyingEvidence {
            qualifying: 1,
            threshold: 2,
            total: 1,
            ..Default::default()
        };
        assert_eq!(
            verify_threshold(&manifest, Some(&unmet))
                .expect_err("1 < 2")
                .reason_code(),
            RefusalReason::ThresholdUnmet
        );

        let degraded = QualifyingEvidence {
            qualifying: 2,
            threshold: 2,
            total: 3,
            unresolved: 1,
            ..Default::default()
        };
        assert_eq!(
            verify_threshold(&manifest, Some(&degraded))
                .expect_err("a provably incomplete count never promotes")
                .reason_code(),
            RefusalReason::ThresholdEvidenceDegraded
        );

        let met = QualifyingEvidence {
            qualifying: 2,
            threshold: 2,
            total: 2,
            ..Default::default()
        };
        verify_threshold(&manifest, Some(&met)).expect("met is met");
    }

    #[test]
    fn artifact_length_and_digest_mismatches_are_different_stories() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let manifest = verify_shape(&body).unwrap();
        let declared = &manifest.artifacts[0];

        let missing = verify_artifacts(&manifest, &[]).expect_err("nothing served");
        assert_eq!(missing.reason_code(), RefusalReason::ArtifactUnavailable);
        assert!(missing.transient, "bytes may not have replicated yet");

        let wrong_len = verify_artifacts(
            &manifest,
            &[FetchedArtifact {
                blob_cid: declared.blob_cid.clone(),
                path: PathBuf::from("/tmp/a"),
                bytes: declared.bytes - 1,
                sha256: declared.sha256.clone(),
            }],
        )
        .expect_err("short read");
        assert_eq!(
            wrong_len.reason_code(),
            RefusalReason::ArtifactLengthMismatch
        );

        let wrong_digest = verify_artifacts(
            &manifest,
            &[FetchedArtifact {
                blob_cid: declared.blob_cid.clone(),
                path: PathBuf::from("/tmp/a"),
                bytes: declared.bytes,
                sha256: "0".repeat(64),
            }],
        )
        .expect_err("substituted bytes");
        assert_eq!(
            wrong_digest.reason_code(),
            RefusalReason::ArtifactDigestMismatch
        );

        let ok = verify_artifacts(
            &manifest,
            &[FetchedArtifact {
                blob_cid: declared.blob_cid.clone(),
                path: PathBuf::from("/tmp/a"),
                bytes: declared.bytes,
                sha256: declared.sha256.to_ascii_uppercase(),
            }],
        )
        .expect("digest comparison is case-insensitive hex");
        assert_eq!(ok, vec![PathBuf::from("/tmp/a")]);
    }

    /// A release resolved from the wrong channel is laundered.
    #[test]
    fn a_manifest_resolved_from_another_channel_is_refused() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let installed = Answer::Unreachable;
        let lineage = Answer::Present(LineageEvidence { supersedes: None });
        let refusal = verify(VerifyInput {
            channel_id: "runtime:coordinators:elohim:commons",
            release_cid: "uhCkkWhatever",
            body: &body,
            installed: &installed,
            lineage: &lineage,
            artifacts: &[],
            attestations: None,
            tier: HeadTier::Earned,
            target_coordinators: &Answer::Absent,
            path: Answer::Absent,
        })
        .expect_err("wrong channel");
        assert_eq!(refusal.reason_code(), RefusalReason::ChannelIdMismatch);
    }

    /// The whole floor, end to end, on a peer the release was cut for.
    #[test]
    fn the_composed_floor_mints_a_verified_release_only_when_every_arm_passes() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let manifest = verify_shape(&body).unwrap();
        let binding = &manifest.applies_to.roles["lamad"];
        let installed = installed_from(
            "lamad",
            &binding.dna_hash,
            "content_store",
            &binding.coordinator_wasm_hashes[0],
        );
        let lineage = Answer::Present(LineageEvidence { supersedes: None });
        let declared = &manifest.artifacts[0];
        let artifacts = vec![FetchedArtifact {
            blob_cid: declared.blob_cid.clone(),
            path: PathBuf::from("/var/lib/elohim/release-staging/x/content_store.wasm"),
            bytes: declared.bytes,
            sha256: declared.sha256.clone(),
        }];
        let evidence = QualifyingEvidence {
            qualifying: 2,
            threshold: 2,
            total: 2,
            ..Default::default()
        };
        let verified = verify(VerifyInput {
            channel_id: &manifest.channel_id,
            release_cid: "uhCkkTheWinningVersion",
            body: &body,
            installed: &installed,
            lineage: &lineage,
            artifacts: &artifacts,
            attestations: Some(&evidence),
            tier: HeadTier::Earned,
            target_coordinators: &Answer::Absent,
            path: Answer::Absent,
        })
        .expect("every arm passes");
        let verified = expect_verified(verified);
        assert_eq!(verified.channel_id, manifest.channel_id);
        assert_eq!(verified.release_cid, "uhCkkTheWinningVersion");
        assert_eq!(verified.artifact_paths.len(), manifest.artifacts.len());
    }

    /// **Design 2026-09-01 (canary-first adoption) — the defect this fixes.**
    /// An EARNED head still enforces the threshold through the full
    /// `verify()` composition (unchanged); a STAGING head with the exact
    /// same unmet evidence verifies anyway — the threshold gates PROMOTION,
    /// never staging adoption, so a canary in observe of its own soak must be
    /// able to verify a staging head with zero qualifying attestations.
    #[test]
    fn the_threshold_gates_earned_adoption_only_not_staging() {
        let body = fixture("release-manifest-coordinator-bundle.json");
        let manifest = verify_shape(&body).unwrap();
        let binding = &manifest.applies_to.roles["lamad"];
        let installed = installed_from(
            "lamad",
            &binding.dna_hash,
            "content_store",
            &binding.coordinator_wasm_hashes[0],
        );
        let lineage = Answer::Present(LineageEvidence { supersedes: None });
        let declared = &manifest.artifacts[0];
        let artifacts = vec![FetchedArtifact {
            blob_cid: declared.blob_cid.clone(),
            path: PathBuf::from("/var/lib/elohim/release-staging/x/content_store.wasm"),
            bytes: declared.bytes,
            sha256: declared.sha256.clone(),
        }];
        let unmet = QualifyingEvidence {
            qualifying: 0,
            threshold: 1,
            total: 0,
            ..Default::default()
        };

        let earned_refused = verify(VerifyInput {
            channel_id: &manifest.channel_id,
            release_cid: "uhCkkEarnedButUnmet",
            body: &body,
            installed: &installed,
            lineage: &lineage,
            artifacts: &artifacts,
            attestations: Some(&unmet),
            tier: HeadTier::Earned,
            target_coordinators: &Answer::Absent,
            path: Answer::Absent,
        })
        .expect_err("an EARNED head with an unmet threshold is still refused");
        assert_eq!(earned_refused.reason_code(), RefusalReason::ThresholdUnmet);

        let staging_verified = verify(VerifyInput {
            channel_id: &manifest.channel_id,
            release_cid: "uhCkkStagingUnmet",
            body: &body,
            installed: &installed,
            lineage: &lineage,
            artifacts: &artifacts,
            attestations: Some(&unmet),
            tier: HeadTier::Staging,
            target_coordinators: &Answer::Absent,
            path: Answer::Absent,
        })
        .expect("the SAME unmet evidence never refuses a STAGING head");
        assert_eq!(
            expect_verified(staging_verified).release_cid,
            "uhCkkStagingUnmet"
        );

        // And with no evidence at all (`threshold_unchecked` on a real read
        // failure): still refused on Earned, still verified on Staging.
        let earned_unchecked = verify(VerifyInput {
            channel_id: &manifest.channel_id,
            release_cid: "uhCkkEarnedUnchecked",
            body: &body,
            installed: &installed,
            lineage: &lineage,
            artifacts: &artifacts,
            attestations: None,
            tier: HeadTier::Earned,
            target_coordinators: &Answer::Absent,
            path: Answer::Absent,
        })
        .expect_err("unchecked is not a pass on an EARNED head");
        assert_eq!(
            earned_unchecked.reason_code(),
            RefusalReason::ThresholdUnchecked
        );

        verify(VerifyInput {
            channel_id: &manifest.channel_id,
            release_cid: "uhCkkStagingUnchecked",
            body: &body,
            installed: &installed,
            lineage: &lineage,
            artifacts: &artifacts,
            attestations: None,
            tier: HeadTier::Staging,
            target_coordinators: &Answer::Absent,
            path: Answer::Absent,
        })
        .expect("an unread threshold never gates a STAGING head either");
    }
}
