//! Phase 7 acceptance — shard ALPN over iroh QUIC.
//!
//! Verifies [`ShardRequest`] / [`ShardResponse`] round-trip through the
//! iroh transport. Wire format (MessagePack) and message schema are
//! reused unchanged from [`crate::p2p::shard_protocol`]; only the
//! transport differs from libp2p.

#![cfg(feature = "p2p-iroh")]

use std::sync::Arc;

use anyhow::Result;
use elohim_storage::p2p::shard_protocol::{ShardRequest, ShardResponse};
use elohim_storage::p2p_iroh::{
    parity_harness::TwoNodeFixture, AlpnRegistration, IrohShardClient, IrohShardProtocol,
    ShardBackend, SHARD_ALPN,
};
use tempfile::tempdir;

struct FixedShardBackend;

#[async_trait::async_trait]
impl ShardBackend for FixedShardBackend {
    async fn handle(&self, req: ShardRequest) -> ShardResponse {
        match req {
            ShardRequest::Get { hash: _ } => {
                ShardResponse::Data(b"shard-bytes-phase7".to_vec())
            }
            ShardRequest::Have { hash: _ } => ShardResponse::Have(true),
            ShardRequest::Push { hash: _, data: _ } => ShardResponse::PushAck,
            ShardRequest::ListContent { .. } => ShardResponse::ContentList {
                items: vec![],
                total: 0,
                has_more: false,
            },
            ShardRequest::GetContent { id: _ } => ShardResponse::ContentNotFound,
        }
    }
}

fn shard_protocols() -> Vec<AlpnRegistration> {
    let backend: Arc<dyn ShardBackend> = Arc::new(FixedShardBackend);
    vec![(
        SHARD_ALPN.to_vec(),
        Box::new(IrohShardProtocol::new(backend)),
    )]
}

#[tokio::test]
async fn shard_get_round_trips_over_iroh_quic() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        TwoNodeFixture::new(provider_dir.path(), fetcher_dir.path(), shard_protocols).await?;

    let req = ShardRequest::Get {
        hash: "sha256-deadbeef".into(),
    };

    let client = IrohShardClient::new(fixture.fetcher.endpoint());
    let res = client.request(fixture.provider_addr.clone(), &req).await?;

    match res {
        ShardResponse::Data(bytes) => {
            assert_eq!(bytes, b"shard-bytes-phase7".to_vec());
        }
        other => panic!("unexpected response: {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn shard_have_round_trips_over_iroh_quic() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        TwoNodeFixture::new(provider_dir.path(), fetcher_dir.path(), shard_protocols).await?;

    let req = ShardRequest::Have {
        hash: "sha256-cafebabe".into(),
    };

    let client = IrohShardClient::new(fixture.fetcher.endpoint());
    let res = client.request(fixture.provider_addr.clone(), &req).await?;

    match res {
        ShardResponse::Have(present) => assert!(present),
        other => panic!("unexpected response: {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}
