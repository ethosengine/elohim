//! Runtime configuration — a re-export shim over `elohim-settings`.
//!
//! The watched-file registry (upgrade-velocity rung 4) moved to
//! `elohim_settings::runtime_config` in step 3 of the storage decomposition,
//! together with `config` — the two are mutually recursive and could not be
//! separated. This path stays alive so `crate::runtime_config::X` resolves
//! unchanged at every call site above.
//!
//! MODULE-SCOPED, not flattened: `src/config.rs` is a sibling shim over
//! `elohim_settings::config`, and a single glob would collide `Key`, `Kind` and
//! `config_path` between the two.
//!
//! What did NOT move is the watcher's CLOCK. `begin_watch()` and
//! `poll_once(path)` are synchronous `std`; the `tokio::spawn` +
//! `tokio::time::interval` that drives them lives in
//! [`crate::runtime_config_watch`], because a settings crate that pulled an
//! async runtime would colour every crate above it with that choice.
//!
//! Design: `genesis/docs/superpowers/specs/2026-09-19-storage-crate-decomposition-design.md`
//! §4 row 3, §8a ("The file watcher stays in storage").

pub use elohim_settings::runtime_config::*;
