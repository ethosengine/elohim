//! The payload-store format marker: `root/STORE`, one line naming the store's format.
//!
//! The marker is the one place a payload directory says which format it holds, and it decides
//! who may open the directory. An owner or shared [`crate::BlobStore`] opens a root only when it
//! is empty (or absent), in which case the open claims it by writing the
//! [`BLOB_STORE_V1`] marker before any other file, or when its marker already says
//! [`BLOB_STORE_V1`]; the unlocked store refuses any root that carries a marker. epr-cli's
//! payload opener claims a new `directory-v1` directory through the same [`claim`], and
//! elohim-storage composes its tier from a marker it never writes. This module is the only
//! implementation of reading and claiming the marker.

use std::io::{self, Write};
use std::path::Path;

/// The marker's file name, in the payload store's root.
pub const FILE: &str = "STORE";

/// The marker spelling of this crate's format.
pub const BLOB_STORE_V1: &str = "blob-store-v1";

/// The reserved prefix of a marker being staged in the root: an opener scanning the root reads
/// such a name as a claim in flight, never as content.
pub const STAGING_PREFIX: &str = ".STORE-";

/// Whether a root entry's name is a marker being staged ([`STAGING_PREFIX`]).
pub fn is_staging(name: &str) -> bool {
    name.starts_with(STAGING_PREFIX)
}

/// The marker's content, trimmed: `Ok(None)` when the root holds no marker (or does not
/// exist). Never creates anything.
///
/// # Errors
///
/// Any read failure other than absence, and a marker that is not UTF-8.
pub fn read(root: &Path) -> io::Result<Option<String>> {
    match std::fs::read(root.join(FILE)) {
        Ok(bytes) => String::from_utf8(bytes)
            .map(|text| Some(text.trim().to_string()))
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "STORE is not UTF-8")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Claim the existing directory `root` for `format`, unless another opener claimed it first, and
/// return the marker that won. The marker is written whole and fsynced under a reserved
/// [`STAGING_PREFIX`] name, then hard-linked to [`FILE`] (which fails if it exists, so nothing is
/// ever replaced), then the root is fsynced. Two first openers race to one winner; the loser
/// reads the winner's marker. Writes nothing but the marker (and its staging name, removed).
///
/// # Errors
///
/// Any failure to stage, link (other than an existing marker) or read back the marker.
pub fn claim(root: &Path, format: &str) -> io::Result<String> {
    let target = root.join(FILE);
    let staging = root.join(format!("{STAGING_PREFIX}{}", uuid::Uuid::new_v4()));
    let written = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging)?;
        file.write_all(format!("{format}\n").as_bytes())?;
        file.sync_all()
    })();
    if let Err(error) = written {
        let _ = std::fs::remove_file(&staging);
        return Err(error);
    }
    let linked = std::fs::hard_link(&staging, &target);
    let _ = std::fs::remove_file(&staging);
    match linked {
        Ok(()) => std::fs::File::open(root)?.sync_all()?,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    read(root)?.ok_or_else(|| io::Error::other("the STORE marker vanished after it was claimed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_absent_root_reads_no_marker_and_creates_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("never-made");
        assert_eq!(read(&root).unwrap(), None);
        assert!(!root.exists());
    }

    #[test]
    fn the_marker_is_read_trimmed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE), format!("{BLOB_STORE_V1}\n")).unwrap();
        assert_eq!(read(dir.path()).unwrap().as_deref(), Some(BLOB_STORE_V1));
    }

    #[test]
    fn concurrent_claims_install_one_marker_and_every_claimant_reads_it() {
        for _ in 0..20 {
            let dir = tempfile::tempdir().unwrap();
            let won: Vec<String> = std::thread::scope(|scope| {
                let handles: Vec<_> = (0..8)
                    .map(|i| {
                        let root = dir.path();
                        scope.spawn(move || {
                            claim(
                                root,
                                if i % 2 == 0 {
                                    BLOB_STORE_V1
                                } else {
                                    "directory-v1"
                                },
                            )
                            .unwrap()
                        })
                    })
                    .collect();
                handles.into_iter().map(|h| h.join().unwrap()).collect()
            });
            assert!(won.windows(2).all(|pair| pair[0] == pair[1]), "{won:?}");
            let names: Vec<String> = std::fs::read_dir(dir.path())
                .unwrap()
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            assert_eq!(names, vec![FILE.to_string()], "no staging residue");
        }
    }
}
