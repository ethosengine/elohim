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
