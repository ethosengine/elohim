//! The last follow attempt per tool store: private device operational state.
//!
//! `follow.json` is atomically replaced: `pending` before any work, then one terminal outcome.
//! It is never notarized or synced, carries no environment values or credentials, and never
//! names the installed head; `current.json` stays authoritative for that.
//!
//! Recording is best effort and never changes what a follow does: a record that cannot be
//! written is reported on stderr, and the follow's result, exit code and stdout stay the same.
use super::{files, Refusal, Result, Store};
use crate::session;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
};

/// Record format version written by this build.
const FOLLOW_RECORD_VERSION: u32 = 1;
const FOLLOW_FILE: &str = "follow.json";
/// A pending record whose completion could not be written, moved aside so status never
/// reports a finished attempt as still running.
const UNFINISHED_FILE: &str = "follow.unfinished.json";
const FOLLOW_LOCK: &str = ".follow.lock";
/// Upper bound, in characters, on a recorded reason.
const DETAIL_LIMIT: usize = 240;
const MODES: [&str; 3] = ["apply", "canary", "observe"];

/// Where one follow attempt stands.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FollowOutcome {
    /// Written before any work; terminal only if the attempt was interrupted.
    Pending,
    /// The attempt moved the device head (a new release, or the same release from a new
    /// source commit).
    Landed,
    /// The attempt stopped without moving the head; `kind` names why.
    Refused,
    /// Nothing to do: the head already serves this checkout.
    Unchanged,
    /// Observe mode staged and verified a candidate without landing it.
    Observed,
}
impl FollowOutcome {
    /// Stable lowercase name, as serialized.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Landed => "landed",
            Self::Refused => "refused",
            Self::Unchanged => "unchanged",
            Self::Observed => "observed",
        }
    }
}

/// One follow attempt as `follow.json` holds it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FollowRecord {
    /// Record format version.
    pub version: u32,
    /// Unique attempt identifier; completion only replaces its own attempt.
    pub attempt: String,
    /// UTC start time.
    pub started_at: String,
    /// UTC end time; absent exactly while pending.
    pub ended_at: Option<String>,
    /// Process running the attempt.
    pub owner_pid: u32,
    /// Owner start time in clock ticks as decimal text; PID alone never proves liveness.
    pub owner_start_time: Option<String>,
    /// Source checkout followed, as the caller named it (URL userinfo and query removed).
    pub source: String,
    /// Source commit observed, once known.
    pub commit: Option<String>,
    /// Follow mode: apply, canary or observe.
    pub mode: String,
    /// Device head when the attempt began.
    pub prior_cid: Option<String>,
    /// Release the attempt staged or tried to land, once known.
    pub candidate_cid: Option<String>,
    /// Device head when the attempt ended.
    pub result_cid: Option<String>,
    /// Where the attempt stands.
    pub outcome: FollowOutcome,
    /// Stable reason category (a refusal kind, or why nothing was done).
    pub kind: Option<String>,
    /// Bounded, sanitised reason; never contains credentials.
    pub reason: Option<String>,
}
impl FollowRecord {
    /// A record this build can report without violating the status view contract.
    fn validate(&self) -> std::result::Result<(), String> {
        if self.version != FOLLOW_RECORD_VERSION {
            return Err(format!(
                "unsupported follow record version {}",
                self.version
            ));
        }
        if self.attempt.is_empty() || self.started_at.is_empty() {
            return Err("follow record has no attempt identity".into());
        }
        if !MODES.contains(&self.mode.as_str()) {
            return Err("follow record names an unknown mode".into());
        }
        for cid in [&self.prior_cid, &self.candidate_cid, &self.result_cid]
            .into_iter()
            .flatten()
        {
            if files::require_cid(cid).is_err() || !cid_shape(cid) {
                return Err("follow record names a malformed CID".into());
            }
        }
        for text in [&self.kind, &self.reason].into_iter().flatten() {
            if text.chars().count() > DETAIL_LIMIT {
                return Err("follow record reason exceeds its bound".into());
            }
        }
        if self
            .owner_start_time
            .as_deref()
            .is_some_and(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err("follow record owner start time is not decimal".into());
        }
        if self.ended_at.is_none() != (self.outcome == FollowOutcome::Pending) {
            return Err("follow record end time disagrees with its outcome".into());
        }
        Ok(())
    }
}

/// The last follow attempt as a reader sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LastFollow {
    /// No follow record: this store has never recorded a follow attempt.
    Never,
    /// A terminal record.
    Recorded(FollowRecord),
    /// A pending record whose owner process was observed running.
    InProgress(FollowRecord),
    /// A pending record whose owner could not be observed; `detail` says why.
    Unknown {
        /// The pending record.
        record: FollowRecord,
        /// Why liveness could not be observed.
        detail: String,
    },
    /// A pending record that will never finish: its owner exited, is a zombie, or its
    /// completion could not be written.
    Interrupted {
        /// The pending record.
        record: FollowRecord,
        /// What ended it.
        detail: String,
    },
    /// The record exists but cannot be read, or fails validation.
    Corrupt(String),
}

/// Fields an attempt learns while it runs, and the early-exit verdict when it decides one.
#[derive(Default)]
pub(super) struct Noted {
    pub(super) commit: Option<String>,
    pub(super) candidate: RefCell<Option<String>>,
    pub(super) settled: Option<(FollowOutcome, &'static str, String)>,
}
impl Noted {
    /// Close the attempt with a decided outcome and its reason; the caller returns `why`.
    pub(super) fn settle(
        &mut self,
        outcome: FollowOutcome,
        kind: &'static str,
        why: &str,
    ) -> Value {
        self.settled = Some((outcome, kind, why.to_owned()));
        serde_json::json!({"followed": false, "why": why})
    }
}

/// The committed head as `current.json` names it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct Head {
    cid: Option<String>,
    commit: Option<String>,
}

/// How the owner of a pending record stands.
enum Liveness {
    Running,
    Gone(String),
    Unknown(String),
}

/// What `follow.json` holds when a completion is about to replace it.
enum OnDisk {
    Missing,
    Ours,
    Newer,
    Unreadable(String),
}

/// The view contract's CID shape, `^baf[k-y]r(ei|4i)[a-z2-7]+$` (sha2-256 and BLAKE3 base32
/// CIDv1), checked beside the store's own admission so status JSON never violates its schema.
fn cid_shape(cid: &str) -> bool {
    let bytes = cid.as_bytes();
    bytes.len() > 7
        && cid.starts_with("baf")
        && (b'k'..=b'y').contains(&bytes[3])
        && (bytes[4..].starts_with(b"rei") || bytes[4..].starts_with(b"r4i"))
        && bytes[7..]
            .iter()
            .all(|b| b.is_ascii_lowercase() || (b'2'..=b'7').contains(b))
}

fn warn(message: &str) {
    eprintln!("epr bin follow: {message}");
}

/// Remove URL userinfo, query and fragment from a location; plain paths pass unchanged.
pub(super) fn sanitize_location(text: &str) -> String {
    let Some(scheme_end) = text.find("://") else {
        return text.to_owned();
    };
    let (scheme, rest) = text.split_at(scheme_end + 3);
    let rest = rest.split(['?', '#']).next().unwrap_or("");
    let authority_end = rest.find('/').unwrap_or(rest.len());
    let (authority, path) = rest.split_at(authority_end);
    let host = authority.rsplit('@').next().unwrap_or(authority);
    format!("{scheme}{host}{path}")
}

/// A bounded, single-line reason with URL credentials and secret-named assignments removed.
pub(super) fn sanitize_detail(text: &str) -> String {
    const SECRET_KEYS: [&str; 6] = [
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "CREDENTIAL",
        "AUTH",
    ];
    let words: Vec<String> = text
        .split_whitespace()
        .map(|word| {
            if word.contains("://") {
                return sanitize_location(word);
            }
            if let Some((key, _)) = word.split_once('=') {
                let upper = key.to_ascii_uppercase();
                if SECRET_KEYS.iter().any(|secret| upper.contains(secret)) {
                    return format!("{key}=<redacted>");
                }
            }
            word.to_owned()
        })
        .collect();
    let joined = words.join(" ");
    if joined.chars().count() <= DETAIL_LIMIT {
        return joined;
    }
    let mut bounded: String = joined.chars().take(DETAIL_LIMIT - 1).collect();
    bounded.push('…');
    bounded
}

fn owner_liveness(proc_root: &Path, pid: u32, recorded_start: Option<&str>) -> Liveness {
    if !proc_root.is_dir() {
        return Liveness::Unknown(format!(
            "process table {} is unavailable",
            proc_root.display()
        ));
    }
    let path = proc_root.join(pid.to_string()).join("stat");
    let stat = match std::fs::read_to_string(&path) {
        Ok(stat) => stat,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Liveness::Gone(format!("owner process {pid} has exited"))
        }
        Err(error) => return Liveness::Unknown(format!("cannot read {}: {error}", path.display())),
    };
    let fields: Option<Vec<&str>> = stat
        .rfind(')')
        .and_then(|close| stat.get(close + 1..))
        .map(|rest| rest.split_whitespace().collect());
    let (Some(state), Some(start)) = (
        fields.as_ref().and_then(|f| f.first().copied()),
        fields.as_ref().and_then(|f| f.get(19).copied()),
    ) else {
        return Liveness::Unknown(format!("cannot parse {}", path.display()));
    };
    let Some(recorded) = recorded_start else {
        return Liveness::Unknown("the attempt recorded no owner start time".into());
    };
    if start != recorded {
        return Liveness::Gone(format!("pid {pid} now belongs to another process"));
    }
    if matches!(state, "Z" | "X" | "x") {
        return Liveness::Gone(format!("owner process {pid} is a zombie"));
    }
    Liveness::Running
}

fn parse_record(bytes: &[u8]) -> std::result::Result<FollowRecord, String> {
    let record: FollowRecord = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    record.validate()?;
    Ok(record)
}

impl Store {
    fn follow_path(&self) -> PathBuf {
        self.root.join(FOLLOW_FILE)
    }

    pub(super) fn head_identity(&self) -> Head {
        let path = self.root.join("current.json");
        if !path.is_file() {
            return Head::default();
        }
        let meta: Value = files::read_json(&path).unwrap_or(Value::Null);
        Head {
            cid: self.committed_current(),
            commit: meta["commit"].as_str().map(str::to_owned),
        }
    }

    /// Read the last follow attempt without mutating the store.
    pub fn last_follow(&self, proc_root: &Path) -> LastFollow {
        match std::fs::read(self.follow_path()) {
            Ok(bytes) => match parse_record(&bytes) {
                Err(detail) => LastFollow::Corrupt(detail),
                Ok(record) if record.outcome != FollowOutcome::Pending => {
                    LastFollow::Recorded(record)
                }
                Ok(record) => match owner_liveness(
                    proc_root,
                    record.owner_pid,
                    record.owner_start_time.as_deref(),
                ) {
                    Liveness::Running => LastFollow::InProgress(record),
                    Liveness::Gone(detail) => LastFollow::Interrupted { record, detail },
                    Liveness::Unknown(detail) => LastFollow::Unknown { record, detail },
                },
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                match std::fs::read(self.root.join(UNFINISHED_FILE)) {
                    Ok(bytes) => match parse_record(&bytes) {
                        Ok(record) => LastFollow::Interrupted {
                            record,
                            detail: "the attempt ended but its outcome could not be recorded"
                                .into(),
                        },
                        Err(detail) => LastFollow::Corrupt(detail),
                    },
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => LastFollow::Never,
                    Err(error) => LastFollow::Corrupt(error.to_string()),
                }
            }
            Err(error) => LastFollow::Corrupt(error.to_string()),
        }
    }

    /// Write the pending record before any follow work.
    pub(super) fn begin_follow(&self, repo: &Path, mode: &str) -> Result<FollowRecord> {
        let pid = std::process::id();
        let start = session::process_start_time(&session::proc_root(), pid);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let record = FollowRecord {
            version: FOLLOW_RECORD_VERSION,
            attempt: format!("{pid}-{}-{nanos}", start.unwrap_or_default()),
            started_at: files::now(),
            ended_at: None,
            owner_pid: pid,
            owner_start_time: start.map(|s| s.to_string()),
            source: sanitize_location(&repo.display().to_string()),
            commit: None,
            mode: mode.to_owned(),
            prior_cid: self.committed_current(),
            candidate_cid: None,
            result_cid: None,
            outcome: FollowOutcome::Pending,
            kind: None,
            reason: None,
        };
        let _lock = files::Lock::acquire_named(&self.root, FOLLOW_LOCK, files::MUTATION_WAIT)?;
        files::atomic_json(&self.follow_path(), &record)?;
        match std::fs::remove_file(self.root.join(UNFINISHED_FILE)) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => warn(&format!(
                "could not clear the unfinished follow record: {error}"
            )),
            _ => {}
        }
        Ok(record)
    }

    fn on_disk(&self, attempt: &str) -> OnDisk {
        match std::fs::read(self.follow_path()) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => OnDisk::Missing,
            Err(error) => OnDisk::Unreadable(error.to_string()),
            Ok(bytes) => match serde_json::from_slice::<FollowRecord>(&bytes) {
                Ok(record) if record.attempt == attempt => OnDisk::Ours,
                Ok(_) => OnDisk::Newer,
                Err(error) => OnDisk::Unreadable(error.to_string()),
            },
        }
    }

    /// Replace this attempt's pending record with its terminal record. Fails closed: a newer
    /// or unreadable record is never overwritten. When the write fails, the pending record is
    /// moved aside so status reads the attempt as interrupted rather than running.
    pub(super) fn complete_follow(
        &self,
        record: &FollowRecord,
        write: &dyn Fn(&Path, &FollowRecord) -> Result<()>,
    ) {
        let _lock = match files::Lock::acquire_named(&self.root, FOLLOW_LOCK, files::MUTATION_WAIT)
        {
            Ok(lock) => lock,
            Err(error) => {
                warn(&format!("could not record the follow outcome: {error}"));
                return;
            }
        };
        let ours = match self.on_disk(&record.attempt) {
            OnDisk::Newer => return,
            OnDisk::Unreadable(error) => {
                warn(&format!(
                    "not replacing an unreadable follow record ({error}); outcome {} unrecorded",
                    record.outcome.as_str()
                ));
                return;
            }
            OnDisk::Ours => true,
            OnDisk::Missing => false,
        };
        if let Err(error) = write(&self.follow_path(), record) {
            warn(&format!("could not record the follow outcome: {error}"));
            if ours {
                if let Err(error) =
                    std::fs::rename(self.follow_path(), self.root.join(UNFINISHED_FILE))
                {
                    warn(&format!(
                        "could not mark the pending follow record unfinished: {error}"
                    ));
                }
            }
        }
    }

    /// Run one follow attempt between a pending record and its terminal outcome.
    pub(super) fn recorded_follow(
        &self,
        repo: &Path,
        mode: &str,
        run: impl FnOnce(&mut Noted) -> Result<Value>,
    ) -> Result<Value> {
        let mut guard = Attempt::begin(self, repo, mode);
        let result = run(&mut guard.noted);
        guard.finish(&result);
        result
    }
}

/// One recorded follow attempt. Dropping it unfinished (an unwind) still records a terminal
/// outcome, so a surviving process never shows a dead attempt as running.
pub(super) struct Attempt<'s> {
    store: &'s Store,
    record: Option<FollowRecord>,
    prior: Head,
    pub(super) noted: Noted,
}
impl<'s> Attempt<'s> {
    pub(super) fn begin(store: &'s Store, repo: &Path, mode: &str) -> Self {
        let prior = store.head_identity();
        let record = match store.begin_follow(repo, mode) {
            Ok(record) => Some(record),
            Err(error) => {
                warn(&format!("could not record the follow attempt: {error}"));
                None
            }
        };
        Self {
            store,
            record,
            prior,
            noted: Noted::default(),
        }
    }

    pub(super) fn finish(&mut self, result: &Result<Value>) {
        let Some(mut record) = self.record.take() else {
            return;
        };
        let noted = std::mem::take(&mut self.noted);
        let after = self.store.head_identity();
        classify(&mut record, &self.prior, &after, noted, result);
        self.store
            .complete_follow(&record, &|path, record| files::atomic_json(path, record));
    }
}
impl Drop for Attempt<'_> {
    fn drop(&mut self) {
        if self.record.is_some() {
            self.finish(&Err(Refusal::new(
                "follow-unwound",
                "the follow attempt unwound before it finished",
            )));
        }
    }
}

/// Decide the terminal outcome from the authoritative head before and after the attempt,
/// then from the attempt's own result.
pub(super) fn classify(
    record: &mut FollowRecord,
    prior: &Head,
    after: &Head,
    noted: Noted,
    result: &Result<Value>,
) {
    record.ended_at = Some(files::now());
    record.commit = noted.commit;
    record.candidate_cid = noted.candidate.into_inner();
    record.result_cid = after.cid.clone();
    if let Ok(value) = result {
        if let Some(cid) = value["cid"].as_str().or(value["observed"].as_str()) {
            record.candidate_cid.get_or_insert_with(|| cid.to_owned());
        }
    }
    let moved = after.cid.is_some() && after != prior;
    let ours = moved && after.cid == record.candidate_cid;
    let (outcome, kind, reason) = match (noted.settled, result) {
        _ if ours => match result {
            Ok(_) => (FollowOutcome::Landed, None, None),
            // The head committed, then a later step (link repair) failed.
            Err(error) => (
                FollowOutcome::Landed,
                Some(error.kind.clone()),
                Some(format!("head committed, then: {}", error.detail)),
            ),
        },
        (_, Err(error)) => (
            FollowOutcome::Refused,
            Some(error.kind.clone()),
            Some(error.detail.clone()),
        ),
        (Some((outcome, kind, why)), Ok(_)) => (outcome, Some(kind.to_owned()), Some(why)),
        (None, Ok(value)) if value["followed"] == true => (
            FollowOutcome::Unchanged,
            Some("already-current".to_owned()),
            Some("the candidate already is the device head at this commit".to_owned()),
        ),
        (None, Ok(value)) if value["observed"].is_string() => (FollowOutcome::Observed, None, None),
        // Every early exit settles explicitly; this names a missed path honestly.
        (None, Ok(value)) => (
            FollowOutcome::Refused,
            Some("unclassified".to_owned()),
            value["why"].as_str().map(str::to_owned),
        ),
    };
    record.outcome = outcome;
    record.kind = kind.map(|kind| sanitize_detail(&kind));
    record.reason = reason.map(|reason| sanitize_detail(&reason));
}
