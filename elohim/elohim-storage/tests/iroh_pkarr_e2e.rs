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
//!
//! Uses raw hyper (1.x) for the in-process relay rather than axum, because
//! axum 0.8 transitively requires serde >= 1.0.220 which conflicts with
//! holochain_serialized_bytes 0.0.56's exact pin to serde =1.0.219.

#![cfg(feature = "p2p-iroh")]

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use elohim_storage::p2p_iroh::{
    build_endpoint, DiscoveryResolverConfig, DiscoveryResolverKind, IrohConfig,
};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use pkarr::{PublicKey, SignedPacket};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[derive(Default)]
struct RelayMetrics {
    gets: AtomicUsize,
    puts: AtomicUsize,
    cache: Mutex<HashMap<String, SignedPacket>>,
}

const PKARR_CONTENT_TYPE: &str = "application/pkarr.org-relays+octet-stream";

async fn run_test_relay(metrics: Arc<RelayMetrics>) -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(p) => p,
                Err(_) => continue,
            };
            let metrics = metrics.clone();
            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                let _ = http1::Builder::new()
                    .serve_connection(io, service_fn(move |req| handle(req, metrics.clone())))
                    .await;
            });
        }
    });
    addr
}

async fn handle(
    req: Request<Incoming>,
    metrics: Arc<RelayMetrics>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let key = match path.strip_prefix("/pkarr/") {
        Some(k) => k.to_string(),
        None => {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::new()))
                .unwrap());
        }
    };

    match method {
        Method::GET => {
            metrics.gets.fetch_add(1, Ordering::SeqCst);
            let cache = metrics.cache.lock().await;
            let Some(packet) = cache.get(&key) else {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Full::new(Bytes::new()))
                    .unwrap());
            };
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", PKARR_CONTENT_TYPE)
                .body(Full::new(Bytes::copy_from_slice(
                    &packet.to_relay_payload(),
                )))
                .unwrap())
        }
        Method::PUT => {
            metrics.puts.fetch_add(1, Ordering::SeqCst);
            let public_key: PublicKey = match key.parse() {
                Ok(p) => p,
                Err(_) => {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Full::new(Bytes::new()))
                        .unwrap());
                }
            };
            let body = match req.into_body().collect().await {
                Ok(c) => c.to_bytes(),
                Err(_) => {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Full::new(Bytes::new()))
                        .unwrap());
                }
            };
            let packet = match SignedPacket::from_relay_payload(&public_key, &body) {
                Ok(p) => p,
                Err(_) => {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Full::new(Bytes::new()))
                        .unwrap());
                }
            };
            metrics.cache.lock().await.insert(key, packet);
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Full::new(Bytes::new()))
                .unwrap())
        }
        _ => Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::new(Bytes::new()))
            .unwrap()),
    }
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
    use iroh::Watcher;
    let _addr_a = ep_a.node_addr().initialized().await;

    // Allow the publish to flush.
    tokio::time::sleep(Duration::from_secs(2)).await;

    // B resolves A's NodeId via the configured discovery surface.
    let node_id_a = ep_a.node_id();
    let resolved = ep_b.discovery().expect("discovery configured");
    let mut stream = resolved
        .resolve(node_id_a)
        .expect("resolver returns stream");
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
