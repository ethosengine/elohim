use bytes::Bytes;
use elohim_epr_rea::{
    atom_cid, AgentRef, FlowRecord, FlowStore, Magnitude, MemoryFlowStore, ReaVerb,
};
use eprfs_core::{BlobCid, DerivationBasis, DerivationKey, DerivationRecord, KeyInput, Witness};
use eprfs_peer::{put_local_blob, read_local_blob, stream_local_blob, LocalBlobContext};
use eprfs_storage::{
    DirectoryStorage, TierBlobChunk, TierBlobDraw, TierBlobSink, TierBlobStreamDraw,
    TierBlobUnavailable, TIER_BLOB_CHUNK_BYTES,
};

fn record_for(bytes: &[u8]) -> DerivationRecord {
    DerivationRecord {
        key: DerivationKey {
            method: BlobCid::compute_raw(b"local-build-method"),
            input: KeyInput::Raw(BlobCid::compute_raw(b"local-build-input")),
        },
        output: BlobCid::compute_raw(bytes),
        computed_by: "uhCAkLocalBuilder".into(),
        basis: DerivationBasis::Computed,
        deterministic: true,
        witness: Witness::Signature {
            sig: "fixture-signature".into(),
        },
    }
}

fn context() -> LocalBlobContext {
    LocalBlobContext {
        custodian: AgentRef("uhCAkLocalDevice".into()),
        in_scope_of: atom_cid(&"local-tier-scope").unwrap(),
        occurred_at: "2026-11-04T12:00:00Z".into(),
    }
}

fn five_chunk_bytes() -> Bytes {
    let mut bytes = Vec::with_capacity(4 * TIER_BLOB_CHUNK_BYTES + 19);
    for marker in 1..=4 {
        bytes.extend(std::iter::repeat_n(marker, TIER_BLOB_CHUNK_BYTES));
    }
    bytes.extend(std::iter::repeat_n(5, 19));
    Bytes::from(bytes)
}

#[derive(Default)]
struct RecordingSink {
    max_bytes: usize,
    order: Vec<u8>,
}

impl TierBlobSink for RecordingSink {
    fn chunk(&mut self, _chunk: &TierBlobChunk, bytes: Bytes) -> std::io::Result<()> {
        self.max_bytes = self.max_bytes.max(bytes.len());
        self.order.push(bytes[0]);
        Ok(())
    }
}

#[tokio::test]
async fn five_chunk_stream_is_bounded_ordered_and_corruption_stops_with_one_unavailable_use() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("objects");
    let store = DirectoryStorage::create(&root).unwrap();
    let mut flow = MemoryFlowStore::new();
    let bytes = five_chunk_bytes();
    let put = put_local_blob(&store, &mut flow, record_for(&bytes), bytes, &context())
        .await
        .unwrap();
    assert_eq!(put.manifest.chunks.len(), 5);
    assert_eq!(put.manifest.chunks[4].byte_len, 19);

    let mut completed_sink = RecordingSink::default();
    let completed = stream_local_blob(
        &store,
        &mut flow,
        &put.manifest_cid,
        &context(),
        &mut completed_sink,
    )
    .await
    .unwrap();
    assert!(matches!(completed, TierBlobStreamDraw::Completed { .. }));
    assert_eq!(completed_sink.max_bytes, TIER_BLOB_CHUNK_BYTES);
    assert_eq!(completed_sink.order, vec![1, 2, 3, 4, 5]);
    let events = flow.events().unwrap();
    assert_eq!(events.len(), 2, "put and completed draw each record once");
    assert!(events[1].1.classified_as.contains(&"draw:hit".into()));

    let corrupt = put.manifest.chunks[2].cid.clone();
    std::fs::write(root.join(corrupt.to_string()), b"corrupt third chunk").unwrap();
    let mut stopped_sink = RecordingSink::default();
    let stopped = stream_local_blob(
        &store,
        &mut flow,
        &put.manifest_cid,
        &context(),
        &mut stopped_sink,
    )
    .await
    .unwrap();
    assert!(matches!(
        stopped,
        TierBlobStreamDraw::Unavailable {
            why: TierBlobUnavailable::ChunkCorrupt { ref cid },
            ..
        } if cid == &corrupt
    ));
    assert_eq!(stopped_sink.max_bytes, TIER_BLOB_CHUNK_BYTES);
    assert_eq!(stopped_sink.order, vec![1, 2]);
    let events = flow.events().unwrap();
    assert_eq!(events.len(), 3, "stopped draw records one additional Use");
    assert!(events[2]
        .1
        .classified_as
        .contains(&"draw:unavailable".into()));
    assert!(!events[2].1.classified_as.contains(&"draw:hit".into()));
}

#[tokio::test]
async fn daemon_down_put_and_read_record_produce_and_use_without_a_peer() {
    let temp = tempfile::tempdir().unwrap();
    let store = DirectoryStorage::create(temp.path().join("objects")).unwrap();
    let mut flow = MemoryFlowStore::new();
    let bytes = Bytes::from_static(b"daemon-independent local artifact");

    let put = put_local_blob(
        &store,
        &mut flow,
        record_for(&bytes),
        bytes.clone(),
        &context(),
    )
    .await
    .unwrap();
    let draw = read_local_blob(&store, &mut flow, &put.manifest_cid, &context())
        .await
        .unwrap();

    assert_eq!(draw.bytes(), Some(&bytes));
    let events = flow.events().unwrap();
    assert_eq!(events.len(), 2);
    let produced = &events[0].1;
    assert_eq!(produced.action, ReaVerb::Produce);
    assert_eq!(produced.provider.0, "uhCAkLocalBuilder");
    assert_eq!(produced.receiver.0, "uhCAkLocalDevice");
    assert_eq!(
        produced.resource,
        put.manifest.record.output.as_cid().clone()
    );
    assert_eq!(
        produced.quantity,
        Magnitude::Count {
            value: bytes.len() as f64,
            unit: "bytes".into(),
        }
    );
    assert!(produced.classified_as.contains(&"custody-blob".into()));
    assert!(produced
        .classified_as
        .contains(&format!("manifest:{}", put.manifest_cid)));

    let used = &events[1].1;
    assert_eq!(used.action, ReaVerb::Use);
    assert!(used.classified_as.contains(&"draw:hit".into()));
    assert!(used.classified_as.contains(&"copy:self".into()));
}

#[tokio::test]
async fn miss_is_a_recorded_use_with_a_typed_printable_reason() {
    let temp = tempfile::tempdir().unwrap();
    let store = DirectoryStorage::create(temp.path().join("objects")).unwrap();
    let mut flow = MemoryFlowStore::new();
    let absent = BlobCid::compute(b"absent manifest");

    let draw = read_local_blob(&store, &mut flow, &absent, &context())
        .await
        .unwrap();

    assert!(matches!(draw, TierBlobDraw::Miss { .. }));
    assert_eq!(draw.reason().as_deref(), Some("manifest is absent"));
    let events = flow.events().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].1.action, ReaVerb::Use);
    assert!(events[0].1.classified_as.contains(&"draw:miss".into()));
    assert!(events[0]
        .1
        .classified_as
        .contains(&"reason:manifest is absent".into()));
}

#[tokio::test]
async fn unavailable_corrupt_draw_records_use_and_withholds_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("objects");
    let store = DirectoryStorage::create(&root).unwrap();
    let mut flow = MemoryFlowStore::new();
    let bytes = Bytes::from_static(b"corruption must not become a cache hit");
    let put = put_local_blob(&store, &mut flow, record_for(&bytes), bytes, &context())
        .await
        .unwrap();
    std::fs::write(
        root.join(put.manifest.chunks[0].cid.to_string()),
        b"corrupt",
    )
    .unwrap();

    let draw = read_local_blob(&store, &mut flow, &put.manifest_cid, &context())
        .await
        .unwrap();

    assert_eq!(draw.bytes(), None);
    assert!(draw.reason().unwrap().contains("chunk is corrupt"));
    let events = flow.events().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].1.action, ReaVerb::Use);
    assert!(events[1]
        .1
        .classified_as
        .contains(&"draw:unavailable".into()));
    assert!(events[1]
        .1
        .classified_as
        .iter()
        .any(|slot| slot.starts_with("reason:chunk is corrupt")));
}

#[derive(Default)]
struct RefusingFlowStore;

impl FlowStore for RefusingFlowStore {
    fn append(&mut self, _record: FlowRecord) -> Result<cid::Cid, elohim_epr_rea::FabricError> {
        Err(elohim_epr_rea::FabricError::Io(std::io::Error::other(
            "injected flow refusal",
        )))
    }

    fn records(&self) -> Result<Vec<(cid::Cid, FlowRecord)>, elohim_epr_rea::FabricError> {
        Ok(Vec::new())
    }
}

#[tokio::test]
async fn failed_flow_record_is_explicitly_unrecorded_never_a_silent_put() {
    let temp = tempfile::tempdir().unwrap();
    let store = DirectoryStorage::create(temp.path().join("objects")).unwrap();
    let bytes = Bytes::from_static(b"persisted but explicitly unrecorded");
    let mut flow = RefusingFlowStore;

    let error = put_local_blob(&store, &mut flow, record_for(&bytes), bytes, &context())
        .await
        .unwrap_err();

    assert!(error.to_string().contains("persisted, unrecorded"));
    assert!(error.to_string().contains("injected flow refusal"));
}
