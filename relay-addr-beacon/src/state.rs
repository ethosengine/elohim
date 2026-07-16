//! Persisted last-published address snapshot, for change detection across
//! poll cycles and process restarts.

use std::path::Path;

use anyhow::{Context, Result};

use crate::sinks::AddrUpdate;

/// Load the last-published snapshot, or `None` if the state file does not yet
/// exist (first run).
///
/// A corrupt/unreadable state file (a serde parse error) is treated as a
/// first-run `None` rather than a hard error — otherwise a single truncated or
/// garbled write (e.g. from a crash mid-write) would wedge the daemon forever.
pub fn load(path: &Path) -> Result<Option<AddrUpdate>> {
    match std::fs::read_to_string(path) {
        Ok(contents) => match serde_json::from_str(&contents) {
            Ok(update) => Ok(Some(update)),
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "state file is corrupt/unreadable — treating as first-run (will overwrite on next successful publish)"
                );
                Ok(None)
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("reading state file {}", path.display())),
    }
}

/// Persist the current snapshot (creating the parent directory if needed).
///
/// The write is ATOMIC: the serialized snapshot is written to a sibling temp
/// file in the same directory and then `rename`d over the target, so a crash
/// mid-write can never leave a half-written (corrupt) state file.
pub fn save(path: &Path, update: &AddrUpdate) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating state dir {}", parent.display()))?;
        }
    }
    let serialized = serde_json::to_string_pretty(update).context("serializing state")?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serialized)
        .with_context(|| format!("writing temp state file {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| {
        format!(
            "renaming temp state file {} -> {}",
            tmp.display(),
            path.display()
        )
    })?;
    Ok(())
}

/// True when `current` differs from the last-published snapshot (a `None`
/// previous state — first run — always counts as changed).
pub fn has_changed(previous: Option<&AddrUpdate>, current: &AddrUpdate) -> bool {
    previous != Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sample(a: u8) -> AddrUpdate {
        AddrUpdate {
            wan_v4: Ipv4Addr::new(203, 0, 113, a),
            wan_v6: None,
            lan_v4: Some(Ipv4Addr::new(192, 168, 1, 100)),
        }
    }

    fn tmp_path(tag: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("relay-addr-beacon-{tag}-{nanos}.json"))
    }

    #[test]
    fn load_missing_file_is_none() {
        let p = tmp_path("missing");
        assert!(load(&p).unwrap().is_none());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let p = tmp_path("roundtrip");
        let update = sample(7);
        save(&p, &update).unwrap();
        let loaded = load(&p).unwrap();
        assert_eq!(loaded, Some(update));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn corrupt_state_file_loads_as_none() {
        let p = tmp_path("corrupt");
        std::fs::write(&p, b"\x00\x01not valid json at all}}}").unwrap();
        // A corrupt state file must be treated as first-run, not a hard error.
        assert!(load(&p).unwrap().is_none());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn change_detection() {
        let a = sample(7);
        let b = sample(8);
        // First run (no previous) is a change.
        assert!(has_changed(None, &a));
        // Identical snapshot is not a change.
        assert!(!has_changed(Some(&a), &a));
        // Different WAN is a change.
        assert!(has_changed(Some(&a), &b));
    }
}
