//! Core traits for content types
//!
//! These traits define the interface that content types must implement
//! to be used with the SDK's content client.

mod content;

pub use content::{ContentReadable, ContentWriteable, ContentQuery};
