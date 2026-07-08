use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::address::{BlobCid, EprRef};

/// Standard attestation classes emitted around projected files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AttestationKind {
    Build,
    Validation,
    Materialization,
    Custody,
    Read,
    Write,
}

/// Unsigned attestation intent produced by a projection consumer.
///
/// The storage layer or domain adapter is responsible for adding protocol
/// signatures, authority checks, and publication.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttestationDraft {
    pub kind: AttestationKind,
    pub subject: EprRef,
    pub blob: Option<BlobCid>,
    pub agent_id: String,
    pub payload: Value,
}
