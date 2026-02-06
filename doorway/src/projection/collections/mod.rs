//! Typed projection collections
//!
//! Provides specialized projections for different entry types with
//! optimized indexes and query patterns.

pub mod content;
pub mod paths;

pub use content::{ContentProjection, ContentQuery};
pub use paths::{PathProjection, PathQuery};
