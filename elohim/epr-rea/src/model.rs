//! VF-named payload structs. All are canonical dag-cbor atoms: identity is the CID of
//! the canonical bytes ([`atom_cid`]), so `fulfills`/`satisfies`/`in_scope_of` edges are
//! tamper-evident by construction (restating an edge changes your own CID).

use cid::Cid;
use elohim_epr::witness::{Magnitude, ReaVerb};
use serde::{Deserialize, Serialize};

use crate::error::{FabricError, Result};

/// Canonical dag-cbor bytes of any fabric atom.
pub fn canonical_bytes<T: Serialize>(atom: &T) -> Result<Vec<u8>> {
    serde_ipld_dagcbor::to_vec(atom).map_err(|e| FabricError::Encode(e.to_string()))
}

/// The CID of a fabric atom: CIDv1 dag-cbor over sha2-256, same mint as the epr codec.
pub fn atom_cid<T: Serialize>(atom: &T) -> Result<Cid> {
    Ok(elohim_epr::cid::compute_cid(&canonical_bytes(atom)?))
}

/// Canonical agent identity (`uhCAk…`) — a Holochain hash string, NOT a CID.
/// Transport identities (libp2p peer id, iroh NodeId) resolve TO this; never the reverse.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentRef(pub String);

/// A declared pin: `id@version`. Which version applies is a DECLARED dependency, never
/// recency (the versioned-entity head principle).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedRef {
    pub id: String,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageSpec {
    pub name: String,
    /// Pattern naming what artifacts this stage holds (schema_ref / glob).
    pub artifact_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorRef {
    pub id: String,
}

/// A conversion edge in a recipe: `from` stage's outputs feed `to` stage.
/// `meaningful: true` marks an economically meaningful joint — events are expected at
/// this crossing. Which edges are meaningful is GOVERNED (an `.epr-meta`-bound policy),
/// not hardcoded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeSpec {
    pub from: String,
    pub to: String,
    pub validators: Vec<ValidatorRef>,
    pub meaningful: bool,
}

/// Knowledge level: a process definition (VF ProcessSpecification / recipe).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSpec {
    pub id: String,
    pub version: u32,
    pub stages: Vec<StageSpec>,
    pub edges: Vec<EdgeSpec>,
}

/// What a flow wants/promises to move: classification + optional expected quantity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSpec {
    pub classified_as: Vec<String>,
    pub quantity: Option<Magnitude>,
}

/// Plan level: a forward-looking desired flow (VF Intent).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Intent {
    pub action: ReaVerb,
    pub resource_spec: ResourceSpec,
    /// The container EPR accountable for this flow (VF `in_scope_of`, made structural).
    pub in_scope_of: Cid,
    pub raised_by: AgentRef,
}

/// Mirrors the runtime rails' lifecycle (`proposed → active`, Slice-2a graduation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommitmentState {
    Proposed,
    Active,
    Fulfilled,
    Revoked,
}

/// Plan level: a promised flow (VF Commitment). Graduated home: `Mishpat::Commitment`
/// (cid = entry_hash — never action_hash).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Commitment {
    pub action: ReaVerb,
    pub provider: AgentRef,
    pub receiver: AgentRef,
    pub resource_spec: ResourceSpec,
    pub in_scope_of: Cid,
    /// RFC3339; None = unbounded.
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    pub state: CommitmentState,
    /// Intent(s) this commitment answers (VF Satisfaction — an edge, not an atom).
    pub satisfies: Vec<Cid>,
}

/// Observation level: what actually happened (granular floor of VF EconomicEvent;
/// crystallizes to the DHT `EconomicEvent` per recipe-declared graduation policy).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowEvent {
    pub action: ReaVerb,
    pub provider: AgentRef,
    pub receiver: AgentRef,
    /// The resource flowed — any content-addressed atom IS a resource.
    pub resource: Cid,
    pub quantity: Magnitude,
    /// The process instance this event belongs to, if any.
    pub process: Option<Cid>,
    pub in_scope_of: Cid,
    /// Commitment(s) this event discharges (VF Fulfillment — the DHT spells it `bounded_by`).
    pub fulfills: Vec<Cid>,
    pub satisfies: Vec<Cid>,
    /// RFC3339.
    pub occurred_at: String,
}

/// A live run of a recipe grouping events (VF Process): consumes/uses `inputs`,
/// produces `outputs` — the conversion duality.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Process {
    pub spec: PinnedRef,
    pub in_scope_of: Cid,
    pub inputs: Vec<Cid>,
    pub outputs: Vec<Cid>,
}
