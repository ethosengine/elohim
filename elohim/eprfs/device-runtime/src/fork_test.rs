//! Bounded fork-before-exec handshake for Unix unit tests.

use std::sync::mpsc::{self, Receiver};
use std::thread::JoinHandle;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) struct ChildRelease(libc::c_int);

impl ChildRelease {
    pub(crate) fn release(&mut self) {
        if self.0 >= 0 {
            let byte = [1_u8];
            // SAFETY: the guard owns the parent's writer and the parked child owns the reader.
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

pub(crate) fn park_child_before_exec() -> (ChildRelease, ForkedChild) {
    let mut ready = [0; 2];
    // SAFETY: `ready` has room for both descriptors.
    assert_eq!(unsafe { libc::pipe(ready.as_mut_ptr()) }, 0);
    let [read_ready, write_ready] = ready;
    let mut release = [0; 2];
    // SAFETY: `release` has room for both descriptors.
    assert_eq!(unsafe { libc::pipe(release.as_mut_ptr()) }, 0);
    let [read_release, write_release] = release;
    let mut release_guard = ChildRelease(write_release);
    let (forked_tx, forked_rx) = mpsc::sync_channel(0);
    let (done_tx, done_rx) = mpsc::sync_channel(0);
    let thread = std::thread::spawn(move || {
        // SAFETY: the child calls only async-signal-safe libc operations before exec.
        let pid = unsafe { libc::fork() };
        if pid == 0 {
            // SAFETY: every descriptor belongs to the child; failures terminate with `_exit`.
            unsafe {
                libc::close(read_ready);
                libc::close(write_release);
                let byte = [1_u8];
                if libc::write(write_ready, byte.as_ptr().cast(), byte.len()) != 1 {
                    libc::_exit(126);
                }
                libc::close(write_ready);
                let mut byte = [0_u8];
                if libc::read(read_release, byte.as_mut_ptr().cast(), byte.len()) != 1 {
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
        // SAFETY: after `fork`, these are the parent's copies. Closing the writer makes an early
        // child death readable as EOF.
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
        // SAFETY: `pid` is the positive child pid and `status` is writable.
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
    // SAFETY: `poll` names one initialized descriptor and has a bounded timeout.
    assert_eq!(unsafe { libc::poll(&mut poll, 1, 30_000) }, 1);
    let mut byte = [0_u8];
    // SAFETY: the buffer holds the one requested byte.
    let read = unsafe { libc::read(read_ready, byte.as_mut_ptr().cast(), byte.len()) };
    // SAFETY: readiness has resolved, so the parent no longer needs this reader.
    unsafe { libc::close(read_ready) };
    if read != 1 {
        release_guard.discard();
        panic!("child died before signalling readiness: read returned {read}");
    }
    assert_eq!(byte[0], 1);
    (
        release_guard,
        ForkedChild {
            done: done_rx,
            thread: Some(thread),
        },
    )
}
