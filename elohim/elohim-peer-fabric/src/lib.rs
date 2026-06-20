//! elohim-peer-fabric — the write-once shared peer-traffic spine.
//!
//! Two pure-logic modules consumed by BOTH `doorway-service` and `elohim-storage`,
//! feature-gated per node role:
//!   - [`guard`]: fail2ban-style admission / ban / rate-shape / challenge (defense-in-depth).
//!   - [`score`]: capability-aware peer ranking with graceful degradation.
//!
//! The crate has NO I/O: all state/time/data access is behind traits the runtimes implement.
//! The absence of `diesel` from `Cargo.toml` IS the purity boundary — impure code won't compile here.
//! See genesis/docs/superpowers/specs/2026-06-20-doorway-membrane-prosocial-routing-design.md.

pub mod guard;
pub mod score;
