//! Shared compute reporting for the Elohim fleet.
//!
//! Defines uniform types and traits for health, resource usage,
//! request throughput, and peer health — consumed by every service's
//! `/status` endpoint and the operator elohim agent.

pub mod build_info;
pub mod counters;
pub mod detail_level;
pub mod health;
pub mod peers;
pub mod report;
pub mod resources;

pub use build_info::BuildInfo;
pub use counters::{RequestCounterSnapshot, RequestCounters};
pub use detail_level::DetailLevel;
pub use health::{HealthReporter, ServiceHealth};
pub use peers::{PeerHealthRegistry, PeerHealthSnapshot};
pub use report::ComputeReport;
pub use resources::{ResourceReporter, ResourceSnapshot};
