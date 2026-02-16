---
name: libp2p-discovery
description: Reference for P2P peer discovery (mDNS, Kademlia), NAT traversal (relay, DCUTR), peer tracking, and network topology. Use when someone asks "peers not connecting", "add bootstrap nodes", "mDNS not finding peers", "configure Kademlia", or works on NAT traversal and peer discovery.
metadata:
  author: elohim-protocol
  version: 1.0.0
---

# libp2p Discovery & NAT Traversal Reference

Peer discovery and connectivity management for the Elohim P2P network.

## Discovery Mechanisms

### 1. mDNS (Local Network Discovery)

Zero-configuration discovery on the local network (LAN). Peers on the same subnet find each other automatically.

```rust
use libp2p::mdns;

// In ElohimBehaviour
pub mdns: mdns::tokio::Behaviour,

// Setup
let mdns = mdns::tokio::Behaviour::new(
    mdns::Config::default(),
    key.public().to_peer_id(),
)?;
```

**Events:**
```rust
// Peer discovered on local network
mdns::Event::Discovered(Vec<(PeerId, Multiaddr)>)

// Peer no longer advertising
mdns::Event::Expired(Vec<(PeerId, Multiaddr)>)
```

**When discovered peers arrive:**
```rust
for (peer_id, addr) in peers {
    // Add to Kademlia routing table for global lookups
    swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
    // Notify coordinator for sync initiation
    event_tx.send(SwarmEvent::PeerDiscovered { peer_id, addrs: vec![addr] }).await;
}
```

**Use case:** Family cluster discovery. Nodes on the same home network find each other without configuration.

---

### 2. Kademlia DHT (Global Discovery)

Distributed hash table for peer discovery across the internet.

```rust
use libp2p::kad;

// In ElohimBehaviour
pub kademlia: kad::Behaviour<kad::store::MemoryStore>,

// Setup
let store = kad::store::MemoryStore::new(key.public().to_peer_id());
let mut kademlia = kad::Behaviour::new(key.public().to_peer_id(), store);
```

**Mode configuration:**

| Mode | Used By | Description |
|------|---------|-------------|
| `Server` | elohim-node | Actively stores and serves routing info |
| `Client` | elohim-storage | Only queries, doesn't serve |

```rust
// elohim-node: Server mode (holds routing table entries)
kademlia.set_mode(Some(kad::Mode::Server));

// elohim-storage: Client mode (lighter, query-only)
kademlia.set_mode(Some(kad::Mode::Client));
```

**Bootstrap peer addition:**
```rust
// Parse multiaddr with /p2p/ component
// e.g., /ip4/1.2.3.4/tcp/4001/p2p/12D3KooWAbCdEf...
fn parse_peer_addr(addr_str: &str) -> Option<(PeerId, Multiaddr)> {
    let addr: Multiaddr = addr_str.parse().ok()?;
    let peer_id = addr.iter().find_map(|p| {
        if let Protocol::P2p(peer_id) = p { Some(peer_id) } else { None }
    })?;
    let addr_without_p2p: Multiaddr = addr.iter()
        .filter(|p| !matches!(p, Protocol::P2p(_)))
        .collect();
    Some((peer_id, addr_without_p2p))
}

// Add bootstrap nodes
for node_str in &config.bootstrap_nodes {
    if let Some((peer_id, addr)) = parse_peer_addr(node_str) {
        swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
    }
}
```

---

### 3. Identify Protocol

Exchanges node metadata with connected peers.

```rust
use libp2p::identify;

let identify = identify::Behaviour::new(
    identify::Config::new(
        "/elohim/id/1.0.0".to_string(),
        key.public(),
    )
    .with_agent_version(format!("elohim-node/{}", env!("CARGO_PKG_VERSION"))),
);
```

**Events:**
```rust
identify::Event::Received { peer_id, info } => {
    // info.agent_version: "elohim-node/0.1.0"
    // info.listen_addrs: Vec<Multiaddr>
    // info.protocols: Vec<StreamProtocol>
}
```

**Use case:** Determine peer capabilities, version, and reachable addresses.

---

## Bootstrap Peer Management

### Signal Server Bootstrap

The doorway signal server provides initial peer discovery:

```
1. New node starts
2. GET /signal/{pubkey} -> list of peer multiaddrs
3. Connect directly to returned peers
4. Add to Kademlia for ongoing discovery
```

### Manual Peer Addition

```rust
// In ElohimSwarm
pub fn add_peer(&mut self, peer_id: PeerId, addrs: Vec<Multiaddr>) {
    for addr in addrs {
        self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
    }
}
```

### Initial Kademlia Population

After bootstrap, initiate Kademlia routing table refresh:

```rust
// Find closest peers to random key (routing table fill)
swarm.behaviour_mut().kademlia.get_closest_peers(PeerId::random());
```

---

## NAT Traversal (Roadmap)

### Current State

NAT traversal is currently **stub/TODO** in `elohim-node/src/p2p/nat.rs`.

### Planned: Relay Protocol

For peers behind NATs that can't be reached directly:

```rust
use libp2p::relay;

// On relay node (public IP)
let relay = relay::Behaviour::new(
    key.public().to_peer_id(),
    relay::Config::default(),
);

// On client behind NAT
let relay_client = relay::client::Behaviour::new(
    key.public().to_peer_id(),
    relay::client::Config::default(),
);
```

**Relay flow:**
```
Node A (behind NAT) -> Relay (public) -> Node B (behind NAT)
```

### Planned: DCUTR (Direct Connection Upgrade through Relay)

After relay establishes connection, attempt direct hole-punching:

```rust
use libp2p::dcutr;

let dcutr = dcutr::Behaviour::new(key.public().to_peer_id());
```

**DCUTR flow:**
```
1. A connects to B via relay
2. Exchange observed addresses
3. Simultaneous connect attempt (hole punch)
4. If successful: direct connection, relay dropped
```

### Planned: AutoNAT

Detect NAT status automatically:

```rust
use libp2p::autonat;

let autonat = autonat::Behaviour::new(
    key.public().to_peer_id(),
    autonat::Config::default(),
);
```

**Status detection:**
- `NatStatus::Public` - Directly reachable
- `NatStatus::Private` - Behind NAT, needs relay
- `NatStatus::Unknown` - Not enough data

---

## Peer Tracking

The `SyncCoordinator` tracks known peers and their sync state:

```rust
// elohim-node/src/network/operator.rs
pub struct PeerState {
    pub peer_id: PeerId,
    pub addrs: Vec<Multiaddr>,
    pub last_seen: Instant,
    pub sync_position: u64,         // Their last known position
    pub pending_requests: HashSet<OutboundRequestId>,
}
```

### Peer Lifecycle

```
1. Discovered (mDNS or Kademlia)    -> PeerDiscovered event
2. Connected (TCP/QUIC handshake)    -> ConnectionEstablished
3. Identified (identify protocol)    -> Know capabilities
4. Syncing (exchange positions)      -> Active data flow
5. Disconnected                      -> ConnectionClosed
6. Expired (mDNS timeout)           -> PeerExpired event
```

---

## Multiaddr Format

```
/ip4/192.168.1.100/tcp/4001                    # TCP IPv4
/ip4/1.2.3.4/udp/4001/quic-v1                  # QUIC IPv4
/ip6/::1/tcp/4001                               # TCP IPv6
/ip4/1.2.3.4/tcp/4001/p2p/12D3KooW...          # With peer ID
/dns4/node.elohim.host/tcp/4001/p2p/12D3KooW... # DNS
```

---

## Gotchas

1. **RUSTFLAGS override** - Must use `RUSTFLAGS="" cargo build` for elohim-node. System sets getrandom cfg for WASM builds.

2. **mDNS requires tokio** - Use `mdns::tokio::Behaviour`, not `mdns::async_io::Behaviour`.

3. **Kademlia Server vs Client** - Server mode on always-on nodes, Client mode on ephemeral devices. Wrong mode wastes resources or limits discovery.

4. **Relay is not implemented yet** - NAT traversal is a TODO. Currently only works on same LAN (mDNS) or with direct connectivity.

5. **Bootstrap cold start** - First node in a network has no peers. Signal server bootstrap is essential.

---

## Key Files

| File | Purpose |
|------|---------|
| `elohim-node/src/p2p/transport.rs` | Swarm builder, Kademlia + mDNS + identify setup |
| `elohim-node/src/p2p/nat.rs` | NAT traversal (stub - TODO) |
| `elohim-node/src/network/operator.rs` | Peer tracking, sync state |
| `elohim-node/src/config.rs` | P2PConfig (listen addrs, bootstrap nodes) |
| `holochain/P2P-DATAPLANE.md` | Bootstrap flow, architecture |
| `holochain/elohim-storage/src/p2p/behaviour.rs` | Storage P2P behaviour |

## External References

- libp2p-kad: `https://docs.rs/libp2p-kad/`
- libp2p-mdns: `https://docs.rs/libp2p-mdns/`
- libp2p-relay: `https://docs.rs/libp2p-relay/`
- libp2p-dcutr: `https://docs.rs/libp2p-dcutr/`
- NAT concepts: `https://libp2p.io/concepts/nat/overview/`
