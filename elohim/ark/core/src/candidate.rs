//! Non-actuating candidate authentication. This checks signed CONTENT, not a grant.
//!
//! C12 remains unbound: a future actor must verify the notarized delegation's
//! author, recipient, scope, validity and revocation before any process effect.
//! The independently provisioned issuer pin is a bootstrap scaffold; graduation
//! requires that grant proof at the acting node. This module has no process API.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use cid::Cid;
use elohim_epr::{Epr, EprKind};
use serde::{Deserialize, Serialize};

/// Domain discriminator; unknown versions cannot silently gain this meaning.
pub const CANDIDATE_SCHEMA_KEY: &str = "ark-candidate-v1";
/// Maximum payload bytes; envelope variable fields have separate fixed caps.
pub const MAX_CANDIDATE_BYTES: usize = 16 * 1024;

/// Independently provisioned issuer and schema pin, never taken from a request.
#[derive(Clone, Debug)]
pub struct CandidatePin {
    /// Agent EPR CID, not a Holochain agent key.
    pub signer: Cid,
    /// Public key associated with that Agent EPR by local provisioning.
    pub public_key: [u8; 32],
    /// Exact schema admitted by local provisioning.
    pub schema: Cid,
}

/// Runtime identity and declared format support, independent of version labels.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CandidateRuntime {
    /// DAG-CBOR runtime manifest identity.
    pub manifest: Cid,
    /// Raw-codec executable identity.
    pub executable: Cid,
    /// Informational only; never used to infer compatibility.
    pub version: String,
    /// Exact target triple.
    pub platform: String,
    /// Durable database format this executable writes.
    pub writes_format: String,
    /// Explicitly supported database read formats.
    pub reads_formats: BTreeSet<String>,
    /// Protocol capabilities this runtime provides.
    pub capabilities: BTreeSet<String>,
}

/// Signed content reconstructable from release, grant and installed observations.
/// It is not a recovery journal or disposable fallback material.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CandidateContent {
    /// Existing runtime channel slug.
    pub channel: String,
    /// Elected Holochain release ActionHash (not an EPR CID).
    pub release_action: String,
    /// Referenced grant EntryHash; its authority is NOT checked here.
    pub grant_entry: String,
    /// Exact grant authoring action; entry identity alone does not bind an author.
    pub grant_action: String,
    /// Independently provisioned berth identity.
    pub berth: Cid,
    /// Placement generation, never an election clock.
    pub incarnation: u64,
    /// Named child; no siblings are implied.
    pub child: String,
    /// Expected incumbent manifest/executable and compatibility declaration.
    pub predecessor: CandidateRuntime,
    /// Proposed candidate identity and compatibility declaration.
    pub candidate: CandidateRuntime,
    /// Required protocol capabilities; independent local requirements also apply.
    pub required_capabilities: BTreeSet<String>,
    /// Earliest accepted time.
    pub valid_from: DateTime<Utc>,
    /// Exclusive expiry.
    pub valid_until: DateTime<Utc>,
}

/// Independently observed local state. Missing observations must not be invented.
#[derive(Clone, Debug)]
pub struct CandidateObservation {
    /// Expected elected channel.
    pub channel: String,
    /// Exact elected release action.
    pub release_action: String,
    /// Exact grant reference selected independently; still not proof of authority.
    pub grant_entry: String,
    /// Exact grant authoring action; entry identity alone does not bind an author.
    pub grant_action: String,
    /// Target berth.
    pub berth: Cid,
    /// Current placement incarnation.
    pub incarnation: u64,
    /// Expected child.
    pub child: String,
    /// Current runtime and format support.
    pub installed: CandidateRuntime,
    /// Actual current durable database format.
    pub database_format: String,
    /// Locally required capabilities, independent of the publisher.
    pub required_capabilities: BTreeSet<String>,
    /// Raw CID computed over separately observed candidate bytes by the caller.
    pub observed_executable: Cid,
    /// Candidate runtime manifest independently resolved from the elected release.
    pub candidate_manifest: Cid,
    /// Explicit observation time; this pure module reads no clock.
    pub now: DateTime<Utc>,
}

/// Authenticated candidate CONTENT only. No conversion to activation authority exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthenticatedCandidateContent {
    cid: Cid,
    content: CandidateContent,
}

impl AuthenticatedCandidateContent {
    /// Identity of the signed content.
    pub fn cid(&self) -> Cid {
        self.cid
    }
    /// Read-only content, not an authorization to mutate processes.
    pub fn content(&self) -> &CandidateContent {
        &self.content
    }
}

/// Stable refusal vocabulary. Consumers must count refusals beside success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateRefusal {
    /// Input exceeds the bounded work contract.
    TooLarge,
    /// Wrong domain, schema or malformed data/identity.
    InvalidContent,
    /// Issuer identity is not independently pinned.
    WrongIssuer,
    /// Canonical CID or detached proof does not verify.
    InvalidProof,
    /// Outside the signed validity window.
    InvalidTime,
    /// Wrong release, grant or target.
    WrongTarget,
    /// Installed predecessor differs.
    StalePredecessor,
    /// Different execution platform.
    WrongPlatform,
    /// Observed bytes differ from signed candidate identity.
    ArtifactMismatch,
    /// Candidate cannot provide a required capability.
    MissingCapability,
    /// Durable formats cannot support executable rollback.
    DatabaseIncompatible,
}

impl seam_contracts::ReasonLabel for CandidateRefusal {
    const ALL: &'static [Self] = &[
        Self::TooLarge,
        Self::InvalidContent,
        Self::WrongIssuer,
        Self::InvalidProof,
        Self::InvalidTime,
        Self::WrongTarget,
        Self::StalePredecessor,
        Self::WrongPlatform,
        Self::ArtifactMismatch,
        Self::MissingCapability,
        Self::DatabaseIncompatible,
    ];
    fn label(&self) -> &'static str {
        match self {
            Self::TooLarge => "candidate_too_large",
            Self::InvalidContent => "candidate_invalid_content",
            Self::WrongIssuer => "candidate_wrong_issuer",
            Self::InvalidProof => "candidate_invalid_proof",
            Self::InvalidTime => "candidate_invalid_time",
            Self::WrongTarget => "candidate_wrong_target",
            Self::StalePredecessor => "candidate_stale_predecessor",
            Self::WrongPlatform => "candidate_wrong_platform",
            Self::ArtifactMismatch => "candidate_artifact_mismatch",
            Self::MissingCapability => "candidate_missing_capability",
            Self::DatabaseIncompatible => "candidate_database_incompatible",
        }
    }
}

/// Authenticate signed content against independently supplied expectations.
/// Success deliberately confers NO process permission or notarization claim.
pub fn authenticate_candidate(
    epr: &Epr,
    pin: &CandidatePin,
    observed: &CandidateObservation,
) -> Result<AuthenticatedCandidateContent, CandidateRefusal> {
    use CandidateRefusal::*;
    // Bound payload and each variable-size envelope field before crypto.
    if epr.payload.len() > MAX_CANDIDATE_BYTES
        || epr.envelope.claims.len() > 64
        || epr.envelope.schema_key.len() > 128
        || epr.envelope.proof.signature.len() > 64
        || epr.envelope.proof.algorithm.len() > 32
    {
        return Err(TooLarge);
    }
    if epr.envelope.kind != EprKind::Content
        || epr.envelope.schema_key != CANDIDATE_SCHEMA_KEY
        || epr.envelope.schema_ref != pin.schema
    {
        return Err(InvalidContent);
    }
    if epr.envelope.proof.signer != pin.signer {
        return Err(WrongIssuer);
    }
    // Existing implementation re-derives canonical bytes, CID and detached proof.
    epr.verify_with_key(&pin.public_key)
        .map_err(|_| InvalidProof)?;
    let content: CandidateContent =
        serde_json::from_slice(&epr.payload).map_err(|_| InvalidContent)?;
    // Lexical screening only: does not prove Holochain hash checksum or provenance.
    // Exact comparison to independently resolved identities below is mandatory.
    let hash_shape = |s: &str, prefix: &str| {
        s.len() == 53
            && s.starts_with(prefix)
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    };
    let cid_shape = |c: &Cid, codec| {
        c.version() == cid::Version::V1
            && c.codec() == codec
            && c.hash().code() == 0x12
            && c.hash().size() == 32
    };
    if !hash_shape(&content.release_action, "uhCkk")
        || !hash_shape(&content.grant_entry, "uhCEk")
        || !hash_shape(&content.grant_action, "uhCkk")
        || !content.channel.starts_with("runtime:")
        || content.channel.split(':').count() != 4
        || content.channel.split(':').any(|p| {
            p.is_empty()
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
        || content.child.is_empty()
        || !cid_shape(&content.berth, 0x71)
        || [&content.predecessor, &content.candidate].iter().any(|r| {
            !cid_shape(&r.manifest, 0x71)
                || !cid_shape(&r.executable, 0x55)
                || r.platform.is_empty()
                || r.writes_format.is_empty()
        })
    {
        return Err(InvalidContent);
    }
    if content.valid_from > observed.now
        || content.valid_until <= observed.now
        || content.valid_from >= content.valid_until
        || epr.envelope.issued_at > observed.now
    {
        return Err(InvalidTime);
    }
    if content.channel != observed.channel
        || content.release_action != observed.release_action
        || content.grant_entry != observed.grant_entry
        || content.grant_action != observed.grant_action
        || content.berth != observed.berth
        || content.child != observed.child
        || content.incarnation != observed.incarnation
        || content.candidate.manifest != observed.candidate_manifest
    {
        return Err(WrongTarget);
    }
    let mut expected_predecessor = content.predecessor.clone();
    // Version labels are presentation, never compatibility or freshness evidence.
    expected_predecessor
        .version
        .clone_from(&observed.installed.version);
    if expected_predecessor != observed.installed {
        return Err(StalePredecessor);
    }
    if content.candidate.platform != observed.installed.platform {
        return Err(WrongPlatform);
    }
    if content.candidate.executable != observed.observed_executable {
        return Err(ArtifactMismatch);
    }
    if !content
        .required_capabilities
        .is_subset(&content.candidate.capabilities)
        || !observed
            .required_capabilities
            .is_subset(&content.candidate.capabilities)
    {
        return Err(MissingCapability);
    }
    let format = &observed.database_format;
    if format.is_empty()
        || [&content.predecessor, &content.candidate]
            .iter()
            .any(|r| &r.writes_format != format || !r.reads_formats.contains(format))
    {
        return Err(DatabaseIncompatible);
    }
    Ok(AuthenticatedCandidateContent {
        cid: epr.envelope.cid,
        content,
    })
}
