//! Head-to-head perf comparison: iroh vs libp2p shard plane on loopback.
//!
//! Same workload — fetcher sends `ShardRequest::Get { hash }` and provider
//! answers with `ShardResponse::Data(bytes)` — driven through both transports.
//!
//! Marked `#[ignore]` so the default `cargo test` run skips it. Invoke via:
//!
//! ```bash
//! just bench-shard
//! # or:
//! cargo test --release --features "p2p p2p-iroh" \
//!     --test bench_shard_perf -- --ignored --nocapture
//! ```

#![cfg(all(feature = "p2p", feature = "p2p-iroh"))]

use std::time::{Duration, Instant};
use tempfile::tempdir;

#[derive(Debug, Clone)]
struct BenchResult {
    transport: &'static str,
    payload_size: usize,
    p50: Duration,
    p95: Duration,
    p99: Duration,
    mean: Duration,
    rps: f64,
}

impl BenchResult {
    fn from_durations(
        transport: &'static str,
        payload_size: usize,
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
            payload_size,
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

fn fmt_size(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{} MiB", bytes / (1024 * 1024))
    } else if bytes >= 1024 {
        format!("{} KiB", bytes / 1024)
    } else {
        format!("{} B", bytes)
    }
}

fn build_hashes(n: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(n);
    let mut state: u64 = 0xFACE_BABE_DEAD_BEEF;
    for _ in 0..n {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        out.push(format!("sha256-{:016x}{:016x}", z, !z));
    }
    out
}

fn build_payload(size: usize, seed: u64) -> Vec<u8> {
    let mut buf = vec![0u8; size];
    let mut state: u64 = seed ^ 0x9E37_79B9_7F4A_7C15;
    for chunk in buf.chunks_mut(8) {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        let bytes = z.to_le_bytes();
        chunk.copy_from_slice(&bytes[..chunk.len()]);
    }
    buf
}

mod iroh_bench {
    use super::*;
    use elohim_storage::p2p::shard_protocol::{ShardRequest, ShardResponse};
    use elohim_storage::p2p_iroh::{
        AlpnRegistration, IrohConfig, IrohNode, IrohShardProtocol, ShardBackend, SHARD_ALPN,
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

    struct FixedShardBackend {
        data: Vec<u8>,
    }

    #[async_trait::async_trait]
    impl ShardBackend for FixedShardBackend {
        async fn handle(&self, req: ShardRequest) -> ShardResponse {
            match req {
                ShardRequest::Get { .. } => ShardResponse::Data(self.data.clone()),
                ShardRequest::Have { .. } => ShardResponse::Have(true),
                _ => ShardResponse::NotFound,
            }
        }
    }

    pub async fn run(
        hashes: Vec<String>,
        payload_size: usize,
        warmup: usize,
        measured: usize,
    ) -> BenchResult {
        let total_iters = warmup + measured;
        assert!(hashes.len() >= total_iters);

        let provider_dir = tempdir().expect("provider tempdir");
        let fetcher_dir = tempdir().expect("fetcher tempdir");

        let data = build_payload(payload_size, 0xC0DE_C0DE);
        let backend: Arc<dyn ShardBackend> = Arc::new(FixedShardBackend { data });

        let provider_protocols: Vec<AlpnRegistration> = vec![(
            SHARD_ALPN.to_vec(),
            Box::new(IrohShardProtocol::new(backend.clone())),
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
            .connect(provider_addr, SHARD_ALPN)
            .await
            .expect("fetcher → provider connect");

        async fn send_one(
            conn: &iroh::endpoint::Connection,
            req: &ShardRequest,
        ) -> ShardResponse {
            let (mut send, mut recv) = conn.open_bi().await.expect("open_bi");
            elohim_storage::p2p_iroh::codec::write_frame(&mut send, req)
                .await
                .expect("write_frame");
            send.finish().expect("finish send");
            elohim_storage::p2p_iroh::codec::read_frame_default::<ShardResponse, _>(&mut recv)
                .await
                .expect("read_frame")
        }

        for hash in &hashes[..warmup] {
            let req = ShardRequest::Get { hash: hash.clone() };
            let res = send_one(&conn, &req).await;
            match res {
                ShardResponse::Data(b) => assert_eq!(b.len(), payload_size),
                other => panic!("warmup: unexpected: {other:?}"),
            }
        }

        let mut durations = Vec::with_capacity(measured);
        for hash in &hashes[warmup..warmup + measured] {
            let req = ShardRequest::Get { hash: hash.clone() };
            let t0 = Instant::now();
            let res = send_one(&conn, &req).await;
            let elapsed = t0.elapsed();
            match res {
                ShardResponse::Data(b) => assert_eq!(b.len(), payload_size),
                other => panic!("measured: unexpected: {other:?}"),
            }
            durations.push(elapsed);
        }

        drop(conn);
        provider.shutdown().await.expect("provider shutdown");
        fetcher.shutdown().await.expect("fetcher shutdown");

        BenchResult::from_durations("iroh", payload_size, durations)
    }
}

mod libp2p_bench {
    use super::*;
    use elohim_storage::p2p::{ShardCodec, ShardProtocol, ShardRequest, ShardResponse};
    use futures::StreamExt;
    use libp2p::{
        identity, noise,
        request_response::{self, ProtocolSupport},
        swarm::SwarmEvent,
        tcp, yamux, Multiaddr, PeerId, SwarmBuilder,
    };
    use std::collections::HashMap;
    use tokio::sync::{mpsc, oneshot};

    type RequestTx = oneshot::Sender<Result<ShardResponse, String>>;

    enum Cmd {
        Dial(Multiaddr, oneshot::Sender<Result<(), String>>),
        Request(PeerId, ShardRequest, RequestTx),
        WaitConnected(PeerId, oneshot::Sender<()>),
    }

    struct Node {
        peer_id: PeerId,
        addr: Multiaddr,
        cmd_tx: mpsc::Sender<Cmd>,
    }

    async fn spawn_node(data: Vec<u8>) -> Node {
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
                request_response::Behaviour::<ShardCodec>::with_codec(
                    ShardCodec,
                    [(ShardProtocol, ProtocolSupport::Full)],
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
                                let response = match request {
                                    ShardRequest::Get { .. } => ShardResponse::Data(data.clone()),
                                    ShardRequest::Have { .. } => ShardResponse::Have(true),
                                    _ => ShardResponse::NotFound,
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

    async fn request(node: &Node, peer: PeerId, req: ShardRequest) -> ShardResponse {
        let (tx, rx) = oneshot::channel();
        node.cmd_tx.send(Cmd::Request(peer, req, tx)).await.unwrap();
        rx.await.unwrap().expect("request")
    }

    pub async fn run(
        hashes: Vec<String>,
        payload_size: usize,
        warmup: usize,
        measured: usize,
    ) -> BenchResult {
        let total_iters = warmup + measured;
        assert!(hashes.len() >= total_iters);

        let data = build_payload(payload_size, 0xC0DE_C0DE);

        let provider = spawn_node(data).await;
        let fetcher = spawn_node(Vec::new()).await;

        dial(&fetcher, provider.addr.clone()).await;
        wait_connected(&fetcher, provider.peer_id).await;

        for hash in &hashes[..warmup] {
            let req = ShardRequest::Get { hash: hash.clone() };
            let res = request(&fetcher, provider.peer_id, req).await;
            match res {
                ShardResponse::Data(b) => assert_eq!(b.len(), payload_size),
                other => panic!("warmup: unexpected: {other:?}"),
            }
        }

        let mut durations = Vec::with_capacity(measured);
        for hash in &hashes[warmup..warmup + measured] {
            let req = ShardRequest::Get { hash: hash.clone() };
            let t0 = Instant::now();
            let res = request(&fetcher, provider.peer_id, req).await;
            let elapsed = t0.elapsed();
            match res {
                ShardResponse::Data(b) => assert_eq!(b.len(), payload_size),
                other => panic!("measured: unexpected: {other:?}"),
            }
            durations.push(elapsed);
        }

        BenchResult::from_durations("libp2p", payload_size, durations)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn compare_shard_perf() {
    // Shard sizes — Reed-Solomon shards are typically 1-4 MiB; bench sweeps
    // small (control-only) through large.
    let payload_sizes: &[usize] = &[1024, 64 * 1024, 1024 * 1024, 4 * 1024 * 1024];
    let warmup = 5;
    let measured = 25;

    let total_iters = warmup + measured;
    let mut all_results: Vec<BenchResult> = Vec::with_capacity(payload_sizes.len() * 2);

    for &size in payload_sizes {
        let hashes = build_hashes(total_iters);
        let iroh_res = iroh_bench::run(hashes.clone(), size, warmup, measured).await;
        all_results.push(iroh_res);

        let libp2p_res = libp2p_bench::run(hashes, size, warmup, measured).await;
        all_results.push(libp2p_res);
    }

    println!();
    println!("## Shard plane perf — iroh vs libp2p (loopback, release)");
    println!();
    println!("| payload | transport | p50 (ms) | p95 (ms) | p99 (ms) | mean (ms) | rps |");
    println!("|---|---|---|---|---|---|---|");
    for r in &all_results {
        println!(
            "| {} | {} | {} | {} | {} | {} | {:.1} |",
            fmt_size(r.payload_size),
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
        "Iters: {warmup} warmup discarded + {measured} measured per (transport, payload). \
        Reed-Solomon shard `Get` round-trip; connections reused; iroh opens fresh bidi streams per request."
    );
    println!();

    let mut by_class: std::collections::BTreeMap<
        usize,
        (Option<&BenchResult>, Option<&BenchResult>),
    > = std::collections::BTreeMap::new();
    for r in &all_results {
        let entry = by_class.entry(r.payload_size).or_insert((None, None));
        match r.transport {
            "iroh" => entry.0 = Some(r),
            "libp2p" => entry.1 = Some(r),
            _ => unreachable!(),
        }
    }

    let mut bump_found = false;
    let mut bump_summary: Vec<String> = Vec::new();
    for (size, (iroh, libp2p)) in &by_class {
        let (Some(i), Some(l)) = (iroh, libp2p) else {
            continue;
        };
        let ratio_p50 = l.p50.as_secs_f64() / i.p50.as_secs_f64().max(f64::EPSILON);
        let ratio_p99 = l.p99.as_secs_f64() / i.p99.as_secs_f64().max(f64::EPSILON);
        let line = format!(
            "  {:>8}: iroh/libp2p p50 ratio = {:.2}x, p99 ratio = {:.2}x  ({})",
            fmt_size(*size),
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
        Expected: iroh p50 < libp2p p50 on at least one of {payload_sizes:?}. \
        See table above for details."
    );

    println!("Tier-3 verdict: iroh delivers a perf bump on at least one isolated size class.");
}
