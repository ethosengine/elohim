//! Automatic affirmation: each device that speaks for a person says, on its
//! own chain, that it saw the person's other devices join.
//!
//! Every device a person has joined speaks for them, and any one may approve
//! the next. Affirming is the weight that grows afterwards: a second device of
//! the person reads a joining record, verifies it stands by the coordinator's
//! rule, and records that it did (`affirm_identity_device`). It is not
//! approving. It adds no voice, counts toward no policy, and changes no
//! verdict; it is evidence a person and their witnesses can read
//! (`affirmedBy` in standing).
//!
//! The trigger is the cheapest one that reaches every device eventually: a
//! pass shortly after this node starts and then every [`AFFIRM_EVERY_SECS`].
//! Nothing is pushed between devices and no device waits for another. A
//! device that was away catches up on its next pass, oldest joining first,
//! at most [`AFFIRM_PER_PASS`] a pass.
//!
//! Cost. Every pass: one standing read and one `identity_devices` read (a
//! network link read on the identity's root, a bounded verifying walk per
//! device, one link read per device for its affirmations). Per affirmation:
//! one mandate grant, one affirmation record and its discovery link on this
//! node's chain. A node that speaks for nobody reads its standing and stops.
//!
//! The last read is kept so the witnessed moments (sign-in, device
//! authorization) can be shown who speaks without reading the network again
//! at the moment ([`recent`]).

use std::sync::{Mutex, OnceLock};

use consent_grant::DevicesRead;
use tracing::{info, warn};

use super::device_consent::{CellStanding, ControllerCell};

/// How many devices one pass affirms at most.
pub const AFFIRM_PER_PASS: usize = 4;

/// How long between passes.
pub const AFFIRM_EVERY_SECS: u64 = 30 * 60;

/// How soon after start the first pass runs: long enough for the conductor
/// to answer, short enough that a device that was away catches up promptly.
pub const FIRST_PASS_AFTER_SECS: u64 = 60;

/// How old a kept read may be and still be shown at a witnessed moment.
const RECENT_FOR_MS: i64 = 2 * AFFIRM_EVERY_SECS as i64 * 1000;

/// The last read of the devices that speak for this node's person.
struct Kept {
    identity_root: String,
    read_at_ms: i64,
    read: DevicesRead,
}

fn kept() -> &'static Mutex<Option<Kept>> {
    static KEPT: OnceLock<Mutex<Option<Kept>>> = OnceLock::new();
    KEPT.get_or_init(|| Mutex::new(None))
}

/// Keep a read for the witnessed moments.
pub fn keep(identity_root: &str, read: &DevicesRead, now_ms: i64) {
    *kept().lock().unwrap_or_else(|p| p.into_inner()) = Some(Kept {
        identity_root: identity_root.to_string(),
        read_at_ms: now_ms,
        read: read.clone(),
    });
}

/// The last read for `identity_root`, when it is recent enough to show.
pub fn recent(identity_root: &str, now_ms: i64) -> Option<DevicesRead> {
    let kept = kept().lock().unwrap_or_else(|p| p.into_inner());
    kept.as_ref()
        .filter(|k| k.identity_root == identity_root)
        .filter(|k| now_ms.saturating_sub(k.read_at_ms) <= RECENT_FOR_MS)
        .map(|k| k.read.clone())
}

/// What one pass did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Pass {
    /// Devices affirmed this pass.
    pub affirmed: usize,
    /// Affirmations the cell refused or could not make (tried again next
    /// pass).
    pub failed: usize,
    /// Devices still due after this pass.
    pub still_due: usize,
}

/// One pass: read whom this node speaks for, read the devices that speak for
/// them, and affirm at most [`AFFIRM_PER_PASS`] of those this node has not
/// affirmed, did not approve, and is not.
pub async fn pass(cell: &dyn ControllerCell, now_ms: i64) -> Pass {
    let me = cell.agent();
    let standing = match cell.standing().await {
        Ok(CellStanding::Ready(s)) if s.controllers.contains(&me) => s,
        // Speaks for nobody: nothing to affirm, nothing more to read.
        Ok(_) => return Pass::default(),
        Err(why) => {
            warn!(?why, "device affirmation: standing could not be read");
            return Pass::default();
        }
    };
    let read = match cell.devices(&standing.identity_root).await {
        Ok(read) => read,
        Err(why) => {
            warn!(?why, "device affirmation: devices could not be read");
            return Pass::default();
        }
    };
    keep(&standing.identity_root, &read, now_ms);
    let due_total = read.to_affirm(&me, usize::MAX).len();
    let mut out = Pass::default();
    for device in read.to_affirm(&me, AFFIRM_PER_PASS) {
        match cell.affirm(device).await {
            Ok(()) => {
                out.affirmed += 1;
                info!(
                    device = %device.device_fingerprint,
                    "device affirmation: affirmed another device of this person"
                );
            }
            Err(why) => {
                out.failed += 1;
                warn!(
                    device = %device.device_fingerprint,
                    ?why,
                    "device affirmation: could not affirm; tried again next pass"
                );
            }
        }
    }
    out.still_due = due_total - out.affirmed;
    if out.affirmed + out.failed > 0 || out.still_due > 0 {
        info!(
            affirmed = out.affirmed,
            failed = out.failed,
            still_due = out.still_due,
            "device affirmation: pass done"
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::device_consent::{ApprovalProofs, ApprovalRequest, CellFailure, NewHuman};
    use async_trait::async_trait;
    use consent_grant::{ControllerStanding, StandingDevice};

    const ME: &str = "me";

    fn device(
        key: &str,
        joined_at: i64,
        approved_by: &[&str],
        affirmed_by: &[&str],
    ) -> StandingDevice {
        StandingDevice {
            device_key: key.into(),
            device_fingerprint: key.into(),
            binding: format!("b-{key}"),
            content_dna: "c".into(),
            joined_at,
            approved_by: approved_by.iter().map(|k| k.to_string()).collect(),
            affirmed_by: affirmed_by.iter().map(|k| k.to_string()).collect(),
            affirmed_count: affirmed_by.len(),
            this_device: key == ME,
        }
    }

    struct Cell {
        speaks: bool,
        devices: Vec<StandingDevice>,
        refuse: Option<String>,
        affirmed: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl ControllerCell for Cell {
        fn agent(&self) -> String {
            ME.into()
        }
        async fn standing(&self) -> Result<CellStanding, CellFailure> {
            Ok(CellStanding::Ready(ControllerStanding {
                identity_root: "root".into(),
                authority: "a".into(),
                network_dna: "n".into(),
                controllers: if self.speaks {
                    vec!["r".into(), ME.into()]
                } else {
                    vec!["r".into()]
                },
                required: 1,
                speaks_via: self.speaks.then(|| "via".to_string()),
                also_speaks_for: vec![],
            }))
        }
        async fn sign(&self, _: &ApprovalRequest) -> Result<ApprovalProofs, CellFailure> {
            unreachable!()
        }
        async fn bootstrap(&self, _: &str) -> Result<(), CellFailure> {
            unreachable!()
        }
        async fn create_human(&self, _: &NewHuman) -> Result<(), CellFailure> {
            unreachable!()
        }
        async fn my_human(&self) -> Result<Option<consent_grant::ExistingIdentity>, CellFailure> {
            Ok(None)
        }
        async fn devices(&self, _: &str) -> Result<DevicesRead, CellFailure> {
            Ok(DevicesRead {
                roots: vec!["r".into()],
                devices: self.devices.clone(),
                not_standing: 0,
                truncated: false,
            })
        }
        async fn affirm(&self, device: &StandingDevice) -> Result<(), CellFailure> {
            if let Some(key) = &self.refuse {
                if key == &device.device_key {
                    return Err(CellFailure::Refused("no".into()));
                }
            }
            self.affirmed
                .lock()
                .unwrap()
                .push(device.device_key.clone());
            Ok(())
        }
    }

    #[tokio::test]
    async fn a_pass_affirms_a_few_others_oldest_first_and_a_later_pass_catches_up() {
        let cell = Cell {
            speaks: true,
            devices: vec![
                device(ME, 1, &["r"], &[]),
                device("d5", 50, &["r"], &[]),
                device("d2", 20, &["r"], &[]),
                device("mine", 10, &[ME], &[]),
                device("seen", 15, &["r"], &[ME]),
                device("d3", 30, &["r"], &[]),
                device("d4", 40, &["r"], &[]),
                device("d6", 60, &["r"], &[]),
            ],
            refuse: Some("d3".into()),
            affirmed: Mutex::new(vec![]),
        };
        let out = pass(&cell, 1_000).await;
        // Never itself, never what it approved, never what it affirmed.
        assert_eq!(*cell.affirmed.lock().unwrap(), vec!["d2", "d4", "d5"]);
        assert_eq!(
            out,
            Pass {
                affirmed: 3,
                failed: 1,
                still_due: 2,
            }
        );
        // The read is kept for the witnessed moments, for this identity only.
        assert_eq!(recent("root", 2_000).unwrap().devices.len(), 8);
        assert!(recent("other", 2_000).is_none());
        assert!(recent("root", 1_000 + RECENT_FOR_MS + 1).is_none());
    }

    #[tokio::test]
    async fn a_node_that_speaks_for_nobody_reads_its_standing_and_stops() {
        let cell = Cell {
            speaks: false,
            devices: vec![device("d2", 20, &["r"], &[])],
            refuse: None,
            affirmed: Mutex::new(vec![]),
        };
        assert_eq!(pass(&cell, 1_000).await, Pass::default());
        assert!(cell.affirmed.lock().unwrap().is_empty());
    }
}
