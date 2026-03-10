# elohim-node Architecture

## Overview

elohim-node is the always-on runtime that forms the stable backbone of the Elohim network. While devices come and go, nodes persist. While Holochain handles trust, nodes handle data.

---

## Design Principles

### 1. Devices Are Ephemeral, Nodes Are Stable

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        EPHEMERALITY SPECTRUM                                 │
│                                                                              │
│   Phone         Laptop        Node          Cluster       Network           │
│   ──────────────────────────────────────────────────────────────►          │
│   minutes       hours         months        years         forever           │
│   (battery)     (sleep)       (hardware)    (family)      (community)       │
│                                                                              │
│   Design for the left. Guarantee the right.                                 │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2. Sync Is The Core Abstraction

Everything is sync:
- Device joins → sync to node
- Node joins cluster → sync to siblings
- Cluster connects to cluster → sync what reach allows
- Content created → sync to replication targets

### 3. Reach Determines Flow

Data flows based on relationship, not topology:
- Private data: only to my devices
- Family data: to family cluster
- Community data: to trusted clusters
- Commons: to anyone who wants it

---

## Component Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           elohim-node INTERNALS                              │
│                                                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                         API LAYER                                    │   │
│   │                                                                      │   │
│   │   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐            │   │
│   │   │  HTTP API   │    │  gRPC API   │    │  WebSocket  │            │   │
│   │   │  (mgmt)     │    │  (devices)  │    │  (realtime) │            │   │
│   │   └──────┬──────┘    └──────┬──────┘    └──────┬──────┘            │   │
│   │          └──────────────────┼──────────────────┘                    │   │
│   └──────────────────────────────┼───────────────────────────────────────┘   │
│                                  │                                           │
│   ┌──────────────────────────────┼───────────────────────────────────────┐   │
│   │                        SYNC ENGINE                                   │   │
│   │                              │                                       │   │
│   │   ┌─────────────────────────┴─────────────────────────┐             │   │
│   │   │                  SyncCoordinator                   │             │   │
│   │   │                                                    │             │   │
│   │   │  • Manages stream positions per peer               │             │   │
│   │   │  • Routes sync messages                            │             │   │
│   │   │  • Handles conflict escalation                     │             │   │
│   │   │                                                    │             │   │
│   │   └─────────────────────────┬─────────────────────────┘             │   │
│   │                             │                                        │   │
│   │   ┌────────────┐    ┌───────┴───────┐    ┌────────────┐             │   │
│   │   │ DeviceSync │    │ Automerge     │    │ClusterSync │             │   │
│   │   │            │    │    Core       │    │            │             │   │
│   │   │ phone/     │───►│              │◄───│ node-to-   │             │   │
│   │   │ laptop     │    │ CRDT merge   │    │ node       │             │   │
│   │   └────────────┘    └───────────────┘    └────────────┘             │   │
│   │                                                                      │   │
│   └──────────────────────────────────────────────────────────────────────┘   │
│                                  │                                           │
│   ┌──────────────────────────────┼───────────────────────────────────────┐   │
│   │                        CLUSTER LAYER                                 │   │
│   │                              │                                       │   │
│   │   ┌─────────────┐    ┌───────┴───────┐    ┌─────────────┐           │   │
│   │   │   mDNS      │    │  Membership   │    │   Leader    │           │   │
│   │   │  Discovery  │───►│   Manager     │◄───│  Election   │           │   │
│   │   └─────────────┘    └───────────────┘    └─────────────┘           │   │
│   │                                                                      │   │
│   └──────────────────────────────────────────────────────────────────────┘   │
│                                  │                                           │
│   ┌──────────────────────────────┼───────────────────────────────────────┐   │
│   │                         P2P LAYER                                    │   │
│   │                              │                                       │   │
│   │   ┌─────────────┐    ┌───────┴───────┐    ┌─────────────┐           │   │
│   │   │   libp2p    │    │   Protocol    │    │    NAT      │           │   │
│   │   │  Transport  │◄──►│   Handlers    │◄──►│  Traversal  │           │   │
│   │   │ QUIC/TCP/WS │    │ sync/shard    │    │ STUN/relay  │           │   │
│   │   └─────────────┘    └───────────────┘    └─────────────┘           │   │
│   │                                                                      │   │
│   └──────────────────────────────────────────────────────────────────────┘   │
│                                  │                                           │
│   ┌──────────────────────────────┼───────────────────────────────────────┐   │
│   │                       STORAGE LAYER                                  │   │
│   │                              │                                       │   │
│   │   ┌─────────────┐    ┌───────┴───────┐    ┌─────────────┐           │   │
│   │   │  Automerge  │    │     Blob      │    │    Reach    │           │   │
│   │   │   DocStore  │    │    Store      │    │   Enforcer  │           │   │
│   │   │  (SQLite)   │    │(elohim-storage)   │  (ACL)      │           │   │
│   │   └─────────────┘    └───────────────┘    └─────────────┘           │   │
│   │                                                                      │   │
│   └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Sync Engine

### Stream Positions

Every sync relationship maintains independent stream positions:

```rust
pub struct SyncState {
    /// My position in my stream (monotonic)
    pub local_position: u64,

    /// Last known position of each peer
    pub peer_positions: HashMap<PeerId, u64>,

    /// Pending events to send
    pub outbox: VecDeque<SyncEvent>,
}

pub struct SyncEvent {
    pub position: u64,
    pub doc_id: DocumentId,
    pub change_hash: ChangeHash,
    pub kind: EventKind,
    pub timestamp: u64,
}
```

### Event Kinds

```rust
pub enum EventKind {
    /// Created locally (highest priority to sync)
    Local,

    /// Just received from peer (propagate quickly)
    New,

    /// Historical catchup (lower priority)
    Backfill,

    /// Received reference before content (resolve later)
    Outlier,
}
```

### Sync Protocol

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          SYNC PROTOCOL FLOW                                  │
│                                                                              │
│   Device                           Node                                      │
│   ┌──────────────────┐            ┌──────────────────┐                      │
│   │                  │            │                  │                      │
│   │  1. Connect      │───────────►│                  │                      │
│   │                  │            │                  │                      │
│   │  2. Send my      │───────────►│  3. Compare      │                      │
│   │     position: 42 │            │     positions    │                      │
│   │                  │            │     (node: 50)   │                      │
│   │                  │            │                  │                      │
│   │                  │◄───────────│  4. Send events  │                      │
│   │                  │  43-50     │     43-50        │                      │
│   │                  │            │                  │                      │
│   │  5. For each     │            │                  │                      │
│   │     event:       │            │                  │                      │
│   │                  │───────────►│                  │                      │
│   │     DocRequest   │            │                  │                      │
│   │     (doc, heads) │            │                  │                      │
│   │                  │◄───────────│  6. DocResponse  │                      │
│   │                  │  changes   │     (changes)    │                      │
│   │                  │            │                  │                      │
│   │  7. Merge via    │            │                  │                      │
│   │     Automerge    │            │                  │                      │
│   │                  │            │                  │                      │
│   │  8. Send my      │───────────►│  9. Merge        │                      │
│   │     local events │  51-53     │     device edits │                      │
│   │                  │            │                  │                      │
│   └──────────────────┘            └──────────────────┘                      │
│                                                                              │
│   Bidirectional. Both sides send what the other is missing.                 │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Cluster Orchestration

### Discovery

Nodes in a family cluster discover each other via mDNS:

```rust
pub struct ClusterDiscovery {
    /// mDNS service name
    service_name: String,  // "_elohim-cluster._tcp.local"

    /// Shared cluster secret (for authentication)
    cluster_key: ClusterKey,

    /// Known cluster members
    members: HashMap<NodeId, NodeInfo>,
}

pub struct NodeInfo {
    pub node_id: NodeId,
    pub addresses: Vec<Multiaddr>,
    pub last_seen: Timestamp,
    pub role: NodeRole,
}

pub enum NodeRole {
    /// Primary node (handles external connections)
    Primary,
    /// Replica (full copy, can become primary)
    Replica,
    /// Observer (partial copy, read-only)
    Observer,
}
```

### Membership Protocol

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        CLUSTER MEMBERSHIP                                    │
│                                                                              │
│   New Node                    Existing Cluster                               │
│   ┌──────────────────┐       ┌──────────────────────────────────────┐       │
│   │                  │       │                                      │       │
│   │  1. Broadcast    │──────►│  Node A        Node B                │       │
│   │     "I'm here"   │ mDNS  │  ┌────┐        ┌────┐               │       │
│   │                  │       │  │    │◄──────►│    │               │       │
│   │  2. Receive      │◄──────│  │    │        │    │               │       │
│   │     cluster info │       │  └────┘        └────┘               │       │
│   │                  │       │                                      │       │
│   │  3. Authenticate │──────►│  Verify cluster_key                 │       │
│   │     with key     │       │                                      │       │
│   │                  │       │                                      │       │
│   │  4. Full sync    │◄─────►│  Replicate all data                 │       │
│   │                  │       │                                      │       │
│   │  5. Join as      │       │  Node A        Node B     New Node  │       │
│   │     replica      │       │  ┌────┐        ┌────┐     ┌────┐   │       │
│   │                  │       │  │    │◄──────►│    │◄───►│    │   │       │
│   │                  │       │  └────┘        └────┘     └────┘   │       │
│   │                  │       │                                      │       │
│   └──────────────────┘       └──────────────────────────────────────┘       │
│                                                                              │
│   Within a cluster, all nodes have full copies.                             │
│   Membership requires the shared cluster key.                               │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Leader Election

For operations requiring coordination (external device registration, cross-cluster negotiation):

```rust
pub struct LeaderElection {
    /// Current leader (if any)
    leader: Option<NodeId>,

    /// Election term
    term: u64,

    /// Last heartbeat from leader
    last_heartbeat: Timestamp,
}

// Simple leader election:
// - Lowest node_id becomes leader
// - Leader sends heartbeats
// - If heartbeat missed, next lowest becomes leader
// - No Raft complexity needed (cluster is small, trusted)
```

---

## P2P Networking

### Transport Stack

```rust
pub struct P2PTransport {
    /// libp2p swarm
    swarm: Swarm<ElohimBehaviour>,

    /// Supported transports (in preference order)
    transports: Vec<TransportType>,
}

pub enum TransportType {
    /// QUIC (preferred - fast, multiplexed, encrypted)
    Quic,
    /// TCP with Noise encryption
    TcpNoise,
    /// WebSocket (for browser compatibility)
    WebSocket,
    /// WebRTC (for NAT traversal)
    WebRtc,
}
```

### Protocol Handlers

```rust
pub enum ElohimProtocol {
    /// Sync protocol for Automerge documents
    Sync {
        protocol: "/elohim/sync/1.0.0",
        handler: SyncHandler,
    },

    /// Shard transfer for blobs
    Shard {
        protocol: "/elohim/shard/1.0.0",
        handler: ShardHandler,
    },

    /// Cluster membership
    Cluster {
        protocol: "/elohim/cluster/1.0.0",
        handler: ClusterHandler,
    },
}
```

### NAT Traversal

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         NAT TRAVERSAL STRATEGY                               │
│                                                                              │
│   1. Try direct connection (works if either has public IP)                  │
│                                                                              │
│   2. Try hole-punching via STUN                                             │
│      - Both peers contact bootstrap server                                  │
│      - Exchange observed addresses                                          │
│      - Attempt simultaneous connection                                      │
│                                                                              │
│   3. Use relay (via doorway or another node)                                │
│      - Only for initial connection                                          │
│      - Upgrade to direct when possible (DCUtR)                              │
│                                                                              │
│   4. If all else fails, relay permanently                                   │
│      - Works but higher latency                                             │
│      - Acceptable for sync (not real-time)                                  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Storage Integration

### Document Store

Automerge documents stored in SQLite:

```sql
CREATE TABLE documents (
    id TEXT PRIMARY KEY,
    automerge_data BLOB NOT NULL,
    reach TEXT NOT NULL DEFAULT 'private',
    owner TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    synced_at INTEGER
);

CREATE TABLE sync_events (
    position INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_id TEXT NOT NULL REFERENCES documents(id),
    change_hash TEXT NOT NULL,
    kind TEXT NOT NULL,
    timestamp INTEGER NOT NULL
);

CREATE INDEX idx_events_doc ON sync_events(doc_id);
CREATE INDEX idx_events_timestamp ON sync_events(timestamp);
```

### Blob Integration

Large content delegated to elohim-storage:

```rust
pub struct BlobReference {
    /// Content hash
    pub hash: ContentHash,

    /// Size in bytes
    pub size: u64,

    /// MIME type
    pub content_type: String,

    /// Reach level for replication
    pub reach: Reach,
}

// elohim-node stores the reference in Automerge doc
// elohim-storage handles the actual bytes
```

---

## API Design

### HTTP Management API

```
GET  /health                    # Node health
GET  /cluster/members           # Cluster members
POST /cluster/join              # Join cluster (with key)

GET  /sync/status               # Sync status with all peers
GET  /sync/peers                # Connected peers
POST /sync/force                # Force sync with peer

GET  /storage/stats             # Storage usage
GET  /storage/reach/{level}     # Content by reach level
```

### gRPC Device API

```protobuf
service ElohimNode {
    // Sync
    rpc Sync(stream SyncMessage) returns (stream SyncMessage);
    rpc GetDocument(GetDocumentRequest) returns (Document);
    rpc PutDocument(PutDocumentRequest) returns (PutDocumentResponse);

    // Blobs
    rpc GetBlob(GetBlobRequest) returns (stream BlobChunk);
    rpc PutBlob(stream BlobChunk) returns (PutBlobResponse);

    // Status
    rpc GetStatus(Empty) returns (NodeStatus);
}
```

### WebSocket Real-time

```typescript
// Client subscribes to document changes
ws.send({ type: 'subscribe', doc_ids: ['doc1', 'doc2'] });

// Server pushes changes
ws.onmessage = (event) => {
    const { type, doc_id, changes } = JSON.parse(event.data);
    if (type === 'change') {
        applyChanges(doc_id, changes);
    }
};
```

---

## Implementation Phases

### Phase 1: Core Sync
- [ ] Automerge document store
- [ ] Stream position tracking
- [ ] Basic sync protocol
- [ ] Single node operation

### Phase 2: Cluster
- [ ] mDNS discovery
- [ ] Cluster membership
- [ ] Intra-cluster replication
- [ ] Leader election

### Phase 3: P2P
- [ ] libp2p transport
- [ ] NAT traversal
- [ ] Cross-cluster sync
- [ ] Bootstrap integration

### Phase 4: Production
- [ ] Reach enforcement
- [ ] Device client SDK
- [ ] Monitoring/metrics
- [ ] Deployment packaging

---

## Related Documentation

- [P2P-DATAPLANE.md](../P2P-DATAPLANE.md) - Overall P2P architecture
- [SYNC-ENGINE.md](../SYNC-ENGINE.md) - Automerge sync design details
- [elohim-storage/P2P-ARCHITECTURE.md](../elohim-storage/P2P-ARCHITECTURE.md) - Blob storage P2P
- [COMMUNITY-COMPUTE.md](../COMMUNITY-COMPUTE.md) - Family node vision
