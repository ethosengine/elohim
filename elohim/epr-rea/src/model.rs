//! VF-named payload structs. All are canonical dag-cbor atoms: identity is the CID of
//! the canonical bytes ([`atom_cid`]), so `fulfills`/`satisfies`/`in_scope_of` edges are
//! tamper-evident by construction (restating an edge changes your own CID).

use cid::Cid;
use elohim_epr::witness::{Magnitude, ReaVerb};
use multihash_codetable::{Code, MultihashDigest};
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

/// The conformance mechanism a [`DepEdge`] carries — exactly one per edge (spec §2). Every
/// variant except [`Governor::CiteSeal`] is a citation of a STRONGER system already enforcing
/// the edge elsewhere (a compiler, a codegen pipeline, a schema contract test, a named test) —
/// those edges are `Governed` and NEVER enter the derived stale set. Only `CiteSeal` carries a
/// `sealed_cid` and can go stale (the seal-aware walk, gap #2, recomputes and compares it).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Governor {
    /// Enforced by a compiler/typechecker; payload names the unit (crate, tsconfig project…).
    Compiler(String),
    /// Enforced by a codegen pipeline; payload names the pipeline.
    Codegen(String),
    /// Enforced by a schema-contract test; payload names the test.
    SchemaContract(String),
    /// Enforced by a named test.
    Test(String),
    /// No stronger external system — the edge's own claim is the conformance mechanism,
    /// sealed against an upstream CID and re-verified by recomputing it.
    CiteSeal,
}

/// The only DECLARED edge state — staleness is always derived (recompute vs `sealed_cid`),
/// never stored as truth (spec §2). `superseded_by` optionally points at the record (its CID)
/// that resolves the hold, once one exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EdgeStatus {
    Held {
        reason: String,
        valid_from: i64,
        superseded_by: Option<Cid>,
    },
}

/// A sealed dependency edge: downstream `from` conforms to upstream `to` under `governor`.
/// Source of truth: the `.eprfs/status/` sidecar (local observation floor, append-only; B2 —
/// graduates to the existing Attestation entry type at push, gap #11). Never a DHT write here.
/// `from`/`to` are repo-relative paths (v1 floor — slug identity for code artifacts is an open
/// question tracked in the spec, not resolved by this record).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepEdge {
    pub from: String,
    pub to: String,
    /// The announcement — why this edge exists.
    pub desc: Option<String>,
    pub governor: Governor,
    /// Full CIDv1 raw of the upstream body at conformance time. `Some` iff
    /// `governor == Governor::CiteSeal` — enforced at CONSTRUCTION by [`DepEdge::new`]
    /// (which calls [`DepEdge::validate`]). That is the only enforcement point: a value
    /// built via this struct's public fields (a hand-rolled literal, a deserialized sidecar
    /// line) can still violate the invariant, so read paths that fold stored records — e.g.
    /// `FlowStore::edges()` — re-check `validate()` per record and SKIP a failing one rather
    /// than erroring the whole read or trusting the invariant blindly.
    pub sealed_cid: Option<Cid>,
    pub sealed_by: AgentRef,
    /// git/appended timestamp — never a wall-clock read in this lib.
    pub sealed_at: i64,
    /// `None` = healthy claim at seal time.
    pub status: Option<EdgeStatus>,
}

impl DepEdge {
    /// Construct a `DepEdge`, enforcing the invariant
    /// `sealed_cid.is_some() ⇔ governor == Governor::CiteSeal`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        from: String,
        to: String,
        desc: Option<String>,
        governor: Governor,
        sealed_cid: Option<Cid>,
        sealed_by: AgentRef,
        sealed_at: i64,
        status: Option<EdgeStatus>,
    ) -> Result<Self> {
        let edge = Self {
            from,
            to,
            desc,
            governor,
            sealed_cid,
            sealed_by,
            sealed_at,
            status,
        };
        edge.validate()?;
        Ok(edge)
    }

    /// Re-check the seal invariant on an already-built value (e.g. one deserialized from the
    /// sidecar). Never panics — callers decide what to do with a rejected edge.
    pub fn validate(&self) -> Result<()> {
        let is_cite_seal = matches!(self.governor, Governor::CiteSeal);
        if is_cite_seal != self.sealed_cid.is_some() {
            return Err(FabricError::InvalidEdge(format!(
                "sealed_cid.is_some()={} must equal governor==CiteSeal={} (from={:?} to={:?})",
                self.sealed_cid.is_some(),
                is_cite_seal,
                self.from,
                self.to
            )));
        }
        Ok(())
    }
}

/// Index/dedup key for a `(from, to)` edge slot: first 12 hex chars of sha2-256 over the
/// literal bytes `"{from}|{to}"`. Deliberately NOT a content address — `edge_fp` hashes the
/// PATH PAIR (an index key over an append-only log so reseal/hold can find "the same slot"),
/// never the sealed payload; it must never be used where an address/fingerprint is expected.
/// Reuses the same sha2-256 primitive as `elohim_epr::cid::compute_cid` (via
/// `multihash-codetable`), not a fresh hashing implementation.
pub fn edge_fp(from: &str, to: &str) -> String {
    let key = format!("{from}|{to}");
    let digest = Code::Sha2_256.digest(key.as_bytes());
    let mut out = String::with_capacity(12);
    for byte in digest.digest().iter().take(6) {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(name: &str) -> AgentRef {
        AgentRef(format!("uhCAk-test-{name}"))
    }

    /// A fixed CID standing in for "the upstream body at seal time" — built the same way
    /// the crate's own tests do (`atom_cid` of a labeled atom), never hand-parsed.
    fn upstream_cid(label: &str) -> Cid {
        atom_cid(&label.to_string()).expect("cid")
    }

    fn fixed_edge() -> DepEdge {
        DepEdge::new(
            "app/elohim-app/src/app/lamad/services/content.service.ts".into(),
            "genesis/docs/superpowers/specs/2026-07-21-sealed-contract-edges-governor-frontier-design.md".into(),
            Some("content.service mirrors the frontier verdict shape".into()),
            Governor::CiteSeal,
            Some(upstream_cid("golden-upstream-body")),
            agent("claude"),
            1_753_084_800,
            None,
        )
        .expect("fixed golden edge must be valid")
    }

    // ── Invariant enforcement ────────────────────────────────────────────────────

    #[test]
    fn constructor_rejects_sealed_cid_on_non_citeseal_governor() {
        let err = DepEdge::new(
            "a".into(),
            "b".into(),
            None,
            Governor::Compiler("tsc".into()),
            Some(upstream_cid("b")),
            agent("claude"),
            1,
            None,
        )
        .expect_err("a Compiler governor must never carry a sealed_cid");
        assert!(matches!(err, FabricError::InvalidEdge(_)));
    }

    #[test]
    fn constructor_rejects_missing_sealed_cid_on_citeseal_governor() {
        let err = DepEdge::new(
            "a".into(),
            "b".into(),
            None,
            Governor::CiteSeal,
            None,
            agent("claude"),
            1,
            None,
        )
        .expect_err("CiteSeal must always carry a sealed_cid");
        assert!(matches!(err, FabricError::InvalidEdge(_)));
    }

    #[test]
    fn constructor_accepts_governed_edge_with_no_seal() {
        let edge = DepEdge::new(
            "a".into(),
            "b".into(),
            None,
            Governor::Test("cargo test export_bindings".into()),
            None,
            agent("claude"),
            1,
            None,
        )
        .expect("a non-CiteSeal governor with no sealed_cid is valid");
        assert!(edge.sealed_cid.is_none());
    }

    #[test]
    fn constructor_accepts_citeseal_edge_with_seal() {
        let edge = fixed_edge();
        assert_eq!(edge.governor, Governor::CiteSeal);
        assert!(edge.sealed_cid.is_some());
    }

    // ── CID stability golden ─────────────────────────────────────────────────────
    // Pinned so canonical dag-cbor encoding of `DepEdge` can never silently drift.

    #[test]
    fn depedge_cid_is_stable() {
        let cid = atom_cid(&fixed_edge()).expect("cid");
        assert_eq!(
            cid.to_string(),
            "bafyreigyvgxxtwtsowy7j2nstjaapzfjcwhvixp5o4a4v3xlsxym7x7wzi"
        );
    }

    // ── edge_fp golden ───────────────────────────────────────────────────────────

    #[test]
    fn edge_fp_is_golden_and_order_sensitive() {
        assert_eq!(edge_fp("a", "b"), "0eab8a0a3380");
        // Never symmetric — it hashes the literal ordered "from|to" pair.
        assert_ne!(edge_fp("a", "b"), edge_fp("b", "a"));
        // Deterministic across calls.
        assert_eq!(edge_fp("a", "b"), edge_fp("a", "b"));
    }
}
