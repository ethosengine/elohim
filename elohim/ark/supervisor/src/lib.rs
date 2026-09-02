//! ark-supervisor — the I/O half of the elohim compute envelope (tevah).
//!
//! Spec: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md §6, §8.
//! I/O lives here and nothing else does: the pure decisions are `ark_core`'s, and
//! there is no network in this crate (no swarm, no HTTP, no DHT).

pub mod driver;
pub mod native;
pub mod pipes;
pub mod reaper;
pub mod spool;
pub mod supervisor;

// Re-exports are uncommented by the task that fills each module.
pub use driver::{Driver, DriverError, Fingerprint, Started};
pub use native::{sha256_file, NativeDriver};
pub use pipes::{spawn_line_reader, StreamTap};
pub use reaper::{
    become_subreaper, proc_status_sample, reap_with_rusage, wait_nowait, ReapError, WaitEvent,
};
pub use spool::{Spool, SpoolError, WitnessSummary};
pub use supervisor::{RunOutcome, Supervisor, SupervisorError, SystemClock};
