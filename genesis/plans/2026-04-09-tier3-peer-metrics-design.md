# Tier 3 Peer Metrics — Design

**Date:** 2026-04-09
**Status:** Approved
**Prerequisite:** `e41aef67` (peer-status schema contract sprint)
**Branch:** `dev` (direct, one commit at sprint end)

## Scope

Populate three PeerInfoView fields that were declared nullable in the schema contract sprint:

| Field | Source | Effort |
|-------|--------|--------|
| `direction` | `SwarmEvent::ConnectionEstablished` endpoint | Trivial |
| `lastSeenMs` | Multiple event timestamps | Easy |
| `rttMs` | New `ping` behaviour + 8-sample ring buffer, median | Medium |

**Explicitly out of scope:** `remoteNatStatus` (no clean data source without custom protocol), `bandwidthIn`/`bandwidthOut` (per-peer tracking not natively supported in libp2p 0.54). Both remain null.

## Design

### New struct: PeerMetrics

```rust
struct PeerMetrics {
    direction: &'static str,           // "inbound" or "outbound"
    last_seen_ms: u64,                 // unix epoch millis
    rtt_samples: VecDeque<Duration>,   // ring buffer, max 8
}
```

Stored in `peer_metrics: Arc<DashMap<String, PeerMetrics>>` on P2PNode. Same pattern as `identify_cache` and `delivery_peers`.

### Event wiring

| Event | Action |
|-------|--------|
| `SwarmEvent::ConnectionEstablished { endpoint, .. }` | Insert `PeerMetrics` with direction from `ConnectedPoint::Dialer` (outbound) vs `Listener` (inbound). Set `last_seen_ms` to now. |
| `SwarmEvent::ConnectionClosed { peer_id, .. }` | Remove from `peer_metrics` and `identify_cache`. Prevents stale data. |
| `ping::Event { peer, result: Ok(rtt) }` | Push `rtt` into ring buffer (pop front if len > 8). Update `last_seen_ms`. |
| `Identify::Event::Received { peer_id, .. }` | Update `last_seen_ms`. |
| Any `request_response::Message::Request` or `Response` | Update `last_seen_ms` for the peer. |

### RTT aggregation

8-sample ring buffer per peer. Report median (resistant to outlier spikes). At libp2p's default ~15s ping interval, this gives a ~2 minute rolling window.

```rust
fn median_rtt(samples: &VecDeque<Duration>) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted: Vec<f64> = samples.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        Some((sorted[mid - 1] + sorted[mid]) / 2.0)
    } else {
        Some(sorted[mid])
    }
}
```

### Cargo.toml change

Add `"ping"` to the libp2p features list (line 126).

### Behaviour change

Add `pub ping: ping::Behaviour` to `ElohimStorageBehaviour` in `behaviour.rs`. Add `Ping(ping::Event)` to `ElohimStorageBehaviourEvent`. Construct with `ping::Behaviour::new(ping::Config::new())` in the behaviour builder.

### ListPeers handler change

In `P2PCommand::ListPeers` match arm (mod.rs), read from `peer_metrics` alongside `identify_cache`:

```rust
let metrics = self.peer_metrics.get(&pid_str);
PeerInfoView {
    // ... existing identify fields ...
    direction: metrics.as_ref().map(|m| m.direction.to_string()).unwrap_or_else(|| "unknown".to_string()),
    rtt_ms: metrics.as_ref().and_then(|m| median_rtt(&m.rtt_samples)),
    last_seen_ms: metrics.as_ref().map(|m| m.last_seen_ms),
    remote_nat_status: None,
    bandwidth_in: None,
    bandwidth_out: None,
}
```

### Files touched

| File | Change |
|------|--------|
| `elohim/elohim-storage/Cargo.toml` | Add `"ping"` feature |
| `elohim/elohim-storage/src/p2p/behaviour.rs` | Add `ping` field + event variant |
| `elohim/elohim-storage/src/p2p/mod.rs` | PeerMetrics struct, event handlers, ListPeers update, cleanup on disconnect |

### No schema or harness changes

All fields already declared in `peer-info-view.schema.json`. Existing harness tests already validate with populated values. No codegen run needed.
