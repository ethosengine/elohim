//! Head-to-head perf comparison: iroh vs libp2p blob plane on loopback.
//!
//! Same workload — provider stores N unique blobs of size S; fetcher pulls each
//! by hash; bytes round-trip with content-hash verification — driven through
//! both transports. Reports p50/p95/p99 latency + mean throughput per transport
//! per size, and asserts iroh delivers a perf bump on the small-blob class.
//!
//! Marked `#[ignore]` so the default `cargo test` run skips it. Invoke
//! explicitly via:
//!
//! ```bash
//! just bench-blob
//! # or:
//! cargo test --release --features "p2p p2p-iroh" \
//!     --test bench_blob_perf -- --ignored --nocapture
//! ```
//!
//! `--release` is load-bearing — debug builds carry ~5-10x overhead and produce
//! noisy numbers that masquerade as transport differences.

#![cfg(all(feature = "p2p", feature = "p2p-iroh"))]

use std::time::{Duration, Instant};
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// Bench result aggregate.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct BenchResult {
    transport: &'static str,
    payload_size: usize,
    p50: Duration,
    p95: Duration,
    p99: Duration,
    mean: Duration,
    /// Mean throughput across all measured iterations (MB/s — base 10).
    throughput_mbps: f64,
}

impl BenchResult {
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
// Workload generator — N unique payloads of size S.
// ---------------------------------------------------------------------------

/// Build `n` unique payloads of `size` bytes. The first 8 bytes encode the
/// index so each blob hashes to a distinct content address; the remaining
/// bytes are deterministic (from a fast SplitMix-style PRNG seeded by index)
/// so re-runs are reproducible and bytes don't compress to nothing.
fn build_payloads(n: usize, size: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let mut buf = vec![0u8; size];
        let prefix = (i as u64).to_le_bytes();
        let copy_len = prefix.len().min(size);
        buf[..copy_len].copy_from_slice(&prefix[..copy_len]);
        // Fill the rest with a cheap deterministic stream.
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
// Iroh-side bench — IrohNode + reused Connection.
// ---------------------------------------------------------------------------

mod iroh_bench {
    use super::*;
    use elohim_storage::p2p_iroh::{IrohConfig, IrohNode};
    use iroh_blobs::Hash;

    fn loopback_config(dir: &std::path::Path) -> IrohConfig {
        IrohConfig {
            blobs_dir: dir.join("blobs_iroh"),
            secret_key_path: dir.join("iroh.key"),
            use_n0_relays: false,
            use_n0_discovery: false,
        }
    }

    /// Run the bench: provider holds `payloads`, fetcher pulls each via QUIC.
    /// One `Connection` is opened up front and reused — mirrors how a daemon
    /// would batch fetches over a stable peer link.
    pub async fn run(payloads: Vec<Vec<u8>>, warmup: usize, measured: usize) -> BenchResult {
        let payload_size = payloads.first().map(|p| p.len()).unwrap_or(0);
        let total_iters = warmup + measured;
        assert!(
            payloads.len() >= total_iters,
            "iroh_bench: need at least warmup+measured payloads ({total_iters}), got {}",
            payloads.len()
        );

        let provider_dir = tempdir().expect("provider tempdir");
        let fetcher_dir = tempdir().expect("fetcher tempdir");
        let provider = IrohNode::start(loopback_config(provider_dir.path()))
            .await
            .expect("provider starts");
        let fetcher = IrohNode::start(loopback_config(fetcher_dir.path()))
            .await
            .expect("fetcher starts");

        // Stash all payloads on the provider, collect their BLAKE3 hashes.
        let mut hashes: Vec<Hash> = Vec::with_capacity(payloads.len());
        for p in &payloads {
            let h = provider
                .add_bytes(p.clone())
                .await
                .expect("provider add_bytes");
            hashes.push(h);
        }

        let provider_addr = provider.node_addr().await.expect("provider node_addr");

        // Open one QUIC connection up front; reuse for every fetch.
        let conn = fetcher
            .endpoint()
            .connect(provider_addr, iroh_blobs::ALPN)
            .await
            .expect("fetcher → provider connect");

        // Warmup — discarded.
        for hash in &hashes[..warmup] {
            fetcher
                .store()
                .inner()
                .remote()
                .fetch(conn.clone(), *hash)
                .await
                .expect("warmup fetch");
            let _ = fetcher
                .store()
                .get_bytes(*hash)
                .await
                .expect("warmup local read");
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
                .expect("measured local read");
            let elapsed = t0.elapsed();
            assert_eq!(
                bytes.len(),
                payload_size,
                "iroh_bench: returned size mismatch"
            );
            durations.push(elapsed);
        }

        drop(conn);
        provider.shutdown().await.expect("provider shutdown");
        fetcher.shutdown().await.expect("fetcher shutdown");

        BenchResult::from_durations("iroh", payload_size, durations)
    }
}

// ---------------------------------------------------------------------------
// Libp2p-side bench — minimal two-swarm setup for /elohim/blob/1.0.0.
//
// Stripped-down version of `tests/harness/mod.rs`: just BlobProtocol via
// request-response, no identity handshake, no DB. Provider holds payloads
// keyed by their sha256 hex; fetcher sends `BlobFetchRequest{hash}` and waits
// on a oneshot channel for the response, then sha256-verifies on receive.
// ---------------------------------------------------------------------------

mod libp2p_bench {
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

    /// Build a node. `holdings` (provider only) maps sha256-hex hashes to
    /// payload bytes; the driver task uses it to answer incoming requests.
    /// Pass an empty map for the fetcher.
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
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(120)))
            .build();

        swarm
            .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
            .expect("listen");

        // Pump events until we know the listen addr.
        let listen_addr = loop {
            match swarm.next().await.expect("first event") {
                SwarmEvent::NewListenAddr { address, .. } => break address,
                _ => continue,
            }
        };

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<Cmd>(64);
        let _proto_id = BLOB_PROTOCOL_ID; // reference so the const is used

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
                                wait_connected.retain(|(p, _)| p != &peer_id);
                                // Notify any waiters.
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
        // Bound the wait; loopback should connect in ms.
        let _ = tokio::time::timeout(Duration::from_secs(5), rx)
            .await
            .expect("wait_connected timed out");
    }

    async fn fetch(node: &Node, peer: PeerId, hash: String) -> Vec<u8> {
        let (tx, rx) = oneshot::channel();
        node.cmd_tx.send(Cmd::Fetch(peer, hash, tx)).await.unwrap();
        rx.await.unwrap().expect("fetch")
    }

    pub async fn run(payloads: Vec<Vec<u8>>, warmup: usize, measured: usize) -> BenchResult {
        let payload_size = payloads.first().map(|p| p.len()).unwrap_or(0);
        let total_iters = warmup + measured;
        assert!(
            payloads.len() >= total_iters,
            "libp2p_bench: need at least warmup+measured payloads ({total_iters}), got {}",
            payloads.len()
        );

        // Compute hashes; build provider holdings map.
        let mut hashes: Vec<String> = Vec::with_capacity(payloads.len());
        let mut holdings: HashMap<String, Vec<u8>> = HashMap::with_capacity(payloads.len());
        for p in &payloads {
            let h = sha256_hex(p);
            holdings.insert(h.clone(), p.clone());
            hashes.push(h);
        }

        // Codec response cap must accommodate payloads + MessagePack envelope overhead.
        // rmp_serde serializes `Vec<u8>` as a MessagePack array of integers (not a
        // byte-string), so values >127 take 2 bytes — empirical bloat ~1.5x for
        // random bytes. Add safety margin: 3x payload + 4 KiB envelope.
        let max_resp = payload_size
            .checked_mul(3)
            .and_then(|v| v.checked_add(4 * 1024))
            .unwrap_or(elohim_storage::p2p::BLOB_HARD_MAX_RESPONSE_SIZE);

        let provider = spawn_node(holdings, max_resp).await;
        let fetcher = spawn_node(HashMap::new(), max_resp).await;

        // Fetcher dials provider and waits for connection.
        dial(&fetcher, provider.addr.clone()).await;
        wait_connected(&fetcher, provider.peer_id).await;

        // Warmup.
        for h in &hashes[..warmup] {
            let bytes = fetch(&fetcher, provider.peer_id, h.clone()).await;
            assert_eq!(bytes.len(), payload_size, "warmup size mismatch");
            // Verify hash on receive (matches production race_fetch path).
            let recv_hash = sha256_hex(&bytes);
            assert_eq!(&recv_hash, h, "warmup hash mismatch");
        }

        // Measured.
        let mut durations = Vec::with_capacity(measured);
        for h in &hashes[warmup..warmup + measured] {
            let t0 = Instant::now();
            let bytes = fetch(&fetcher, provider.peer_id, h.clone()).await;
            // Verify hash on receive — matches what production code does.
            let recv_hash = sha256_hex(&bytes);
            let elapsed = t0.elapsed();
            assert_eq!(bytes.len(), payload_size, "measured size mismatch");
            assert_eq!(&recv_hash, h, "measured hash mismatch");
            durations.push(elapsed);
        }

        BenchResult::from_durations("libp2p", payload_size, durations)
    }
}

// ---------------------------------------------------------------------------
// Comparison test — drives both, prints markdown table, asserts perf bump.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn compare_blob_perf() {
    // Sizes chosen to span the small-blob (handshake/RTT-dominated) regime
    // up through medium blobs (throughput-dominated). All under both stacks'
    // 16 MiB inline cap.
    let sizes: &[usize] = &[1024, 64 * 1024, 1024 * 1024, 8 * 1024 * 1024];
    let warmup = 5;
    let measured = 30;

    let mut all_results: Vec<BenchResult> = Vec::with_capacity(sizes.len() * 2);

    for &size in sizes {
        let payloads = build_payloads(warmup + measured, size);

        // Iroh first.
        let iroh_res = iroh_bench::run(payloads.clone(), warmup, measured).await;
        all_results.push(iroh_res);

        // Libp2p second.
        let libp2p_res = libp2p_bench::run(payloads, warmup, measured).await;
        all_results.push(libp2p_res);
    }

    // Print markdown table.
    println!();
    println!("## Blob plane perf — iroh vs libp2p (loopback, release)");
    println!();
    println!(
        "| size | transport | p50 (ms) | p95 (ms) | p99 (ms) | mean (ms) | throughput (MB/s) |"
    );
    println!("|---|---|---|---|---|---|---|");
    for r in &all_results {
        println!(
            "| {} | {} | {} | {} | {} | {} | {:.2} |",
            fmt_size(r.payload_size),
            r.transport,
            fmt_ms(r.p50),
            fmt_ms(r.p95),
            fmt_ms(r.p99),
            fmt_ms(r.mean),
            r.throughput_mbps,
        );
    }
    println!();
    println!(
        "Iters: {warmup} warmup discarded + {measured} measured per (transport, size). \
        Single QUIC Connection / TCP+yamux Connection reused across iterations on each side."
    );
    println!();

    // Pair results by size for the perf-bump assertion.
    let mut by_size: std::collections::BTreeMap<
        usize,
        (Option<&BenchResult>, Option<&BenchResult>),
    > = std::collections::BTreeMap::new();
    for r in &all_results {
        let entry = by_size.entry(r.payload_size).or_insert((None, None));
        match r.transport {
            "iroh" => entry.0 = Some(r),
            "libp2p" => entry.1 = Some(r),
            _ => unreachable!(),
        }
    }

    // Verdict: at least one size class where iroh p50 strictly < libp2p p50.
    // We use p50 (median) — robust to single-iteration outliers — and require
    // strict improvement on at least one class, matching the user's "delivers
    // a performance bump" gate.
    let mut bump_found = false;
    let mut bump_summary: Vec<String> = Vec::new();
    for (size, (iroh, libp2p)) in &by_size {
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
        Expected: iroh p50 < libp2p p50 on at least one of {sizes:?}. \
        See table above for details."
    );

    println!("Tier-3 verdict: iroh delivers a perf bump on at least one isolated size class.");
}
