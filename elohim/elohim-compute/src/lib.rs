//! Shared compute reporting for the Elohim fleet.
//!
//! Defines uniform types and traits for health, resource usage,
//! request throughput, and peer health — consumed by every service's
//! `/status` endpoint and the operator elohim agent.

pub mod counters;
pub mod health;
pub mod peers;
pub mod report;
pub mod resources;

pub use counters::{RequestCounterSnapshot, RequestCounters};
pub use health::{HealthReporter, ServiceHealth};
pub use peers::{PeerHealthRegistry, PeerHealthSnapshot};
pub use report::ComputeReport;
pub use resources::{ResourceReporter, ResourceSnapshot};
