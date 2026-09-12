//! Persisted last-published address snapshot, for change detection across
//! poll cycles and process restarts.

use std::path::Path;

use anyhow::{Context, Result};

use crate::config::SharedRecordLane;
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

/// Ephemeral serving evidence for ONE shared lane, rebuilt after every
/// restart. DNS is its projection, never evidence that a doorway is currently
/// serving.
#[derive(Default)]
pub struct Membership {
    pub serving: bool,
    consecutive: u64,
    last_probe: Option<bool>,
    pub applied: Option<bool>,
}

impl Membership {
    pub fn observe(&mut self, serving: bool, leave_after: u64, join_after: u64) {
        self.consecutive = if self.last_probe == Some(serving) {
            self.consecutive.saturating_add(1)
        } else {
            1
        };
        self.last_probe = Some(serving);
        if self.consecutive >= if serving { join_after } else { leave_after } {
            self.serving = serving;
        }
    }
}

/// One lane's identity plus the serving evidence held for it.
pub struct LaneMembership {
    pub lane: SharedRecordLane,
    pub membership: Membership,
}

/// Per-lane serving evidence for every shared lane this beacon contributes to.
///
/// One serving probe decides one thing — "is MY doorway serving?" — and every
/// lane is reconciled against that same verdict. The hysteresis counters and
/// the applied-projection marker are nonetheless held PER LANE: a lane whose
/// projection failed (a Cloudflare 503, a wedged sibling lock) must retry on
/// its own, and must not be recorded as applied because a different lane
/// succeeded on the same tick. That is the whole difference between two lanes
/// and one lane written twice.
#[derive(Default)]
pub struct MembershipSet {
    lanes: Vec<LaneMembership>,
}

impl MembershipSet {
    pub fn new(lanes: &[SharedRecordLane]) -> Self {
        Self {
            lanes: lanes
                .iter()
                .map(|lane| LaneMembership {
                    lane: lane.clone(),
                    membership: Membership::default(),
                })
                .collect(),
        }
    }

    /// Feed ONE probe result into every lane's own counters.
    pub fn observe(&mut self, serving: bool, leave_after: u64, join_after: u64) {
        for lane in &mut self.lanes {
            lane.membership.observe(serving, leave_after, join_after);
        }
    }

    pub fn lanes(&self) -> &[LaneMembership] {
        &self.lanes
    }

    pub fn lanes_mut(&mut self) -> &mut [LaneMembership] {
        &mut self.lanes
    }

    /// The evidence held for one lane, by either DNS spelling of its name.
    ///
    /// A reader's accessor: the reconcile path walks every lane in order
    /// (`lanes_mut`), so this exists for tests and for any future diagnostic
    /// that asks about one lane by name.
    #[cfg(test)]
    pub fn lane(&self, record_name: &str) -> Option<&LaneMembership> {
        let key = crate::config::lane_key(record_name);
        self.lanes.iter().find(|held| held.lane.key() == key)
    }
}
