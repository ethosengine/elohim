//! EPR Head IPLD codec — DAG-CBOR encoding for three-pillar content metadata.
//!
//! An EPR Head is the ~500B gossip-friendly metadata envelope. This module
//! provides encode/decode between Rust structs and DAG-CBOR bytes, plus
//! CID computation with the correct IPLD codec (0x71 = dag-cbor).
//!
//! ## Wire Format Detection
//!
//! `decode_epr_head` auto-detects JSON (first byte 0x7B = `{`) vs CBOR,
//! enabling backward compatibility during migration from JSON to DAG-CBOR.
//!
//! ## Multicodec Constants
//!
//! EPR defines private-use multicodec values for future IPLD integration:
//! - `0x300001` — EPR Head
//! - `0x300002` — EPR Document
//! - `0x300003` — EPR Relationship
//!
//! These are in the private-use range (0x300000–0x3FFFFF) per multicodec spec.

use cid::Cid;
use multihash_codetable::{Code, MultihashDigest};
use serde::{Deserialize, Serialize};

// ── Multicodec Constants ────────────────────────────────────────────────────

/// DAG-CBOR multicodec (standard IPLD codec)
pub const DAG_CBOR_CODEC: u64 = 0x71;

/// EPR Head multicodec (private-use range)
pub const EPR_HEAD_CODEC: u64 = 0x300001;

/// EPR Document multicodec (private-use range)
pub const EPR_DOCUMENT_CODEC: u64 = 0x300002;

/// EPR Relationship multicodec (private-use range)
pub const EPR_RELATIONSHIP_CODEC: u64 = 0x300003;

// ── Three-Pillar Context Structs ────────────────────────────────────────────

/// Lamad pillar — knowledge context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EprLamadContext {
    pub title: String,
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_format: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Shefa pillar — value context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EprShefaContext {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stewards: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allocations: Vec<f64>,
}

/// Qahal pillar — governance context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EprQahalContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reach: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    /// Attestation requirements for body access (Layer 2 gate).
    /// Format: "type:reference" e.g. "prerequisite-mastery:calculus-101"
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attestation_requirements: Vec<String>,
}

/// Typed relationship to another EPR Head.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EprRelationship {
    /// Relationship type: PREREQUISITE, TEACHES, CONTAINS, REFERENCES, etc.
    #[serde(rename = "type")]
    pub rel_type: String,
    /// Target content ID
    pub target: String,
    /// Target content address (CID)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_cid: Option<String>,
}

// ── EPR Head ────────────────────────────────────────────────────────────────

/// The EPR Head — the ~500B gossip-friendly metadata envelope.
///
/// Mirrors the TypeScript `EprHead` interface in `epr-head.model.ts`.
/// When serialized as DAG-CBOR, this becomes a self-describing IPLD document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EprHead {
    /// Schema version for forward compatibility
    pub version: u32,
    /// Stable content ID (slug)
    pub id: String,
    /// CID of the current content bytes (IPLD link to Tier 3)
    pub content: String,
    /// Knowledge pillar context
    pub lamad: EprLamadContext,
    /// Value pillar context
    pub shefa: EprShefaContext,
    /// Governance pillar context
    pub qahal: EprQahalContext,
    /// Typed relationships to other content
    #[serde(default)]
    pub relationships: Vec<EprRelationship>,
    /// Author DID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// ISO 8601 timestamp of last update
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
}

// ── Encode / Decode ─────────────────────────────────────────────────────────

/// Encode an EPR Head as DAG-CBOR and compute its CIDv1 (codec 0x71).
///
/// Returns the raw CBOR bytes and the CID.
pub fn encode_epr_head(head: &EprHead) -> Result<(Vec<u8>, Cid), String> {
    let bytes =
        serde_ipld_dagcbor::to_vec(head).map_err(|e| format!("DAG-CBOR encode failed: {e}"))?;
    let hash = Code::Sha2_256.digest(&bytes);
    let cid = Cid::new_v1(DAG_CBOR_CODEC, hash);
    Ok((bytes, cid))
}

/// Decode an EPR Head from bytes, auto-detecting JSON (0x7B) vs DAG-CBOR.
///
/// This enables backward compatibility: existing JSON-encoded heads can be
/// decoded alongside new DAG-CBOR heads.
pub fn decode_epr_head(bytes: &[u8]) -> Result<EprHead, String> {
    if bytes.is_empty() {
        return Err("Empty input".to_string());
    }

    if bytes[0] == 0x7B {
        // JSON (starts with '{')
        serde_json::from_slice(bytes).map_err(|e| format!("JSON decode failed: {e}"))
    } else {
        // DAG-CBOR
        serde_ipld_dagcbor::from_slice(bytes).map_err(|e| format!("DAG-CBOR decode failed: {e}"))
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_head() -> EprHead {
        EprHead {
            version: 1,
            id: "test-concept".to_string(),
            content: "bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku".to_string(),
            lamad: EprLamadContext {
                title: "Test Concept".to_string(),
                content_type: "concept".to_string(),
                description: Some("A test concept for unit testing".to_string()),
                content_format: Some("markdown".to_string()),
                tags: vec!["test".to_string(), "ipld".to_string()],
            },
            shefa: EprShefaContext {
                stewards: vec!["did:web:alice.example.com".to_string()],
                allocations: vec![100.0],
            },
            qahal: EprQahalContext {
                reach: Some("commons".to_string()),
                layer: Some("communal".to_string()),
                attestation_requirements: vec![],
            },
            relationships: vec![EprRelationship {
                rel_type: "TEACHES".to_string(),
                target: "related-concept".to_string(),
                target_cid: None,
            }],
            author: Some("did:web:alice.example.com".to_string()),
            updated: Some("2026-02-27T12:00:00Z".to_string()),
        }
    }

    #[test]
    fn test_roundtrip_dag_cbor() {
        let head = sample_head();
        let (bytes, cid) = encode_epr_head(&head).unwrap();

        // CID should use DAG-CBOR codec
        assert_eq!(cid.codec(), DAG_CBOR_CODEC);

        // Should not start with '{' (not JSON)
        assert_ne!(bytes[0], 0x7B);

        // Decode back
        let decoded = decode_epr_head(&bytes).unwrap();
        assert_eq!(decoded, head);
    }

    #[test]
    fn test_json_auto_detection() {
        let head = sample_head();
        let json_bytes = serde_json::to_vec(&head).unwrap();

        // First byte should be '{'
        assert_eq!(json_bytes[0], 0x7B);

        // decode_epr_head should auto-detect JSON
        let decoded = decode_epr_head(&json_bytes).unwrap();
        assert_eq!(decoded, head);
    }

    #[test]
    fn test_cid_codec_is_dag_cbor() {
        let head = sample_head();
        let (_bytes, cid) = encode_epr_head(&head).unwrap();

        // Verify codec is 0x71 (dag-cbor), not 0x55 (raw)
        assert_eq!(cid.codec(), 0x71);
        assert_ne!(cid.codec(), 0x55);

        // CID string should start with bafyr (dag-cbor CIDs differ from raw)
        let cid_str = cid.to_string();
        assert!(
            cid_str.starts_with("bafyr"),
            "DAG-CBOR CID should start with bafyr, got: {}",
            &cid_str[..8]
        );
    }

    #[test]
    fn test_empty_input_error() {
        assert!(decode_epr_head(&[]).is_err());
    }

    #[test]
    fn test_invalid_cbor_error() {
        // Random bytes that aren't valid CBOR
        assert!(decode_epr_head(&[0xA0, 0x01, 0x02]).is_err());
    }

    #[test]
    fn test_invalid_json_error() {
        assert!(decode_epr_head(b"{invalid json}").is_err());
    }

    #[test]
    fn test_multicodec_constants() {
        // Verify constants are in the private-use range (0x300000–0x3FFFFF)
        assert!(EPR_HEAD_CODEC >= 0x300000 && EPR_HEAD_CODEC <= 0x3FFFFF);
        assert!(EPR_DOCUMENT_CODEC >= 0x300000 && EPR_DOCUMENT_CODEC <= 0x3FFFFF);
        assert!(EPR_RELATIONSHIP_CODEC >= 0x300000 && EPR_RELATIONSHIP_CODEC <= 0x3FFFFF);

        // Verify they're distinct
        assert_ne!(EPR_HEAD_CODEC, EPR_DOCUMENT_CODEC);
        assert_ne!(EPR_HEAD_CODEC, EPR_RELATIONSHIP_CODEC);
        assert_ne!(EPR_DOCUMENT_CODEC, EPR_RELATIONSHIP_CODEC);
    }

    #[test]
    fn test_camel_case_serialization() {
        // Use a head with targetCid set so it's not skipped
        let mut head = sample_head();
        head.relationships[0].target_cid = Some("bafkrei...".to_string());
        let json = serde_json::to_string(&head).unwrap();

        // Check camelCase field names in JSON
        assert!(json.contains("\"contentType\""), "JSON: {}", json);
        assert!(json.contains("\"contentFormat\""), "JSON: {}", json);
        assert!(json.contains("\"targetCid\""), "JSON: {}", json);
        // Should NOT contain snake_case
        assert!(!json.contains("\"content_type\""), "JSON: {}", json);
        assert!(!json.contains("\"content_format\""), "JSON: {}", json);
        assert!(!json.contains("\"target_cid\""), "JSON: {}", json);
    }

    #[test]
    fn test_minimal_head() {
        // Test with minimal required fields
        let head = EprHead {
            version: 1,
            id: "minimal".to_string(),
            content: "bafkrei...".to_string(),
            lamad: EprLamadContext {
                title: "Minimal".to_string(),
                content_type: "concept".to_string(),
                description: None,
                content_format: None,
                tags: vec![],
            },
            shefa: EprShefaContext {
                stewards: vec![],
                allocations: vec![],
            },
            qahal: EprQahalContext {
                reach: None,
                layer: None,
                attestation_requirements: vec![],
            },
            relationships: vec![],
            author: None,
            updated: None,
        };

        let (bytes, cid) = encode_epr_head(&head).unwrap();
        assert_eq!(cid.codec(), DAG_CBOR_CODEC);

        let decoded = decode_epr_head(&bytes).unwrap();
        assert_eq!(decoded, head);
    }
}
