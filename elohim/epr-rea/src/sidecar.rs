//! Filesystem mechanics shared by the two existing JSONL stores, not a domain model.
//!
//! Cooperating processes lock the log inode itself. Never replace/unlink a live log or
//! reopen it while holding a transaction. Drop explicitly unlocks before closing; otherwise
//! a descriptor inherited by a concurrently spawned child can extend the lock past our scope.
//! After a crash the OS releases the lock when the last inherited handle closes.
//! Each accepted append has completed `sync_all`; directory creation and multi-record
//! transactions are NOT power-loss atomic. A failed sync is an uncertain write, not success.
//! Interrupted/tampered bytes remain in place and refuse both reads and further appends;
//! this layer never guesses a repair or silently truncates evidence.
//! Uses standard-library file locks (Rust 1.89+); no lock-file registry or dependency.

use std::cell::Cell;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::error::{FabricError, Result};

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
}
