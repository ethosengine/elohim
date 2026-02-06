//! Elohim Autonomous Entity (EAE) - The Autonomous Agent Framework
//!
//! Implements the MACE pattern (Monitor-Analyze-Decide-Execute) for
//! autonomous constitutional agents:
//!
//! - **Layer-aware governance**: Global to individual layers
//! - **Anomaly detection**: Spiral, manipulation, constitutional drift
//! - **Precedent tracking**: Learn from past decisions
//! - **Cross-agent consensus**: Distributed agreement for risky actions
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │               ElohimAutonomousEntity                        │
//! │                                                             │
//! │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐       │
//! │  │ Monitor │──│ Analyze │──│ Decide  │──│ Execute │       │
//! │  └─────────┘  └─────────┘  └─────────┘  └─────────┘       │
//! │                                  │                          │
//! │                          ┌───────▼───────┐                 │
//! │                          │   Consensus   │                 │
//! │                          └───────────────┘                 │
//! └─────────────────────────────────────────────────────────────┘
//! ```

pub mod anomaly;
pub mod config;
pub mod entity;
pub mod governance;
pub mod mace;
pub mod precedent;
pub mod types;

// Re-export main types
pub use config::EaeConfig;
pub use entity::ElohimAutonomousEntity;
pub use types::*;
