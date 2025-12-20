//! Human Session Management
//!
//! Tracks authenticated human sessions and their associated agent pubkeys.

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Represents an authenticated human session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanSession {
    /// Unique session ID
    pub session_id: String,

    /// Human's identity (from JWT sub claim or similar)
    pub human_id: String,

    /// Human's display name (if available)
    pub display_name: Option<String>,

    /// The Holochain agent pubkey associated with this human
    /// This is derived deterministically from their identity
    pub agent_pubkey: String,

    /// Session creation timestamp
    pub created_at: u64,

    /// Session expiry timestamp
    pub expires_at: u64,

    /// Number of entries signed in this session
    pub entries_signed: u64,

    /// Last activity timestamp
    pub last_activity: u64,
}

impl HumanSession {
    /// Create a new session for a human
    pub fn new(
        session_id: String,
        human_id: String,
        display_name: Option<String>,
        agent_pubkey: String,
        ttl_seconds: u64,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            session_id,
            human_id,
            display_name,
            agent_pubkey,
            created_at: now,
            expires_at: now + ttl_seconds,
            entries_signed: 0,
            last_activity: now,
        }
    }

    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.expires_at
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Increment signed entries counter
    pub fn record_signing(&mut self) {
        self.entries_signed += 1;
        self.touch();
    }

    /// Remaining session time in seconds
    pub fn remaining_seconds(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.expires_at.saturating_sub(now)
    }
}

/// In-memory session store with expiration
pub struct SessionStore {
    /// Active sessions by session ID
    sessions: DashMap<String, HumanSession>,

    /// Session ID lookup by human ID (for finding existing sessions)
    by_human: DashMap<String, String>,

    /// Default session TTL
    default_ttl: Duration,

    /// Maximum sessions per human
    max_sessions_per_human: usize,

    /// Last cleanup timestamp
    last_cleanup: std::sync::atomic::AtomicU64,
}

impl SessionStore {
    /// Create a new session store
    pub fn new(default_ttl_seconds: u64, max_sessions_per_human: usize) -> Self {
        Self {
            sessions: DashMap::new(),
            by_human: DashMap::new(),
            default_ttl: Duration::from_secs(default_ttl_seconds),
            max_sessions_per_human,
            last_cleanup: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Create or retrieve a session for a human
    pub fn get_or_create(
        &self,
        human_id: &str,
        display_name: Option<String>,
        agent_pubkey: &str,
    ) -> HumanSession {
        // Check for existing valid session
        if let Some(session_id) = self.by_human.get(human_id) {
            if let Some(mut session) = self.sessions.get_mut(session_id.value()) {
                if !session.is_expired() {
                    session.touch();
                    return session.clone();
                }
            }
        }

        // Create new session
        let session_id = format!("sess_{}", uuid::Uuid::new_v4());
        let session = HumanSession::new(
            session_id.clone(),
            human_id.to_string(),
            display_name,
            agent_pubkey.to_string(),
            self.default_ttl.as_secs(),
        );

        self.sessions.insert(session_id.clone(), session.clone());
        self.by_human.insert(human_id.to_string(), session_id);

        info!("Created new session for human: {}", human_id);

        // Periodic cleanup
        self.maybe_cleanup();

        session
    }

    /// Get a session by ID
    pub fn get(&self, session_id: &str) -> Option<HumanSession> {
        self.sessions.get(session_id).map(|s| s.clone())
    }

    /// Get a session by human ID
    pub fn get_by_human(&self, human_id: &str) -> Option<HumanSession> {
        let session_id = self.by_human.get(human_id)?;
        self.get(session_id.value())
    }

    /// Validate and return a session, or None if invalid/expired
    pub fn validate(&self, session_id: &str) -> Option<HumanSession> {
        let session = self.sessions.get(session_id)?;
        if session.is_expired() {
            // Remove expired session
            drop(session);
            self.remove(session_id);
            return None;
        }
        Some(session.clone())
    }

    /// Record a signing event for a session
    pub fn record_signing(&self, session_id: &str) -> bool {
        if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.record_signing();
            true
        } else {
            false
        }
    }

    /// Remove a session
    pub fn remove(&self, session_id: &str) {
        if let Some((_, session)) = self.sessions.remove(session_id) {
            self.by_human.remove(&session.human_id);
            debug!("Removed session: {}", session_id);
        }
    }

    /// Remove all sessions for a human
    pub fn remove_human(&self, human_id: &str) {
        if let Some((_, session_id)) = self.by_human.remove(human_id) {
            self.sessions.remove(&session_id);
            info!("Removed all sessions for human: {}", human_id);
        }
    }

    /// Get statistics about the session store
    pub fn stats(&self) -> SessionStoreStats {
        let total = self.sessions.len();
        let expired = self.sessions.iter().filter(|s| s.is_expired()).count();

        SessionStoreStats {
            total_sessions: total,
            expired_sessions: expired,
            active_sessions: total - expired,
        }
    }

    /// Clean up expired sessions (called periodically)
    fn maybe_cleanup(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let last = self.last_cleanup.load(std::sync::atomic::Ordering::Relaxed);

        // Cleanup every 5 minutes
        if now - last < 300 {
            return;
        }

        if self.last_cleanup.compare_exchange(
            last,
            now,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::Relaxed,
        ).is_ok() {
            self.cleanup();
        }
    }

    /// Force cleanup of expired sessions
    pub fn cleanup(&self) {
        let expired: Vec<String> = self
            .sessions
            .iter()
            .filter(|s| s.is_expired())
            .map(|s| s.session_id.clone())
            .collect();

        let count = expired.len();
        for session_id in expired {
            self.remove(&session_id);
        }

        if count > 0 {
            info!("Cleaned up {} expired sessions", count);
        }
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new(3600, 5) // 1 hour TTL, max 5 sessions per human
    }
}

/// Session store statistics
#[derive(Debug, Clone, Serialize)]
pub struct SessionStoreStats {
    pub total_sessions: usize,
    pub expired_sessions: usize,
    pub active_sessions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = HumanSession::new(
            "sess_123".to_string(),
            "human_456".to_string(),
            Some("Alice".to_string()),
            "uhCAk...".to_string(),
            3600,
        );

        assert!(!session.is_expired());
        assert_eq!(session.entries_signed, 0);
        assert!(session.remaining_seconds() > 3500);
    }

    #[test]
    fn test_session_store() {
        let store = SessionStore::new(3600, 5);

        let session1 = store.get_or_create("human_1", Some("Alice".to_string()), "uhCAk...");
        let session2 = store.get_or_create("human_1", None, "uhCAk...");

        // Same human should get same session
        assert_eq!(session1.session_id, session2.session_id);

        // Different human gets different session
        let session3 = store.get_or_create("human_2", Some("Bob".to_string()), "uhCAk...");
        assert_ne!(session1.session_id, session3.session_id);
    }

    #[test]
    fn test_session_validation() {
        let store = SessionStore::new(3600, 5);

        let session = store.get_or_create("human_1", None, "uhCAk...");
        assert!(store.validate(&session.session_id).is_some());

        store.remove(&session.session_id);
        assert!(store.validate(&session.session_id).is_none());
    }
}
