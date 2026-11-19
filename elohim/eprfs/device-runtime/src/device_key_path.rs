//! Where this device's key lives: the one resolution every device-scoped record beside the key
//! (roster pins, checkout-trust declarations, session envelopes) is located from.
//!
//! The key itself — loading, minting and its directory checks — stays with `epr`'s
//! `device_key` module. Only the path is here, so a process that never touches the key (ark,
//! reading which checkouts this device declared trusted) finds the same directory `epr` writes.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

/// Test-and-tooling override for the key's path. Its only intended uses are tests and the
/// two-device enrolment rehearsal; an ordinary session resolves the config-home path.
pub const DEVICE_KEY_ENV: &str = "ELOHIM_DEVICE_KEY_FILE";

/// Resolve the seed path from the process environment. See [`resolve_path_with`].
pub fn resolve_path() -> io::Result<PathBuf> {
    resolve_path_with(|name| std::env::var_os(name))
}

/// Resolve the seed path from an environment lookup, in order:
///
/// 1. `ELOHIM_DEVICE_KEY_FILE`, when set and non-empty — it must be ABSOLUTE and lie outside
///    every git repository (see [`check_override_path`]); anything else is refused, never used.
/// 2. `$XDG_CONFIG_HOME/elohim/device/ed25519.seed`, when `XDG_CONFIG_HOME` is set to an
///    ABSOLUTE path (the XDG base-directory spec says a relative value is invalid and ignored).
/// 3. `$HOME/.config/elohim/device/ed25519.seed`.
///
/// No home at all is an error, never a fallback into the working directory: a key written into a
/// checkout is exactly the per-clone identity this module exists to avoid.
///
/// Takes the lookup as a function so the precedence is testable without mutating the process
/// environment, which parallel tests share.
pub fn resolve_path_with<F>(lookup: F) -> io::Result<PathBuf>
where
    F: Fn(&str) -> Option<OsString>,
{
    let non_empty = |name: &str| lookup(name).filter(|value| !value.is_empty());

    if let Some(explicit) = non_empty(DEVICE_KEY_ENV) {
        let explicit = PathBuf::from(explicit);
        check_override_path(&explicit)?;
        return Ok(explicit);
    }
    let config_home = match non_empty("XDG_CONFIG_HOME").map(PathBuf::from) {
        Some(xdg) if xdg.is_absolute() => xdg,
        _ => match non_empty("HOME") {
            Some(home) => PathBuf::from(home).join(".config"),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "no device key path: {DEVICE_KEY_ENV}, XDG_CONFIG_HOME and HOME are all \
                         unset — the device key lives in the OS user's config home, never in a \
                         checkout"
                    ),
                ))
            }
        },
    };
    Ok(config_home
        .join("elohim")
        .join("device")
        .join("ed25519.seed"))
}

/// Refuse an `ELOHIM_DEVICE_KEY_FILE` that is relative, or that lies inside a git repository.
///
/// The repository test walks up from the path's parent looking for a `.git` entry (a directory in
/// a clone, a file in a worktree or submodule); the first one found refuses.
pub fn check_override_path(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{DEVICE_KEY_ENV}=`{}` is relative — a relative key path names a different key in \
                 every checkout; give an absolute path outside any repository",
                path.display()
            ),
        ));
    }
    let mut dir = path.parent();
    while let Some(current) = dir {
        if current.join(".git").exists() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "{DEVICE_KEY_ENV}=`{}` lies inside the git repository at `{}` — a seed in a \
                     repository is one commit from being published; keep it outside every \
                     checkout",
                    path.display(),
                    current.display()
                ),
            ));
        }
        dir = current.parent();
    }
    Ok(())
}
