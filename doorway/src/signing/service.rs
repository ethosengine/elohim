//! Signing Service
//!
//! Handles signing requests from authenticated humans, validates them,
//! and submits signed entries to the Holochain conductor.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::types::DoorwayError;

use super::session::{HumanSession, SessionStore};

/// Signing service configuration
#[derive(Debug, Clone)]
pub struct SigningConfig {
    /// Whether signing is enabled
    pub enabled: bool,

    /// Maximum entries per session
    pub max_entries_per_session: u64,

    /// Rate limit: entries per minute per session
    pub rate_limit_per_minute: u32,

    /// Allowed entry types (empty = all allowed)
    pub allowed_entry_types: Vec<String>,

    /// Blocked entry types (takes precedence over allowed)
    pub blocked_entry_types: Vec<String>,

    /// Session TTL in seconds
    pub session_ttl_seconds: u64,
}

impl Default for SigningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries_per_session: 1000,
            rate_limit_per_minute: 60,
            allowed_entry_types: Vec::new(), // All allowed by default
            blocked_entry_types: vec![
                // Block sensitive entry types by default
                "Agent".to_string(),
                "CapGrant".to_string(),
                "CapClaim".to_string(),
            ],
            session_ttl_seconds: 3600, // 1 hour
        }
    }
}

/// Request to sign and submit an entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignRequest {
    /// Session ID for authentication
    pub session_id: String,

    /// DNA hash to target
    pub dna_hash: String,

    /// Zome name
    pub zome: String,

    /// Function name (should be a create/update function)
    pub fn_name: String,

    /// Entry data to sign
    pub payload: JsonValue,

    /// Optional idempotency key to prevent duplicate submissions
    pub idempotency_key: Option<String>,
}

/// Response from signing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignResponse {
    /// Whether the signing was successful
    pub success: bool,

    /// The action hash of the committed entry (if successful)
    pub action_hash: Option<String>,

    /// The entry hash (if successful)
    pub entry_hash: Option<String>,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Remaining entries allowed in this session
    pub remaining_entries: u64,
}

impl SignResponse {
    /// Create a success response
    pub fn success(action_hash: String, entry_hash: String, remaining: u64) -> Self {
        Self {
            success: true,
            action_hash: Some(action_hash),
            entry_hash: Some(entry_hash),
            error: None,
            remaining_entries: remaining,
        }
    }

    /// Create an error response
    pub fn error(msg: impl Into<String>, remaining: u64) -> Self {
        Self {
            success: false,
            action_hash: None,
            entry_hash: None,
            error: Some(msg.into()),
            remaining_entries: remaining,
        }
    }
}

/// Rate limiter for signing requests
struct RateLimiter {
    /// Requests per session in the current window
    requests: dashmap::DashMap<String, Vec<u64>>,
    /// Window size in seconds
    window_seconds: u64,
    /// Max requests per window
    max_requests: u32,
}

impl RateLimiter {
    fn new(window_seconds: u64, max_requests: u32) -> Self {
        Self {
            requests: dashmap::DashMap::new(),
            window_seconds,
            max_requests,
        }
    }

    /// Check if a request is allowed and record it
    fn check_and_record(&self, session_id: &str) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let cutoff = now.saturating_sub(self.window_seconds);

        let mut entry = self.requests.entry(session_id.to_string()).or_default();

        // Remove old requests
        entry.retain(|&ts| ts > cutoff);

        // Check limit
        if entry.len() >= self.max_requests as usize {
            return false;
        }

        // Record this request
        entry.push(now);
        true
    }

    /// Clean up old entries
    fn cleanup(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let cutoff = now.saturating_sub(self.window_seconds);

        self.requests.retain(|_, requests| {
            requests.retain(|&ts| ts > cutoff);
            !requests.is_empty()
        });
    }
}

/// Idempotency cache to prevent duplicate submissions
struct IdempotencyCache {
    /// Cached results by idempotency key
    cache: dashmap::DashMap<String, (SignResponse, u64)>,
    /// TTL for cached results
    ttl_seconds: u64,
}

impl IdempotencyCache {
    fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: dashmap::DashMap::new(),
            ttl_seconds,
        }
    }

    /// Get cached result if exists and not expired
    fn get(&self, key: &str) -> Option<SignResponse> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(entry) = self.cache.get(key) {
            let (response, created_at) = entry.value();
            if now - created_at < self.ttl_seconds {
                return Some(response.clone());
            }
        }
        None
    }

    /// Store a result
    fn set(&self, key: &str, response: &SignResponse) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.cache.insert(key.to_string(), (response.clone(), now));
    }

    /// Clean up expired entries
    fn cleanup(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.cache.retain(|_, (_, created_at)| now - *created_at < self.ttl_seconds);
    }
}

/// Gateway Signing Service
///
/// Manages signing requests from authenticated humans and submits
/// signed entries to the Holochain conductor.
pub struct SigningService {
    /// Configuration
    config: SigningConfig,

    /// Session store
    sessions: Arc<SessionStore>,

    /// Rate limiter
    rate_limiter: RateLimiter,

    /// Idempotency cache
    idempotency_cache: IdempotencyCache,
}

impl SigningService {
    /// Create a new signing service
    pub fn new(config: SigningConfig) -> Self {
        let sessions = Arc::new(SessionStore::new(
            config.session_ttl_seconds,
            5, // max 5 sessions per human
        ));

        let rate_limiter = RateLimiter::new(60, config.rate_limit_per_minute);
        let idempotency_cache = IdempotencyCache::new(300); // 5 minute cache

        Self {
            config,
            sessions,
            rate_limiter,
            idempotency_cache,
        }
    }

    /// Check if signing is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get or create a session for a human
    pub fn get_or_create_session(
        &self,
        human_id: &str,
        display_name: Option<String>,
        agent_pubkey: &str,
    ) -> HumanSession {
        self.sessions.get_or_create(human_id, display_name, agent_pubkey)
    }

    /// Validate a session
    pub fn validate_session(&self, session_id: &str) -> Option<HumanSession> {
        self.sessions.validate(session_id)
    }

    /// Process a signing request
    pub async fn sign(&self, request: SignRequest) -> Result<SignResponse, DoorwayError> {
        // Check if signing is enabled
        if !self.config.enabled {
            return Ok(SignResponse::error("Signing service is disabled", 0));
        }

        // Validate session
        let session = match self.sessions.validate(&request.session_id) {
            Some(s) => s,
            None => {
                return Ok(SignResponse::error("Invalid or expired session", 0));
            }
        };

        // Check session entry limit
        let remaining = self.config.max_entries_per_session.saturating_sub(session.entries_signed);
        if remaining == 0 {
            return Ok(SignResponse::error("Session entry limit reached", 0));
        }

        // Check rate limit
        if !self.rate_limiter.check_and_record(&request.session_id) {
            return Ok(SignResponse::error(
                "Rate limit exceeded. Please wait before submitting more entries.",
                remaining,
            ));
        }

        // Check idempotency
        if let Some(ref key) = request.idempotency_key {
            if let Some(cached) = self.idempotency_cache.get(key) {
                debug!("Returning cached response for idempotency key: {}", key);
                return Ok(cached);
            }
        }

        // Validate entry type
        if !self.is_entry_type_allowed(&request.fn_name) {
            return Ok(SignResponse::error(
                format!("Entry type '{}' is not allowed for gateway signing", request.fn_name),
                remaining,
            ));
        }

        // TODO: Actually submit to conductor
        // This would involve:
        // 1. Getting a connection from the worker pool
        // 2. Making a zome call with the session's agent pubkey
        // 3. Returning the action/entry hashes

        // For now, return a placeholder indicating the conductor bridge is needed
        let response = SignResponse::error(
            format!(
                "Signing bridge not yet implemented. Request would sign: {}/{}/{}",
                request.dna_hash, request.zome, request.fn_name
            ),
            remaining,
        );

        // Record the attempt (even though it failed)
        self.sessions.record_signing(&request.session_id);

        // Cache the result for idempotency
        if let Some(ref key) = request.idempotency_key {
            self.idempotency_cache.set(key, &response);
        }

        Ok(response)
    }

    /// Check if an entry type is allowed for gateway signing
    fn is_entry_type_allowed(&self, fn_name: &str) -> bool {
        // Check blocked list first
        if self.config.blocked_entry_types.iter().any(|t| fn_name.contains(t)) {
            return false;
        }

        // If allowed list is empty, all non-blocked types are allowed
        if self.config.allowed_entry_types.is_empty() {
            return true;
        }

        // Check allowed list
        self.config.allowed_entry_types.iter().any(|t| fn_name.contains(t))
    }

    /// Get session store reference
    pub fn sessions(&self) -> &SessionStore {
        &self.sessions
    }

    /// Perform periodic cleanup
    pub fn cleanup(&self) {
        self.sessions.cleanup();
        self.rate_limiter.cleanup();
        self.idempotency_cache.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signing_config_default() {
        let config = SigningConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_entries_per_session, 1000);
        assert!(config.blocked_entry_types.contains(&"Agent".to_string()));
    }

    #[test]
    fn test_sign_response() {
        let success = SignResponse::success(
            "uhCkk...".to_string(),
            "uhCEk...".to_string(),
            999,
        );
        assert!(success.success);
        assert!(success.action_hash.is_some());

        let error = SignResponse::error("Test error", 500);
        assert!(!error.success);
        assert!(error.error.is_some());
    }

    #[test]
    fn test_entry_type_blocking() {
        let config = SigningConfig::default();
        let service = SigningService::new(config);

        // Blocked types
        assert!(!service.is_entry_type_allowed("create_Agent"));
        assert!(!service.is_entry_type_allowed("update_CapGrant"));

        // Allowed types
        assert!(service.is_entry_type_allowed("create_content"));
        assert!(service.is_entry_type_allowed("update_learning_path"));
    }

    #[tokio::test]
    async fn test_session_validation() {
        let service = SigningService::new(SigningConfig::default());

        // Create a session
        let session = service.get_or_create_session(
            "human_123",
            Some("Alice".to_string()),
            "uhCAk...",
        );

        // Validate should succeed
        assert!(service.validate_session(&session.session_id).is_some());

        // Invalid session should fail
        assert!(service.validate_session("invalid_session").is_none());
    }
}
