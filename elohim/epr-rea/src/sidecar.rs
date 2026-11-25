//! Filesystem mechanics shared by the two existing JSONL stores, not a domain model.
//!
//! Cooperating processes lock the log inode itself. Never replace/unlink a live log or
//! reopen it while holding a transaction. Drop explicitly unlocks before closing; otherwise
//! a descriptor inherited by a concurrently spawned child can extend the lock past our scope.
//! After a crash the OS releases the lock when the last inherited handle closes.
//! Each accepted append or batch has completed `sync_all`; multi-record transactions are NOT
//! power-loss atomic. First creation syncs each new directory entry and the directory holding the
//! new log. A failed sync is an uncertain write, not success.
//! Interrupted/tampered bytes remain in place; readers refuse any invalid record and writers
//! refuse an invalid tail. Writers do not notice corruption before a healthy tail; full readers
//! still refuse it. This layer never guesses a repair or silently truncates evidence.
//!
//! Flow-log readers hold a shared lock only long enough to capture the file length, then read
//! exactly that immutable prefix and decode after unlocking. Cooperating writers only append, so
//! bytes below that length cannot change and an append beyond it is deferred to the next read.
//! Flow writers serialize first, then hold the exclusive lock only while they re-read and verify
//! the final non-empty record (including its newline and CID), append one whole batch, and sync it
//! once. The cost of admitting an append therefore follows the tail record, not the log history.
//! `SidecarFlowTransaction` separately retains the feedback coordination lock: `feedback_seen`
//! remains a snapshot of evidence that no feedback writer can replace before reference admission,
//! without making readers or unrelated flow writers wait for a whole-log decode.
//! Uses standard-library file locks (Rust 1.89+); no lock-file registry or dependency.

use std::cell::Cell;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::error::{FabricError, Result};

const TAIL_WINDOW_BYTES: u64 = 8 * 1024;

pub(crate) fn open_log(root: &Path, name: &str) -> Result<PathBuf> {
    open_log_with_directory_sync(root, name, sync_directory)
}

fn sync_directory(path: &Path) -> std::io::Result<()> {
    File::open(path)?.sync_all()
}

fn open_log_with_directory_sync(
    root: &Path,
    name: &str,
    mut sync_directory: impl FnMut(&Path) -> std::io::Result<()>,
) -> Result<PathBuf> {
    let dir = root.join(".eprfs").join("status");
    let mut missing = Vec::new();
    let mut ancestor = dir.as_path();
    while !ancestor.exists() {
        missing.push(ancestor.to_path_buf());
        ancestor = ancestor.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "sidecar directory has no existing ancestor",
            )
        })?;
    }
    fs::create_dir_all(&dir)?;
    for created in missing.iter().rev() {
        let parent = created.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "created sidecar directory has no parent",
            )
        })?;
        sync_directory(parent)?;
    }
    let path = dir.join(name);
    // Atomic create-if-absent. In particular, no exists() / truncating create race.
    match OpenOptions::new().create_new(true).write(true).open(&path) {
        Ok(file) => {
            file.sync_all()?;
            sync_directory(&dir)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    Ok(path)
}

impl crate::store::SidecarFlowStore {
    /// Probe the flow log under `root` through a shared, read-only handle.
    ///
    /// Returns `false` when the log is absent. This method never creates the status directory or
    /// log, so health checks cannot turn an empty checkout into a flow store.
    pub fn probe(root: &Path) -> Result<bool> {
        let path = root.join(".eprfs").join("status").join("flows.jsonl");
        let contents = match flow_contents(&path) {
            Ok(contents) => contents,
            Err(FabricError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(false)
            }
            Err(error) => return Err(error),
        };
        if !contents.is_empty() && !contents.ends_with('\n') {
            return Err(FabricError::Decode(
                "sidecar has an unterminated record; bytes preserved, repair requires review"
                    .into(),
            ));
        }
        Ok(true)
    }
}

/// Capture one append-only flow-log prefix without retaining a lock during UTF-8 or JSON decode.
pub(crate) fn flow_contents(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    file.lock_shared()?;
    let length = file.metadata()?.len();
    file.unlock()?;

    let capacity = usize::try_from(length)
        .map_err(|_| FabricError::Decode("flow log is too large for this reader".into()))?;
    let mut bytes = Vec::with_capacity(capacity);
    let read = (&file).take(length).read_to_end(&mut bytes)?;
    if read as u64 != length {
        return Err(FabricError::Decode(
            "flow log shrank while reading an append-only prefix".into(),
        ));
    }
    String::from_utf8(bytes).map_err(|error| FabricError::Decode(error.to_string()))
}

#[derive(Debug)]
pub(crate) struct LockedLog {
    file: File,
    validated: Cell<bool>,
}

impl Drop for LockedLog {
    fn drop(&mut self) {
        // Closing this descriptor alone is insufficient while a forked child still holds
        // the same open file description. Explicit unlock ends OUR transaction now.
        // Drop cannot return an unlock error; File's close remains the OS fallback.
        let _ = self.file.unlock();
    }
}

impl LockedLog {
    pub(crate) fn contents_bounded(&self, max_bytes: u64) -> Result<String> {
        if self.file.metadata()?.len() > max_bytes {
            return Err(FabricError::InvalidFeedback(
                "feedback store exceeds retention limit".into(),
            ));
        }
        self.contents()
    }
    pub(crate) fn exclusive(path: &Path) -> Result<Self> {
        let file = OpenOptions::new().read(true).append(true).open(path)?;
        file.lock()?;
        Ok(Self {
            file,
            validated: Cell::new(false),
        })
    }

    pub(crate) fn shared(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        file.lock_shared()?;
        Ok(Self {
            file,
            validated: Cell::new(false),
        })
    }

    pub(crate) fn try_shared(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        file.try_lock_shared().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => {
                std::io::Error::from(std::io::ErrorKind::WouldBlock)
            }
            std::fs::TryLockError::Error(error) => error,
        })?;
        Ok(Self {
            file,
            validated: Cell::new(false),
        })
    }

    pub(crate) fn contents(&self) -> Result<String> {
        self.validated.set(false);
        let mut file = &self.file;
        file.seek(SeekFrom::Start(0))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        if !contents.is_empty() && !contents.ends_with('\n') {
            return Err(FabricError::Decode(
                "sidecar has an unterminated record; bytes preserved, repair requires review"
                    .into(),
            ));
        }
        Ok(contents)
    }

    pub(crate) fn validated(&self) -> bool {
        self.validated.get()
    }

    pub(crate) fn mark_validated(&self) {
        self.validated.set(true);
    }

    pub(crate) fn append(&mut self, line: String) -> Result<()> {
        self.append_batch([line])
    }

    pub(crate) fn append_batch(&mut self, lines: impl IntoIterator<Item = String>) -> Result<()> {
        // Validation is transaction-local: no cooperating writer can change these bytes
        // while we hold the exclusive lock. A failed/uncertain write invalidates it.
        self.validated.set(false);
        for mut line in lines {
            line.push('\n');
            self.file.write_all(line.as_bytes())?;
        }
        self.file.sync_all()?;
        self.validated.set(true);
        Ok(())
    }

    /// Prepare an empty log for its first append while this writer owns the exclusive lock.
    ///
    /// A process that lost the create race can reach this point before the creator has synced the
    /// new names. Sync the empty file and every directory entry the creator may have made before
    /// any bytes become visible. Non-empty logs were already admitted by an earlier synced write.
    pub(crate) fn prepare_append(&self, root: &Path, path: &Path) -> Result<Option<String>> {
        self.prepare_append_with_directory_sync(root, path, sync_directory)
    }

    fn prepare_append_with_directory_sync(
        &self,
        root: &Path,
        path: &Path,
        mut sync_directory: impl FnMut(&Path) -> std::io::Result<()>,
    ) -> Result<Option<String>> {
        let tail = self.tail_record()?;
        if tail.is_some() {
            return Ok(tail);
        }
        self.file.sync_all()?;
        for directory in [
            root.to_path_buf(),
            root.join(".eprfs"),
            root.join(".eprfs/status"),
        ] {
            sync_directory(&directory)?;
        }
        let expected = root
            .join(".eprfs/status")
            .join(path.file_name().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "sidecar log path has no file name",
                )
            })?);
        if path != expected {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "sidecar log is outside its root status directory",
            )
            .into());
        }
        Ok(None)
    }

    /// The final non-empty, newline-terminated record under this handle's exclusive lock.
    pub(crate) fn tail_record(&self) -> Result<Option<String>> {
        let length = self.file.metadata()?.len();
        if length == 0 {
            return Ok(None);
        }

        let mut final_byte = [0_u8; 1];
        let mut file = &self.file;
        file.seek(SeekFrom::End(-1))?;
        file.read_exact(&mut final_byte)?;
        if final_byte[0] != b'\n' {
            return Err(FabricError::Decode(
                "sidecar has an unterminated record; bytes preserved, repair requires review"
                    .into(),
            ));
        }

        let mut window = TAIL_WINDOW_BYTES.min(length);
        loop {
            let start = length - window;
            file.seek(SeekFrom::Start(start))?;
            let mut bytes = vec![0_u8; window as usize];
            file.read_exact(&mut bytes)?;
            let skip_first = usize::from(start > 0);
            let lines: Vec<_> = bytes.split(|byte| *byte == b'\n').collect();
            if let Some(line) = lines[skip_first..]
                .iter()
                .rev()
                .copied()
                .find(|line| !line.iter().all(u8::is_ascii_whitespace))
            {
                return String::from_utf8(line.to_vec())
                    .map(Some)
                    .map_err(|error| FabricError::Decode(error.to_string()));
            }
            if start == 0 {
                return Ok(None);
            }
            window = window.saturating_mul(2).min(length);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropping_transaction_unlocks_even_if_an_inherited_handle_remains_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = open_log(dir.path(), "actors.jsonl").unwrap();
        let transaction = LockedLog::exclusive(&path).unwrap();
        // A duplicate holds the same open file description, as a child temporarily does
        // between fork and exec. Closing only our handle must not extend the transaction.
        let inherited = transaction.file.try_clone().unwrap();
        drop(transaction);
        let reader = LockedLog::try_shared(&path)
            .expect("transaction ended even while a duplicate descriptor remains alive");
        drop(reader);
        drop(inherited);
    }

    #[test]
    fn flow_probe_reads_without_creating() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!crate::store::SidecarFlowStore::probe(dir.path()).unwrap());
        assert!(!dir.path().join(".eprfs").exists());

        let path = open_log(dir.path(), "flows.jsonl").unwrap();
        assert!(crate::store::SidecarFlowStore::probe(dir.path()).unwrap());
        assert_eq!(std::fs::read(path).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn first_log_creation_syncs_new_directory_entries_and_log_directory() {
        let dir = tempfile::tempdir().unwrap();
        let mut synced = Vec::new();
        let path = open_log_with_directory_sync(dir.path(), "flows.jsonl", |path| {
            synced.push(path.to_path_buf());
            Ok(())
        })
        .unwrap();

        assert_eq!(
            synced,
            vec![
                dir.path().to_path_buf(),
                dir.path().join(".eprfs"),
                dir.path().join(".eprfs/status"),
            ]
        );
        assert!(path.is_file());
    }

    #[test]
    fn empty_log_writer_syncs_the_file_and_directory_chain_before_first_append() {
        let dir = tempfile::tempdir().unwrap();
        let path = open_log(dir.path(), "flows.jsonl").unwrap();
        let log = LockedLog::exclusive(&path).unwrap();
        let mut synced = Vec::new();

        assert_eq!(
            log.prepare_append_with_directory_sync(dir.path(), &path, |directory| {
                synced.push(directory.to_path_buf());
                Ok(())
            })
            .unwrap(),
            None
        );
        assert_eq!(
            synced,
            vec![
                dir.path().to_path_buf(),
                dir.path().join(".eprfs"),
                dir.path().join(".eprfs/status"),
            ]
        );
    }

    #[test]
    fn an_empty_retry_batch_syncs_before_it_is_acknowledged() {
        let dir = tempfile::tempdir().unwrap();
        let path = open_log(dir.path(), "flows.jsonl").unwrap();
        let mut log = LockedLog::exclusive(&path).unwrap();

        log.append_batch(std::iter::empty()).unwrap();

        assert!(
            log.validated(),
            "validated is set only after sync_all succeeds"
        );
    }
}
