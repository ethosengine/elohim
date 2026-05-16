//! Graph-native view builders — lamad + shefa domains.
//!
//! These builders compose CozoDB graph queries with the existing view struct shapes.
//! They are the single composition layer between the graph projection engine and the
//! REST + GraphQL wire surfaces.
//!
//! All builders are gated on the `graph-native` Cargo feature (default-on).
//! Thin-client builds (`--no-default-features`) exclude this module entirely.

pub mod data_value;
pub mod lamad;
pub mod shefa;
