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

/// Who, beside this process's own uid and groups, may write a path in the device directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stranger {
    /// The world-write bit is set.
    Everyone,
    /// The group-write bit is set for a group this process does not hold.
    ForeignGroup,
}

/// The custody rule the device directory is held to, on a mode and group already read: a path is
/// refused when everyone may write it, or a group this process is not in (`own` is this
/// process's groups, see [`own_groups`]). Ownership is deliberately not part of the rule — the
/// uid owning a container's home volume shifts across restarts.
pub fn stranger_may_write(mode: u32, gid: u32, own: &[u32]) -> Option<Stranger> {
    if mode & 0o002 != 0 {
        Some(Stranger::Everyone)
    } else if mode & 0o020 != 0 && !own.contains(&gid) {
        Some(Stranger::ForeignGroup)
    } else {
        None
    }
}

/// The refusal `what` (a path already stat'ed as `meta`) earns under [`stranger_may_write`],
/// worded as `check_key_dir` words it; `None` when its custody holds.
#[cfg(unix)]
pub fn custody_refusal(
    what: &str,
    path: &Path,
    meta: &std::fs::Metadata,
    consequence: &str,
) -> Option<io::Error> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let mode = meta.permissions().mode();
    let stranger = stranger_may_write(mode, meta.gid(), &own_groups())?;
    let world = stranger == Stranger::Everyone;
    Some(io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!(
            "{what} `{}` is writable by {} (mode {:o}, group {}) — anyone who can write there \
             {consequence}; `chmod {}` it first",
            path.display(),
            if world {
                "everyone"
            } else {
                "a group this process is not in"
            },
            mode & 0o7777,
            meta.gid(),
            if world { "o-w" } else { "g-w" }
        ),
    ))
}

/// Refuse a key directory that strangers may write: whoever can write it can replace the key.
/// Only an EXISTING directory is checked — one `epr` creates is 0700 from birth.
///
/// World-writable is always refused. Group-writable is refused unless the directory's group is
/// one THIS process already holds: a container home volume mounted with a Kubernetes `fsGroup`
/// (Eclipse Che's is `2770 root:1234`) forces `g+rwx` on every directory the pod creates, and the
/// only writers that group admits are the pod's own processes. A group this process is not in is
/// a stranger. On a platform where the process's groups cannot be read, group-writable refuses.
///
/// The checkout-trust declaration beside the key is held to this same rule, directory and file
/// ([`crate::checkout_trust`]): a directory acceptable for the key is acceptable for it.
#[cfg(unix)]
pub fn check_key_dir(path: &Path) -> io::Result<()> {
    let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) else {
        return Ok(());
    };
    let meta = match std::fs::metadata(parent) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    match custody_refusal(
        "the device key directory",
        parent,
        &meta,
        "can swap the key",
    ) {
        Some(refused) => Err(refused),
        None => Ok(()),
    }
}

/// This process's real, effective and supplementary group ids, read from `/proc/self/status`;
/// empty where that file does not exist (so a group-writable key directory refuses there).
#[cfg(unix)]
pub fn own_groups() -> Vec<u32> {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return Vec::new();
    };
    status
        .lines()
        .filter_map(|line| {
            line.strip_prefix("Gid:")
                .or_else(|| line.strip_prefix("Groups:"))
        })
        .flat_map(|ids| ids.split_whitespace().filter_map(|id| id.parse().ok()))
        .collect()
}

/// No custody rule where the platform has no unix modes.
#[cfg(not(unix))]
pub fn check_key_dir(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strangers_are_everyone_or_a_group_this_process_does_not_hold() {
        assert_eq!(stranger_may_write(0o600, 7, &[]), None);
        assert_eq!(stranger_may_write(0o2770, 1234, &[0, 1234]), None);
        assert_eq!(
            stranger_may_write(0o2770, 1234, &[0]),
            Some(Stranger::ForeignGroup)
        );
        assert_eq!(
            stranger_may_write(0o777, 1234, &[1234]),
            Some(Stranger::Everyone)
        );
        assert_eq!(stranger_may_write(0o640, 9, &[]), None, "group-read only");
    }
}
