//! Host registry for multi-operator support
//!
//! Provides host registration, heartbeat monitoring, and load balancing.

pub mod heartbeat;
pub mod registry;

pub use heartbeat::HeartbeatService;
pub use registry::HostRegistry;
