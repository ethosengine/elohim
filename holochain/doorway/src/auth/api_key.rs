//! API Key authentication for backward compatibility
//!
//! Supports the legacy API key authentication from admin-proxy.
//! API keys are passed via X-API-Key header and grant specific permission levels.

use crate::auth::PermissionLevel;

/// API key validator for legacy authentication
#[derive(Debug, Clone)]
pub struct ApiKeyValidator {
    /// API key for authenticated (normal user) operations
    authenticated_key: Option<String>,
    /// API key for admin (destructive) operations
    admin_key: Option<String>,
}

impl ApiKeyValidator {
    /// Create a new API key validator
    pub fn new(authenticated_key: Option<String>, admin_key: Option<String>) -> Self {
        Self {
            authenticated_key: authenticated_key.filter(|k| !k.is_empty()),
            admin_key: admin_key.filter(|k| !k.is_empty()),
        }
    }

    /// Check if API key authentication is configured
    pub fn is_configured(&self) -> bool {
        self.authenticated_key.is_some() || self.admin_key.is_some()
    }

    /// Validate an API key and return the granted permission level
    ///
    /// Returns None if the key is invalid or not configured.
    /// Returns Public level if no key is provided but no keys are required.
    pub fn validate(&self, api_key: Option<&str>) -> Option<PermissionLevel> {
        match api_key {
            Some(key) => {
                // Check admin key first (highest privilege)
                if let Some(ref admin) = self.admin_key {
                    if constant_time_compare(key, admin) {
                        return Some(PermissionLevel::Admin);
                    }
                }

                // Check authenticated key
                if let Some(ref auth) = self.authenticated_key {
                    if constant_time_compare(key, auth) {
                        return Some(PermissionLevel::Authenticated);
                    }
                }

                // Invalid key
                None
            }
            None => {
                // No key provided - grant public access only
                Some(PermissionLevel::Public)
            }
        }
    }

    /// Extract API key from request headers
    pub fn extract_from_header(header: Option<&str>) -> Option<&str> {
        header.filter(|h| !h.is_empty())
    }
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_keys_configured() {
        let validator = ApiKeyValidator::new(None, None);
        assert!(!validator.is_configured());

        // No key should get public access
        assert_eq!(validator.validate(None), Some(PermissionLevel::Public));

        // Any key should be rejected (not configured)
        assert_eq!(validator.validate(Some("any-key")), None);
    }

    #[test]
    fn test_admin_key_validation() {
        let validator = ApiKeyValidator::new(None, Some("admin-secret".into()));
        assert!(validator.is_configured());

        // Correct admin key
        assert_eq!(
            validator.validate(Some("admin-secret")),
            Some(PermissionLevel::Admin)
        );

        // Wrong key
        assert_eq!(validator.validate(Some("wrong-key")), None);
    }

    #[test]
    fn test_authenticated_key_validation() {
        let validator = ApiKeyValidator::new(Some("auth-secret".into()), None);
        assert!(validator.is_configured());

        // Correct auth key
        assert_eq!(
            validator.validate(Some("auth-secret")),
            Some(PermissionLevel::Authenticated)
        );

        // Wrong key
        assert_eq!(validator.validate(Some("wrong-key")), None);
    }

    #[test]
    fn test_both_keys_configured() {
        let validator =
            ApiKeyValidator::new(Some("auth-secret".into()), Some("admin-secret".into()));

        // Admin key gets admin level
        assert_eq!(
            validator.validate(Some("admin-secret")),
            Some(PermissionLevel::Admin)
        );

        // Auth key gets authenticated level
        assert_eq!(
            validator.validate(Some("auth-secret")),
            Some(PermissionLevel::Authenticated)
        );

        // No key gets public
        assert_eq!(validator.validate(None), Some(PermissionLevel::Public));
    }

    #[test]
    fn test_empty_keys_treated_as_none() {
        let validator = ApiKeyValidator::new(Some("".into()), Some("".into()));
        assert!(!validator.is_configured());
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("hello", "hello"));
        assert!(!constant_time_compare("hello", "world"));
        assert!(!constant_time_compare("hello", "hell"));
        assert!(!constant_time_compare("hell", "hello"));
    }
}
