//! Phase 11 acceptance — EPR ALPN over iroh QUIC dispatched into a real
//! [`EprService`] backend (not the stub used by `iroh_epr_parity`).
//!
//! This proves the wire → backend → service chain end-to-end on iroh:
//! provider mounts [`EprServiceBackend`] over a real [`EprService`]; an
//! EPR request is sent via the iroh client; the response reflects the
//! service's actual logic (Authorization gates, DB lookups, cache-tier
//! reporting). If the chain breaks anywhere — codec, ALPN dispatch,
//! adapter mapping, EprService wiring — the assertion blows up.
//!
//! Companion to `iroh_epr_parity` (which uses a fixed stub backend to
//! pin wire-byte parity). Together they cover wire framing AND production
//! dispatch.
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
//! the EPR plane is dual-stack permanent. The Announce variant is the
//! asymmetric outlier (libp2p put_record vs iroh pkarr-not-yet-wired);
//! one of the tests pins the iroh-side "not yet wired" reason so a
//! premature wire-up doesn't ship without spec amendment.
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use std::sync::Arc;

use anyhow::Result;
use elohim_storage::epr_service::EprService;
use elohim_storage::p2p::epr_protocol::{EprRequest, EprResponse};
use elohim_storage::p2p::trust_cache::PeerTrustCache;
use elohim_storage::p2p_iroh::{
    parity_harness::TwoNodeFixture, AlpnRegistration, EprBackend, EprServiceBackend, IrohEprClient,
    IrohEprProtocol, EPR_ALPN,
};
use tempfile::tempdir;

fn build_real_backend() -> Arc<dyn EprBackend> {
    // No DB pool, no policy, no cache, empty trust cache. This is the
    // "Stage-4-lightweight" iroh-mode posture: no libp2p-side trust
    // handshake has populated the cache, so reach-auth falls through to
    // slow-path DB lookups (which all return None / NotFound here).
    let service = Arc::new(EprService::new(None, None, None, PeerTrustCache::new()));
    Arc::new(EprServiceBackend::new(service))
}

async fn fixture_with_real_provider_backend(
    provider_dir: &std::path::Path,
    fetcher_dir: &std::path::Path,
) -> Result<TwoNodeFixture> {
    let backend = build_real_backend();
    let handler = IrohEprProtocol::new(backend);
    let provider_extras: Vec<AlpnRegistration> = vec![(EPR_ALPN.to_vec(), Box::new(handler))];
    TwoNodeFixture::new_asymmetric(provider_dir, provider_extras, fetcher_dir, Vec::new()).await
}

/// Resolve against a no-DB EprService — the provider has no content and
/// no DB, so the wire chain must surface `NotFound` (not a stub success
/// or a panic). Catches "adapter dispatched but didn't read service."
#[tokio::test]
async fn resolve_via_iroh_against_real_backend_returns_not_found() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohEprClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &EprRequest::Resolve {
                id: "missing-content".into(),
                agent_pubkey: None,
            },
        )
        .await?;

    match res {
        EprResponse::NotFound => {}
        other => panic!("expected NotFound from real backend with no DB, got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

/// QueryDelivery against a no-cache EprService must report `cache_tier =
/// "blob-only"`, `serves_extracted = false`, `warm = false`, and
/// `serves_compressed = true` — these are the values an iroh peer with
/// no extraction cache MUST advertise so federation peers don't try to
/// dispatch extracted-format reads at it.
#[tokio::test]
async fn query_delivery_via_iroh_reports_blob_only_when_no_cache() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohEprClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &EprRequest::QueryDelivery {
                blob_hash: "sha256-abc".into(),
            },
        )
        .await?;

    match res {
        EprResponse::DeliveryInfo {
            serves_extracted,
            serves_compressed,
            cache_tier,
            warm,
        } => {
            assert!(!serves_extracted, "no cache → no extracted serving");
            assert!(serves_compressed, "raw blobs always serveable");
            assert_eq!(cache_tier, "blob-only");
            assert!(!warm, "no cache → not warm");
        }
        other => panic!("expected DeliveryInfo, got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

/// ResolveBatch against a no-DB EprService — every requested id is
/// missing, so the response must be a `HeadBatch` with one empty `Vec`
/// per requested id (preserving order + count).
#[tokio::test]
async fn resolve_batch_via_iroh_emits_per_id_empty_vecs() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohEprClient::new(fixture.fetcher.endpoint());
    let ids = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &EprRequest::ResolveBatch { ids: ids.clone() },
        )
        .await?;

    match res {
        EprResponse::HeadBatch(results) => {
            assert_eq!(results.len(), ids.len(), "one result per requested id");
            for (idx, bytes) in results.iter().enumerate() {
                assert!(
                    bytes.is_empty(),
                    "result {idx} should be empty for missing id"
                );
            }
        }
        other => panic!("expected HeadBatch, got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

/// GetDocument is shape-correct but unimplemented on both transports;
/// pinning this here lets us notice when the libp2p side ships full
/// document fetch and we forget to mirror it on the iroh side.
#[tokio::test]
async fn get_document_via_iroh_reports_not_yet_implemented() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohEprClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &EprRequest::GetDocument { id: "doc-9".into() },
        )
        .await?;

    match res {
        EprResponse::Error(msg) => assert!(
            msg.contains("not yet implemented"),
            "GetDocument error should keep its parity message; got: {msg}"
        ),
        other => panic!("expected Error, got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

/// Announce in iroh mode is intentionally unwired pending the n0
/// mitigation roadmap (pkarr / iroh-gossip identity-binding). Pin the
/// "not yet wired" response shape so a premature wire-up trips this
/// test instead of silently shipping. When the n0 mitigation lands,
/// this test gets rewritten to assert the new positive shape.
#[tokio::test]
async fn announce_via_iroh_returns_not_yet_wired_per_spec() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohEprClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &EprRequest::Announce {
                head: b"would-decode-to-an-EprHead".to_vec(),
            },
        )
        .await?;

    match res {
        EprResponse::Announced { accepted, reason } => {
            assert!(!accepted, "iroh-mode Announce must report not-accepted");
            let reason = reason.unwrap_or_default();
            assert!(
                reason.contains("not yet wired") || reason.contains("n0 mitigation"),
                "reason should reference the spec's n0 mitigation roadmap; got: {reason}"
            );
        }
        other => panic!("expected Announced, got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}
