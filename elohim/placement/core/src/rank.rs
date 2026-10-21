//! D.5 — ranking, generalizing elohim-storage's `PlacementStrategy`.
//!
//! Rail R-P3: weights are integer basis points and every score here is an integer, so a tie is
//! broken by a total key and never by input order. Rail R-P7: every `Ranking` implementation in this
//! file is run through [`contract::assert_ranking_contract`] in this file's tests.

use core::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};

use cid::Cid;
use seam_contracts::ArbitrationKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::eligibility::Eligible;
use crate::need::Need;
use crate::offer::{CidString, FailureDomain};

/// A content-addressed ranking method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MethodId(#[serde(with = "elohim_epr_rea::index::cid_serde")] Cid);

/// `<namespace>:<area>:<name>@<version>`; the `elohim` namespace is reserved for built-ins.
pub fn is_method_name(spec: &str) -> bool {
    let Some((path, version)) = spec.rsplit_once('@') else {
        return false;
    };
    let segment = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    let parts: Vec<&str> = path.split(':').collect();
    parts.len() >= 2
        && parts.iter().all(|p| segment(p))
        && !version.is_empty()
        && version.bytes().all(|b| b.is_ascii_digit())
        && !version.starts_with('0')
}

impl MethodId {
    /// CIDv1, codec raw (0x55), sha2-256 over the UTF-8 bytes of `spec`: the construction of
    /// `BlobCid::compute_raw` (here `elohim_epr::cid::compute_raw_cid`, the same mint). A name is
    /// `<namespace>:<area>:<name>@<version>`, matching `^[a-z0-9-]+(:[a-z0-9-]+)+@[1-9][0-9]*$`, e.g.
    /// `"elohim:placement:xor-distance@1"`. A method whose behaviour changes takes a new version, and
    /// so a new `MethodId`. `named` hashes whatever it is given; a unit test checks every built-in
    /// name against the grammar, including the legacy custody scope (§D.2).
    pub fn named(spec: &str) -> Self {
        Self(elohim_epr::cid::compute_raw_cid(spec.as_bytes()))
    }
    pub fn cid(&self) -> &Cid {
        &self.0
    }
}

/// What a ranking may weigh. No reach or consent factor can exist: eligibility is not ranked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Factor {
    Distance,
    DomainSpread,
    SpareCapacity,
    NearBytes,
    AlwaysOn,
}

impl Factor {
    pub const ALL: &'static [Factor] = &[
        Factor::Distance,
        Factor::DomainSpread,
        Factor::SpareCapacity,
        Factor::NearBytes,
        Factor::AlwaysOn,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Factor::Distance => "distance",
            Factor::DomainSpread => "domain-spread",
            Factor::SpareCapacity => "spare-capacity",
            Factor::NearBytes => "near-bytes",
            Factor::AlwaysOn => "always-on",
        }
    }
}

/// Basis points (rail R-P3); every weight is > 0 in a set the ceiling admits (E6).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Weights(BTreeMap<Factor, i32>);

impl Weights {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }
    pub fn with(mut self, factor: Factor, basis_points: i32) -> Self {
        self.0.insert(factor, basis_points);
        self
    }
    pub fn get(&self, factor: Factor) -> Option<i32> {
        self.0.get(&factor).copied()
    }
    pub fn iter(&self) -> impl Iterator<Item = (Factor, i32)> + '_ {
        self.0.iter().map(|(f, w)| (*f, *w))
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The rank key: incumbency first (scoped to the parameter set in force, §D.7), then the method's
/// score, then the smaller agent key (today's XOR tie-break, kept byte for byte), then the smaller
/// failure domain (today's final tie-break, `placement.rs:177-205`; D12).
pub type RankKey<'a, S> = ArbitrationKey<bool, S, (Reverse<&'a str>, Reverse<&'a FailureDomain>)>;

/// What a ranking may read about the need (D8, F4): the key, the receiver's domain and the task's
/// input root, resolved in `Scratch`.
pub struct RankInput<'a> {
    pub need: &'a Need,
    pub receiver_domain: Option<&'a FailureDomain>,
    pub input_root: Option<&'a Cid>,
}

/// Object-safe, so `MethodRegistry` can hold `Box<dyn Ranking>`. A method's score type is internal
/// to it; the order it emits is built from `RankKey` values.
///
/// An implementor outside this crate MUST run [`contract::assert_ranking_contract`] over its type in
/// its own tests (feature `testing`); R-P7 checks that only for the built-ins in this file, so an
/// adapter's call is its own obligation, and R-P8 (elohim-storage's shim) is where a consumer's
/// rail would check it. A new behaviour is a new `MethodId` with a version, registered with
/// `MethodRegistry::with` under a non-`elohim` name, never an edit to a built-in.
pub trait Ranking: Send + Sync {
    fn method(&self) -> MethodId;
    /// The factors this method reads; `within_band` refuses a weight on any other (E2).
    fn factors(&self) -> &'static [Factor];
    /// A TOTAL order over `eligible`, best first, as indices into `eligible`. Pure; reads only the
    /// input, the candidates, the weights and the scratch's precomputed agent keys. `out` is cleared
    /// first.
    fn rank(
        &self,
        input: &RankInput<'_>,
        eligible: &[Eligible<'_>],
        incumbents: &[bool],
        weights: &Weights,
        scratch: &Scratch,
        out: &mut Vec<usize>,
    );
}

/// Reused across passes: precomputed sha256(agent) per candidate (once per snapshot, not once per
/// need). Only the current snapshot's carriers are kept, so a long-lived floor holds as many entries
/// as its latest snapshot has candidates, never every carrier it has ever seen (Codex review 1,
/// finding 8); the map's allocation is reused.
#[derive(Debug, Default)]
pub struct Scratch {
    agent_keys: HashMap<String, [u8; 32]>,
}

impl Scratch {
    /// sha256 of the agent key's UTF-8 bytes, as `placement.rs::hash_into_keyspace` computes it.
    /// `decide` fills it for every candidate in the snapshot before any `rank` call; `None` only for
    /// an agent it was not given, and a method that sees `None` computes the same digest itself.
    pub fn agent_key(&self, agent: &str) -> Option<&[u8; 32]> {
        self.agent_keys.get(agent)
    }

    /// Hash every agent of this snapshot once. A key already present for one of them is kept, so a
    /// reused scratch costs nothing for a carrier still in the snapshot; a key for any agent not
    /// in `agents` is dropped.
    pub fn prepare<'a>(&mut self, agents: impl IntoIterator<Item = &'a str>) {
        let current: std::collections::BTreeSet<&str> = agents.into_iter().collect();
        self.agent_keys.retain(|a, _| current.contains(a.as_str()));
        for agent in current {
            if !self.agent_keys.contains_key(agent) {
                self.agent_keys
                    .insert(agent.to_string(), sha256(agent.as_bytes()));
            }
        }
    }
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(&Sha256::digest(bytes));
    out
}

fn xor(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (o, (x, y)) in out.iter_mut().zip(a.iter().zip(b.iter())) {
        *o = x ^ y;
    }
    out
}

fn agent_key(scratch: &Scratch, agent: &str) -> [u8; 32] {
    scratch
        .agent_key(agent)
        .copied()
        .unwrap_or_else(|| sha256(agent.as_bytes()))
}

/// Closeness in basis points from the top 16 bits of the XOR distance: 10 000 at distance 0.
fn closeness_bp(distance: &[u8; 32]) -> i64 {
    let top = u16::from_be_bytes([distance[0], distance[1]]) as i64;
    (65_535 - top) * 10_000 / 65_535
}

/// Sort `0..keys.len()` greatest key first into `out`.
fn emit<K: Ord>(keys: &[K], out: &mut Vec<usize>) {
    out.clear();
    out.extend(0..keys.len());
    out.sort_by(|&a, &b| keys[b].cmp(&keys[a]));
}

/// The XOR order: smaller distance first, then the smaller agent, then the smaller domain.
fn xor_keys<'e>(
    input: &RankInput<'_>,
    eligible: &'e [Eligible<'e>],
    incumbents: &[bool],
    scratch: &Scratch,
) -> Vec<RankKey<'e, Reverse<[u8; 32]>>> {
    let need = sha256(&input.need.key.rank_bytes()); // once per need
    eligible
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let agent = e.agent().0.as_str();
            ArbitrationKey {
                tier: incumbents.get(i).copied().unwrap_or(false),
                clock: Reverse(xor(&need, &agent_key(scratch, agent))),
                tiebreak: (Reverse(agent), Reverse(&e.candidate().domain)),
            }
        })
        .collect()
}

/// `XorDistanceStrategy`, moved, byte-identical. Reads no weight.
pub struct XorDistance;
/// `DiversityAwarePlacementStrategy`, moved; reads no weight; its greedy emits a full order.
pub struct DiversityAware;
/// The task kind's method (§F, F9): input-root holders, then base-root holders, then the receiver's
/// domain, then XOR. A lexicographic order that reads no weight, as `XorDistance` reads none, so
/// no tuning is in band under it: a weight that could not reorder would still move `ranking_cid`
/// and reset incumbency for nothing (E5).
pub struct NearestBytes;
/// The custody tuning's method (E2): a basis-point sum of closeness, spare capacity and
/// always-on, then XOR.
pub struct WeightedCapacity;

impl WeightedCapacity {
    /// Placeholder numbers: the spec leaves `WeightedCapacity`'s default weights open (§6).
    pub const DEFAULT_WEIGHTS: &'static [(Factor, i32)] = &[
        (Factor::Distance, 5_000),
        (Factor::SpareCapacity, 3_000),
        (Factor::AlwaysOn, 2_000),
    ];
}

fn weight(weights: &Weights, defaults: &[(Factor, i32)], factor: Factor) -> i64 {
    weights
        .get(factor)
        .or_else(|| defaults.iter().find(|(f, _)| *f == factor).map(|(_, w)| *w))
        .unwrap_or(0) as i64
}

impl Ranking for XorDistance {
    fn method(&self) -> MethodId {
        MethodId::named("elohim:placement:xor-distance@1")
    }
    fn factors(&self) -> &'static [Factor] {
        &[]
    }
    fn rank(
        &self,
        input: &RankInput<'_>,
        eligible: &[Eligible<'_>],
        incumbents: &[bool],
        _weights: &Weights,
        scratch: &Scratch,
        out: &mut Vec<usize>,
    ) {
        emit(&xor_keys(input, eligible, incumbents, scratch), out);
    }
}

impl Ranking for DiversityAware {
    fn method(&self) -> MethodId {
        MethodId::named("elohim:placement:diversity-aware@1")
    }
    fn factors(&self) -> &'static [Factor] {
        &[]
    }
    fn rank(
        &self,
        input: &RankInput<'_>,
        eligible: &[Eligible<'_>],
        incumbents: &[bool],
        _weights: &Weights,
        scratch: &Scratch,
        out: &mut Vec<usize>,
    ) {
        let mut ordered = Vec::with_capacity(eligible.len());
        emit(
            &xor_keys(input, eligible, incumbents, scratch),
            &mut ordered,
        );
        let domains: Vec<&FailureDomain> = eligible.iter().map(|e| &e.candidate().domain).collect();
        // Diversified within each incumbency tier, never across them (Codex review 2, finding 10):
        // `xor_keys` already puts every incumbent first, and one diversity pass over the whole order
        // would put a non-incumbent of a fresh domain ahead of a second incumbent of a used one.
        let split = ordered
            .iter()
            .position(|&i| !incumbents[i])
            .unwrap_or(ordered.len());
        let (held, rest) = ordered.split_at(split);
        let mut tier = Vec::with_capacity(rest.len());
        diversify(held, &domains, held.len(), out);
        diversify(rest, &domains, rest.len(), &mut tier);
        out.extend(tier);
    }
}

impl Ranking for NearestBytes {
    fn method(&self) -> MethodId {
        MethodId::named("elohim:placement:nearest-bytes@1")
    }
    fn factors(&self) -> &'static [Factor] {
        &[]
    }
    fn rank(
        &self,
        input: &RankInput<'_>,
        eligible: &[Eligible<'_>],
        incumbents: &[bool],
        _weights: &Weights,
        scratch: &Scratch,
        out: &mut Vec<usize>,
    ) {
        let need = sha256(&input.need.key.rank_bytes());
        let input_root = input.input_root.map(CidString::from);
        let base_root = match &input.need.terms {
            crate::need::NeedTerms::Task { base_root, .. } => base_root.clone(),
            crate::need::NeedTerms::None => None,
        };
        // (input or base holder, the receiver's domain, nearer by XOR).
        type Score = (u8, bool, Reverse<[u8; 32]>);
        let keys: Vec<RankKey<'_, Score>> = eligible
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let holds = &e.offer().holds;
                let near = if input_root.as_ref().is_some_and(|r| holds.contains(r)) {
                    2
                } else if base_root.as_ref().is_some_and(|r| holds.contains(r)) {
                    1
                } else {
                    0
                };
                let agent = e.agent().0.as_str();
                let distance = xor(&need, &agent_key(scratch, agent));
                let same_domain = input.receiver_domain == Some(&e.candidate().domain);
                ArbitrationKey {
                    tier: incumbents.get(i).copied().unwrap_or(false),
                    clock: (near, same_domain, Reverse(distance)),
                    tiebreak: (Reverse(agent), Reverse(&e.candidate().domain)),
                }
            })
            .collect();
        emit(&keys, out);
    }
}

impl Ranking for WeightedCapacity {
    fn method(&self) -> MethodId {
        MethodId::named("elohim:placement:weighted-capacity@1")
    }
    fn factors(&self) -> &'static [Factor] {
        &[Factor::Distance, Factor::SpareCapacity, Factor::AlwaysOn]
    }
    fn rank(
        &self,
        input: &RankInput<'_>,
        eligible: &[Eligible<'_>],
        incumbents: &[bool],
        weights: &Weights,
        scratch: &Scratch,
        out: &mut Vec<usize>,
    ) {
        let need = sha256(&input.need.key.rank_bytes());
        let w_dist = weight(weights, Self::DEFAULT_WEIGHTS, Factor::Distance);
        let w_spare = weight(weights, Self::DEFAULT_WEIGHTS, Factor::SpareCapacity);
        let w_on = weight(weights, Self::DEFAULT_WEIGHTS, Factor::AlwaysOn);
        let max_spare = eligible
            .iter()
            .filter_map(|e| e.offer().spare_bytes)
            .max()
            .unwrap_or(0);
        type Score = (i64, Reverse<[u8; 32]>);
        let keys: Vec<RankKey<'_, Score>> = eligible
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let agent = e.agent().0.as_str();
                let distance = xor(&need, &agent_key(scratch, agent));
                let spare_bp = match (e.offer().spare_bytes, max_spare) {
                    (Some(s), m) if m > 0 => ((s as u128) * 10_000 / (m as u128)) as i64,
                    _ => 0,
                };
                // The posture carries no always-on fact of its own; a deferred offer is the one
                // that is not always on (D11).
                let on_bp = if e.offer().deferred { 0 } else { 10_000 };
                let score = w_dist * closeness_bp(&distance) + w_spare * spare_bp + w_on * on_bp;
                ArbitrationKey {
                    tier: incumbents.get(i).copied().unwrap_or(false),
                    clock: (score, Reverse(distance)),
                    tiebreak: (Reverse(agent), Reverse(&e.candidate().domain)),
                }
            })
            .collect();
        emit(&keys, out);
    }
}

/// The one diversity pass, as today's `placement.rs:207-237`: repeated passes, each taking at most
/// one candidate per not-yet-used failure domain in rank order, until `count` are taken or the pool
/// is exhausted (D15). With every candidate in its own domain it equals rank order. `domains` is
/// indexed like `ordered`'s values.
pub fn diversify(
    ordered: &[usize],
    domains: &[&FailureDomain],
    count: usize,
    out: &mut Vec<usize>,
) {
    out.clear();
    let mut taken = vec![false; ordered.len()];
    while out.len() < count {
        let mut used: Vec<&FailureDomain> = Vec::new();
        let mut progressed = false;
        for (pos, &i) in ordered.iter().enumerate() {
            if out.len() >= count {
                break;
            }
            if taken[pos] {
                continue;
            }
            let d = domains[i];
            if !used.contains(&d) {
                used.push(d);
                out.push(i);
                taken[pos] = true;
                progressed = true;
            }
        }
        if !progressed {
            break;
        }
    }
}

/// The ranking contract, binding on every `Ranking`.
#[cfg(any(test, feature = "testing"))]
pub mod contract {
    use std::collections::BTreeMap;

    use elohim_epr::Reach;
    use elohim_epr_rea::AgentRef;
    use seam_contracts::harness::check_ordering;
    use seam_contracts::Answer;

    use super::*;
    use crate::eligibility::eligible_for_contract;
    use crate::kind::KindId;
    use crate::need::{
        Demand, Expresser, ExpresserKind, NeedClass, NeedKey, NeedTerms, Requirements,
    };
    use crate::offer::{Candidate, OfferTerms, Posture};

    fn candidate(i: usize) -> Candidate {
        let carrier = AgentRef(format!("uhCAk-contract-{i}"));
        let mut offers = BTreeMap::new();
        let holds_root = CidString::from(&elohim_epr::cid::compute_cid(b"contract-input"));
        offers.insert(
            KindId::Task,
            OfferTerms {
                spare_bytes: Some(100 * (i as u64 % 4)),
                holds: if i.is_multiple_of(3) {
                    vec![holds_root]
                } else {
                    vec![]
                },
                deferred: !i.is_multiple_of(2),
                ..OfferTerms::default()
            },
        );
        Candidate {
            domain: FailureDomain::of(Some(&format!("{}", i % 3)), &carrier),
            posture: Posture {
                carrier,
                offers,
                reach: Reach::Public,
                written_at: 0,
            },
            live: Answer::Present(true),
            admitted: BTreeMap::new(),
            consent: BTreeMap::new(),
        }
    }

    fn need(key: &str) -> Need {
        let input = elohim_epr::cid::compute_cid(b"contract-input");
        Need {
            kind: KindId::Task,
            key: NeedKey::Blob(key.to_string()),
            demand: Demand {
                holders: 2,
                domains: 1,
            },
            requires: Requirements::default(),
            terms: NeedTerms::Task {
                input_root: CidString::from(&input),
                scope: crate::kind::GrantScope::MeasureStage,
                base_root: None,
            },
            reach: Reach::Public,
            receiver: AgentRef("uhCAk-receiver".into()),
            expressed_by: Expresser {
                by: ExpresserKind::Agent,
                id: "uhCAk-receiver".into(),
            },
            class: NeedClass::Member,
            in_scope_of: elohim_epr::cid::compute_cid(b"contract-space"),
        }
    }

    fn order(
        r: &dyn Ranking,
        need: &Need,
        items: &[(Candidate, bool)],
        weights: &Weights,
    ) -> Vec<(Candidate, bool)> {
        let eligible: Vec<Eligible<'_>> = items
            .iter()
            .map(|(c, _)| eligible_for_contract(c, &c.posture.offers[&KindId::Task]))
            .collect();
        let incumbents: Vec<bool> = items.iter().map(|(_, inc)| *inc).collect();
        // Fixed, never derived from the input order: the contract permutes the candidates.
        let receiver_domain = FailureDomain::of(Some("0"), &need.receiver);
        let input_root = need.input_root();
        let input = RankInput {
            need,
            receiver_domain: Some(&receiver_domain),
            input_root: input_root.as_ref(),
        };
        let mut scratch = Scratch::default();
        scratch.prepare(items.iter().map(|(c, _)| c.agent().0.as_str()));
        let mut out = vec![usize::MAX]; // `out` is cleared first
        r.rank(&input, &eligible, &incumbents, weights, &scratch, &mut out);
        out.iter().map(|&i| items[i].clone()).collect()
    }

    fn agents(v: &[(Candidate, bool)]) -> Vec<String> {
        v.iter().map(|(c, _)| c.agent().0.clone()).collect()
    }

    /// Binding on every Ranking: determinism; total order; no duplicates; output ⊆ input; empty →
    /// empty; agreement under any permutation of the input; incumbents first; weights reorder and
    /// never add or remove, and a method that declares factors shows at least one fixture where
    /// making a different factor dominant changes the order; a weight on a factor not in
    /// `factors()` changes nothing. Built on
    /// seam-contracts' `check_ordering`, the list form of the arbitration harness.
    ///
    /// # Panics
    /// When any property fails.
    pub fn assert_ranking_contract(r: &dyn Ranking) {
        let mut reorders = false;
        for key in ["sha256-aa", "sha256-bb", "uhblob-X"] {
            let need = need(key);
            // Two incumbents sharing one failure domain (`candidate(i)` is in domain `i % 3`), so a
            // diversity pass that crosses the incumbency tier is caught (Codex review 2, finding 10).
            let items: Vec<(Candidate, bool)> =
                (0..6).map(|i| (candidate(i), i == 1 || i == 4)).collect();
            let none = Weights::new();
            if let Err(v) = check_ordering(&items, |slice| order(r, &need, slice, &none)) {
                panic!("ranking contract failed for {:?}: {v}", r.method());
            }
            let full = order(r, &need, &items, &none);
            assert_eq!(
                full.len(),
                items.len(),
                "a ranking orders every eligible candidate and never truncates"
            );
            let first_newcomer = full.iter().position(|(_, inc)| !inc).unwrap_or(full.len());
            assert!(
                full[first_newcomer..].iter().all(|(_, inc)| !inc),
                "{:?}: every incumbent ranks ahead of every non-incumbent",
                r.method()
            );
            assert_eq!(first_newcomer, 2, "both incumbents lead");
            let mut seen = agents(&full);
            seen.sort();
            seen.dedup();
            assert_eq!(seen.len(), items.len(), "no duplicates");

            // Weights reorder and never add or remove.
            let mut weighted = Weights::new();
            for (n, f) in r.factors().iter().enumerate() {
                weighted = weighted.with(*f, 1 + 7_000 * (n as i32 % 2));
            }
            let mut a = agents(&order(r, &need, &items, &weighted));
            a.sort();
            let mut b = agents(&full);
            b.sort();
            assert_eq!(a, b, "weights reorder and never add or remove");

            // A method that declares factors is tunable only if its weights can reorder: each
            // factor made dominant in turn must give more than one order across the fixture.
            if !r.factors().is_empty() {
                let mut orders: Vec<Vec<String>> = r
                    .factors()
                    .iter()
                    .map(|dominant| {
                        let w = r.factors().iter().fold(Weights::new(), |w, f| {
                            w.with(*f, if f == dominant { 10_000 } else { 1 })
                        });
                        agents(&order(r, &need, &items, &w))
                    })
                    .collect();
                orders.sort();
                orders.dedup();
                reorders |= orders.len() > 1;
            }
            // A weight on a factor the method does not read changes nothing.
            for f in Factor::ALL.iter().filter(|f| !r.factors().contains(f)) {
                let foreign = none.clone().with(*f, 9_999);
                assert_eq!(
                    agents(&order(r, &need, &items, &foreign)),
                    agents(&full),
                    "a weight on {f:?}, which {:?} does not read, changed its order",
                    r.method()
                );
            }
            assert!(order(r, &need, &[], &none).is_empty(), "empty → empty");
        }
        assert!(
            r.factors().is_empty() || reorders,
            "{:?} declares factors {:?} but no weight reorders it: a tuning would reset incumbency \
             and change nothing (declare no factors instead)",
            r.method(),
            r.factors()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::contract::assert_ranking_contract;
    use super::*;
    use crate::eligibility::eligible_for_contract;
    use crate::kind::KindId;
    use crate::need::{
        Demand, Expresser, ExpresserKind, NeedClass, NeedKey, NeedTerms, Requirements,
    };
    use crate::offer::{Candidate, OfferTerms, Posture};
    use elohim_epr::Reach;
    use elohim_epr_rea::AgentRef;
    use seam_contracts::Answer;

    #[test]
    fn every_builtin_ranking_runs_the_contract() {
        assert_ranking_contract(&XorDistance);
        assert_ranking_contract(&DiversityAware);
        assert_ranking_contract(&NearestBytes);
        assert_ranking_contract(&WeightedCapacity);
    }

    /// Codex review 1, finding 8: many disjoint fixed-size snapshots leave the scratch holding one
    /// snapshot's worth of keys, and a carrier that stays keeps its digest.
    #[test]
    fn the_scratch_holds_only_the_current_snapshot() {
        let mut scratch = Scratch::default();
        for round in 0..200 {
            let names: Vec<String> = (0..8).map(|i| format!("uhCAk-{round}-{i}")).collect();
            scratch.prepare(names.iter().map(String::as_str).chain(["uhCAk-stays"]));
            assert_eq!(scratch.agent_keys.len(), 9, "round {round}");
        }
        assert_eq!(
            scratch.agent_key("uhCAk-stays"),
            Some(&sha256(b"uhCAk-stays"))
        );
        assert!(
            scratch.agent_key("uhCAk-0-0").is_none(),
            "an old carrier is dropped"
        );
    }

    #[test]
    fn builtin_names_match_the_grammar() {
        for name in [
            "elohim:placement:xor-distance@1",
            "elohim:placement:diversity-aware@1",
            "elohim:placement:nearest-bytes@1",
            "elohim:placement:weighted-capacity@1",
            "elohim:placement:legacy-custody-scope@1",
        ] {
            assert!(is_method_name(name), "{name}");
        }
        for bad in ["elohim@1", "Elohim:x@1", "a:b@0", "a:b@", "a::b@1", "a:b"] {
            assert!(!is_method_name(bad), "{bad}");
        }
        assert_eq!(
            XorDistance.method(),
            MethodId::named("elohim:placement:xor-distance@1")
        );
        assert_eq!(MethodId::named("x").cid().codec(), 0x55);
    }

    #[test]
    fn method_id_is_the_raw_sha256_cid_of_its_name() {
        let digest = Sha256::digest(b"elohim:placement:xor-distance@1");
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        let expected = elohim_epr::cid::raw_cid_from_sha256_hex(&hex).unwrap();
        assert_eq!(*XorDistance.method().cid(), expected);
    }

    // --- Compatibility, by golden test (§D.5) ------------------------------------------------
    //
    // The reference strategies below are `elohim-storage/src/reconcile/placement.rs`'s
    // `XorDistanceStrategy` and `DiversityAwarePlacementStrategy` bodies, copied verbatim as the
    // oracle (placement-core cannot depend on elohim-storage). The vectors are that file's test
    // vectors. When the storage shim lands over these rankings, its own tests keep the vectors on
    // the storage side.

    struct RefCandidate {
        agent_cid: String,
        household_id: Option<String>,
    }

    fn ref_xor(blob_marker: &str, candidates: &[RefCandidate], target_n: usize) -> Vec<String> {
        let blob_key = sha256(blob_marker.as_bytes());
        let mut scored: Vec<([u8; 32], &str)> = candidates
            .iter()
            .map(|c| {
                (
                    xor(&blob_key, &sha256(c.agent_cid.as_bytes())),
                    c.agent_cid.as_str(),
                )
            })
            .collect();
        scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
        scored.dedup_by(|a, b| a.1 == b.1);
        scored
            .into_iter()
            .take(target_n)
            .map(|(_, c)| c.to_string())
            .collect()
    }

    fn ref_diversity(
        blob_marker: &str,
        candidates: &[RefCandidate],
        target_n: usize,
    ) -> Vec<String> {
        let blob_key = sha256(blob_marker.as_bytes());
        let domain = |c: &RefCandidate| match &c.household_id {
            Some(h) => format!("hh:{h}"),
            None => format!("cid:{}", c.agent_cid),
        };
        let mut scored: Vec<([u8; 32], &str, String)> = candidates
            .iter()
            .map(|c| {
                (
                    xor(&blob_key, &sha256(c.agent_cid.as_bytes())),
                    c.agent_cid.as_str(),
                    domain(c),
                )
            })
            .collect();
        scored.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.cmp(b.1))
                .then_with(|| a.2.cmp(&b.2))
        });
        scored.dedup_by(|a, b| a.1 == b.1);
        let mut picked: Vec<String> = Vec::new();
        let mut taken = vec![false; scored.len()];
        while picked.len() < target_n {
            let mut used = std::collections::HashSet::new();
            let mut progressed = false;
            for i in 0..scored.len() {
                if picked.len() >= target_n {
                    break;
                }
                if taken[i] {
                    continue;
                }
                if used.insert(scored[i].2.as_str()) {
                    picked.push(scored[i].1.to_string());
                    taken[i] = true;
                    progressed = true;
                }
            }
            if !progressed {
                break;
            }
        }
        picked
    }

    /// The adapter's dedup (D12): unique by carrier, keeping the smallest failure domain.
    fn snapshot_candidates(items: &[RefCandidate]) -> Vec<Candidate> {
        let mut by_agent: BTreeMap<String, Candidate> = BTreeMap::new();
        for it in items {
            let carrier = AgentRef(it.agent_cid.clone());
            let domain = FailureDomain::of(it.household_id.as_deref(), &carrier);
            let mut offers = BTreeMap::new();
            offers.insert(KindId::CustodyBlob, OfferTerms::default());
            let c = Candidate {
                domain,
                posture: Posture {
                    carrier,
                    offers,
                    reach: Reach::Public,
                    written_at: 0,
                },
                live: Answer::Present(true),
                admitted: BTreeMap::new(),
                consent: BTreeMap::new(),
            };
            match by_agent.get(&it.agent_cid) {
                Some(existing) if existing.domain <= c.domain => {}
                _ => {
                    by_agent.insert(it.agent_cid.clone(), c);
                }
            }
        }
        by_agent.into_values().collect()
    }

    fn blob_need(marker: &str) -> Need {
        Need {
            kind: KindId::CustodyBlob,
            key: NeedKey::Blob(marker.to_string()),
            demand: Demand {
                holders: 1,
                domains: 1,
            },
            requires: Requirements::default(),
            terms: NeedTerms::None,
            reach: Reach::Public,
            receiver: AgentRef("uhCAk-r".into()),
            expressed_by: Expresser {
                by: ExpresserKind::Agent,
                id: "uhCAk-r".into(),
            },
            class: NeedClass::Member,
            in_scope_of: elohim_epr::cid::compute_cid(b"legacy"),
        }
    }

    fn ours(r: &dyn Ranking, marker: &str, items: &[RefCandidate], n: usize) -> Vec<String> {
        let candidates = snapshot_candidates(items);
        let eligible: Vec<Eligible<'_>> = candidates
            .iter()
            .map(|c| eligible_for_contract(c, &c.posture.offers[&KindId::CustodyBlob]))
            .collect();
        let need = blob_need(marker);
        let input = RankInput {
            need: &need,
            receiver_domain: None,
            input_root: None,
        };
        let mut scratch = Scratch::default();
        scratch.prepare(candidates.iter().map(|c| c.agent().0.as_str()));
        let mut out = Vec::new();
        r.rank(
            &input,
            &eligible,
            &vec![false; eligible.len()],
            &Weights::new(),
            &scratch,
            &mut out,
        );
        out.truncate(n);
        out.iter()
            .map(|&i| candidates[i].agent().0.clone())
            .collect()
    }

    fn cids(items: &[&str]) -> Vec<RefCandidate> {
        items
            .iter()
            .map(|c| RefCandidate {
                agent_cid: c.to_string(),
                household_id: None,
            })
            .collect()
    }

    fn with_hh(items: &[(&str, &str)]) -> Vec<RefCandidate> {
        items
            .iter()
            .map(|(c, h)| RefCandidate {
                agent_cid: c.to_string(),
                household_id: Some(h.to_string()),
            })
            .collect()
    }

    #[test]
    fn matches_storage_strategy_vectors() {
        let five = cids(&["uhCAk-a", "uhCAk-b", "uhCAk-c", "uhCAk-d", "uhCAk-e"]);
        let reversed = cids(&["uhCAk-e", "uhCAk-d", "uhCAk-c", "uhCAk-b", "uhCAk-a"]);
        let dup = cids(&["uhCAk-a", "uhCAk-a", "uhCAk-b"]);
        let xor_vectors: Vec<(&str, &Vec<RefCandidate>, usize)> = vec![
            ("uhblob-X", &five, 0),
            ("uhblob-X", &five, 2),
            ("uhblob-X", &five, 3),
            ("uhblob-X", &five, 10),
            ("uhblob-X", &reversed, 3),
            ("uhblob-X", &dup, 10),
            ("uhblob-Y", &five, 3),
        ];
        for (m, c, n) in &xor_vectors {
            assert_eq!(
                ours(&XorDistance, m, c, *n),
                ref_xor(m, c, *n),
                "xor {m} n={n}"
            );
        }
        for i in 0..50 {
            let m = format!("uhblob-{i}");
            assert_eq!(ours(&XorDistance, &m, &five, 1), ref_xor(&m, &five, 1));
        }

        let hh_vectors: Vec<Vec<RefCandidate>> = vec![
            with_hh(&[
                ("uhCAk-a", "hh-1"),
                ("uhCAk-b", "hh-2"),
                ("uhCAk-c", "hh-3"),
                ("uhCAk-d", "hh-4"),
                ("uhCAk-e", "hh-5"),
            ]),
            with_hh(&[
                ("uhCAk-a", "hh-1"),
                ("uhCAk-b", "hh-1"),
                ("uhCAk-c", "hh-2"),
            ]),
            with_hh(&[
                ("uhCAk-a", "hh-1"),
                ("uhCAk-b", "hh-2"),
                ("uhCAk-c", "hh-1"),
                ("uhCAk-d", "hh-2"),
            ]),
            with_hh(&[
                ("uhCAk-a", "hh-1"),
                ("uhCAk-a", "hh-1"),
                ("uhCAk-b", "hh-2"),
            ]),
            with_hh(&[
                ("uhCAk-a", "hh-1"),
                ("uhCAk-b", "hh-2"),
                ("uhCAk-c", "hh-1"),
                ("uhCAk-d", "hh-2"),
                ("uhCAk-e", "hh-3"),
            ]),
            with_hh(&[
                ("uhCAk-a", "hh-1"),
                ("uhCAk-b", "hh-1"),
                ("uhCAk-c", "hh-2"),
                ("uhCAk-d", "hh-2"),
            ]),
            with_hh(&[
                ("uhCAk-a", "hh-1"),
                ("uhCAk-b", "hh-1"),
                ("uhCAk-c", "hh-1"),
                ("uhCAk-d", "hh-1"),
            ]),
            // The conflicting-duplicate regression (placement.rs:522-543), both orders.
            with_hh(&[
                ("uhCAk-b", "hh-A"),
                ("uhCAk-b", "hh-B"),
                ("uhCAk-a", "hh-B"),
                ("uhCAk-c", "hh-A"),
            ]),
            with_hh(&[
                ("uhCAk-c", "hh-A"),
                ("uhCAk-a", "hh-B"),
                ("uhCAk-b", "hh-B"),
                ("uhCAk-b", "hh-A"),
            ]),
            five.iter()
                .map(|c| RefCandidate {
                    agent_cid: c.agent_cid.clone(),
                    household_id: None,
                })
                .collect(),
        ];
        for c in &hh_vectors {
            for n in [0, 1, 2, 3, 4, 10] {
                for m in ["uhblob-X", "uhblob-Y"] {
                    assert_eq!(
                        ours(&DiversityAware, m, c, n),
                        ref_diversity(m, c, n),
                        "diversity {m} n={n}"
                    );
                }
            }
        }
    }

    #[test]
    fn multi_pass_diversity_vector() {
        // D15: A{a1,a2,a3}, B{b1,b2}, rank order a1,a2,a3,b1,b2, n = 4 → {a1, b1, a2, b2}.
        let a = FailureDomain::of(Some("A"), &AgentRef("x".into()));
        let b = FailureDomain::of(Some("B"), &AgentRef("x".into()));
        let domains = [&a, &a, &a, &b, &b];
        let mut out = Vec::new();
        diversify(&[0, 1, 2, 3, 4], &domains, 4, &mut out);
        assert_eq!(out, vec![0, 3, 1, 4]);
        diversify(&[0, 1, 2], &[&a, &a, &a], 2, &mut out);
        assert_eq!(out, vec![0, 1], "no domain data → rank order");
    }

    #[test]
    fn nearest_bytes_prefers_input_then_base_holders_whatever_the_xor_order() {
        let input = elohim_epr::cid::compute_cid(b"in");
        let base = elohim_epr::cid::compute_cid(b"base");
        let mut need = blob_need("task-x");
        need.terms = NeedTerms::Task {
            input_root: CidString::from(&input),
            scope: crate::kind::GrantScope::MeasureStage,
            base_root: Some(CidString::from(&base)),
        };
        for seed in 0..20 {
            let mk = |name: String, holds: Vec<CidString>| {
                let carrier = AgentRef(name);
                let mut offers = BTreeMap::new();
                offers.insert(
                    KindId::Task,
                    OfferTerms {
                        holds,
                        ..OfferTerms::default()
                    },
                );
                Candidate {
                    domain: FailureDomain::of(None, &carrier),
                    posture: Posture {
                        carrier,
                        offers,
                        reach: Reach::Public,
                        written_at: 0,
                    },
                    live: Answer::Present(true),
                    admitted: BTreeMap::new(),
                    consent: BTreeMap::new(),
                }
            };
            let cs = [
                mk(format!("neither-{seed}"), vec![]),
                mk(format!("base-{seed}"), vec![CidString::from(&base)]),
                mk(format!("input-{seed}"), vec![CidString::from(&input)]),
            ];
            let eligible: Vec<Eligible<'_>> = cs
                .iter()
                .map(|c| eligible_for_contract(c, &c.posture.offers[&KindId::Task]))
                .collect();
            let input_root = need.input_root();
            let ri = RankInput {
                need: &need,
                receiver_domain: None,
                input_root: input_root.as_ref(),
            };
            let mut out = Vec::new();
            NearestBytes.rank(
                &ri,
                &eligible,
                &[false; 3],
                &Weights::new(),
                &Scratch::default(),
                &mut out,
            );
            assert_eq!(out, vec![2, 1, 0], "seed {seed}");
        }
    }
}
