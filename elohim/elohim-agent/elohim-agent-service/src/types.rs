//! Common types for the elohim-agent crate.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Cost information for an Elohim computation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct ComputationCost {
    /// Number of input tokens processed
    pub input_tokens: u32,
    /// Number of output tokens generated
    pub output_tokens: u32,
    /// Total processing time in milliseconds
    pub processing_time_ms: u64,
    /// Number of constitutional checks performed
    pub constitutional_checks: u32,
}

/// Metadata about an Elohim agent instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct ElohimAgentInfo {
    /// Unique identifier for this agent
    pub agent_id: String,
    /// Human-readable name
    pub name: String,
    /// Constitutional layer this agent operates at
    pub layer: constitution::ConstitutionalLayer,
    /// Capabilities this agent provides
    pub capabilities: Vec<crate::capability::types::ElohimCapability>,
    /// Whether this agent is currently available
    pub available: bool,
    /// When this agent was last active
    pub last_active: Option<DateTime<Utc>>,
}

impl Default for ElohimAgentInfo {
    fn default() -> Self {
        Self {
            agent_id: uuid::Uuid::new_v4().to_string(),
            name: "Unnamed Agent".to_string(),
            layer: constitution::ConstitutionalLayer::Individual,
            capabilities: Vec::new(),
            available: false,
            last_active: None,
        }
    }
}
