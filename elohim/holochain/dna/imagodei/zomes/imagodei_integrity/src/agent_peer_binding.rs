//! AgentPeerBinding — EPR Phase 2B, Task A.2
//!
//! Declares that a libp2p peer identity (PeerId) is operated by a specific
//! Holochain agent (AgentCid). DHT is the source of truth (Category A —
//! Notarized). The SQLite projection in `peer_identity_bindings` (Task A.5)
//! carries `dht_anchor_hash` to link back.
//!
//! Wire-format contract: `elohim/sdk/schemas/v1/objects/agent-peer-binding.schema.json`
//! Enum contract:        `elohim/sdk/schemas/v1/enums/device-archetype.schema.json`
//!
//! Signing invariant: `canonical_bytes()` serializes all fields **except**
//! `signature` and `superseded_by`. This keeps signatures stable across the
//! binding's lifetime — `superseded_by` is filled in post-creation by the
//! reconciliation controller (Principle P1) and must not participate in the
//! signed body.

use hdi::prelude::*;

// =============================================================================
// DeviceArchetype enum
// =============================================================================

/// Hardware/deployment archetype of the device presenting an AgentPeerBinding.
///
/// DNA-notarized as part of the AgentPeerBinding entry type. Values must exactly
/// match `elohim/sdk/schemas/v1/enums/device-archetype.schema.json`.
///
/// - `node`    — dedicated elohim-node server/blade (household or cluster)
/// - `desktop` — Tauri desktop app (workstation or laptop)
/// - `mobile`  — mobile client (phone or tablet)
/// - `steward` — steward process managing collective infrastructure
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum DeviceArchetype {
    Node,
    Desktop,
    Mobile,
    Steward,
}

/// All valid device archetype values (snake_case, matches schema enum).
///
/// Referenced by `pnpm run schema:check-dna` via `DEVICE_ARCHETYPES` name declared
/// in `device-archetype.schema.json` `_dna.constant`. The generated
/// `CORE_DEVICE_ARCHETYPES` / `ALL_DEVICE_ARCHETYPES` constants in
/// `generated_enums.rs` cover the codegen check; this constant is the in-zome
/// authority for runtime validation.
pub const DEVICE_ARCHETYPES: &[&str] = &["node", "desktop", "mobile", "steward"];

// =============================================================================
// Canonical-body helper struct
// =============================================================================

/// Fields included in the Ed25519 signing body.
///
/// All AgentPeerBinding fields **except** `signature` and `superseded_by`.
/// Used exclusively by `canonical_bytes()` — never stored directly.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
struct AgentPeerBindingCanonical {
    peer_id: String,
    agent_cid: String,
    valid_from: i64, // Timestamp as microseconds-since-epoch (i64, deterministic)
    valid_until: Option<i64>,
    device_archetype: String, // serialized as snake_case via DeviceArchetype::to_str()
}

// =============================================================================
// AgentPeerBinding entry
// =============================================================================

/// Binds a libp2p peer identity (PeerId) to a Holochain agent (AgentCid).
///
/// Source of truth: DHT (Category A — Notarized). The ActionHash of this
/// entry is the canonical identifier. The `peer_identity_bindings` SQLite table
/// (Task A.5) carries `dht_anchor_hash` to link back.
///
/// Field notes:
/// - `peer_id`          — multibase-encoded libp2p PeerId (base58btc multihash)
/// - `agent_cid`        — CIDv1 (dag-cbor sha256) of the Agent EPR in imagodei
/// - `valid_from`       — UTC timestamp from which this binding is active
/// - `valid_until`      — optional expiry; absent = no declared expiry
/// - `device_archetype` — hardware/deployment role for resource allocation
/// - `signature`        — raw Ed25519 signature bytes over `canonical_bytes()`
/// - `superseded_by`    — ActionHash of the binding that replaces this one;
///                        `None` at creation, set externally when superseded.
///                        NOT included in canonical signing bytes.
#[hdk_entry_helper]
#[derive(Clone, PartialEq, Eq)]
pub struct AgentPeerBinding {
    pub peer_id: String,
    pub agent_cid: String,
    pub valid_from: Timestamp,
    pub valid_until: Option<Timestamp>,
    pub device_archetype: DeviceArchetype,
    pub signature: Vec<u8>,
    pub superseded_by: Option<ActionHash>,
}

impl AgentPeerBinding {
    /// Serialise the canonical signing body.
    ///
    /// Includes all fields **except** `signature` and `superseded_by`.
    /// Uses `holochain_serialized_bytes` (MessagePack via rmp-serde), which is
    /// deterministic for identical inputs: same field values → identical bytes
    /// on every call. Field order is fixed by `AgentPeerBindingCanonical`.
    ///
    /// The resulting bytes are what the creating agent's Ed25519 key signs, and
    /// what remote peers verify via the signature embedded in the binding.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, SerializedBytesError> {
        let archetype_str = match self.device_archetype {
            DeviceArchetype::Node => "node",
            DeviceArchetype::Desktop => "desktop",
            DeviceArchetype::Mobile => "mobile",
            DeviceArchetype::Steward => "steward",
        };

        let canonical = AgentPeerBindingCanonical {
            peer_id: self.peer_id.clone(),
            agent_cid: self.agent_cid.clone(),
            valid_from: self.valid_from.as_micros(),
            valid_until: self.valid_until.as_ref().map(|t| t.as_micros()),
            device_archetype: archetype_str.to_string(),
        };

        let sb: SerializedBytes = canonical.try_into()?;
        Ok(sb.bytes().to_vec())
    }
}

// =============================================================================
// Validator (HDI-compatible — no get_links)
// =============================================================================

/// Validate creation of an `AgentPeerBinding` entry.
///
/// Rules enforced here (integrity layer — deterministic, DHT-verifiable):
///   1. `peer_id` must be non-empty.
///   2. `agent_cid` must be non-empty.
///   3. `valid_from` must precede `valid_until` when `valid_until` is present.
///   4. `signature` must be non-empty (structural; full Ed25519 verification
///      requires the agent's current key, which involves DHT traversal best
///      done in the coordinator pre-commit gate — see §HDI constraint note).
///
/// NOTE — HDI constraint: `get_links` is NOT available in integrity-zome validation
/// callbacks. Cross-entity checks that need link traversal (e.g., resolving
/// `agent_cid` to an Agent EPR entry, verifying the signing key against the
/// current rotation chain) must live in the **coordinator pre-commit gate**
/// (Task A.3+, not this task). Rule 4 is a structural non-empty check only.
///
/// NOTE — `device_archetype` is validated implicitly by serde: an unknown
/// variant will fail deserialisation before this function is called.
///
/// NOTE — `superseded_by` is always `None` at creation; the reconciliation
/// controller sets it post-creation. No creation-time validator check needed.
pub fn validate_create_agent_peer_binding(
    binding: &AgentPeerBinding,
) -> ExternResult<ValidateCallbackResult> {
    // Rule 1: peer_id must be non-empty.
    if binding.peer_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "AgentPeerBinding peer_id cannot be empty".to_string(),
        ));
    }

    // Rule 2: agent_cid must be non-empty.
    if binding.agent_cid.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "AgentPeerBinding agent_cid cannot be empty".to_string(),
        ));
    }

    // Rule 3: valid_from must precede valid_until when present.
    if let Some(ref until) = binding.valid_until {
        if binding.valid_from >= *until {
            return Ok(ValidateCallbackResult::Invalid(
                "AgentPeerBinding valid_from must be before valid_until".to_string(),
            ));
        }
    }

    // Rule 4: signature must be non-empty (structural; Ed25519 full verification
    // belongs in coordinator pre-commit gate — get_links not available here).
    if binding.signature.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "AgentPeerBinding signature cannot be empty".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

// =============================================================================
// Unit Tests — pure logic, no HDI runtime required
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use hdi::prelude::Timestamp;

    fn fixture_binding() -> AgentPeerBinding {
        AgentPeerBinding {
            peer_id: "12D3KooWTest".to_string(),
            agent_cid: "bafyreiabc123".to_string(),
            valid_from: Timestamp::from_micros(1_000_000),
            valid_until: Some(Timestamp::from_micros(2_000_000)),
            device_archetype: DeviceArchetype::Node,
            signature: vec![0x01, 0x02, 0x03],
            superseded_by: None,
        }
    }

    // -- canonical_bytes stability --

    /// Golden test: identical inputs produce identical canonical bytes.
    /// Verifies determinism of MessagePack serialisation via holochain_serialized_bytes.
    #[test]
    fn canonical_bytes_stable_on_repeated_call() {
        let binding = fixture_binding();
        let first = binding
            .canonical_bytes()
            .expect("canonical_bytes failed first");
        let second = binding
            .canonical_bytes()
            .expect("canonical_bytes failed second");
        assert_eq!(
            first, second,
            "canonical_bytes must be identical on repeated calls"
        );
    }

    /// Changing a covered field changes the canonical bytes.
    #[test]
    fn canonical_bytes_changes_with_peer_id() {
        let b1 = fixture_binding();
        let mut b2 = fixture_binding();
        b2.peer_id = "12D3KooWOther".to_string();
        let bytes1 = b1.canonical_bytes().unwrap();
        let bytes2 = b2.canonical_bytes().unwrap();
        assert_ne!(
            bytes1, bytes2,
            "different peer_id must produce different canonical bytes"
        );
    }

    /// Changing `signature` does NOT change canonical bytes (signature is excluded).
    #[test]
    fn canonical_bytes_excludes_signature() {
        let b1 = fixture_binding();
        let mut b2 = fixture_binding();
        b2.signature = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let bytes1 = b1.canonical_bytes().unwrap();
        let bytes2 = b2.canonical_bytes().unwrap();
        assert_eq!(
            bytes1, bytes2,
            "signature field must NOT affect canonical bytes"
        );
    }

    /// Changing `superseded_by` does NOT change canonical bytes.
    #[test]
    fn canonical_bytes_excludes_superseded_by() {
        let b1 = fixture_binding();
        let mut b2 = fixture_binding();
        b2.superseded_by = Some(ActionHash::from_raw_36(vec![0xFFu8; 36]));
        let bytes1 = b1.canonical_bytes().unwrap();
        let bytes2 = b2.canonical_bytes().unwrap();
        assert_eq!(
            bytes1, bytes2,
            "superseded_by field must NOT affect canonical bytes"
        );
    }

    /// All four DeviceArchetype variants produce distinct canonical bytes.
    #[test]
    fn canonical_bytes_archetype_variants_distinct() {
        let archetypes = [
            DeviceArchetype::Node,
            DeviceArchetype::Desktop,
            DeviceArchetype::Mobile,
            DeviceArchetype::Steward,
        ];
        let bytes: Vec<Vec<u8>> = archetypes
            .iter()
            .map(|a| {
                let mut b = fixture_binding();
                b.device_archetype = a.clone();
                b.canonical_bytes().unwrap()
            })
            .collect();
        for i in 0..bytes.len() {
            for j in (i + 1)..bytes.len() {
                assert_ne!(
                    bytes[i], bytes[j],
                    "DeviceArchetype variants must produce distinct bytes"
                );
            }
        }
    }

    // -- validator rules --

    #[test]
    fn validate_accepts_valid_binding() {
        let binding = fixture_binding();
        let result = validate_create_agent_peer_binding(&binding).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Valid));
    }

    #[test]
    fn validate_rejects_empty_peer_id() {
        let mut binding = fixture_binding();
        binding.peer_id = "".to_string();
        let result = validate_create_agent_peer_binding(&binding).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(msg) if msg.contains("peer_id")));
    }

    #[test]
    fn validate_rejects_empty_agent_cid() {
        let mut binding = fixture_binding();
        binding.agent_cid = "".to_string();
        let result = validate_create_agent_peer_binding(&binding).unwrap();
        assert!(
            matches!(result, ValidateCallbackResult::Invalid(msg) if msg.contains("agent_cid"))
        );
    }

    #[test]
    fn validate_rejects_valid_from_after_valid_until() {
        let mut binding = fixture_binding();
        // valid_from (2_000_000) > valid_until (1_000_000)
        binding.valid_from = Timestamp::from_micros(2_000_000);
        binding.valid_until = Some(Timestamp::from_micros(1_000_000));
        let result = validate_create_agent_peer_binding(&binding).unwrap();
        assert!(
            matches!(result, ValidateCallbackResult::Invalid(msg) if msg.contains("valid_from"))
        );
    }

    #[test]
    fn validate_rejects_valid_from_equal_to_valid_until() {
        let mut binding = fixture_binding();
        binding.valid_from = Timestamp::from_micros(1_000_000);
        binding.valid_until = Some(Timestamp::from_micros(1_000_000));
        let result = validate_create_agent_peer_binding(&binding).unwrap();
        assert!(
            matches!(result, ValidateCallbackResult::Invalid(msg) if msg.contains("valid_from"))
        );
    }

    #[test]
    fn validate_accepts_no_valid_until() {
        let mut binding = fixture_binding();
        binding.valid_until = None;
        let result = validate_create_agent_peer_binding(&binding).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Valid));
    }

    #[test]
    fn validate_rejects_empty_signature() {
        let mut binding = fixture_binding();
        binding.signature = vec![];
        let result = validate_create_agent_peer_binding(&binding).unwrap();
        assert!(
            matches!(result, ValidateCallbackResult::Invalid(msg) if msg.contains("signature"))
        );
    }
}
