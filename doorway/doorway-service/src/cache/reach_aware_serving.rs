//! Reach-aware serving logic for the Doorway API
//!
//! Integrates reach-level access control with cache key generation and serving decisions.

use crate::cache::{can_serve_at_reach, CacheKey, RequesterContext};
use serde_json::Value;
use std::net::IpAddr;

/// Extract reach level from API response data
pub fn extract_reach_from_response(response_data: &[u8]) -> Option<String> {
    // Parse JSON response
    if let Ok(json_val) = serde_json::from_slice::<Value>(response_data) {
        // Check for "reach" field (scalar)
        if let Some(reach_str) = json_val.get("reach").and_then(|v| v.as_str()) {
            return Some(reach_str.to_string());
        }

        // Check for reach in array items (for list responses)
        if let Some(arr) = json_val.as_array() {
            if let Some(first) = arr.first() {
                if let Some(reach_str) = first.get("reach").and_then(|v| v.as_str()) {
                    return Some(reach_str.to_string());
                }
            }
        }
    }

    None
}

/// Create cache key with reach level for reach-aware caching
pub fn create_reach_aware_cache_key(cache_key: &CacheKey, response_data: &[u8]) -> CacheKey {
    if let Some(reach) = extract_reach_from_response(response_data) {
        // Create a copy with reach
        let mut key = cache_key.clone();
        key.reach = Some(reach);
        key
    } else {
        // No reach found, use normal cache key
        cache_key.clone()
    }
}

/// Check if requester can be served the response based on reach level
///
/// Gates serving by:
/// 1. Reach level (private, local, commons, etc.)
/// 2. Requester authentication
/// 3. Requester identity (for private reach)
pub fn should_serve_response(
    response_data: &[u8],
    requester: &RequesterContext,
    beneficiary_id: &str,
) -> bool {
    // Extract reach from response
    if let Some(reach) = extract_reach_from_response(response_data) {
        // Check if requester can access this reach level
        can_serve_at_reach(&reach, requester, beneficiary_id)
    } else {
        // No reach specified, allow serving (backward compat)
        true
    }
}

/// Extract requester context from HTTP headers
pub fn extract_requester_context(
    auth_header: Option<&str>,
    ip_address: Option<IpAddr>,
) -> RequesterContext {
    // Simplified: extract agent ID from auth header (JWT, agent pubkey, etc.)
    let agent_id = auth_header
        .and_then(parse_agent_from_auth)
        .unwrap_or_else(|| "anonymous".to_string());

    // Extract location from IP (simplified, would use GeoIP in production)
    let location = ip_address.map(|ip| ip.to_string());

    RequesterContext {
        agent_id,
        location,
        authenticated: auth_header.is_some(),
    }
}

/// Parse agent ID from Authorization header (simplified)
fn parse_agent_from_auth(auth_header: &str) -> Option<String> {
    // Format: "Bearer <agent_id>" or "Signature <agent_id>"
    let parts: Vec<&str> = auth_header.split_whitespace().collect();
    if parts.len() == 2 {
        Some(parts[1].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_reach_from_single_object() {
        let data = json!({
            "id": "content123",
            "title": "My Content",
            "reach": "commons"
        });

        let bytes = serde_json::to_vec(&data).unwrap();
        let reach = extract_reach_from_response(&bytes);

        assert_eq!(reach, Some("commons".to_string()));
    }

    #[test]
    fn test_extract_reach_from_array() {
        let data = json!([
            {
                "id": "content1",
                "reach": "private"
            },
            {
                "id": "content2",
                "reach": "commons"
            }
        ]);

        let bytes = serde_json::to_vec(&data).unwrap();
        let reach = extract_reach_from_response(&bytes);

        // Should extract from first item
        assert_eq!(reach, Some("private".to_string()));
    }

    #[test]
    fn test_extract_reach_missing() {
        let data = json!({
            "id": "content123",
            "title": "No reach field"
        });

        let bytes = serde_json::to_vec(&data).unwrap();
        let reach = extract_reach_from_response(&bytes);

        assert_eq!(reach, None);
    }

    #[test]
    fn test_should_serve_commons_content_unauthenticated() {
        let response = json!({
            "id": "content123",
            "reach": "commons"
        });
        let bytes = serde_json::to_vec(&response).unwrap();

        let requester = RequesterContext {
            agent_id: "stranger".to_string(),
            location: None,
            authenticated: false,
        };

        assert!(should_serve_response(&bytes, &requester, "alice"));
    }

    #[test]
    fn test_should_not_serve_private_content_to_non_owner() {
        let response = json!({
            "id": "content123",
            "reach": "private"
        });
        let bytes = serde_json::to_vec(&response).unwrap();

        let requester = RequesterContext {
            agent_id: "bob".to_string(),
            location: None,
            authenticated: true,
        };

        assert!(!should_serve_response(&bytes, &requester, "alice"));
    }

    #[test]
    fn test_should_serve_private_content_to_owner() {
        let response = json!({
            "id": "content123",
            "reach": "private"
        });
        let bytes = serde_json::to_vec(&response).unwrap();

        let requester = RequesterContext {
            agent_id: "alice".to_string(),
            location: None,
            authenticated: true,
        };

        assert!(should_serve_response(&bytes, &requester, "alice"));
    }

    #[test]
    fn test_extract_requester_context() {
        let auth_header = "Bearer alice-pubkey-123";
        let context = extract_requester_context(Some(auth_header), None);

        assert_eq!(context.agent_id, "alice-pubkey-123");
        assert!(context.authenticated);
    }
}
