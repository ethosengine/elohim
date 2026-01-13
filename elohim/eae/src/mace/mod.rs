//! MACE - Monitor, Analyze, Decide, Execute pattern.
//!
//! The MACE pattern provides a structured approach to autonomous decision-making:
//!
//! 1. **Monitor**: Collects observations from various sources
//! 2. **Analyzer**: Detects patterns and anomalies in observations
//! 3. **Decider**: Makes decisions using rules and LLM fallback
//! 4. **Executor**: Executes actions resulting from decisions
//! 5. **Consensus**: Gathers consensus for risky decisions

mod analyzer;
mod consensus;
mod decider;
mod executor;
mod monitor;

pub use analyzer::{Analyzer, AnalyzerBuilder, Pattern, PatternMatch};
pub use consensus::{ConsensusManager, ConsensusRequest};
pub use decider::{Decider, DeciderBuilder, Rule, RuleEngine};
pub use executor::{ActionHandler, Executor, ExecutorBuilder};
pub use monitor::{Monitor, MonitorBuilder};

/// The complete MACE pipeline.
pub struct MacePipeline {
    /// Monitor component
    pub monitor: Monitor,
    /// Analyzer component
    pub analyzer: Analyzer,
    /// Decider component
    pub decider: Decider,
    /// Executor component
    pub executor: Executor,
    /// Consensus manager
    pub consensus: ConsensusManager,
}

impl MacePipeline {
    /// Create a new MACE pipeline with default configuration.
    pub fn new() -> Self {
        Self {
            monitor: Monitor::new(),
            analyzer: Analyzer::new(),
            decider: Decider::new(),
            executor: Executor::new(),
            consensus: ConsensusManager::new(),
        }
    }

    /// Create with custom components.
    pub fn with_components(
        monitor: Monitor,
        analyzer: Analyzer,
        decider: Decider,
        executor: Executor,
        consensus: ConsensusManager,
    ) -> Self {
        Self {
            monitor,
            analyzer,
            decider,
            executor,
            consensus,
        }
    }
}

impl Default for MacePipeline {
    fn default() -> Self {
        Self::new()
    }
}
