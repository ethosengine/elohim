//! Zome Call Helpers - Make zome calls from HTTP route handlers
//!
//! Provides helper functions for calling specific zome functions from the
//! doorway's HTTP handlers, particularly for identity management operations.

use tracing::{debug, warn};

use crate::server::AppState;
use crate::types::{DoorwayError, Result};
use crate::worker::ZomeCallConfig;

// Wire types from SDK domain crate — compiler enforces zome/doorway agreement
pub use imagodei_types::{CreateHumanInput, Human, HumanOutput};

// =============================================================================
// Zome Call Functions
// =============================================================================

/// Call imagodei::create_human via ZomeCaller
///
/// This creates a new Human profile in the imagodei DNA, bound to the
/// calling agent's public key.
///
/// Uses ZomeCaller which passes role_name directly to the conductor
/// (the conductor resolves role_name to cell_id internally). This avoids
/// depending on discovery populating zome_configs.
///
/// # Arguments
/// * `state` - AppState containing ZomeCaller
/// * `input` - CreateHumanInput with profile data
///
/// # Returns
/// * `Ok(HumanOutput)` - Created human with action_hash
/// * `Err(DoorwayError)` - If ZomeCaller unavailable or zome call fails
pub async fn call_create_human(state: &AppState, input: CreateHumanInput) -> Result<HumanOutput> {
    let zome_caller = state.zome_caller.as_ref().ok_or_else(|| {
        DoorwayError::Internal("ZomeCaller not available - conductor not configured?".into())
    })?;

    debug!(
        human_id = %input.id,
        display_name = %input.display_name,
        "Calling create_human on imagodei zome via ZomeCaller"
    );

    let result: HumanOutput = zome_caller
        .call("imagodei", "imagodei", "create_human", &input)
        .await
        .map_err(|e| DoorwayError::Holochain(format!("create_human failed: {e}")))?;

    debug!(
        human_id = %result.human.id,
        "Successfully created human in imagodei zome"
    );

    Ok(result)
}

/// Get agent public key from the imagodei zome config
///
/// Returns the agent public key that the conductor uses for this app.
/// This is needed for auth responses.
///
/// Requires discovery to have completed successfully (imagodei role
/// must be in zome_configs). Returns error if not found.
pub fn get_agent_pub_key(state: &AppState) -> Result<String> {
    get_zome_config_by_role(state, "imagodei").map(|config| config.agent_pub_key)
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Get ZomeCallConfig by role name
///
/// Searches through discovered zome configs to find the one with matching role_name.
/// Role names are defined in the hApp manifest (e.g., "lamad", "imagodei", "infrastructure").
fn get_zome_config_by_role(state: &AppState, role_name: &str) -> Result<ZomeCallConfig> {
    for entry in state.zome_configs.iter() {
        let config = entry.value();
        if config.role_name == role_name {
            // Clone the config and set the correct zome name
            let mut result = config.clone();
            // For imagodei role, the zome is also named "imagodei"
            result.zome_name = role_name.to_string();
            return Ok(result);
        }
    }

    // Log available configs for debugging
    let available: Vec<String> = state
        .zome_configs
        .iter()
        .map(|e| e.value().role_name.clone())
        .collect();
    warn!(
        role_name = %role_name,
        available = ?available,
        "Zome config not found for role"
    );

    Err(DoorwayError::Internal(format!(
        "No zome config found for role '{role_name}'. Available: {available:?}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_human_input_serialization() {
        let input = CreateHumanInput {
            id: "test-human-123".to_string(),
            display_name: "Test User".to_string(),
            bio: Some("A test user".to_string()),
            affinities: vec!["testing".to_string()],
            profile_reach: "public".to_string(),
            location: None,
        };

        // Test MessagePack serialization (what conductor expects)
        let bytes = rmp_serde::to_vec(&input).unwrap();
        assert!(!bytes.is_empty());

        // Test JSON serialization (for debugging)
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("test-human-123"));
        assert!(json.contains("Test User"));
    }

    #[test]
    fn test_human_deserialization() {
        // Test Human struct deserialization (HumanOutput uses ActionHash
        // which requires MessagePack from the conductor, not JSON)
        let json = r#"{
            "id": "test-123",
            "display_name": "Test",
            "bio": null,
            "affinities": [],
            "profile_reach": "public",
            "location": null,
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;

        let human: Human = serde_json::from_str(json).unwrap();
        assert_eq!(human.id, "test-123");
        assert_eq!(human.display_name, "Test");
    }
}
