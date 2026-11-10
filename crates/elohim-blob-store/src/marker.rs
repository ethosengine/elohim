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

/// The prefix of the one temporary file [`claim`] writes in a root: `.STORE.claim-` followed by
/// exactly 32 lowercase hex digits.
pub const CLAIM_TEMP_PREFIX: &str = ".STORE.claim-";

/// The most bytes a claim temp can hold: one format name and a newline.
pub const CLAIM_TEMP_MAX_BYTES: u64 = 64;

/// Whether `name` is exactly a claim temp's name ([`CLAIM_TEMP_PREFIX`] and 32 lowercase hex).
fn is_claim_temp_name(name: &str) -> bool {
    name.strip_prefix(CLAIM_TEMP_PREFIX).is_some_and(|rest| {
        rest.len() == 32
            && rest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

/// Whether the root entry `path` is a claim in flight (or a crashed claim's residue) and nothing
/// else: a regular file, not a symlink or directory, named exactly as [`claim`] names its temp,
/// and no larger than [`CLAIM_TEMP_MAX_BYTES`]. Every other entry, a `.STORE-…` or
/// `.STORE.claim-…` look-alike included, is content.
pub fn is_claim_temp(path: &Path) -> bool {
    let named = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(is_claim_temp_name);
    named
        && std::fs::symlink_metadata(path)
            .is_ok_and(|meta| meta.file_type().is_file() && meta.len() <= CLAIM_TEMP_MAX_BYTES)
}

/// Remove every claim temp ([`is_claim_temp`]) in `root`: the residue of claims that crashed
/// between staging and cleanup. For an owner at open, under its exclusive lock; a claim racing
/// this sweep re-reads the marker it lost its temp to ([`claim`]).
///
/// # Errors
///
/// A root that cannot be listed, or a temp that cannot be removed.
pub fn sweep_claim_temps(root: &Path) -> io::Result<()> {
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        if is_claim_temp(&path) {
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
    }
    Ok(())
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
/// return the marker that won. The marker is written whole and fsynced under a claim temp's name
/// ([`is_claim_temp`]), then hard-linked to [`FILE`] (which fails if it exists, so nothing is
/// ever replaced), then the root is fsynced. Two first openers race to one winner; the loser
/// reads the winner's marker. Writes nothing but the marker (and its staging name, removed).
///
/// # Errors
///
/// Any failure to stage, link (other than an existing marker) or read back the marker.
pub fn claim(root: &Path, format: &str) -> io::Result<String> {
    let target = root.join(FILE);
    let staging = root.join(format!(
        "{CLAIM_TEMP_PREFIX}{}",
        uuid::Uuid::new_v4().simple()
    ));
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
        // An owner swept the temp: it only does so once a marker exists, so read that one.
        Err(error) if error.kind() == io::ErrorKind::NotFound && read(root)?.is_some() => {}
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
    fn only_a_small_regular_file_named_exactly_as_a_claim_temp_is_one() {
        let dir = tempfile::tempdir().unwrap();
        let genuine = dir.path().join(format!(
            "{CLAIM_TEMP_PREFIX}{}",
            "0123456789abcdef".repeat(2)
        ));
        std::fs::write(&genuine, format!("{BLOB_STORE_V1}\n")).unwrap();
        assert!(is_claim_temp(&genuine));
        let oversized = dir
            .path()
            .join(format!("{CLAIM_TEMP_PREFIX}{}", "f".repeat(32)));
        std::fs::write(&oversized, vec![b'x'; CLAIM_TEMP_MAX_BYTES as usize + 1]).unwrap();
        assert!(!is_claim_temp(&oversized));
        let as_dir = dir
            .path()
            .join(format!("{CLAIM_TEMP_PREFIX}{}", "e".repeat(32)));
        std::fs::create_dir(&as_dir).unwrap();
        assert!(!is_claim_temp(&as_dir));
        for name in [
            ".STORE-data".to_string(),
            format!("{CLAIM_TEMP_PREFIX}{}", "A".repeat(32)),
            format!("{CLAIM_TEMP_PREFIX}{}", "a".repeat(31)),
            format!("{CLAIM_TEMP_PREFIX}{}x", "a".repeat(32)),
        ] {
            let path = dir.path().join(&name);
            std::fs::write(&path, b"x").unwrap();
            assert!(!is_claim_temp(&path), "{name}");
        }
        #[cfg(unix)]
        {
            let link = dir
                .path()
                .join(format!("{CLAIM_TEMP_PREFIX}{}", "d".repeat(32)));
            std::os::unix::fs::symlink(&genuine, &link).unwrap();
            assert!(!is_claim_temp(&link));
        }
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
