//! Envelope — the protocol-owned header of an EPR (spec §4.1).

use crate::{Coupling, EprKind, Reach, Signature};
use chrono::{DateTime, Utc};
use cid::Cid;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../sdk/epr-ts/src/generated/")]
pub struct Envelope {
    /// Self-derived content identifier. NOT included in canonical signing bytes.
    #[ts(type = "string")]
    pub cid: Cid,

    pub kind: EprKind,

    /// CID of the Manifest EPR that declares the payload schema.
    #[ts(type = "string")]
    pub schema_ref: Cid,

    /// Content-type key within the referenced manifest.
    pub schema_key: String,

    pub reach: Reach,

    pub coupling: Coupling,

    /// Outcome claims this EPR asserts.
    #[ts(type = "string[]")]
    pub claims: Vec<Cid>,

    /// Prior version if this is a revision.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[ts(type = "string | null", optional)]
    pub supersedes: Option<Cid>,

    /// Forward pointer; DERIVED from supersedence index, NOT in canonical bytes.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[ts(type = "string | null", optional)]
    pub superseded_by: Option<Cid>,

    /// UTC timestamp, included in canonical bytes.
    pub issued_at: DateTime<Utc>,

    /// Detached signature. NOT included in canonical signing bytes.
    pub proof: Signature,
}
