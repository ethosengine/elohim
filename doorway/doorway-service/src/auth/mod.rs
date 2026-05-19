//! Authentication and authorization for Doorway
//!
//! Provides:
//! - JWT token generation and validation
//! - API key authentication (for backward compatibility)
//! - Permission levels for operation authorization
//! - Password hashing with Argon2

pub mod api_key;
pub mod jwt;
pub mod operator;
pub mod password;
pub mod permissions;
pub mod portal_host;

pub use api_key::ApiKeyValidator;
pub use jwt::{
    extract_token_from_header, Claims, JwtValidator, OperatorSnapshot, TokenInput,
    TokenValidationResult,
};
pub use operator::{
    capability_satisfies, resolve_operator_capability, CapabilityError, CapabilityResult,
    WILDCARD_CAPABILITY,
};
pub use password::{hash_password, verify_password};
pub use permissions::{get_required_permission, is_operation_allowed, PermissionLevel};
