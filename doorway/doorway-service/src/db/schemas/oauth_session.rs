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
/// Scheme / authority / path split of an absolute URI or URI pattern.
struct UriParts<'a> {
    scheme: &'a str,
    host: &'a str,
    port: Option<&'a str>,
    path: &'a str,
}

/// Split `scheme://host[:port][/path...]`. Returns `None` for anything that is
/// not an absolute URI, which the caller treats as "no match" rather than
/// falling back to substring comparison.
fn split_uri(raw: &str) -> Option<UriParts<'_>> {
    let (scheme, rest) = raw.split_once("://")?;
    if scheme.is_empty() || rest.is_empty() {
        return None;
    }
    // The authority ends at the first '/', '?' or '#'. Anything after is path.
    let auth_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, path) = rest.split_at(auth_end);
    if authority.is_empty() {
        return None;
    }
    // Reject userinfo: `https://elohim.host@evil.tld/` must never read as the
    // elohim.host authority.
    if authority.contains('@') {
        return None;
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (h, Some(p)),
        None => (authority, None),
    };
    if host.is_empty() {
        return None;
    }
    Some(UriParts {
        scheme,
        host,
        port,
        path,
    })
}

/// Glob match where `*` may span any character, anchored at both ends.
///
/// Used for the PATH component only. Every literal between wildcards must be
/// present, in order — the defect this replaces compared only the text before
/// the first `*` and after the last one.
fn glob_match(pattern: &str, value: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == value;
    }
    let mut rest = value;
    // First literal is anchored at the start.
    match rest.strip_prefix(parts[0]) {
        Some(r) => rest = r,
        None => return false,
    }
    // Last literal is anchored at the end (empty when the pattern ends in `*`).
    let last = parts[parts.len() - 1];
    if !last.is_empty() {
        if rest.len() < last.len() || !rest.ends_with(last) {
            return false;
        }
        rest = &rest[..rest.len() - last.len()];
    }
    // Interior literals must appear in order, consuming left to right.
    for lit in &parts[1..parts.len() - 1] {
        match rest.find(lit) {
            Some(i) => rest = &rest[i + lit.len()..],
            None => return false,
        }
    }
    true
}

/// Match a registered `redirect_uri` pattern against a requested `redirect_uri`.
///
/// Redirect-URI safety is a property of scheme + host + port, not of string
/// shape, so the authority is compared structurally and only the path is
/// glob-matched. A host wildcard is a single leading `*.` label wildcard and can
/// never span a `/` — an attacker cannot park the expected authority in their
/// own path (`https://evil.tld/.elohim.host/cb`).
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

    // A relative pattern never admits an absolute URI, and vice versa.
    let (pat, req) = match (split_uri(pattern), split_uri(uri)) {
        (Some(p), Some(u)) => (p, u),
        _ => return false,
    };

    // Scheme: exact, case-insensitive. No wildcards — an attacker must not be
    // able to downgrade https to http.
    if !pat.scheme.eq_ignore_ascii_case(req.scheme) {
        return false;
    }

    // Host: either a literal (exact, case-insensitive) or a single leading
    // `*.` label wildcard. The wildcard part is taken from the parsed host, so
    // it cannot contain `/`, `:`, `@` or a path.
    let host_ok = if let Some(suffix) = pat.host.strip_prefix('*') {
        // `*.elohim.host` -> the request host must END WITH `.elohim.host` and
        // carry at least one label in front of it.
        !suffix.is_empty()
            && req.host.len() > suffix.len()
            && req.host[req.host.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
    } else {
        pat.host.eq_ignore_ascii_case(req.host)
    };
    if !host_ok {
        return false;
    }

    // Port: `*` admits any port; otherwise it must match exactly, and an absent
    // pattern port requires an absent request port.
    let port_ok = match (pat.port, req.port) {
        (Some("*"), _) => true,
        (a, b) => a == b,
    };
    if !port_ok {
        return false;
    }

    // Path: an empty pattern path admits any path (the registered
    // `http://localhost:*` shape); otherwise glob-match it.
    pat.path.is_empty() || glob_match(pat.path, req.path)
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

    /// A wildcard-suffixed pattern must not degenerate into "any URI sharing the
    /// scheme".
    ///
    /// Regression for the authorization-code interception proved on the local mesh
    /// 2026-08-28: `matches_uri_pattern` compared only `pattern_parts[0]` (the text
    /// before the FIRST `*`) as a prefix and `pattern_parts.last()` as a suffix. For
    /// `https://*.elohim.host/*` the last part is empty, so the suffix check was
    /// skipped and the literal `.elohim.host/` in the middle was never compared at
    /// all — the pattern accepted every `https://` URI. `GET /auth/authorize` then
    /// issued a real authorization code to an attacker-controlled `redirect_uri`,
    /// which exchanges at `POST /auth/token` for a full access token.
    #[test]
    fn wildcard_pattern_does_not_accept_a_foreign_host() {
        for hostile in [
            "https://attacker.tld/steal",
            "https://a.b.c.evil.co.uk/x?q=1",
            "https://elohim.host.evil.tld/cb",
            "https://evil.tld/?next=.elohim.host/",
        ] {
            assert!(
                !matches_uri_pattern("https://*.elohim.host/*", hostile),
                "hostile redirect_uri accepted by wildcard pattern: {hostile}"
            );
        }
    }

    /// The middle literal of a multi-wildcard pattern is load-bearing: a `*` in the
    /// ORIGIN must never match across a `/`, or an attacker parks the expected
    /// authority in their own path.
    #[test]
    fn origin_wildcard_does_not_span_a_path_separator() {
        assert!(!matches_uri_pattern(
            "https://*.elohim.host/*",
            "https://evil.tld/.elohim.host/cb"
        ));
    }

    /// The legitimate shapes these patterns exist to admit must keep matching.
    #[test]
    fn legitimate_redirect_uris_still_match() {
        assert!(matches_uri_pattern(
            "https://*.elohim.host/*",
            "https://app.elohim.host/callback"
        ));
        // a wildcard in the PATH may span separators
        assert!(matches_uri_pattern(
            "https://*.elohim.host/*",
            "https://doorway-alpha.elohim.host/auth/cb?state=x"
        ));
        assert!(matches_uri_pattern(
            "https://elohim.host/*",
            "https://elohim.host/cb"
        ));
        assert!(matches_uri_pattern(
            "http://localhost:*",
            "http://localhost:4200/callback"
        ));
        assert!(matches_uri_pattern(
            "https://*.ethosengine.com/*",
            "https://ws-7f2.ethosengine.com/oauth/cb"
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
