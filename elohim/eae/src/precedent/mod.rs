//! Precedent tracking and retrieval.
//!
//! Learns from past decisions to improve future decision-making.

pub mod tracker;

pub use tracker::{Precedent, PrecedentTracker, PrecedentMatch, PrecedentStats};

use crate::types::DecisionType;

/// Summary of precedent for a decision type.
#[derive(Debug, Clone)]
pub struct PrecedentSummary {
    /// Number of similar past decisions
    pub count: usize,
    /// Most common outcome
    pub common_outcome: DecisionType,
    /// Average confidence
    pub avg_confidence: f32,
    /// Most relevant precedent IDs
    pub top_precedents: Vec<String>,
}
