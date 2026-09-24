//! ATTEST — a fold's attestation, written beside its store.
//!
//! Every fold that ran ends in a [`FoldAttestation`]: appended to [`LOG_FILE`] as one JSON line,
//! then the same act renamed into [`LATEST_FILE`] — the log is never behind the snapshot. The
//! directory is the caller's (the recall executor's `<measure-cid>/<embedder>`, the storage peer's
//! `index/<measure-cid>`); this file only writes the two names inside it.
//!
//! Lifted out of the recall executor (post-station-4 sprint, ruling R-S1) with its bodies
//! unchanged.
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use elohim_epr_rea::{FoldAttestation, FoldState};

use crate::error::Result;

/// The latest attestation, pretty-printed.
pub const LATEST_FILE: &str = "attestation.json";

/// Every attestation, one JSON line each, append-only.
pub const LOG_FILE: &str = "attestations.jsonl";

/// The previous attestation's state decides `retried`: a degraded run following a degraded run
/// is the next retry of the same backlog.
pub fn next_retry(dir: &Path) -> u32 {
    std::fs::read(dir.join(LATEST_FILE))
        .ok()
        .and_then(|raw| serde_json::from_slice::<FoldAttestation>(&raw).ok())
        .map_or(0, |last| match last.state {
            FoldState::Degraded { retried } => retried + 1,
            _ => 0,
        })
}

/// Append the act, then rename the latest snapshot into place — the log is never behind the
/// snapshot.
pub fn record(dir: &Path, attestation: &FoldAttestation) -> Result<String> {
    std::fs::create_dir_all(dir)?;
    let cid = attestation.cid()?.to_string();
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(LOG_FILE))?;
    writeln!(log, "{}", serde_json::to_string(attestation)?)?;
    let staged = dir.join(format!("{LATEST_FILE}.tmp"));
    std::fs::write(&staged, serde_json::to_string_pretty(attestation)? + "\n")?;
    std::fs::rename(&staged, dir.join(LATEST_FILE))?;
    Ok(cid)
}
