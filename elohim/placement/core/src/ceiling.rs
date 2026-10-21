//! E — the ceiling: draft, ratification reference, parameter-set head (interfaces only).
//!
//! A member's elohim drafts; the space ratifies under its charter's tuning rule; the floor verifies
//! and elects. Nothing here calls a model, and no draft becomes an [`InForce`](crate::InForce)
//! (rail R-P6, the `compile_fail` doctests on `InForce`).

use cid::Cid;
use elohim_epr::Reach;
use elohim_epr_rea::AgentRef;
use seam_contracts::{Answer, ComeBack, Meanwhile, Unanswered};
use serde::{Deserialize, Serialize};

use crate::decide::Charter;
use crate::floor::PortError;
use crate::need::{Need, NeedKey};
use crate::offer::CidString;
use crate::params::{HeadDeclaration, ParamRefusal, ParameterSet, PlaceError};
use crate::rank::Ranking;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PressureKind {
    NotCarried,
    UnderHeld,
    Churn,
    UnevenCost,
    MemberConcern,
}

/// Counts only; a key is admitted only for the drafter's own person's need at or wider than the
/// draft's reach (E11). Fields private, and `Serialize` only: a pressure enters a [`Draft`] through
/// [`Draft::press_count`] or [`Draft::press_own_need`], which check a key against THAT draft's author
/// and reach, and a received one is a [`PressureInput`] until [`DraftInput::validate`] admits it.
/// (Codex review 1, findings 7 and 10: a `Deserialize` pressure, or one built for an intimate draft
/// and pushed into a public one, bypassed the check.)
///
/// ```compile_fail
/// // A pressure is never read straight off the wire.
/// let _: placement_core::Pressure = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pressure {
    kind: PressureKind,
    key: Option<NeedKey>,
    count: u32,
}

impl Pressure {
    pub fn kind(&self) -> PressureKind {
        self.kind
    }
    pub fn key(&self) -> Option<&NeedKey> {
        self.key.as_ref()
    }
    pub fn n(&self) -> u32 {
        self.count
    }
}

/// The E11 check, against the draft that will carry the key: the need is the drafter's own, and at a
/// reach no narrower than the draft's, so a draft never carries another member's key past where it
/// may be seen.
fn admit_key(need: &Need, drafter: &AgentRef, reach: Reach) -> Result<(), PlaceError> {
    if &need.receiver != drafter {
        return Err(PlaceError(
            "a pressure names only the drafter's own need".into(),
        ));
    }
    if need.reach.openness() < reach.openness() {
        return Err(PlaceError(format!(
            "the need is at {:?}, narrower than the draft's {reach:?}",
            need.reach
        )));
    }
    Ok(())
}

/// Drafted by a member's elohim, named by its layered identity; never by a space-elohim (R5). It
/// carries the space's reach, so it travels to `touches` only within it (E11).
///
/// Fields private: the author and reach a keyed pressure was checked against cannot change after it
/// was admitted (Codex review 1, findings 7 and 10). `Serialize` only; a received draft is a
/// [`DraftInput`].
///
/// ```compile_fail
/// fn widen(d: &mut placement_core::Draft) { d.reach = elohim_epr::Reach::Public; }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    proposed: ParameterSet,
    against: CidString,
    scope: CidString,
    reach: Reach,
    drafted_by: AgentRef,
    lineage: String,
    pressures: Vec<Pressure>,
    touches: Vec<AgentRef>,
}

impl Draft {
    /// A draft with no pressures yet.
    pub fn new(
        proposed: ParameterSet,
        against: CidString,
        scope: CidString,
        reach: Reach,
        drafted_by: AgentRef,
        lineage: String,
        touches: Vec<AgentRef>,
    ) -> Self {
        Self {
            proposed,
            against,
            scope,
            reach,
            drafted_by,
            lineage,
            pressures: Vec::new(),
            touches,
        }
    }
    /// A pressure that names no need: a count.
    pub fn press_count(&mut self, kind: PressureKind, count: u32) {
        self.pressures.push(Pressure {
            kind,
            key: None,
            count,
        });
    }
    /// A pressure naming one need, checked against this draft's own author and reach (E11).
    pub fn press_own_need(&mut self, kind: PressureKind, need: &Need) -> Result<(), PlaceError> {
        admit_key(need, &self.drafted_by, self.reach)?;
        self.pressures.push(Pressure {
            kind,
            key: Some(need.key.clone()),
            count: 1,
        });
        Ok(())
    }
    pub fn proposed(&self) -> &ParameterSet {
        &self.proposed
    }
    pub fn against(&self) -> &CidString {
        &self.against
    }
    pub fn scope(&self) -> &CidString {
        &self.scope
    }
    pub fn reach(&self) -> Reach {
        self.reach
    }
    pub fn drafted_by(&self) -> &AgentRef {
        &self.drafted_by
    }
    pub fn lineage(&self) -> &str {
        &self.lineage
    }
    pub fn pressures(&self) -> &[Pressure] {
        &self.pressures
    }
    pub fn touches(&self) -> &[AgentRef] {
        &self.touches
    }
}

/// A pressure as received, unchecked. Becomes a [`Pressure`] only inside [`DraftInput::validate`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PressureInput {
    pub kind: PressureKind,
    pub key: Option<NeedKey>,
    pub count: u32,
}

/// A draft as received (the wire form of [`Draft`]), unchecked. Nothing publishes it:
/// [`TuningSink::propose`] takes a [`Draft`], which only [`DraftInput::validate`] yields.
///
/// ```
/// // The unchecked input types are what the wire reads into.
/// let p: placement_core::PressureInput =
///     serde_json::from_str(r#"{"kind":"churn","key":null,"count":3}"#).unwrap();
/// assert_eq!(p.count, 3);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftInput {
    pub proposed: ParameterSet,
    pub against: CidString,
    pub scope: CidString,
    pub reach: Reach,
    pub drafted_by: AgentRef,
    pub lineage: String,
    pub pressures: Vec<PressureInput>,
    pub touches: Vec<AgentRef>,
}

impl DraftInput {
    /// Admit each keyed pressure only against a need in `own_needs` with that key, the drafter's own
    /// and at or wider than the draft's reach (E11); a keyed pressure counts one need, so its count is
    /// 1. A key that names no such need is refused, never passed through.
    pub fn validate(self, own_needs: &[Need]) -> Result<Draft, PlaceError> {
        let mut d = Draft::new(
            self.proposed,
            self.against,
            self.scope,
            self.reach,
            self.drafted_by,
            self.lineage,
            self.touches,
        );
        for p in self.pressures {
            match p.key {
                None => d.press_count(p.kind, p.count),
                Some(key) => {
                    if p.count != 1 {
                        return Err(PlaceError("a keyed pressure counts one need".into()));
                    }
                    let mine: Vec<&Need> = own_needs
                        .iter()
                        .filter(|n| n.key == key && n.receiver == d.drafted_by)
                        .collect();
                    let Some(first) = mine.first() else {
                        return Err(PlaceError(
                            "a pressure names only the drafter's own need".into(),
                        ));
                    };
                    let need = mine
                        .iter()
                        .find(|n| admit_key(n, &d.drafted_by, d.reach).is_ok())
                        .unwrap_or(first);
                    d.press_own_need(p.kind, need)?;
                }
            }
        }
        Ok(d)
    }
}

fn band(why: ParamRefusal) -> Unanswered<ParamRefusal> {
    Unanswered::new(
        why,
        ComeBack::OnEvent {
            event: "draft-revised".into(),
        },
        Meanwhile::UseHeld {
            what: "the set in force".into(),
        },
    )
}

/// A tuning may only reweight (E2, E5, E6): same kind and method; the same weight keys, every key a
/// factor `method.factors()` lists, every weight > 0; `max_moves_per_event` inside the charter's
/// range. Whether a set leaves some member's carrier unchosen for every need (an exclusion) is not
/// detectable here; it is a ratification question.
///
/// A revert (proposed equals a set this space verified before) is exempt from the band and is
/// checked against that set; this function does not know the space's history, so the caller routes a
/// revert past it (`InForce::from_head` still verifies the revert's ratification).
pub fn within_band(
    head: &ParameterSet,
    proposed: &ParameterSet,
    method: &dyn Ranking,
    charter: &Charter,
) -> Result<(), Unanswered<ParamRefusal>> {
    if proposed.kind != head.kind
        || proposed.method != head.method
        || method.method() != proposed.method
    {
        return Err(band(ParamRefusal::Band {
            rule: "same-kind-and-method".into(),
        }));
    }
    if proposed
        .weights
        .iter()
        .any(|(f, _)| !method.factors().contains(&f))
    {
        return Err(band(ParamRefusal::UnknownFactor));
    }
    let keys = |s: &ParameterSet| s.weights.iter().map(|(f, _)| f).collect::<Vec<_>>();
    if keys(proposed) != keys(head) {
        return Err(band(ParamRefusal::Band {
            rule: "same-weight-keys".into(),
        }));
    }
    structural_band(proposed, Some(method), charter).map_err(band)
}

/// The band checks that need no history, applied to every set that goes into force, not only to
/// drafts (`InForce::from_head` runs this for `Charter` and `Ratified` sets): every weight > 0;
/// when this peer runs the method, the weight keys are exactly `method.factors()`; and
/// `max_moves_per_event` inside the charter's range (`lo <= hi`). With no method here the factor
/// check is skipped: this peer only observes a need under that method (`MethodNotRun`), so the
/// factors it cannot read never rank anything on it.
pub(crate) fn structural_band(
    set: &ParameterSet,
    method: Option<&dyn Ranking>,
    charter: &Charter,
) -> Result<(), ParamRefusal> {
    if let Some(m) = method {
        if set.weights.iter().any(|(f, _)| !m.factors().contains(&f)) {
            return Err(ParamRefusal::UnknownFactor);
        }
        if m.factors().iter().any(|f| set.weights.get(*f).is_none()) {
            return Err(ParamRefusal::Band {
                rule: "every-factor-weighted".into(),
            });
        }
    }
    if set.weights.iter().any(|(_, w)| w <= 0) {
        return Err(ParamRefusal::Band {
            rule: "weight-positive".into(),
        });
    }
    let (lo, hi) = charter.moves_per_event;
    if lo > hi || set.max_moves_per_event < lo || set.max_moves_per_event > hi {
        return Err(ParamRefusal::Band {
            rule: "move-range".into(),
        });
    }
    Ok(())
}

/// Implemented by the adapter that reads the space's declared head: EVERY candidate declaration it
/// holds (E3), so election happens after verification, in core.
pub trait ParameterHead: Send + Sync {
    fn declarations(&self, scope: &Cid) -> Answer<Vec<HeadDeclaration>>;
}

/// Implemented by the adapter for `Ratified` only: do the ratifying act's signers satisfy the
/// charter's `tuning_rule` for these touches, AND does that act name exactly this head's `scope`,
/// `sets` and `ratified_at`? Re-derived by every floor (concern C5).
///
/// The whole head is handed over, not only its `RatificationRef`, so a valid act for set X cannot be
/// attached to a head naming set Y, and `ratified_at` (the election clock, E4) is a fact the act
/// proves rather than one the declarer writes. Core has already checked the head's scope, kind and
/// set CID against the set it fetched.
pub trait RatificationCheck: Send + Sync {
    fn verify(
        &self,
        head: &HeadDeclaration,
        charter: &Charter,
    ) -> Result<(), Unanswered<ParamRefusal>>;
}

/// Implemented by a member's elohim. Nothing in placement-core calls a model.
pub trait TuningSink: Send + Sync {
    fn propose(&self, draft: &Draft) -> Result<String, PortError>;
}
