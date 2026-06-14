//! `CoverageRollup` — the aggregate-with-descent operator.
//!
//! The keystone of the recursive architecture (graduated spec
//! `2026-06-14-recursive-architecture-design` §2.1): a content-addressed,
//! Category-C (recompute-on-read, never persisted as truth, **zero DNA spend**)
//! aggregate that rolls child coverages UP a layer (household → collective →
//! region → planetary) **while preserving the path back down**. A counting
//! roll-up ascends but destroys descent; this carries the descent pointer
//! (`constituents`) and the externality (`deficit = required \ covered`) *inside*
//! the aggregate, so an AI can walk from any layer's rollup down to the one
//! trapped atom — the operator's "aggregation must preserve descent" charge.
//!
//! Two invariants the design pins:
//! - **The metric is the deficit (externality emitted), never the holding** — so
//!   abundance is invisible and a person's total account is never built.
//! - **Per-row degrade, never fail-closed** — one unreadable child is `warn!`'d
//!   and surfaced as deficit, never collapsing the whole aggregate
//!   (the `project_epr_router_empties_on_poisoned_scope` lesson).
//!
//! `rollup_hash = BLAKE3(scope, domain, covered, sorted constituents)` makes two
//! peers who hold the same children compute the *same* hash — consilience as
//! content-addressed agreement (the basis of `witness_quorum`).

use serde::{Deserialize, Serialize};

/// A coverage set over a `u64` key/byte space: normalized (sorted,
/// non-overlapping, non-empty) half-open `[start, end)` intervals. The `∪`
/// (union) and `\` (difference) operations are the load-bearing recursion math —
/// coverage is a SET, never a scalar score (so it cannot be collapsed into a
/// rank over a person).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CoverageSet {
    intervals: Vec<(u64, u64)>,
}

impl CoverageSet {
    pub fn empty() -> Self {
        Self { intervals: Vec::new() }
    }

    /// A single `[start, end)` interval (empty if `start >= end`).
    pub fn interval(start: u64, end: u64) -> Self {
        if start >= end {
            Self::empty()
        } else {
            Self { intervals: vec![(start, end)] }
        }
    }

    /// `[0, span)` — the whole keyspace/corpus of a layer.
    pub fn full(span: u64) -> Self {
        Self::interval(0, span)
    }

    /// Normalize an arbitrary interval list: drop empties, sort, merge overlaps.
    pub fn from_intervals(mut v: Vec<(u64, u64)>) -> Self {
        v.retain(|(s, e)| s < e);
        v.sort_unstable();
        let mut out: Vec<(u64, u64)> = Vec::with_capacity(v.len());
        for (s, e) in v {
            if let Some(last) = out.last_mut() {
                if s <= last.1 {
                    if e > last.1 {
                        last.1 = e;
                    }
                    continue;
                }
            }
            out.push((s, e));
        }
        Self { intervals: out }
    }

    /// `self ∪ other`.
    pub fn union(&self, other: &Self) -> Self {
        let mut v = self.intervals.clone();
        v.extend_from_slice(&other.intervals);
        Self::from_intervals(v)
    }

    /// `self \ other` (the part of `self` not covered by `other`). Both operands
    /// are normalized, so a single sorted sweep suffices.
    pub fn difference(&self, other: &Self) -> Self {
        let mut result: Vec<(u64, u64)> = Vec::new();
        for &(s, e) in &self.intervals {
            let mut cur = s;
            for &(os, oe) in &other.intervals {
                if oe <= cur || os >= e {
                    continue; // no overlap with the remaining [cur, e)
                }
                if os > cur {
                    result.push((cur, os));
                }
                cur = oe;
                if cur >= e {
                    break;
                }
            }
            if cur < e {
                result.push((cur, e));
            }
        }
        Self::from_intervals(result)
    }

    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    /// Total length covered (a convenience magnitude for felt surfaces — the
    /// truth is the set, this is a read of it).
    pub fn measure(&self) -> u64 {
        self.intervals.iter().map(|(s, e)| e - s).sum()
    }

    /// Read-only view of the normalized intervals.
    pub fn intervals(&self) -> &[(u64, u64)] {
        &self.intervals
    }

    /// Deterministic bytes for content-addressing (big-endian, normalized order).
    fn canonical_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(self.intervals.len() * 16);
        for (s, e) in &self.intervals {
            b.extend_from_slice(&s.to_be_bytes());
            b.extend_from_slice(&e.to_be_bytes());
        }
        b
    }
}

/// The commons-only coverage domains a rollup aggregates (never a per-person
/// dimension — there is no `CoverageDomain` over a soul).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoverageDomain {
    /// RS-quilt corpus bytes a layer's stewards hold.
    CorpusBytes,
    /// Conductor DHT authority-arc keyspace.
    ArcKeyspace,
    /// The donut's inner ring — dignity/care floor.
    CareFloor,
    /// The donut's outer ring — anti-monopoly ceiling.
    DonutCeiling,
    /// Served-head freshness coverage.
    HeadFreshness,
}

impl CoverageDomain {
    fn tag(self) -> &'static str {
        match self {
            CoverageDomain::CorpusBytes => "corpus-bytes",
            CoverageDomain::ArcKeyspace => "arc-keyspace",
            CoverageDomain::CareFloor => "care-floor",
            CoverageDomain::DonutCeiling => "donut-ceiling",
            CoverageDomain::HeadFreshness => "head-freshness",
        }
    }
}

/// A child's contribution to a rollup. `covered: None` means the child's
/// coverage could not be read THIS pass — it is kept in `constituents` (descent
/// preserved) and surfaced as deficit, never silently dropped or fail-closed.
#[derive(Debug, Clone)]
pub struct ChildCoverage {
    pub cid: String,
    pub covered: Option<CoverageSet>,
}

impl ChildCoverage {
    pub fn readable(cid: impl Into<String>, covered: CoverageSet) -> Self {
        Self { cid: cid.into(), covered: Some(covered) }
    }
    pub fn unreadable(cid: impl Into<String>) -> Self {
        Self { cid: cid.into(), covered: None }
    }
}

/// A content-addressed aggregate that preserves descent. See the module docs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageRollup {
    /// The layer node this rollup is for (household | collective | region | planetary).
    pub scope_cid: String,
    pub domain: CoverageDomain,
    /// `∪` of child coverages.
    pub covered: CoverageSet,
    /// This layer's share of FULL.
    pub required: CoverageSet,
    /// `required \ covered` — the externality, the descent target.
    pub deficit: CoverageSet,
    /// Pointers DOWN to child rollups / leaf commitments (sorted; descent preserved).
    pub constituents: Vec<String>,
    /// BLAKE3 over (scope, domain, covered, sorted constituents) — the Merkle commitment.
    pub rollup_hash: String,
    /// Peers who independently recomputed the same `rollup_hash` (consilience-as-agreement).
    pub witness_quorum: u32,
    /// Automerge heads computed against (freshness + reproducibility).
    pub as_of_heads: Vec<String>,
}

impl CoverageRollup {
    /// Aggregate child coverages into a rollup, preserving descent. Per-row
    /// degrade: an unreadable child is `warn!`'d and counted as deficit, never
    /// collapsing the aggregate.
    pub fn rollup(
        scope_cid: impl Into<String>,
        domain: CoverageDomain,
        required: CoverageSet,
        children: &[ChildCoverage],
    ) -> Self {
        let scope_cid = scope_cid.into();
        let mut covered = CoverageSet::empty();
        let mut constituents: Vec<String> = Vec::with_capacity(children.len());
        for c in children {
            constituents.push(c.cid.clone());
            match &c.covered {
                Some(cov) => covered = covered.union(cov),
                None => tracing::warn!(
                    child = %c.cid,
                    scope = %scope_cid,
                    "CoverageRollup: unreadable child coverage — degrading per-row (surfaced as deficit, NOT fail-closed)"
                ),
            }
        }
        constituents.sort();
        let deficit = required.difference(&covered);
        let rollup_hash = Self::compute_hash(&scope_cid, domain, &covered, &constituents);
        Self {
            scope_cid,
            domain,
            covered,
            required,
            deficit,
            constituents,
            rollup_hash,
            witness_quorum: 0,
            as_of_heads: Vec::new(),
        }
    }

    fn compute_hash(
        scope_cid: &str,
        domain: CoverageDomain,
        covered: &CoverageSet,
        sorted_constituents: &[String],
    ) -> String {
        let mut h = blake3::Hasher::new();
        h.update(scope_cid.as_bytes());
        h.update(b"\x00");
        h.update(domain.tag().as_bytes());
        h.update(b"\x00");
        h.update(&covered.canonical_bytes());
        h.update(b"\x00");
        for c in sorted_constituents {
            h.update(c.as_bytes());
            h.update(b"\x00");
        }
        h.finalize().to_hex().to_string()
    }

    /// The down-pointer: descend to the child rollups / leaf commitments.
    pub fn descend(&self) -> &[String] {
        &self.constituents
    }

    /// True iff this layer's required coverage is fully met (no externality).
    pub fn is_covered(&self) -> bool {
        self.deficit.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_set_union_merges_overlaps() {
        let a = CoverageSet::interval(0, 40);
        let b = CoverageSet::interval(30, 60);
        assert_eq!(a.union(&b), CoverageSet::interval(0, 60));
        // disjoint stays two intervals
        let c = CoverageSet::interval(60, 100);
        let u = a.union(&c);
        assert_eq!(u.intervals(), &[(0, 40), (60, 100)]);
        assert_eq!(u.measure(), 80);
    }

    #[test]
    fn coverage_set_difference_is_the_gap() {
        let required = CoverageSet::full(100);
        let covered = CoverageSet::interval(0, 40).union(&CoverageSet::interval(60, 100));
        // required \ covered = the hole in the middle
        assert_eq!(required.difference(&covered), CoverageSet::interval(40, 60));
        // full coverage → empty deficit
        assert!(required.difference(&CoverageSet::full(100)).is_empty());
        // nothing covered → deficit is the whole required
        assert_eq!(required.difference(&CoverageSet::empty()), CoverageSet::full(100));
    }

    fn child(cid: &str, s: u64, e: u64) -> ChildCoverage {
        ChildCoverage::readable(cid, CoverageSet::interval(s, e))
    }

    #[test]
    fn rollup_aggregates_coverage_and_exposes_the_deficit() {
        let r = CoverageRollup::rollup(
            "household:dowell",
            CoverageDomain::CorpusBytes,
            CoverageSet::full(100),
            &[child("a", 0, 40), child("b", 60, 100)],
        );
        assert_eq!(r.covered, CoverageSet::interval(0, 40).union(&CoverageSet::interval(60, 100)));
        // the externality is the [40,60) gap — the descent target, not a score
        assert_eq!(r.deficit, CoverageSet::interval(40, 60));
        assert!(!r.is_covered());
        // descent preserved: both children reachable (sorted)
        assert_eq!(r.descend(), &["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn rollup_is_content_addressed_and_order_independent() {
        // The consilience invariant: two peers holding the same children compute
        // the SAME rollup_hash regardless of the order they saw them.
        let ab = CoverageRollup::rollup(
            "collective:church",
            CoverageDomain::ArcKeyspace,
            CoverageSet::full(64),
            &[child("a", 0, 32), child("b", 32, 64)],
        );
        let ba = CoverageRollup::rollup(
            "collective:church",
            CoverageDomain::ArcKeyspace,
            CoverageSet::full(64),
            &[child("b", 32, 64), child("a", 0, 32)],
        );
        assert_eq!(ab.rollup_hash, ba.rollup_hash);
        assert!(ab.is_covered()); // 0..32 ∪ 32..64 = full(64)
        // a different scope or domain → a different content address
        let other_domain = CoverageRollup::rollup(
            "collective:church",
            CoverageDomain::CareFloor,
            CoverageSet::full(64),
            &[child("a", 0, 32), child("b", 32, 64)],
        );
        assert_ne!(ab.rollup_hash, other_domain.rollup_hash);
    }

    #[test]
    fn rollup_degrades_per_row_never_fail_closed() {
        // One unreadable child must NOT empty the aggregate — it stays in
        // constituents (descent preserved) and widens the deficit.
        let r = CoverageRollup::rollup(
            "household:dowell",
            CoverageDomain::CorpusBytes,
            CoverageSet::full(100),
            &[child("a", 0, 50), ChildCoverage::unreadable("b")],
        );
        // 'a' still counted; 'b' surfaced as part of the deficit, not a panic/empty
        assert_eq!(r.covered, CoverageSet::interval(0, 50));
        assert_eq!(r.deficit, CoverageSet::interval(50, 100));
        // BOTH children remain reachable for descent (the bad row is not hidden)
        assert_eq!(r.descend(), &["a".to_string(), "b".to_string()]);
    }
}
