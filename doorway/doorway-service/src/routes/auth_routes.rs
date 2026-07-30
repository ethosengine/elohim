//! HTTP Routes for Authentication
//!
//! Provides REST API endpoints for hosted human authentication:
//! - POST /auth/register - Create credentials after Holochain registration
//! - POST /auth/login    - Authenticate and get JWT token
//! - POST /auth/logout   - Invalidate token (optional, client-side mainly)
//! - POST /auth/refresh  - Refresh an expiring token
//! - GET  /auth/me       - Get current user info from token
//!
//! Ported from admin-proxy/src/auth-routes.ts

use bson::doc;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::auth::{
    extract_token_from_header, hash_password, verify_password, Claims, JwtValidator,
    PermissionLevel, TokenInput,
};
use crate::conductor::AgentProvisioner;
use crate::custodial_keys::{CustodialKeyService, KeyExportFormat};
use crate::db::schemas::{
    get_registered_clients, validate_redirect_uri, OAuthSessionDoc, UserDoc,
    OAUTH_SESSION_COLLECTION, USER_COLLECTION,
};
use crate::routes::zome_helpers::{
    call_create_human, call_create_human_on_conductor, call_get_my_human, get_agent_pub_key,
    CreateHumanInput,
};
use crate::server::AppState;
use crate::types::DoorwayError;
use rand::Rng;

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

// =============================================================================
// Session Transfer Store (cross-app session handoff)
// =============================================================================

/// Data stored for each pending session transfer token.
struct SessionTransferEntry {
    human_id: String,
    agent_pub_key: String,
    identifier: String,
    permission_level: crate::auth::PermissionLevel,
    session_id: Option<String>,
    conductor_id: Option<String>,
    installed_app_id: Option<String>,
    is_steward: bool,
    has_local_conductor: bool,
    doorway_id: Option<String>,
    doorway_url: Option<String>,
    expires_at: Instant,
    consumed: bool,
}

/// Module-level singleton for the session transfer store.
/// Short-lived (60 s TTL) single-use tokens — no persistence needed.
fn session_transfer_store() -> &'static Arc<RwLock<HashMap<String, SessionTransferEntry>>> {
    static STORE: OnceLock<Arc<RwLock<HashMap<String, SessionTransferEntry>>>> = OnceLock::new();
    STORE.get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
}

// =============================================================================
// Session Transfer Response Types
// =============================================================================

/// Response for GET /auth/session-token
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTokenResponse {
    /// Short-lived single-use transfer token (60 s TTL)
    pub session_token: String,
    /// Unix timestamp when the token expires
    pub expires_at: u64,
}

/// Response for GET /auth/exchange-session
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeSessionResponse {
    pub token: String,
    pub human_id: String,
    pub agent_pub_key: String,
    pub identifier: String,
    pub expires_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doorway_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doorway_url: Option<String>,
    /// First reachable portal host URL for this human, when `is_steward` is
    /// true and at least one registered host responds to a health probe.
    /// Omitted when not a steward or when no host is reachable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portal_host_url: Option<String>,
}

// =============================================================================
// Request/Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    /// Holochain human ID (optional for doorway-hosted registration)
    #[serde(default)]
    pub human_id: String,
    /// Holochain agent public key (optional for doorway-hosted registration)
    #[serde(default)]
    pub agent_pub_key: String,
    pub identifier: String,
    pub password: String,
    #[serde(default = "default_identifier_type")]
    pub identifier_type: String,
    // === Profile fields for doorway-hosted registration ===
    /// Display name for doorway-hosted registration (used to create identity)
    #[serde(default)]
    pub display_name: String,
    /// Optional bio/description
    #[serde(default)]
    pub bio: Option<String>,
    /// User interests/affinities
    #[serde(default)]
    pub affinities: Vec<String>,
    /// Profile visibility (public, connections, private)
    #[serde(default = "default_profile_reach")]
    pub profile_reach: String,
    /// Optional location
    #[serde(default)]
    pub location: Option<String>,
    /// Bootstrap key to grant Admin permission on registration.
    /// Must match the API_KEY_ADMIN environment variable.
    #[serde(default)]
    pub admin_bootstrap_key: Option<String>,
    /// Agency phase for graduated stewardship: doorway, node, device, hosted, visitor.
    /// Determines registration flow — whether doorway creates identity or just DB record.
    #[serde(default)]
    pub agency_phase: Option<String>,
}

fn default_profile_reach() -> String {
    "public".to_string()
}

fn default_identifier_type() -> String {
    "email".to_string()
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub identifier: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub token: String,
    pub human_id: String,
    pub agent_pub_key: String,
    pub identifier: String,
    pub expires_at: u64,
    /// Doorway that issued this token (for federation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doorway_id: Option<String>,
    /// Doorway URL for cross-doorway validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doorway_url: Option<String>,
    /// Holochain installed app ID for this user (multi-conductor routing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_app_id: Option<String>,
    /// Human profile (returned on registration)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<HumanProfileResponse>,
    /// True when the authenticated human's stewardship is confirmed at the
    /// substrate level (`UserDoc.is_steward`). Mirrors the claim embedded
    /// in the JWT; surfaced explicitly so the client doesn't have to decode
    /// the JWT just to choose between the hosted-visitor and hosted-steward
    /// surfaces.
    ///
    /// Always serialized (both `true` and `false`). A hosted visitor's client
    /// must be able to read `isSteward: false` to select the visitor surface;
    /// an omitted field reads as `undefined`, indistinguishable from an old
    /// doorway that never emitted the claim. The steward-login portal-handoff
    /// a2o (`isSteward: false` for a hosted visitor) pins this.
    pub is_steward: bool,
    /// First reachable portal host URL for this human, when `is_steward` is
    /// true and at least one registered host responds to a health probe.
    /// The client uses this to redirect the steward to their peer-native
    /// OAuth portal — doorway is the relying party, the portal host is the
    /// identity provider.
    /// Omitted when not a steward or when no host is reachable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portal_host_url: Option<String>,
}

/// Human profile response (from imagodei zome)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HumanProfileResponse {
    pub id: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    pub affinities: Vec<String>,
    pub profile_reach: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Human-readable authority reference surfaced by the portal trust-indicator chrome.
///
/// For `doorway-host` mode: label is the doorway hostname (e.g. "alpha.elohim.host");
/// id is the doorway_id slug.
/// For `peer-conductor` mode (deferred to Task A4): label and id describe the
/// peer's conductor location. Mode B activation comes from elohim-storage's /auth/me
/// projection; this doorway endpoint always returns `doorway-host`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorityRef {
    /// Human-readable label rendered by the trust-indicator chip.
    pub label: String,
    /// Optional stable identifier (doorway_id slug or conductor peer_id).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    pub human_id: String,
    pub agent_pub_key: String,
    pub identifier: String,
    pub permission_level: String,
    /// Doorway that issued this token (for federation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doorway_id: Option<String>,
    /// Doorway URL for cross-doorway validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doorway_url: Option<String>,
    /// Whether the caller has a valid session. Always `true` on the 200 path;
    /// the unauthenticated path returns 401 (no MeResponse body).
    pub authenticated: bool,
    /// Auth mode for the current session.
    ///
    /// - `"doorway-host"` — doorway runs the conductor (flywheel; eviction-capable;
    ///   default for hosted accounts).
    /// - `"peer-conductor"` — conductor lives on a peer-managed storage instance
    ///   or Tauri-local device; doorway is at most transparent ingress.
    ///
    /// MVP: always `"doorway-host"`. Mode B detection is deferred to Task A4
    /// (elohim-storage /auth/me projection) per spec §6.2.
    pub trust_mode: String,
    /// Human-readable authority reference surfaced by the portal trust-indicator.
    /// Derived from the issuing doorway's URL + id for `doorway-host` mode.
    pub authority: AuthorityRef,
    /// Conductor's reachable URL or peer-id descriptor when `trust_mode` is
    /// `"peer-conductor"`. Null in MVP (Mode B substrate deferred).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conductor_endpoint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountResponse {
    pub human_id: String,
    pub identifier: String,
    pub permission_level: String,
    // Storage
    pub storage_bytes: u64,
    pub storage_limit: u64,
    pub storage_percent: f64,
    // Queries
    pub projection_queries: u64,
    pub daily_query_limit: u64,
    pub queries_percent: f64,
    // Bandwidth
    pub bandwidth_bytes: u64,
    pub daily_bandwidth_limit: u64,
    pub bandwidth_percent: f64,
    // Hosting / stewardship
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conductor_id: Option<String>,
    pub is_steward: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stewardship_at: Option<String>,
    pub key_exported: bool,
    // Timestamps
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login_at: Option<String>,
}

// =============================================================================
// OAuth Request/Response Types
// =============================================================================

/// OAuth authorization request query parameters.
#[derive(Debug, Deserialize)]
pub struct OAuthAuthorizeRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub state: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub login_hint: Option<String>,
}

/// OAuth token exchange request body.
#[derive(Debug, Deserialize)]
pub struct OAuthTokenRequest {
    pub grant_type: String,
    pub code: String,
    pub redirect_uri: String,
    pub client_id: String,
}

/// OAuth token response (RFC 6749 compliant).
#[derive(Debug, Serialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Custom: Human ID for Holochain identity
    pub human_id: String,
    /// Custom: Agent public key for Holochain
    pub agent_pub_key: String,
    /// Custom: User identifier
    pub identifier: String,
    /// Custom: Doorway that issued this token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doorway_id: Option<String>,
    /// Custom: Doorway URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doorway_url: Option<String>,
    /// Custom: First reachable portal host URL for this human, when the human
    /// is a confirmed steward and at least one registered host responds to a
    /// health probe. The OAuth-code consumer uses this to hand the session off
    /// to the peer-native OAuth portal — doorway is the relying party, the
    /// portal host is the identity provider. Mirrors `AuthResponse.portalHostUrl`
    /// on the login path so both auth flows expose the same handoff hint.
    ///
    /// Emitted as camelCase `portalHostUrl` to match the login-path wire
    /// contract the elohim-app callback already consumes. The RFC 6749 standard
    /// fields above intentionally stay snake_case; this is additive metadata,
    /// outside the RFC envelope. Omitted entirely when not a steward or when no
    /// host is reachable (probe failure degrades silently to None).
    #[serde(rename = "portalHostUrl")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portal_host_url: Option<String>,
}

/// OAuth error response (RFC 6749 compliant).
#[derive(Debug, Serialize)]
pub struct OAuthErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

// =============================================================================
// Native Handoff Types (Tauri Session Migration)
// =============================================================================

/// Response for native handoff endpoint.
/// Returns identity + network context for Tauri to create a local session.
/// Content syncs via P2P (DHT gossip) once the native conductor joins the network.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeHandoffResponse {
    /// Holochain Human ID
    pub human_id: String,
    /// User identifier (email/username)
    pub identifier: String,
    /// Conductor-generated agent public key (base64)
    pub agent_pub_key: String,
    /// Doorway ID that issued this session
    pub doorway_id: String,
    /// Doorway URL for future recovery
    pub doorway_url: String,
    /// Display name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Profile image blob hash (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_image_hash: Option<String>,
    /// Bootstrap URL for P2P discovery (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap_url: Option<String>,
    /// Signal relay URL for WebRTC signaling (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_url: Option<String>,
    /// Custom network seed (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_seed: Option<String>,
    /// Installed app ID on the doorway conductor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_app_id: Option<String>,
    /// Which conductor hosts this user's agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conductor_id: Option<String>,
    /// Encrypted key bundle for identity import (inline, non-destructive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_bundle: Option<KeyExportFormat>,
    /// Whether the user has confirmed stewardship (graduated from custodial)
    pub is_steward: bool,
}

// =============================================================================
// Key Export Types (Stewardship Migration)
// =============================================================================

/// Response containing the encrypted key bundle for migration to Tauri.
/// The private key is still encrypted with the user's password - they must
/// provide it to the Tauri app to decrypt it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyExportResponse {
    /// The exported key bundle
    pub key_bundle: KeyExportFormat,
    /// Instructions for importing to Tauri
    pub instructions: String,
}

/// Request to confirm stewardship migration.
/// Called by Tauri app after successful key import.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmStewardshipRequest {
    /// Signature proving possession of the key (signs the human_id)
    pub signature: String,
}

// =============================================================================
// Recovery Request/Response Types
// =============================================================================

/// Request to initiate disaster recovery for a steward user.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverCustodyRequest {
    /// User identifier (email)
    pub identifier: String,
    /// Recovery method: "social", "elohim_check", or "hint"
    #[serde(default = "default_recovery_method")]
    pub recovery_method: String,
    /// Custom expiry in hours (default 48)
    #[serde(default)]
    pub expires_in_hours: Option<u32>,
}

fn default_recovery_method() -> String {
    "social".to_string()
}

/// Response after initiating recovery.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverCustodyResponse {
    /// Recovery request ID for polling
    pub request_id: String,
    /// Number of approvals required (M)
    pub required_approvals: u32,
    /// When the request expires
    pub expires_at: String,
    /// Current status
    pub status: String,
    /// Instructions for the user
    pub instructions: String,
}

/// Request to check recovery status.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckRecoveryStatusRequest {
    /// Recovery request ID
    pub request_id: String,
}

/// Response with recovery status.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckRecoveryStatusResponse {
    /// Current status: pending, approved, rejected, expired, completed
    pub status: String,
    /// Current approval count
    pub current_approvals: u32,
    /// Required approvals (M)
    pub required_approvals: u32,
    /// Confidence score (0-100)
    pub confidence_score: f64,
    /// Recovery session token (only if approved)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_token: Option<String>,
    /// When the request expires
    pub expires_at: String,
    /// Votes received (for transparency)
    pub votes: Vec<RecoveryVoteInfo>,
}

/// Info about a recovery vote.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryVoteInfo {
    /// Anonymized voter identifier
    pub voter_display: String,
    /// Whether they approved
    pub approved: bool,
    /// Their attestation message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestation: Option<String>,
    /// When they voted
    pub voted_at: String,
}

/// Request to activate recovery after approval.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivateRecoveryRequest {
    /// Recovery request ID
    pub request_id: String,
    /// New password for the recovered account
    pub new_password: String,
}

/// Response after successful recovery activation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivateRecoveryResponse {
    /// JWT token for immediate access
    pub token: String,
    /// Human ID (unchanged)
    pub human_id: String,
    /// New agent public key (from new custodial key)
    pub agent_pub_key: String,
    /// User identifier
    pub identifier: String,
    /// Token expiry
    pub expires_at: u64,
    /// Instructions for the user
    pub instructions: String,
}

// =============================================================================
// Elohim Verification Request/Response Types
// =============================================================================

/// Request to start Elohim verification
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElohimVerifyStartRequest {
    /// Recovery request ID (links to the recovery flow)
    pub request_id: String,
}

/// Response with verification questions
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElohimVerifyStartResponse {
    /// Session ID for this verification attempt
    pub session_id: String,
    /// Questions to answer (no expected answers included)
    pub questions: Vec<crate::services::ClientQuestion>,
    /// Time limit in seconds
    pub time_limit_seconds: u64,
    /// Instructions
    pub instructions: String,
}

/// Request to submit verification answers
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElohimVerifyAnswerRequest {
    /// Session ID from start response
    pub session_id: String,
    /// Answers to questions
    pub answers: Vec<crate::services::QuestionAnswer>,
}

/// Response with verification result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElohimVerifyAnswerResponse {
    /// Whether verification passed
    pub passed: bool,
    /// Accuracy score (0-100)
    pub accuracy_percent: f64,
    /// Confidence contribution (0-60)
    pub confidence_score: f64,
    /// Summary message
    pub summary: String,
    /// Individual question feedback (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback: Option<Vec<QuestionFeedback>>,
}

/// Feedback for a single question
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionFeedback {
    pub question_id: String,
    pub correct: bool,
    pub message: String,
}

/// Response for stewardship confirmation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StewardshipConfirmedResponse {
    pub success: bool,
    pub message: String,
    /// When the user became a steward
    pub stewardship_at: String,
}

// =============================================================================
// Response Helpers
// =============================================================================

fn json_response<T: Serialize>(status: StatusCode, body: &T) -> Response<BoxBody> {
    let json = serde_json::to_string(body).unwrap_or_else(|_| "{}".to_string());

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(full_body(json))
        .unwrap()
}

fn full_body(data: impl Into<Bytes>) -> BoxBody {
    Full::new(data.into())
        .map_err(|never| match never {})
        .boxed()
}

fn empty_body() -> BoxBody {
    Full::new(Bytes::new())
        .map_err(|never| match never {})
        .boxed()
}

async fn parse_json_body<T: for<'de> Deserialize<'de>>(
    req: Request<hyper::body::Incoming>,
) -> Result<T, DoorwayError> {
    let body = req
        .collect()
        .await
        .map_err(|e| DoorwayError::Http(format!("Failed to read body: {e}")))?;

    let bytes = body.to_bytes();
    if bytes.len() > 10240 {
        return Err(DoorwayError::Http("Request body too large".into()));
    }

    serde_json::from_slice(&bytes).map_err(|e| DoorwayError::Http(format!("Invalid JSON: {e}")))
}

fn get_auth_header(req: &Request<hyper::body::Incoming>) -> Option<&str> {
    req.headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
}

// =============================================================================
// Route Handlers
// =============================================================================

/// POST /auth/register
///
/// Derive the doorway's **gateway domain** — the suffix the doorway appends to a bare
/// identifier so login/register are gateway-scoped: you authenticate at THIS doorway's
/// account namespace, never an arbitrary domain (cross-doorway lives behind the
/// "Use a different doorway" link). Sourced from the CONFIGURED `DOORWAY_URL`, never the
/// inbound `Host` header — behind an ingress the Host can be the internal service name
/// or client-controlled, so it must not drive credential resolution.
///
/// `https://doorway-alpha.elohim.host` -> `alpha.elohim.host`. This mirrors the frontend
/// `gatewayDomain()` in `threshold-login.component.ts`, which strips the `doorway-`
/// prefix off `window.location.hostname`. Returns `None` when no URL is configured (e.g.
/// local dev) — callers then leave the identifier untouched.
fn gateway_domain(doorway_url: Option<&str>) -> Option<String> {
    let url = doorway_url?;
    let host = url
        .split_once("://")
        .map_or(url, |(_, rest)| rest)
        .split(['/', ':'])
        .next()
        .unwrap_or("");
    if host.is_empty() {
        return None;
    }
    Some(host.strip_prefix("doorway-").unwrap_or(host).to_string())
}

/// Re-qualify an identifier's local-part with the doorway's gateway domain. Idempotent
/// for an already-own-domain identifier (no double-qualify); converges bare,
/// full-own-domain, and full-foreign-domain inputs all to `localpart@gateway`.
fn normalize_identifier(identifier: &str, gateway_domain: &str) -> String {
    let local = identifier.split('@').next().unwrap_or(identifier);
    format!("{local}@{gateway_domain}")
}

/// Create authentication credentials for an existing Holochain identity.
/// Called after successful register_human zome call.
///
/// Flow:
/// 1. Validate required fields
/// 2. Check if identifier already exists in MongoDB
/// 3. Hash password with argon2
/// 4. Store credentials in MongoDB
/// 5. Generate and return JWT token
async fn handle_register(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    let mut body: RegisterRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid JSON body: {e}"),
                    code: None,
                },
            )
        }
    };

    // Validate required fields (identifier and password always required)
    if body.identifier.is_empty() || body.password.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "Missing required fields: identifier, password".into(),
                code: None,
            },
        );
    }

    // Gateway-scope the identifier: re-qualify its local-part with the doorway's OWN
    // configured domain so a new account is stored canonically (`localpart@gateway`),
    // matching what `handle_login` resolves against. No-op when no domain is configured.
    if let Some(domain) = gateway_domain(state.args.doorway_url.as_deref()) {
        body.identifier = normalize_identifier(&body.identifier, &domain);
    }

    // Determine display name for registration
    let display_name = if body.display_name.is_empty() {
        body.identifier
            .split('@')
            .next()
            .unwrap_or("User")
            .to_string()
    } else {
        body.display_name.clone()
    };

    // Parse agency phase (default to "hosted" for backwards compatibility)
    let agency_phase = body.agency_phase.as_deref().unwrap_or("hosted");

    // Reject visitors — they don't register
    if agency_phase == "visitor" {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "Visitors do not register — use the app without an account".into(),
                code: Some("VISITOR_NO_REGISTER".into()),
            },
        );
    }

    // Branch registration flow by agency phase.
    // Returns (human_id, agent_pub_key, profile, provisioned).
    let (human_id, agent_pub_key, profile, provisioned) = match agency_phase {
        // ── DOORWAY ────────────────────────────────────────────────────────────
        // Operator bootstrap: use the singleton ZomeCaller targeting the
        // operator's conductor. No provisioning step — the doorway IS the
        // conductor for this identity.
        "doorway" => {
            // Prefer a caller-supplied canonical human_id — the same override the
            // node/device branch below already honors — over minting a fresh UUID.
            // Minting an unconditional UUID here is the root cause of a household-
            // formation founder-capture bug: seed-household-formation.ts binds
            // conductor sessions to household members by exact humanId
            // (HOUSEHOLD_MEMBERS' canonical `human-<name>` ids); a doorway-phase
            // registrant whose Human entry got a random UUID id can never bind,
            // and if that registrant is the ceremony's founder, formation FATALs
            // with "no conductor found for the founder". There is no general
            // email→humanId resolution available at this call site (the household
            // roster is dev/genesis fixture data, not runtime state doorway-service
            // holds) — so a canonical id is only resolvable when the caller passes
            // one explicitly. Healing an already-registered account's UUID Human is
            // out of scope here (chain-captured; needs migration/lineage, not a
            // silent overwrite) — this only prevents NEW doorway-phase registrations
            // from recreating the bug when the caller knows its canonical id.
            let generated_human_id = if !body.human_id.is_empty() {
                body.human_id.clone()
            } else {
                warn!(
                    identifier = %body.identifier,
                    "Doorway: registering agencyPhase=doorway with no caller-supplied \
                     human_id — minting a random UUID for this Human. If this registrant \
                     is meant to be a household-formation founder (e.g. matthew), the \
                     resulting conductor identity will never match HOUSEHOLD_MEMBERS' \
                     canonical humanId and seed-household-formation.ts will FATAL with \
                     'no conductor found for the founder' — pass human_id explicitly on \
                     /auth/register for any household/genesis registration."
                );
                uuid::Uuid::new_v4().to_string()
            };

            let zome_result = call_create_human(
                &state,
                CreateHumanInput {
                    id: generated_human_id.clone(),
                    display_name: display_name.clone(),
                    bio: body.bio.clone(),
                    affinities: body.affinities.clone(),
                    profile_reach: body.profile_reach.clone(),
                    location: body.location.clone(),
                },
            )
            .await;

            match zome_result {
                Ok(human_output) => {
                    let agent_key = match get_agent_pub_key(&state) {
                        Ok(k) => k,
                        Err(e) => {
                            warn!("Failed to get agent_pub_key: {}", e);
                            return json_response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                &ErrorResponse {
                                    error: "Failed to get agent identity".into(),
                                    code: Some("AGENT_KEY_ERROR".into()),
                                },
                            );
                        }
                    };
                    info!(
                        "Doorway: created Holochain identity via imagodei zome: {} (display_name={})",
                        human_output.human.id, display_name
                    );
                    let profile = HumanProfileResponse {
                        id: human_output.human.id.clone(),
                        display_name: human_output.human.display_name,
                        bio: human_output.human.bio,
                        affinities: human_output.human.affinities,
                        profile_reach: human_output.human.profile_reach,
                        location: human_output.human.location,
                        created_at: human_output.human.created_at,
                        updated_at: human_output.human.updated_at,
                    };
                    (human_output.human.id, agent_key, Some(profile), None)
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("Agent already has a Human profile") {
                        // Conductor not reset but DB was cleared — recover.
                        warn!(
                            identifier = %body.identifier,
                            "Doorway: agent already has Human profile in DHT — recovering for DB re-registration"
                        );
                        match call_get_my_human(&state).await {
                            Ok(Some(existing)) => {
                                let agent_key = match get_agent_pub_key(&state) {
                                    Ok(k) => k,
                                    Err(e2) => {
                                        warn!(
                                            "Failed to get agent_pub_key during recovery: {}",
                                            e2
                                        );
                                        return json_response(
                                            StatusCode::INTERNAL_SERVER_ERROR,
                                            &ErrorResponse {
                                                error:
                                                    "Failed to get agent identity during recovery"
                                                        .into(),
                                                code: Some("AGENT_KEY_ERROR".into()),
                                            },
                                        );
                                    }
                                };
                                let profile = HumanProfileResponse {
                                    id: existing.human.id.clone(),
                                    display_name: existing.human.display_name,
                                    bio: existing.human.bio,
                                    affinities: existing.human.affinities,
                                    profile_reach: existing.human.profile_reach,
                                    location: existing.human.location,
                                    created_at: existing.human.created_at,
                                    updated_at: existing.human.updated_at,
                                };
                                (existing.human.id, agent_key, Some(profile), None)
                            }
                            Ok(None) => {
                                warn!(
                                    "get_my_human returned None despite 'already has profile' error"
                                );
                                return json_response(
                                    StatusCode::SERVICE_UNAVAILABLE,
                                    &ErrorResponse {
                                        error: format!("Failed to create Holochain identity: {e}"),
                                        code: Some("IDENTITY_CREATION_FAILED".into()),
                                    },
                                );
                            }
                            Err(e2) => {
                                warn!("Failed to recover existing Human profile: {}", e2);
                                return json_response(
                                    StatusCode::SERVICE_UNAVAILABLE,
                                    &ErrorResponse {
                                        error: format!("Failed to create Holochain identity: {e}"),
                                        code: Some("IDENTITY_CREATION_FAILED".into()),
                                    },
                                );
                            }
                        }
                    } else if state.args.dev_mode {
                        warn!(
                            "Doorway: imagodei zome unavailable, using dev fallback: {}",
                            e
                        );
                        use sha2::{Digest, Sha256};
                        let mut hasher = Sha256::new();
                        hasher.update(body.identifier.as_bytes());
                        hasher.update(b"human_id_salt");
                        let hash = hasher.finalize();
                        let human_id = format!("uhCHk{}", hex::encode(&hash[..20]));
                        let mut hasher2 = Sha256::new();
                        hasher2.update(body.identifier.as_bytes());
                        hasher2.update(b"agent_pub_key_salt");
                        let hash2 = hasher2.finalize();
                        let agent_pub_key = format!("uhCAk{}", hex::encode(&hash2[..20]));
                        (human_id, agent_pub_key, None, None)
                    } else {
                        warn!(
                            "Doorway: failed to create identity via imagodei zome: {}",
                            e
                        );
                        return json_response(
                            StatusCode::SERVICE_UNAVAILABLE,
                            &ErrorResponse {
                                error: format!("Failed to create Holochain identity: {e}"),
                                code: Some("IDENTITY_CREATION_FAILED".into()),
                            },
                        );
                    }
                }
            }
        }

        // ── HOSTED ─────────────────────────────────────────────────────────────
        // Standard hosted registration: provision first, then create_human on
        // the provisioned conductor. This is the default flow for new users who
        // are hosted by the doorway operator.
        "hosted" => {
            // Step 1: provision an agent cell on a conductor
            let provisioner_result = if let Some(registry) = &state.conductor_registry {
                if !state.args.dev_mode {
                    let provisioner = AgentProvisioner::new(Arc::clone(registry))
                        .with_app_id(state.args.installed_app_id.clone())
                        .with_bundle_path(state.args.happ_bundle_path.clone());
                    match provisioner.provision_agent(&body.identifier).await {
                        Ok(p) => {
                            info!(
                                conductor = %p.conductor_id,
                                agent = %p.agent_pub_key,
                                "Hosted: agent provisioned on conductor during registration"
                            );
                            Some(p)
                        }
                        Err(e) => {
                            error!(
                                "Hosted: agent provisioning failed — cannot complete registration: {}",
                                e
                            );
                            return json_response(
                                StatusCode::SERVICE_UNAVAILABLE,
                                &ErrorResponse {
                                    error: format!("Agent provisioning failed: {e}"),
                                    code: Some("PROVISIONING_FAILED".into()),
                                },
                            );
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // Step 2: create_human on the provisioned conductor (or fall back to
            // singleton ZomeCaller / dev mode if no conductor available)
            //
            // Same caller-supplied-human_id preference as the "doorway" branch above
            // (see its comment for the household-formation founder-capture rationale) —
            // a hosted-phase household registration is exposed to the identical class
            // of bug if it ever mints an unbindable UUID Human.
            let generated_human_id = if !body.human_id.is_empty() {
                body.human_id.clone()
            } else {
                warn!(
                    identifier = %body.identifier,
                    "Hosted: registering with no caller-supplied human_id — minting a \
                     random UUID for this Human. If this registrant is meant to be a \
                     household-formation founder, the resulting conductor identity will \
                     never match HOUSEHOLD_MEMBERS' canonical humanId and \
                     seed-household-formation.ts will FATAL with 'no conductor found for \
                     the founder' — pass human_id explicitly on /auth/register for any \
                     household/genesis registration."
                );
                uuid::Uuid::new_v4().to_string()
            };

            let zome_result = if let Some(ref p) = provisioner_result {
                call_create_human_on_conductor(
                    &p.conductor_url,
                    &p.installed_app_id,
                    CreateHumanInput {
                        id: generated_human_id.clone(),
                        display_name: display_name.clone(),
                        bio: body.bio.clone(),
                        affinities: body.affinities.clone(),
                        profile_reach: body.profile_reach.clone(),
                        location: body.location.clone(),
                    },
                )
                .await
            } else {
                // No conductor registry (dev mode or not configured) — fall back
                // to singleton ZomeCaller
                call_create_human(
                    &state,
                    CreateHumanInput {
                        id: generated_human_id.clone(),
                        display_name: display_name.clone(),
                        bio: body.bio.clone(),
                        affinities: body.affinities.clone(),
                        profile_reach: body.profile_reach.clone(),
                        location: body.location.clone(),
                    },
                )
                .await
            };

            match zome_result {
                Ok(human_output) => {
                    // Prefer the provisioned agent key; fall back to zome config
                    let agent_key = if let Some(ref p) = provisioner_result {
                        p.agent_pub_key.clone()
                    } else {
                        match get_agent_pub_key(&state) {
                            Ok(k) => k,
                            Err(e) => {
                                warn!("Hosted: failed to get agent_pub_key: {}", e);
                                return json_response(
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    &ErrorResponse {
                                        error: "Failed to get agent identity".into(),
                                        code: Some("AGENT_KEY_ERROR".into()),
                                    },
                                );
                            }
                        }
                    };
                    info!(
                        "Hosted: created Holochain identity: {} (display_name={})",
                        human_output.human.id, display_name
                    );
                    let profile = HumanProfileResponse {
                        id: human_output.human.id.clone(),
                        display_name: human_output.human.display_name,
                        bio: human_output.human.bio,
                        affinities: human_output.human.affinities,
                        profile_reach: human_output.human.profile_reach,
                        location: human_output.human.location,
                        created_at: human_output.human.created_at,
                        updated_at: human_output.human.updated_at,
                    };
                    (
                        human_output.human.id,
                        agent_key,
                        Some(profile),
                        provisioner_result,
                    )
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("Agent already has a Human profile") {
                        // Recover existing identity — conductor wasn't reset but DB was cleared.
                        warn!(
                            identifier = %body.identifier,
                            "Hosted: agent already has Human profile in DHT — recovering for DB re-registration"
                        );
                        // Use a temporary ZomeCaller on the provisioned conductor if available
                        let recovery_result = if let Some(ref p) = provisioner_result {
                            let admin_url = crate::derive_admin_url_from_app(&p.conductor_url);
                            let caller = crate::services::ZomeCaller::new(
                                &admin_url,
                                &p.conductor_url,
                                &p.installed_app_id,
                            );
                            caller
                                .call::<(), Option<crate::routes::zome_helpers::HumanOutput>>(
                                    "imagodei",
                                    "imagodei",
                                    "get_my_human",
                                    &(),
                                )
                                .await
                                .map_err(|e2| {
                                    crate::types::DoorwayError::Holochain(format!(
                                        "get_my_human on conductor failed: {e2}"
                                    ))
                                })
                        } else {
                            call_get_my_human(&state).await
                        };

                        match recovery_result {
                            Ok(Some(existing)) => {
                                let agent_key = if let Some(ref p) = provisioner_result {
                                    p.agent_pub_key.clone()
                                } else {
                                    match get_agent_pub_key(&state) {
                                        Ok(k) => k,
                                        Err(e2) => {
                                            warn!(
                                                "Hosted: failed to get agent_pub_key during recovery: {}",
                                                e2
                                            );
                                            return json_response(
                                                StatusCode::INTERNAL_SERVER_ERROR,
                                                &ErrorResponse {
                                                    error: "Failed to get agent identity during recovery".into(),
                                                    code: Some("AGENT_KEY_ERROR".into()),
                                                },
                                            );
                                        }
                                    }
                                };
                                let profile = HumanProfileResponse {
                                    id: existing.human.id.clone(),
                                    display_name: existing.human.display_name,
                                    bio: existing.human.bio,
                                    affinities: existing.human.affinities,
                                    profile_reach: existing.human.profile_reach,
                                    location: existing.human.location,
                                    created_at: existing.human.created_at,
                                    updated_at: existing.human.updated_at,
                                };
                                (
                                    existing.human.id,
                                    agent_key,
                                    Some(profile),
                                    provisioner_result,
                                )
                            }
                            Ok(None) => {
                                warn!("Hosted: get_my_human returned None despite 'already has profile' error");
                                return json_response(
                                    StatusCode::SERVICE_UNAVAILABLE,
                                    &ErrorResponse {
                                        error: format!("Failed to create Holochain identity: {e}"),
                                        code: Some("IDENTITY_CREATION_FAILED".into()),
                                    },
                                );
                            }
                            Err(e2) => {
                                warn!("Hosted: failed to recover existing Human profile: {}", e2);
                                return json_response(
                                    StatusCode::SERVICE_UNAVAILABLE,
                                    &ErrorResponse {
                                        error: format!("Failed to create Holochain identity: {e}"),
                                        code: Some("IDENTITY_CREATION_FAILED".into()),
                                    },
                                );
                            }
                        }
                    } else if state.args.dev_mode {
                        warn!(
                            "Hosted: imagodei zome unavailable, using dev fallback: {}",
                            e
                        );
                        use sha2::{Digest, Sha256};
                        let mut hasher = Sha256::new();
                        hasher.update(body.identifier.as_bytes());
                        hasher.update(b"human_id_salt");
                        let hash = hasher.finalize();
                        let human_id = format!("uhCHk{}", hex::encode(&hash[..20]));
                        let mut hasher2 = Sha256::new();
                        hasher2.update(body.identifier.as_bytes());
                        hasher2.update(b"agent_pub_key_salt");
                        let hash2 = hasher2.finalize();
                        let agent_pub_key = format!("uhCAk{}", hex::encode(&hash2[..20]));
                        (human_id, agent_pub_key, None, None)
                    } else {
                        warn!("Hosted: failed to create identity via imagodei zome: {}", e);
                        return json_response(
                            StatusCode::SERVICE_UNAVAILABLE,
                            &ErrorResponse {
                                error: format!("Failed to create Holochain identity: {e}"),
                                code: Some("IDENTITY_CREATION_FAILED".into()),
                            },
                        );
                    }
                }
            }
        }

        // ── NODE / DEVICE ───────────────────────────────────────────────────────
        // The human already has an identity on their own conductor.
        // We only create a DB record so doorway can issue JWTs and route traffic.
        // No create_human zome call — their conductor already has it.
        "node" | "device" => {
            // Find (or create) the existing app via provisioner — idempotent.
            let provisioner_result = if let Some(registry) = &state.conductor_registry {
                if !state.args.dev_mode {
                    let provisioner = AgentProvisioner::new(Arc::clone(registry))
                        .with_app_id(state.args.installed_app_id.clone())
                        .with_bundle_path(state.args.happ_bundle_path.clone());
                    match provisioner.provision_agent(&body.identifier).await {
                        Ok(p) => {
                            info!(
                                conductor = %p.conductor_id,
                                agent = %p.agent_pub_key,
                                agency_phase = %agency_phase,
                                "Node/device: found existing agent on conductor"
                            );
                            Some(p)
                        }
                        Err(e) => {
                            // Non-fatal for node/device — they may not be on this operator's conductor
                            warn!("Node/device: conductor lookup failed (non-fatal): {}", e);
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // Use caller-supplied human_id/agent_pub_key if available; otherwise
            // derive deterministic identifiers from the email.
            let resolved_human_id = if !body.human_id.is_empty() {
                body.human_id.clone()
            } else if let Some(ref p) = provisioner_result {
                // Use conductor agent key as deterministic human_id seed
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(p.agent_pub_key.as_bytes());
                hasher.update(b"human_id_salt");
                let hash = hasher.finalize();
                format!("uhCHk{}", hex::encode(&hash[..20]))
            } else {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(body.identifier.as_bytes());
                hasher.update(b"human_id_salt");
                let hash = hasher.finalize();
                format!("uhCHk{}", hex::encode(&hash[..20]))
            };

            let resolved_agent_key = if let Some(ref p) = provisioner_result {
                p.agent_pub_key.clone()
            } else if !body.agent_pub_key.is_empty() {
                body.agent_pub_key.clone()
            } else {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(body.identifier.as_bytes());
                hasher.update(b"agent_pub_key_salt");
                let hash = hasher.finalize();
                format!("uhCAk{}", hex::encode(&hash[..20]))
            };

            info!(
                agency_phase = %agency_phase,
                human_id = %resolved_human_id,
                "Node/device: DB-only registration (no create_human zome call)"
            );

            // No profile returned — identity lives on their own conductor
            (
                resolved_human_id,
                resolved_agent_key,
                None,
                provisioner_result,
            )
        }

        _ => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Unknown agency phase: '{agency_phase}'"),
                    code: Some("INVALID_AGENCY_PHASE".into()),
                },
            );
        }
    };

    // Validate password strength (minimum 8 characters)
    if body.password.len() < 8 {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "Password must be at least 8 characters".into(),
                code: Some("WEAK_PASSWORD".into()),
            },
        );
    }

    // Get JWT validator
    let jwt = match get_jwt_validator(&state) {
        Ok(j) => j,
        Err(resp) => return resp,
    };

    // In dev mode without MongoDB, use simplified flow
    if state.args.dev_mode && state.mongo.is_none() {
        info!("Dev mode register (no MongoDB): {}", body.identifier);
        return generate_auth_response(
            &jwt,
            &state,
            &human_id,
            &agent_pub_key,
            &body.identifier,
            None, // No session_id for registration (key not activated yet)
            StatusCode::CREATED,
            profile,
            PermissionLevel::Authenticated,
            None,
            None, // No conductor_id yet for dev-mode register
            false,
            false,
        )
        .await;
    }

    // Production flow: use MongoDB
    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return json_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &ErrorResponse {
                    error: "Database not available".into(),
                    code: Some("DB_UNAVAILABLE".into()),
                },
            )
        }
    };

    // Get users collection
    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: format!("Database error: {e}"),
                    code: Some("DB_ERROR".into()),
                },
            )
        }
    };

    // Check if identifier already exists
    match collection
        .find_one(doc! { "identifier": &body.identifier })
        .await
    {
        Ok(Some(_)) => {
            return json_response(
                StatusCode::CONFLICT,
                &ErrorResponse {
                    error: "An account with this identifier already exists".into(),
                    code: Some("USER_EXISTS".into()),
                },
            )
        }
        Ok(None) => {} // Good, doesn't exist
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: format!("Database error: {e}"),
                    code: Some("DB_ERROR".into()),
                },
            )
        }
    }

    // Hash password
    let password_hash = match hash_password(&body.password) {
        Ok(h) => h,
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: format!("Failed to hash password: {e}"),
                    code: Some("HASH_ERROR".into()),
                },
            )
        }
    };

    // Generate custodial key material
    let custodial_key_service = CustodialKeyService::new();
    let custodial_key = match custodial_key_service.generate_key_material(&body.password) {
        Ok(key) => key,
        Err(e) => {
            warn!("Failed to generate custodial key: {}", e);
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: "Failed to generate identity key".into(),
                    code: Some("KEY_GEN_ERROR".into()),
                },
            );
        }
    };

    // Use conductor-generated key if provisioned, otherwise use custodial key
    let actual_agent_pub_key = if let Some(ref p) = provisioned {
        p.agent_pub_key.clone()
    } else {
        custodial_key.public_key.clone()
    };

    // Create user document with custodial key
    let mut user = UserDoc::new_with_custodial_key(
        body.identifier.clone(),
        body.identifier_type.clone(),
        password_hash,
        human_id.clone(),
        custodial_key,
    );

    // If agent was provisioned on a conductor, set conductor_id and override agent key
    if let Some(ref p) = provisioned {
        user.set_conductor(p.conductor_id.clone());
        user.agent_pub_key = p.agent_pub_key.clone();
    }

    // Check admin bootstrap key - promote to Admin if key matches API_KEY_ADMIN.
    //
    // When a bootstrap key is *explicitly supplied* it expresses an intent to
    // register as admin; if that intent cannot be honoured (server has no admin
    // key configured, or the supplied key does not match) we must NOT silently
    // fall through to an Authenticated registration — that produced an invisible
    // failure where the operator could not distinguish "wrong key" from "key not
    // honoured" until a later /auth/me permission assertion. Reject explicitly
    // with a machine-readable code. The no-key path is unchanged: ordinary
    // registrations never supply a bootstrap key and are unaffected.
    if let Some(ref bootstrap_key) = body.admin_bootstrap_key {
        match state.args.api_key_admin.as_ref() {
            Some(admin_key) if !admin_key.is_empty() && bootstrap_key == admin_key => {
                user.permission_level = PermissionLevel::Admin;
                info!("Admin bootstrap: promoting {} to Admin", body.identifier);
            }
            Some(admin_key) if !admin_key.is_empty() => {
                warn!("Admin bootstrap key mismatch for {}", body.identifier);
                return json_response(
                    StatusCode::FORBIDDEN,
                    &ErrorResponse {
                        error: "Admin bootstrap key does not match the configured admin key".into(),
                        code: Some("ADMIN_KEY_REJECTED".into()),
                    },
                );
            }
            _ => {
                // api_key_admin is unset or empty: the server cannot honour a
                // bootstrap-key promotion at all. Surface it rather than issuing
                // a silently-downgraded Authenticated token.
                warn!(
                    "Admin bootstrap key supplied for {} but no admin key is configured",
                    body.identifier
                );
                return json_response(
                    StatusCode::FORBIDDEN,
                    &ErrorResponse {
                        error: "Admin bootstrap is not configured on this doorway".into(),
                        code: Some("ADMIN_KEY_UNCONFIGURED".into()),
                    },
                );
            }
        }
    }

    // Capture permission level before user is moved into insert
    let user_permission_level = user.permission_level;

    // Insert into MongoDB
    if let Err(e) = collection.insert_one(user).await {
        // Check for duplicate key error (race condition)
        let error_str = e.to_string();
        if error_str.contains("duplicate key") || error_str.contains("E11000") {
            return json_response(
                StatusCode::CONFLICT,
                &ErrorResponse {
                    error: "An account with this identifier already exists".into(),
                    code: Some("USER_EXISTS".into()),
                },
            );
        }
        return json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &ErrorResponse {
                error: format!("Failed to create user: {e}"),
                code: Some("DB_ERROR".into()),
            },
        );
    }

    info!(
        "Registered new user: {} with custodial key",
        body.identifier
    );

    generate_auth_response(
        &jwt,
        &state,
        &human_id,
        &actual_agent_pub_key,
        &body.identifier,
        None, // No session_id for registration (key not activated yet)
        StatusCode::CREATED,
        profile,
        user_permission_level,
        None,
        provisioned.as_ref().map(|p| p.conductor_id.clone()),
        false, // New registrations are never stewards
        false,
    )
    .await
}

/// POST /auth/login
///
/// Authenticate with identifier and password.
///
/// Flow:
/// 1. Look up user by identifier in MongoDB
/// 2. Verify password hash with argon2
/// 3. Generate and return JWT token
async fn handle_login(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    let mut body: LoginRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid JSON body: {e}"),
                    code: None,
                },
            )
        }
    };

    if body.identifier.is_empty() || body.password.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "Missing required fields: identifier, password".into(),
                code: None,
            },
        );
    }

    // Gateway-scoped identifier resolution. The doorway re-qualifies the submitted
    // identifier's local-part with its OWN configured gateway domain (from DOORWAY_URL,
    // never the inbound Host header). THIS is where "the suffix is enforced by the
    // doorway" actually happens — the frontend only strips + displays it. So a bare
    // `matthew.dowell`, the full `matthew.dowell@alpha.elohim.host`, and a foreign-domain
    // paste all resolve to this doorway's namespace. See doorway-access-tier-patterns.md
    // ("No third portal — the doorway auth_routes.rs IS the substrate auth surface").
    let original_identifier = body.identifier.clone();
    if let Some(domain) = gateway_domain(state.args.doorway_url.as_deref()) {
        body.identifier = normalize_identifier(&body.identifier, &domain);
    }

    let jwt = match get_jwt_validator(&state) {
        Ok(j) => j,
        Err(resp) => return resp,
    };

    // In dev mode without MongoDB, accept any credentials
    if state.args.dev_mode && state.mongo.is_none() {
        info!("Dev mode login (no MongoDB): {}", body.identifier);
        let dev_session_id = uuid::Uuid::new_v4().to_string();
        return generate_auth_response(
            &jwt,
            &state,
            &format!("human-{}", body.identifier),
            "uhCAk-dev-mode-agent-key",
            &body.identifier,
            Some(dev_session_id),
            StatusCode::OK,
            None,
            PermissionLevel::Admin, // Dev mode gets admin access
            None,
            None,  // No conductor_id in dev mode
            false, // Dev mode: not a steward
            false,
        )
        .await;
    }

    // Production flow: verify against MongoDB
    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return json_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &ErrorResponse {
                    error: "Database not available".into(),
                    code: Some("DB_UNAVAILABLE".into()),
                },
            )
        }
    };

    // Get users collection
    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: format!("Database error: {e}"),
                    code: Some("DB_ERROR".into()),
                },
            )
        }
    };

    // Look up user by the normalized identifier first. For backward-compatibility,
    // fall back to a legacy record stored under the BARE local-part (accounts created
    // before identifier normalization landed) so this fix never orphans an existing
    // login. A foreign-domain input is NOT retried verbatim — the doorway is
    // gateway-scoped, so only the bare local-part is an admissible fallback.
    let mut lookup_candidates = vec![body.identifier.clone()];
    if !original_identifier.contains('@') && original_identifier != body.identifier {
        lookup_candidates.push(original_identifier.clone());
    }

    let mut found_user: Option<UserDoc> = None;
    for candidate in &lookup_candidates {
        match collection
            .find_one(doc! { "identifier": candidate, "is_active": true })
            .await
        {
            Ok(Some(u)) => {
                found_user = Some(u);
                break;
            }
            Ok(None) => continue,
            Err(e) => {
                return json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &ErrorResponse {
                        error: format!("Database error: {e}"),
                        code: Some("DB_ERROR".into()),
                    },
                )
            }
        }
    }

    let user = match found_user {
        Some(u) => u,
        None => {
            warn!("Login failed - user not found: {}", body.identifier);
            // Use generic error to prevent user enumeration
            return json_response(
                StatusCode::UNAUTHORIZED,
                &ErrorResponse {
                    error: "Invalid credentials".into(),
                    code: Some("INVALID_CREDENTIALS".into()),
                },
            );
        }
    };

    // Verify password
    let password_valid = match verify_password(&body.password, &user.password_hash) {
        Ok(valid) => valid,
        Err(e) => {
            warn!("Password verification error: {}", e);
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: "Authentication error".into(),
                    code: Some("AUTH_ERROR".into()),
                },
            );
        }
    };

    if !password_valid {
        warn!("Login failed - invalid password: {}", body.identifier);
        return json_response(
            StatusCode::UNAUTHORIZED,
            &ErrorResponse {
                error: "Invalid credentials".into(),
                code: Some("INVALID_CREDENTIALS".into()),
            },
        );
    }

    // Generate session ID for key cache lookup
    let session_id = uuid::Uuid::new_v4().to_string();

    // Activate custodial key if user has one
    if user.has_custodial_key() {
        let custodial_key_service = CustodialKeyService::new();
        match custodial_key_service.activate_key(&session_id, &user, &body.password) {
            Ok(_verifying_key) => {
                info!(
                    "Activated custodial key for session {} (user: {})",
                    session_id, body.identifier
                );
            }
            Err(e) => {
                warn!(
                    "Failed to activate custodial key for {}: {}",
                    body.identifier, e
                );
                return json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &ErrorResponse {
                        error: "Failed to activate signing key".into(),
                        code: Some("KEY_ACTIVATION_ERROR".into()),
                    },
                );
            }
        }
    }

    // Auto-provision on conductor if user has no conductor assignment
    let (final_agent_pub_key, installed_app_id, login_conductor_id) = if user.conductor_id.is_none()
    {
        if let Some(ref registry) = state.conductor_registry {
            if !state.args.dev_mode {
                let provisioner = AgentProvisioner::new(Arc::clone(registry))
                    .with_app_id(state.args.installed_app_id.clone())
                    .with_bundle_path(state.args.happ_bundle_path.clone());
                match provisioner.provision_agent(&body.identifier).await {
                    Ok(p) => {
                        info!(
                            "Auto-provisioned {} on {} (app: {})",
                            body.identifier, p.conductor_id, p.installed_app_id
                        );
                        // Update UserDoc with conductor assignment
                        let update = doc! {
                            "$set": {
                                "conductor_id": &p.conductor_id,
                                "agent_pub_key": &p.agent_pub_key,
                            }
                        };
                        if let Err(e) = collection
                            .update_one(doc! { "identifier": &body.identifier }, update)
                            .await
                        {
                            warn!("Failed to update user doc after provisioning: {}", e);
                        }
                        let cid = p.conductor_id.clone();
                        (p.agent_pub_key, Some(p.installed_app_id), Some(cid))
                    }
                    Err(e) => {
                        warn!("Auto-provisioning failed for {}: {}", body.identifier, e);
                        (user.agent_pub_key.clone(), None, None)
                    }
                }
            } else {
                (user.agent_pub_key.clone(), None, None)
            }
        } else {
            (user.agent_pub_key.clone(), None, None)
        }
    } else {
        // Already provisioned — look up installed_app_id from registry
        let (app_id, cid) = state
            .conductor_registry
            .as_ref()
            .and_then(|r| r.get_conductor_for_agent(&user.agent_pub_key))
            .map(|e| (Some(e.app_id), Some(e.conductor_id)))
            .unwrap_or((None, user.conductor_id.clone()));
        (user.agent_pub_key.clone(), app_id, cid)
    };

    info!(
        "Login successful: {} (permission: {:?})",
        body.identifier, user.permission_level
    );

    generate_auth_response(
        &jwt,
        &state,
        &user.human_id,
        &final_agent_pub_key,
        &user.identifier,
        Some(session_id),
        StatusCode::OK,
        None,
        user.permission_level,
        installed_app_id,
        login_conductor_id,
        user.is_steward,
        user.conductor_id.is_some(),
    )
    .await
}

/// POST /auth/logout
///
/// Logout (primarily client-side, but can be used for token blacklisting).
/// For now, this is a no-op as tokens are stateless.
async fn handle_logout(
    _req: Request<hyper::body::Incoming>,
    _state: Arc<AppState>,
) -> Response<BoxBody> {
    // In the future, we could implement token blacklisting here
    // For now, logout is handled client-side by removing the token
    json_response(
        StatusCode::OK,
        &SuccessResponse {
            success: true,
            message: "Logged out successfully".into(),
        },
    )
}

/// POST /auth/refresh
///
/// Refresh an existing token.
async fn handle_refresh(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    let auth_header = get_auth_header(&req);
    let token = match extract_token_from_header(auth_header) {
        Some(t) => t,
        None => {
            return json_response(
                StatusCode::UNAUTHORIZED,
                &ErrorResponse {
                    error: "No token provided".into(),
                    code: None,
                },
            )
        }
    };

    let jwt = match get_jwt_validator(&state) {
        Ok(j) => j,
        Err(resp) => return resp,
    };

    let result = jwt.verify_token(token);
    if !result.valid {
        return json_response(
            StatusCode::UNAUTHORIZED,
            &ErrorResponse {
                error: result.error.unwrap_or_else(|| "Invalid token".into()),
                code: Some("INVALID_TOKEN".into()),
            },
        );
    }

    let old_claims = result.claims.unwrap();

    generate_auth_response(
        &jwt,
        &state,
        &old_claims.human_id,
        &old_claims.agent_pub_key,
        &old_claims.identifier,
        old_claims.session_id, // Preserve session_id from old token
        StatusCode::OK,
        None,
        old_claims.permission_level, // Preserve permission from old token
        old_claims.installed_app_id,
        old_claims.conductor_id, // Preserve conductor_id from old token
        old_claims.is_steward,
        old_claims.has_local_conductor,
    )
    .await
}

/// Decision arm of the durable-state session-revocation check (`handle_me`):
/// an explicitly inactive account row revokes the session; an active row or no
/// row at all (legacy accounts, never-registered native agents) proceeds on
/// JWT validity alone.
///
/// Invariant pinned by `revocation_targets_unique_identifier_not_human_id`:
/// the row passed here must be selected by the account-unique `identifier`,
/// never by `human_id`, which collides across hosted accounts under
/// dev_mode's shared-singleton provisioning.
fn session_revoked_by_user_doc(user: Option<&UserDoc>) -> bool {
    matches!(user, Some(u) if !u.is_active)
}

/// GET /auth/me
///
/// Get current user info from token.
async fn handle_me(req: Request<hyper::body::Incoming>, state: Arc<AppState>) -> Response<BoxBody> {
    let auth_header = get_auth_header(&req);
    let token = match extract_token_from_header(auth_header) {
        Some(t) => t,
        None => {
            return json_response(
                StatusCode::UNAUTHORIZED,
                &ErrorResponse {
                    error: "No token provided".into(),
                    code: None,
                },
            )
        }
    };

    let jwt = match get_jwt_validator(&state) {
        Ok(j) => j,
        Err(resp) => return resp,
    };

    let result = jwt.verify_token(token);
    if !result.valid {
        return json_response(
            StatusCode::UNAUTHORIZED,
            &ErrorResponse {
                error: result
                    .error
                    .unwrap_or_else(|| "Invalid or expired token".into()),
                code: None,
            },
        );
    }

    let claims = result.claims.unwrap();

    // Consult durable user state after JWT signature/expiry validation.
    //
    // A JWT is only signature+expiry-checked by verify_token; on its own it can
    // outlive an admin suspension. If the user has since been deactivated
    // (adminSetUserStatus(active=false) / soft-delete sets is_active=false), a
    // still-held token must NOT continue to authenticate. We mirror the
    // existing MongoDB lookup in handle_account. This is the active-flag partial
    // of server-side revocation: it covers suspend-a-bad-actor-now without a
    // shared cross-replica token blacklist (that broader substrate decision is
    // tracked separately). When MongoDB is unavailable we degrade to the prior
    // JWT-only behaviour rather than failing closed for every caller.
    //
    // The lookup keys on `identifier` — the register-time uniqueness key (and
    // the login key, see handle_login) — NOT `human_id`: under dev_mode the
    // hosted-register path skips per-user provisioning, so every registrant
    // persists the shared singleton agent's human_id. A human_id-keyed
    // find_one then resolves an arbitrary (typically still-active) account
    // and a suspended session keeps authenticating (genesis #1105).
    if let Some(mongo) = &state.mongo {
        if let Ok(collection) = mongo.collection::<UserDoc>(USER_COLLECTION).await {
            match collection
                .find_one(doc! { "identifier": &claims.identifier })
                .await
            {
                Ok(user) if session_revoked_by_user_doc(user.as_ref()) => {
                    warn!(
                        "Rejecting /auth/me for suspended user {}",
                        claims.identifier
                    );
                    return json_response(
                        StatusCode::UNAUTHORIZED,
                        &ErrorResponse {
                            error: "Account is suspended".into(),
                            code: Some("ACCOUNT_SUSPENDED".into()),
                        },
                    );
                }
                Ok(_) => {} // active user, or no row (legacy/dev) — proceed
                Err(e) => {
                    // DB error: degrade to JWT-only rather than fail closed.
                    warn!("/auth/me active-status lookup failed (degrading): {}", e);
                }
            }
        }
    }

    // Derive authority label from doorway_url hostname; fall back to doorway_id,
    // then to a generic placeholder. This is the doorway-host mode label shown by
    // the trust-indicator chip in the portal shell.
    let authority_label = claims
        .doorway_url
        .as_deref()
        .and_then(|url| {
            // Strip scheme: "https://alpha.elohim.host" → "alpha.elohim.host"
            url.strip_prefix("https://")
                .or_else(|| url.strip_prefix("http://"))
                .map(|h| h.trim_end_matches('/').to_string())
        })
        .or_else(|| claims.doorway_id.clone())
        .unwrap_or_else(|| "elohim.host".to_string());

    json_response(
        StatusCode::OK,
        &MeResponse {
            human_id: claims.human_id,
            agent_pub_key: claims.agent_pub_key,
            identifier: claims.identifier,
            permission_level: claims.permission_level.to_string(),
            doorway_id: claims.doorway_id.clone(),
            doorway_url: claims.doorway_url,
            authenticated: true,
            trust_mode: "doorway-host".to_string(),
            authority: AuthorityRef {
                label: authority_label,
                id: claims.doorway_id,
            },
            conductor_endpoint: None,
        },
    )
}

/// GET /auth/account
///
/// Get full account context including usage, quotas, hosting status.
async fn handle_account(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    let auth_header = get_auth_header(&req);
    let token = match extract_token_from_header(auth_header) {
        Some(t) => t,
        None => {
            return json_response(
                StatusCode::UNAUTHORIZED,
                &ErrorResponse {
                    error: "No token provided".into(),
                    code: None,
                },
            )
        }
    };

    let jwt = match get_jwt_validator(&state) {
        Ok(j) => j,
        Err(resp) => return resp,
    };

    let result = jwt.verify_token(token);
    if !result.valid {
        return json_response(
            StatusCode::UNAUTHORIZED,
            &ErrorResponse {
                error: result
                    .error
                    .unwrap_or_else(|| "Invalid or expired token".into()),
                code: None,
            },
        );
    }

    let claims = result.claims.unwrap();

    // In dev mode without MongoDB, synthesize the account context from the
    // verified claims (same pattern as the dev-mode register/login flows).
    // Keeps the operator dashboard's auth guard (which probes /auth/account)
    // exercisable on a local stack that runs no Mongo.
    if state.args.dev_mode && state.mongo.is_none() {
        return json_response(
            StatusCode::OK,
            &AccountResponse {
                human_id: claims.human_id.clone(),
                identifier: claims.identifier.clone(),
                permission_level: claims.permission_level.to_string(),
                storage_bytes: 0,
                storage_limit: 0,
                storage_percent: 0.0,
                projection_queries: 0,
                daily_query_limit: 0,
                queries_percent: 0.0,
                bandwidth_bytes: 0,
                daily_bandwidth_limit: 0,
                bandwidth_percent: 0.0,
                conductor_id: None,
                is_steward: false,
                stewardship_at: None,
                key_exported: false,
                created_at: None,
                last_login_at: None,
            },
        );
    }

    // Look up full user doc from MongoDB
    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return json_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &ErrorResponse {
                    error: "Database not available".into(),
                    code: Some("DB_UNAVAILABLE".into()),
                },
            )
        }
    };

    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: format!("Database error: {e}"),
                    code: Some("DB_ERROR".into()),
                },
            )
        }
    };

    // Keyed on the account-unique `identifier`, not `human_id` — under
    // dev_mode every hosted registrant shares the singleton agent's human_id,
    // so a human_id lookup returns an arbitrary account (see handle_me).
    let user = match collection
        .find_one(doc! { "identifier": &claims.identifier })
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return json_response(
                StatusCode::NOT_FOUND,
                &ErrorResponse {
                    error: "User not found".into(),
                    code: Some("USER_NOT_FOUND".into()),
                },
            )
        }
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: format!("Database error: {e}"),
                    code: Some("DB_ERROR".into()),
                },
            )
        }
    };

    let usage = &user.usage;
    let quota = &user.quota;

    let storage_percent = if quota.storage_limit > 0 {
        (usage.storage_bytes as f64 / quota.storage_limit as f64) * 100.0
    } else {
        0.0
    };
    let queries_percent = if quota.daily_query_limit > 0 {
        (usage.projection_queries as f64 / quota.daily_query_limit as f64) * 100.0
    } else {
        0.0
    };
    let bandwidth_percent = if quota.daily_bandwidth_limit > 0 {
        (usage.bandwidth_bytes as f64 / quota.daily_bandwidth_limit as f64) * 100.0
    } else {
        0.0
    };

    let key_exported = user
        .custodial_key
        .as_ref()
        .map(|k| k.exported)
        .unwrap_or(false);

    json_response(
        StatusCode::OK,
        &AccountResponse {
            human_id: user.human_id,
            identifier: user.identifier,
            permission_level: user.permission_level.to_string(),
            storage_bytes: usage.storage_bytes,
            storage_limit: quota.storage_limit,
            storage_percent,
            projection_queries: usage.projection_queries,
            daily_query_limit: quota.daily_query_limit,
            queries_percent,
            bandwidth_bytes: usage.bandwidth_bytes,
            daily_bandwidth_limit: quota.daily_bandwidth_limit,
            bandwidth_percent,
            conductor_id: user.conductor_id,
            is_steward: user.is_steward,
            stewardship_at: user.stewardship_at.map(|d| d.to_string()),
            key_exported,
            created_at: user.metadata.created_at.map(|d| d.to_string()),
            last_login_at: user.last_login_at.map(|d| d.to_string()),
        },
    )
}

// =============================================================================
// Native Handoff Handler (Tauri Session Migration)
// =============================================================================

/// GET /auth/native-handoff
///
/// Returns identity information for Tauri native session creation.
/// Called after OAuth token exchange when migrating from doorway to native.
///
/// The response contains only identity info, not content. Content syncs
/// automatically via P2P (Holochain DHT gossip) once the native conductor
/// joins the network.
///
/// Response includes:
/// - human_id, identifier: Core identity
/// - doorway_id, doorway_url: For future recovery
/// - display_name, profile_image_hash: Optional profile info
/// - bootstrap_url: For P2P discovery
async fn handle_native_handoff(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    // Validate token from Authorization header
    let auth_header = get_auth_header(&req);
    let token = match extract_token_from_header(auth_header) {
        Some(t) => t,
        None => {
            return json_response(
                StatusCode::UNAUTHORIZED,
                &ErrorResponse {
                    error: "No token provided".into(),
                    code: Some("NO_TOKEN".into()),
                },
            )
        }
    };

    let jwt = match get_jwt_validator(&state) {
        Ok(j) => j,
        Err(resp) => return resp,
    };

    let result = jwt.verify_token(token);
    if !result.valid {
        return json_response(
            StatusCode::UNAUTHORIZED,
            &ErrorResponse {
                error: result
                    .error
                    .unwrap_or_else(|| "Invalid or expired token".into()),
                code: Some("INVALID_TOKEN".into()),
            },
        );
    }

    let claims = result.claims.unwrap();

    // Get doorway identity from config (required for handoff)
    let doorway_id = match &state.args.doorway_id {
        Some(id) => id.clone(),
        None => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: "Doorway ID not configured".into(),
                    code: Some("CONFIG_ERROR".into()),
                },
            )
        }
    };

    let doorway_url = match &state.args.doorway_url {
        Some(url) => url.clone(),
        None => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: "Doorway URL not configured".into(),
                    code: Some("CONFIG_ERROR".into()),
                },
            )
        }
    };

    // Get network config from args
    let bootstrap_url = state.args.bootstrap_url.clone();
    let signal_url = state.args.signal_url.clone();

    // Look up UserDoc from MongoDB for agent_pub_key, conductor_id, custodial key, is_steward
    let (agent_pub_key, conductor_id, display_name, profile_image_hash, key_bundle, is_steward) =
        if let Some(ref mongo) = state.mongo {
            match mongo.collection::<UserDoc>(USER_COLLECTION).await {
                Ok(collection) => {
                    match collection
                        .find_one(doc! { "identifier": &claims.identifier })
                        .await
                    {
                        Ok(Some(user)) => {
                            // Export key bundle inline (non-destructive, does NOT mark as exported)
                            let bundle = if user.has_custodial_key() {
                                let key_service = CustodialKeyService::new();
                                match key_service.export_key(&user, &doorway_id) {
                                    Ok(export) => Some(export),
                                    Err(e) => {
                                        warn!(
                                            identifier = %claims.identifier,
                                            error = %e,
                                            "Failed to export key for native handoff"
                                        );
                                        None
                                    }
                                }
                            } else {
                                None
                            };

                            (
                                user.agent_pub_key.clone(),
                                user.conductor_id.clone(),
                                None::<String>, // TODO: display_name from profile
                                None::<String>, // TODO: profile_image_hash from profile
                                bundle,
                                user.is_steward,
                            )
                        }
                        Ok(None) => {
                            warn!(
                                identifier = %claims.identifier,
                                "User not found in MongoDB during native handoff"
                            );
                            (claims.human_id.clone(), None, None, None, None, false)
                        }
                        Err(e) => {
                            warn!(error = %e, "MongoDB lookup failed during native handoff");
                            (claims.human_id.clone(), None, None, None, None, false)
                        }
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Failed to get collection during native handoff");
                    (claims.human_id.clone(), None, None, None, None, false)
                }
            }
        } else {
            (claims.human_id.clone(), None, None, None, None, false)
        };

    // Look up installed_app_id from conductor registry
    let installed_app_id = if let Some(ref registry) = state.conductor_registry {
        if let Some(entry) = registry.get_conductor_for_agent(&agent_pub_key) {
            Some(entry.app_id)
        } else {
            Some(state.args.installed_app_id.clone())
        }
    } else {
        None
    };

    info!(
        identifier = %claims.identifier,
        agent_pub_key = %agent_pub_key,
        conductor_id = ?conductor_id,
        has_key_bundle = key_bundle.is_some(),
        is_steward = is_steward,
        "Native handoff: identity + network context provided"
    );

    json_response(
        StatusCode::OK,
        &NativeHandoffResponse {
            human_id: claims.human_id,
            identifier: claims.identifier,
            agent_pub_key,
            doorway_id,
            doorway_url,
            display_name,
            profile_image_hash,
            bootstrap_url,
            signal_url,
            network_seed: None, // reserved for future custom network seeds
            installed_app_id,
            conductor_id,
            key_bundle,
            is_steward,
        },
    )
}

// =============================================================================
// Stewardship Migration Handlers
// =============================================================================

/// GET /auth/export-key
///
/// Export the user's encrypted key bundle for migration to stewardship (Tauri).
/// The private key remains encrypted with the user's password - they must
/// enter their password in the Tauri app to decrypt it.
///
/// This endpoint:
/// 1. Validates the user's JWT token
/// 2. Looks up their custodial key material in MongoDB
/// 3. Returns the encrypted key bundle
/// 4. Marks the key as exported in MongoDB (audit trail)
async fn handle_export_key(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    // Validate token
    let auth_header = get_auth_header(&req);
    let token = match extract_token_from_header(auth_header) {
        Some(t) => t,
        None => {
            return json_response(
                StatusCode::UNAUTHORIZED,
                &ErrorResponse {
                    error: "No token provided".into(),
                    code: Some("NO_TOKEN".into()),
                },
            )
        }
    };

    let jwt = match get_jwt_validator(&state) {
        Ok(j) => j,
        Err(resp) => return resp,
    };

    let result = jwt.verify_token(token);
    if !result.valid {
        return json_response(
            StatusCode::UNAUTHORIZED,
            &ErrorResponse {
                error: result.error.unwrap_or_else(|| "Invalid token".into()),
                code: Some("INVALID_TOKEN".into()),
            },
        );
    }

    let claims = result.claims.unwrap();

    // Get doorway ID for export
    let doorway_id = match &state.args.doorway_id {
        Some(id) => id.clone(),
        None => "unknown".to_string(),
    };

    // Get MongoDB connection
    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return json_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &ErrorResponse {
                    error: "Database not available".into(),
                    code: Some("DB_UNAVAILABLE".into()),
                },
            )
        }
    };

    // Get user from MongoDB
    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: format!("Database error: {e}"),
                    code: Some("DB_ERROR".into()),
                },
            )
        }
    };

    let user = match collection
        .find_one(doc! { "identifier": &claims.identifier })
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return json_response(
                StatusCode::NOT_FOUND,
                &ErrorResponse {
                    error: "User not found".into(),
                    code: Some("USER_NOT_FOUND".into()),
                },
            )
        }
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: format!("Database error: {e}"),
                    code: Some("DB_ERROR".into()),
                },
            )
        }
    };

    // Export the key
    let key_service = CustodialKeyService::new();
    let export = match key_service.export_key(&user, &doorway_id) {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to export key for {}: {}", claims.identifier, e);
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Cannot export key: {e}"),
                    code: Some("EXPORT_ERROR".into()),
                },
            );
        }
    };

    // Mark the key as exported in MongoDB
    if let Err(e) = collection
        .update_one(
            doc! { "identifier": &claims.identifier },
            doc! {
                "$set": {
                    "custodial_key.exported": true,
                    "custodial_key.exported_at": bson::DateTime::now(),
                }
            },
        )
        .await
    {
        warn!("Failed to mark key as exported: {}", e);
    }

    info!(
        "Exported custodial key for {} (preparing for stewardship)",
        claims.identifier
    );

    json_response(
        StatusCode::OK,
        &KeyExportResponse {
            key_bundle: export,
            instructions: "Import this key bundle into your Elohim Tauri app. \
                You will need to enter your password to decrypt the key. \
                Once imported, call /auth/confirm-stewardship to complete migration."
                .to_string(),
        },
    )
}

/// POST /auth/confirm-stewardship
///
/// Confirm that the user has successfully migrated their key to Tauri.
/// Called by the Tauri app after successful key import and decryption.
///
/// The request must include a signature of the human_id, proving that the
/// user actually has access to the private key.
///
/// This endpoint:
/// 1. Validates the JWT token
/// 2. Verifies Ed25519 signature proves key possession
/// 3. Marks the user as steward in MongoDB
/// 4. Deprovisions conductor cell (best effort)
/// 5. Clears conductor_id on UserDoc
/// 6. Deactivates ALL cached signing keys
async fn handle_confirm_stewardship(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use ed25519_dalek::Verifier;

    // Get auth header before consuming request
    let auth_header = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Parse request body
    let body: ConfirmStewardshipRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid JSON body: {e}"),
                    code: None,
                },
            )
        }
    };

    // Validate token
    let token = match extract_token_from_header(auth_header.as_deref()) {
        Some(t) => t,
        None => {
            return json_response(
                StatusCode::UNAUTHORIZED,
                &ErrorResponse {
                    error: "No token provided".into(),
                    code: Some("NO_TOKEN".into()),
                },
            )
        }
    };

    let jwt = match get_jwt_validator(&state) {
        Ok(j) => j,
        Err(resp) => return resp,
    };

    let result = jwt.verify_token(token);
    if !result.valid {
        return json_response(
            StatusCode::UNAUTHORIZED,
            &ErrorResponse {
                error: result.error.unwrap_or_else(|| "Invalid token".into()),
                code: Some("INVALID_TOKEN".into()),
            },
        );
    }

    let claims = result.claims.unwrap();

    if body.signature.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "Signature required".into(),
                code: Some("SIGNATURE_REQUIRED".into()),
            },
        );
    }

    // Get MongoDB connection
    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return json_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &ErrorResponse {
                    error: "Database not available".into(),
                    code: Some("DB_UNAVAILABLE".into()),
                },
            )
        }
    };

    // Fetch user from MongoDB (need custodial_key.public_key for sig verification)
    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: format!("Database error: {e}"),
                    code: Some("DB_ERROR".into()),
                },
            )
        }
    };

    let user = match collection
        .find_one(doc! { "identifier": &claims.identifier })
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return json_response(
                StatusCode::NOT_FOUND,
                &ErrorResponse {
                    error: "User not found".into(),
                    code: Some("USER_NOT_FOUND".into()),
                },
            )
        }
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: format!("Database error: {e}"),
                    code: Some("DB_ERROR".into()),
                },
            )
        }
    };

    // Idempotency check: if already steward, return success immediately
    if user.is_steward {
        info!(
            "User {} already a steward, returning success (idempotent)",
            claims.identifier
        );
        return json_response(
            StatusCode::OK,
            &StewardshipConfirmedResponse {
                success: true,
                message: "Already a steward.".into(),
                stewardship_at: user
                    .stewardship_at
                    .map(|dt| {
                        chrono::DateTime::from_timestamp(
                            dt.timestamp_millis() / 1000,
                            ((dt.timestamp_millis() % 1000) * 1_000_000) as u32,
                        )
                        .unwrap_or_default()
                        .to_rfc3339()
                    })
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            },
        );
    }

    // Verify Ed25519 signature — steward signs human_id, we verify with custodial public key
    let custodial_key = match &user.custodial_key {
        Some(k) => k,
        None => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: "User has no custodial key to verify against".into(),
                    code: Some("NO_CUSTODIAL_KEY".into()),
                },
            )
        }
    };

    let pub_key_bytes = match BASE64.decode(&custodial_key.public_key) {
        Ok(b) if b.len() == 32 => b,
        Ok(b) => {
            warn!(
                "Invalid custodial public key length for {}: expected 32, got {}",
                claims.identifier,
                b.len()
            );
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: "Invalid custodial key format".into(),
                    code: Some("KEY_FORMAT_ERROR".into()),
                },
            );
        }
        Err(e) => {
            warn!("Failed to decode custodial public key: {}", e);
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: "Failed to decode custodial key".into(),
                    code: Some("KEY_DECODE_ERROR".into()),
                },
            );
        }
    };

    let verifying_key =
        match ed25519_dalek::VerifyingKey::from_bytes(pub_key_bytes.as_slice().try_into().unwrap())
        {
            Ok(k) => k,
            Err(e) => {
                warn!(
                    "Invalid Ed25519 verifying key for {}: {}",
                    claims.identifier, e
                );
                return json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &ErrorResponse {
                        error: "Invalid custodial key".into(),
                        code: Some("KEY_ERROR".into()),
                    },
                );
            }
        };

    let sig_bytes = match BASE64.decode(&body.signature) {
        Ok(b) if b.len() == 64 => b,
        Ok(b) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid signature length: expected 64, got {}", b.len()),
                    code: Some("INVALID_SIGNATURE".into()),
                },
            )
        }
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid signature encoding: {e}"),
                    code: Some("INVALID_SIGNATURE".into()),
                },
            )
        }
    };

    let signature = ed25519_dalek::Signature::from_bytes(sig_bytes.as_slice().try_into().unwrap());
    if verifying_key
        .verify(user.human_id.as_bytes(), &signature)
        .is_err()
    {
        warn!(
            "Stewardship signature verification failed for {}",
            claims.identifier
        );
        return json_response(
            StatusCode::FORBIDDEN,
            &ErrorResponse {
                error: "Signature verification failed — cannot prove key possession".into(),
                code: Some("SIGNATURE_INVALID".into()),
            },
        );
    }

    // Mark is_steward: true + set stewardship_at in MongoDB
    let stewardship_time = bson::DateTime::now();
    if let Err(e) = collection
        .update_one(
            doc! { "identifier": &claims.identifier },
            doc! {
                "$set": {
                    "is_steward": true,
                    "stewardship_at": stewardship_time,
                    "conductor_id": bson::Bson::Null,
                }
            },
        )
        .await
    {
        return json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &ErrorResponse {
                error: format!("Failed to update user: {e}"),
                code: Some("DB_ERROR".into()),
            },
        );
    }

    // Deprovision conductor cell (best effort)
    if let Some(ref registry) = state.conductor_registry {
        let provisioner = AgentProvisioner::new(Arc::clone(registry))
            .with_app_id(state.args.installed_app_id.clone())
            .with_bundle_path(state.args.happ_bundle_path.clone());
        match provisioner.deprovision_agent(&user.agent_pub_key).await {
            Ok(()) => {
                info!(
                    agent = %user.agent_pub_key,
                    "Conductor cell deprovisioned during stewardship graduation"
                );
            }
            Err(e) => {
                warn!(
                    agent = %user.agent_pub_key,
                    error = %e,
                    "Failed to deprovision conductor cell during graduation (will be cleaned up later)"
                );
            }
        }
    }

    // Deactivate ALL cached signing keys for this user
    let key_service = CustodialKeyService::new();
    key_service.deactivate_all(&user.human_id);

    info!(
        "User {} has graduated to stewardship! Conductor cell retired.",
        claims.identifier
    );

    json_response(
        StatusCode::OK,
        &StewardshipConfirmedResponse {
            success: true,
            message: "Welcome to stewardship! You now have full control of your identity.".into(),
            stewardship_at: chrono::Utc::now().to_rfc3339(),
        },
    )
}

// =============================================================================
// Disaster Recovery Handlers
// =============================================================================

/// POST /auth/recover-custody
///
/// Initiate disaster recovery for a steward user who has lost device access.
/// This creates a RecoveryRequest in the DHT and notifies emergency contacts.
///
/// Flow:
/// 1. Validate user exists and is_steward == true
/// 2. Create RecoveryRequest in DHT via imagodei zome
/// 3. Return request_id, required_approvals, expires_at
async fn handle_recover_custody(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    let body: RecoverCustodyRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid JSON body: {e}"),
                    code: None,
                },
            )
        }
    };

    if body.identifier.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "Missing required field: identifier".into(),
                code: None,
            },
        );
    }

    // Get MongoDB connection
    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return json_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &ErrorResponse {
                    error: "Database not available".into(),
                    code: Some("DB_UNAVAILABLE".into()),
                },
            )
        }
    };

    // Get user from MongoDB
    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: format!("Database error: {e}"),
                    code: Some("DB_ERROR".into()),
                },
            )
        }
    };

    let user = match collection
        .find_one(doc! { "identifier": &body.identifier })
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            // Use generic error to prevent user enumeration
            warn!("Recovery attempt for unknown user: {}", body.identifier);
            return json_response(
                StatusCode::NOT_FOUND,
                &ErrorResponse {
                    error: "User not found".into(),
                    code: Some("USER_NOT_FOUND".into()),
                },
            );
        }
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: format!("Database error: {e}"),
                    code: Some("DB_ERROR".into()),
                },
            )
        }
    };

    // Verify user is a steward (recovery only applies to steward users)
    if !user.is_steward {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "Recovery is only available for steward users. Use regular login.".into(),
                code: Some("NOT_STEWARD".into()),
            },
        );
    }

    // Recovery requires imagodei zome integration (RecoveryRequest DHT entry).
    // Not yet implemented — return 501 so callers know the feature doesn't exist.
    json_response(
        StatusCode::NOT_IMPLEMENTED,
        &ErrorResponse {
            error: "Social recovery is not yet implemented. Requires imagodei zome integration."
                .into(),
            code: Some("NOT_IMPLEMENTED".into()),
        },
    )
}

/// POST /auth/check-recovery-status
///
/// Poll for recovery request approval status.
/// Returns current vote count and status. If approved, includes recovery_token.
async fn handle_check_recovery_status(
    req: Request<hyper::body::Incoming>,
    _state: Arc<AppState>,
) -> Response<BoxBody> {
    let body: CheckRecoveryStatusRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid JSON body: {e}"),
                    code: None,
                },
            )
        }
    };

    if body.request_id.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "Missing required field: request_id".into(),
                code: None,
            },
        );
    }

    json_response(
        StatusCode::NOT_IMPLEMENTED,
        &ErrorResponse {
            error: "Recovery status checking is not yet implemented.".into(),
            code: Some("NOT_IMPLEMENTED".into()),
        },
    )
}

/// POST /auth/activate-recovery
///
/// Activate recovery after social verification approval.
/// Generates a NEW custodial keypair and returns JWT token.
///
/// Flow:
/// 1. Validate recovery session token
/// 2. Generate NEW custodial keypair (old key is lost)
/// 3. Update user: custodial_key = new, is_steward = false
/// 4. Activate key, generate JWT with recovery_mode flag
async fn handle_activate_recovery(
    req: Request<hyper::body::Incoming>,
    _state: Arc<AppState>,
) -> Response<BoxBody> {
    let body: ActivateRecoveryRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid JSON body: {e}"),
                    code: None,
                },
            )
        }
    };

    if body.request_id.is_empty() || body.new_password.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "Missing required fields: request_id, new_password".into(),
                code: None,
            },
        );
    }

    // Recovery activation requires DHT-verified approval.
    // Not yet implemented — return 501.
    json_response(
        StatusCode::NOT_IMPLEMENTED,
        &ErrorResponse {
            error: "Recovery activation is not yet implemented.".into(),
            code: Some("NOT_IMPLEMENTED".into()),
        },
    )
}

// =============================================================================
// Elohim Verification Handlers
// =============================================================================

/// POST /auth/elohim-verify/start
///
/// Start an Elohim verification session. Returns questions based on the user's
/// imagodei profile that only the real user should be able to answer.
async fn handle_elohim_verify_start(
    req: Request<hyper::body::Incoming>,
    _state: Arc<AppState>,
) -> Response<BoxBody> {
    use crate::services::{ElohimVerifier, PathCompletion, QuizScore, UserProfileData};

    let body: ElohimVerifyStartRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid JSON body: {e}"),
                    code: None,
                },
            )
        }
    };

    if body.request_id.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "Missing required field: request_id".into(),
                code: None,
            },
        );
    }

    // TODO: Fetch user's profile data from DHT via imagodei zome
    // For now, use mock data to demonstrate the flow
    let mock_profile = UserProfileData {
        human_id: "human-123".to_string(),
        display_name: "Test User".to_string(),
        affinities: vec!["Technology".to_string(), "Philosophy".to_string()],
        completed_paths: vec![PathCompletion {
            path_id: "elohim-protocol".to_string(),
            path_title: "Elohim Protocol Foundations".to_string(),
            completed_at: "2024-12-01".to_string(),
        }],
        quiz_scores: vec![QuizScore {
            quiz_id: "quiz-manifesto".to_string(),
            quiz_title: "Manifesto Foundations".to_string(),
            score: 8.0,
            max_score: 10.0,
            completed_at: "2024-12-05".to_string(),
        }],
        relationship_names: vec!["Alice".to_string(), "Bob".to_string()],
        learning_preferences: None,
        milestones: vec!["First Path Complete".to_string()],
        created_at: "2024-06-15".to_string(),
    };

    // Generate questions
    let questions = ElohimVerifier::generate_questions(&mock_profile);
    let client_questions = ElohimVerifier::questions_for_client(&questions);

    // Create session ID
    let session_id = format!("elohim-session-{}", uuid::Uuid::new_v4());

    // TODO: Store questions with session_id for later scoring
    // In production, we'd store this in Redis or MongoDB with TTL

    info!(
        "Started Elohim verification session {} for request {}",
        session_id, body.request_id
    );

    json_response(
        StatusCode::OK,
        &ElohimVerifyStartResponse {
            session_id,
            questions: client_questions,
            time_limit_seconds: 300, // 5 minutes
            instructions: "Answer the following questions about your profile. \
                These questions are based on your actual usage and only you should \
                know the answers. You have 5 minutes to complete."
                .to_string(),
        },
    )
}

/// POST /auth/elohim-verify/answer
///
/// Submit answers to Elohim verification questions.
/// Scores the answers and returns confidence contribution.
async fn handle_elohim_verify_answer(
    req: Request<hyper::body::Incoming>,
    _state: Arc<AppState>,
) -> Response<BoxBody> {
    use crate::services::{ElohimVerifier, PathCompletion, QuizScore, UserProfileData};

    let body: ElohimVerifyAnswerRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid JSON body: {e}"),
                    code: None,
                },
            )
        }
    };

    if body.session_id.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "Missing required field: session_id".into(),
                code: None,
            },
        );
    }

    if body.answers.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "No answers provided".into(),
                code: None,
            },
        );
    }

    // TODO: Look up stored questions for session_id
    // For now, regenerate the same mock questions
    let mock_profile = UserProfileData {
        human_id: "human-123".to_string(),
        display_name: "Test User".to_string(),
        affinities: vec!["Technology".to_string(), "Philosophy".to_string()],
        completed_paths: vec![PathCompletion {
            path_id: "elohim-protocol".to_string(),
            path_title: "Elohim Protocol Foundations".to_string(),
            completed_at: "2024-12-01".to_string(),
        }],
        quiz_scores: vec![QuizScore {
            quiz_id: "quiz-manifesto".to_string(),
            quiz_title: "Manifesto Foundations".to_string(),
            score: 8.0,
            max_score: 10.0,
            completed_at: "2024-12-05".to_string(),
        }],
        relationship_names: vec!["Alice".to_string(), "Bob".to_string()],
        learning_preferences: None,
        milestones: vec!["First Path Complete".to_string()],
        created_at: "2024-06-15".to_string(),
    };

    let questions = ElohimVerifier::generate_questions(&mock_profile);

    // Score the answers
    let result = ElohimVerifier::score_answers(&questions, &body.answers);

    // Build feedback
    let feedback: Vec<QuestionFeedback> = result
        .answer_scores
        .iter()
        .map(|s| QuestionFeedback {
            question_id: s.question_id.clone(),
            correct: s.correct,
            message: s.feedback.clone(),
        })
        .collect();

    info!(
        "Elohim verification complete for session {}: accuracy={:.2}, passed={}",
        body.session_id, result.accuracy, result.passed
    );

    // TODO: Update recovery request confidence score in DHT

    json_response(
        StatusCode::OK,
        &ElohimVerifyAnswerResponse {
            passed: result.passed,
            accuracy_percent: result.accuracy * 100.0,
            confidence_score: result.confidence_score,
            summary: result.summary,
            feedback: Some(feedback),
        },
    )
}

// =============================================================================
// OAuth Handlers
// =============================================================================

/// GET /auth/authorize
///
/// OAuth 2.0 authorization endpoint. Validates the client and redirect URI,
/// then redirects to the login page. After successful login, the user is
/// redirected back to the client with an authorization code.
///
/// Query Parameters:
/// - client_id: OAuth client ID (e.g., "elohim-app")
/// - redirect_uri: Where to redirect after authorization
/// - response_type: Must be "code" for authorization code flow
/// - state: CSRF protection token (passed back to client)
/// - scope: Optional requested scope
///
/// Flow:
/// 1. Validate client_id and redirect_uri
/// 2. If user not authenticated, redirect to /threshold/login with OAuth params
/// 3. If authenticated, generate auth code and redirect to redirect_uri
async fn handle_authorize(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    // Parse query parameters
    let query_str = req.uri().query().unwrap_or("");
    let params: OAuthAuthorizeRequest = match serde_urlencoded::from_str(query_str) {
        Ok(p) => p,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &OAuthErrorResponse {
                    error: "invalid_request".to_string(),
                    error_description: Some(format!("Invalid query parameters: {e}")),
                    state: None,
                },
            )
        }
    };

    // Validate response_type
    if params.response_type != "code" {
        return json_response(
            StatusCode::BAD_REQUEST,
            &OAuthErrorResponse {
                error: "unsupported_response_type".to_string(),
                error_description: Some("Only 'code' response type is supported".to_string()),
                state: Some(params.state),
            },
        );
    }

    // Validate client_id
    let clients = get_registered_clients();
    let client = match clients.iter().find(|c| c.client_id == params.client_id) {
        Some(c) => c,
        None => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &OAuthErrorResponse {
                    error: "invalid_client".to_string(),
                    error_description: Some(format!("Unknown client_id: {}", params.client_id)),
                    state: Some(params.state),
                },
            );
        }
    };

    // Validate redirect_uri
    if !validate_redirect_uri(client, &params.redirect_uri) {
        return json_response(
            StatusCode::BAD_REQUEST,
            &OAuthErrorResponse {
                error: "invalid_redirect_uri".to_string(),
                error_description: Some("Redirect URI not allowed for this client".to_string()),
                state: Some(params.state),
            },
        );
    }

    // Check if this is an AJAX/fetch request (Bearer token = SPA calling us)
    // SPA fetch requests can't follow cross-origin redirects due to CORS,
    // so we return JSON with the redirect URL instead of a 302.
    let is_ajax = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.starts_with("Bearer "))
        .unwrap_or(false);

    // Check if user is already authenticated (via cookie or header)
    let auth_header = get_auth_header(&req);
    let token = extract_token_from_header(auth_header);

    if let Some(token) = token {
        // User is authenticated - verify token and generate auth code
        let jwt = match get_jwt_validator(&state) {
            Ok(j) => j,
            Err(resp) => return resp,
        };

        let result = jwt.verify_token(token);
        if result.valid {
            let claims = result.claims.unwrap();

            // Generate authorization code
            let code = generate_auth_code();

            // Store in MongoDB
            if let Some(mongo) = &state.mongo {
                let session = OAuthSessionDoc::new(
                    code.clone(),
                    params.client_id.clone(),
                    params.redirect_uri.clone(),
                    params.state.clone(),
                    params.scope.clone(),
                    claims.human_id.clone(),
                    claims.agent_pub_key.clone(),
                    claims.identifier.clone(),
                );

                if let Ok(collection) = mongo
                    .collection::<OAuthSessionDoc>(OAUTH_SESSION_COLLECTION)
                    .await
                {
                    if let Err(e) = collection.insert_one(session).await {
                        warn!("Failed to store OAuth session: {}", e);
                        return json_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            &OAuthErrorResponse {
                                error: "server_error".to_string(),
                                error_description: Some(
                                    "Failed to create authorization".to_string(),
                                ),
                                state: Some(params.state),
                            },
                        );
                    }
                }
            }

            // Redirect to client with code
            let redirect_url = format!(
                "{}{}code={}&state={}",
                params.redirect_uri,
                if params.redirect_uri.contains('?') {
                    "&"
                } else {
                    "?"
                },
                urlencoding::encode(&code),
                urlencoding::encode(&params.state)
            );

            info!(
                "OAuth authorize: {} {} to client with code",
                if is_ajax {
                    "returning redirect_uri to"
                } else {
                    "redirecting"
                },
                claims.identifier
            );

            // SPA fetch can't follow cross-origin 302s (CORS), so return JSON
            if is_ajax {
                return json_response(
                    StatusCode::OK,
                    &serde_json::json!({ "redirect_uri": redirect_url }),
                );
            }

            return Response::builder()
                .status(StatusCode::FOUND)
                .header("Location", redirect_url)
                .header("Cache-Control", "no-store")
                .body(empty_body())
                .unwrap();
        }
    }

    // User not authenticated - redirect to login page with OAuth params
    // The login page will handle authentication and then call /auth/authorize again
    let mut login_params = vec![
        ("client_id", params.client_id.as_str()),
        ("redirect_uri", params.redirect_uri.as_str()),
        ("response_type", params.response_type.as_str()),
        ("state", params.state.as_str()),
        ("scope", params.scope.as_deref().unwrap_or("")),
    ];
    if let Some(ref hint) = params.login_hint {
        login_params.push(("login_hint", hint.as_str()));
    }
    let login_url = format!(
        "/threshold/login?{}",
        serde_urlencoded::to_string(&login_params).unwrap_or_default()
    );

    info!("OAuth authorize: redirecting to login page");

    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", login_url)
        .header("Cache-Control", "no-store")
        .body(empty_body())
        .unwrap()
}

/// POST /auth/token
///
/// OAuth 2.0 token endpoint. Exchanges an authorization code for an access token.
///
/// Request Body (x-www-form-urlencoded or JSON):
/// - grant_type: Must be "authorization_code"
/// - code: Authorization code from /auth/authorize
/// - redirect_uri: Must match the original redirect_uri
/// - client_id: OAuth client ID
///
/// Response:
/// - access_token: JWT token for API access
/// - token_type: "Bearer"
/// - expires_in: Token lifetime in seconds
/// - human_id, agent_pub_key, identifier: Holochain identity info
async fn handle_token(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    // Parse request body (support both JSON and form-urlencoded)
    // Clone content-type before consuming request
    let content_type = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let body_bytes = match req.collect().await {
        Ok(b) => b.to_bytes(),
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &OAuthErrorResponse {
                    error: "invalid_request".to_string(),
                    error_description: Some(format!("Failed to read body: {e}")),
                    state: None,
                },
            )
        }
    };

    let token_req: OAuthTokenRequest = if content_type.contains("application/json") {
        match serde_json::from_slice(&body_bytes) {
            Ok(r) => r,
            Err(e) => {
                return json_response(
                    StatusCode::BAD_REQUEST,
                    &OAuthErrorResponse {
                        error: "invalid_request".to_string(),
                        error_description: Some(format!("Invalid JSON: {e}")),
                        state: None,
                    },
                )
            }
        }
    } else {
        // Assume form-urlencoded
        match serde_urlencoded::from_bytes(&body_bytes) {
            Ok(r) => r,
            Err(e) => {
                return json_response(
                    StatusCode::BAD_REQUEST,
                    &OAuthErrorResponse {
                        error: "invalid_request".to_string(),
                        error_description: Some(format!("Invalid form data: {e}")),
                        state: None,
                    },
                )
            }
        }
    };

    // Validate grant_type
    if token_req.grant_type != "authorization_code" {
        return json_response(
            StatusCode::BAD_REQUEST,
            &OAuthErrorResponse {
                error: "unsupported_grant_type".to_string(),
                error_description: Some(
                    "Only 'authorization_code' grant type is supported".to_string(),
                ),
                state: None,
            },
        );
    }

    // Validate client_id
    let clients = get_registered_clients();
    if !clients.iter().any(|c| c.client_id == token_req.client_id) {
        return json_response(
            StatusCode::BAD_REQUEST,
            &OAuthErrorResponse {
                error: "invalid_client".to_string(),
                error_description: Some(format!("Unknown client_id: {}", token_req.client_id)),
                state: None,
            },
        );
    }

    // In dev mode without MongoDB, use simplified flow
    if state.args.dev_mode && state.mongo.is_none() {
        info!("OAuth token exchange (dev mode, no MongoDB)");
        let jwt = match get_jwt_validator(&state) {
            Ok(j) => j,
            Err(resp) => return resp,
        };

        // Dev mode has no UserDoc to consult for stewardship; treat as
        // non-steward so the probe is skipped and portal_host_url stays absent.
        return generate_oauth_token_response(
            &jwt,
            &state,
            "dev-human-id",
            "uhCAk-dev-mode-agent-key",
            "dev@example.com",
            false,
        )
        .await;
    }

    // Look up authorization code in MongoDB
    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return json_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &OAuthErrorResponse {
                    error: "server_error".to_string(),
                    error_description: Some("Database not available".to_string()),
                    state: None,
                },
            )
        }
    };

    let collection = match mongo
        .collection::<OAuthSessionDoc>(OAUTH_SESSION_COLLECTION)
        .await
    {
        Ok(c) => c,
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &OAuthErrorResponse {
                    error: "server_error".to_string(),
                    error_description: Some(format!("Database error: {e}")),
                    state: None,
                },
            )
        }
    };

    // Find the session by code
    let session = match collection.find_one(doc! { "code": &token_req.code }).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            warn!("OAuth token exchange: code not found");
            return json_response(
                StatusCode::BAD_REQUEST,
                &OAuthErrorResponse {
                    error: "invalid_grant".to_string(),
                    error_description: Some("Authorization code not found or expired".to_string()),
                    state: None,
                },
            );
        }
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &OAuthErrorResponse {
                    error: "server_error".to_string(),
                    error_description: Some(format!("Database error: {e}")),
                    state: None,
                },
            )
        }
    };

    // Validate session
    if !session.is_valid() {
        warn!("OAuth token exchange: code expired or already used");
        return json_response(
            StatusCode::BAD_REQUEST,
            &OAuthErrorResponse {
                error: "invalid_grant".to_string(),
                error_description: Some("Authorization code expired or already used".to_string()),
                state: None,
            },
        );
    }

    // Validate redirect_uri matches
    if session.redirect_uri != token_req.redirect_uri {
        warn!("OAuth token exchange: redirect_uri mismatch");
        return json_response(
            StatusCode::BAD_REQUEST,
            &OAuthErrorResponse {
                error: "invalid_grant".to_string(),
                error_description: Some("Redirect URI does not match".to_string()),
                state: None,
            },
        );
    }

    // Validate client_id matches
    if session.client_id != token_req.client_id {
        warn!("OAuth token exchange: client_id mismatch");
        return json_response(
            StatusCode::BAD_REQUEST,
            &OAuthErrorResponse {
                error: "invalid_grant".to_string(),
                error_description: Some("Client ID does not match".to_string()),
                state: None,
            },
        );
    }

    // Mark code as used
    if let Err(e) = collection
        .update_one(
            doc! { "code": &token_req.code },
            doc! { "$set": { "used": true } },
        )
        .await
    {
        warn!("Failed to mark OAuth code as used: {}", e);
    }

    info!("OAuth token exchange successful: {}", session.identifier);

    let jwt = match get_jwt_validator(&state) {
        Ok(j) => j,
        Err(resp) => return resp,
    };

    // Derive stewardship from the substrate UserDoc — the OAuthSessionDoc does
    // not carry it. This mirrors the login path, which reads UserDoc.is_steward
    // before calling generate_auth_response. A lookup miss or error degrades to
    // non-steward, so the portal-host probe is simply skipped and the token
    // exchange is never blocked or delayed by an absent/erroring user record.
    let is_steward = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(users) => users
            .find_one(doc! { "identifier": &session.identifier })
            .await
            .ok()
            .flatten()
            .map(|u| u.is_steward)
            .unwrap_or(false),
        Err(e) => {
            warn!(
                identifier = %session.identifier,
                error = %e,
                "OAuth token exchange: UserDoc lookup failed; treating as non-steward"
            );
            false
        }
    };

    generate_oauth_token_response(
        &jwt,
        &state,
        &session.human_id,
        &session.agent_pub_key,
        &session.identifier,
        is_steward,
    )
    .await
}

/// Generate a random authorization code.
fn generate_auth_code() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}

/// Generate OAuth token response with JWT.
///
/// When `is_steward` is true, the function opportunistically probes the
/// human's registered portal hosts (1 s timeout per host) and populates
/// `portal_host_url` with the first reachable one. This mirrors the login
/// path's `generate_auth_response` so the OAuth-code consumer can hand the
/// session off to the peer-native portal exactly as the login consumer does.
///
/// A probe failure or timeout degrades silently to `None` — it MUST never
/// error or delay-fail the token exchange (the OAuth `code`+`state` redirect
/// contract is untouched by this additive metadata).
async fn generate_oauth_token_response(
    jwt: &JwtValidator,
    state: &AppState,
    human_id: &str,
    agent_pub_key: &str,
    identifier: &str,
    is_steward: bool,
) -> Response<BoxBody> {
    let doorway_id = state.args.doorway_id.clone();
    let doorway_url = state.args.doorway_url.clone();

    // OAuth tokens don't get session_id - they're used for different purposes
    // (authorization grants, not direct signing key access)
    let input = TokenInput {
        human_id: human_id.to_string(),
        agent_pub_key: agent_pub_key.to_string(),
        identifier: identifier.to_string(),
        permission_level: PermissionLevel::Authenticated,
        session_id: None,
        doorway_id: doorway_id.clone(),
        doorway_url: doorway_url.clone(),
        conductor_id: None,
        installed_app_id: None,
        is_steward,
        has_local_conductor: false,
    };

    // Opportunistic portal-host probe for graduated stewards — same helper the
    // login path uses. Non-stewards skip the probe entirely.
    let portal_host_url = if is_steward {
        let storage_base = state
            .args
            .storage_url
            .as_deref()
            .unwrap_or("http://127.0.0.1:8090");
        let overrides = state.portal_health_override.read().await;
        probe_first_portal_host(storage_base, agent_pub_key, state.args.dev_mode, &overrides).await
    } else {
        None
    };

    match jwt.generate_token(input) {
        Ok(token) => {
            let claims = jwt.verify_token(&token);
            let expires_in = claims
                .claims
                .map(|c| c.exp.saturating_sub(c.iat))
                .unwrap_or(3600);

            json_response(
                StatusCode::OK,
                &OAuthTokenResponse {
                    access_token: token,
                    token_type: "Bearer".to_string(),
                    expires_in,
                    refresh_token: None, // Could add refresh token support
                    human_id: human_id.to_string(),
                    agent_pub_key: agent_pub_key.to_string(),
                    identifier: identifier.to_string(),
                    doorway_id,
                    doorway_url,
                    portal_host_url,
                },
            )
        }
        Err(e) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &OAuthErrorResponse {
                error: "server_error".to_string(),
                error_description: Some(format!("Failed to generate token: {e}")),
                state: None,
            },
        ),
    }
}

// =============================================================================
// Portal-Host Handler
// =============================================================================

/// Minimal projection of a storage PortalHostView — only the fields we need
/// for the probe loop.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoragePortalHostRow {
    host_url: String,
}

/// GET /auth/portal-host
///
/// Returns the first reachable portal host registered for the authenticated
/// human.
///
/// Flow:
/// 1. Validate Bearer JWT
/// 2. GET {storage_url}/api/v1/account/portal-hosts with X-Agent-Id header
/// 3. HEAD-probe each host's /healthz with a 1 s timeout
/// 4. Return PortalHostResponse (200 even when nothing is reachable)
async fn handle_portal_host(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    let auth_header = get_auth_header(&req);
    let token = match extract_token_from_header(auth_header) {
        Some(t) => t,
        None => {
            return json_response(
                StatusCode::UNAUTHORIZED,
                &ErrorResponse {
                    error: "No token provided".into(),
                    code: Some("NO_TOKEN".into()),
                },
            )
        }
    };

    let jwt = match get_jwt_validator(&state) {
        Ok(j) => j,
        Err(resp) => return resp,
    };

    let result = jwt.verify_token(token);
    if !result.valid {
        return json_response(
            StatusCode::UNAUTHORIZED,
            &ErrorResponse {
                error: result
                    .error
                    .unwrap_or_else(|| "Invalid or expired token".into()),
                code: Some("INVALID_TOKEN".into()),
            },
        );
    }

    let claims = result.claims.unwrap();

    // Build storage URL for this human's portal hosts.
    let storage_base = state
        .args
        .storage_url
        .as_deref()
        .unwrap_or("http://127.0.0.1:8090");
    let storage_url = format!(
        "{}/api/v1/account/portal-hosts",
        storage_base.trim_end_matches('/')
    );

    let client = reqwest::Client::new();

    // Fetch hosts from storage; on any failure return { reachable: false }.
    let hosts: Vec<StoragePortalHostRow> = match client
        .get(&storage_url)
        .header("X-Agent-Id", &claims.agent_pub_key)
        .send()
        .await
        .and_then(|r| r.error_for_status())
    {
        Ok(resp) => resp.json().await.unwrap_or_default(),
        Err(e) => {
            warn!(error = %e, "Failed to fetch portal hosts from storage");
            return json_response(
                StatusCode::OK,
                &crate::auth::portal_host::PortalHostResponse {
                    reachable: false,
                    host_url: None,
                    all_hosts: vec![],
                },
            );
        }
    };

    // Probe each host; pick the first one that replies with 2xx.
    let mut all_urls: Vec<String> = Vec::with_capacity(hosts.len());
    let mut chosen: Option<String> = None;

    for h in &hosts {
        all_urls.push(h.host_url.clone());
        if chosen.is_none() {
            let probe_url = format!("{}/healthz", h.host_url.trim_end_matches('/'));
            let probe = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                client.head(&probe_url).send(),
            )
            .await;
            if let Ok(Ok(resp)) = probe {
                if resp.status().is_success() {
                    chosen = Some(h.host_url.clone());
                }
            }
        }
    }

    json_response(
        StatusCode::OK,
        &crate::auth::portal_host::PortalHostResponse {
            reachable: chosen.is_some(),
            host_url: chosen,
            all_hosts: all_urls,
        },
    )
}

/// Per-host probe decision — the pure core of `probe_first_portal_host`.
///
/// Split out so the dev-mode-override branch is unit-testable without a live
/// HTTP server. The live HEAD itself (`ProbeLive`) stays in the async loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PortalProbeDecision {
    /// Treat this host as reachable — return it immediately, no live HEAD.
    Return,
    /// Treat this host as unreachable — skip it, no live HEAD.
    Skip,
    /// No override applies — fall back to the real live HEAD probe.
    ProbeLive,
}

/// Decide how to probe a single portal host.
///
/// The **only** behaviour change from the original live-only probe is gated
/// behind `dev_mode`: when on AND the host has a doorway-local health override,
/// the override flag short-circuits the live HEAD (`Return`/`Skip`). With
/// `dev_mode` off, OR with no override entry for this host, the result is always
/// `ProbeLive` — so production behaviour is byte-for-byte unchanged.
///
/// This consults doorway-local OPERATIONAL state (`AppState::portal_health_override`),
/// never the notarized `portal_hosts` DHT entry. See that field's doc comment.
fn portal_probe_decision(
    dev_mode: bool,
    override_map: &std::collections::HashMap<String, bool>,
    host_url: &str,
) -> PortalProbeDecision {
    if dev_mode {
        if let Some(&healthy) = override_map.get(host_url) {
            return if healthy {
                PortalProbeDecision::Return
            } else {
                PortalProbeDecision::Skip
            };
        }
    }
    PortalProbeDecision::ProbeLive
}

/// Probe a portal host URL and return the URL if reachable, or None.
///
/// Used by the login / OAuth-token / session-exchange paths to opportunistically
/// populate `portal_host_url` without a full `handle_portal_host` round-trip.
///
/// `dev_mode` + `override_map` thread doorway-local OPERATIONAL health state
/// (`AppState::portal_health_override`) into the per-host decision via
/// `portal_probe_decision`. When `dev_mode` is off the override is ignored and
/// every host takes the original live HEAD path — production is unchanged.
async fn probe_first_portal_host(
    storage_base: &str,
    agent_pub_key: &str,
    dev_mode: bool,
    override_map: &std::collections::HashMap<String, bool>,
) -> Option<String> {
    let storage_url = format!(
        "{}/api/v1/account/portal-hosts",
        storage_base.trim_end_matches('/')
    );
    let client = reqwest::Client::new();
    let hosts: Vec<StoragePortalHostRow> = client
        .get(&storage_url)
        .header("X-Agent-Id", agent_pub_key)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .ok()?
        .json()
        .await
        .ok()?;

    for h in &hosts {
        match portal_probe_decision(dev_mode, override_map, &h.host_url) {
            PortalProbeDecision::Return => return Some(h.host_url.clone()),
            PortalProbeDecision::Skip => continue,
            PortalProbeDecision::ProbeLive => {
                let probe_url = format!("{}/healthz", h.host_url.trim_end_matches('/'));
                let probe = tokio::time::timeout(
                    std::time::Duration::from_secs(1),
                    client.head(&probe_url).send(),
                )
                .await;
                if let Ok(Ok(resp)) = probe {
                    if resp.status().is_success() {
                        return Some(h.host_url.clone());
                    }
                }
            }
        }
    }
    None
}

// =============================================================================
// Session Transfer Handlers
// =============================================================================

/// GET /auth/session-token
///
/// Given a valid Bearer token, issues a short-lived (60 s) single-use transfer
/// token that doorway-app can exchange for a full JWT without re-login.
///
/// Flow:
/// 1. Validate Bearer token from Authorization header
/// 2. Generate a random UUID transfer token
/// 3. Store identity snapshot with 60 s TTL in the in-memory store
/// 4. Opportunistically sweep expired/consumed entries
/// 5. Return { sessionToken, expiresAt }
async fn handle_session_token(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    let auth_header = get_auth_header(&req);
    let token = match extract_token_from_header(auth_header) {
        Some(t) => t,
        None => {
            return json_response(
                StatusCode::UNAUTHORIZED,
                &ErrorResponse {
                    error: "No token provided".into(),
                    code: Some("NO_TOKEN".into()),
                },
            )
        }
    };

    let jwt = match get_jwt_validator(&state) {
        Ok(j) => j,
        Err(resp) => return resp,
    };

    let result = jwt.verify_token(token);
    if !result.valid {
        return json_response(
            StatusCode::UNAUTHORIZED,
            &ErrorResponse {
                error: result
                    .error
                    .unwrap_or_else(|| "Invalid or expired token".into()),
                code: Some("INVALID_TOKEN".into()),
            },
        );
    }

    let claims = result.claims.unwrap();

    // Generate a random single-use transfer token.
    let transfer_token = uuid::Uuid::new_v4().to_string();
    let ttl = std::time::Duration::from_secs(60);
    let expires_at_instant = Instant::now() + ttl;

    // Unix timestamp for the client (Instant is not serializable).
    let expires_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() + 60)
        .unwrap_or(0);

    let entry = SessionTransferEntry {
        human_id: claims.human_id,
        agent_pub_key: claims.agent_pub_key,
        identifier: claims.identifier,
        permission_level: claims.permission_level,
        session_id: claims.session_id,
        conductor_id: claims.conductor_id,
        installed_app_id: claims.installed_app_id,
        is_steward: claims.is_steward,
        has_local_conductor: claims.has_local_conductor,
        doorway_id: claims.doorway_id,
        doorway_url: claims.doorway_url,
        expires_at: expires_at_instant,
        consumed: false,
    };

    let store = session_transfer_store();
    {
        let mut map = store.write().await;

        // Opportunistic cleanup: remove expired or consumed entries.
        let now = Instant::now();
        map.retain(|_, v| !v.consumed && v.expires_at > now);

        map.insert(transfer_token.clone(), entry);
    }

    info!("Issued session transfer token for cross-app handoff");

    json_response(
        StatusCode::OK,
        &SessionTokenResponse {
            session_token: transfer_token,
            expires_at: expires_at_unix,
        },
    )
}

/// GET /auth/exchange-session?session_token=xxx
///
/// Validates a transfer token issued by /auth/session-token and returns a
/// full JWT.  The token is marked consumed on first use.
///
/// Flow:
/// 1. Parse `session_token` query parameter
/// 2. Look up in store — reject if missing, expired, or already consumed
/// 3. Mark as consumed
/// 4. Generate a fresh JWT from the stored identity snapshot
/// 5. Return full auth response
async fn handle_exchange_session(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    // Parse the session_token query parameter.
    let query = req.uri().query().unwrap_or("");
    let session_token = serde_urlencoded::from_str::<HashMap<String, String>>(query)
        .ok()
        .and_then(|m| m.get("session_token").cloned());

    let session_token = match session_token {
        Some(t) if !t.is_empty() => t,
        _ => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: "Missing required query parameter: session_token".into(),
                    code: Some("MISSING_PARAM".into()),
                },
            )
        }
    };

    let store = session_transfer_store();

    // Lock, validate, and consume atomically.
    let entry_snapshot = {
        let mut map = store.write().await;
        match map.get_mut(&session_token) {
            None => {
                return json_response(
                    StatusCode::UNAUTHORIZED,
                    &ErrorResponse {
                        error: "Invalid or unknown session token".into(),
                        code: Some("INVALID_SESSION_TOKEN".into()),
                    },
                )
            }
            Some(entry) => {
                if entry.consumed {
                    return json_response(
                        StatusCode::UNAUTHORIZED,
                        &ErrorResponse {
                            error: "Session token has already been used".into(),
                            code: Some("TOKEN_CONSUMED".into()),
                        },
                    );
                }
                if Instant::now() > entry.expires_at {
                    return json_response(
                        StatusCode::UNAUTHORIZED,
                        &ErrorResponse {
                            error: "Session token has expired".into(),
                            code: Some("TOKEN_EXPIRED".into()),
                        },
                    );
                }
                // Mark consumed before releasing the lock.
                entry.consumed = true;
                // Clone the identity fields we need to generate the JWT.
                (
                    entry.human_id.clone(),
                    entry.agent_pub_key.clone(),
                    entry.identifier.clone(),
                    entry.permission_level,
                    entry.session_id.clone(),
                    entry.conductor_id.clone(),
                    entry.installed_app_id.clone(),
                    entry.is_steward,
                    entry.has_local_conductor,
                    entry.doorway_id.clone(),
                    entry.doorway_url.clone(),
                )
            }
        }
    };

    let (
        human_id,
        agent_pub_key,
        identifier,
        permission_level,
        session_id,
        conductor_id,
        installed_app_id,
        is_steward,
        has_local_conductor,
        doorway_id,
        doorway_url,
    ) = entry_snapshot;

    let jwt = match get_jwt_validator(&state) {
        Ok(j) => j,
        Err(resp) => return resp,
    };

    let input = TokenInput {
        human_id: human_id.clone(),
        agent_pub_key: agent_pub_key.clone(),
        identifier: identifier.clone(),
        permission_level,
        session_id,
        doorway_id: doorway_id.clone(),
        doorway_url: doorway_url.clone(),
        conductor_id,
        installed_app_id,
        is_steward,
        has_local_conductor,
    };

    match jwt.generate_token(input) {
        Ok(new_token) => {
            let verification = jwt.verify_token(&new_token);
            let expires_at = verification.claims.map(|c| c.exp).unwrap_or(0);

            // If the human is a steward, opportunistically probe for a
            // reachable portal host so the receiving app can redirect
            // immediately without a separate /auth/portal-host round-trip.
            let portal_host_url = if is_steward {
                let storage_base = state
                    .args
                    .storage_url
                    .as_deref()
                    .unwrap_or("http://127.0.0.1:8090");
                let overrides = state.portal_health_override.read().await;
                probe_first_portal_host(
                    storage_base,
                    &agent_pub_key,
                    state.args.dev_mode,
                    &overrides,
                )
                .await
            } else {
                None
            };

            info!("Session token exchanged for JWT (user: {})", identifier);

            json_response(
                StatusCode::OK,
                &ExchangeSessionResponse {
                    token: new_token,
                    human_id,
                    agent_pub_key,
                    identifier,
                    expires_at,
                    doorway_id,
                    doorway_url,
                    portal_host_url,
                },
            )
        }
        Err(e) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &ErrorResponse {
                error: format!("Failed to generate token: {e}"),
                code: Some("TOKEN_ERROR".into()),
            },
        ),
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

#[allow(clippy::result_large_err)]
fn get_jwt_validator(state: &AppState) -> Result<JwtValidator, Response<BoxBody>> {
    if state.args.dev_mode {
        Ok(JwtValidator::new_dev())
    } else {
        match &state.args.jwt_secret {
            Some(secret) => JwtValidator::new(secret.clone(), state.args.jwt_expiry_seconds)
                .map_err(|e| {
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &ErrorResponse {
                            error: format!("JWT configuration error: {e}"),
                            code: Some("CONFIG_ERROR".into()),
                        },
                    )
                }),
            None => Err(json_response(
                StatusCode::NOT_IMPLEMENTED,
                &ErrorResponse {
                    error: "Authentication not enabled (missing JWT_SECRET)".into(),
                    code: Some("NOT_ENABLED".into()),
                },
            )),
        }
    }
}

/// Generate a successful auth response with JWT token.
///
/// When `is_steward` is true, the function opportunistically probes the
/// human's registered portal hosts and populates `portal_host_url` with the
/// first reachable one (1 s timeout per host). The client uses this to hand
/// the session off to the peer-native OAuth portal — doorway is the relying
/// party, the portal host is the identity provider. Mirrors the probe used
/// by `handle_exchange_session`.
#[allow(clippy::too_many_arguments)]
async fn generate_auth_response(
    jwt: &JwtValidator,
    state: &AppState,
    human_id: &str,
    agent_pub_key: &str,
    identifier: &str,
    session_id: Option<String>,
    status: StatusCode,
    profile: Option<HumanProfileResponse>,
    permission_level: PermissionLevel,
    installed_app_id: Option<String>,
    conductor_id: Option<String>,
    is_steward: bool,
    has_local_conductor: bool,
) -> Response<BoxBody> {
    // Get doorway identity from config
    let doorway_id = state.args.doorway_id.clone();
    let doorway_url = state.args.doorway_url.clone();

    let input = TokenInput {
        human_id: human_id.to_string(),
        agent_pub_key: agent_pub_key.to_string(),
        identifier: identifier.to_string(),
        permission_level,
        session_id,
        doorway_id: doorway_id.clone(),
        doorway_url: doorway_url.clone(),
        conductor_id,
        installed_app_id: installed_app_id.clone(),
        is_steward,
        has_local_conductor,
    };

    let portal_host_url = if is_steward {
        let storage_base = state
            .args
            .storage_url
            .as_deref()
            .unwrap_or("http://127.0.0.1:8090");
        let overrides = state.portal_health_override.read().await;
        probe_first_portal_host(storage_base, agent_pub_key, state.args.dev_mode, &overrides).await
    } else {
        None
    };

    match jwt.generate_token(input) {
        Ok(token) => {
            let claims = jwt.verify_token(&token);
            let expires_at = claims.claims.map(|c| c.exp).unwrap_or(0);

            json_response(
                status,
                &AuthResponse {
                    token,
                    human_id: human_id.to_string(),
                    agent_pub_key: agent_pub_key.to_string(),
                    identifier: identifier.to_string(),
                    expires_at,
                    doorway_id,
                    doorway_url,
                    installed_app_id,
                    profile,
                    is_steward,
                    portal_host_url,
                },
            )
        }
        Err(e) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &ErrorResponse {
                error: format!("Failed to generate token: {e}"),
                code: Some("TOKEN_ERROR".into()),
            },
        ),
    }
}

// =============================================================================
// Main Router
// =============================================================================

/// Handle auth-related HTTP requests.
///
/// Returns Some(response) if request was handled, None if not an auth route.
pub async fn handle_auth_request(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Option<Response<BoxBody>> {
    let path = req.uri().path();
    let method = req.method();

    // Only handle /auth/* routes
    if !path.starts_with("/auth") {
        return None;
    }

    // Remove query string for matching
    let path = path.split('?').next().unwrap_or(path);

    let response = match (method, path) {
        // Standard auth endpoints
        (&Method::POST, "/auth/register") => handle_register(req, state).await,
        (&Method::POST, "/auth/login") => handle_login(req, state).await,
        (&Method::POST, "/auth/logout") => handle_logout(req, state).await,
        (&Method::POST, "/auth/refresh") => handle_refresh(req, state).await,
        (&Method::GET, "/auth/me") => handle_me(req, state).await,
        (&Method::GET, "/auth/account") => handle_account(req, state).await,

        // OAuth 2.0 endpoints
        (&Method::GET, "/auth/authorize") => handle_authorize(req, state).await,
        (&Method::POST, "/auth/token") => handle_token(req, state).await,

        // Native handoff (Tauri session migration)
        (&Method::GET, "/auth/native-handoff") => handle_native_handoff(req, state).await,

        // Cross-app session handoff (elohim-app -> doorway-app)
        (&Method::GET, "/auth/session-token") => handle_session_token(req, state).await,
        (&Method::GET, "/auth/exchange-session") => handle_exchange_session(req, state).await,

        // Portal-host discovery (steward redirect)
        (&Method::GET, "/auth/portal-host") => handle_portal_host(req, state).await,

        // Stewardship migration endpoints
        (&Method::GET, "/auth/export-key") => handle_export_key(req, state).await,
        (&Method::POST, "/auth/confirm-stewardship")
        | (&Method::POST, "/auth/confirm-sovereignty") => {
            handle_confirm_stewardship(req, state).await
        }

        // Disaster recovery endpoints
        (&Method::POST, "/auth/recover-custody") => handle_recover_custody(req, state).await,
        (&Method::POST, "/auth/check-recovery-status") => {
            handle_check_recovery_status(req, state).await
        }
        (&Method::POST, "/auth/activate-recovery") => handle_activate_recovery(req, state).await,

        // Elohim verification endpoints
        (&Method::POST, "/auth/elohim-verify/start") => {
            handle_elohim_verify_start(req, state).await
        }
        (&Method::POST, "/auth/elohim-verify/answer") => {
            handle_elohim_verify_answer(req, state).await
        }

        // Method not allowed
        (_, "/auth/register")
        | (_, "/auth/login")
        | (_, "/auth/logout")
        | (_, "/auth/refresh")
        | (_, "/auth/me")
        | (_, "/auth/account")
        | (_, "/auth/authorize")
        | (_, "/auth/token")
        | (_, "/auth/native-handoff")
        | (_, "/auth/export-key")
        | (_, "/auth/confirm-stewardship")
        | (_, "/auth/confirm-sovereignty")
        | (_, "/auth/recover-custody")
        | (_, "/auth/check-recovery-status")
        | (_, "/auth/activate-recovery")
        | (_, "/auth/elohim-verify/start")
        | (_, "/auth/elohim-verify/answer")
        | (_, "/auth/session-token")
        | (_, "/auth/exchange-session")
        | (_, "/auth/portal-host") => json_response(
            StatusCode::METHOD_NOT_ALLOWED,
            &ErrorResponse {
                error: "Method not allowed".into(),
                code: None,
            },
        ),

        // Auth endpoint not found
        _ => json_response(
            StatusCode::NOT_FOUND,
            &ErrorResponse {
                error: "Auth endpoint not found".into(),
                code: None,
            },
        ),
    };

    Some(response)
}

/// Validate a token and extract claims for WebSocket authentication
pub fn validate_ws_token(state: &AppState, token: &str) -> Option<Claims> {
    let jwt = if state.args.dev_mode {
        JwtValidator::new_dev()
    } else {
        state
            .args
            .jwt_secret
            .as_ref()
            .and_then(|s| JwtValidator::new(s.clone(), state.args.jwt_expiry_seconds).ok())?
    };

    let result = jwt.verify_token(token);
    if result.valid {
        result.claims
    } else {
        None
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_domain_strips_scheme_port_path_and_doorway_prefix() {
        assert_eq!(
            gateway_domain(Some("https://doorway-alpha.elohim.host")),
            Some("alpha.elohim.host".to_string())
        );
        assert_eq!(
            gateway_domain(Some("https://doorway-alpha.elohim.host/bootstrap")),
            Some("alpha.elohim.host".to_string())
        );
        assert_eq!(
            gateway_domain(Some("https://doorway-alpha.elohim.host:8080")),
            Some("alpha.elohim.host".to_string())
        );
        // a host that is not `doorway-`-prefixed is returned unchanged
        assert_eq!(
            gateway_domain(Some("https://elohim.host")),
            Some("elohim.host".to_string())
        );
        // no configured URL → no domain → caller leaves the identifier untouched
        assert_eq!(gateway_domain(None), None);
    }

    #[test]
    fn normalize_identifier_is_idempotent_and_gateway_scoped() {
        let d = "alpha.elohim.host";
        // bare username gains the gateway suffix (the bug that 401'd matthew)
        assert_eq!(
            normalize_identifier("matthew.dowell", d),
            "matthew.dowell@alpha.elohim.host"
        );
        // an already-own-domain identifier is unchanged — no double-qualify
        assert_eq!(
            normalize_identifier("matthew.dowell@alpha.elohim.host", d),
            "matthew.dowell@alpha.elohim.host"
        );
        // applying twice is stable (load-bearing: the seeder stores full emails)
        let once = normalize_identifier("matthew.dowell", d);
        assert_eq!(normalize_identifier(&once, d), once);
        // a foreign domain is re-qualified to the gateway (doorway is gateway-scoped)
        assert_eq!(
            normalize_identifier("matthew.dowell@gmail.com", d),
            "matthew.dowell@alpha.elohim.host"
        );
    }

    /// Regression for genesis #1105 ("Matthew suspends a user"): under
    /// dev_mode, hosted registration skips per-user provisioning, so EVERY
    /// account persists the shared singleton agent's human_id. Suspension
    /// flips `is_active` on one account's unique row; the /auth/me revocation
    /// lookup must therefore select by the account-unique `identifier` —
    /// a human_id-keyed find_one resolves an arbitrary (typically still
    /// active) account and the suspended session keeps authenticating.
    ///
    /// This pins the decision arm + selection-key invariant in-memory; the
    /// live Mongo query is exercised by the a2o auth-lifecycle scenarios.
    #[test]
    fn revocation_targets_unique_identifier_not_human_id() {
        let shared_human_id = "human-dev-singleton".to_string();
        let mut troublemaker = UserDoc::new(
            "troublemaker@example.com".into(),
            "email".into(),
            "argon2-hash".into(),
            shared_human_id.clone(),
            "uhCAk-shared-agent".into(),
            None,
        );
        troublemaker.is_active = false; // adminSetUserStatus(active=false)
        let matthew = UserDoc::new(
            "matthew@example.com".into(),
            "email".into(),
            "argon2-hash".into(),
            shared_human_id.clone(),
            "uhCAk-shared-agent".into(),
            None,
        );
        assert_eq!(
            troublemaker.human_id, matthew.human_id,
            "dev-mode collision premise"
        );

        let accounts = [&troublemaker, &matthew];
        let by_identifier = |id: &str| accounts.iter().copied().find(|u| u.identifier == id);

        // identifier-keyed selection addresses the suspended account exactly.
        assert!(session_revoked_by_user_doc(by_identifier(
            "troublemaker@example.com"
        )));
        assert!(!session_revoked_by_user_doc(by_identifier(
            "matthew@example.com"
        )));
        // No row (legacy accounts / native agents) proceeds on JWT validity.
        assert!(!session_revoked_by_user_doc(None));
    }

    /// Verify that MeResponse serialises with the new trust-mode fields in
    /// camelCase, that `conductorEndpoint` is absent when None, and that the
    /// existing fields are unaffected.
    ///
    /// This is a serialisation-contract test — it does not exercise the HTTP
    /// handler (which requires a running conductor + MongoDB). Handler-level
    /// integration tests live in the sweettest workspace.
    #[test]
    fn me_response_serializes_trust_mode_and_authority() {
        let me = MeResponse {
            human_id: "human-matthew".into(),
            agent_pub_key: "uhCAk-test-key".into(),
            identifier: "matthew@alpha.elohim.host".into(),
            permission_level: "standard".into(),
            doorway_id: Some("alpha-elohim-host".into()),
            doorway_url: Some("https://alpha.elohim.host".into()),
            authenticated: true,
            trust_mode: "doorway-host".into(),
            authority: AuthorityRef {
                label: "alpha.elohim.host".into(),
                id: Some("alpha-elohim-host".into()),
            },
            conductor_endpoint: None,
        };

        let json = serde_json::to_value(&me).unwrap();

        // New trust-mode fields
        assert_eq!(json["authenticated"], true);
        assert_eq!(json["trustMode"], "doorway-host");
        assert_eq!(json["authority"]["label"], "alpha.elohim.host");
        assert_eq!(json["authority"]["id"], "alpha-elohim-host");
        // conductorEndpoint must be absent (skip_serializing_if = "Option::is_none")
        assert!(
            json.get("conductorEndpoint").is_none(),
            "conductorEndpoint should be omitted when None"
        );

        // Existing fields must remain intact and camelCase
        assert_eq!(json["humanId"], "human-matthew");
        assert_eq!(json["agentPubKey"], "uhCAk-test-key");
        assert_eq!(json["identifier"], "matthew@alpha.elohim.host");
        assert_eq!(json["permissionLevel"], "standard");
        assert_eq!(json["doorwayId"], "alpha-elohim-host");
        assert_eq!(json["doorwayUrl"], "https://alpha.elohim.host");
    }

    /// Verify that when doorway_id and doorway_url are absent, authority still
    /// serialises with a non-empty label and no id key.
    #[test]
    fn me_response_authority_without_doorway_fields() {
        let me = MeResponse {
            human_id: "human-dev".into(),
            agent_pub_key: "uhCAk-dev".into(),
            identifier: "dev@local".into(),
            permission_level: "admin".into(),
            doorway_id: None,
            doorway_url: None,
            authenticated: true,
            trust_mode: "doorway-host".into(),
            authority: AuthorityRef {
                label: "elohim.host".into(),
                id: None,
            },
            conductor_endpoint: None,
        };

        let json = serde_json::to_value(&me).unwrap();

        assert_eq!(json["authenticated"], true);
        assert_eq!(json["trustMode"], "doorway-host");
        assert_eq!(json["authority"]["label"], "elohim.host");
        // id must be absent when None
        assert!(
            json["authority"].get("id").is_none(),
            "authority.id should be omitted when None"
        );
        // doorwayId / doorwayUrl must be absent when None
        assert!(json.get("doorwayId").is_none());
        assert!(json.get("doorwayUrl").is_none());
    }

    /// Verify the authority label derivation helper logic used in handle_me:
    /// doorway_url hostname extraction must strip scheme and trailing slash.
    #[test]
    fn authority_label_derived_from_doorway_url_hostname() {
        // Simulate the derivation logic from handle_me
        let derive_label = |doorway_url: Option<&str>, doorway_id: Option<&str>| -> String {
            doorway_url
                .and_then(|url| {
                    url.strip_prefix("https://")
                        .or_else(|| url.strip_prefix("http://"))
                        .map(|h| h.trim_end_matches('/').to_string())
                })
                .or_else(|| doorway_id.map(str::to_string))
                .unwrap_or_else(|| "elohim.host".to_string())
        };

        assert_eq!(
            derive_label(Some("https://alpha.elohim.host"), Some("alpha-elohim-host")),
            "alpha.elohim.host"
        );
        assert_eq!(
            derive_label(
                Some("https://alpha.elohim.host/"),
                Some("alpha-elohim-host")
            ),
            "alpha.elohim.host"
        );
        assert_eq!(
            derive_label(Some("http://localhost:8888"), None),
            "localhost:8888"
        );
        // Falls back to doorway_id when url is absent
        assert_eq!(
            derive_label(None, Some("alpha-elohim-host")),
            "alpha-elohim-host"
        );
        // Falls back to generic placeholder when both absent
        assert_eq!(derive_label(None, None), "elohim.host");
    }

    /// GAP-1: When the OAuth `/auth/token` response is built for a graduated
    /// steward whose portal host was reachable, the response JSON must carry
    /// `portalHostUrl`. This mirrors the login path's `AuthResponse`
    /// (which already populates `portalHostUrl` via `probe_first_portal_host`),
    /// so the OAuth-code consumer can hand the session off to the peer-native
    /// portal exactly as the login consumer does.
    ///
    /// Serialisation-contract test: it constructs the response struct directly
    /// (the camelCase key is the wire contract the elohim-app callback reads).
    /// Full handler coverage (MongoDB code lookup + UserDoc steward derivation)
    /// lives in the sweettest / integration surface.
    #[test]
    fn oauth_token_response_carries_portal_host_url_for_steward() {
        let resp = OAuthTokenResponse {
            access_token: "jwt-token".into(),
            token_type: "Bearer".into(),
            expires_in: 3600,
            refresh_token: None,
            human_id: "human-matthew".into(),
            agent_pub_key: "uhCAk-matthew".into(),
            identifier: "matthew@alpha.elohim.host".into(),
            doorway_id: Some("alpha-elohim-host".into()),
            doorway_url: Some("https://alpha.elohim.host".into()),
            portal_host_url: Some("https://matthew.steward.example".into()),
        };

        let json = serde_json::to_value(&resp).unwrap();

        // The portal-host handoff hint must be present, camelCase, exactly as
        // the login path's AuthResponse exposes it.
        assert_eq!(
            json["portalHostUrl"], "https://matthew.steward.example",
            "steward token exchange must surface portalHostUrl for handoff"
        );

        // RFC 6749 standard fields must remain snake_case and intact — the
        // portal_host_url addition is additive metadata only.
        assert_eq!(json["access_token"], "jwt-token");
        assert_eq!(json["token_type"], "Bearer");
        assert_eq!(json["expires_in"], 3600);
        assert_eq!(json["human_id"], "human-matthew");
        assert_eq!(json["agent_pub_key"], "uhCAk-matthew");
    }

    /// GAP-1: A non-steward (or a steward whose portal host was unreachable)
    /// produces `portal_host_url: None`, which `skip_serializing_if` must omit
    /// from the wire entirely — the absent field signals "complete the session
    /// locally" to the client, exactly as the login path does.
    #[test]
    fn oauth_token_response_omits_portal_host_url_for_non_steward() {
        let resp = OAuthTokenResponse {
            access_token: "jwt-token".into(),
            token_type: "Bearer".into(),
            expires_in: 3600,
            refresh_token: None,
            human_id: "human-susan".into(),
            agent_pub_key: "uhCAk-susan".into(),
            identifier: "susan@alpha.elohim.host".into(),
            doorway_id: Some("alpha-elohim-host".into()),
            doorway_url: Some("https://alpha.elohim.host".into()),
            portal_host_url: None,
        };

        let json = serde_json::to_value(&resp).unwrap();

        assert!(
            json.get("portalHostUrl").is_none(),
            "portalHostUrl must be omitted when None (skip_serializing_if)"
        );
        // Also assert the snake_case form is absent, guarding against a
        // rename regression that would emit both or the wrong key.
        assert!(
            json.get("portal_host_url").is_none(),
            "portal_host_url (snake_case) must never appear on the wire"
        );

        // RFC fields and the existing custom fields are unaffected.
        assert_eq!(json["access_token"], "jwt-token");
        assert_eq!(json["human_id"], "human-susan");
    }

    // -----------------------------------------------------------------------
    // F1: DEV_MODE portal-host health-probe override
    //
    // The override decision is split into a pure helper so it is unit-testable
    // without a live HTTP server. The live HEAD path (ProbeLive) is exercised
    // by the steward-login-portal-handoff a2o scenarios.
    // -----------------------------------------------------------------------

    use std::collections::HashMap;

    const HOST: &str = "https://matthew.steward.example/account";

    #[test]
    fn dev_mode_override_healthy_returns_without_live_probe() {
        let mut overrides = HashMap::new();
        overrides.insert(HOST.to_string(), true);
        assert_eq!(
            portal_probe_decision(true, &overrides, HOST),
            PortalProbeDecision::Return,
            "dev_mode + override healthy=true must short-circuit to Return (no live HEAD)"
        );
    }

    #[test]
    fn dev_mode_override_unhealthy_skips_without_live_probe() {
        let mut overrides = HashMap::new();
        overrides.insert(HOST.to_string(), false);
        assert_eq!(
            portal_probe_decision(true, &overrides, HOST),
            PortalProbeDecision::Skip,
            "dev_mode + override healthy=false must Skip this host (no live HEAD)"
        );
    }

    #[test]
    fn dev_mode_absent_override_falls_back_to_live_probe() {
        let overrides: HashMap<String, bool> = HashMap::new();
        assert_eq!(
            portal_probe_decision(true, &overrides, HOST),
            PortalProbeDecision::ProbeLive,
            "dev_mode but no override for this host must fall back to the live HEAD probe"
        );
    }

    #[test]
    fn prod_mode_ignores_override_entirely() {
        // Even with an override present, dev_mode OFF must take the live path —
        // production behaviour is byte-for-byte unchanged.
        let mut overrides = HashMap::new();
        overrides.insert(HOST.to_string(), true);
        assert_eq!(
            portal_probe_decision(false, &overrides, HOST),
            PortalProbeDecision::ProbeLive,
            "dev_mode OFF must ignore the override map and always probe live"
        );

        overrides.insert(HOST.to_string(), false);
        assert_eq!(
            portal_probe_decision(false, &overrides, HOST),
            PortalProbeDecision::ProbeLive,
            "dev_mode OFF must ignore an unhealthy override too"
        );
    }
}
