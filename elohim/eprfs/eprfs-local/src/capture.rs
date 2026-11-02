//! Explicit filesystem selection -> private canonical Tree v1.
//!
//! No Git discovery, hooks, publication or head election. Reads are Linux
//! descriptor-relative and never traverse a symlink. The store and its parent
//! must be trusted against concurrent replacement, as required by DirectoryStorage.
//! All selection checks and source reads precede store creation or object writes.
//! Observed races refuse; a second bounded byte pass also checks content that
//! shares a filesystem timestamp tick. This is not an atomic snapshot of a
//! changing filesystem: a writer can still change it after the last check.
//!
//! This first capture lane verifies the retained closure but does NOT yet claim
//! crash durability: DirectoryStorage's directory-sync protocol is a separate
//! custody obligation. A failed/cancelled write can leave verified, unreferenced
//! objects; it never deletes prior candidates or writes a mutable root pointer.

use std::path::{Path, PathBuf};

use eprfs_core::{BlobCid, TreeLimits};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(all(test, target_os = "linux"))]
mod tests;

/// Per raw leaf or encoded directory object; callers can lower tree limits but
/// cannot increase this ceiling or the 1 GiB expanded closure ceiling.
pub const MAX_CAPTURE_OBJECT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("filesystem capture refused: {0}")]
    Refused(String),
    #[error("source changed during capture at {0}")]
    Changed(PathBuf),
    #[error("capture resource budget exceeded: {0}")]
    Budget(&'static str),
    #[error("capture I/O at {path}: {source}", path = .path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Tree(#[from] Box<eprfs_core::TreeError>),
    #[error(transparent)]
    Storage(#[from] Box<eprfs_core::EprfsError>),
}

impl From<eprfs_core::TreeError> for CaptureError {
    fn from(error: eprfs_core::TreeError) -> Self {
        Self::Tree(Box::new(error))
    }
}

impl From<eprfs_core::EprfsError> for CaptureError {
    fn from(error: eprfs_core::EprfsError) -> Self {
        Self::Storage(Box::new(error))
    }
}

/// Exact selected scope, in canonical byte-path order. Counts include inferred
/// directories (excluding the root) and expanded leaf bytes, including hardlinks
/// at each selected name. This is custody evidence, never publication authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureReport {
    pub root: BlobCid,
    pub selected_paths: Vec<PathBuf>,
    pub files: usize,
    pub directories: usize,
    pub symlinks: usize,
    pub bytes_captured: usize,
    pub published: bool,
}

/// Capture explicit relative regular files and literal symlinks into a private
/// store, returning only after local verification of the entire Tree v1 closure.
/// The store may already exist or be a new directory in an existing trusted parent.
/// Neither source nor store may contain the other, including directory aliases.
/// Selections reject dot/parent/empty components, metadata (`.git`, `.eprfs`,
/// `.brit`), glob syntax, duplicate/ancestor names, directories and special files.
/// Distinct hardlink names are valid; byte names and executable bits are preserved.
///
/// No power-loss durability is claimed by this phase. Storage failures may retain
/// partial immutable objects. The source and prior objects are never removed.
pub async fn capture_tree(
    source: impl AsRef<Path>,
    store: impl AsRef<Path>,
    selected: &[PathBuf],
    limits: &TreeLimits,
) -> Result<CaptureReport, CaptureError> {
    #[cfg(target_os = "linux")]
    {
        linux::capture(source.as_ref(), store.as_ref(), selected, limits, |_, _| {}).await
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (source, store, selected, limits);
        Err(CaptureError::Refused(
            "confined filesystem capture is supported on Linux only".into(),
        ))
    }
}
