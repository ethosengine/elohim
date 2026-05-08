//! Phase 8 acceptance — view-federation ALPN over iroh QUIC.
//!
//! Verifies [`ViewFederationRequest`] / [`ViewFederationResponse`]
//! round-trip through the iroh transport. 256 KiB cap matches libp2p
//! side. Wire shape unchanged from libp2p; only the carrier differs.

#![cfg(feature = "p2p-iroh")]

use std::sync::Arc;

use anyhow::Result;
use elohim_storage::p2p_iroh::{
    parity_harness::TwoNodeFixture, AlpnRegistration, IrohViewFederationClient,
    IrohViewFederationProtocol, ViewFederationBackend, VIEW_FED_ALPN,
};
use elohim_storage::views::{
    Freshness, FreshnessState, JsonVal, ViewFederationRequest, ViewFederationResponse, ViewKind,
    ViewSlice,
};
use tempfile::tempdir;

struct FixedSliceBackend;

#[async_trait::async_trait]
impl ViewFederationBackend for FixedSliceBackend {
    async fn handle(&self, req: ViewFederationRequest) -> ViewFederationResponse {
        ViewFederationResponse {
            view_kind: req.view_kind.clone(),
            agent_cid: req.agent_cid.clone(),
            request_id: req.request_id.clone(),
            slice: ViewSlice {
                peer_id: "test-peer".into(),
                view_kind: req.view_kind,
                freshness: Freshness {
                    state: FreshnessState::Live,
                    stale_since_ms: None,
                },
                payload: JsonVal(serde_json::json!({"phase": 8})),
                signature: "test-sig".into(),
            },
        }
    }
}

fn view_fed_protocols() -> Vec<AlpnRegistration> {
    let backend: Arc<dyn ViewFederationBackend> = Arc::new(FixedSliceBackend);
    vec![(
        VIEW_FED_ALPN.to_vec(),
        Box::new(IrohViewFederationProtocol::new(backend)),
    )]
}

#[tokio::test]
async fn view_fed_request_round_trips_over_iroh_quic() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        TwoNodeFixture::new(provider_dir.path(), fetcher_dir.path(), view_fed_protocols).await?;

    let req = ViewFederationRequest {
        view_kind: ViewKind::Cluster,
        agent_cid: "agent-phase8".into(),
        request_id: "req-001".into(),
    };

    let client = IrohViewFederationClient::new(fixture.fetcher.endpoint());
    let res = client.request(fixture.provider_addr.clone(), &req).await?;

    assert_eq!(res.view_kind, ViewKind::Cluster);
    assert_eq!(res.agent_cid, "agent-phase8");
    assert_eq!(res.request_id, "req-001");
    assert_eq!(res.slice.peer_id, "test-peer");

    fixture.shutdown().await?;
    Ok(())
}
