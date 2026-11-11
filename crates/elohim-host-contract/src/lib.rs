//! Pure host compatibility, independent of observation, transport and execution.
//!
//! Requirements are attributes of artifacts, not new notarized heads. Host facts
//! are ephemeral: lane A2 rebuilds them by observation. This crate performs no I/O.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod admission;
mod bundle;
mod requirement;
mod vocabulary;

pub use admission::{admit, Provides, Refusal};
pub use bundle::{Bundle, BundleSpec, X86_64_V2, X86_64_V3};
pub use requirement::{Clause, ClauseSource, Disagreement, MergeError, MergedClause, Requires};
pub use vocabulary::{AxisId, AxisValue, ComparisonKind, Predicate, Version};
