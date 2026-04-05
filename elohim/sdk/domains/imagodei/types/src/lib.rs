//! Wire types for imagodei domain coordinator functions.
//!
//! These types define the MessagePack-serialized inputs and outputs for
//! imagodei zome calls. They are consumed by:
//! - The imagodei coordinator zome (WASM target)
//! - Doorway gateway service (native target)
//! - Any future client that calls imagodei functions
//!
//! This crate is an IoC artifact in `sdk/domains/imagodei/`, alongside
//! the domain's schemas and manifest. It must NOT depend on HDK, HDI,
//! or any WASM-specific crates.

use holo_hash::ActionHash;
use serde::{Deserialize, Serialize};

// =============================================================================
// Human Profile Types
// =============================================================================

/// Input for imagodei::create_human coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateHumanInput {
    pub id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    pub affinities: Vec<String>,
    pub profile_reach: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// Human profile fields.
///
/// Matches the integrity zome's Human entry type field-for-field.
/// The integrity zome wraps this with `#[hdk_entry_helper]` for DHT storage;
/// this version uses plain serde for wire format compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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

/// Output from imagodei::create_human coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HumanOutput {
    pub action_hash: ActionHash,
    pub human: Human,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_human_input_msgpack_roundtrip() {
        let input = CreateHumanInput {
            id: "test-123".to_string(),
            display_name: "Test User".to_string(),
            bio: Some("A test user".to_string()),
            affinities: vec!["testing".to_string()],
            profile_reach: "public".to_string(),
            location: None,
        };

        let bytes = rmp_serde::to_vec(&input).unwrap();
        let decoded: CreateHumanInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "test-123");
        assert_eq!(decoded.display_name, "Test User");
    }

    #[test]
    fn human_msgpack_roundtrip() {
        let human = Human {
            id: "test-456".to_string(),
            display_name: "Another User".to_string(),
            bio: None,
            affinities: vec![],
            profile_reach: "community".to_string(),
            location: Some("Earth".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let bytes = rmp_serde::to_vec(&human).unwrap();
        let decoded: Human = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "test-456");
        assert_eq!(decoded.location, Some("Earth".to_string()));
    }
}
