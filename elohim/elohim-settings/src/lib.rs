//! The storage node's settings layer.
//!
//! Two modules that are mutually recursive and therefore move as one unit:
//!
//! - [`config`] — the boot `Config` (env → TOML → struct), the process-wide
//!   `OnceLock` mirrors the sweeps read, and the `set_*` publishers `main`
//!   calls once the config is assembled.
//! - [`runtime_config`] — the WATCHED-FILE registry, so a flag flip applies to
//!   a RUNNING node instead of costing a pod roll. `config`'s publishers seed
//!   its boot values; its `SPECS` read `config`'s `DEFAULT_*` constants.
//!
//! They are re-exported from `elohim-storage` as `crate::config` and
//! `crate::runtime_config`, module-scoped rather than flattened — a single
//! namespace would collide `Key`, `Kind` and `config_path`.
//!
//! # What it refuses to know
//!
//! No service, no transport, no database, no HTTP route — and no async
//! runtime. The watcher's *decisions* live here and are synchronous `std`; its
//! CLOCK (`tokio::spawn` + `tokio::time::interval`) lives one layer up, in
//! `elohim_storage::runtime_config_watch`, so a settings layer cannot colour
//! every crate above it with a runtime choice. `tests/boundary.rs` holds that
//! over this crate's own lockfile.
//!
//! Design: `genesis/docs/superpowers/specs/2026-09-19-storage-crate-decomposition-design.md`
//! (§3 principles, §4 row 3 — this crate, §5 proof, §8a the cut list).

#![forbid(unsafe_code)]

pub mod config;
pub mod runtime_config;
