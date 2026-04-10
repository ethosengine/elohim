# Tier 3 Peer Metrics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Populate the three remaining tractable PeerInfoView fields (direction, lastSeenMs, rttMs) using libp2p event loop wiring.

**Architecture:** Add a `PeerMetrics` DashMap to P2PNode that tracks per-peer connection direction, last-seen timestamp, and RTT samples from the new ping behaviour. The ListPeers command handler reads from this map alongside the existing identify cache.

**Tech Stack:** libp2p 0.54 (ping feature), DashMap, VecDeque ring buffer

**Branch:** `dev` (direct, one commit at sprint end)

---

### Task 1: Add ping feature and behaviour

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml:126-141`
- Modify: `elohim/elohim-storage/src/p2p/behaviour.rs`

- [ ] **Step 1: Add ping feature to Cargo.toml**

In `elohim/elohim-storage/Cargo.toml`, add `"ping"` to the libp2p features list:

```toml
libp2p = { version = "0.54", features = [
    "tcp",
    "dns",
    "noise",
    "yamux",
    "kad",
    "request-response",
    "mdns",
    "relay",
    "dcutr",
    "autonat",
    "tokio",
    "macros",
    "identify",
    "ed25519",
    "serde",
    "ping",
], optional = true }
```

- [ ] **Step 2: Add ping import to behaviour.rs**

In `elohim/elohim-storage/src/p2p/behaviour.rs`, add `ping` to the libp2p import (line 3):

```rust
use libp2p::{
    autonat, dcutr, identify,
    identity::Keypair,
    kad::{self, Behaviour as Kademlia},
    mdns, ping, relay,
    request_response::{self, Behaviour as RequestResponse, ProtocolSupport},
    swarm::{behaviour::toggle::Toggle, NetworkBehaviour},
    PeerId,
};
```

- [ ] **Step 3: Add ping field to ElohimStorageBehaviour**

After the `autonat` field (line 85), add:

```rust
    /// Ping protocol for RTT measurement
    pub ping: ping::Behaviour,
```

- [ ] **Step 4: Add Ping event variant**

After the `AutoNat` variant (line 117), add:

```rust
    /// Ping event (RTT measurement)
    Ping(ping::Event),
```

- [ ] **Step 5: Add From impl for ping::Event**

After the `From<autonat::Event>` impl (line 201), add:

```rust
impl From<ping::Event> for ElohimStorageBehaviourEvent {
    fn from(event: ping::Event) -> Self {
        Self::Ping(event)
    }
}
```

- [ ] **Step 6: Construct ping behaviour in ElohimStorageBehaviour::new()**

After the autonat construction (before the `Self { ... }` block at line 276), add:

```rust
        // Ping for RTT measurement
        let ping = ping::Behaviour::new(ping::Config::new());
```

And add `ping` to the `Self { ... }` struct literal (after `autonat`):

```rust
        Self {
            kademlia,
            shard_protocol,
            sync_protocol,
            epr_protocol,
            trust_protocol,
            mdns,
            relay_client,
            relay_server,
            dcutr,
            identify,
            autonat,
            ping,
        }
```

- [ ] **Step 7: Verify compilation**

Run:
```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | grep "^error" | head -5
```

Expected: no errors.

---

### Task 2: Add PeerMetrics struct and DashMap

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs`

- [ ] **Step 1: Add PeerMetrics struct**

After the `CachedIdentifyInfo` struct (around line 237), add:

```rust
/// Per-peer runtime metrics tracked from swarm events.
/// Stored in `peer_metrics: Arc<DashMap<String, PeerMetrics>>` on P2PNode.
struct PeerMetrics {
    /// Connection direction: "inbound" or "outbound"
    direction: &'static str,
    /// Unix epoch millis of last peer activity
    last_seen_ms: u64,
    /// Ring buffer of RTT samples from ping (max 8)
    rtt_samples: std::collections::VecDeque<Duration>,
}
```

- [ ] **Step 2: Add now_unix_ms helper**

After the `PeerMetrics` struct, add a helper to avoid repeating the timestamp pattern:

```rust
/// Current unix epoch in milliseconds.
fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
```

- [ ] **Step 3: Add median_rtt helper**

After `now_unix_ms`, add:

```rust
/// Compute median RTT from a ring buffer of samples.
fn median_rtt(samples: &std::collections::VecDeque<Duration>) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted: Vec<f64> = samples.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        Some((sorted[mid - 1] + sorted[mid]) / 2.0)
    } else {
        Some(sorted[mid])
    }
}
```

- [ ] **Step 4: Add peer_metrics field to P2PNode**

In the P2PNode struct, after the `identify_cache` field, add:

```rust
    /// Per-peer runtime metrics (direction, last_seen, RTT)
    peer_metrics: Arc<DashMap<String, PeerMetrics>>,
```

- [ ] **Step 5: Initialize peer_metrics in P2PNode constructor**

In the `P2PNode::new()` method, after `identify_cache: Arc::new(DashMap::new()),`, add:

```rust
            peer_metrics: Arc::new(DashMap::new()),
```

- [ ] **Step 6: Verify compilation**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | grep "^error" | head -5
```

Expected: no errors (PeerMetrics is defined but not yet used — that's fine, we wire it next).

---

### Task 3: Wire event handlers

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs`

- [ ] **Step 1: Track direction + last_seen on ConnectionEstablished**

In the `SwarmEvent::ConnectionEstablished` handler (line 1304), after the existing `debug!` log, add peer metrics tracking before the Kademlia block:

```rust
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                debug!(peer = %peer_id, "Connected to peer");
                // Track connection direction and last-seen for /p2p/peers
                let direction = if endpoint.is_dialer() {
                    "outbound"
                } else {
                    "inbound"
                };
                self.peer_metrics
                    .entry(peer_id.to_string())
                    .and_modify(|m| {
                        m.last_seen_ms = now_unix_ms();
                    })
                    .or_insert_with(|| PeerMetrics {
                        direction,
                        last_seen_ms: now_unix_ms(),
                        rtt_samples: std::collections::VecDeque::with_capacity(8),
                    });
                {
                    // ... existing Kademlia block ...
```

- [ ] **Step 2: Clean up on ConnectionClosed**

In the `SwarmEvent::ConnectionClosed` handler (line 1338), after the existing `self.peer_trust_cache.remove`, add:

```rust
                self.peer_metrics.remove(&peer_id.to_string());
                self.identify_cache.remove(&peer_id.to_string());
```

- [ ] **Step 3: Handle ping events**

In the `handle_behaviour_event` method, find the catch-all for identify events. After the autonat handler block, add a new match arm for ping:

```rust
            behaviour::ElohimStorageBehaviourEvent::Ping(ping::Event {
                peer,
                result: Ok(rtt),
                ..
            }) => {
                let pid = peer.to_string();
                self.peer_metrics
                    .entry(pid)
                    .and_modify(|m| {
                        if m.rtt_samples.len() >= 8 {
                            m.rtt_samples.pop_front();
                        }
                        m.rtt_samples.push_back(rtt);
                        m.last_seen_ms = now_unix_ms();
                    })
                    .or_insert_with(|| {
                        let mut samples = std::collections::VecDeque::with_capacity(8);
                        samples.push_back(rtt);
                        PeerMetrics {
                            direction: "unknown",
                            last_seen_ms: now_unix_ms(),
                            rtt_samples: samples,
                        }
                    });
            }
            behaviour::ElohimStorageBehaviourEvent::Ping(ping::Event {
                result: Err(_), ..
            }) => {
                // Ping failure — don't update metrics
            }
```

- [ ] **Step 4: Update last_seen on identify events**

In the existing `Identify::Event::Received` handler (around line 1835), after the identify cache insert and before the Kademlia block, add:

```rust
                // Update last-seen timestamp
                if let Some(mut m) = self.peer_metrics.get_mut(&peer_id.to_string()) {
                    m.last_seen_ms = now_unix_ms();
                }
```

- [ ] **Step 5: Verify compilation**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | grep "^error" | head -5
```

Expected: no errors. There may be a warning about `ping` needing to be imported — if so, add `use libp2p::ping;` at the top of mod.rs alongside the existing libp2p imports.

---

### Task 4: Update ListPeers handler to read metrics

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs`

- [ ] **Step 1: Replace hardcoded values with metrics lookups**

Replace the `P2PCommand::ListPeers` handler (around line 1037) with:

```rust
            P2PCommand::ListPeers { reply } => {
                let peers: Vec<PeerInfoView> = swarm
                    .connected_peers()
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|pid| {
                        let pid_str = pid.to_string();
                        let cached = self.identify_cache.get(&pid_str);
                        let metrics = self.peer_metrics.get(&pid_str);
                        PeerInfoView {
                            peer_id: pid_str,
                            multiaddrs: cached
                                .as_ref()
                                .map(|c| c.listen_addrs.clone())
                                .unwrap_or_default(),
                            protocols: cached
                                .as_ref()
                                .map(|c| c.protocols.clone())
                                .unwrap_or_default(),
                            agent_version: cached
                                .as_ref()
                                .map(|c| c.agent_version.clone())
                                .unwrap_or_default(),
                            direction: metrics
                                .as_ref()
                                .map(|m| m.direction.to_string())
                                .unwrap_or_else(|| "unknown".to_string()),
                            rtt_ms: metrics.as_ref().and_then(|m| median_rtt(&m.rtt_samples)),
                            last_seen_ms: metrics.as_ref().map(|m| m.last_seen_ms),
                            remote_nat_status: None,
                            bandwidth_in: None,
                            bandwidth_out: None,
                        }
                    })
                    .collect();
                let _ = reply.send(peers);
            }
```

- [ ] **Step 2: Verify compilation and run schema contract tests**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | grep "^error" | head -5
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract 2>&1 | tail -12
```

Expected: no errors, all 7 schema contract tests pass.

---

### Task 5: Quality gates and commit

- [ ] **Step 1: Format and lint**

```bash
cd elohim/elohim-storage
cargo fmt
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings 2>&1 | grep "^error" | head -5
```

Expected: no errors.

- [ ] **Step 2: Run full schema contract tests**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract 2>&1 | tail -12
```

Expected: all 7 tests pass.

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim
git add \
  elohim/elohim-storage/Cargo.toml \
  elohim/elohim-storage/Cargo.lock \
  elohim/elohim-storage/src/p2p/behaviour.rs \
  elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(p2p): populate PeerInfoView direction, rttMs, lastSeenMs

- Add ping behaviour to ElohimStorageBehaviour for RTT measurement
- Add PeerMetrics struct with 8-sample RTT ring buffer (median)
- Track connection direction from SwarmEvent::ConnectionEstablished
- Update last_seen_ms from ping, identify, and connection events
- Clean up peer_metrics and identify_cache on ConnectionClosed
- Wire metrics into ListPeers command handler for /p2p/peers endpoint

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```
