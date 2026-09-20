//! Configuration for elohim-storage — a re-export shim over `elohim-settings`.
//!
//! The boot `Config`, its `OnceLock` mirrors and its `set_*` publishers moved
//! to `elohim_settings::config` in step 3 of the storage decomposition. This
//! path stays alive so `crate::config::X` and `crate::Config` resolve unchanged
//! at every call site above.
//!
//! MODULE-SCOPED, not flattened: `src/runtime_config.rs` is a sibling shim over
//! `elohim_settings::runtime_config`, and a single glob would collide `Key`,
//! `Kind` and `config_path` between the two.
//!
//! Design: `genesis/docs/superpowers/specs/2026-09-19-storage-crate-decomposition-design.md`
//! §4 row 3, §8a.

pub use elohim_settings::config::*;

#[cfg(test)]
mod tests {
    /// Every boot publisher `elohim-settings` declares must be called from the
    /// boot sequence in THIS crate's `main.rs`.
    ///
    /// A publisher nobody calls is not a compile error: the key is registered,
    /// it is read live, and it silently rides its default while the operator's
    /// env var is ignored. That shipped on 2026-09-19 for
    /// `REANCHOR_HELD_BACKOFF_SECONDS` and `CONTEST_REMINT_WINDOW_SECONDS` —
    /// setters with no call sites, through review.
    ///
    /// The check lives HERE, and stays here, because `main.rs` is here. It
    /// reads `config::BOOT_PUBLISHERS` — a DECLARED registry — rather than
    /// `include_str!`ing the settings crate's source: the storage Dockerfile
    /// flattens sibling crates into `/app` and rewrites only Cargo paths, so a
    /// relative cross-crate source include resolves in the dev tree and breaks
    /// in the image. The settings crate holds the other half, asserting that
    /// registry against its own `config.rs`.
    ///
    /// This is the static half of the boot assertion. The runtime half —
    /// "and the call actually landed on the key" — is
    /// `runtime_config::assert_boot_published()`, called from `main` just
    /// before the watcher starts.
    #[test]
    fn every_boot_publisher_is_called_from_main() {
        let main_src = include_str!("main.rs");
        let orphans: Vec<&str> = elohim_settings::config::BOOT_PUBLISHERS
            .iter()
            .copied()
            .filter(|name| !main_src.contains(&format!("config::set_{name}")))
            .collect();
        assert!(
            orphans.is_empty(),
            "config publishers with no call site in main.rs (their env value is never published): {}",
            orphans
                .iter()
                .map(|n| format!("set_{n}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}
