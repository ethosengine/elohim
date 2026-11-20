//! Row 35 A-min: what a restart after a landing writes down, and the one switch it reads.
//!
//! `restarts.jsonl` sits beside `supervise.json` in the device ark's data root: device-private,
//! Ephemeral (C), append-only, one row per landing that moved a channel a declared process runs
//! while ark ran here. Only `epr device status` reads it. Ark's own death witnesses stay ark's.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, io::Write, path::Path};

/// The record the switch lives in, read at the device checkout's HEAD.
pub(crate) const POLICY: &str = "genesis/agentic/pool-policy.json";
const RECEIPT: &str = "restarts.jsonl";

/// `device_restart.on_landing` at `checkout`'s HEAD, read with this device's declared checkout
/// trust as `put` reads `declared_targets`: `true` restarts (the default; also when the record,
/// the switch or its value cannot be read), `false` is `off`.
pub(crate) fn restart_on_landing(checkout: &Path) -> bool {
    let Ok(output) = crate::git::command(checkout)
        .args(["show", &format!("HEAD:{POLICY}")])
        .output()
    else {
        return true;
    };
    let record = output
        .status
        .success()
        .then(|| serde_json::from_slice::<Value>(&output.stdout).ok())
        .flatten();
    match record
        .as_ref()
        .and_then(|r| r["device_restart"]["on_landing"].as_str())
    {
        Some("off") => false,
        None | Some("restart") => true,
        Some(other) => {
            eprintln!(
                "device ark: {POLICY} device_restart.on_landing is {other:?}, neither \"restart\" \
                 nor \"off\"; restarting (the default)"
            );
            true
        }
    }
}

/// One landing as `restarts.jsonl` records it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Row {
    pub(crate) at: String,
    pub(crate) trigger: String,
    pub(crate) session: Option<String>,
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
    /// `ready`, `not-ready` or `skipped`.
    pub(crate) outcome: String,
    /// Why a restart was skipped or did not come up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) reason: Option<String>,
    /// Seconds from the restart to readiness, or to the end of the wait.
    #[serde(default)]
    pub(crate) seconds: u64,
}

impl Row {
    pub(crate) fn new(trigger: &str, from: Option<String>, to: Option<String>) -> Row {
        Row {
            at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            trigger: trigger.to_owned(),
            session: elohim_device_runtime::session::resolve().session,
            from,
            to,
            outcome: String::new(),
            reason: None,
            seconds: 0,
        }
    }

    /// Append this row with `outcome`; a receipt that cannot be written is said, never fatal.
    pub(crate) fn record(
        mut self,
        data_root: &Path,
        outcome: &str,
        reason: Option<String>,
        seconds: u64,
    ) {
        self.outcome = outcome.to_owned();
        self.reason = reason;
        self.seconds = seconds;
        if let Err(error) = append(data_root, &self) {
            eprintln!("device ark: the restart receipt was not written: {error}");
        }
    }
}

fn append(data_root: &Path, row: &Row) -> Result<(), String> {
    let path = data_root.join(RECEIPT);
    let mut line = serde_json::to_vec(row).map_err(|e| e.to_string())?;
    line.push(b'\n');
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut file| file.write_all(&line))
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// The newest restart row this data root holds.
pub(crate) fn newest(data_root: &Path) -> Option<Row> {
    fs::read_to_string(data_root.join(RECEIPT))
        .ok()?
        .lines()
        .rev()
        .find_map(|line| serde_json::from_str(line).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_newest_row_is_the_last_readable_line() {
        let dir = tempfile::tempdir().unwrap();
        assert!(newest(dir.path()).is_none());
        Row::new("follow", Some("x".into()), Some("y".into())).record(dir.path(), "ready", None, 3);
        Row::new("land", None, Some("z".into())).record(
            dir.path(),
            "skipped",
            Some("off".into()),
            0,
        );
        fs::OpenOptions::new()
            .append(true)
            .open(dir.path().join(RECEIPT))
            .unwrap()
            .write_all(b"{torn\n")
            .unwrap();
        let row = newest(dir.path()).unwrap();
        assert_eq!(
            (row.trigger.as_str(), row.outcome.as_str()),
            ("land", "skipped")
        );
        assert_eq!(row.reason.as_deref(), Some("off"));
    }

    #[test]
    fn an_unreadable_policy_keeps_the_default() {
        let dir = tempfile::tempdir().unwrap();
        assert!(restart_on_landing(dir.path()));
    }
}
