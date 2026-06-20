//! elohim-peer-fabric — the write-once shared peer-traffic spine.
//!
//! Two pure-logic modules consumed by BOTH `doorway-service` and `elohim-storage`,
//! feature-gated per node role (default features compile both — dev/CI run all tests; slim a
//! build with `default-features = false` + only the role feature(s) you drive):
//!   - [`guard`]: fail2ban-style admission / ban / rate-shape / challenge — `edge-defense`/`peer-defense`.
//!   - [`score`]: capability-aware peer ranking with graceful degradation — `serve-routing`/`identity-routing`.
//!
//! The crate has NO I/O: all state/time/data access is behind traits the runtimes implement.
//! The absence of `diesel` from `Cargo.toml` IS the purity boundary — impure code won't compile here.
//! See genesis/docs/superpowers/specs/2026-06-20-doorway-membrane-prosocial-routing-design.md.

#[cfg(any(feature = "edge-defense", feature = "peer-defense"))]
pub mod guard;
#[cfg(any(feature = "serve-routing", feature = "identity-routing"))]
pub mod score;
