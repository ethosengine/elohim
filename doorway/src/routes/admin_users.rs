//! Admin API endpoints for user management
//!
//! ## Endpoints
//!
//! - `GET /admin/users` - List users with pagination, search, filter
//! - `GET /admin/users/{id}` - Get user details with usage stats
//! - `PUT /admin/users/{id}/status` - Activate/deactivate user
//! - `POST /admin/users/{id}/force-logout` - Invalidate all tokens
//! - `DELETE /admin/users/{id}` - Soft delete user
//! - `POST /admin/users/{id}/reset-password` - Admin password reset
//! - `PUT /admin/users/{id}/permission` - Change permission level
//! - `PUT /admin/users/{id}/quota` - Update quota limits
//! - `POST /admin/users/{id}/usage/reset` - Reset usage counters
//!
//! ## Authentication
//!
//! All endpoints require Admin permission level via JWT token.
//!
//! ## Extensibility
//!
//! Usage tracking uses the `UsageTracker` trait to allow future
//! billing service integration without modifying these endpoints.

use bson::{doc, oid::ObjectId, DateTime};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use mongodb::options::FindOptions;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

use crate::auth::{
    extract_token_from_header, hash_password, Claims, JwtValidator, PermissionLevel,
};
use crate::db::schemas::{UserDoc, UserQuota, UserUsage, USER_COLLECTION};
use crate::db::MongoClient;
use crate::server::AppState;

type FullBody = Full<Bytes>;

// =============================================================================
// Traits for Extensibility
// =============================================================================

/// Trait for usage tracking - allows swapping implementations
/// (e.g., in-memory for dev, MongoDB for prod, billing service for future)
#[async_trait::async_trait]
pub trait UsageTracker: Send + Sync {
    /// Increment projection query count
    async fn increment_queries(&self, user_id: &str, count: u64) -> Result<(), String>;
    /// Increment bandwidth usage in bytes
    async fn increment_bandwidth(&self, user_id: &str, bytes: u64) -> Result<(), String>;
    /// Update storage usage in bytes
    async fn update_storage(&self, user_id: &str, bytes: u64) -> Result<(), String>;
    /// Get current usage for a user
    async fn get_usage(&self, user_id: &str) -> Result<UserUsage, String>;
    /// Reset usage counters for a user
    async fn reset_usage(&self, user_id: &str) -> Result<(), String>;
}

/// Trait for quota enforcement - allows swapping implementations
#[async_trait::async_trait]
pub trait QuotaEnforcer: Send + Sync {
    /// Check if user is within quota limits
    async fn check_quota(&self, user_id: &str) -> Result<QuotaStatus, String>;
    /// Update quota limits for a user
    async fn update_quota(&self, user_id: &str, quota: UserQuota) -> Result<(), String>;
}

/// Result of quota check
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaStatus {
    /// Whether operations are allowed
    pub allowed: bool,
    /// Whether storage limit is exceeded
    pub storage_exceeded: bool,
    /// Whether daily query limit is exceeded
    pub queries_exceeded: bool,
    /// Whether daily bandwidth limit is exceeded
    pub bandwidth_exceeded: bool,
    /// Human-readable message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// =============================================================================
// MongoDB Implementation of UsageTracker
// =============================================================================

/// MongoDB-backed usage tracker
pub struct MongoUsageTracker {
    mongo: MongoClient,
}

impl MongoUsageTracker {
    pub fn new(mongo: MongoClient) -> Self {
        Self { mongo }
    }
}

#[async_trait::async_trait]
impl UsageTracker for MongoUsageTracker {
    async fn increment_queries(&self, user_id: &str, count: u64) -> Result<(), String> {
        let collection = self
            .mongo
            .collection::<UserDoc>(USER_COLLECTION)
            .await
            .map_err(|e| e.to_string())?;
        let filter = user_filter(user_id);

        collection
            .update_one(
                filter,
                doc! {
                    "$inc": { "usage.projection_queries": count as i64 },
                    "$set": { "usage.last_updated": DateTime::now() }
                },
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn increment_bandwidth(&self, user_id: &str, bytes: u64) -> Result<(), String> {
        let collection = self
            .mongo
            .collection::<UserDoc>(USER_COLLECTION)
            .await
            .map_err(|e| e.to_string())?;
        let filter = user_filter(user_id);

        collection
            .update_one(
                filter,
                doc! {
                    "$inc": { "usage.bandwidth_bytes": bytes as i64 },
                    "$set": { "usage.last_updated": DateTime::now() }
                },
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn update_storage(&self, user_id: &str, bytes: u64) -> Result<(), String> {
        let collection = self
            .mongo
            .collection::<UserDoc>(USER_COLLECTION)
            .await
            .map_err(|e| e.to_string())?;
        let filter = user_filter(user_id);

        collection
            .update_one(
                filter,
                doc! {
                    "$set": {
                        "usage.storage_bytes": bytes as i64,
                        "usage.last_updated": DateTime::now()
                    }
                },
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn get_usage(&self, user_id: &str) -> Result<UserUsage, String> {
        let collection = self
            .mongo
            .collection::<UserDoc>(USER_COLLECTION)
            .await
            .map_err(|e| e.to_string())?;
        let filter = user_filter(user_id);

        let user = collection
            .find_one(filter)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "User not found".to_string())?;

        Ok(user.usage)
    }

    async fn reset_usage(&self, user_id: &str) -> Result<(), String> {
        let collection = self
            .mongo
            .collection::<UserDoc>(USER_COLLECTION)
            .await
            .map_err(|e| e.to_string())?;
        let filter = user_filter(user_id);

        collection
            .update_one(
                filter,
                doc! {
                    "$set": {
                        "usage.storage_bytes": 0_i64,
                        "usage.projection_queries": 0_i64,
                        "usage.bandwidth_bytes": 0_i64,
                        "usage.period_start": DateTime::now(),
                        "usage.last_updated": DateTime::now()
                    }
                },
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

/// Create a filter that works with either MongoDB ObjectId or human_id.
/// This allows the usage tracker to be called with either identifier.
fn user_filter(user_id: &str) -> bson::Document {
    // Try to parse as ObjectId first, otherwise use as human_id
    if let Ok(oid) = ObjectId::parse_str(user_id) {
        doc! { "_id": oid }
    } else {
        // Assume it's a human_id (from JWT)
        doc! { "human_id": user_id }
    }
}

// =============================================================================
// MongoDB Implementation of QuotaEnforcer
// =============================================================================

/// MongoDB-backed quota enforcer
pub struct MongoQuotaEnforcer {
    mongo: MongoClient,
}

impl MongoQuotaEnforcer {
    pub fn new(mongo: MongoClient) -> Self {
        Self { mongo }
    }
}

#[async_trait::async_trait]
impl QuotaEnforcer for MongoQuotaEnforcer {
    async fn check_quota(&self, user_id: &str) -> Result<QuotaStatus, String> {
        let collection = self
            .mongo
            .collection::<UserDoc>(USER_COLLECTION)
            .await
            .map_err(|e| e.to_string())?;
        let filter = user_filter(user_id);

        let user = collection
            .find_one(filter)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "User not found".to_string())?;

        let q = &user.quota;
        let u = &user.usage;

        let storage_exceeded = q.storage_limit > 0 && u.storage_bytes > q.storage_limit;
        let queries_exceeded =
            q.daily_query_limit > 0 && u.projection_queries > q.daily_query_limit;
        let bandwidth_exceeded =
            q.daily_bandwidth_limit > 0 && u.bandwidth_bytes > q.daily_bandwidth_limit;

        let any_exceeded = storage_exceeded || queries_exceeded || bandwidth_exceeded;
        let allowed = !any_exceeded || !q.enforce_hard_limit;

        let message = if any_exceeded {
            let mut parts = Vec::new();
            if storage_exceeded {
                parts.push("storage");
            }
            if queries_exceeded {
                parts.push("queries");
            }
            if bandwidth_exceeded {
                parts.push("bandwidth");
            }
            Some(format!("Quota exceeded: {}", parts.join(", ")))
        } else {
            None
        };

        Ok(QuotaStatus {
            allowed,
            storage_exceeded,
            queries_exceeded,
            bandwidth_exceeded,
            message,
        })
    }

    async fn update_quota(&self, user_id: &str, quota: UserQuota) -> Result<(), String> {
        let collection = self
            .mongo
            .collection::<UserDoc>(USER_COLLECTION)
            .await
            .map_err(|e| e.to_string())?;
        let filter = user_filter(user_id);

        collection
            .update_one(
                filter,
                doc! {
                    "$set": {
                        "quota.storage_limit": quota.storage_limit as i64,
                        "quota.daily_query_limit": quota.daily_query_limit as i64,
                        "quota.daily_bandwidth_limit": quota.daily_bandwidth_limit as i64,
                        "quota.enforce_hard_limit": quota.enforce_hard_limit,
                        "metadata.updated_at": DateTime::now()
                    }
                },
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

// =============================================================================
// Request/Response Types
// =============================================================================

/// Query parameters for listing users
#[derive(Debug, Default)]
pub struct ListUsersQuery {
    pub page: u32,
    pub limit: u32,
    pub search: Option<String>,
    pub permission_level: Option<String>,
    pub is_active: Option<bool>,
    pub over_quota: Option<bool>,
    pub sort_by: String,
    pub sort_dir: String,
}

impl ListUsersQuery {
    fn from_query_string(query: Option<&str>) -> Self {
        let mut params = Self {
            page: 1,
            limit: 20,
            search: None,
            permission_level: None,
            is_active: None,
            over_quota: None,
            sort_by: "metadata.created_at".to_string(),
            sort_dir: "desc".to_string(),
        };

        if let Some(q) = query {
            for pair in q.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    let value = urlencoding::decode(value).unwrap_or_default();
                    match key {
                        "page" => params.page = value.parse().unwrap_or(1),
                        "limit" => params.limit = value.parse().unwrap_or(20),
                        "search" => params.search = Some(value.to_string()),
                        "permissionLevel" | "permission_level" => {
                            params.permission_level = Some(value.to_string())
                        }
                        "isActive" | "is_active" => params.is_active = value.parse().ok(),
                        "overQuota" | "over_quota" => params.over_quota = value.parse().ok(),
                        "sortBy" | "sort_by" => params.sort_by = value.to_string(),
                        "sortDir" | "sort_dir" => params.sort_dir = value.to_string(),
                        _ => {}
                    }
                }
            }
        }

        params
    }
}

/// User summary for list view
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSummary {
    pub id: String,
    pub identifier: String,
    pub identifier_type: String,
    pub permission_level: String,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login_at: Option<String>,
    pub storage_used_mb: f64,
    pub storage_limit_mb: f64,
    pub storage_percent: f64,
    pub is_over_quota: bool,
}

/// Paginated users response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsersResponse {
    pub users: Vec<UserSummary>,
    pub total: u64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

/// Full user details response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDetailsResponse {
    pub id: String,
    pub identifier: String,
    pub identifier_type: String,
    pub human_id: String,
    pub agent_pub_key: String,
    pub permission_level: String,
    pub is_active: bool,
    pub token_version: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login_at: Option<String>,
    pub usage: UsageResponse,
    pub quota: QuotaResponse,
}

/// Usage stats response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageResponse {
    pub storage_bytes: u64,
    pub storage_mb: f64,
    pub projection_queries: u64,
    pub bandwidth_bytes: u64,
    pub bandwidth_mb: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
}

/// Quota stats response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaResponse {
    pub storage_limit_bytes: u64,
    pub storage_limit_mb: f64,
    pub storage_percent_used: f64,
    pub daily_query_limit: u64,
    pub queries_percent_used: f64,
    pub daily_bandwidth_limit_bytes: u64,
    pub daily_bandwidth_limit_mb: f64,
    pub bandwidth_percent_used: f64,
    pub enforce_hard_limit: bool,
    pub is_over_quota: bool,
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Success response
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

// =============================================================================
// Mutation Request Types
// =============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatusRequest {
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePermissionRequest {
    pub permission_level: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordRequest {
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateQuotaRequest {
    pub storage_limit_mb: Option<f64>,
    pub daily_query_limit: Option<u64>,
    pub daily_bandwidth_limit_mb: Option<f64>,
    pub enforce_hard_limit: Option<bool>,
}

// =============================================================================
// Response Helpers
// =============================================================================

fn json_response<T: Serialize>(status: StatusCode, body: &T) -> Response<FullBody> {
    let json = serde_json::to_string(body).unwrap_or_else(|_| "{}".to_string());
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap()
}

fn error_response(status: StatusCode, error: &str, code: Option<&str>) -> Response<FullBody> {
    json_response(
        status,
        &ErrorResponse {
            error: error.to_string(),
            code: code.map(|c| c.to_string()),
        },
    )
}

// =============================================================================
// Auth Helpers
// =============================================================================

fn get_auth_header(req: &Request<Incoming>) -> Option<&str> {
    req.headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
}

#[allow(clippy::result_large_err)]
fn get_jwt_validator(state: &AppState) -> Result<JwtValidator, Response<FullBody>> {
    if state.args.dev_mode {
        Ok(JwtValidator::new_dev())
    } else {
        match &state.args.jwt_secret {
            Some(secret) => JwtValidator::new(secret.clone(), state.args.jwt_expiry_seconds)
                .map_err(|e| {
                    error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &format!("JWT config error: {e}"),
                        Some("JWT_CONFIG_ERROR"),
                    )
                }),
            None => Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "JWT secret not configured",
                Some("JWT_CONFIG_ERROR"),
            )),
        }
    }
}

/// Validate admin access from request
async fn require_admin(
    req: &Request<Incoming>,
    state: &AppState,
) -> Result<Claims, Response<FullBody>> {
    let auth_header = get_auth_header(req);
    let token = match extract_token_from_header(auth_header) {
        Some(t) => t,
        None => {
            return Err(error_response(
                StatusCode::UNAUTHORIZED,
                "No token provided",
                Some("NO_TOKEN"),
            ))
        }
    };

    let jwt = get_jwt_validator(state)?;
    let result = jwt.verify_token(token);

    if !result.valid {
        return Err(error_response(
            StatusCode::UNAUTHORIZED,
            result.error.as_deref().unwrap_or("Invalid token"),
            Some("INVALID_TOKEN"),
        ));
    }

    let claims = result.claims.unwrap();

    if claims.permission_level < PermissionLevel::Admin {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "Admin permission required",
            Some("FORBIDDEN"),
        ));
    }

    Ok(claims)
}

// =============================================================================
// Route Handler
// =============================================================================

/// Main handler for /admin/users/* routes
pub async fn handle_admin_users_request(
    req: Request<Incoming>,
    state: Arc<AppState>,
    path: &str,
) -> Response<FullBody> {
    let method = req.method().clone();

    // Extract subpath after /admin/users
    let subpath = path.strip_prefix("/admin/users").unwrap_or("");

    match (method, subpath) {
        // GET /admin/users - List users
        (Method::GET, "") | (Method::GET, "/") => handle_list_users(req, state).await,

        // GET /admin/users/{id} - Get user details
        (Method::GET, p) if !p.contains('/') || p.matches('/').count() == 1 => {
            let id = p.trim_start_matches('/');
            if id.is_empty() {
                handle_list_users(req, state).await
            } else {
                handle_get_user(req, state, id).await
            }
        }

        // PUT /admin/users/{id}/status - Update status
        (Method::PUT, p) if p.ends_with("/status") => {
            let id = p
                .strip_prefix('/')
                .and_then(|s| s.strip_suffix("/status"))
                .unwrap_or("");
            handle_update_status(req, state, id).await
        }

        // POST /admin/users/{id}/force-logout - Force logout
        (Method::POST, p) if p.ends_with("/force-logout") => {
            let id = p
                .strip_prefix('/')
                .and_then(|s| s.strip_suffix("/force-logout"))
                .unwrap_or("");
            handle_force_logout(req, state, id).await
        }

        // DELETE /admin/users/{id} - Delete user
        (Method::DELETE, p) if !p.contains('/') || p.matches('/').count() == 1 => {
            let id = p.trim_start_matches('/');
            handle_delete_user(req, state, id).await
        }

        // POST /admin/users/{id}/reset-password - Reset password
        (Method::POST, p) if p.ends_with("/reset-password") => {
            let id = p
                .strip_prefix('/')
                .and_then(|s| s.strip_suffix("/reset-password"))
                .unwrap_or("");
            handle_reset_password(req, state, id).await
        }

        // PUT /admin/users/{id}/permission - Update permission
        (Method::PUT, p) if p.ends_with("/permission") => {
            let id = p
                .strip_prefix('/')
                .and_then(|s| s.strip_suffix("/permission"))
                .unwrap_or("");
            handle_update_permission(req, state, id).await
        }

        // PUT /admin/users/{id}/quota - Update quota
        (Method::PUT, p) if p.ends_with("/quota") => {
            let id = p
                .strip_prefix('/')
                .and_then(|s| s.strip_suffix("/quota"))
                .unwrap_or("");
            handle_update_quota(req, state, id).await
        }

        // POST /admin/users/{id}/usage/reset - Reset usage
        (Method::POST, p) if p.ends_with("/usage/reset") => {
            let id = p
                .strip_prefix('/')
                .and_then(|s| s.strip_suffix("/usage/reset"))
                .unwrap_or("");
            handle_reset_usage(req, state, id).await
        }

        _ => error_response(StatusCode::NOT_FOUND, "Not found", None),
    }
}

// =============================================================================
// Endpoint Handlers
// =============================================================================

/// GET /admin/users - List users with pagination
async fn handle_list_users(req: Request<Incoming>, state: Arc<AppState>) -> Response<FullBody> {
    // Verify admin access
    if let Err(resp) = require_admin(&req, &state).await {
        return resp;
    }

    // Parse query params
    let params = ListUsersQuery::from_query_string(req.uri().query());

    // Get MongoDB
    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "Database not available",
                Some("DB_UNAVAILABLE"),
            )
        }
    };

    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            warn!("Error getting collection: {}", e);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            );
        }
    };

    // Build filter
    let mut filter = doc! { "metadata.is_deleted": { "$ne": true } };

    if let Some(ref search) = params.search {
        filter.insert(
            "identifier",
            doc! { "$regex": search.clone(), "$options": "i" },
        );
    }

    if let Some(ref perm) = params.permission_level {
        filter.insert("permission_level", perm.to_uppercase());
    }

    if let Some(is_active) = params.is_active {
        filter.insert("is_active", is_active);
    }

    // Count total (using inner() to access raw mongodb Collection)
    let total = match collection.inner().count_documents(filter.clone()).await {
        Ok(c) => c,
        Err(e) => {
            warn!("Error counting users: {}", e);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            );
        }
    };

    // Build sort
    let sort_dir = if params.sort_dir == "asc" { 1 } else { -1 };
    let sort = doc! { &params.sort_by: sort_dir };

    // Pagination
    let skip = ((params.page.max(1) - 1) * params.limit) as u64;
    let limit = params.limit.min(100) as i64;

    let options = FindOptions::builder()
        .sort(sort)
        .skip(skip)
        .limit(limit)
        .build();

    // Execute query (using inner() to access raw mongodb Collection for find with options)
    let mut cursor = match collection.inner().find(filter).with_options(options).await {
        Ok(c) => c,
        Err(e) => {
            warn!("Error finding users: {}", e);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            );
        }
    };

    // Collect results
    let mut users = Vec::new();
    use futures::stream::StreamExt;
    while let Some(result) = cursor.next().await {
        if let Ok(user) = result {
            // Apply over_quota filter in memory
            let is_over_quota = user.is_over_quota();
            if let Some(over_quota) = params.over_quota {
                if over_quota != is_over_quota {
                    continue;
                }
            }

            users.push(user_to_summary(&user));
        }
    }

    let total_pages = ((total as f64) / (params.limit as f64)).ceil() as u32;

    json_response(
        StatusCode::OK,
        &UsersResponse {
            users,
            total,
            page: params.page,
            limit: params.limit,
            total_pages,
        },
    )
}

/// GET /admin/users/{id} - Get user details
async fn handle_get_user(
    req: Request<Incoming>,
    state: Arc<AppState>,
    user_id: &str,
) -> Response<FullBody> {
    if let Err(resp) = require_admin(&req, &state).await {
        return resp;
    }

    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "Database not available",
                Some("DB_UNAVAILABLE"),
            )
        }
    };

    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            warn!("Error getting collection: {}", e);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            );
        }
    };

    let oid = match ObjectId::parse_str(user_id) {
        Ok(o) => o,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Invalid user ID",
                Some("INVALID_ID"),
            )
        }
    };

    let user = match collection.find_one(doc! { "_id": oid }).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return error_response(StatusCode::NOT_FOUND, "User not found", Some("NOT_FOUND"))
        }
        Err(e) => {
            warn!("Error finding user: {}", e);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            );
        }
    };

    json_response(StatusCode::OK, &user_to_details(&user))
}

/// PUT /admin/users/{id}/status - Update user status
async fn handle_update_status(
    req: Request<Incoming>,
    state: Arc<AppState>,
    user_id: &str,
) -> Response<FullBody> {
    let admin_claims = match require_admin(&req, &state).await {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let body_bytes = match req.into_body().collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid body", None),
    };

    let request: UpdateStatusRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid JSON", None),
    };

    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "Database not available",
                Some("DB_UNAVAILABLE"),
            )
        }
    };

    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            warn!("Error getting collection: {}", e);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            );
        }
    };

    let oid = match ObjectId::parse_str(user_id) {
        Ok(o) => o,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Invalid user ID",
                Some("INVALID_ID"),
            )
        }
    };

    let result = collection
        .update_one(
            doc! { "_id": oid },
            doc! {
                "$set": {
                    "is_active": request.is_active,
                    "metadata.updated_at": DateTime::now()
                }
            },
        )
        .await;

    match result {
        Ok(r) if r.modified_count > 0 => {
            let action = if request.is_active {
                "activated"
            } else {
                "deactivated"
            };
            info!(
                "User {} {} by admin {}",
                user_id, action, admin_claims.identifier
            );
            json_response(
                StatusCode::OK,
                &SuccessResponse {
                    success: true,
                    message: format!("User {action}"),
                },
            )
        }
        Ok(_) => error_response(StatusCode::NOT_FOUND, "User not found", Some("NOT_FOUND")),
        Err(e) => {
            warn!("Error updating user status: {}", e);
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            )
        }
    }
}

/// POST /admin/users/{id}/force-logout - Invalidate all tokens
async fn handle_force_logout(
    req: Request<Incoming>,
    state: Arc<AppState>,
    user_id: &str,
) -> Response<FullBody> {
    let admin_claims = match require_admin(&req, &state).await {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "Database not available",
                Some("DB_UNAVAILABLE"),
            )
        }
    };

    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            warn!("Error getting collection: {}", e);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            );
        }
    };

    let oid = match ObjectId::parse_str(user_id) {
        Ok(o) => o,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Invalid user ID",
                Some("INVALID_ID"),
            )
        }
    };

    let result = collection
        .update_one(
            doc! { "_id": oid },
            doc! {
                "$inc": { "token_version": 1 },
                "$set": { "metadata.updated_at": DateTime::now() }
            },
        )
        .await;

    match result {
        Ok(r) if r.modified_count > 0 => {
            info!(
                "Force logout for user {} by admin {}",
                user_id, admin_claims.identifier
            );
            json_response(
                StatusCode::OK,
                &SuccessResponse {
                    success: true,
                    message: "All sessions invalidated".to_string(),
                },
            )
        }
        Ok(_) => error_response(StatusCode::NOT_FOUND, "User not found", Some("NOT_FOUND")),
        Err(e) => {
            warn!("Error forcing logout: {}", e);
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            )
        }
    }
}

/// DELETE /admin/users/{id} - Soft delete user
async fn handle_delete_user(
    req: Request<Incoming>,
    state: Arc<AppState>,
    user_id: &str,
) -> Response<FullBody> {
    let admin_claims = match require_admin(&req, &state).await {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "Database not available",
                Some("DB_UNAVAILABLE"),
            )
        }
    };

    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            warn!("Error getting collection: {}", e);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            );
        }
    };

    let oid = match ObjectId::parse_str(user_id) {
        Ok(o) => o,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Invalid user ID",
                Some("INVALID_ID"),
            )
        }
    };

    let result = collection
        .update_one(
            doc! { "_id": oid },
            doc! {
                "$set": {
                    "metadata.is_deleted": true,
                    "metadata.updated_at": DateTime::now(),
                    "is_active": false
                }
            },
        )
        .await;

    match result {
        Ok(r) if r.modified_count > 0 => {
            info!(
                "User {} deleted by admin {}",
                user_id, admin_claims.identifier
            );
            json_response(
                StatusCode::OK,
                &SuccessResponse {
                    success: true,
                    message: "User deleted".to_string(),
                },
            )
        }
        Ok(_) => error_response(StatusCode::NOT_FOUND, "User not found", Some("NOT_FOUND")),
        Err(e) => {
            warn!("Error deleting user: {}", e);
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            )
        }
    }
}

/// POST /admin/users/{id}/reset-password - Admin password reset
async fn handle_reset_password(
    req: Request<Incoming>,
    state: Arc<AppState>,
    user_id: &str,
) -> Response<FullBody> {
    let admin_claims = match require_admin(&req, &state).await {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let body_bytes = match req.into_body().collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid body", None),
    };

    let request: ResetPasswordRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid JSON", None),
    };

    if request.new_password.len() < 8 {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Password must be at least 8 characters",
            Some("WEAK_PASSWORD"),
        );
    }

    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "Database not available",
                Some("DB_UNAVAILABLE"),
            )
        }
    };

    let password_hash = match hash_password(&request.new_password) {
        Ok(h) => h,
        Err(e) => {
            warn!("Error hashing password: {}", e);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Password hash error",
                None,
            );
        }
    };

    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            warn!("Error getting collection: {}", e);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            );
        }
    };

    let oid = match ObjectId::parse_str(user_id) {
        Ok(o) => o,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Invalid user ID",
                Some("INVALID_ID"),
            )
        }
    };

    let result = collection
        .update_one(
            doc! { "_id": oid },
            doc! {
                "$set": {
                    "password_hash": password_hash,
                    "metadata.updated_at": DateTime::now()
                },
                "$inc": { "token_version": 1 }
            },
        )
        .await;

    match result {
        Ok(r) if r.modified_count > 0 => {
            info!(
                "Password reset for user {} by admin {}",
                user_id, admin_claims.identifier
            );
            json_response(
                StatusCode::OK,
                &SuccessResponse {
                    success: true,
                    message: "Password reset successfully".to_string(),
                },
            )
        }
        Ok(_) => error_response(StatusCode::NOT_FOUND, "User not found", Some("NOT_FOUND")),
        Err(e) => {
            warn!("Error resetting password: {}", e);
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            )
        }
    }
}

/// PUT /admin/users/{id}/permission - Update permission level
async fn handle_update_permission(
    req: Request<Incoming>,
    state: Arc<AppState>,
    user_id: &str,
) -> Response<FullBody> {
    let admin_claims = match require_admin(&req, &state).await {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let body_bytes = match req.into_body().collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid body", None),
    };

    let request: UpdatePermissionRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid JSON", None),
    };

    let perm_level = match request.permission_level.to_uppercase().as_str() {
        "PUBLIC" => "PUBLIC",
        "AUTHENTICATED" => "AUTHENTICATED",
        "ADMIN" => "ADMIN",
        _ => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Invalid permission level. Must be PUBLIC, AUTHENTICATED, or ADMIN",
                Some("INVALID_PERMISSION"),
            )
        }
    };

    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "Database not available",
                Some("DB_UNAVAILABLE"),
            )
        }
    };

    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            warn!("Error getting collection: {}", e);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            );
        }
    };

    let oid = match ObjectId::parse_str(user_id) {
        Ok(o) => o,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Invalid user ID",
                Some("INVALID_ID"),
            )
        }
    };

    let result = collection
        .update_one(
            doc! { "_id": oid },
            doc! {
                "$set": {
                    "permission_level": perm_level,
                    "metadata.updated_at": DateTime::now()
                }
            },
        )
        .await;

    match result {
        Ok(r) if r.modified_count > 0 => {
            info!(
                "Permission for user {} changed to {} by admin {}",
                user_id, perm_level, admin_claims.identifier
            );
            json_response(
                StatusCode::OK,
                &SuccessResponse {
                    success: true,
                    message: format!("Permission level updated to {perm_level}"),
                },
            )
        }
        Ok(_) => error_response(StatusCode::NOT_FOUND, "User not found", Some("NOT_FOUND")),
        Err(e) => {
            warn!("Error updating permission: {}", e);
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            )
        }
    }
}

/// PUT /admin/users/{id}/quota - Update quota limits
async fn handle_update_quota(
    req: Request<Incoming>,
    state: Arc<AppState>,
    user_id: &str,
) -> Response<FullBody> {
    let admin_claims = match require_admin(&req, &state).await {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let body_bytes = match req.into_body().collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid body", None),
    };

    let request: UpdateQuotaRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid JSON", None),
    };

    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "Database not available",
                Some("DB_UNAVAILABLE"),
            )
        }
    };

    // Build update document
    let mut set_doc = doc! { "metadata.updated_at": DateTime::now() };

    if let Some(storage_mb) = request.storage_limit_mb {
        set_doc.insert("quota.storage_limit", (storage_mb * 1024.0 * 1024.0) as i64);
    }
    if let Some(query_limit) = request.daily_query_limit {
        set_doc.insert("quota.daily_query_limit", query_limit as i64);
    }
    if let Some(bandwidth_mb) = request.daily_bandwidth_limit_mb {
        set_doc.insert(
            "quota.daily_bandwidth_limit",
            (bandwidth_mb * 1024.0 * 1024.0) as i64,
        );
    }
    if let Some(enforce) = request.enforce_hard_limit {
        set_doc.insert("quota.enforce_hard_limit", enforce);
    }

    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            warn!("Error getting collection: {}", e);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            );
        }
    };

    let oid = match ObjectId::parse_str(user_id) {
        Ok(o) => o,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Invalid user ID",
                Some("INVALID_ID"),
            )
        }
    };

    let result = collection
        .update_one(doc! { "_id": oid }, doc! { "$set": set_doc })
        .await;

    match result {
        Ok(r) if r.modified_count > 0 => {
            info!(
                "Quota updated for user {} by admin {}",
                user_id, admin_claims.identifier
            );
            json_response(
                StatusCode::OK,
                &SuccessResponse {
                    success: true,
                    message: "Quota updated".to_string(),
                },
            )
        }
        Ok(_) => error_response(StatusCode::NOT_FOUND, "User not found", Some("NOT_FOUND")),
        Err(e) => {
            warn!("Error updating quota: {}", e);
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some("DB_ERROR"),
            )
        }
    }
}

/// POST /admin/users/{id}/usage/reset - Reset usage counters
async fn handle_reset_usage(
    req: Request<Incoming>,
    state: Arc<AppState>,
    user_id: &str,
) -> Response<FullBody> {
    let admin_claims = match require_admin(&req, &state).await {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "Database not available",
                Some("DB_UNAVAILABLE"),
            )
        }
    };

    let tracker = MongoUsageTracker::new(mongo.clone());

    match tracker.reset_usage(user_id).await {
        Ok(()) => {
            info!(
                "Usage reset for user {} by admin {}",
                user_id, admin_claims.identifier
            );
            json_response(
                StatusCode::OK,
                &SuccessResponse {
                    success: true,
                    message: "Usage counters reset".to_string(),
                },
            )
        }
        Err(e) => {
            warn!("Error resetting usage: {}", e);
            if e.contains("not found") {
                error_response(StatusCode::NOT_FOUND, "User not found", Some("NOT_FOUND"))
            } else {
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error",
                    Some("DB_ERROR"),
                )
            }
        }
    }
}

// =============================================================================
// Conversion Helpers
// =============================================================================

fn user_to_summary(user: &UserDoc) -> UserSummary {
    let storage_limit_mb = user.quota.storage_limit as f64 / (1024.0 * 1024.0);
    let storage_used_mb = user.usage.storage_bytes as f64 / (1024.0 * 1024.0);

    UserSummary {
        id: user._id.map(|o| o.to_hex()).unwrap_or_default(),
        identifier: user.identifier.clone(),
        identifier_type: user.identifier_type.clone(),
        permission_level: user.permission_level.to_string(),
        is_active: user.is_active,
        created_at: user.metadata.created_at.map(|d| d.to_string()),
        last_login_at: user.last_login_at.map(|d| d.to_string()),
        storage_used_mb,
        storage_limit_mb,
        storage_percent: user.storage_percent(),
        is_over_quota: user.is_over_quota(),
    }
}

fn user_to_details(user: &UserDoc) -> UserDetailsResponse {
    let storage_mb = user.usage.storage_bytes as f64 / (1024.0 * 1024.0);
    let bandwidth_mb = user.usage.bandwidth_bytes as f64 / (1024.0 * 1024.0);
    let storage_limit_mb = user.quota.storage_limit as f64 / (1024.0 * 1024.0);
    let bandwidth_limit_mb = user.quota.daily_bandwidth_limit as f64 / (1024.0 * 1024.0);

    UserDetailsResponse {
        id: user._id.map(|o| o.to_hex()).unwrap_or_default(),
        identifier: user.identifier.clone(),
        identifier_type: user.identifier_type.clone(),
        human_id: user.human_id.clone(),
        agent_pub_key: user.agent_pub_key.clone(),
        permission_level: user.permission_level.to_string(),
        is_active: user.is_active,
        token_version: user.token_version,
        created_at: user.metadata.created_at.map(|d| d.to_string()),
        updated_at: user.metadata.updated_at.map(|d| d.to_string()),
        last_login_at: user.last_login_at.map(|d| d.to_string()),
        usage: UsageResponse {
            storage_bytes: user.usage.storage_bytes,
            storage_mb,
            projection_queries: user.usage.projection_queries,
            bandwidth_bytes: user.usage.bandwidth_bytes,
            bandwidth_mb,
            period_start: user.usage.period_start.map(|d| d.to_string()),
            last_updated: user.usage.last_updated.map(|d| d.to_string()),
        },
        quota: QuotaResponse {
            storage_limit_bytes: user.quota.storage_limit,
            storage_limit_mb,
            storage_percent_used: user.storage_percent(),
            daily_query_limit: user.quota.daily_query_limit,
            queries_percent_used: user.queries_percent(),
            daily_bandwidth_limit_bytes: user.quota.daily_bandwidth_limit,
            daily_bandwidth_limit_mb: bandwidth_limit_mb,
            bandwidth_percent_used: user.bandwidth_percent(),
            enforce_hard_limit: user.quota.enforce_hard_limit,
            is_over_quota: user.is_over_quota(),
        },
    }
}

// =============================================================================
// Usage Tracking Integration Helpers
// =============================================================================

/// Try to extract user ID from request JWT for usage tracking.
/// Returns None if no valid JWT is present (anonymous request).
///
/// Use this in routes where tracking is optional (blob/projection routes).
pub fn try_extract_user_id_for_tracking<B>(
    req: &Request<B>,
    jwt_secret: Option<&str>,
    dev_mode: bool,
) -> Option<String> {
    let auth_header = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let token = extract_token_from_header(auth_header)?;

    let jwt = if dev_mode {
        JwtValidator::new_dev()
    } else {
        JwtValidator::new(jwt_secret?.to_string(), 86400).ok()?
    };

    let result = jwt.verify_token(token);
    if result.valid {
        result.claims.map(|c| c.human_id)
    } else {
        None
    }
}

/// Track bandwidth usage for a request.
/// Call this after serving content to a user.
///
/// This is a fire-and-forget operation - errors are logged but don't affect the response.
pub async fn track_bandwidth_if_user(
    tracker: &dyn UsageTracker,
    user_id: Option<&str>,
    bytes: u64,
) {
    if let Some(uid) = user_id {
        if let Err(e) = tracker.increment_bandwidth(uid, bytes).await {
            warn!("Failed to track bandwidth for user {}: {}", uid, e);
        }
    }
}

/// Track projection query usage for a request.
/// Call this after executing a projection query for a user.
pub async fn track_query_if_user(tracker: &dyn UsageTracker, user_id: Option<&str>) {
    if let Some(uid) = user_id {
        if let Err(e) = tracker.increment_queries(uid, 1).await {
            warn!("Failed to track query for user {}: {}", uid, e);
        }
    }
}

/// Check quota before allowing an operation.
/// Returns QuotaStatus indicating whether the operation is allowed.
///
/// For anonymous users, returns an allowed status.
pub async fn check_quota_if_user(
    enforcer: &dyn QuotaEnforcer,
    user_id: Option<&str>,
) -> QuotaStatus {
    match user_id {
        Some(uid) => match enforcer.check_quota(uid).await {
            Ok(status) => status,
            Err(_) => QuotaStatus {
                allowed: true,
                storage_exceeded: false,
                queries_exceeded: false,
                bandwidth_exceeded: false,
                message: None,
            },
        },
        None => QuotaStatus {
            allowed: true,
            storage_exceeded: false,
            queries_exceeded: false,
            bandwidth_exceeded: false,
            message: None,
        },
    }
}
