//! Native Elohim development coherence and reach-readiness tooling.
//!
//! The working tree remains a local authoring space. This library produces
//! reconstructible operational evidence; only the `ready` composition treats a
//! finding as relevant to a requested reach event.

pub mod actor;
pub mod authority;
pub mod canon_lift;
pub mod check;
pub mod doctor;
pub mod error;
pub mod explain;
pub mod flow;
pub mod git;
pub mod govern;
pub mod process;
pub mod ready;
pub mod report;
pub mod repository_validators;
pub mod setup;

pub use error::{Error, Result};
pub use report::{Finding, FindingStatus, Report};
