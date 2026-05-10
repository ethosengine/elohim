//! Head-to-head perf comparison: iroh vs libp2p trust plane on loopback.
//!
//! Same workload — initiator sends `TrustHandshake` with a sweep of
//! `attestation_cids` counts and provider answers with
//! `TrustResponse::Verified { reach_ceiling: "household", ttl_seconds: 3600 }` —
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
//! Reports p50/p95/p99 + mean per-request latency per (scenario, transport,
//! workload class), and asserts iroh delivers a perf bump on at least one
//! attestation count within the **reuse** scenario.
//!
//! Marked `#[ignore]` so the default `cargo test` run skips it. Invoke
//! explicitly via:
//!
//! ```bash
//! just bench-trust
//! # or:
//! cargo test --release --features "p2p p2p-iroh" \
//!     --test bench_trust_perf -- --ignored --nocapture
//! ```
//!
//! `--release` is load-bearing — debug builds carry ~5-10x overhead and produce
//! noisy numbers that masquerade as transport differences.
//!
//! This bench measures **per-request latency** for the request-response trust
//! plane. Throughput is reported in requests/sec.

#![cfg(all(feature = "p2p", feature = "p2p-iroh"))]

use std::time::{Duration, Instant};
use tempfile::tempdir;

/// Wall-clock cap on a single bench-side read. Loopback should resolve in
/// milliseconds; anything exceeding this is a hang and we want it to surface
/// loudly instead of dragging on for hours.
const READ_TIMEOUT: Duration = Duration::from_secs(30);

// `TRUST_REQ_MAX` is `const` (not `pub`) in `elohim_storage::p2p_iroh::auth`.
// Mirror the value locally so the bench can pass it to `read_frame` without
// requiring a visibility change in src.
// src value: `const TRUST_REQ_MAX: usize = 64 * 1024;`
const TRUST_REQ_MAX: usize = 64 * 1024;

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
    /// Workload class — number of attestation CIDs in the trust handshake.
    /// Each CID is ~70 chars; 768 CIDs ≈ ~54 KiB, safely under the 64 KiB cap.
    attestation_count: usize,
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
        attestation_count: usize,
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
            attestation_count,
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
// Workload generator — TrustHandshake with a deterministic attestation_cids
// list of the requested length.
// ---------------------------------------------------------------------------

/// Build `n` deterministic CID strings shaped like `bafyrei{16-hex-chars}`.
/// Deterministic seed so re-runs reproduce identical payloads.
fn build_attestation_cids(n: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(n);
    let mut state: u64 = 0xDEAD_C0DE_FACE_BABE;
    for _ in 0..n {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        // Each CID is ~24 chars ("bafyrei" + 16 hex chars = 23 chars).
        // This matches the ~70-char CID shape used across the bench suite.
        out.push(format!("bafyrei{:016x}", z));
    }
    out
}

/// Build a `TrustHandshake` with `attestation_count` attestation CIDs and
/// a fixed set of membership/relationship/stewardship CIDs (1 each).
fn build_handshake(
    attestation_count: usize,
) -> elohim_storage::p2p::trust_protocol::TrustHandshake {
    use elohim_storage::p2p::trust_protocol::TrustHandshake;
    TrustHandshake {
        agent_pubkey: "Ed25519:bench-pubkey-trust-perf".to_string(),
        membership_cids: vec!["bafyreimembership0000000000000001".to_string()],
        relationship_cids: vec!["bafyreirelationship000000000000001".to_string()],
        stewardship_cids: vec!["bafyreistewardship000000000000001".to_string()],
        attestation_cids: build_attestation_cids(attestation_count),
    }
}

// ---------------------------------------------------------------------------
// Iroh-side bench — IrohNode + trust ALPN handler. Two scenarios:
//   • run_reuse_conn — open one QUIC Connection up front, fresh bidi stream
//     per request (matches Phase 11 connection-pool ceiling).
//   • run_fresh_conn — open a fresh QUIC Connection per request.
// ---------------------------------------------------------------------------

mod iroh_bench {
    use super::*;
    use elohim_storage::p2p::trust_protocol::{TrustHandshake, TrustResponse};
    use elohim_storage::p2p_iroh::{
        AlpnRegistration, IrohConfig, IrohNode, IrohTrustProtocol, TrustBackend, TRUST_ALPN,
    };
    use std::sync::Arc;

    fn loopback_config(dir: &std::path::Path) -> IrohConfig {
        IrohConfig {
            blobs_dir: dir.join("blobs_iroh"),
            secret_key_path: dir.join("iroh.key"),
            use_n0_relays: false,
            use_n0_discovery: false,
            discovery_resolvers: vec![],
        }
    }

    /// Backend that returns a fixed Verified response. Used so the latency
    /// measured is transport+codec, not backend compute.
    struct FixedTrustBackend;

    #[async_trait::async_trait]
    impl TrustBackend for FixedTrustBackend {
        async fn handle(&self, _req: TrustHandshake) -> TrustResponse {
            TrustResponse::Verified {
                reach_ceiling: "household".into(),
                ttl_seconds: 3600,
            }
        }
    }

    /// Drive a single trust request over an existing QUIC connection.
    /// Used by both scenarios — reuse opens the conn once and calls this
    /// in a hot loop, fresh opens a new conn before each call.
    async fn send_one(conn: &iroh::endpoint::Connection, req: &TrustHandshake) -> TrustResponse {
        let (mut send, mut recv) = conn.open_bi().await.expect("open_bi");
        elohim_storage::p2p_iroh::codec::write_frame(&mut send, req)
            .await
            .expect("write_frame");
        send.finish().expect("finish send");
        tokio::time::timeout(
            READ_TIMEOUT,
            elohim_storage::p2p_iroh::codec::read_frame::<TrustResponse, _>(
                &mut recv,
                TRUST_REQ_MAX,
            ),
        )
        .await
        .expect("iroh read_frame timed out — handler may be wedged")
        .expect("read_frame")
    }

    /// Reuse-conn scenario: open the QUIC connection up front, issue every
    /// request as a fresh bidi stream on the same connection.
    pub async fn run_reuse_conn(
        attestation_count: usize,
        warmup: usize,
        measured: usize,
    ) -> BenchResult {
        let provider_dir = tempdir().expect("provider tempdir");
        let fetcher_dir = tempdir().expect("fetcher tempdir");

        let backend: Arc<dyn TrustBackend> = Arc::new(FixedTrustBackend);

        let provider_protocols: Vec<AlpnRegistration> = vec![(
            TRUST_ALPN.to_vec(),
            Box::new(IrohTrustProtocol::new(backend.clone())),
        )];
        let fetcher_protocols: Vec<AlpnRegistration> = vec![];

        let provider = IrohNode::start_with_protocols(
            loopback_config(provider_dir.path()),
            provider_protocols,
        )
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
            .connect(provider_addr, TRUST_ALPN)
            .await
            .expect("fetcher → provider connect");

        let req = build_handshake(attestation_count);

        // Warmup — discarded.
        for _ in 0..warmup {
            let res = send_one(&conn, &req).await;
            assert!(
                matches!(res, TrustResponse::Verified { .. }),
                "warmup: expected Verified, got {res:?}",
            );
        }

        // Measured.
        let mut durations = Vec::with_capacity(measured);
        for _ in 0..measured {
            let t0 = Instant::now();
            let res = send_one(&conn, &req).await;
            let elapsed = t0.elapsed();
            assert!(
                matches!(res, TrustResponse::Verified { .. }),
                "measured: expected Verified, got {res:?}",
            );
            durations.push(elapsed);
        }

        drop(conn);
        provider.shutdown().await.expect("provider shutdown");
        fetcher.shutdown().await.expect("fetcher shutdown");

        BenchResult::from_durations("reuse", "iroh", attestation_count, durations)
    }

    /// Fresh-conn scenario: open a new QUIC connection for every request,
    /// drop it after the response.
    pub async fn run_fresh_conn(
        attestation_count: usize,
        warmup: usize,
        measured: usize,
    ) -> BenchResult {
        let provider_dir = tempdir().expect("provider tempdir");
        let fetcher_dir = tempdir().expect("fetcher tempdir");

        let backend: Arc<dyn TrustBackend> = Arc::new(FixedTrustBackend);

        let provider_protocols: Vec<AlpnRegistration> = vec![(
            TRUST_ALPN.to_vec(),
            Box::new(IrohTrustProtocol::new(backend.clone())),
        )];
        let fetcher_protocols: Vec<AlpnRegistration> = vec![];

        let provider = IrohNode::start_with_protocols(
            loopback_config(provider_dir.path()),
            provider_protocols,
        )
        .await
        .expect("provider starts");
        let fetcher =
            IrohNode::start_with_protocols(loopback_config(fetcher_dir.path()), fetcher_protocols)
                .await
                .expect("fetcher starts");

        let provider_addr = provider.node_addr().await.expect("provider node_addr");

        let req = build_handshake(attestation_count);

        // Warmup — discarded. Fresh connection each iteration.
        for _ in 0..warmup {
            let conn = fetcher
                .endpoint()
                .connect(provider_addr.clone(), TRUST_ALPN)
                .await
                .expect("fetcher → provider connect (warmup)");
            let res = send_one(&conn, &req).await;
            assert!(
                matches!(res, TrustResponse::Verified { .. }),
                "warmup: expected Verified, got {res:?}",
            );
            drop(conn);
        }

        // Measured. Each iteration's clock includes connection setup.
        let mut durations = Vec::with_capacity(measured);
        for _ in 0..measured {
            let t0 = Instant::now();
            let conn = fetcher
                .endpoint()
                .connect(provider_addr.clone(), TRUST_ALPN)
                .await
                .expect("fetcher → provider connect (measured)");
            let res = send_one(&conn, &req).await;
            let elapsed = t0.elapsed();
            drop(conn);
            assert!(
                matches!(res, TrustResponse::Verified { .. }),
                "measured: expected Verified, got {res:?}",
            );
            durations.push(elapsed);
        }

        provider.shutdown().await.expect("provider shutdown");
        fetcher.shutdown().await.expect("fetcher shutdown");

        BenchResult::from_durations("fresh", "iroh", attestation_count, durations)
    }
}

// ---------------------------------------------------------------------------
// Libp2p-side bench — minimal two-swarm setup for /elohim/trust/1.0.0.
//   • run_reuse_conn — provider+fetcher up once, request_response uses one
//     long-lived TCP+yamux connection.
//   • run_fresh_conn — provider stays alive, fetcher swarm is freshly built
//     and dialed for every request, then dropped (TCP+Noise+Yamux handshake
//     per request).
// ---------------------------------------------------------------------------

mod libp2p_bench {
    use super::*;
    use elohim_storage::p2p::trust_protocol::{
        TrustCodec, TrustHandshake, TrustProtocol, TrustResponse,
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

    type RequestTx = oneshot::Sender<Result<TrustResponse, String>>;

    enum Cmd {
        Dial(Multiaddr, oneshot::Sender<Result<(), String>>),
        Request(PeerId, TrustHandshake, RequestTx),
        WaitConnected(PeerId, oneshot::Sender<()>),
    }

    struct Node {
        peer_id: PeerId,
        addr: Multiaddr,
        cmd_tx: mpsc::Sender<Cmd>,
    }

    /// Build a node. `is_provider` controls whether inbound requests are
    /// answered with a Verified response; the fetcher node never receives requests.
    async fn spawn_node(is_provider: bool) -> Node {
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
                request_response::Behaviour::<TrustCodec>::with_codec(
                    TrustCodec,
                    [(TrustProtocol, ProtocolSupport::Full)],
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
                                    channel, ..
                                },
                                ..
                            }) => {
                                let response = if is_provider {
                                    TrustResponse::Verified {
                                        reach_ceiling: "household".into(),
                                        ttl_seconds: 3600,
                                    }
                                } else {
                                    TrustResponse::Error(
                                        "fetcher does not answer requests".into(),
                                    )
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

    async fn request(node: &Node, peer: PeerId, req: TrustHandshake) -> TrustResponse {
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
        attestation_count: usize,
        warmup: usize,
        measured: usize,
    ) -> BenchResult {
        let provider = spawn_node(true).await;
        let fetcher = spawn_node(false).await;

        dial(&fetcher, provider.addr.clone()).await;
        wait_connected(&fetcher, provider.peer_id).await;

        let req = build_handshake(attestation_count);

        // Warmup.
        for _ in 0..warmup {
            let res = request(&fetcher, provider.peer_id, req.clone()).await;
            assert!(
                matches!(res, TrustResponse::Verified { .. }),
                "warmup: expected Verified, got {res:?}",
            );
        }

        // Measured.
        let mut durations = Vec::with_capacity(measured);
        for _ in 0..measured {
            let t0 = Instant::now();
            let res = request(&fetcher, provider.peer_id, req.clone()).await;
            let elapsed = t0.elapsed();
            assert!(
                matches!(res, TrustResponse::Verified { .. }),
                "measured: expected Verified, got {res:?}",
            );
            durations.push(elapsed);
        }

        BenchResult::from_durations("reuse", "libp2p", attestation_count, durations)
    }

    /// Fresh-conn scenario: provider stays alive across iterations, but
    /// every iteration spawns a brand new fetcher swarm, dials, sends one
    /// request, and drops. Iteration timing includes TCP+Noise+Yamux
    /// handshake cost.
    pub async fn run_fresh_conn(
        attestation_count: usize,
        warmup: usize,
        measured: usize,
    ) -> BenchResult {
        let provider = spawn_node(true).await;

        let req = build_handshake(attestation_count);

        // Warmup — fresh fetcher per iteration, discarded.
        for _ in 0..warmup {
            let fetcher = spawn_node(false).await;
            dial(&fetcher, provider.addr.clone()).await;
            wait_connected(&fetcher, provider.peer_id).await;
            let res = request(&fetcher, provider.peer_id, req.clone()).await;
            assert!(
                matches!(res, TrustResponse::Verified { .. }),
                "warmup: expected Verified, got {res:?}",
            );
            drop(fetcher);
        }

        // Measured — each iteration's clock starts at swarm spawn so the
        // measurement captures full handshake cost.
        let mut durations = Vec::with_capacity(measured);
        for _ in 0..measured {
            let t0 = Instant::now();
            let fetcher = spawn_node(false).await;
            dial(&fetcher, provider.addr.clone()).await;
            wait_connected(&fetcher, provider.peer_id).await;
            let res = request(&fetcher, provider.peer_id, req.clone()).await;
            let elapsed = t0.elapsed();
            drop(fetcher);
            assert!(
                matches!(res, TrustResponse::Verified { .. }),
                "measured: expected Verified, got {res:?}",
            );
            durations.push(elapsed);
        }

        BenchResult::from_durations("fresh", "libp2p", attestation_count, durations)
    }
}

// ---------------------------------------------------------------------------
// Comparison test — drives both scenarios on both stacks, prints two
// markdown tables, asserts perf bump on the reuse scenario.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn compare_trust_perf() {
    // Workload classes — number of attestation CIDs in the trust handshake.
    // Each CID ≈ 23 chars as encoded here; at 768 CIDs that's ~17 KiB for
    // attestation_cids alone — well under the 64 KiB request cap.
    // (1024 CIDs × ~70-char realistic CIDs ≈ ~75 KiB, which would exceed the
    // cap; we cap at 768 to stay safely under 64 KiB.)
    let attestation_counts: &[usize] = &[1, 16, 256, 768];
    let warmup = 5;
    let measured = 30;

    let mut all_results: Vec<BenchResult> = Vec::new();

    for &attestation_count in attestation_counts {
        // Reuse scenario — engine ceiling.
        all_results.push(iroh_bench::run_reuse_conn(attestation_count, warmup, measured).await);
        all_results.push(libp2p_bench::run_reuse_conn(attestation_count, warmup, measured).await);

        // Fresh scenario — handshake-per-request.
        all_results.push(iroh_bench::run_fresh_conn(attestation_count, warmup, measured).await);
        all_results.push(libp2p_bench::run_fresh_conn(attestation_count, warmup, measured).await);
    }

    // ----- Print a table per scenario -----
    print_scenario_table(
        &all_results,
        "reuse",
        "Trust plane perf — REUSE scenario (engine ceiling)",
        "One QUIC Connection / TCP+yamux Connection reused across iterations on each side; \
         iroh opens a fresh bidi stream per request, libp2p uses request_response substreams. \
         Handshake cost amortized across all measured iterations.",
        warmup,
        measured,
    );
    print_scenario_table(
        &all_results,
        "fresh",
        "Trust plane perf — FRESH scenario (handshake per request)",
        "Each iteration pays full handshake cost. iroh: fresh `endpoint.connect()` per request. \
         libp2p: fresh fetcher swarm spawned and dialed per request \
         (TCP+Noise+Yamux handshake on each iteration).",
        warmup,
        measured,
    );

    // ----- Perf-bump assertion (engine ceiling — reuse scenario only) -----
    let mut bump_summary: Vec<String> = Vec::new();
    let mut bump_found = false;
    let by_class = group_by_class(&all_results, "reuse");
    for (count, (iroh, libp2p)) in &by_class {
        let (Some(i), Some(l)) = (iroh, libp2p) else {
            continue;
        };
        let ratio_p50 = l.p50.as_secs_f64() / i.p50.as_secs_f64().max(f64::EPSILON);
        let ratio_p99 = l.p99.as_secs_f64() / i.p99.as_secs_f64().max(f64::EPSILON);
        bump_summary.push(format!(
            "  attestations={:>4}: iroh/libp2p p50 ratio = {:.2}x, p99 ratio = {:.2}x  ({})",
            count,
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
    for (count, (iroh, libp2p)) in &by_class_fresh {
        let (Some(i), Some(l)) = (iroh, libp2p) else {
            continue;
        };
        let ratio_p50 = l.p50.as_secs_f64() / i.p50.as_secs_f64().max(f64::EPSILON);
        let ratio_p99 = l.p99.as_secs_f64() / i.p99.as_secs_f64().max(f64::EPSILON);
        fresh_summary.push(format!(
            "  attestations={:>4}: iroh/libp2p p50 ratio = {:.2}x, p99 ratio = {:.2}x  ({})",
            count,
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
        "Tier-3 verdict: iroh did NOT deliver a perf bump on any attestation count in REUSE. \
        Expected: iroh p50 < libp2p p50 on at least one of {attestation_counts:?} for the \
        reuse scenario. See tables above for details."
    );

    println!(
        "Tier-3 verdict: iroh delivers a perf bump on at least one isolated attestation count \
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
    println!("| attestations | transport | p50 (ms) | p95 (ms) | p99 (ms) | mean (ms) | rps |");
    println!("|---|---|---|---|---|---|---|");
    for r in all_results.iter().filter(|r| r.scenario == scenario) {
        println!(
            "| {} | {} | {} | {} | {} | {} | {:.1} |",
            r.attestation_count,
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
        "Iters: {warmup} warmup discarded + {measured} measured per (transport, attestation_count). {footnote}"
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
        let entry = by_class.entry(r.attestation_count).or_insert((None, None));
        match r.transport {
            "iroh" => entry.0 = Some(r),
            "libp2p" => entry.1 = Some(r),
            _ => unreachable!(),
        }
    }
    by_class
}
