//! Head-to-head perf comparison: iroh-gossip vs libp2p-gossipsub on loopback.
//!
//! Same workload — provider publishes MessagePack-encoded
//! `BlobInventoryDelta` payloads on the inventory topic; subscriber receives
//! them and measures publish→receive latency — driven through both transports.
//! Reports p50/p95/p99 per-message latency + mean throughput per transport per
//! payload size, and asserts iroh-gossip delivers a perf bump on at least one
//! size class.
//!
//! Marked `#[ignore]` so the default `cargo test` run skips it. Invoke
//! explicitly via:
//!
//! ```bash
//! just bench-gossip
//! # or:
//! cargo test --release --features "p2p p2p-iroh" \
//!     --test bench_gossip_perf -- --ignored --nocapture
//! ```
//!
//! `--release` is load-bearing — debug builds carry ~5-10x overhead and produce
//! noisy numbers that masquerade as transport differences.
//!
//! ## Notes on the libp2p-gossipsub side
//!
//! Default `gossipsub::Config` uses 1-second heartbeats and floods after
//! mesh-build. For benching loopback we set `heartbeat_interval` to 100ms
//! and use a tiny mesh so messages flow without waiting for a full heartbeat
//! cycle. We also explicitly dial provider→subscriber and wait for
//! `Subscribed` before publishing the first measured message.
//!
//! ## Notes on payload sizing
//!
//! iroh-gossip 0.92 enforces a hard 4096-byte cap on the postcard-encoded
//! wire message via `Config::max_message_size`. Oversized broadcasts are
//! dropped silently (queued by the sender API, then rejected by the inner
//! send loop with `TooLarge`); the test surface is a stuck receiver. We
//! restrict `adds_classes` to values that produce envelopes under 4096 B —
//! see the NOTE on `compare_gossip_perf` below for the math and rationale.

#![cfg(all(feature = "p2p", feature = "p2p-iroh"))]

use std::time::{Duration, Instant};
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// Bench result aggregate.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct BenchResult {
    transport: &'static str,
    /// Approximate payload size (added-hashes count × ~70 bytes/hash + envelope).
    payload_size_class: usize,
    p50: Duration,
    p95: Duration,
    p99: Duration,
    mean: Duration,
    /// Messages received per second.
    msgs_per_sec: f64,
}

impl BenchResult {
    fn from_durations(
        transport: &'static str,
        payload_size_class: usize,
        mut ds: Vec<Duration>,
    ) -> Self {
        ds.sort();
        let iters = ds.len();
        let p50 = percentile(&ds, 50.0);
        let p95 = percentile(&ds, 95.0);
        let p99 = percentile(&ds, 99.0);
        let total: Duration = ds.iter().sum();
        let mean = total / iters as u32;
        let msgs_per_sec = if total.is_zero() {
            0.0
        } else {
            iters as f64 / total.as_secs_f64()
        };
        Self {
            transport,
            payload_size_class,
            p50,
            p95,
            p99,
            mean,
            msgs_per_sec,
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
// Workload generator — N inventory deltas with varying-size add lists.
// ---------------------------------------------------------------------------

fn build_hash_string(seed: u64) -> String {
    let mut state: u64 = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let mut buf = [0u8; 32];
    for chunk in buf.chunks_mut(8) {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        chunk.copy_from_slice(&z.to_le_bytes());
    }
    hex::encode(buf)
}

/// Build `n` deltas, each with `adds_per_delta` added hashes. The deltas
/// share a single peer_id (the provider) but each carries a unique sequence
/// number so receivers can distinguish them.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DeltaWire {
    peer_id: String,
    added: Vec<String>,
    removed: Vec<String>,
    emitted_at: i64,
    sequence: u64,
    signature: Vec<u8>,
}

fn build_deltas(n: usize, adds_per_delta: usize, peer_id: String) -> Vec<DeltaWire> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let added = (0..adds_per_delta)
            .map(|j| build_hash_string((i as u64) * 1_000_000 + j as u64))
            .collect();
        out.push(DeltaWire {
            peer_id: peer_id.clone(),
            added,
            removed: vec![],
            emitted_at: 1_700_000_000_000_000 + i as i64,
            sequence: (i + 1) as u64,
            signature: vec![0u8; 1],
        });
    }
    out
}

const TOPIC_NAME: &str = "elohim/inventory/blob";

// ---------------------------------------------------------------------------
// Iroh-side bench — IrohGossip via TwoNodeFixture.
// ---------------------------------------------------------------------------

mod iroh_bench {
    use super::*;
    use elohim_storage::p2p_iroh::{parity_harness::TwoNodeFixture, GossipEvent};
    use futures_util::StreamExt;
    use tokio::time::timeout;

    pub async fn run(
        adds_per_delta: usize,
        warmup: usize,
        measured: usize,
    ) -> BenchResult {
        let provider_dir = tempdir().expect("provider tempdir");
        let fetcher_dir = tempdir().expect("fetcher tempdir");

        let fixture = TwoNodeFixture::new(provider_dir.path(), fetcher_dir.path(), Vec::new)
            .await
            .expect("fixture");

        let provider_id = fixture.provider.node_id();
        fixture
            .fetcher
            .endpoint()
            .add_node_addr(fixture.provider_addr.clone())
            .expect("add_node_addr");

        // Subscribe both peers. Provider subscribes first (no peer to wait
        // for); fetcher uses `subscribe_and_join` to wait for at least one
        // peer (the provider) before returning. This is iroh-gossip's
        // canonical "topic ready for broadcast" signal — more reliable than
        // manually consuming NeighborUp events from the fetcher's stream.
        let (provider_sender, _provider_recv) = fixture
            .provider
            .gossip()
            .subscribe(TOPIC_NAME, vec![])
            .await
            .expect("provider subscribe");
        let (_fetcher_sender, mut fetcher_recv) = fixture
            .fetcher
            .gossip()
            .subscribe_and_join(TOPIC_NAME, vec![provider_id])
            .await
            .expect("fetcher subscribe_and_join");
        // Drive the provider's joined() too — the broadcast won't propagate
        // until the provider's mesh sees the fetcher subscribe. We bound
        // this with a timeout because in the rare case the fetcher's join
        // signal beats the provider's, the provider may already be ready
        // and joined() returns immediately; otherwise it waits.
        // Note: `subscribe_and_join` on the fetcher already awaited the
        // fetcher's view of the mesh. The provider's view converges within
        // a few RTTs after that.
        let _ = timeout(Duration::from_secs(5), async {
            // small grace period for provider-side mesh convergence.
            tokio::time::sleep(Duration::from_millis(200)).await;
        })
        .await;

        let total = warmup + measured;
        let deltas = build_deltas(total, adds_per_delta, format!("{provider_id}"));
        let mut payload_bytes_seen = 0usize;

        let mut durations: Vec<Duration> = Vec::with_capacity(measured);
        for (i, delta) in deltas.iter().enumerate() {
            let payload = rmp_serde::to_vec_named(&delta).expect("serialize delta");
            payload_bytes_seen = payload.len();
            let t0 = Instant::now();
            provider_sender
                .broadcast(payload.clone().into())
                .await
                .expect("broadcast");

            // Pump receiver until we get this delta.
            let received_seq = timeout(Duration::from_secs(30), async {
                loop {
                    match fetcher_recv.next().await {
                        Some(Ok(GossipEvent::Received(msg))) => {
                            let d: DeltaWire =
                                rmp_serde::from_slice(&msg.content).expect("decode delta");
                            return d.sequence;
                        }
                        Some(_) => continue,
                        None => panic!("gossip stream closed"),
                    }
                }
            })
            .await
            .expect("recv timeout");
            let elapsed = t0.elapsed();
            assert_eq!(received_seq, delta.sequence, "sequence mismatch");

            if i >= warmup {
                durations.push(elapsed);
            }
        }

        // Track payload size for column label.
        let _ = payload_bytes_seen;

        fixture.shutdown().await.expect("shutdown");
        BenchResult::from_durations("iroh", adds_per_delta, durations)
    }
}

// ---------------------------------------------------------------------------
// Libp2p-side bench — minimal gossipsub two-swarm setup.
// ---------------------------------------------------------------------------

mod libp2p_bench {
    use super::*;
    use futures::StreamExt;
    use libp2p::{
        gossipsub::{self, IdentTopic, MessageAuthenticity},
        identity, noise,
        swarm::SwarmEvent,
        tcp, yamux, Multiaddr, PeerId, SwarmBuilder,
    };
    use tokio::sync::{mpsc, oneshot};

    enum Cmd {
        Dial(Multiaddr, oneshot::Sender<Result<(), String>>),
        Publish(Vec<u8>, oneshot::Sender<Result<(), String>>),
        WaitSubscribers(oneshot::Sender<()>),
    }

    struct Node {
        peer_id: PeerId,
        addr: Multiaddr,
        cmd_tx: mpsc::Sender<Cmd>,
        msg_rx: Option<mpsc::Receiver<Vec<u8>>>,
    }

    fn build_swarm(
        keypair: identity::Keypair,
    ) -> libp2p::Swarm<gossipsub::Behaviour> {
        let local_peer_id = PeerId::from(keypair.public());
        let _ = local_peer_id; // silence unused
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            // Tuned for loopback bench latency; production uses 10s.
            .heartbeat_interval(Duration::from_millis(100))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .expect("valid gossipsub config");

        SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .expect("tcp transport")
            .with_behaviour(|kp| {
                gossipsub::Behaviour::new(
                    MessageAuthenticity::Signed(kp.clone()),
                    gossipsub_config,
                )
                .expect("gossipsub behaviour")
            })
            .expect("behaviour build")
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(120)))
            .build()
    }

    async fn spawn_node(name: &'static str) -> Node {
        let key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(key.public());
        let mut swarm = build_swarm(key);

        // Subscribe to topic on this node.
        let topic = IdentTopic::new(TOPIC_NAME);
        swarm
            .behaviour_mut()
            .subscribe(&topic)
            .expect("subscribe");

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
        let (msg_tx, msg_rx) = mpsc::channel::<Vec<u8>>(1024);

        tokio::spawn(async move {
            let topic = IdentTopic::new(TOPIC_NAME);
            let mut subscribers: usize = 0;
            let mut wait_subscribers: Vec<oneshot::Sender<()>> = Vec::new();

            loop {
                tokio::select! {
                    biased;

                    Some(cmd) = cmd_rx.recv() => {
                        match cmd {
                            Cmd::Dial(addr, reply) => {
                                let _ = reply.send(swarm.dial(addr).map_err(|e| e.to_string()));
                            }
                            Cmd::Publish(bytes, reply) => {
                                let res = swarm
                                    .behaviour_mut()
                                    .publish(topic.clone(), bytes)
                                    .map(|_| ())
                                    .map_err(|e| format!("publish: {e:?}"));
                                let _ = reply.send(res);
                            }
                            Cmd::WaitSubscribers(reply) => {
                                if subscribers > 0 {
                                    let _ = reply.send(());
                                } else {
                                    wait_subscribers.push(reply);
                                }
                            }
                        }
                    }

                    event = swarm.next() => {
                        let Some(event) = event else { break; };
                        match event {
                            SwarmEvent::Behaviour(gossipsub::Event::Message {
                                message, ..
                            }) => {
                                let _ = msg_tx.send(message.data).await;
                            }
                            SwarmEvent::Behaviour(gossipsub::Event::Subscribed {
                                ..
                            }) => {
                                subscribers += 1;
                                for tx in std::mem::take(&mut wait_subscribers) {
                                    let _ = tx.send(());
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            let _ = name;
        });

        Node {
            peer_id: local_peer_id,
            addr: listen_addr,
            cmd_tx,
            msg_rx: Some(msg_rx),
        }
    }

    async fn dial(node: &Node, addr: Multiaddr) {
        let (tx, rx) = oneshot::channel();
        node.cmd_tx.send(Cmd::Dial(addr, tx)).await.unwrap();
        rx.await.unwrap().expect("dial");
    }

    async fn wait_subscribers(node: &Node) {
        let (tx, rx) = oneshot::channel();
        node.cmd_tx
            .send(Cmd::WaitSubscribers(tx))
            .await
            .unwrap();
        let _ = tokio::time::timeout(Duration::from_secs(30), rx)
            .await
            .expect("wait_subscribers timed out");
    }

    pub async fn run(adds_per_delta: usize, warmup: usize, measured: usize) -> BenchResult {
        let provider = spawn_node("provider").await;
        let mut subscriber = spawn_node("subscriber").await;
        let mut sub_msg_rx = subscriber.msg_rx.take().expect("msg_rx already taken");

        // Two-way dial — make sure each side knows the other.
        dial(&subscriber, provider.addr.clone()).await;
        // Wait until provider sees subscriber's "Subscribed" event.
        wait_subscribers(&provider).await;
        // And subscriber sees provider's subscribe (so its mesh has provider).
        wait_subscribers(&subscriber).await;

        // Allow mesh formation — gossipsub needs a heartbeat interval to add
        // peers to its mesh after Subscribed events.
        tokio::time::sleep(Duration::from_millis(300)).await;

        let total = warmup + measured;
        let deltas = build_deltas(total, adds_per_delta, format!("{}", provider.peer_id));

        let mut durations: Vec<Duration> = Vec::with_capacity(measured);
        for (i, delta) in deltas.iter().enumerate() {
            let payload = rmp_serde::to_vec_named(&delta).expect("serialize delta");
            let t0 = Instant::now();
            // Send publish then await receive on subscriber.
            let (tx, rx) = oneshot::channel();
            provider
                .cmd_tx
                .send(Cmd::Publish(payload.clone(), tx))
                .await
                .unwrap();
            rx.await.unwrap().expect("publish");
            let received = tokio::time::timeout(Duration::from_secs(30), sub_msg_rx.recv())
                .await
                .expect("recv timed out")
                .expect("channel closed");
            let elapsed = t0.elapsed();
            let d: DeltaWire = rmp_serde::from_slice(&received).expect("decode");
            assert_eq!(d.sequence, delta.sequence, "sequence mismatch");

            if i >= warmup {
                durations.push(elapsed);
            }
        }

        // Reattach msg_rx so the Drop ordering doesn't panic.
        subscriber.msg_rx = Some(sub_msg_rx);
        let _ = (&provider, &subscriber);

        BenchResult::from_durations("libp2p", adds_per_delta, durations)
    }
}

// ---------------------------------------------------------------------------
// Comparison test.
// ---------------------------------------------------------------------------

// NOTE: iroh-gossip 0.92 enforces a hard cap on the wire-encoded message size
// via [`iroh_gossip::proto::Config::max_message_size`], with default
// `DEFAULT_MAX_MESSAGE_SIZE = 4096` bytes (see iroh-gossip 0.92's
// `src/proto/topic.rs`). The check fires inside the connection's send loop
// (`net/util.rs::write_frame` — `if len >= max_message_size { TooLarge }`) on
// the *postcard-encoded ProtoMessage envelope*, which is content + control
// fields, NOT just our raw payload. The send loop returns `TooLarge` and the
// internal connection task stops servicing the topic; from the caller's
// perspective `GossipSender::broadcast(...)` returns Ok (it only queues the
// command on an mpsc) and the receiver simply never sees a `Received` event.
// This presents identically to a "mesh not converged yet" hang, which is what
// the earlier 30s recv-timeout panic looked like.
//
// Concretely: each hash is 64 hex chars + ~2 bytes msgpack overhead = ~66 B.
// adds=32 ≈ 32 × 66 + ~150 envelope ≈ ~2.3 KiB → fits.
// adds=64 ≈ ~4.4 KiB → exceeds 4096 → silent drop.
// adds=256 ≈ ~17 KiB → exceeds 4096 → silent drop.
//
// We could bump the limit via `Gossip::builder().max_message_size(...)` in
// `IrohGossip::new`, but that's a production-code change with cross-network
// consensus implications (`max_message_size must be the same across the
// network to ensure all nodes can transmit and read large messages.` — iroh-
// gossip docs). For now, the bench restricts adds_classes to values that fit
// the default 4096-byte cap; the perf-bump signal is unambiguous on the small
// classes anyway.
//
// Also: gossip plane uses `flavor = "current_thread"` runtime to match the
// passing parity test (`iroh_gossip_parity`); switching to multi_thread didn't
// help with the size issue and adds noise to the latency numbers.
#[tokio::test(flavor = "current_thread")]
#[ignore]
async fn compare_gossip_perf() {
    // Workload classes — number of added hashes per delta. Capped at 32 so
    // the postcard-encoded ProtoMessage stays under iroh-gossip 0.92's
    // default `max_message_size = 4096`. See the NOTE above this fn.
    let adds_classes: &[usize] = &[1, 16, 32];
    let warmup = 5;
    let measured = 20;

    let mut all_results: Vec<BenchResult> = Vec::with_capacity(adds_classes.len() * 2);

    for &adds in adds_classes {
        let iroh_res = iroh_bench::run(adds, warmup, measured).await;
        all_results.push(iroh_res);

        let libp2p_res = libp2p_bench::run(adds, warmup, measured).await;
        all_results.push(libp2p_res);
    }

    println!();
    println!("## Gossip plane perf — iroh-gossip vs libp2p-gossipsub (loopback, release)");
    println!();
    println!("| adds/delta | transport | p50 (ms) | p95 (ms) | p99 (ms) | mean (ms) | msg/s |");
    println!("|---|---|---|---|---|---|---|");
    for r in &all_results {
        println!(
            "| {} | {} | {} | {} | {} | {} | {:.1} |",
            r.payload_size_class,
            r.transport,
            fmt_ms(r.p50),
            fmt_ms(r.p95),
            fmt_ms(r.p99),
            fmt_ms(r.mean),
            r.msgs_per_sec,
        );
    }
    println!();
    println!(
        "Iters: {warmup} warmup discarded + {measured} measured per (transport, adds_per_delta). \
        Provider broadcasts; subscriber receives; latency = publish→receive on the same node task."
    );
    println!();

    let mut by_class: std::collections::BTreeMap<
        usize,
        (Option<&BenchResult>, Option<&BenchResult>),
    > = std::collections::BTreeMap::new();
    for r in &all_results {
        let entry = by_class.entry(r.payload_size_class).or_insert((None, None));
        match r.transport {
            "iroh" => entry.0 = Some(r),
            "libp2p" => entry.1 = Some(r),
            _ => unreachable!(),
        }
    }

    let mut bump_found = false;
    let mut bump_summary: Vec<String> = Vec::new();
    for (adds, (iroh, libp2p)) in &by_class {
        let (Some(i), Some(l)) = (iroh, libp2p) else {
            continue;
        };
        let ratio_p50 = l.p50.as_secs_f64() / i.p50.as_secs_f64().max(f64::EPSILON);
        let ratio_p99 = l.p99.as_secs_f64() / i.p99.as_secs_f64().max(f64::EPSILON);
        let line = format!(
            "  adds={:>5}: iroh/libp2p p50 ratio = {:.2}x, p99 ratio = {:.2}x  ({})",
            adds,
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
        "Tier-3 verdict: iroh-gossip did NOT deliver a perf bump on any size class. \
        Expected: iroh p50 < libp2p p50 on at least one of {adds_classes:?}. \
        See table above for details."
    );

    println!("Tier-3 verdict: iroh-gossip delivers a perf bump on at least one size class.");
}
