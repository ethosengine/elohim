//! Peer policy engine.
//!
//! Operator-declared policy (`peer-policy.toml`) + live runtime state →
//! `PeerCapabilityFlags` that the heartbeat task projects into Mishpat.
//!
//! - [`config`] — TOML-loaded configuration types.
//! - `evaluator` (Task 11) — derives capability flags from config + live state.

pub mod config;

pub use config::{AutoOrBool, NetworkConfig, PolicyConfig, PoolConfig, StewardshipConfig};
