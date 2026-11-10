//! The device's payload store: which format a payload directory holds, and opening it
//! (artifact-store E4).
//!
//! A payload directory names its format in a marker, `root/STORE` ([`MARKER_FILE`]), holding
//! [`PayloadFormat::BlobStoreV1`]'s `blob-store-v1` or [`PayloadFormat::DirectoryV1`]'s
//! `directory-v1`. Every process that opens the directory reads the marker first and opens that
//! format or refuses ([`PayloadStoreError::Format`]); none ever creates a second store beside the
//! first.
//!
//! - **A marked directory** opens in its marker's format, if this binary links it.
//! - **An unmarked directory** is every device's payload directory from before the marker. Its
//!   layout decides, and nothing is written to it: flat `root/<CID>` objects (eprfs-storage's
//!   `DirectoryStorage`, its `.tmp*` staging files included) open as [`PayloadFormat::DirectoryV1`]
//!   exactly as before; a blob-store layout (`blobs/`, `chunks/`, `index/`, …) without a marker,
//!   a mix of both, or entries that are neither are refused ([`PayloadStoreError::Layout`]).
//! - **A new directory** (absent, or empty) selects its format before any layout exists:
//!   [`PayloadFormat::BlobStoreV1`] when this binary is built with feature `blob-store`, else
//!   [`PayloadFormat::DirectoryV1`]. A new blob-store directory is claimed by the store's own open,
//!   which writes the marker before any other file; a new directory-store one by
//!   [`select_format`]. Both claim through `elohim_blob_store::marker::claim`, the one
//!   implementation of the marker: it is staged under a reserved `.STORE-` name and hard-linked
//!   into place, so first openers race to one winner and a loser opens the winner's format. A
//!   crash between the marker and the layout leaves a marked, empty store that the next open
//!   completes.
//!
//! [`PayloadFormat::BlobStoreV1`] is `elohim-blob-store`'s owner/shared store: `epr peer serve`
//! opens it as the owner ([`Access::Owner`]), the daemon-down `epr artifact put` as a shared
//! instance ([`Access::Shared`]), and elohim-storage composes a shared tier over the same root
//! from `EPR_PAYLOAD_STORE`. [`PayloadFormat::DirectoryV1`] opens through
//! `eprfs_peer::open_payload_store`, unchanged.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use elohim_blob_store::marker;
use eprfs_core::{BlobCid, EprfsStorage};

/// The marker's file name, in the payload directory's root.
pub const MARKER_FILE: &str = marker::FILE;

/// The names a blob-store layout puts in its root.
const BLOB_STORE_NAMES: [&str; 8] = [
    "blobs",
    "chunks",
    "index",
    "records",
    "quarantine",
    "tmp",
    "owner.lock",
    "ops.lock",
];

/// The formats a payload directory can hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadFormat {
    /// `elohim-blob-store`: chunked payloads behind markers, verify-on-read, owner/shared.
    BlobStoreV1,
    /// eprfs-storage's flat `DirectoryStorage`: one file per object, named by its CID.
    DirectoryV1,
}

impl PayloadFormat {
    /// The marker spelling.
    pub const fn marker(self) -> &'static str {
        match self {
            Self::BlobStoreV1 => marker::BLOB_STORE_V1,
            Self::DirectoryV1 => "directory-v1",
        }
    }

    /// The format a marker names, if it names one this source knows.
    pub fn parse(marker: &str) -> Option<Self> {
        [Self::BlobStoreV1, Self::DirectoryV1]
            .into_iter()
            .find(|format| format.marker() == marker)
    }

    /// Whether this binary links the format's backend.
    pub const fn linked(self) -> bool {
        match self {
            Self::BlobStoreV1 => cfg!(feature = "blob-store"),
            Self::DirectoryV1 => true,
        }
    }

    /// Every format this binary opens.
    pub fn supported() -> Vec<Self> {
        [Self::BlobStoreV1, Self::DirectoryV1]
            .into_iter()
            .filter(|format| format.linked())
            .collect()
    }

    /// The format a new payload directory is created in.
    pub const fn for_new_store() -> Self {
        if cfg!(feature = "blob-store") {
            Self::BlobStoreV1
        } else {
            Self::DirectoryV1
        }
    }
}

impl fmt::Display for PayloadFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.marker())
    }
}

/// How a process opens the payload store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// The one instance that may move or remove files (`epr peer serve`).
    Owner,
    /// Stores and reads only (`epr artifact put`).
    Shared,
}

/// An opened payload store.
pub struct PayloadStore {
    /// The format the directory holds.
    pub format: PayloadFormat,
    /// The directory.
    pub root: PathBuf,
    /// The store, as the eprfs tier every payload consumer speaks.
    pub store: Arc<dyn EprfsStorage>,
}

/// Why a payload store did not open.
#[derive(Debug, thiserror::Error)]
pub enum PayloadStoreError {
    /// The marker names a format this binary does not link (or one no source knows).
    #[error(
        "the payload store at {} is `{found}`, which this epr does not open (it opens: {}); \
         refused rather than open a second store beside it",
        root.display(),
        supported.iter().map(|format| format.marker()).collect::<Vec<_>>().join(", ")
    )]
    Format {
        root: PathBuf,
        found: String,
        supported: Vec<PayloadFormat>,
    },

    /// An unmarked directory whose layout is not a legacy directory store.
    #[error("the payload store at {} is refused: {reason}", root.display())]
    Layout { root: PathBuf, reason: String },

    /// The filesystem refused.
    #[error("payload store {}: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    /// The format's backend refused to open.
    #[error("the {format} payload store at {} does not open: {reason}", root.display())]
    Open {
        root: PathBuf,
        format: PayloadFormat,
        reason: String,
    },
}

fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> PayloadStoreError + '_ {
    move |source| PayloadStoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// What an unopened payload directory holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Detected {
    /// Its marker says this (trimmed).
    Marked(String),
    /// No marker, and a legacy flat directory store.
    UnmarkedDirectory,
    /// Absent, or empty: a new store.
    New,
}

/// Read a payload directory's marker or, unmarked, classify its layout. Writes nothing.
///
/// # Errors
///
/// [`PayloadStoreError::Layout`] for an unmarked blob-store, mixed or unrecognisable layout;
/// [`PayloadStoreError::Io`] when the directory cannot be read.
pub fn detect(root: &Path) -> Result<Detected, PayloadStoreError> {
    detect_with(root, &|| {})
}

/// [`detect`], calling `after_marker_read` between the first marker read and the scan (the
/// window a racing first open publishes its marker in; tests widen it).
fn detect_with(root: &Path, after_marker_read: &dyn Fn()) -> Result<Detected, PayloadStoreError> {
    if let Some(marker) = read_marker(root)? {
        return Ok(Detected::Marked(marker));
    }
    after_marker_read();
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Detected::New),
        Err(error) => return Err(io_error(root)(error)),
    };
    let (mut blob_store, mut legacy, mut other) = (Vec::new(), false, Vec::new());
    for entry in entries {
        let name = entry.map_err(io_error(root))?.file_name();
        let name = name.to_string_lossy();
        if marker::is_staging(&name) {
            // Another opener is selecting the format: it is a new store.
            continue;
        }
        if BLOB_STORE_NAMES.contains(&name.as_ref()) {
            blob_store.push(name.into_owned());
        } else if name.starts_with(".tmp") || BlobCid::parse(&name).is_ok() {
            legacy = true;
        } else {
            other.push(name.into_owned());
        }
    }
    // The marker is authoritative. A first open racing this one may have published it (and
    // built its layout) between the read above and the scan, so it is read again before an
    // unmarked layout is classified or refused.
    if let Some(marker) = read_marker(root)? {
        return Ok(Detected::Marked(marker));
    }
    let refused = |reason: String| {
        Err(PayloadStoreError::Layout {
            root: root.to_path_buf(),
            reason,
        })
    };
    match (blob_store.is_empty(), legacy, other.is_empty()) {
        (true, false, true) => Ok(Detected::New),
        (true, true, _) => Ok(Detected::UnmarkedDirectory),
        (false, true, _) => refused(format!(
            "it mixes a blob-store layout ({}) with flat directory objects and has no {MARKER_FILE} \
             marker",
            blob_store.join(", ")
        )),
        (false, false, _) => refused(format!(
            "it holds a blob-store layout ({}) but no {MARKER_FILE} marker",
            blob_store.join(", ")
        )),
        (true, false, false) => refused(format!(
            "it holds neither a {MARKER_FILE} marker nor a recognisable store (found: {})",
            other.join(", ")
        )),
    }
}

/// The marker's trimmed content; `None` when there is none. A marker another opener has just
/// linked is complete (it is written before the link), so an empty one is refused as found.
fn read_marker(root: &Path) -> Result<Option<String>, PayloadStoreError> {
    marker::read(root).map_err(io_error(&root.join(MARKER_FILE)))
}

/// Select a new payload directory's format: claim it for `wanted` unless another opener got
/// there first, and return the marker that won (`elohim_blob_store::marker::claim`: exactly one
/// opener's marker is ever installed, and it is complete when it appears).
///
/// # Errors
///
/// [`PayloadStoreError::Io`] when the root or the marker cannot be written.
pub fn select_format(root: &Path, wanted: PayloadFormat) -> Result<String, PayloadStoreError> {
    create_private_dir(root)?;
    marker::claim(root, wanted.marker()).map_err(io_error(&root.join(MARKER_FILE)))
}

/// `root` (and any missing parent) as a directory private to this account (0700).
fn create_private_dir(root: &Path) -> Result<(), PayloadStoreError> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(root).map_err(io_error(root))
}

/// The format `root` holds or would be created in, without opening or writing anything.
///
/// # Errors
///
/// As [`detect`], and [`PayloadStoreError::Format`] for a marker this binary cannot open.
pub fn format_of(root: &Path) -> Result<PayloadFormat, PayloadStoreError> {
    match detect(root)? {
        Detected::Marked(marker) => linked_format(root, &marker),
        Detected::UnmarkedDirectory => Ok(PayloadFormat::DirectoryV1),
        Detected::New => Ok(PayloadFormat::for_new_store()),
    }
}

fn linked_format(root: &Path, marker: &str) -> Result<PayloadFormat, PayloadStoreError> {
    match PayloadFormat::parse(marker) {
        Some(format) if format.linked() => Ok(format),
        _ => Err(PayloadStoreError::Format {
            root: root.to_path_buf(),
            found: marker.to_string(),
            supported: PayloadFormat::supported(),
        }),
    }
}

/// Open the payload store at `root` (see the module docs), as `access`'s instance. Call it
/// inside the Tokio runtime the store will be used from: a blob-store backend runs its
/// filesystem work on that runtime's blocking pool.
///
/// # Errors
///
/// [`PayloadStoreError::Format`] for a marker this binary does not open, and nothing is
/// created; [`PayloadStoreError::Layout`] for an unmarked layout that is not a legacy directory
/// store; [`PayloadStoreError::Open`] when the backend refuses (a second owner, a root that is
/// a symlink or open to group/other).
pub async fn open_payload_store(
    root: &Path,
    access: Access,
) -> Result<PayloadStore, PayloadStoreError> {
    open_with(root, access, &|| {}).await
}

/// [`open_payload_store`] with [`detect_with`]'s hook.
async fn open_with(
    root: &Path,
    access: Access,
    after_marker_read: &dyn Fn(),
) -> Result<PayloadStore, PayloadStoreError> {
    let (format, store) = match detect_with(root, after_marker_read)? {
        Detected::Marked(marker) => {
            let format = linked_format(root, &marker)?;
            (format, open_backend(root, format, access).await?)
        }
        Detected::UnmarkedDirectory => (
            PayloadFormat::DirectoryV1,
            open_backend(root, PayloadFormat::DirectoryV1, access).await?,
        ),
        Detected::New => open_new(root, access).await?,
    };
    Ok(PayloadStore {
        format,
        root: root.to_path_buf(),
        store,
    })
}

/// Open a new (absent or empty) payload directory in [`PayloadFormat::for_new_store`]. A blob
/// store claims its own root as it opens (it writes the marker before any other file), so the
/// marker is written once. When another first open claimed the root for a different format, the
/// root opens in the winner's format.
async fn open_new(
    root: &Path,
    access: Access,
) -> Result<(PayloadFormat, Arc<dyn EprfsStorage>), PayloadStoreError> {
    let wanted = PayloadFormat::for_new_store();
    if wanted == PayloadFormat::DirectoryV1 {
        let format = linked_format(root, &select_format(root, wanted)?)?;
        return Ok((format, open_backend(root, format, access).await?));
    }
    match open_backend(root, wanted, access).await {
        Ok(store) => Ok((wanted, store)),
        Err(refused) => match read_marker(root)? {
            Some(won) if won != wanted.marker() => {
                let format = linked_format(root, &won)?;
                Ok((format, open_backend(root, format, access).await?))
            }
            _ => Err(refused),
        },
    }
}

async fn open_backend(
    root: &Path,
    format: PayloadFormat,
    access: Access,
) -> Result<Arc<dyn EprfsStorage>, PayloadStoreError> {
    let refused = |reason: String| PayloadStoreError::Open {
        root: root.to_path_buf(),
        format,
        reason,
    };
    match format {
        // Exactly as before the marker: the directory store has no owner/shared split.
        PayloadFormat::DirectoryV1 => {
            let _ = access;
            eprfs_peer::open_payload_store(root).map_err(|error| refused(error.to_string()))
        }
        #[cfg(feature = "blob-store")]
        PayloadFormat::BlobStoreV1 => {
            let mode = match access {
                Access::Owner => elohim_blob_store::OpenMode::Owner,
                Access::Shared => elohim_blob_store::OpenMode::Shared,
            };
            let tier = elohim_blob_store::tier::BlobStoreTier::at(root, mode)
                .await
                .map_err(|error| refused(error.to_string()))?;
            Ok(Arc::new(tier))
        }
        #[cfg(not(feature = "blob-store"))]
        PayloadFormat::BlobStoreV1 => Err(refused("this epr does not link the blob store".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_round_trip_and_only_linked_formats_are_supported() {
        for format in [PayloadFormat::BlobStoreV1, PayloadFormat::DirectoryV1] {
            assert_eq!(PayloadFormat::parse(format.marker()), Some(format));
        }
        assert_eq!(PayloadFormat::parse("directory-v2"), None);
        assert!(PayloadFormat::supported().contains(&PayloadFormat::DirectoryV1));
        assert_eq!(
            PayloadFormat::supported().contains(&PayloadFormat::BlobStoreV1),
            cfg!(feature = "blob-store")
        );
        assert!(PayloadFormat::for_new_store().linked());
    }

    #[cfg(feature = "blob-store")]
    #[test]
    fn the_blob_store_marker_is_the_one_storage_composes_from() {
        assert_eq!(MARKER_FILE, elohim_blob_store::marker::FILE);
        assert_eq!(
            PayloadFormat::BlobStoreV1.marker(),
            elohim_blob_store::marker::BLOB_STORE_V1
        );
        for name in [elohim_blob_store::OWNER_LOCK, elohim_blob_store::OPS_LOCK] {
            assert!(BLOB_STORE_NAMES.contains(&name), "{name}");
        }
    }

    #[test]
    fn an_open_that_loses_the_first_open_race_opens_the_winners_format() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("payload");
        create_private_dir(&root).unwrap();
        let winner = PayloadFormat::for_new_store();
        // Between this open's marker read and its scan, another first open selects the
        // format and builds its layout.
        let race = || {
            assert_eq!(select_format(&root, winner).unwrap(), winner.marker());
            if winner == PayloadFormat::BlobStoreV1 {
                for area in ["blobs", "chunks", "index", "records", "quarantine", "tmp"] {
                    std::fs::create_dir_all(root.join(area)).unwrap();
                }
                std::fs::write(root.join("ops.lock"), b"").unwrap();
            }
        };
        let opened = runtime
            .block_on(open_with(&root, Access::Shared, &race))
            .unwrap_or_else(|error| panic!("the race's loser refused the winner: {error}"));
        assert_eq!(opened.format, winner);
        assert_eq!(
            detect(&root).unwrap(),
            Detected::Marked(winner.marker().into())
        );
    }
}
