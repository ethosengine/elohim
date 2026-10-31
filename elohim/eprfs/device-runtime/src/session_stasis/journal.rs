//! Journal replay and the append path, under the same `journal.lock` flock protocol as the
//! Python writer (`.claude/hooks/dialogue-stasis-sense.py`): the fold reads under a shared
//! lock; an append (event, `state.json` sequence, segment rotation) holds the exclusive lock.

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use rustix::fs::{flock, FlockOperation};
use serde_json::Value;

use super::event::Event;

/// Why a journal operation did not produce evidence.
#[derive(Debug)]
pub enum StasisError {
    /// A line or flag could not be parsed (CLI exit 1).
    Parse(String),
    /// There is no journal to read (CLI exit 2): unmeasured, not zero.
    NoJournal(String),
    /// A filesystem failure.
    Io(String),
}

impl std::fmt::Display for StasisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(m) | Self::NoJournal(m) | Self::Io(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for StasisError {}

fn io(path: &Path, e: std::io::Error) -> StasisError {
    StasisError::Io(format!("{}: {e}", path.display()))
}

/// A gap the replay itself discovered.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayGap {
    /// `retention`, `sequence-discontinuity`, `torn-tail` or `duplicate-seq`.
    pub reason: String,
    /// The first sequence number involved, when any.
    pub from_seq: Option<u64>,
    /// The last sequence number involved, when any.
    pub to_seq: Option<u64>,
}

/// One session's replayed journal.
#[derive(Debug)]
pub struct Replay {
    /// Session id (the directory name).
    pub session: String,
    /// Events in sequence order, duplicates removed.
    pub events: Vec<Event>,
    /// Gaps found while replaying.
    pub gaps: Vec<ReplayGap>,
    /// Number of segment files read.
    pub segments: usize,
}

/// The numbered segments (`events.<n>.jsonl`) in increasing `n`.
pub fn segments(dir: &Path) -> Vec<(u64, PathBuf)> {
    let mut out: Vec<(u64, PathBuf)> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let n = name.strip_prefix("events.")?.strip_suffix(".jsonl")?;
            Some((n.parse().ok()?, e.path()))
        })
        .collect();
    out.sort();
    out
}

/// Session directories under `root` that hold any journal file.
pub fn sessions(root: &Path) -> Result<Vec<String>, StasisError> {
    let entries = fs::read_dir(root)
        .map_err(|e| StasisError::NoJournal(format!("{}: {e}", root.display())))?;
    let mut out: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter(|e| e.path().join("events.jsonl").exists() || !segments(&e.path()).is_empty())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    Ok(out)
}

/// A held journal lock; released on drop.
#[derive(Debug)]
pub struct Lock {
    _held: Option<File>,
}

fn lock_path(dir: &Path) -> PathBuf {
    dir.join("journal.lock")
}

impl Lock {
    /// Shared lock for reading. A journal with no lock file (never written by the adapter) is
    /// read unlocked.
    pub fn shared(dir: &Path) -> Result<Self, StasisError> {
        match File::open(lock_path(dir)) {
            Ok(f) => {
                flock(&f, FlockOperation::LockShared)
                    .map_err(|e| StasisError::Io(format!("flock: {e}")))?;
                Ok(Self { _held: Some(f) })
            }
            Err(_) => Ok(Self { _held: None }),
        }
    }

    /// Exclusive lock for appending, creating the lock file as the writer does.
    pub fn exclusive(dir: &Path) -> Result<Self, StasisError> {
        let f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(lock_path(dir))
            .map_err(|e| io(&lock_path(dir), e))?;
        flock(&f, FlockOperation::LockExclusive)
            .map_err(|e| StasisError::Io(format!("flock: {e}")))?;
        Ok(Self { _held: Some(f) })
    }
}

/// Replay `<root>/<session>` under a shared lock.
pub fn replay(root: &Path, session: &str) -> Result<Replay, StasisError> {
    let dir = root.join(session);
    if !dir.is_dir() {
        return Err(StasisError::NoJournal(format!(
            "no journal for session {session}"
        )));
    }
    let _lock = Lock::shared(&dir)?;
    replay_unlocked(&dir, session)
}

/// Replay a session directory the caller already holds the lock for.
pub fn replay_unlocked(dir: &Path, session: &str) -> Result<Replay, StasisError> {
    let mut files: Vec<PathBuf> = segments(dir).into_iter().map(|(_, p)| p).collect();
    let active = dir.join("events.jsonl");
    if active.exists() {
        files.push(active);
    }
    if files.is_empty() {
        return Err(StasisError::NoJournal(format!(
            "no journal for session {session}"
        )));
    }
    let mut events: Vec<Event> = Vec::new();
    let mut gaps = Vec::new();
    for path in &files {
        let raw = fs::read_to_string(path).map_err(|e| io(path, e))?;
        for (n, line) in raw.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let value = match serde_json::from_str::<Value>(line) {
                Ok(v) => v,
                // A record cut short by a crash (the writer terminates it with a newline before
                // appending again): missing evidence wherever it sits, not a malformed journal.
                Err(e) if e.is_eof() => {
                    gaps.push(ReplayGap {
                        reason: "torn-record".into(),
                        from_seq: None,
                        to_seq: None,
                    });
                    continue;
                }
                Err(e) => {
                    return Err(StasisError::Parse(format!(
                        "{}:{}: {e}",
                        path.display(),
                        n + 1
                    )))
                }
            };
            let event = Event::from_value(&value)
                .map_err(|e| StasisError::Parse(format!("{}:{}: {e}", path.display(), n + 1)))?;
            events.push(event);
        }
    }
    // Segment order is the journal's order: increasing `n`, then the active file. A sequence
    // number that repeats or goes backwards is evidence the order cannot be trusted, so it is
    // named; a repeat is dropped (idempotent), a backwards step is kept where it was written.
    let mut seen = std::collections::HashSet::new();
    let mut ordered: Vec<Event> = Vec::with_capacity(events.len());
    let mut previous: Option<u64> = None;
    for event in events {
        if !seen.insert(event.seq) {
            gaps.push(ReplayGap {
                reason: "duplicate-seq".into(),
                from_seq: Some(event.seq),
                to_seq: Some(event.seq),
            });
            continue;
        }
        match previous {
            Some(p) if event.seq < p => gaps.push(ReplayGap {
                reason: "backwards-seq".into(),
                from_seq: Some(event.seq),
                to_seq: Some(p),
            }),
            Some(p) if event.seq > p + 1 => gaps.push(ReplayGap {
                reason: "sequence-discontinuity".into(),
                from_seq: Some(p + 1),
                to_seq: Some(event.seq - 1),
            }),
            _ => {}
        }
        previous = Some(previous.map_or(event.seq, |p| p.max(event.seq)));
        ordered.push(event);
    }
    let events = ordered;
    if let Some(first) = events.first() {
        if first.seq > 1 {
            gaps.push(ReplayGap {
                reason: "retention".into(),
                from_seq: Some(1),
                to_seq: Some(first.seq - 1),
            });
        }
    }
    Ok(Replay {
        session: session.to_owned(),
        events,
        gaps,
        segments: files.len(),
    })
}

/// Append one fully formed event line, mirroring the writer: rotate `events.jsonl` to
/// `events.<n>.jsonl` past `segment_limit` bytes and keep `state.json`'s `nextSeq`/`nextSegment`
/// in step so the writer does not record a discontinuity. The caller holds the exclusive lock.
pub fn append_locked(
    dir: &Path,
    event: &Value,
    seq: u64,
    segment_limit: u64,
) -> Result<(), StasisError> {
    let mut raw = serde_json::to_vec(event).map_err(|e| StasisError::Io(e.to_string()))?;
    raw.push(b'\n');
    let active = dir.join("events.jsonl");
    let mut rotated_to = None;
    if let Ok(meta) = fs::metadata(&active) {
        if meta.len() > 0 && meta.len() + raw.len() as u64 > segment_limit.max(1) {
            let highest = segments(dir).last().map_or(0, |(n, _)| *n);
            let next_segment = state_u64(dir, "nextSegment").unwrap_or(1).max(highest + 1);
            fs::rename(&active, dir.join(format!("events.{next_segment}.jsonl")))
                .map_err(|e| io(&active, e))?;
            rotated_to = Some(next_segment + 1);
        }
    }
    // A crash can leave the active file without a final newline; a new record must start a line
    // of its own (the Python writer terminates the torn record the same way).
    if let Ok(existing) = fs::read(&active) {
        if existing.last().is_some_and(|b| *b != b'\n') {
            raw.insert(0, b'\n');
        }
    }
    let mut stream = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&active)
        .map_err(|e| io(&active, e))?;
    stream.write_all(&raw).map_err(|e| io(&active, e))?;
    stream.sync_all().map_err(|e| io(&active, e))?;
    update_state(dir, seq + 1, rotated_to)
}

fn state_u64(dir: &Path, key: &str) -> Option<u64> {
    let v: Value = serde_json::from_slice(&fs::read(dir.join("state.json")).ok()?).ok()?;
    v.get(key)?.as_u64()
}

fn update_state(dir: &Path, next_seq: u64, next_segment: Option<u64>) -> Result<(), StasisError> {
    let path = dir.join("state.json");
    // No cache, or one the writer cannot read: leave it, the writer rebuilds from the journal.
    let Ok(bytes) = fs::read(&path) else {
        return Ok(());
    };
    let Ok(mut state) = serde_json::from_slice::<Value>(&bytes) else {
        return Ok(());
    };
    let Some(object) = state.as_object_mut() else {
        return Ok(());
    };
    object.insert("nextSeq".into(), next_seq.into());
    if let Some(n) = next_segment {
        object.insert("nextSegment".into(), n.into());
    }
    let temp = dir.join("state.json.tmp");
    let mut stream = File::create(&temp).map_err(|e| io(&temp, e))?;
    stream
        .write_all(&serde_json::to_vec(&state).map_err(|e| StasisError::Io(e.to_string()))?)
        .map_err(|e| io(&temp, e))?;
    stream.sync_all().map_err(|e| io(&temp, e))?;
    fs::rename(&temp, &path).map_err(|e| io(&path, e))
}
