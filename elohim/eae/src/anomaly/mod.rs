//! Anomaly detection modules.
//!
//! Detects various types of anomalies:
//! - **Spiral patterns**: Escalating negative behavior loops
//! - **Manipulation attempts**: Attempts to game the system
//! - **Constitutional drift**: Gradual departure from principles

mod drift;
mod manipulation;
mod spiral;

pub use drift::{DriftDetector, DriftSignal};
pub use manipulation::{ManipulationDetector, ManipulationSignal};
pub use spiral::{SpiralDetector, SpiralSignal};

use crate::types::{Observation, Result};

/// Common trait for anomaly detectors.
#[async_trait::async_trait]
pub trait AnomalyDetector: Send + Sync {
    /// Name of the detector.
    fn name(&self) -> &str;

    /// Analyze observations and return anomaly score (0.0 - 1.0).
    async fn analyze(&self, observations: &[Observation]) -> Result<AnomalyResult>;

    /// Get detector threshold.
    fn threshold(&self) -> f32;
}

/// Result from anomaly detection.
#[derive(Debug, Clone)]
pub struct AnomalyResult {
    /// Anomaly score (0.0 - 1.0)
    pub score: f32,
    /// Whether threshold was exceeded
    pub detected: bool,
    /// Severity level if detected
    pub severity: Option<AnomalySeverity>,
    /// Signals that contributed to detection
    pub signals: Vec<String>,
    /// Suggested response
    pub suggested_response: Option<String>,
}

impl AnomalyResult {
    /// Create a result with no anomaly.
    pub fn none() -> Self {
        Self {
            score: 0.0,
            detected: false,
            severity: None,
            signals: vec![],
            suggested_response: None,
        }
    }

    /// Create a result with detected anomaly.
    pub fn detected(score: f32, severity: AnomalySeverity, signals: Vec<String>) -> Self {
        Self {
            score,
            detected: true,
            severity: Some(severity),
            signals,
            suggested_response: None,
        }
    }

    /// Add a suggested response.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggested_response = Some(suggestion.into());
        self
    }
}

/// Severity of detected anomaly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalySeverity {
    /// Low severity - worth noting
    Low,
    /// Medium severity - requires attention
    Medium,
    /// High severity - immediate action needed
    High,
    /// Critical - system protection triggered
    Critical,
}

impl AnomalySeverity {
    /// Get string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            AnomalySeverity::Low => "low",
            AnomalySeverity::Medium => "medium",
            AnomalySeverity::High => "high",
            AnomalySeverity::Critical => "critical",
        }
    }
}
