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
use crate::db::schemas::{UserDoc, USER_COLLECTION};
use crate::server::AppState;
use crate::types::DoorwayError;

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

// =============================================================================
// Request/Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub human_id: String,
    pub agent_pub_key: String,
    pub identifier: String,
    pub password: String,
    #[serde(default = "default_identifier_type")]
    pub identifier_type: String,
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    pub human_id: String,
    pub agent_pub_key: String,
    pub identifier: String,
    pub permission_level: String,
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

    // Validate required fields
    if body.human_id.is_empty()
        || body.agent_pub_key.is_empty()
        || body.identifier.is_empty()
        || body.password.is_empty()
    {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "Missing required fields: humanId, agentPubKey, identifier, password".into(),
                code: None,
            },
        );
    }

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
            &body.human_id,
            &body.agent_pub_key,
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
        body.human_id.clone(),
        body.agent_pub_key.clone(),
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
        &body.human_id,
        &body.agent_pub_key,
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
        },
    )
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
    human_id: &str,
    agent_pub_key: &str,
    identifier: &str,
    status: StatusCode,
) -> Response<BoxBody> {
    let input = TokenInput {
        human_id: human_id.to_string(),
        agent_pub_key: agent_pub_key.to_string(),
        identifier: identifier.to_string(),
        permission_level: PermissionLevel::Authenticated,
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
        (&Method::POST, "/auth/register") => handle_register(req, state).await,
        (&Method::POST, "/auth/login") => handle_login(req, state).await,
        (&Method::POST, "/auth/logout") => handle_logout(req, state).await,
        (&Method::POST, "/auth/refresh") => handle_refresh(req, state).await,
        (&Method::GET, "/auth/me") => handle_me(req, state).await,

        // Method not allowed
        (_, "/auth/register")
        | (_, "/auth/login")
        | (_, "/auth/logout")
        | (_, "/auth/refresh")
        | (_, "/auth/me") => json_response(
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
