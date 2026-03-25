//! Typed projection collections
//!
//! Provides specialized projections for different entry types with
//! optimized indexes and query patterns.

pub mod content;

pub use content::{ContentProjection, ContentQuery};
