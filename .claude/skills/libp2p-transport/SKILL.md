---
name: libp2p-transport
description: Reference for libp2p swarm setup, transport configuration, behaviour composition, and version differences between elohim-node (0.53) and elohim-storage (0.54). Use when someone asks "configure the swarm", "add a new behaviour", "set up transport", "libp2p version differences", or debugs P2P connection issues.
metadata:
  author: elohim-protocol
  version: 1.0.0
---

# libp2p Transport Reference

The Elohim Protocol uses libp2p for P2P networking in two crates with different versions.

## Two Implementations

| Crate | libp2p Version | Purpose |
|-------|---------------|---------|
| `elohim-node` | 0.53 | Always-on infrastructure runtime (family nodes) |
| `elohim-storage` | 0.54 | Content storage sidecar (dormant, needs activation) |

### Version Differences

| Feature | 0.53 (elohim-node) | 0.54 (elohim-storage) |
|---------|--------------------|-----------------------|
| Request-Response | `Behaviour::with_codec()` | `Behaviour::new()` or `with_codec()` |
| Swarm event loop | `StreamExt::next()` | `StreamExt::next()` |
| NetworkBehaviour derive | `#[derive(NetworkBehaviour)]` | `#[derive(NetworkBehaviour)]` |
| Auto-generated events | `ElohimBehaviourEvent` | Same pattern |
| Required features | `macros` + `ed25519` | `macros` + `ed25519` |

---

## `#[derive(NetworkBehaviour)]` Pattern

The derive macro composes multiple sub-behaviours into a single behaviour:

```rust
use libp2p::swarm::NetworkBehaviour;

#[derive(NetworkBehaviour)]
pub struct ElohimBehaviour {
    pub request_response: request_response::Behaviour<SyncCodec>,
    pub mdns: mdns::tokio::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    pub identify: libp2p::identify::Behaviour,
}
```

**Requires** `macros` and `ed25519` features in `Cargo.toml`:
```toml
libp2p = { version = "0.53", features = [
    "tokio", "tcp", "quic", "noise", "yamux",
    "mdns", "kad", "identify", "relay", "dcutr",
    "request-response",
    "macros",    # Required for #[derive(NetworkBehaviour)]
    "ed25519",   # Required for identity
] }
```

The derive **auto-generates** `ElohimBehaviourEvent` enum with variants for each sub-behaviour.

---

## Transport Stack

```
TCP + Noise (encryption) + Yamux (multiplexing)
  +
QUIC (built-in encryption + multiplexing)
```

### Building the Swarm

```rust
use libp2p::{identity, noise, tcp, yamux, SwarmBuilder, StreamProtocol};

let keypair = identity::Keypair::generate_ed25519();
let local_peer_id = PeerId::from(keypair.public());

let swarm = SwarmBuilder::with_existing_identity(keypair)
    .with_tokio()
    .with_tcp(
        tcp::Config::default(),
        noise::Config::new,       // Noise XX handshake
        yamux::Config::default,   // Yamux stream muxer
    )?
    .with_quic()                  // QUIC transport (UDP)
    .with_behaviour(|key| {
        // Build composite behaviour here
        ElohimBehaviour { ... }
    })?
    .with_swarm_config(|c| {
        c.with_idle_connection_timeout(Duration::from_secs(60))
    })
    .build();
```

---

## Swarm Event Loop

Uses `StreamExt::next()` (**not** `select_next_event()` which was deprecated):

```rust
use futures::StreamExt;
use libp2p::swarm::SwarmEvent as LibSwarmEvent;

loop {
    let Some(event) = swarm.next().await else {
        break;
    };

    match event {
        LibSwarmEvent::Behaviour(ElohimBehaviourEvent::Mdns(
            mdns::Event::Discovered(peers),
        )) => {
            for (peer_id, addr) in peers {
                // Handle peer discovery
            }
        }

        LibSwarmEvent::Behaviour(ElohimBehaviourEvent::RequestResponse(
            request_response::Event::Message { peer, message },
        )) => match message {
            request_response::Message::Request { request, channel, .. } => {
                // Handle incoming request, send response via channel
            }
            request_response::Message::Response { request_id, response } => {
                // Handle response to our outbound request
            }
        },

        LibSwarmEvent::NewListenAddr { address, .. } => {
            info!("Listening on {}", address);
        }

        _ => {}
    }
}
```

---

## Request-Response Setup

### libp2p 0.53 (elohim-node)

```rust
use libp2p::{request_response, StreamProtocol};

let sync_protocol = StreamProtocol::new("/elohim/sync/1.0.0");
let rr_config = request_response::Config::default()
    .with_request_timeout(Duration::from_secs(30));

// 0.53: Use with_codec()
let rr = request_response::Behaviour::with_codec(
    SyncCodec,
    [(sync_protocol, request_response::ProtocolSupport::Full)],
    rr_config,
);
```

### Sending Requests

```rust
// Send request
let request_id = swarm.behaviour_mut()
    .request_response
    .send_request(&peer_id, request);

// Send response (on incoming request channel)
swarm.behaviour_mut()
    .request_response
    .send_response(channel, response)?;
```

---

## Identity Keypair Management

```rust
use libp2p::identity;

// Generate new
let keypair = identity::Keypair::generate_ed25519();

// Persist to disk
let bytes = keypair.to_protobuf_encoding()?;
std::fs::write("node_key", &bytes)?;

// Load from disk
let bytes = std::fs::read("node_key")?;
let keypair = identity::Keypair::from_protobuf_encoding(&bytes)?;

// Get PeerId
let peer_id = PeerId::from(keypair.public());
```

---

## Listen Addresses

```rust
// From P2PConfig
for addr_str in &config.listen_addrs {
    let addr: Multiaddr = addr_str.parse()?;
    swarm.listen_on(addr)?;
}

// Common addresses:
// /ip4/0.0.0.0/tcp/4001        - TCP on all interfaces
// /ip4/0.0.0.0/udp/4001/quic-v1 - QUIC on all interfaces
// /ip6/::/tcp/4001              - IPv6 TCP
```

---

## Configuration

```rust
// elohim-node/src/config.rs
pub struct P2PConfig {
    pub listen_addrs: Vec<String>,     // Multiaddr strings
    pub bootstrap_nodes: Vec<String>,  // /ip4/.../p2p/12D3Koo...
    pub enable_mdns: bool,             // LAN discovery
    pub enable_kademlia: bool,         // Global DHT
}
```

---

## Build Requirements

```bash
# elohim-node: Override RUSTFLAGS (system sets getrandom for WASM)
RUSTFLAGS="" cargo build
RUSTFLAGS="" cargo test

# elohim-storage: Needs getrandom for WASM target
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```

---

## Key Files

| File | Purpose |
|------|---------|
| `elohim-node/src/p2p/transport.rs` | Swarm builder, behaviour composition, event loop |
| `elohim-node/src/p2p/mod.rs` | Module exports |
| `elohim-node/src/p2p/protocols.rs` | SyncCodec, protocol constants |
| `elohim-node/src/p2p/nat.rs` | NAT traversal (stub/TODO) |
| `elohim-node/Cargo.toml` | libp2p 0.53 with features |
| `holochain/elohim-storage/src/p2p/behaviour.rs` | Storage P2P behaviour (0.54) |
| `holochain/P2P-DATAPLANE.md` | Architecture design document |

## External References

- libp2p 0.53 docs: `https://docs.rs/libp2p/0.53.2/libp2p/`
- libp2p 0.54 docs: `https://docs.rs/libp2p/0.54.1/libp2p/`
- libp2p concepts: `https://libp2p.io/concepts/transports/overview/`
