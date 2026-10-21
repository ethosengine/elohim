//! Blob placement strategy seam (Phase 3), now a shim over placement-core's ranking.
//!
//! The DURABLE artifact is the [`PlacementStrategy`] trait + [`PlacementCandidate`]
//! — NOT the XOR heuristic. XOR-distance ([`XorDistanceStrategy`]) is the MVP
//! strategy chosen for developer convenience (deterministic + coordination-free),
//! deliberately behind the seam so intentional strategies (household/failure-domain
//! diversity, affinity, capacity/standing, governance) slot in additively later
//! WITHOUT reworking salvage. See
//! `genesis/docs/superpowers/specs/2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md`
//! and backlog `intentional-placement-strategy-beyond-xor.md` (the P3-8 door).
//!
//! # One ranking home (rail R-P8)
//!
//! The rankings themselves live in `elohim-placement-core` (`placement_core::rank`):
//! `XorDistanceStrategy` is [`placement_core::XorDistance`] and
//! `DiversityAwarePlacementStrategy` is [`placement_core::DiversityAware`], moved there
//! byte for byte and pinned by golden tests on both sides (interfaces spec §D.5,
//! `rank::tests::matches_storage_strategy_vectors`; this file's own tests, unchanged). This
//! file keeps the old trait as a thin shim so no caller and no existing placement moves: it
//! builds the snapshot the ranking reads (one public `NeedKey::Blob` need, every candidate
//! admitted through `placement_core::admit`, unique by carrier keeping the smallest failure
//! domain, D12), asks the ranking for its total order and truncates it to `target_n`. No
//! ranking, no XOR helper and no diversity pass is written here any more; the
//! `boundary::one_ranking_home` test (`src/boundary.rs`) fails the build if one comes back.
//!
//! Identity namespace is `agent_cid` throughout — the canonical join key
//! (commitments + `shard_locations` use it). The XOR metric NEVER crosses
//! namespaces (the all-zeros incident); callers resolve libp2p ids → `agent_cid`
//! before building the candidate set.
//!
//! # XOR distance is a PLACEMENT input, never a RETENTION input.
//!
//! Distance decides *where a new copy goes*. It must never enter an eviction or
//! retention ranking. Prior art (Freenet's demand-driven hosting design, surveyed
//! 2026-07-27): distance's causal pull on demand already flows through subscriber
//! count via routing gravity, so ranking on both double-counts it — and locality
//! is delivered by routing, not by the retention decision. They demoted their own
//! distance-derived demand estimator to telemetry-only for exactly this reason.
//!
//! Our arc factor IS a distance input. Keep it on the authority plane; do not let
//! it become the storage plane's retention rule.

use std::collections::BTreeMap;

use elohim_epr::Reach;
use elohim_epr_rea::AgentRef;
use placement_core::{
    admit, AgentRefKey, Candidate, Capacity, Charge, Demand, DiversityAware, Eligible, Expresser,
    ExpresserKind, FailureDomain, GrantScope, IneligibleReason, KindId, MethodId, MishpatAction,
    Need, NeedClass, NeedKey, NeedTerms, OfferTerms, PlacementKind, Posture, RankInput, Ranking,
    Requirements, Scratch, Weights, XorDistance,
};
use seam_contracts::Answer;

/// A candidate holder for blob placement. Carries the context intentional
/// strategies weigh; the MVP [`XorDistanceStrategy`] reads ONLY `agent_cid`.
/// New strategies extend behavior WITHOUT changing this type.
#[derive(Debug, Clone)]
pub struct PlacementCandidate {
    /// Canonical agent identity (`uhCAk…`). The only field the MVP strategy uses.
    pub agent_cid: String,
    /// Failure-domain key for diversity-aware strategies (P3-8). Unused by MVP.
    pub household_id: Option<String>,
    /// Always-on class. Unused by MVP.
    pub archetype: Option<String>,
    /// Spare capacity for capacity-weighted strategies (P3-8). Unused by MVP.
    pub spare_bytes: Option<u64>,
}

impl PlacementCandidate {
    /// Construct a candidate from just an `agent_cid` (the MVP-sufficient form).
    pub fn from_agent_cid(agent_cid: impl Into<String>) -> Self {
        Self {
            agent_cid: agent_cid.into(),
            household_id: None,
            archetype: None,
            spare_bytes: None,
        }
    }
}

/// Deterministically rank candidate holders for a blob, nearest-first.
///
/// Contract (binding on EVERY implementation — the test contract):
/// - **deterministic**: same inputs → same output, on every peer;
/// - **total order**: ties broken so the ordering is unambiguous;
/// - `len() == min(target_n, unique candidate count)`;
/// - **no duplicate** `agent_cid`s in the output;
/// - empty candidates → empty output;
/// - **agreement**: independent of candidate input order (coordination-free
///   self-selection depends on this).
///
/// Implemented only by the two shims below, over placement-core's rankings (R-P8). A new
/// behaviour is a new `placement_core::Ranking` with its own `MethodId`, registered in
/// placement-core's `MethodRegistry`, never a new impl of this trait.
pub trait PlacementStrategy {
    fn rank(
        &self,
        blob_marker: &str,
        candidates: &[PlacementCandidate],
        target_n: usize,
    ) -> Vec<String>;
}

/// MVP strategy: rank by XOR distance between `sha256(blob_marker)` and
/// `sha256(agent_cid)` in a 256-bit space. Uniform spread; blind to household,
/// capacity, affinity, standing (the cost of MVP convenience — see the seam doc).
///
/// A shim over [`placement_core::XorDistance`].
#[derive(Debug, Clone, Default)]
pub struct XorDistanceStrategy;

impl PlacementStrategy for XorDistanceStrategy {
    fn rank(
        &self,
        blob_marker: &str,
        candidates: &[PlacementCandidate],
        target_n: usize,
    ) -> Vec<String> {
        rank_through(&XorDistance, blob_marker, candidates, target_n)
    }
}

/// Intentional strategy (P3-8): rank holders to MAXIMIZE distinct failure-domain
/// (household) coverage first, with XOR distance as the within/across-domain
/// tiebreak. This is what keeps *salvage* re-placement from silently re-converging
/// replicas into one household: XOR-closest can co-locate multiple replicas in one
/// household (the spec's "sharpest blind spot"), eroding the diversity ingest
/// established.
///
/// **Failure domain** = `household_id` when known; a candidate whose `household_id`
/// is `None` is treated as its OWN distinct domain — we have no evidence of
/// co-location, so we neither claim nor penalize diversity for it. A consequence worth
/// stating: a pool with NO household data degrades **exactly** to
/// [`XorDistanceStrategy`] — same output, candidate-for-candidate.
///
/// Still a **pure, deterministic** function of the candidate set, so it preserves the
/// **agreement** property and stays inside the coordination-free self-selection model.
///
/// A shim over [`placement_core::DiversityAware`], whose multi-pass greedy is
/// `placement_core::diversify`.
#[derive(Debug, Clone, Default)]
pub struct DiversityAwarePlacementStrategy;

impl PlacementStrategy for DiversityAwarePlacementStrategy {
    fn rank(
        &self,
        blob_marker: &str,
        candidates: &[PlacementCandidate],
        target_n: usize,
    ) -> Vec<String> {
        rank_through(&DiversityAware, blob_marker, candidates, target_n)
    }
}

/// The legacy custody kind the shim admits through. Custody-shaped (the receiver never carries
/// its own need), no grant scope (a declared posture suffices, R3), and no charge: the legacy
/// strategies never weighed capacity. The full custody kinds with their `Ports` are
/// `reconcile/custody_floor.rs`'s (a follow-on); this one exists only so the shim reaches the
/// ranking through `admit`, the one constructor of `Eligible` (R-P5).
struct LegacyCustodyBlob;

impl PlacementKind for LegacyCustodyBlob {
    fn id(&self) -> KindId {
        KindId::CustodyBlob
    }
    fn action(&self, _need: &Need) -> MishpatAction {
        MishpatAction::CustodyBlob
    }
    fn scope(&self, _need: &Need) -> Option<GrantScope> {
        None
    }
    fn receiver_may_carry(&self) -> bool {
        false
    }
    fn fits(
        &self,
        _need: &Need,
        _candidate: &Candidate,
        _offer: &OfferTerms,
        _remaining: &Capacity,
    ) -> Result<Charge, IneligibleReason> {
        Ok(Charge::default())
    }
    fn default_method(&self) -> MethodId {
        XorDistance.method()
    }
}

/// The legacy scope every shimmed need is expressed in (interfaces spec §D.2, the legacy custody
/// mapping): one named CID, the same on every peer.
const LEGACY_SCOPE: &str = "elohim:placement:legacy-custody-scope@1";

/// The receiver of a shimmed need. The legacy strategies had no receiver, so the shim names one
/// no agent key can equal (a NUL byte never occurs in a `uhCAk…` key or a transport id).
const NO_RECEIVER: &str = "\u{0}legacy-placement-shim";

/// The shimmed need: one public `NeedKey::Blob(blob_marker)`. Its `rank_bytes` are the marker's
/// UTF-8 bytes, which is exactly what `hash_into_keyspace(blob_marker)` hashed.
fn legacy_need(blob_marker: &str, target_n: usize) -> Need {
    let holders = u16::try_from(target_n).unwrap_or(u16::MAX);
    Need {
        kind: KindId::CustodyBlob,
        key: NeedKey::Blob(blob_marker.to_string()),
        demand: Demand {
            holders,
            domains: holders.min(1),
        },
        requires: Requirements::default(),
        terms: NeedTerms::None,
        reach: Reach::Public,
        receiver: AgentRef(NO_RECEIVER.to_string()),
        expressed_by: Expresser {
            by: ExpresserKind::Agent,
            id: NO_RECEIVER.to_string(),
        },
        class: NeedClass::Member,
        in_scope_of: *MethodId::named(LEGACY_SCOPE).cid(),
    }
}

/// The snapshot's candidates: unique by carrier, keeping the smallest failure domain (D12), so a
/// duplicate carrier with conflicting household enrichment resolves the same way whatever the
/// input order. Every candidate is live, admitted at `public` for the shim's receiver, and offers
/// the legacy custody kind with no stated capacity.
fn legacy_candidates(candidates: &[PlacementCandidate], receiver: &AgentRef) -> Vec<Candidate> {
    let receiver = AgentRefKey::of(receiver);
    let mut by_agent: BTreeMap<&str, Candidate> = BTreeMap::new();
    for c in candidates {
        let carrier = AgentRef(c.agent_cid.clone());
        let domain = FailureDomain::of(c.household_id.as_deref(), &carrier);
        if by_agent
            .get(c.agent_cid.as_str())
            .is_some_and(|kept| kept.domain <= domain)
        {
            continue;
        }
        let mut offers = BTreeMap::new();
        offers.insert(KindId::CustodyBlob, OfferTerms::default());
        let mut admitted = BTreeMap::new();
        admitted.insert(receiver.clone(), Answer::Present(Reach::Public));
        by_agent.insert(
            c.agent_cid.as_str(),
            Candidate {
                posture: Posture {
                    carrier,
                    offers,
                    reach: Reach::Public,
                    written_at: 0,
                },
                domain,
                live: Answer::Present(true),
                admitted,
                consent: BTreeMap::new(),
            },
        );
    }
    by_agent.into_values().collect()
}

/// Rank `candidates` for `blob_marker` through a placement-core ranking: every candidate
/// eligible, no incumbents, default weights, the order truncated to `target_n`.
fn rank_through(
    ranking: &dyn Ranking,
    blob_marker: &str,
    candidates: &[PlacementCandidate],
    target_n: usize,
) -> Vec<String> {
    if target_n == 0 || candidates.is_empty() {
        return Vec::new();
    }
    let need = legacy_need(blob_marker, target_n);
    let pool = legacy_candidates(candidates, &need.receiver);
    let kind = LegacyCustodyBlob;
    let remaining = Capacity::default();
    let eligible: Vec<Eligible<'_>> = pool
        .iter()
        .filter_map(|c| match admit(&kind, &need, c, None, &remaining, 0) {
            Ok(e) => Some(e),
            // Unreachable for a well-formed agent key: the shim's need admits every live
            // public candidate. A candidate refused here (one whose key equals the shim's
            // NUL-prefixed receiver) is said, never silently ranked.
            Err(refused) => {
                tracing::warn!(
                    agent = %c.agent().0,
                    reason = ?refused.why.why,
                    "legacy placement shim: candidate not admitted"
                );
                None
            }
        })
        .collect();
    let mut scratch = Scratch::default();
    scratch.prepare(eligible.iter().map(|e| e.agent().0.as_str()));
    let input = RankInput {
        need: &need,
        receiver_domain: None,
        input_root: None,
    };
    let mut order = Vec::with_capacity(eligible.len());
    ranking.rank(
        &input,
        &eligible,
        &vec![false; eligible.len()],
        &Weights::new(),
        &scratch,
        &mut order,
    );
    order.truncate(target_n);
    order
        .into_iter()
        .map(|i| eligible[i].agent().0.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cids(items: &[&str]) -> Vec<PlacementCandidate> {
        items
            .iter()
            .map(|c| PlacementCandidate::from_agent_cid(*c))
            .collect()
    }

    #[test]
    fn empty_candidates_yields_empty() {
        let s = XorDistanceStrategy;
        assert!(s.rank("uhblob-X", &[], 3).is_empty());
    }

    #[test]
    fn caps_at_target_n() {
        let s = XorDistanceStrategy;
        let pool = cids(&["uhCAk-a", "uhCAk-b", "uhCAk-c", "uhCAk-d", "uhCAk-e"]);
        assert_eq!(s.rank("uhblob-X", &pool, 2).len(), 2);
    }

    #[test]
    fn target_n_exceeding_candidates_returns_all_unique() {
        let s = XorDistanceStrategy;
        let pool = cids(&["uhCAk-a", "uhCAk-b", "uhCAk-c"]);
        assert_eq!(s.rank("uhblob-X", &pool, 10).len(), 3);
    }

    #[test]
    fn deterministic_repeated_calls_identical() {
        let s = XorDistanceStrategy;
        let pool = cids(&["uhCAk-a", "uhCAk-b", "uhCAk-c", "uhCAk-d"]);
        assert_eq!(s.rank("uhblob-X", &pool, 3), s.rank("uhblob-X", &pool, 3));
    }

    #[test]
    fn order_independent_agreement() {
        // Two peers may hold the candidate list in different orders; they MUST
        // compute the same closest-N (coordination-free self-selection).
        let s = XorDistanceStrategy;
        let forward = cids(&["uhCAk-a", "uhCAk-b", "uhCAk-c", "uhCAk-d", "uhCAk-e"]);
        let reversed = cids(&["uhCAk-e", "uhCAk-d", "uhCAk-c", "uhCAk-b", "uhCAk-a"]);
        assert_eq!(
            s.rank("uhblob-X", &forward, 3),
            s.rank("uhblob-X", &reversed, 3)
        );
    }

    #[test]
    fn dedups_repeated_agent_cid() {
        let s = XorDistanceStrategy;
        let pool = cids(&["uhCAk-a", "uhCAk-a", "uhCAk-b"]);
        let out = s.rank("uhblob-X", &pool, 10);
        assert_eq!(out.len(), 2, "duplicate agent_cid must appear once");
        let mut sorted = out.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), out.len(), "no duplicates in output");
    }

    #[test]
    fn blob_marker_influences_ordering() {
        // The metric must actually depend on the blob — not pick the same
        // nearest peer for every blob. Across 50 distinct blobs over 4 peers,
        // a uniform hash makes a single universal winner astronomically unlikely.
        let s = XorDistanceStrategy;
        let pool = cids(&["uhCAk-a", "uhCAk-b", "uhCAk-c", "uhCAk-d"]);
        let winners: std::collections::HashSet<String> = (0..50)
            .map(|i| s.rank(&format!("uhblob-{i}"), &pool, 1)[0].clone())
            .collect();
        assert!(winners.len() > 1, "ranking must depend on the blob marker");
    }

    #[test]
    fn mvp_ignores_household_and_capacity_fields() {
        // The door property: the seam CARRIES diversity/capacity context, but the
        // MVP strategy is blind to it. Candidates equal in agent_cid but differing
        // in household_id / spare_bytes rank identically.
        let s = XorDistanceStrategy;
        let bare = cids(&["uhCAk-a", "uhCAk-b", "uhCAk-c"]);
        let enriched: Vec<PlacementCandidate> = ["uhCAk-a", "uhCAk-b", "uhCAk-c"]
            .iter()
            .enumerate()
            .map(|(i, c)| PlacementCandidate {
                agent_cid: (*c).to_string(),
                household_id: Some(format!("household-{i}")),
                archetype: Some("node".to_string()),
                spare_bytes: Some(1_000_000 * (i as u64 + 1)),
            })
            .collect();
        assert_eq!(
            s.rank("uhblob-X", &bare, 3),
            s.rank("uhblob-X", &enriched, 3)
        );
    }

    // ---- DiversityAwarePlacementStrategy (P3-8) -------------------------------

    /// Build candidates with explicit `(agent_cid, household_id)` pairs.
    fn with_hh(items: &[(&str, &str)]) -> Vec<PlacementCandidate> {
        items
            .iter()
            .map(|(cid, hh)| PlacementCandidate {
                agent_cid: (*cid).to_string(),
                household_id: Some((*hh).to_string()),
                archetype: None,
                spare_bytes: None,
            })
            .collect()
    }

    /// Look up the household a chosen `agent_cid` belongs to, for spread asserts.
    fn household_of<'a>(cid: &str, items: &'a [(&str, &str)]) -> &'a str {
        items
            .iter()
            .find(|(c, _)| *c == cid)
            .map(|(_, h)| *h)
            .expect("cid in items")
    }

    #[test]
    fn diversity_empty_yields_empty() {
        let s = DiversityAwarePlacementStrategy;
        assert!(s.rank("uhblob-X", &[], 3).is_empty());
    }

    #[test]
    fn diversity_caps_at_target_n() {
        let s = DiversityAwarePlacementStrategy;
        let pool = with_hh(&[
            ("uhCAk-a", "hh-1"),
            ("uhCAk-b", "hh-2"),
            ("uhCAk-c", "hh-3"),
            ("uhCAk-d", "hh-4"),
            ("uhCAk-e", "hh-5"),
        ]);
        assert_eq!(s.rank("uhblob-X", &pool, 2).len(), 2);
    }

    #[test]
    fn diversity_target_n_exceeding_candidates_returns_all_unique() {
        let s = DiversityAwarePlacementStrategy;
        let pool = with_hh(&[
            ("uhCAk-a", "hh-1"),
            ("uhCAk-b", "hh-1"),
            ("uhCAk-c", "hh-2"),
        ]);
        let out = s.rank("uhblob-X", &pool, 10);
        assert_eq!(out.len(), 3, "returns every unique candidate, no more");
        let mut sorted = out.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), out.len(), "no duplicate agent_cids");
    }

    #[test]
    fn diversity_deterministic_repeated_calls_identical() {
        let s = DiversityAwarePlacementStrategy;
        let pool = with_hh(&[
            ("uhCAk-a", "hh-1"),
            ("uhCAk-b", "hh-2"),
            ("uhCAk-c", "hh-1"),
            ("uhCAk-d", "hh-2"),
        ]);
        assert_eq!(s.rank("uhblob-X", &pool, 3), s.rank("uhblob-X", &pool, 3));
    }

    #[test]
    fn diversity_dedups_repeated_agent_cid() {
        let s = DiversityAwarePlacementStrategy;
        let pool = with_hh(&[
            ("uhCAk-a", "hh-1"),
            ("uhCAk-a", "hh-1"),
            ("uhCAk-b", "hh-2"),
        ]);
        let out = s.rank("uhblob-X", &pool, 10);
        assert_eq!(out.len(), 2, "duplicate agent_cid must appear once");
    }

    #[test]
    fn diversity_order_independent_agreement_with_households() {
        // Two peers may hold the candidate list (and its household enrichment) in
        // different orders; they MUST compute the same set (coordination-free).
        let s = DiversityAwarePlacementStrategy;
        let fwd = with_hh(&[
            ("uhCAk-a", "hh-1"),
            ("uhCAk-b", "hh-2"),
            ("uhCAk-c", "hh-1"),
            ("uhCAk-d", "hh-2"),
            ("uhCAk-e", "hh-3"),
        ]);
        let rev: Vec<PlacementCandidate> = fwd.iter().rev().cloned().collect();
        assert_eq!(s.rank("uhblob-X", &fwd, 3), s.rank("uhblob-X", &rev, 3));
    }

    #[test]
    fn diversity_spans_distinct_households_when_available() {
        // The core P3-8 property: N candidates over N households → the chosen N
        // span N distinct households (no silent co-location), regardless of XOR order.
        let s = DiversityAwarePlacementStrategy;
        let items = [
            ("uhCAk-a", "hh-1"),
            ("uhCAk-b", "hh-2"),
            ("uhCAk-c", "hh-3"),
            ("uhCAk-d", "hh-4"),
        ];
        let pool = with_hh(&items);
        let out = s.rank("uhblob-X", &pool, 4);
        let hhs: std::collections::HashSet<&str> =
            out.iter().map(|c| household_of(c, &items)).collect();
        assert_eq!(
            hhs.len(),
            4,
            "4 peers over 4 households must span 4 households"
        );
    }

    #[test]
    fn diversity_avoids_colocation_when_two_households_available() {
        // 2 households, 2 peers each, target 2 → MUST place one per household,
        // never two from whichever household holds the XOR-nearest pair. This is
        // the exact failure XorDistanceStrategy permits and this strategy fixes.
        let s = DiversityAwarePlacementStrategy;
        let items = [
            ("uhCAk-a", "hh-1"),
            ("uhCAk-b", "hh-1"),
            ("uhCAk-c", "hh-2"),
            ("uhCAk-d", "hh-2"),
        ];
        let pool = with_hh(&items);
        let out = s.rank("uhblob-X", &pool, 2);
        assert_eq!(out.len(), 2);
        let hhs: std::collections::HashSet<&str> =
            out.iter().map(|c| household_of(c, &items)).collect();
        assert_eq!(
            hhs.len(),
            2,
            "two replicas must span two households when available"
        );
    }

    #[test]
    fn diversity_degrades_to_xor_when_single_household() {
        // All replicas in one known household → no diversity to exploit → the
        // strategy reduces EXACTLY to XorDistanceStrategy (candidate-for-candidate).
        let div = DiversityAwarePlacementStrategy;
        let xor = XorDistanceStrategy;
        let pool = with_hh(&[
            ("uhCAk-a", "hh-1"),
            ("uhCAk-b", "hh-1"),
            ("uhCAk-c", "hh-1"),
            ("uhCAk-d", "hh-1"),
        ]);
        assert_eq!(
            div.rank("uhblob-X", &pool, 3),
            xor.rank("uhblob-X", &pool, 3),
            "single-household pool must match pure XOR ranking"
        );
    }

    #[test]
    fn diversity_with_no_household_data_equals_xor() {
        // The live salvage-pool case before 1b: every candidate household_id=None →
        // each is its own failure domain → output is EXACTLY XorDistanceStrategy's.
        // (This is why the strategy is safe to wire before household plumbing lands:
        // it is never WORSE than XOR, and strictly better once households are known.)
        let div = DiversityAwarePlacementStrategy;
        let xor = XorDistanceStrategy;
        let pool = cids(&["uhCAk-a", "uhCAk-b", "uhCAk-c", "uhCAk-d", "uhCAk-e"]);
        assert_eq!(
            div.rank("uhblob-Y", &pool, 3),
            xor.rank("uhblob-Y", &pool, 3),
            "all-None-household pool must match pure XOR ranking"
        );
    }

    #[test]
    fn diversity_agreement_under_conflicting_household_duplicate() {
        // Regression: the SAME agent_cid arriving twice with CONFLICTING households
        // (reachable when the salvage candidate set is assembled from partial /
        // in-flux multi-source data). Before the failure_domain tiebreak, the stable
        // sort kept the duplicate's INPUT-order household through dedup, so the
        // greedy's domain accounting — and thus the chosen set — depended on input
        // order, silently breaking coordination-free agreement. Two peers holding the
        // list in opposite orders MUST still agree.
        let s = DiversityAwarePlacementStrategy;
        let fwd = with_hh(&[
            ("uhCAk-b", "hh-A"),
            ("uhCAk-b", "hh-B"),
            ("uhCAk-a", "hh-B"),
            ("uhCAk-c", "hh-A"),
        ]);
        let rev: Vec<PlacementCandidate> = fwd.iter().rev().cloned().collect();
        assert_eq!(
            s.rank("uhblob-X", &fwd, 2),
            s.rank("uhblob-X", &rev, 2),
            "conflicting-household duplicate cid must not make placement input-order-dependent"
        );
    }

    #[test]
    fn diversity_target_n_zero_yields_empty() {
        let s = DiversityAwarePlacementStrategy;
        let pool = with_hh(&[("uhCAk-a", "hh-1"), ("uhCAk-b", "hh-2")]);
        assert!(s.rank("uhblob-X", &pool, 0).is_empty());
    }

    #[test]
    fn xor_target_n_zero_yields_empty() {
        let s = XorDistanceStrategy;
        let pool = cids(&["uhCAk-a", "uhCAk-b"]);
        assert!(s.rank("uhblob-X", &pool, 0).is_empty());
    }
}
