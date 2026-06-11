---
name: libp2p-transport
description: Reference for libp2p swarm setup, transport configuration, and behaviour composition in steward/node and elohim-storage (both resolve libp2p 0.54.1). Use when someone asks "configure the swarm", "add a new behaviour", "set up transport", or debugs P2P connection issues.
metadata:
  author: elohim-protocol
  version: 1.0.0
---

# libp2p Transport Reference

> **Parallel stack:** A second transport stack lives alongside libp2p — `elohim-storage/src/p2p_iroh/` (iroh QUIC, ALPN-based, with iroh-blobs + iroh-gossip). Services are transport-neutral; libp2p and iroh adapters delegate to them. Selection is via `TransportBackend` config; runtime is mode-exclusive (libp2p OR iroh, never both). See `rust-architect.md` §"Truth in motion" for the design. This skill describes the libp2p side specifically; for iroh ALPN handler patterns see `project_iroh_alpn_handlers_one_stream_design.md` and the linked iroh memory entries, which carry the temporal phase/gate state.

The Elohim Protocol uses libp2p for P2P networking in two crates — both on the same version.

## Two Implementations

| Crate | libp2p Version | Purpose |
|-------|---------------|---------|
| `elohim-node` | **0.54.1** (Cargo.lock verified 2026-06-11) | Always-on infrastructure runtime (family nodes) |
| `elohim-storage` | **0.54.1** (Cargo.lock verified 2026-06-11) | Content storage sidecar |

> **Historical note:** An older "node 0.53 vs storage 0.54" split was documented here. The lockfiles falsify it — both crates have been on 0.54 since before the steward/ restructure. The `with_codec()` constructor was a pre-0.54 idiom; actual code uses `Behaviour::new()`.

### Shared API Idioms (0.54.1)

| Feature | Idiom |
|---------|-------|
| Request-Response construction | `Behaviour::new([(Proto, ProtocolSupport::Full)], cfg)` |
| Swarm event loop | `StreamExt::next()` (not the deprecated `select_next_event()`) |
| NetworkBehaviour derive | `#[derive(NetworkBehaviour)]` |
| Auto-generated events | `ElohimBehaviourEvent` enum |
| Required features | `macros` + `ed25519` |

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
libp2p = { version = "0.54", features = [
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

### libp2p 0.54 (both crates)

```rust
use libp2p::{request_response, StreamProtocol};

let sync_protocol = StreamProtocol::new("/elohim/sync/1.0.0");
let rr_config = request_response::Config::default()
    .with_request_timeout(Duration::from_secs(30));

// 0.54: Use new() with (protocol, support) pairs
let rr = request_response::Behaviour::new(
    [(sync_protocol, request_response::ProtocolSupport::Full)],
    rr_config,
);
// Note: with_codec() was a pre-0.54 idiom — no longer used here.
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
// steward/node/src/config.rs
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
| `steward/node/src/p2p/transport.rs` | Swarm builder, behaviour composition, event loop |
| `steward/node/src/p2p/mod.rs` | Module exports |
| `steward/node/src/p2p/protocols.rs` | SyncCodec, protocol constants |
| `steward/node/src/p2p/nat.rs` | NAT traversal (stub/TODO) |
| `steward/node/Cargo.toml` | libp2p 0.54 features declaration (resolves 0.54.1) |
| `elohim/elohim-storage/src/p2p/behaviour.rs` | Storage P2P behaviour (0.54) |
| `elohim/holochain/docs/P2P-DATAPLANE.md` | Architecture design document |

## External References

- libp2p 0.54.1 docs: `https://docs.rs/libp2p/0.54.1/libp2p/`
- libp2p concepts: `https://libp2p.io/concepts/transports/overview/`
