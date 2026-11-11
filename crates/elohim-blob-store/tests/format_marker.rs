//! The format marker decides who may open a root (artifact-store E4, review round 4).
//!
//! An owner or shared open accepts a root only when it is empty or absent (and then claims it by
//! writing `STORE` = `blob-store-v1` before anything else) or when its marker already says
//! `blob-store-v1`; anything else is refused before any write. The unlocked store refuses a root
//! carrying any marker. Inside a marked root every `blobs/` file is a marker by construction, so
//! one that is not is corruption: the owner quarantines it, a shared instance refuses that unit
//! and leaves it in place. It is never read as an inline payload.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use elohim_blob_store::{marker, BlobStore, BlobStoreError, Exists, MARKER_MAGIC};
use elohim_epr::address::{compute_bytes_cid, compute_record_cid, ByteCid};

fn bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut out = Vec::with_capacity(len + 8);
    while out.len() < len {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        out.extend_from_slice(&state.wrapping_mul(0x2545_F491_4F6C_DD1D).to_le_bytes());
    }
    out.truncate(len);
    out
}

fn private_root() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    dir
}

fn blob_path(root: &Path, cid: &ByteCid) -> PathBuf {
    elohim_blob_store::fanout_path(&root.join("blobs"), &cid.to_string())
}

/// Every path under `root` with its bytes (`None` for a directory): any write, move or
/// removal changes it.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn visit(dir: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                out.insert(path.clone(), None);
                visit(&path, out);
            } else {
                out.insert(path.clone(), Some(std::fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, &mut out);
    out
}

/// The refusal of an unmarked, non-empty root.
fn refused_unmarked(root: &Path, result: &Result<BlobStore, BlobStoreError>) -> bool {
    matches!(result, Err(BlobStoreError::LegacyLayout { path }) if path == root)
}

/// The unlocked store's refusal of a marked root.
fn refused_marked(result: &Result<BlobStore, BlobStoreError>) -> bool {
    matches!(result, Err(BlobStoreError::MarkedRoot { .. }))
}

/// Bytes shaped exactly like a marker (magic and a valid record CID) that are stored as their
/// own payload, so they hash to the name they are filed under.
fn marker_shaped_payload() -> Vec<u8> {
    let mut shaped = MARKER_MAGIC.to_vec();
    shaped.extend_from_slice(&compute_record_cid(b"any layout record").to_bytes());
    shaped
}

async fn open(root: &Path, owner: bool) -> Result<BlobStore, BlobStoreError> {
    if owner {
        BlobStore::open_owner(root).await
    } else {
        BlobStore::open_shared(root).await
    }
}

/// What an unmarked root holds.
#[derive(Debug, Clone, Copy)]
enum Unmarked {
    /// A whole inline object, written by the unlocked store.
    WholeInline,
    /// The same, with its bytes then damaged in place.
    CorruptInline,
    /// Bytes shaped exactly like a marker, stored by the unlocked store as their own payload.
    MarkerShaped,
    /// A `blobs/` entry under a name that is not a byte CID.
    StrayName,
    /// A file whose name only begins like the marker's staging (review round 5, ruling 1).
    StoreLookalikeFile,
    /// A populated directory under such a name.
    StoreLookalikeDir,
}

async fn plant(root: &Path, case: Unmarked) {
    match case {
        Unmarked::WholeInline => {
            let chunk = bytes(7, 4096);
            BlobStore::new(root)
                .await
                .unwrap()
                .store(&chunk)
                .await
                .unwrap();
        }
        Unmarked::CorruptInline => {
            let chunk = bytes(8, 4096);
            let cid = compute_bytes_cid(&chunk);
            BlobStore::new(root)
                .await
                .unwrap()
                .store(&chunk)
                .await
                .unwrap();
            std::fs::write(blob_path(root, &cid), bytes(9, 4096)).unwrap();
        }
        Unmarked::MarkerShaped => {
            let shaped = marker_shaped_payload();
            let store = BlobStore::new(root).await.unwrap();
            let stored = store.store(&shaped).await.unwrap();
            assert_eq!(&store.get(&stored.cid).await.unwrap()[..], &shaped[..]);
        }
        Unmarked::StrayName => {
            std::fs::create_dir_all(root.join("blobs").join("zzzz")).unwrap();
            std::fs::write(root.join("blobs").join("zzzz").join("not-a-cid"), b"x").unwrap();
        }
        Unmarked::StoreLookalikeFile => {
            std::fs::write(root.join(".STORE-data"), bytes(10, 4096)).unwrap();
        }
        Unmarked::StoreLookalikeDir => {
            std::fs::create_dir(root.join(".STORE-x")).unwrap();
            std::fs::write(root.join(".STORE-x").join("inside"), b"content").unwrap();
        }
    }
}

#[tokio::test]
async fn an_unmarked_root_that_is_not_empty_is_refused_by_owner_and_shared_and_left_untouched() {
    for case in [
        Unmarked::WholeInline,
        Unmarked::CorruptInline,
        Unmarked::MarkerShaped,
        Unmarked::StrayName,
        Unmarked::StoreLookalikeFile,
        Unmarked::StoreLookalikeDir,
    ] {
        for owner in [true, false] {
            let dir = private_root();
            plant(dir.path(), case).await;
            let before = snapshot(dir.path());
            let result = open(dir.path(), owner).await;
            assert!(
                refused_unmarked(dir.path(), &result),
                "{case:?}, owner={owner}: opened or refused otherwise: {:?}",
                result.map(|_| ())
            );
            assert_eq!(
                snapshot(dir.path()),
                before,
                "{case:?}, owner={owner}: nothing written"
            );
        }
    }
}

#[tokio::test]
async fn the_unlocked_store_refuses_a_marked_root_and_writes_nothing() {
    for found in [marker::BLOB_STORE_V1, "directory-v1"] {
        let dir = private_root();
        if found == marker::BLOB_STORE_V1 {
            let owner = BlobStore::open_owner(dir.path()).await.unwrap();
            owner.store(&bytes(3, 4096)).await.unwrap();
        } else {
            std::fs::write(dir.path().join(marker::FILE), format!("{found}\n")).unwrap();
        }
        let before = snapshot(dir.path());
        let result = BlobStore::new(dir.path()).await;
        assert!(refused_marked(&result), "{found}: {:?}", result.map(|_| ()));
        assert_eq!(snapshot(dir.path()), before, "{found}: nothing written");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_first_opens_by_an_owner_and_a_shared_instance_claim_one_marker() {
    for round in 0..20 {
        let dir = private_root();
        // Half the rounds open a root that does not exist yet.
        let root = if round % 2 == 0 {
            dir.path().to_path_buf()
        } else {
            dir.path().join("payload")
        };
        let (owner, shared) = tokio::join!(
            {
                let root = root.clone();
                tokio::spawn(async move { BlobStore::open_owner(&root).await })
            },
            {
                let root = root.clone();
                tokio::spawn(async move { BlobStore::open_shared(&root).await })
            },
        );
        let (owner, shared) = (owner.unwrap(), shared.unwrap());
        assert!(
            owner.is_ok(),
            "round {round}: owner {:?}",
            owner.map(|_| ())
        );
        assert!(
            shared.is_ok(),
            "round {round}: shared {:?}",
            shared.map(|_| ())
        );
        assert_eq!(
            marker::read(&root).unwrap().as_deref(),
            Some(marker::BLOB_STORE_V1),
            "round {round}"
        );
        let markers: Vec<String> = std::fs::read_dir(&root)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with("STORE") || name.starts_with(".STORE"))
            .collect();
        assert_eq!(markers, vec![marker::FILE.to_string()], "round {round}");
    }
}

#[tokio::test]
async fn a_stray_blobs_name_in_a_marked_root_is_quarantined_by_the_owner_and_left_by_a_shared_one()
{
    let dir = private_root();
    let chunk = bytes(11, 4096);
    let cid = compute_bytes_cid(&chunk);
    BlobStore::open_owner(dir.path())
        .await
        .unwrap()
        .store(&chunk)
        .await
        .unwrap();
    let blobs = dir.path().join("blobs");
    std::fs::create_dir_all(blobs.join("zzzz")).unwrap();
    let strays = [
        blobs.join("zzzz").join("not-a-cid"),
        blobs.join("at-the-top"),
        // A byte CID, but not in its own bucket: nothing would ever read or list it.
        blobs
            .join("zzzz")
            .join(compute_bytes_cid(b"elsewhere").to_string()),
    ];
    for stray in &strays {
        std::fs::write(stray, b"stray").unwrap();
    }

    let shared = BlobStore::open_shared(dir.path()).await.unwrap();
    assert_eq!(&shared.get(&cid).await.unwrap()[..], &chunk[..]);
    drop(shared);
    for stray in &strays {
        assert!(
            stray.exists(),
            "a shared instance moved {}",
            stray.display()
        );
    }

    let owner = BlobStore::open_owner(dir.path()).await.unwrap();
    for stray in &strays {
        assert!(!stray.exists(), "the owner left {}", stray.display());
    }
    let quarantined: Vec<String> = std::fs::read_dir(dir.path().join("quarantine"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("unnamed-"))
        .collect();
    assert_eq!(quarantined.len(), strays.len(), "{quarantined:?}");
    assert_eq!(&owner.get(&cid).await.unwrap()[..], &chunk[..]);
}

#[tokio::test]
async fn a_damaged_entry_under_a_byte_cid_is_corruption_never_an_inline_payload() {
    let dir = private_root();
    let chunk = bytes(12, 4096);
    let cid = compute_bytes_cid(&chunk);
    let zero = compute_bytes_cid(b"a payload that is not empty");
    BlobStore::open_owner(dir.path())
        .await
        .unwrap()
        .store(&chunk)
        .await
        .unwrap();
    // The legacy inline representation of the same bytes where the marker belongs, and a
    // zero-length file under a byte CID that is not the empty payload's.
    std::fs::write(blob_path(dir.path(), &cid), &chunk).unwrap();
    std::fs::create_dir_all(blob_path(dir.path(), &zero).parent().unwrap()).unwrap();
    std::fs::write(blob_path(dir.path(), &zero), b"").unwrap();

    let shared = Arc::new(BlobStore::open_shared(dir.path()).await.unwrap());
    match shared.get(&cid).await {
        Err(BlobStoreError::HashMismatch { .. }) => {}
        other => panic!(
            "a shared instance read a damaged entry: {:?}",
            other.map(|b| b.len())
        ),
    }
    assert_eq!(shared.exists(&cid).await, Exists::Corrupt);
    assert_eq!(
        std::fs::read(blob_path(dir.path(), &cid)).unwrap(),
        chunk,
        "a shared instance moved nothing"
    );
    drop(shared);

    let owner = Arc::new(BlobStore::open_owner(dir.path()).await.unwrap());
    {
        let owner = owner.clone();
        tokio::task::spawn_blocking(move || owner.scrub())
            .await
            .unwrap()
            .unwrap();
    }
    for damaged in [&cid, &zero] {
        assert!(
            !blob_path(dir.path(), damaged).exists(),
            "the owner left the damaged entry {damaged}"
        );
        assert_eq!(
            owner.exists(damaged).await,
            Exists::Quarantined,
            "{damaged}"
        );
    }
    // The bytes themselves were never the defect: storing the object again republishes it.
    owner.store(&chunk).await.unwrap();
    assert_eq!(&owner.get(&cid).await.unwrap()[..], &chunk[..]);
}

#[tokio::test]
async fn a_stale_claim_temp_is_tolerated_and_removed_only_by_the_owner() {
    for owner_first in [false, true] {
        let dir = private_root();
        // A claim that crashed after staging its marker and before linking it.
        let temp = dir.path().join(format!(
            "{}{}",
            marker::CLAIM_TEMP_PREFIX,
            "0123456789abcdef".repeat(2)
        ));
        std::fs::write(&temp, format!("{}\n", marker::BLOB_STORE_V1)).unwrap();
        if !owner_first {
            let shared = BlobStore::open_shared(dir.path()).await.unwrap();
            shared.store(&bytes(13, 4096)).await.unwrap();
            assert!(temp.exists(), "a shared instance removed the temp");
        }
        let _owner = BlobStore::open_owner(dir.path()).await.unwrap();
        assert!(
            !temp.exists(),
            "owner_first={owner_first}: the owner left the temp"
        );
        assert_eq!(
            marker::read(dir.path()).unwrap().as_deref(),
            Some(marker::BLOB_STORE_V1)
        );
    }
}
