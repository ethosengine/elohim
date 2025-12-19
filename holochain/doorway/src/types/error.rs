//! Error types for Doorway
//!
//! Pattern adapted from holo-host/rust/holo-gateway/src/types/error.rs

use hyper::StatusCode;

/// Main error type for Doorway operations
#[derive(Debug, thiserror::Error)]
pub enum DoorwayError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Holochain error: {0}")]
    Holochain(String),

    #[error("NATS error: {0}")]
    Nats(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication error: {0}")]
    Auth(String),
}

impl DoorwayError {
    /// Convert error to HTTP status code
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::WebSocket(_) => StatusCode::BAD_GATEWAY,
            Self::Holochain(_) => StatusCode::BAD_GATEWAY,
            Self::Nats(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Database(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Http(_) => StatusCode::BAD_REQUEST,
            Self::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Auth(_) => StatusCode::UNAUTHORIZED,
        }
    }

    /// Convert to status code and body tuple for HTTP response
    pub fn into_status_code_and_body(self) -> (StatusCode, String) {
        let status = self.status_code();
        let body = self.to_string();
        (status, body)
    }
}

// Implement From conversions for common error types

impl From<std::io::Error> for DoorwayError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<serde_json::Error> for DoorwayError {
    fn from(err: serde_json::Error) -> Self {
        Self::BadRequest(format!("JSON error: {}", err))
    }
}

impl From<hyper::Error> for DoorwayError {
    fn from(err: hyper::Error) -> Self {
        Self::Internal(format!("HTTP error: {}", err))
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for DoorwayError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WebSocket(err.to_string())
    }
}

impl From<async_nats::Error> for DoorwayError {
    fn from(err: async_nats::Error) -> Self {
        Self::Nats(err.to_string())
    }
}

impl From<mongodb::error::Error> for DoorwayError {
    fn from(err: mongodb::error::Error) -> Self {
        Self::Database(err.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for DoorwayError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        Self::Unauthorized(format!("JWT error: {}", err))
    }
}

/// Result type alias for Doorway operations
pub type Result<T> = std::result::Result<T, DoorwayError>;
