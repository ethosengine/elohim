//! E — the parameter set, its head and the one form the floor accepts.
//!
//! The ceiling changes the floor's inputs, never its outcome (spec §7.3). R5's three authorities each
//! live in exactly one type: the carrier's `Posture`, the space's `Charter` (demand and domains, serve
//! order, holding floor, forwarding rule, the tuning rule and the move range), and the parameter set
//! (`InForce`). Rail R-P3: weights are integer basis points.

use std::collections::BTreeMap;

use cid::Cid;
use elohim_epr::Reach;
use elohim_epr_rea::atom_cid;
use seam_contracts::{
    select_arbitrated_winner, Answer, ArbitrationKey, ComeBack, Meanwhile, ReasonLabel, Unanswered,
};
use serde::{Deserialize, Serialize};

use crate::ceiling::{structural_band, RatificationCheck};
use crate::decide::Charter;
use crate::kind::{KindId, MethodRegistry};
use crate::need::AgentRefKey;
use crate::offer::{CidString, EpochSecs};
use crate::rank::{
    Factor, MethodId, NearestBytes, Ranking, WeightedCapacity, Weights, XorDistance,
};

/// The version of `ParameterSet`'s canonical encoding this floor knows.
pub const PARAMETER_SET_SCHEMA: u32 = 1;

/// The built-in move cap. The spec leaves `max_moves_per_event` defaults open (§6); one is the
/// cautious placeholder.
pub const BUILTIN_MAX_MOVES_PER_EVENT: u16 = 1;

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[error("{0}")]
pub struct PlaceError(pub String);

/// Scope-free, so the built-in set has one CID everywhere and ships with the floor (E1). Its CID is
/// over {schema, kind, method, weights, max_moves_per_event} only; where it is in force is the head's
/// fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParameterSet {
    /// The version of this struct's canonical encoding: 1 for the fields below, and bumped only when
    /// a field is added, removed or reinterpreted. Part of `cid()` and not of `ranking_cid()`.
    pub schema: u32,
    pub kind: KindId,
    pub method: MethodId,
    /// Every factor the method reads, each > 0 (E2, E6).
    pub weights: Weights,
    /// C6a; within the charter's declared range (E6).
    pub max_moves_per_event: u16,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RankingIdentity<'a> {
    kind: KindId,
    method: &'a MethodId,
    weights: &'a Weights,
}

impl ParameterSet {
    /// `atom_cid` of the canonical encoding (E17).
    pub fn cid(&self) -> Result<Cid, PlaceError> {
        atom_cid(self).map_err(|e| PlaceError(format!("parameter set encoding: {e}")))
    }
    /// The incumbency key (E5): over (kind, method, weights) only, so republishing a set at another
    /// reach or move cap, or under a new schema version, does not reset incumbency.
    pub fn ranking_cid(&self) -> Result<Cid, PlaceError> {
        atom_cid(&RankingIdentity {
            kind: self.kind,
            method: &self.method,
            weights: &self.weights,
        })
        .map_err(|e| PlaceError(format!("ranking identity encoding: {e}")))
    }
}

/// The space's head (E14): one per space, naming one set per kind. The scope and the predecessor
/// live here, not in the set (E1, E5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HeadDeclaration {
    #[serde(with = "elohim_epr_rea::index::cid_serde")]
    pub scope: Cid,
    pub sets: BTreeMap<KindId, CidString>,
    /// Published at (who may see it); in force for the space's members.
    pub reach: Reach,
    /// The head declaration this one follows.
    pub predecessor: Option<CidString>,
    pub ratification: RatificationRef,
    /// The ratification's notarized timestamp (spec C2; E4).
    pub ratified_at: EpochSecs,
}

impl HeadDeclaration {
    pub fn cid(&self) -> Result<Cid, PlaceError> {
        atom_cid(self).map_err(|e| PlaceError(format!("head encoding: {e}")))
    }
}

/// A struct with a kind field rather than an internally tagged enum carrying CIDs, so its JSON and
/// DAG-CBOR forms are both honest (G1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RatificationRef {
    pub by: RatificationKind,
    /// The record hash, or the charter CID.
    pub id: String,
    pub rule: Option<CidString>,
    pub touches: Vec<AgentRefKey>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RatificationKind {
    /// The floor's built-in sets, content-addressed and shipped with the floor (spec §7.3). Nobody
    /// put them in force; they run when nothing has been. Explanations say "no tuning chosen".
    BuiltIn,
    /// Sets the space's charter itself names (`Charter::genesis_params`); the charter's adoption
    /// ratified them.
    Charter,
    /// A tuning ratified under the charter's `tuning_rule`: id = the ratifying act's hash; rule = the
    /// clause for `Clause`; touches = whose consent `TouchedConsent` requires (E8).
    Ratified,
}

/// How a space ratifies a tuning (E8). The household default is each touched member's consent (the
/// design gate); a charter may name a clause and require drafts from several lineages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RatificationRule {
    TouchedConsent,
    Clause { cid: Cid, min_lineages: u16 },
}

/// The ONLY parameter form the floor accepts. Fields private; the CIDs are computed at construction.
///
/// Rail R-P6: a draft never reaches the floor, because nothing converts one:
///
/// ```compile_fail
/// use placement_core::{Draft, InForce};
/// fn promote(d: Draft) -> InForce { d.into() }
/// ```
///
/// and a literal cannot be written outside the crate:
///
/// ```compile_fail
/// use placement_core::{InForce, KindId};
/// fn forge() -> InForce {
///     let b = InForce::builtin(KindId::CustodyRoot);
///     InForce { set: b.set().clone(), cid: *b.cid(), ranking_cid: *b.ranking_cid(), ratification: b.ratification().clone() }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InForce {
    set: ParameterSet,
    cid: Cid,
    ranking_cid: Cid,
    ratification: RatificationRef,
    /// The scope this set was verified for (`from_head`); `None` for the built-in, which is
    /// scope-free and binds every scope. Private: a verified set's authority is never moved to
    /// another scope by a map entry or a retained copy (Codex review 2, finding 9).
    scope: Option<Cid>,
    /// Where this set came from when it is not a head elected now (E10, §E contract 3):
    /// `LastVerified` when `select_in_force` answered with the retained set because no declaration
    /// verified. `None` for an elected head and for the built-in, whose ratification already says
    /// `BuiltIn`. Codex review 1, finding 4.
    fallback: Option<ParamFallback>,
}

/// The built-in set for a kind (E2).
pub fn builtin_set(kind: KindId) -> ParameterSet {
    let (method, weights) = match kind {
        KindId::CustodyBlob => (XorDistance.method(), Weights::new()),
        KindId::Task => (NearestBytes.method(), Weights::new()),
        _ => (
            WeightedCapacity.method(),
            weights_of(WeightedCapacity::DEFAULT_WEIGHTS),
        ),
    };
    ParameterSet {
        schema: PARAMETER_SET_SCHEMA,
        kind,
        method,
        weights,
        max_moves_per_event: BUILTIN_MAX_MOVES_PER_EVENT,
    }
}

fn weights_of(pairs: &[(Factor, i32)]) -> Weights {
    pairs
        .iter()
        .fold(Weights::new(), |w, (f, bp)| w.with(*f, *bp))
}

fn refuse(why: ParamRefusal) -> Unanswered<ParamRefusal> {
    Unanswered::new(
        why,
        ComeBack::OnEvent {
            event: "parameters-declared".into(),
        },
        Meanwhile::BuiltInInForce,
    )
}

impl InForce {
    fn build(set: ParameterSet, ratification: RatificationRef) -> Result<Self, PlaceError> {
        let cid = set.cid()?;
        let ranking_cid = set.ranking_cid()?;
        Ok(Self {
            set,
            cid,
            ranking_cid,
            ratification,
            scope: None,
            fallback: None,
        })
    }

    /// Whether this set is authority for `kind` in `scope`: its own kind, and either the built-in
    /// (scope-free) or a set verified for exactly that scope. `decide` consumes a held set and
    /// `select_in_force` retains one only where this holds.
    pub fn binds(&self, scope: &Cid, kind: KindId) -> bool {
        self.set.kind == kind && self.scope.as_ref().is_none_or(|s| s == scope)
    }

    /// The scope this set was verified for; `None` for the built-in. Additive to the spec.
    pub fn scope(&self) -> Option<&Cid> {
        self.scope.as_ref()
    }

    /// Which fallback put this set in force, when one did (see the field). Additive to the spec.
    pub fn fallback(&self) -> Option<ParamFallback> {
        self.fallback
    }

    /// The built-in set for a kind: `CustodyBlob` → `XorDistance` (the legacy method, reading no
    /// weights, so no tuning is in band under it); `CustodyRoot` and the other custody-shaped kinds →
    /// `WeightedCapacity` with every factor it reads present and non-zero; `Task` → `NearestBytes`,
    /// which also reads no weight.
    pub fn builtin(kind: KindId) -> Self {
        let set = builtin_set(kind);
        let id = set.cid().map(|c| c.to_string()).unwrap_or_default();
        Self::build(
            set,
            RatificationRef {
                by: RatificationKind::BuiltIn,
                id,
                rule: None,
                touches: Vec::new(),
            },
        )
        .expect("a built-in parameter set always encodes")
    }

    /// Verifies `BuiltIn` against `builtin(set.kind).cid` and `Charter` against `charter.id` and
    /// `charter.genesis_params[set.kind]` IN CORE; only `Ratified` is handed to `check`, with the
    /// whole head, so the act must name this head's scope, sets and `ratified_at` (E9). Refuses a set
    /// whose kind is not `kind` or whose head names another scope (E15).
    ///
    /// `Charter` and `Ratified` sets are also held to the band checks that need no history
    /// (`structural_band`: weights > 0, the weight keys exactly the method's factors when `methods`
    /// holds the method, `max_moves_per_event` inside the charter's range), so no ratification,
    /// however lenient the adapter's check, puts an off-band set in force. `methods` revises the
    /// spec's signature for that.
    pub fn from_head(
        scope: &Cid,
        kind: KindId,
        set: ParameterSet,
        head: &HeadDeclaration,
        check: &dyn RatificationCheck,
        methods: &MethodRegistry,
        charter: &Charter,
    ) -> Result<Self, Unanswered<ParamRefusal>> {
        if set.schema != PARAMETER_SET_SCHEMA {
            return Err(refuse(ParamRefusal::Unverifiable {
                input: "schema".into(),
            }));
        }
        if set.kind != kind || head.scope != *scope {
            return Err(refuse(ParamRefusal::Mismatch));
        }
        let set_cid = set.cid().map_err(|_| {
            refuse(ParamRefusal::Unverifiable {
                input: "set".into(),
            })
        })?;
        if head.sets.get(&kind) != Some(&CidString::from(&set_cid)) {
            return Err(refuse(ParamRefusal::Mismatch));
        }
        // Neither the built-in nor a charter's genesis set is ratified by an act that could name
        // its own `rule` or `touches`, so a head claiming either carries none; anything else there
        // would be copied into `ratification()` as provenance nobody proved.
        let bare = head.ratification.rule.is_none() && head.ratification.touches.is_empty();
        match head.ratification.by {
            RatificationKind::BuiltIn => {
                if set != builtin_set(kind) || head.ratification.id != set_cid.to_string() || !bare
                {
                    return Err(refuse(ParamRefusal::Refused));
                }
                // The canonical built-in, never the head's copy of its provenance.
                return Ok(Self::builtin(kind));
            }
            RatificationKind::Charter => {
                let named = charter.id.map(|c| c.to_string()) == Some(head.ratification.id.clone())
                    && charter.genesis_params.get(&kind) == Some(&set_cid);
                if !named || !bare {
                    return Err(refuse(ParamRefusal::Refused));
                }
            }
            RatificationKind::Ratified => check.verify(head, charter)?,
        }
        structural_band(&set, methods.get(set.method), charter).map_err(refuse)?;
        let mut verified = Self::build(set, head.ratification.clone()).map_err(|_| {
            refuse(ParamRefusal::Unverifiable {
                input: "set".into(),
            })
        })?;
        verified.scope = Some(*scope);
        Ok(verified)
    }

    pub fn set(&self) -> &ParameterSet {
        &self.set
    }
    pub fn cid(&self) -> &Cid {
        &self.cid
    }
    pub fn ranking_cid(&self) -> &Cid {
        &self.ranking_cid
    }
    pub fn ratification(&self) -> &RatificationRef {
        &self.ratification
    }
    pub fn max_moves_per_event(&self) -> u16 {
        self.set.max_moves_per_event
    }
}

/// R7's typed reason for the ceiling (E10); every refusal arrives as `Unanswered<ParamRefusal>`,
/// with come_back `OnEvent("parameters-declared")` or the missing record's arrival, and meanwhile
/// `BuiltInInForce` or `UseHeld { the last verified set }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "reason"
)]
#[non_exhaustive]
pub enum ParamRefusal {
    Mismatch,
    Refused,
    Unverifiable { input: String },
    UnknownFactor,
    UnknownMethod,
    Band { rule: String },
}

/// The `Copy` face of [`ParamRefusal`], for `ReasonLabel` (§2.0's contract: every seam's reason is a
/// `ReasonLabel`). Additive to the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamRefusalKind {
    Mismatch,
    Refused,
    Unverifiable,
    UnknownFactor,
    UnknownMethod,
    Band,
}

impl ReasonLabel for ParamRefusalKind {
    const ALL: &'static [Self] = &[
        ParamRefusalKind::Mismatch,
        ParamRefusalKind::Refused,
        ParamRefusalKind::Unverifiable,
        ParamRefusalKind::UnknownFactor,
        ParamRefusalKind::UnknownMethod,
        ParamRefusalKind::Band,
    ];
    fn label(&self) -> &'static str {
        match self {
            ParamRefusalKind::Mismatch => "mismatch",
            ParamRefusalKind::Refused => "refused",
            ParamRefusalKind::Unverifiable => "unverifiable",
            ParamRefusalKind::UnknownFactor => "unknown-factor",
            ParamRefusalKind::UnknownMethod => "unknown-method",
            ParamRefusalKind::Band => "band",
        }
    }
}

impl ParamRefusal {
    pub fn kind(&self) -> ParamRefusalKind {
        match self {
            ParamRefusal::Mismatch => ParamRefusalKind::Mismatch,
            ParamRefusal::Refused => ParamRefusalKind::Refused,
            ParamRefusal::Unverifiable { .. } => ParamRefusalKind::Unverifiable,
            ParamRefusal::UnknownFactor => ParamRefusalKind::UnknownFactor,
            ParamRefusal::UnknownMethod => ParamRefusalKind::UnknownMethod,
            ParamRefusal::Band { .. } => ParamRefusalKind::Band,
        }
    }
}

/// Which fallback an explanation reports (E10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParamFallback {
    BuiltIn,
    LastVerified,
}

/// The source of parameter sets by CID.
pub trait SetSource {
    fn set(&self, cid: &Cid) -> Answer<ParameterSet>;
}

/// The election key of a head (E4): one tier, the clock a proven fact. A `Ratified` head's
/// `ratified_at` is proven by `check.verify`, which binds the act to this head's scope, sets and
/// `ratified_at` (E9). A `Charter` head verifies only against the charter in force, so its clock is
/// that charter's `adopted_at`, held by core; the head's own `ratified_at` is never read. A
/// `BuiltIn` head is never elected (it says no tuning was chosen), so its clock is 0. Newest wins:
/// a charter amendment naming a new genesis set displaces an older tuning, and a later ratified
/// tuning displaces it in turn, while an inflated clock on a head nothing proves cannot win.
/// (Fix round 2's tiers, `Ratified` over `Charter`, are withdrawn: they made a tuning outrank the
/// constitution it was made under.)
fn election_key(
    decl: &HeadDeclaration,
    charter: &Charter,
    head_cid: Vec<u8>,
) -> ArbitrationKey<(), EpochSecs, Vec<u8>> {
    let clock = match decl.ratification.by {
        RatificationKind::BuiltIn => 0,
        RatificationKind::Charter => charter.adopted_at,
        RatificationKind::Ratified => decl.ratified_at,
    };
    ArbitrationKey {
        tier: (),
        clock,
        tiebreak: head_cid,
    }
}

/// Pure (E3, E13): filter the declarations with `check` FIRST, then elect among the verified ones
/// only, with `select_arbitrated_winner` over [`election_key`]: the newest proven clock wins, a
/// `Charter` head at its charter's adoption and a `Ratified` head at its ratification (E4). A verified
/// `BuiltIn` head is not elected at all: it says no tuning was chosen, so it never displaces
/// `last_verified`, and the built-in that answers carries the canonical provenance. An unverifiable
/// newer declaration never shadows an older verified one. The second value is the refusal of the
/// newest declaration ranking above the one elected, so an explanation can say what was set aside.
// The spec's seven inputs plus `methods`, which the structural band check needs (fix round 1).
#[allow(clippy::too_many_arguments)]
pub fn select_in_force(
    scope: &Cid,
    kind: KindId,
    decls: &[HeadDeclaration],
    sets: &dyn SetSource,
    check: &dyn RatificationCheck,
    methods: &MethodRegistry,
    charter: &Charter,
    last_verified: Option<&InForce>,
) -> (InForce, Option<Unanswered<ParamRefusal>>) {
    type Key = ArbitrationKey<(), EpochSecs, Vec<u8>>;
    let mut verified: Vec<(InForce, Key)> = Vec::new();
    let mut refused: Vec<(Key, Unanswered<ParamRefusal>)> = Vec::new();
    for decl in decls.iter().filter(|d| d.scope == *scope) {
        let Some(set_cid) = decl.sets.get(&kind) else {
            continue;
        };
        let key = election_key(
            decl,
            charter,
            decl.cid().map(|c| c.to_bytes()).unwrap_or_default(),
        );
        let outcome = match set_cid.to_cid() {
            Err(_) => Err(refuse(ParamRefusal::Unverifiable {
                input: "set".into(),
            })),
            Ok(cid) => match sets.set(&cid) {
                Answer::Present(set) => {
                    InForce::from_head(scope, kind, set, decl, check, methods, charter)
                }
                Answer::Absent | Answer::Unreachable => Err(refuse(ParamRefusal::Unverifiable {
                    input: "set".into(),
                })),
            },
        };
        match outcome {
            // A verified built-in head is "no tuning chosen": not a candidate.
            Ok(_) if decl.ratification.by == RatificationKind::BuiltIn => {}
            Ok(f) => verified.push((f, key)),
            Err(u) => refused.push((key, u)),
        }
    }
    let winner = select_arbitrated_winner(verified, |(_, key)| key.clone());
    let newer_refusal = |above: Option<&Key>| {
        refused
            .iter()
            .filter(|(key, _)| above.is_none_or(|w| key > w))
            .max_by(|a, b| a.0.cmp(&b.0))
            .map(|(_, u)| u.clone())
    };
    match winner {
        Some((f, key)) => {
            let refusal = newer_refusal(Some(&key)).map(|mut u| {
                u.meanwhile = Meanwhile::UseHeld {
                    what: "the newest verified parameter set".into(),
                };
                u
            });
            (f, refusal)
        }
        None => {
            // The retained set says it is retained, so an explanation never reads it as a
            // currently verified election (§E contract 3). A retained built-in stays `BuiltIn`.
            let fallback = last_verified
                .filter(|f| f.binds(scope, kind))
                .cloned()
                .map(|mut f| {
                    if f.ratification.by != RatificationKind::BuiltIn {
                        f.fallback = Some(ParamFallback::LastVerified);
                    }
                    f
                });
            let refusal = newer_refusal(None).map(|mut u| {
                if fallback.is_some() {
                    u.meanwhile = Meanwhile::UseHeld {
                        what: "the last verified parameter set".into(),
                    };
                }
                u
            });
            (fallback.unwrap_or_else(|| InForce::builtin(kind)), refusal)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_sets_are_scope_free_and_stable() {
        for k in KindId::ALL {
            let a = InForce::builtin(*k);
            assert_eq!(a, InForce::builtin(*k));
            assert_eq!(a.ratification().by, RatificationKind::BuiltIn);
            assert_eq!(a.ratification().id, a.cid().to_string());
            for (_, w) in a.set().weights.iter() {
                assert!(w > 0);
            }
        }
        assert_eq!(
            InForce::builtin(KindId::CustodyBlob).set().method,
            XorDistance.method()
        );
        assert!(InForce::builtin(KindId::CustodyBlob)
            .set()
            .weights
            .is_empty());
        assert_eq!(
            InForce::builtin(KindId::CustodyRoot).set().method,
            WeightedCapacity.method()
        );
        assert_eq!(
            InForce::builtin(KindId::Task).set().method,
            NearestBytes.method()
        );
    }

    #[test]
    fn ranking_cid_ignores_schema_and_move_cap() {
        let a = builtin_set(KindId::CustodyRoot);
        let mut b = a.clone();
        b.max_moves_per_event = 9;
        b.schema = 2;
        assert_ne!(a.cid().unwrap(), b.cid().unwrap());
        assert_eq!(a.ranking_cid().unwrap(), b.ranking_cid().unwrap());
        let mut c = a.clone();
        c.weights = c.weights.with(Factor::AlwaysOn, 1);
        assert_ne!(a.ranking_cid().unwrap(), c.ranking_cid().unwrap());
    }

    #[test]
    fn parameter_set_json_names_weights_by_factor() {
        let json = serde_json::to_string(&builtin_set(KindId::CustodyRoot)).unwrap();
        assert!(
            json.contains(r#""weights":{"distance":5000,"spare-capacity":3000,"always-on":2000}"#),
            "{json}"
        );
        let back: ParameterSet = serde_json::from_str(&json).unwrap();
        assert_eq!(
            back.cid().unwrap(),
            builtin_set(KindId::CustodyRoot).cid().unwrap()
        );
    }

    #[test]
    fn param_refusal_labels_conform() {
        seam_contracts::assert_reason_labels_conformant::<ParamRefusalKind>();
    }
}
