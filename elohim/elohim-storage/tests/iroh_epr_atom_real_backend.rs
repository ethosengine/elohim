//! Phase 11 acceptance — EPR-atom ALPN over iroh QUIC dispatched into
//! a real [`EprAtomService`] backend (not the stub used by
//! `iroh_epr_parity`).
//!
//! Proves the wire → backend → service chain end-to-end on iroh:
//! provider mounts [`EprAtomServiceBackend`] over a real
//! [`EprAtomService`]; an EPR-atom request is sent via the iroh
//! client; the response reflects the service's actual logic (CBOR
//! decode, dedup, reach gate). If the chain breaks anywhere — codec,
//! ALPN dispatch, adapter mapping, EprAtomService wiring — the
//! assertion blows up.
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
//! the EPR-atom plane is dual-stack permanent. Caller identity defaults
//! to Anonymous in iroh mode pending the cross-stack peer-map
//! graduation, so these tests exercise the unauthenticated paths.
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use std::sync::Arc;

use anyhow::Result;
use elohim_storage::epr_atom_service::EprAtomService;
use elohim_storage::p2p::dedup::DedupLru;
use elohim_storage::p2p::epr_atom_protocol::{EprAtomRequest, EprAtomResponse, MAX_BATCH_CIDS};
use elohim_storage::p2p_iroh::{
    parity_harness::TwoNodeFixture, AlpnRegistration, EprAtomBackend, EprAtomServiceBackend,
    IrohEprAtomClient, IrohEprAtomProtocol, EPR_ATOM_ALPN,
};
use tempfile::tempdir;

fn build_real_backend() -> Arc<dyn EprAtomBackend> {
    // No DB pool, fresh dedup. This is the iroh-mode "no-content,
    // anonymous-caller" posture — anyone fetching here gets either
    // Error(storage unavailable) for Fetch or NotFound for missing
    // CIDs in batches.
    let service = Arc::new(EprAtomService::new(None, Arc::new(DedupLru::new())));
    Arc::new(EprAtomServiceBackend::new(service))
}

async fn fixture_with_real_provider_backend(
    provider_dir: &std::path::Path,
    fetcher_dir: &std::path::Path,
) -> Result<TwoNodeFixture> {
    let backend = build_real_backend();
    let handler = IrohEprAtomProtocol::new(backend);
    let provider_extras: Vec<AlpnRegistration> = vec![(EPR_ATOM_ALPN.to_vec(), Box::new(handler))];
    TwoNodeFixture::new_asymmetric(provider_dir, provider_extras, fetcher_dir, Vec::new()).await
}

/// Fetch against a no-DB-pool service must surface
/// `Error { message: "storage unavailable" }`. Catches "adapter
/// dispatched but didn't read the service's pool option."
#[tokio::test]
async fn fetch_via_iroh_against_real_backend_surfaces_storage_unavailable() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohEprAtomClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &EprAtomRequest::Fetch {
                cid: "bafyreigh-real-backend".into(),
            },
        )
        .await?;

    match res {
        EprAtomResponse::Error { message } => {
            assert!(
                message.contains("storage unavailable"),
                "expected 'storage unavailable' error; got: {message}"
            );
        }
        other => panic!("expected Error(storage unavailable), got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

/// Announce with garbage CBOR must surface a precise, leak-free error
/// reason — proving the service decoded and the adapter passed the
/// reason through the wire intact.
#[tokio::test]
async fn announce_via_iroh_with_garbage_cbor_reports_decode_failure() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohEprAtomClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &EprAtomRequest::Announce {
                envelope_bytes: b"this-is-not-cbor".to_vec(),
            },
        )
        .await?;

    match res {
        EprAtomResponse::Announced { accepted, reason } => {
            assert!(!accepted, "garbage CBOR must not be accepted");
            let reason = reason.unwrap_or_default();
            assert!(
                reason.contains("cbor decode"),
                "expected 'cbor decode' reason; got: {reason}"
            );
        }
        other => panic!("expected Announced(false, cbor decode...), got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

/// FetchBatch oversized must surface `Error { batch too large }` — the
/// MAX_BATCH_CIDS guard runs in the service before any DB I/O, so the
/// chain proves itself even with no DB pool.
#[tokio::test]
async fn fetch_batch_oversized_via_iroh_is_rejected() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohEprAtomClient::new(fixture.fetcher.endpoint());
    let cids: Vec<String> = (0..MAX_BATCH_CIDS + 1)
        .map(|i| format!("bafy-{i}"))
        .collect();
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &EprAtomRequest::FetchBatch { cids },
        )
        .await?;

    match res {
        EprAtomResponse::Error { message } => {
            assert!(
                message.contains("batch too large"),
                "expected 'batch too large' error; got: {message}"
            );
        }
        other => panic!("expected Error(batch too large), got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

/// IntegrityNotify with an unhandled kind must surface
/// `IntegrityAck { received: false, reason: contains kind }` —
/// proving the service routed by kind and the iroh side preserved
/// the wire-byte response identical to libp2p.
#[tokio::test]
async fn integrity_notify_unhandled_kind_via_iroh_acks_received_false() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohEprAtomClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &EprAtomRequest::IntegrityNotify {
                kind: "UnhandledForNow".into(),
                payload_bytes: b"x".to_vec(),
            },
        )
        .await?;

    match res {
        EprAtomResponse::IntegrityAck { received, reason } => {
            assert!(!received);
            let reason = reason.unwrap_or_default();
            assert!(
                reason.contains("UnhandledForNow"),
                "expected reason to mention the kind; got: {reason}"
            );
        }
        other => panic!("expected IntegrityAck(false, ...), got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}
