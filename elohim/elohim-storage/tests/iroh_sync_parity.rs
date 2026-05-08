//! Phase 5 acceptance — sync ALPN over iroh QUIC.
//!
//! Verifies that [`SyncRequest`] / [`SyncResponse`] round-trip through
//! the iroh transport using the codec helpers, with a stub backend that
//! mirrors the existing libp2p contract. Wire bytes are unchanged from
//! the libp2p side; only the carrier differs.
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use std::sync::Arc;

use anyhow::Result;
use elohim_storage::p2p::sync_protocol::{SyncRequest, SyncResponse};
use elohim_storage::p2p_iroh::{
    parity_harness::TwoNodeFixture, IrohSyncClient, IrohSyncProtocol, SyncBackend, SYNC_ALPN,
};
use tempfile::tempdir;

/// Stub backend used to drive the wire round-trip. Returns a fixed
/// non-trivial response shape so the assertion has something to anchor on.
struct FixedHeadsBackend;

#[async_trait::async_trait]
impl SyncBackend for FixedHeadsBackend {
    async fn handle(&self, req: SyncRequest) -> SyncResponse {
        match req {
            SyncRequest::GetHeads { h_app_id, doc_id } => SyncResponse::Heads {
                h_app_id,
                doc_id,
                heads: vec!["abc123".into(), "def456".into()],
                change_count: 42,
            },
            other => SyncResponse::Error {
                message: format!("test backend got non-GetHeads: {other:?}"),
            },
        }
    }
}

fn sync_protocols() -> Vec<elohim_storage::p2p_iroh::AlpnRegistration> {
    let backend: Arc<dyn SyncBackend> = Arc::new(FixedHeadsBackend);
    let handler = IrohSyncProtocol::new(backend);
    vec![(SYNC_ALPN.to_vec(), Box::new(handler))]
}

#[tokio::test]
async fn sync_request_round_trips_over_iroh_quic() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        TwoNodeFixture::new(provider_dir.path(), fetcher_dir.path(), sync_protocols).await?;

    let req = SyncRequest::GetHeads {
        h_app_id: "lamad".into(),
        doc_id: "doc-phase5".into(),
    };

    let client = IrohSyncClient::new(fixture.fetcher.endpoint());
    let res = client.request(fixture.provider_addr.clone(), &req).await?;

    match res {
        SyncResponse::Heads {
            h_app_id,
            doc_id,
            heads,
            change_count,
        } => {
            assert_eq!(h_app_id, "lamad");
            assert_eq!(doc_id, "doc-phase5");
            assert_eq!(heads, vec!["abc123".to_string(), "def456".to_string()]);
            assert_eq!(change_count, 42);
        }
        other => panic!("unexpected response variant: {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}
