use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use elohim_epr::profile::{chunk_bytes, Chunker, Erasure, BYTE_PROFILE_0};
use eprfs_core::{
    AttestationDraft, BlobCid, BlobHandle, BlobPresence, DerivationBasis, DerivationKey,
    DerivationRecord, EprRecord, EprRef, EprfsStorage, FetchPolicy, KeyInput, Witness,
};
use eprfs_storage::{
    put_tier_blob, read_tier_blob, DirectoryStorage, MemoryStorage, TierBlobDraw, TierBlobManifest,
    TierBlobUnavailable, TIER_BLOB_MANIFEST_EPOCH, TIER_BLOB_PROFILE,
};
use serde::Serialize;
use tokio::sync::Notify;

/// The byte profile's largest chunk (`fastcdc-2020` max, epoch 0).
const MAX_CHUNK: usize = 4_194_304;

fn record_for(bytes: &[u8]) -> DerivationRecord {
    record_with_output(BlobCid::compute_bytes(bytes))
}

fn record_with_output(output: BlobCid) -> DerivationRecord {
    DerivationRecord {
        key: DerivationKey {
            method: BlobCid::compute_raw(b"fixture-method"),
            input: KeyInput::Raw(BlobCid::compute_raw(b"fixture-input")),
        },
        output,
        computed_by: "uhCAkFixtureDevice".into(),
        basis: DerivationBasis::Computed,
        deterministic: true,
        witness: Witness::Signature {
            sig: "fixture-signature".into(),
        },
    }
}

/// Zeros never satisfy a `fastcdc-2020` mask, so the chunker cuts at `max`: one full 4 MiB chunk
/// and a 12-byte tail (the chunker vectors' `forced-max` shape).
fn two_chunk_bytes() -> Bytes {
    let mut bytes = vec![0u8; MAX_CHUNK];
    bytes.extend_from_slice(b"second chunk");
    Bytes::from(bytes)
}

#[tokio::test]
async fn local_put_chunks_by_the_byte_profile_and_records_each_cid_in_a_manifest() {
    let temp = tempfile::tempdir().unwrap();
    let store = DirectoryStorage::create(temp.path().join("objects")).unwrap();
    let bytes = two_chunk_bytes();

    let put = put_tier_blob(&store, record_for(&bytes), bytes.clone())
        .await
        .unwrap();

    assert_eq!(put.manifest.record.output, BlobCid::compute_bytes(&bytes));
    assert_eq!(put.manifest.epoch, TIER_BLOB_MANIFEST_EPOCH);
    assert_eq!(put.manifest.epoch, 2);
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
    assert_eq!(put.manifest.chunks[0].byte_len, MAX_CHUNK as u32);
    assert_eq!(put.manifest.chunks[1].byte_len, 12);
    // The layout is the profile's chunker, and every chunk is a byte-profile address.
    let layout: Vec<u32> = chunk_bytes(&BYTE_PROFILE_0, &bytes)
        .into_iter()
        .map(|(_, len)| len as u32)
        .collect();
    assert_eq!(
        put.manifest
            .chunks
            .iter()
            .map(|c| c.byte_len)
            .collect::<Vec<_>>(),
        layout
    );
    for chunk in &put.manifest.chunks {
        assert!(chunk.cid.byte_cid().is_ok(), "{} is a byte CID", chunk.cid);
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
            manifest: Box::new(put.manifest),
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

/// The epoch-1 schema, as a pre-crossing tier store holds it: fixed 1 MiB chunks under raw sha2,
/// and no recorded profile. Written here by hand so the test never runs through the epoch-2 type.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EpochOneManifest {
    epoch: u32,
    record: DerivationRecord,
    byte_len: u64,
    chunks: Vec<EpochOneChunk>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EpochOneChunk {
    cid: BlobCid,
    byte_len: u32,
}

#[tokio::test]
async fn epoch_1_manifest_is_held_not_read_and_not_a_parse_error() {
    let store = MemoryStorage::default();
    let bytes = Bytes::from_static(b"an artifact the old tier store chunked");
    let chunk = BlobCid::compute_raw(&bytes);
    store
        .put_blob_verified(&chunk, bytes.clone())
        .await
        .unwrap();
    let old = EpochOneManifest {
        epoch: 1,
        record: record_with_output(BlobCid::compute_raw(&bytes)),
        byte_len: bytes.len() as u64,
        chunks: vec![EpochOneChunk {
            cid: chunk,
            byte_len: bytes.len() as u32,
        }],
    };
    let (manifest_cid, manifest_bytes) = BlobCid::encode_manifest(&old).unwrap();
    // The epoch-2 schema alone would refuse this object for its missing `profile` field.
    assert!(serde_ipld_dagcbor::from_slice::<TierBlobManifest>(&manifest_bytes).is_err());
    store
        .put_blob_verified(&manifest_cid, Bytes::from(manifest_bytes))
        .await
        .unwrap();

    let draw = read_tier_blob(&store, &manifest_cid).await;
    assert_eq!(draw.bytes(), None);
    assert_eq!(
        draw,
        TierBlobDraw::Unavailable {
            manifest_cid: manifest_cid.clone(),
            why: TierBlobUnavailable::UnsupportedEpoch { epoch: 1 },
        }
    );
    assert_eq!(
        draw.reason().as_deref(),
        Some("manifest epoch 1 is unsupported")
    );
    // Held, not discarded: the object is still in custody.
    assert!(store
        .fetch_blob(&manifest_cid, FetchPolicy::LocalOnly)
        .await
        .is_ok());
}

#[tokio::test]
async fn profile_is_persisted_with_every_layout() {
    assert_eq!(TIER_BLOB_PROFILE.chunker, BYTE_PROFILE_0.chunker);
    assert_eq!(
        (TIER_BLOB_PROFILE.codec, TIER_BLOB_PROFILE.multihash),
        (0x55, 0x1e)
    );
    assert_eq!(TIER_BLOB_PROFILE.erasure, Erasure::None);
    let Chunker::Fastcdc2020 { min, .. } = TIER_BLOB_PROFILE.chunker;
    let store = MemoryStorage::default();
    // Zero chunks, one sub-minimum chunk, and a multi-chunk layout.
    for bytes in [
        Bytes::new(),
        Bytes::from(vec![7u8; min as usize - 1]),
        two_chunk_bytes(),
    ] {
        let put = put_tier_blob(&store, record_for(&bytes), bytes.clone())
            .await
            .unwrap();
        let stored = store
            .fetch_blob(&put.manifest_cid, FetchPolicy::LocalOnly)
            .await
            .unwrap();
        let decoded: TierBlobManifest = serde_ipld_dagcbor::from_slice(&stored.bytes).unwrap();
        assert_eq!(decoded.profile, TIER_BLOB_PROFILE, "{} bytes", bytes.len());
        assert_eq!(decoded, put.manifest);
        assert_eq!(
            read_tier_blob(&store, &put.manifest_cid).await.bytes(),
            Some(&bytes)
        );
    }
}

#[tokio::test]
async fn a_sha2_output_still_verifies_through_byte_profile_chunks() {
    // Record and citation outputs stay sha2: the whole-artifact digest follows the output's own
    // declared profile while the chunks stay under the byte profile.
    let store = MemoryStorage::default();
    let bytes = two_chunk_bytes();
    for output in [BlobCid::compute_raw(&bytes), BlobCid::compute(&bytes)] {
        let put = put_tier_blob(&store, record_with_output(output), bytes.clone())
            .await
            .unwrap();
        assert_eq!(
            read_tier_blob(&store, &put.manifest_cid).await.bytes(),
            Some(&bytes)
        );
    }
}

#[tokio::test]
async fn reordered_or_substituted_chunks_never_return_bytes() {
    let store = MemoryStorage::default();
    let bytes = two_chunk_bytes();
    let put = put_tier_blob(&store, record_for(&bytes), bytes.clone())
        .await
        .unwrap();

    let mut reordered = put.manifest.clone();
    reordered.chunks.reverse();
    let substitute = Bytes::from(vec![1u8; MAX_CHUNK]);
    let mut substituted = put.manifest.clone();
    substituted.chunks[0].cid = BlobCid::compute_bytes(&substitute);
    store
        .put_blob_verified(&substituted.chunks[0].cid, substitute)
        .await
        .unwrap();
    for forged in [reordered, substituted] {
        let (cid, manifest_bytes) = BlobCid::encode_manifest(&forged).unwrap();
        store
            .put_blob_verified(&cid, Bytes::from(manifest_bytes))
            .await
            .unwrap();
        let draw = read_tier_blob(&store, &cid).await;
        assert_eq!(draw.bytes(), None);
        assert!(matches!(
            draw,
            TierBlobDraw::Unavailable {
                why: TierBlobUnavailable::BlobCorrupt { .. },
                ..
            }
        ));
    }
}

#[tokio::test]
async fn an_unsupported_output_profile_is_unavailable_never_a_hit() {
    let store = MemoryStorage::default();
    let bytes = Bytes::from_static(b"payload");
    let put = put_tier_blob(&store, record_for(&bytes), bytes.clone())
        .await
        .unwrap();
    let mut manifest = put.manifest.clone();
    // A dag-pb (0x70) output CID: a valid multicodec no profile admits.
    let dag_pb =
        BlobCid::parse("bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi").unwrap();
    manifest.record.output = dag_pb.clone();
    let (cid, manifest_bytes) = BlobCid::encode_manifest(&manifest).unwrap();
    store
        .put_blob_verified(&cid, Bytes::from(manifest_bytes))
        .await
        .unwrap();
    assert_eq!(
        read_tier_blob(&store, &cid).await,
        TierBlobDraw::Unavailable {
            manifest_cid: cid,
            why: TierBlobUnavailable::UnsupportedProfile { cid: dag_pb },
        }
    );
}
