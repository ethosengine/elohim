//! ComputeReport — the top-level envelope every service returns.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::build_info::BuildInfo;
use crate::detail_level::DetailLevel;
use crate::{HealthReporter, PeerHealthSnapshot, ResourceSnapshot, ServiceHealth};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeReport {
    pub service_id: String,
    pub build: BuildInfo,
    pub health: ServiceHealth,
    pub health_reason: String,
    pub started_at: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub resources: ResourceSnapshot,
    pub peers: Vec<PeerHealthSnapshot>,
    pub extensions: serde_json::Value,
}

impl ComputeReport {
    pub fn build(
        reporter: &dyn HealthReporter,
        build_info: &BuildInfo,
        resources: ResourceSnapshot,
        peers: Vec<PeerHealthSnapshot>,
        extensions: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        let started = reporter.started_at();
        let uptime = (now - started).num_seconds().max(0) as u64;
        Self {
            service_id: reporter.service_id().to_string(),
            build: build_info.clone(),
            health: reporter.health(),
            health_reason: reporter.health_reason(),
            started_at: started,
            uptime_seconds: uptime,
            resources,
            peers,
            extensions,
        }
    }

    /// Serialize to JSON and strip fields above the requested detail level.
    ///
    /// - Below `Trace`: remove `extensions`
    /// - Below `Debug`: also remove `resources` and `peers`
    pub fn filter(&self, level: DetailLevel) -> serde_json::Value {
        let mut value = serde_json::to_value(self).expect("ComputeReport always serializes");
        if let Some(obj) = value.as_object_mut() {
            if level < DetailLevel::Trace {
                obj.remove("extensions");
            }
            if level < DetailLevel::Debug {
                obj.remove("resources");
                obj.remove("peers");
            }
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        HealthReporter, PeerHealthSnapshot, RequestCounterSnapshot, ResourceSnapshot, ServiceHealth,
    };
    use chrono::Utc;
    use std::collections::HashMap;

    struct MockReporter;

    impl HealthReporter for MockReporter {
        fn service_id(&self) -> &str {
            "test-service"
        }
        fn health(&self) -> ServiceHealth {
            ServiceHealth::Healthy
        }
        fn health_reason(&self) -> String {
            "all systems go".to_string()
        }
        fn started_at(&self) -> chrono::DateTime<Utc> {
            Utc::now() - chrono::Duration::seconds(120)
        }
    }

    fn test_build_info() -> BuildInfo {
        BuildInfo::new("test-service")
    }

    #[test]
    fn test_build_from_reporter() {
        let reporter = MockReporter;
        let build_info = test_build_info();
        let resources = ResourceSnapshot {
            timestamp: Utc::now(),
            requests: RequestCounterSnapshot {
                total: 42,
                by_category: HashMap::from([("Content".to_string(), 42)]),
            },
            active_connections: 3,
            managed_storage_bytes: 1024,
            managed_document_count: 10,
        };
        let peers = vec![PeerHealthSnapshot {
            peer_id: "conductor-0".to_string(),
            address: "ws://host:8445".to_string(),
            health: ServiceHealth::Healthy,
            reason: "connected".to_string(),
            signals_received: 100,
            last_signal_at: Some(Utc::now()),
            reconnect_attempts: 0,
        }];
        let extensions = serde_json::json!({ "hotCacheEntries": 500 });

        let report = ComputeReport::build(&reporter, &build_info, resources, peers, extensions);

        assert_eq!(report.service_id, "test-service");
        assert_eq!(report.build.service, "test-service");
        assert_eq!(report.health, ServiceHealth::Healthy);
        assert_eq!(report.health_reason, "all systems go");
        assert!(report.uptime_seconds >= 119);
        assert_eq!(report.resources.requests.total, 42);
        assert_eq!(report.peers.len(), 1);
        assert_eq!(report.extensions["hotCacheEntries"], 500);
    }

    #[test]
    fn test_report_serializes_camel_case() {
        let reporter = MockReporter;
        let build_info = test_build_info();
        let resources = ResourceSnapshot {
            timestamp: Utc::now(),
            requests: RequestCounterSnapshot {
                total: 0,
                by_category: HashMap::new(),
            },
            active_connections: 0,
            managed_storage_bytes: 0,
            managed_document_count: 0,
        };
        let report = ComputeReport::build(
            &reporter,
            &build_info,
            resources,
            vec![],
            serde_json::Value::Null,
        );
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"serviceId\""));
        assert!(json.contains("\"healthReason\""));
        assert!(json.contains("\"startedAt\""));
        assert!(json.contains("\"uptimeSeconds\""));
    }

    #[test]
    fn test_report_roundtrip() {
        let reporter = MockReporter;
        let build_info = test_build_info();
        let resources = ResourceSnapshot {
            timestamp: Utc::now(),
            requests: RequestCounterSnapshot {
                total: 5,
                by_category: HashMap::new(),
            },
            active_connections: 1,
            managed_storage_bytes: 2048,
            managed_document_count: 3,
        };
        let report = ComputeReport::build(
            &reporter,
            &build_info,
            resources,
            vec![],
            serde_json::json!({}),
        );
        let json = serde_json::to_string(&report).unwrap();
        let deserialized: ComputeReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.service_id, "test-service");
        assert_eq!(deserialized.build.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(deserialized.health, ServiceHealth::Healthy);
    }

    #[test]
    fn test_filter_trace_includes_everything() {
        let reporter = MockReporter;
        let build_info = test_build_info();
        let resources = ResourceSnapshot {
            timestamp: Utc::now(),
            requests: RequestCounterSnapshot {
                total: 1,
                by_category: HashMap::new(),
            },
            active_connections: 0,
            managed_storage_bytes: 0,
            managed_document_count: 0,
        };
        let report = ComputeReport::build(
            &reporter,
            &build_info,
            resources,
            vec![],
            serde_json::json!({ "extra": true }),
        );
        let filtered = report.filter(DetailLevel::Trace);
        let obj = filtered.as_object().unwrap();
        assert!(obj.contains_key("extensions"));
        assert!(obj.contains_key("resources"));
        assert!(obj.contains_key("peers"));
    }

    #[test]
    fn test_filter_debug_removes_extensions() {
        let reporter = MockReporter;
        let build_info = test_build_info();
        let resources = ResourceSnapshot {
            timestamp: Utc::now(),
            requests: RequestCounterSnapshot {
                total: 1,
                by_category: HashMap::new(),
            },
            active_connections: 0,
            managed_storage_bytes: 0,
            managed_document_count: 0,
        };
        let report = ComputeReport::build(
            &reporter,
            &build_info,
            resources,
            vec![],
            serde_json::json!({ "extra": true }),
        );
        let filtered = report.filter(DetailLevel::Debug);
        let obj = filtered.as_object().unwrap();
        assert!(!obj.contains_key("extensions"));
        assert!(obj.contains_key("resources"));
        assert!(obj.contains_key("peers"));
    }

    #[test]
    fn test_filter_info_removes_resources_peers_extensions() {
        let reporter = MockReporter;
        let build_info = test_build_info();
        let resources = ResourceSnapshot {
            timestamp: Utc::now(),
            requests: RequestCounterSnapshot {
                total: 1,
                by_category: HashMap::new(),
            },
            active_connections: 0,
            managed_storage_bytes: 0,
            managed_document_count: 0,
        };
        let report = ComputeReport::build(
            &reporter,
            &build_info,
            resources,
            vec![],
            serde_json::json!({ "extra": true }),
        );
        let filtered = report.filter(DetailLevel::Info);
        let obj = filtered.as_object().unwrap();
        assert!(!obj.contains_key("extensions"));
        assert!(!obj.contains_key("resources"));
        assert!(!obj.contains_key("peers"));
        // Core fields still present
        assert!(obj.contains_key("serviceId"));
        assert!(obj.contains_key("build"));
        assert!(obj.contains_key("health"));
    }
}
