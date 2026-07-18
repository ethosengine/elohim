//! `AgentPubKey` ↔ `did:key` codec.
//!
//! A Holochain agent hash is **39 bytes**:
//! `0x84 0x20 0x24` (3-byte AgentPubKey prefix) + 32-byte ed25519 core +
//! 4-byte DHT location. Its canonical string form is the multibase `u` prefix
//! followed by unpadded base64url of the 39 bytes (e.g. `uhCAk…`).
//!
//! `did:key` for an ed25519 key is `did:key:z` + base58btc of the
//! multicodec-tagged public key (`0xed 0x01` + the 32-byte core), per the
//! did:key method spec.
//!
//! The reverse path (did:key → agent_cid) **recomputes the 4 DHT location
//! bytes** from the core exactly as `holo_hash` does — blake2b-128 of the core,
//! XOR-folded 16→4 bytes — so the round-trip is byte-identical for a real
//! Holochain agent key. Cross-checked against the `holo_hash` reference
//! implementation's own test vectors (see did-tests).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use thiserror::Error;

/// The 3-byte multihash prefix for a Holochain `AgentPubKey` (`hCAk…`).
pub const AGENT_PUBKEY_PREFIX: [u8; 3] = [0x84, 0x20, 0x24];
/// The multicodec prefix for an ed25519 public key (`0xed 0x01`), used by did:key.
pub const ED25519_MULTICODEC: [u8; 2] = [0xed, 0x01];
/// Full length of a Holochain hash in bytes (3 prefix + 32 core + 4 loc).
pub const HOLO_HASH_LEN: usize = 39;
/// Length of the ed25519 key core.
pub const CORE_LEN: usize = 32;

/// Errors from the agent-key / did:key codec.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CodecError {
    /// The agent_cid string was not multibase-`u` base64url.
    #[error("agent_cid must be multibase 'u' (base64url) form: {0}")]
    NotBase64UrlMultibase(String),
    /// The decoded agent hash was not 39 bytes.
    #[error("agent hash must be {HOLO_HASH_LEN} bytes, got {0}")]
    BadHashLength(usize),
    /// The agent hash prefix was not the AgentPubKey prefix.
    #[error("expected AgentPubKey prefix 0x842024, got {0:02x?}")]
    BadPrefix([u8; 3]),
    /// The did:key string lacked the `did:key:z` multibase-base58btc form.
    #[error("did:key must be 'did:key:z<base58btc>': {0}")]
    NotDidKeyBase58(String),
    /// base58 decoding failed.
    #[error("base58btc decode failed: {0}")]
    Base58(String),
    /// The multicodec prefix was not the ed25519 tag `0xed01`.
    #[error("expected ed25519 multicodec 0xed01, got {0:02x?}")]
    BadMulticodec(Vec<u8>),
    /// The multicodec key material was not 32 bytes.
    #[error("ed25519 key material must be {CORE_LEN} bytes, got {0}")]
    BadKeyLength(usize),
}

/// Compute the 4-byte DHT location for a 32-byte core, exactly as `holo_hash`:
/// blake2b with a 16-byte digest, then XOR-fold the 16 bytes down to 4.
pub fn dht_location_bytes(core: &[u8; CORE_LEN]) -> [u8; 4] {
    let hash = blake2b_simd::Params::new()
        .hash_length(16)
        .hash(core)
        .as_bytes()
        .to_vec();
    let mut out = [hash[0], hash[1], hash[2], hash[3]];
    let mut i = 4;
    while i < 16 {
        out[0] ^= hash[i];
        out[1] ^= hash[i + 1];
        out[2] ^= hash[i + 2];
        out[3] ^= hash[i + 3];
        i += 4;
    }
    out
}

/// Extract the 32-byte ed25519 core from an `agent_cid` (`uhCAk…`) string.
pub fn agent_cid_to_core32(agent_cid: &str) -> Result<[u8; CORE_LEN], CodecError> {
    let b64 = agent_cid
        .strip_prefix('u')
        .ok_or_else(|| CodecError::NotBase64UrlMultibase(agent_cid.to_string()))?;
    let raw = URL_SAFE_NO_PAD
        .decode(b64)
        .map_err(|_| CodecError::NotBase64UrlMultibase(agent_cid.to_string()))?;
    if raw.len() != HOLO_HASH_LEN {
        return Err(CodecError::BadHashLength(raw.len()));
    }
    let prefix = [raw[0], raw[1], raw[2]];
    if prefix != AGENT_PUBKEY_PREFIX {
        return Err(CodecError::BadPrefix(prefix));
    }
    let mut core = [0u8; CORE_LEN];
    core.copy_from_slice(&raw[3..3 + CORE_LEN]);
    Ok(core)
}

/// Build the canonical `agent_cid` (`uhCAk…`) from a 32-byte core, recomputing
/// the 4 DHT location bytes.
pub fn core32_to_agent_cid(core: &[u8; CORE_LEN]) -> String {
    let mut raw = Vec::with_capacity(HOLO_HASH_LEN);
    raw.extend_from_slice(&AGENT_PUBKEY_PREFIX);
    raw.extend_from_slice(core);
    raw.extend_from_slice(&dht_location_bytes(core));
    format!("u{}", URL_SAFE_NO_PAD.encode(&raw))
}

/// Encode a 32-byte ed25519 core as a `did:key` string.
pub fn core32_to_did_key(core: &[u8; CORE_LEN]) -> String {
    let mut tagged = Vec::with_capacity(2 + CORE_LEN);
    tagged.extend_from_slice(&ED25519_MULTICODEC);
    tagged.extend_from_slice(core);
    format!("did:key:z{}", bs58::encode(tagged).into_string())
}

/// Return just the multibase key value (`z…`) — the `publicKeyMultibase` for a
/// Multikey verification method — for a 32-byte core.
pub fn core32_to_multikey(core: &[u8; CORE_LEN]) -> String {
    let mut tagged = Vec::with_capacity(2 + CORE_LEN);
    tagged.extend_from_slice(&ED25519_MULTICODEC);
    tagged.extend_from_slice(core);
    format!("z{}", bs58::encode(tagged).into_string())
}

/// Decode a `did:key` string back to its 32-byte ed25519 core.
pub fn did_key_to_core32(did_key: &str) -> Result<[u8; CORE_LEN], CodecError> {
    let mb = did_key
        .strip_prefix("did:key:z")
        .ok_or_else(|| CodecError::NotDidKeyBase58(did_key.to_string()))?;
    let decoded = bs58::decode(mb)
        .into_vec()
        .map_err(|e| CodecError::Base58(e.to_string()))?;
    if decoded.len() < 2 || decoded[0..2] != ED25519_MULTICODEC {
        return Err(CodecError::BadMulticodec(
            decoded.into_iter().take(2).collect(),
        ));
    }
    let key = &decoded[2..];
    if key.len() != CORE_LEN {
        return Err(CodecError::BadKeyLength(key.len()));
    }
    let mut core = [0u8; CORE_LEN];
    core.copy_from_slice(key);
    Ok(core)
}

/// Convert an `agent_cid` (`uhCAk…`) to its `did:key` form.
pub fn agent_cid_to_did_key(agent_cid: &str) -> Result<String, CodecError> {
    let core = agent_cid_to_core32(agent_cid)?;
    Ok(core32_to_did_key(&core))
}

/// Convert a `did:key` form back to a canonical `agent_cid` (`uhCAk…`),
/// recomputing the DHT location bytes.
pub fn did_key_to_agent_cid(did_key: &str) -> Result<String, CodecError> {
    let core = did_key_to_core32(did_key)?;
    Ok(core32_to_agent_cid(&core))
}

#[cfg(test)]
mod tests {
    use super::*;

    // A real Holochain agent key — the `holo_hash` reference implementation's own
    // test vector. Its loc bytes are cross-checked against holo_hash's blake2b
    // XOR-fold (see did-tests for the full vector set).
    const AGENT_KEY: &str = "uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH";
    const AGENT_KEY_DIDKEY: &str = "did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr";

    #[test]
    fn agent_key_loc_recompute_is_identical() {
        // Extract core, then reconstruct the full agent_cid recomputing loc —
        // must equal the original key byte-for-byte.
        let core = agent_cid_to_core32(AGENT_KEY).unwrap();
        assert_eq!(core32_to_agent_cid(&core), AGENT_KEY);
    }

    #[test]
    fn agent_key_did_key_vector() {
        assert_eq!(agent_cid_to_did_key(AGENT_KEY).unwrap(), AGENT_KEY_DIDKEY);
        assert_eq!(did_key_to_agent_cid(AGENT_KEY_DIDKEY).unwrap(), AGENT_KEY);
    }

    #[test]
    fn rejects_bad_prefix_and_length() {
        assert!(matches!(
            agent_cid_to_core32("uAAAA"),
            Err(CodecError::BadHashLength(_)) | Err(CodecError::NotBase64UrlMultibase(_))
        ));
        assert!(matches!(
            agent_cid_to_core32("39SDf7"),
            Err(CodecError::NotBase64UrlMultibase(_))
        ));
    }
}
