//! Portable rule meaning, independently of the force a receiving space grants it.
//!
//! Version 2 uses canonical JSON as RAW content (0x55), never mislabeled DAG-CBOR.
//! Legacy registry pins are not rewritten. Intrinsic predicate/measure defaults remain
//! semantic content; scope, enforcement class and adopter provenance are standing.
//! This module verifies content only. It neither authorizes adoption nor executes validators.

use eprfs_core::BlobCid;
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};

use crate::{canonical_json, CanonError, MAX_REGISTRY_BYTES};

pub const PORTABLE_POLICY_FORMAT: &str = "epr-policy-content@2";

const STANDING_KEYS: &[&str] = &[
    "binding",
    "class",
    "scope",
    "established_by",
    "established_at",
    "status",
    "superseded_by",
    "retire-when",
    "created_at",
    "updated_at",
    "citations",
];

/// Content shared by spaces. Unknown semantic fields are retained, never discarded;
/// this is an artifact container, not a claim that a host implements those fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortablePolicy {
    pub format: String,
    pub semantics: Mapping,
}

/// Source-space standing offered alongside the content. Copying this grants no authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyStanding {
    pub fields: Mapping,
}

#[derive(Debug, thiserror::Error)]
pub enum PortablePolicyError {
    #[error("portable policy canonicalization: {0}")]
    Canonical(#[from] CanonError),
    #[error("portable policy encoding: {0}")]
    Encoding(#[from] serde_yaml::Error),
    #[error("portable policy decoding: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("portable policy invalid: {0}")]
    Invalid(String),
}

impl PortablePolicy {
    pub fn validate(&self) -> Result<(), PortablePolicyError> {
        if self.format != PORTABLE_POLICY_FORMAT {
            return Err(PortablePolicyError::Invalid("unsupported format".into()));
        }
        if self
            .semantics
            .get("id")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
            || self
                .semantics
                .get("version")
                .and_then(Value::as_u64)
                .is_none_or(|v| v == 0)
        {
            return Err(PortablePolicyError::Invalid(
                "nonempty id and positive version required".into(),
            ));
        }
        for key in STANDING_KEYS.iter().copied().chain(["contentHash"]) {
            if self.semantics.contains_key(key) {
                return Err(PortablePolicyError::Invalid(format!(
                    "standing field {key} in content"
                )));
            }
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, PortablePolicyError> {
        self.validate()?;
        let bytes = canonical_json(&serde_yaml::to_value(self)?)?.into_bytes();
        if bytes.len() > MAX_REGISTRY_BYTES {
            return Err(PortablePolicyError::Invalid("size limit exceeded".into()));
        }
        Ok(bytes)
    }

    pub fn cid(&self) -> Result<BlobCid, PortablePolicyError> {
        Ok(BlobCid::compute_raw(&self.canonical_bytes()?))
    }
}

pub fn split_registry_row(
    row: &Mapping,
) -> Result<(PortablePolicy, PolicyStanding), PortablePolicyError> {
    let mut semantics = row.clone();
    semantics.remove("contentHash");
    let mut fields = Mapping::new();
    for key in STANDING_KEYS {
        if let Some(value) = semantics.remove(*key) {
            fields.insert(Value::String((*key).into()), value);
        }
    }
    let policy = PortablePolicy {
        format: PORTABLE_POLICY_FORMAT.into(),
        semantics,
    };
    policy.validate()?;
    Ok((policy, PolicyStanding { fields }))
}

/// Verify the exact adopted address, codec, format and canonical bytes before interpretation.
pub fn verify_policy(
    expected: &BlobCid,
    bytes: &[u8],
) -> Result<PortablePolicy, PortablePolicyError> {
    if bytes.len() > MAX_REGISTRY_BYTES {
        return Err(PortablePolicyError::Invalid("size limit exceeded".into()));
    }
    if *expected != BlobCid::compute_raw(bytes) {
        return Err(PortablePolicyError::Invalid(
            "raw CID does not match bytes".into(),
        ));
    }
    let policy: PortablePolicy = serde_json::from_slice(bytes)?;
    if policy.canonical_bytes()? != bytes {
        return Err(PortablePolicyError::Invalid("noncanonical content".into()));
    }
    Ok(policy)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> Mapping {
        serde_yaml::from_str("id: sample\nversion: 1\nvalidator: example\nmeasure: {limit: 7}\nbinding: binding-local\nclass: deny\nscope: {write: '*.rs'}\nestablished_by: space-a\ncontentHash: legacy\n").expect("fixture")
    }

    #[test]
    fn standing_changes_do_not_change_content() {
        let original = row();
        let mut other = original.clone();
        for key in ["binding", "class", "scope", "established_by"] {
            other.insert(Value::String(key.into()), Value::String("different".into()));
        }
        let (a, standing_a) = split_registry_row(&original).expect("split");
        let (b, standing_b) = split_registry_row(&other).expect("split");
        assert_eq!(a.cid().expect("cid"), b.cid().expect("cid"));
        assert_ne!(standing_a, standing_b);
        other.insert(
            Value::String("measure".into()),
            Value::String("new meaning".into()),
        );
        assert_ne!(
            a.cid().expect("cid"),
            split_registry_row(&other)
                .expect("split")
                .0
                .cid()
                .expect("cid")
        );
    }

    #[test]
    fn verifies_raw_codec_and_rejects_tamper_and_legacy_codec() {
        let policy = split_registry_row(&row()).expect("split").0;
        let bytes = policy.canonical_bytes().expect("bytes");
        let cid = policy.cid().expect("cid");
        assert!(cid.to_string().starts_with("bafkrei"));
        assert_eq!(verify_policy(&cid, &bytes).expect("verified"), policy);
        assert!(verify_policy(&BlobCid::compute(&bytes), &bytes).is_err());
        let mut tampered = bytes.clone();
        tampered.push(b' ');
        assert!(verify_policy(&cid, &tampered).is_err());
        assert!(verify_policy(&BlobCid::compute_raw(&tampered), &tampered).is_err());
    }

    #[test]
    fn rejects_unknown_format_fields_and_standing_in_content() {
        for bytes in [
            br#"{"format":"unknown","semantics":{"id":"a","version":1}}"#.as_slice(),
            br#"{"format":"epr-policy-content@2","extra":true,"semantics":{"id":"a","version":1}}"#.as_slice(),
            br#"{"format":"epr-policy-content@2","semantics":{"binding":"deny","id":"a","version":1}}"#.as_slice(),
        ] {
            assert!(verify_policy(&BlobCid::compute_raw(bytes), bytes).is_err());
        }
        let oversized = vec![b' '; MAX_REGISTRY_BYTES + 1];
        assert!(verify_policy(&BlobCid::compute_raw(&oversized), &oversized).is_err());
    }
}
