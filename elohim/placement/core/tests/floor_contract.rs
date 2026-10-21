//! Contract for D (interfaces spec §D, "Contract for D"), with `Ports` doubles.
//!
//! Each test names the numbered contract it pins. Fixtures are a small household graph built here;
//! nothing reads a clock or a store.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use cid::Cid;
use elohim_epr::Reach;
use elohim_epr_rea::AgentRef;
use placement_core::kind::testing::BytesKind;
use placement_core::*;
use seam_contracts::harness::check_quiescence;
use seam_contracts::{Answer, ComeBack};

// ---- fixtures ---------------------------------------------------------------------------------

fn agent(s: &str) -> AgentRef {
    AgentRef(format!("uhCAk-{s}"))
}

fn space(tag: &str) -> Cid {
    elohim_epr::cid::compute_cid(format!("space-{tag}").as_bytes())
}

/// Declares the demand the fixtures' needs ask for: a need is held to the charter's widest declared
/// demand (`Charter::bound`), never to its own ask.
fn charter() -> Charter {
    Charter {
        id: None,
        adopted_at: 0,
        demand: [
            (
                LossDomain::Household,
                Demand {
                    holders: 2,
                    domains: 1,
                },
            ),
            (
                LossDomain::Region,
                Demand {
                    holders: 3,
                    domains: 2,
                },
            ),
        ]
        .into_iter()
        .collect(),
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

fn need(
    kind: KindId,
    key: &str,
    demand: (u16, u16),
    reach: Reach,
    receiver: &str,
    scope: &Cid,
) -> Need {
    let key = match kind {
        KindId::CustodyBlob => NeedKey::Blob(format!("sha256-{key}")),
        _ => NeedKey::TreeView(elohim_epr::cid::compute_cid(key.as_bytes())),
    };
    Need {
        kind,
        key,
        demand: Demand {
            holders: demand.0,
            domains: demand.1,
        },
        requires: Requirements {
            bytes: Some(10),
            ..Requirements::default()
        },
        terms: NeedTerms::None,
        reach,
        receiver: agent(receiver),
        expressed_by: Expresser {
            by: ExpresserKind::Person,
            id: agent(receiver).0,
        },
        class: NeedClass::Member,
        in_scope_of: *scope,
    }
}

struct Cast {
    /// (name, household, spare bytes, deferred)
    members: Vec<(&'static str, &'static str, u64, bool)>,
    receivers: Vec<&'static str>,
    admitted_at: Reach,
}

fn candidates(cast: &Cast) -> Vec<Candidate> {
    cast.members
        .iter()
        .map(|(name, hh, spare, deferred)| {
            let carrier = agent(name);
            let terms = OfferTerms {
                spare_bytes: Some(*spare),
                deferred: *deferred,
                household_only: *deferred,
                ..OfferTerms::default()
            };
            let offers = [
                (KindId::CustodyBlob, terms.clone()),
                (KindId::CustodyRoot, terms),
            ]
            .into_iter()
            .collect();
            let admitted = cast
                .receivers
                .iter()
                .map(|r| {
                    (
                        AgentRefKey::of(&agent(r)),
                        Answer::Present(cast.admitted_at),
                    )
                })
                .collect();
            Candidate {
                domain: FailureDomain::of(Some(hh), &carrier),
                posture: Posture {
                    carrier,
                    offers,
                    reach: Reach::Public,
                    written_at: 0,
                },
                live: Answer::Present(true),
                admitted,
                consent: BTreeMap::new(),
            }
        })
        .collect()
}

fn snapshot(me: &str, cast: &Cast, needs: Vec<Need>) -> Snapshot {
    let mut s = Snapshot::empty(agent(me));
    s.now = 1_000;
    s.period = Period {
        start: 0,
        len_secs: 86_400,
    };
    for n in &needs {
        s.charters
            .insert(CidString::from(&n.in_scope_of), charter());
        s.needs_read
            .insert(CidString::from(&n.in_scope_of), Answer::Present(()));
    }
    s.needs = needs;
    s.candidates = candidates(cast);
    s
}

fn kinds() -> KindRegistry {
    KindRegistry::new()
        .with(Box::new(BytesKind::custody(KindId::CustodyBlob)))
        .with(Box::new(BytesKind::custody(KindId::CustodyRoot)))
}

fn plan_for(s: &Snapshot) -> Plan {
    decide(
        s,
        &kinds(),
        &MethodRegistry::builtin(),
        &mut Scratch::default(),
    )
}

fn as_me(s: &Snapshot, me: &AgentRef) -> Snapshot {
    let mut s = s.clone();
    s.me = me.clone();
    s
}

/// Apply every member's plan to the shared snapshot: adoptions become honored carryings (as a byte
/// mover that always succeeds would make them), releases remove theirs.
fn apply_all(s: &mut Snapshot, plans: &[Plan]) -> usize {
    let mut effects = 0;
    for p in plans {
        for d in &p.adopt {
            s.carrying.push(Carrying {
                commitment: d.id.clone(),
                need: d.need.to_cid().unwrap(),
                key: d.key.clone(),
                kind: d.kind,
                carrier: d.provider.clone(),
                receiver: d.receiver.clone(),
                in_scope_of: d.in_scope_of.to_cid().unwrap(),
                lease: d.lease,
                under_params: d.ranking.to_cid().unwrap(),
                honored: Answer::Present(true),
            });
            effects += 1;
        }
        for r in &p.release {
            s.carrying.retain(|c| c.commitment != r.commitment);
            effects += 1;
        }
    }
    effects
}

fn everyone(s: &Snapshot) -> Vec<Plan> {
    s.candidates
        .iter()
        .map(|c| plan_for(&as_me(s, c.agent())))
        .collect()
}

fn household() -> Cast {
    Cast {
        members: vec![
            ("hub", "A", 1_000, false),
            ("laptop", "A", 500, false),
            ("phone", "A", 50, true),
            ("gp-hub", "B", 1_000, false),
            ("gp-tablet", "B", 200, false),
            ("friend", "C", 800, false),
        ],
        receivers: vec!["hub", "laptop", "phone", "reader"],
        admitted_at: Reach::Intimate,
    }
}

fn public_needs(scope: &Cid) -> Vec<Need> {
    vec![
        need(
            KindId::CustodyRoot,
            "doc-1",
            (2, 2),
            Reach::Public,
            "reader",
            scope,
        ),
        need(
            KindId::CustodyRoot,
            "doc-2",
            (3, 2),
            Reach::Public,
            "reader",
            scope,
        ),
        need(
            KindId::CustodyBlob,
            "aa",
            (2, 1),
            Reach::Public,
            "reader",
            scope,
        ),
        need(
            KindId::CustodyBlob,
            "bb",
            (1, 1),
            Reach::Public,
            "reader",
            scope,
        ),
    ]
}

fn honored_of(s: &Snapshot, id: &Cid) -> Vec<AgentRef> {
    s.carrying
        .iter()
        .filter(|c| c.need == *id && c.honored == Answer::Present(true))
        .map(|c| c.carrier.clone())
        .collect()
}

// ---- 1. Agreement ---------------------------------------------------------------------------

#[test]
fn c1_agreement_input_order_never_reaches_the_plan() {
    let scope = space("a");
    let base = snapshot("hub", &household(), public_needs(&scope));
    let mut shuffled = base.clone();
    shuffled.candidates.reverse();
    shuffled.needs.reverse();
    shuffled.candidates.rotate_left(2);
    for c in &base.candidates {
        assert_eq!(
            plan_for(&as_me(&base, c.agent())),
            plan_for(&as_me(&shuffled, c.agent())),
            "{:?}",
            c.agent()
        );
    }
}

#[test]
fn c1_agreement_union_of_slices_is_the_global_assignment() {
    let scope = space("a");
    let s = snapshot("hub", &household(), public_needs(&scope));
    let global = chosen(
        &s,
        &kinds(),
        &MethodRegistry::builtin(),
        &mut Scratch::default(),
    );
    let mut union: BTreeMap<Cid, BTreeSet<String>> = BTreeMap::new();
    for p in everyone(&s) {
        for d in p.adopt {
            union
                .entry(d.need.to_cid().unwrap())
                .or_default()
                .insert(d.provider.0);
        }
    }
    let expected: BTreeMap<Cid, BTreeSet<String>> = global
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().map(|a| a.0).collect()))
        .collect();
    assert_eq!(union, expected);
}

// ---- 2. Self only ---------------------------------------------------------------------------

#[test]
fn c2_every_draft_and_release_is_my_own() {
    let scope = space("a");
    let mut s = snapshot("hub", &household(), public_needs(&scope));
    let plans = everyone(&s);
    apply_all(&mut s, &plans);
    // Over-hold one need so drains are on the table.
    let id = s.needs[3].id().unwrap();
    for name in ["gp-hub", "friend"] {
        s.carrying.push(Carrying {
            commitment: format!("extra-{name}"),
            need: id,
            key: s.needs[3].key.clone(),
            kind: KindId::CustodyBlob,
            carrier: agent(name),
            receiver: agent("reader"),
            in_scope_of: scope,
            lease: Lease {
                from: 0,
                until: None,
            },
            under_params: *InForce::builtin(KindId::CustodyBlob).ranking_cid(),
            honored: Answer::Present(true),
        });
    }
    let mut releases = 0;
    for c in s.candidates.clone() {
        let p = plan_for(&as_me(&s, c.agent()));
        assert!(p.adopt.iter().all(|d| &d.provider == c.agent()));
        for r in &p.release {
            releases += 1;
            assert_eq!(&r.provider, c.agent());
            let carried = s
                .carrying
                .iter()
                .find(|x| x.commitment == r.commitment)
                .unwrap();
            assert_eq!(
                &carried.carrier,
                c.agent(),
                "a release names only my own commitment (D18)"
            );
        }
    }
    assert_eq!(releases, 1, "one drain per need per pass");
}

// ---- 3. Idempotent --------------------------------------------------------------------------

#[test]
fn c3_replay_authors_nothing() {
    let scope = space("a");
    let s = snapshot("hub", &household(), public_needs(&scope));
    check_quiescence(s, everyone, |s, plans| apply_all(s, plans)).unwrap();
}

#[test]
fn c3_a_pending_own_commitment_is_not_redrafted() {
    let scope = space("a");
    let mut s = snapshot("hub", &household(), public_needs(&scope));
    let first = everyone(&s);
    apply_all(&mut s, &first);
    for c in &mut s.carrying {
        c.honored = Answer::Present(false); // authored, not yet honored
    }
    for p in everyone(&s) {
        assert!(p.adopt.is_empty(), "{:?}", p.adopt);
    }
}

// ---- 4 and 6. Make-before-break and revert ---------------------------------------------------

struct Lenient;
impl RatificationCheck for Lenient {
    fn verify(
        &self,
        _h: &HeadDeclaration,
        _c: &Charter,
    ) -> Result<(), seam_contracts::Unanswered<ParamRefusal>> {
        Ok(())
    }
}

fn ratified(scope: &Cid, set: ParameterSet, at: u64) -> InForce {
    let head = HeadDeclaration {
        scope: *scope,
        sets: [(set.kind, CidString::from(&set.cid().unwrap()))]
            .into_iter()
            .collect(),
        reach: Reach::Intimate,
        predecessor: None,
        ratification: RatificationRef {
            by: RatificationKind::Ratified,
            id: format!("act-{at}"),
            rule: None,
            touches: Vec::new(),
        },
        ratified_at: at,
    };
    InForce::from_head(
        scope,
        set.kind,
        set,
        &head,
        &Lenient,
        &MethodRegistry::builtin(),
        &charter(),
    )
    .unwrap()
}

fn spare_heavy() -> ParameterSet {
    let mut set = builtin_set(KindId::CustodyRoot);
    set.weights = Weights::new()
        .with(Factor::Distance, 1)
        .with(Factor::SpareCapacity, 9_000)
        .with(Factor::AlwaysOn, 1);
    set.max_moves_per_event = 1;
    set
}

fn settle_household(s: &mut Snapshot) {
    for _ in 0..8 {
        let plans = everyone(s);
        if apply_all(s, &plans) == 0 {
            return;
        }
    }
    panic!("the household did not settle in 8 rounds");
}

#[test]
fn c4_a_new_set_moves_make_before_break_within_the_cap() {
    let scope = space("m");
    let tree = need(
        KindId::CustodyRoot,
        "tree",
        (2, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    let id = tree.id().unwrap();
    let mut s = snapshot("hub", &household(), vec![tree]);
    settle_household(&mut s);
    let before = honored_of(&s, &id);
    assert_eq!(before.len(), 2);

    // A newly ratified set resets incumbency for this (scope, kind).
    s.params.insert(
        (CidString::from(&scope), KindId::CustodyRoot),
        ratified(&scope, spare_heavy(), 10),
    );
    let under_new = chosen(
        &s,
        &kinds(),
        &MethodRegistry::builtin(),
        &mut Scratch::default(),
    )[&id]
        .clone();
    let mut rounds = 0;
    loop {
        let plans = everyone(&s);
        let moves: usize = plans
            .iter()
            .map(|p| p.adopt.iter().filter(|d| d.replaces.is_some()).count())
            .sum();
        assert!(moves <= 1, "at most max_moves_per_event moves per event");
        for p in &plans {
            for r in &p.release {
                if let ReleaseReason::Superseded { by } = &r.reason {
                    let successor = s.carrying.iter().find(|c| &c.commitment == by).unwrap();
                    assert_eq!(
                        successor.honored,
                        Answer::Present(true),
                        "make before break"
                    );
                }
            }
        }
        if apply_all(&mut s, &plans) == 0 {
            break;
        }
        assert!(
            honored_of(&s, &id).len() >= 2,
            "honored never falls below min(holders, before)"
        );
        rounds += 1;
        assert!(rounds < 12, "the move did not settle");
    }
    let after: BTreeSet<String> = honored_of(&s, &id).into_iter().map(|a| a.0).collect();
    assert_eq!(
        after,
        under_new.into_iter().map(|a| a.0).collect(),
        "the new set's placement, once the moves settle"
    );
}

#[test]
fn c6_revert_restores_the_earlier_sets_placement() {
    let scope = space("r");
    let tree = need(
        KindId::CustodyRoot,
        "tree",
        (2, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    let id = tree.id().unwrap();
    let mut s = snapshot("hub", &household(), vec![tree]);
    let key = (CidString::from(&scope), KindId::CustodyRoot);
    let builtin_placement: BTreeSet<String> = chosen(
        &s,
        &kinds(),
        &MethodRegistry::builtin(),
        &mut Scratch::default(),
    )[&id]
        .iter()
        .map(|a| a.0.clone())
        .collect();
    settle_household(&mut s);
    s.params
        .insert(key.clone(), ratified(&scope, spare_heavy(), 10));
    settle_household(&mut s);
    // The revert: a new, later declaration naming the built-in set's CID, ratified the same way.
    s.params
        .insert(key, ratified(&scope, builtin_set(KindId::CustodyRoot), 20));
    settle_household(&mut s);
    let after: BTreeSet<String> = honored_of(&s, &id).into_iter().map(|a| a.0).collect();
    assert_eq!(after, builtin_placement);
}

// ---- 5. Hysteresis ----------------------------------------------------------------------------

#[test]
fn c5_a_better_newcomer_moves_no_incumbent() {
    let scope = space("h");
    let tree = need(
        KindId::CustodyRoot,
        "tree",
        (2, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    let mut s = snapshot("hub", &household(), vec![tree]);
    settle_household(&mut s);
    let held = s.carrying.clone();
    let mut richer = household();
    richer.members.push(("newcomer", "D", 1_000_000, false));
    s.candidates = candidates(&richer);
    for p in everyone(&s) {
        assert!(p.adopt.is_empty() && p.release.is_empty(), "{p:?}");
    }
    assert_eq!(s.carrying, held);
}

#[test]
fn c5_carve_out_a_domain_gap_fills_then_an_over_held_domain_drains() {
    let scope = space("d");
    let tree = need(
        KindId::CustodyRoot,
        "tree",
        (2, 2),
        Reach::Public,
        "reader",
        &scope,
    );
    let id = tree.id().unwrap();
    let mut s = snapshot("hub", &household(), vec![tree.clone()]);
    // Two incumbents in one household.
    for name in ["hub", "laptop"] {
        s.carrying.push(Carrying {
            commitment: format!("c-{name}"),
            need: id,
            key: tree.key.clone(),
            kind: KindId::CustodyRoot,
            carrier: agent(name),
            receiver: agent("reader"),
            in_scope_of: scope,
            lease: Lease {
                from: 0,
                until: None,
            },
            under_params: *InForce::builtin(KindId::CustodyRoot).ranking_cid(),
            honored: Answer::Present(true),
        });
    }
    let domains = |s: &Snapshot| {
        honored_of(s, &id)
            .iter()
            .map(|a| {
                s.candidates
                    .iter()
                    .find(|c| c.agent() == a)
                    .unwrap()
                    .domain
                    .clone()
            })
            .collect::<BTreeSet<_>>()
            .len()
    };
    let mut held_min = usize::MAX;
    for _ in 0..6 {
        let plans = everyone(&s);
        if apply_all(&mut s, &plans) == 0 {
            break;
        }
        held_min = held_min.min(honored_of(&s, &id).len());
    }
    assert!(held_min >= 2);
    assert_eq!(honored_of(&s, &id).len(), 2, "drained back to the demand");
    assert_eq!(domains(&s), 2, "two domains throughout the carve-out");
}

// ---- 7, 12, 16. Every need explained once, explainable, R7 parts --------------------------------

#[test]
fn c7_c12_c16_every_need_is_explained_and_the_digest_matches() {
    let scope = space("a");
    let s = snapshot("hub", &household(), public_needs(&scope));
    let plans: Vec<(AgentRef, Plan)> = s
        .candidates
        .iter()
        .map(|c| (c.agent().clone(), plan_for(&as_me(&s, c.agent()))))
        .collect();
    for n in &s.needs {
        let id = n.id().unwrap();
        let e = explain(
            &s,
            &kinds(),
            &MethodRegistry::builtin(),
            &id,
            &agent("phone"),
            None,
        )
        .unwrap();
        assert!(matches!(
            e.outcome,
            Outcome::Carried | Outcome::Held | Outcome::NotCarried { .. }
        ));
        assert_eq!(e.inputs_digest.len(), 64);
        assert!(
            e.own_refusals.iter().all(|r| r.agent == agent("phone")),
            "no other member's refusal listed"
        );
        if let Outcome::NotCarried { why } = &e.outcome {
            assert!(matches!(why.come_back, ComeBack::OnEvent { .. }));
        }
    }
    for (me, p) in &plans {
        for d in &p.adopt {
            let id = d.need.to_cid().unwrap();
            let e = explain(
                &s,
                &kinds(),
                &MethodRegistry::builtin(),
                &id,
                me,
                Some(d.inputs_digest),
            )
            .unwrap();
            assert_eq!(e.digest_matches_commitment, Some(true));
            assert_eq!(e.method, d.method);
        }
        for u in &p.unserved {
            assert!(
                matches!(u.why.come_back, ComeBack::OnEvent { .. }),
                "R7: when to come back"
            );
        }
    }
}

// ---- 8. No refusal is reachable through a parameter set ----------------------------------------

/// A small deterministic generator, so the property runs over many sets with no new dependency.
fn generated_weights(n: usize) -> Vec<(i32, i32, i32)> {
    let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x % 10_000) as i32 + 1
    };
    (0..n).map(|_| (next(), next(), next())).collect()
}

/// Contract 8, stated per need given the capacity the pass has left (the spec seat absorbs the
/// restatement): no reason but `OverCapacity` changes with the parameter set, and `OverCapacity`
/// that is not there at the pass's start comes only from serve-order charging, i.e. the carrier was
/// chosen for a need served earlier in the same pass. Two needs share a capacity-limited carrier.
#[test]
fn c8_parameter_sets_never_change_eligibility() {
    let scope = space("p");
    let mut cast = household();
    cast.members.push(("tiny", "E", 1, false)); // over capacity for every need from the start
    cast.members.push(("narrow", "F", 15, false)); // room for exactly one 10-byte need
    let needs = vec![
        need(
            KindId::CustodyRoot,
            "tree-1",
            (3, 2),
            Reach::Public,
            "reader",
            &scope,
        ),
        need(
            KindId::CustodyRoot,
            "tree-2",
            (3, 2),
            Reach::Public,
            "reader",
            &scope,
        ),
    ];
    let base = snapshot("hub", &cast, needs);
    let mut ids: Vec<Cid> = base.needs.iter().map(|n| n.id().unwrap()).collect();
    ids.sort_by_key(|c| c.to_bytes()); // the pass's serve order within one class
    let members: Vec<AgentRef> = base.candidates.iter().map(|c| c.agent().clone()).collect();
    let refusal_of = |s: &Snapshot, id: &Cid, who: &AgentRef| {
        explain(s, &kinds(), &MethodRegistry::builtin(), id, who, None)
            .unwrap()
            .own_refusals
            .into_iter()
            .next()
            .map(|r| r.why.why)
    };
    let without_charging = |r: Option<IneligibleReason>| match r {
        Some(IneligibleReason::OverCapacity) => None,
        other => other,
    };
    let reference: Vec<Vec<Option<IneligibleReason>>> = ids
        .iter()
        .map(|id| {
            members
                .iter()
                .map(|m| without_charging(refusal_of(&base, id, m)))
                .collect()
        })
        .collect();
    let mut charged_somewhere = false;
    let mut sets = vec![(5_000, 3_000, 2_000)];
    sets.extend(generated_weights(48));
    for (d, sp, on) in sets {
        let mut set = builtin_set(KindId::CustodyRoot);
        set.weights = Weights::new()
            .with(Factor::Distance, d)
            .with(Factor::SpareCapacity, sp)
            .with(Factor::AlwaysOn, on);
        let mut s = base.clone();
        s.params.insert(
            (CidString::from(&scope), KindId::CustodyRoot),
            ratified(&scope, set, 5),
        );
        let chosen = chosen(
            &s,
            &kinds(),
            &MethodRegistry::builtin(),
            &mut Scratch::default(),
        );
        for (n, id) in ids.iter().enumerate() {
            for (m, who) in members.iter().enumerate() {
                let r = refusal_of(&s, id, who);
                assert_eq!(
                    without_charging(r.clone()),
                    reference[n][m],
                    "weights ({d},{sp},{on}) changed {who:?}'s eligibility for need {n}"
                );
                if r == Some(IneligibleReason::OverCapacity) && who.0 != agent("tiny").0 {
                    let earlier = ids[..n].iter().any(|e| chosen[e].contains(who));
                    assert!(
                        earlier,
                        "{who:?} is over capacity for need {n} only through serve-order charging"
                    );
                    charged_somewhere = true;
                }
            }
        }
    }
    assert!(
        charged_somewhere,
        "the property exercised the serve-order charging channel"
    );
}

// ---- 9. R2, and the worked example ------------------------------------------------------------

#[test]
fn c9_below_public_is_household_local_in_the_receivers_household() {
    let scope = space("w");
    let tree = need(
        KindId::CustodyRoot,
        "family-tree",
        (2, 2),
        Reach::Intimate,
        "phone",
        &scope,
    );
    let id = tree.id().unwrap();
    let s = snapshot("hub", &household(), vec![tree]);
    let plans = everyone(&s);
    let adopted: Vec<&CommitmentDraft> = plans.iter().flat_map(|p| p.adopt.iter()).collect();
    assert_eq!(adopted.len(), 2, "two holders in the household");
    for d in &adopted {
        assert_eq!(
            d.plane,
            RecordPlane::HouseholdLocal {
                household: "A".into()
            }
        );
    }
    // The second domain is short, said with R7's parts, and hh:B is refused by name.
    let e = explain(
        &s,
        &kinds(),
        &MethodRegistry::builtin(),
        &id,
        &agent("gp-hub"),
        None,
    )
    .unwrap();
    match &e.outcome {
        Outcome::NotCarried { why } => {
            assert_eq!(why.why, NotCarriedReason::DomainShort { have: 1, need: 2 });
            assert_eq!(
                why.come_back,
                ComeBack::OnEvent {
                    event: "posture-changed".into()
                }
            );
        }
        other => panic!("{other:?}"),
    }
    assert!(e
        .own_refusals
        .iter()
        .any(|r| r.why.why == IneligibleReason::CrossHouseholdBelowPublic));
    // Both explanations of the household's holders are identical.
    let a = explain(
        &s,
        &kinds(),
        &MethodRegistry::builtin(),
        &id,
        &agent("x"),
        None,
    )
    .unwrap();
    let b = explain(
        &s,
        &kinds(),
        &MethodRegistry::builtin(),
        &id,
        &agent("y"),
        None,
    )
    .unwrap();
    assert_eq!(a, b);
}

#[test]
fn worked_example_public_tree_fills_the_domain_gap_then_drains_a_laptop_or_hub() {
    let scope = space("w2");
    let tree = need(
        KindId::CustodyRoot,
        "public-tree",
        (2, 2),
        Reach::Public,
        "phone",
        &scope,
    );
    let id = tree.id().unwrap();
    let mut s = snapshot("hub", &household(), vec![tree]);
    settle_household(&mut s);
    let holders = honored_of(&s, &id);
    assert_eq!(holders.len(), 2);
    let domains: BTreeSet<_> = holders
        .iter()
        .map(|a| {
            s.candidates
                .iter()
                .find(|c| c.agent() == a)
                .unwrap()
                .domain
                .clone()
        })
        .collect();
    assert_eq!(domains.len(), 2, "a second household fills the domain gap");
    for c in &s.carrying {
        assert_eq!(c.honored, Answer::Present(true));
    }
}

// ---- 10. No ceiling needed --------------------------------------------------------------------

#[test]
fn c10_converges_under_the_builtin_sets() {
    let scope = space("n");
    let mut s = snapshot("hub", &household(), public_needs(&scope));
    assert!(s.params.is_empty());
    settle_household(&mut s);
    for n in s.needs.clone() {
        let id = n.id().unwrap();
        assert!(honored_of(&s, &id).len() >= n.demand.holders as usize);
    }
}

// ---- 11. Idle is free, through the Floor ------------------------------------------------------

struct Double {
    snapshot: RefCell<Snapshot>,
    authored: RefCell<Vec<CommitmentDraft>>,
    released: RefCell<Vec<Release>>,
    settled: RefCell<Vec<FulfilmentDraft>>,
    reads: RefCell<u32>,
}

impl Ports for &Double {
    fn snapshot(
        &self,
        _scopes: &[Cid],
        now: EpochSecs,
        into: &mut Snapshot,
    ) -> Result<(), PortError> {
        *self.reads.borrow_mut() += 1;
        *into = self.snapshot.borrow().clone();
        into.now = now;
        Ok(())
    }
    fn author(&self, d: &CommitmentDraft) -> Result<(), PortError> {
        self.authored.borrow_mut().push(d.clone());
        let mut s = self.snapshot.borrow_mut();
        s.carrying.push(Carrying {
            commitment: d.id.clone(),
            need: d.need.to_cid().unwrap(),
            key: d.key.clone(),
            kind: d.kind,
            carrier: d.provider.clone(),
            receiver: d.receiver.clone(),
            in_scope_of: d.in_scope_of.to_cid().unwrap(),
            lease: d.lease,
            under_params: d.ranking.to_cid().unwrap(),
            honored: Answer::Present(true),
        });
        Ok(())
    }
    fn release(&self, r: &Release) -> Result<(), PortError> {
        self.released.borrow_mut().push(r.clone());
        Ok(())
    }
    fn settle(&self, f: &FulfilmentDraft) -> Result<(), PortError> {
        self.settled.borrow_mut().push(f.clone());
        Ok(())
    }
    fn evidence(&self, c: &Carrying, p: Period) -> Option<(u64, RealEvidence)> {
        // Observed at noon of the first day; every period these tests close starts at 0.
        assert_eq!(
            p.start, 0,
            "the fixture's reading lies in the period from 0"
        );
        let reading = seam_contracts::Reading::new(
            "holding-is-honored@1".into(),
            "@concern:holding".into(),
            c.key.as_id_string(),
            seam_contracts::ProbeAnswer::present(10u64),
            seam_contracts::EvidenceStage::Real,
            seam_contracts::Disclosure::Holder,
            "1970-01-01T12:00:00Z".into(),
        );
        Some((10, RealEvidence::from_readings(c, p, &[reading]).unwrap()))
    }
}

#[test]
fn c11_idle_is_free_and_period_closed_only_settles() {
    let scope = space("i");
    let tree = need(
        KindId::CustodyRoot,
        "tree",
        (1, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    let s = snapshot("hub", &household(), vec![tree]);
    // Only the chosen carrier's floor adopts; run as each member until settled.
    let mut settled = s.clone();
    settle_household(&mut settled);
    let holder = settled.carrying[0].carrier.clone();
    let mut start = s.clone();
    start.me = holder;
    let double = Double {
        snapshot: RefCell::new(start),
        authored: RefCell::default(),
        released: RefCell::default(),
        settled: RefCell::default(),
        reads: RefCell::new(0),
    };
    let mut floor = Floor::new(&double, kinds(), MethodRegistry::builtin(), vec![scope]);
    let first = floor.on(&Trigger::MembershipChanged, 1_000).unwrap();
    assert_eq!(first.adopted, 1);
    let idle = floor.on(&Trigger::MembershipChanged, 1_001).unwrap();
    assert_eq!(
        idle,
        PassOutcome::default(),
        "unchanged inputs write nothing"
    );
    let skipped = floor
        .on(&Trigger::CarrierLost(agent("stranger")), 1_002)
        .unwrap();
    assert_eq!(
        skipped,
        PassOutcome::default(),
        "a trigger with no affected set skips the pass"
    );
    let period = floor
        .on(
            &Trigger::PeriodClosed(Period {
                start: 0,
                len_secs: 86_400,
            }),
            90_000,
        )
        .unwrap();
    assert_eq!(period.settled, 1);
    assert_eq!(
        period.adopted + period.released,
        0,
        "PeriodClosed settles and never re-places"
    );
    assert_eq!(double.authored.borrow().len(), 1);
    assert_eq!(double.settled.borrow()[0].unit, "byte-period");
    // A re-delivered PeriodClosed drafts the same fulfilment id, which `Ports::settle` is
    // idempotent by: one settlement per commitment per period.
    let closed = Period {
        start: 0,
        len_secs: 86_400,
    };
    floor.on(&Trigger::PeriodClosed(closed), 90_001).unwrap();
    let settled = double.settled.borrow();
    assert_eq!(settled.len(), 2);
    assert_eq!(settled[0].id, settled[1].id);
    assert_eq!(
        settled[0].id,
        fulfilment_id(&double.authored.borrow()[0].id, closed)
    );
}

// ---- 13 and 14. Legacy behaviour and dead carriers ----------------------------------------------

fn legacy_row(name: &str, n: &Need, honored: Answer<bool>) -> Carrying {
    Carrying {
        commitment: commitment_id(KindId::CustodyBlob, &agent(name), &n.receiver, &n.key),
        need: n.id().unwrap(),
        key: n.key.clone(),
        kind: KindId::CustodyBlob,
        carrier: agent(name),
        receiver: n.receiver.clone(),
        in_scope_of: n.in_scope_of,
        lease: Lease {
            from: 0,
            until: None,
        },
        under_params: *InForce::builtin(KindId::CustodyBlob).ranking_cid(),
        honored,
    }
}

#[test]
fn c13_legacy_holders_are_incumbents_and_the_first_pass_moves_nothing() {
    let scope = space("legacy");
    let blob = need(
        KindId::CustodyBlob,
        "cafe",
        (2, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    let mut s = snapshot("hub", &household(), vec![blob.clone()]);
    // Two legacy holders that XOR would not pick today.
    let global = chosen(
        &s,
        &kinds(),
        &MethodRegistry::builtin(),
        &mut Scratch::default(),
    );
    let picked: Vec<String> = global[&blob.id().unwrap()]
        .iter()
        .map(|a| a.0.clone())
        .collect();
    let others: Vec<&str> = ["hub", "laptop", "gp-hub", "gp-tablet", "friend"]
        .into_iter()
        .filter(|n| !picked.contains(&agent(n).0))
        .take(2)
        .collect();
    for n in &others {
        s.carrying.push(legacy_row(n, &blob, Answer::Present(true)));
    }
    for p in everyone(&s) {
        assert!(p.adopt.is_empty() && p.release.is_empty(), "{p:?}");
    }
}

#[test]
fn c13_committed_not_honored_and_transport_id_rows_are_not_redrafted() {
    let scope = space("legacy2");
    let blob = need(
        KindId::CustodyBlob,
        "beef",
        (1, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    let s = snapshot("hub", &household(), vec![blob.clone()]);
    let winner = chosen(
        &s,
        &kinds(),
        &MethodRegistry::builtin(),
        &mut Scratch::default(),
    )[&blob.id().unwrap()][0]
        .clone();
    let mut pending = as_me(&s, &winner);
    pending.carrying.push(Carrying {
        honored: Answer::Present(false),
        ..legacy_row(&winner.0["uhCAk-".len()..], &blob, Answer::Present(false))
    });
    assert!(
        plan_for(&pending).adopt.is_empty(),
        "a self already committed is not re-drafted"
    );

    let mut aliased = as_me(&s, &winner);
    aliased.me_aliases = vec!["12D3KooW-transport".into()];
    let mut row = legacy_row("x", &blob, Answer::Present(true));
    row.carrier = AgentRef("12D3KooW-transport".into());
    aliased.carrying.push(row);
    let p = plan_for(&aliased);
    assert!(
        p.adopt.is_empty() && p.release.is_empty(),
        "a transport-id provider row counts as me: {p:?}"
    );
}

#[test]
fn c14_a_dead_carrier_does_not_hold_a_need() {
    let scope = space("dead");
    let blob = need(
        KindId::CustodyBlob,
        "dead",
        (1, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    let base = snapshot("hub", &household(), vec![blob.clone()]);
    let id = blob.id().unwrap();
    let winner = chosen(
        &base,
        &kinds(),
        &MethodRegistry::builtin(),
        &mut Scratch::default(),
    )[&id][0]
        .clone();
    // The carrier went silent past the horizon: its liveness is unknown. Its claim still reads
    // honored (a claim outlives its carrier until a holding probe exists) in the first leg, and is
    // unreachable in the second; only liveness differs from a live holder in the first.
    for claimed in [Answer::Present(true), Answer::Unreachable] {
        let mut s = base.clone();
        s.carrying.push(Carrying {
            carrier: winner.clone(),
            ..legacy_row("ignored", &blob, claimed)
        });
        for c in &mut s.candidates {
            if c.agent() == &winner {
                c.live = Answer::Unreachable;
            }
        }
        let plans = everyone(&s);
        let adopters: Vec<AgentRef> = plans
            .iter()
            .flat_map(|p| p.adopt.iter().map(|d| d.provider.clone()))
            .collect();
        assert_eq!(adopters.len(), 1, "the need re-fills ({claimed:?})");
        assert_ne!(adopters[0], winner);
        let e = explain(&s, &kinds(), &MethodRegistry::builtin(), &id, &winner, None).unwrap();
        assert!(e
            .own_refusals
            .iter()
            .any(|r| r.why.why == IneligibleReason::Unknown { input: "liveness" }));
        assert_ne!(
            e.outcome,
            Outcome::Carried,
            "a dead carrier's claim never reads as carried ({claimed:?})"
        );
        // Unreachable is not "not honored": nobody releases on it.
        assert!(plans.iter().all(|p| p.release.is_empty()), "{claimed:?}");

        // The new holder settles, and the dead carrier stays held, not released, until it returns.
        let mut after = s.clone();
        apply_all(&mut after, &plans);
        assert!(everyone(&after)
            .iter()
            .all(|p| p.adopt.is_empty() && p.release.is_empty()));
        let e = explain(
            &after,
            &kinds(),
            &MethodRegistry::builtin(),
            &id,
            &winner,
            None,
        )
        .unwrap();
        assert_eq!(e.outcome, Outcome::Carried);
    }
}

// ---- 15. Capacity is charged once across scopes -------------------------------------------------

#[test]
fn c15_capacity_is_charged_once_across_scopes() {
    let (a, b) = (space("one"), space("two"));
    let cast = Cast {
        members: vec![("only", "A", 15, false), ("spare", "B", 1_000, false)],
        receivers: vec!["reader"],
        admitted_at: Reach::Public,
    };
    let mut first = need(
        KindId::CustodyRoot,
        "x",
        (1, 1),
        Reach::Public,
        "reader",
        &a,
    );
    let mut second = need(
        KindId::CustodyRoot,
        "y",
        (1, 1),
        Reach::Public,
        "reader",
        &b,
    );
    first.requires.bytes = Some(10);
    second.requires.bytes = Some(10);
    let s = snapshot("only", &cast, vec![first, second]);
    let only = plan_for(&s);
    let spare = plan_for(&as_me(&s, &agent("spare")));
    let total = only.adopt.len() + spare.adopt.len();
    assert_eq!(total, 2, "both needs are carried");
    assert!(
        only.adopt.len() <= 1,
        "15 bytes cannot carry two 10-byte needs across two scopes"
    );
}

#[test]
fn c15_a_need_draws_only_on_its_own_kinds_offer_and_one_device_is_charged_once() {
    let scope = space("kinds");
    let cast = Cast {
        members: vec![("both", "A", 0, false), ("spare", "B", 1_000, false)],
        receivers: vec!["reader"],
        admitted_at: Reach::Public,
    };
    let mut s = snapshot("both", &cast, Vec::new());
    // "both" offers 15 bytes of custody-root and 1000 of custody-blob: two offers, two figures.
    let both = s
        .candidates
        .iter_mut()
        .find(|c| c.agent() == &agent("both"))
        .unwrap();
    both.posture
        .offers
        .get_mut(&KindId::CustodyRoot)
        .unwrap()
        .spare_bytes = Some(15);
    both.posture
        .offers
        .get_mut(&KindId::CustodyBlob)
        .unwrap()
        .spare_bytes = Some(1_000);
    let mut root = need(
        KindId::CustodyRoot,
        "big-root",
        (1, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    root.requires.bytes = Some(100);
    s.needs = vec![root.clone()];
    s.charters.insert(CidString::from(&scope), charter());
    s.needs_read
        .insert(CidString::from(&scope), Answer::Present(()));
    let e = explain(
        &s,
        &kinds(),
        &MethodRegistry::builtin(),
        &root.id().unwrap(),
        &agent("both"),
        None,
    )
    .unwrap();
    assert!(
        e.own_refusals
            .iter()
            .any(|r| r.why.why == IneligibleReason::OverCapacity),
        "a 100-byte custody-root need never draws on the blob offer's 1000: {e:?}"
    );
    assert!(plan_for(&s).adopt.is_empty());

    // Charged once across kinds: a 990-byte blob placed on "both" leaves its root offer nothing,
    // although that offer's own figure (15) would still admit a 10-byte root.
    let mut blob = need(
        KindId::CustodyBlob,
        "big-blob",
        (1, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    blob.requires.bytes = Some(990);
    blob.class = NeedClass::Care; // served first
    blob.requires.carriers = Some([AgentRefKey::of(&agent("both"))].into_iter().collect());
    let mut small_root = need(
        KindId::CustodyRoot,
        "small-root",
        (1, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    small_root.requires.bytes = Some(10);
    let mut two = s.clone();
    two.needs = vec![blob.clone(), small_root.clone()];
    let global = chosen(
        &two,
        &kinds(),
        &MethodRegistry::builtin(),
        &mut Scratch::default(),
    );
    assert_eq!(global[&blob.id().unwrap()], vec![agent("both")]);
    assert_eq!(
        global[&small_root.id().unwrap()],
        vec![agent("spare")],
        "one device's bytes are charged once across its kinds' offers"
    );
    // And alone, the 10-byte root fits the 15-byte root offer.
    let mut alone = s.clone();
    alone.needs = vec![small_root.clone()];
    let e = explain(
        &alone,
        &kinds(),
        &MethodRegistry::builtin(),
        &small_root.id().unwrap(),
        &agent("both"),
        None,
    )
    .unwrap();
    assert!(e.own_refusals.is_empty(), "{e:?}");
}

// ---- fix round 1: re-expression, several moves, demand bounded by the charter --------------------

fn root_row(name: &str, n: &Need, under: &Cid) -> Carrying {
    Carrying {
        commitment: commitment_id(KindId::CustodyRoot, &agent(name), &n.receiver, &n.key),
        need: n.id().unwrap(),
        key: n.key.clone(),
        kind: KindId::CustodyRoot,
        carrier: agent(name),
        receiver: n.receiver.clone(),
        in_scope_of: n.in_scope_of,
        lease: Lease {
            from: 0,
            until: None,
        },
        under_params: *under,
        honored: Answer::Present(true),
    }
}

#[test]
fn c4_a_re_expressed_need_keeps_its_commitments() {
    let scope = space("re");
    let narrow = need(
        KindId::CustodyRoot,
        "tree",
        (2, 1),
        Reach::Intimate,
        "hub",
        &scope,
    );
    let mut s = snapshot("hub", &household(), vec![narrow.clone()]);
    settle_household(&mut s);
    let holders_before: BTreeSet<String> = honored_of(&s, &narrow.id().unwrap())
        .into_iter()
        .map(|a| a.0)
        .collect();
    assert!(!holders_before.is_empty());

    // The same key re-expressed at a wider reach: a new Need::id, the same commitment identity.
    let mut widened = narrow.clone();
    widened.reach = Reach::Public;
    assert_ne!(widened.id().unwrap(), narrow.id().unwrap());
    s.needs = vec![widened];
    for p in everyone(&s) {
        let adopted: BTreeSet<&str> = p.adopt.iter().map(|d| d.id.as_str()).collect();
        for r in &p.release {
            assert!(
                !adopted.contains(r.commitment.as_str()),
                "one plan adopts and releases {}",
                r.commitment
            );
            assert_ne!(
                r.reason,
                ReleaseReason::NeedWithdrawn,
                "a re-expressed need is not withdrawn"
            );
        }
    }
    let plans = everyone(&s);
    apply_all(&mut s, &plans);
    let holders_after: BTreeSet<String> = s
        .carrying
        .iter()
        .filter(|c| c.key == narrow.key && c.honored == Answer::Present(true))
        .map(|c| c.carrier.0.clone())
        .collect();
    assert!(
        holders_before.is_subset(&holders_after),
        "the holders stay honored"
    );

    // A need that is really gone still releases its carrying.
    s.needs.clear();
    let released: usize = everyone(&s)
        .iter()
        .flat_map(|p| p.release.iter())
        .filter(|r| r.reason == ReleaseReason::NeedWithdrawn)
        .count();
    assert_eq!(released, holders_after.len());
}

#[test]
fn a_failed_need_read_never_drains_a_scope() {
    let scope = space("unread");
    let tree = need(
        KindId::CustodyRoot,
        "tree",
        (2, 1),
        Reach::Intimate,
        "hub",
        &scope,
    );
    let mut s = snapshot("hub", &household(), vec![tree.clone()]);
    settle_household(&mut s);
    let held = s.carrying.len();
    assert!(held > 0);
    // The source could not read the scope and came back empty: nothing is withdrawn.
    s.needs.clear();
    for read in [Some(Answer::Unreachable), None] {
        match read {
            Some(a) => s.needs_read.insert(CidString::from(&scope), a),
            None => s.needs_read.remove(&CidString::from(&scope)),
        };
        let released: usize = everyone(&s).iter().map(|p| p.release.len()).sum();
        assert_eq!(released, 0, "an unread scope holds every carrying");
    }
    // Read and genuinely empty, every holder releases.
    s.needs_read
        .insert(CidString::from(&scope), Answer::Present(()));
    let released: usize = everyone(&s)
        .iter()
        .flat_map(|p| p.release.iter())
        .filter(|r| r.reason == ReleaseReason::NeedWithdrawn)
        .count();
    assert_eq!(released, held);
}

#[test]
fn c4_a_cap_above_one_supersedes_distinct_holders() {
    let scope = space("m2");
    let tree = need(
        KindId::CustodyRoot,
        "tree",
        (2, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    let id = tree.id().unwrap();
    let mut s = snapshot("hub", &household(), vec![tree.clone()]);
    let old = *InForce::builtin(KindId::CustodyRoot).ranking_cid();
    s.carrying = vec![
        root_row("laptop", &tree, &old),
        root_row("gp-tablet", &tree, &old),
    ];
    let mut set = spare_heavy();
    set.max_moves_per_event = 2;
    s.params.insert(
        (CidString::from(&scope), KindId::CustodyRoot),
        ratified(&scope, set, 10),
    );
    let under_new = chosen(
        &s,
        &kinds(),
        &MethodRegistry::builtin(),
        &mut Scratch::default(),
    )[&id]
        .clone();
    assert!(
        !under_new.contains(&agent("laptop")) && !under_new.contains(&agent("gp-tablet")),
        "the fixture moves both holders: {under_new:?}"
    );
    let replaces: Vec<String> = everyone(&s)
        .iter()
        .flat_map(|p| p.adopt.iter())
        .filter_map(|d| d.replaces.clone())
        .collect();
    assert_eq!(replaces.len(), 2, "two moves under a cap of two");
    let distinct: BTreeSet<&String> = replaces.iter().collect();
    assert_eq!(
        distinct.len(),
        2,
        "each successor names its own predecessor"
    );
}

#[test]
fn a_need_cannot_raise_its_own_demand_past_the_charter() {
    let scope = space("ask");
    let greedy = need(
        KindId::CustodyRoot,
        "tree",
        (500, 9),
        Reach::Public,
        "reader",
        &scope,
    );
    let id = greedy.id().unwrap();
    let s = snapshot("hub", &household(), vec![greedy]);
    let ceiling = charter().ceiling();
    assert_eq!(
        ceiling,
        Demand {
            holders: 3,
            domains: 2
        }
    );
    let placed = chosen(
        &s,
        &kinds(),
        &MethodRegistry::builtin(),
        &mut Scratch::default(),
    )[&id]
        .clone();
    assert_eq!(placed.len(), ceiling.holders as usize);
    let adopted: usize = everyone(&s).iter().map(|p| p.adopt.len()).sum();
    assert_eq!(adopted, ceiling.holders as usize);
    // The holding floor still raises an under-ask, and domains never exceed holders.
    let c = charter();
    assert_eq!(
        c.bound(Demand {
            holders: 0,
            domains: 0
        }),
        c.holding_floor
    );
    assert_eq!(
        c.bound(Demand {
            holders: 2,
            domains: 9
        }),
        Demand {
            holders: 2,
            domains: 2
        }
    );
}

// ---- Codex review 1 ----------------------------------------------------------------------------

/// Findings 2 and 6: a carrier the move cap keeps from adopting is not charged, so a later need that
/// fits only that carrier's unused capacity is still carried. `big` (25 bytes) is preferred under a
/// spare-heavy set; two needs held under the old set compete for one move, and a third, unheld,
/// served last, fits `big` only if the capped need took nothing.
#[test]
fn a_move_the_cap_withholds_charges_no_capacity() {
    let scope = space("cap");
    let cast = Cast {
        members: vec![
            ("big", "A", 25, false),
            ("o1", "B", 5, false),
            ("o2", "C", 5, false),
        ],
        receivers: vec!["reader"],
        admitted_at: Reach::Public,
    };
    let mut needs: Vec<Need> = ["n0", "n1", "n2", "n3", "n4", "n5"]
        .iter()
        .map(|k| {
            need(
                KindId::CustodyRoot,
                k,
                (1, 1),
                Reach::Public,
                "reader",
                &scope,
            )
        })
        .collect();
    needs.sort_by_key(|n| n.id().unwrap().to_bytes());
    // Served in id order: the first two are held under the old set, the last is unheld.
    let (held_a, held_b, unheld) = (needs[0].clone(), needs[1].clone(), needs[5].clone());
    let mut s = snapshot(
        "big",
        &cast,
        vec![held_a.clone(), held_b.clone(), unheld.clone()],
    );
    let old = *InForce::builtin(KindId::CustodyRoot).ranking_cid();
    s.carrying = vec![root_row("o1", &held_a, &old), root_row("o2", &held_b, &old)];
    let set = spare_heavy();
    assert_eq!(set.max_moves_per_event, 1);
    s.params.insert(
        (CidString::from(&scope), KindId::CustodyRoot),
        ratified(&scope, set, 10),
    );
    let all = chosen(
        &s,
        &kinds(),
        &MethodRegistry::builtin(),
        &mut Scratch::default(),
    );
    for n in [&held_a, &held_b] {
        assert_eq!(
            all[&n.id().unwrap()],
            vec![agent("big")],
            "big is preferred"
        );
    }
    let plan = plan_for(&s);
    let moves = plan.adopt.iter().filter(|d| d.replaces.is_some()).count();
    assert_eq!(moves, 1, "one move under a cap of one");
    assert!(
        plan.adopt
            .iter()
            .any(|d| d.key == unheld.key && d.replaces.is_none()),
        "the unheld need fits the capacity the capped move did not take: {:?}",
        plan.unserved
    );
    assert!(plan.unserved.iter().all(|u| u.key != unheld.key));
}

/// Finding 5: one honored commitment observed twice (under two scopes) is one holding. It never
/// satisfies a two-holder demand, and never over-holds a one-holder demand into its own release.
#[test]
fn one_commitment_observed_twice_is_one_holding() {
    let scope = space("dup");
    let other = space("dup-elsewhere");
    let builtin = *InForce::builtin(KindId::CustodyRoot).ranking_cid();
    for (holders, expect_fill) in [(1u16, false), (2, true)] {
        let tree = need(
            KindId::CustodyRoot,
            "tree",
            (holders, 1),
            Reach::Public,
            "reader",
            &scope,
        );
        let mut s = snapshot("hub", &household(), vec![tree.clone()]);
        s.needs_read
            .insert(CidString::from(&other), Answer::Present(()));
        let row = root_row("hub", &tree, &builtin);
        let mut twice = row.clone();
        twice.in_scope_of = other;
        s.carrying = vec![row, twice];
        let plans = everyone(&s);
        let released: usize = plans.iter().map(|p| p.release.len()).sum();
        assert_eq!(
            released, 0,
            "holders {holders}: the sole holding is never released"
        );
        let adopted: usize = plans.iter().map(|p| p.adopt.len()).sum();
        assert_eq!(
            adopted > 0,
            expect_fill,
            "holders {holders}: a duplicate row never stands in for a second holder"
        );
    }
}

/// The view schema, with its `$ref`s resolved from the in-tree schema files. Each fetched document's
/// own `$id` (an `epr:schema:` URN, which cannot be a base) is dropped, so its relative refs resolve
/// against the file it came from.
fn explanation_validator() -> jsonschema::Validator {
    struct InTree;
    impl jsonschema::Retrieve for InTree {
        fn retrieve(
            &self,
            uri: &jsonschema::Uri<&str>,
        ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
            let rest = uri
                .as_str()
                .split('#')
                .next()
                .and_then(|u| u.strip_prefix("file:///v1/"))
                .ok_or_else(|| format!("not an in-tree schema: {}", uri.as_str()))?;
            let path = format!("{}/../../sdk/schemas/v1/{rest}", env!("CARGO_MANIFEST_DIR"));
            let mut doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
            if let Some(o) = doc.as_object_mut() {
                o.remove("$id");
            }
            Ok(doc)
        }
    }
    let mut schema: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../sdk/schemas/v1/views/placement-explanation-view.schema.json"
    )))
    .unwrap();
    schema["$id"] = "file:///v1/views/placement-explanation-view.schema.json".into();
    jsonschema::options()
        .with_retriever(InTree)
        .build(&schema)
        .unwrap_or_else(|e| panic!("the explanation view schema does not compile: {e}"))
}

fn assert_valid(v: &jsonschema::Validator, e: &Explanation, label: &str) {
    let json = serde_json::to_value(e).unwrap();
    let errors: Vec<String> = v
        .iter_errors(&json)
        .map(|err| format!("{err} (at {})", err.instance_path))
        .collect();
    assert!(
        errors.is_empty(),
        "{label}: {errors:#?}\n{}",
        serde_json::to_string_pretty(&json).unwrap()
    );
}

/// Findings 3 and 4: every outcome `explain` produces, the observe-only ones included, validates
/// against `views/placement-explanation-view.schema.json` (a 64-hex `inputsDigest` on every path),
/// and a retained set reports `last-verified`.
#[test]
fn every_explanation_outcome_validates_against_the_view_schema() {
    let v = explanation_validator();
    let scope = space("schema");
    let explain_one = |s: &Snapshot, n: &Need, who: &str| {
        explain(
            s,
            &kinds(),
            &MethodRegistry::builtin(),
            &n.id().unwrap(),
            &agent(who),
            Some([7; 32]),
        )
        .unwrap()
    };
    let tree = need(
        KindId::CustodyRoot,
        "tree",
        (2, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    let mut huge = need(
        KindId::CustodyRoot,
        "huge",
        (1, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    huge.requires.bytes = Some(1_000_000);
    let mut s = snapshot("phone", &household(), vec![tree.clone(), huge.clone()]);

    let held = explain_one(&s, &tree, "phone");
    assert_eq!(held.outcome, Outcome::Held);
    assert_valid(&v, &held, "held");
    let short = explain_one(&s, &huge, "phone");
    assert!(matches!(short.outcome, Outcome::NotCarried { .. }));
    assert!(
        !short.own_refusals.is_empty(),
        "the phone's own refusal is listed"
    );
    assert_valid(&v, &short, "not-carried");

    settle_household(&mut s);
    let carried = explain_one(&s, &tree, "phone");
    assert_eq!(carried.outcome, Outcome::Carried);
    assert_valid(&v, &carried, "carried");

    // Observe-only: no charter for the need's scope. Still a 64-hex digest.
    let unchartered = need(
        KindId::CustodyRoot,
        "lost",
        (1, 1),
        Reach::Public,
        "reader",
        &space("no-charter"),
    );
    let mut observe = snapshot("phone", &household(), vec![unchartered.clone()]);
    observe.charters.clear();
    let o = explain_one(&observe, &unchartered, "phone");
    assert!(matches!(o.outcome, Outcome::NotCarried { .. }));
    assert_eq!(o.inputs_digest.len(), 64);
    assert!(o.inputs_digest.bytes().all(|b| b.is_ascii_hexdigit()));
    assert_eq!(o.digest_matches_commitment, None, "no ranking to compare");
    assert_valid(&v, &o, "observe-only");
    // The validator itself refuses the empty digest the observe-only path used to write.
    let mut empty = o.clone();
    empty.inputs_digest = String::new();
    assert!(!v.is_valid(&serde_json::to_value(&empty).unwrap()));

    // Inputs gone has one form; `explain` never produces it from a snapshot, so build it.
    let mut gone = o.clone();
    gone.outcome = Outcome::InputsGone {
        why: seam_contracts::Unanswered::new(
            NotCarriedReason::InputsGone,
            ComeBack::OnEvent {
                event: "need-expressed".into(),
            },
            seam_contracts::Meanwhile::Nothing {
                because: "the inputs are no longer held".into(),
            },
        ),
    };
    assert_valid(&v, &gone, "inputs-gone");

    // A retained set, answered because no declaration verified, says so (§E contract 3).
    let retained = ratified(&scope, spare_heavy(), 10);
    assert_eq!(retained.fallback(), None, "an elected head is no fallback");
    let (in_force, _) = select_in_force(
        &scope,
        KindId::CustodyRoot,
        &[],
        &sets_none(),
        &Lenient,
        &MethodRegistry::builtin(),
        &charter(),
        Some(&retained),
    );
    assert_eq!(in_force.fallback(), Some(ParamFallback::LastVerified));
    assert_eq!(in_force.cid(), retained.cid());
    s.params
        .insert((CidString::from(&scope), KindId::CustodyRoot), in_force);
    let lv = explain_one(&s, &tree, "phone");
    assert_eq!(lv.fallback, Some(ParamFallback::LastVerified));
    assert_valid(&v, &lv, "last-verified");
    // The built-in, when nothing is held, still reports `built-in`.
    assert_eq!(held.fallback, Some(ParamFallback::BuiltIn));
}

struct NoSets;
impl SetSource for NoSets {
    fn set(&self, _: &Cid) -> Answer<ParameterSet> {
        Answer::Absent
    }
}

fn sets_none() -> NoSets {
    NoSets
}

// ---- Codex review 2 ---------------------------------------------------------------------------

/// Findings 1 and 5: a pending commitment keeps its carrier's capacity reserved whether or not the
/// ranking still chooses that carrier, and whichever class is served first. `a` (15 bytes) has a
/// pending 10-byte commitment for `x`; `y` names `a` as its only carrier, and 10 more bytes do not
/// fit beside the pending ones.
#[test]
fn a_pending_commitment_keeps_its_reservation_out_of_the_prefix_and_against_earlier_classes() {
    let scope = space("pending");
    let cast = Cast {
        members: vec![("a", "A", 15, false), ("b", "B", 1_000, false)],
        receivers: vec!["reader"],
        admitted_at: Reach::Public,
    };
    let builtin = *InForce::builtin(KindId::CustodyRoot).ranking_cid();
    let x = need(
        KindId::CustodyRoot,
        "x",
        (1, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    for (ranking_changed, y_class) in [(true, NeedClass::Member), (false, NeedClass::Care)] {
        let mut y = need(
            KindId::CustodyRoot,
            "y",
            (1, 1),
            Reach::Public,
            "reader",
            &scope,
        );
        y.class = y_class;
        y.requires.carriers = Some([AgentRefKey::of(&agent("a"))].into_iter().collect());
        let mut s = snapshot("a", &cast, vec![x.clone(), y.clone()]);
        let mut pending = root_row("a", &x, &builtin);
        pending.honored = Answer::Present(false);
        s.carrying = vec![pending.clone()];
        if ranking_changed {
            s.params.insert(
                (CidString::from(&scope), KindId::CustodyRoot),
                ratified(&scope, spare_heavy(), 10),
            );
            let all = chosen(
                &s,
                &kinds(),
                &MethodRegistry::builtin(),
                &mut Scratch::default(),
            );
            assert_eq!(
                all[&x.id().unwrap()],
                vec![agent("b")],
                "the pending carrier has left x's prefix"
            );
        }
        let plans = everyone(&s);
        for p in &plans {
            assert!(
                p.adopt.iter().all(|d| d.key != y.key),
                "y fits only beside capacity a's pending commitment still holds \
                 (ranking changed: {ranking_changed}, y: {y_class:?}): {:?}",
                p.adopt
            );
            assert!(p.adopt.iter().all(|d| d.id != pending.commitment));
        }
        let mine = plan_for(&s);
        assert!(
            mine.unserved.iter().any(|u| u.key == y.key),
            "y is refused by name"
        );
    }
}

/// Finding 6: two expressions of one commitment (the same kind, key and receiver at two reaches)
/// charge their carrier once. 25 bytes hold the commitment's 10 and another need's 10 only so.
#[test]
fn two_expressions_of_one_commitment_charge_its_carrier_once() {
    let scope = space("twice");
    let cast = Cast {
        members: vec![("only", "A", 25, false)],
        receivers: vec!["reader"],
        admitted_at: Reach::Public,
    };
    let public = need(
        KindId::CustodyRoot,
        "tree",
        (1, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    let mut commons = public.clone();
    commons.reach = Reach::Commons;
    assert_ne!(public.id().unwrap(), commons.id().unwrap());
    let other = need(
        KindId::CustodyRoot,
        "z",
        (1, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    let s = snapshot("only", &cast, vec![public.clone(), commons, other.clone()]);
    let plan = plan_for(&s);
    assert_eq!(
        plan.adopt.iter().filter(|d| d.key == public.key).count(),
        1,
        "one commitment, one draft"
    );
    assert!(
        plan.adopt.iter().any(|d| d.key == other.key),
        "the other need fits once the shared commitment is charged once: {:?}",
        plan.unserved
    );
    assert!(plan.unserved.is_empty(), "{:?}", plan.unserved);
}

/// Findings 2 and 7: a need whose live, honored holdings meet its demand is served, though its
/// holder has no spare for another copy (a posture's spare already nets out what it holds), and
/// though the holder is no longer eligible for a new adoption at all.
#[test]
fn an_honored_need_is_served_though_its_holder_has_no_spare() {
    let scope = space("full");
    let cast = Cast {
        members: vec![("full", "A", 0, false)],
        receivers: vec!["reader"],
        admitted_at: Reach::Public,
    };
    let builtin = *InForce::builtin(KindId::CustodyRoot).ranking_cid();
    let tree = need(
        KindId::CustodyRoot,
        "tree",
        (1, 1),
        Reach::Public,
        "reader",
        &scope,
    );
    let mut named_elsewhere = tree.clone();
    named_elsewhere.requires.carriers = Some(
        [AgentRefKey::of(&agent("someone-else"))]
            .into_iter()
            .collect(),
    );
    for n in [tree, named_elsewhere] {
        let mut s = snapshot("full", &cast, vec![n.clone()]);
        s.carrying = vec![root_row("full", &n, &builtin)];
        let plan = plan_for(&s);
        assert!(plan.unserved.is_empty(), "{:?}", plan.unserved);
        assert!(plan.adopt.is_empty() && plan.release.is_empty());
        let e = explain(
            &s,
            &kinds(),
            &MethodRegistry::builtin(),
            &n.id().unwrap(),
            &agent("full"),
            None,
        )
        .unwrap();
        assert!(
            matches!(e.outcome, Outcome::Carried),
            "{:?} for {:?}",
            e.outcome,
            n.requires.carriers
        );
        if n.requires.carriers.is_none() {
            // Retaining a holding is measured with the holding given back: the holder stays
            // eligible and chosen, never refused `OverCapacity` for a copy it already holds.
            assert!(e.own_refusals.is_empty(), "{:?}", e.own_refusals);
            assert_eq!(e.chosen, vec![agent("full")]);
        }
    }
}

/// Finding 9: a set verified for one scope is never another scope's authority, neither as the
/// retained fallback nor through a snapshot map entry, and a map entry of another kind is not
/// consumed either.
#[test]
fn a_set_verified_for_one_scope_is_never_anothers_authority() {
    let (a, b) = (space("scope-a"), space("scope-b"));
    let verified_a = ratified(&a, spare_heavy(), 10);
    assert!(verified_a.binds(&a, KindId::CustodyRoot));
    assert!(!verified_a.binds(&b, KindId::CustodyRoot));
    assert!(!verified_a.binds(&a, KindId::CustodyBlob));
    let builtin = InForce::builtin(KindId::CustodyRoot);
    assert!(builtin.binds(&a, KindId::CustodyRoot) && builtin.binds(&b, KindId::CustodyRoot));

    let elect = |scope: &Cid| {
        select_in_force(
            scope,
            KindId::CustodyRoot,
            &[],
            &sets_none(),
            &Lenient,
            &MethodRegistry::builtin(),
            &charter(),
            Some(&verified_a),
        )
        .0
    };
    let in_a = elect(&a);
    assert_eq!(in_a.cid(), verified_a.cid());
    assert_eq!(in_a.fallback(), Some(ParamFallback::LastVerified));
    let in_b = elect(&b);
    assert_eq!(in_b.cid(), builtin.cid(), "A's retained set is not B's");

    let tree = need(
        KindId::CustodyRoot,
        "tree",
        (1, 1),
        Reach::Public,
        "reader",
        &b,
    );
    let mut s = snapshot("hub", &household(), vec![tree]);
    let params_of = |s: &Snapshot| -> Vec<CidString> {
        everyone(s)
            .iter()
            .flat_map(|p| p.adopt.iter().map(|d| d.params.clone()))
            .collect()
    };
    let key = (CidString::from(&b), KindId::CustodyRoot);
    for (label, entry) in [
        ("another scope's set", verified_a.clone()),
        ("another kind's set", InForce::builtin(KindId::CustodyBlob)),
    ] {
        s.params.insert(key.clone(), entry);
        let params = params_of(&s);
        assert!(!params.is_empty());
        assert!(
            params.iter().all(|p| *p == CidString::from(builtin.cid())),
            "{label} is not consumed: {params:?}"
        );
    }
    let verified_b = ratified(&b, spare_heavy(), 10);
    s.params.insert(key, verified_b.clone());
    assert!(params_of(&s)
        .iter()
        .all(|p| *p == CidString::from(verified_b.cid())));
}
