//! Blob plane latency stress test — 10,000 round-trips per transport.
//!
//! This is the Gate #8 acceptance harness for the iroh cutover. It drives
//! 10,000 fresh blob round-trips through each transport (iroh and libp2p) and
//! reports p50/p95/p99 histograms. The acceptance criterion is:
//!
//!   p99(iroh) ≤ p99(libp2p) × 1.10   (iroh p99 must not exceed libp2p by more than 10%)
//!
//! This is deliberately conservative — the loopback bench (`bench_blob_perf.rs`)
//! shows iroh wins p50 by 4×–290× across size classes, so the p99 guard is
//! achievable even under scheduler jitter.
//!
//! ## Why 10,000 iterations?
//!
//! The 30-iteration `bench_blob_perf.rs` bench surfaces median behaviour.
//! 10,000 iterations drives out p99 tail events: GC pauses, tokio runtime
//! scheduling hiccups, QUIC handshake retransmits, TCP slow-start on the
//! loopback path. A transport with a bad p99 will reveal itself here even if
//! its p50 is excellent.
//!
//! ## Running
//!
//! ```bash
//! just bench-stress
//! # or directly:
//! cargo test --release --features "p2p p2p-iroh" \
//!     --test bench_blob_stress_10k -- --ignored --nocapture
//! ```
//!
//! `--release` is load-bearing — debug overhead (~5–10×) masks transport
//! differences and produces unrepresentative tail numbers.
//!
//! ## Baseline numbers (loopback, release, 2026-05-10)
//!
//! These are the reference numbers from the first gate-opening run.
//! Future runs that deviate by more than 20% at p99 warrant investigation.
//!
//! | transport | payload | p50 (ms) | p95 (ms) | p99 (ms) | throughput (MB/s) |
//! |-----------|---------|----------|----------|----------|-------------------|
//! | (baseline numbers will be filled in after first `just bench-stress` run) |
//!
//! Gate #8 is considered closed when:
//!   1. The harness compiles and runs to completion without assertion failure.
//!   2. The p99 ratio assertion passes (iroh p99 ≤ libp2p p99 × 1.10).
//!   3. Results are recorded in the table above and committed.

#![cfg(all(feature = "p2p", feature = "p2p-iroh"))]

use std::time::{Duration, Instant};
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// Shared types (mirrors bench_blob_perf.rs — kept local to avoid test-binary
// coupling; the two bench files are independently runnable).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct StressResult {
    transport: &'static str,
    payload_size: usize,
    p50: Duration,
    p95: Duration,
    p99: Duration,
    mean: Duration,
    throughput_mbps: f64,
    iterations: usize,
}

impl StressResult {
    fn from_durations(transport: &'static str, payload_size: usize, mut ds: Vec<Duration>) -> Self {
        ds.sort();
        let iters = ds.len();
        let p50 = percentile(&ds, 50.0);
        let p95 = percentile(&ds, 95.0);
        let p99 = percentile(&ds, 99.0);
        let total: Duration = ds.iter().sum();
        let mean = total / iters as u32;
        let throughput_mbps = if total.is_zero() {
            0.0
        } else {
            (payload_size as f64 * iters as f64) / total.as_secs_f64() / 1_000_000.0
        };
        Self {
            transport,
            payload_size,
            p50,
            p95,
            p99,
            mean,
            throughput_mbps,
            iterations: iters,
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

// ---------------------------------------------------------------------------
// Payload generator — same deterministic SplitMix stream as bench_blob_perf.
// ---------------------------------------------------------------------------

fn build_payloads(n: usize, size: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let mut buf = vec![0u8; size];
        let prefix = (i as u64).to_le_bytes();
        let copy_len = prefix.len().min(size);
        buf[..copy_len].copy_from_slice(&prefix[..copy_len]);
        let mut state = i as u64 ^ 0x9E37_79B9_7F4A_7C15;
        for chunk in buf[copy_len..].chunks_mut(8) {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            let bytes = z.to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
        out.push(buf);
    }
    out
}

// ---------------------------------------------------------------------------
// Iroh stress driver.
// ---------------------------------------------------------------------------

mod iroh_stress {
    use super::*;
    use elohim_storage::p2p_iroh::{IrohConfig, IrohNode};
    use iroh_blobs::Hash;

    fn loopback_config(dir: &std::path::Path) -> IrohConfig {
        IrohConfig {
            blobs_dir: dir.join("blobs_iroh"),
            secret_key_path: dir.join("iroh.key"),
            use_n0_relays: false,
            use_n0_discovery: false,
            discovery_resolvers: vec![],
        }
    }

    /// Drive `measured` round-trips through iroh QUIC. Uses a single reused
    /// Connection — models a stable peer link, not cold-start per fetch.
    pub async fn run(payloads: Vec<Vec<u8>>, warmup: usize, measured: usize) -> StressResult {
        let payload_size = payloads.first().map(|p| p.len()).unwrap_or(0);
        let total = warmup + measured;
        assert!(
            payloads.len() >= total,
            "iroh_stress: need {total} payloads, got {}",
            payloads.len()
        );

        let provider_dir = tempdir().expect("provider tempdir");
        let fetcher_dir = tempdir().expect("fetcher tempdir");

        let provider = IrohNode::start(loopback_config(provider_dir.path()))
            .await
            .expect("iroh provider start");
        let fetcher = IrohNode::start(loopback_config(fetcher_dir.path()))
            .await
            .expect("iroh fetcher start");

        let mut hashes: Vec<Hash> = Vec::with_capacity(payloads.len());
        for p in &payloads {
            let h = provider
                .add_bytes(p.clone())
                .await
                .expect("provider add_bytes");
            hashes.push(h);
        }

        let provider_addr = provider.node_addr().await.expect("provider node_addr");
        let conn = fetcher
            .endpoint()
            .connect(provider_addr, iroh_blobs::ALPN)
            .await
            .expect("fetcher connect");

        // Warmup — discarded.
        for hash in &hashes[..warmup] {
            fetcher
                .store()
                .inner()
                .remote()
                .fetch(conn.clone(), *hash)
                .await
                .expect("warmup fetch");
            let _ = fetcher.store().get_bytes(*hash).await.expect("warmup read");
        }

        // Measured.
        let mut durations = Vec::with_capacity(measured);
        for hash in &hashes[warmup..warmup + measured] {
            let t0 = Instant::now();
            fetcher
                .store()
                .inner()
                .remote()
                .fetch(conn.clone(), *hash)
                .await
                .expect("measured fetch");
            let bytes = fetcher
                .store()
                .get_bytes(*hash)
                .await
                .expect("measured read");
            let elapsed = t0.elapsed();
            assert_eq!(bytes.len(), payload_size, "iroh_stress: size mismatch");
            durations.push(elapsed);
        }

        drop(conn);
        provider.shutdown().await.expect("provider shutdown");
        fetcher.shutdown().await.expect("fetcher shutdown");

        StressResult::from_durations("iroh", payload_size, durations)
    }
}

// ---------------------------------------------------------------------------
// Libp2p stress driver (mirrors bench_blob_perf libp2p_bench module).
// ---------------------------------------------------------------------------

mod libp2p_stress {
    use super::*;
    use elohim_storage::p2p::{
        BlobCodec, BlobFetchRequest, BlobFetchResponse, BlobProtocol, BLOB_PROTOCOL_ID,
    };
    use futures::StreamExt;
    use libp2p::{
        identity, noise,
        request_response::{self, ProtocolSupport},
        swarm::SwarmEvent,
        tcp, yamux, Multiaddr, PeerId, SwarmBuilder,
    };
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;
    use tokio::sync::{mpsc, oneshot};

    type FetchTx = oneshot::Sender<Result<Vec<u8>, String>>;

    enum Cmd {
        Dial(Multiaddr, oneshot::Sender<Result<(), String>>),
        Fetch(PeerId, String, FetchTx),
        WaitConnected(PeerId, oneshot::Sender<()>),
    }

    struct Node {
        peer_id: PeerId,
        addr: Multiaddr,
        cmd_tx: mpsc::Sender<Cmd>,
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        format!("sha256-{}", hex::encode(h.finalize()))
    }

    async fn spawn_node(holdings: HashMap<String, Vec<u8>>, max_resp_size: usize) -> Node {
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
                request_response::Behaviour::<BlobCodec>::with_codec(
                    BlobCodec::with_max_response_size(max_resp_size),
                    [(BlobProtocol, ProtocolSupport::Full)],
                    request_response::Config::default()
                        .with_request_timeout(Duration::from_secs(60)),
                )
            })
            .expect("behaviour build")
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(300)))
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

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<Cmd>(256);
        let _proto_id = BLOB_PROTOCOL_ID;

        tokio::spawn(async move {
            let mut pending_fetches: HashMap<request_response::OutboundRequestId, FetchTx> =
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
                            Cmd::Fetch(peer, hash, reply) => {
                                let req_id = swarm
                                    .behaviour_mut()
                                    .send_request(&peer, BlobFetchRequest { hash });
                                pending_fetches.insert(req_id, reply);
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
                                let response = match holdings.get(&request.hash) {
                                    Some(bytes) => BlobFetchResponse::Found(bytes.clone()),
                                    None => BlobFetchResponse::NotFound,
                                };
                                let _ = swarm.behaviour_mut().send_response(channel, response);
                            }
                            SwarmEvent::Behaviour(request_response::Event::Message {
                                message: request_response::Message::Response {
                                    request_id, response,
                                },
                                ..
                            }) => {
                                if let Some(tx) = pending_fetches.remove(&request_id) {
                                    let result = match response {
                                        BlobFetchResponse::Found(bytes) => Ok(bytes),
                                        BlobFetchResponse::NotFound => Err("not found".into()),
                                        BlobFetchResponse::Error(e) => Err(e),
                                    };
                                    let _ = tx.send(result);
                                }
                            }
                            SwarmEvent::Behaviour(request_response::Event::OutboundFailure {
                                request_id, error, ..
                            }) => {
                                if let Some(tx) = pending_fetches.remove(&request_id) {
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
        tokio::time::timeout(Duration::from_secs(5), rx)
            .await
            .expect("wait_connected timeout")
            .unwrap();
    }

    async fn fetch(node: &Node, peer: PeerId, hash: String) -> Vec<u8> {
        let (tx, rx) = oneshot::channel();
        node.cmd_tx.send(Cmd::Fetch(peer, hash, tx)).await.unwrap();
        rx.await.unwrap().expect("fetch")
    }

    pub async fn run(payloads: Vec<Vec<u8>>, warmup: usize, measured: usize) -> StressResult {
        let payload_size = payloads.first().map(|p| p.len()).unwrap_or(0);
        let total = warmup + measured;
        assert!(
            payloads.len() >= total,
            "libp2p_stress: need {total} payloads, got {}",
            payloads.len()
        );

        let mut hashes: Vec<String> = Vec::with_capacity(payloads.len());
        let mut holdings: HashMap<String, Vec<u8>> = HashMap::with_capacity(payloads.len());
        for p in &payloads {
            let h = sha256_hex(p);
            holdings.insert(h.clone(), p.clone());
            hashes.push(h);
        }

        let max_resp = payload_size
            .checked_mul(3)
            .and_then(|v| v.checked_add(4 * 1024))
            .unwrap_or(elohim_storage::p2p::BLOB_HARD_MAX_RESPONSE_SIZE);

        let provider = spawn_node(holdings, max_resp).await;
        let fetcher = spawn_node(HashMap::new(), max_resp).await;

        dial(&fetcher, provider.addr.clone()).await;
        wait_connected(&fetcher, provider.peer_id).await;

        // Warmup.
        for h in &hashes[..warmup] {
            let bytes = fetch(&fetcher, provider.peer_id, h.clone()).await;
            assert_eq!(bytes.len(), payload_size, "warmup size mismatch");
            let recv_hash = sha256_hex(&bytes);
            assert_eq!(&recv_hash, h, "warmup hash mismatch");
        }

        // Measured — 10,000 round-trips.
        let mut durations = Vec::with_capacity(measured);
        for h in &hashes[warmup..warmup + measured] {
            let t0 = Instant::now();
            let bytes = fetch(&fetcher, provider.peer_id, h.clone()).await;
            let recv_hash = sha256_hex(&bytes);
            let elapsed = t0.elapsed();
            assert_eq!(bytes.len(), payload_size, "measured size mismatch");
            assert_eq!(&recv_hash, h, "measured hash mismatch");
            durations.push(elapsed);
        }

        StressResult::from_durations("libp2p", payload_size, durations)
    }
}

// ---------------------------------------------------------------------------
// Stress test — 10,000 iterations per transport, p99 acceptance gate.
// ---------------------------------------------------------------------------

/// 10,000 round-trip stress test per transport. Marked `#[ignore]`; run via
/// `just bench-stress` or `cargo test --release --features "p2p p2p-iroh"
/// --test bench_blob_stress_10k -- --ignored --nocapture`.
///
/// Gate #8 acceptance criterion: p99(iroh) ≤ p99(libp2p) × 1.10
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn stress_blob_10k_round_trips() {
    // Single representative payload size: 64 KiB.
    // Small enough to avoid test timeouts at 10k iters; large enough that
    // throughput differences are visible in the p99 tail.
    let payload_size: usize = 64 * 1024;

    // 10,000 measured + 50 warmup per transport = 10,050 payloads each.
    let warmup = 50;
    let measured = 10_000;
    let total = warmup + measured;

    println!();
    println!("## Blob plane stress — 10,000 round-trips per transport (Gate #8)");
    println!(
        "   Payload: {} | Warmup: {} | Measured: {}",
        fmt_size(payload_size),
        warmup,
        measured
    );
    println!();

    // Build payloads — each transport gets its own unique set so hashes don't
    // collide across runs (iroh and libp2p use different hash schemes).
    let iroh_payloads = build_payloads(total, payload_size);
    // Shift index by `total` so iroh and libp2p blobs have distinct content.
    let libp2p_payloads: Vec<Vec<u8>> = (total..total * 2)
        .map(|i| {
            let mut buf = vec![0u8; payload_size];
            let prefix = (i as u64).to_le_bytes();
            let copy_len = prefix.len().min(payload_size);
            buf[..copy_len].copy_from_slice(&prefix[..copy_len]);
            let mut state = i as u64 ^ 0x9E37_79B9_7F4A_7C15;
            for chunk in buf[copy_len..].chunks_mut(8) {
                state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
                let mut z = state;
                z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
                z ^= z >> 31;
                let bytes = z.to_le_bytes();
                chunk.copy_from_slice(&bytes[..chunk.len()]);
            }
            buf
        })
        .collect();

    // Run iroh first (QUIC), then libp2p (TCP+yamux).
    println!("Running iroh stress ({measured} iters)...");
    let iroh_res = iroh_stress::run(iroh_payloads, warmup, measured).await;

    println!("Running libp2p stress ({measured} iters)...");
    let libp2p_res = libp2p_stress::run(libp2p_payloads, warmup, measured).await;

    // Print results table.
    println!();
    println!(
        "| transport | iters | p50 (ms) | p95 (ms) | p99 (ms) | mean (ms) | throughput (MB/s) |"
    );
    println!("|---|---|---|---|---|---|---|");
    for r in [&iroh_res, &libp2p_res] {
        println!(
            "| {} | {} | {} | {} | {} | {} | {:.2} |",
            r.transport,
            r.iterations,
            fmt_ms(r.p50),
            fmt_ms(r.p95),
            fmt_ms(r.p99),
            fmt_ms(r.mean),
            r.throughput_mbps,
        );
    }
    println!();

    // p99 comparison.
    let iroh_p99_ms = iroh_res.p99.as_secs_f64() * 1000.0;
    let libp2p_p99_ms = libp2p_res.p99.as_secs_f64() * 1000.0;
    let ratio = iroh_p99_ms / libp2p_p99_ms.max(f64::EPSILON);

    println!(
        "p99 ratio: iroh {:.3} ms / libp2p {:.3} ms = {:.3}x",
        iroh_p99_ms, libp2p_p99_ms, ratio
    );

    // Gate #8 acceptance: iroh p99 must not exceed libp2p p99 by more than 10%.
    assert!(
        ratio <= 1.10,
        "Gate #8 FAILED: iroh p99 ({:.3} ms) exceeds libp2p p99 ({:.3} ms) by {:.1}% (threshold: 10%). \
        See bench results above. iroh is NOT ready for cutover on the blob plane until p99 is within budget.",
        iroh_p99_ms,
        libp2p_p99_ms,
        (ratio - 1.0) * 100.0,
    );

    println!(
        "Gate #8 PASSED: iroh p99 within {:.1}% of libp2p p99 (threshold: 10%).",
        (ratio - 1.0).abs() * 100.0
    );
}
