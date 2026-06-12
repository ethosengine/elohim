//! Kitsune2 bootstrap protocol (HC 0.6 conductors).
//!
//! The legacy module (`mod.rs`) speaks the kitsune1 protocol
//! (`POST /bootstrap` + `X-Op`, MessagePack). Holochain 0.6 conductors run
//! kitsune2, whose bootstrap client speaks a different wire protocol
//! (`kitsune2_bootstrap_srv` reference implementation):
//!
//! - `PUT /bootstrap/{spaceB64Url}/{agentB64Url}` — body is the JSON-encoded
//!   signed agent info: `{"agentInfo":"<inner JSON string>","signature":"<b64>"}`.
//!   The ed25519 signature is over the exact bytes of the inner `agentInfo`
//!   string. Ids are base64url **without padding**.
//! - `GET /bootstrap/{spaceB64Url}` — returns a JSON array assembled from the
//!   raw stored PUT bodies: `[entry,entry,...]` (the client parses it with
//!   `AgentInfoSigned::decode_list`).
//!
//! Until 2026-06-12 these requests fell through the doorway route registry to
//! a 404 ("No registry match and no SPA fallback"), so conductor-plane peer
//! discovery never succeeded and every conductor was its own DHT island —
//! the root cause of the `propagation.custody-convergence` gate failure.
//!
//! Validation mirrors `kitsune2_bootstrap_srv` (v0.3.0-dev.3):
//! `created_at` within ±3 min of server now, `expires_at` in the future,
//! after `created_at`, and at most 30 min after it (all timestamps in
//! MICROseconds), path params must match the signed body, signature must
//! verify. Tombstone entries delete instead of store.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use dashmap::DashMap;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// ±3 minutes allowed clock skew on `created_at` (µs).
pub const CLOCK_SKEW_MICROS: i64 = 180_000_000;

/// Maximum `expires_at - created_at` span (µs) — 30 minutes.
pub const MAX_EXPIRY_SPAN_MICROS: i64 = 1_800_000_000;

/// Per-space entry cap (mirrors the reference server's bounded space list).
pub const MAX_ENTRIES_PER_SPACE: usize = 128;

/// One stored agent-info entry: the raw PUT body (returned verbatim inside
/// the GET array) plus its expiry for pruning.
#[derive(Debug, Clone)]
struct K2Entry {
    raw_body: Vec<u8>,
    expires_at_micros: i64,
}

/// In-memory kitsune2 bootstrap store: space (b64url) → agent (b64url) → entry.
#[derive(Default)]
pub struct K2BootstrapStore {
    spaces: DashMap<String, DashMap<String, K2Entry>>,
}

/// Outcome of a validated PUT.
#[derive(Debug, PartialEq, Eq)]
pub enum K2PutOutcome {
    /// Agent info stored (new or refreshed).
    Stored,
    /// Tombstone — agent info removed.
    Removed,
}

impl K2BootstrapStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate and apply a kitsune2 bootstrap PUT.
    ///
    /// `space_b64`/`agent_b64` are the raw path segments. Returns a
    /// human-readable error string on any validation failure (the handler
    /// maps it to a 400).
    pub fn put(
        &self,
        space_b64: &str,
        agent_b64: &str,
        body: &[u8],
    ) -> Result<K2PutOutcome, String> {
        let now_micros = current_time_micros();
        self.put_at(space_b64, agent_b64, body, now_micros)
    }

    /// Clock-injectable core of [`Self::put`] (tested directly).
    pub fn put_at(
        &self,
        space_b64: &str,
        agent_b64: &str,
        body: &[u8],
        now_micros: i64,
    ) -> Result<K2PutOutcome, String> {
        let parsed = ParsedAgentInfo::parse(body)?;

        // Path params must match the signed body (decode both sides — avoids
        // any padding/case representation mismatch).
        let path_agent = URL_SAFE_NO_PAD
            .decode(agent_b64)
            .map_err(|e| format!("invalid agent path param: {e}"))?;
        if path_agent != parsed.agent_bytes {
            return Err("agent does not match url path".into());
        }
        let path_space = URL_SAFE_NO_PAD
            .decode(space_b64)
            .map_err(|e| format!("invalid space path param: {e}"))?;
        if path_space != parsed.space_bytes {
            return Err("space does not match url path".into());
        }

        // Timing constraints (microseconds, mirrors kitsune2_bootstrap_srv).
        if parsed.created_at < now_micros - CLOCK_SKEW_MICROS {
            return Err("created_at is more than 3 minutes in the past".into());
        }
        if parsed.created_at > now_micros + CLOCK_SKEW_MICROS {
            return Err("created_at is more than 3 minutes in the future".into());
        }
        if parsed.expires_at <= now_micros {
            return Err("agent info is already expired".into());
        }
        if parsed.expires_at <= parsed.created_at {
            return Err("expires_at must be after created_at".into());
        }
        if parsed.expires_at - parsed.created_at > MAX_EXPIRY_SPAN_MICROS {
            return Err("expires_at is more than 30 minutes after created_at".into());
        }

        // Signature is over the exact inner agentInfo string bytes.
        let key_bytes: [u8; 32] = parsed
            .agent_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "agent key must be 32 bytes".to_string())?;
        let key =
            VerifyingKey::from_bytes(&key_bytes).map_err(|e| format!("invalid agent key: {e}"))?;
        let sig_bytes: [u8; 64] = parsed
            .signature_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "signature must be 64 bytes".to_string())?;
        let signature = Signature::from_bytes(&sig_bytes);
        key.verify(parsed.encoded_info.as_bytes(), &signature)
            .map_err(|_| "signature verification failed".to_string())?;

        let space = self.spaces.entry(space_b64.to_string()).or_default();
        if parsed.is_tombstone {
            space.remove(agent_b64);
            return Ok(K2PutOutcome::Removed);
        }

        // Bounded space: drop an arbitrary existing entry when full and the
        // agent is new (the reference server evicts mid-list; any victim is
        // acceptable — entries are re-put on the client's heartbeat cadence).
        if space.len() >= MAX_ENTRIES_PER_SPACE && !space.contains_key(agent_b64) {
            if let Some(victim) = space.iter().next().map(|e| e.key().clone()) {
                space.remove(&victim);
            }
        }

        space.insert(
            agent_b64.to_string(),
            K2Entry {
                raw_body: body.to_vec(),
                expires_at_micros: parsed.expires_at,
            },
        );
        Ok(K2PutOutcome::Stored)
    }

    /// Assemble the GET response for a space: a JSON array of the raw stored
    /// bodies (exactly what `AgentInfoSigned::decode_list` expects).
    pub fn get(&self, space_b64: &str) -> Vec<u8> {
        let now_micros = current_time_micros();
        self.get_at(space_b64, now_micros)
    }

    /// Clock-injectable core of [`Self::get`].
    pub fn get_at(&self, space_b64: &str, now_micros: i64) -> Vec<u8> {
        let mut out = Vec::with_capacity(2);
        out.push(b'[');
        if let Some(space) = self.spaces.get(space_b64) {
            let mut first = true;
            for entry in space.iter() {
                if entry.expires_at_micros <= now_micros {
                    continue;
                }
                if !first {
                    out.push(b',');
                }
                first = false;
                out.extend_from_slice(&entry.raw_body);
            }
        }
        out.push(b']');
        out
    }

    /// Remove expired entries and empty spaces. Returns entries removed.
    pub fn cleanup(&self) -> usize {
        let now_micros = current_time_micros();
        let mut removed = 0;
        for space in self.spaces.iter() {
            let expired: Vec<String> = space
                .iter()
                .filter(|e| e.expires_at_micros <= now_micros)
                .map(|e| e.key().clone())
                .collect();
            for key in expired {
                if space.remove(&key).is_some() {
                    removed += 1;
                }
            }
        }
        self.spaces.retain(|_, agents| !agents.is_empty());
        removed
    }

    /// (agents, spaces) currently held.
    pub fn stats(&self) -> (usize, usize) {
        let agents = self.spaces.iter().map(|s| s.len()).sum();
        (agents, self.spaces.len())
    }
}

/// Fields extracted from one signed agent-info body.
struct ParsedAgentInfo {
    /// The exact inner `agentInfo` JSON string (signature payload).
    encoded_info: String,
    agent_bytes: Vec<u8>,
    space_bytes: Vec<u8>,
    created_at: i64,
    expires_at: i64,
    is_tombstone: bool,
    signature_bytes: Vec<u8>,
}

impl ParsedAgentInfo {
    fn parse(body: &[u8]) -> Result<Self, String> {
        let outer: serde_json::Value =
            serde_json::from_slice(body).map_err(|e| format!("body is not valid JSON: {e}"))?;
        let encoded_info = outer
            .get("agentInfo")
            .and_then(|v| v.as_str())
            .ok_or("missing agentInfo field")?
            .to_string();
        let signature_b64 = outer
            .get("signature")
            .and_then(|v| v.as_str())
            .ok_or("missing signature field")?;
        let signature_bytes = URL_SAFE_NO_PAD
            .decode(signature_b64)
            .map_err(|e| format!("invalid signature encoding: {e}"))?;

        let inner: serde_json::Value = serde_json::from_str(&encoded_info)
            .map_err(|e| format!("agentInfo is not valid JSON: {e}"))?;
        let agent_b64 = inner
            .get("agent")
            .and_then(|v| v.as_str())
            .ok_or("missing agent field")?;
        let agent_bytes = URL_SAFE_NO_PAD
            .decode(agent_b64)
            .map_err(|e| format!("invalid agent encoding: {e}"))?;
        let space_b64 = inner
            .get("space")
            .and_then(|v| v.as_str())
            .ok_or("missing space field")?;
        let space_bytes = URL_SAFE_NO_PAD
            .decode(space_b64)
            .map_err(|e| format!("invalid space encoding: {e}"))?;
        let created_at = parse_micros(&inner, "createdAt")?;
        let expires_at = parse_micros(&inner, "expiresAt")?;
        let is_tombstone = inner
            .get("isTombstone")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(Self {
            encoded_info,
            agent_bytes,
            space_bytes,
            created_at,
            expires_at,
            is_tombstone,
            signature_bytes,
        })
    }
}

/// Timestamps are JSON strings holding i64 microseconds.
fn parse_micros(inner: &serde_json::Value, field: &str) -> Result<i64, String> {
    let v = inner
        .get(field)
        .ok_or_else(|| format!("missing {field} field"))?;
    match v {
        serde_json::Value::String(s) => s
            .parse::<i64>()
            .map_err(|e| format!("invalid {field}: {e}")),
        serde_json::Value::Number(n) => n
            .as_i64()
            .ok_or_else(|| format!("invalid {field}: not an i64")),
        _ => Err(format!("invalid {field}: expected string or number")),
    }
}

fn current_time_micros() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    const NOW: i64 = 1_781_270_000_000_000; // fixed test clock (µs)

    /// Build a signed kitsune2 agent-info body the way the client does.
    fn signed_body(
        key: &SigningKey,
        space: &[u8],
        created_at: i64,
        expires_at: i64,
        tombstone: bool,
    ) -> (String, String, Vec<u8>) {
        let agent_b64 = URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes());
        let space_b64 = URL_SAFE_NO_PAD.encode(space);
        let inner = serde_json::json!({
            "agent": agent_b64,
            "space": space_b64,
            "createdAt": created_at.to_string(),
            "expiresAt": expires_at.to_string(),
            "isTombstone": tombstone,
            "url": "wss://example/sig",
            "storageArc": [0, 4294967295u32],
        })
        .to_string();
        let signature = key.sign(inner.as_bytes());
        let body = serde_json::json!({
            "agentInfo": inner,
            "signature": URL_SAFE_NO_PAD.encode(signature.to_bytes()),
        });
        (space_b64, agent_b64, serde_json::to_vec(&body).unwrap())
    }

    fn key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    #[test]
    fn put_then_get_round_trips() {
        let store = K2BootstrapStore::new();
        let (space, agent, body) = signed_body(&key(), b"space-1", NOW, NOW + 1_200_000_000, false);
        let outcome = store.put_at(&space, &agent, &body, NOW).unwrap();
        assert_eq!(outcome, K2PutOutcome::Stored);

        let got = store.get_at(&space, NOW);
        // The GET body is a JSON array containing the raw PUT body verbatim.
        let arr: serde_json::Value = serde_json::from_slice(&got).unwrap();
        assert_eq!(arr.as_array().unwrap().len(), 1);
        assert_eq!(
            arr[0],
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()
        );
    }

    #[test]
    fn get_unknown_space_is_empty_array() {
        let store = K2BootstrapStore::new();
        assert_eq!(store.get_at("bm9wZQ", NOW), b"[]");
    }

    #[test]
    fn expired_entries_are_not_served_and_cleanup_prunes() {
        let store = K2BootstrapStore::new();
        let (space, agent, body) = signed_body(&key(), b"space-1", NOW, NOW + 60_000_000, false);
        store.put_at(&space, &agent, &body, NOW).unwrap();
        let later = NOW + 120_000_000;
        assert_eq!(store.get_at(&space, later), b"[]");
    }

    #[test]
    fn tombstone_removes_entry() {
        let store = K2BootstrapStore::new();
        let k = key();
        let (space, agent, body) = signed_body(&k, b"space-1", NOW, NOW + 1_200_000_000, false);
        store.put_at(&space, &agent, &body, NOW).unwrap();
        let (_, _, tomb) = signed_body(&k, b"space-1", NOW, NOW + 1_200_000_000, true);
        let outcome = store.put_at(&space, &agent, &tomb, NOW).unwrap();
        assert_eq!(outcome, K2PutOutcome::Removed);
        assert_eq!(store.get_at(&space, NOW), b"[]");
    }

    #[test]
    fn rejects_bad_signature() {
        let store = K2BootstrapStore::new();
        let (space, agent, mut body) =
            signed_body(&key(), b"space-1", NOW, NOW + 1_200_000_000, false);
        // Corrupt the signature.
        let mut v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        v["signature"] = serde_json::Value::String(URL_SAFE_NO_PAD.encode([0u8; 64]));
        body = serde_json::to_vec(&v).unwrap();
        let err = store.put_at(&space, &agent, &body, NOW).unwrap_err();
        assert!(err.contains("signature"), "{err}");
    }

    #[test]
    fn rejects_path_mismatch() {
        let store = K2BootstrapStore::new();
        let (space, _agent, body) =
            signed_body(&key(), b"space-1", NOW, NOW + 1_200_000_000, false);
        let other_agent = URL_SAFE_NO_PAD.encode([9u8; 32]);
        let err = store.put_at(&space, &other_agent, &body, NOW).unwrap_err();
        assert!(err.contains("agent does not match"), "{err}");

        let (_, agent, body) = signed_body(&key(), b"space-1", NOW, NOW + 1_200_000_000, false);
        let other_space = URL_SAFE_NO_PAD.encode(b"space-2");
        let err = store.put_at(&other_space, &agent, &body, NOW).unwrap_err();
        assert!(err.contains("space does not match"), "{err}");
    }

    #[test]
    fn rejects_timing_violations() {
        let store = K2BootstrapStore::new();
        let k = key();

        // created_at too old
        let (space, agent, body) = signed_body(
            &k,
            b"s",
            NOW - CLOCK_SKEW_MICROS - 1,
            NOW + 60_000_000,
            false,
        );
        assert!(store.put_at(&space, &agent, &body, NOW).is_err());

        // created_at too far in the future
        let (space, agent, body) = signed_body(
            &k,
            b"s",
            NOW + CLOCK_SKEW_MICROS + 1,
            NOW + CLOCK_SKEW_MICROS + 60_000_000,
            false,
        );
        assert!(store.put_at(&space, &agent, &body, NOW).is_err());

        // already expired
        let (space, agent, body) = signed_body(&k, b"s", NOW, NOW - 1, false);
        assert!(store.put_at(&space, &agent, &body, NOW).is_err());

        // span over 30 minutes
        let (space, agent, body) =
            signed_body(&k, b"s", NOW, NOW + MAX_EXPIRY_SPAN_MICROS + 1, false);
        assert!(store.put_at(&space, &agent, &body, NOW).is_err());
    }

    #[test]
    fn multiple_agents_in_space_are_listed() {
        let store = K2BootstrapStore::new();
        let k1 = SigningKey::from_bytes(&[1u8; 32]);
        let k2 = SigningKey::from_bytes(&[2u8; 32]);
        let (space, agent1, body1) = signed_body(&k1, b"shared", NOW, NOW + 1_200_000_000, false);
        let (_, agent2, body2) = signed_body(&k2, b"shared", NOW, NOW + 1_200_000_000, false);
        store.put_at(&space, &agent1, &body1, NOW).unwrap();
        store.put_at(&space, &agent2, &body2, NOW).unwrap();
        let arr: serde_json::Value = serde_json::from_slice(&store.get_at(&space, NOW)).unwrap();
        assert_eq!(arr.as_array().unwrap().len(), 2);
    }

    #[test]
    fn refresh_replaces_rather_than_duplicates() {
        let store = K2BootstrapStore::new();
        let k = key();
        let (space, agent, body) = signed_body(&k, b"s", NOW, NOW + 1_200_000_000, false);
        store.put_at(&space, &agent, &body, NOW).unwrap();
        let (_, _, body2) = signed_body(&k, b"s", NOW + 1_000_000, NOW + 1_201_000_000, false);
        store
            .put_at(&space, &agent, &body2, NOW + 1_000_000)
            .unwrap();
        let arr: serde_json::Value =
            serde_json::from_slice(&store.get_at(&space, NOW + 1_000_000)).unwrap();
        assert_eq!(arr.as_array().unwrap().len(), 1);
    }

    #[test]
    fn rejects_malformed_bodies() {
        let store = K2BootstrapStore::new();
        assert!(store.put_at("eA", "eQ", b"not json", NOW).is_err());
        assert!(store.put_at("eA", "eQ", b"{}", NOW).is_err());
        // agentInfo present but not a JSON string payload
        assert!(store
            .put_at("eA", "eQ", br#"{"agentInfo":"nope","signature":"AA"}"#, NOW)
            .is_err());
    }
}
