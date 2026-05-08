//! Head-to-head perf comparison: iroh vs libp2p sync plane on loopback.
//!
//! Same workload — fetcher sends `SyncRequest::GetHeads { h_app_id, doc_id }`
//! and provider answers with a fixed `SyncResponse::Heads { heads, ... }` —
//! driven through both transports. Reports p50/p95/p99 + mean per-request
//! latency per transport per workload class, and asserts iroh delivers a
//! perf bump on at least one class.
//!
//! Marked `#[ignore]` so the default `cargo test` run skips it. Invoke
//! explicitly via:
//!
//! ```bash
//! just bench-sync
//! # or:
//! cargo test --release --features "p2p p2p-iroh" \
//!     --test bench_sync_perf -- --ignored --nocapture
//! ```
//!
//! `--release` is load-bearing — debug builds carry ~5-10x overhead and produce
//! noisy numbers that masquerade as transport differences.
//!
//! This bench measures **per-request latency** for the request-response sync
//! plane. Throughput is reported in requests/sec.

#![cfg(all(feature = "p2p", feature = "p2p-iroh"))]

use std::time::{Duration, Instant};
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// Bench result aggregate.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct BenchResult {
    transport: &'static str,
    /// Workload class — number of heads per response. Influences MessagePack
    /// frame size and gives us a few size points without asking the protocol
    /// to carry blob-sized payloads.
    heads_per_response: usize,
    p50: Duration,
    p95: Duration,
    p99: Duration,
    mean: Duration,
    /// Requests per second across all measured iterations.
    rps: f64,
}

impl BenchResult {
    fn from_durations(
        transport: &'static str,
        heads_per_response: usize,
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
            heads_per_response,
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

// ---------------------------------------------------------------------------
// Workload generator — N unique GetHeads requests + matching response shapes.
// ---------------------------------------------------------------------------

/// Build `n` unique doc_ids. Deterministic — re-runs reproduce. The PRNG is
/// the same SplitMix-style stream as `bench_blob_perf::build_payloads`.
fn build_doc_ids(n: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(n);
    let mut state: u64 = 0xDEAD_BEEF_CAFE_F00D;
    for _ in 0..n {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        out.push(format!("doc-{:016x}", z));
    }
    out
}

/// Build `n` deterministic head hashes for the response. Each head is a
/// 64-char hex string (same shape as Automerge change hashes).
fn build_heads(n: usize, seed: u64) -> Vec<String> {
    let mut out = Vec::with_capacity(n);
    let mut state: u64 = seed ^ 0x9E37_79B9_7F4A_7C15;
    for _ in 0..n {
        let mut buf = [0u8; 32];
        for chunk in buf.chunks_mut(8) {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            chunk.copy_from_slice(&z.to_le_bytes());
        }
        out.push(hex::encode(buf));
    }
    out
}

// ---------------------------------------------------------------------------
// Iroh-side bench — IrohNode + sync ALPN handler + reused Connection.
// ---------------------------------------------------------------------------

mod iroh_bench {
    use super::*;
    use elohim_storage::p2p::sync_protocol::{SyncRequest, SyncResponse};
    use elohim_storage::p2p_iroh::{
        AlpnRegistration, IrohConfig, IrohNode, IrohSyncProtocol, SyncBackend, SYNC_ALPN,
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

    /// Backend that returns a fixed-size head list. Used so the latency
    /// measured is transport+codec, not backend compute.
    struct FixedHeadsBackend {
        heads: Vec<String>,
    }

    #[async_trait::async_trait]
    impl SyncBackend for FixedHeadsBackend {
        async fn handle(&self, req: SyncRequest) -> SyncResponse {
            match req {
                SyncRequest::GetHeads { h_app_id, doc_id } => SyncResponse::Heads {
                    h_app_id,
                    doc_id,
                    heads: self.heads.clone(),
                    change_count: self.heads.len() as u64,
                },
                other => SyncResponse::Error {
                    message: format!("bench backend: unexpected variant: {other:?}"),
                },
            }
        }
    }

    /// Run the bench: provider serves a fixed-size SyncResponse::Heads;
    /// fetcher issues `total_iters` GetHeads requests over a single reused
    /// QUIC connection (one bidi stream per request).
    pub async fn run(
        doc_ids: Vec<String>,
        heads_per_response: usize,
        warmup: usize,
        measured: usize,
    ) -> BenchResult {
        let total_iters = warmup + measured;
        assert!(
            doc_ids.len() >= total_iters,
            "iroh_bench: need at least warmup+measured doc_ids ({total_iters}), got {}",
            doc_ids.len()
        );

        let provider_dir = tempdir().expect("provider tempdir");
        let fetcher_dir = tempdir().expect("fetcher tempdir");

        let heads = build_heads(heads_per_response, 0xC0FF_EE);
        let backend: Arc<dyn SyncBackend> = Arc::new(FixedHeadsBackend { heads });

        let provider_protocols: Vec<AlpnRegistration> = vec![(
            SYNC_ALPN.to_vec(),
            Box::new(IrohSyncProtocol::new(backend.clone())),
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

        // Open one QUIC connection up front; reuse for every request.
        let conn = fetcher
            .endpoint()
            .connect(provider_addr, SYNC_ALPN)
            .await
            .expect("fetcher → provider connect");

        async fn send_one(
            conn: &iroh::endpoint::Connection,
            req: &SyncRequest,
        ) -> SyncResponse {
            let (mut send, mut recv) = conn.open_bi().await.expect("open_bi");
            elohim_storage::p2p_iroh::codec::write_frame(&mut send, req)
                .await
                .expect("write_frame");
            send.finish().expect("finish send");
            elohim_storage::p2p_iroh::codec::read_frame_default::<SyncResponse, _>(&mut recv)
                .await
                .expect("read_frame")
        }

        // Warmup — discarded.
        for doc_id in &doc_ids[..warmup] {
            let req = SyncRequest::GetHeads {
                h_app_id: "lamad".into(),
                doc_id: doc_id.clone(),
            };
            let res = send_one(&conn, &req).await;
            match res {
                SyncResponse::Heads { heads, .. } => {
                    assert_eq!(heads.len(), heads_per_response);
                }
                other => panic!("warmup: unexpected variant: {other:?}"),
            }
        }

        // Measured.
        let mut durations = Vec::with_capacity(measured);
        for doc_id in &doc_ids[warmup..warmup + measured] {
            let req = SyncRequest::GetHeads {
                h_app_id: "lamad".into(),
                doc_id: doc_id.clone(),
            };
            let t0 = Instant::now();
            let res = send_one(&conn, &req).await;
            let elapsed = t0.elapsed();
            match res {
                SyncResponse::Heads { heads, .. } => {
                    assert_eq!(heads.len(), heads_per_response);
                }
                other => panic!("measured: unexpected variant: {other:?}"),
            }
            durations.push(elapsed);
        }

        drop(conn);
        provider.shutdown().await.expect("provider shutdown");
        fetcher.shutdown().await.expect("fetcher shutdown");

        BenchResult::from_durations("iroh", heads_per_response, durations)
    }
}

// ---------------------------------------------------------------------------
// Libp2p-side bench — minimal two-swarm setup for /elohim/storage-sync/1.0.0.
// ---------------------------------------------------------------------------

mod libp2p_bench {
    use super::*;
    use elohim_storage::p2p::{
        sync_protocol::{SyncRequest, SyncResponse},
        SyncCodec, SyncProtocol,
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

    type RequestTx = oneshot::Sender<Result<SyncResponse, String>>;

    enum Cmd {
        Dial(Multiaddr, oneshot::Sender<Result<(), String>>),
        Request(PeerId, SyncRequest, RequestTx),
        WaitConnected(PeerId, oneshot::Sender<()>),
    }

    struct Node {
        peer_id: PeerId,
        addr: Multiaddr,
        cmd_tx: mpsc::Sender<Cmd>,
    }

    /// Build a node. `heads` (provider only) is the fixed heads list returned
    /// by every GetHeads response. Pass an empty Vec for the fetcher.
    async fn spawn_node(heads: Vec<String>) -> Node {
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
                request_response::Behaviour::<SyncCodec>::with_codec(
                    SyncCodec,
                    [(SyncProtocol, ProtocolSupport::Full)],
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
                                let req_id = swarm
                                    .behaviour_mut()
                                    .send_request(&peer, req);
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
                                let response = match request {
                                    SyncRequest::GetHeads { h_app_id, doc_id } => {
                                        SyncResponse::Heads {
                                            h_app_id,
                                            doc_id,
                                            heads: heads.clone(),
                                            change_count: heads.len() as u64,
                                        }
                                    }
                                    other => SyncResponse::Error {
                                        message: format!("unexpected: {other:?}"),
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

        Node {
            peer_id: local_peer_id,
            addr: listen_addr,
            cmd_tx,
        }
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

    async fn request(node: &Node, peer: PeerId, req: SyncRequest) -> SyncResponse {
        let (tx, rx) = oneshot::channel();
        node.cmd_tx.send(Cmd::Request(peer, req, tx)).await.unwrap();
        rx.await.unwrap().expect("request")
    }

    pub async fn run(
        doc_ids: Vec<String>,
        heads_per_response: usize,
        warmup: usize,
        measured: usize,
    ) -> BenchResult {
        let total_iters = warmup + measured;
        assert!(
            doc_ids.len() >= total_iters,
            "libp2p_bench: need at least warmup+measured doc_ids ({total_iters}), got {}",
            doc_ids.len()
        );

        let heads = build_heads(heads_per_response, 0xC0FF_EE);

        let provider = spawn_node(heads.clone()).await;
        let fetcher = spawn_node(Vec::new()).await;

        dial(&fetcher, provider.addr.clone()).await;
        wait_connected(&fetcher, provider.peer_id).await;

        // Warmup.
        for doc_id in &doc_ids[..warmup] {
            let req = SyncRequest::GetHeads {
                h_app_id: "lamad".into(),
                doc_id: doc_id.clone(),
            };
            let res = request(&fetcher, provider.peer_id, req).await;
            match res {
                SyncResponse::Heads { heads: h, .. } => {
                    assert_eq!(h.len(), heads_per_response);
                }
                other => panic!("warmup: unexpected: {other:?}"),
            }
        }

        // Measured.
        let mut durations = Vec::with_capacity(measured);
        for doc_id in &doc_ids[warmup..warmup + measured] {
            let req = SyncRequest::GetHeads {
                h_app_id: "lamad".into(),
                doc_id: doc_id.clone(),
            };
            let t0 = Instant::now();
            let res = request(&fetcher, provider.peer_id, req).await;
            let elapsed = t0.elapsed();
            match res {
                SyncResponse::Heads { heads: h, .. } => {
                    assert_eq!(h.len(), heads_per_response);
                }
                other => panic!("measured: unexpected: {other:?}"),
            }
            durations.push(elapsed);
        }

        BenchResult::from_durations("libp2p", heads_per_response, durations)
    }
}

// ---------------------------------------------------------------------------
// Comparison test — drives both, prints markdown table, asserts perf bump.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn compare_sync_perf() {
    // Workload classes — number of heads returned per response. Spans the
    // tiny-control regime (1 head) up through medium-sized responses.
    // 1024 heads × ~70 bytes/head ≈ 70 KiB which is well under the codec
    // 64 MiB cap.
    let head_counts: &[usize] = &[1, 16, 256, 1024];
    let warmup = 5;
    let measured = 30;

    let total_iters = warmup + measured;
    let mut all_results: Vec<BenchResult> = Vec::with_capacity(head_counts.len() * 2);

    for &heads_per_response in head_counts {
        let doc_ids = build_doc_ids(total_iters);

        let iroh_res =
            iroh_bench::run(doc_ids.clone(), heads_per_response, warmup, measured).await;
        all_results.push(iroh_res);

        let libp2p_res =
            libp2p_bench::run(doc_ids, heads_per_response, warmup, measured).await;
        all_results.push(libp2p_res);
    }

    println!();
    println!("## Sync plane perf — iroh vs libp2p (loopback, release)");
    println!();
    println!(
        "| heads/response | transport | p50 (ms) | p95 (ms) | p99 (ms) | mean (ms) | rps |"
    );
    println!("|---|---|---|---|---|---|---|");
    for r in &all_results {
        println!(
            "| {} | {} | {} | {} | {} | {} | {:.1} |",
            r.heads_per_response,
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
        "Iters: {warmup} warmup discarded + {measured} measured per (transport, head_count). \
        Single QUIC Connection / TCP+yamux Connection reused across iterations on each side; \
        iroh opens a fresh bidi stream per request, libp2p uses request-response."
    );
    println!();

    let mut by_class: std::collections::BTreeMap<
        usize,
        (Option<&BenchResult>, Option<&BenchResult>),
    > = std::collections::BTreeMap::new();
    for r in &all_results {
        let entry = by_class.entry(r.heads_per_response).or_insert((None, None));
        match r.transport {
            "iroh" => entry.0 = Some(r),
            "libp2p" => entry.1 = Some(r),
            _ => unreachable!(),
        }
    }

    let mut bump_found = false;
    let mut bump_summary: Vec<String> = Vec::new();
    for (heads, (iroh, libp2p)) in &by_class {
        let (Some(i), Some(l)) = (iroh, libp2p) else {
            continue;
        };
        let ratio_p50 = l.p50.as_secs_f64() / i.p50.as_secs_f64().max(f64::EPSILON);
        let ratio_p99 = l.p99.as_secs_f64() / i.p99.as_secs_f64().max(f64::EPSILON);
        let line = format!(
            "  heads={:>5}: iroh/libp2p p50 ratio = {:.2}x, p99 ratio = {:.2}x  ({})",
            heads,
            ratio_p50,
            ratio_p99,
            if i.p50 < l.p50 {
                "iroh wins p50"
            } else {
                "iroh ≥ libp2p p50"
            },
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
        Expected: iroh p50 < libp2p p50 on at least one of {head_counts:?}. \
        See table above for details."
    );

    println!("Tier-3 verdict: iroh delivers a perf bump on at least one isolated size class.");
}
