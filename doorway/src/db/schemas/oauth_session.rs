//! OAuth Session Schema
//!
//! Stores OAuth authorization codes for the code exchange flow.
//! Authorization codes are short-lived (5 minutes) and single-use.

use bson::{doc, oid::ObjectId, Document};
use chrono::{DateTime, Utc};
use mongodb::options::IndexOptions;
use serde::{Deserialize, Serialize};

use super::metadata::Metadata;
use crate::db::mongo::{IntoIndexes, MutMetadata};

/// Collection name for OAuth sessions
pub const OAUTH_SESSION_COLLECTION: &str = "oauth_sessions";

/// OAuth authorization code session.
///
/// Created when user authorizes an OAuth client (e.g., elohim-app).
/// Used to exchange for access token via POST /auth/token.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OAuthSessionDoc {
    /// MongoDB document ID
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    /// Standard metadata (created_at, updated_at, is_deleted)
    #[serde(default)]
    pub metadata: Metadata,

    /// Authorization code (short random string)
    #[serde(default)]
    pub code: String,

    /// OAuth client ID (e.g., "elohim-app")
    #[serde(default)]
    pub client_id: String,

    /// Redirect URI where code was issued to
    #[serde(default)]
    pub redirect_uri: String,

    /// State parameter for CSRF protection
    #[serde(default)]
    pub state: String,

    /// Requested scope (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,

    /// Human ID from Holochain identity
    #[serde(default)]
    pub human_id: String,

    /// Agent public key from Holochain
    #[serde(default)]
    pub agent_pub_key: String,

    /// User identifier (email/username)
    #[serde(default)]
    pub identifier: String,

    /// When the code expires (5 minutes from creation)
    #[serde(default = "default_expires_at")]
    pub expires_at: DateTime<Utc>,

    /// Whether code has been used (codes are single-use)
    #[serde(default)]
    pub used: bool,
}

fn default_expires_at() -> DateTime<Utc> {
    Utc::now()
}

impl OAuthSessionDoc {
    /// Create a new OAuth session with 5-minute expiry.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        code: String,
        client_id: String,
        redirect_uri: String,
        state: String,
        scope: Option<String>,
        human_id: String,
        agent_pub_key: String,
        identifier: String,
    ) -> Self {
        Self {
            id: None,
            metadata: Metadata::new(),
            code,
            client_id,
            redirect_uri,
            state,
            scope,
            human_id,
            agent_pub_key,
            identifier,
            expires_at: Utc::now() + chrono::Duration::minutes(5),
            used: false,
        }
    }

    /// Check if the authorization code is still valid.
    pub fn is_valid(&self) -> bool {
        !self.used && !self.metadata.is_deleted && Utc::now() < self.expires_at
    }
}

/// Registered OAuth client.
///
/// For now, clients are hardcoded (elohim-app), but this could be
/// extended to support dynamic client registration.
#[derive(Debug, Clone)]
pub struct OAuthClient {
    /// Client ID (e.g., "elohim-app")
    pub client_id: String,

    /// Display name for consent screen
    pub name: String,

    /// Allowed redirect URIs (patterns)
    pub redirect_uri_patterns: Vec<String>,

    /// Whether this client is trusted (skip consent screen)
    pub trusted: bool,
}

/// Get registered OAuth clients.
///
/// Currently hardcoded; could be moved to database in future.
pub fn get_registered_clients() -> Vec<OAuthClient> {
    vec![
        OAuthClient {
            client_id: "elohim-app".to_string(),
            name: "Elohim App".to_string(),
            redirect_uri_patterns: vec![
                // Local development
                "http://localhost:*".to_string(),
                "http://127.0.0.1:*".to_string(),
                // Deployed environments
                "https://*.elohim.host/*".to_string(),
                "https://elohim.host/*".to_string(),
                // Eclipse Che workspaces
                "https://*.ethosengine.com/*".to_string(),
            ],
            trusted: true, // Skip consent screen for first-party app
        },
        OAuthClient {
            client_id: "doorway-app".to_string(),
            name: "Doorway Operator Dashboard".to_string(),
            redirect_uri_patterns: vec![
                // Same-origin (no cross-origin needed)
                "/threshold/*".to_string(),
            ],
            trusted: true,
        },
    ]
}

/// Validate that a redirect URI matches allowed patterns for a client.
pub fn validate_redirect_uri(client: &OAuthClient, redirect_uri: &str) -> bool {
    for pattern in &client.redirect_uri_patterns {
        if matches_uri_pattern(pattern, redirect_uri) {
            return true;
        }
    }
    false
}

/// Simple wildcard pattern matching for URIs.
fn matches_uri_pattern(pattern: &str, uri: &str) -> bool {
    // Handle exact match
    if pattern == uri {
        return true;
    }

    // Handle relative paths (e.g., "/threshold/*")
    if pattern.starts_with('/') && uri.starts_with('/') {
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let uri_parts: Vec<&str> = uri.split('/').collect();

        for (i, p) in pattern_parts.iter().enumerate() {
            if *p == "*" {
                return true; // Wildcard matches rest
            }
            if i >= uri_parts.len() || *p != uri_parts[i] {
                return false;
            }
        }
        return pattern_parts.len() == uri_parts.len();
    }

    // Handle URL patterns with wildcards
    // Convert pattern to regex-like matching
    let pattern_parts: Vec<&str> = pattern.split('*').collect();

    if pattern_parts.len() == 1 {
        // No wildcard, exact match only
        return pattern == uri;
    }

    // Check prefix
    if !uri.starts_with(pattern_parts[0]) {
        return false;
    }

    // Check suffix if present
    if pattern_parts.len() > 1 {
        let suffix = pattern_parts.last().unwrap();
        if !suffix.is_empty() && !uri.ends_with(suffix) {
            return false;
        }
    }

    true
}

impl IntoIndexes for OAuthSessionDoc {
    fn into_indices() -> Vec<(Document, Option<IndexOptions>)> {
        vec![
            // Unique index on authorization code
            (
                doc! { "code": 1 },
                Some(
                    IndexOptions::builder()
                        .unique(true)
                        .name("code_unique".to_string())
                        .build(),
                ),
            ),
            // TTL index for automatic expiration cleanup
            (
                doc! { "expires_at": 1 },
                Some(
                    IndexOptions::builder()
                        .expire_after(std::time::Duration::from_secs(0))
                        .name("expires_at_ttl".to_string())
                        .build(),
                ),
            ),
            // Index on client_id for lookups
            (
                doc! { "client_id": 1 },
                Some(
                    IndexOptions::builder()
                        .name("client_id_index".to_string())
                        .build(),
                ),
            ),
        ]
    }
}

impl MutMetadata for OAuthSessionDoc {
    fn mut_metadata(&mut self) -> &mut Metadata {
        &mut self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uri_pattern_matching() {
        assert!(matches_uri_pattern(
            "http://localhost:*",
            "http://localhost:4200"
        ));
        assert!(matches_uri_pattern(
            "http://localhost:*",
            "http://localhost:4200/callback"
        ));
        assert!(matches_uri_pattern(
            "https://*.elohim.host/*",
            "https://app.elohim.host/callback"
        ));
        assert!(matches_uri_pattern("/threshold/*", "/threshold/callback"));
        assert!(!matches_uri_pattern(
            "http://localhost:*",
            "http://example.com:4200"
        ));
    }

    #[test]
    fn test_session_validity() {
        let session = OAuthSessionDoc::new(
            "code123".to_string(),
            "elohim-app".to_string(),
            "http://localhost:4200/callback".to_string(),
            "state123".to_string(),
            None,
            "human-123".to_string(),
            "uhCAk-123".to_string(),
            "user@example.com".to_string(),
        );

        assert!(session.is_valid());
    }
}
