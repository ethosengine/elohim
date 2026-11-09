//! One physical copy per byte CID (artifact-store E4 review, finding 3).
//!
//! A byte CID can reach the store two ways: as a standalone object (an inline payload,
//! `blobs/<cid>`) and as one chunk of a larger payload (`chunks/<cid>`). Whichever arrives
//! first keeps the bytes; the second reuses them and writes no second copy. These tests put the
//! same chunk both ways, in both orders, and follow reads, presence, listing and deletion.

use std::path::Path;

use elohim_blob_store::{BlobStore, Exists};
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
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    dir
}

/// How many files under `root/blobs` and `root/chunks` hold exactly the bytes `cid` names.
fn copies(root: &Path, cid: &ByteCid) -> usize {
    ["blobs", "chunks"]
        .iter()
        .map(|area| elohim_blob_store::fanout_path(&root.join(area), &cid.to_string()))
        .filter(|path| std::fs::read(path).is_ok_and(|held| &compute_bytes_cid(&held) == cid))
        .count()
}

/// A multi-chunk payload and its first chunk's bytes.
fn payload_and_chunk() -> (Vec<u8>, Vec<u8>) {
    let payload = bytes(31, 9 * 1024 * 1024);
    let cuts = chunk_bytes(&ValidatedProfile::EPOCH_0, &payload);
    assert!(cuts.len() > 1, "the fixture is multi-chunk");
    let (offset, len) = cuts[0];
    let chunk = payload[offset..offset + len].to_vec();
    (payload, chunk)
}

#[tokio::test]
async fn a_chunk_stored_standalone_after_its_payload_is_not_held_twice() {
    for owner in [false, true] {
        let dir = private_root();
        let store = if owner {
            BlobStore::open_owner(dir.path()).await.unwrap()
        } else {
            BlobStore::new(dir.path()).await.unwrap()
        };
        let (payload, chunk) = payload_and_chunk();
        let whole = store.store(&payload).await.unwrap();
        let standalone = store.store(&chunk).await.unwrap();
        let cid = compute_bytes_cid(&chunk);
        assert_eq!(standalone.cid, cid);
        assert_eq!(copies(dir.path(), &cid), 1, "one physical copy");

        // Both read whole, verify, and are listed.
        assert_eq!(&store.get(&cid).await.unwrap()[..], &chunk[..]);
        assert_eq!(&store.get_chunk(&cid).await.unwrap()[..], &chunk[..]);
        assert_eq!(store.size(&cid).await.unwrap(), chunk.len() as u64);
        assert_eq!(store.exists(&cid).await, Exists::Verified);
        assert!(store.present(&cid).await);
        assert_eq!(&store.get(&whole.cid).await.unwrap()[..], &payload[..]);
        let listed = store.list_cids().unwrap();
        assert!(listed.contains(&cid) && listed.contains(&whole.cid));
        assert!(store.store(&chunk).await.unwrap().already_existed);

        // Deleting the payload keeps the bytes the standalone object still names...
        store.delete(&whole.cid).await.unwrap();
        assert_eq!(&store.get(&cid).await.unwrap()[..], &chunk[..]);
        assert_eq!(copies(dir.path(), &cid), 1);
        // ...and deleting the standalone object then collects them.
        store.delete(&cid).await.unwrap();
        assert_eq!(store.exists(&cid).await, Exists::Absent);
        assert_eq!(copies(dir.path(), &cid), 0);
    }
}

#[tokio::test]
async fn a_payload_stored_after_its_chunk_reuses_the_standalone_copy() {
    for owner in [false, true] {
        let dir = private_root();
        let store = if owner {
            BlobStore::open_owner(dir.path()).await.unwrap()
        } else {
            BlobStore::new(dir.path()).await.unwrap()
        };
        let (payload, chunk) = payload_and_chunk();
        let cid = compute_bytes_cid(&chunk);
        store.store(&chunk).await.unwrap();
        let whole = store.store(&payload).await.unwrap();
        assert_eq!(copies(dir.path(), &cid), 1, "one physical copy");

        assert_eq!(&store.get(&whole.cid).await.unwrap()[..], &payload[..]);
        assert_eq!(store.exists(&whole.cid).await, Exists::Verified);
        assert!(store.present(&whole.cid).await);
        assert_eq!(&store.get(&cid).await.unwrap()[..], &chunk[..]);
        assert!(store.list_units().unwrap().contains(&cid));
        // A streamed store of the payload reuses it too.
        let streamed = store
            .store_stream(std::io::Cursor::new(payload.clone()))
            .await
            .unwrap();
        assert!(streamed.already_existed);
        assert_eq!(copies(dir.path(), &cid), 1);

        // Deleting the standalone object keeps the bytes the payload still names...
        store.delete(&cid).await.unwrap();
        assert!(!store.list_cids().unwrap().contains(&cid));
        assert_eq!(&store.get(&whole.cid).await.unwrap()[..], &payload[..]);
        assert_eq!(store.exists(&whole.cid).await, Exists::Verified);
        assert_eq!(copies(dir.path(), &cid), 1);
        // ...and deleting the payload then collects them.
        store.delete(&whole.cid).await.unwrap();
        assert_eq!(copies(dir.path(), &cid), 0);
        assert_eq!(store.stats().total_blobs, 0);
    }
}

#[tokio::test]
async fn a_standalone_copy_a_payload_reuses_is_verified_when_the_payload_is_read() {
    let dir = private_root();
    let store = BlobStore::new(dir.path()).await.unwrap();
    let (payload, chunk) = payload_and_chunk();
    let cid = compute_bytes_cid(&chunk);
    store.store(&chunk).await.unwrap();
    let whole = store.store(&payload).await.unwrap();
    // Spoil the one copy (same length): the payload that reuses it no longer verifies.
    let path = elohim_blob_store::fanout_path(&dir.path().join("blobs"), &cid.to_string());
    let mut held = std::fs::read(&path).unwrap();
    held[0] ^= 0x01;
    std::fs::write(&path, held).unwrap();
    assert!(store.get(&whole.cid).await.is_err());
    assert_ne!(store.exists(&whole.cid).await, Exists::Verified);
    // Fresh bytes heal both.
    store.store(&payload).await.unwrap();
    assert_eq!(&store.get(&whole.cid).await.unwrap()[..], &payload[..]);
}
