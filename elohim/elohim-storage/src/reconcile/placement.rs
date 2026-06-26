//! Blob placement strategy seam (Phase 3).
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
//! Identity namespace is `agent_cid` throughout — the canonical join key
//! (commitments + `shard_locations` use it). The XOR metric NEVER crosses
//! namespaces (the all-zeros incident); callers resolve libp2p ids → `agent_cid`
//! before building the candidate set.

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
#[derive(Debug, Clone, Default)]
pub struct XorDistanceStrategy;

/// Hash an arbitrary identifier into the 256-bit distance space. Used for both
/// the blob marker and each `agent_cid` so they live in one comparable space
/// (standard Kademlia "hash the key into the ID space").
fn hash_into_keyspace(input: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input);
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Byte-wise XOR of two 256-bit keys. The result compares (big-endian, via the
/// derived array `Ord`) as the Kademlia distance magnitude.
fn xor_distance(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (o, (x, y)) in out.iter_mut().zip(a.iter().zip(b.iter())) {
        *o = x ^ y;
    }
    out
}

impl PlacementStrategy for XorDistanceStrategy {
    fn rank(
        &self,
        blob_marker: &str,
        candidates: &[PlacementCandidate],
        target_n: usize,
    ) -> Vec<String> {
        let blob_key = hash_into_keyspace(blob_marker.as_bytes());
        let mut scored: Vec<([u8; 32], &str)> = candidates
            .iter()
            .map(|c| {
                let agent_key = hash_into_keyspace(c.agent_cid.as_bytes());
                (xor_distance(&blob_key, &agent_key), c.agent_cid.as_str())
            })
            .collect();
        // Total order: nearest distance first, then agent_cid for an unambiguous,
        // input-order-independent tie-break (the "agreement" property every peer
        // relies on for coordination-free self-selection).
        scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
        // No duplicate agent_cids: after the sort, repeats are adjacent (identical
        // distance + identical cid), so a consecutive dedup keeps the first.
        scored.dedup_by(|a, b| a.1 == b.1);
        scored
            .into_iter()
            .take(target_n)
            .map(|(_, cid)| cid.to_string())
            .collect()
    }
}

/// Intentional strategy (P3-8): rank holders to MAXIMIZE distinct failure-domain
/// (household) coverage first, with XOR distance as the within/across-domain
/// tiebreak. This is what keeps *salvage* re-placement from silently re-converging
/// replicas into one household: XOR-closest can co-locate multiple replicas in one
/// household (the spec's "sharpest blind spot"), eroding the diversity ingest
/// established. The ingest selector (`services/peer_selection.rs`) already enforces
/// this household-first discipline; this brings salvage onto the same footing.
///
/// **Failure domain** = `household_id` when known; a candidate whose `household_id`
/// is `None` is treated as its OWN distinct domain — we have no evidence of
/// co-location, so we neither claim nor penalize diversity for it (and unknown
/// candidates must never collapse into one bucket, which would starve selection
/// against the not-yet-`household`-populated salvage pool). A consequence worth
/// stating: a pool with NO household data degrades **exactly** to
/// [`XorDistanceStrategy`] — same output, candidate-for-candidate.
///
/// Still a **pure, deterministic** function of the candidate set (no per-peer
/// mutable state, no network, no capacity/standing weighting), so it preserves the
/// **agreement** property and stays inside the coordination-free self-selection
/// model — it does NOT cross into the spec's "coordinated-authoring" fork.
#[derive(Debug, Clone, Default)]
pub struct DiversityAwarePlacementStrategy;

impl DiversityAwarePlacementStrategy {
    /// The failure-domain key: the known household (prefixed to avoid colliding
    /// with the unknown sentinel), else a per-candidate unique sentinel keyed by
    /// `agent_cid` so unknown-household candidates each form their own domain.
    fn failure_domain(c: &PlacementCandidate) -> String {
        match &c.household_id {
            Some(h) => format!("hh:{h}"),
            None => format!("cid:{}", c.agent_cid),
        }
    }
}

impl PlacementStrategy for DiversityAwarePlacementStrategy {
    fn rank(
        &self,
        blob_marker: &str,
        candidates: &[PlacementCandidate],
        target_n: usize,
    ) -> Vec<String> {
        let blob_key = hash_into_keyspace(blob_marker.as_bytes());
        // Same (distance, agent_cid) RANKING as XorDistanceStrategy, with
        // failure_domain appended as a FINAL tiebreak. That last key matters only
        // when the SAME agent_cid arrives twice with CONFLICTING household
        // enrichment — reachable, because the salvage candidate set is assembled
        // from partial / in-flux multi-source data. Without it, two equal
        // (distance, cid) entries carrying different domains keep their INPUT order
        // under the stable sort, and the dedup below would retain an
        // input-order-dependent domain — silently breaking the agreement property
        // the coordination-free self-selection rests on. With it, dedup keeps the
        // lexicographically-smallest domain, fully independent of input order.
        let mut scored: Vec<([u8; 32], &str, String)> = candidates
            .iter()
            .map(|c| {
                let agent_key = hash_into_keyspace(c.agent_cid.as_bytes());
                (
                    xor_distance(&blob_key, &agent_key),
                    c.agent_cid.as_str(),
                    Self::failure_domain(c),
                )
            })
            .collect();
        scored.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.cmp(b.1))
                .then_with(|| a.2.cmp(&b.2))
        });
        // No duplicate agent_cids: repeats are adjacent after the sort (keep first —
        // now the deterministic lexicographically-smallest domain).
        scored.dedup_by(|a, b| a.1 == b.1);

        // Diversity-first multi-pass greedy over the XOR-sorted list. Each pass
        // takes at most one candidate per not-yet-used failure domain, so the first
        // `target_n` picks come from DISTINCT domains whenever enough domains exist;
        // when more slots than domains remain, later passes fill from the leftovers
        // in XOR order. With every candidate in its own domain this reduces to a
        // single pass in pure XOR order == XorDistanceStrategy.
        let mut picked: Vec<String> = Vec::with_capacity(target_n.min(scored.len()));
        let mut taken = vec![false; scored.len()];
        while picked.len() < target_n {
            let mut used_domains: std::collections::HashSet<&str> =
                std::collections::HashSet::new();
            let mut progressed = false;
            for i in 0..scored.len() {
                if picked.len() >= target_n {
                    break;
                }
                if taken[i] {
                    continue;
                }
                if used_domains.insert(scored[i].2.as_str()) {
                    picked.push(scored[i].1.to_string());
                    taken[i] = true;
                    progressed = true;
                }
            }
            // No untaken candidate found this pass → pool exhausted; stop (also
            // guards against an infinite loop when target_n exceeds the pool).
            if !progressed {
                break;
            }
        }
        picked
    }
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
