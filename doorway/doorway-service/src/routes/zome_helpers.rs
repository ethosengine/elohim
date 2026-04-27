//! Zome Call Helpers - Make zome calls from HTTP route handlers
//!
//! Provides helper functions for calling specific zome functions from the
//! doorway's HTTP handlers, particularly for identity management operations.

use holo_hash::ActionHash;
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

    // create_human now returns the bare ActionHash so its wire shape can be
    // read by sweettests as either typed ActionHash or serde_json::Value.
    // Chase with get_my_human to recover the full HumanOutput projection.
    let action_hash: ActionHash = zome_caller
        .call("imagodei", "imagodei", "create_human", &input)
        .await
        .map_err(|e| DoorwayError::Holochain(format!("create_human failed: {e}")))?;

    let projected: Option<HumanOutput> = zome_caller
        .call("imagodei", "imagodei", "get_my_human", &())
        .await
        .map_err(|e| {
            DoorwayError::Holochain(format!(
                "create_human ok but get_my_human projection failed: {e}"
            ))
        })?;

    let result = projected.ok_or_else(|| {
        DoorwayError::Internal(
            "create_human ok but get_my_human returned None — DHT projection lag?".into(),
        )
    })?;

    debug!(
        human_id = %result.human.id,
        action_hash = %action_hash,
        "Successfully created human in imagodei zome"
    );

    Ok(result)
}

/// Call imagodei::get_my_human via ZomeCaller
///
/// Returns the existing Human profile for the calling agent, or None if not found.
/// Used to recover an existing identity when create_human is rejected because the
/// agent already has a profile (e.g. doorway DB cleared but conductor not reset).
pub async fn call_get_my_human(state: &AppState) -> Result<Option<HumanOutput>> {
    let zome_caller = state.zome_caller.as_ref().ok_or_else(|| {
        DoorwayError::Internal("ZomeCaller not available - conductor not configured?".into())
    })?;

    debug!("Calling get_my_human on imagodei zome via ZomeCaller");

    let result: Option<HumanOutput> = zome_caller
        .call("imagodei", "imagodei", "get_my_human", &())
        .await
        .map_err(|e| DoorwayError::Holochain(format!("get_my_human failed: {e}")))?;

    Ok(result)
}

/// Call imagodei::create_human on a specific conductor (not the singleton ZomeCaller).
///
/// Used for `hosted` registrations where the human's identity is created on
/// the operator's conductor (identified during provisioning), not the doorway's
/// default ZomeCaller target.
pub async fn call_create_human_on_conductor(
    conductor_url: &str,
    installed_app_id: &str,
    input: CreateHumanInput,
) -> crate::types::Result<HumanOutput> {
    let admin_url = crate::derive_admin_url_from_app(conductor_url);

    debug!(
        conductor_url = %conductor_url,
        admin_url = %admin_url,
        installed_app_id = %installed_app_id,
        human_id = %input.id,
        "Creating temporary ZomeCaller for hosted registration"
    );

    let caller = crate::services::ZomeCaller::new(&admin_url, conductor_url, installed_app_id);

    // Same chase pattern as call_create_human — see that helper for rationale.
    let action_hash: ActionHash = caller
        .call("imagodei", "imagodei", "create_human", &input)
        .await
        .map_err(|e| {
            crate::types::DoorwayError::Holochain(format!("create_human on conductor failed: {e}"))
        })?;

    let projected: Option<HumanOutput> = caller
        .call("imagodei", "imagodei", "get_my_human", &())
        .await
        .map_err(|e| {
            crate::types::DoorwayError::Holochain(format!(
                "create_human on conductor ok but get_my_human projection failed: {e}"
            ))
        })?;

    let result = projected.ok_or_else(|| {
        crate::types::DoorwayError::Internal(
            "create_human on conductor ok but get_my_human returned None — DHT projection lag?"
                .into(),
        )
    })?;

    debug!(action_hash = %action_hash, "create_human on conductor succeeded");

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
