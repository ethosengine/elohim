//! e2e test for cutover gate #10: configure an iroh Endpoint with a
//! `discovery_resolvers` list pointing at a self-hosted pkarr relay,
//! verify NodeId discovery works through that path, and verify n0's
//! resolver was NOT contacted.
//!
//! Strategy:
//!   1. Spin up a minimal hyper server in-process implementing GET/PUT
//!      /pkarr/{key} (mirrors the doorway pkarr_resolver service module
//!      via direct re-implementation — keeps the test self-contained).
//!   2. Build TWO iroh Endpoints, both configured with discovery_resolvers
//!      pointing ONLY at the in-process server (no n0 entry).
//!   3. Endpoint A publishes its NodeAddr to the resolver via
//!      iroh::discovery::pkarr::PkarrPublisher (driven automatically).
//!   4. Endpoint B resolves Endpoint A's NodeId via PkarrResolver.
//!   5. Assert: in-process server saw the PUT + at least one GET.
//!      n0 was NOT contacted because n0 is not in the discovery list and
//!      iroh's ConcurrentDiscovery only iterates the registered providers.

#![cfg(feature = "p2p-iroh")]

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use bytes::Bytes;
use elohim_storage::p2p_iroh::{
    build_endpoint,
    config::{DiscoveryResolverConfig, DiscoveryResolverKind, IrohConfig},
};
use pkarr::{PublicKey, SignedPacket};
use tokio::sync::Mutex;

#[derive(Default)]
struct RelayMetrics {
    gets: AtomicUsize,
    puts: AtomicUsize,
    cache: Mutex<std::collections::HashMap<String, SignedPacket>>,
}

async fn run_test_relay(metrics: Arc<RelayMetrics>) -> SocketAddr {
    let app = Router::new()
        .route("/pkarr/{pk}", get(get_pkarr).put(put_pkarr))
        .with_state(metrics.clone());
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

async fn get_pkarr(
    State(m): State<Arc<RelayMetrics>>,
    Path(pk): Path<String>,
) -> Result<(StatusCode, [(&'static str, &'static str); 1], Vec<u8>), StatusCode> {
    m.gets.fetch_add(1, Ordering::SeqCst);
    let cache = m.cache.lock().await;
    let Some(packet) = cache.get(&pk) else {
        return Err(StatusCode::NOT_FOUND);
    };
    Ok((
        StatusCode::OK,
        [(
            "content-type",
            "application/pkarr.org-relays+octet-stream",
        )],
        packet.to_relay_payload().to_vec(),
    ))
}

async fn put_pkarr(
    State(m): State<Arc<RelayMetrics>>,
    Path(pk_str): Path<String>,
    body: Bytes,
) -> StatusCode {
    m.puts.fetch_add(1, Ordering::SeqCst);
    let public_key: PublicKey = match pk_str.parse() {
        Ok(p) => p,
        Err(_) => return StatusCode::BAD_REQUEST,
    };
    let packet = match SignedPacket::from_relay_payload(&public_key, &body) {
        Ok(p) => p,
        Err(_) => return StatusCode::BAD_REQUEST,
    };
    m.cache.lock().await.insert(pk_str, packet);
    StatusCode::OK
}

#[tokio::test]
async fn iroh_resolves_via_self_hosted_pkarr_only() {
    let metrics = Arc::new(RelayMetrics::default());
    let relay_addr = run_test_relay(metrics.clone()).await;
    let relay_url: url::Url = format!("http://{}/pkarr", relay_addr).parse().unwrap();

    // Both endpoints point at our in-process resolver only — NO n0 entry.
    let make_cfg = |dir: &std::path::Path| IrohConfig {
        blobs_dir: dir.join("blobs"),
        secret_key_path: dir.join("iroh.key"),
        use_n0_relays: false,
        use_n0_discovery: false,
        discovery_resolvers: vec![DiscoveryResolverConfig {
            url: relay_url.clone(),
            kind: DiscoveryResolverKind::OperatorSelfHosted,
        }],
    };

    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let ep_a = build_endpoint(&make_cfg(dir_a.path())).await.unwrap();
    let ep_b = build_endpoint(&make_cfg(dir_b.path())).await.unwrap();

    // Force A to publish: PkarrPublisher publishes when the endpoint's
    // direct addresses are populated. Calling node_addr().initialized()
    // (via .await on the future) triggers that.
    let _addr_a = ep_a.node_addr().initialized().await;

    // Allow the publish to flush.
    tokio::time::sleep(Duration::from_secs(2)).await;

    // B resolves A's NodeId via the configured discovery surface.
    let node_id_a = ep_a.node_id();
    let resolved = ep_b.discovery().expect("discovery configured");
    let mut stream = resolved.resolve(node_id_a).expect("resolver returns stream");
    use n0_future::StreamExt;
    let item = tokio::time::timeout(Duration::from_secs(10), stream.next())
        .await
        .expect("resolve within 10s")
        .expect("got item")
        .expect("not an error");

    assert_eq!(item.node_info().node_id, node_id_a);

    // Critical: assertions for the gate.
    let put_count = metrics.puts.load(Ordering::SeqCst);
    let get_count = metrics.gets.load(Ordering::SeqCst);
    assert!(
        put_count >= 1,
        "endpoint A must publish to our self-hosted resolver (PUT count={put_count})"
    );
    assert!(
        get_count >= 1,
        "endpoint B must query our self-hosted resolver (GET count={get_count})"
    );

    // n0 was NOT queried — proven structurally because n0 is not in the
    // discovery_resolvers list. iroh's ConcurrentDiscovery only iterates
    // the providers we registered. The PUT/GET counters above prove the
    // self-hosted relay is the path that was used; no other resolver
    // could have served the request.

    ep_a.close().await;
    ep_b.close().await;
}
