//! Bootstrap data types
//!
//! MessagePack-encoded structures for the bootstrap protocol.

use super::{MAX_EXPIRES_MS, MAX_URLS, MAX_URL_SIZE, MIN_EXPIRES_MS};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// Kitsune binary identifier length (4 byte location + 32 byte hash)
pub const KITSUNE_BIN_LEN: usize = 36;

/// Ed25519 signature length
pub const SIGNATURE_LEN: usize = 64;

/// Space identifier (DNA hash with location prefix)
pub type Space = [u8; KITSUNE_BIN_LEN];

/// Agent identifier (pubkey with location prefix)
pub type Agent = [u8; KITSUNE_BIN_LEN];

/// AgentInfo - the inner data that gets signed
#[derive(Debug, Clone)]
pub struct AgentInfo {
    /// DHT space (DNA hash)
    pub space: Space,
    /// Agent public key
    pub agent: Agent,
    /// URLs where the agent can be reached
    pub urls: Vec<String>,
    /// Unix timestamp (ms) when signed
    pub signed_at_ms: i64,
    /// Milliseconds after signed_at_ms when this expires
    pub expires_after_ms: i64,
}

impl AgentInfo {
    /// Parse AgentInfo from MessagePack bytes
    pub fn from_msgpack(data: &[u8]) -> Result<Self, String> {
        let value: rmpv::Value =
            rmpv::decode::read_value(&mut &data[..]).map_err(|e| format!("Decode error: {}", e))?;

        let map = value
            .as_map()
            .ok_or_else(|| "Expected map for agent_info".to_string())?;

        let space_bytes = extract_bytes(map, "space", KITSUNE_BIN_LEN)?;
        let agent_bytes = extract_bytes(map, "agent", KITSUNE_BIN_LEN)?;
        let urls = extract_string_array(map, "urls")?;
        let signed_at_ms = extract_int(map, "signed_at_ms")?;
        let expires_after_ms = extract_int(map, "expires_after_ms")?;

        let mut space = [0u8; KITSUNE_BIN_LEN];
        space.copy_from_slice(&space_bytes);

        let mut agent = [0u8; KITSUNE_BIN_LEN];
        agent.copy_from_slice(&agent_bytes);

        Ok(Self {
            space,
            agent,
            urls,
            signed_at_ms,
            expires_after_ms,
        })
    }
}

/// SignedAgentInfo - the outer envelope with signature
#[derive(Debug, Clone)]
pub struct SignedAgentInfo {
    /// Agent public key (must match inner agent_info.agent)
    pub agent: Agent,
    /// Ed25519 signature over agent_info bytes
    pub signature: [u8; SIGNATURE_LEN],
    /// The raw agent_info bytes (for verification and re-serialization)
    pub agent_info_bytes: Vec<u8>,
    /// Parsed agent info
    pub agent_info: AgentInfo,
}

impl SignedAgentInfo {
    /// Decode and verify signed agent info from MessagePack bytes
    pub fn decode_and_verify(data: &[u8]) -> Result<Self, String> {
        // Decode the outer envelope
        let value: rmpv::Value =
            rmpv::decode::read_value(&mut &data[..]).map_err(|e| format!("Decode error: {}", e))?;

        let map = value
            .as_map()
            .ok_or_else(|| "Expected map at root".to_string())?;

        // Extract fields
        let agent = extract_bytes(map, "agent", KITSUNE_BIN_LEN)?;
        let signature_bytes = extract_bytes(map, "signature", SIGNATURE_LEN)?;
        let agent_info_bytes = extract_raw_bytes(map, "agent_info")?;

        // Convert to fixed arrays
        let mut agent_arr = [0u8; KITSUNE_BIN_LEN];
        agent_arr.copy_from_slice(&agent);

        let mut sig_arr = [0u8; SIGNATURE_LEN];
        sig_arr.copy_from_slice(&signature_bytes);

        // Verify signature
        // The public key is the first 32 bytes (the hash part, not the location suffix)
        let pubkey_bytes: [u8; 32] = agent[0..32]
            .try_into()
            .map_err(|_| "Invalid pubkey length")?;

        let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)
            .map_err(|e| format!("Invalid public key: {}", e))?;

        let signature = Signature::from_bytes(&sig_arr);

        verifying_key
            .verify(&agent_info_bytes, &signature)
            .map_err(|_| "Signature verification failed".to_string())?;

        // Decode the inner agent info using our custom parser
        let agent_info = AgentInfo::from_msgpack(&agent_info_bytes)?;

        // Verify inner agent matches outer agent
        if agent_arr != agent_info.agent {
            return Err("Outer agent does not match inner agent".to_string());
        }

        // Validate URLs
        if agent_info.urls.len() > MAX_URLS {
            return Err(format!("Too many URLs (max {})", MAX_URLS));
        }
        for url in &agent_info.urls {
            if url.len() > MAX_URL_SIZE {
                return Err(format!("URL too long (max {} bytes)", MAX_URL_SIZE));
            }
        }

        // Validate expiry times
        if agent_info.expires_after_ms < MIN_EXPIRES_MS as i64 {
            return Err(format!("Expires too short (min {} ms)", MIN_EXPIRES_MS));
        }
        if agent_info.expires_after_ms > MAX_EXPIRES_MS as i64 {
            return Err(format!("Expires too long (max {} ms)", MAX_EXPIRES_MS));
        }

        Ok(Self {
            agent: agent_arr,
            signature: sig_arr,
            agent_info_bytes,
            agent_info,
        })
    }

    /// Validate timing constraints against current time
    pub fn validate_timing(&self, now_ms: u64) -> Result<(), String> {
        let signed_at = self.agent_info.signed_at_ms as u64;

        // Signature can't be from the future (with some tolerance)
        let tolerance_ms = 5 * 60 * 1000; // 5 minutes
        if signed_at > now_ms + tolerance_ms {
            return Err("Signature timestamp is in the future".to_string());
        }

        // Check if already expired
        let expires_at = signed_at + self.agent_info.expires_after_ms as u64;
        if expires_at <= now_ms {
            return Err("Agent info has expired".to_string());
        }

        Ok(())
    }

    /// Get the expiry timestamp in milliseconds
    pub fn expires_at_ms(&self) -> u64 {
        (self.agent_info.signed_at_ms + self.agent_info.expires_after_ms) as u64
    }
}

/// Query for random agents in a space
#[derive(Debug, Clone)]
pub struct RandomQuery {
    pub space: Space,
    pub limit: usize,
}

impl RandomQuery {
    /// Decode from MessagePack bytes
    pub fn decode(data: &[u8]) -> Result<Self, String> {
        let value: rmpv::Value =
            rmpv::decode::read_value(&mut &data[..]).map_err(|e| format!("Decode error: {}", e))?;

        let map = value
            .as_map()
            .ok_or_else(|| "Expected map at root".to_string())?;

        let space_bytes = extract_bytes(map, "space", KITSUNE_BIN_LEN)?;
        let limit = extract_int(map, "limit")?;

        let mut space = [0u8; KITSUNE_BIN_LEN];
        space.copy_from_slice(&space_bytes);

        Ok(Self {
            space,
            limit: limit as usize,
        })
    }
}

/// Extract binary field from MessagePack map
fn extract_bytes(
    map: &[(rmpv::Value, rmpv::Value)],
    key: &str,
    expected_len: usize,
) -> Result<Vec<u8>, String> {
    for (k, v) in map {
        if let Some(k_str) = k.as_str() {
            if k_str == key {
                if let Some(bytes) = v.as_slice() {
                    if bytes.len() != expected_len {
                        return Err(format!(
                            "Field '{}' has wrong length: {} (expected {})",
                            key,
                            bytes.len(),
                            expected_len
                        ));
                    }
                    return Ok(bytes.to_vec());
                }
            }
        }
    }
    Err(format!("Missing field: {}", key))
}

/// Extract raw bytes field from MessagePack map (any length)
fn extract_raw_bytes(map: &[(rmpv::Value, rmpv::Value)], key: &str) -> Result<Vec<u8>, String> {
    for (k, v) in map {
        if let Some(k_str) = k.as_str() {
            if k_str == key {
                if let Some(bytes) = v.as_slice() {
                    return Ok(bytes.to_vec());
                }
            }
        }
    }
    Err(format!("Missing field: {}", key))
}

/// Extract integer field from MessagePack map
fn extract_int(map: &[(rmpv::Value, rmpv::Value)], key: &str) -> Result<i64, String> {
    for (k, v) in map {
        if let Some(k_str) = k.as_str() {
            if k_str == key {
                if let Some(n) = v.as_i64() {
                    return Ok(n);
                }
                if let Some(n) = v.as_u64() {
                    return Ok(n as i64);
                }
            }
        }
    }
    Err(format!("Missing or invalid field: {}", key))
}

/// Extract string array field from MessagePack map
fn extract_string_array(
    map: &[(rmpv::Value, rmpv::Value)],
    key: &str,
) -> Result<Vec<String>, String> {
    for (k, v) in map {
        if let Some(k_str) = k.as_str() {
            if k_str == key {
                if let Some(arr) = v.as_array() {
                    let mut result = Vec::new();
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            result.push(s.to_string());
                        } else {
                            return Err(format!("Array item in '{}' is not a string", key));
                        }
                    }
                    return Ok(result);
                }
            }
        }
    }
    Err(format!("Missing field: {}", key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_int() {
        let map = vec![(
            rmpv::Value::String("limit".into()),
            rmpv::Value::Integer(10.into()),
        )];
        assert_eq!(extract_int(&map, "limit").unwrap(), 10);
    }
}
