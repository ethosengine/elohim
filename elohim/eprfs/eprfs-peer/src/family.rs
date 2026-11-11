//! Family lookup over custody claims. This is discovery only: neither the family tag nor a
//! foreign key is trusted for fetching. A consumer must verify the method's family and derive
//! the selected variant key using its own input before resolving holders (K3 part 2).

use std::collections::{BTreeMap, BTreeSet};

use eprfs_core::BlobCid;

use crate::{fold_holders, SiblingRecord};

/// One key's current claims in a family; outputs and holder rows retain divergence evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub key: BlobCid,
    pub outputs: Vec<BlobCid>,
    /// A unanimous, present claim only. Missing or conflicting claims yield None, never an
    /// empty requirement set. Individual claims remain in `holders` for the consumer to judge.
    pub requires: Option<BlobCid>,
    /// Latest rows per (device, key, output), sorted by device then output. Includes self;
    /// callers mark self using their device identity, without changing this pure fold.
    pub holders: Vec<SiblingRecord>,
}

/// Group the current custody fold by key, ordered by CID, without selection or admission.
/// Fold before filtering: a newer claim that removes/changes its family replaces its old tag.
pub fn family_variants(records: &[SiblingRecord], family: &BlobCid) -> Vec<Variant> {
    let mut keys: BTreeMap<BlobCid, Vec<SiblingRecord>> = BTreeMap::new();
    for row in fold_holders(records) {
        if row.family.as_ref() == Some(family) {
            keys.entry(row.key.clone()).or_default().push(row.clone());
        }
    }
    keys.into_iter()
        .map(|(key, holders)| {
            let outputs: BTreeSet<_> = holders.iter().map(|row| row.output.clone()).collect();
            let claims: BTreeSet<_> = holders.iter().map(|row| row.requires.clone()).collect();
            let requires = if claims.len() == 1 {
                claims.into_iter().next().flatten()
            } else {
                None
            };
            Variant {
                key,
                outputs: outputs.into_iter().collect(),
                requires,
                holders,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(key: &[u8], device: &str, output: &[u8]) -> SiblingRecord {
        SiblingRecord {
            key: BlobCid::compute(key),
            family: Some(BlobCid::compute(b"family")),
            requires: Some(BlobCid::compute_bytes(b"[]")),
            device: device.into(),
            output: BlobCid::compute_bytes(output),
            manifest: BlobCid::compute(output),
        }
    }

    #[test]
    fn deterministic_keys_outputs_and_distinct_holders_keep_divergence() {
        let records = vec![
            row(b"v3", "b", b"x"),
            row(b"v2", "a", b"y"),
            row(b"v2", "b", b"y"),
            row(b"v2", "a", b"z"),
            row(b"v2", "a", b"y"),
        ];
        let family = records[0].family.as_ref().unwrap();
        let variants = family_variants(&records, family);
        let mut reversed = records.clone();
        reversed.reverse();
        assert_eq!(variants, family_variants(&reversed, family));
        assert_eq!(variants.len(), 2);
        assert!(variants.windows(2).all(|pair| pair[0].key < pair[1].key));
        let v2 = variants.iter().find(|v| v.key == records[1].key).unwrap();
        assert_eq!(v2.outputs.len(), 2);
        assert_eq!(v2.holders.len(), 3);
        assert_eq!(v2.requires, records[1].requires);
        assert!(family_variants(&records, &BlobCid::compute(b"absent")).is_empty());
    }

    #[test]
    fn latest_claim_wins_but_missing_or_conflicting_requires_is_not_unconstrained() {
        let first = row(b"v2", "a", b"y");
        let family = first.family.clone().unwrap();
        let mut changed = first.clone();
        changed.requires = None;
        let mut records = vec![first.clone(), changed.clone()];
        assert_eq!(family_variants(&records, &family)[0].requires, None);
        changed.family = None;
        records.push(changed);
        assert!(family_variants(&records, &family).is_empty());
        let mut sibling = row(b"v2", "b", b"y");
        sibling.requires = Some(BlobCid::compute_bytes(b"different claim"));
        let variants = family_variants(&[first, sibling], &family);
        assert_eq!(variants[0].requires, None);
        assert_eq!(variants[0].holders.len(), 2);
    }
}
