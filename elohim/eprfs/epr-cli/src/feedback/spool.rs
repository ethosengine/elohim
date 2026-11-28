//! Brit's capture-gap spool: one small file per session in which the recorder could not be
//! reached or refused. The drain turns each valid file into one record through the same store
//! path capture uses, then removes it; the device status reads the same files without draining.

use super::{claim, Result};
use elohim_epr_rea::{
    feedback::{Feedback, Occurrence, Outcome, Source},
    feedback_store::FeedbackStore,
};
use serde_json::{json, Value};
use std::{
    ffi::OsString,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

/// The largest spool file brit writes.
pub(crate) const FILE_BYTES: u64 = 1024;
/// The most `*.json` entries one read considers (brit never keeps more); the rest wait for the
/// next drain.
pub(crate) const BATCH: usize = 256;
/// The causes brit names, each paired with the claim `epr/recorder-unavailable:<cause>`.
pub(crate) const CAUSES: [&str; 4] = ["not-found", "timeout", "refused", "spawn-failed"];

/// `$XDG_STATE_HOME/brit/feedback-spool` when that variable is an absolute path, else
/// `$HOME/.local/state/brit/feedback-spool` when HOME is absolute, else no directory.
/// The one resolver the drain and the status share.
pub(crate) fn dir(env: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    if let Some(state) = env("XDG_STATE_HOME").map(PathBuf::from) {
        if state.is_absolute() {
            return Some(state.join("brit/feedback-spool"));
        }
    }
    env("HOME")
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
        .map(|home| home.join(".local/state/brit/feedback-spool"))
}

/// One valid spool file. Open fields (`stderr`, `recorder`) are never carried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Gap {
    pub(crate) path: PathBuf,
    pub(crate) claim: String,
    pub(crate) cause: String,
    pub(crate) at: String,
    pub(crate) session: String,
    pub(crate) emitter: Option<String>,
    instant: chrono::DateTime<chrono::Utc>,
    /// The `(device, inode)` of the file that was read, so the drain removes only that file.
    identity: (u64, u64),
}

/// What a read of the spool found: valid gaps, oldest first (by `at`, then file name), the
/// counts of unreadable notes and other entries, and whether more than [`BATCH`] notes waited.
#[derive(Debug, Default)]
pub(crate) struct Scan {
    pub(crate) gaps: Vec<Gap>,
    pub(crate) skipped: usize,
    pub(crate) other: usize,
    pub(crate) bounded: bool,
}

impl Scan {
    /// How many entries were read: a count, or `256 or more` when the bound was reached.
    pub(crate) fn read(&self) -> Value {
        if self.bounded {
            json!(format!("{BATCH} or more"))
        } else {
            json!(self.gaps.len() + self.skipped)
        }
    }
}

/// Read at most [`BATCH`] `*.json` entries in `dir`. An absent directory is an empty spool. The
/// directory itself may be a symbolic link: it is the person's own configuration.
pub(crate) fn scan(dir: &Path) -> std::io::Result<Scan> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Scan::default()),
        Err(error) => return Err(error),
    };
    let mut scan = Scan::default();
    let mut read = 0;
    for entry in entries {
        let path = entry?.path();
        // Brit writes a note by renaming a temporary file it makes in this directory, named
        // `.tmp` and random letters, so such a name may exist for a moment; it is neither read
        // nor counted. Brit's lock lives beside the spool, not in it.
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(".tmp"))
        {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            scan.other += 1;
            continue;
        }
        if read == BATCH {
            scan.bounded = true;
            break;
        }
        read += 1;
        match read_bounded(&path).and_then(|(bytes, identity)| parse(&path, &bytes, identity)) {
            Some(gap) => scan.gaps.push(gap),
            None => scan.skipped += 1,
        }
    }
    scan.gaps
        .sort_by(|a, b| a.instant.cmp(&b.instant).then_with(|| a.path.cmp(&b.path)));
    Ok(scan)
}

/// Read one note without following a symbolic link and without blocking on a FIFO: the handle
/// must be a regular file of at most [`FILE_BYTES`]. Returns the bytes and the file's identity.
fn read_bounded(path: &Path) -> Option<(Vec<u8>, (u64, u64))> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > FILE_BYTES {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(FILE_BYTES + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= FILE_BYTES).then_some((bytes, (metadata.dev(), metadata.ino())))
}

/// Remove the note only when the path still names the file that was read. `Ok(false)` leaves
/// it in place: it was replaced (or became a link) after the read.
fn remove_if_unchanged(gap: &Gap) -> std::io::Result<bool> {
    match std::fs::symlink_metadata(&gap.path) {
        Ok(now) if now.is_file() && (now.dev(), now.ino()) == gap.identity => {}
        Ok(_) => return Ok(false),
        // Another drain removed it after holding the same record.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(error) => return Err(error),
    }
    match std::fs::remove_file(&gap.path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error),
    }
}

fn parse(path: &Path, bytes: &[u8], identity: (u64, u64)) -> Option<Gap> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    if value.get("v")?.as_u64()? != 1 {
        return None;
    }
    let cause = value.get("cause")?.as_str()?;
    if !CAUSES.contains(&cause) {
        return None;
    }
    let claim = value.get("claim")?.as_str()?;
    if claim != format!("epr/recorder-unavailable:{cause}") {
        return None;
    }
    let at = value.get("at")?.as_str()?;
    let instant = chrono::DateTime::parse_from_rfc3339(at).ok()?.to_utc();
    let session = value.get("session")?.as_str()?;
    if session.len() != 64
        || !session
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    let emitter = claim::valid_emitter(value.get("emitter").and_then(Value::as_str));
    Some(Gap {
        path: path.to_path_buf(),
        claim: claim.into(),
        cause: cause.into(),
        at: at.into(),
        session: session.into(),
        emitter: emitter.map(str::to_owned),
        instant,
        identity,
    })
}

/// The record one gap becomes. The emitter is part of the item (as on the claim path), so two
/// releases noting one session's gap are two records rather than conflicting evidence; equal
/// notes make equal records, so a second drain of the same gap returns the held address.
pub(crate) fn occurrence(gap: &Gap) -> Occurrence {
    Occurrence {
        source: Source {
            producer: "brit".into(),
            run: format!("spool-{}", gap.session),
            item: match &gap.emitter {
                Some(emitter) => format!("{} failed {emitter}", gap.claim),
                None => format!("{} failed", gap.claim),
            },
        },
        observed_at: gap.at.clone(),
        participant: None,
        subject: gap.claim.clone(),
        outcome: Outcome::Failed,
        count: 1,
        duration_ms: None,
        impact: None,
        message: None,
        detail: gap
            .emitter
            .as_ref()
            .map_or(Value::Null, |emitter| json!({ "emitter": emitter })),
    }
}

/// Append each valid gap and remove its file. A store refusal stops the drain with that error
/// and leaves the file in place. `skipped` counts unreadable notes and replaced or unremovable
/// files; `unremoved` lists each removal error with the note's path and the error's text.
pub(crate) fn drain(store: &FeedbackStore, dir: &Path) -> Result<Value> {
    drain_with_remove(store, dir, remove_if_unchanged)
}

fn drain_with_remove(
    store: &FeedbackStore,
    dir: &Path,
    mut remove: impl FnMut(&Gap) -> std::io::Result<bool>,
) -> Result<Value> {
    let scan =
        scan(dir).map_err(|error| format!("spool unreadable: {}: {error}", dir.display()))?;
    let mut drained = 0;
    let mut skipped = scan.skipped;
    let mut unremoved = Vec::new();
    for gap in &scan.gaps {
        store
            .append(Feedback::Occurrence(occurrence(gap)), false)
            .map_err(|error| format!("{}: {error}", gap.path.display()))?;
        // A removal failure is safe to retry: occurrence makes equal records for equal
        // notes, so the next drain returns the held address without recording it twice.
        match remove(gap) {
            Ok(true) => drained += 1,
            Ok(false) => skipped += 1,
            Err(error) => {
                skipped += 1;
                // A path is printed as text: one that is not UTF-8 cannot be a JSON string.
                unremoved.push(json!({"path": gap.path.display().to_string(),
                    "error": error.to_string()}));
            }
        }
    }
    Ok(
        json!({"drained": drained, "skipped": skipped, "other": scan.other,
        "unremoved": unremoved, "read": scan.read(), "dir": dir.display().to_string()}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gap(at: &str, session: &str) -> Vec<u8> {
        json!({"v":1,"claim":"epr/recorder-unavailable:timeout","cause":"timeout","at":at,
            "emitter":"unrecorded:0.3.6+exe.0123456789ab","session":session,
            "recorder":{"path":"/secret/epr","size":null,"mtime":null},"stderr":"SECRET"})
        .to_string()
        .into_bytes()
    }

    #[test]
    fn spool_directory_prefers_an_absolute_state_home() {
        let env = |state: Option<&'static str>, home: Option<&'static str>| {
            move |name: &str| match name {
                "XDG_STATE_HOME" => state.map(OsString::from),
                "HOME" => home.map(OsString::from),
                _ => None,
            }
        };
        assert_eq!(
            dir(&env(Some("/s"), Some("/h"))),
            Some(PathBuf::from("/s/brit/feedback-spool"))
        );
        assert_eq!(
            dir(&env(Some("relative"), Some("/h"))),
            Some(PathBuf::from("/h/.local/state/brit/feedback-spool"))
        );
        assert_eq!(dir(&env(None, None)), None);
        assert_eq!(dir(&env(None, Some("relative"))), None);
        assert_eq!(dir(&env(Some("relative"), Some("relative"))), None);
    }

    #[test]
    fn scan_counts_non_notes_but_not_writer_files() {
        let temp = tempfile::tempdir().unwrap();
        for name in ["note.txt", "unknown", ".tmpAb12Cd"] {
            std::fs::write(temp.path().join(name), b"untouched").unwrap();
        }
        let found = scan(temp.path()).unwrap();
        assert!(found.gaps.is_empty());
        assert_eq!(found.skipped, 0);
        assert_eq!(found.other, 2);
        assert_eq!(found.read(), json!(0));
        for name in ["note.txt", "unknown", ".tmpAb12Cd"] {
            assert_eq!(std::fs::read(temp.path().join(name)).unwrap(), b"untouched");
        }
    }

    #[test]
    fn drain_continues_after_a_removal_error_and_retries_safely() {
        let temp = tempfile::tempdir().unwrap();
        let spool = temp.path().join("spool");
        std::fs::create_dir(&spool).unwrap();
        for (name, at, session) in [
            ("old.json", "2026-11-27T09:00:00Z", "a".repeat(64)),
            ("new.json", "2026-11-27T10:00:00Z", "b".repeat(64)),
        ] {
            std::fs::write(spool.join(name), gap(at, &session)).unwrap();
        }
        let policy = serde_json::from_value(json!({
            "id":"fixture@1", "authority":"human:fixture", "producers":["brit"],
            "automaticCapture":true, "deliberateEvidence":true,
            "maxBytes":1000000, "maxRecordBytes":65536,
            "response":"report", "sharing":"separately-authorized"
        }))
        .unwrap();
        let store = FeedbackStore::new(temp.path(), policy).unwrap();
        let first = drain_with_remove(&store, &spool, |gap| {
            if gap.path == spool.join("old.json") {
                Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
            } else {
                remove_if_unchanged(gap)
            }
        })
        .unwrap();
        assert_eq!(first["drained"], 1);
        assert_eq!(first["skipped"], 1);
        assert_eq!(
            first["unremoved"],
            json!([{"path": spool.join("old.json"), "error":
                std::io::Error::from(std::io::ErrorKind::PermissionDenied).to_string()}])
        );
        assert!(spool.join("old.json").exists());
        assert!(!spool.join("new.json").exists());
        let held: Vec<_> = store
            .read()
            .unwrap()
            .into_iter()
            .map(|entry| entry.cid)
            .collect();
        assert_eq!(held.len(), 2);
        let second = drain(&store, &spool).unwrap();
        assert_eq!(second["drained"], 1);
        assert_eq!(second["skipped"], 0);
        assert_eq!(second["unremoved"], json!([]));
        let reread: Vec<_> = store
            .read()
            .unwrap()
            .into_iter()
            .map(|entry| entry.cid)
            .collect();
        assert_eq!(reread, held);
        assert!(!spool.join("old.json").exists());
    }

    #[test]
    fn a_spool_path_that_is_not_utf8_is_reported_as_text() {
        use std::os::unix::ffi::OsStrExt;
        let temp = tempfile::tempdir().unwrap();
        let spool = temp.path().join(std::ffi::OsStr::from_bytes(b"sp\xffool"));
        std::fs::create_dir(&spool).unwrap();
        std::fs::write(
            spool.join("note.json"),
            gap("2026-11-27T09:00:00Z", &"a".repeat(64)),
        )
        .unwrap();
        let policy = serde_json::from_value(json!({
            "id":"fixture@1", "authority":"human:fixture", "producers":["brit"],
            "automaticCapture":true, "deliberateEvidence":true,
            "maxBytes":1000000, "maxRecordBytes":65536,
            "response":"report", "sharing":"separately-authorized"
        }))
        .unwrap();
        let store = FeedbackStore::new(temp.path(), policy).unwrap();
        // A path that is not UTF-8 cannot be a JSON string as it stands; the report prints it.
        let report = drain_with_remove(&store, &spool, |_| {
            Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
        })
        .unwrap();
        assert_eq!(report["drained"], 0);
        assert_eq!(report["dir"], json!(spool.display().to_string()));
        assert_eq!(
            report["unremoved"][0]["path"],
            json!(spool.join("note.json").display().to_string())
        );
    }

    #[test]
    fn parse_accepts_brits_shape_and_refuses_the_rest() {
        let path = Path::new("x.json");
        let session = "a".repeat(64);
        let ok = parse(path, &gap("2026-11-27T10:00:00Z", &session), (0, 0)).expect("valid");
        assert_eq!(ok.claim, "epr/recorder-unavailable:timeout");
        assert_eq!(
            ok.emitter.as_deref(),
            Some("unrecorded:0.3.6+exe.0123456789ab")
        );
        assert!(parse(path, &gap("yesterday", &session), (0, 0)).is_none());
        assert!(parse(path, &gap("2026-11-27T10:00:00Z", &"A".repeat(64)), (0, 0)).is_none());
        assert!(parse(path, &gap("2026-11-27T10:00:00Z", "abc"), (0, 0)).is_none());
        let mismatched = json!({"v":1,"claim":"epr/recorder-unavailable:refused","cause":"timeout",
            "at":"2026-11-27T10:00:00Z","session":session});
        assert!(parse(path, mismatched.to_string().as_bytes(), (0, 0)).is_none());
        let version = json!({"v":2,"claim":"epr/recorder-unavailable:timeout","cause":"timeout",
            "at":"2026-11-27T10:00:00Z","session":session});
        assert!(parse(path, version.to_string().as_bytes(), (0, 0)).is_none());
        let spaced = json!({"v":1,"claim":"epr/recorder-unavailable:timeout","cause":"timeout",
            "at":"2026-11-27T10:00:00Z","session":session,"emitter":"brit 1"});
        assert_eq!(
            parse(path, spaced.to_string().as_bytes(), (0, 0))
                .unwrap()
                .emitter,
            None
        );
        assert_eq!(
            occurrence(&parse(path, spaced.to_string().as_bytes(), (0, 0)).unwrap()).detail,
            Value::Null
        );
    }

    #[test]
    fn notes_are_read_without_following_links_or_blocking_on_fifos() {
        let temp = tempfile::tempdir().unwrap();
        let session = "f".repeat(64);
        let outside = temp.path().join("outside.note");
        std::fs::write(&outside, gap("2026-11-27T10:00:00Z", &session)).unwrap();
        let spool = temp.path().join("spool");
        std::fs::create_dir(&spool).unwrap();
        std::os::unix::fs::symlink(&outside, spool.join("link.json")).unwrap();
        let fifo = std::process::Command::new("mkfifo")
            .arg(spool.join("pipe.json"))
            .status()
            .unwrap();
        assert!(fifo.success());
        let found = scan(&spool).unwrap();
        assert!(found.gaps.is_empty());
        assert_eq!(found.skipped, 2);
        // The spool directory itself may be a link (the person's own configuration).
        let real = temp.path().join("real");
        std::fs::create_dir(&real).unwrap();
        std::fs::write(real.join("n.json"), gap("2026-11-27T10:00:00Z", &session)).unwrap();
        std::os::unix::fs::symlink(&real, temp.path().join("linked")).unwrap();
        assert_eq!(scan(&temp.path().join("linked")).unwrap().gaps.len(), 1);
    }

    #[test]
    fn a_note_replaced_after_its_read_is_kept() {
        let temp = tempfile::tempdir().unwrap();
        let session = "f".repeat(64);
        let note = temp.path().join("n.json");
        std::fs::write(&note, gap("2026-11-27T10:00:00Z", &session)).unwrap();
        let read = scan(temp.path()).unwrap().gaps.remove(0);
        let other = temp.path().join("other.tmp");
        std::fs::write(&other, b"not the note that was read").unwrap();
        std::fs::rename(&other, &note).unwrap();
        assert!(!remove_if_unchanged(&read).unwrap());
        assert!(note.exists());
        let reread = scan(temp.path()).unwrap();
        assert_eq!(reread.skipped, 1);
        std::fs::write(&note, gap("2026-11-27T10:00:00Z", &session)).unwrap();
        let fresh = scan(temp.path()).unwrap().gaps.remove(0);
        assert!(remove_if_unchanged(&fresh).unwrap());
        assert!(!note.exists());
    }

    #[test]
    fn scan_reads_at_most_the_batch_and_says_when_more_wait() {
        let temp = tempfile::tempdir().unwrap();
        for n in 0..BATCH {
            std::fs::write(temp.path().join(format!("{n}.json")), b"{}").unwrap();
        }
        // The name brit's writer gives its temporary file: neither read nor counted.
        std::fs::write(temp.path().join(".tmpAb12Cd"), b"{}").unwrap();
        let exact = scan(temp.path()).unwrap();
        assert!(!exact.bounded);
        assert_eq!(exact.read(), json!(BATCH));
        std::fs::write(temp.path().join("extra.json"), b"{}").unwrap();
        let over = scan(temp.path()).unwrap();
        assert!(over.bounded);
        assert_eq!(over.skipped, BATCH);
        assert_eq!(over.read(), json!("256 or more"));
    }

    #[test]
    fn other_entries_do_not_count_toward_the_note_batch() {
        let temp = tempfile::tempdir().unwrap();
        for n in 0..300 {
            std::fs::write(temp.path().join(format!("n{n:03}.txt")), b"other").unwrap();
        }
        std::fs::write(
            temp.path().join("note.json"),
            gap("2026-11-27T10:00:00Z", &"a".repeat(64)),
        )
        .unwrap();
        let found = scan(temp.path()).unwrap();
        assert_eq!(found.other, 300);
        assert_eq!(found.skipped, 0);
        assert!(!found.bounded);
        assert_eq!(found.read(), json!(1));
    }

    #[test]
    fn scan_orders_by_time_then_name_and_counts_the_invalid() {
        let temp = tempfile::tempdir().unwrap();
        let session = "b".repeat(64);
        std::fs::write(
            temp.path().join("z.json"),
            gap("2026-11-27T09:00:00Z", &session),
        )
        .unwrap();
        std::fs::write(
            temp.path().join("a.json"),
            gap("2026-11-27T10:00:00Z", &session),
        )
        .unwrap();
        std::fs::write(temp.path().join("b.json"), b"not json").unwrap();
        std::fs::write(temp.path().join("note.txt"), b"ignored").unwrap();
        let found = scan(temp.path()).unwrap();
        assert_eq!(found.skipped, 1);
        assert_eq!(found.other, 1);
        let names: Vec<_> = found
            .gaps
            .iter()
            .map(|g| g.path.file_name().unwrap().to_str().unwrap().to_owned())
            .collect();
        assert_eq!(names, ["z.json", "a.json"]);
        assert!(scan(&temp.path().join("absent")).unwrap().gaps.is_empty());
    }
}
