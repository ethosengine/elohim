//! Error types for the gate-client.
//!
//! All definitions live in `gate-types::error`; this module re-exports them
//! for backward compatibility with any code importing `gate_client::error::*`.

pub use gate_types::error::{GateError, GateResult};
