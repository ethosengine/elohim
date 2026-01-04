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

For understanding the architecture:
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Technical component overview
- [ARCHITECTURE-GAP.md](./ARCHITECTURE-GAP.md) - Agentic vs client vs community compute
- [COMMUNITY-COMPUTE.md](./COMMUNITY-COMPUTE.md) - The model we're building toward
