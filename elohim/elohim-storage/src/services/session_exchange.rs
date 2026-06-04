//! Steward-side portal session exchange (doorway → steward portal-handoff, GAP-2b).
//!
//! When a human logs in at a doorway login portal and that doorway recognizes
//! them as a graduated steward with a reachable portal host, the doorway
//! redirects the browser to the steward's portal-host origin carrying a
//! single-use redemption token. The browser hits `POST /session/exchange` on
//! storage; storage redeems the token back-channel against the *issuing*
//! doorway and, on success, seeds a `LocalSession`.
//!
//! ## Source of truth
//!
//! The redemption token is **Operational** (P2P design-gate Category C):
//! single-use, ~5-minute TTL, issuer-side truth. Storage never persists the
//! token — it exchanges it for an identity snapshot exactly once. The
//! conductor's `/auth/me` remains the authority over doorway claims; this
//! exchange only SEEDS a `LocalSession`, nothing more.
//!
//! ## Anti-spoofing boundary (TOFU allowlist)
//!
//! The `doorwayUrl` in the request is attacker-controllable. We MUST NOT
//! redeem against an issuer named only by the inbound request — that would let
//! any origin point the steward's node at a malicious "doorway" and mint a
//! session. Instead, the doorway URL must already appear (origin-normalized)
//! in this steward's prior `local_sessions` history. Those rows are the
//! steward's trust-on-first-use record: graduation seeded the first session
//! from a doorway the steward chose, and every doorway the steward has ever
//! authenticated through names itself there. Redemption is only ever attempted
//! against an already-trusted issuer.

use crate::db::local_sessions::CreateLocalSessionInput;
use crate::db::models::LocalSession;

/// Identity snapshot returned by a doorway's `GET /auth/exchange-session`
/// endpoint on a successful (HTTP 200) redemption.
///
/// Mirrors the doorway-side `ExchangeSessionResponse` wire shape (camelCase).
/// Only the identity-bearing fields are captured here; anything not needed to
/// seed a `LocalSession` is ignored — notably the fresh doorway JWT (`token`)
/// and `expiresAt`/`portalHostUrl`, which serde simply drops. The local
/// conductor remains the session authority; the doorway JWT is never stored.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RedeemSnapshot {
    pub human_id: String,
    pub agent_pub_key: String,
    pub identifier: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub doorway_id: Option<String>,
    /// The doorway URL the issuer reports for itself. Informational only — the
    /// session is always stamped with the *validated* (allowlisted) URL the
    /// steward already trusts, never with a value the redeem response asserts.
    #[serde(default)]
    pub doorway_url: Option<String>,
    #[serde(default)]
    pub permission_level: Option<String>,
}

/// Outcome of a back-channel redeem call.
#[derive(Debug)]
pub enum RedeemOutcome {
    /// Doorway returned 200 with a valid identity snapshot.
    Ok(Box<RedeemSnapshot>),
    /// Doorway returned 401 / a non-200 status, or the body did not parse —
    /// the token is invalid/expired/already-redeemed. Maps to a 401 to the
    /// browser.
    Invalid,
    /// Network error reaching the doorway (DNS, connect, timeout). The issuer
    /// is allowlisted and trusted but unreachable right now. Maps to a 502 to
    /// the browser (upstream failure), distinct from an invalid token.
    Upstream,
}

/// Normalize a URL to its origin (scheme://host[:port]) for allowlist
/// comparison. Strips path, query, fragment, trailing slash, and lowercases
/// scheme+host so that `https://alpha.elohim.host/`, `https://alpha.elohim.host`,
/// and `https://Alpha.Elohim.Host/auth/foo` all compare equal.
///
/// Falls back to a trimmed, trailing-slash-stripped, lowercased copy of the
/// input when the string does not parse as an absolute URL (so a stored bare
/// origin still matches a bare-origin request).
pub fn normalize_origin(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Ok(url) = url::Url::parse(trimmed) {
        // url::Url::origin() yields the ASCII serialization for special
        // schemes (http/https) — exactly scheme://host[:port] with no path.
        let origin = url.origin();
        if origin.is_tuple() {
            return origin.ascii_serialization();
        }
    }
    trimmed.trim_end_matches('/').to_lowercase()
}

/// TOFU allowlist check: does `doorway_url` (origin-normalized) match the
/// origin of any doorway URL the steward has previously authenticated through?
///
/// `known_doorway_urls` is the set of `doorway_url` values across all rows in
/// `local_sessions` (active and inactive — the full history is the trust
/// record). Returns `true` iff the request's issuer origin is already trusted.
pub fn is_doorway_allowlisted<'a, I>(doorway_url: &str, known_doorway_urls: I) -> bool
where
    I: IntoIterator<Item = &'a str>,
{
    let target = normalize_origin(doorway_url);
    if target.is_empty() {
        return false;
    }
    known_doorway_urls
        .into_iter()
        .any(|known| normalize_origin(known) == target)
}

/// Build the `CreateLocalSessionInput` for the new session from a redeemed
/// identity snapshot plus the *validated* doorway URL.
///
/// The session is always stamped with `validated_doorway_url` — the value that
/// passed the allowlist — never with whatever `snapshot.doorway_url` asserts.
/// `id` is left `None` so `create_session` mints a fresh UUID, and the session
/// becomes the sole active session (priors deactivated) just like `POST
/// /session`.
pub fn build_create_input(
    snapshot: &RedeemSnapshot,
    validated_doorway_url: &str,
) -> CreateLocalSessionInput {
    CreateLocalSessionInput {
        id: None,
        human_id: snapshot.human_id.clone(),
        agent_pub_key: snapshot.agent_pub_key.clone(),
        doorway_url: validated_doorway_url.to_string(),
        doorway_id: snapshot.doorway_id.clone(),
        identifier: snapshot.identifier.clone(),
        display_name: snapshot.display_name.clone(),
        profile_image_hash: None,
        bootstrap_url: None,
    }
}

/// Collect the distinct set of doorway URLs the steward has authenticated
/// through, from the full `local_sessions` history. The returned vector is the
/// TOFU allowlist input for [`is_doorway_allowlisted`].
pub fn known_doorway_urls(sessions: &[LocalSession]) -> Vec<String> {
    sessions
        .iter()
        .map(|s| s.doorway_url.clone())
        .collect::<Vec<_>>()
}

/// Redeem a session token back-channel against the issuing doorway.
///
/// Calls `GET {doorway_url}/auth/exchange-session?session_token=...` — the
/// doorway's EXISTING single-use session-transfer redeem (the same endpoint
/// HandoffService/account-guard already consume; the portal-handoff
/// deliberately reuses it rather than minting a parallel mechanism). Reuses
/// the same reqwest pattern storage uses for other outbound probes/callbacks
/// (rustls, ~5s timeout — see
/// `views_convert::infrastructure::load_render_capability_from_url`).
///
/// - HTTP 200 with a parseable [`RedeemSnapshot`] → [`RedeemOutcome::Ok`].
/// - Any non-200 status, or a 200 body that fails to parse → [`RedeemOutcome::Invalid`].
/// - Transport error (connect/timeout/DNS) → [`RedeemOutcome::Upstream`].
///
/// `validated_doorway_url` is the allowlisted origin/URL — the caller has
/// already confirmed it is trusted before invoking this.
pub async fn redeem_token_http(validated_doorway_url: &str, session_token: &str) -> RedeemOutcome {
    let endpoint = format!(
        "{}/auth/exchange-session",
        validated_doorway_url.trim_end_matches('/')
    );
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "failed to build redeem HTTP client");
            return RedeemOutcome::Upstream;
        }
    };

    let resp = match client
        .get(&endpoint)
        .query(&[("session_token", session_token)])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(endpoint = %endpoint, error = %e, "doorway redeem unreachable");
            return RedeemOutcome::Upstream;
        }
    };

    if resp.status() != reqwest::StatusCode::OK {
        tracing::info!(
            endpoint = %endpoint,
            status = %resp.status(),
            "doorway redeem returned non-200 — token invalid"
        );
        return RedeemOutcome::Invalid;
    }

    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(endpoint = %endpoint, error = %e, "failed to read redeem body");
            return RedeemOutcome::Upstream;
        }
    };

    match serde_json::from_slice::<RedeemSnapshot>(&bytes) {
        Ok(snapshot) => RedeemOutcome::Ok(Box::new(snapshot)),
        Err(e) => {
            tracing::warn!(endpoint = %endpoint, error = %e, "redeem 200 body did not parse");
            RedeemOutcome::Invalid
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> RedeemSnapshot {
        RedeemSnapshot {
            human_id: "human-123".into(),
            agent_pub_key: "uhCAk_test".into(),
            identifier: "matthew@alpha.elohim.host".into(),
            display_name: Some("Matthew".into()),
            doorway_id: Some("doorway-alpha".into()),
            doorway_url: Some("https://alpha.elohim.host".into()),
            permission_level: Some("standard".into()),
        }
    }

    #[test]
    fn normalize_origin_strips_path_query_and_trailing_slash() {
        assert_eq!(
            normalize_origin("https://alpha.elohim.host/auth/foo?x=1#frag"),
            "https://alpha.elohim.host"
        );
        assert_eq!(
            normalize_origin("https://alpha.elohim.host/"),
            "https://alpha.elohim.host"
        );
        assert_eq!(
            normalize_origin("https://alpha.elohim.host"),
            "https://alpha.elohim.host"
        );
    }

    #[test]
    fn normalize_origin_is_case_insensitive_on_host() {
        assert_eq!(
            normalize_origin("https://Alpha.Elohim.Host/path"),
            normalize_origin("https://alpha.elohim.host"),
        );
    }

    #[test]
    fn normalize_origin_distinguishes_port_and_scheme() {
        assert_ne!(
            normalize_origin("https://alpha.elohim.host"),
            normalize_origin("https://alpha.elohim.host:8443"),
        );
        assert_ne!(
            normalize_origin("https://alpha.elohim.host"),
            normalize_origin("http://alpha.elohim.host"),
        );
    }

    #[test]
    fn allowlist_matches_known_issuer_origin() {
        let known = ["https://alpha.elohim.host/some/old/path"];
        assert!(is_doorway_allowlisted(
            "https://alpha.elohim.host/auth/handoff",
            known.iter().copied()
        ));
    }

    #[test]
    fn allowlist_rejects_unknown_issuer() {
        let known = ["https://alpha.elohim.host"];
        assert!(!is_doorway_allowlisted(
            "https://evil.example.com",
            known.iter().copied()
        ));
    }

    #[test]
    fn allowlist_rejects_when_history_empty() {
        let known: Vec<&str> = Vec::new();
        assert!(!is_doorway_allowlisted("https://alpha.elohim.host", known));
    }

    #[test]
    fn allowlist_rejects_empty_doorway_url() {
        let known = ["https://alpha.elohim.host"];
        assert!(!is_doorway_allowlisted("", known.iter().copied()));
        assert!(!is_doorway_allowlisted("   ", known.iter().copied()));
    }

    #[test]
    fn build_input_stamps_validated_url_not_snapshot_url() {
        let mut snap = snapshot();
        // Snapshot claims a *different* doorway than the validated one.
        snap.doorway_url = Some("https://evil.example.com".into());
        let input = build_create_input(&snap, "https://alpha.elohim.host");
        // Session must carry the validated/trusted URL, never the asserted one.
        assert_eq!(input.doorway_url, "https://alpha.elohim.host");
        assert_eq!(input.human_id, "human-123");
        assert_eq!(input.agent_pub_key, "uhCAk_test");
        assert_eq!(input.identifier, "matthew@alpha.elohim.host");
        assert_eq!(input.display_name.as_deref(), Some("Matthew"));
        assert_eq!(input.doorway_id.as_deref(), Some("doorway-alpha"));
        // id is None so create_session mints a fresh UUID.
        assert!(input.id.is_none());
    }

    #[test]
    fn redeem_snapshot_deserializes_minimal_camelcase_body() {
        // Only the three required fields present; optionals default to None.
        let body = r#"{
            "humanId": "h1",
            "agentPubKey": "k1",
            "identifier": "u@d"
        }"#;
        let snap: RedeemSnapshot = serde_json::from_str(body).unwrap();
        assert_eq!(snap.human_id, "h1");
        assert_eq!(snap.agent_pub_key, "k1");
        assert_eq!(snap.identifier, "u@d");
        assert!(snap.display_name.is_none());
        assert!(snap.permission_level.is_none());
    }

    #[test]
    fn redeem_snapshot_parses_real_exchange_session_response() {
        // The doorway's actual ExchangeSessionResponse wire shape
        // (auth_routes.rs::ExchangeSessionResponse) carries a fresh JWT plus
        // expiry and the portal-host hint. serde must DROP those — the local
        // conductor stays authority; the doorway JWT is never captured.
        let body = r#"{
            "token": "eyJhbGciOiJIUzI1NiJ9.fresh-doorway-jwt",
            "humanId": "human-matthew",
            "agentPubKey": "uhCAk-matthew",
            "identifier": "matthew@alpha.elohim.host",
            "expiresAt": 1780000000,
            "doorwayId": "alpha-elohim-host",
            "doorwayUrl": "https://alpha.elohim.host",
            "portalHostUrl": "https://matthew.steward.example"
        }"#;
        let snap: RedeemSnapshot = serde_json::from_str(body).unwrap();
        assert_eq!(snap.human_id, "human-matthew");
        assert_eq!(snap.agent_pub_key, "uhCAk-matthew");
        assert_eq!(snap.identifier, "matthew@alpha.elohim.host");
        assert_eq!(snap.doorway_id.as_deref(), Some("alpha-elohim-host"));
        assert_eq!(
            snap.doorway_url.as_deref(),
            Some("https://alpha.elohim.host")
        );
        // ExchangeSessionResponse has no displayName/permissionLevel — both None.
        assert!(snap.display_name.is_none());
        assert!(snap.permission_level.is_none());
    }
}
