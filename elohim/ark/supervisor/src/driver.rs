//! The driver seam: how one declared child is started, signalled, and measured.
//!
//! A driver is the only place in the envelope that turns a declaration into a running
//! process, so it is also the only place that can promise the artifact it started is the
//! artifact the manifest pinned (spec §12 item 7). S0 ships exactly one implementation,
//! [`crate::native::NativeDriver`]; the trait exists so `wasm` and `delegated` kinds arrive
//! later without the supervision loop learning anything new.

use std::{
    path::PathBuf,
    process::{ChildStderr, ChildStdout},
};

use ark_core::{
    berth::Berth,
    manifest::{ChildSpec, ProcessKind},
    passport::EffectiveTier,
    sample::ProcessSample,
};

/// Host facts a driver reports about the machine it launches into.
///
/// These are read, never declared: a manifest says what should run, and a fingerprint says
/// what the host will actually enforce while it runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Fingerprint {
    /// The host's kernel hostname.
    pub hostname: String,
    /// The running kernel release.
    pub kernel: String,
    /// Whether this process could write the cgroup-v2 delegation file.
    ///
    /// Probed read-only with `access(W_OK)`; false wherever the envelope has not been
    /// delegated a cgroup subtree, which is every S0 host.
    pub cgroup_v2_delegated: bool,
    /// The enforcement tier the host offers.
    ///
    /// [`EffectiveTier::None`] in S0: the envelope declares limits and witnesses deaths, and
    /// enforces nothing yet.
    pub effective_tier: EffectiveTier,
}

/// A child that is running, together with what was verified before it started.
///
/// The pipes are moved out of the `std::process::Child` and the handle is dropped, because
/// the reaper — not `Child::wait` — owns every death in this crate: a `Child` that reaped
/// itself would consume the exit status the witness is made of.
#[derive(Debug)]
pub struct Started {
    /// The child's operating-system process identifier.
    pub pid: u32,
    /// The child's standard-output pipe.
    pub stdout: ChildStdout,
    /// The child's standard-error pipe.
    pub stderr: ChildStderr,
    /// The lowercase hexadecimal SHA-256 of the bytes actually executed.
    pub artifact_sha256: String,
    /// The local path the artifact was resolved to.
    pub artifact_path: PathBuf,
    /// Wall-clock spawn time in milliseconds since the Unix epoch.
    pub started_at_epoch_ms: u64,
}

/// The seam between a declared child and a running one.
///
/// `Send + Sync` is part of the seam, not an implementation detail: the supervisor gives each
/// supervised process its own thread and every one of them reaches the same driver, so a
/// `Box<dyn Driver>` (or `Arc<dyn Driver>`) has to cross thread boundaries and be shared while
/// it is there. A driver that needed a lock to be shared would push that lock into the
/// supervision loop, where a slow `start` on one child would stall every other child's death.
pub trait Driver: Send + Sync {
    /// Reports what this host is and what it will enforce.
    fn fingerprint(&self) -> Fingerprint;

    /// Verifies the artifact and starts the child, or refuses without spawning anything.
    fn start(&self, spec: &ChildSpec, berth: &Berth) -> Result<Started, DriverError>;

    /// Sends a signal to a running child.
    fn signal(&self, pid: u32, signal: i32) -> Result<(), DriverError>;

    /// Samples a live child's resource use; `None` when the process is gone or unreadable.
    fn stats(&self, pid: u32) -> Option<ProcessSample>;
}

/// A refusal or failure on the path from declaration to running process.
#[derive(thiserror::Error, Debug)]
pub enum DriverError {
    /// The child declares an execution model this driver does not implement.
    #[error("unsupported process kind: {0:?}")]
    UnsupportedKind(ProcessKind),
    /// The child names a mutable channel, which S0 cannot resolve to bytes.
    #[error("artifact channel {channel_id} is unresolved in S0; pin the artifact by digest")]
    ChannelUnresolvedInS0 {
        /// The declared channel identifier.
        channel_id: String,
    },
    /// The artifact could not be resolved to readable bytes.
    ///
    /// An empty path means the berth carried no `artifacts` entry for this child at all; a
    /// non-empty path means the entry exists and the file does not.
    #[error("artifact missing: {0:?}")]
    ArtifactMissing(PathBuf),
    /// The bytes on disk are not the bytes the manifest pinned.
    ///
    /// This is a refusal, never a warning: the passport hashes what it runs, so a mismatch
    /// ends the spawn (exit 66) rather than starting an unknown artifact.
    #[error("artifact {path:?} hashes to {actual}, manifest pinned {expected}")]
    ArtifactHashMismatch {
        /// The digest the manifest pinned.
        expected: String,
        /// The digest of the bytes on disk.
        actual: String,
        /// The resolved artifact path.
        path: PathBuf,
    },
    /// An argv or environment template could not be resolved against the berth.
    #[error("template: {0}")]
    Template(String),
    /// The child could not be spawned, or its pipes could not be set up.
    #[error("spawn: {0}")]
    Spawn(String),
    /// A signal could not be delivered.
    #[error("signal: {0}")]
    Signal(String),
}
