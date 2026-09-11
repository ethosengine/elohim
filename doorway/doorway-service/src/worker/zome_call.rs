//! Zome Call Builder - Build MessagePack payloads for Holochain zome calls
//!
//! Provides utilities to construct proper Holochain zome call requests
//! for the conductor's app WebSocket interface.
//!
//! ## DNA Contract
//!
//! For conductor fallback to work, the DNA must implement a `__doorway_get`
//! function that accepts `{ doc_type: String, id: String }` and returns
//! the document data or null.
//!
//! ```rust,ignore
//! // In DNA zome:
//! #[hdk_extern]
//! pub fn __doorway_get(input: DoorwayGetInput) -> ExternResult<Option<DoorwayGetOutput>> {
//!     // Dispatch based on doc_type to actual getters
//!     match input.doc_type.as_str() {
//!         "Content" => get_content_by_id(&input.id),
//!         _ => Ok(None),  // Unknown type
//!     }
//! }
//! ```

use rmpv::Value;
use serde::{Deserialize, Serialize};

use crate::conductor::agent_key;
use crate::types::{DoorwayError, Result};

/// Requester identity for access control (passed to DNA)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequesterIdentity {
    /// Requester's agent public key (if authenticated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Requester's geographic location (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// Whether requester is authenticated
    #[serde(default)]
    pub authenticated: bool,
}

/// Input for the DNA's `__doorway_get` function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorwayGetInput {
    /// Document type (e.g., "Content", "Human")
    pub doc_type: String,
    /// Document ID
    pub id: String,
    /// Requester identity for access control (DNA decides based on this)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requester: Option<RequesterIdentity>,
}

/// Input for the DNA's `__doorway_write` function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorwayWriteInput {
    /// Document type (e.g., "Content", "Human")
    pub doc_type: String,
    /// Operation type ("create", "update", "delete")
    pub op_type: String,
    /// Document ID
    pub id: String,
    /// Document data (for create/update)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Entry hash (for update/delete operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_hash: Option<String>,
}

/// Input for the DNA's `__doorway_batch` function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorwayBatchInput {
    /// List of write operations
    pub operations: Vec<DoorwayWriteInput>,
}

/// Configuration for zome calls
#[derive(Debug, Clone)]
pub struct ZomeCallConfig {
    /// DNA hash (base64 encoded)
    pub dna_hash: String,
    /// Agent public key (base64 encoded)
    pub agent_pub_key: String,
    /// Zome name (default: "content_store")
    pub zome_name: String,
    /// Installed app ID
    pub app_id: String,
    /// Role name from hApp manifest (e.g., "lamad", "imagodei", "infrastructure")
    /// Used to identify which DNA this config belongs to in multi-DNA apps
    pub role_name: String,
}

impl Default for ZomeCallConfig {
    fn default() -> Self {
        Self {
            dna_hash: String::new(),
            agent_pub_key: String::new(),
            zome_name: "content_store".to_string(),
            app_id: "elohim".to_string(),
            role_name: "lamad".to_string(),
        }
    }
}

/// Build a zome call payload for conductor
pub struct ZomeCallBuilder {
    config: ZomeCallConfig,
}

impl ZomeCallBuilder {
    /// Create a new builder with the given config
    pub fn new(config: ZomeCallConfig) -> Self {
        Self { config }
    }

    /// Build a `__doorway_get` call payload
    ///
    /// This calls the DNA's generic getter function that doorway uses
    /// for conductor fallback resolution.
    ///
    /// Identity is passed through for DNA to make access control decisions.
    /// Doorway doesn't enforce access - DNA does.
    pub fn build_doorway_get(
        &self,
        doc_type: &str,
        id: &str,
        requester: Option<RequesterIdentity>,
    ) -> Result<Vec<u8>> {
        let input = DoorwayGetInput {
            doc_type: doc_type.to_string(),
            id: id.to_string(),
            requester,
        };

        self.build_zome_call("__doorway_get", &input)
    }

    /// Build a `__doorway_write` call payload
    ///
    /// This calls the DNA's generic write function for single operations.
    pub fn build_doorway_write(
        &self,
        doc_type: &str,
        op_type: &str,
        id: &str,
        data: Option<serde_json::Value>,
        entry_hash: Option<String>,
    ) -> Result<Vec<u8>> {
        let input = DoorwayWriteInput {
            doc_type: doc_type.to_string(),
            op_type: op_type.to_string(),
            id: id.to_string(),
            data,
            entry_hash,
        };

        self.build_zome_call("__doorway_write", &input)
    }

    /// Build a `__doorway_batch` call payload
    ///
    /// This calls the DNA's batch write function for multiple operations.
    /// More efficient than individual calls for bulk operations.
    pub fn build_doorway_batch(&self, operations: Vec<DoorwayWriteInput>) -> Result<Vec<u8>> {
        let input = DoorwayBatchInput { operations };
        self.build_zome_call("__doorway_batch", &input)
    }

    /// Build a generic zome call payload
    pub fn build_zome_call<T: Serialize>(&self, fn_name: &str, payload: &T) -> Result<Vec<u8>> {
        // Serialize the payload to MessagePack
        let payload_bytes = rmp_serde::to_vec(payload)
            .map_err(|e| DoorwayError::Internal(format!("Failed to serialize payload: {e}")))?;

        // Build the zome call request
        // Format: { type: "call_zome", data: { cell_id, zome_name, fn_name, payload, provenance, cap_secret } }
        let call_data = Value::Map(vec![
            (
                Value::String("cell_id".into()),
                Value::Array(vec![
                    Value::Binary(decode_cell_id_half(&self.config.dna_hash, "dna_hash")?),
                    Value::Binary(decode_cell_id_half(
                        &self.config.agent_pub_key,
                        "agent_pub_key",
                    )?),
                ]),
            ),
            (
                Value::String("zome_name".into()),
                Value::String(self.config.zome_name.clone().into()),
            ),
            (
                Value::String("fn_name".into()),
                Value::String(fn_name.into()),
            ),
            (
                Value::String("payload".into()),
                Value::Binary(payload_bytes),
            ),
            (
                Value::String("provenance".into()),
                Value::Binary(decode_cell_id_half(
                    &self.config.agent_pub_key,
                    "provenance",
                )?),
            ),
            (
                Value::String("cap_secret".into()),
                Value::Nil, // No capability secret for public functions
            ),
        ]);

        let request = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("call_zome".into()),
            ),
            (Value::String("data".into()), call_data),
        ]);

        let mut buf = Vec::new();
        rmpv::encode::write_value(&mut buf, &request)
            .map_err(|e| DoorwayError::Internal(format!("Failed to encode request: {e}")))?;

        Ok(buf)
    }

    /// Parse a zome call response
    pub fn parse_response<T: for<'de> Deserialize<'de>>(
        &self,
        response: &[u8],
    ) -> Result<Option<T>> {
        // Decode the response envelope
        let mut cursor = std::io::Cursor::new(response);
        let value = rmpv::decode::read_value(&mut cursor)
            .map_err(|e| DoorwayError::Holochain(format!("Failed to decode response: {e}")))?;

        // Extract the response data
        if let Value::Map(ref map) = value {
            // Check for error response
            if let Some(Value::String(ref type_str)) = get_field(map, "type") {
                if type_str.as_str() == Some("error") {
                    if let Some(Value::Map(ref error_map)) = get_field(map, "data") {
                        if let Some(Value::String(ref msg)) = get_field(error_map, "message") {
                            return Err(DoorwayError::Holochain(
                                msg.as_str().unwrap_or("Unknown error").to_string(),
                            ));
                        }
                    }
                    return Err(DoorwayError::Holochain("Unknown zome error".into()));
                }
            }

            // Extract successful response data
            if let Some(Value::Binary(ref data)) = get_field(map, "data") {
                // The data is MessagePack-encoded ExternIO
                // Holochain wraps the actual return value
                let result: Option<T> = rmp_serde::from_slice(data)
                    .map_err(|e| DoorwayError::Holochain(format!("Failed to parse result: {e}")))?;
                return Ok(result);
            }
        }

        // Nil/null response means not found
        Ok(None)
    }
}

/// Decode a cell-id half (a `DnaHash` or an `AgentPubKey`) to its raw 39 bytes,
/// through the ONE decoder, whatever vintage of string the role map holds.
///
/// WHY (encode/decode mismatch, found alongside the 2026-09-11 agent-key
/// canonicalization): this used to decode with `base64::STANDARD` what
/// `services::discovery::encode_base64` writes with `URL_SAFE_NO_PAD` —
/// `discovery.rs:245-246`, straight off the conductor's `app_info.cell_ids`, into
/// the `ZomeCallConfig` this builder is handed. The two alphabets agree only on a
/// string containing no `-`/`_`, which is about one key in five at this length,
/// so ~80% of discovered cells could not be called at all: `Invalid base64:
/// Invalid byte 45`. Padding was never the issue — 39 bytes is a multiple of 3
/// and encodes to 52 characters with none.
///
/// The cure is to stop CHOOSING an alphabet here. `agent_key::decode_key_bytes`
/// normalizes first and reads the canonical body, so every spelling a doorway has
/// ever written — bare url-safe (discovery, the provisioner), bare STANDARD (the
/// startup walk), canonical `uhCAk…`/`uhC0k…` — yields identical bytes, and a
/// string that is not a HoloHash in any encoding is REFUSED rather than decoded
/// into a short, plausible-looking cell id the conductor would reject obscurely.
fn decode_cell_id_half(s: &str, field: &str) -> Result<Vec<u8>> {
    agent_key::decode_key_bytes(s).ok_or_else(|| {
        DoorwayError::Internal(format!(
            "{field} is not a HoloHash in any encoding (got {len} chars)",
            len = s.chars().count()
        ))
    })
}

/// Get a field from a MessagePack map
fn get_field<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    for (k, v) in map {
        if let Value::String(k_str) = k {
            if k_str.as_str() == Some(key) {
                return Some(v);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doorway_get_input_serialization() {
        let input = DoorwayGetInput {
            doc_type: "Content".to_string(),
            id: "manifesto".to_string(),
            requester: None,
        };

        let bytes = rmp_serde::to_vec(&input).unwrap();
        let decoded: DoorwayGetInput = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(decoded.doc_type, "Content");
        assert_eq!(decoded.id, "manifesto");
    }

    #[test]
    fn test_doorway_get_input_with_identity() {
        let input = DoorwayGetInput {
            doc_type: "Content".to_string(),
            id: "private-doc".to_string(),
            requester: Some(RequesterIdentity {
                agent_id: Some("uhCAk...".to_string()),
                location: None,
                authenticated: true,
            }),
        };

        // Test JSON serialization (used at API boundary)
        let json = serde_json::to_string(&input).unwrap();
        let decoded: DoorwayGetInput = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.doc_type, "Content");
        assert!(decoded.requester.is_some());
        assert!(decoded.requester.unwrap().authenticated);
    }

    #[test]
    fn test_requester_identity_default() {
        let identity = RequesterIdentity::default();
        assert!(!identity.authenticated);
        assert!(identity.agent_id.is_none());
    }

    #[test]
    fn test_config_default() {
        let config = ZomeCallConfig::default();
        assert_eq!(config.zome_name, "content_store");
        assert_eq!(config.app_id, "elohim");
        assert_eq!(config.role_name, "lamad");
    }

    // ---- cell-id round-trip: the worker path carries BYTES, not a spelling ----
    //
    // The value's journey. `DiscoveryService::discover_cells` reads the raw
    // `(dna_hash, agent_pub_key)` off the conductor's `app_info.cell_ids` and
    // encodes both with `URL_SAFE_NO_PAD` (`services/discovery.rs:245-246, :292`).
    // They are carried as strings — `CellInfo` → `ZomeCallConfig`, into the
    // `zome_configs` DashMap on `AppState` and back out through
    // `zome_helpers::get_zome_config_by_role`. There is no JWT and no Mongo row on
    // this path; it is an in-process role map. They are CONSUMED here, in
    // `build_zome_call`, which must hand the conductor the original bytes back.
    //
    // Before this fix the consumer decoded with `STANDARD`, which cannot read the
    // producer's alphabet — so these round-trips did not close.

    use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
    use base64::Engine;

    /// A REAL 39-byte agent key, minted the way a conductor mints one.
    fn raw_key(fill: u8) -> Vec<u8> {
        holo_hash::AgentPubKey::from_raw_32(vec![fill; 32])
            .get_raw_39()
            .to_vec()
    }

    /// A REAL 39-byte DNA hash — the other half of a cell id, same length and
    /// same multibase tag, different 3-byte prefix.
    fn raw_dna(fill: u8) -> Vec<u8> {
        holo_hash::DnaHash::from_raw_32(vec![fill; 32])
            .get_raw_39()
            .to_vec()
    }

    /// Pull `(dna_bytes, agent_bytes, provenance_bytes)` back out of a built call.
    fn cell_id_of(payload: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let mut cursor = std::io::Cursor::new(payload);
        let value = rmpv::decode::read_value(&mut cursor).expect("built call is valid msgpack");
        let Value::Map(ref outer) = value else {
            panic!("call envelope must be a map")
        };
        let Some(Value::Map(ref data)) = get_field(outer, "data") else {
            panic!("call envelope must carry a data map")
        };
        let Some(Value::Array(ref cell_id)) = get_field(data, "cell_id") else {
            panic!("data must carry a cell_id pair")
        };
        let Some(Value::Binary(ref provenance)) = get_field(data, "provenance") else {
            panic!("data must carry a binary provenance")
        };
        let (Value::Binary(dna), Value::Binary(agent)) = (&cell_id[0], &cell_id[1]) else {
            panic!("cell_id halves must be binary")
        };
        (dna.clone(), agent.clone(), provenance.clone())
    }

    fn build_with(dna_hash: String, agent_pub_key: String) -> Result<Vec<u8>> {
        ZomeCallBuilder::new(ZomeCallConfig {
            dna_hash,
            agent_pub_key,
            ..ZomeCallConfig::default()
        })
        .build_doorway_get("Content", "manifesto", None)
    }

    #[test]
    fn the_legacy_url_safe_spelling_discovery_writes_round_trips_to_the_original_bytes() {
        // THE LIVE DEFECT. This is exactly what `services/discovery.rs` puts in
        // the role map, and 0xFB forces `-`/`_` into the url-safe alphabet — the
        // bytes the old STANDARD decoder choked on.
        let (dna, agent) = (raw_dna(0xFB), raw_key(0xFB));
        let payload = build_with(URL_SAFE_NO_PAD.encode(&dna), URL_SAFE_NO_PAD.encode(&agent))
            .expect("the form discovery actually writes must build");
        let (got_dna, got_agent, _) = cell_id_of(&payload);
        assert_eq!(got_dna, dna);
        assert_eq!(got_agent, agent);
    }

    #[test]
    fn the_pre_fix_standard_decoder_refused_the_form_discovery_actually_writes() {
        // Teeth for the test above: the alphabets really do disagree here, so the
        // round-trip is proving a fix rather than restating a tautology.
        let bare = URL_SAFE_NO_PAD.encode(raw_key(0xFB));
        assert!(
            bare.contains('-') || bare.contains('_'),
            "fixture must exercise the url-safe alphabet: {bare}"
        );
        assert!(
            STANDARD.decode(&bare).is_err(),
            "the decoder this fix replaced could not read {bare}"
        );
    }

    #[test]
    fn a_canonical_cell_id_round_trips_to_the_original_bytes() {
        let (dna, agent) = (raw_dna(0xFB), raw_key(0xFB));
        let payload = build_with(
            crate::conductor::canonical_agent_key(&dna),
            crate::conductor::canonical_agent_key(&agent),
        )
        .expect("the canonical form must build");
        let (got_dna, got_agent, _) = cell_id_of(&payload);
        assert_eq!(got_dna, dna);
        assert_eq!(got_agent, agent);
    }

    #[test]
    fn the_standard_alphabet_spelling_also_round_trips() {
        // The startup agent-discovery walk registers this encoding too.
        let (dna, agent) = (raw_dna(0xFB), raw_key(0xFB));
        let payload = build_with(STANDARD.encode(&dna), STANDARD.encode(&agent))
            .expect("the STANDARD form must build");
        let (got_dna, got_agent, _) = cell_id_of(&payload);
        assert_eq!(got_dna, dna);
        assert_eq!(got_agent, agent);
    }

    #[test]
    fn every_spelling_produces_a_byte_identical_call() {
        // One identity, three vintages of string, one wire payload — the property
        // that makes the role map's spelling stop mattering.
        let (dna, agent) = (raw_dna(3), raw_key(3));
        let canonical = build_with(
            crate::conductor::canonical_agent_key(&dna),
            crate::conductor::canonical_agent_key(&agent),
        )
        .unwrap();
        for (d, a) in [
            (URL_SAFE_NO_PAD.encode(&dna), URL_SAFE_NO_PAD.encode(&agent)),
            (STANDARD.encode(&dna), STANDARD.encode(&agent)),
        ] {
            assert_eq!(
                build_with(d, a).unwrap(),
                canonical,
                "a legacy spelling must build the same call as the canonical one"
            );
        }
    }

    #[test]
    fn provenance_carries_the_same_bytes_as_the_cell_id_agent() {
        // The conductor checks provenance against the cell's agent; two decoders
        // could disagree, one decoder cannot.
        let agent = raw_key(9);
        let payload = build_with(
            URL_SAFE_NO_PAD.encode(raw_dna(9)),
            URL_SAFE_NO_PAD.encode(&agent),
        )
        .unwrap();
        let (_, got_agent, provenance) = cell_id_of(&payload);
        assert_eq!(provenance, agent);
        assert_eq!(provenance, got_agent);
    }

    #[test]
    fn a_cell_id_half_that_is_not_a_holo_hash_is_refused_not_truncated() {
        // An honest error beats a short, plausible-looking cell id the conductor
        // would reject obscurely.
        let agent = URL_SAFE_NO_PAD.encode(raw_key(1));
        assert!(build_with("uhC0k-dev-mode".to_string(), agent.clone()).is_err());
        assert!(build_with(URL_SAFE_NO_PAD.encode(raw_dna(1)), String::new()).is_err());
    }
}
