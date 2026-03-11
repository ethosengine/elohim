//! Authentication and authorization for Doorway
//!
//! Provides:
//! - JWT token generation and validation
//! - API key authentication (for backward compatibility)
//! - Permission levels for operation authorization
//! - Password hashing with Argon2

pub mod api_key;
pub mod jwt;
pub mod password;
pub mod permissions;

pub use api_key::ApiKeyValidator;
pub use jwt::{extract_token_from_header, Claims, JwtValidator, TokenInput, TokenValidationResult};
pub use password::{hash_password, verify_password};
pub use permissions::{get_required_permission, is_operation_allowed, PermissionLevel};
