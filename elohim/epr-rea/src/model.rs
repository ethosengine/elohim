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

/// A **limit a commitment declares on itself**: the stock level this promise must stay the
/// safe side of, and the band edge at which the approach fires before it.
///
/// A limit is only ever a bound on a promise — which is why the algedonic `bound_ref` is the
/// bounding commitment's own CID rather than a separate atom's, and why this lives as a field
/// on [`Commitment`] instead of as a fourth record kind. [`Sense`] says which side is safe;
/// [`LimitSource`] says whether the number was claimed here or rolled up from contained
/// scopes.
///
/// `threshold_pct` is a **percentage** (`85.0` means 85%), never a fraction — the same unit the
/// wire evidence uses. [`Bound::new`] is the sole constructor and refuses anything below `1.0`
/// precisely because `0.85` is the fraction/percent confusion: read as a percent it puts the
/// band edge at 0.85% of the limit, so the first unit of stock that flows reads as pain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bound {
    /// The limit, denominated in [`Self::unit`]. Which side of it is safe is [`Self::sense`].
    pub limit: f64,
    /// What the limit measures — only `Magnitude::Count` flows in this unit accumulate
    /// against it.
    pub unit: String,
    /// Band edge as a percentage of the limit (`85.0` = 85%), in `[1, 100]`.
    pub threshold_pct: f64,
    /// Which side of the line is the safe side. `None` is [`Sense::Ceiling`] — the v1
    /// semantics, and the reason this is `Option` + skipped rather than a bare field: a
    /// ceiling bound authored before floors existed must keep byte-identical dag-cbor, or
    /// every bounded commitment is re-addressed. Same additive discipline as
    /// `Confidence.unknown_reason` (spec Q17).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sense: Option<Sense>,
    /// Where the number in [`Self::limit`] came from. `None` is [`LimitSource::Declared`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<LimitSource>,
}

/// Which side of a limit is the safe side.
///
/// **Ecological stewardship policy is overwhelmingly floor-shaped** — minimum instream flow,
/// minimum spawning biomass, soil carbon at or above a line, canopy cover at or above a line.
/// Before this, the fabric could express *"take no more than"* and could not express *"keep at
/// least"*: a floor had to be smuggled in as a ceiling on a complementary "depletion" stock,
/// which works arithmetically and destroys the reading — the `basis`, the `bound_ref`, and
/// every human-facing rendering then describe the wrong quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sense {
    /// Safe below; pain as the stock rises. `max_sustainable_yield`, an emissions cap.
    Ceiling,
    /// Safe above; pain as the stock falls. A minimum flow, a minimum viable population.
    Floor,
}

/// Where a limit's number came from — a claim, or a roll-up of the parts.
///
/// This is the distinction that separates an ecosystem from a factory, and the reason one
/// generic sum-fold cannot serve both. A watershed's sustainable yield is **declared at the
/// container**: it is emergent, and no summation over sub-catchments produces it. A factory's
/// throughput *is* a roll-up of its machines. Same word, opposite provenance — so a reader who
/// cannot tell which one a number is cannot know whether re-deriving it is even meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum LimitSource {
    /// A claim made at this scope by whoever promised it. Not derivable from the parts.
    Declared,
    /// A roll-up of the bounds of the scopes this one contains, by the named rule.
    Folded { rule: Composition },
}

/// How the bounds of contained scopes compose into their container's.
///
/// Not uniform across a [`crate::scope::Scopes`], which is exactly why it must be declared
/// per bound rather than defaulted — silently summing is how the shipped carrying-capacity
/// unit error happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Composition {
    /// Liebig's law of the minimum — the whole is bounded by its most-limiting part. A
    /// factory, a supply chain, a process with required inputs. Summing overstates the whole.
    Min,
    /// Worst-off-sensitive: one badly-served member cannot be averaged away. A community or
    /// cohort, where an arithmetic mean would let abundance for some hide scarcity for others.
    Harmonic,
    /// Genuinely additive parts — separate stores of the same fungible thing.
    Sum,
}

impl Composition {
    /// Compose contained scopes' limits into their container's.
    ///
    /// Refuses an empty input rather than returning a zero: no parts is not a capacity of
    /// nothing, it is no answer (the same honest-absence discipline as
    /// `FulfillmentStatus::ratio` and `Interval::unknown`). Refuses negatives, which are not
    /// capacities in any of the three rules.
    ///
    /// A zero part is *kept*, not refused: under [`Self::Harmonic`] a member with zero
    /// capacity correctly drives the whole to zero, which is the entire reason for choosing
    /// the harmonic mean over the arithmetic one.
    pub fn fold(self, parts: &[f64]) -> Result<f64> {
        if parts.is_empty() {
            return Err(FabricError::InvalidBound(
                "no contained scopes to compose — a folded limit with no parts is no answer, \
                 not a capacity of zero"
                    .into(),
            ));
        }
        if let Some(bad) = parts.iter().find(|p| !p.is_finite() || **p < 0.0) {
            return Err(FabricError::InvalidBound(format!(
                "a contained scope offered {bad} as a limit — not a capacity"
            )));
        }
        Ok(match self {
            Composition::Min => parts.iter().copied().fold(f64::INFINITY, f64::min),
            Composition::Sum => parts.iter().sum(),
            // 1/0.0 is +inf and n/inf is 0.0, so a zero part zeroes the whole — the
            // worst-off answer, arrived at by the arithmetic rather than special-cased.
            Composition::Harmonic => {
                let reciprocal_sum: f64 = parts.iter().map(|p| 1.0 / p).sum();
                parts.len() as f64 / reciprocal_sum
            }
        })
    }
}

impl Bound {
    /// Construct a bound, enforcing the unit and range rules. The only validating path —
    /// the fields are `pub` (the crate's existing convention), so [`Self::validate`] is
    /// re-checkable on a hand-built or deserialized value.
    pub fn new(limit: f64, unit: String, threshold_pct: f64) -> Result<Self> {
        let bound = Self {
            limit,
            unit,
            threshold_pct,
            sense: None,
            source: None,
        };
        bound.validate()?;
        Ok(bound)
    }

    /// Declare which side of the limit is safe.
    ///
    /// **Normalizes rather than stores**: [`Sense::Ceiling`] is encoded as `None`, because it
    /// is the v1 semantics every existing bound already means. One meaning must have exactly
    /// one encoding — two spellings of "ceiling" would give two CIDs to one promise, which is
    /// the duplication content addressing exists to prevent.
    pub fn with_sense(mut self, sense: Sense) -> Self {
        self.sense = match sense {
            Sense::Ceiling => None,
            Sense::Floor => Some(Sense::Floor),
        };
        self
    }

    /// Declare where this limit's number came from. Normalizes for the same reason as
    /// [`Self::with_sense`]: [`LimitSource::Declared`] is the v1 meaning and encodes as `None`.
    pub fn with_source(mut self, source: LimitSource) -> Self {
        self.source = match source {
            LimitSource::Declared => None,
            folded @ LimitSource::Folded { .. } => Some(folded),
        };
        self
    }

    /// Which side of the limit is safe. Undeclared means [`Sense::Ceiling`].
    pub fn sense(&self) -> Sense {
        self.sense.unwrap_or(Sense::Ceiling)
    }

    /// Where the limit came from. Undeclared means [`LimitSource::Declared`].
    pub fn source(&self) -> LimitSource {
        self.source.unwrap_or(LimitSource::Declared)
    }

    /// Has `stock` crossed the limit itself?
    pub fn breached_by(&self, stock: f64) -> bool {
        match self.sense() {
            Sense::Ceiling => stock >= self.limit,
            Sense::Floor => stock <= self.limit,
        }
    }

    /// Has `stock` reached the band edge — close to the limit, on the unsafe side of the
    /// margin, but not yet across?
    pub fn approached_by(&self, stock: f64) -> bool {
        match self.sense() {
            Sense::Ceiling => stock >= self.band_edge(),
            Sense::Floor => stock <= self.band_edge(),
        }
    }

    /// Re-check a bound's rules. Never panics — callers decide what to do with a rejection.
    pub fn validate(&self) -> Result<()> {
        if !self.limit.is_finite() || self.limit <= 0.0 {
            return Err(FabricError::InvalidBound(format!(
                "limit must be a finite positive quantity, got {}",
                self.limit
            )));
        }
        // One meaning, one encoding — otherwise a ceiling authored two ways gives one promise
        // two CIDs. `None` is the canonical spelling of the v1 defaults; the explicit
        // redundant variants are refused rather than silently normalized on decode, so a
        // producer minting them learns immediately instead of drifting.
        if self.sense == Some(Sense::Ceiling) {
            return Err(FabricError::InvalidBound(
                "encode a ceiling as an absent `sense` (the v1 default), not Some(Ceiling) — \
                 two spellings of one meaning give one promise two CIDs"
                    .into(),
            ));
        }
        if self.source == Some(LimitSource::Declared) {
            return Err(FabricError::InvalidBound(
                "encode a declared limit as an absent `source` (the v1 default), not \
                 Some(Declared) — two spellings of one meaning give one promise two CIDs"
                    .into(),
            ));
        }
        if self.unit.trim().is_empty() {
            return Err(FabricError::InvalidBound(
                "unit must name what the limit measures".into(),
            ));
        }
        if !self.threshold_pct.is_finite() || !(1.0..=100.0).contains(&self.threshold_pct) {
            return Err(FabricError::InvalidBound(format!(
                "threshold_pct is a percent of the limit in [1, 100] (85.0 means 85%), got {} \
                 — anything below 1.0 is the fraction/percent confusion, not a band edge",
                self.threshold_pct
            )));
        }
        Ok(())
    }

    /// The stock level at which the approach fires.
    ///
    /// `threshold_pct` is a **margin**, and the margin is mirrored around the limit so a floor
    /// and a ceiling with the same percentage warn with the same headroom:
    /// - ceiling: `limit × pct/100` — 85% of a limit of 100 is 85, warning 15% *below*.
    /// - floor:   `limit × (200-pct)/100` — the same 15% margin *above*, so 100 warns at 115.
    ///
    /// The alternative — reusing `limit × pct/100` for a floor — would put a floor's warning
    /// *below* the floor, firing only after the breach it exists to precede.
    pub fn band_edge(&self) -> f64 {
        match self.sense() {
            Sense::Ceiling => self.limit * self.threshold_pct / 100.0,
            Sense::Floor => self.limit * (200.0 - self.threshold_pct) / 100.0,
        }
    }
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
    /// The ceiling this promise declares on itself, if any. `None` is an **unbounded
    /// promise**: no bound, no pain (honest absence — see [`crate::fold::fulfillment`]).
    ///
    /// Skipped when absent so that declaring this vocabulary did not move a single existing
    /// commitment's CID — pinned by `an_unbounded_commitment_keeps_its_pre_bound_cid`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound: Option<Bound>,
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
    /// What KIND of observation this is, when the event's own verb and unit cannot say.
    ///
    /// [`ResourceSpec::classified_as`] carries this for [`Intent`] and [`Commitment`]; an event
    /// has no resource spec, so a run-scale observation (`run:failed-approach | run:correction |
    /// run:observation`) had no carrier at all. The established two-slot convention applies —
    /// **tag first, subject second** (`["gap:<state>", item.id]`, `["a2o:scenario-green", rel]`,
    /// read back positionally by `fulfill`) — and slots beyond the second are the authoring
    /// leg's, prefixed so a reader can tell them from a tag.
    ///
    /// Skipped when empty so that declaring this vocabulary did not move a single existing
    /// event's CID — the verbatim discipline [`Commitment::bound`] uses, pinned here by
    /// `an_unclassified_event_keeps_its_pre_classified_cid`. That matters more for events than
    /// for commitments: every sidecar line re-verifies its stored CID on read
    /// ([`crate::store::SidecarFlowStore::records`]), so a moved encoding would turn the whole
    /// append-only log into an integrity error rather than a quiet re-address.
    ///
    /// The rejected alternative was smuggling the tag into `Magnitude::Count{unit}`, which is a
    /// FOLD KEY — `crate::stock::count_in` filters on it by exact string match, so a
    /// classification hidden there would silently partition every stock fold that names a unit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classified_as: Vec<String>,
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

    // ── Bound: the ceiling a commitment declares on itself ───────────────────────

    /// The fixture whose CID the golden below pins — a commitment carrying NO bound.
    fn unbounded_commitment() -> Commitment {
        Commitment {
            action: ReaVerb::Produce,
            provider: agent("claude"),
            receiver: agent("operator"),
            resource_spec: ResourceSpec {
                classified_as: vec!["dev:joint".into()],
                quantity: Some(Magnitude::Count {
                    value: 6.0,
                    unit: "joint".into(),
                }),
            },
            in_scope_of: upstream_cid("epic"),
            valid_from: None,
            valid_until: None,
            state: CommitmentState::Active,
            satisfies: vec![],
            bound: None,
        }
    }

    #[test]
    fn bound_band_edge_is_a_percentage_of_the_limit() {
        let bound = Bound::new(1000.0, "token".into(), 85.0).expect("valid bound");
        assert_eq!(bound.band_edge(), 850.0);

        // 100% is legal and puts the band edge on the limit itself.
        let full = Bound::new(1000.0, "token".into(), 100.0).expect("100% is a valid band edge");
        assert_eq!(full.band_edge(), 1000.0);
    }

    /// `0.85` is the fraction/percent confusion: read as a percent it puts the band edge at
    /// 0.85% of the limit, so the first token spent is "pain". The sole constructor refuses it.
    #[test]
    fn bound_new_refuses_a_fraction_masquerading_as_a_percent() {
        let err = Bound::new(1000.0, "token".into(), 0.85)
            .expect_err("0.85 must be refused as a fraction, not accepted as 0.85%");
        assert!(
            err.to_string().contains("percent"),
            "the refusal must name the unit; got: {err}"
        );
        assert!(matches!(err, FabricError::InvalidBound(_)));
    }

    #[test]
    fn bound_new_refuses_an_out_of_range_or_non_finite_threshold() {
        for bad in [0.0, -5.0, 100.5, f64::NAN, f64::INFINITY] {
            assert!(
                Bound::new(1000.0, "token".into(), bad).is_err(),
                "threshold_pct {bad} must be refused"
            );
        }
        Bound::new(1000.0, "token".into(), 1.0).expect("1% is the lowest honest band edge");
    }

    #[test]
    fn bound_new_refuses_a_ceiling_that_is_not_a_finite_positive_quantity() {
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(
                Bound::new(bad, "token".into(), 85.0).is_err(),
                "limit {bad} must be refused"
            );
        }
        assert!(
            Bound::new(1000.0, "  ".into(), 85.0).is_err(),
            "a limit measures something — the unit must be named"
        );
    }

    /// Declaring the bound vocabulary must not have moved any existing commitment: the field
    /// is skipped when absent, so an unbounded commitment's canonical bytes are unchanged.
    /// The golden was computed against the pre-bound `Commitment` struct.
    #[test]
    fn an_unbounded_commitment_keeps_its_pre_bound_cid() {
        let cid = atom_cid(&unbounded_commitment()).expect("cid");
        assert_eq!(
            cid.to_string(),
            "bafyreib77zpf3g4daj54622eqjmkao7cfbv4n27kylnekoous4atg5h52q"
        );
    }

    // ── FlowEvent classification, and the address-stability it must not cost ─────

    /// The fixture whose CID the golden below pins — an event carrying NO classification.
    /// Shaped like a `run:` note (`Cite`, unit `run-note`, empty `fulfills`) so the pin sits on
    /// exactly the record the note leg mints.
    fn unclassified_event() -> FlowEvent {
        FlowEvent {
            action: ReaVerb::Cite,
            provider: agent("claude"),
            receiver: agent("repo"),
            resource: upstream_cid("golden-note-target"),
            quantity: Magnitude::Count {
                value: 1.0,
                unit: "run-note".into(),
            },
            process: None,
            in_scope_of: upstream_cid("epic"),
            fulfills: vec![],
            satisfies: vec![],
            classified_as: vec![],
            occurred_at: "2026-08-13T00:00:00Z".into(),
        }
    }

    /// Declaring the `classified_as` vocabulary must not have moved any existing event: the
    /// field is skipped when empty, so an unclassified event's canonical bytes are unchanged.
    /// The golden was computed against the pre-`classified_as` `FlowEvent` struct — the sibling
    /// of `an_unbounded_commitment_keeps_its_pre_bound_cid`, and the reason 4,866 already-
    /// appended sidecar records still re-verify their stored CIDs on read.
    #[test]
    fn an_unclassified_event_keeps_its_pre_classified_cid() {
        let cid = atom_cid(&unclassified_event()).expect("cid");
        assert_eq!(
            cid.to_string(),
            "bafyreibhg2ciu4hborzezeg4thgluq4vskdm7nzyhzkdk4eokcnqd6slve"
        );
        // Byte-level, not just address-level: an absent classification must not appear in the
        // canonical dag-cbor at all (the `sense`/`source` discipline, asserted the same way).
        let bytes = canonical_bytes(&unclassified_event()).expect("encodes");
        assert!(
            !bytes.windows(12).any(|w| w == b"classifiedAs"),
            "an empty classification must not appear in the canonical bytes at all"
        );
    }

    /// …but a classification IS part of the observation, so declaring one re-addresses it —
    /// which is exactly what makes two notes with different reasons two distinct records
    /// rather than one CID-deduped no-op.
    #[test]
    fn classifying_an_event_changes_its_cid() {
        let mut classified = unclassified_event();
        classified.classified_as = vec!["run:correction".into(), "genesis/plan.md".into()];
        assert!(canonical_bytes(&classified)
            .expect("encodes")
            .windows(12)
            .any(|w| w == b"classifiedAs"));
        assert_ne!(
            atom_cid(&unclassified_event()).unwrap(),
            atom_cid(&classified).unwrap()
        );
    }

    /// …but a bound IS part of the promise, so declaring one is a different commitment.
    #[test]
    fn declaring_a_bound_changes_the_commitment_cid() {
        let mut bounded = unbounded_commitment();
        bounded.bound = Some(Bound::new(1000.0, "token".into(), 85.0).expect("valid bound"));
        assert_ne!(
            atom_cid(&unbounded_commitment()).unwrap(),
            atom_cid(&bounded).unwrap()
        );
    }

    // ── sense, source, and the address-stability they must not cost ──────────────

    /// The whole reason `sense`/`source` are `Option` + skipped. If a ceiling bound's bytes
    /// moved, every bounded commitment ever authored would be re-addressed.
    #[test]
    fn adding_sense_and_source_moves_no_existing_ceiling_bounds_bytes() {
        let ceiling = Bound::new(1000.0, "token".into(), 85.0).expect("valid");
        let bytes = canonical_bytes(&ceiling).expect("encodes");
        let window = bytes.windows(5).any(|w| w == b"sense");
        assert!(
            !window,
            "an undeclared sense must not appear in the canonical bytes at all"
        );
        assert!(
            !bytes.windows(6).any(|w| w == b"source"),
            "an undeclared source must not appear in the canonical bytes at all"
        );
        // …and a floor is a genuinely different promise, so it SHOULD re-address.
        let floor = Bound::new(1000.0, "token".into(), 85.0)
            .expect("valid")
            .with_sense(Sense::Floor);
        assert!(canonical_bytes(&floor)
            .expect("encodes")
            .windows(5)
            .any(|w| w == b"sense"));
        assert_ne!(atom_cid(&ceiling).unwrap(), atom_cid(&floor).unwrap());
    }

    #[test]
    fn declaring_the_v1_default_explicitly_is_refused_so_one_meaning_has_one_encoding() {
        let mut b = Bound::new(1000.0, "token".into(), 85.0).expect("valid");
        b.sense = Some(Sense::Ceiling);
        assert!(
            b.validate().is_err(),
            "Some(Ceiling) and None mean the same thing — two CIDs for one promise"
        );
        let mut b = Bound::new(1000.0, "token".into(), 85.0).expect("valid");
        b.source = Some(LimitSource::Declared);
        assert!(b.validate().is_err());
        // The builders normalize rather than producing the refused shape.
        let normalized = Bound::new(1000.0, "token".into(), 85.0)
            .expect("valid")
            .with_sense(Sense::Ceiling)
            .with_source(LimitSource::Declared);
        assert_eq!(normalized.sense, None);
        assert_eq!(normalized.source, None);
        assert!(normalized.validate().is_ok());
    }

    /// A floor's warning must fire *above* the line it protects. Reusing the ceiling formula
    /// would put it below, so it could only fire after the breach it exists to precede.
    #[test]
    fn a_floors_band_edge_mirrors_the_margin_instead_of_sitting_under_the_floor() {
        let ceiling = Bound::new(100.0, "af".into(), 85.0).expect("valid");
        let floor = Bound::new(100.0, "af".into(), 85.0)
            .expect("valid")
            .with_sense(Sense::Floor);

        assert_eq!(ceiling.band_edge(), 85.0, "15% of headroom below the limit");
        assert_eq!(floor.band_edge(), 115.0, "the same 15% margin, above it");
        assert!(floor.band_edge() > floor.limit);

        // Ceiling: pain as the stock rises.
        assert!(ceiling.approached_by(90.0) && !ceiling.breached_by(90.0));
        assert!(ceiling.breached_by(100.0));
        assert!(!ceiling.approached_by(10.0));

        // Floor: pain as the stock falls — the exact inversion.
        assert!(floor.approached_by(110.0) && !floor.breached_by(110.0));
        assert!(floor.breached_by(100.0));
        assert!(
            !floor.approached_by(200.0),
            "a stock far above its floor is not in pain"
        );
    }

    // ── composition: the four §3 rules, and what each refuses ────────────────────

    #[test]
    fn min_is_liebig_and_sum_would_overstate_the_whole() {
        let parts = [10.0, 3.0, 7.0];
        assert_eq!(Composition::Min.fold(&parts).unwrap(), 3.0);
        assert_eq!(Composition::Sum.fold(&parts).unwrap(), 20.0);
        assert!(
            Composition::Min.fold(&parts).unwrap() < Composition::Sum.fold(&parts).unwrap(),
            "summing machine throughputs overstates a factory bounded by its slowest input"
        );
    }

    #[test]
    fn harmonic_refuses_to_average_away_a_badly_served_member() {
        let parts = [100.0, 100.0, 1.0];
        let arithmetic = parts.iter().sum::<f64>() / parts.len() as f64;
        let harmonic = Composition::Harmonic.fold(&parts).unwrap();
        assert!(
            harmonic < 3.0 && arithmetic > 60.0,
            "harmonic {harmonic} must stay near the worst-served member while the arithmetic \
             mean {arithmetic} hides them behind the well-served"
        );
        // A member with nothing drives the whole to nothing — the point of choosing harmonic.
        assert_eq!(Composition::Harmonic.fold(&[10.0, 0.0]).unwrap(), 0.0);
    }

    #[test]
    fn composing_no_parts_is_no_answer_rather_than_a_capacity_of_zero() {
        for rule in [Composition::Min, Composition::Harmonic, Composition::Sum] {
            assert!(
                rule.fold(&[]).is_err(),
                "{rule:?} over an empty set must refuse, not return 0.0 — honest absence"
            );
            assert!(
                rule.fold(&[5.0, -1.0]).is_err(),
                "a negative is not a capacity"
            );
        }
    }

    /// Q4, answered in the type: a reader can tell a claim from a roll-up.
    #[test]
    fn a_declared_limit_is_distinguishable_from_a_folded_one() {
        let watershed = Bound::new(500.0, "ML/year".into(), 85.0).expect("valid");
        let factory = Bound::new(500.0, "units/day".into(), 85.0)
            .expect("valid")
            .with_source(LimitSource::Folded {
                rule: Composition::Min,
            });
        assert_eq!(watershed.source(), LimitSource::Declared);
        assert_eq!(
            factory.source(),
            LimitSource::Folded {
                rule: Composition::Min
            }
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
