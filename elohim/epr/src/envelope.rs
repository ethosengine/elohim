//! Envelope — the protocol-owned header of an EPR (spec §4.1).

use crate::cbor;
use crate::error::Result;
use crate::{Coupling, EprKind, Reach, Signature};
use chrono::{DateTime, Utc};
use cid::Cid;
use ipld_core::ipld::Ipld;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
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

impl Envelope {
    /// Compute the canonical bytes that get hashed for CID and signed for proof.
    ///
    /// Includes (in alphabetical map order): claims, coupling, issuedAt, kind,
    /// payload, reach, schemaKey, schemaRef, supersedes.
    /// Excludes: cid, proof, supersededBy.
    ///
    /// See spec §4.3 (canonical serialization) and §4.6 (supersededBy exclusion).
    pub fn canonical_bytes(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut map: BTreeMap<String, Ipld> = BTreeMap::new();

        map.insert(
            "claims".into(),
            Ipld::List(self.claims.iter().map(|c| Ipld::Link(*c)).collect()),
        );
        map.insert("coupling".into(), coupling_ipld(&self.coupling));
        map.insert(
            "issuedAt".into(),
            Ipld::String(
                self.issued_at
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            ),
        );
        map.insert("kind".into(), Ipld::String(kind_canonical(self.kind)));
        map.insert("payload".into(), Ipld::Bytes(payload.to_vec()));
        map.insert("reach".into(), Ipld::String(reach_canonical(self.reach)));
        map.insert("schemaKey".into(), Ipld::String(self.schema_key.clone()));
        map.insert("schemaRef".into(), Ipld::Link(self.schema_ref));
        if let Some(s) = self.supersedes {
            map.insert("supersedes".into(), Ipld::Link(s));
        }

        cbor::encode(&Ipld::Map(map))
    }
}

fn coupling_ipld(c: &Coupling) -> Ipld {
    let mut m: BTreeMap<String, Ipld> = BTreeMap::new();
    if let Some(k) = c.knowledge {
        m.insert("knowledge".into(), Ipld::Link(k));
    }
    if let Some(v) = c.value {
        m.insert("value".into(), Ipld::Link(v));
    }
    if let Some(g) = c.governance {
        m.insert("governance".into(), Ipld::Link(g));
    }
    Ipld::Map(m)
}

fn kind_canonical(k: EprKind) -> String {
    match k {
        EprKind::Content => "Content",
        EprKind::Agent => "Agent",
        EprKind::Manifest => "Manifest",
        EprKind::Claim => "Claim",
        EprKind::Observation => "Observation",
        EprKind::EconomicEvent => "EconomicEvent",
        EprKind::Commitment => "Commitment",
        EprKind::Attestation => "Attestation",
        EprKind::Delegation => "Delegation",
    }
    .into()
}

fn reach_canonical(r: Reach) -> String {
    match r {
        Reach::Private => "private",
        Reach::SelfScope => "self",
        Reach::Intimate => "intimate",
        Reach::Trusted => "trusted",
        Reach::Familiar => "familiar",
        Reach::Community => "community",
        Reach::Public => "public",
        Reach::Commons => "commons",
    }
    .into()
}
