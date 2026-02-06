//! Zome Call Helpers - Make zome calls from HTTP route handlers
//!
//! Provides helper functions for calling specific zome functions from the
//! doorway's HTTP handlers, particularly for identity management operations.

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::server::AppState;
use crate::types::{DoorwayError, Result};
use crate::worker::{ZomeCallBuilder, ZomeCallConfig};

// =============================================================================
// Imagodei Zome Types
// =============================================================================

/// Input for imagodei::create_human zome call
/// Must match the Rust struct in holochain/dna/imagodei/zomes/imagodei/src/lib.rs
#[derive(Debug, Clone, Serialize)]
pub struct CreateHumanInput {
    pub id: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    pub affinities: Vec<String>,
    pub profile_reach: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// Output from imagodei::create_human
/// Matches HumanOutput in the zome
#[derive(Debug, Clone, Deserialize)]
pub struct HumanOutput {
    pub action_hash: Vec<u8>,
    pub human: Human,
}

/// Human entry from the zome
#[derive(Debug, Clone, Deserialize)]
pub struct Human {
    pub id: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub affinities: Vec<String>,
    pub profile_reach: String,
    pub location: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Zome Call Functions
// =============================================================================

/// Call imagodei::create_human via the worker pool
///
/// This creates a new Human profile in the imagodei DNA, bound to the
/// calling agent's public key.
///
/// # Arguments
/// * `state` - AppState containing worker pool and zome configs
/// * `input` - CreateHumanInput with profile data
///
/// # Returns
/// * `Ok(HumanOutput)` - Created human with action_hash
/// * `Err(DoorwayError)` - If worker pool unavailable, zome config not found, or zome call fails
pub async fn call_create_human(
    state: &AppState,
    input: CreateHumanInput,
) -> Result<HumanOutput> {
    // Get worker pool for APP interface
    let pool = state.pool.as_ref()
        .ok_or_else(|| DoorwayError::Internal("Worker pool not available - conductor not connected?".into()))?;

    // Get imagodei zome config
    let zome_config = get_zome_config_by_role(state, "imagodei")?;

    debug!(
        human_id = %input.id,
        display_name = %input.display_name,
        "Calling create_human on imagodei zome"
    );

    // Build the zome call
    let builder = ZomeCallBuilder::new(zome_config);
    let payload = builder.build_zome_call("create_human", &input)?;

    // Send via worker pool
    let response = pool.request(payload).await
        .map_err(|e| DoorwayError::Holochain(format!("Zome call failed: {}", e)))?;

    // Parse response
    let result: HumanOutput = builder.parse_response(&response)?
        .ok_or_else(|| DoorwayError::Holochain("Empty response from create_human".into()))?;

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
pub fn get_agent_pub_key(state: &AppState) -> Result<String> {
    // Try imagodei first, fall back to any available config
    if let Ok(config) = get_zome_config_by_role(state, "imagodei") {
        return Ok(config.agent_pub_key);
    }

    // Fall back to first available config
    for entry in state.zome_configs.iter() {
        return Ok(entry.value().agent_pub_key.clone());
    }

    Err(DoorwayError::Internal("No zome configs discovered - conductor not ready?".into()))
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
    let available: Vec<String> = state.zome_configs.iter()
        .map(|e| e.value().role_name.clone())
        .collect();
    warn!(
        role_name = %role_name,
        available = ?available,
        "Zome config not found for role"
    );

    Err(DoorwayError::Internal(format!(
        "No zome config found for role '{}'. Available: {:?}",
        role_name, available
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
    fn test_human_output_deserialization() {
        // Simulate a response from the zome
        let json = r#"{
            "action_hash": [1, 2, 3, 4],
            "human": {
                "id": "test-123",
                "display_name": "Test",
                "bio": null,
                "affinities": [],
                "profile_reach": "public",
                "location": null,
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            }
        }"#;

        let output: HumanOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.human.id, "test-123");
        assert_eq!(output.human.display_name, "Test");
    }
}
