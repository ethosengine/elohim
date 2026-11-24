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
use std::time::{Duration, Instant};

use rustix::fs::{flock, FlockOperation};

/// A held flock on its file. Dropping it unlocks, then closes.
///
/// One guard is one holder. [`Flock::file`] and [`AsFd`] let a caller duplicate the descriptor,
/// but a duplicate is the same open file description, not an independent holder: a lock taken
/// through it (a shared lock converted to exclusive, say) converts this one lock, a failed
/// conversion can lose it, and unlocking or dropping either side releases both. Take a second
/// holder through a second open of the file, never through a copy of this one.
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

    /// Take the exclusive lock within `patience`, trying immediately and then every `poll`.
    /// Zero patience makes exactly one attempt.
    ///
    /// # Errors
    ///
    /// A lock call failed for a reason other than contention.
    pub fn exclusive_within(
        file: File,
        patience: Duration,
        poll: Duration,
    ) -> std::io::Result<Result<Flock, File>> {
        Self::take_within(
            file,
            patience,
            poll,
            FlockOperation::NonBlockingLockExclusive,
        )
    }

    /// Take the shared lock within `patience`, trying immediately and then every `poll`.
    /// Zero patience makes exactly one attempt.
    ///
    /// # Errors
    ///
    /// A lock call failed for a reason other than contention.
    pub fn shared_within(
        file: File,
        patience: Duration,
        poll: Duration,
    ) -> std::io::Result<Result<Flock, File>> {
        Self::take_within(file, patience, poll, FlockOperation::NonBlockingLockShared)
    }

    fn take_within(
        mut file: File,
        patience: Duration,
        poll: Duration,
        operation: FlockOperation,
    ) -> std::io::Result<Result<Flock, File>> {
        let started = Instant::now();
        loop {
            match Self::try_take(file, operation)? {
                Ok(held) => return Ok(Ok(held)),
                Err(contended) => file = contended,
            }
            let elapsed = started.elapsed();
            if elapsed >= patience {
                return Ok(Err(file));
            }
            std::thread::sleep(poll.min(patience - elapsed));
        }
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
        // On a descriptor this guard owns, `LOCK_UN` fails only for what the guard rules out (a bad
        // descriptor, an invalid operation) or for kernel resource exhaustion: it should not
        // happen. If it does, the release is left to the close that follows, and that release
        // waits for the last copy of the descriptor, so a forked child's copy keeps the lock
        // held until its `exec` or exit: the window this guard exists to close. Said on stderr,
        // as this crate's other warnings are.
        if let Err(errno) = flock(&self.file, FlockOperation::Unlock) {
            eprintln!(
                "WARNING: flock unlock failed ({errno}); the lock is released when the last copy \
                 of its descriptor closes"
            );
        }
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

    #[test]
    fn zero_patience_refuses_a_held_lock_at_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.lock");
        let held = Flock::try_exclusive(open(&path)).unwrap().expect("free");
        let started = Instant::now();
        assert!(
            Flock::exclusive_within(open(&path), Duration::ZERO, Duration::from_secs(1))
                .unwrap()
                .is_err()
        );
        assert!(started.elapsed() < Duration::from_millis(500));
        drop(held);
    }

    #[test]
    fn a_lock_released_within_patience_is_taken() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.lock");
        let held = Flock::try_exclusive(open(&path)).unwrap().expect("free");
        let releaser = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            drop(held);
        });
        let acquired = Flock::exclusive_within(
            open(&path),
            Duration::from_secs(1),
            Duration::from_millis(10),
        )
        .unwrap();
        releaser.join().unwrap();
        assert!(acquired.is_ok());
    }

    /// A child forked by another thread holds the descriptor through its pre-exec pause, but
    /// dropping the guard unlocks the shared open file description immediately.
    #[cfg(unix)]
    #[test]
    fn a_guard_release_is_not_delayed_by_a_forked_child() {
        use std::os::unix::process::CommandExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.lock");
        let held = Flock::try_exclusive(open(&path)).unwrap().expect("free");
        let forker = std::thread::spawn(|| {
            let mut command = std::process::Command::new("true");
            // SAFETY: the hook only sleeps, holding the forked child before exec with the
            // descriptors it inherited.
            unsafe {
                command.pre_exec(|| {
                    std::thread::sleep(Duration::from_millis(1500));
                    Ok(())
                });
            }
            command.status().unwrap()
        });
        std::thread::sleep(Duration::from_millis(300));
        drop(held);
        let reacquired = Flock::exclusive_within(
            open(&path),
            Duration::from_millis(100),
            Duration::from_millis(10),
        );
        assert!(forker.join().unwrap().success());
        assert!(reacquired.unwrap().is_ok());
    }
}
