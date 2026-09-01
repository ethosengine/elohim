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
    AdoptionRefusal, Artifact, RefusalReason, ReleaseManifest, VerifiedRelease,
    RELEASE_MANIFEST_KIND,
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
    /// The release CID the channel's current head supersedes, from the head
    /// declaration itself — the L2 chain, not the manifest's self-report.
    /// `None` means the head declaration supersedes nothing (a first release).
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
        // `happ_manager::lineage_mismatch_error`'s refusal, moved to verify time.
        if binding.dna_hash != installed_role.dna_hash {
            return Err(refuse(
                RefusalReason::DnaLineageMismatch,
                format!(
                    "role '{role}': release binds DNA {} but this peer runs {} — crossing the \
                     DNA line is rung 6's migration ceremony, structurally refused here",
                    binding.dna_hash, installed_role.dna_hash
                ),
            ));
        }

        // For a coordinator-bundle release the declared hashes are what the
        // release SUPERSEDES — what it applies ONTO. A peer that does not run
        // them is not the peer this release was cut for.
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
                "envelope.lineageParentCid declares {declared:?} but the channel's head \
                 declaration supersedes {actual:?} — the body field is a hint that MUST match \
                 the L2 chain"
            ),
        ));
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
}

/// Run the whole floor. The ONLY constructor of a [`VerifiedRelease`].
///
/// Order matters for cost, not correctness: shape and channel identity are free
/// and reject the largest class of garbage before any installed-reality read.
pub fn verify(input: VerifyInput<'_>) -> Result<VerifiedRelease, AdoptionRefusal> {
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
    verify_threshold(&manifest, input.attestations)?;
    let artifact_paths = verify_artifacts(&manifest, input.artifacts)?;

    Ok(VerifiedRelease {
        channel_id: input.channel_id.to_string(),
        release_cid: input.release_cid.to_string(),
        manifest,
        artifact_paths,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_passport::{HappPassport, HappRolePassport};
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
            }],
            error: None,
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
            }],
            error: None,
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
        let refusal = verify_envelope(&manifest, &installed).expect_err("wrong generation");
        assert_eq!(
            refusal.reason_code(),
            RefusalReason::CoordinatorLineageMismatch
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
        })
        .expect("every arm passes");
        assert_eq!(verified.channel_id, manifest.channel_id);
        assert_eq!(verified.release_cid, "uhCkkTheWinningVersion");
        assert_eq!(verified.artifact_paths.len(), manifest.artifacts.len());
    }
}
