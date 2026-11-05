//! Derivation by identity (interfaces spec §C): the types and ports, slice 4.
//!
//! A derivation is keyed by (method, view-projected input root), so a change outside the method's
//! declared view never invalidates its record, and two peers that ran the same deterministic
//! method on the same input share one output. The record is evidence, never authority (spec
//! §12.5): the reader judges it through its own [`TrustPolicy`], and an accepted record whose
//! output bytes cannot be fetched is a miss.
//!
//! This module carries the types and ports plus [`project_view`], the constructor for sparse
//! projected roots. [`ProjectedRoot::verify_whole_tree`] is the narrow identity projection for a
//! verified whole tree; it cannot stand in for `project_view` on a narrower declared view.

use std::collections::BTreeMap;

use async_trait::async_trait;
use bytes::Bytes;
use seam_contracts::{ComeBack, Meanwhile, Unanswered};
use serde::de::{Deserializer, IgnoredAny};
use serde::{Deserialize, Serialize};

use crate::address::{BlobCid, EprRef};
use crate::attestation::{AttestationDraft, AttestationKind};
use crate::composition::{DerivationEdge, DerivationKind};
use crate::error::{EprfsError, Result};
use crate::reach::{ReachRefusal, Requester};
use crate::storage::{BatchItem, EprfsStorage};
use crate::view_decl::TreeView;
use crate::walk::{Coverage, SpineWalk, TreePath, View};
use crate::{TreeEntry, TreeEntryKind, TreeNode};

/// The canonical DAG-CBOR descriptor of everything that changes the output except the input
/// bytes (C4, C12, C15). Its CID is the method CID. `view` is the method's declared input set (for
/// a gate, the step's `inputs.sources`), so the key cannot be computed under another view and a
/// change inside any declared input moves the projected root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MethodDescriptor {
    /// e.g. `brit-gate`, `a2o-measure`, `recall-fold`.
    pub name: String,
    /// tool → version.
    pub tools: BTreeMap<String, String>,
    pub flags: Vec<String>,
    /// The parameter set the method runs with, if any.
    pub params: Option<BlobCid>,
    /// CID of the child spec (artifact, argv, env), when it runs a process.
    pub exec: Option<BlobCid>,
    /// brit's step name.
    pub step: Option<String>,
    /// recall's embedder.
    pub embedder: Option<String>,
    /// The declared input paths (no root), normalized as `TreeView`'s.
    pub view: Vec<TreePath>,
    /// Declared identities with no path (a DNA hash, an `A2O_SUT_*` value).
    pub non_path_inputs: BTreeMap<String, String>,
    /// Bound to the method CID; a record's flag must equal it.
    pub deterministic: bool,
}

impl MethodDescriptor {
    /// The method CID: the DAG-CBOR CID of the descriptor with its view normalized, so two
    /// spellings of one declared input set name one method.
    pub fn cid(&self) -> Result<BlobCid> {
        let mut canonical = self.clone();
        canonical.view = self.view().paths().cloned().collect();
        let bytes = serde_ipld_dagcbor::to_vec(&canonical)
            .map_err(|error| EprfsError::Storage(format!("method descriptor encoding: {error}")))?;
        Ok(BlobCid::compute(&bytes))
    }

    pub fn view(&self) -> View {
        View::from_tree_paths(self.view.iter().cloned())
    }
}

/// A view-projected input root. A whole tree has an identity projection, available through
/// [`Self::verify_whole_tree`]. Build narrower projections with [`project_view`].
/// Deserialization retains the existing record wire contract; a decoded value is a writer's
/// claim, not evidence that this reader verified a closure.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProjectedRoot(BlobCid);

impl ProjectedRoot {
    pub fn cid(&self) -> &BlobCid {
        &self.0
    }

    /// Verify an entire Tree v1 closure before using its root as a projected key.
    /// Refuse empty/narrow views before fetching: their projection is NOT the original root.
    /// Opaque external boundaries cannot form a complete native input. This establishes byte
    /// identity only; it neither authorizes execution nor creates reusable derivation evidence.
    /// The storage adapter remains responsible for bounded fetch allocation and duration.
    pub async fn verify_whole_tree<S: EprfsStorage + ?Sized>(
        storage: &S,
        root: &BlobCid,
        view: &View,
        policy: crate::FetchPolicy,
        limits: &crate::TreeLimits,
    ) -> crate::tree::TreeResult<Self> {
        if view != &View::whole() {
            return Err(crate::TreeError::InvalidView(
                "whole-tree projection requires the whole view".into(),
            ));
        }
        let verified = crate::load_verified_tree(storage, root, policy, limits).await?;
        if !verified.external_paths.is_empty() {
            return Err(crate::TreeError::InvalidView(
                "whole-tree projection cannot include external boundaries".into(),
            ));
        }
        Ok(Self(root.clone()))
    }
}

/// Re-encode the verified spine as the sparse tree named by its view.
///
/// In-view directory links and every kept leaf or external entry remain byte-for-byte the entry
/// the source tree carried. Only spine nodes are re-encoded. A spine directory whose projected
/// child is empty disappears from its parent; the root itself remains an empty tree when no view
/// path exists in the source. The returned nodes are ordered deepest-first, ending at the
/// projected root, so a caller can store them in one pass.
pub fn project_view(
    spine: &SpineWalk,
    limits: &crate::TreeLimits,
) -> crate::tree::TreeResult<(ProjectedRoot, Vec<(BlobCid, Bytes)>)> {
    if spine
        .nodes
        .first()
        .is_none_or(|node| !node.path.is_root() || node.cid != spine.root)
    {
        return Err(crate::TreeError::InvalidView(
            "a spine walk must begin with its root node".into(),
        ));
    }

    let mut projected: BTreeMap<TreePath, Option<BlobCid>> = BTreeMap::new();
    let mut encoded = Vec::with_capacity(spine.nodes.len());
    let mut total_bytes = 0usize;
    for walked in spine.nodes.iter().rev() {
        if walked.node.entries.len() != walked.entry_coverage.len() {
            return Err(crate::TreeError::InvalidView(
                "a spine node's entry coverage is incomplete".into(),
            ));
        }
        let mut entries = Vec::new();
        for (entry, coverage) in walked.node.entries.iter().zip(&walked.entry_coverage) {
            match coverage {
                Coverage::Outside => {}
                Coverage::InView => entries.push(entry.clone()),
                Coverage::Spine => match &entry.kind {
                    TreeEntryKind::Directory { .. } => {
                        let child_path = walked.path.join(&entry.name);
                        match projected.get(&child_path) {
                            Some(Some(child)) => entries.push(TreeEntry {
                                name: entry.name.clone(),
                                kind: TreeEntryKind::Directory {
                                    tree: child.clone().into(),
                                },
                            }),
                            Some(None) => {}
                            None => {
                                return Err(crate::TreeError::InvalidView(format!(
                                    "spine walk has no node for {child_path}"
                                )))
                            }
                        }
                    }
                    TreeEntryKind::File { .. }
                    | TreeEntryKind::Symlink { .. }
                    | TreeEntryKind::External { .. } => entries.push(entry.clone()),
                },
            }
        }

        if entries.is_empty() && !walked.path.is_root() {
            projected.insert(walked.path.clone(), None);
            continue;
        }
        let node = TreeNode::new(entries)?;
        let (cid, bytes) = crate::encode_tree(&node, limits)?;
        total_bytes = total_bytes
            .checked_add(bytes.len())
            .ok_or(crate::TreeError::Budget("projected node bytes"))?;
        if total_bytes > limits.max_total_bytes {
            return Err(crate::TreeError::Budget("projected node bytes"));
        }
        projected.insert(walked.path.clone(), Some(cid.clone()));
        encoded.push((cid, Bytes::from(bytes)));
    }

    let root = projected
        .remove(&TreePath::root())
        .flatten()
        .ok_or_else(|| crate::TreeError::InvalidView("projection produced no root".into()))?;
    Ok((ProjectedRoot(root), encoded))
}

/// What a derivation key's input is. `Raw` is a CID that is not a tree root (a wasm module).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "form", content = "cid")]
pub enum KeyInput {
    Projected(ProjectedRoot),
    Raw(BlobCid),
}

impl KeyInput {
    pub fn cid(&self) -> &BlobCid {
        match self {
            KeyInput::Projected(root) => root.cid(),
            KeyInput::Raw(cid) => cid,
        }
    }
}

/// (method CID, input). The storage key, never the reach key: reach is judged on the input root
/// (C1).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivationKey {
    pub method: BlobCid,
    pub input: KeyInput,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DerivationKeyWire {
    method: BlobCid,
    input: KeyInput,
    /// D7: extensions inside the namespace carry no decision meaning. The enclosing retained body
    /// preserves their exact bytes; the key decoder deliberately does not interpret them.
    #[serde(default)]
    ext: Option<IgnoredAny>,
}

impl<'de> Deserialize<'de> for DerivationKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let wire = DerivationKeyWire::deserialize(deserializer)?;
        let _ = wire.ext;
        Ok(Self {
            method: wire.method,
            input: wire.input,
        })
    }
}

/// What the WRITER observed (C3). Agreement is not a recorded basis: the reader computes it from
/// distinct authenticated records with equal outputs and judges independence from its own roster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "basis"
)]
#[non_exhaustive]
pub enum DerivationBasis {
    /// Computed by `computed_by`; "here" is the reader's comparison.
    Computed,
    /// Carried for its requester under that grant.
    UnderGrant { grant: String },
    /// Recomputed the other record's output; a mismatch names what it refutes (C10).
    Recomputed {
        checked: BlobCid,
        against: String,
        matched: bool,
    },
    /// A newer basis; a trust policy never accepts it (C11).
    #[serde(other)]
    Unknown,
}

/// Authenticates `computed_by` (C3, C16): a signature by that agent over the record's canonical
/// bytes, or, once notarized, the attestation's reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "witness"
)]
pub enum Witness {
    Signature { sig: String },
    Attestation { epr: EprRef },
}

/// Injected authentication for a newly computed derivation record. The core passes the exact D7
/// canonical body bytes and never reaches for a key, signer, filesystem or process itself (R-E2).
#[async_trait]
pub trait RecordWitness: Send + Sync {
    /// The agent key this witness signs as; becomes [`DerivationRecord::computed_by`].
    fn agent(&self) -> &str;

    /// Witness over the record's canonical body bytes (the bytes a
    /// [`RetainedDerivationRecord`] retains), never over a re-serialization.
    async fn witness(&self, body: &[u8]) -> Result<Witness>;
}

/// Records travel between peers on different versions. D7 accepts additions only under `ext`;
/// fields outside that namespace may carry meaning and therefore refuse in the current epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivationRecord {
    pub key: DerivationKey,
    pub output: BlobCid,
    /// The agent key; the `composedBy` role.
    pub computed_by: String,
    pub basis: DerivationBasis,
    /// Must equal the descriptor's; an index rejects a mismatch (C15).
    pub deterministic: bool,
    pub witness: Witness,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DerivationRecordWire {
    key: DerivationKey,
    output: BlobCid,
    computed_by: String,
    basis: DerivationBasis,
    deterministic: bool,
    witness: Witness,
    #[serde(default)]
    ext: Option<IgnoredAny>,
}

/// D7's signed body. Authentication is framing around these bytes, never a field inside them.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DerivationRecordBodyWire {
    key: DerivationKey,
    output: BlobCid,
    computed_by: String,
    basis: DerivationBasis,
    deterministic: bool,
    #[serde(default)]
    ext: Option<IgnoredAny>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CanonicalDerivationRecordBody<'a> {
    key: &'a DerivationKey,
    output: &'a BlobCid,
    computed_by: &'a str,
    basis: &'a DerivationBasis,
    deterministic: bool,
}

impl<'de> Deserialize<'de> for DerivationRecord {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let wire = DerivationRecordWire::deserialize(deserializer)?;
        let _ = wire.ext;
        Ok(Self {
            key: wire.key,
            output: wire.output,
            computed_by: wire.computed_by,
            basis: wire.basis,
            deterministic: wire.deterministic,
            witness: wire.witness,
        })
    }
}

/// One known-epoch record plus the exact body bytes it arrived with. D7 extensions are retained
/// here without interpretation or re-encoding; a signature verifier uses [`Self::body`] directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedDerivationRecord {
    record: DerivationRecord,
    body: Bytes,
}

impl RetainedDerivationRecord {
    pub fn record(&self) -> &DerivationRecord {
        &self.record
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn into_parts(self) -> (DerivationRecord, Bytes) {
        (self.record, self.body)
    }
}

impl DerivationRecord {
    /// Canonical D7 body bytes. The witness is deliberately outside this encoding, so signing is
    /// neither circular nor dependent on a later serialization of the logical record.
    pub fn canonical_body(&self) -> Result<Bytes> {
        encode_record_body(
            &self.key,
            &self.output,
            &self.computed_by,
            &self.basis,
            self.deterministic,
        )
    }

    /// Decode one record body in the current epoch under D7's three-case law.
    ///
    /// The five body fields decide the current record. Additions are accepted only beneath the
    /// top-level `ext` namespace, whose exact representation remains in the returned body bytes.
    /// Any other top-level field is refused because an older reader cannot know whether it narrows
    /// authority. Epoch selection happens in the signed framing layer before this decoder.
    pub fn decode_known_epoch(body: &[u8], witness: Witness) -> Result<RetainedDerivationRecord> {
        let wire: DerivationRecordBodyWire = serde_ipld_dagcbor::from_slice(body)
            .map_err(|error| EprfsError::Storage(format!("derivation record: {error}")))?;
        let _ = wire.ext;
        Ok(RetainedDerivationRecord {
            record: DerivationRecord {
                key: wire.key,
                output: wire.output,
                computed_by: wire.computed_by,
                basis: wire.basis,
                deterministic: wire.deterministic,
                witness,
            },
            body: Bytes::copy_from_slice(body),
        })
    }

    /// Projects onto the existing edge: source = input, derived = output, attributed_to =
    /// computed_by, relation = [`DerivationKind::Derivation`], metadata = `{method}`.
    pub fn to_edge(&self) -> DerivationEdge {
        DerivationEdge {
            source: self.key.input.cid().clone(),
            derived: self.output.clone(),
            relation: DerivationKind::Derivation,
            attributed_to: self.computed_by.clone(),
            metadata: serde_json::json!({ "method": self.key.method.to_string() }),
        }
    }

    /// A Notarized (A) attestation draft for the existing `publish_attestation` path. It takes an
    /// [`AdoptedHead`], which a draft does not have: drafts' derivations stay ephemeral and do not
    /// attest (spec §14, C7).
    pub fn to_attestation(&self, adopted: &AdoptedHead) -> Result<AttestationDraft> {
        Ok(AttestationDraft {
            kind: AttestationKind::Derivation,
            subject: EprRef::new(adopted.head.clone()),
            blob: Some(self.output.clone()),
            agent_id: self.computed_by.clone(),
            payload: serde_json::json!({
                "record": serde_json::to_value(self)?,
                "adoptedRoot": adopted.root.to_string(),
            }),
        })
    }
}

fn encode_record_body(
    key: &DerivationKey,
    output: &BlobCid,
    computed_by: &str,
    basis: &DerivationBasis,
    deterministic: bool,
) -> Result<Bytes> {
    let body = CanonicalDerivationRecordBody {
        key,
        output,
        computed_by,
        basis,
        deterministic,
    };
    serde_ipld_dagcbor::to_vec(&body)
        .map(Bytes::from)
        .map_err(|error| EprfsError::Storage(format!("derivation record body: {error}")))
}

/// A declared head and the root it adopted; drafts have none. Plain data, built by the caller from
/// the head plane (slug → {root, reach}); `head` is the head's slug.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptedHead {
    pub head: String,
    pub root: BlobCid,
}

/// Spec §14: one corpus digest per adopted head per method (C7). Named by its CID, reached through
/// the head, which is not cached; the bundle itself is immutable once named.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivationBundle {
    pub head: String,
    pub root: BlobCid,
    pub method: BlobCid,
    pub records: Vec<DerivationRecord>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LenientBundle {
    head: String,
    root: BlobCid,
    method: BlobCid,
    records: Vec<serde_json::Value>,
}

impl DerivationBundle {
    /// Decodes record by record: an undecodable record is skipped and counted, never failing the
    /// bundle (C11). Records hold no tag-42 links (every CID is a string), so each one decodes
    /// through a self-describing value first.
    pub fn decode_lenient(bytes: &[u8]) -> Result<(Self, usize)> {
        let wire: LenientBundle = serde_ipld_dagcbor::from_slice(bytes)
            .map_err(|error| EprfsError::Storage(format!("derivation bundle: {error}")))?;
        let mut skipped = 0;
        let mut records = Vec::with_capacity(wire.records.len());
        for value in wire.records {
            match serde_json::from_value::<DerivationRecord>(value) {
                Ok(record) => records.push(record),
                Err(_) => skipped += 1,
            }
        }
        Ok((
            Self {
                head: wire.head,
                root: wire.root,
                method: wire.method,
                records,
            },
            skipped,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivationLookup {
    Found(Vec<DerivationRecord>),
    NotRecorded,
}

/// R7's typed reason for seam C (C13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "reason"
)]
#[non_exhaustive]
pub enum DerivationUnanswered {
    NotRecorded,
    IndexUnreachable,
    Refused {
        refusal: ReachRefusal,
    },
    InputIncomplete {
        at: String,
    },
    OutputNotFetchable,
    Mismatch {
        outputs: Vec<BlobCid>,
    },
    /// Capacity did not admit a run here.
    NotCarried,
}

impl DerivationUnanswered {
    /// The wire tag of this variant, from an exhaustive match: what a fieldless label face maps.
    pub fn kind(&self) -> &'static str {
        match self {
            DerivationUnanswered::NotRecorded => "not-recorded",
            DerivationUnanswered::IndexUnreachable => "index-unreachable",
            DerivationUnanswered::Refused { .. } => "refused",
            DerivationUnanswered::InputIncomplete { .. } => "input-incomplete",
            DerivationUnanswered::OutputNotFetchable => "output-not-fetchable",
            DerivationUnanswered::Mismatch { .. } => "mismatch",
            DerivationUnanswered::NotCarried => "not-carried",
        }
    }

    /// One value of every variant, for label faces outside this crate.
    pub fn one_of_each() -> Vec<DerivationUnanswered> {
        vec![
            DerivationUnanswered::NotRecorded,
            DerivationUnanswered::IndexUnreachable,
            DerivationUnanswered::Refused {
                refusal: ReachRefusal::new(crate::reach::ReachReason::OutsideStanding, None),
            },
            DerivationUnanswered::InputIncomplete { at: "a".into() },
            DerivationUnanswered::OutputNotFetchable,
            DerivationUnanswered::Mismatch { outputs: vec![] },
            DerivationUnanswered::NotCarried,
        ]
    }
}

/// The look-ups return ruling 7's `Unanswered` by value as the interfaces spec signs them: a
/// derivation look-up is cold (once per task), so its error size is allowed.
#[async_trait]
#[allow(
    clippy::result_large_err,
    reason = "The interfaces contract returns Unanswered by value on this once-per-task lookup"
)]
pub trait DerivationIndex: Send + Sync {
    /// Carries what reach is judged on (C1). A serving index admits `who` to `at.root`, recomputes
    /// the projected root over its own nodes (a membership check), then answers the records keyed
    /// (method, projected root). On the requester side each index is gated with
    /// `AskPolicy::may_ask(tier, at.root, …)`, so the projected root, a hash of private content,
    /// never reaches an index the root's reach does not admit.
    async fn lookup(
        &self,
        who: &Requester,
        at: &TreeView,
        method: &MethodDescriptor,
    ) -> std::result::Result<DerivationLookup, Unanswered<DerivationUnanswered>>;

    /// One fetch answers K methods on one input (contract 6). Keyed by method CID.
    async fn lookup_many(
        &self,
        who: &Requester,
        at: &TreeView,
        methods: &[MethodDescriptor],
    ) -> std::result::Result<BTreeMap<BlobCid, DerivationLookup>, Unanswered<DerivationUnanswered>>;

    /// Idempotent on (key, output, computed_by). Records sit side by side and never overwrite.
    /// Refuses an unauthenticated record and one whose `deterministic` differs from its descriptor.
    async fn record(&self, record: &DerivationRecord) -> Result<()>;
}

/// The reader's judgement, never stored in the record.
#[allow(
    clippy::result_large_err,
    reason = "TrustPolicy preserves the interfaces contract's structured Unanswered refusal by value"
)]
pub trait TrustPolicy: Send + Sync {
    /// `records` are authenticated. `roster` is the reader's own view of failure domains.
    fn accept<'r>(
        &self,
        key: &DerivationKey,
        records: &'r [DerivationRecord],
        me: &str,
        roster: &dyn DomainRoster,
    ) -> std::result::Result<&'r DerivationRecord, Unanswered<DerivationUnanswered>>;
}

/// The reader's own view of which agents share a failure domain.
pub trait DomainRoster: Send + Sync {
    fn domain_of(&self, agent: &str) -> Option<String>;
}

#[async_trait]
pub trait Derive: Send + Sync {
    fn descriptor(&self) -> &MethodDescriptor;

    /// `prior`: the accepted record and output for a parent root, when the caller names one, so a
    /// history-shaped method costs only its change (spec §12.5; C9). `input_store` is the store
    /// the method reads `input`'s root and paths from; `output_store` is the store it writes the
    /// derived output's objects to. The two may be the same store. The returned CID is the output
    /// root in `output_store`.
    async fn derive(
        &self,
        input: &TreeView,
        input_store: &dyn EprfsStorage,
        output_store: &dyn EprfsStorage,
        prior: Option<&Derived>,
    ) -> Result<BlobCid>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Derived {
    pub output: BlobCid,
    pub record: DerivationRecord,
    pub reused: bool,
}

/// Why a computed output has no reusable record. This is an outcome rather than an error because
/// the method already produced and stored the output; hiding that successful computation would
/// make a failed witness or index write look like no work happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnrecordedReason {
    WitnessFailed { message: String },
    IndexFailed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecordedDerivation {
    pub output: BlobCid,
    /// Present when witnessing succeeded but the index refused or failed the write.
    pub record: Option<DerivationRecord>,
    pub reason: UnrecordedReason,
}

/// The spec's shape, unboxed: an outcome is returned once per task, so its size is allowed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::large_enum_variant,
    reason = "The interfaces contract returns one unboxed outcome per task"
)]
pub enum DeriveOutcome {
    Derived(Derived),
    ComputedUnrecorded(UnrecordedDerivation),
    NotDerived(Unanswered<DerivationUnanswered>),
}

fn derivation_unanswered(why: DerivationUnanswered) -> Unanswered<DerivationUnanswered> {
    match why {
        DerivationUnanswered::OutputNotFetchable => Unanswered::new(
            DerivationUnanswered::OutputNotFetchable,
            ComeBack::OnEvent {
                event: "derivation-output-fetchable".into(),
            },
            Meanwhile::RunHere,
        ),
        DerivationUnanswered::NotCarried => Unanswered::new(
            DerivationUnanswered::NotCarried,
            ComeBack::OnEvent {
                event: "capacity-admits-derivation".into(),
            },
            Meanwhile::Nothing {
                because: "this device did not admit the derivation run".into(),
            },
        ),
        other => Unanswered::new(
            other,
            ComeBack::OnEvent {
                event: "derivation-recorded".into(),
            },
            Meanwhile::RunHere,
        ),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "derive_or_reuse's explicit policy and storage seams pass through unchanged to its lookup step"
)]
async fn accepted_output(
    index: &dyn DerivationIndex,
    trust: &dyn TrustPolicy,
    descriptor: &MethodDescriptor,
    input: &TreeView,
    storage: &dyn EprfsStorage,
    who: &Requester,
    me: &str,
    roster: &dyn DomainRoster,
) -> std::result::Result<Option<Derived>, Unanswered<DerivationUnanswered>> {
    let key = DerivationKey {
        method: descriptor.cid().map_err(|_| {
            derivation_unanswered(DerivationUnanswered::InputIncomplete {
                at: "method descriptor".into(),
            })
        })?,
        input: KeyInput::Projected(ProjectedRoot(input.root().clone())),
    };
    let records = match index.lookup(who, input, descriptor).await? {
        DerivationLookup::Found(records) => records,
        DerivationLookup::NotRecorded => return Ok(None),
    };
    let accepted = trust.accept(&key, &records, me, roster)?.clone();
    let item = BatchItem::derived(accepted.output.clone(), &key.method);
    let fetched = storage
        .fetch_at(
            input.root(),
            &item,
            &crate::BatchOptions::from(crate::FetchPolicy::FetchIfMissing),
        )
        .await;
    match fetched {
        Ok(handle)
            if handle.cid == accepted.output && accepted.output.verifies(handle.bytes.as_ref()) =>
        {
            Ok(Some(Derived {
                output: accepted.output.clone(),
                record: accepted,
                reused: true,
            }))
        }
        Ok(_) | Err(_) => Err(derivation_unanswered(
            DerivationUnanswered::OutputNotFetchable,
        )),
    }
}

/// Reuse an accepted, fetchable output; otherwise compute only when capacity admits the run.
/// Newly computed records are authenticated through the injected [`RecordWitness`] port.
#[allow(
    clippy::too_many_arguments,
    reason = "The placement contract keeps every policy and storage seam explicit"
)]
pub async fn derive_or_reuse(
    index: &dyn DerivationIndex,
    trust: &dyn TrustPolicy,
    method: &dyn Derive,
    input_root: &BlobCid,
    storage: &dyn EprfsStorage,
    who: &Requester,
    witness: &dyn RecordWitness,
    roster: &dyn DomainRoster,
    parents: &[BlobCid],
    capacity_admits: bool,
) -> Result<DeriveOutcome> {
    let descriptor = method.descriptor();
    let input = TreeView::new(input_root.clone(), &descriptor.view());
    let miss = match accepted_output(
        index,
        trust,
        descriptor,
        &input,
        storage,
        who,
        witness.agent(),
        roster,
    )
    .await
    {
        Ok(Some(derived)) => return Ok(DeriveOutcome::Derived(derived)),
        Ok(None) => None,
        Err(why) => Some(why),
    };

    if !capacity_admits {
        return Ok(DeriveOutcome::NotDerived(miss.unwrap_or_else(|| {
            derivation_unanswered(DerivationUnanswered::NotCarried)
        })));
    }

    let mut prior = None;
    for parent in parents {
        let parent_input = TreeView::new(parent.clone(), &descriptor.view());
        if let Ok(Some(found)) = accepted_output(
            index,
            trust,
            descriptor,
            &parent_input,
            storage,
            who,
            witness.agent(),
            roster,
        )
        .await
        {
            prior = Some(found);
            break;
        }
    }

    let output = method
        .derive(&input, storage, storage, prior.as_ref())
        .await?;
    let key = DerivationKey {
        method: descriptor.cid()?,
        input: KeyInput::Projected(ProjectedRoot(input_root.clone())),
    };
    let basis = DerivationBasis::Computed;
    let body = encode_record_body(
        &key,
        &output,
        witness.agent(),
        &basis,
        descriptor.deterministic,
    )?;
    let authentication = match witness.witness(&body).await {
        Ok(authentication) => authentication,
        Err(error) => {
            return Ok(DeriveOutcome::ComputedUnrecorded(UnrecordedDerivation {
                output,
                record: None,
                reason: UnrecordedReason::WitnessFailed {
                    message: error.to_string(),
                },
            }));
        }
    };
    let record = DerivationRecord {
        key,
        output: output.clone(),
        computed_by: witness.agent().to_owned(),
        basis,
        deterministic: descriptor.deterministic,
        witness: authentication,
    };
    if let Err(error) = index.record(&record).await {
        return Ok(DeriveOutcome::ComputedUnrecorded(UnrecordedDerivation {
            output,
            record: Some(record),
            reason: UnrecordedReason::IndexFailed {
                message: error.to_string(),
            },
        }));
    }
    Ok(DeriveOutcome::Derived(Derived {
        output,
        record,
        reused: false,
    }))
}

const DERIVED_PREFIX: &[u8] = b"\0derived:";

impl TreePath {
    /// The reserved path a derivation output is fetched under (C2): one component containing NUL,
    /// which `valid_name` refuses, so it can never name a tree entry. It has no JSON form that
    /// decodes back (the slash form refuses NUL). Its binary form (DAG-CBOR: one byte-string
    /// component) decodes back through [`TreePath::parse_derived`], which accepts only the exact
    /// form this constructor writes.
    pub fn derived(method: &BlobCid) -> TreePath {
        let mut component = DERIVED_PREFIX.to_vec();
        component.extend_from_slice(method.to_string().as_bytes());
        TreePath::from_trusted(vec![component])
    }

    /// The reserved derived path these components spell, if they spell exactly one: a single
    /// component, the prefix, and a method CID in its canonical text form. Anything else is
    /// `None`, and the caller treats the components as an ordinary path (which refuses NUL).
    pub(crate) fn parse_derived(components: &[Vec<u8>]) -> Option<TreePath> {
        let [only] = components else { return None };
        let text = std::str::from_utf8(only.strip_prefix(DERIVED_PREFIX)?).ok()?;
        let method = BlobCid::parse(text).ok()?;
        let path = TreePath::derived(&method);
        (path.components() == components).then_some(path)
    }
}

impl BatchItem {
    /// A derivation output under its input root (C2): the output inherits the input root's
    /// reach.
    ///
    /// No serving tier admits one yet. `resolve_links` finds no entry at the reserved path, so a
    /// `TierEndpoint` answers it `Refused { NotUnderRoot }`; admitting it through the derivation
    /// record the tier holds is slice 4's work (recorded for the interfaces spec's §6).
    pub fn derived(output: BlobCid, method: &BlobCid) -> BatchItem {
        BatchItem::new(output, TreePath::derived(method))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::tp;

    fn descriptor(view: Vec<TreePath>) -> MethodDescriptor {
        MethodDescriptor {
            name: "brit-gate".into(),
            tools: BTreeMap::from([("cargo".into(), "1.96.1".into())]),
            flags: vec![],
            params: None,
            exec: None,
            step: Some("rust-build-test".into()),
            embedder: None,
            view,
            non_path_inputs: BTreeMap::new(),
            deterministic: true,
        }
    }

    fn record(basis: DerivationBasis) -> DerivationRecord {
        DerivationRecord {
            key: DerivationKey {
                method: BlobCid::compute(b"method"),
                input: KeyInput::Raw(BlobCid::compute_raw(b"wasm")),
            },
            output: BlobCid::compute_raw(b"out"),
            computed_by: "uhCAkjessica".into(),
            basis,
            deterministic: true,
            witness: Witness::Signature { sig: "sig".into() },
        }
    }

    #[test]
    fn method_cid_is_a_function_of_the_normalized_view() {
        let a = descriptor(vec![tp("elohim/eprfs"), tp("elohim/epr")]);
        let b = descriptor(vec![
            tp("elohim/epr"),
            tp("elohim/eprfs/eprfs-core"),
            tp("elohim/eprfs"),
        ]);
        assert_eq!(a.cid().unwrap(), b.cid().unwrap());
        let c = descriptor(vec![tp("elohim/epr")]);
        assert_ne!(a.cid().unwrap(), c.cid().unwrap());
    }

    #[test]
    fn unknown_basis_decodes_as_unknown() {
        let mut json = serde_json::to_value(record(DerivationBasis::Computed)).unwrap();
        json["basis"] = serde_json::json!({"basis": "agreed-by-council", "council": "x"});
        json["ext"] = serde_json::json!({"futureField": true});
        let decoded: DerivationRecord = serde_json::from_value(json).unwrap();
        assert_eq!(decoded.basis, DerivationBasis::Unknown);
    }

    #[test]
    fn derivation_key_accepts_only_ext_namespaced_additions() {
        let key = record(DerivationBasis::Computed).key;
        let mut value = serde_json::to_value(&key).unwrap();
        value["ext"] = serde_json::json!({"newWriter": {"opaque": [0, 255]}});
        assert_eq!(
            serde_json::from_value::<DerivationKey>(value.clone()).unwrap(),
            key
        );
        value["restriction"] = serde_json::json!("household-only");
        assert!(serde_json::from_value::<DerivationKey>(value).is_err());
    }

    #[test]
    fn known_epoch_decoder_retains_ext_bytes_and_refuses_other_additions() {
        let expected = record(DerivationBasis::Computed);
        let mut value = serde_json::to_value(&expected).unwrap();
        value.as_object_mut().unwrap().remove("witness");
        value["ext"] = serde_json::json!({"newWriter": {"opaque": [0, 255]}});
        let body = serde_ipld_dagcbor::to_vec(&value).unwrap();
        let retained =
            DerivationRecord::decode_known_epoch(&body, expected.witness.clone()).unwrap();
        assert_eq!(retained.record(), &expected);
        assert_eq!(retained.body(), body);

        value["restriction"] = serde_json::json!("household-only");
        let restricted = serde_ipld_dagcbor::to_vec(&value).unwrap();
        assert!(
            DerivationRecord::decode_known_epoch(&restricted, expected.witness.clone()).is_err()
        );
    }

    #[test]
    fn bundle_decodes_leniently_counting_what_it_skips() {
        let good = serde_json::to_value(record(DerivationBasis::UnderGrant { grant: "g1".into() }))
            .unwrap();
        let bad = serde_json::json!({"key": "not a key"});
        let wire = serde_json::json!({
            "head": "epr:release-head",
            "root": BlobCid::compute(b"root").to_string(),
            "method": BlobCid::compute(b"method").to_string(),
            "records": [good, bad],
        });
        let bytes = serde_ipld_dagcbor::to_vec(&wire).unwrap();
        let (bundle, skipped) = DerivationBundle::decode_lenient(&bytes).unwrap();
        assert_eq!(skipped, 1);
        assert_eq!(bundle.records.len(), 1);
    }

    #[test]
    fn record_projects_onto_the_existing_edge_and_attests_only_with_a_head() {
        let r = record(DerivationBasis::Computed);
        let edge = r.to_edge();
        assert_eq!(edge.relation, DerivationKind::Derivation);
        assert_eq!(edge.derived, r.output);
        assert_eq!(edge.attributed_to, "uhCAkjessica");
        let draft = r
            .to_attestation(&AdoptedHead {
                head: "epr:release-head".into(),
                root: BlobCid::compute(b"root"),
            })
            .unwrap();
        assert_eq!(draft.kind, AttestationKind::Derivation);
        assert_eq!(draft.blob, Some(r.output));
    }

    #[test]
    fn derived_path_can_never_name_a_tree_entry() {
        let path = TreePath::derived(&BlobCid::compute(b"method"));
        assert!(TreePath::from_components(path.components().to_vec()).is_err());
        let item = BatchItem::derived(BlobCid::compute_raw(b"out"), &BlobCid::compute(b"m"));
        assert_eq!(item.path.components().len(), 1);
    }

    /// The reserved path's binary form decodes back, and only the exact form does: a near form
    /// (a bad CID, a second component, a non-canonical CID text) is an ordinary path, refused.
    #[test]
    fn derived_path_round_trips_in_dag_cbor_and_near_forms_are_refused() {
        let path = TreePath::derived(&BlobCid::compute(b"method"));
        let bytes = serde_ipld_dagcbor::to_vec(&path).unwrap();
        let back: TreePath = serde_ipld_dagcbor::from_slice(&bytes).unwrap();
        assert_eq!(back, path);

        let item = BatchItem::derived(BlobCid::compute_raw(b"out"), &BlobCid::compute(b"m"));
        let back: TreePath =
            serde_ipld_dagcbor::from_slice(&serde_ipld_dagcbor::to_vec(&item.path).unwrap())
                .unwrap();
        assert_eq!(back, item.path);

        let mut forged = DERIVED_PREFIX.to_vec();
        forged.extend_from_slice(b"not-a-cid");
        let mut upper = DERIVED_PREFIX.to_vec();
        upper.extend_from_slice(BlobCid::compute(b"m").to_string().to_uppercase().as_bytes());
        for components in [
            vec![forged],
            vec![path.components()[0].clone(), b"x".to_vec()],
            vec![upper],
        ] {
            assert!(TreePath::parse_derived(&components).is_none());
            let wire = serde_ipld_dagcbor::to_vec(
                &components
                    .iter()
                    .map(|c| serde_bytes::ByteBuf::from(c.clone()))
                    .collect::<Vec<_>>(),
            )
            .unwrap();
            assert!(serde_ipld_dagcbor::from_slice::<TreePath>(&wire).is_err());
        }
    }
}
