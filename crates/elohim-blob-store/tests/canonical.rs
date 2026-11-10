//! One canonical byte location on owner and shared stores (artifact-store E4, review round 2).
//!
//! On a store opened with `open_owner` or `open_shared`, every byte CID's bytes live at exactly
//! one path, `chunks/<cid>`, whether the CID arrived as a standalone object or as a chunk of a
//! larger payload; a standalone object is a layout record and marker naming that file. The
//! unlocked store (`BlobStore::new`, elohim-storage's `/blob`) keeps the legacy representation.
#![cfg(unix)]

use std::collections::BTreeSet;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::Arc;

use elohim_blob_store::{BlobStore, BlobStoreError, Exists, MARKER_MAGIC};
use elohim_epr::address::{compute_bytes_cid, ByteCid};
use elohim_epr::profile::{chunk_bytes, ValidatedProfile};

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

fn path_of(root: &Path, area: &str, cid: &ByteCid) -> std::path::PathBuf {
    elohim_blob_store::fanout_path(&root.join(area), &cid.to_string())
}

/// The distinct inodes under `blobs/` and `chunks/` whose bytes are exactly `cid`'s: a hard
/// link counts once.
fn carriers(root: &Path, cid: &ByteCid) -> BTreeSet<(u64, u64)> {
    ["blobs", "chunks"]
        .iter()
        .map(|area| path_of(root, area, cid))
        .filter(|path| std::fs::read(path).is_ok_and(|held| &compute_bytes_cid(&held) == cid))
        .map(|path| {
            let meta = std::fs::metadata(path).unwrap();
            (meta.dev(), meta.ino())
        })
        .collect()
}

/// A multi-chunk payload and its first chunk.
fn payload_and_chunk(seed: u64) -> (Vec<u8>, Vec<u8>) {
    let payload = bytes(seed, 6 * 1024 * 1024);
    let cuts = chunk_bytes(&ValidatedProfile::EPOCH_0, &payload);
    assert!(cuts.len() > 1, "the fixture is multi-chunk");
    let (offset, len) = cuts[0];
    let chunk = payload[offset..offset + len].to_vec();
    (payload, chunk)
}

fn is_marker(path: &Path) -> bool {
    std::fs::read(path).is_ok_and(|held| held.starts_with(MARKER_MAGIC))
}

#[tokio::test]
async fn a_standalone_object_on_an_owner_or_shared_store_lives_at_its_chunk_path() {
    for owner in [true, false] {
        let dir = private_root();
        let store = if owner {
            BlobStore::open_owner(dir.path()).await.unwrap()
        } else {
            BlobStore::open_shared(dir.path()).await.unwrap()
        };
        let small = bytes(5, 4096);
        let cid = store.store(&small).await.unwrap().cid;
        assert_eq!(
            std::fs::read(path_of(dir.path(), "chunks", &cid)).unwrap(),
            small,
            "the bytes live at chunks/<cid>"
        );
        assert!(
            is_marker(&path_of(dir.path(), "blobs", &cid)),
            "blobs/<cid> is a marker naming them, not a copy"
        );
        assert_eq!(carriers(dir.path(), &cid).len(), 1);
        assert_eq!(&store.get(&cid).await.unwrap()[..], &small[..]);
        assert_eq!(store.exists(&cid).await, Exists::Verified);
        assert!(store.list_cids().unwrap().contains(&cid));
    }
}

#[tokio::test]
async fn the_unlocked_store_keeps_a_retained_inline_unit_listed_after_a_delete() {
    // Review round 3, ruling 3: the one behaviour of `new()` that differs from dev. A payload
    // whose chunk duplicates an inline object is deleted; the inline unit stays verified on disk
    // and stays listed (dev unlisted it until the next scrub).
    let dir = private_root();
    let store = BlobStore::new(dir.path()).await.unwrap();
    let (payload, chunk) = payload_and_chunk(23);
    let cid = compute_bytes_cid(&chunk);
    store.store(&chunk).await.unwrap();
    let whole = store.store(&payload).await.unwrap();
    store.delete(&whole.cid).await.unwrap();
    assert!(
        !path_of(dir.path(), "chunks", &cid).exists(),
        "the payload's own chunk file went"
    );
    assert_eq!(
        std::fs::read(path_of(dir.path(), "blobs", &cid)).unwrap(),
        chunk,
        "the inline object stays"
    );
    assert!(
        store.list_servable().unwrap().contains(&cid),
        "and stays listed"
    );
    assert!(store.holds_verified(&cid));
}

#[tokio::test]
async fn the_unlocked_store_keeps_the_legacy_representation() {
    for chunk_first in [true, false] {
        let dir = private_root();
        let store = BlobStore::new(dir.path()).await.unwrap();
        let (payload, chunk) = payload_and_chunk(9);
        let cid = compute_bytes_cid(&chunk);
        if chunk_first {
            store.store(&chunk).await.unwrap();
            store.store(&payload).await.unwrap();
        } else {
            store.store(&payload).await.unwrap();
            store.store(&chunk).await.unwrap();
        }
        // As dev has it: the standalone object inline, the chunk its own file.
        assert_eq!(
            std::fs::read(path_of(dir.path(), "blobs", &cid)).unwrap(),
            chunk,
            "chunk_first={chunk_first}: inline standalone object"
        );
        assert_eq!(
            std::fs::read(path_of(dir.path(), "chunks", &cid)).unwrap(),
            chunk,
            "chunk_first={chunk_first}: the payload's own chunk file"
        );
        assert!(!dir.path().join("index").join(cid.to_string()).exists());
    }
}

#[tokio::test]
async fn quarantining_a_standalone_object_leaves_a_payload_that_shares_its_bytes_readable() {
    for chunk_first in [true, false] {
        let dir = private_root();
        let owner = BlobStore::open_owner(dir.path()).await.unwrap();
        let (payload, chunk) = payload_and_chunk(11);
        let cid = compute_bytes_cid(&chunk);
        let whole = if chunk_first {
            owner.store(&chunk).await.unwrap();
            owner.store(&payload).await.unwrap()
        } else {
            let whole = owner.store(&payload).await.unwrap();
            owner.store(&chunk).await.unwrap();
            whole
        };
        owner.quarantine(&cid, "asked").await.unwrap();
        assert_eq!(
            owner.exists(&cid).await,
            Exists::Quarantined,
            "chunk_first={chunk_first}"
        );
        assert_eq!(
            &owner.get(&whole.cid).await.unwrap()[..],
            &payload[..],
            "chunk_first={chunk_first}: the payload keeps the good shared bytes"
        );
        assert_eq!(owner.exists(&whole.cid).await, Exists::Verified);
        assert_eq!(carriers(dir.path(), &cid).len(), 1);
        assert!(
            owner.list_servable().unwrap().contains(&cid),
            "still a verified unit"
        );
    }
}

#[tokio::test]
async fn quarantining_bytes_that_fail_leaves_every_layout_naming_them_damaged() {
    for chunk_first in [true, false] {
        let dir = private_root();
        let owner = BlobStore::open_owner(dir.path()).await.unwrap();
        let (payload, chunk) = payload_and_chunk(13);
        let cid = compute_bytes_cid(&chunk);
        let whole = if chunk_first {
            owner.store(&chunk).await.unwrap();
            owner.store(&payload).await.unwrap()
        } else {
            let whole = owner.store(&payload).await.unwrap();
            owner.store(&chunk).await.unwrap();
            whole
        };
        let path = path_of(dir.path(), "chunks", &cid);
        let mut held = std::fs::read(&path).unwrap();
        held[2] ^= 0x01;
        std::fs::write(&path, held).unwrap();
        owner.quarantine(&cid, "asked").await.unwrap();
        assert!(
            !path.exists(),
            "chunk_first={chunk_first}: the failing bytes left"
        );
        assert!(
            matches!(
                owner.get(&whole.cid).await,
                Err(BlobStoreError::HashMismatch { .. })
            ),
            "chunk_first={chunk_first}: the payload reads as damaged, typed"
        );
        assert_ne!(owner.exists(&whole.cid).await, Exists::Verified);
    }
}

#[tokio::test]
async fn a_delete_never_unlists_a_retained_verified_unit() {
    for delete_standalone_first in [true, false] {
        let dir = private_root();
        let owner = BlobStore::open_owner(dir.path()).await.unwrap();
        let (payload, chunk) = payload_and_chunk(17);
        let cid = compute_bytes_cid(&chunk);
        owner.store(&chunk).await.unwrap();
        let whole = owner.store(&payload).await.unwrap();
        let (first, survivor) = if delete_standalone_first {
            (cid, whole.cid)
        } else {
            (whole.cid, cid)
        };
        owner.delete(&first).await.unwrap();
        assert_eq!(carriers(dir.path(), &cid).len(), 1);
        assert!(
            owner.list_servable().unwrap().contains(&cid),
            "delete_standalone_first={delete_standalone_first}: the retained unit stays listed"
        );
        assert!(owner.holds_verified(&cid));
        assert_eq!(owner.exists(&survivor).await, Exists::Verified);
        owner.delete(&survivor).await.unwrap();
        assert!(
            carriers(dir.path(), &cid).is_empty(),
            "the last reference collects it"
        );
        assert!(!owner.list_servable().unwrap().contains(&cid));
    }
}

#[test]
fn concurrent_standalone_and_payload_arrivals_hold_one_copy() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    let (payload, chunk) = payload_and_chunk(19);
    let (payload, chunk) = (Arc::new(payload), Arc::new(chunk));
    let cid = compute_bytes_cid(&chunk);
    for round in 0..200 {
        let dir = private_root();
        runtime.block_on(async {
            // Two instances, two open file descriptions of ops.lock, racing on two threads.
            let a = Arc::new(BlobStore::open_shared(dir.path()).await.unwrap());
            let b = Arc::new(BlobStore::open_shared(dir.path()).await.unwrap());
            let gate = Arc::new(tokio::sync::Barrier::new(2));
            let standalone = {
                let (a, chunk, gate) = (a.clone(), chunk.clone(), gate.clone());
                tokio::spawn(async move {
                    gate.wait().await;
                    a.store(&chunk).await.unwrap()
                })
            };
            let whole = {
                let (b, payload, gate) = (b.clone(), payload.clone(), gate.clone());
                tokio::spawn(async move {
                    gate.wait().await;
                    b.store(&payload).await.unwrap()
                })
            };
            let (standalone, whole) = (standalone.await.unwrap(), whole.await.unwrap());
            assert_eq!(
                carriers(dir.path(), &cid).len(),
                1,
                "round {round}: one physical copy of the shared chunk"
            );
            for store in [&a, &b] {
                assert_eq!(&store.get(&standalone.cid).await.unwrap()[..], &chunk[..]);
                assert_eq!(&store.get(&whole.cid).await.unwrap()[..], &payload[..]);
            }
        });
    }
}
