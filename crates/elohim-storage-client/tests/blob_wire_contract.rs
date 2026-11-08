//! Contract: what `PUT /blob/{cid}` answers is what the client parses.
//!
//! Storage answers with `elohim_views::infrastructure::PutBlobResponseView`
//! serialised by serde. This test builds that very struct, serialises it with
//! the same serde derive storage uses, and feeds the bytes through the
//! client's parse path. A rename on either side (`blobCid`, `byteLen`,
//! `mimeType`) or a non-byte-CID address fails `cargo test` here instead of
//! at runtime. No hand-written JSON.

use elohim_address::compute_bytes_cid;
use elohim_storage_client::PutBlobReceipt;
use elohim_views::infrastructure::{
    BlobChunkRefView, BlobChunkerView, BlobErasureView, BlobProfileView, PutBlobResponseView,
};

fn storage_answer(bytes: &[u8], mime: &str) -> PutBlobResponseView {
    let cid = compute_bytes_cid(bytes).to_string();
    PutBlobResponseView {
        blob_cid: cid.clone(),
        byte_len: bytes.len() as u64,
        mime_type: mime.to_string(),
        profile: BlobProfileView {
            epoch: 0,
            chunker: BlobChunkerView {
                id: "fastcdc-2020".into(),
                min: 1,
                avg: 2,
                max: 3,
                level: 2,
            },
            erasure: BlobErasureView {
                id: "none".into(),
                data: 0,
                parity: 0,
            },
        },
        chunks: vec![BlobChunkRefView {
            cid,
            len: bytes.len() as u32,
        }],
        stripes: vec![],
        reach: "commons".into(),
        author_id: None,
        created_at: "2026-11-08T00:00:00Z".into(),
        verified_at: None,
    }
}

#[test]
fn client_parses_the_view_storage_serialises() {
    let payload = b"contract payload";
    let wire = serde_json::to_vec(&storage_answer(payload, "text/plain")).unwrap();

    let receipt: PutBlobReceipt = serde_json::from_slice(&wire).unwrap();

    assert_eq!(receipt.blob_cid, compute_bytes_cid(payload));
    assert_eq!(receipt.byte_len, payload.len() as u64);
    assert_eq!(receipt.mime_type, "text/plain");
    assert!(receipt.blob_cid.to_string().starts_with("bafkr4i"));
}

#[test]
fn client_refuses_a_non_byte_cid_address() {
    let mut view = storage_answer(b"x", "text/plain");
    view.blob_cid = "sha256-abc123".into();
    let wire = serde_json::to_vec(&view).unwrap();

    assert!(serde_json::from_slice::<PutBlobReceipt>(&wire).is_err());
}
