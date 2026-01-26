# elohim-storage: P2P Architecture

> **See also**:
> - [P2P-DATAPLANE.md](../P2P-DATAPLANE.md) - Overall architecture vision
> - [SYNC-ENGINE.md](../SYNC-ENGINE.md) - Automerge sync design

## The Problem: Holochain DHT Performance

Holochain's DHT is designed for provenance and validation, not bulk data transfer. Every DHT write involves:

1. Serialize entry
2. Hash + sign
3. Write to source chain (SQLite)
4. Validate (run WASM)
5. Publish to DHT peers
6. Wait for validation receipts

**Result**: For 1000 content items, this takes minutes and can choke the conductor. For video blobs, it's completely impractical.

## Dual-Plane Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│   CONTROL PLANE (Holochain)          DATA PLANE (elohim-storage)       │
│   ┌─────────────────────┐            ┌─────────────────────┐           │
│   │ • Who owns what     │            │ • Actual bytes      │           │
│   │ • Permissions       │            │ • Fast R/W          │           │
│   │ • Manifests/hashes  │            │ • P2P transfer      │           │
│   │ • Contracts         │            │ • RS shards         │           │
│   │ • Agent identity    │            │ • Streaming         │           │
│   │                     │            │                     │           │
│   │ SLOW but TRUSTED    │            │ FAST but SIMPLE     │           │
│   └─────────────────────┘            └─────────────────────┘           │
│              │                                  │                       │
│              └──── manifest points to ──────────┘                       │
└─────────────────────────────────────────────────────────────────────────┘
```

**Holochain handles**: Identity, provenance, permissions, contracts, small metadata
**elohim-storage handles**: Blob storage, transfer, RS encoding, streaming

## Where Doorway Fits

Doorway is a **thin Web 2.0 bridge** - it translates HTTP/WebSocket to the P2P-native application.

**Think Cloudflare, not AWS.** It's an edge layer that protects and bridges, Fediverse patterns live here,
but unlike the fediverse, this in not the infrastructure where data lives.


| | Doorway | elohim-storage |
|---|---------|----------------|
| **Analogy** | Cloudflare | Decentralized S3 |
| **Role** | Edge, protection, caching | Actual storage infrastructure |
| **Data lives here?** | No (cache only) | Yes (authoritative) |
| **Part of P2P network?** | No | Yes |

**Doorway CAN have a read cache** - but its purpose is to:
- Protect the P2P network from Web 2.0 traffic stampedes
- Warm content based on Holochain contracts (e.g., "popular in region X")
- Shield P2P natives from noisy web2 clients

**Doorway is NOT**:
- The authoritative storage layer
- Part of the P2P network itself
- Where P2P natives get their data

## Complementary Caching (Separate Concerns)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    TWO CACHES, TWO PURPOSES                            │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   WEB 2.0 WORLD                      P2P NATIVE WORLD                  │
│                                                                         │
│   ┌─────────────┐                    ┌─────────────────────────────┐   │
│   │  Doorway    │                    │     elohim-storage          │   │
│   │  + cache    │                    │     + local cache           │   │
│   └──────┬──────┘                    └──────────────┬──────────────┘   │
│          │                                          │                   │
│   Purpose:                           Purpose:                          │
│   • Shield P2P from                  • Blazing fast for natives       │
│     web2 stampedes                   • First-class P2P experience     │
│   • Warm based on                    • Local-first, network-second    │
│     Holochain contracts              • Part of the P2P network        │
│   • Protect native                                                     │
│     experience                                                         │
│          │                                          │                   │
│          └──────────────────┬───────────────────────┘                  │
│                             │                                           │
│                             ▼                                           │
│                  ┌─────────────────────┐                               │
│                  │   P2P Storage       │                               │
│                  │   Network           │                               │
│                  │   (the real thing)  │                               │
│                  └─────────────────────┘                               │
│                                                                         │
│   Complementary, but separate concerns.                                │
│   Web2 cache protects P2P. P2P cache serves natives.                  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                         │
│   Web 2.0 World              │           P2P Native World              │
│                              │                                          │
│   Browser/App                │                                          │
│       │                      │                                          │
│       │ HTTP/WS              │                                          │
│       ▼                      │                                          │
│   ┌─────────┐                │   ┌─────────────────────────────────┐   │
│   │ Doorway │ ◄──────────────┼──►│        elohim-storage           │   │
│   │ (thin   │   bridge       │   │   (true P2P storage network)    │   │
│   │ bridge) │                │   │                                 │   │
│   └─────────┘                │   │   • Peer discovery (no DNS)     │   │
│                              │   │   • Direct node-to-node         │   │
│                              │   │   • RS shard distribution       │   │
│                              │   │   • Content routing             │   │
│                              │   │   • NAT traversal               │   │
│                              │   └─────────────────────────────────┘   │
│                              │                 │                        │
│                              │                 │ provenance only        │
│                              │                 ▼                        │
│                              │   ┌─────────────────────────────────┐   │
│                              │   │          Holochain DHT          │   │
│                              │   │   (identity, permissions,       │   │
│                              │   │    manifests, contracts)        │   │
│                              │   └─────────────────────────────────┘   │
│                              │                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

## P2P Performance Reality

### The Read Question: 1000 clients reading from one node

P2P networks are optimized for:
- **Resilience** (data survives node failure)
- **Distribution** (spread load across many nodes)
- **Censorship resistance**

P2P networks are NOT optimized for:
- Single-node hot-spot serving (1 node, 1000 clients)
- Low-latency first-byte (discovery overhead)
- Predictable performance (network variability)

**Solution**: Tiered storage with local caching

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    READ PATH                                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   1. Check local cache (memory)     → HIT: return instantly            │
│   2. Check local disk               → HIT: return fast                 │
│   3. Query P2P network              → SLOW but resilient               │
│   4. Cache result locally           → Future reads are fast            │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### The Write Question: 1000 concurrent writes

For each P2P write, a naive implementation does:

| Step | Speed |
|------|-------|
| Hash content | Fast (~10,000+/sec) |
| Write to local disk | Fast (~10,000+/sec) |
| Announce to DHT | Slow (~100ms RTT) |
| Replicate to N peers | Slow (~seconds) |
| Wait for confirmations | Slow (~seconds) |

If DHT announce and replication are synchronous: **~10 writes/sec max**

**Solution**: Async replication

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    WRITE-OPTIMIZED ARCHITECTURE                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   SYNC PATH (must be fast):                                            │
│   ┌──────────────────────────────────────────────────────────────┐     │
│   │   Client → elohim-storage (LOCAL WRITE ONLY)                 │     │
│   │                    │                                          │     │
│   │               hash + disk                                    │     │
│   │                    │                                          │     │
│   │               return hash immediately                        │     │
│   └──────────────────────────────────────────────────────────────┘     │
│                                                                         │
│   ASYNC PATH (can be slow):                                            │
│   ┌──────────────────────────────────────────────────────────────┐     │
│   │   Background worker picks up new blobs                       │     │
│   │        │                                                      │     │
│   │        ├── RS encode (if large)                              │     │
│   │        ├── Replicate to N peers                              │     │
│   │        ├── Announce to DHT                                   │     │
│   │        └── Update Holochain manifest                         │     │
│   │                                                               │     │
│   │   Takes seconds/minutes, user doesn't wait                   │     │
│   └──────────────────────────────────────────────────────────────┘     │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## The Durability Tradeoff

Async replication creates a window where data exists on one node but isn't replicated yet.

**Risk**: Node dies before replication completes → data lost

**Mitigations**:

1. **Fast replication** - Prioritize recent writes
2. **Write-ahead log** - Persist replication queue to disk
3. **Multi-node write** - Client writes to 2+ storage nodes
4. **Acknowledgment tiers**:
   ```
   "accepted" = local write done (instant)
   "durable"  = replicated to N peers (seconds)
   "verified" = Holochain manifest committed (minutes)
   ```

## Storage Tiers

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    elohim-storage LAYERS                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   ┌──────────────────────────────────────────────────────────────┐     │
│   │  HOT LAYER: Local BlobStore                                   │     │
│   │  • Handles all writes (sync)                                 │     │
│   │  • Handles most reads (cache)                                │     │
│   │  • ~10,000+ ops/sec                                          │     │
│   └──────────────────────────────────────────────────────────────┘     │
│                          │                                              │
│                   async replication                                     │
│                          ▼                                              │
│   ┌──────────────────────────────────────────────────────────────┐     │
│   │  COLD LAYER: P2P Network                                      │     │
│   │  • Receives replicated shards                                │     │
│   │  • Serves cache misses                                       │     │
│   │  • Provides resilience                                       │     │
│   │  • ~100 ops/sec (fine, it's async)                          │     │
│   └──────────────────────────────────────────────────────────────┘     │
│                          │                                              │
│                   manifest only                                         │
│                          ▼                                              │
│   ┌──────────────────────────────────────────────────────────────┐     │
│   │  PROVENANCE: Holochain DHT                                   │     │
│   │  • Tiny manifests (hash, owner, permissions)                │     │
│   │  • ~10 ops/sec (fine, small data)                           │     │
│   └──────────────────────────────────────────────────────────────┘     │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## P2P Layer: libp2p

**Decision**: Use libp2p (already integrated, dormant).

```rust
use libp2p::{
    identity, PeerId, Swarm,
    kad::Kademlia,           // DHT for peer/content discovery
    request_response,        // Shard transfer
    relay,                   // NAT traversal
    dcutr,                   // Direct connection upgrade
    mdns,                    // Local network discovery
};
```

**Why libp2p**:
- Same stack as Holochain (consistent ecosystem)
- Already integrated in elohim-storage (needs activation)
- Multi-transport: QUIC, WebRTC, TCP, UDP
- NAT traversal: relay, DCUTR, AutoNAT
- Multi-language: Rust, Go, JS

### Bootstrap via Signal Server

Nodes discover each other using the existing doorway signal server:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    PEER DISCOVERY FLOW                                       │
│                                                                              │
│   1. New node starts                                                         │
│      ┌─────────────┐                                                         │
│      │  Native App │                                                         │
│      └──────┬──────┘                                                         │
│             │                                                                │
│   2. Connect to signal server                                                │
│             │                                                                │
│             ▼                                                                │
│      ┌──────────────────────────────────────┐                               │
│      │  Doorway Signal Server               │                               │
│      │  GET /signal/{agent_pubkey}          │                               │
│      │                                      │                               │
│      │  Returns: multiaddrs of known peers  │                               │
│      └──────────────────────────────────────┘                               │
│             │                                                                │
│   3. Direct P2P connection                                                   │
│             │                                                                │
│             ├───────────────────────────────┐                                │
│             ▼                               ▼                                │
│      ┌─────────────┐                 ┌─────────────┐                        │
│      │   Node A    │◄───────────────►│   Node B    │                        │
│      └─────────────┘    libp2p       └─────────────┘                        │
│                                                                              │
│   4. After initial connection, use Kademlia DHT for discovery               │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

The doorway already has `/signal/{pubkey}` for WebRTC signaling. We extend it for libp2p peer exchange.

### Content Discovery via DHT

Holochain DHT stores content location entries:

```rust
// Entry in Holochain DHT (infrastructure DNA)
#[hdk_entry]
pub struct ContentLocation {
    pub content_hash: String,           // SHA256 of content
    pub holders: Vec<AgentPubKey>,      // Agents who have it
    pub reach: String,                  // Access level
    pub updated_at: Timestamp,
}
```

When fetching content:
1. Query Holochain DHT: "Who has hash X?"
2. Get list of holder agents
3. Look up their P2P multiaddrs
4. Connect via libp2p and fetch shard

## Reed-Solomon Distribution

Current state: RS encoding is implemented in `sharding.rs` but shards are stored locally.

For true distributed resilience:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    RS-P2P DISTRIBUTION                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│    Creator Node                  Other Nodes (DHT-based placement)     │
│    ┌──────────┐                  ┌───────┐ ┌───────┐ ┌───────┐        │
│    │ Original │ ── RS encode ──► │Shard 1│ │Shard 3│ │Shard 5│        │
│    │   blob   │     (rs-4-7)     └───────┘ └───────┘ └───────┘        │
│    └──────────┘                  ┌───────┐ ┌───────┐ ┌───────┐        │
│         │                        │Shard 2│ │Shard 4│ │Shard 6│        │
│         ▼                        └───────┘ └───────┘ └───────┘        │
│  ┌─────────────┐                                       ┌───────┐       │
│  │ Manifest    │ ── stored in Holochain DHT ──────────│Shard 7│       │
│  │ (pointers)  │                                       └───────┘       │
│  └─────────────┘                                                       │
│                                                                         │
│    rs-4-7: Any 4 of 7 shards → full reconstruction                    │
│    Creator can go offline → data survives                             │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## Identity Bridge

Linking P2P identity to Holochain identity:

```rust
struct ElohimStorageNode {
    // P2P identity (ed25519 keypair)
    p2p_node_id: PeerId,

    // Holochain identity
    agent_pubkey: AgentPubKey,
}

// Registered in Holochain DHT:
#[hdk_entry]
struct StorageNodeRegistration {
    agent: AgentPubKey,
    p2p_node_id: String,
    capabilities: Vec<String>,
    announced_at: Timestamp,
}
```

This allows:
- Holochain knows which P2P nodes belong to which agents
- Permissions/provenance flow from Holochain
- Actual bytes flow through P2P network

## Summary

| Layer | Purpose | Performance |
|-------|---------|-------------|
| **Doorway** | Web 2.0 bridge (thin) | N/A - just proxies |
| **Local BlobStore** | Hot storage, writes | ~10,000 ops/sec |
| **P2P Network** | Replication, resilience | ~100 ops/sec (async) |
| **Holochain DHT** | Provenance, permissions | ~10 ops/sec (small data) |

**Key principle**: P2P is for **durability and distribution**, not **performance**. Fast paths stay local; P2P happens in the background.

---

## Automerge Sync Integration

Content metadata syncs via Automerge CRDT (see [SYNC-ENGINE.md](../SYNC-ENGINE.md) for details):

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    elohim-storage + Sync Engine                              │
│                                                                              │
│   ┌─────────────────────────┐      ┌─────────────────────────┐              │
│   │     Blob Storage        │      │     Sync Engine         │              │
│   │                         │      │     (Automerge)         │              │
│   │  • Large files (media)  │      │                         │              │
│   │  • RS shards            │      │  • Content metadata     │              │
│   │  • Shard protocol       │      │  • Path definitions     │              │
│   │                         │      │  • User progress        │              │
│   └────────────┬────────────┘      └────────────┬────────────┘              │
│                │                                 │                           │
│                │ refs blobs by hash              │ syncs docs via CRDT      │
│                └─────────────────────────────────┤                           │
│                                                  │                           │
│   ┌──────────────────────────────────────────────┴────────────┐              │
│   │                    P2P Network (libp2p)                    │              │
│   │                                                            │              │
│   │   /elohim/shard/1.0.0  - Blob shard transfer              │              │
│   │   /elohim/sync/1.0.0   - Automerge sync protocol          │              │
│   │                                                            │              │
│   └────────────────────────────────────────────────────────────┘              │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### What Goes Where

| Data Type | Storage | Transport |
|-----------|---------|-----------|
| Media files | Blob (RS shards) | Shard protocol |
| Content metadata | Automerge doc | Sync protocol |
| Path definitions | Automerge doc | Sync protocol |
| User progress | Automerge doc | Sync protocol |
| Attestations | Holochain DHT | Holochain gossip |
| Content location | Holochain DHT | Holochain gossip |

---

## Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| Blob storage | ✅ Complete | `blob_store.rs` |
| RS sharding | ✅ Complete | `sharding.rs` |
| libp2p foundation | ⚠️ Dormant | `p2p/mod.rs` - needs activation |
| Shard protocol | ⚠️ Dormant | `p2p/shard_protocol.rs` - needs wiring |
| Signal server bootstrap | ❌ Missing | Extend doorway `/signal` |
| ContentLocation DHT | ❌ Missing | New entry in infrastructure DNA |
| Automerge sync | ❌ Missing | Add automerge dependency |
| Replication worker | ❌ Missing | Background shard distribution |

---

## Related Documentation

- [P2P-DATAPLANE.md](../P2P-DATAPLANE.md) - Overall P2P architecture
- [SYNC-ENGINE.md](../SYNC-ENGINE.md) - Automerge sync design
- [REACH.md](./REACH.md) - Reach-based access control
