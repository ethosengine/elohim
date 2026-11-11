//! Family lookup over custody claims. This is discovery only: neither the family tag nor a
//! foreign key is trusted for fetching. A consumer must verify the method's family and derive
//! the selected variant key using its own input before resolving holders (K3 part 2).

use std::collections::{BTreeMap, BTreeSet};

use eprfs_core::BlobCid;

use crate::{fold_holders, SiblingRecord};

/// Distinct Requires claims. No supersession rule exists: absence is not an empty contract.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum Requires {
    Unanimous(BlobCid),
    Unclaimed,
    Conflict(BTreeSet<Option<BlobCid>>),
}

impl Requires {
    pub fn from_claims(claims: impl IntoIterator<Item = Option<BlobCid>>) -> Self {
        let claims: BTreeSet<_> = claims.into_iter().collect();
        if claims.len() > 1 {
            Self::Conflict(claims)
        } else {
            match claims.into_iter().next().flatten() {
                Some(cid) => Self::Unanimous(cid),
                None => Self::Unclaimed,
            }
        }
    }
}

/// One historical claim, independent of the latest custody row.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct RequiresClaim {
    pub device: String,
    pub requires: Option<BlobCid>,
}

/// One key's complete custody fold and historical claims in a family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub key: BlobCid,
    pub outputs: Vec<BlobCid>,
    pub requires: Requires,
    /// All distinct (device, Requires CID or none) claims per output, including older events.
    pub claims: BTreeMap<BlobCid, BTreeSet<RequiresClaim>>,
    /// Latest rows per (device, key, output), including self and pre-K3 untagged records.
    pub holders: Vec<SiblingRecord>,
}

/// Discover keys from any family claim, then use their COMPLETE custody folds.
/// Requires documents are stored locally and are not yet served to siblings. Remote retrieval
/// is deferred to K3 part 2 with admit and K6 slice 1's explicit, default-deny disclosure
/// permission pattern. A custody claim grants no permission to disclose its document.
pub fn family_variants(records: &[SiblingRecord], family: &BlobCid) -> Vec<Variant> {
    let keys: BTreeSet<_> = records
        .iter()
        .filter(|row| row.family.as_ref() == Some(family))
        .map(|row| row.key.clone())
        .collect();
    let folded = fold_holders(records);
    keys.into_iter()
        .map(|key| {
            let holders: Vec<_> = folded
                .iter()
                .filter(|row| row.key == key)
                .map(|row| (*row).clone())
                .collect();
            let outputs: BTreeSet<_> = holders.iter().map(|row| row.output.clone()).collect();
            let mut claims: BTreeMap<BlobCid, BTreeSet<RequiresClaim>> = BTreeMap::new();
            for row in records.iter().filter(|row| row.key == key) {
                claims
                    .entry(row.output.clone())
                    .or_default()
                    .insert(RequiresClaim {
                        device: row.device.clone(),
                        requires: row.requires.clone(),
                    });
            }
            let requires =
                Requires::from_claims(claims.values().flatten().map(|c| c.requires.clone()));
            Variant {
                key,
                outputs: outputs.into_iter().collect(),
                requires,
                claims,
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
        assert_eq!(
            v2.requires,
            Requires::Unanimous(records[1].requires.clone().unwrap())
        );
        assert!(family_variants(&records, &BlobCid::compute(b"absent")).is_empty());
    }

    #[test]
    fn same_device_claims_never_supersede_even_without_family_tag() {
        let first = row(b"v2", "a", b"y");
        let family = first.family.clone().unwrap();
        let mut changed = first.clone();
        changed.family = None;
        changed.requires = Some(BlobCid::compute_bytes(b"second"));
        let expected = Requires::Conflict(BTreeSet::from([
            first.requires.clone(),
            changed.requires.clone(),
        ]));
        let records = [first.clone(), changed.clone()];
        let variants = family_variants(&records, &family);
        assert_eq!(variants[0].requires, expected);
        assert_eq!(variants[0].holders, vec![changed.clone()]);
        assert_eq!(variants[0].claims[&first.output].len(), 2);
        assert_eq!(
            family_variants(&[changed, first], &family)[0].requires,
            expected
        );
    }

    #[test]
    fn family_includes_complete_custody_with_untagged_pre_k3_holder() {
        let first = row(b"v2", "a", b"y");
        let family = first.family.clone().unwrap();
        let mut older = row(b"v2", "b", b"y");
        older.family = None;
        older.requires = None;
        let records = [older, first.clone()];
        let variants = family_variants(&records, &family);
        assert_eq!(
            variants[0].holders,
            fold_holders(&records)
                .into_iter()
                .cloned()
                .collect::<Vec<_>>()
        );
        assert_eq!(
            variants[0].requires,
            Requires::Conflict(BTreeSet::from([None, first.requires]))
        );
        let claims = &variants[0].claims[&first.output];
        assert_eq!(
            Requires::from_claims(
                claims
                    .iter()
                    .filter(|c| c.device == "b")
                    .map(|c| c.requires.clone())
            ),
            Requires::Unclaimed
        );
    }
}
