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
use crate::db::schemas::{
    get_registered_clients, validate_redirect_uri, OAuthSessionDoc, UserDoc,
    OAUTH_SESSION_COLLECTION, USER_COLLECTION,
};
use crate::server::AppState;
use crate::types::DoorwayError;
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
    /// Display name for doorway-hosted registration (used to create identity)
    #[serde(default)]
    pub display_name: String,
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

    // For doorway-hosted registration, generate identity if not provided
    // TODO: In production, this should call the imagodei zome to create a proper
    // Holochain identity. For now, we generate placeholder values for dev/testing.
    let (human_id, agent_pub_key) = if body.human_id.is_empty() || body.agent_pub_key.is_empty() {
        // Generate placeholder identity for doorway-hosted registration
        let display_name = if body.display_name.is_empty() {
            body.identifier.split('@').next().unwrap_or("User").to_string()
        } else {
            body.display_name.clone()
        };

        // Generate deterministic IDs based on identifier (for consistency)
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

        info!(
            "Doorway-hosted registration: generated identity for {} (display_name={})",
            body.identifier, display_name
        );

        (human_id, agent_pub_key)
    } else {
        (body.human_id.clone(), body.agent_pub_key.clone())
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
            StatusCode::CREATED,
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

    // Create user document
    let user = UserDoc::new(
        body.identifier.clone(),
        body.identifier_type.clone(),
        password_hash,
        human_id.clone(),
        agent_pub_key.clone(),
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

    info!("Registered new user: {}", body.identifier);

    generate_auth_response(
        &jwt,
        &state,
        &human_id,
        &agent_pub_key,
        &body.identifier,
        StatusCode::CREATED,
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
        return generate_auth_response(
            &jwt,
            &state,
            &format!("human-{}", &body.identifier),
            "uhCAk-dev-mode-agent-key",
            &body.identifier,
            StatusCode::OK,
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

    info!("Login successful: {}", body.identifier);

    generate_auth_response(
        &jwt,
        &state,
        &user.human_id,
        &user.agent_pub_key,
        &user.identifier,
        StatusCode::OK,
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
        StatusCode::OK,
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

    let input = TokenInput {
        human_id: human_id.to_string(),
        agent_pub_key: agent_pub_key.to_string(),
        identifier: identifier.to_string(),
        permission_level: PermissionLevel::Authenticated,
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
    status: StatusCode,
) -> Response<BoxBody> {
    // Get doorway identity from config
    let doorway_id = state.args.doorway_id.clone();
    let doorway_url = state.args.doorway_url.clone();

    let input = TokenInput {
        human_id: human_id.to_string(),
        agent_pub_key: agent_pub_key.to_string(),
        identifier: identifier.to_string(),
        permission_level: PermissionLevel::Authenticated,
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

        // Method not allowed
        (_, "/auth/register")
        | (_, "/auth/login")
        | (_, "/auth/logout")
        | (_, "/auth/refresh")
        | (_, "/auth/me")
        | (_, "/auth/authorize")
        | (_, "/auth/token")
        | (_, "/auth/native-handoff") => json_response(
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
