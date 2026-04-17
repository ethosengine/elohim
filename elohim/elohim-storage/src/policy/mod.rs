//! Peer policy engine.
//!
//! Operator-declared policy (`peer-policy.toml`) + live runtime state →
//! `PeerCapabilityFlags` that the heartbeat task projects into Mishpat.
//!
//! - [`config`] — TOML-loaded configuration types.
//! - [`evaluator`] — derives capability flags from config + live state.

pub mod config;
pub mod evaluator;

pub use config::{AutoOrBool, NetworkConfig, PolicyConfig, PoolConfig, StewardshipConfig};
pub use evaluator::{evaluate, EvaluatedFlags, LiveState};
