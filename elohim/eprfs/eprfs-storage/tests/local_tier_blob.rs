use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use eprfs_core::{
    AttestationDraft, BlobCid, BlobHandle, BlobPresence, DerivationBasis, DerivationKey,
    DerivationRecord, EprRecord, EprRef, EprfsStorage, FetchPolicy, KeyInput, Witness,
};
use eprfs_storage::{
    put_tier_blob, read_tier_blob, DirectoryStorage, TierBlobDraw, TierBlobUnavailable,
    TIER_BLOB_CHUNK_BYTES,
};
use tokio::sync::Notify;

fn record_for(bytes: &[u8]) -> DerivationRecord {
    DerivationRecord {
        key: DerivationKey {
            method: BlobCid::compute_raw(b"fixture-method"),
            input: KeyInput::Raw(BlobCid::compute_raw(b"fixture-input")),
        },
        output: BlobCid::compute_raw(bytes),
        computed_by: "uhCAkFixtureDevice".into(),
        basis: DerivationBasis::Computed,
        deterministic: true,
        witness: Witness::Signature {
            sig: "fixture-signature".into(),
        },
    }
}

fn two_chunk_bytes() -> Bytes {
    let mut bytes = vec![b'a'; TIER_BLOB_CHUNK_BYTES];
    bytes.extend_from_slice(b"second chunk");
    Bytes::from(bytes)
}

#[tokio::test]
async fn local_put_chunks_at_one_mib_and_records_each_cid_in_a_manifest() {
    let temp = tempfile::tempdir().unwrap();
    let store = DirectoryStorage::create(temp.path().join("objects")).unwrap();
    let bytes = two_chunk_bytes();

    let put = put_tier_blob(&store, record_for(&bytes), bytes.clone())
        .await
        .unwrap();

    assert_eq!(put.manifest.record.output, BlobCid::compute_raw(&bytes));
    assert_eq!(put.manifest.record.computed_by, "uhCAkFixtureDevice");
    assert_eq!(
        put.manifest.record.key.method,
        BlobCid::compute_raw(b"fixture-method")
    );
    assert_eq!(
        put.manifest.record.key.input,
        KeyInput::Raw(BlobCid::compute_raw(b"fixture-input"))
    );
    assert_eq!(put.manifest.byte_len, bytes.len() as u64);
    assert_eq!(put.manifest.chunks.len(), 2);
    assert_eq!(
        put.manifest.chunks[0].byte_len,
        TIER_BLOB_CHUNK_BYTES as u32
    );
    assert_eq!(put.manifest.chunks[1].byte_len, 12);
    for chunk in &put.manifest.chunks {
        assert!(temp
            .path()
            .join("objects")
            .join(chunk.cid.to_string())
            .is_file());
    }
    assert!(temp
        .path()
        .join("objects")
        .join(put.manifest_cid.to_string())
        .is_file());
    assert_eq!(
        read_tier_blob(&store, &put.manifest_cid).await,
        TierBlobDraw::Hit {
            manifest: put.manifest,
            bytes,
        }
    );
}

#[tokio::test]
async fn one_device_wipe_is_a_typed_miss_and_reput_recovers_identical_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("objects");
    let store = DirectoryStorage::create(&root).unwrap();
    let bytes = two_chunk_bytes();
    let record = record_for(&bytes);
    let put = put_tier_blob(&store, record.clone(), bytes.clone())
        .await
        .unwrap();

    std::fs::remove_dir_all(&root).unwrap();
    let empty = DirectoryStorage::create(&root).unwrap();
    let draw = read_tier_blob(&empty, &put.manifest_cid).await;
    assert!(matches!(draw, TierBlobDraw::Miss { .. }));
    assert_eq!(draw.reason().as_deref(), Some("manifest is absent"));

    let recovered = put_tier_blob(&empty, record, bytes.clone()).await.unwrap();
    assert_eq!(recovered.manifest_cid, put.manifest_cid);
    assert_eq!(
        read_tier_blob(&empty, &put.manifest_cid).await.bytes(),
        Some(&bytes)
    );
}

#[tokio::test]
async fn one_device_corruption_never_returns_bytes_and_reput_recovers_identically() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("objects");
    let store = DirectoryStorage::create(&root).unwrap();
    let bytes = two_chunk_bytes();
    let record = record_for(&bytes);
    let put = put_tier_blob(&store, record.clone(), bytes.clone())
        .await
        .unwrap();
    let corrupt = &put.manifest.chunks[0].cid;
    std::fs::write(root.join(corrupt.to_string()), b"corrupt").unwrap();

    let draw = read_tier_blob(&store, &put.manifest_cid).await;
    assert_eq!(draw.bytes(), None);
    assert!(matches!(
        draw,
        TierBlobDraw::Unavailable {
            why: TierBlobUnavailable::ChunkCorrupt { ref cid },
            ..
        } if cid == corrupt
    ));
    assert!(draw.reason().unwrap().contains("chunk is corrupt"));

    std::fs::remove_file(root.join(corrupt.to_string())).unwrap();
    let recovered = put_tier_blob(&store, record, bytes.clone()).await.unwrap();
    assert_eq!(recovered.manifest_cid, put.manifest_cid);
    assert_eq!(
        read_tier_blob(&store, &put.manifest_cid).await.bytes(),
        Some(&bytes)
    );
}

#[derive(Clone)]
struct BlockingFetch {
    inner: DirectoryStorage,
    calls: Arc<AtomicUsize>,
    blocked: Arc<Notify>,
}

#[async_trait]
impl EprfsStorage for BlockingFetch {
    async fn resolve_epr(&self, reference: &EprRef) -> eprfs_core::Result<EprRecord> {
        self.inner.resolve_epr(reference).await
    }

    async fn has_blob(&self, cid: &BlobCid) -> eprfs_core::Result<BlobPresence> {
        self.inner.has_blob(cid).await
    }

    async fn fetch_blob(
        &self,
        cid: &BlobCid,
        policy: FetchPolicy,
    ) -> eprfs_core::Result<BlobHandle> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if call == 2 {
            self.blocked.notify_one();
            return std::future::pending().await;
        }
        self.inner.fetch_blob(cid, policy).await
    }

    async fn put_blob(&self, bytes: Bytes) -> eprfs_core::Result<BlobCid> {
        self.inner.put_blob(bytes).await
    }

    async fn put_blob_verified(&self, cid: &BlobCid, bytes: Bytes) -> eprfs_core::Result<()> {
        self.inner.put_blob_verified(cid, bytes).await
    }

    async fn publish_attestation(&self, draft: AttestationDraft) -> eprfs_core::Result<EprRef> {
        self.inner.publish_attestation(draft).await
    }
}

#[tokio::test]
async fn one_device_killed_fetch_exposes_no_partial_hit_and_retry_is_identical() {
    let temp = tempfile::tempdir().unwrap();
    let store = DirectoryStorage::create(temp.path().join("objects")).unwrap();
    let bytes = two_chunk_bytes();
    let put = put_tier_blob(&store, record_for(&bytes), bytes.clone())
        .await
        .unwrap();
    let blocked = Arc::new(Notify::new());
    let interrupted = BlockingFetch {
        inner: store.clone(),
        calls: Arc::new(AtomicUsize::new(0)),
        blocked: blocked.clone(),
    };
    let manifest_cid = put.manifest_cid.clone();
    let fetch = tokio::spawn(async move { read_tier_blob(&interrupted, &manifest_cid).await });

    blocked.notified().await;
    fetch.abort();
    let killed = fetch.await.unwrap_err();
    assert!(
        killed.is_cancelled(),
        "killed fetch must report cancellation"
    );
    assert!(killed.to_string().contains("cancelled"));
    assert_eq!(
        read_tier_blob(&store, &put.manifest_cid).await.bytes(),
        Some(&bytes)
    );
}

#[tokio::test]
async fn revoked_peer_credential_is_irrelevant_to_self_reach_local_read() {
    let temp = tempfile::tempdir().unwrap();
    let store = DirectoryStorage::create(temp.path().join("objects")).unwrap();
    let credential = temp.path().join("peer.credential");
    std::fs::write(&credential, b"revoked fixture credential").unwrap();
    let bytes = Bytes::from_static(b"self reach needs no credential");
    let put = put_tier_blob(&store, record_for(&bytes), bytes.clone())
        .await
        .unwrap();

    std::fs::remove_file(credential).unwrap();
    assert_eq!(
        read_tier_blob(&store, &put.manifest_cid).await.bytes(),
        Some(&bytes)
    );
}
