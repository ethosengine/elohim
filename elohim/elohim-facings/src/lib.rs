//! # elohim-facings — the pure lens crate
//!
//! The select→fold→aggregate substrate for *facings*: a facing is a projection
//! that materializes a relation ONCE and folds it into the lenses it needs. The
//! resiliency facing is the reference implementation (and the only one migrated
//! by the §11 slice); the four child lens charters (reach-projection,
//! rea-economic, operational-weave, epr-content-perspective) each add their own
//! `folds/<facing>.rs` over the same substrate.
//!
//! ## The boundary is the dependency graph, not a lint
//!
//! This crate depends on `elohim-views` + `std` ONLY. It has NO `diesel`, no
//! `elohim-storage`. A fold that reaches for a `&mut SqliteConnection` (or
//! `use diesel;`) **fails to compile** — the unresolved import IS the boundary
//! (the certain mechanism). The impure loaders (`load_holder_relation`, manifest
//! and region queries) live in `elohim-storage::services::household_resilience`,
//! which calls these folds as a thin adapter.
//!
//! Design: `genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md`
//! (§3 the holder-relation, §11 the Lens Framework / migration slice).

pub mod fold;
pub mod folds;
pub mod relation;
