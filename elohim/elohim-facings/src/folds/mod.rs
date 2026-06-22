//! Per-facing fold modules. Each facing materializes the relation it needs and
//! folds it into its lenses here. `resiliency` is the reference implementation
//! migrated by the §11 slice; the four child lens charters add siblings
//! (`reach_projection`, `rea_economic`, `operational_weave`,
//! `epr_content_perspective`) over the same `crate::fold` combinators.

pub mod contributor_reflexive;
pub mod epr_content;
pub mod operational_weave;
pub mod rea;
pub mod resiliency;
