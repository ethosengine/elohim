//! Resource usage snapshot and reporting trait.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::RequestCounterSnapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSnapshot {
    pub timestamp: DateTime<Utc>,
    pub requests: RequestCounterSnapshot,
    pub active_connections: usize,
    pub managed_storage_bytes: u64,
    pub managed_document_count: u64,
}

pub trait ResourceReporter: Send + Sync {
    fn resource_snapshot(&self) -> ResourceSnapshot;
    fn extension_snapshot(&self) -> serde_json::Value;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RequestCounterSnapshot;
    use std::collections::HashMap;

    #[test]
    fn test_resource_snapshot_serializes_camel_case() {
        let snap = ResourceSnapshot {
            timestamp: Utc::now(),
            requests: RequestCounterSnapshot {
                total: 42,
                by_category: HashMap::new(),
            },
            active_connections: 3,
            managed_storage_bytes: 1024 * 1024,
            managed_document_count: 100,
        };
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("\"activeConnections\""));
        assert!(json.contains("\"managedStorageBytes\""));
        assert!(json.contains("\"managedDocumentCount\""));
    }

    #[test]
    fn test_resource_snapshot_roundtrip() {
        let snap = ResourceSnapshot {
            timestamp: Utc::now(),
            requests: RequestCounterSnapshot {
                total: 10,
                by_category: HashMap::from([("Content".to_string(), 10)]),
            },
            active_connections: 2,
            managed_storage_bytes: 500,
            managed_document_count: 5,
        };
        let json = serde_json::to_string(&snap).unwrap();
        let deserialized: ResourceSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.requests.total, 10);
        assert_eq!(deserialized.active_connections, 2);
        assert_eq!(deserialized.managed_storage_bytes, 500);
    }
}
