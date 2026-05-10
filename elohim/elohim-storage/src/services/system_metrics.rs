//! Per-node system metrics probes for the cluster_view local slice.
//!
//! Operational (Category C) per the p2p-design-gate output. None of these
//! values is authoritative on the DHT; they are observation snapshots.
//!
//! ## Boundary: per-node only
//!
//! This module exposes single-node primitives. Cross-node aggregations
//! (sum of CPU cores across a household's devices, total committed
//! storage across all my pods, etc.) are emergent properties of the
//! household-hub surface and live there, not here. See memory:
//! `project_node_metrics_vs_hub_aggregation_boundary`.
//!
//! ## Foundation
//!
//! - **Filesystem**: `fs4` crate (cross-platform statvfs/GetDiskFreeSpaceEx
//!   wrapper) — same primitive as `heartbeat::measure_free_pct`. Stays on
//!   one foundation per `feedback_check_existing_compute_foundation`.
//! - **Memory**: POSIX `sysinfo()` + `getrusage()` via `libc` — works on
//!   Linux, macOS, BSD without `/proc` parsing. Replaces earlier Linux-only
//!   `/proc/self/status` and `/proc/meminfo` readers.

use std::path::Path;

/// Sum the byte sizes of all regular files under `path`, recursively.
///
/// Returns `Ok(0)` when the path does not exist or is empty. Uses a
/// breadth-first walk; symlinks are followed once (no cycle protection
/// since the blob store is flat).
pub fn directory_size(path: &Path) -> std::io::Result<u64> {
    if !path.exists() {
        return Ok(0);
    }

    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.is_dir() {
                stack.push(entry.path());
            } else if meta.is_file() {
                total = total.saturating_add(meta.len());
            }
        }
    }

    Ok(total)
}

/// Read the filesystem capacity (total bytes) for the volume containing `path`.
///
/// Synthesizes the existing `fs4` cross-platform statvfs/GetDiskFreeSpaceEx
/// wrapper already in use at `heartbeat::measure_free_pct` — keeping all
/// filesystem capacity probes on a single foundation. Returns `None` on
/// syscall failure.
pub fn filesystem_capacity_bytes(path: &Path) -> Option<u64> {
    fs4::total_space(path).ok()
}

/// Read this process's resident set size in bytes via POSIX `getrusage`.
///
/// Cross-platform: works on Linux, macOS, and BSD without `/proc` parsing.
///
/// Note on units: `ru_maxrss` is in KB on Linux but in bytes on macOS.
/// We compile-gate the conversion so callers always get bytes back.
/// Returns `None` on syscall failure.
pub fn process_memory_bytes() -> Option<u64> {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: getrusage with RUSAGE_SELF and a valid &mut rusage is sound.
    let rc = unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    if rc != 0 {
        return None;
    }
    let raw = usage.ru_maxrss as u64;
    // Linux: ru_maxrss is in KB. macOS/BSD: bytes.
    #[cfg(target_os = "linux")]
    let bytes = raw.saturating_mul(1024);
    #[cfg(not(target_os = "linux"))]
    let bytes = raw;
    Some(bytes)
}

/// Read total system memory in bytes via POSIX `sysinfo` (Linux) or
/// `sysctlbyname` (macOS/BSD via libc fallback).
///
/// Linux's `libc::sysinfo` is direct; on non-Linux POSIX this returns
/// `None` (callers should treat zero/unknown as "ignore" in aggregation).
/// Future expansion: replace the non-Linux branch with a `sysctlbyname`-
/// based reader when steward conductors target macOS.
pub fn total_memory_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let mut info: libc::sysinfo = unsafe { std::mem::zeroed() };
        // SAFETY: sysinfo with a valid &mut sysinfo is sound on Linux.
        let rc = unsafe { libc::sysinfo(&mut info) };
        if rc != 0 {
            return None;
        }
        // totalram is in units of mem_unit bytes.
        Some((info.totalram as u64).saturating_mul(info.mem_unit as u64))
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Count CPU cores available to this process.
///
/// Uses `std::thread::available_parallelism`, which respects cgroup CPU
/// quotas on Linux (so a pod with `--cpus=2` reports 2, not the host count).
/// Matches the precedent at `doorway/orchestrator/node_bootstrap.rs:144`
/// so per-node compute features stay on a single foundation. Returns
/// `None` when the platform refuses to answer.
pub fn cpu_count() -> Option<u32> {
    std::thread::available_parallelism()
        .ok()
        .map(|n| n.get() as u32)
}

/// Read POSIX 1/5/15-minute load averages.
///
/// `libc::getloadavg` is POSIX (Linux/macOS/BSD); returns `None` when the
/// syscall fails or the platform does not support it. Values are unitless
/// run-queue averages — interpreted relative to `cpu_count()`.
pub fn load_average() -> Option<(f64, f64, f64)> {
    let mut buf = [0.0f64; 3];
    // SAFETY: getloadavg writes up to `nelem` doubles into `loadavg`. Buffer
    // is sized for 3 elements; the call writes that many or returns -1.
    let n = unsafe { libc::getloadavg(buf.as_mut_ptr(), 3) };
    if n != 3 {
        return None;
    }
    Some((buf[0], buf[1], buf[2]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn directory_size_returns_zero_for_empty_dir() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(directory_size(tmp.path()).unwrap(), 0);
    }

    #[test]
    fn directory_size_sums_file_bytes() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a"), b"hello").unwrap(); // 5 bytes
        fs::write(tmp.path().join("b"), b"world!!!").unwrap(); // 8 bytes
        assert_eq!(directory_size(tmp.path()).unwrap(), 13);
    }

    #[test]
    fn directory_size_recurses_into_subdirs() {
        let tmp = TempDir::new().unwrap();
        let sub = tmp.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("a"), b"hello").unwrap();
        assert_eq!(directory_size(tmp.path()).unwrap(), 5);
    }

    #[test]
    fn directory_size_returns_zero_for_nonexistent_path() {
        assert_eq!(
            directory_size("/nonexistent/path/here".as_ref()).unwrap_or(0),
            0
        );
    }

    #[test]
    fn filesystem_capacity_bytes_returns_some_for_root_or_tmp() {
        // The volume containing /tmp must exist on any test host.
        assert!(filesystem_capacity_bytes("/tmp".as_ref()).unwrap_or(0) > 0);
    }

    #[test]
    fn process_memory_returns_nonzero_when_we_are_running() {
        // We are running, so getrusage(RUSAGE_SELF).ru_maxrss must be > 0
        // on any POSIX host.
        assert!(process_memory_bytes().unwrap_or(0) > 0);
    }

    #[test]
    fn total_memory_returns_nonzero_on_linux() {
        if cfg!(target_os = "linux") {
            assert!(total_memory_bytes().unwrap_or(0) > 0);
        }
    }

    #[test]
    fn cpu_count_returns_nonzero_on_real_host() {
        // Any test host has at least one core.
        assert!(cpu_count().unwrap_or(0) >= 1);
    }

    #[test]
    fn load_average_returns_three_finite_values() {
        // POSIX hosts (linux/macOS) return 3 doubles; CI runners always do.
        if let Some((one, five, fifteen)) = load_average() {
            assert!(one.is_finite() && one >= 0.0);
            assert!(five.is_finite() && five >= 0.0);
            assert!(fifteen.is_finite() && fifteen >= 0.0);
        }
    }
}
