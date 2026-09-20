//! The tokio half of the runtime-config watcher — and nothing else.
//!
//! Everything the watcher *decides* — the registry, the parse, the apply, the
//! provenance, the logging, the signature comparison — lives in
//! [`crate::runtime_config`] (the `elohim-settings` crate), which is
//! deliberately std-only. What could not follow it down is the CLOCK: a
//! `tokio::spawn` and a `tokio::time::interval`.
//!
//! So the clock stays here. A settings layer that pulled an async runtime would
//! colour every crate above it with that choice for the sake of one poll loop,
//! and the settings crate's lockfile boundary test denies `tokio` outright.
//! This module is the whole of that seam: one call site (`main.rs`), one task.
//!
//! Design: `genesis/docs/superpowers/specs/2026-09-19-storage-crate-decomposition-design.md`
//! §8a ("The file watcher stays in storage").

use std::time::Duration;

use crate::runtime_config;

/// Spawn the config watcher. Returns whether it is active.
///
/// Inactive (and one INFO line, emitted by
/// [`runtime_config::begin_watch`]) when `ELOHIM_RUNTIME_CONFIG_PATH` is unset —
/// the default, so a node that never mounts a ConfigMap pays nothing and
/// behaves exactly as it did before the watcher existed.
pub fn spawn_watcher() -> bool {
    let Some(path) = runtime_config::begin_watch() else {
        return false;
    };

    tokio::spawn(async move {
        // bounded-work: one `stat(2)` per POLL_INTERVAL_SECS tick, and a read +
        // parse of a single operator-authored file ONLY when its (mtime, len)
        // signature changed. No retry ladder, no queue, no fan-out: the budget
        // is the fixed cadence itself, and `MissedTickBehavior::Skip` means a
        // stalled runtime coalesces missed ticks instead of catching up in a
        // burst. There is nothing here to pace — this is the poll, not a drain.
        let mut ticker =
            tokio::time::interval(Duration::from_secs(runtime_config::POLL_INTERVAL_SECS));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            runtime_config::poll_once(&path);
        }
    });

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With no path configured the watcher does not spawn a task at all — the
    /// default for every node that never mounts a ConfigMap.
    #[tokio::test]
    async fn an_unconfigured_watcher_never_spawns() {
        if runtime_config::config_path().is_none() {
            assert!(!spawn_watcher());
        }
    }
}
