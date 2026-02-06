# elohim-node

The Elohim Protocol's infrastructure runtime for always-on nodes.

## What Is This?

elohim-node is the daemon that runs on family hardware - the plug-and-play nodes in a rack that form the backbone of the Elohim network. It's not a Holochain conductor wrapper; it's our own runtime designed for:

- **Device-to-Node Sync**: Your phone and laptop sync to your family node
- **Cluster-to-Cluster Sync**: Family clusters replicate to each other
- **Backup & Replication**: Data survives device and node failures
- **Always-On Presence**: The node that's there when your devices aren't

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           THE ELOHIM NETWORK                                 │
│                                                                              │
│   DEVICES (ephemeral)              NODES (always-on)                        │
│   ┌─────────────────┐              ┌─────────────────────────────────────┐  │
│   │  Phone          │              │         FAMILY CLUSTER A            │  │
│   │  Laptop         │◄────────────►│  ┌─────────┐  ┌─────────┐          │  │
│   │  Tablet         │   sync       │  │ Node 1  │◄►│ Node 2  │  blades  │  │
│   └─────────────────┘              │  └─────────┘  └─────────┘          │  │
│                                    └──────────────────┬──────────────────┘  │
│                                                       │                      │
│                                                       │ cluster sync         │
│                                                       │                      │
│                                    ┌──────────────────┴──────────────────┐  │
│                                    │         FAMILY CLUSTER B            │  │
│                                    │  ┌─────────┐  ┌─────────┐          │  │
│                                    │  │ Node 1  │◄►│ Node 2  │          │  │
│                                    │  └─────────┘  └─────────┘          │  │
│                                    └─────────────────────────────────────┘  │
│                                                                              │
│   Devices come and go.             Nodes are the stable infrastructure.     │
│   They sync when online.           They're always there.                    │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Why Not Just Use Holochain?

Holochain is excellent for what it does: agent-centric validation, cryptographic identity, and distributed consensus. We use it for the **Trust Layer**.

But Holochain's DHT wasn't designed for:
- Large content storage (chokes at ~3000 entries)
- Real-time sync (gossip latency 200-2000ms)
- Device-to-node backup patterns
- Cluster orchestration

elohim-node handles the **Data Layer** - the actual bytes, the sync, the replication.

See [P2P-DATAPLANE.md](../P2P-DATAPLANE.md) for the full architectural separation.

## Core Components

```
elohim-node/
├── src/
│   ├── main.rs           # Daemon entry point
│   ├── config.rs         # Node configuration
│   │
│   ├── sync/             # Automerge sync engine
│   │   ├── mod.rs
│   │   ├── stream.rs     # Stream positions (Matrix-inspired)
│   │   ├── merge.rs      # CRDT conflict resolution
│   │   └── protocol.rs   # Sync wire protocol
│   │
│   ├── cluster/          # Family cluster orchestration
│   │   ├── mod.rs
│   │   ├── discovery.rs  # mDNS local discovery
│   │   ├── membership.rs # Cluster join/leave
│   │   └── leader.rs     # Leader election (if needed)
│   │
│   ├── storage/          # Content-addressed storage
│   │   ├── mod.rs
│   │   ├── blobs.rs      # Blob store
│   │   ├── sharding.rs   # Reed-Solomon encoding
│   │   └── reach.rs      # Reach-based access control
│   │
│   ├── p2p/              # libp2p networking
│   │   ├── mod.rs
│   │   ├── transport.rs  # QUIC/TCP/WebRTC
│   │   ├── protocols.rs  # /elohim/sync, /elohim/shard
│   │   └── nat.rs        # NAT traversal
│   │
│   └── api/              # Control APIs
│       ├── mod.rs
│       ├── http.rs       # REST API for management
│       └── grpc.rs       # gRPC for device clients
│
├── Cargo.toml
└── README.md
```

## Sync Topology

### Device → Node

Devices (phones, laptops) are ephemeral. They sync to their home node:

```
Phone                          Family Node
  │                                │
  │──── "I'm at position 42" ─────►│
  │                                │
  │◄─── "Here's 43-50" ────────────│
  │                                │
  │──── "Here's my local 51" ─────►│
  │                                │
  │     (positions converge)       │
```

The node is the stable anchor. When your phone dies, your data is safe.

### Node → Node (within cluster)

Blades in a family cluster replicate for redundancy:

```
Node A (blade 1)              Node B (blade 2)
  │                                │
  │◄────── mDNS discovery ────────►│
  │                                │
  │◄────── full replication ──────►│
  │                                │
  │    (both have everything)      │
```

Within a cluster, nodes maintain full copies. One blade can fail.

### Cluster → Cluster

Families replicate to other families based on trust relationships:

```
Family Cluster A              Family Cluster B
  │                                │
  │──── "We have content X" ──────►│
  │                                │
  │◄─── "We'll replicate it" ──────│
  │     (if reach allows)          │
  │                                │
  │    (selective replication)     │
```

Cross-cluster replication follows reach rules. Your private data stays private.

## Reach-Based Replication

Content replicates based on its reach level:

| Reach | Replicates To | Encrypted |
|-------|---------------|-----------|
| `private` | Only my devices | Yes (device keys) |
| `invited` | Named agents | Yes (per-relationship) |
| `local` | Family cluster | Yes (cluster key) |
| `neighborhood` | Trusted clusters | Optional |
| `commons` | Any willing node | No |

See [REACH.md](../REACH.md) for enforcement details.

## Configuration

```toml
# elohim-node.toml

[node]
id = "family-node-01"
data_dir = "/var/lib/elohim"
cluster_name = "johnson-family"

[sync]
# Automerge settings
max_document_size = "10MB"
sync_interval_ms = 1000

[cluster]
# Local cluster discovery
mdns_enabled = true
cluster_key = "..."  # Shared secret for cluster membership

[p2p]
# libp2p settings
listen_addrs = ["/ip4/0.0.0.0/tcp/4001", "/ip4/0.0.0.0/udp/4001/quic-v1"]
bootstrap_nodes = [
    "https://doorway.elohim.host/signal/p2p"
]

[storage]
max_capacity = "500GB"
shard_redundancy = 3  # Reed-Solomon 4+3

[api]
http_port = 8080
grpc_port = 9090
```

## Relationship to Other Components

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          ELOHIM INFRASTRUCTURE                               │
│                                                                              │
│   ┌─────────────────────┐     ┌─────────────────────┐                       │
│   │     elohim-node     │     │      doorway        │                       │
│   │                     │     │                     │                       │
│   │  • Always-on daemon │     │  • Web2 gateway     │                       │
│   │  • Device sync      │◄───►│  • Bootstrap node   │                       │
│   │  • Cluster sync     │     │  • Browser bridge   │                       │
│   │  • P2P backbone     │     │                     │                       │
│   └──────────┬──────────┘     └─────────────────────┘                       │
│              │                                                               │
│              │ uses                                                          │
│              ▼                                                               │
│   ┌─────────────────────┐     ┌─────────────────────┐                       │
│   │   elohim-storage    │     │   Holochain DNA     │                       │
│   │                     │     │                     │                       │
│   │  • Blob storage     │     │  • Identity         │                       │
│   │  • RS sharding      │     │  • Attestations     │                       │
│   │  • Content-address  │     │  • Trust graph      │                       │
│   └─────────────────────┘     └─────────────────────┘                       │
│                                                                              │
│   elohim-node orchestrates. elohim-storage stores. Holochain validates.     │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Getting Started

```bash
# Build
cargo build --release

# Run with config
./target/release/elohim-node --config elohim-node.toml

# Or with environment
ELOHIM_DATA_DIR=/var/lib/elohim \
ELOHIM_CLUSTER_NAME=my-family \
./target/release/elohim-node
```

## Deployment Options

### Docker (Recommended for Testing)

```bash
# Build image
docker build -t elohim-node .

# Run single node
docker run -d \
  -e ELOHIM_CLUSTER_NAME=my-family \
  -e ELOHIM_CLUSTER_KEY=my-secret-key \
  -v elohim-data:/var/lib/elohim \
  -p 4001:4001 -p 8080:8080 \
  elohim-node
```

### Cluster Simulation

Simulate multi-family clusters with network conditions:

```bash
cd simulation

# Start 2 families × 2 nodes
./simulate.sh start

# With WAN latency (50ms between clusters)
./simulate.sh start --latency

# Check status
./simulate.sh status

# Simulate network partition
./simulate.sh partition

# Heal partition
./simulate.sh heal
```

See [simulation/README.md](./simulation/README.md) for full testing scenarios.

### NixOS (Production Hardware)

For real family node hardware:

```nix
# In your NixOS configuration
{
  imports = [ ./path/to/elohim-node/nix/module.nix ];

  services.elohim-node = {
    enable = true;
    clusterName = "johnson-family";
    clusterKeyFile = "/run/secrets/cluster-key";
    openFirewall = true;

    settings = {
      storage.max_capacity = "1TB";
    };
  };
}
```

Or use the flake:

```bash
# Development shell
nix develop

# Build package
nix build .#elohim-node

# Build Docker image via Nix
nix build .#dockerImage
```

## Development Status

**Current**: Architecture design phase

**Next Steps**:
1. Implement core sync engine (Automerge integration)
2. Implement cluster discovery (mDNS)
3. Implement P2P transport (libp2p)
4. Integrate with elohim-storage for blob handling
5. Build device client SDK

See [ARCHITECTURE.md](./ARCHITECTURE.md) for implementation details.
