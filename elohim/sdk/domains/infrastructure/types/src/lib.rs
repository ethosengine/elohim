//! Wire types for infrastructure domain coordinator functions.
//!
//! These types define the MessagePack-serialized inputs and outputs for
//! infrastructure zome calls. Consumed by:
//! - The infrastructure coordinator zome (WASM target)
//! - Doorway gateway service — federation.rs (native target)

use holo_hash::ActionHash;
use serde::{Deserialize, Serialize};

// =============================================================================
// Doorway Registration Types
// =============================================================================

/// Input for infrastructure::register_doorway coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RegisterDoorwayInput {
    pub id: String,
    pub url: String,
    pub capabilities_json: String,
    pub reach: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandwidth_mbps: Option<u32>,
    pub version: String,
}

/// DoorwayRegistration entry fields (mirrors integrity zome).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DoorwayRegistration {
    pub id: String,
    pub url: String,
    pub operator_agent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_human: Option<String>,
    pub capabilities_json: String,
    pub reach: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandwidth_mbps: Option<u32>,
    pub version: String,
    pub tier: String,
    pub registered_at: String,
    pub updated_at: String,
}

/// Output from doorway registration operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DoorwayOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub doorway: DoorwayRegistration,
}

// =============================================================================
// Heartbeat & Health Types
// =============================================================================

/// Input for infrastructure::record_heartbeat.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RecordHeartbeatInput {
    pub doorway_id: String,
    pub status: String,
    pub uptime_ratio: f32,
    pub active_connections: u32,
    pub content_served: u64,
}

/// Input for infrastructure::record_daily_summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RecordSummaryInput {
    pub doorway_id: String,
    pub date: String,
    pub uptime_ratio: f32,
    pub total_content_served: u64,
    pub peak_connections: u32,
    pub heartbeat_count: u32,
}

/// Input for infrastructure::record_health_attestation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RecordHealthAttestationInput {
    pub attestor_doorway_id: String,
    pub subject_doorway_id: String,
    pub observed_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_time_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conductor_healthy: Option<bool>,
}

/// HealthAttestation entry fields (mirrors integrity zome).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HealthAttestation {
    pub attestor_doorway_id: String,
    pub operator_agent: String,
    pub subject_doorway_id: String,
    pub observed_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_time_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conductor_healthy: Option<bool>,
    pub timestamp: i64,
}

/// Output from health attestation queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HealthAttestationOutput {
    pub action_hash: String,
    pub entry_hash: String,
    pub attestation: HealthAttestation,
    pub author: String,
}

// =============================================================================
// Content Server Types
// =============================================================================

/// Input for infrastructure::register_content_server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RegisterContentServerInput {
    pub content_hash: String,
    pub capability: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serve_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<StorageEndpointInput>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandwidth_mbps: Option<u32>,
}

/// Storage endpoint input (used in RegisterContentServerInput).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StorageEndpointInput {
    pub url: String,
    pub protocol: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
}

/// Storage endpoint entry fields (mirrors integrity zome).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StorageEndpoint {
    pub url: String,
    pub protocol: String,
    pub priority: u8,
}

/// ContentServer entry fields (mirrors integrity zome).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ContentServer {
    pub content_hash: String,
    pub capability: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serve_url: Option<String>,
    pub endpoints: Vec<StorageEndpoint>,
    pub online: bool,
    pub priority: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandwidth_mbps: Option<u32>,
    pub registered_at: u64,
    pub last_heartbeat: u64,
}

/// Output from content server operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ContentServerOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub server: ContentServer,
}

/// Input for infrastructure::find_publishers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FindPublishersInput {
    pub content_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefer_region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub online_only: Option<bool>,
}

/// Output from infrastructure::find_publishers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FindPublishersOutput {
    pub content_hash: String,
    pub publishers: Vec<ContentServerOutput>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: serialize as named map (matches Holochain's MessagePack convention).
    fn roundtrip<T: serde::Serialize + serde::de::DeserializeOwned>(val: &T) -> T {
        let bytes = rmp_serde::to_vec_named(val).unwrap();
        rmp_serde::from_slice(&bytes).unwrap()
    }

    #[test]
    fn register_doorway_input_msgpack_roundtrip() {
        let input = RegisterDoorwayInput {
            id: "doorway-1".to_string(),
            url: "https://doorway.example.com".to_string(),
            capabilities_json: r#"{"bootstrap":true}"#.to_string(),
            reach: "commons".to_string(),
            region: Some("us-east".to_string()),
            bandwidth_mbps: Some(100),
            version: "1.0.0".to_string(),
        };
        let decoded = roundtrip(&input);
        assert_eq!(decoded.id, "doorway-1");
        assert_eq!(decoded.region, Some("us-east".to_string()));
    }

    #[test]
    fn record_heartbeat_input_msgpack_roundtrip() {
        let input = RecordHeartbeatInput {
            doorway_id: "doorway-1".to_string(),
            status: "healthy".to_string(),
            uptime_ratio: 0.99,
            active_connections: 42,
            content_served: 1024,
        };
        let decoded = roundtrip(&input);
        assert_eq!(decoded.doorway_id, "doorway-1");
        assert_eq!(decoded.active_connections, 42);
    }

    #[test]
    fn record_health_attestation_input_msgpack_roundtrip() {
        let input = RecordHealthAttestationInput {
            attestor_doorway_id: "doorway-1".to_string(),
            subject_doorway_id: "doorway-2".to_string(),
            observed_status: "healthy".to_string(),
            response_time_ms: Some(45),
            conductor_healthy: Some(true),
        };
        let decoded = roundtrip(&input);
        assert_eq!(decoded.attestor_doorway_id, "doorway-1");
        assert_eq!(decoded.response_time_ms, Some(45));
    }

    #[test]
    fn find_publishers_input_msgpack_roundtrip() {
        let input = FindPublishersInput {
            content_hash: "bafkrei123".to_string(),
            capability: Some("serve".to_string()),
            prefer_region: None,
            limit: Some(10),
            online_only: Some(true),
        };
        let decoded = roundtrip(&input);
        assert_eq!(decoded.content_hash, "bafkrei123");
        assert_eq!(decoded.limit, Some(10));
    }
}

#[cfg(test)]
#[cfg(feature = "ts")]
mod ts_export {
    use super::*;
    use ts_rs::TS;

    #[test]
    fn export_bindings() {
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bindings");
        RegisterDoorwayInput::export_all_to(&out).unwrap();
        DoorwayRegistration::export_all_to(&out).unwrap();
        DoorwayOutput::export_all_to(&out).unwrap();
        RecordHeartbeatInput::export_all_to(&out).unwrap();
        RecordSummaryInput::export_all_to(&out).unwrap();
        RecordHealthAttestationInput::export_all_to(&out).unwrap();
        HealthAttestation::export_all_to(&out).unwrap();
        HealthAttestationOutput::export_all_to(&out).unwrap();
        RegisterContentServerInput::export_all_to(&out).unwrap();
        StorageEndpointInput::export_all_to(&out).unwrap();
        StorageEndpoint::export_all_to(&out).unwrap();
        ContentServer::export_all_to(&out).unwrap();
        ContentServerOutput::export_all_to(&out).unwrap();
        FindPublishersInput::export_all_to(&out).unwrap();
        FindPublishersOutput::export_all_to(&out).unwrap();
    }
}
