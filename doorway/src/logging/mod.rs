//! Logging infrastructure for Doorway
//!
//! Provides structured logging compatible with Unyt's billing system.

pub mod usage;

pub use usage::{UsageEvent, UsageLogger};
