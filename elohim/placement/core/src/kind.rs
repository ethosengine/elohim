//! D.1 — placement kinds: a closed list, because grant scopes are a closed list.
//!
//! One `decide` pass covers every kind, so capacity is charged across kinds in the charter's serve
//! order (§D.7). A new kind is four coupled steps landed together (interfaces spec §D.1): a
//! [`KindId`] variant and its `enums/placement-kind.schema.json` entry; a [`PlacementKind`] in the
//! crate that owns the carrier; a [`GrantScope`] if it spends compute; a probe.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::eligibility::IneligibleReason;
use crate::need::Need;
use crate::offer::{Candidate, Capacity, OfferTerms};
use crate::params::PlaceError;
use crate::rank::{
    is_method_name, DiversityAware, MethodId, NearestBytes, Ranking, WeightedCapacity, XorDistance,
};

/// Which kind of carrying a need asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum KindId {
    CustodyBlob,
    CustodyRoot,
    LiveSession,
    Relay,
    WakeIngress,
    HubRuntime,
    Inference,
    IndexFold,
    Task,
}

impl KindId {
    /// Every kind, in declaration order (the order `enums/placement-kind.schema.json` lists).
    pub const ALL: &'static [KindId] = &[
        KindId::CustodyBlob,
        KindId::CustodyRoot,
        KindId::LiveSession,
        KindId::Relay,
        KindId::WakeIngress,
        KindId::HubRuntime,
        KindId::Inference,
        KindId::IndexFold,
        KindId::Task,
    ];

    /// The kebab-case wire label, as serde writes it.
    pub fn label(self) -> &'static str {
        match self {
            KindId::CustodyBlob => "custody-blob",
            KindId::CustodyRoot => "custody-root",
            KindId::LiveSession => "live-session",
            KindId::Relay => "relay",
            KindId::WakeIngress => "wake-ingress",
            KindId::HubRuntime => "hub-runtime",
            KindId::Inference => "inference",
            KindId::IndexFold => "index-fold",
            KindId::Task => "task",
        }
    }
}

/// Mirrors `enums/grant-scope.schema.json` and elohim-storage's `GRANT_SCOPES` (a test there asserts
/// equality). The compute-grant scopes only; delegates-compute's own `scope` stays an open string
/// (finding 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum GrantScope {
    SweettestFeedback,
    HostedCell,
    MeasureStage,
}

impl GrantScope {
    /// Every scope, in `GRANT_SCOPES` order (`compute_grants.rs:46`).
    pub const ALL: &'static [GrantScope] = &[
        GrantScope::SweettestFeedback,
        GrantScope::HostedCell,
        GrantScope::MeasureStage,
    ];

    /// The wire label, as serde writes it and `GRANT_SCOPES` spells it.
    pub fn label(self) -> &'static str {
        match self {
            GrantScope::SweettestFeedback => "sweettest-feedback",
            GrantScope::HostedCell => "hosted-cell",
            GrantScope::MeasureStage => "measure-stage",
        }
    }
}

/// The project prefix that marks a peer-executed a2o stage (`compute_tasks.rs::STAGE_PROJECT_PREFIX`).
pub const STAGE_PROJECT_PREFIX: &str = "a2o-stage:";

/// `compute_tasks.rs::required_scope`, moved here as the one-way mapping onto a scope (§F, F1): a
/// requester-supplied project prefix selects the scope a grant must name, so granted authority is
/// never derived from the requester's own bytes. Absent or any other project → the historical
/// `sweettest-feedback`.
pub fn required_scope(project: Option<&str>) -> GrantScope {
    match project {
        Some(p) if p.starts_with(STAGE_PROJECT_PREFIX) => GrantScope::MeasureStage,
        _ => GrantScope::SweettestFeedback,
    }
}

/// The mishpat action a commitment for this kind carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "action"
)]
pub enum MishpatAction {
    CustodyBlob,
    ReplicatesContent,
    DelegatesCompute { scope: GrantScope },
}

/// What adoption charges against the carrier's remaining offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Charge {
    pub bytes: u64,
    pub memory_bytes: u64,
    pub cpu_millis: u32,
    pub pids: u32,
}

impl Charge {
    /// Component-wise sum, saturating: what a carrier has been charged across needs.
    pub(crate) fn plus(&self, other: &Self) -> Self {
        Self {
            bytes: self.bytes.saturating_add(other.bytes),
            memory_bytes: self.memory_bytes.saturating_add(other.memory_bytes),
            cpu_millis: self.cpu_millis.saturating_add(other.cpu_millis),
            pids: self.pids.saturating_add(other.pids),
        }
    }

    /// Component-wise difference, saturating at zero: a reservation taken back out of a total.
    pub(crate) fn minus(&self, other: &Self) -> Self {
        Self {
            bytes: self.bytes.saturating_sub(other.bytes),
            memory_bytes: self.memory_bytes.saturating_sub(other.memory_bytes),
            cpu_millis: self.cpu_millis.saturating_sub(other.cpu_millis),
            pids: self.pids.saturating_sub(other.pids),
        }
    }

    /// Component-wise maximum: one commitment's reservation across the expressions that share it.
    pub(crate) fn max(&self, other: &Self) -> Self {
        Self {
            bytes: self.bytes.max(other.bytes),
            memory_bytes: self.memory_bytes.max(other.memory_bytes),
            cpu_millis: self.cpu_millis.max(other.cpu_millis),
            pids: self.pids.max(other.pids),
        }
    }

    /// The least any admission charges for a need (the charge floor, §D.1): what it requires,
    /// unstated components zero.
    pub(crate) fn of_requires(r: &crate::need::Requirements) -> Self {
        Self {
            bytes: r.bytes.unwrap_or(0),
            memory_bytes: r.memory_bytes.unwrap_or(0),
            cpu_millis: r.cpu_millis.unwrap_or(0),
            pids: r.pids.unwrap_or(0),
        }
    }
}

/// One kind of carrying. Implemented in the crate that owns the carrier (custody in elohim-storage,
/// task in ark-core); placement-core ships none of its own beyond the test kinds.
pub trait PlacementKind: Send + Sync {
    fn id(&self) -> KindId;
    fn action(&self, need: &Need) -> MishpatAction;
    /// The scope consumed when the need touches another person (R3); `None`: a declared posture
    /// suffices, and the consent step of `admit` does not run.
    ///
    /// Derived one way by the kind from what the need names (for a task, the `TaskSpec` behind
    /// `NeedKey::Task`, through [`required_scope`]), NEVER read from `NeedTerms::Task.scope`, which
    /// is the requester's own declaration (F1). Core refuses a need whose declared scope differs
    /// from this (`Kind { label: "scope" }`), and refuses an offer whose `scopes` lack it; an
    /// implementor pins the derivation with [`testing::assert_scope_is_derived`].
    fn scope(&self, need: &Need) -> Option<GrantScope>;
    /// Whether the receiver may carry its own need. Custody kinds: false (a copy on the receiver is
    /// not a second holder). Task: true, so the requester's own envelope is the first candidate (F5).
    fn receiver_may_carry(&self) -> bool;
    /// What adoption charges against the carrier's remaining offer, or the kind's own refusal. It can
    /// only refuse: the floor's universal checks run first and a kind cannot widen them. Those are
    /// consent, posture and liveness, named carriers, the receiver rule, reach, the household rule,
    /// the offer's `scopes` and `isolation` (F6), and capacity: every component of
    /// `need.requires` against `remaining`, the capacity this kind's offer has left, with an unstated
    /// component refused as `Unknown { input: "capacity" }`.
    fn fits(
        &self,
        need: &Need,
        candidate: &Candidate,
        offer: &OfferTerms,
        remaining: &Capacity,
    ) -> Result<Charge, IneligibleReason>;
    /// The method used when no parameter set for this kind is in force.
    fn default_method(&self) -> MethodId;
}

/// The kinds this peer can decide.
#[derive(Default)]
pub struct KindRegistry(BTreeMap<KindId, Box<dyn PlacementKind>>);

impl KindRegistry {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }
    pub fn with(mut self, kind: Box<dyn PlacementKind>) -> Self {
        self.0.insert(kind.id(), kind);
        self
    }
    pub fn get(&self, id: KindId) -> Option<&dyn PlacementKind> {
        self.0.get(&id).map(|k| k.as_ref())
    }
}

/// `MethodId` → `Ranking` (D7). "This peer does not run the in-force method" means the `MethodId`
/// is not here.
///
/// One `MethodId` names one behaviour on every peer (floor contract 1), so the registry never
/// replaces an entry, and only the built-ins are registered under the reserved `elohim` namespace:
/// [`MethodRegistry::with`] takes the method's name, proves the id is that name's hash, and refuses
/// a taken id or an `elohim:` name.
pub struct MethodRegistry(BTreeMap<MethodId, Box<dyn Ranking>>);

impl MethodRegistry {
    /// `XorDistance`, `DiversityAware`, `NearestBytes`, `WeightedCapacity`.
    pub fn builtin() -> Self {
        let mut m = Self(BTreeMap::new());
        for r in [
            Box::new(XorDistance) as Box<dyn Ranking>,
            Box::new(DiversityAware),
            Box::new(NearestBytes),
            Box::new(WeightedCapacity),
        ] {
            m.0.insert(r.method(), r);
        }
        m
    }
    /// An empty registry: a peer that runs no method observes every need.
    pub fn empty() -> Self {
        Self(BTreeMap::new())
    }
    /// Registers an adapter's ranking under `name` (`<namespace>:<area>:<name>@<version>`). Refuses
    /// a malformed name, an id that is not `MethodId::named(name)`, the reserved `elohim` namespace,
    /// and an id already registered, so a built-in is never replaced and one id never runs two
    /// behaviours. The implementor runs `contract::assert_ranking_contract` over it in its own tests
    /// (see [`Ranking`]). Revises the spec's infallible `with(self, r) -> Self` (fix round 2).
    pub fn with(mut self, name: &str, r: Box<dyn Ranking>) -> Result<Self, PlaceError> {
        if !is_method_name(name) {
            return Err(PlaceError(format!(
                "method name {name:?} is not well formed"
            )));
        }
        if MethodId::named(name) != r.method() {
            return Err(PlaceError(format!(
                "the ranking's method id is not the hash of {name:?}"
            )));
        }
        if name.split(':').next() == Some("elohim") {
            return Err(PlaceError(format!(
                "{name:?}: the elohim namespace is reserved for the built-in methods"
            )));
        }
        if self.0.contains_key(&r.method()) {
            return Err(PlaceError(format!("{name:?} is already registered")));
        }
        self.0.insert(r.method(), r);
        Ok(self)
    }
    pub fn get(&self, m: MethodId) -> Option<&dyn Ranking> {
        self.0.get(&m).map(|r| r.as_ref())
    }
}

/// Doubles for contract suites, in this crate and in implementors' tests.
#[cfg(any(test, feature = "testing"))]
pub mod testing {
    use super::*;
    use crate::eligibility::IneligibleReason;
    use crate::need::NeedTerms;

    /// A kind that charges `requires.bytes` against `remaining.bytes` and refuses `OverCapacity`
    /// past it. Custody-shaped unless `receiver_may_carry` is set. Its scope is its own field, never
    /// the need's declaration (F1).
    pub struct BytesKind {
        pub id: KindId,
        pub scope: Option<GrantScope>,
        pub receiver_may_carry: bool,
    }

    impl BytesKind {
        /// The custody shape: no scope, the receiver may not carry.
        pub fn custody(id: KindId) -> Self {
            Self {
                id,
                scope: None,
                receiver_may_carry: false,
            }
        }
        /// The task shape: the scope the kind derives (measure-stage), the receiver may carry.
        pub fn task() -> Self {
            Self {
                id: KindId::Task,
                scope: Some(GrantScope::MeasureStage),
                receiver_may_carry: true,
            }
        }
    }

    impl PlacementKind for BytesKind {
        fn id(&self) -> KindId {
            self.id
        }
        fn action(&self, need: &Need) -> MishpatAction {
            match (self.scope(need), self.id) {
                (Some(scope), _) => MishpatAction::DelegatesCompute { scope },
                (None, KindId::CustodyBlob) => MishpatAction::CustodyBlob,
                (None, _) => MishpatAction::ReplicatesContent,
            }
        }
        fn scope(&self, _need: &Need) -> Option<GrantScope> {
            self.scope
        }
        fn receiver_may_carry(&self) -> bool {
            self.receiver_may_carry
        }
        fn fits(
            &self,
            need: &Need,
            _candidate: &Candidate,
            _offer: &OfferTerms,
            remaining: &Capacity,
        ) -> Result<Charge, IneligibleReason> {
            let bytes = need.requires.bytes.unwrap_or(0);
            if let Some(left) = remaining.bytes {
                if bytes > left {
                    return Err(IneligibleReason::OverCapacity);
                }
            }
            Ok(Charge {
                bytes,
                ..Charge::default()
            })
        }
        fn default_method(&self) -> MethodId {
            crate::params::InForce::builtin(self.id).set().method
        }
    }

    /// Binding on every `PlacementKind` (F1): the scope it requires, and the scope its mishpat
    /// action carries, do not follow the need's own `NeedTerms::Task.scope`. Rewrites `need`'s
    /// declared scope to every `GrantScope` and asserts that neither answer moves.
    ///
    /// # Panics
    /// When `scope()` or `action()` reads the requester's declaration.
    pub fn assert_scope_is_derived(kind: &dyn PlacementKind, need: &Need) {
        let (scope, action) = (kind.scope(need), kind.action(need));
        let mut forged = need.clone();
        for declared in GrantScope::ALL {
            forged.terms = match &need.terms {
                NeedTerms::Task {
                    input_root,
                    base_root,
                    ..
                } => NeedTerms::Task {
                    input_root: input_root.clone(),
                    scope: *declared,
                    base_root: base_root.clone(),
                },
                NeedTerms::None => NeedTerms::Task {
                    input_root: crate::offer::CidString::from(&elohim_epr::cid::compute_cid(
                        b"assert-scope-is-derived",
                    )),
                    scope: *declared,
                    base_root: None,
                },
            };
            assert_eq!(
                kind.scope(&forged),
                scope,
                "{:?}'s scope follows the need's declared {declared:?}",
                kind.id()
            );
            assert_eq!(
                kind.action(&forged),
                action,
                "{:?}'s action follows the need's declared {declared:?}",
                kind.id()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_match_serde() {
        for k in KindId::ALL {
            assert_eq!(
                serde_json::to_string(k).unwrap(),
                format!("\"{}\"", k.label())
            );
        }
        for g in GrantScope::ALL {
            assert_eq!(
                serde_json::to_string(g).unwrap(),
                format!("\"{}\"", g.label())
            );
        }
    }

    #[test]
    fn required_scope_is_one_way_from_the_project() {
        assert_eq!(
            required_scope(Some("a2o-stage:storage")),
            GrantScope::MeasureStage
        );
        assert_eq!(
            required_scope(Some("anything")),
            GrantScope::SweettestFeedback
        );
        assert_eq!(required_scope(None), GrantScope::SweettestFeedback);
    }

    fn task_need(declared: GrantScope) -> Need {
        let task = elohim_epr::cid::compute_cid(b"task");
        Need {
            kind: KindId::Task,
            key: crate::need::NeedKey::Task(task),
            demand: crate::need::Demand {
                holders: 1,
                domains: 1,
            },
            requires: crate::need::Requirements::default(),
            terms: crate::need::NeedTerms::Task {
                input_root: crate::offer::CidString::from(&task),
                scope: declared,
                base_root: None,
            },
            reach: elohim_epr::Reach::Public,
            receiver: elohim_epr_rea::AgentRef("uhCAk-r".into()),
            expressed_by: crate::need::Expresser {
                by: crate::need::ExpresserKind::Agent,
                id: "uhCAk-r".into(),
            },
            class: crate::need::NeedClass::Member,
            in_scope_of: elohim_epr::cid::compute_cid(b"space"),
        }
    }

    /// The double the earlier rounds shipped: its scope was the need's own declaration.
    struct ReadsTheNeed;
    impl PlacementKind for ReadsTheNeed {
        fn id(&self) -> KindId {
            KindId::Task
        }
        fn action(&self, need: &Need) -> MishpatAction {
            MishpatAction::DelegatesCompute {
                scope: self.scope(need).unwrap(),
            }
        }
        fn scope(&self, need: &Need) -> Option<GrantScope> {
            match &need.terms {
                crate::need::NeedTerms::Task { scope, .. } => Some(*scope),
                crate::need::NeedTerms::None => None,
            }
        }
        fn receiver_may_carry(&self) -> bool {
            true
        }
        fn fits(
            &self,
            _: &Need,
            _: &Candidate,
            _: &OfferTerms,
            _: &Capacity,
        ) -> Result<Charge, IneligibleReason> {
            Ok(Charge::default())
        }
        fn default_method(&self) -> MethodId {
            NearestBytes.method()
        }
    }

    #[test]
    fn a_kind_derives_its_scope_and_never_reads_the_requesters() {
        let need = task_need(GrantScope::MeasureStage);
        testing::assert_scope_is_derived(&testing::BytesKind::task(), &need);
        testing::assert_scope_is_derived(&testing::BytesKind::custody(KindId::CustodyRoot), &need);
        let caught = std::panic::catch_unwind(|| {
            testing::assert_scope_is_derived(&ReadsTheNeed, &need);
        });
        assert!(caught.is_err(), "a kind reading the need's scope is caught");
    }

    #[test]
    fn delegates_compute_carries_a_closed_scope() {
        let a = MishpatAction::DelegatesCompute {
            scope: GrantScope::HostedCell,
        };
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(
            json,
            r#"{"action":"delegates-compute","scope":"hosted-cell"}"#
        );
        assert_eq!(serde_json::from_str::<MishpatAction>(&json).unwrap(), a);
        assert!(serde_json::from_str::<MishpatAction>(
            r#"{"action":"delegates-compute","scope":"free-text"}"#
        )
        .is_err());
    }

    #[test]
    fn builtin_registry_holds_the_four_methods() {
        let m = MethodRegistry::builtin();
        for r in [
            &XorDistance as &dyn Ranking,
            &DiversityAware,
            &NearestBytes,
            &WeightedCapacity,
        ] {
            assert!(m.get(r.method()).is_some());
        }
        assert!(MethodRegistry::empty().get(XorDistance.method()).is_none());
    }

    /// An adapter's ranking that claims some id, delegating its order to `XorDistance`.
    struct Claims(MethodId);
    impl Ranking for Claims {
        fn method(&self) -> MethodId {
            self.0
        }
        fn factors(&self) -> &'static [crate::rank::Factor] {
            &[]
        }
        fn rank(
            &self,
            input: &crate::rank::RankInput<'_>,
            eligible: &[crate::eligibility::Eligible<'_>],
            incumbents: &[bool],
            weights: &crate::rank::Weights,
            scratch: &crate::rank::Scratch,
            out: &mut Vec<usize>,
        ) {
            XorDistance.rank(input, eligible, incumbents, weights, scratch, out)
        }
    }

    /// R-P7: every ranking in the crate runs the contract, the test double included.
    #[test]
    fn the_adapter_double_runs_the_contract() {
        crate::rank::contract::assert_ranking_contract(&Claims(MethodId::named(
            "acme:placement:nearest@1",
        )));
    }

    #[test]
    fn a_registered_method_never_replaces_or_impersonates_another() {
        let builtin = "elohim:placement:xor-distance@1";
        // Replacing a built-in under its own id is refused, as is any other elohim: name.
        assert!(MethodRegistry::builtin()
            .with(builtin, Box::new(Claims(MethodId::named(builtin))))
            .is_err());
        let reserved = "elohim:placement:mine@1";
        assert!(MethodRegistry::empty()
            .with(reserved, Box::new(Claims(MethodId::named(reserved))))
            .is_err());
        // An id that is not the name's hash, or a malformed name, is refused.
        let name = "acme:placement:nearest@1";
        assert!(MethodRegistry::empty()
            .with(name, Box::new(Claims(XorDistance.method())))
            .is_err());
        assert!(MethodRegistry::empty()
            .with("Acme@1", Box::new(Claims(MethodId::named("Acme@1"))))
            .is_err());
        // A well-formed adapter name registers once, and only once.
        let m = MethodRegistry::builtin()
            .with(name, Box::new(Claims(MethodId::named(name))))
            .unwrap();
        assert!(m.get(MethodId::named(name)).is_some());
        assert!(m
            .with(name, Box::new(Claims(MethodId::named(name))))
            .is_err());
    }
}
