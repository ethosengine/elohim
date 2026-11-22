//! A `flock` its owner releases by unlocking, not by closing.
//!
//! A flock belongs to the open file description, and every copy of the descriptor shares it: a
//! `dup`, and the copy a child forked by another thread holds until its `exec` closes it
//! (close-on-exec acts at exec, not at fork). A lock released by `close` lasts until the last copy
//! closes, so a process that forks can find its own finished lock still held. `LOCK_UN` releases
//! the lock on the description at once, whatever copies remain. [`Flock`] unlocks in `Drop`, so
//! every return path of its owner releases, then closes.

use std::fs::File;
use std::os::fd::{AsFd, BorrowedFd};

use rustix::fs::{flock, FlockOperation};

/// A held flock on its file. Dropping it unlocks, then closes.
#[derive(Debug)]
pub struct Flock {
    file: File,
}

impl Flock {
    /// Take the exclusive lock without waiting: the guard, or the file back when another open
    /// file description holds the lock.
    ///
    /// # Errors
    ///
    /// The lock call failed for a reason other than contention.
    pub fn try_exclusive(file: File) -> std::io::Result<Result<Flock, File>> {
        Self::try_take(file, FlockOperation::NonBlockingLockExclusive)
    }

    /// Take a shared lock without waiting: the guard, or the file back when another open file
    /// description holds the lock exclusively.
    ///
    /// # Errors
    ///
    /// The lock call failed for a reason other than contention.
    pub fn try_shared(file: File) -> std::io::Result<Result<Flock, File>> {
        Self::try_take(file, FlockOperation::NonBlockingLockShared)
    }

    fn try_take(file: File, operation: FlockOperation) -> std::io::Result<Result<Flock, File>> {
        match flock(&file, operation) {
            Ok(()) => Ok(Ok(Flock { file })),
            Err(errno) if errno == rustix::io::Errno::WOULDBLOCK => Ok(Err(file)),
            Err(errno) => Err(errno.into()),
        }
    }

    /// The locked file, for what its owner writes under the lock.
    pub fn file(&self) -> &File {
        &self.file
    }
}

impl AsFd for Flock {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}

impl Drop for Flock {
    fn drop(&mut self) {
        // A failed unlock leaves the close that follows to release it, as before this guard.
        let _ = flock(&self.file, FlockOperation::Unlock);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(path: &std::path::Path) -> File {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .unwrap()
    }

    /// The kernel fact the guard answers: a lock released by close alone stays held through a
    /// copy of its descriptor.
    #[test]
    fn a_closed_lock_stays_held_through_a_copy_of_its_descriptor() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.lock");
        let file = open(&path);
        flock(&file, FlockOperation::NonBlockingLockExclusive).unwrap();
        let copy = rustix::io::dup(&file).unwrap();
        drop(file);
        assert!(Flock::try_exclusive(open(&path)).unwrap().is_err());
        drop(copy);
        assert!(Flock::try_exclusive(open(&path)).unwrap().is_ok());
    }

    /// A dropped guard, exclusive or shared, frees the lock at once while a copy of its
    /// descriptor is still open.
    #[test]
    fn a_dropped_guard_frees_the_lock_while_a_copy_is_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.lock");
        for take in [Flock::try_exclusive, Flock::try_shared] {
            let held = take(open(&path)).unwrap().expect("free");
            let copy = rustix::io::dup(&held).unwrap();
            drop(held);
            assert!(Flock::try_exclusive(open(&path)).unwrap().is_ok());
            drop(copy);
        }
    }

    /// A live guard is contended: exclusive refuses every other take, shared refuses exclusive.
    #[test]
    fn a_live_guard_returns_the_contender_its_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.lock");
        let exclusive = Flock::try_exclusive(open(&path)).unwrap().expect("free");
        assert!(Flock::try_exclusive(open(&path)).unwrap().is_err());
        assert!(Flock::try_shared(open(&path)).unwrap().is_err());
        drop(exclusive);
        let shared = Flock::try_shared(open(&path)).unwrap().expect("free");
        assert!(Flock::try_shared(open(&path)).unwrap().is_ok());
        assert!(Flock::try_exclusive(open(&path)).unwrap().is_err());
        drop(shared);
    }
}
