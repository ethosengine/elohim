//! Cross-DNA Content entry decoder, mirroring elohim DNA's
//! `attestation_validator::decode_content_entry`. Pure deserialization helper —
//! callers handle the DHT `get`/`must_get_entry` themselves.

use hdk::prelude::*;
use serde::{Deserialize, Serialize};

/// Subset of the elohim DNA's `Content` entry that imagodei gates need to read.
///
/// Only the discriminator (`content_type`) + identity (`id`, `author_id`) +
/// extensible payload (`metadata_json`) matter to the cross-DNA gate path. The
/// full elohim `Content` shape has many additional fields (title, summary,
/// blob_cid, etc.) which we intentionally ignore here — `#[serde(default)]` on
/// the optional `author_id` keeps deserialization tolerant of older Content
/// entries that wrote `None` for the author.
///
/// Mirrors the relevant subset of
/// `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:457`.
///
/// **Wire format note**: `SerializedBytes` from a DHT `Entry::App` is
/// MessagePack (rmp_serde), NOT JSON. We use `holochain_serialized_bytes::decode`
/// to deserialize — `serde_json::from_slice` would fail on real entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDnaContent {
    pub id: String,
    pub content_type: String,
    /// Optional in the elohim `Content` schema. Many gate paths require it to
    /// be present; that check is the caller's responsibility.
    #[serde(default)]
    pub author_id: Option<String>,
    /// JSON-stringified metadata (parsed by caller into the gate-specific shape).
    #[serde(default)]
    pub metadata_json: String,
}

/// Decode a Content entry from a cross-DNA `get()` result.
///
/// Mirrors elohim DNA's `attestation_validator::decode_content_entry` at
/// `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attestation_validator.rs:623`.
///
/// The elohim version deserializes into the full `Content` struct via the
/// `#[hdk_entry_helper]`-generated `TryFrom<SerializedBytes>` impl; we don't
/// import that crate (it would create a tight cross-DNA coupling), so we use
/// `holochain_serialized_bytes::decode` directly with a structural subset.
pub fn decode_content_entry(entry: Entry) -> Result<CrossDnaContent, String> {
    match entry {
        Entry::App(app_bytes) => {
            let sb: SerializedBytes = app_bytes.into_sb();
            holochain_serialized_bytes::decode::<_, CrossDnaContent>(sb.bytes())
                .map_err(|e| format!("decode_content_entry: {e}"))
        }
        _ => Err("decode_content_entry: entry is not an App entry".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hdk::prelude::AgentPubKey;

    #[test]
    fn decode_rejects_non_app_entry() {
        // Use Entry::Agent rather than Entry::CounterSign to construct a
        // non-App entry — CounterSign requires a Box<CounterSigningSessionData>
        // and an AppEntryBytes, neither of which has a trivial Default ctor
        // in the pinned HDI 0.7.0. Entry::Agent(AgentPubKey) is the
        // simplest non-App variant.
        let entry = Entry::Agent(AgentPubKey::from_raw_36(vec![0u8; 36]));
        let result = decode_content_entry(entry);
        assert!(
            result.is_err(),
            "expected non-App entry to be rejected, got {:?}",
            result
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("not an App entry"),
            "expected 'not an App entry' error, got {err:?}"
        );
    }

    #[test]
    fn decode_succeeds_on_well_formed_app_entry() {
        // Round-trip: encode a CrossDnaContent into MessagePack, wrap it in
        // Entry::App, decode it back. Proves the production decode path is
        // wired against the right format (rmp_serde, not JSON).
        let original = CrossDnaContent {
            id: "test-content".to_string(),
            content_type: "key-revocation".to_string(),
            author_id: Some("uhCAk...".to_string()),
            metadata_json: r#"{"revoked_key":"abc"}"#.to_string(),
        };
        let bytes: Vec<u8> = holochain_serialized_bytes::encode(&original).expect("encode");
        let sb: SerializedBytes =
            SerializedBytes::from(holochain_serialized_bytes::UnsafeBytes::from(bytes));
        let entry = Entry::App(AppEntryBytes::try_from(sb).expect("app entry bytes"));
        let decoded = decode_content_entry(entry).expect("decode should succeed");
        assert_eq!(decoded.id, original.id);
        assert_eq!(decoded.content_type, original.content_type);
        assert_eq!(decoded.author_id, original.author_id);
        assert_eq!(decoded.metadata_json, original.metadata_json);
    }
}
