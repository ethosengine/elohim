//! A task placed on a berth (interfaces spec §2.F): the task spec, the berth offer and the task kind.
//!
//! Spec §12.6: a task is (method, input root, view, envelope). The requester names the task and never
//! ships its inputs: `input` is a [`TreeView`] the carrying berth materializes from its own tier
//! chain. The task is placed through placement-core's floor like any other need: [`TaskSpec::as_need`]
//! expresses it, [`BerthOffer::posture`] is a berth's public offer, and [`TaskKind`] is the
//! [`PlacementKind`] the floor admits it with.
//!
//! Pure, like the rest of ark-core: no I/O, no runtime, no clock. Materializing the view, running the
//! child and recording the derivation are ark-supervisor's TaskRunner (spec §F steps 5–6).
//!
//! Declaration is not enforcement (`elohim/ark/.epr-meta`): the capacity a berth offers is its
//! manifest's declared `envelope.bound`; the isolation it offers is the weakest
//! [`EffectiveTier`] its passport has observed, which is `None` until a runtime enforces and
//! witnesses a limit.

use std::collections::{BTreeMap, BTreeSet};

use cid::Cid;
use elohim_epr::Reach;
use elohim_epr_rea::AgentRef;
use eprfs_core::{BlobCid, DerivationKey, KeyInput, MethodDescriptor, ProjectedRoot, TreeView};
use placement_core::eligibility::{ISOLATION, SCOPE};
use placement_core::{
    required_scope, AgentRefKey, Candidate, Capacity, Charge, CidString, Demand, EpochSecs,
    Expresser, ExpresserKind, GrantScope, IneligibleReason, Isolation, KindId, MethodId,
    MishpatAction, NearestBytes, Need, NeedClass, NeedKey, NeedTerms, OfferTerms, PlacementKind,
    Posture, Ranking, Requirements,
};
use serde::{Deserialize, Serialize};

use crate::exit::ExitClass;
use crate::manifest::{
    ArtifactRef, ChildPolicy, ChildSpec, ManifestError, Probe, ProcessKind, ResourceQuota, Restart,
    RuntimeEnvelope, RuntimeManifest, StdinSource,
};
use crate::passport::{EffectiveTier, Passport};
use crate::Berth;

/// DAG-CBOR's multicodec: a tree root (and so a task's input or base root) is a DAG-CBOR CID.
const DAG_CBOR: u64 = 0x71;

/// The most roots a berth offer lists (spec §F, F8): its depth-3 holdings, sorted by CID.
pub const MAX_OFFER_HOLDS: usize = 64;

/// The name the TaskRunner gives the one child a task runs, when it builds the [`ChildSpec`].
pub const TASK_CHILD_NAME: &str = "task";

// ---- the task -----------------------------------------------------------------------------------

/// Spec §12.6: (method, input root, view, envelope). The device names the task and never ships
/// inputs. No requester-written scope (F1): the scope is derived one way from the method, as
/// `compute_tasks::required_scope` does today, so granted authority never comes from the
/// requester's own bytes.
///
/// Decoding runs the same checks as [`TaskSpec::new`] (`try_from`), so a spec read off the wire
/// whose `method.exec` is not its (child, envelope), or whose `input` is not the method's view, is
/// refused before anything reads it.
///
/// The fields are private, read through getters: every `TaskSpec` value passed `new` or decoding,
/// so the exec binding cannot be broken after the fact by swapping the child or the envelope under
/// an unchanged method CID (Codex review 1; the spec showed `pub` fields). Neither a mutation nor a
/// literal compiles outside this module:
///
/// ```compile_fail,E0616
/// fn swap(spec: &mut ark_core::TaskSpec) {
///     spec.child.argv[0] = "/bin/sh".into();
/// }
/// ```
///
/// ```compile_fail,E0616
/// fn widen(spec: &mut ark_core::TaskSpec) {
///     spec.envelope = None;
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "TaskSpecWire")]
pub struct TaskSpec {
    /// §C; its `exec` is the CID of (child, envelope) (F2).
    method: MethodDescriptor,
    /// What it reads: root + the descriptor's view (§B.3).
    input: TreeView,
    /// A root the requester believes a berth holds, so the delta is fetched through the tier chain
    /// (F9).
    base_root: Option<BlobCid>,
    /// F12: no restart policy to get wrong.
    child: TaskChild,
    /// The existing six-field contract (manifest.rs). Absent moves no CID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    envelope: Option<RuntimeEnvelope>,
    /// Output path prefixes, relative to `{output}` (F13); sorted, each once, and bound into
    /// `method.exec`.
    outputs: Vec<String>,
}

/// The decode side of [`TaskSpec`]: the same fields, refused unknown, then checked.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskSpecWire {
    method: MethodDescriptor,
    input: TreeView,
    base_root: Option<BlobCid>,
    child: TaskChild,
    #[serde(default)]
    envelope: Option<RuntimeEnvelope>,
    outputs: Vec<String>,
}

impl TryFrom<TaskSpecWire> for TaskSpec {
    type Error = ManifestError;

    fn try_from(wire: TaskSpecWire) -> Result<Self, Self::Error> {
        let spec = TaskSpec {
            method: wire.method,
            input: wire.input,
            base_root: wire.base_root,
            child: wire.child,
            envelope: wire.envelope,
            outputs: wire.outputs,
        };
        spec.check()?;
        Ok(spec)
    }
}

/// The task-shaped subset of [`ChildSpec`]: artifact (`ArtifactRef::Pinned`), argv, env, env_scrub,
/// stdin, readiness. It has no restart policy; it converts to a `ChildSpec` with
/// `Restart::Temporary` only (F12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskChild {
    /// Pinned by content digest; a channel is refused (a task runs exactly what its method binds).
    pub artifact: ArtifactRef,
    /// Process argv, including argv\[0\]. `{input}` and `{output}` are template keys the TaskRunner
    /// resolves beside the berth's own (F13).
    pub argv: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Must be true: a requester's program never inherits the carrying ark's environment.
    #[serde(default = "default_env_scrub")]
    pub env_scrub: bool,
    /// Must be `Null`: a requester's program is never handed the berth's passphrase.
    #[serde(default)]
    pub stdin: StdinSource,
    #[serde(default)]
    pub readiness: Vec<Probe>,
}

fn default_env_scrub() -> bool {
    true
}

/// What `method.exec` is the CID of (F2): the child, its envelope and its output selection, the
/// envelope omitted when absent (so a task without one has the CID it would have had before
/// envelopes existed) and the selection omitted when empty.
///
/// The output selection is bound here (Codex review 2) because the derivation key is (method CID,
/// projected input): two specs selecting different outputs under one method would otherwise share a
/// key, and the second would be answered with the first's cached result.
#[derive(Serialize)]
struct ExecBinding<'a> {
    child: &'a TaskChild,
    #[serde(skip_serializing_if = "Option::is_none")]
    envelope: Option<&'a RuntimeEnvelope>,
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    outputs: &'a [String],
}

/// The canonical output selection: sorted, each prefix once.
fn normalized(outputs: &[String]) -> Vec<String> {
    let mut outputs = outputs.to_vec();
    outputs.sort();
    outputs.dedup();
    outputs
}

impl TaskChild {
    /// The CID a method descriptor's `exec` must carry for this child under `envelope`, selecting
    /// `outputs` (F2): the DAG-CBOR CID of `{ child, envelope?, outputs? }`, the selection
    /// normalized first (sorted, each prefix once), so its order never moves the CID. A method
    /// author computes it once; [`TaskSpec::new`] refuses a descriptor whose `exec` is anything else.
    pub fn exec_cid(
        &self,
        envelope: Option<&RuntimeEnvelope>,
        outputs: &[String],
    ) -> Result<BlobCid, ManifestError> {
        let outputs = normalized(outputs);
        let bytes = serde_ipld_dagcbor::to_vec(&ExecBinding {
            child: self,
            envelope,
            outputs: &outputs,
        })
        .map_err(|error| ManifestError::Encode(error.to_string()))?;
        Ok(BlobCid::compute(&bytes))
    }

    /// The berth-local declaration the TaskRunner supervises: `Restart::Temporary`, never anything
    /// else (F12), and no quota of its own (the task's envelope is its bound).
    ///
    /// Runs the same checks [`TaskSpec::new`] does on the child, so an unsafe child (a passphrase
    /// stdin, an unscrubbed environment, a channel artifact) never becomes a `ChildSpec`
    /// (Codex review 1; the spec showed it infallible). [`TaskSpec::child_spec`] is the infallible
    /// form for a child already inside a checked spec.
    pub fn to_child_spec(&self) -> Result<ChildSpec, ManifestError> {
        self.check()?;
        Ok(self.child_spec_unchecked())
    }

    fn child_spec_unchecked(&self) -> ChildSpec {
        ChildSpec {
            name: TASK_CHILD_NAME.to_string(),
            kind: ProcessKind::Native,
            artifact: self.artifact.clone(),
            argv: self.argv.clone(),
            env: self.env.clone(),
            env_scrub: self.env_scrub,
            stdin: self.stdin.clone(),
            readiness: self.readiness.clone(),
            policy: ChildPolicy {
                restart: Restart::Temporary,
                ..ChildPolicy::default()
            },
            listen: Default::default(),
            quota: None,
        }
    }

    fn check(&self) -> Result<(), ManifestError> {
        if self.argv.is_empty() {
            return Err(ManifestError::Invalid(
                "task child has an empty argv".into(),
            ));
        }
        let ArtifactRef::Pinned { cid, sha256, .. } = &self.artifact else {
            return Err(ManifestError::Invalid(
                "task child artifact must be pinned by digest, not a channel".into(),
            ));
        };
        let valid = sha256.len() == 64
            && sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if !valid {
            return Err(ManifestError::Invalid(
                "task child pinned artifact sha256 must be 64 lowercase hexadecimal characters"
                    .into(),
            ));
        }
        if let Some(cid) = cid {
            // `ArtifactRef::Pinned` is the raw CID of the digest (spec §F, first instances).
            let raw = elohim_epr::cid::raw_cid_from_sha256_hex(sha256)
                .map_err(|error| ManifestError::Invalid(error.to_string()))?;
            if cid != &raw.to_string() {
                return Err(ManifestError::Invalid(format!(
                    "task child artifact cid {cid} is not the raw CID of its sha256 ({raw})"
                )));
            }
        }
        if !self.env_scrub {
            return Err(ManifestError::Invalid(
                "task child must scrub the inherited environment".into(),
            ));
        }
        if self.stdin != StdinSource::Null {
            return Err(ManifestError::Invalid(
                "task child stdin must be null; a berth's passphrase is never a task's input"
                    .into(),
            ));
        }
        Ok(())
    }
}

impl TaskSpec {
    /// Refuses when `method.exec` is not the CID of (child, envelope, outputs), when the descriptor
    /// declares no view, when a root is not a DAG-CBOR tree root, when an output prefix leaves
    /// `{output}`, or when the method's derived scope is none (F2). `input` is built from
    /// `input_root` and the descriptor's own view, so it is the descriptor's view by construction;
    /// `outputs` is normalized (sorted, each prefix once), the form decoding requires.
    pub fn new(
        method: MethodDescriptor,
        input_root: BlobCid,
        base_root: Option<BlobCid>,
        child: TaskChild,
        envelope: Option<RuntimeEnvelope>,
        outputs: Vec<String>,
    ) -> Result<Self, ManifestError> {
        let input = TreeView::new(input_root, &method.view());
        let spec = TaskSpec {
            method,
            input,
            base_root,
            child,
            envelope,
            outputs: normalized(&outputs),
        };
        spec.check()?;
        Ok(spec)
    }

    /// §C; its `exec` is the CID of (child, envelope) (F2).
    pub fn method(&self) -> &MethodDescriptor {
        &self.method
    }

    /// What it reads: root + the descriptor's view (§B.3).
    pub fn input(&self) -> &TreeView {
        &self.input
    }

    /// The root the requester believes a berth holds (F9).
    pub fn base_root(&self) -> Option<&BlobCid> {
        self.base_root.as_ref()
    }

    /// The one child the task runs (F12).
    pub fn child(&self) -> &TaskChild {
        &self.child
    }

    /// The six-field envelope, when the task declares one.
    pub fn envelope(&self) -> Option<&RuntimeEnvelope> {
        self.envelope.as_ref()
    }

    /// Output path prefixes, relative to `{output}` (F13).
    pub fn outputs(&self) -> &[String] {
        &self.outputs
    }

    /// The berth-local declaration the TaskRunner supervises. Infallible: the child was checked
    /// when this spec was built or decoded.
    pub fn child_spec(&self) -> ChildSpec {
        self.child.child_spec_unchecked()
    }

    /// The canonical DAG-CBOR CID; an absent envelope moves nothing.
    pub fn cid(&self) -> Result<Cid, ManifestError> {
        let bytes = serde_ipld_dagcbor::to_vec(self)
            .map_err(|error| ManifestError::Encode(error.to_string()))?;
        Ok(elohim_epr::cid::compute_cid(&bytes))
    }

    /// `NeedKey::Task(self.cid()?)`.
    pub fn need_key(&self) -> Result<NeedKey, ManifestError> {
        Ok(NeedKey::Task(self.cid()?))
    }

    /// The scope, one way from the method (F1): `required_scope` (moved into placement-core) over
    /// the method's name, which plays the part the envelope's `project` plays in
    /// `compute_tasks.rs` (`a2o-stage:<stage>` spends `measure-stage`, anything else the historical
    /// `sweettest-feedback`). The descriptor is bound to the task CID, so nothing the requester
    /// writes beside it moves the scope.
    pub fn scope(&self) -> Option<GrantScope> {
        Some(required_scope(Some(self.method.name.as_str())))
    }

    /// (method CID, projected input): the method CID already binds the program that runs and the
    /// outputs it selects (F2), so two specs with one key ask for the same result.
    pub fn derivation_key(
        &self,
        projected: &ProjectedRoot,
    ) -> Result<DerivationKey, ManifestError> {
        Ok(DerivationKey {
            method: self
                .method
                .cid()
                .map_err(|error| ManifestError::Encode(error.to_string()))?,
            input: KeyInput::Projected(projected.clone()),
        })
    }

    /// kind Task, demand {1,1}, `Requirements` from `envelope.bound` (pids included; bytes =
    /// `disk_bytes` plus `view_bytes`, the materialized view's size, F16),
    /// `NeedTerms::Task { input_root, scope, base_root }`. Consent is the snapshot builder's to
    /// resolve per candidate (F5): `NotNeeded` for the requester's own envelope, `Granted` from the
    /// requester's delegates-compute grant otherwise. `carriers` narrows every peer's admit (`--on`,
    /// F14).
    ///
    /// `view_bytes` revises the spec's signature: the size of the materialized view is the
    /// requester's reading (it ran `walk_spine`), and nothing in the spec itself carries it. It is
    /// required, not optional (Codex review 1): a task always materializes its view, so a need
    /// without a measured view size would reserve no room for its input. The sum is checked, so
    /// an overflow refuses rather than understating the total. The requirement's `isolation` is
    /// `None`: the six-field envelope declares no isolation floor.
    pub fn as_need(
        &self,
        requester: AgentRef,
        space: Cid,
        reach: Reach,
        class: NeedClass,
        carriers: Option<BTreeSet<AgentRefKey>>,
        view_bytes: u64,
    ) -> Result<Need, ManifestError> {
        let scope = self
            .scope()
            .ok_or_else(|| ManifestError::Invalid("the method derives no scope".into()))?;
        let bound = self
            .envelope
            .as_ref()
            .map(|envelope| envelope.bound.clone())
            .unwrap_or_default();
        let bytes = bound
            .disk_bytes
            .unwrap_or(0)
            .checked_add(view_bytes)
            .ok_or_else(|| {
                ManifestError::Invalid(
                    "the task's disk bound plus its view size overflows a byte count".into(),
                )
            })?;
        Ok(Need {
            kind: KindId::Task,
            key: self.need_key()?,
            demand: Demand {
                holders: 1,
                domains: 1,
            },
            requires: Requirements {
                always_on: false,
                bytes: Some(bytes),
                memory_bytes: bound.memory_bytes,
                cpu_millis: bound.cpu_millis,
                pids: bound.pids,
                isolation: None,
                carriers,
            },
            terms: NeedTerms::Task {
                input_root: CidString::from(self.input.root().as_cid()),
                scope,
                base_root: self
                    .base_root
                    .as_ref()
                    .map(|root| CidString::from(root.as_cid())),
            },
            reach,
            expressed_by: Expresser {
                by: ExpresserKind::Agent,
                id: requester.0.clone(),
            },
            receiver: requester,
            class,
            in_scope_of: space,
        })
    }

    /// Every refusal `new` and decoding share.
    fn check(&self) -> Result<(), ManifestError> {
        self.child.check()?;
        // Canonical form, so a decoded spec's CID is the CID of the bytes it was decoded from.
        if !self.outputs.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(ManifestError::Invalid(
                "the task's output prefixes are not sorted and distinct".into(),
            ));
        }
        let exec = self.child.exec_cid(self.envelope.as_ref(), &self.outputs)?;
        if self.method.exec.as_ref() != Some(&exec) {
            return Err(ManifestError::Invalid(format!(
                "method exec is not the CID of the task's (child, envelope, outputs): expected {exec}"
            )));
        }
        let view = self.method.view();
        if view.is_empty() {
            return Err(ManifestError::Invalid(
                "the method declares no view, so the task reads nothing".into(),
            ));
        }
        if !self.input.paths().iter().eq(view.paths()) {
            return Err(ManifestError::Invalid(
                "the task's input paths are not the method's view".into(),
            ));
        }
        for (what, root) in [
            ("input", Some(self.input.root())),
            ("base", self.base_root.as_ref()),
        ] {
            if let Some(root) = root {
                if root.as_cid().codec() != DAG_CBOR {
                    return Err(ManifestError::Invalid(format!(
                        "the task's {what} root {root} is not a DAG-CBOR tree root"
                    )));
                }
            }
        }
        for prefix in &self.outputs {
            let escapes = prefix.is_empty()
                || prefix.starts_with('/')
                || prefix.contains('\\')
                || prefix.split('/').any(|part| part == ".." || part == ".");
            if escapes {
                return Err(ManifestError::Invalid(format!(
                    "output prefix {prefix:?} is not a plain path relative to {{output}}"
                )));
            }
        }
        if self.scope().is_none() {
            return Err(ManifestError::Invalid("the method derives no scope".into()));
        }
        Ok(())
    }
}

// ---- the berth offer ----------------------------------------------------------------------------

/// The public projection of a local [`Berth`]. Built only by [`BerthOffer::from_berth`], which
/// copies no data_root, port, artifact path or passphrase (R6).
///
/// `holds` is private and read through [`BerthOffer::holds`] (Codex review 2): it carries the
/// offer's two invariants, at most [`MAX_OFFER_HOLDS`] roots and each root once at one reach, and
/// decoding refuses a list that breaks either (`try_from`), so neither a wire value nor a mutation
/// can hand [`BerthOffer::posture`] an unbounded list or a root at two reaches. The other fields
/// are plain declarations with no invariant between them, and stay public.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "BerthOfferWire")]
pub struct BerthOffer {
    pub blade: AgentRef,
    /// `CandidateContent.berth`: the independently provisioned berth identity.
    pub berth: CidString,
    /// The CID string of the manifest occupying the berth.
    pub manifest: String,
    pub incarnation: u64,
    /// `None` for a berth with no process yet.
    pub isolation: Option<Isolation>,
    /// `None` fields when the manifest declares no envelope. A declaration, not enforcement.
    pub capacity: Capacity,
    /// Sorted, at most [`MAX_OFFER_HOLDS`] roots kept at depth 3, each with its own reach (F8).
    holds: Vec<(CidString, Reach)>,
    pub scopes: BTreeSet<GrantScope>,
}

/// The decode side of [`BerthOffer`]: the same fields, refused unknown, then checked.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BerthOfferWire {
    blade: AgentRef,
    berth: CidString,
    manifest: String,
    incarnation: u64,
    isolation: Option<Isolation>,
    capacity: Capacity,
    holds: Vec<(CidString, Reach)>,
    scopes: BTreeSet<GrantScope>,
}

impl TryFrom<BerthOfferWire> for BerthOffer {
    type Error = ManifestError;

    /// Refuses more than [`MAX_OFFER_HOLDS`] roots, and roots that are not strictly ascending: a
    /// root listed twice (at any reach) is not ascending, so a wire offer never shows one root at
    /// two reaches. `from_berth` produces exactly this form.
    fn try_from(wire: BerthOfferWire) -> Result<Self, Self::Error> {
        if wire.holds.len() > MAX_OFFER_HOLDS {
            return Err(ManifestError::Invalid(format!(
                "a berth offer lists {} roots; at most {MAX_OFFER_HOLDS}",
                wire.holds.len()
            )));
        }
        if !wire.holds.windows(2).all(|pair| pair[0].0 < pair[1].0) {
            return Err(ManifestError::Invalid(
                "a berth offer's roots must be sorted and listed once each".into(),
            ));
        }
        Ok(Self {
            blade: wire.blade,
            berth: wire.berth,
            manifest: wire.manifest,
            incarnation: wire.incarnation,
            isolation: wire.isolation,
            capacity: wire.capacity,
            holds: wire.holds,
            scopes: wire.scopes,
        })
    }
}

impl BerthOffer {
    /// capacity = `From<&envelope.bound>` (`None` fields without an envelope); isolation = the
    /// weakest `effective_tier` across the passport's processes, `None` for an empty passport (F7).
    /// A root listed twice keeps its narrower reach, so a duplicate never widens what a root is
    /// shown at.
    pub fn from_berth(
        berth: &Berth,
        berth_cid: Cid,
        blade: AgentRef,
        manifest: &RuntimeManifest,
        passport: &Passport,
        holds: Vec<(Cid, Reach)>,
        scopes: BTreeSet<GrantScope>,
    ) -> Self {
        let isolation = passport
            .processes
            .iter()
            .map(|process| Isolation::from(&process.effective_tier))
            .min();
        let capacity = manifest
            .envelope
            .as_ref()
            .map(|envelope| Capacity::from(&envelope.bound))
            .unwrap_or_default();
        let mut kept: BTreeMap<CidString, Reach> = BTreeMap::new();
        for (root, reach) in holds {
            kept.entry(CidString::from(&root))
                .and_modify(|held| {
                    if reach.openness() < held.openness() {
                        *held = reach;
                    }
                })
                .or_insert(reach);
        }
        Self {
            blade,
            berth: CidString::from(&berth_cid),
            manifest: berth.manifest.clone(),
            incarnation: berth.incarnation,
            isolation,
            capacity,
            holds: kept.into_iter().take(MAX_OFFER_HOLDS).collect(),
            scopes,
        }
    }

    /// The roots this berth keeps at depth 3, each with its own reach: sorted, each once, at most
    /// [`MAX_OFFER_HOLDS`].
    pub fn holds(&self) -> &[(CidString, Reach)] {
        &self.holds
    }

    /// Offers `KindId::Task` with scopes and isolation in its `OfferTerms`, and lists only the holds
    /// whose own reach is no narrower than the posture's (F8): no posture names a root below its own
    /// reach.
    pub fn posture(&self, reach: Reach, written_at: EpochSecs) -> Posture {
        let holds = self
            .holds
            .iter()
            .filter(|(_, held)| held.openness() >= reach.openness())
            .map(|(root, _)| root.clone())
            .collect();
        let terms = OfferTerms {
            spare_bytes: None,
            compute: Some(self.capacity),
            holds,
            reach_ceiling: None,
            scopes: self.scopes.clone(),
            isolation: self.isolation,
            deferred: false,
            household_only: false,
        };
        Posture {
            carrier: self.blade.clone(),
            offers: BTreeMap::from([(KindId::Task, terms)]),
            reach,
            written_at,
        }
    }
}

/// 1:1 onto `placement_core::Isolation`, whose ladder is ark's own, mirrored.
impl From<&EffectiveTier> for Isolation {
    fn from(tier: &EffectiveTier) -> Self {
        match tier {
            EffectiveTier::Enforced => Isolation::Enforced,
            EffectiveTier::Bounded => Isolation::Bounded,
            EffectiveTier::Delegated => Isolation::Delegated,
            EffectiveTier::Intrinsic => Isolation::Intrinsic,
            EffectiveTier::None => Isolation::None,
        }
    }
}

impl From<EffectiveTier> for Isolation {
    fn from(tier: EffectiveTier) -> Self {
        Isolation::from(&tier)
    }
}

/// `memory_bytes`, `cpu_millis`, `pids`; `disk_bytes` → `bytes`.
impl From<&ResourceQuota> for Capacity {
    fn from(quota: &ResourceQuota) -> Self {
        Capacity {
            bytes: quota.disk_bytes,
            memory_bytes: quota.memory_bytes,
            cpu_millis: quota.cpu_millis,
            pids: quota.pids,
        }
    }
}

/// `eprfs_core::Depth` onto placement-core's mirror (placement-core does not depend on eprfs-core;
/// ark-core depends on both, so it holds the 1:1 map). A function, not `From`: both types are
/// foreign here.
pub fn placement_depth(depth: eprfs_core::Depth) -> placement_core::Depth {
    match depth {
        eprfs_core::Depth::Query => placement_core::Depth::Query,
        eprfs_core::Depth::Sparse => placement_core::Depth::Sparse,
        eprfs_core::Depth::OnRead => placement_core::Depth::OnRead,
        eprfs_core::Depth::Kept => placement_core::Depth::Kept,
    }
}

/// The inverse of [`placement_depth`].
pub fn eprfs_depth(depth: placement_core::Depth) -> eprfs_core::Depth {
    match depth {
        placement_core::Depth::Query => eprfs_core::Depth::Query,
        placement_core::Depth::Sparse => eprfs_core::Depth::Sparse,
        placement_core::Depth::OnRead => eprfs_core::Depth::OnRead,
        placement_core::Depth::Kept => eprfs_core::Depth::Kept,
    }
}

// ---- the task kind ------------------------------------------------------------------------------

/// The kind's own refusal when a need's `NeedTerms::Task` names another input or base root than the
/// spec behind its `NeedKey::Task` (Codex review 1).
pub const TASK_TERMS: &str = "task-terms";

/// `impl PlacementKind`: `receiver_may_carry = true`; refuses `Kind { label: "scope" }` when the
/// offer's scopes lack the need's derived scope, `Kind { label: "isolation" }` when the offer's
/// isolation is weaker than the need's, and `OverCapacity` when capacity < the need's requirements
/// (pids included); default method = `NearestBytes`.
///
/// The scope is derived from the [`TaskSpec`] behind `NeedKey::Task`, never from the need's own
/// `NeedTerms::Task.scope` (F1). A need carries only the spec's CID, so the kind holds the specs this
/// peer has read ([`TaskKind::with_spec`]); this revises the spec's unit `struct TaskKind;`, which
/// could not derive a scope from a CID. A task whose spec this peer has not read derives no scope,
/// so core refuses its declared scope (`Kind { "scope" }`) and `fits` refuses it as
/// `Unknown { input: "task-spec" }`: an unread spec is never carried.
///
/// The need's other declarations are checked against the registered spec too (Codex review 1):
/// its input and base roots must be the spec's (`Kind { "task-terms" }`), it must state its byte
/// requirement (`Unknown { input: "view-bytes" }`), and every component is charged at least the
/// spec's own envelope bound, so lowering a need's requirements under an unchanged task key never
/// admits a task for less than its envelope.
///
/// The byte floor is the envelope's `disk_bytes` plus the materialized view's size, the same sum
/// [`TaskSpec::as_need`] states (Codex review 2). The spec does not carry the view's size, so the
/// registering peer supplies its own measurement ([`TaskKind::with_spec`]); a task registered
/// without one is refused `Unknown { input: "view-bytes" }` rather than charged for its disk bound
/// alone.
#[derive(Debug, Clone, Default)]
pub struct TaskKind {
    tasks: BTreeMap<Cid, Registered>,
}

/// What a read spec fixes about every need that names it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Registered {
    scope: GrantScope,
    input_root: CidString,
    base_root: Option<CidString>,
    /// The envelope's bound, `bytes` being `disk_bytes` plus the measured view size: the least a
    /// carrier is charged. `bytes` is `None` when the view was not measured, and then the task
    /// never fits.
    floor: Capacity,
}

impl TaskKind {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a spec this peer has read, under its CID, with the scope it derives, its roots
    /// and its envelope's bound. The spec is checked again here, at the registration boundary,
    /// though every `TaskSpec` value is checked by construction.
    ///
    /// `view_bytes` is this peer's measurement of the materialized view's size (the `walk_spine`
    /// reading [`TaskSpec::as_need`] takes), `None` when it has not measured it. The byte floor is
    /// the checked sum of the envelope's `disk_bytes` and that size; an overflow refuses here, and
    /// an unmeasured view refuses every admission (`Unknown { input: "view-bytes" }`).
    pub fn with_spec(
        mut self,
        spec: &TaskSpec,
        view_bytes: Option<u64>,
    ) -> Result<Self, ManifestError> {
        spec.check()?;
        let scope = spec
            .scope()
            .ok_or_else(|| ManifestError::Invalid("the method derives no scope".into()))?;
        let mut floor = spec
            .envelope()
            .map(|envelope| Capacity::from(&envelope.bound))
            .unwrap_or_default();
        floor.bytes = match view_bytes {
            None => None,
            Some(view) => Some(floor.bytes.unwrap_or(0).checked_add(view).ok_or_else(|| {
                ManifestError::Invalid(
                    "the task's disk bound plus its view size overflows a byte count".into(),
                )
            })?),
        };
        self.tasks.insert(
            spec.cid()?,
            Registered {
                scope,
                input_root: CidString::from(spec.input().root().as_cid()),
                base_root: spec.base_root().map(|root| CidString::from(root.as_cid())),
                floor,
            },
        );
        Ok(self)
    }

    /// Whether this peer has read the spec behind `task`.
    pub fn knows(&self, task: &Cid) -> bool {
        self.tasks.contains_key(task)
    }

    fn registered(&self, need: &Need) -> Option<&Registered> {
        match (&need.kind, &need.key) {
            (KindId::Task, NeedKey::Task(task)) => self.tasks.get(task),
            _ => None,
        }
    }

    fn derived(&self, need: &Need) -> Option<GrantScope> {
        self.registered(need).map(|task| task.scope)
    }
}

impl PlacementKind for TaskKind {
    fn id(&self) -> KindId {
        KindId::Task
    }

    /// Delegates-compute under the derived scope. A task whose spec is unread is never admitted
    /// (`fits`), so the fallback, `required_scope(None)`, names no carrying.
    fn action(&self, need: &Need) -> MishpatAction {
        MishpatAction::DelegatesCompute {
            scope: self.derived(need).unwrap_or_else(|| required_scope(None)),
        }
    }

    fn scope(&self, need: &Need) -> Option<GrantScope> {
        self.derived(need)
    }

    fn receiver_may_carry(&self) -> bool {
        true
    }

    fn fits(
        &self,
        need: &Need,
        _candidate: &Candidate,
        offer: &OfferTerms,
        remaining: &Capacity,
    ) -> Result<Charge, IneligibleReason> {
        let task = self
            .registered(need)
            .ok_or(IneligibleReason::Unknown { input: "task-spec" })?;
        match &need.terms {
            NeedTerms::Task {
                input_root,
                base_root,
                ..
            } if *input_root == task.input_root && *base_root == task.base_root => {}
            _ => return Err(IneligibleReason::Kind { label: TASK_TERMS }),
        }
        if !offer.scopes.contains(&task.scope) {
            return Err(IneligibleReason::Kind { label: SCOPE });
        }
        if let Some(floor) = need.requires.isolation {
            if offer.isolation.is_none_or(|offered| offered < floor) {
                return Err(IneligibleReason::Kind { label: ISOLATION });
            }
        }
        /// The larger of what the need asks and what the spec's envelope bounds; `None` only when
        /// neither states the component.
        fn at_least<T: Ord + Copy>(asked: Option<T>, floor: Option<T>) -> Option<T> {
            match (asked, floor) {
                (Some(asked), Some(floor)) => Some(asked.max(floor)),
                (asked, floor) => asked.or(floor),
            }
        }
        fn within<T: Ord + Copy + Default>(
            asked: Option<T>,
            left: Option<T>,
        ) -> Result<T, IneligibleReason> {
            match (asked, left) {
                (None, _) => Ok(T::default()),
                (Some(_), None) => Err(IneligibleReason::Unknown { input: "capacity" }),
                (Some(asked), Some(left)) if asked > left => Err(IneligibleReason::OverCapacity),
                (Some(asked), Some(_)) => Ok(asked),
            }
        }
        let r = &need.requires;
        // A task always materializes its view, so neither a need with no byte requirement nor a
        // spec registered without a measured view size ever fits.
        let unmeasured = || IneligibleReason::Unknown {
            input: "view-bytes",
        };
        let bytes = r.bytes.ok_or_else(unmeasured)?;
        let floor = &task.floor;
        let view_floor = floor.bytes.ok_or_else(unmeasured)?;
        Ok(Charge {
            bytes: within(Some(bytes.max(view_floor)), remaining.bytes)?,
            memory_bytes: within(
                at_least(r.memory_bytes, floor.memory_bytes),
                remaining.memory_bytes,
            )?,
            cpu_millis: within(
                at_least(r.cpu_millis, floor.cpu_millis),
                remaining.cpu_millis,
            )?,
            pids: within(at_least(r.pids, floor.pids), remaining.pids)?,
        })
    }

    fn default_method(&self) -> MethodId {
        NearestBytes.method()
    }
}

// ---- the outcome --------------------------------------------------------------------------------

/// Settles onto the existing provider-authored `rakia-compute-complete-v1` EconomicEvent (F11), not
/// a parallel record. `completion_event` names that event.
///
/// Build it with [`TaskOutcome::new`], which refuses a derivation record for a run that did not
/// exit cleanly: a failed run never short-circuits a later request (spec §F step 6). The fields are
/// private, read through getters, so a built outcome cannot acquire a record or a failing exit
/// afterwards (Codex review 1; the spec showed `pub` fields):
///
/// ```compile_fail,E0616
/// fn fail(outcome: &mut ark_core::TaskOutcome) {
///     outcome.exit = ark_core::ExitClass::Unknown;
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskOutcome {
    task: Cid,
    exit: ExitClass,
    output: Option<BlobCid>,
    record: Option<eprfs_core::DerivationRecord>,
    completion_event: String,
}

impl TaskOutcome {
    /// Refuses a record on any exit but `Exited { code: 0 }`, a record without an output, and a
    /// record whose output is not the outcome's.
    pub fn new(
        task: Cid,
        exit: ExitClass,
        output: Option<BlobCid>,
        record: Option<eprfs_core::DerivationRecord>,
        completion_event: String,
    ) -> Result<Self, ManifestError> {
        if let Some(record) = &record {
            if !matches!(exit, ExitClass::Exited { code: 0 }) {
                return Err(ManifestError::Invalid(format!(
                    "a run that ended {exit:?} records no derivation"
                )));
            }
            if output.as_ref() != Some(&record.output) {
                return Err(ManifestError::Invalid(
                    "a derivation record's output must be the task's sealed output".into(),
                ));
            }
        }
        Ok(Self {
            task,
            exit,
            output,
            record,
            completion_event,
        })
    }

    pub fn task(&self) -> &Cid {
        &self.task
    }

    pub fn exit(&self) -> &ExitClass {
        &self.exit
    }

    pub fn output(&self) -> Option<&BlobCid> {
        self.output.as_ref()
    }

    /// Present only for a clean exit, and then its output is the outcome's.
    pub fn record(&self) -> Option<&eprfs_core::DerivationRecord> {
        self.record.as_ref()
    }

    /// The `rakia-compute-complete-v1` EconomicEvent this outcome settles onto (F11).
    pub fn completion_event(&self) -> &str {
        &self.completion_event
    }

    /// Whether the run exited cleanly.
    pub fn succeeded(&self) -> bool {
        matches!(self.exit, ExitClass::Exited { code: 0 })
    }
}
