//! JWT Token Handling for Hosted Human Authentication
//!
//! Provides functions for generating and validating JWT tokens used to
//! authenticate hosted humans to the edge node.
//!
//! Security notes:
//! - Tokens are signed with HS256 (HMAC-SHA256)
//! - Default expiry is 1 hour
//! - In production, JWT_SECRET should be a strong random value from environment
//!
//! Ported from admin-proxy/src/jwt.ts

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::auth::PermissionLevel;
use crate::types::DoorwayError;

/// Payload stored in JWT token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Holochain human ID
    pub human_id: String,
    /// Holochain agent public key (hex string)
    pub agent_pub_key: String,
    /// User identifier (email/username)
    pub identifier: String,
    /// Permission level granted
    pub permission_level: PermissionLevel,
    /// Token version (for future invalidation)
    pub version: u32,
    /// Issued at (Unix timestamp)
    pub iat: u64,
    /// Expiration time (Unix timestamp)
    pub exp: u64,
}

/// Input for creating a new token
#[derive(Debug, Clone)]
pub struct TokenInput {
    pub human_id: String,
    pub agent_pub_key: String,
    pub identifier: String,
    pub permission_level: PermissionLevel,
}

/// Result of token validation
#[derive(Debug)]
pub struct TokenValidationResult {
    pub valid: bool,
    pub claims: Option<Claims>,
    pub error: Option<String>,
}

impl TokenValidationResult {
    pub fn valid(claims: Claims) -> Self {
        Self {
            valid: true,
            claims: Some(claims),
            error: None,
        }
    }

    pub fn invalid(error: impl Into<String>) -> Self {
        Self {
            valid: false,
            claims: None,
            error: Some(error.into()),
        }
    }
}

/// JWT validator and generator
#[derive(Clone)]
pub struct JwtValidator {
    secret: String,
    expiry_seconds: u64,
}

impl JwtValidator {
    /// Create a new JWT validator
    ///
    /// Returns an error if the secret is empty or too short
    pub fn new(secret: String, expiry_seconds: u64) -> Result<Self, DoorwayError> {
        if secret.is_empty() {
            return Err(DoorwayError::Config(
                "JWT_SECRET is required in production mode".into(),
            ));
        }

        if secret.len() < 32 {
            return Err(DoorwayError::Config(
                "JWT_SECRET must be at least 32 characters".into(),
            ));
        }

        Ok(Self {
            secret,
            expiry_seconds,
        })
    }

    /// Create a validator for dev mode (allows empty secret)
    pub fn new_dev() -> Self {
        Self {
            secret: "dev-mode-secret-not-for-production-use-123456".into(),
            expiry_seconds: 3600,
        }
    }

    /// Generate a JWT token for an authenticated user
    pub fn generate_token(&self, input: TokenInput) -> Result<String, DoorwayError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| DoorwayError::Auth(format!("System time error: {}", e)))?
            .as_secs();

        let claims = Claims {
            human_id: input.human_id,
            agent_pub_key: input.agent_pub_key,
            identifier: input.identifier,
            permission_level: input.permission_level,
            version: 1,
            iat: now,
            exp: now + self.expiry_seconds,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| DoorwayError::Auth(format!("Failed to generate token: {}", e)))?;

        Ok(token)
    }

    /// Generate a refresh token with longer expiry (7 days)
    pub fn generate_refresh_token(&self, input: TokenInput) -> Result<String, DoorwayError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| DoorwayError::Auth(format!("System time error: {}", e)))?
            .as_secs();

        // Refresh tokens last 7 days
        let refresh_expiry = 7 * 24 * 60 * 60;

        let claims = Claims {
            human_id: input.human_id,
            agent_pub_key: input.agent_pub_key,
            identifier: input.identifier,
            permission_level: input.permission_level,
            version: 1,
            iat: now,
            exp: now + refresh_expiry,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| DoorwayError::Auth(format!("Failed to generate refresh token: {}", e)))?;

        Ok(token)
    }

    /// Verify and decode a JWT token
    pub fn verify_token(&self, token: &str) -> TokenValidationResult {
        let validation = Validation::default();

        match decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        ) {
            Ok(token_data) => TokenValidationResult::valid(token_data.claims),
            Err(err) => {
                use jsonwebtoken::errors::ErrorKind;
                let error_msg = match err.kind() {
                    ErrorKind::ExpiredSignature => "Token expired",
                    ErrorKind::InvalidToken => "Invalid token",
                    ErrorKind::InvalidSignature => "Invalid signature",
                    _ => "Token validation failed",
                };
                TokenValidationResult::invalid(error_msg)
            }
        }
    }

    /// Check if a token is close to expiring
    pub fn is_token_expiring_soon(&self, claims: &Claims, threshold_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        claims.exp.saturating_sub(now) < threshold_seconds
    }

    /// Get remaining time until token expiry
    pub fn get_token_time_remaining(&self, claims: &Claims) -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        claims.exp as i64 - now as i64
    }
}

/// Extract token from Authorization header.
/// Supports "Bearer <token>" format and raw tokens.
pub fn extract_token_from_header(auth_header: Option<&str>) -> Option<&str> {
    let header = auth_header?;

    // Support "Bearer <token>" format
    if let Some(token) = header.strip_prefix("Bearer ") {
        let token = token.trim();
        if !token.is_empty() {
            return Some(token);
        }
    }

    // Also support raw token (for flexibility)
    if !header.contains(' ') {
        let token = header.trim();
        if !token.is_empty() {
            return Some(token);
        }
    }

    None
}

/// Extract token from URL query parameter
pub fn extract_token_from_url(url: &str, param_name: &str) -> Option<String> {
    // Simple query string parsing
    let query_start = url.find('?')?;
    let query = &url[query_start + 1..];

    for param in query.split('&') {
        if let Some((key, value)) = param.split_once('=') {
            if key == param_name {
                return Some(value.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_validator() -> JwtValidator {
        JwtValidator::new(
            "test-secret-that-is-at-least-32-characters-long".into(),
            3600,
        )
        .unwrap()
    }

    #[test]
    fn test_generate_and_verify_token() {
        let validator = test_validator();

        let input = TokenInput {
            human_id: "human-123".into(),
            agent_pub_key: "uhCAk...".into(),
            identifier: "test@example.com".into(),
            permission_level: PermissionLevel::Authenticated,
        };

        let token = validator.generate_token(input).unwrap();
        assert!(!token.is_empty());

        let result = validator.verify_token(&token);
        assert!(result.valid);

        let claims = result.claims.unwrap();
        assert_eq!(claims.human_id, "human-123");
        assert_eq!(claims.identifier, "test@example.com");
        assert_eq!(claims.permission_level, PermissionLevel::Authenticated);
    }

    #[test]
    fn test_invalid_token() {
        let validator = test_validator();

        let result = validator.verify_token("invalid-token");
        assert!(!result.valid);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_wrong_secret() {
        let validator1 = test_validator();
        let validator2 = JwtValidator::new(
            "different-secret-that-is-at-least-32-characters".into(),
            3600,
        )
        .unwrap();

        let input = TokenInput {
            human_id: "human-123".into(),
            agent_pub_key: "uhCAk...".into(),
            identifier: "test@example.com".into(),
            permission_level: PermissionLevel::Authenticated,
        };

        let token = validator1.generate_token(input).unwrap();

        // Verify with wrong secret should fail
        let result = validator2.verify_token(&token);
        assert!(!result.valid);
    }

    #[test]
    fn test_extract_token_from_header() {
        // Bearer format
        assert_eq!(
            extract_token_from_header(Some("Bearer abc123")),
            Some("abc123")
        );

        // Raw token
        assert_eq!(extract_token_from_header(Some("abc123")), Some("abc123"));

        // Empty cases
        assert_eq!(extract_token_from_header(None), None);
        assert_eq!(extract_token_from_header(Some("")), None);
        assert_eq!(extract_token_from_header(Some("Bearer ")), None);

        // Invalid format
        assert_eq!(extract_token_from_header(Some("Basic abc123")), None);
    }

    #[test]
    fn test_extract_token_from_url() {
        assert_eq!(
            extract_token_from_url("http://localhost?token=abc123", "token"),
            Some("abc123".into())
        );

        assert_eq!(
            extract_token_from_url("http://localhost?foo=bar&token=abc123", "token"),
            Some("abc123".into())
        );

        assert_eq!(
            extract_token_from_url("http://localhost?foo=bar", "token"),
            None
        );

        assert_eq!(extract_token_from_url("http://localhost", "token"), None);
    }

    #[test]
    fn test_secret_validation() {
        // Too short
        assert!(JwtValidator::new("short".into(), 3600).is_err());

        // Empty
        assert!(JwtValidator::new("".into(), 3600).is_err());

        // Valid
        assert!(JwtValidator::new("this-secret-is-at-least-32-chars-long".into(), 3600).is_ok());
    }

    #[test]
    fn test_dev_mode_validator() {
        let validator = JwtValidator::new_dev();

        let input = TokenInput {
            human_id: "human-123".into(),
            agent_pub_key: "uhCAk...".into(),
            identifier: "test@example.com".into(),
            permission_level: PermissionLevel::Admin,
        };

        let token = validator.generate_token(input).unwrap();
        let result = validator.verify_token(&token);
        assert!(result.valid);
    }
}
