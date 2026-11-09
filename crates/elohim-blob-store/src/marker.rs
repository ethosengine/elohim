//! The payload-store format marker: `root/STORE`, one line naming the store's format.
//!
//! The marker is the one place a payload directory says which format it holds. The opener
//! that creates a store selects its format by writing the marker atomically before any
//! layout exists (epr-cli `payload.rs`); every other process reads it and refuses a format
//! it does not link, rather than opening a second store beside the first. This crate only
//! names the file and its blob-store spelling and reads it: elohim-storage composes its tier
//! from a marker it never writes.

use std::io;
use std::path::Path;

/// The marker's file name, in the payload store's root.
pub const FILE: &str = "STORE";

/// The marker spelling of this crate's format.
pub const BLOB_STORE_V1: &str = "blob-store-v1";

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
}
