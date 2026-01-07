# Elohim Protocol: Holochain Infrastructure

## Why Holochain?

The Elohim Protocol requires infrastructure that humans can own. Not rent. Own.

Cloud services create dependency. Dependency creates extraction. Extraction erodes sovereignty. Holochain breaks this pattern by enabling truly peer-to-peer applications where:

- **Data lives with its owners** - Your family's data on your family's hardware
- **Identity is cryptographic** - No platform can lock you out of your own identity
- **Validation is distributed** - Rules enforced by math, not corporate policy
- **Networks heal themselves** - No single point of failure

## The Vision: Progressive Sovereignty

People shouldn't need to understand cryptography to own their digital lives. The Elohim Protocol meets people where they are:

```
Stage 1: Visitor     → Browse public content via any doorway
Stage 2: Hosted      → Account on elohim.host (custodial, but exportable)
Stage 3: App User    → Desktop app with local keys (self-sovereign)
Stage 4: Node Op     → Family Node serving your household and community
```

Each stage preserves identity, content, and reputation. No lock-in at any level.

## The Hardware Vision

Stage 4 is the destination: a Family Node in your home. Think of it as:

- **Your family's private cloud** - Photos, documents, learning progress
- **A backup node for relatives** - Grandma's data replicated to your hardware
- **A community service** - Serve public content to neighbors
- **Regional resilience** - Geographic redundancy through relationships

Multiple Family Nodes form clusters. Clusters form regional networks. Regional networks form a global mesh. CDN-like distribution emerges from relationships, not corporations.

## What This Directory Contains

This is the P2P infrastructure layer. Everything here exists to make the vision above real:

| Component | Purpose |
|-----------|---------|
| `dna/` | Holochain DNAs - the validation rules and data structures |
| `doorway/` | Web2 gateway - bridges browsers to the P2P network |
| `elohim-storage/` | Blob storage - media files, Reed-Solomon shards |
| `holochain-cache-core/` | Performance primitives - caching, write buffering |
| `edgenode/` | Deployment packaging for Kubernetes |
| `sdk/` | TypeScript client for browser/Node.js |

See [ARCHITECTURE.md](./ARCHITECTURE.md) for technical details.

## Key Principles

**1. Doorway is Cloudflare, not AWS**

Doorway is a thin web2 bridge. It caches and protects, but doesn't own. Users can switch doorways freely. Data lives in the DHT.

**2. Performance happens at the agent level**

The Family Node runs `holochain-cache-core` and `elohim-storage`. These provide web2-like performance on P2P infrastructure. Doorway is optional for P2P-native clients.

**3. Relationships drive replication**

Your data replicates to people you trust: family, church, neighborhood. Not to anonymous cloud regions. Backup is a social contract, not a billing tier.

**4. The DNA is law**

Validation rules in the DNA are cryptographically enforced. No doorway, no admin, no corporation can override them. This is what makes sovereignty real.

## The Gap We're Bridging

The Holochain DHT promises automatic distributed storage, but in practice:
- Chokes at ~3000 entries
- Gossip latency of 200-2000ms
- No query capability

We're building the **Community Compute Model** - a hybrid that preserves sovereignty while actually working at scale. See:

- [ARCHITECTURE-GAP.md](./ARCHITECTURE-GAP.md) - Why pure agent-centric doesn't scale
- [COMMUNITY-COMPUTE.md](./COMMUNITY-COMPUTE.md) - The family node and community replication model

**Key insight:** The DHT stores *coordination* (who exists, who trusts whom, where content lives), not *content*. Content lives in `elohim-storage`, replicated by community investment.

## Getting Started

For development setup, see [DEVELOPMENT.md](./DEVELOPMENT.md).

For deployment options, see [DEPLOYMENT-RUNTIMES.md](./DEPLOYMENT-RUNTIMES.md).

## Documentation Guide

### Architecture Vision

| Document | Purpose |
|----------|---------|
| [P2P-DATAPLANE.md](./P2P-DATAPLANE.md) | **Start here** - Master P2P architecture, layer separation, technology choices |
| [COMMUNITY-COMPUTE.md](./COMMUNITY-COMPUTE.md) | Vision: family nodes, community replication, sovereignty model |
| [ARCHITECTURE-GAP.md](./ARCHITECTURE-GAP.md) | Why pure agent-centric DHT doesn't scale |
| [SYNC-ENGINE.md](./SYNC-ENGINE.md) | Automerge CRDT sync design, stream positions |

### Technical Reference

| Document | Purpose |
|----------|---------|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | Component overview, data flow, deployment topologies |
| [elohim-node/ARCHITECTURE.md](../elohim-node/ARCHITECTURE.md) | Infrastructure runtime: sync, cluster, P2P |
| [doorway/FEDERATION.md](./doorway/FEDERATION.md) | Doorway federation, DIDs, P2P bootstrap role |
| [doorway/ARCHITECTURE.md](./doorway/ARCHITECTURE.md) | Doorway internals, routes, caching |
| [elohim-storage/P2P-ARCHITECTURE.md](./elohim-storage/P2P-ARCHITECTURE.md) | Storage P2P implementation, shard protocol |

### Implementation Guides

| Document | Purpose |
|----------|---------|
| [DEVELOPMENT.md](./DEVELOPMENT.md) | Local development setup |
| [DEPLOYMENT-RUNTIMES.md](./DEPLOYMENT-RUNTIMES.md) | Deployment modes and options |
| [dna/NETWORK_UPGRADES.md](./dna/NETWORK_UPGRADES.md) | DNA migration strategy |

### Reading Order (Recommended)

1. **P2P-DATAPLANE.md** - Understand the 4-layer architecture
2. **COMMUNITY-COMPUTE.md** - Understand the vision and values
3. **SYNC-ENGINE.md** - Understand how sync works
4. **ARCHITECTURE.md** - Understand the components
5. **doorway/FEDERATION.md** - Understand doorway's role
