//! Bounded fork-before-exec handshake shared by Unix unit tests.

use std::sync::mpsc::{self, Receiver};
use std::thread::JoinHandle;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) struct ChildRelease(libc::c_int);

impl ChildRelease {
    pub(crate) fn release(&mut self) {
        if self.0 >= 0 {
            let byte = [1_u8];
            // SAFETY: this guard owns the parent's release-pipe write end, and the parked child
            // is reading one byte from the other end.
            unsafe {
                libc::write(self.0, byte.as_ptr().cast(), byte.len());
                libc::close(self.0);
            }
            self.0 = -1;
        }
    }

    fn discard(&mut self) {
        if self.0 >= 0 {
            // SAFETY: no live child is waiting for this byte, so close without writing.
            unsafe { libc::close(self.0) };
            self.0 = -1;
        }
    }
}

impl Drop for ChildRelease {
    fn drop(&mut self) {
        self.release();
    }
}

pub(crate) struct ForkedChild {
    done: Receiver<Result<(), String>>,
    thread: Option<JoinHandle<()>>,
}

impl ForkedChild {
    pub(crate) fn join(mut self) {
        let result = self
            .done
            .recv_timeout(TIMEOUT)
            .expect("forked child did not exit within 30 seconds");
        self.thread
            .take()
            .expect("fork thread")
            .join()
            .expect("fork thread panicked");
        result.expect("forked child failed");
    }
}

/// Fork a child on another thread and park it immediately before exec.
///
/// The forking thread closes the parent's ready-pipe writer before reporting that the fork
/// completed. A child that dies before its ready byte therefore produces EOF rather than an
/// unbounded read. Both readiness and completion are hard test failures after 30 seconds.
pub(crate) fn park_child_before_exec() -> (ChildRelease, ForkedChild) {
    let mut ready = [0; 2];
    // SAFETY: `ready` has room for the two descriptors `pipe` writes.
    assert_eq!(unsafe { libc::pipe(ready.as_mut_ptr()) }, 0);
    let [read_ready, write_ready] = ready;
    let mut release = [0; 2];
    // SAFETY: `release` has room for the two descriptors `pipe` writes.
    assert_eq!(unsafe { libc::pipe(release.as_mut_ptr()) }, 0);
    let [read_release, write_release] = release;
    let mut release_guard = ChildRelease(write_release);
    let (forked_tx, forked_rx) = mpsc::sync_channel(0);
    let (done_tx, done_rx) = mpsc::sync_channel(0);
    let thread = std::thread::spawn(move || {
        // SAFETY: the child branch calls only async-signal-safe libc operations before exec.
        let pid = unsafe { libc::fork() };
        if pid == 0 {
            // SAFETY: these descriptors belong to the just-forked child. `_exit` is used on every
            // failure path so no Rust runtime or allocator state is touched after the fork.
            unsafe {
                libc::close(read_ready);
                libc::close(write_release);
                let byte = [1_u8];
                if libc::write(write_ready, byte.as_ptr().cast(), byte.len()) != 1 {
                    libc::_exit(126);
                }
                libc::close(write_ready);
                let mut release_byte = [0_u8];
                if libc::read(
                    read_release,
                    release_byte.as_mut_ptr().cast(),
                    release_byte.len(),
                ) != 1
                {
                    libc::_exit(126);
                }
                libc::close(read_release);
                let path = c"/bin/true";
                libc::execl(
                    path.as_ptr(),
                    path.as_ptr(),
                    std::ptr::null::<libc::c_char>(),
                );
                libc::_exit(127);
            }
        }

        // SAFETY: after `fork`, these are the parent's copies. Closing `write_ready` here is the
        // structural EOF guarantee; the child has its own descriptor table by this point.
        unsafe {
            libc::close(write_ready);
            libc::close(read_release);
        }
        if pid < 0 {
            let error = std::io::Error::last_os_error().to_string();
            let _ = forked_tx.send(Err(error.clone()));
            let _ = done_tx.send(Err(error));
            return;
        }
        if forked_tx.send(Ok(())).is_err() {
            return;
        }
        let mut status = 0;
        // SAFETY: `pid` is the positive child pid returned by `fork` and `status` is writable.
        let waited = unsafe { libc::waitpid(pid, &mut status, 0) };
        let result = if waited != pid {
            Err(std::io::Error::last_os_error().to_string())
        } else if libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0 {
            Ok(())
        } else {
            Err(format!("child wait status {status}"))
        };
        let _ = done_tx.send(result);
    });

    match forked_rx.recv_timeout(TIMEOUT) {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            release_guard.discard();
            panic!("fork failed: {error}");
        }
        Err(error) => panic!("fork did not complete within 30 seconds: {error}"),
    }
    let mut poll = libc::pollfd {
        fd: read_ready,
        events: libc::POLLIN | libc::POLLHUP,
        revents: 0,
    };
    // SAFETY: `poll` points to one initialized descriptor and the timeout is bounded.
    let polled = unsafe { libc::poll(&mut poll, 1, 30_000) };
    assert_eq!(polled, 1, "child readiness timed out or poll failed");
    let mut byte = [0_u8];
    // SAFETY: `read_ready` is the parent's pipe reader and `byte` has the requested capacity.
    let read = unsafe { libc::read(read_ready, byte.as_mut_ptr().cast(), byte.len()) };
    // SAFETY: readiness has resolved, so the parent no longer needs its reader.
    unsafe { libc::close(read_ready) };
    if read != 1 {
        release_guard.discard();
        panic!("child died before signalling readiness: read returned {read}");
    }
    assert_eq!(byte[0], 1, "child sent an invalid readiness byte");

    (
        release_guard,
        ForkedChild {
            done: done_rx,
            thread: Some(thread),
        },
    )
}
