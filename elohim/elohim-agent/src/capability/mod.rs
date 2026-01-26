//! Capability-based invocation system.
//!
//! Elohim agents provide specific capabilities that can be invoked.
//! This module defines those capabilities and the registry for them.

pub mod registry;
pub mod types;

pub use registry::CapabilityRegistry;
pub use types::ElohimCapability;
