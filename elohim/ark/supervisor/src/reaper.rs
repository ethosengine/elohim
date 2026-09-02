//! Learning of a death without consuming it, then consuming it with its accounting.
//!
//! The supervision loop must know a child died *before* it decides anything, and it must
//! still be able to read the kernel's accounting when it acts. Those two needs conflict for
//! anyone who reaps with a plain `wait`: the first successful wait destroys the zombie and
//! every fact attached to it. So the order here is fixed (spec §12 item 19):
//!
//! 1. [`wait_nowait`] — `waitid(P_PID, WEXITED|WNOWAIT|WNOHANG)` learns of the death and
//!    deliberately leaves the zombie in place;
//! 2. [`proc_status_sample`] — best-effort `/proc` read while that zombie still exists;
//! 3. [`reap_with_rusage`] — `wait4` consumes it and returns `rusage`, which is where peak
//!    RSS and CPU time come from, because a zombie's `/proc/<pid>/status` has neither.
//!
//! `std::process::Child::wait` is never used for a supervised child: it would consume the
//! exit status the witness is made of.

use std::fs;

use ark_core::{exit::ExitClass, sample::ProcessSample};
use nix::{
    errno::Errno,
    sys::wait::{waitid, Id, WaitPidFlag, WaitStatus},
    unistd::Pid,
};

/// What a non-consuming wait observed.
#[derive(Clone, Debug, PartialEq)]
pub enum WaitEvent {
    /// The child has not terminated.
    StillRunning,
    /// The child has terminated and has NOT been reaped.
    Exited {
        /// The kernel's termination cause.
        class: ExitClass,
        /// Whatever `/proc` still holds for the zombie; often mostly empty.
        sample: ProcessSample,
    },
}

/// A failure while learning of, or consuming, a child's death.
#[derive(thiserror::Error, Debug)]
pub enum ReapError {
    /// The pid is not a child of this process (or has already been reaped).
    #[error("no such child: {pid}")]
    NoSuchChild {
        /// The pid that was waited on.
        pid: u32,
    },
    /// `waitid` failed for a reason other than the child not existing.
    #[error("waitid({pid}): {message}")]
    Wait {
        /// The pid that was waited on.
        pid: u32,
        /// The operating-system error.
        message: String,
    },
    /// `wait4` failed for a reason other than the child not existing.
    #[error("wait4({pid}): {message}")]
    Reap {
        /// The pid that was reaped.
        pid: u32,
        /// The operating-system error.
        message: String,
    },
    /// The process could not become a subreaper.
    #[error("prctl(PR_SET_CHILD_SUBREAPER): {message}")]
    Subreaper {
        /// The operating-system error.
        message: String,
    },
}

/// Learns whether a child has died, without consuming its exit status.
///
/// `WNOWAIT` is the whole point: the caller may observe the same death repeatedly until it
/// is ready to reap, so a witness can be written before the status is destroyed.
pub fn wait_nowait(pid: u32) -> Result<WaitEvent, ReapError> {
    let flags = WaitPidFlag::WEXITED | WaitPidFlag::WNOWAIT | WaitPidFlag::WNOHANG;
    match waitid(Id::Pid(Pid::from_raw(pid as i32)), flags) {
        Ok(WaitStatus::Exited(_, code)) => Ok(exited(pid, ExitClass::Exited { code })),
        Ok(WaitStatus::Signaled(_, signal, core_dumped)) => Ok(exited(
            pid,
            ExitClass::Signaled {
                signal: signal as i32,
                core_dumped,
            },
        )),
        // `StillAlive`, and the stop/continue transitions this flag set does not ask for:
        // none of them is a termination, so none of them is a death.
        Ok(_) => Ok(WaitEvent::StillRunning),
        Err(Errno::ECHILD) => Err(ReapError::NoSuchChild { pid }),
        Err(errno) => Err(ReapError::Wait {
            pid,
            message: errno.to_string(),
        }),
    }
}

fn exited(pid: u32, class: ExitClass) -> WaitEvent {
    WaitEvent::Exited {
        class,
        sample: proc_status_sample(pid).unwrap_or_default(),
    }
}

/// Reads what `/proc` currently holds for a process, best effort.
///
/// Every field is optional because every one of them can be denied or absent: a zombie keeps
/// its `status` file but loses its memory counters, and `io` is unreadable for a process
/// this one does not own. Returns `None` only when the process itself is gone.
pub fn proc_status_sample(pid: u32) -> Option<ProcessSample> {
    let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let mut sample = ProcessSample::default();

    for line in status.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key {
            "VmHWM" => sample.max_rss_bytes = kib_value(value),
            "VmRSS" => sample.rss_bytes = kib_value(value),
            "Threads" => sample.threads = value.parse().ok(),
            _ => {}
        }
    }

    sample.fds = fs::read_dir(format!("/proc/{pid}/fd"))
        .ok()
        .map(|entries| entries.filter(Result::is_ok).count() as u32);
    sample.oom_score_adj = fs::read_to_string(format!("/proc/{pid}/oom_score_adj"))
        .ok()
        .and_then(|value| value.trim().parse().ok());

    if let Ok(io) = fs::read_to_string(format!("/proc/{pid}/io")) {
        for line in io.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim().parse().ok();
            match key {
                "read_bytes" => sample.io_read_bytes = value,
                "write_bytes" => sample.io_write_bytes = value,
                _ => {}
            }
        }
    }

    Some(sample)
}

/// Consumes a dead child and returns its termination cause with the kernel's accounting.
///
/// The returned sample carries only what `rusage` knows — peak RSS and CPU time — and
/// deliberately does not re-read `/proc`: the pid is free for reuse the instant `wait4`
/// returns, so a read after the reap could describe a different process entirely.
pub fn reap_with_rusage(pid: u32) -> Result<(ExitClass, ProcessSample), ReapError> {
    let mut status: libc::c_int = 0;
    // SAFETY: `rusage` is plain old data with no invalid bit patterns, and zeroing is how a
    // libc caller initializes the out-parameter `wait4` fills.
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: `status` and `usage` are live, correctly typed locals owned by this frame, and
    // `wait4` writes only through those two pointers.
    let reaped = unsafe { libc::wait4(pid as libc::pid_t, &mut status, 0, &mut usage) };

    if reaped < 0 {
        let error = std::io::Error::last_os_error();
        return if error.raw_os_error() == Some(libc::ECHILD) {
            Err(ReapError::NoSuchChild { pid })
        } else {
            Err(ReapError::Reap {
                pid,
                message: error.to_string(),
            })
        };
    }

    let sample = ProcessSample {
        max_rss_bytes: Some(usage.ru_maxrss.max(0) as u64 * 1024),
        user_us: Some(timeval_us(usage.ru_utime)),
        system_us: Some(timeval_us(usage.ru_stime)),
        ..ProcessSample::default()
    };

    Ok((ExitClass::from_raw_wait_status(status), sample))
}

/// Declares this process the reaper of its orphaned descendants.
///
/// A conductor that forks and dies leaves grandchildren whose parent becomes pid 1 unless
/// the envelope claims them — and a death nobody is the parent of is a death nobody can
/// witness. `PR_SET_PDEATHSIG` is deliberately never set on a child: the envelope decides
/// when a conductor dies, not the kernel.
pub fn become_subreaper() -> Result<(), ReapError> {
    let on: libc::c_ulong = 1;
    let unused: libc::c_ulong = 0;
    // SAFETY: `PR_SET_CHILD_SUBREAPER` takes its arguments by value and writes through no
    // pointer; the call only sets a flag on the calling process.
    let result = unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, on, unused, unused, unused) };

    if result != 0 {
        return Err(ReapError::Subreaper {
            message: std::io::Error::last_os_error().to_string(),
        });
    }
    Ok(())
}

/// Parses a `/proc` `"1234 kB"` field into bytes.
fn kib_value(value: &str) -> Option<u64> {
    value
        .split_whitespace()
        .next()?
        .parse::<u64>()
        .ok()
        .map(|kib| kib * 1024)
}

fn timeval_us(time: libc::timeval) -> u64 {
    time.tv_sec.max(0) as u64 * 1_000_000 + time.tv_usec.max(0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_sample_reads_this_live_process() {
        let sample = proc_status_sample(std::process::id()).expect("/proc for self");

        assert!(
            sample.rss_bytes.unwrap_or(0) > 0,
            "a live process has an RSS"
        );
        assert!(sample.threads.unwrap_or(0) > 0);
        assert!(sample.fds.unwrap_or(0) > 0);
    }

    #[test]
    fn proc_sample_is_none_for_an_impossible_pid() {
        // Above /proc/sys/kernel/pid_max on every supported host.
        assert!(proc_status_sample(u32::MAX).is_none());
    }

    #[test]
    fn waiting_on_a_process_that_is_not_our_child_names_it() {
        // pid 1 is never a child of this process.
        assert!(matches!(
            wait_nowait(1),
            Err(ReapError::NoSuchChild { pid: 1 })
        ));
    }

    #[test]
    fn becoming_a_subreaper_is_idempotent() {
        become_subreaper().unwrap();
        become_subreaper().unwrap();
    }

    #[test]
    fn kib_fields_become_bytes() {
        assert_eq!(kib_value("1234 kB"), Some(1_263_616));
        assert_eq!(kib_value("0 kB"), Some(0));
        assert_eq!(kib_value("unlimited"), None);
    }
}
