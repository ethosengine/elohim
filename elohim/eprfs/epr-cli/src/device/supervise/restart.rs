//! Row 35: a landing restarts the supervised child onto the landed release, and the device
//! learns whether it came up.
//!
//! After `epr bin follow`, a forward `epr bin land` or an arrival moves a tool's current, the
//! device ark is restarted onto it (when `device_restart.on_landing` allows) and the child's
//! readiness is waited for, bounded by the manifest's own readiness patience. A release whose
//! child does not come up is HELD here: the act that landed it reverts one step (when
//! `device_restart.on_failed_start` allows) and exits non-zero, and nothing restarts or lands
//! onto that release again until a different release lands or a person runs `epr device
//! supervise --release <cid>`.
//!
//! Every record here is device-private and Ephemeral (C), written beside `supervise.json` in the
//! device ark's data root: `restarts.jsonl` (append-only, one row per restart, read by `epr
//! device status`) and `held-releases.json`. Ark's own death witnesses stay ark's; a row names
//! the witness ark wrote and never re-tells the death.
use super::{proc_ppid, Outcome, Started};
use ark_core::{Passport, Probe, RuntimeManifest};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

/// The record the restart policy lives in, read at the device checkout's HEAD.
pub(crate) const POLICY: &str = "genesis/agentic/pool-policy.json";
const RECEIPT: &str = "restarts.jsonl";
const HELD: &str = "held-releases.json";
/// How often the readiness wait reads the passport.
const POLL: Duration = Duration::from_millis(200);

/// What moved current, written on the receipt as the cause (never as authority).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Trigger {
    Follow,
    Land,
    Arrival,
    Hand,
}

impl Trigger {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Follow => "follow",
            Self::Land => "land",
            Self::Arrival => "arrival",
            Self::Hand => "hand",
        }
    }
}

/// One act that moved a tool's current, as the act itself knows it.
pub(crate) struct Landing<'a> {
    pub(crate) trigger: Trigger,
    /// The tool whose current moved; `None` asks about every declared channel (arrival).
    pub(crate) tool: Option<&'a str>,
    /// The release that was current the moment before this act.
    pub(crate) from: Option<String>,
    /// The release this act landed.
    pub(crate) to: Option<String>,
    /// Revert this act's own landing one step, to `from`. `None` when the act cannot (arrival,
    /// a hand supervise): a failed start is then held and left.
    pub(crate) revert: Option<&'a dyn Fn() -> Outcome<()>>,
}

/// How a restart after a landing ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Settled {
    /// Nothing was restarted (no ark, the switch is off, nothing declared, nothing behind).
    Untouched,
    Ready,
    /// The landed release failed readiness, the act reverted one step and the previous serves.
    Reverted,
    /// The landed release failed readiness and nothing was recovered (the revert or the
    /// previous release failed too); the device is degraded.
    Held,
    /// The landed release failed readiness and stays current (`on_failed_start = leave`, or an
    /// act that cannot revert).
    Left,
}

impl Settled {
    fn as_str(self) -> &'static str {
        match self {
            Self::Untouched => "untouched",
            Self::Ready => "ready",
            Self::Reverted => "reverted",
            Self::Held => "held",
            Self::Left => "left",
        }
    }

    /// Whether the landing that caused this is held: a held landing is not a landed one.
    pub(crate) fn held(self) -> bool {
        matches!(self, Self::Reverted | Self::Held | Self::Left)
    }
}

/// `device_restart` in pool-policy.json at the device checkout's HEAD.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Policy {
    pub(crate) restart_on_landing: bool,
    pub(crate) revert_on_failed_start: bool,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            restart_on_landing: true,
            revert_on_failed_start: true,
        }
    }
}

/// Read the restart switches from `checkout`'s HEAD with this device's declared checkout trust,
/// as `put` reads `declared_targets`. An absent record or switch keeps the default; an unknown
/// value keeps the default and says so.
pub(crate) fn policy(checkout: &Path) -> Policy {
    let mut policy = Policy::default();
    let Ok(output) = crate::git::command(checkout)
        .args(["show", &format!("HEAD:{POLICY}")])
        .output()
    else {
        return policy;
    };
    if !output.status.success() {
        return policy;
    }
    let Ok(record) = serde_json::from_slice::<Value>(&output.stdout) else {
        return policy;
    };
    let restart = &record["device_restart"];
    let read = |key: &str, on: &str, off: &str, slot: &mut bool| match restart[key].as_str() {
        None => {}
        Some(value) if value == on => *slot = true,
        Some(value) if value == off => *slot = false,
        Some(value) => eprintln!(
            "device ark: {POLICY} device_restart.{key} is {value:?}, neither {on:?} nor {off:?}; \
             keeping the default"
        ),
    };
    read(
        "on_landing",
        "restart",
        "off",
        &mut policy.restart_on_landing,
    );
    read(
        "on_failed_start",
        "revert",
        "leave",
        &mut policy.revert_on_failed_start,
    );
    policy
}

/// A release that failed readiness on this device.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Hold {
    pub(crate) at: String,
    pub(crate) trigger: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) witness: Option<Witness>,
}

/// The death witness ark wrote for a failed start, named and never retold.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Witness {
    pub(crate) cid: String,
    pub(crate) path: PathBuf,
}

fn held_path(data_root: &Path) -> PathBuf {
    data_root.join(HELD)
}

fn read_held(data_root: &Path) -> BTreeMap<String, Hold> {
    fs::read(held_path(data_root))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_held(data_root: &Path, held: &BTreeMap<String, Hold>) -> Outcome<()> {
    super::write_atomic(
        &held_path(data_root),
        &serde_json::to_vec_pretty(held).map_err(|e| e.to_string())?,
    )
}

/// The hold on `cid`, when this device holds it.
pub(crate) fn held(data_root: &Path, cid: &str) -> Option<Hold> {
    read_held(data_root).remove(cid)
}

/// Every hold, by release CID.
pub(crate) fn holds(data_root: &Path) -> BTreeMap<String, Hold> {
    read_held(data_root)
}

fn hold(data_root: &Path, cid: &str, hold: Hold) -> Outcome<()> {
    let mut held = read_held(data_root);
    held.insert(cid.to_owned(), hold);
    write_held(data_root, &held)
}

/// Clear the hold on `cid`; whether there was one.
pub(crate) fn release(data_root: &Path, cid: &str) -> Outcome<bool> {
    let mut held = read_held(data_root);
    let was = held.remove(cid).is_some();
    if was {
        write_held(data_root, &held)?;
    }
    Ok(was)
}

/// One restart as `restarts.jsonl` records it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Row {
    pub(crate) at: String,
    pub(crate) trigger: String,
    pub(crate) session: Option<String>,
    pub(crate) process: String,
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
    pub(crate) outcome: String,
    /// Seconds from the restart to readiness, or to the end of the wait when it failed.
    pub(crate) seconds: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) witness: Option<Witness>,
}

fn append(data_root: &Path, row: &Row) -> Outcome<()> {
    let path = data_root.join(RECEIPT);
    let mut line = serde_json::to_vec(row).map_err(|e| e.to_string())?;
    line.push(b'\n');
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    file.write_all(&line)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// The newest restart this device recorded.
pub(crate) fn newest(data_root: &Path) -> Option<Row> {
    let text = fs::read_to_string(data_root.join(RECEIPT)).ok()?;
    text.lines()
        .rev()
        .find_map(|line| serde_json::from_str(line).ok())
}

pub(crate) fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// The longest this act waits for readiness: the sum of every declared readiness patience.
fn patience(head: &RuntimeManifest) -> Duration {
    let millis = head
        .processes
        .iter()
        .flat_map(|process| &process.readiness)
        .map(|probe| match probe {
            Probe::ExecutableIdentity { patience_ms }
            | Probe::StdoutLine { patience_ms, .. }
            | Probe::TcpListen { patience_ms, .. } => *patience_ms,
        })
        .fold(0u64, u64::saturating_add);
    Duration::from_millis(millis)
}

/// Why a started ark did not come up.
pub(crate) enum NotReady {
    /// The ark itself exited.
    ArkExited,
    /// The patience ran out with these children not ready.
    Waiting(Vec<String>),
}

impl NotReady {
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::ArkExited => "ark exited".into(),
            Self::Waiting(names) => format!("not ready: {}", names.join(", ")),
        }
    }
}

/// Wait for every declared child of `started` to be ready, reading ark's passport. Holds no lock
/// of any kind: the passport is read whole each time, and nothing else is opened.
pub(super) fn wait_ready(
    data_root: &Path,
    head: &RuntimeManifest,
    started: &Started,
) -> Result<Duration, (Duration, NotReady)> {
    let begun = Instant::now();
    let deadline = begun + patience(head);
    loop {
        if !super::is_same_process(started.pid, started.start_ticks) {
            return Err((begun.elapsed(), NotReady::ArkExited));
        }
        let waiting = not_ready(data_root, head, started);
        if waiting.is_empty() {
            return Ok(begun.elapsed());
        }
        if Instant::now() >= deadline {
            return Err((begun.elapsed(), NotReady::Waiting(waiting)));
        }
        thread::sleep(POLL);
    }
}

/// The declared children of `started` that are not ready: a child that claims readiness until
/// ark marks it ready, one that claims none until it has a live process under this ark.
fn not_ready(data_root: &Path, head: &RuntimeManifest, started: &Started) -> Vec<String> {
    let passport: Option<Passport> = fs::read(data_root.join("ark/passport.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    head.processes
        .iter()
        .filter(|declared| {
            let Some(child) = passport
                .as_ref()
                .and_then(|p| p.processes.iter().find(|child| child.name == declared.name))
            else {
                return true;
            };
            // Only a child of this very ark counts: a passport row left by the ark it replaced
            // names a pid that is gone or belongs to someone else.
            let ours = child
                .pid
                .is_some_and(|pid| proc_ppid(pid) == Some(started.pid));
            !(ours && (declared.readiness.is_empty() || child.ready))
        })
        .map(|declared| declared.name.clone())
        .collect()
}

/// The newest death witness ark wrote for one of `names` since `since_ms`.
pub(crate) fn witness_since(data_root: &Path, names: &[String], since_ms: u64) -> Option<Witness> {
    let dir = data_root.join("ark/witnesses");
    fs::read_dir(&dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|x| x == "json"))
        .filter_map(|path| {
            let record: Value = serde_json::from_slice(&fs::read(&path).ok()?).ok()?;
            let died = record["died_at_epoch_ms"].as_u64()?;
            let process = record["process"].as_str()?;
            (died >= since_ms && names.iter().any(|name| name == process)).then_some((died, path))
        })
        .max_by_key(|(died, _)| *died)
        .and_then(|(_, path)| {
            let cid = path.file_stem()?.to_string_lossy().into_owned();
            Some(Witness { cid, path })
        })
}

/// Everything a restart after a landing writes down.
pub(crate) struct Receipt<'a> {
    pub(crate) data_root: &'a Path,
    pub(crate) trigger: Trigger,
    pub(crate) session: Option<String>,
    pub(crate) process: String,
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
}

impl Receipt<'_> {
    pub(crate) fn record(
        &self,
        outcome: Settled,
        seconds: u64,
        witness: Option<Witness>,
    ) -> Outcome<()> {
        append(
            self.data_root,
            &Row {
                at: now(),
                trigger: self.trigger.as_str().into(),
                session: self.session.clone(),
                process: self.process.clone(),
                from: self.from.clone(),
                to: self.to.clone(),
                outcome: outcome.as_str().into(),
                seconds,
                witness,
            },
        )
    }

    /// Hold the release this restart failed on.
    pub(crate) fn hold(&self, commit: Option<String>, witness: Option<Witness>) -> Outcome<()> {
        let Some(to) = &self.to else {
            return Ok(());
        };
        hold(
            self.data_root,
            to,
            Hold {
                at: now(),
                trigger: self.trigger.as_str().into(),
                commit,
                witness,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_are_set_read_and_cleared_by_cid() {
        let dir = tempfile::tempdir().unwrap();
        assert!(held(dir.path(), "bafy1").is_none());
        hold(
            dir.path(),
            "bafy1",
            Hold {
                at: "t".into(),
                trigger: "follow".into(),
                commit: Some("c".into()),
                witness: None,
            },
        )
        .unwrap();
        assert_eq!(
            held(dir.path(), "bafy1").unwrap().commit.as_deref(),
            Some("c")
        );
        assert!(held(dir.path(), "bafy2").is_none());
        assert!(release(dir.path(), "bafy1").unwrap());
        assert!(!release(dir.path(), "bafy1").unwrap());
        assert!(held(dir.path(), "bafy1").is_none());
    }

    #[test]
    fn unknown_or_absent_switches_keep_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(policy(dir.path()), Policy::default(), "no checkout at all");
    }

    #[test]
    fn the_newest_row_is_the_last_readable_line() {
        let dir = tempfile::tempdir().unwrap();
        assert!(newest(dir.path()).is_none());
        let receipt = Receipt {
            data_root: dir.path(),
            trigger: Trigger::Follow,
            session: Some("s".into()),
            process: "peer".into(),
            from: Some("x".into()),
            to: Some("y".into()),
        };
        receipt.record(Settled::Ready, 3, None).unwrap();
        receipt.record(Settled::Reverted, 9, None).unwrap();
        let row = newest(dir.path()).unwrap();
        assert_eq!((row.outcome.as_str(), row.seconds), ("reverted", 9));
    }
}
