//! gate-types — Pure data types for the Elohim Protocol gate interface.
//!
//! This crate contains only data type definitions: enums, structs, serde
//! attributes, and `thiserror`-based error types. It has no async runtime,
//! no network dependencies, and no elohim-agent dependency. It compiles for
//! `wasm32-unknown-unknown` without RUSTFLAGS overrides.
//!
//! # Consumers
//!
//! - **gate-client**: imports all types from here, re-exports them at the
//!   crate surface for backward compatibility with existing callers.
//! - **gate-client-zome**: imports types from here to build a pure-sync
//!   Allow mock for Holochain WASM zome coordinators.
//! - **Holochain zomes**: depend on `gate-client-zome` (not gate-client)
//!   because gate-client pulls in tokio/reqwest which do not compile in WASM.

pub mod error;
pub mod events;
pub mod phase;
pub mod space;
pub mod types;

// Flat re-exports for ergonomic use by gate-client-zome and zome callers.
pub use error::{GateError, GateResult};
pub use events::RelationalImpactEvent;
pub use phase::Phase;
pub use space::{SpaceContext, SpaceType};
pub use types::{
    ConstitutionalReasoningSummary, DeclineGrounds, EscalationTarget, GateDecision, GateStatus,
    GateTag, Severity, SideEffect,
};
