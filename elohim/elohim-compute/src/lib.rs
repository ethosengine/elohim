//! Shared compute reporting for the Elohim fleet.
//!
//! Defines uniform types and traits for health, resource usage,
//! request throughput, and peer health — consumed by every service's
//! `/status` endpoint and the operator elohim agent.
//!
//! The reporting modules sit behind the default-on `runtime` feature. With
//! `default-features = false` the crate is the actuation contract alone
//! ([`Governor`], [`Refusal`], [`LimitOwner`]) plus the serde-only
//! [`BuildInfo`] and [`DetailLevel`]: no clock, no concurrent map, no lock.

pub mod actuation;
pub mod build_info;
#[cfg(feature = "runtime")]
pub mod counters;
pub mod detail_level;
#[cfg(feature = "runtime")]
pub mod health;
#[cfg(feature = "runtime")]
pub mod peers;
#[cfg(feature = "runtime")]
pub mod report;
#[cfg(feature = "runtime")]
pub mod resources;

pub use actuation::{Governor, LimitOwner, Refusal, RefusalCode};
pub use build_info::BuildInfo;
#[cfg(feature = "runtime")]
pub use counters::{RequestCounterSnapshot, RequestCounters};
pub use detail_level::DetailLevel;
#[cfg(feature = "runtime")]
pub use health::{HealthReporter, ServiceHealth};
#[cfg(feature = "runtime")]
pub use peers::{CircuitBreaker, CircuitState, PeerHealthRegistry, PeerHealthSnapshot};
#[cfg(feature = "runtime")]
pub use report::ComputeReport;
#[cfg(feature = "runtime")]
pub use resources::{ResourceReporter, ResourceSnapshot};
