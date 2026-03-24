//! Service health state machine and reporting trait.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Three-state health vocabulary for any service in the fleet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceHealth {
    Healthy,
    Degraded,
    #[default]
    Offline,
}

impl fmt::Display for ServiceHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Offline => write!(f, "offline"),
        }
    }
}

/// Trait for services to report their health.
pub trait HealthReporter: Send + Sync {
    fn service_id(&self) -> &str;
    fn health(&self) -> ServiceHealth;
    fn health_reason(&self) -> String;
    fn started_at(&self) -> DateTime<Utc>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_health_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&ServiceHealth::Healthy).unwrap(),
            "\"healthy\""
        );
        assert_eq!(
            serde_json::to_string(&ServiceHealth::Degraded).unwrap(),
            "\"degraded\""
        );
        assert_eq!(
            serde_json::to_string(&ServiceHealth::Offline).unwrap(),
            "\"offline\""
        );
    }

    #[test]
    fn test_service_health_deserializes() {
        let h: ServiceHealth = serde_json::from_str("\"healthy\"").unwrap();
        assert_eq!(h, ServiceHealth::Healthy);
        let d: ServiceHealth = serde_json::from_str("\"degraded\"").unwrap();
        assert_eq!(d, ServiceHealth::Degraded);
    }

    #[test]
    fn test_service_health_default_is_offline() {
        assert_eq!(ServiceHealth::default(), ServiceHealth::Offline);
    }

    #[test]
    fn test_service_health_display() {
        assert_eq!(format!("{}", ServiceHealth::Healthy), "healthy");
        assert_eq!(format!("{}", ServiceHealth::Degraded), "degraded");
        assert_eq!(format!("{}", ServiceHealth::Offline), "offline");
    }
}
