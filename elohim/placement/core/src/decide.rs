//! D.7 — the charter, the snapshot and `decide`.
//!
//! `decide` computes the WHOLE deterministic assignment, then returns `me`'s slice, so agreement is
//! checkable directly ([`chosen`] exposes the global assignment the slices are cut from).

use std::collections::{BTreeMap, BTreeSet};

use cid::Cid;
use elohim_epr::Reach;
use elohim_epr_rea::AgentRef;
use seam_contracts::{Answer, ComeBack, Meanwhile, Unanswered};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::adopt::{
    commitment_id, Carrying, CommitmentDraft, FulfilmentDraft, Lease, Period, RecordPlane, Release,
    ReleaseReason,
};
use crate::eligibility::{admit, Eligible, Ineligible, IneligibleReason};
use crate::explain::{NotCarried, NotCarriedReason};
use crate::kind::{Charge, KindId, KindRegistry, MethodRegistry, PlacementKind};
use crate::need::{Demand, Depth, Need, NeedClass, NeedKey, PrivateNeed};
use crate::offer::{
    Candidate, Capacity, CidString, DeviceFacts, EpochSecs, FailureDomain, PersonChoices, Posture,
};
use crate::params::{InForce, RatificationRule};
use crate::rank::{diversify, MethodId, RankInput, Ranking, Scratch, Weights};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LossDomain {
    Device,
    Household,
    Region,
}

/// The space's constitutional input (one of R5's three authorities). Not tunable by a parameter set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Charter {
    /// `None`: no charter (every household today).
    pub id: Option<Cid>,
    /// When the charter `id` names was adopted, from the adoption record the snapshot builder holds
    /// (0 with no charter). The election clock of a `Charter` head (E4): core reads it here, never
    /// from the head's own `ratified_at`, so a charter amendment's genesis set displaces an older
    /// tuning without the declarer's clock deciding anything.
    pub adopted_at: EpochSecs,
    /// loss → holders and domains (spec §19 Q2 owner).
    pub demand: BTreeMap<LossDomain, Demand>,
    /// Scarcity order; empty: the built-in order, said so.
    pub serve_order: Vec<NeedClass>,
    pub holding_floor: Demand,
    /// A shared tier's appropriation rule (spec §12.3 rule 3).
    pub forwarding: Option<ForwardingRule>,
    /// Per kind, the sets the charter itself ratifies (§E; E7).
    pub genesis_params: BTreeMap<KindId, Cid>,
    /// How a tuning is ratified here (§E; E8).
    pub tuning_rule: RatificationRule,
    /// The charter-declared range for `max_moves_per_event` (E6).
    pub moves_per_event: (u16, u16),
    /// Liveness cut-off for postures and inventory (D3, D4).
    pub freshness_horizon_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardingRule {
    pub max_in_flight: u16,
    pub bytes_per_window: u64,
    pub window_secs: u64,
    pub clause: String,
}

impl Charter {
    /// Never below `holding_floor`; `domains <= holders`.
    pub fn resolve(&self, loss: LossDomain) -> Demand {
        let d = self
            .demand
            .get(&loss)
            .copied()
            .unwrap_or(self.holding_floor);
        clamp(d, self.holding_floor)
    }

    /// The widest demand this charter declares for any loss domain, never below `holding_floor`.
    /// A `Need` names no loss domain, so this is the most any need in the space may ask for.
    pub fn ceiling(&self) -> Demand {
        self.demand
            .values()
            .fold(self.holding_floor, |a, d| Demand {
                holders: a.holders.max(d.holders),
                domains: a.domains.max(d.domains),
            })
    }

    /// What a need is placed at: its own demand, raised to `holding_floor` and capped at
    /// [`Charter::ceiling`], with `domains <= holders`. A need cannot raise its own demand past the
    /// charter: whoever writes a `Need` does not decide how much of every peer's capacity it draws.
    pub fn bound(&self, asked: Demand) -> Demand {
        let (floor, cap) = (self.holding_floor, self.ceiling());
        let holders = asked.holders.clamp(floor.holders, cap.holders);
        Demand {
            holders,
            domains: asked.domains.clamp(floor.domains, cap.domains).min(holders),
        }
    }

    fn serve_position(&self, class: NeedClass) -> usize {
        let order: &[NeedClass] = if self.serve_order.is_empty() {
            NeedClass::BUILTIN_ORDER
        } else {
            &self.serve_order
        };
        order
            .iter()
            .position(|c| *c == class)
            .unwrap_or(order.len())
    }
}

fn clamp(d: Demand, floor: Demand) -> Demand {
    let holders = d.holders.max(floor.holders);
    Demand {
        holders,
        domains: d.domains.max(floor.domains).min(holders),
    }
}

/// One `Floor` per peer across every scope it is a member of (D9): capacity, which a posture offers
/// without a scope, is charged once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub me: AgentRef,
    /// Legacy transport ids of me (D5).
    pub me_aliases: Vec<String>,
    pub now: EpochSecs,
    pub period: Period,
    /// Per scope.
    pub charters: BTreeMap<CidString, Charter>,
    pub needs: Vec<Need>,
    /// Per scope, how its needs read (`NeedSource::needs`): `needs` holds a scope's expressed needs
    /// only when this reads `Present`. A carrying is released `NeedWithdrawn` only in such a scope;
    /// a scope with no entry was not read, and nothing in it is withdrawn.
    pub needs_read: BTreeMap<CidString, Answer<()>>,
    /// Every posture held, unique by carrier, resolved by the observer.
    pub candidates: Vec<Candidate>,
    /// Observed commitments, mine pending included, and their honored answers.
    pub carrying: Vec<Carrying>,
    /// §E, keyed by (scope, kind) (E15); absent → `InForce::builtin`.
    pub params: BTreeMap<(CidString, KindId), InForce>,
}

impl Snapshot {
    /// An empty snapshot for `me`, for a `Ports` adapter to fill.
    pub fn empty(me: AgentRef) -> Self {
        Self {
            me,
            me_aliases: Vec::new(),
            now: 0,
            period: Period {
                start: 0,
                len_secs: 0,
            },
            charters: BTreeMap::new(),
            needs: Vec::new(),
            needs_read: BTreeMap::new(),
            candidates: Vec::new(),
            carrying: Vec::new(),
            params: BTreeMap::new(),
        }
    }

    /// A carrier as the snapshot knows it: `me` for any of my legacy transport ids.
    pub(crate) fn normalize<'s>(&'s self, agent: &'s AgentRef) -> &'s AgentRef {
        if self.me_aliases.contains(&agent.0) {
            &self.me
        } else {
            agent
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Plan {
    pub adopt: Vec<CommitmentDraft>,
    pub release: Vec<Release>,
    pub settle: Vec<FulfilmentDraft>,
    pub unserved: Vec<NotCarried>,
    pub observe_only: Vec<(Cid, MethodId)>,
    /// Needs whose `Need::id` did not encode, by commitment identity (kind, key, receiver). Never
    /// dropped silently: each is named here, nothing is decided for it, and its carryings are held
    /// (never `NeedWithdrawn`). Additive to the spec (fix round 2).
    pub unencodable: Vec<(KindId, NeedKey, AgentRef)>,
}

// ---- the pass ---------------------------------------------------------------------------------

/// Why a need was only observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Observed {
    NoCharter,
    UnknownKind,
    MethodNotRun,
}

pub(crate) struct Ranked<'s> {
    /// Eligible carriers in the method's full order: (agent, domain, incumbent, posture reach).
    pub order: Vec<(&'s AgentRef, &'s FailureDomain, bool, Reach)>,
    pub ineligible: Vec<Ineligible>,
    pub chosen: Vec<&'s AgentRef>,
    pub demand: Demand,
    pub honored: usize,
    pub honored_domains: usize,
    pub digest: [u8; 32],
    pub short: Option<NotCarriedReason>,
    /// Global planned adoptions: (carrier, replaces).
    pub adopt: Vec<(&'s AgentRef, Option<String>)>,
    /// The one drain or supersession this pass: (commitment, provider, reason).
    pub release: Option<(String, &'s AgentRef, ReleaseReason)>,
    pub receiver_household: Option<String>,
}

pub(crate) struct NeedEval<'s> {
    pub need: &'s Need,
    pub id: Cid,
    pub in_force: InForce,
    /// True when no set was held for (scope, kind) and the built-in answered.
    pub builtin_fallback: bool,
    pub outcome: Result<Ranked<'s>, Observed>,
}

pub(crate) struct Pass<'s> {
    pub evals: Vec<NeedEval<'s>>,
    /// Releases of commitments whose need is gone: (commitment, provider).
    pub withdrawn: Vec<(&'s Carrying, &'s AgentRef)>,
    /// Needs whose id did not encode (`Plan::unencodable`).
    pub unencodable: Vec<&'s Need>,
}

/// The identity a commitment id hashes, (kind, key, receiver) (`commitment_id`): a need re-expressed
/// under a new `Need::id` (a wider reach, another demand) keeps its carryings as its own, so the
/// commitment is neither withdrawn nor re-adopted under the same id (make-before-break).
pub(crate) type Ident<'s> = (KindId, &'s NeedKey, &'s str);

fn ident_of_need(n: &Need) -> Ident<'_> {
    (n.kind, &n.key, n.receiver.0.as_str())
}

fn ident_of_carrying(c: &Carrying) -> Ident<'_> {
    (c.kind, &c.key, c.receiver.0.as_str())
}

/// One reservation per (carrier, commitment identity): what the pass holds on that carrier for that
/// commitment. `charged` is always the sum of a carrier's reservations, so one commitment is never
/// charged twice, whichever expression of its need or whichever ranking prefix reached it.
type Reserved<'s> = BTreeMap<(&'s str, Ident<'s>), Charge>;

fn domains_count(domains: &[&FailureDomain]) -> usize {
    domains.iter().collect::<BTreeSet<_>>().len()
}

fn digest_of(
    need_id: &Cid,
    in_force: &InForce,
    order: &[(&AgentRef, &FailureDomain, bool, Reach)],
) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(need_id.to_bytes());
    h.update(in_force.cid().to_bytes());
    h.update(in_force.ranking_cid().to_bytes());
    for (agent, domain, incumbent, _) in order {
        h.update(agent.0.as_bytes());
        h.update([0]);
        h.update(domain.as_str().as_bytes());
        h.update([0, u8::from(*incumbent)]);
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&h.finalize());
    out
}

/// The inputs digest of a need this peer only observed: the need and the set in force, with no
/// ranked order (no ranking ran). Distinct from every ranked digest that has at least one eligible
/// carrier; equal to a ranked digest with none, which has the same inputs.
pub(crate) fn observed_digest(need_id: &Cid, in_force: &InForce) -> [u8; 32] {
    digest_of(need_id, in_force, &[])
}

pub(crate) fn run_pass<'s>(
    s: &'s Snapshot,
    kinds: &KindRegistry,
    methods: &MethodRegistry,
    scratch: &mut Scratch,
) -> Pass<'s> {
    scratch.prepare(s.candidates.iter().map(|c| c.agent().0.as_str()));

    // Canonical candidate order, unique by carrier (smallest domain kept, D12): no input order
    // reaches the plan.
    let mut by_agent: BTreeMap<&str, &Candidate> = BTreeMap::new();
    for c in &s.candidates {
        let key = c.agent().0.as_str();
        match by_agent.get(key) {
            Some(existing) if existing.domain <= c.domain => {}
            _ => {
                by_agent.insert(key, c);
            }
        }
    }
    let candidates: Vec<&Candidate> = by_agent.values().copied().collect();
    let domain_of = |agent: &AgentRef| by_agent.get(agent.0.as_str()).copied().map(|c| &c.domain);

    // What each carrier has been charged this pass, summed across kinds and scopes. A need of one
    // kind is admitted against that kind's own offer net of this sum (`remaining_for`), so no kind
    // draws on capacity another kind offered, and one device is charged once.
    let mut charged: BTreeMap<&str, Charge> = BTreeMap::new();
    let mut charged_by: BTreeMap<&str, NeedClass> = BTreeMap::new();
    // Carriers whose posture is live within the horizon (D3): only their carryings hold a need.
    let live: BTreeSet<&str> = candidates
        .iter()
        .filter(|c| c.live == Answer::Present(true))
        .map(|c| c.agent().0.as_str())
        .collect();

    // Needs in (serve-order position of class, need id), deduplicated by id. A need whose id does
    // not encode is set aside by name and still counts as expressed, so its carryings are held.
    let unencodable: Vec<&Need> = s.needs.iter().filter(|n| n.id().is_err()).collect();
    let mut needs: Vec<(usize, Vec<u8>, Cid, &Need)> = s
        .needs
        .iter()
        .filter_map(|n| {
            let id = n.id().ok()?;
            let pos = s
                .charters
                .get(&CidString::from(&n.in_scope_of))
                .map(|c| c.serve_position(n.class))
                .unwrap_or(usize::MAX);
            Some((pos, id.to_bytes(), id, n))
        })
        .collect();
    needs.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    needs.dedup_by(|a, b| a.1 == b.1);
    // Built once per pass (§4's `O(needs × k log k)`): the expressed identities and the carryings
    // by identity, so no need scans every carrying and no carrying scans every need.
    let expressed: BTreeSet<Ident<'s>> = needs
        .iter()
        .map(|n| ident_of_need(n.3))
        .chain(unencodable.iter().map(|n| ident_of_need(n)))
        .collect();
    let mut carrying_by: BTreeMap<Ident<'s>, Vec<&'s Carrying>> = BTreeMap::new();
    for c in &s.carrying {
        carrying_by.entry(ident_of_carrying(c)).or_default().push(c);
    }

    // Outstanding pending commitments are reserved before any need is allocated, whatever the
    // ranking now prefers and whichever class comes first (Codex review 2, findings 1 and 5): a
    // pending commitment still needs its capacity until it is honored or released. The reservation
    // includes the kind's charge for every expression, so a trusted task envelope cannot be
    // under-reserved by a lowered requirement. The need's own evaluation credits it back.
    // An honored holding is not reserved: a posture's spare
    // already nets it out. A carrying whose need is not expressed reserves nothing (it is released
    // `NeedWithdrawn`, or held unread, and either way no admission here weighs against it).
    let mut floor_of: BTreeMap<Ident<'s>, (Charge, NeedClass, Vec<&Need>)> = BTreeMap::new();
    for (_, _, _, n) in &needs {
        let slot =
            floor_of
                .entry(ident_of_need(n))
                .or_insert((Charge::default(), n.class, Vec::new()));
        slot.0 = slot.0.max(&Charge::of_requires(&n.requires));
        slot.2.push(n);
    }
    let mut reserved: Reserved<'s> = BTreeMap::new();
    for (ident, rows) in &carrying_by {
        let Some((floor, class, expressions)) = floor_of.get(ident) else {
            continue;
        };
        let honored: BTreeSet<&str> = rows
            .iter()
            .map(|c| s.normalize(&c.carrier).0.as_str())
            .zip(rows.iter())
            .filter(|(a, c)| c.honored == Answer::Present(true) && live.contains(a))
            .map(|(a, _)| a)
            .collect();
        for c in rows {
            let agent = s.normalize(&c.carrier).0.as_str();
            if c.honored == Answer::Absent || honored.contains(agent) {
                continue;
            }
            let key = (agent, *ident);
            if reserved.contains_key(&key) {
                continue;
            }
            let mut charge = *floor;
            if let Some(candidate) = by_agent.get(agent) {
                if let Some(offer) = candidate.posture.offers.get(&c.kind) {
                    let capacity = Capacity::of_offer(offer);
                    // A retained pending commitment whose charge cannot be established does not
                    // make its offered room free. Hold that room until the commitment is resolved.
                    let unresolved = Charge {
                        bytes: capacity.bytes.unwrap_or(0),
                        memory_bytes: capacity.memory_bytes.unwrap_or(0),
                        cpu_millis: capacity.cpu_millis.unwrap_or(0),
                        pids: capacity.pids.unwrap_or(0),
                    };
                    for need in expressions {
                        let trusted = kinds
                            .get(c.kind)
                            .and_then(|kind| kind.fits(need, candidate, offer, &capacity).ok())
                            .unwrap_or(unresolved);
                        charge = charge.max(&trusted);
                    }
                }
            }
            if reserved.insert(key, charge).is_none() {
                let total = charged.entry(agent).or_default();
                *total = total.plus(&charge);
                charged_by.entry(agent).or_insert(*class);
            }
        }
    }

    let mut moves: BTreeMap<(CidString, KindId), u16> = BTreeMap::new();
    let mut evals = Vec::with_capacity(needs.len());

    for (_, _, id, need) in needs {
        let scope = CidString::from(&need.in_scope_of);
        // A held set is consumed only for the (scope, kind) it was verified for (Codex review 2,
        // finding 9): a map entry naming another kind, or a set verified for another scope, is not
        // this need's authority, and the built-in answers.
        let held = s
            .params
            .get(&(scope.clone(), need.kind))
            .filter(|f| f.binds(&need.in_scope_of, need.kind));
        let in_force = held.cloned().unwrap_or_else(|| InForce::builtin(need.kind));
        let builtin_fallback = held.is_none();
        let charter = s.charters.get(&scope);
        let kind = kinds.get(need.kind);
        let method = methods.get(in_force.set().method);
        let observed = match (charter, kind, method) {
            (None, _, _) => Some(Observed::NoCharter),
            (_, None, _) => Some(Observed::UnknownKind),
            (_, _, None) => Some(Observed::MethodNotRun),
            _ => None,
        };
        let (Some(charter), Some(kind), Some(method)) = (charter, kind, method) else {
            evals.push(NeedEval {
                need,
                id,
                in_force,
                builtin_fallback,
                outcome: Err(observed.unwrap_or(Observed::MethodNotRun)),
            });
            continue;
        };
        let ident = ident_of_need(need);
        let ranked = rank_need(RankArgs {
            s,
            need,
            ident,
            carrying: carrying_by.get(&ident).map(Vec::as_slice).unwrap_or(&[]),
            reserved: &mut reserved,
            id: &id,
            kind,
            method,
            in_force: &in_force,
            charter,
            candidates: &candidates,
            live: &live,
            charged: &mut charged,
            charged_by: &mut charged_by,
            moves: moves.entry((scope, need.kind)).or_insert(0),
            scratch,
            domain_of: &domain_of,
        });
        evals.push(NeedEval {
            need,
            id,
            in_force,
            builtin_fallback,
            outcome: Ok(ranked),
        });
    }

    // Withdrawn only where the carrying's scope read `Present`: a scope that was not read, or read
    // `Unreachable`, holds every carrying it had.
    let read = |c: &Carrying| {
        matches!(
            s.needs_read.get(&CidString::from(&c.in_scope_of)),
            Some(Answer::Present(()))
        )
    };
    let withdrawn = s
        .carrying
        .iter()
        .filter(|c| {
            kinds.get(c.kind).is_some() && read(c) && !expressed.contains(&ident_of_carrying(c))
        })
        .map(|c| (c, s.normalize(&c.carrier)))
        .collect();

    Pass {
        evals,
        withdrawn,
        unencodable,
    }
}

struct RankArgs<'s, 'p, F: Fn(&AgentRef) -> Option<&'s FailureDomain>> {
    s: &'s Snapshot,
    need: &'s Need,
    ident: Ident<'s>,
    /// The carryings of this need's commitment identity (the pass's index).
    carrying: &'p [&'s Carrying],
    reserved: &'p mut Reserved<'s>,
    id: &'p Cid,
    kind: &'p dyn PlacementKind,
    method: &'p dyn Ranking,
    in_force: &'p InForce,
    charter: &'s Charter,
    candidates: &'p [&'s Candidate],
    live: &'p BTreeSet<&'s str>,
    charged: &'p mut BTreeMap<&'s str, Charge>,
    charged_by: &'p mut BTreeMap<&'s str, NeedClass>,
    moves: &'p mut u16,
    scratch: &'p Scratch,
    domain_of: &'p F,
}

fn rank_need<'s, 'p, F: Fn(&AgentRef) -> Option<&'s FailureDomain>>(
    a: RankArgs<'s, 'p, F>,
) -> Ranked<'s> {
    let RankArgs {
        s,
        need,
        ident,
        carrying,
        reserved,
        id,
        kind,
        method,
        in_force,
        charter,
        candidates,
        live,
        charged,
        charged_by,
        moves,
        scratch,
        domain_of,
    } = a;
    let receiver_domain = domain_of(&need.receiver);
    let carrier = |c: &'s Carrying| s.normalize(&c.carrier);

    // Honored holders: claimed Present(true), any set, by a carrier live within the horizon (D3,
    // contract 14). A claim outlives its carrier until a holding probe exists, so a carrier gone
    // silent never holds the need; it is not released either ("Unreachable is not not-honored").
    //
    // One entry per distinct carrier (Codex review 1, finding 5): one commitment observed under
    // several scopes, or a carrier holding under a legacy id and its own, is one holding, never two.
    // A carrier's row under the set in force is the one kept, so a duplicate row under an older set
    // never invents a move. Releasing a carrier's commitment therefore removes every observation of
    // it from the counts; a second commitment on the same carrier, if one exists, is left held and
    // drains on a later pass, so the counts never claim fewer holdings than remain.
    let mut honored: Vec<&'s Carrying> = Vec::new();
    for c in carrying
        .iter()
        .copied()
        .filter(|c| c.honored == Answer::Present(true) && live.contains(carrier(c).0.as_str()))
    {
        match honored.iter().position(|h| carrier(h) == carrier(c)) {
            None => honored.push(c),
            Some(at) => {
                let current = |x: &Carrying| x.under_params == *in_force.ranking_cid();
                let keep = honored[at];
                let replace = (current(c) && !current(keep))
                    || (current(c) == current(keep) && c.commitment < keep.commitment);
                if replace {
                    honored[at] = c;
                }
            }
        }
    }
    let honored_by: BTreeSet<&str> = honored.iter().map(|c| carrier(c).0.as_str()).collect();
    // A holding's own footprint, given back when its holder is admitted to retain it.
    let holding = Charge::of_requires(&need.requires);

    // Eligibility first; refusals are kept, never ranked away. Each carrier is measured net of
    // everything charged to it except this commitment's own reservation (a pending commitment, or
    // another expression of this need, is not weighed against itself), and an honored holder with
    // its holding given back: retaining a holding never needs spare for another copy (Codex review
    // 2, findings 2 and 7).
    let mut eligible: Vec<Eligible<'s>> = Vec::new();
    let mut ineligible = Vec::new();
    let mut served_first: Option<NeedClass> = None;
    for c in candidates {
        let agent = c.agent().0.as_str();
        let own = reserved.get(&(agent, ident)).copied().unwrap_or_default();
        let net = charged.get(agent).map(|t| t.minus(&own));
        let mut left = remaining_for(c, kind.id(), net.as_ref());
        if honored_by.contains(agent) {
            left = left.credited(&holding);
        }
        match admit(kind, need, c, receiver_domain, &left, s.now) {
            Ok(e) => eligible.push(e),
            Err(refusal) => {
                if refusal.why.why == IneligibleReason::OverCapacity {
                    if let Some(class) = charged_by.get(c.agent().0.as_str()) {
                        served_first.get_or_insert(*class);
                    }
                }
                ineligible.push(refusal);
            }
        }
    }

    // Incumbency is scoped to the set in force (E5).
    let incumbents: Vec<bool> = eligible
        .iter()
        .map(|e| {
            carrying.iter().any(|c| {
                carrier(c) == e.agent()
                    && c.honored == Answer::Present(true)
                    && c.under_params == *in_force.ranking_cid()
            })
        })
        .collect();

    let input_root = need.input_root();
    let input = RankInput {
        need,
        receiver_domain,
        input_root: input_root.as_ref(),
    };
    let mut out = Vec::with_capacity(eligible.len());
    method.rank(
        &input,
        &eligible,
        &incumbents,
        &in_force.set().weights,
        scratch,
        &mut out,
    );

    let demand = charter.bound(need.demand);
    let holders = demand.holders as usize;
    let domains: Vec<&FailureDomain> = eligible.iter().map(|e| &e.candidate().domain).collect();
    let mut chosen_idx = Vec::with_capacity(holders);
    if demand.domains > 1 {
        diversify(&out, &domains, holders, &mut chosen_idx);
    } else {
        chosen_idx.extend(out.iter().copied().take(holders));
    }
    let order: Vec<(&'s AgentRef, &'s FailureDomain, bool, Reach)> = out
        .iter()
        .map(|&i| {
            let c = eligible[i].candidate();
            (
                eligible[i].agent(),
                &c.domain,
                incumbents[i],
                c.posture.reach,
            )
        })
        .collect();
    let chosen: Vec<&'s AgentRef> = chosen_idx.iter().map(|&i| eligible[i].agent()).collect();
    let digest = digest_of(id, in_force, &order);

    let fallback_domain: Vec<FailureDomain> = honored
        .iter()
        .map(|c| FailureDomain::of(None, carrier(c)))
        .collect();
    let honored_domain = |i: usize| domain_of(carrier(honored[i])).unwrap_or(&fallback_domain[i]);
    let honored_domains_of = |skip: Option<usize>| {
        domains_count(
            &(0..honored.len())
                .filter(|i| Some(*i) != skip)
                .map(honored_domain)
                .collect::<Vec<_>>(),
        )
    };
    let honored_n = honored.len();
    let honored_domains = honored_domains_of(None);
    let fill_needed = honored_n < holders || honored_domains < demand.domains as usize;
    let is_honored = |agent: &AgentRef| honored_by.contains(agent.0.as_str());
    let has_live = |agent: &AgentRef| {
        carrying
            .iter()
            .any(|c| carrier(c) == agent && c.honored != Answer::Absent)
    };
    let position = |agent: &AgentRef| order.iter().position(|(a, ..)| *a == agent);

    // Worst-ranked first: eligible by reversed rank, then the ineligible honored by agent.
    let mut honored_worst_first: Vec<usize> = (0..honored_n).collect();
    honored_worst_first.sort_by(|&x, &y| {
        let px = position(carrier(honored[x]));
        let py = position(carrier(honored[y]));
        match (px, py) {
            (Some(a), Some(b)) => b.cmp(&a),
            (None, Some(_)) => core::cmp::Ordering::Less,
            (Some(_), None) => core::cmp::Ordering::Greater,
            (None, None) => carrier(honored[y]).0.cmp(&carrier(honored[x]).0),
        }
    });

    let mut adopt: Vec<(&'s AgentRef, Option<String>)> = Vec::new();
    // Each superseded commitment is named by one successor only, so a cap above one moves several
    // distinct holders rather than spending every move on the worst one.
    let mut claimed: BTreeSet<&str> = BTreeSet::new();
    for &agent in &chosen {
        if is_honored(agent) || has_live(agent) {
            continue; // already holding, or pending: no re-draft every pass (D5)
        }
        if fill_needed {
            adopt.push((agent, None));
        } else if *moves < in_force.max_moves_per_event() {
            // A move: only once a newly in-force set reset incumbency; the successor names what it
            // replaces, and the superseded one is released only once the successor reads honored.
            let superseded = honored_worst_first.iter().map(|&i| honored[i]).find(|c| {
                c.under_params != *in_force.ranking_cid()
                    && !chosen.contains(&carrier(c))
                    && !claimed.contains(c.commitment.as_str())
            });
            if let Some(old) = superseded {
                claimed.insert(old.commitment.as_str());
                adopt.push((agent, Some(old.commitment.clone())));
                *moves += 1;
            }
        }
    }

    // Charge, in serve order across kinds and scopes, only what this pass actually takes: the
    // adoptions planned above, and the chosen carriers already pending a commitment for this need.
    // A chosen carrier the move cap kept from adopting takes nothing, so a later need may use that
    // capacity (Codex review 1, findings 2 and 6). An honored holder is not charged again. The
    // charge is this commitment's reservation on the carrier, raised to the admitted charge and
    // never added twice: a pending commitment reserved before the pass, or the same commitment
    // reached by another expression of its need, is one reservation (Codex review 2, finding 6).
    //
    // `planned` is what holds the need after this pass: the honored holdings, whether or not they
    // are still chosen, and the chosen carriers that adopt or are pending.
    let mut planned: BTreeSet<&str> = honored_by.clone();
    let mut planned_domains: Vec<&FailureDomain> = (0..honored_n).map(honored_domain).collect();
    for &i in &chosen_idx {
        let agent = eligible[i].agent();
        let adopted = adopt.iter().any(|(a, _)| *a == agent);
        let pending = has_live(agent) && !is_honored(agent);
        if !(adopted || pending) {
            continue;
        }
        let key = agent.0.as_str();
        if planned.insert(key) {
            planned_domains.push(domains[i]);
        }
        let slot = reserved.entry((key, ident)).or_default();
        let was = *slot;
        *slot = was.max(&eligible[i].charge());
        let total = charged.entry(key).or_default();
        *total = total.minus(&was).plus(slot);
        charged_by.entry(key).or_insert(need.class);
    }

    // Unmet demand is read from what holds the need, never from the chosen prefix alone: a fully
    // honored need whose holders cannot take another copy is served (Codex review 2, findings 2
    // and 7).
    let have = domains_count(&planned_domains);
    let short = if planned.len() >= holders && have >= demand.domains as usize {
        None
    } else if planned.len() < holders {
        Some(match served_first {
            Some(class) if !charter.serve_order.is_empty() => {
                NotCarriedReason::ServedFirst { class }
            }
            Some(_) => NotCarriedReason::NoPriorityChosen,
            None => NotCarriedReason::NoEligible,
        })
    } else {
        Some(NotCarriedReason::DomainShort {
            have: have as u16,
            need: demand.domains,
        })
    };

    // Drain (D1): the lowest-ranked honored carrier whose release keeps both counts.
    let mut release = None;
    if honored_n > holders || honored_domains > demand.domains as usize {
        for &i in &honored_worst_first {
            if honored_n > holders && honored_domains_of(Some(i)) >= demand.domains as usize {
                let c = honored[i];
                let reason = if c.under_params != *in_force.ranking_cid() {
                    honored
                        .iter()
                        .find(|h| {
                            h.under_params == *in_force.ranking_cid()
                                && chosen.contains(&carrier(h))
                        })
                        .map(|h| ReleaseReason::Superseded {
                            by: h.commitment.clone(),
                        })
                        .unwrap_or(ReleaseReason::OverHeld)
                } else {
                    ReleaseReason::OverHeld
                };
                release = Some((c.commitment.clone(), carrier(c), reason));
                break;
            }
        }
    }

    Ranked {
        order,
        ineligible,
        chosen,
        demand,
        honored: honored_n,
        honored_domains,
        digest,
        short,
        adopt,
        release,
        receiver_household: receiver_domain
            .and_then(|d| d.household())
            .map(str::to_string),
    }
}

/// What `candidate`'s offer for `kind` has left: that offer's own capacity, net of everything
/// already charged to the carrier this pass. With no offer for the kind, nothing is stated.
fn remaining_for(candidate: &Candidate, kind: KindId, charged: Option<&Charge>) -> Capacity {
    let offered = candidate
        .posture
        .offers
        .get(&kind)
        .map(Capacity::of_offer)
        .unwrap_or_default();
    match charged {
        Some(c) => offered.charged(c),
        None => offered,
    }
}

pub(crate) fn not_carried(why: NotCarriedReason) -> Unanswered<NotCarriedReason> {
    let on = |e: &str| ComeBack::OnEvent {
        event: e.to_string(),
    };
    let nothing = |b: &str| Meanwhile::Nothing {
        because: b.to_string(),
    };
    let (come_back, meanwhile) = match &why {
        NotCarriedReason::NoEligible => (on("posture-changed"), nothing("no eligible carrier")),
        NotCarriedReason::DomainShort { have, .. } => (
            on("posture-changed"),
            Meanwhile::UseHeld {
                what: format!("the holders in {have} failure domain(s)"),
            },
        ),
        NotCarriedReason::ServedFirst { .. } => (
            on("capacity-freed"),
            nothing("capacity went to an earlier class"),
        ),
        NotCarriedReason::NoPriorityChosen => (
            on("charter-declared"),
            nothing("capacity went first under the built-in serve order"),
        ),
        NotCarriedReason::MethodNotRun => (
            on("method-installed"),
            nothing("this peer does not run the method in force"),
        ),
        NotCarriedReason::InputsGone => (
            on("need-expressed"),
            nothing("the inputs are no longer held"),
        ),
    };
    Unanswered::new(why, come_back, meanwhile)
}

/// Computes the WHOLE deterministic assignment, then returns `me`'s slice. Needs are taken in
/// (charter serve-order position of class, need id); capacity is charged across kinds and scopes in
/// that order within the pass. See the interfaces spec §D.7 for fill, move, drain, observe and
/// scarcity; every rule there is implemented in `rank_need`.
pub fn decide(
    s: &Snapshot,
    kinds: &KindRegistry,
    methods: &MethodRegistry,
    scratch: &mut Scratch,
) -> Plan {
    let pass = run_pass(s, kinds, methods, scratch);
    let mut plan = Plan {
        unencodable: pass
            .unencodable
            .iter()
            .map(|n| (n.kind, n.key.clone(), n.receiver.clone()))
            .collect(),
        ..Plan::default()
    };
    // Two needs sharing one commitment identity (an old and a re-expressed `Need::id` both still
    // held) draft and release that commitment once.
    let mut drafted: BTreeSet<String> = BTreeSet::new();
    let mut released: BTreeSet<String> = BTreeSet::new();
    for eval in &pass.evals {
        let need = eval.need;
        let ranked = match &eval.outcome {
            Err(_) => {
                plan.observe_only
                    .push((eval.id, eval.in_force.set().method));
                continue;
            }
            Ok(r) => r,
        };
        for (agent, replaces) in &ranked.adopt {
            if *agent != &s.me {
                continue;
            }
            let Some(kind) = kinds.get(need.kind) else {
                continue;
            };
            let id = commitment_id(need.kind, &s.me, &need.receiver, &need.key);
            if !drafted.insert(id.clone()) {
                continue;
            }
            plan.adopt.push(CommitmentDraft {
                id,
                need: CidString::from(&eval.id),
                kind: need.kind,
                action: kind.action(need),
                provider: s.me.clone(),
                receiver: need.receiver.clone(),
                key: need.key.clone(),
                in_scope_of: CidString::from(&need.in_scope_of),
                lease: Lease {
                    from: s.now,
                    until: None,
                },
                plane: RecordPlane::for_need(need, ranked.receiver_household.as_deref()),
                method: eval.in_force.set().method,
                params: CidString::from(eval.in_force.cid()),
                ranking: CidString::from(eval.in_force.ranking_cid()),
                inputs_digest: ranked.digest,
                replaces: replaces.clone(),
            });
        }
        if let Some((commitment, provider, reason)) = &ranked.release {
            if *provider == &s.me && released.insert(commitment.clone()) {
                plan.release.push(Release {
                    commitment: commitment.clone(),
                    provider: s.me.clone(),
                    reason: reason.clone(),
                });
            }
        }
        if let Some(why) = &ranked.short {
            plan.unserved.push(NotCarried {
                need: CidString::from(&eval.id),
                key: need.key.clone(),
                receiver: need.receiver.clone(),
                why: not_carried(why.clone()),
            });
        }
    }
    for (c, provider) in &pass.withdrawn {
        if *provider == &s.me && released.insert(c.commitment.clone()) {
            plan.release.push(Release {
                commitment: c.commitment.clone(),
                provider: s.me.clone(),
                reason: ReleaseReason::NeedWithdrawn,
            });
        }
    }
    plan
}

/// The global assignment `decide` slices: each ranked need's chosen carriers, best first. Additive to
/// the spec, so the agreement contract compares slices with the whole directly.
pub fn chosen(
    s: &Snapshot,
    kinds: &KindRegistry,
    methods: &MethodRegistry,
    scratch: &mut Scratch,
) -> BTreeMap<Cid, Vec<AgentRef>> {
    run_pass(s, kinds, methods, scratch)
        .evals
        .iter()
        .filter_map(|e| {
            e.outcome
                .as_ref()
                .ok()
                .map(|r| (e.id, r.chosen.iter().map(|a| (*a).clone()).collect()))
        })
        .collect()
}

// ---- the device membrane ----------------------------------------------------------------------

/// Device membrane: residency and the device's own posture. Its inputs never leave the device.
pub struct PrivateSnapshot {
    pub facts: DeviceFacts,
    pub choices: PersonChoices,
    pub needs: Vec<PrivateNeed>,
    pub me: AgentRef,
    /// D20: the posture's.
    pub reach: Reach,
    pub now: EpochSecs,
    pub params: Option<DeviceParams>,
}

/// A person-ratified tuning of their own device's floor; device state, not `Serialize`, never
/// published (spec §7.3; E16). Read by nothing yet: the device floor keeps its needs in key order.
pub struct DeviceParams {
    pub weights: Weights,
}

pub struct PrivatePlan {
    pub posture: Posture,
    pub keep: Vec<NeedKey>,
    pub deferred: Vec<NeedKey>,
    pub refusals: Vec<(KindId, Unanswered<IneligibleReason>)>,
}

/// The device's own floor: what it keeps at depth, what waits for a charging or unmetered window,
/// and the posture that is all it publishes. Needs are taken in key order against the budget. A
/// need of unknown size is refused by name (`Unknown { input: "need-size" }`), never kept as if it
/// were free, and the published `spare_bytes` is what the budget holds after the device's own keeps.
pub fn decide_private(s: &PrivateSnapshot) -> PrivatePlan {
    let mut posture = s.facts.posture(s.me.clone(), &s.choices, s.reach, s.now);
    let mut budget = match s.choices.budget_bytes {
        Some(chosen) => chosen.min(s.facts.budget_bytes),
        None => s.facts.budget_bytes,
    };
    let constrained = s.facts.on_battery || s.facts.metered;
    let mut needs: Vec<&PrivateNeed> = s.needs.iter().collect();
    needs.sort_by(|a, b| a.key.cmp(&b.key));
    let (mut keep, mut deferred, mut refusals) = (Vec::new(), Vec::new(), Vec::new());
    for n in needs {
        if !matches!(n.depth, Depth::Kept | Depth::Sparse) {
            continue; // query and on-read hold no bytes ahead of a read
        }
        let Some(size) = n.budget_bytes else {
            refusals.push((
                KindId::CustodyRoot,
                IneligibleReason::Unknown { input: "need-size" }.unanswered(s.now),
            ));
            continue;
        };
        if size <= budget {
            budget -= size;
            keep.push(n.key.clone());
        } else if constrained {
            deferred.push(n.key.clone());
        } else {
            refusals.push((
                KindId::CustodyRoot,
                IneligibleReason::OverCapacity.unanswered(s.now),
            ));
        }
    }
    // The bytes just earmarked for the device's own keeps are not offered to anyone else.
    for offer in posture.offers.values_mut() {
        if offer.spare_bytes.is_some() {
            offer.spare_bytes = Some(budget);
        }
    }
    for kind in &s.choices.will_carry {
        if !posture.offers.contains_key(kind) {
            refusals.push((*kind, IneligibleReason::NoPosture.unanswered(s.now)));
        }
    }
    PrivatePlan {
        posture,
        keep,
        deferred,
        refusals,
    }
}

#[cfg(test)]
mod private_tests {
    use super::*;
    use std::collections::BTreeSet;

    fn snap(budget: u64, needs: Vec<PrivateNeed>) -> PrivateSnapshot {
        PrivateSnapshot {
            facts: DeviceFacts {
                on_battery: false,
                metered: false,
                always_on: true,
                can_steward: true,
                can_infer: false,
                budget_bytes: budget,
            },
            choices: PersonChoices {
                will_carry: BTreeSet::from([KindId::CustodyRoot]),
                lift_battery_default: false,
                budget_bytes: None,
            },
            needs,
            me: AgentRef("uhCAk-me".into()),
            reach: Reach::Intimate,
            now: 100,
            params: None,
        }
    }

    fn need(seed: &[u8], size: Option<u64>) -> PrivateNeed {
        PrivateNeed {
            key: NeedKey::TreeView(elohim_epr::cid::compute_cid(seed)),
            depth: Depth::Kept,
            budget_bytes: size,
        }
    }

    #[test]
    fn an_unsized_need_is_refused_by_name_never_kept_free() {
        let plan = decide_private(&snap(0, vec![need(b"a", None)]));
        assert!(plan.keep.is_empty(), "budget 0 keeps nothing");
        assert_eq!(plan.refusals.len(), 1);
        assert_eq!(
            plan.refusals[0].1.why,
            IneligibleReason::Unknown { input: "need-size" }
        );
    }

    #[test]
    fn the_published_spare_is_the_budget_after_own_keeps() {
        let plan = decide_private(&snap(
            1_000,
            vec![need(b"a", Some(300)), need(b"b", Some(200))],
        ));
        assert_eq!(plan.keep.len(), 2);
        assert_eq!(
            plan.posture.offers[&KindId::CustodyRoot].spare_bytes,
            Some(500)
        );
    }
}
