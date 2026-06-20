//! Generic select→fold→aggregate combinators over a materialized relation.
//!
//! Pure, DB-free — the composability primitives every facing's folds reuse
//! instead of a bespoke hand loop. The resiliency facing's `intra_hub_peers` is
//! the §11 composability proof: `bucket_by` then per-bucket `distinct_count_by`
//! reproduces the prior hand-rolled "distinct agents per hub" exactly.

use std::collections::{BTreeMap, HashSet};
use std::hash::Hash;

/// Bucket rows by a key projection. Rows whose key is `None` are **dropped**
/// (this is what preserves the resiliency facing's "exclude null-hub" rule).
/// Returns a `BTreeMap` from key to the borrowed rows in that bucket.
///
/// Returns a `BTreeMap` (NOT `HashMap`) **deliberately**: this is the foundational
/// combinator every facing reuses, so its iteration order is **deterministic by
/// construction**. A lens that folds a bucketed result into a serialized `Vec`
/// therefore cannot leak hash-iteration order onto the wire — the determinism the
/// resiliency golden test guards is true for every future lens for free.
pub fn bucket_by<R, K, F>(rows: &[R], key: F) -> BTreeMap<K, Vec<&R>>
where
    K: Ord,
    F: Fn(&R) -> Option<K>,
{
    let mut out: BTreeMap<K, Vec<&R>> = BTreeMap::new();
    for r in rows {
        if let Some(k) = key(r) {
            out.entry(k).or_default().push(r);
        }
    }
    out
}

/// Count DISTINCT key values across a slice of borrowed rows (the shape a
/// `bucket_by` bucket hands you). Order-independent; safe to feed a
/// `&Vec<&R>` directly.
pub fn distinct_count_by<R, K, F>(rows: &[&R], key: F) -> usize
where
    K: Eq + Hash,
    F: Fn(&R) -> K,
{
    let mut seen: HashSet<K> = HashSet::new();
    for r in rows {
        seen.insert(key(r));
    }
    seen.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Row {
        k: Option<String>,
        v: String,
    }

    fn row(k: Option<&str>, v: &str) -> Row {
        Row {
            k: k.map(str::to_string),
            v: v.to_string(),
        }
    }

    #[test]
    fn bucket_by_drops_none_keys_and_groups() {
        let rows = vec![
            row(Some("a"), "x"),
            row(None, "y"), // dropped
            row(Some("a"), "z"),
            row(Some("b"), "w"),
        ];
        let buckets = bucket_by(&rows, |r| r.k.clone());
        assert_eq!(buckets.len(), 2, "None key dropped, two real buckets");
        assert_eq!(buckets.get("a").unwrap().len(), 2);
        assert_eq!(buckets.get("b").unwrap().len(), 1);
        assert!(buckets.values().all(|rs| !rs.is_empty()));
    }

    #[test]
    fn distinct_count_by_collapses_duplicates() {
        let rows = [row(None, "x"), row(None, "x"), row(None, "y")];
        let refs: Vec<&Row> = rows.iter().collect();
        assert_eq!(
            distinct_count_by(&refs, |r| r.v.clone()),
            2,
            "duplicate 'x' collapses to one distinct value"
        );
    }

    #[test]
    fn bucket_then_distinct_count_composes() {
        // The §11 composability proof shape: bucket by k, distinct-count v per
        // bucket. bucket "a" has values {x, x} → 1 distinct; "b" has {w} → 1.
        let rows = vec![
            row(Some("a"), "x"),
            row(Some("a"), "x"),
            row(Some("b"), "w"),
        ];
        let buckets = bucket_by(&rows, |r| r.k.clone());
        assert_eq!(
            distinct_count_by(buckets.get("a").unwrap(), |r| r.v.clone()),
            1
        );
        assert_eq!(
            distinct_count_by(buckets.get("b").unwrap(), |r| r.v.clone()),
            1
        );
    }
}
