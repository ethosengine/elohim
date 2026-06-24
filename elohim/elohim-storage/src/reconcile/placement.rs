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
}
