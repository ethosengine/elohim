//! Filesystem mechanics shared by the two existing JSONL stores, not a domain model.
//!
//! Cooperating processes lock the log inode itself. Never replace/unlink a live log or
//! reopen it while holding a transaction. Drop explicitly unlocks before closing; otherwise
//! a descriptor inherited by a concurrently spawned child can extend the lock past our scope.
//! After a crash the OS releases the lock when the last inherited handle closes.
//! Each accepted append has completed `sync_all`; directory creation and multi-record
//! transactions are NOT power-loss atomic. A failed sync is an uncertain write, not success.
//! Interrupted/tampered bytes remain in place; readers refuse any invalid record and writers
//! refuse an invalid tail. This layer never guesses a repair or silently truncates evidence.
//!
//! Flow-log readers hold a shared lock only long enough to capture the file length, then read
//! exactly that immutable prefix and decode after unlocking. Cooperating writers only append, so
//! bytes below that length cannot change and an append beyond it is deferred to the next read.
//! Flow writers serialize first, then hold the exclusive lock only while they re-read and verify
//! the final non-empty record (including its newline and CID), append one whole line, and sync it.
//! The cost of admitting an append therefore follows the tail record, not the log history.
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
    let dir = root.join(".eprfs").join("status");
    fs::create_dir_all(&dir)?;
    let path = dir.join(name);
    // Atomic create-if-absent. In particular, no exists() / truncating create race.
    match OpenOptions::new().create_new(true).write(true).open(&path) {
        Ok(_) => {}
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

    pub(crate) fn append(&mut self, mut line: String) -> Result<()> {
        // Validation is transaction-local: no cooperating writer can change these bytes
        // while we hold the exclusive lock. A failed/uncertain write invalidates it.
        self.validated.set(false);
        line.push('\n');
        self.file.write_all(line.as_bytes())?;
        self.file.sync_all()?;
        self.validated.set(true);
        Ok(())
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
}
