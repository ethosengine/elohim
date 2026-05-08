//! Head-to-head perf comparison: iroh vs libp2p view-federation plane.
//!
//! Same workload — fetcher sends `ViewFederationRequest` and provider answers
//! with a signed `ViewFederationResponse` carrying a `ViewSlice` payload —
//! driven through both transports. View-federation has a 256 KiB payload cap.
//!
//! Marked `#[ignore]` so the default `cargo test` run skips it. Invoke via:
//!
//! ```bash
//! just bench-view-fed
//! # or:
//! cargo test --release --features "p2p p2p-iroh" \
//!     --test bench_view_fed_perf -- --ignored --nocapture
//! ```

#![cfg(all(feature = "p2p", feature = "p2p-iroh"))]

use std::time::{Duration, Instant};
use tempfile::tempdir;

#[derive(Debug, Clone)]
struct BenchResult {
    transport: &'static str,
    /// Workload class — number of edges in the topology payload.
    edges: usize,
    p50: Duration,
    p95: Duration,
    p99: Duration,
    mean: Duration,
    rps: f64,
}

impl BenchResult {
    fn from_durations(
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

    pub async fn run(
        request_ids: Vec<String>,
        edges: usize,
        warmup: usize,
        measured: usize,
    ) -> BenchResult {
        let total_iters = warmup + measured;
        assert!(request_ids.len() >= total_iters);

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

        let conn = fetcher
            .endpoint()
            .connect(provider_addr, VIEW_FED_ALPN)
            .await
            .expect("fetcher → provider connect");

        async fn send_one(
            conn: &iroh::endpoint::Connection,
            req: &ViewFederationRequest,
        ) -> ViewFederationResponse {
            let (mut send, mut recv) = conn.open_bi().await.expect("open_bi");
            elohim_storage::p2p_iroh::codec::write_frame(&mut send, req)
                .await
                .expect("write_frame");
            send.finish().expect("finish send");
            elohim_storage::p2p_iroh::codec::read_frame_default::<ViewFederationResponse, _>(
                &mut recv,
            )
            .await
            .expect("read_frame")
        }

        for rid in &request_ids[..warmup] {
            let req = ViewFederationRequest {
                view_kind: ViewKind::PeerTopology,
                agent_cid: "agent-bench".into(),
                request_id: rid.clone(),
            };
            let res = send_one(&conn, &req).await;
            assert_eq!(&res.request_id, rid);
        }

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

        BenchResult::from_durations("iroh", edges, durations)
    }
}

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
                                let mut still = Vec::new();
                                for (p, tx) in std::mem::take(&mut wait_connected) {
                                    if p == peer_id { let _ = tx.send(()); }
                                    else { still.push((p, tx)); }
                                }
                                wait_connected = still;
                            }
                            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                                connected.remove(&peer_id);
                            }
                            SwarmEvent::Behaviour(request_response::Event::Message {
                                message: request_response::Message::Request { request, channel, .. },
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
                                let _ = swarm.behaviour_mut().send_response(channel, response);
                            }
                            SwarmEvent::Behaviour(request_response::Event::Message {
                                message: request_response::Message::Response { request_id, response },
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
        node.cmd_tx.send(Cmd::WaitConnected(peer, tx)).await.unwrap();
        let _ = tokio::time::timeout(Duration::from_secs(5), rx)
            .await
            .expect("wait_connected timed out");
    }

    async fn request(node: &Node, peer: PeerId, req: ViewFederationRequest) -> ViewFederationResponse {
        let (tx, rx) = oneshot::channel();
        node.cmd_tx.send(Cmd::Request(peer, req, tx)).await.unwrap();
        rx.await.unwrap().expect("request")
    }

    pub async fn run(
        request_ids: Vec<String>,
        edges: usize,
        warmup: usize,
        measured: usize,
    ) -> BenchResult {
        let total_iters = warmup + measured;
        assert!(request_ids.len() >= total_iters);

        let payload = build_payload(edges);

        let provider = spawn_node(payload).await;
        let fetcher = spawn_node(serde_json::Value::Null).await;

        dial(&fetcher, provider.addr.clone()).await;
        wait_connected(&fetcher, provider.peer_id).await;

        for rid in &request_ids[..warmup] {
            let req = ViewFederationRequest {
                view_kind: ViewKind::PeerTopology,
                agent_cid: "agent-bench".into(),
                request_id: rid.clone(),
            };
            let res = request(&fetcher, provider.peer_id, req).await;
            assert_eq!(&res.request_id, rid);
        }

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

        BenchResult::from_durations("libp2p", edges, durations)
    }
}

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
    let mut all_results: Vec<BenchResult> = Vec::with_capacity(edge_counts.len() * 2);

    for &edges in edge_counts {
        let request_ids = build_request_ids(total_iters);
        let iroh_res = iroh_bench::run(request_ids.clone(), edges, warmup, measured).await;
        all_results.push(iroh_res);

        let libp2p_res = libp2p_bench::run(request_ids, edges, warmup, measured).await;
        all_results.push(libp2p_res);
    }

    println!();
    println!("## View-fed plane perf — iroh vs libp2p (loopback, release)");
    println!();
    println!("| edges | transport | p50 (ms) | p95 (ms) | p99 (ms) | mean (ms) | rps |");
    println!("|---|---|---|---|---|---|---|");
    for r in &all_results {
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
        "Iters: {warmup} warmup discarded + {measured} measured per (transport, edges). \
        ViewFederationRequest/Response round-trip; connections reused; iroh opens fresh bidi streams per request."
    );
    println!();

    let mut by_class: std::collections::BTreeMap<
        usize,
        (Option<&BenchResult>, Option<&BenchResult>),
    > = std::collections::BTreeMap::new();
    for r in &all_results {
        let entry = by_class.entry(r.edges).or_insert((None, None));
        match r.transport {
            "iroh" => entry.0 = Some(r),
            "libp2p" => entry.1 = Some(r),
            _ => unreachable!(),
        }
    }

    let mut bump_found = false;
    let mut bump_summary: Vec<String> = Vec::new();
    for (edges, (iroh, libp2p)) in &by_class {
        let (Some(i), Some(l)) = (iroh, libp2p) else {
            continue;
        };
        let ratio_p50 = l.p50.as_secs_f64() / i.p50.as_secs_f64().max(f64::EPSILON);
        let ratio_p99 = l.p99.as_secs_f64() / i.p99.as_secs_f64().max(f64::EPSILON);
        let line = format!(
            "  edges={:>5}: iroh/libp2p p50 ratio = {:.2}x, p99 ratio = {:.2}x  ({})",
            edges,
            ratio_p50,
            ratio_p99,
            if i.p50 < l.p50 { "iroh wins p50" } else { "iroh ≥ libp2p p50" },
        );
        bump_summary.push(line);
        if i.p50 < l.p50 {
            bump_found = true;
        }
    }

    println!("Perf-bump check:");
    for line in &bump_summary {
        println!("{line}");
    }
    println!();

    assert!(
        bump_found,
        "Tier-3 verdict: iroh did NOT deliver a perf bump on any size class. \
        Expected: iroh p50 < libp2p p50 on at least one of {edge_counts:?}. \
        See table above for details."
    );

    println!("Tier-3 verdict: iroh delivers a perf bump on at least one isolated size class.");
}
