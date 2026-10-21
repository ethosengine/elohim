//! Contract for E (interfaces spec §E, "Contract", E13), with doubles.

use std::collections::BTreeMap;

use cid::Cid;
use elohim_epr::Reach;
use elohim_epr_rea::AgentRef;
use placement_core::*;
use seam_contracts::{Answer, Meanwhile, Unanswered};

fn scope() -> Cid {
    elohim_epr::cid::compute_cid(b"household-space")
}

fn charter() -> Charter {
    Charter {
        id: None,
        adopted_at: 0,
        demand: BTreeMap::new(),
        serve_order: Vec::new(),
        holding_floor: Demand {
            holders: 1,
            domains: 1,
        },
        forwarding: None,
        genesis_params: BTreeMap::new(),
        tuning_rule: RatificationRule::TouchedConsent,
        moves_per_event: (1, 4),
        freshness_horizon_secs: 600,
    }
}

/// The ratifying acts it holds, each naming the head it ratified (scope, sets, `ratified_at`), as a
/// real adapter reads them from the ratifying record: a head is verified only when an act names
/// exactly that head.
struct Acts(Vec<HeadDeclaration>);
impl RatificationCheck for Acts {
    fn verify(&self, h: &HeadDeclaration, _c: &Charter) -> Result<(), Unanswered<ParamRefusal>> {
        let named = self.0.iter().any(|a| {
            a.ratification.id == h.ratification.id
                && a.scope == h.scope
                && a.sets == h.sets
                && a.ratified_at == h.ratified_at
        });
        if named {
            Ok(())
        } else {
            Err(Unanswered::new(
                ParamRefusal::Refused,
                seam_contracts::ComeBack::OnEvent {
                    event: "ratification-recorded".into(),
                },
                Meanwhile::BuiltInInForce,
            ))
        }
    }
}

/// Accepts anything: core must still refuse a forged BuiltIn or Charter claim.
struct Anything;
impl RatificationCheck for Anything {
    fn verify(&self, _h: &HeadDeclaration, _c: &Charter) -> Result<(), Unanswered<ParamRefusal>> {
        Ok(())
    }
}

struct Sets(BTreeMap<Cid, ParameterSet>);
impl SetSource for Sets {
    fn set(&self, cid: &Cid) -> Answer<ParameterSet> {
        self.0
            .get(cid)
            .cloned()
            .map(Answer::Present)
            .unwrap_or(Answer::Absent)
    }
}

fn tuned(spare: i32) -> ParameterSet {
    let mut s = builtin_set(KindId::CustodyRoot);
    s.weights = Weights::new()
        .with(Factor::Distance, 5_000)
        .with(Factor::SpareCapacity, spare)
        .with(Factor::AlwaysOn, 2_000);
    s
}

fn head(set: &ParameterSet, by: RatificationKind, act: &str, at: u64) -> HeadDeclaration {
    HeadDeclaration {
        scope: scope(),
        sets: [(set.kind, CidString::from(&set.cid().unwrap()))]
            .into_iter()
            .collect(),
        reach: Reach::Intimate,
        predecessor: None,
        ratification: RatificationRef {
            by,
            id: act.into(),
            rule: None,
            touches: Vec::new(),
        },
        ratified_at: at,
    }
}

fn sets(list: &[&ParameterSet]) -> Sets {
    Sets(
        list.iter()
            .map(|s| (s.cid().unwrap(), (*s).clone()))
            .collect(),
    )
}

#[test]
fn e1_a_lenient_check_cannot_admit_a_draft_as_builtin_or_charter() {
    let forged = tuned(9_000);
    for by in [RatificationKind::BuiltIn, RatificationKind::Charter] {
        let h = head(&forged, by, "anything", 5);
        let refusal = InForce::from_head(
            &scope(),
            KindId::CustodyRoot,
            forged.clone(),
            &h,
            &Anything,
            &MethodRegistry::builtin(),
            &charter(),
        )
        .unwrap_err();
        assert_eq!(
            refusal.why,
            ParamRefusal::Refused,
            "{by:?} is verified in core"
        );
    }
    // The genuine built-in verifies in core with no check at all.
    let b = builtin_set(KindId::CustodyRoot);
    let h = head(
        &b,
        RatificationKind::BuiltIn,
        &b.cid().unwrap().to_string(),
        1,
    );
    assert!(InForce::from_head(
        &scope(),
        KindId::CustodyRoot,
        b,
        &h,
        &Acts(vec![]),
        &MethodRegistry::builtin(),
        &charter()
    )
    .is_ok());
    // A charter's genesis set verifies against the charter's own id and list.
    let genesis = tuned(4_000);
    let mut c = charter();
    let charter_cid = elohim_epr::cid::compute_cid(b"charter");
    c.id = Some(charter_cid);
    c.genesis_params
        .insert(KindId::CustodyRoot, genesis.cid().unwrap());
    let h = head(
        &genesis,
        RatificationKind::Charter,
        &charter_cid.to_string(),
        2,
    );
    assert!(InForce::from_head(
        &scope(),
        KindId::CustodyRoot,
        genesis,
        &h,
        &Acts(vec![]),
        &MethodRegistry::builtin(),
        &c
    )
    .is_ok());
}

#[test]
fn e1_a_set_for_another_scope_or_kind_is_refused() {
    let set = tuned(9_000);
    let mut h = head(&set, RatificationKind::Ratified, "act", 5);
    h.scope = elohim_epr::cid::compute_cid(b"elsewhere");
    let r = InForce::from_head(
        &scope(),
        KindId::CustodyRoot,
        set.clone(),
        &h,
        &Anything,
        &MethodRegistry::builtin(),
        &charter(),
    )
    .unwrap_err();
    assert_eq!(r.why, ParamRefusal::Mismatch);
    let h = head(&set, RatificationKind::Ratified, "act", 5);
    let r = InForce::from_head(
        &scope(),
        KindId::CustodyBlob,
        set,
        &h,
        &Anything,
        &MethodRegistry::builtin(),
        &charter(),
    )
    .unwrap_err();
    assert_eq!(r.why, ParamRefusal::Mismatch);
}

#[test]
fn e3_an_unratified_head_is_never_in_force() {
    let older = tuned(4_000);
    let newer = tuned(9_000);
    let decls = vec![
        head(&older, RatificationKind::Ratified, "act-old", 10),
        head(&newer, RatificationKind::Ratified, "act-forged", 20),
    ];
    let (f, refusal) = select_in_force(
        &scope(),
        KindId::CustodyRoot,
        &decls,
        &sets(&[&older, &newer]),
        &Acts(vec![decls[0].clone()]),
        &MethodRegistry::builtin(),
        &charter(),
        None,
    );
    assert_eq!(f.set(), &older, "the newest VERIFIED declaration wins");
    let refusal = refusal.expect("the newer refusal is reported");
    assert_eq!(refusal.why, ParamRefusal::Refused);
    assert!(matches!(refusal.meanwhile, Meanwhile::UseHeld { .. }));
}

#[test]
fn e3_no_declaration_leaves_the_last_verified_or_the_builtin() {
    let older = tuned(4_000);
    let held = InForce::from_head(
        &scope(),
        KindId::CustodyRoot,
        older.clone(),
        &head(&older, RatificationKind::Ratified, "act-old", 10),
        &Anything,
        &MethodRegistry::builtin(),
        &charter(),
    )
    .unwrap();
    let (f, r) = select_in_force(
        &scope(),
        KindId::CustodyRoot,
        &[],
        &sets(&[]),
        &Anything,
        &MethodRegistry::builtin(),
        &charter(),
        Some(&held),
    );
    // The retained set answers, and says it is retained (§E contract 3; Codex review 1, finding 4).
    assert_eq!(
        (f.set(), f.cid(), f.ratification()),
        (held.set(), held.cid(), held.ratification())
    );
    assert_eq!(f.fallback(), Some(ParamFallback::LastVerified));
    assert_eq!(held.fallback(), None);
    assert!(r.is_none());
    let (f, _) = select_in_force(
        &scope(),
        KindId::CustodyRoot,
        &[],
        &sets(&[]),
        &Anything,
        &MethodRegistry::builtin(),
        &charter(),
        None,
    );
    assert_eq!(f, InForce::builtin(KindId::CustodyRoot));
    // A declaration whose set cannot be fetched is unverifiable and leaves the fallback.
    let decls = vec![head(&older, RatificationKind::Ratified, "act-old", 10)];
    let (f, r) = select_in_force(
        &scope(),
        KindId::CustodyRoot,
        &decls,
        &sets(&[]),
        &Anything,
        &MethodRegistry::builtin(),
        &charter(),
        None,
    );
    assert_eq!(f, InForce::builtin(KindId::CustodyRoot));
    assert_eq!(
        r.unwrap().why,
        ParamRefusal::Unverifiable {
            input: "set".into()
        }
    );
}

#[test]
fn e4_a_revert_is_a_newer_declaration_of_the_earlier_set() {
    let a = tuned(4_000);
    let b = tuned(9_000);
    let decls = vec![
        head(&a, RatificationKind::Ratified, "act-1", 10),
        head(&b, RatificationKind::Ratified, "act-2", 20),
        head(&a, RatificationKind::Ratified, "act-3", 30),
    ];
    let (f, _) = select_in_force(
        &scope(),
        KindId::CustodyRoot,
        &decls,
        &sets(&[&a, &b]),
        &Acts(decls.clone()),
        &MethodRegistry::builtin(),
        &charter(),
        None,
    );
    assert_eq!(f.set(), &a);
    assert_eq!(
        f.ratification().id,
        "act-3",
        "the head moved forward, to a new declaration"
    );
}

#[test]
fn e5_every_peer_reading_the_same_declarations_elects_the_same_set() {
    let a = tuned(4_000);
    let b = tuned(9_000);
    let mut decls = vec![
        head(&a, RatificationKind::Ratified, "act-a", 10),
        head(&b, RatificationKind::Ratified, "act-b", 10), // same time: the head CID breaks the tie
    ];
    let check = Acts(decls.clone());
    let src = sets(&[&a, &b]);
    let (first, _) = select_in_force(
        &scope(),
        KindId::CustodyRoot,
        &decls,
        &src,
        &check,
        &MethodRegistry::builtin(),
        &charter(),
        None,
    );
    decls.reverse();
    let (second, _) = select_in_force(
        &scope(),
        KindId::CustodyRoot,
        &decls,
        &src,
        &check,
        &MethodRegistry::builtin(),
        &charter(),
        None,
    );
    assert_eq!(first, second);
}

#[test]
fn e6_within_band_only_reweights() {
    let head_set = builtin_set(KindId::CustodyRoot);
    let m = WeightedCapacity;
    let c = charter();
    assert!(within_band(&head_set, &tuned(9_000), &m, &c).is_ok());

    let mut foreign = tuned(9_000);
    foreign.weights = foreign.weights.with(Factor::NearBytes, 10);
    assert_eq!(
        within_band(&head_set, &foreign, &m, &c).unwrap_err().why,
        ParamRefusal::UnknownFactor
    );

    let mut removed = head_set.clone();
    removed.weights = Weights::new()
        .with(Factor::Distance, 1)
        .with(Factor::SpareCapacity, 1);
    assert!(matches!(
        within_band(&head_set, &removed, &m, &c).unwrap_err().why,
        ParamRefusal::Band { .. }
    ));

    let zero = tuned(0);
    assert_eq!(
        within_band(&head_set, &zero, &m, &c).unwrap_err().why,
        ParamRefusal::Band {
            rule: "weight-positive".into()
        }
    );

    let mut cap = tuned(9_000);
    cap.max_moves_per_event = 9;
    assert_eq!(
        within_band(&head_set, &cap, &m, &c).unwrap_err().why,
        ParamRefusal::Band {
            rule: "move-range".into()
        }
    );

    // XorDistance reads no weight, so no tuning is in band under the legacy built-in.
    let legacy = builtin_set(KindId::CustodyBlob);
    let mut tuned_legacy = legacy.clone();
    tuned_legacy.weights = Weights::new().with(Factor::Distance, 1);
    assert_eq!(
        within_band(&legacy, &tuned_legacy, &XorDistance, &c)
            .unwrap_err()
            .why,
        ParamRefusal::UnknownFactor
    );
}

#[test]
fn e7_republishing_at_another_reach_keeps_the_ranking_cid() {
    let set = tuned(9_000);
    let mut h1 = head(&set, RatificationKind::Ratified, "act", 10);
    let mut h2 = h1.clone();
    h1.reach = Reach::Intimate;
    h2.reach = Reach::Community;
    let a = InForce::from_head(
        &scope(),
        KindId::CustodyRoot,
        set.clone(),
        &h1,
        &Anything,
        &MethodRegistry::builtin(),
        &charter(),
    )
    .unwrap();
    let b = InForce::from_head(
        &scope(),
        KindId::CustodyRoot,
        set,
        &h2,
        &Anything,
        &MethodRegistry::builtin(),
        &charter(),
    )
    .unwrap();
    assert_eq!(a.ranking_cid(), b.ranking_cid());
}

#[test]
fn pressures_name_only_the_drafters_own_need() {
    let need = Need {
        kind: KindId::CustodyRoot,
        key: NeedKey::TreeView(elohim_epr::cid::compute_cid(b"t")),
        demand: Demand {
            holders: 2,
            domains: 1,
        },
        requires: Requirements::default(),
        terms: NeedTerms::None,
        reach: Reach::Intimate,
        receiver: AgentRef("uhCAk-matthew".into()),
        expressed_by: Expresser {
            by: ExpresserKind::Person,
            id: "uhCAk-matthew".into(),
        },
        class: NeedClass::Member,
        in_scope_of: scope(),
    };
    let matthew = AgentRef("uhCAk-matthew".into());
    let draft = |by: &AgentRef, reach: Reach| {
        Draft::new(
            tuned(4_000),
            CidString::from(&scope()),
            CidString::from(&scope()),
            reach,
            by.clone(),
            "elohim:matthew".into(),
            Vec::new(),
        )
    };
    let mut own = draft(&matthew, Reach::Intimate);
    assert!(own.press_own_need(PressureKind::UnderHeld, &need).is_ok());
    assert_eq!(own.pressures()[0].key(), Some(&need.key));
    // Another member's need, or the drafter's own need in a draft wider than the need.
    assert!(draft(&AgentRef("uhCAk-other".into()), Reach::Intimate)
        .press_own_need(PressureKind::UnderHeld, &need)
        .is_err());
    assert!(draft(&matthew, Reach::Public)
        .press_own_need(PressureKind::UnderHeld, &need)
        .is_err());
    let mut counted = draft(&matthew, Reach::Public);
    counted.press_count(PressureKind::UnevenCost, 9);
    assert_eq!(counted.pressures()[0].n(), 9);
    assert_eq!(counted.pressures()[0].key(), None);
}

/// Codex review 1, findings 7 and 10: the check cannot be bypassed by deserializing, nor by moving
/// an admitted keyed pressure into a wider draft. A received draft is a `DraftInput`, admitted only
/// by `validate` against the drafter's own needs, and a validated `Draft`'s author and reach are
/// fixed (the `compile_fail` doctests on `Draft` and `Pressure` pin the type side).
#[test]
fn a_received_draft_is_checked_before_it_carries_a_key() {
    let matthew = AgentRef("uhCAk-matthew".into());
    let need = Need {
        kind: KindId::CustodyRoot,
        key: NeedKey::TreeView(elohim_epr::cid::compute_cid(b"t")),
        demand: Demand {
            holders: 2,
            domains: 1,
        },
        requires: Requirements::default(),
        terms: NeedTerms::None,
        reach: Reach::Intimate,
        receiver: matthew.clone(),
        expressed_by: Expresser {
            by: ExpresserKind::Person,
            id: matthew.0.clone(),
        },
        class: NeedClass::Member,
        in_scope_of: scope(),
    };
    let mut intimate = Draft::new(
        tuned(4_000),
        CidString::from(&scope()),
        CidString::from(&scope()),
        Reach::Intimate,
        matthew.clone(),
        "elohim:matthew".into(),
        Vec::new(),
    );
    intimate
        .press_own_need(PressureKind::UnderHeld, &need)
        .unwrap();
    intimate.press_count(PressureKind::Churn, 2);
    // The wire round-trips into the unchecked form and validates back to the same draft.
    let wire = serde_json::to_value(&intimate).unwrap();
    let input: DraftInput = serde_json::from_value(wire).unwrap();
    assert_eq!(
        input.clone().validate(std::slice::from_ref(&need)).unwrap(),
        intimate
    );
    // Widening the received draft to public refuses the intimate key.
    let mut widened = input.clone();
    widened.reach = Reach::Public;
    assert!(widened.validate(std::slice::from_ref(&need)).is_err());
    // Another author, or no such need held, refuses the key.
    let mut other = input.clone();
    other.drafted_by = AgentRef("uhCAk-other".into());
    assert!(other.validate(std::slice::from_ref(&need)).is_err());
    assert!(input.clone().validate(&[]).is_err());
    // A keyed pressure counts one need.
    let mut inflated = input;
    inflated.pressures[0].count = 40;
    assert!(inflated.validate(std::slice::from_ref(&need)).is_err());
}

#[test]
fn e9_a_ratification_names_its_head_exactly() {
    let x = tuned(4_000);
    let y = tuned(9_000);
    let ratified_x = head(&x, RatificationKind::Ratified, "act-x", 10);
    let check = Acts(vec![ratified_x.clone()]);
    assert!(InForce::from_head(
        &scope(),
        KindId::CustodyRoot,
        x.clone(),
        &ratified_x,
        &check,
        &MethodRegistry::builtin(),
        &charter()
    )
    .is_ok());

    // The act for set X attached to a head naming set Y is refused.
    let mut moved = head(&y, RatificationKind::Ratified, "act-x", 10);
    moved.ratification = ratified_x.ratification.clone();
    let r = InForce::from_head(
        &scope(),
        KindId::CustodyRoot,
        y.clone(),
        &moved,
        &check,
        &MethodRegistry::builtin(),
        &charter(),
    )
    .unwrap_err();
    assert_eq!(r.why, ParamRefusal::Refused);

    // A republished head with an inflated ratified_at does not win the election: the clock is a
    // fact the act proves, so the inflated copy is refused and the original stays in force.
    let mut inflated = ratified_x.clone();
    inflated.ratified_at = 9_999;
    let older = tuned(5_000);
    let ratified_older = head(&older, RatificationKind::Ratified, "act-o", 20);
    let check = Acts(vec![ratified_x.clone(), ratified_older.clone()]);
    let (f, refusal) = select_in_force(
        &scope(),
        KindId::CustodyRoot,
        &[ratified_x, ratified_older, inflated],
        &sets(&[&x, &older]),
        &check,
        &MethodRegistry::builtin(),
        &charter(),
        None,
    );
    assert_eq!(f.set(), &older, "the newest PROVEN ratification wins");
    assert_eq!(refusal.unwrap().why, ParamRefusal::Refused);
}

#[test]
fn e6_an_off_band_set_is_never_in_force_whatever_the_check() {
    let mut c = charter();
    let charter_cid = elohim_epr::cid::compute_cid(b"charter");
    c.id = Some(charter_cid);
    let mut negative = tuned(9_000);
    negative.weights = negative.weights.with(Factor::Distance, -5_000);
    let mut zero = tuned(9_000);
    zero.weights = zero.weights.with(Factor::AlwaysOn, 0);
    let mut foreign = tuned(9_000);
    foreign.weights = foreign.weights.with(Factor::NearBytes, 10);
    let mut missing = tuned(9_000);
    missing.weights = Weights::new()
        .with(Factor::Distance, 1)
        .with(Factor::SpareCapacity, 1);
    let mut uncapped = tuned(9_000);
    uncapped.max_moves_per_event = u16::MAX;
    let cases = [
        (
            negative,
            ParamRefusal::Band {
                rule: "weight-positive".into(),
            },
        ),
        (
            zero,
            ParamRefusal::Band {
                rule: "weight-positive".into(),
            },
        ),
        (foreign, ParamRefusal::UnknownFactor),
        (
            missing,
            ParamRefusal::Band {
                rule: "every-factor-weighted".into(),
            },
        ),
        (
            uncapped,
            ParamRefusal::Band {
                rule: "move-range".into(),
            },
        ),
    ];
    for (set, why) in cases {
        // Ratified under a lenient check.
        let h = head(&set, RatificationKind::Ratified, "act", 5);
        let r = InForce::from_head(
            &scope(),
            KindId::CustodyRoot,
            set.clone(),
            &h,
            &Anything,
            &MethodRegistry::builtin(),
            &c,
        )
        .unwrap_err();
        assert_eq!(r.why, why, "ratified {set:?}");
        // Named by the charter itself.
        let mut named = c.clone();
        named
            .genesis_params
            .insert(KindId::CustodyRoot, set.cid().unwrap());
        let h = head(&set, RatificationKind::Charter, &charter_cid.to_string(), 5);
        let r = InForce::from_head(
            &scope(),
            KindId::CustodyRoot,
            set.clone(),
            &h,
            &Anything,
            &MethodRegistry::builtin(),
            &named,
        )
        .unwrap_err();
        assert_eq!(r.why, why, "charter-named {set:?}");
    }
    // A charter range with lo > hi admits no set but the built-in.
    let mut inverted = charter();
    inverted.moves_per_event = (3, 1);
    let set = tuned(9_000);
    let h = head(&set, RatificationKind::Ratified, "act", 5);
    assert!(InForce::from_head(
        &scope(),
        KindId::CustodyRoot,
        set,
        &h,
        &Anything,
        &MethodRegistry::builtin(),
        &inverted
    )
    .is_err());
}

fn chartered() -> (Charter, ParameterSet) {
    let genesis = tuned(4_000);
    let mut c = charter();
    c.id = Some(elohim_epr::cid::compute_cid(b"charter"));
    // Adopted before the fixtures' tuning (ratified at 10).
    c.adopted_at = 5;
    c.genesis_params
        .insert(KindId::CustodyRoot, genesis.cid().unwrap());
    (c, genesis)
}

#[test]
fn e4_an_unproven_clock_on_a_builtin_or_charter_head_never_wins() {
    let (c, genesis) = chartered();
    let tuning = tuned(9_000);
    let ratified = head(&tuning, RatificationKind::Ratified, "act-t", 10);
    let check = Acts(vec![ratified.clone()]);
    let b = builtin_set(KindId::CustodyRoot);
    let forged_builtin = head(
        &b,
        RatificationKind::BuiltIn,
        &b.cid().unwrap().to_string(),
        u64::MAX,
    );
    let forged_charter = head(
        &genesis,
        RatificationKind::Charter,
        &c.id.unwrap().to_string(),
        u64::MAX,
    );
    // Both forged heads verify in core (they name the genuine sets), and neither displaces the
    // ratified tuning, whatever ratified_at they carry.
    for decls in [
        vec![ratified.clone(), forged_builtin.clone()],
        vec![forged_charter.clone(), ratified.clone()],
        vec![
            forged_builtin.clone(),
            ratified.clone(),
            forged_charter.clone(),
        ],
    ] {
        let (f, refusal) = select_in_force(
            &scope(),
            KindId::CustodyRoot,
            &decls,
            &sets(&[&tuning, &b, &genesis]),
            &check,
            &MethodRegistry::builtin(),
            &c,
            None,
        );
        assert_eq!(f.set(), &tuning, "the proven ratification stays in force");
        assert_eq!(f.ratification().by, RatificationKind::Ratified);
        assert!(refusal.is_none());
    }
    // A charter head beats a built-in head, whatever either's clock.
    let (f, _) = select_in_force(
        &scope(),
        KindId::CustodyRoot,
        &[forged_builtin.clone(), {
            let mut h = forged_charter.clone();
            h.ratified_at = 0;
            h
        }],
        &sets(&[&b, &genesis]),
        &check,
        &MethodRegistry::builtin(),
        &c,
        None,
    );
    assert_eq!(f.set(), &genesis);
    // A verified built-in head never displaces the last verified set.
    let held = InForce::from_head(
        &scope(),
        KindId::CustodyRoot,
        tuning.clone(),
        &ratified,
        &check,
        &MethodRegistry::builtin(),
        &c,
    )
    .unwrap();
    let (f, _) = select_in_force(
        &scope(),
        KindId::CustodyRoot,
        &[forged_builtin],
        &sets(&[&b]),
        &check,
        &MethodRegistry::builtin(),
        &c,
        Some(&held),
    );
    assert_eq!(
        f.cid(),
        held.cid(),
        "a built-in head never displaces the held set"
    );
    assert_eq!(f.ratification(), held.ratification());
    assert_eq!(f.fallback(), Some(ParamFallback::LastVerified));
}

#[test]
fn e4_a_charter_amendment_displaces_an_older_tuning_and_a_later_tuning_displaces_it() {
    // One tier (E4): the newest proven clock wins, the charter's at its adoption.
    let (mut c, _) = chartered();
    let tuning = tuned(9_000);
    let ratified = head(&tuning, RatificationKind::Ratified, "act-t", 10);
    // The space amends its charter at 20, naming a new genesis set.
    let amended_genesis = tuned(6_000);
    c.id = Some(elohim_epr::cid::compute_cid(b"charter-amended"));
    c.adopted_at = 20;
    c.genesis_params
        .insert(KindId::CustodyRoot, amended_genesis.cid().unwrap());
    // The amendment's head carries a stale clock of its own: core reads the adoption, not it.
    let amendment = head(
        &amended_genesis,
        RatificationKind::Charter,
        &c.id.unwrap().to_string(),
        0,
    );
    let later = tuned(7_000);
    let later_head = head(&later, RatificationKind::Ratified, "act-later", 30);
    let check = Acts(vec![ratified.clone(), later_head.clone()]);
    let all = sets(&[&tuning, &amended_genesis, &later]);
    let elect = |decls: &[HeadDeclaration]| {
        select_in_force(
            &scope(),
            KindId::CustodyRoot,
            decls,
            &all,
            &check,
            &MethodRegistry::builtin(),
            &c,
            None,
        )
        .0
    };
    for decls in [
        vec![ratified.clone(), amendment.clone()],
        vec![amendment.clone(), ratified.clone()],
    ] {
        let f = elect(&decls);
        assert_eq!(
            f.set(),
            &amended_genesis,
            "the amendment displaces the older tuning"
        );
        assert_eq!(f.ratification().by, RatificationKind::Charter);
    }
    for decls in [
        vec![ratified.clone(), amendment.clone(), later_head.clone()],
        vec![later_head.clone(), amendment.clone(), ratified.clone()],
    ] {
        assert_eq!(
            elect(&decls).set(),
            &later,
            "a tuning ratified after the amendment displaces it in turn"
        );
    }
    // An old charter's head no longer verifies once the charter is amended, whatever its clock.
    let (old_c, old_genesis) = chartered();
    let stale = head(
        &old_genesis,
        RatificationKind::Charter,
        &old_c.id.unwrap().to_string(),
        u64::MAX,
    );
    let f = select_in_force(
        &scope(),
        KindId::CustodyRoot,
        &[ratified.clone(), stale],
        &sets(&[&tuning, &old_genesis]),
        &check,
        &MethodRegistry::builtin(),
        &c,
        None,
    )
    .0;
    assert_eq!(f.set(), &tuning);
}

#[test]
fn e1_a_builtin_or_charter_head_carries_no_unproven_provenance() {
    let (c, genesis) = chartered();
    let b = builtin_set(KindId::CustodyRoot);
    let canonical = head(
        &b,
        RatificationKind::BuiltIn,
        &b.cid().unwrap().to_string(),
        7,
    );
    let from = |set: &ParameterSet, h: &HeadDeclaration| {
        InForce::from_head(
            &scope(),
            KindId::CustodyRoot,
            set.clone(),
            h,
            &Anything,
            &MethodRegistry::builtin(),
            &c,
        )
        .map_err(|u| u.why)
    };
    // The verified built-in is the canonical one, never the head's copy.
    assert_eq!(
        from(&b, &canonical).unwrap(),
        InForce::builtin(KindId::CustodyRoot)
    );
    let mut wrong_id = canonical.clone();
    wrong_id.ratification.id = "anything".into();
    let mut with_rule = canonical.clone();
    with_rule.ratification.rule = Some(CidString::from(&scope()));
    let mut with_touches = canonical.clone();
    with_touches.ratification.touches = vec![AgentRefKey::of(&AgentRef("uhCAk-x".into()))];
    for h in [wrong_id, with_rule, with_touches] {
        assert_eq!(from(&b, &h).unwrap_err(), ParamRefusal::Refused, "{h:?}");
    }
    let charter_head = head(
        &genesis,
        RatificationKind::Charter,
        &c.id.unwrap().to_string(),
        7,
    );
    assert!(from(&genesis, &charter_head).is_ok());
    let mut touched = charter_head.clone();
    touched.ratification.touches = vec![AgentRefKey::of(&AgentRef("uhCAk-x".into()))];
    assert_eq!(from(&genesis, &touched).unwrap_err(), ParamRefusal::Refused);
}
