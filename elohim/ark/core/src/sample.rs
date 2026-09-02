//! Pure snapshots of process resource use.

use serde::{Deserialize, Serialize};

/// Best-effort resource measurements for one child process.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ProcessSample {
    /// Peak resident set size in bytes.
    pub max_rss_bytes: Option<u64>,
    /// Current resident set size in bytes.
    pub rss_bytes: Option<u64>,
    /// Accumulated user CPU time in microseconds.
    pub user_us: Option<u64>,
    /// Accumulated system CPU time in microseconds.
    pub system_us: Option<u64>,
    /// Number of open file descriptors.
    pub fds: Option<u32>,
    /// Number of process threads.
    pub threads: Option<u32>,
    /// Bytes read through the process I/O counters.
    pub io_read_bytes: Option<u64>,
    /// Bytes written through the process I/O counters.
    pub io_write_bytes: Option<u64>,
    /// Linux OOM score adjustment, when available.
    pub oom_score_adj: Option<i32>,
}
