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
use std::sync::Arc;
use tracing::{info, warn};

use crate::auth::{
    extract_token_from_header, hash_password, verify_password, Claims, JwtValidator,
    PermissionLevel, TokenInput,
};
use crate::custodial_keys::{CustodialKeyService, KeyExportFormat};
use crate::db::schemas::{
    get_registered_clients, validate_redirect_uri, CustodialKeyMaterial, OAuthSessionDoc, UserDoc,
    OAUTH_SESSION_COLLECTION, USER_COLLECTION,
};
use crate::server::AppState;
use crate::types::DoorwayError;
use crate::routes::zome_helpers::{call_create_human, get_agent_pub_key, CreateHumanInput};
use rand::Rng;

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

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
    /// Human profile (returned on registration)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<HumanProfileResponse>,
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
/// Returns identity info for Tauri to create a local session.
/// Content syncs via P2P (DHT gossip), not from doorway.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeHandoffResponse {
    /// Holochain Human ID
    pub human_id: String,
    /// User identifier (email/username)
    pub identifier: String,
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
}

// =============================================================================
// Key Export Types (Sovereignty Migration)
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

/// Request to confirm sovereignty migration.
/// Called by Tauri app after successful key import.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmSovereigntyRequest {
    /// Signature proving possession of the key (signs the human_id)
    pub signature: String,
}

// =============================================================================
// Recovery Request/Response Types
// =============================================================================

/// Request to initiate disaster recovery for a sovereign user.
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

/// Response for sovereignty confirmation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SovereigntyConfirmedResponse {
    pub success: bool,
    pub message: String,
    /// When the user became sovereign
    pub sovereignty_at: String,
}

// =============================================================================
// Response Helpers
// =============================================================================

fn json_response<T: Serialize>(status: StatusCode, body: &T) -> Response<BoxBody> {
    let json = serde_json::to_string(body).unwrap_or_else(|_| "{}".to_string());

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type, Authorization")
        .body(full_body(json))
        .unwrap()
}

fn cors_preflight() -> Response<BoxBody> {
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type, Authorization")
        .header("Access-Control-Max-Age", "86400")
        .body(empty_body())
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
        .map_err(|e| DoorwayError::Http(format!("Failed to read body: {}", e)))?;

    let bytes = body.to_bytes();
    if bytes.len() > 10240 {
        return Err(DoorwayError::Http("Request body too large".into()));
    }

    serde_json::from_slice(&bytes)
        .map_err(|e| DoorwayError::Http(format!("Invalid JSON: {}", e)))
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
    let body: RegisterRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid JSON body: {}", e),
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

    // Determine display name for registration
    let display_name = if body.display_name.is_empty() {
        body.identifier.split('@').next().unwrap_or("User").to_string()
    } else {
        body.display_name.clone()
    };

    // For doorway-hosted registration, create identity via imagodei zome
    let (human_id, agent_pub_key, profile) = if body.human_id.is_empty() || body.agent_pub_key.is_empty() {
        // Generate UUID for human_id
        let generated_human_id = uuid::Uuid::new_v4().to_string();

        // Try to call imagodei zome (only if conductor is connected)
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
        ).await;

        match zome_result {
            Ok(human_output) => {
                // Get agent_pub_key from discovered zome config
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
                    "Created Holochain identity via imagodei zome: {} (display_name={})",
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

                (human_output.human.id, agent_key, Some(profile))
            }
            Err(e) => {
                // Zome call failed - check if we should fall back to placeholder (dev mode)
                if state.args.dev_mode {
                    warn!(
                        "Imagodei zome unavailable, using dev fallback: {}",
                        e
                    );
                    // Generate deterministic IDs for dev mode
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

                    (human_id, agent_pub_key, None)
                } else {
                    // Production mode - fail if zome unavailable
                    warn!("Failed to create identity via imagodei zome: {}", e);
                    return json_response(
                        StatusCode::SERVICE_UNAVAILABLE,
                        &ErrorResponse {
                            error: format!("Failed to create Holochain identity: {}", e),
                            code: Some("IDENTITY_CREATION_FAILED".into()),
                        },
                    );
                }
            }
        }
    } else {
        // human_id and agent_pub_key provided (legacy/external registration)
        (body.human_id.clone(), body.agent_pub_key.clone(), None)
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
        info!(
            "Dev mode register (no MongoDB): {}",
            body.identifier
        );
        return generate_auth_response(
            &jwt,
            &state,
            &human_id,
            &agent_pub_key,
            &body.identifier,
            None, // No session_id for registration (key not activated yet)
            StatusCode::CREATED,
            profile,
        );
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
                    error: format!("Database error: {}", e),
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
                    error: format!("Database error: {}", e),
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
                    error: format!("Failed to hash password: {}", e),
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

    // Use custodial key's public key as the agent_pub_key
    let actual_agent_pub_key = custodial_key.public_key.clone();

    // Create user document with custodial key
    let user = UserDoc::new_with_custodial_key(
        body.identifier.clone(),
        body.identifier_type.clone(),
        password_hash,
        human_id.clone(),
        custodial_key,
    );

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
                error: format!("Failed to create user: {}", e),
                code: Some("DB_ERROR".into()),
            },
        );
    }

    info!("Registered new user: {} with custodial key", body.identifier);

    generate_auth_response(
        &jwt,
        &state,
        &human_id,
        &actual_agent_pub_key,
        &body.identifier,
        None, // No session_id for registration (key not activated yet)
        StatusCode::CREATED,
        profile,
    )
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
    let body: LoginRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid JSON body: {}", e),
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
            &format!("human-{}", &body.identifier),
            "uhCAk-dev-mode-agent-key",
            &body.identifier,
            Some(dev_session_id),
            StatusCode::OK,
            None,
        );
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
                    error: format!("Database error: {}", e),
                    code: Some("DB_ERROR".into()),
                },
            )
        }
    };

    // Look up user by identifier
    let user = match collection
        .find_one(doc! { "identifier": &body.identifier, "is_active": true })
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
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
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: format!("Database error: {}", e),
                    code: Some("DB_ERROR".into()),
                },
            )
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

    info!("Login successful: {}", body.identifier);

    generate_auth_response(
        &jwt,
        &state,
        &user.human_id,
        &user.agent_pub_key,
        &user.identifier,
        Some(session_id),
        StatusCode::OK,
        None,
    )
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
    )
}

/// GET /auth/me
///
/// Get current user info from token.
async fn handle_me(
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
    json_response(
        StatusCode::OK,
        &MeResponse {
            human_id: claims.human_id,
            agent_pub_key: claims.agent_pub_key,
            identifier: claims.identifier,
            permission_level: claims.permission_level.to_string(),
            doorway_id: claims.doorway_id,
            doorway_url: claims.doorway_url,
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

    // Get bootstrap URL from config (optional)
    let bootstrap_url = state.args.bootstrap_url.clone();

    // TODO: Fetch display_name and profile_image_hash from user profile in MongoDB
    // For now, return None for these optional fields
    let display_name: Option<String> = None;
    let profile_image_hash: Option<String> = None;

    info!(
        "Native handoff: {} migrating to native session",
        claims.identifier
    );

    json_response(
        StatusCode::OK,
        &NativeHandoffResponse {
            human_id: claims.human_id,
            identifier: claims.identifier,
            doorway_id,
            doorway_url,
            display_name,
            profile_image_hash,
            bootstrap_url,
        },
    )
}

// =============================================================================
// Sovereignty Migration Handlers
// =============================================================================

/// GET /auth/export-key
///
/// Export the user's encrypted key bundle for migration to sovereignty (Tauri).
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
                    error: format!("Database error: {}", e),
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
                    error: format!("Database error: {}", e),
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
                    error: format!("Cannot export key: {}", e),
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
        "Exported custodial key for {} (preparing for sovereignty)",
        claims.identifier
    );

    json_response(
        StatusCode::OK,
        &KeyExportResponse {
            key_bundle: export,
            instructions: "Import this key bundle into your Elohim Tauri app. \
                You will need to enter your password to decrypt the key. \
                Once imported, call /auth/confirm-sovereignty to complete migration."
                .to_string(),
        },
    )
}

/// POST /auth/confirm-sovereignty
///
/// Confirm that the user has successfully migrated their key to Tauri.
/// Called by the Tauri app after successful key import and decryption.
///
/// The request must include a signature of the human_id, proving that the
/// user actually has access to the private key.
///
/// This endpoint:
/// 1. Validates the JWT token
/// 2. Verifies the signature proves key possession
/// 3. Marks the user as sovereign in MongoDB
/// 4. Clears their cached signing key
async fn handle_confirm_sovereignty(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    // Get auth header before consuming request
    let auth_header = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Parse request body
    let body: ConfirmSovereigntyRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid JSON body: {}", e),
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

    // TODO: Verify signature proves key possession
    // For now, we trust that if they have a valid token and the exported key,
    // they're the rightful owner. In production, we should verify the signature.
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

    // Update user to mark as sovereign
    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: format!("Database error: {}", e),
                    code: Some("DB_ERROR".into()),
                },
            )
        }
    };

    let sovereignty_time = bson::DateTime::now();
    if let Err(e) = collection
        .update_one(
            doc! { "identifier": &claims.identifier },
            doc! {
                "$set": {
                    "is_sovereign": true,
                    "sovereignty_at": sovereignty_time,
                }
            },
        )
        .await
    {
        return json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &ErrorResponse {
                error: format!("Failed to update user: {}", e),
                code: Some("DB_ERROR".into()),
            },
        );
    }

    // Clear cached signing key if session_id exists
    if let Some(session_id) = &claims.session_id {
        let key_service = CustodialKeyService::new();
        key_service.deactivate_key(session_id);
    }

    info!(
        "User {} has migrated to sovereignty! Custodial keys no longer needed.",
        claims.identifier
    );

    json_response(
        StatusCode::OK,
        &SovereigntyConfirmedResponse {
            success: true,
            message: "Welcome to sovereignty! You now have full control of your identity.".into(),
            sovereignty_at: chrono::Utc::now().to_rfc3339(),
        },
    )
}

// =============================================================================
// Disaster Recovery Handlers
// =============================================================================

/// POST /auth/recover-custody
///
/// Initiate disaster recovery for a sovereign user who has lost device access.
/// This creates a RecoveryRequest in the DHT and notifies emergency contacts.
///
/// Flow:
/// 1. Validate user exists and is_sovereign == true
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
                    error: format!("Invalid JSON body: {}", e),
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

    // Get doorway ID for the recovery request
    let doorway_id = match &state.args.doorway_id {
        Some(id) => id.clone(),
        None => "unknown-doorway".to_string(),
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
                    error: format!("Database error: {}", e),
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
                    error: format!("Database error: {}", e),
                    code: Some("DB_ERROR".into()),
                },
            )
        }
    };

    // Verify user is sovereign (recovery only applies to sovereign users)
    if !user.is_sovereign {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "Recovery is only available for sovereign users. Use regular login.".into(),
                code: Some("NOT_SOVEREIGN".into()),
            },
        );
    }

    // TODO: Call imagodei zome to create RecoveryRequest in DHT
    // For now, create a mock response until zome integration is complete
    let request_id = format!("recovery-{}-{}", user.human_id, chrono::Utc::now().timestamp());
    let expires_at = (chrono::Utc::now() + chrono::Duration::hours(48)).to_rfc3339();
    let required_approvals = 2u32; // TODO: Calculate from relationships

    info!(
        "Recovery request created for {} (request_id: {})",
        body.identifier, request_id
    );

    // TODO: Emit signal to notify emergency contacts

    json_response(
        StatusCode::OK,
        &RecoverCustodyResponse {
            request_id,
            required_approvals,
            expires_at,
            status: "pending".to_string(),
            instructions: format!(
                "Your recovery request has been submitted to doorway '{}'. \
                 Contact your emergency contacts to approve your recovery. \
                 You need {} approvals to regain access.",
                doorway_id, required_approvals
            ),
        },
    )
}

/// POST /auth/check-recovery-status
///
/// Poll for recovery request approval status.
/// Returns current vote count and status. If approved, includes recovery_token.
async fn handle_check_recovery_status(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    let body: CheckRecoveryStatusRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid JSON body: {}", e),
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

    // TODO: Fetch RecoveryRequest from DHT via imagodei zome
    // For now, return mock status

    // Mock: Check if request_id looks valid
    if !body.request_id.starts_with("recovery-") {
        return json_response(
            StatusCode::NOT_FOUND,
            &ErrorResponse {
                error: "Recovery request not found".into(),
                code: Some("REQUEST_NOT_FOUND".into()),
            },
        );
    }

    // TODO: Get actual status from DHT
    let status = "pending"; // Could be: pending, approved, rejected, expired, completed
    let current_approvals = 0u32;
    let required_approvals = 2u32;
    let confidence_score = 0.0f64;

    // Generate recovery_token if approved
    let recovery_token = if status == "approved" {
        // Generate a short-lived token for recovery activation
        Some(format!("recovery-token-{}", uuid::Uuid::new_v4()))
    } else {
        None
    };

    json_response(
        StatusCode::OK,
        &CheckRecoveryStatusResponse {
            status: status.to_string(),
            current_approvals,
            required_approvals,
            confidence_score,
            recovery_token,
            expires_at: (chrono::Utc::now() + chrono::Duration::hours(48)).to_rfc3339(),
            votes: vec![], // TODO: Fetch from DHT
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
/// 3. Update user: custodial_key = new, is_sovereign = false
/// 4. Activate key, generate JWT with recovery_mode flag
async fn handle_activate_recovery(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    let body: ActivateRecoveryRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid JSON body: {}", e),
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

    // Validate password strength
    if body.new_password.len() < 8 {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "Password must be at least 8 characters".into(),
                code: Some("WEAK_PASSWORD".into()),
            },
        );
    }

    // TODO: Validate recovery request is approved in DHT
    // For now, extract human_id from request_id format: "recovery-{human_id}-{timestamp}"
    let parts: Vec<&str> = body.request_id.split('-').collect();
    if parts.len() < 2 || parts[0] != "recovery" {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "Invalid recovery request ID".into(),
                code: Some("INVALID_REQUEST_ID".into()),
            },
        );
    }

    // TODO: Get actual human_id from DHT recovery request
    // For now, assume the second part is human_id (this is a placeholder)

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

    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ErrorResponse {
                    error: format!("Database error: {}", e),
                    code: Some("DB_ERROR".into()),
                },
            )
        }
    };

    // TODO: Look up user by human_id from recovery request
    // For now, this is a placeholder - in production, we'd validate against DHT
    warn!("Recovery activation placeholder - would validate request {} in DHT", body.request_id);

    // In production:
    // 1. Fetch RecoveryRequest from DHT
    // 2. Verify status == "approved"
    // 3. Verify not expired
    // 4. Get human_id and identifier from request
    // 5. Generate new custodial key
    // 6. Update MongoDB user
    // 7. Mark recovery as completed in DHT

    // For now, return error indicating feature is in development
    json_response(
        StatusCode::NOT_IMPLEMENTED,
        &ErrorResponse {
            error: "Recovery activation requires DHT integration. Coming soon.".into(),
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
    use crate::services::{ElohimVerifier, UserProfileData, PathCompletion, QuizScore};

    let body: ElohimVerifyStartRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid JSON body: {}", e),
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
        completed_paths: vec![
            PathCompletion {
                path_id: "elohim-protocol".to_string(),
                path_title: "Elohim Protocol Foundations".to_string(),
                completed_at: "2024-12-01".to_string(),
            },
        ],
        quiz_scores: vec![
            QuizScore {
                quiz_id: "quiz-manifesto".to_string(),
                quiz_title: "Manifesto Foundations".to_string(),
                score: 8.0,
                max_score: 10.0,
                completed_at: "2024-12-05".to_string(),
            },
        ],
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
                know the answers. You have 5 minutes to complete.".to_string(),
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
    use crate::services::{ElohimVerifier, UserProfileData, PathCompletion, QuizScore};

    let body: ElohimVerifyAnswerRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Invalid JSON body: {}", e),
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
        completed_paths: vec![
            PathCompletion {
                path_id: "elohim-protocol".to_string(),
                path_title: "Elohim Protocol Foundations".to_string(),
                completed_at: "2024-12-01".to_string(),
            },
        ],
        quiz_scores: vec![
            QuizScore {
                quiz_id: "quiz-manifesto".to_string(),
                quiz_title: "Manifesto Foundations".to_string(),
                score: 8.0,
                max_score: 10.0,
                completed_at: "2024-12-05".to_string(),
            },
        ],
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
                    error_description: Some(format!("Invalid query parameters: {}", e)),
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
                                error_description: Some("Failed to create authorization".to_string()),
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
                if params.redirect_uri.contains('?') { "&" } else { "?" },
                urlencoding::encode(&code),
                urlencoding::encode(&params.state)
            );

            info!(
                "OAuth authorize: redirecting {} to client with code",
                claims.identifier
            );

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
    let login_url = format!(
        "/threshold/login?{}",
        serde_urlencoded::to_string(&[
            ("client_id", params.client_id.as_str()),
            ("redirect_uri", params.redirect_uri.as_str()),
            ("response_type", params.response_type.as_str()),
            ("state", params.state.as_str()),
            ("scope", params.scope.as_deref().unwrap_or("")),
        ]).unwrap_or_default()
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
                    error_description: Some(format!("Failed to read body: {}", e)),
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
                        error_description: Some(format!("Invalid JSON: {}", e)),
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
                        error_description: Some(format!("Invalid form data: {}", e)),
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
                error_description: Some("Only 'authorization_code' grant type is supported".to_string()),
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

        return generate_oauth_token_response(
            &jwt,
            &state,
            "dev-human-id",
            "uhCAk-dev-mode-agent-key",
            "dev@example.com",
        );
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
                    error_description: Some(format!("Database error: {}", e)),
                    state: None,
                },
            )
        }
    };

    // Find the session by code
    let session = match collection
        .find_one(doc! { "code": &token_req.code })
        .await
    {
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
                    error_description: Some(format!("Database error: {}", e)),
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

    generate_oauth_token_response(
        &jwt,
        &state,
        &session.human_id,
        &session.agent_pub_key,
        &session.identifier,
    )
}

/// Generate a random authorization code.
fn generate_auth_code() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}

/// Generate OAuth token response with JWT.
fn generate_oauth_token_response(
    jwt: &JwtValidator,
    state: &AppState,
    human_id: &str,
    agent_pub_key: &str,
    identifier: &str,
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
                },
            )
        }
        Err(e) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &OAuthErrorResponse {
                error: "server_error".to_string(),
                error_description: Some(format!("Failed to generate token: {}", e)),
                state: None,
            },
        ),
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn get_jwt_validator(state: &AppState) -> Result<JwtValidator, Response<BoxBody>> {
    if state.args.dev_mode {
        Ok(JwtValidator::new_dev())
    } else {
        match &state.args.jwt_secret {
            Some(secret) => {
                JwtValidator::new(secret.clone(), state.args.jwt_expiry_seconds).map_err(|e| {
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &ErrorResponse {
                            error: format!("JWT configuration error: {}", e),
                            code: Some("CONFIG_ERROR".into()),
                        },
                    )
                })
            }
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

/// Generate a successful auth response with JWT token
fn generate_auth_response(
    jwt: &JwtValidator,
    state: &AppState,
    human_id: &str,
    agent_pub_key: &str,
    identifier: &str,
    session_id: Option<String>,
    status: StatusCode,
    profile: Option<HumanProfileResponse>,
) -> Response<BoxBody> {
    // Get doorway identity from config
    let doorway_id = state.args.doorway_id.clone();
    let doorway_url = state.args.doorway_url.clone();

    let input = TokenInput {
        human_id: human_id.to_string(),
        agent_pub_key: agent_pub_key.to_string(),
        identifier: identifier.to_string(),
        permission_level: PermissionLevel::Authenticated,
        session_id,
        doorway_id: doorway_id.clone(),
        doorway_url: doorway_url.clone(),
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
                    profile,
                },
            )
        }
        Err(e) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &ErrorResponse {
                error: format!("Failed to generate token: {}", e),
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

    // Handle CORS preflight
    if method == Method::OPTIONS {
        return Some(cors_preflight());
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

        // OAuth 2.0 endpoints
        (&Method::GET, "/auth/authorize") => handle_authorize(req, state).await,
        (&Method::POST, "/auth/token") => handle_token(req, state).await,

        // Native handoff (Tauri session migration)
        (&Method::GET, "/auth/native-handoff") => handle_native_handoff(req, state).await,

        // Sovereignty migration endpoints
        (&Method::GET, "/auth/export-key") => handle_export_key(req, state).await,
        (&Method::POST, "/auth/confirm-sovereignty") => handle_confirm_sovereignty(req, state).await,

        // Disaster recovery endpoints
        (&Method::POST, "/auth/recover-custody") => handle_recover_custody(req, state).await,
        (&Method::POST, "/auth/check-recovery-status") => handle_check_recovery_status(req, state).await,
        (&Method::POST, "/auth/activate-recovery") => handle_activate_recovery(req, state).await,

        // Elohim verification endpoints
        (&Method::POST, "/auth/elohim-verify/start") => handle_elohim_verify_start(req, state).await,
        (&Method::POST, "/auth/elohim-verify/answer") => handle_elohim_verify_answer(req, state).await,

        // Method not allowed
        (_, "/auth/register")
        | (_, "/auth/login")
        | (_, "/auth/logout")
        | (_, "/auth/refresh")
        | (_, "/auth/me")
        | (_, "/auth/authorize")
        | (_, "/auth/token")
        | (_, "/auth/native-handoff")
        | (_, "/auth/export-key")
        | (_, "/auth/confirm-sovereignty")
        | (_, "/auth/recover-custody")
        | (_, "/auth/check-recovery-status")
        | (_, "/auth/activate-recovery")
        | (_, "/auth/elohim-verify/start")
        | (_, "/auth/elohim-verify/answer") => json_response(
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
