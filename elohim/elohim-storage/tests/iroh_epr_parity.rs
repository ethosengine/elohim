//! Phase 6 acceptance — EPR + EPR-atom ALPNs over iroh QUIC.
//!
//! Two parity tests:
//! - `epr_request_round_trips_over_iroh_quic` — MessagePack-encoded
//!   `EprRequest` / `EprResponse` round-trip via [`IrohEprClient`].
//! - `epr_atom_request_round_trips_over_iroh_quic` — CBOR-encoded
//!   `EprAtomRequest` / `EprAtomResponse` round-trip via
//!   [`IrohEprAtomClient`].
//!
//! Wire shapes are reused unchanged from
//! [`crate::p2p::epr_protocol`] / [`crate::p2p::epr_atom_protocol`].

#![cfg(feature = "p2p-iroh")]

use std::sync::Arc;

use anyhow::Result;
use elohim_storage::p2p::epr_atom_protocol::{EprAtomRequest, EprAtomResponse};
use elohim_storage::p2p::epr_protocol::{EprRequest, EprResponse};
use elohim_storage::p2p_iroh::{
    parity_harness::TwoNodeFixture, AlpnRegistration, EprAtomBackend, EprBackend,
    IrohEprAtomClient, IrohEprAtomProtocol, IrohEprClient, IrohEprProtocol, EPR_ALPN,
    EPR_ATOM_ALPN,
};
use tempfile::tempdir;

// ============================================================================
// EPR resolution (MessagePack)
// ============================================================================

struct FixedEprBackend;

#[async_trait::async_trait]
impl EprBackend for FixedEprBackend {
    async fn handle(&self, req: EprRequest) -> EprResponse {
        match req {
            EprRequest::Resolve { id, .. } => {
                EprResponse::Head(format!("head-bytes-for-{id}").into_bytes())
            }
            EprRequest::QueryDelivery { .. } => EprResponse::DeliveryInfo {
                serves_extracted: true,
                serves_compressed: false,
                cache_tier: "extraction".into(),
                warm: true,
            },
            _ => EprResponse::NotFound,
        }
    }
}

fn epr_protocols() -> Vec<AlpnRegistration> {
    let backend: Arc<dyn EprBackend> = Arc::new(FixedEprBackend);
    vec![(EPR_ALPN.to_vec(), Box::new(IrohEprProtocol::new(backend)))]
}

#[tokio::test]
async fn epr_request_round_trips_over_iroh_quic() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        TwoNodeFixture::new(provider_dir.path(), fetcher_dir.path(), epr_protocols).await?;

    let req = EprRequest::Resolve {
        id: "epr-phase6".into(),
        agent_pubkey: None,
    };

    let client = IrohEprClient::new(fixture.fetcher.endpoint());
    let res = client.request(fixture.provider_addr.clone(), &req).await?;

    match res {
        EprResponse::Head(bytes) => {
            assert_eq!(bytes, b"head-bytes-for-epr-phase6".to_vec());
        }
        other => panic!("unexpected response: {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

// ============================================================================
// EPR-atom transfer (CBOR)
// ============================================================================

struct FixedAtomBackend;

#[async_trait::async_trait]
impl EprAtomBackend for FixedAtomBackend {
    async fn handle(&self, req: EprAtomRequest) -> EprAtomResponse {
        match req {
            EprAtomRequest::Fetch { cid: _ } => EprAtomResponse::Atom {
                envelope_bytes: b"cbor-atom-envelope".to_vec(),
            },
            EprAtomRequest::FetchBatch { cids } => EprAtomResponse::AtomBatch {
                atoms: cids
                    .into_iter()
                    .map(|c| Some(format!("env-{c}").into_bytes()))
                    .collect(),
            },
            _ => EprAtomResponse::NotFound,
        }
    }
}

fn epr_atom_protocols() -> Vec<AlpnRegistration> {
    let backend: Arc<dyn EprAtomBackend> = Arc::new(FixedAtomBackend);
    vec![(
        EPR_ATOM_ALPN.to_vec(),
        Box::new(IrohEprAtomProtocol::new(backend)),
    )]
}

#[tokio::test]
async fn epr_atom_request_round_trips_over_iroh_quic() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        TwoNodeFixture::new(provider_dir.path(), fetcher_dir.path(), epr_atom_protocols).await?;

    let req = EprAtomRequest::Fetch {
        cid: "bafyreigh3...example".into(),
    };

    let client = IrohEprAtomClient::new(fixture.fetcher.endpoint());
    let res = client.request(fixture.provider_addr.clone(), &req).await?;

    match res {
        EprAtomResponse::Atom { envelope_bytes } => {
            assert_eq!(envelope_bytes, b"cbor-atom-envelope".to_vec());
        }
        other => panic!("unexpected response: {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}
