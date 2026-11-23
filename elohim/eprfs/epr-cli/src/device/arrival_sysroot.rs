//! The `sysroot` line of `epr device status`: this host's system link inputs, by the artifact
//! key's own probe, remembered by `epr device arrive` and re-checked by a status read.
use super::{Check, State};
use elohim_device_runtime::flock::Flock;
use serde::{Deserialize, Serialize};
use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

const SYSROOT_CHECK: &str = "sysroot";
/// The remembered sysroot, under the layout's state directory (persistent across a workspace
/// restart; `$HOME` is not), and the lock that serialises arrivals' read-update-replace of it.
const SYSROOT_RECORD: &str = "sysroot/observed.json";
const SYSROOT_LOCK: &str = "observed.lock";
/// A real record is a few KiB; anything past this is not one, and is never read into memory.
const SYSROOT_RECORD_LIMIT: u64 = 64 * 1024;
/// Earlier identities kept in the record, newest first, so a second change inside the notice
/// does not erase the first.
const SYSROOT_HISTORY: usize = 8;
/// How long a changed sysroot stays a degraded line after the change is first recorded: long
/// enough for every session of a working week to read it, short enough not to become wallpaper.
/// A judgement, not a measured bound; the change itself stays in the record's history.
const SYSROOT_NOTICE_DAYS: i64 = 7;
/// Wall time an arrival's full observation may take; every child is killed at it. The SessionStart
/// hook that runs `epr device arrive` gives all of arrival 30 s.
const SYSROOT_PROBE_SECONDS: u64 = 10;
/// How long an arrival waits for another arrival's hold on the record.
const SYSROOT_LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(5);
/// A status read's own bound for re-checking the recorded files.
const SYSROOT_RECHECK: std::time::Duration = std::time::Duration::from_secs(2);
/// What the identity cannot see, said wherever a change is reported or nothing is recorded yet.
const SYSROOT_BLIND: &str = "the identity covers cc's version and predefined macros and the crt, \
     libgcc, libc and libm files it links; a same-path gcc rebuild with identical output is not \
     seen";

/// A full observation: the identity lines of the artifact key's probe and the human context.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SysrootObserved {
    /// [`crate::host_link::sysroot_lines`]: sorted `tool:link-driver*` and `link:` lines.
    lines: Vec<String>,
    /// `/etc/os-release` `ID VERSION_ID`, when readable.
    os: Option<String>,
    /// The loaded glibc's `gnu_get_libc_version`, on a glibc host.
    glibc: Option<String>,
}
impl SysrootObserved {
    /// CID over the lines, one per line with a trailing newline.
    fn cid(&self) -> String {
        let document: String = self.lines.iter().map(|line| format!("{line}\n")).collect();
        eprfs_core::BlobCid::compute_bytes(document.as_bytes()).to_string()
    }
}

/// One remembered identity and when this device first recorded it (RFC 3339, UTC). Only the
/// current identity keeps its lines, so a status read can re-check their files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SysrootSeen {
    cid: String,
    os: Option<String>,
    glibc: Option<String>,
    first_seen: String,
    /// When an arrival first saw this identity past its notice (RFC 3339): from then on the
    /// change is settled, whatever a later clock says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    settled_at: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    lines: Vec<String>,
}

/// The current identity and the earlier ones, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SysrootRecord {
    current: SysrootSeen,
    #[serde(default)]
    history: Vec<SysrootSeen>,
}

/// The line an observation past its deadline gives.
fn probe_exceeded() -> String {
    format!("probe exceeded {SYSROOT_PROBE_SECONDS}s")
}

/// Observe this host's sysroot with the artifact key's own link-input primitives under this
/// process's PATH. Runs only the C driver, never rustc; `deadline` covers every child, its pipes
/// and every file read.
fn observe_sysroot(
    env: &dyn Fn(&str) -> Option<String>,
    deadline: std::time::Instant,
) -> Result<SysrootObserved, String> {
    let path = env("PATH")
        .filter(|value| !value.is_empty())
        .ok_or("PATH is not set")?;
    let lines = crate::host_link::sysroot_lines(&path, deadline).map_err(|e| {
        let why = e.to_string();
        if why.contains("exceeded the probe deadline") {
            probe_exceeded()
        } else {
            why
        }
    })?;
    Ok(SysrootObserved {
        lines,
        os: os_context(
            &[
                Path::new("/etc/os-release"),
                Path::new("/usr/lib/os-release"),
            ],
            deadline,
        ),
        glibc: glibc_version(),
    })
}

/// os-release is read only as context: through the bounded read (regular file, at most
/// [`OS_RELEASE_LIMIT`] bytes, inside `deadline`); anything unreadable omits the context.
fn os_context(files: &[&Path], deadline: std::time::Instant) -> Option<String> {
    files
        .iter()
        .find_map(|file| crate::host_link::read_context(file, OS_RELEASE_LIMIT, deadline))
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|text| os_release(&text))
}

/// A real os-release is well under 1 KiB.
const OS_RELEASE_LIMIT: u64 = 64 * 1024;

/// `ID VERSION_ID` from an os-release document (either may be quoted); `ID` alone without a
/// version.
fn os_release(text: &str) -> Option<String> {
    let field = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
            .map(|value| value.trim().trim_matches(['"', '\'']).to_owned())
            .filter(|value| !value.is_empty())
    };
    let id = field("ID")?;
    Some(match field("VERSION_ID") {
        Some(version) => format!("{id} {version}"),
        None => id,
    })
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn glibc_version() -> Option<String> {
    // SAFETY: glibc returns a static, NUL-terminated string for the loaded libc.
    let version = unsafe { std::ffi::CStr::from_ptr(libc::gnu_get_libc_version()) };
    version.to_str().ok().map(str::to_owned)
}
#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
fn glibc_version() -> Option<String> {
    None
}

/// `<short cid> (<os>, glibc <v>)`, leaving out what was not observed.
fn describe_sysroot(seen: &SysrootSeen) -> String {
    let context: Vec<String> = seen
        .os
        .clone()
        .into_iter()
        .chain(seen.glibc.as_ref().map(|v| format!("glibc {v}")))
        .collect();
    let short = crate::frames::short_cid(&seen.cid);
    if context.is_empty() {
        short
    } else {
        format!("{short} ({})", context.join(", "))
    }
}

fn parse_stamp(stamp: &str) -> Result<chrono::DateTime<chrono::Utc>, String> {
    chrono::DateTime::parse_from_rfc3339(stamp)
        .map(|time| time.with_timezone(&chrono::Utc))
        .map_err(|e| format!("invalid time {stamp:?}: {e}"))
}

/// Whether `now` is at or past the end of the notice that began at `since`.
fn past_notice(since: &str, now: chrono::DateTime<chrono::Utc>) -> bool {
    parse_stamp(since).is_ok_and(|since| {
        now.signed_duration_since(since) >= chrono::TimeDelta::days(SYSROOT_NOTICE_DAYS)
    })
}

/// Whether `now` is inside the notice that began at `since`: `[since, since + 7 days)`. A clock
/// behind the recorded change is outside it, so a backward step cannot revive a settled notice.
fn in_notice(since: &str, now: chrono::DateTime<chrono::Utc>) -> bool {
    parse_stamp(since).is_ok_and(|since| {
        let elapsed = now.signed_duration_since(since);
        elapsed >= chrono::TimeDelta::zero()
            && elapsed < chrono::TimeDelta::days(SYSROOT_NOTICE_DAYS)
    })
}

fn unobserved(why: impl fmt::Display, cure: Option<String>) -> Check {
    Check::new(
        SYSROOT_CHECK,
        State::Degraded,
        format!("unobserved ({why})"),
        cure,
    )
}

fn sysroot_changed(was: &SysrootSeen, now: &SysrootSeen) -> Check {
    Check::new(
        SYSROOT_CHECK,
        State::Degraded,
        format!(
            "changed since {} — was {}, now {}; artifact keys and pool build caches made before \
             are for the previous system libraries",
            now.first_seen,
            describe_sysroot(was),
            describe_sysroot(now)
        ),
        Some(format!(
            "none needed for correctness; expect cold builds and key misses; record the new image \
             in the journal; this line settles at the first arrival {SYSROOT_NOTICE_DAYS} days \
             after the change (a status read alone does not store it); \
             {SYSROOT_BLIND}"
        )),
    )
}

/// The line for a recorded identity that is still current: a change inside its notice and not
/// yet settled, else ok.
fn sysroot_settled(record: &SysrootRecord, now: chrono::DateTime<chrono::Utc>) -> Check {
    match record.history.first() {
        Some(was)
            if record.current.settled_at.is_none()
                && in_notice(&record.current.first_seen, now) =>
        {
            sysroot_changed(was, &record.current)
        }
        _ => Check::new(
            SYSROOT_CHECK,
            State::Ok,
            describe_sysroot(&record.current),
            None,
        ),
    }
}

/// An arrival's judgement of a full observation against the remembered record. Returns the line
/// and, when the record should change, the record to write.
fn sysroot_judge(
    observed: SysrootObserved,
    remembered: Option<SysrootRecord>,
    now: chrono::DateTime<chrono::Utc>,
) -> (Check, Option<SysrootRecord>) {
    let seen = SysrootSeen {
        cid: observed.cid(),
        os: observed.os,
        glibc: observed.glibc,
        first_seen: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        settled_at: None,
        lines: observed.lines,
    };
    match remembered {
        Some(mut record) if record.current.cid == seen.cid => {
            let check = sysroot_settled(&record, now);
            // Store the settle decision the first time an arrival sees the notice over, so a
            // clock stepped back afterwards cannot revive it.
            if !record.history.is_empty()
                && record.current.settled_at.is_none()
                && past_notice(&record.current.first_seen, now)
            {
                record.current.settled_at = Some(seen.first_seen);
                (check, Some(record))
            } else {
                (check, None)
            }
        }
        Some(record) => {
            let check = sysroot_changed(&record.current, &seen);
            let mut history = vec![SysrootSeen {
                lines: Vec::new(),
                settled_at: None,
                ..record.current
            }];
            history.extend(record.history);
            history.truncate(SYSROOT_HISTORY);
            (
                check,
                Some(SysrootRecord {
                    current: seen,
                    history,
                }),
            )
        }
        None => (
            Check::new(SYSROOT_CHECK, State::Ok, describe_sysroot(&seen), None),
            Some(SysrootRecord {
                current: seen,
                history: Vec::new(),
            }),
        ),
    }
}

/// A status read's judgement: the record, plus a cheap re-check of the files its current lines
/// name ([`crate::host_link::sysroot_drift`]). Runs no program and writes nothing.
fn sysroot_status(remembered: Option<SysrootRecord>, now: chrono::DateTime<chrono::Utc>) -> Check {
    let Some(record) = remembered else {
        return Check::new(
            SYSROOT_CHECK,
            State::Unknown,
            format!("not yet observed; an arrival records it ({SYSROOT_BLIND})"),
            Some("epr device arrive".into()),
        );
    };
    let deadline = std::time::Instant::now() + SYSROOT_RECHECK;
    let drifted = match crate::host_link::sysroot_drift(&record.current.lines, deadline) {
        Ok(drifted) => drifted,
        Err(why) => return unobserved(format!("re-check: {why}"), None),
    };
    if drifted.is_empty() {
        return sysroot_settled(&record, now);
    }
    let shown: Vec<&str> = drifted.iter().take(3).map(String::as_str).collect();
    Check::new(
        SYSROOT_CHECK,
        State::Degraded,
        format!(
            "changed after {}: {} of {} recorded inputs differ ({}{}) from {}; artifact keys and \
             pool build caches made before are for the previous system libraries",
            record.current.first_seen,
            drifted.len(),
            record.current.lines.len(),
            shown.join(", "),
            if drifted.len() > shown.len() {
                ", …"
            } else {
                ""
            },
            describe_sysroot(&record.current)
        ),
        Some(format!(
            "epr device arrive records the new identity; none needed for correctness; expect cold \
             builds and key misses; {SYSROOT_BLIND}"
        )),
    )
}

/// Read the record without following a link at its path, refusing anything but a regular file
/// of at most [`SYSROOT_RECORD_LIMIT`] bytes, and refusing an unparseable timestamp.
fn read_sysroot_record(file: &Path) -> Result<Option<SysrootRecord>, String> {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    if let Some(dir) = file.parent() {
        refuse_linked_dir(dir)?;
    }
    let opened = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(file);
    let handle = match opened {
        Ok(handle) => handle,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) if e.raw_os_error() == Some(libc::ELOOP) => {
            return Err(format!("{} is a symbolic link", file.display()))
        }
        Err(e) => return Err(format!("{}: {e}", file.display())),
    };
    let meta = handle
        .metadata()
        .map_err(|e| format!("{}: {e}", file.display()))?;
    if !meta.is_file() {
        return Err(format!("{} is not a regular file", file.display()));
    }
    let mut bytes = Vec::new();
    handle
        .take(SYSROOT_RECORD_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", file.display()))?;
    if bytes.len() as u64 > SYSROOT_RECORD_LIMIT {
        return Err(format!(
            "{} exceeds {} bytes",
            file.display(),
            SYSROOT_RECORD_LIMIT
        ));
    }
    let record: SysrootRecord =
        serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", file.display()))?;
    check_sysroot_record(&record).map_err(|why| format!("{}: {why}", file.display()))?;
    Ok(Some(record))
}

/// A record is data from disk: the current lines must match the probe's grammar and hash to the
/// recorded identity, earlier identities carry no lines, and every time and CID must parse.
fn check_sysroot_record(record: &SysrootRecord) -> Result<(), String> {
    crate::host_link::check_sysroot_lines(&record.current.lines)?;
    let recomputed = SysrootObserved {
        lines: record.current.lines.clone(),
        os: None,
        glibc: None,
    }
    .cid();
    if recomputed != record.current.cid {
        return Err(format!(
            "recorded identity {} is not the CID of its lines ({recomputed})",
            record.current.cid
        ));
    }
    if record.history.len() > SYSROOT_HISTORY {
        return Err(format!("{} earlier identities", record.history.len()));
    }
    for seen in std::iter::once(&record.current).chain(&record.history) {
        eprfs_core::BlobCid::parse(&seen.cid).map_err(|e| format!("cid {:?}: {e}", seen.cid))?;
        parse_stamp(&seen.first_seen)?;
        if let Some(settled) = &seen.settled_at {
            parse_stamp(settled)?;
        }
    }
    if record
        .history
        .iter()
        .any(|seen| !seen.lines.is_empty() || seen.settled_at.is_some())
    {
        return Err("an earlier identity carries lines or a settle time".into());
    }
    Ok(())
}

/// Replace the record whole through an exclusively created temporary file in its directory,
/// renamed over it: a link or file at a predictable name is never written through.
fn write_sysroot_record(file: &Path, record: &SysrootRecord) -> Result<(), String> {
    use std::io::Write;
    let parent = file.parent().ok_or("record has no directory")?;
    fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    // Never write what the reader refuses: drop the oldest identities until it fits.
    let mut record = record.clone();
    let bytes = loop {
        let bytes = serde_json::to_vec_pretty(&record).map_err(|e| e.to_string())?;
        if bytes.len() as u64 <= SYSROOT_RECORD_LIMIT {
            break bytes;
        }
        if record.history.pop().is_none() {
            return Err(format!(
                "the record would be {} bytes, over the {SYSROOT_RECORD_LIMIT}-byte bound",
                bytes.len()
            ));
        }
    };
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| format!("{}: {e}", parent.display()))?;
    temp.write_all(&bytes)
        .and_then(|()| temp.as_file().sync_all())
        .map_err(|e| format!("{}: {e}", temp.path().display()))?;
    temp.persist(file)
        .map(drop)
        .map_err(|e| format!("{}: {}", file.display(), e.error))
}

/// Refuse a record directory that is itself a symbolic link: the record and its lock live in
/// it, and neither is read or created through a link. Directories above it are not checked.
fn refuse_linked_dir(dir: &Path) -> Result<(), String> {
    if fs::symlink_metadata(dir).is_ok_and(|meta| meta.file_type().is_symlink()) {
        Err(format!("{} is a symbolic link", dir.display()))
    } else {
        Ok(())
    }
}

/// Hold the arrivals' exclusive lock beside the record (the kernel lock `arrival_work` uses for
/// its worker), waiting at most `wait` for another arrival to finish. The open never blocks (a
/// FIFO planted at the name) and the lock must be a regular file. The guard unlocks when dropped,
/// so a child another thread forked meanwhile, whose copy of the descriptor lasts until its
/// `exec`, does not keep a finished arrival's lock held.
fn lock_sysroot(file: &Path, wait: std::time::Duration) -> Result<Flock, String> {
    use std::os::unix::fs::OpenOptionsExt;
    let dir = file.parent().ok_or("record has no directory")?;
    refuse_linked_dir(dir)?;
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(SYSROOT_LOCK);
    // A FIFO or device opened for writing without a reader fails rather than blocking, but say
    // what it is before trying.
    if fs::symlink_metadata(&path).is_ok_and(|meta| !meta.is_file()) {
        return Err(format!("{} is not a regular file", path.display()));
    }
    let mut lock = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if !lock.metadata().is_ok_and(|meta| meta.is_file()) {
        return Err(format!("{} is not a regular file", path.display()));
    }
    let until = std::time::Instant::now() + wait;
    loop {
        match Flock::try_exclusive(lock) {
            Ok(Ok(held)) => return Ok(held),
            Ok(Err(file)) if std::time::Instant::now() < until => {
                lock = file;
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Ok(Err(_)) => return Err(format!("another arrival holds {}", path.display())),
            Err(e) => return Err(format!("{}: {e}", path.display())),
        }
    }
}

/// The sysroot line for `epr device status` / `arrive`, with the record under `state_dir` and
/// this process's environment for the probe.
pub(super) fn line(
    env: &dyn Fn(&str) -> Option<String>,
    state_dir: Result<&Path, String>,
    remember: bool,
) -> Check {
    sysroot_check(
        &|deadline| observe_sysroot(env, deadline),
        state_dir.map(|dir| dir.join(SYSROOT_RECORD)),
        remember,
        chrono::Utc::now(),
    )
}

/// The sysroot line. A status read (`remember` false) reads the record and re-checks its files
/// within bounds: the sysroot check runs no program and writes nothing. Only `epr device arrive`
/// runs the full observation and replaces the record, under the arrivals' lock; one deadline of
/// [`SYSROOT_PROBE_SECONDS`] covers the whole observation, and every exit path drops the lock.
///
/// Stated limits: a stalled filesystem can hold one synchronous open, read or `sync_all` past
/// either budget, as it can every other check of this command; and a status read that sees the
/// notice over does not persist settlement (only an arrival stores `settledAt`), so a clock
/// stepped back before any such arrival can show the notice again.
fn sysroot_check(
    observe: &dyn Fn(std::time::Instant) -> Result<SysrootObserved, String>,
    record_file: Result<PathBuf, String>,
    remember: bool,
    now: chrono::DateTime<chrono::Utc>,
) -> Check {
    let file = match record_file {
        Ok(file) => file,
        Err(why) => return unobserved(format!("no record place: {why}"), None),
    };
    let unreadable = |why: String| {
        unobserved(
            format!("record unreadable: {why}"),
            Some(format!(
                "remove {}; the next epr device arrive records afresh",
                file.display()
            )),
        )
    };
    if !remember {
        return match read_sysroot_record(&file) {
            Ok(remembered) => sysroot_status(remembered, now),
            Err(why) => unreadable(why),
        };
    }
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(SYSROOT_PROBE_SECONDS);
    let _lock = match lock_sysroot(&file, SYSROOT_LOCK_WAIT) {
        Ok(lock) => lock,
        Err(why) => return unobserved(why, None),
    };
    let remembered = match read_sysroot_record(&file) {
        Ok(remembered) => remembered,
        Err(why) => return unreadable(why),
    };
    if std::time::Instant::now() >= deadline {
        return unobserved(probe_exceeded(), None);
    }
    let observed = match observe(deadline) {
        Ok(observed) => observed,
        Err(why) => return unobserved(why, None),
    };
    let (mut check, next) = sysroot_judge(observed, remembered, now);
    if let Some(next) = next {
        if let Err(why) = write_sysroot_record(&file, &next) {
            // An observation this arrival could not remember is not an ok one.
            check.state = State::Degraded;
            check.detail = format!("{}; not remembered: {why}", check.detail);
        }
    }
    check
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn at(stamp: &str) -> chrono::DateTime<chrono::Utc> {
        parse_stamp(stamp).unwrap()
    }
    fn cid_of(bytes: &[u8]) -> String {
        eprfs_core::BlobCid::compute_bytes(bytes).to_string()
    }
    /// A sysroot of real files under `dir`, whose identity lines name them by content in the
    /// probe's own grammar.
    struct Host {
        dir: tempfile::TempDir,
    }
    impl Host {
        fn new() -> Self {
            let host = Self {
                dir: tempfile::tempdir().unwrap(),
            };
            host.put("crt1.o", b"crt1 v1");
            host.put("libc.so.6", b"libc 2.39");
            host.put("gcc", b"#!/bin/sh\n");
            host
        }
        fn path(&self, name: &str) -> PathBuf {
            fs::canonicalize(self.dir.path()).unwrap().join(name)
        }
        fn put(&self, name: &str, bytes: &[u8]) {
            fs::write(self.dir.path().join(name), bytes).unwrap();
        }
        fn record(&self) -> PathBuf {
            self.dir.path().join("state").join(SYSROOT_RECORD)
        }
        /// The observation a full probe would make now, with `os` as context.
        fn observe(&self, os: &str) -> SysrootObserved {
            let line = |label: String, name: &str| {
                let file = self.path(name);
                format!(
                    "{label}\t{}\t{}",
                    file.display(),
                    cid_of(&fs::read(&file).unwrap())
                )
            };
            let mut lines = vec![
                line("link:crt1.o".into(), "crt1.o"),
                line(
                    format!("link:libc.so>{}", self.path("libc.so.6").display()),
                    "libc.so.6",
                ),
                line("tool:link-driver".into(), "gcc"),
            ];
            lines.sort();
            SysrootObserved {
                lines,
                os: Some(os.into()),
                glibc: Some("2.39".into()),
            }
        }
        fn arrive(&self, os: &str, now: &str) -> Check {
            let observed = self.observe(os);
            sysroot_check(&|_| Ok(observed.clone()), Ok(self.record()), true, at(now))
        }
        /// A status read; its probe panics, proving a read never runs one.
        fn status(&self, now: &str) -> Check {
            sysroot_check(
                &|_| panic!("a status read must not run the probe"),
                Ok(self.record()),
                false,
                at(now),
            )
        }
        fn saved(&self) -> SysrootRecord {
            read_sysroot_record(&self.record()).unwrap().unwrap()
        }
        /// Rewrite the saved record's JSON text with `edit`.
        fn tamper(&self, edit: impl Fn(Value) -> Value) {
            let value: Value = serde_json::from_slice(&fs::read(self.record()).unwrap()).unwrap();
            fs::write(self.record(), serde_json::to_vec(&edit(value)).unwrap()).unwrap();
        }
        fn assert_unreadable(&self, why: &str) {
            let check = self.status("2026-11-18T10:00:00Z");
            assert_eq!(check.state, State::Degraded, "{why}");
            assert!(
                check.detail.starts_with("unobserved (record unreadable: ")
                    && check.detail.contains(why),
                "{why}: {}",
                check.detail
            );
            assert!(check.cure.unwrap().starts_with("remove "));
        }
    }

    #[test]
    fn sysroot_status_runs_no_probe_and_writes_nothing_before_an_arrival() {
        let host = Host::new();
        let read = host.status("2026-11-18T10:00:00Z");
        assert_eq!(read.state, State::Unknown);
        assert!(read
            .detail
            .starts_with("not yet observed; an arrival records it ("));
        assert!(read.detail.contains("same-path gcc rebuild"));
        assert_eq!(read.cure.as_deref(), Some("epr device arrive"));
        assert!(
            !host.dir.path().join("state").exists(),
            "a status read must not write"
        );
    }

    #[test]
    fn sysroot_arrival_records_then_status_reads_the_record() {
        let host = Host::new();
        let arrival = host.arrive("almalinux 10.2", "2026-11-18T10:00:00Z");
        assert_eq!(arrival.state, State::Ok);
        let record = host.saved();
        assert_eq!(record.current.first_seen, "2026-11-18T10:00:00Z");
        assert_eq!(record.current.lines, host.observe("x").lines);
        assert!(record.history.is_empty());
        let before = fs::read(host.record()).unwrap();
        let read = host.status("2026-11-19T10:00:00Z");
        assert_eq!(read.to_string(), arrival.to_string());
        assert!(arrival
            .to_string()
            .ends_with("(almalinux 10.2, glibc 2.39)"));
        // An unchanged arrival leaves the record byte-identical.
        host.arrive("almalinux 10.2", "2026-11-20T10:00:00Z");
        assert_eq!(fs::read(host.record()).unwrap(), before);
    }

    #[test]
    fn sysroot_status_sees_a_changed_input_without_a_probe_and_an_arrival_records_it() {
        let host = Host::new();
        host.arrive("almalinux 10.1", "2026-11-01T00:00:00Z");
        host.put("libc.so.6", b"libc 2.39-145");
        let read = host.status("2026-11-18T10:00:00Z");
        assert_eq!(read.state, State::Degraded);
        assert!(
            read.detail.starts_with(
                "changed after 2026-11-01T00:00:00Z: 1 of 3 recorded inputs differ (link:libc.so>"
            ),
            "{}",
            read.detail
        );
        assert!(read
            .cure
            .unwrap()
            .starts_with("epr device arrive records the new identity"));
        assert!(
            host.saved().history.is_empty(),
            "a status read must not write"
        );
        let arrival = host.arrive("almalinux 10.2", "2026-11-18T10:00:00Z");
        let old = host.saved().history[0].clone();
        let now = host.saved().current;
        assert_eq!(
            arrival.to_string(),
            format!(
                "degraded sysroot: changed since 2026-11-18T10:00:00Z — was {}, now {}; \
                 artifact keys and pool build caches made before are for the previous system \
                 libraries — none needed for correctness; expect cold builds and key misses; \
                 record the new image in the journal; this line settles at the first arrival \
                 7 days after the change (a status read alone does not store it); \
                 {SYSROOT_BLIND}",
                describe_sysroot(&old),
                describe_sysroot(&now)
            )
        );
        assert!(old.lines.is_empty() && !now.lines.is_empty());
        assert_eq!(
            host.status("2026-11-18T11:00:00Z").to_string(),
            arrival.to_string()
        );
    }

    /// The notice is `[change, change + 7 days)`; an arrival past it stores `settledAt`, so a
    /// clock stepped back from day 8 to day 6 does not revive it. A clock behind the change, and
    /// a second change inside the notice, keep the first identity in history.
    #[test]
    fn sysroot_notice_window_boundary_rollback_and_second_change() {
        let host = Host::new();
        host.arrive("v1", "2026-11-01T00:00:00Z");
        host.put("crt1.o", b"crt1 v2");
        host.arrive("v2", "2026-11-10T00:00:00Z");
        for (now, state) in [
            ("2026-11-10T00:00:00Z", State::Degraded),
            ("2026-11-16T23:59:59Z", State::Degraded),
            ("2026-11-17T00:00:00Z", State::Ok),
            ("2026-11-09T23:59:59Z", State::Ok),
            ("2020-01-01T00:00:00Z", State::Ok),
        ] {
            assert_eq!(host.status(now).state, state, "{now}");
        }
        // Status alone never writes, so day 6 is still inside the notice until an arrival has
        // seen it over.
        assert_eq!(host.status("2026-11-16T00:00:00Z").state, State::Degraded);
        assert!(host.saved().current.settled_at.is_none());
        let day8 = host.arrive("v2", "2026-11-18T00:00:00Z");
        assert_eq!(day8.state, State::Ok);
        assert_eq!(
            host.saved().current.settled_at.as_deref(),
            Some("2026-11-18T00:00:00Z")
        );
        assert_eq!(host.status("2026-11-16T00:00:00Z").state, State::Ok);
        assert_eq!(host.arrive("v2", "2026-11-16T00:00:00Z").state, State::Ok);

        host.put("crt1.o", b"crt1 v3");
        host.arrive("v3", "2026-11-19T00:00:00Z");
        let record = host.saved();
        assert!(record.current.settled_at.is_none());
        let os: Vec<_> = record
            .history
            .iter()
            .map(|s| s.os.clone().unwrap())
            .collect();
        assert_eq!(os, ["v2", "v1"]);
        assert!(record.history.iter().all(|s| s.settled_at.is_none()));
        for n in 4..=12 {
            host.put("crt1.o", format!("crt1 v{n}").as_bytes());
            host.arrive(&format!("v{n}"), "2026-11-20T00:00:00Z");
        }
        assert_eq!(host.saved().history.len(), SYSROOT_HISTORY);
    }

    /// The record is read without following a link, never past its size bound, never from a
    /// non-regular file, and never with an unparseable time; each is said, never a refusal.
    #[test]
    fn sysroot_record_reads_are_bounded_and_refuse_links_and_bad_times() {
        let host = Host::new();
        host.arrive("v1", "2026-11-01T00:00:00Z");
        let record = host.record();
        let good = fs::read(&record).unwrap();
        let elsewhere = host.dir.path().join("elsewhere.json");
        fs::write(&elsewhere, &good).unwrap();
        fs::remove_file(&record).unwrap();
        std::os::unix::fs::symlink(&elsewhere, &record).unwrap();
        host.assert_unreadable("is a symbolic link");
        fs::remove_file(&record).unwrap();
        fs::write(&record, vec![b' '; SYSROOT_RECORD_LIMIT as usize + 1]).unwrap();
        host.assert_unreadable("exceeds");
        fs::remove_file(&record).unwrap();
        fs::create_dir(&record).unwrap();
        host.assert_unreadable("not a regular file");
        fs::remove_dir(&record).unwrap();
        let text = String::from_utf8(good.clone()).unwrap();
        fs::write(&record, text.replace("2026-11-01T00:00:00Z", "yesterday")).unwrap();
        host.assert_unreadable("invalid time");
        // An arrival does not overwrite a record it cannot read.
        let check = host.arrive("v1", "2026-11-18T10:00:00Z");
        assert!(check.detail.starts_with("unobserved (record unreadable"));
        assert!(fs::read_to_string(&record).unwrap().contains("yesterday"));
    }

    /// A record is untrusted data: its lines must match the probe's grammar, hash to its
    /// identity and name at least the driver and one input; an unknown shape is unreadable.
    /// Nothing outside the grammar is ever opened.
    #[test]
    fn sysroot_record_lines_are_validated_before_any_path_is_used() {
        let host = Host::new();
        host.arrive("v1", "2026-11-01T00:00:00Z");
        let good = fs::read(host.record()).unwrap();
        let restore = || fs::write(host.record(), &good).unwrap();
        host.tamper(|mut v| {
            v["current"]["lines"][0] = json!(format!("link:secret\t/etc/shadow\t{}", cid_of(b"x")));
            v
        });
        host.assert_unreadable("is not a sysroot identity line");
        restore();
        host.tamper(|mut v| {
            v["current"]["lines"] = json!([]);
            v
        });
        host.assert_unreadable(
            "the driver line, link:crt1.o or libc.so's libc.so.6 referent is missing",
        );
        restore();
        host.tamper(|mut v| {
            v["current"]["cid"] = json!(cid_of(b"another identity"));
            v
        });
        host.assert_unreadable("is not the CID of its lines");
        restore();
        // A line rewritten to another real file, with its own CID, still breaks the identity.
        host.tamper(|mut v| {
            let other = host.path("gcc");
            v["current"]["lines"][0] = json!(format!(
                "link:crt1.o\t{}\t{}",
                other.display(),
                cid_of(&fs::read(&other).unwrap())
            ));
            v
        });
        host.assert_unreadable("is not the CID of its lines");
        restore();
        // The format of an earlier, unshipped revision is an unknown shape.
        host.tamper(|mut v| {
            v["previous"] = Value::Null;
            v
        });
        host.assert_unreadable("unknown field");
        restore();
        host.tamper(|mut v| {
            v["history"] = json!([{"cid": cid_of(b"h"), "os": null, "glibc": null,
                "firstSeen": "2026-10-01T00:00:00Z", "lines": v["current"]["lines"].clone()}]);
            v
        });
        host.assert_unreadable("earlier identity carries lines");
        restore();
        assert_eq!(host.status("2026-11-18T10:00:00Z").state, State::Ok);
    }

    /// The record's own directory may not be a symbolic link: neither the record nor the lock is
    /// read or created through one.
    #[test]
    fn sysroot_record_directory_that_is_a_link_is_refused() {
        let host = Host::new();
        let elsewhere = host.dir.path().join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        let dir = host.record().parent().unwrap().to_path_buf();
        fs::create_dir_all(dir.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&elsewhere, &dir).unwrap();
        host.assert_unreadable("is a symbolic link");
        let arrival = host.arrive("v1", "2026-11-18T10:00:00Z");
        assert!(
            arrival.detail.contains("is a symbolic link"),
            "{}",
            arrival.detail
        );
        assert_eq!(fs::read_dir(&elsewhere).unwrap().count(), 0);
    }

    /// The writer never emits what the reader refuses: oversized history is dropped oldest first
    /// until the record fits.
    #[test]
    fn sysroot_record_writer_stays_within_the_reader_bound() {
        let host = Host::new();
        host.arrive("v1", "2026-11-01T00:00:00Z");
        let mut record = host.saved();
        record.history = (0..SYSROOT_HISTORY)
            .map(|n| SysrootSeen {
                cid: cid_of(format!("h{n}").as_bytes()),
                os: Some(format!("{n}{}", "x".repeat(12 * 1024))),
                glibc: None,
                first_seen: "2026-10-01T00:00:00Z".into(),
                settled_at: None,
                lines: Vec::new(),
            })
            .collect();
        write_sysroot_record(&host.record(), &record).unwrap();
        let bytes = fs::metadata(host.record()).unwrap().len();
        assert!(bytes <= SYSROOT_RECORD_LIMIT, "{bytes}");
        let saved = host.saved();
        assert!(saved.history.len() < SYSROOT_HISTORY);
        assert!(saved.history[0].os.as_deref().unwrap().starts_with('0'));
    }

    /// Arrivals serialise on the lock beside the record; a held lock makes another arrival say
    /// so instead of racing, and a FIFO at the lock's name is refused without blocking.
    #[test]
    fn sysroot_arrivals_serialise_on_the_record_lock() {
        let host = Host::new();
        let held = lock_sysroot(&host.record(), std::time::Duration::ZERO).unwrap();
        let waited = lock_sysroot(&host.record(), std::time::Duration::from_millis(50));
        assert!(waited.unwrap_err().starts_with("another arrival holds"));
        drop(held);
        // The guard unlocks on drop, so a test child forked meanwhile, whose copy of the
        // descriptor lasts until its exec, does not hold the lock: the next arrival needs no wait.
        lock_sysroot(&host.record(), std::time::Duration::ZERO).unwrap();
        let lock = host.record().parent().unwrap().join(SYSROOT_LOCK);
        fs::remove_file(&lock).unwrap();
        assert!(std::process::Command::new("mkfifo")
            .arg(&lock)
            .status()
            .unwrap()
            .success());
        let started = std::time::Instant::now();
        let fifo = lock_sysroot(&host.record(), std::time::Duration::ZERO);
        assert!(fifo.unwrap_err().contains("not a regular file"));
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }

    /// An arrival whose observation fails or overruns says so, writes nothing, and releases
    /// the lock; with no record place the line is unobserved.
    #[test]
    fn sysroot_unobservable_arrival_degrades_and_never_writes() {
        let host = Host::new();
        let check = sysroot_check(
            &|deadline| {
                assert!(deadline > std::time::Instant::now());
                Err(probe_exceeded())
            },
            Ok(host.record()),
            true,
            at("2026-11-18T10:00:00Z"),
        );
        assert_eq!(
            check.to_string(),
            "degraded sysroot: unobserved (probe exceeded 10s)"
        );
        assert!(!host.record().exists());
        assert!(lock_sysroot(&host.record(), std::time::Duration::ZERO).is_ok());
        let nowhere = sysroot_check(
            &|_| panic!("no record place: nothing to compare"),
            Err("no state directory".into()),
            false,
            at("2026-11-18T10:00:00Z"),
        );
        assert_eq!(nowhere.state, State::Degraded);
        assert!(nowhere.detail.contains("no state directory"));
    }

    /// os-release is context only: a FIFO is not waited on, an oversized file is not read, and
    /// either omits the context; the next candidate still answers.
    #[test]
    fn sysroot_os_release_is_read_within_bounds_or_omitted() {
        let dir = tempfile::tempdir().unwrap();
        let fifo = dir.path().join("fifo");
        assert!(std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success());
        let large = dir.path().join("large");
        let mut text = b"ID=big\nVERSION_ID=1\n".to_vec();
        text.resize(OS_RELEASE_LIMIT as usize + 1, b'#');
        fs::write(&large, text).unwrap();
        let good = dir.path().join("good");
        fs::write(&good, "ID=rhel\nVERSION_ID=10.0\n").unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let (fifo_c, large_c, good_c) = (fifo.clone(), large.clone(), good.clone());
        std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let _ = tx.send((
                os_context(&[&fifo_c], deadline),
                os_context(&[&large_c], deadline),
                os_context(&[&fifo_c, &large_c, &good_c], deadline),
            ));
        });
        let (from_fifo, from_large, from_good) = rx
            .recv_timeout(std::time::Duration::from_secs(3))
            .expect("os-release read waited on a FIFO");
        assert_eq!(from_fifo, None);
        assert_eq!(from_large, None);
        assert_eq!(from_good.as_deref(), Some("rhel 10.0"));
    }

    #[test]
    fn sysroot_context_reads_os_release_quoting() {
        assert_eq!(
            os_release(
                "NAME=\"AlmaLinux\"\nID=\"almalinux\"\nID_LIKE=\"rhel centos fedora\"\nVERSION_ID=\"10.2\"\n"
            ),
            Some("almalinux 10.2".into())
        );
        assert_eq!(
            os_release("ID=rhel\nVERSION_ID='10.0'\n"),
            Some("rhel 10.0".into())
        );
        assert_eq!(os_release("ID=arch\n"), Some("arch".into()));
        assert_eq!(os_release("NAME=x\n"), None);
    }

    /// Measures the two paths on this host: `EPR_MEASURE_SYSROOT=1` prints the wall time of an
    /// arrival's full probe and of a status read's bounded re-check; otherwise it does nothing.
    #[test]
    fn sysroot_probe_cost_on_this_host() {
        if std::env::var_os("EPR_MEASURE_SYSROOT").is_none() {
            return;
        }
        let env = |key: &str| std::env::var(key).ok();
        let started = std::time::Instant::now();
        let deadline = started + std::time::Duration::from_secs(SYSROOT_PROBE_SECONDS);
        let observed = observe_sysroot(&env, deadline).unwrap();
        let full = started.elapsed();
        let started = std::time::Instant::now();
        let drifted =
            crate::host_link::sysroot_drift(&observed.lines, started + SYSROOT_RECHECK).unwrap();
        let cheap = started.elapsed();
        eprintln!(
            "sysroot probe on this host: full {full:?} ({} lines, cid {}), bounded re-check \
             {cheap:?} ({} drifted)",
            observed.lines.len(),
            observed.cid(),
            drifted.len()
        );
        assert!(drifted.is_empty());
    }
}
