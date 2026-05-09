//! Head-to-head perf comparison: iroh vs libp2p view-federation plane on loopback.
//!
//! Same workload — fetcher sends `ViewFederationRequest` and provider answers
//! with a signed `ViewFederationResponse` carrying a `ViewSlice` payload —
//! driven through both transports under two scenarios:
//!
//! - **fresh** — fresh handshake per request. iroh: `endpoint.connect()` +
//!   `open_bi` + write + read; libp2p: spawn a new fetcher swarm + dial +
//!   request + drop. This is the symmetric "every request pays full
//!   handshake cost" comparison.
//! - **reuse** — open one connection, then issue many requests over fresh
//!   streams (iroh) or fresh request-response interactions (libp2p). This
//!   is the "engine ceiling" — what each transport's stream multiplexing
//!   can deliver once handshake is amortized.
//!
//! View-federation uses `read_frame`/`write_frame` with an explicit 256 KiB
//! `MAX_PAYLOAD` cap (not the `_default` variants). The workload class is
//! number of view-graph edges per response.
//!
//! Reports p50/p95/p99 + mean per-request latency per (scenario, transport,
//! workload class), and asserts iroh delivers a perf bump on at least one
//! edge count within the **reuse** scenario.
//!
//! Marked `#[ignore]` so the default `cargo test` run skips it. Invoke
//! explicitly via:
//!
//! ```bash
//! just bench-view-fed
//! # or:
//! cargo test --release --features "p2p p2p-iroh" \
//!     --test bench_view_fed_perf -- --ignored --nocapture
//! ```
//!
//! `--release` is load-bearing — debug builds carry ~5-10x overhead and produce
//! noisy numbers that masquerade as transport differences.
//!
//! This bench measures **per-request latency** for the request-response
//! view-federation plane. Throughput is reported in requests/sec.

#![cfg(all(feature = "p2p", feature = "p2p-iroh"))]

use std::time::{Duration, Instant};
use tempfile::tempdir;

/// Wall-clock cap on a single bench-side read. Loopback should resolve in
/// milliseconds; anything exceeding this is a hang and we want it to surface
/// loudly instead of dragging on for hours.
const READ_TIMEOUT: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------------------
// Bench result aggregate.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct BenchResult {
    /// "fresh" (handshake per request) or "reuse" (handshake amortized across
    /// many requests). Threaded through so the same renderer can produce
    /// both tables.
    scenario: &'static str,
    transport: &'static str,
    /// Workload class — number of view-graph edges per response. Each edge
    /// is ~120 bytes serialized, so 256 edges ≈ 30 KiB — well under the
    /// 256 KiB view-federation cap.
    edges: usize,
    p50: Duration,
    p95: Duration,
    p99: Duration,
    mean: Duration,
    /// Requests per second across all measured iterations.
    rps: f64,
}

impl BenchResult {
    fn from_durations(
        scenario: &'static str,
        transport: &'static str,
        edges: usize,
        mut ds: Vec<Duration>,
    ) -> Self {
        ds.sort();
        let iters = ds.len();
        let p50 = percentile(&ds, 50.0);
        let p95 = percentile(&ds, 95.0);
        let p99 = percentile(&ds, 99.0);
        let total: Duration = ds.iter().sum();
        let mean = total / iters as u32;
        let rps = if total.is_zero() {
            0.0
        } else {
            iters as f64 / total.as_secs_f64()
        };
        Self {
            scenario,
            transport,
            edges,
            p50,
            p95,
            p99,
            mean,
            rps,
        }
    }
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    debug_assert!((0.0..=100.0).contains(&p));
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let n = sorted.len();
    let rank = (p / 100.0) * (n as f64 - 1.0);
    let idx = rank.round() as usize;
    sorted[idx.min(n - 1)]
}

fn fmt_ms(d: Duration) -> String {
    format!("{:.3}", d.as_secs_f64() * 1000.0)
}

fn build_request_ids(n: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(n);
    let mut state: u64 = 0xCAFE_F00D_DEAD_BEEF;
    for _ in 0..n {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        out.push(format!("req-{:016x}", z));
    }
    out
}

/// Build a JSON payload with `edges` rows, each ~120 bytes serialized — keeps
/// the encoded slice well under the 256 KiB view-federation cap up to ~1500
/// edges. The bench sweeps small (1) up through medium (256).
fn build_payload(edges: usize) -> serde_json::Value {
    let rows: Vec<serde_json::Value> = (0..edges)
        .map(|i| {
            serde_json::json!({
                "household_id": format!("hh-{:08x}", i),
                "online": (i % 2) == 0,
                "my_cids_hosted_by_them": (i * 17) % 4096,
                "their_cids_hosted_by_me": (i * 23) % 4096,
                "net_diff": (i as i64) - (edges as i64 / 2),
            })
        })
        .collect();
    serde_json::json!({
        "edges": rows,
        "reciprocation_count": edges,
    })
}

// ---------------------------------------------------------------------------
// Iroh-side bench — IrohNode + view-federation ALPN handler. Two scenarios:
//   • run_reuse_conn — open one QUIC Connection up front, fresh bidi stream
//     per request (engine ceiling). Uses read_frame/write_frame with explicit
//     MAX_PAYLOAD (256 KiB) — same as the production IrohViewFederationClient.
//   • run_fresh_conn — open a fresh QUIC Connection per request (handshake
//     cost included in measurement).
// ---------------------------------------------------------------------------

mod iroh_bench {
    use super::*;
    use elohim_storage::p2p_iroh::{
        AlpnRegistration, IrohConfig, IrohNode, IrohViewFederationProtocol, ViewFederationBackend,
        VIEW_FED_ALPN,
    };
    use elohim_storage::views::{
        Freshness, FreshnessState, JsonVal, ViewFederationRequest, ViewFederationResponse,
        ViewKind, ViewSlice,
    };
    use std::sync::Arc;

    // Matches the cap used by IrohViewFederationClient in production.
    use elohim_storage::p2p_iroh::view_fed::MAX_PAYLOAD;

    fn loopback_config(dir: &std::path::Path) -> IrohConfig {
        IrohConfig {
            blobs_dir: dir.join("blobs_iroh"),
            secret_key_path: dir.join("iroh.key"),
            use_n0_relays: false,
            use_n0_discovery: false,
        }
    }

    struct FixedSliceBackend {
        payload: serde_json::Value,
    }

    #[async_trait::async_trait]
    impl ViewFederationBackend for FixedSliceBackend {
        async fn handle(&self, req: ViewFederationRequest) -> ViewFederationResponse {
            ViewFederationResponse {
                view_kind: req.view_kind.clone(),
                agent_cid: req.agent_cid.clone(),
                request_id: req.request_id.clone(),
                slice: ViewSlice {
                    peer_id: "bench-peer".into(),
                    view_kind: req.view_kind,
                    freshness: Freshness {
                        state: FreshnessState::Live,
                        stale_since_ms: None,
                    },
                    payload: JsonVal(self.payload.clone()),
                    signature: "bench-sig".into(),
                },
            }
        }
    }

    /// Drive a single ViewFederationRequest over an existing QUIC connection.
    /// Uses read_frame/write_frame with explicit MAX_PAYLOAD — not _default.
    async fn send_one(
        conn: &iroh::endpoint::Connection,
        req: &ViewFederationRequest,
    ) -> ViewFederationResponse {
        let (mut send, mut recv) = conn.open_bi().await.expect("open_bi");
        elohim_storage::p2p_iroh::codec::write_frame(&mut send, req)
            .await
            .expect("write_frame");
        send.finish().expect("finish send");
        tokio::time::timeout(
            READ_TIMEOUT,
            elohim_storage::p2p_iroh::codec::read_frame::<ViewFederationResponse, _>(
                &mut recv,
                MAX_PAYLOAD,
            ),
        )
        .await
        .expect("iroh read_frame timed out — handler may be wedged")
        .expect("read_frame")
    }

    /// Reuse-conn scenario: open the QUIC connection up front, issue every
    /// request as a fresh bidi stream on the same connection.
    pub async fn run_reuse_conn(
        request_ids: Vec<String>,
        edges: usize,
        warmup: usize,
        measured: usize,
    ) -> BenchResult {
        let total_iters = warmup + measured;
        assert!(
            request_ids.len() >= total_iters,
            "iroh reuse: need at least warmup+measured request_ids ({total_iters}), got {}",
            request_ids.len()
        );

        let provider_dir = tempdir().expect("provider tempdir");
        let fetcher_dir = tempdir().expect("fetcher tempdir");

        let payload = build_payload(edges);
        let backend: Arc<dyn ViewFederationBackend> = Arc::new(FixedSliceBackend { payload });

        let provider_protocols: Vec<AlpnRegistration> = vec![(
            VIEW_FED_ALPN.to_vec(),
            Box::new(IrohViewFederationProtocol::new(backend.clone())),
        )];
        let fetcher_protocols: Vec<AlpnRegistration> = vec![];

        let provider =
            IrohNode::start_with_protocols(loopback_config(provider_dir.path()), provider_protocols)
                .await
                .expect("provider starts");
        let fetcher =
            IrohNode::start_with_protocols(loopback_config(fetcher_dir.path()), fetcher_protocols)
                .await
                .expect("fetcher starts");

        let provider_addr = provider.node_addr().await.expect("provider node_addr");

        // One QUIC connection up front; reuse for every request.
        let conn = fetcher
            .endpoint()
            .connect(provider_addr, VIEW_FED_ALPN)
            .await
            .expect("fetcher → provider connect");

        // Warmup — discarded.
        for rid in &request_ids[..warmup] {
            let req = ViewFederationRequest {
                view_kind: ViewKind::PeerTopology,
                agent_cid: "agent-bench".into(),
                request_id: rid.clone(),
            };
            let res = send_one(&conn, &req).await;
            assert_eq!(&res.request_id, rid);
        }

        // Measured.
        let mut durations = Vec::with_capacity(measured);
        for rid in &request_ids[warmup..warmup + measured] {
            let req = ViewFederationRequest {
                view_kind: ViewKind::PeerTopology,
                agent_cid: "agent-bench".into(),
                request_id: rid.clone(),
            };
            let t0 = Instant::now();
            let res = send_one(&conn, &req).await;
            let elapsed = t0.elapsed();
            assert_eq!(&res.request_id, rid);
            durations.push(elapsed);
        }

        drop(conn);
        provider.shutdown().await.expect("provider shutdown");
        fetcher.shutdown().await.expect("fetcher shutdown");

        BenchResult::from_durations("reuse", "iroh", edges, durations)
    }

    /// Fresh-conn scenario: open a new QUIC connection for every request,
    /// drop it after the response. Measured clock includes connection setup.
    pub async fn run_fresh_conn(
        request_ids: Vec<String>,
        edges: usize,
        warmup: usize,
        measured: usize,
    ) -> BenchResult {
        let total_iters = warmup + measured;
        assert!(
            request_ids.len() >= total_iters,
            "iroh fresh: need at least warmup+measured request_ids ({total_iters}), got {}",
            request_ids.len()
        );

        let provider_dir = tempdir().expect("provider tempdir");
        let fetcher_dir = tempdir().expect("fetcher tempdir");

        let payload = build_payload(edges);
        let backend: Arc<dyn ViewFederationBackend> = Arc::new(FixedSliceBackend { payload });

        let provider_protocols: Vec<AlpnRegistration> = vec![(
            VIEW_FED_ALPN.to_vec(),
            Box::new(IrohViewFederationProtocol::new(backend.clone())),
        )];
        let fetcher_protocols: Vec<AlpnRegistration> = vec![];

        let provider =
            IrohNode::start_with_protocols(loopback_config(provider_dir.path()), provider_protocols)
                .await
                .expect("provider starts");
        let fetcher =
            IrohNode::start_with_protocols(loopback_config(fetcher_dir.path()), fetcher_protocols)
                .await
                .expect("fetcher starts");

        let provider_addr = provider.node_addr().await.expect("provider node_addr");

        // Warmup — discarded. Fresh connection each iteration.
        for rid in &request_ids[..warmup] {
            let req = ViewFederationRequest {
                view_kind: ViewKind::PeerTopology,
                agent_cid: "agent-bench".into(),
                request_id: rid.clone(),
            };
            let conn = fetcher
                .endpoint()
                .connect(provider_addr.clone(), VIEW_FED_ALPN)
                .await
                .expect("fetcher → provider connect (warmup)");
            let res = send_one(&conn, &req).await;
            assert_eq!(&res.request_id, rid);
            drop(conn);
        }

        // Measured. Each iteration's clock includes connection setup.
        let mut durations = Vec::with_capacity(measured);
        for rid in &request_ids[warmup..warmup + measured] {
            let req = ViewFederationRequest {
                view_kind: ViewKind::PeerTopology,
                agent_cid: "agent-bench".into(),
                request_id: rid.clone(),
            };
            let t0 = Instant::now();
            let conn = fetcher
                .endpoint()
                .connect(provider_addr.clone(), VIEW_FED_ALPN)
                .await
                .expect("fetcher → provider connect (measured)");
            let res = send_one(&conn, &req).await;
            let elapsed = t0.elapsed();
            drop(conn);
            assert_eq!(&res.request_id, rid);
            durations.push(elapsed);
        }

        provider.shutdown().await.expect("provider shutdown");
        fetcher.shutdown().await.expect("fetcher shutdown");

        BenchResult::from_durations("fresh", "iroh", edges, durations)
    }
}

// ---------------------------------------------------------------------------
// Libp2p-side bench — minimal two-swarm setup for the view-federation ALPN.
//   • run_reuse_conn — provider+fetcher up once, request_response uses one
//     long-lived TCP+yamux connection.
//   • run_fresh_conn — provider stays alive, fetcher swarm is freshly built
//     and dialed for every request, then dropped (TCP+Noise+Yamux handshake
//     per request).
// ---------------------------------------------------------------------------

mod libp2p_bench {
    use super::*;
    use elohim_storage::p2p::{ViewFederationCodec, ViewFederationProtocol};
    use elohim_storage::views::{
        Freshness, FreshnessState, JsonVal, ViewFederationRequest, ViewFederationResponse,
        ViewKind, ViewSlice,
    };
    use futures::StreamExt;
    use libp2p::{
        identity, noise,
        request_response::{self, ProtocolSupport},
        swarm::SwarmEvent,
        tcp, yamux, Multiaddr, PeerId, SwarmBuilder,
    };
    use std::collections::HashMap;
    use tokio::sync::{mpsc, oneshot};

    type RequestTx = oneshot::Sender<Result<ViewFederationResponse, String>>;

    enum Cmd {
        Dial(Multiaddr, oneshot::Sender<Result<(), String>>),
        Request(PeerId, ViewFederationRequest, RequestTx),
        WaitConnected(PeerId, oneshot::Sender<()>),
    }

    struct Node {
        peer_id: PeerId,
        addr: Multiaddr,
        cmd_tx: mpsc::Sender<Cmd>,
    }

    async fn spawn_node(payload: serde_json::Value) -> Node {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        let mut swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .expect("tcp transport")
            .with_behaviour(|_| {
                request_response::Behaviour::<ViewFederationCodec>::with_codec(
                    ViewFederationCodec,
                    [(ViewFederationProtocol, ProtocolSupport::Full)],
                    request_response::Config::default()
                        .with_request_timeout(Duration::from_secs(60)),
                )
            })
            .expect("behaviour build")
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(120)))
            .build();

        swarm
            .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
            .expect("listen");

        let listen_addr = loop {
            match swarm.next().await.expect("first event") {
                SwarmEvent::NewListenAddr { address, .. } => break address,
                _ => continue,
            }
        };

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<Cmd>(64);

        tokio::spawn(async move {
            let mut pending: HashMap<request_response::OutboundRequestId, RequestTx> =
                HashMap::new();
            let mut connected: HashMap<PeerId, ()> = HashMap::new();
            let mut wait_connected: Vec<(PeerId, oneshot::Sender<()>)> = Vec::new();

            loop {
                tokio::select! {
                    biased;

                    Some(cmd) = cmd_rx.recv() => {
                        match cmd {
                            Cmd::Dial(addr, reply) => {
                                let _ = reply.send(swarm.dial(addr).map_err(|e| e.to_string()));
                            }
                            Cmd::Request(peer, req, reply) => {
                                let req_id = swarm.behaviour_mut().send_request(&peer, req);
                                pending.insert(req_id, reply);
                            }
                            Cmd::WaitConnected(peer, reply) => {
                                if connected.contains_key(&peer) {
                                    let _ = reply.send(());
                                } else {
                                    wait_connected.push((peer, reply));
                                }
                            }
                        }
                    }

                    event = swarm.next() => {
                        let Some(event) = event else { break; };
                        match event {
                            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                                connected.insert(peer_id, ());
                                let mut still_waiting = Vec::new();
                                for (p, tx) in std::mem::take(&mut wait_connected) {
                                    if p == peer_id {
                                        let _ = tx.send(());
                                    } else {
                                        still_waiting.push((p, tx));
                                    }
                                }
                                wait_connected = still_waiting;
                            }
                            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                                connected.remove(&peer_id);
                            }
                            SwarmEvent::Behaviour(request_response::Event::Message {
                                message: request_response::Message::Request {
                                    request, channel, ..
                                },
                                ..
                            }) => {
                                let response = ViewFederationResponse {
                                    view_kind: request.view_kind.clone(),
                                    agent_cid: request.agent_cid.clone(),
                                    request_id: request.request_id.clone(),
                                    slice: ViewSlice {
                                        peer_id: "bench-peer".into(),
                                        view_kind: request.view_kind,
                                        freshness: Freshness {
                                            state: FreshnessState::Live,
                                            stale_since_ms: None,
                                        },
                                        payload: JsonVal(payload.clone()),
                                        signature: "bench-sig".into(),
                                    },
                                };
                                let _ = swarm
                                    .behaviour_mut()
                                    .send_response(channel, response);
                            }
                            SwarmEvent::Behaviour(request_response::Event::Message {
                                message: request_response::Message::Response {
                                    request_id, response,
                                },
                                ..
                            }) => {
                                if let Some(tx) = pending.remove(&request_id) {
                                    let _ = tx.send(Ok(response));
                                }
                            }
                            SwarmEvent::Behaviour(request_response::Event::OutboundFailure {
                                request_id, error, ..
                            }) => {
                                if let Some(tx) = pending.remove(&request_id) {
                                    let _ = tx.send(Err(format!("outbound: {error:?}")));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        Node { peer_id: local_peer_id, addr: listen_addr, cmd_tx }
    }

    async fn dial(node: &Node, addr: Multiaddr) {
        let (tx, rx) = oneshot::channel();
        node.cmd_tx.send(Cmd::Dial(addr, tx)).await.unwrap();
        rx.await.unwrap().expect("dial");
    }

    async fn wait_connected(node: &Node, peer: PeerId) {
        let (tx, rx) = oneshot::channel();
        node.cmd_tx
            .send(Cmd::WaitConnected(peer, tx))
            .await
            .unwrap();
        let _ = tokio::time::timeout(Duration::from_secs(5), rx)
            .await
            .expect("wait_connected timed out");
    }

    async fn request(
        node: &Node,
        peer: PeerId,
        req: ViewFederationRequest,
    ) -> ViewFederationResponse {
        let (tx, rx) = oneshot::channel();
        node.cmd_tx.send(Cmd::Request(peer, req, tx)).await.unwrap();
        tokio::time::timeout(READ_TIMEOUT, rx)
            .await
            .expect("libp2p request timed out — peer may be wedged")
            .unwrap()
            .expect("request")
    }

    /// Reuse-conn scenario: provider+fetcher built once, single TCP+yamux
    /// connection between them, request_response sends every iteration as
    /// a fresh substream on the existing connection.
    pub async fn run_reuse_conn(
        request_ids: Vec<String>,
        edges: usize,
        warmup: usize,
        measured: usize,
    ) -> BenchResult {
        let total_iters = warmup + measured;
        assert!(
            request_ids.len() >= total_iters,
            "libp2p reuse: need at least warmup+measured request_ids ({total_iters}), got {}",
            request_ids.len()
        );

        let payload = build_payload(edges);

        let provider = spawn_node(payload.clone()).await;
        let fetcher = spawn_node(serde_json::Value::Null).await;

        dial(&fetcher, provider.addr.clone()).await;
        wait_connected(&fetcher, provider.peer_id).await;

        // Warmup.
        for rid in &request_ids[..warmup] {
            let req = ViewFederationRequest {
                view_kind: ViewKind::PeerTopology,
                agent_cid: "agent-bench".into(),
                request_id: rid.clone(),
            };
            let res = request(&fetcher, provider.peer_id, req).await;
            assert_eq!(&res.request_id, rid);
        }

        // Measured.
        let mut durations = Vec::with_capacity(measured);
        for rid in &request_ids[warmup..warmup + measured] {
            let req = ViewFederationRequest {
                view_kind: ViewKind::PeerTopology,
                agent_cid: "agent-bench".into(),
                request_id: rid.clone(),
            };
            let t0 = Instant::now();
            let res = request(&fetcher, provider.peer_id, req).await;
            let elapsed = t0.elapsed();
            assert_eq!(&res.request_id, rid);
            durations.push(elapsed);
        }

        BenchResult::from_durations("reuse", "libp2p", edges, durations)
    }

    /// Fresh-conn scenario: provider stays alive across iterations, but
    /// every iteration spawns a brand new fetcher swarm, dials, sends one
    /// request, and drops. Iteration timing includes TCP+Noise+Yamux
    /// handshake cost.
    pub async fn run_fresh_conn(
        request_ids: Vec<String>,
        edges: usize,
        warmup: usize,
        measured: usize,
    ) -> BenchResult {
        let total_iters = warmup + measured;
        assert!(
            request_ids.len() >= total_iters,
            "libp2p fresh: need at least warmup+measured request_ids ({total_iters}), got {}",
            request_ids.len()
        );

        let payload = build_payload(edges);

        let provider = spawn_node(payload.clone()).await;

        // Warmup — fresh fetcher per iteration, discarded.
        for rid in &request_ids[..warmup] {
            let fetcher = spawn_node(serde_json::Value::Null).await;
            dial(&fetcher, provider.addr.clone()).await;
            wait_connected(&fetcher, provider.peer_id).await;
            let req = ViewFederationRequest {
                view_kind: ViewKind::PeerTopology,
                agent_cid: "agent-bench".into(),
                request_id: rid.clone(),
            };
            let res = request(&fetcher, provider.peer_id, req).await;
            assert_eq!(&res.request_id, rid);
            drop(fetcher);
        }

        // Measured — each iteration's clock starts at swarm spawn so the
        // measurement captures full handshake cost.
        let mut durations = Vec::with_capacity(measured);
        for rid in &request_ids[warmup..warmup + measured] {
            let req = ViewFederationRequest {
                view_kind: ViewKind::PeerTopology,
                agent_cid: "agent-bench".into(),
                request_id: rid.clone(),
            };
            let t0 = Instant::now();
            let fetcher = spawn_node(serde_json::Value::Null).await;
            dial(&fetcher, provider.addr.clone()).await;
            wait_connected(&fetcher, provider.peer_id).await;
            let res = request(&fetcher, provider.peer_id, req).await;
            let elapsed = t0.elapsed();
            drop(fetcher);
            assert_eq!(&res.request_id, rid);
            durations.push(elapsed);
        }

        BenchResult::from_durations("fresh", "libp2p", edges, durations)
    }
}

// ---------------------------------------------------------------------------
// Comparison test — drives both scenarios on both stacks, prints two
// markdown tables, asserts perf bump on the reuse scenario.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn compare_view_fed_perf() {
    // Edge counts span single-edge (control) through medium topologies.
    // Each edge ≈ 120 bytes encoded → 256 edges ≈ 30 KiB, well under
    // 256 KiB cap.
    let edge_counts: &[usize] = &[1, 16, 64, 256];
    let warmup = 5;
    let measured = 30;

    let total_iters = warmup + measured;
    let mut all_results: Vec<BenchResult> = Vec::new();

    for &edges in edge_counts {
        let request_ids = build_request_ids(total_iters);

        // Reuse scenario — engine ceiling.
        all_results.push(
            iroh_bench::run_reuse_conn(request_ids.clone(), edges, warmup, measured).await,
        );
        all_results.push(
            libp2p_bench::run_reuse_conn(request_ids.clone(), edges, warmup, measured).await,
        );

        // Fresh scenario — handshake-per-request.
        all_results.push(
            iroh_bench::run_fresh_conn(request_ids.clone(), edges, warmup, measured).await,
        );
        all_results.push(
            libp2p_bench::run_fresh_conn(request_ids, edges, warmup, measured).await,
        );
    }

    // ----- Print a table per scenario -----
    print_scenario_table(
        &all_results,
        "reuse",
        "View-fed plane perf — REUSE scenario (engine ceiling)",
        "One QUIC Connection / TCP+yamux Connection reused across iterations on each side; \
         iroh opens a fresh bidi stream per request (with explicit 256 KiB MAX_PAYLOAD cap), \
         libp2p uses request_response substreams. \
         Handshake cost amortized across all measured iterations.",
        warmup,
        measured,
    );
    print_scenario_table(
        &all_results,
        "fresh",
        "View-fed plane perf — FRESH scenario (handshake per request)",
        "Each iteration pays full handshake cost. iroh: fresh `endpoint.connect()` per request \
         (256 KiB MAX_PAYLOAD). libp2p: fresh fetcher swarm spawned and dialed per request \
         (TCP+Noise+Yamux handshake on each iteration).",
        warmup,
        measured,
    );

    // ----- Perf-bump assertion (engine ceiling — reuse scenario only) -----
    let mut bump_summary: Vec<String> = Vec::new();
    let mut bump_found = false;
    let by_class = group_by_class(&all_results, "reuse");
    for (edges, (iroh, libp2p)) in &by_class {
        let (Some(i), Some(l)) = (iroh, libp2p) else {
            continue;
        };
        let ratio_p50 = l.p50.as_secs_f64() / i.p50.as_secs_f64().max(f64::EPSILON);
        let ratio_p99 = l.p99.as_secs_f64() / i.p99.as_secs_f64().max(f64::EPSILON);
        bump_summary.push(format!(
            "  edges={:>5}: iroh/libp2p p50 ratio = {:.2}x, p99 ratio = {:.2}x  ({})",
            edges,
            ratio_p50,
            ratio_p99,
            if i.p50 < l.p50 {
                "iroh wins p50"
            } else {
                "iroh ≥ libp2p p50"
            },
        ));
        if i.p50 < l.p50 {
            bump_found = true;
        }
    }
    println!("Perf-bump check (reuse scenario):");
    for line in &bump_summary {
        println!("{line}");
    }
    println!();

    // ----- Informational ratios for the fresh scenario -----
    let mut fresh_summary: Vec<String> = Vec::new();
    let by_class_fresh = group_by_class(&all_results, "fresh");
    for (edges, (iroh, libp2p)) in &by_class_fresh {
        let (Some(i), Some(l)) = (iroh, libp2p) else {
            continue;
        };
        let ratio_p50 = l.p50.as_secs_f64() / i.p50.as_secs_f64().max(f64::EPSILON);
        let ratio_p99 = l.p99.as_secs_f64() / i.p99.as_secs_f64().max(f64::EPSILON);
        fresh_summary.push(format!(
            "  edges={:>5}: iroh/libp2p p50 ratio = {:.2}x, p99 ratio = {:.2}x  ({})",
            edges,
            ratio_p50,
            ratio_p99,
            if i.p50 < l.p50 {
                "iroh wins p50"
            } else {
                "libp2p wins p50"
            },
        ));
    }
    println!("Handshake comparison (fresh scenario, informational — no assertion):");
    for line in &fresh_summary {
        println!("{line}");
    }
    println!();

    assert!(
        bump_found,
        "Tier-3 verdict: iroh did NOT deliver a perf bump on any edge count in REUSE. \
        Expected: iroh p50 < libp2p p50 on at least one of {edge_counts:?} for the reuse \
        scenario. See tables above for details."
    );

    println!(
        "Tier-3 verdict: iroh delivers a perf bump on at least one isolated edge count \
         (reuse scenario)."
    );
}

fn print_scenario_table(
    all_results: &[BenchResult],
    scenario: &str,
    title: &str,
    footnote: &str,
    warmup: usize,
    measured: usize,
) {
    println!();
    println!("## {title}");
    println!();
    println!("| edges | transport | p50 (ms) | p95 (ms) | p99 (ms) | mean (ms) | rps |");
    println!("|---|---|---|---|---|---|---|");
    for r in all_results.iter().filter(|r| r.scenario == scenario) {
        println!(
            "| {} | {} | {} | {} | {} | {} | {:.1} |",
            r.edges,
            r.transport,
            fmt_ms(r.p50),
            fmt_ms(r.p95),
            fmt_ms(r.p99),
            fmt_ms(r.mean),
            r.rps,
        );
    }
    println!();
    println!(
        "Iters: {warmup} warmup discarded + {measured} measured per (transport, edges). {footnote}"
    );
    println!();
}

fn group_by_class<'a>(
    all_results: &'a [BenchResult],
    scenario: &str,
) -> std::collections::BTreeMap<usize, (Option<&'a BenchResult>, Option<&'a BenchResult>)> {
    let mut by_class: std::collections::BTreeMap<
        usize,
        (Option<&'a BenchResult>, Option<&'a BenchResult>),
    > = std::collections::BTreeMap::new();
    for r in all_results.iter().filter(|r| r.scenario == scenario) {
        let entry = by_class.entry(r.edges).or_insert((None, None));
        match r.transport {
            "iroh" => entry.0 = Some(r),
            "libp2p" => entry.1 = Some(r),
            _ => unreachable!(),
        }
    }
    by_class
}
