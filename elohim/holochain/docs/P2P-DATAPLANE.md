# P2P Data Plane Architecture

## Executive Summary

The Elohim Protocol requires a clear separation between coordination and content:

- **Holochain DHT** handles coordination (identity, attestations, trust, content location)
- **P2P Data Plane** handles content (storage, replication, sync, delivery)

This separation exists because the DHT cannot scale for content storage (chokes at ~3000 entries, 200-2000ms gossip latency, no query capability) but excels at coordination where lightweight entries and cryptographic validation matter.

```
The DHT stores WHO HAS WHAT, not WHAT.
Content itself flows through the P2P data plane.
```

---

## Separation of Concerns

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           EXPERIENCE LAYER                                   │
│                                                                              │
│   ┌────────────────────────┐         ┌──────────────────────────────────┐  │
│   │     Native Apps        │         │         Web Users                 │  │
│   │  (Tauri/Electron)      │         │        (Browsers)                 │  │
│   │                        │         │                                   │  │
│   │  • Local conductor     │         │  • No conductor                   │  │
│   │  • Local storage       │         │  • Doorway as P2P bridge          │  │
│   │  • Direct P2P sync     │         │  • Caching for performance        │  │
│   └──────────┬─────────────┘         └───────────────┬──────────────────┘  │
│              │                                        │                      │
│              │ Direct P2P                             │ HTTP/WebSocket       │
│              │                                        │                      │
│              ▼                                        ▼                      │
├──────────────────────────────────────────────────────────────────────────────┤
│                            SYNC LAYER (Automerge)                            │
│                                                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  • Stream positions (monotonic sequence per agent)                   │   │
│   │  • CRDT merge for conflict resolution                                │   │
│   │  • Partial sync (only what's relevant to you)                        │   │
│   │  • Event kinds: Local, New, Backfill, Outlier                        │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│                         DATA LAYER (elohim-storage + libp2p)                 │
│                                                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  Content-Addressed Storage                                           │   │
│   │  ├── Blobs: SHA256 hash → content                                    │   │
│   │  ├── Shards: Reed-Solomon 4+3 for resilience                         │   │
│   │  ├── Manifests: Stored in DHT (hash → shard locations)               │   │
│   │  └── Reach-aware replication: private/local/neighborhood/commons     │   │
│   │                                                                      │   │
│   │  P2P Network (libp2p)                                                │   │
│   │  ├── Kademlia DHT for peer discovery                                 │   │
│   │  ├── mDNS for local cluster discovery                                │   │
│   │  ├── Request-response for shard transfer                             │   │
│   │  └── NAT traversal via relay/DCUTR                                   │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│                          TRUST LAYER (Holochain DHT)                         │
│                                                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  WHAT BELONGS HERE (100s-1000s of entries):                          │   │
│   │  ├── Agent registration (keypairs, public identifiers)               │   │
│   │  ├── Attestations (signed claims, witnessed events)                  │   │
│   │  ├── Trust graph (who trusts whom, relationships)                    │   │
│   │  ├── Content location index (hash → [peer_ids])                      │   │
│   │  ├── Economic events (hREA value flows)                              │   │
│   │  └── Doorway registry (available gateways)                           │   │
│   │                                                                      │   │
│   │  WHAT DOES NOT BELONG HERE:                                          │   │
│   │  ✗ Actual content (use data layer)                                   │   │
│   │  ✗ Queries (use projection cache)                                    │   │
│   │  ✗ Blobs (use elohim-storage)                                        │   │
│   │  ✗ High-frequency operations (use sync layer)                        │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Layer Responsibilities

### Trust Layer (Holochain DHT)

The DHT is for **coordination**, not storage. Each entry should be small (<1KB) and infrequent.

| Entry Type | Purpose | Size | Frequency |
|------------|---------|------|-----------|
| Agent Registration | Identity establishment | ~200B | Once per agent |
| Attestation | Signed claim about something | ~500B | Occasional |
| Trust Link | Who trusts whom | ~100B | Occasional |
| Content Location | Hash → peer IDs who have it | ~200B | Per content item |
| Economic Event | hREA value flow | ~300B | Per transaction |
| Doorway Registration | Gateway availability | ~500B | Per doorway |

**Total expected entries**: 100s to low 1000s — well within DHT capacity.

### Data Layer (elohim-storage + libp2p)

The data layer stores **actual content** using content-addressed storage:

```rust
// Content is stored by hash
let hash = sha256(content);
let path = format!("{}/{}", &hash[0..4], hash);
fs::write(path, content)?;
```

**Sharding Strategy**:

| Blob Size | Encoding | Shards | Rationale |
|-----------|----------|--------|-----------|
| ≤16MB | `none` | 1 | Single shard is the blob |
| 16MB–100MB | `chunked` | N sequential | Chunked for streaming |
| >100MB | `rs-4-7` | 7 (4 data + 3 parity) | Erasure coding for resilience |

**P2P Networking** uses libp2p with:
- **Kademlia**: Peer discovery across the network
- **mDNS**: Local cluster discovery (home network)
- **Request-Response**: Shard transfer protocol (`/elohim/shard/1.0.0`)
- **NAT Traversal**: Relay + DCUTR for hole-punching

### Sync Layer (Automerge)

The sync layer provides **local-first** experience with CRDT conflict resolution:

- **Stream Positions**: Each agent maintains a monotonic sequence number
- **Delta Sync**: "Give me all changes since position N"
- **CRDT Merge**: Automerge handles concurrent edits without conflicts
- **Event Kinds**: Local (I created), New (just received), Backfill (historical), Outlier (DAG gap)

See [SYNC-ENGINE.md](./SYNC-ENGINE.md) for implementation details.

### Experience Layer

**Native Apps** (Tauri):
- Local Holochain conductor
- Embedded elohim-storage
- Direct P2P sync with other native nodes
- Full offline capability

**Web Users** (via Doorway):
- No local conductor (browsers can't run it)
- Doorway proxies to P2P network
- Caching for performance
- Custodial keys for identity (exportable)

---

## Technology Choices

| Layer | Component | Technology | Rationale |
|-------|-----------|------------|-----------|
| Trust | Identity | Holochain | Agent-centric, capture-resistant |
| Trust | Content Index | Holochain DHT | Lightweight entries |
| Trust | Economics | hREA | Constitutional value flows |
| Data | Storage | elohim-storage | Already built, RS sharding |
| Data | P2P Network | libp2p | Multi-transport, NAT traversal |
| Data | Discovery | Kademlia + mDNS | Global + local |
| Sync | CRDT | Automerge 3.0 | JSON CRDT, Rust+WASM |
| Sync | Positions | Custom | Matrix-inspired streams |
| Experience | Native | Tauri | Cross-platform, Rust backend |
| Experience | Web | Doorway | Existing, stateless bridge |

### Why libp2p (Not Hyperswarm)?

- Already integrated in elohim-storage (dormant, needs activation)
- Multi-language (Rust, Go, JS) — important for ecosystem
- More transports (QUIC, WebRTC, Bluetooth, TCP, UDP)
- Better NAT traversal options (relay, DCUTR, AutoNAT)

### Alternative: Pinecone (Matrix P2P Overlay)

Pinecone (`/research/matrix/pinecone/`) is Matrix's P2P overlay routing protocol, worth serious consideration:

**Strengths:**
- **Routes by public key** — aligns with agent-centric identity
- **SNEK routing** — virtual snake topology handles mobility gracefully
- **Transport-agnostic** — TCP, WebSockets, Bluetooth LE
- **QUIC sessions** — multiplexed streams, connection migration, TLS 1.3
- **Self-healing** — alternative paths discovered automatically
- **NAT/firewall traversal** — works through restrictive networks
- **Mobile-first** — designed for topology changes

**Architecture:**
```
┌─────────────────────────────────────────────────────────────┐
│                    PINECONE TOPOLOGY                         │
│                                                              │
│   Global Spanning Tree (for bootstrap/path setup)           │
│        +                                                     │
│   Virtual Snake (linear ordering by public key)             │
│        =                                                     │
│   Efficient multi-hop routing to any public key             │
└─────────────────────────────────────────────────────────────┘
```

**Considerations:**
- Go implementation (would need Rust bindings for native elohim-storage)
- Less mature ecosystem than libp2p
- Could complement libp2p (Pinecone for routing, libp2p for content transfer)

### Why Automerge (Not Yjs)?

- JSON data model matches our content structure
- Rust core with WASM bindings
- Better for document-like data (Yjs optimized for text editing)
- Automerge 3.0 addresses historical performance issues

---

## Bootstrap Flow

Native nodes discover each other using the existing doorway signal server:

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                          P2P BOOTSTRAP FLOW                                   │
│                                                                               │
│   1. New Node Starts                                                          │
│      ┌─────────────┐                                                          │
│      │  Native App │                                                          │
│      │  (Node C)   │                                                          │
│      └──────┬──────┘                                                          │
│             │                                                                 │
│   2. Connect to Signal Server                                                 │
│             │                                                                 │
│             ▼                                                                 │
│      ┌─────────────────────────────────────────┐                             │
│      │   Doorway Signal Server                  │                             │
│      │   GET /signal/{pubkey}                   │                             │
│      │                                          │                             │
│      │   Returns: [PeerA, PeerB, ...]          │                             │
│      │   (libp2p multiaddrs)                   │                             │
│      └──────────────────────────────────────────┘                             │
│             │                                                                 │
│   3. Direct P2P Connection                                                    │
│             │                                                                 │
│             ├───────────────────────────────────┐                             │
│             ▼                                   ▼                             │
│      ┌─────────────┐                     ┌─────────────┐                      │
│      │   Node A    │◄───────────────────►│   Node B    │                      │
│      └─────────────┘    Direct P2P       └─────────────┘                      │
│             ▲                                   ▲                             │
│             │                                   │                             │
│             └───────────────────────────────────┘                             │
│                          │                                                    │
│   4. Sync Content        │                                                    │
│                          ▼                                                    │
│      ┌──────────────────────────────────────────┐                             │
│      │   Exchange stream positions               │                             │
│      │   Fetch delta content via shard protocol │                             │
│      │   Merge with Automerge                   │                             │
│      └──────────────────────────────────────────┘                             │
│                                                                               │
└──────────────────────────────────────────────────────────────────────────────┘
```

The doorway signal server (`/signal/{pubkey}` route) already exists for WebRTC signaling. We extend it for libp2p peer exchange.

---

## Replication Protocol

### Content Location Index

When a node stores content, it announces to the DHT:

```rust
// Entry in Holochain DHT
pub struct ContentLocation {
    pub content_hash: String,           // SHA256 of content
    pub holders: Vec<AgentPubKey>,      // Agents who have it
    pub reach: String,                  // Access level
    pub updated_at: Timestamp,
}
```

When a node needs content:
1. Query DHT: "Who has hash X?"
2. Get list of holders
3. Connect directly to nearest holder
4. Fetch via shard protocol

### Shard Transfer Protocol

The libp2p request-response protocol (`/elohim/shard/1.0.0`):

```rust
pub enum ShardRequest {
    Get { hash: String },                    // Request shard by hash
    Have { hash: String },                   // Check if peer has shard
    Push { hash: String, data: Vec<u8> },   // Replicate shard to peer
}

pub enum ShardResponse {
    Data(Vec<u8>),    // Shard bytes
    Have(bool),       // Yes/no
    PushAck,          // Replication acknowledged
    NotFound,
    Error(String),
}
```

### Replication Strategy

For MVP, we use simple commons-level replication:

```
When I store new "commons" content:
  1. Store locally
  2. Update ContentLocation in DHT
  3. Background: Push to N connected peers
  4. Peers update their ContentLocation entries
```

---

## Reach-Based Distribution

Reach controls who can access and replicate content:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        REACH → DISTRIBUTION                                  │
│                                                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  private ─────► Only my devices                                      │   │
│   │                 • Encrypted with device key                          │   │
│   │                 • Never replicates to others                         │   │
│   │                 • Stored locally only                                │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  invited ─────► Explicitly named agents                              │   │
│   │                 • Encrypted with per-relationship key                │   │
│   │                 • Replicates only to named agents                    │   │
│   │                 • Requires trust link in DHT                         │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  local ───────► Family cluster                                       │   │
│   │                 • Encrypted with cluster key                         │   │
│   │                 • Replicates to family nodes                         │   │
│   │                 • Discovered via mDNS                                │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  neighborhood ► Extended trust network                               │   │
│   │  municipal ───► Community level                                      │   │
│   │  bioregional ─► Regional level                                       │   │
│   │                 • Not encrypted at rest                              │   │
│   │                 • Replicates to trust-connected peers                │   │
│   │                 • Reach gates delivery                               │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  commons ─────► Anyone willing to store                              │   │
│   │                 • Not encrypted                                      │   │
│   │                 • Replicates freely                                  │   │
│   │                 • Anyone can serve                                   │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

See [elohim-storage/REACH.md](./elohim-storage/REACH.md) for implementation details.

---

## MVP Scope

### What We Build First

1. **Activate P2P in elohim-storage**
   - Enable dormant libp2p code
   - Wire Kademlia + mDNS
   - Implement shard transfer

2. **ContentLocation DHT entries**
   - New entry type in infrastructure DNA
   - Announce when storing
   - Query when fetching

3. **Signal server bootstrap**
   - Extend `/signal/{pubkey}` for libp2p
   - Return peer multiaddrs
   - Enable direct connection

4. **Automerge sync**
   - Add dependency to elohim-storage
   - Content → Automerge doc
   - Stream position tracking

5. **Tauri foundation**
   - Project structure only
   - Embed elohim-storage
   - Basic conductor shell

### Success Criteria

1. **Two native nodes sync via P2P**
   - Node A seeds content
   - Node B discovers A via signal server
   - Node B replicates from A directly

2. **Content survives node failure**
   - Node A stores, replicates to B
   - Node A goes offline
   - Node B still has content

3. **Conflicts merge cleanly**
   - Both nodes edit offline
   - Reconnect
   - Automerge resolves

---

## Related Documentation

| Document | Purpose |
|----------|---------|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | Component overview |
| [COMMUNITY-COMPUTE.md](./COMMUNITY-COMPUTE.md) | Vision: Community-scaled compute |
| [ARCHITECTURE-GAP.md](./ARCHITECTURE-GAP.md) | Why DHT alone doesn't work |
| [SYNC-ENGINE.md](./SYNC-ENGINE.md) | Automerge integration design |
| [doorway/FEDERATION.md](./doorway/FEDERATION.md) | Cross-doorway communication |
| [elohim-storage/P2P-ARCHITECTURE.md](./elohim-storage/P2P-ARCHITECTURE.md) | Storage P2P implementation |
| [elohim-storage/REACH.md](./elohim-storage/REACH.md) | Reach enforcement |
