# Sprint: P2P Data Plane Architecture

## Context

We've established that the Holochain DHT cannot serve as our primary data plane:
- Chokes at ~3000 entries
- Gossip latency of 200-2000ms
- No query capability
- Not designed for content storage at scale

But we want to preserve what Holochain IS good at:
- Agent identity (keypairs, self-sovereign)
- Attestations (signed claims, witnessed events)
- Trust relationships (who trusts whom)
- Lightweight coordination

## The Vision We're Building Toward

**Community Compute Model** — See `holochain/COMMUNITY-COMPUTE.md`

- **Ephemeral sovereignty**: Your device is sovereign, but that sovereignty is made real through community
- **Offline-first native experience**: Full app on your device, sync when connected, fully distributed mesh
- **Family nodes**: Layered storage (sovereign → reciprocal → invested → gift)
- **hREA economics**: Value flows for compute contribution, constitutional rates, commons fund
- **Explainability**: Decisions are traceable, education over compliance
- **Negotiated values**: Each layer's values aligned TO its operational reality

## The Architectural Pivot

```
FROM: DHT as data plane
      (everything in Holochain, content storage, queries, sync)

TO:   DHT as coordination plane
      + Generalized P2P data plane for content
      + Local-first sync engine
      + Community replication protocol
```

## Your Task

Review the documentation in `holochain/` to understand the current architecture and vision:

1. `holochain/README.md` — Overview and principles
2. `holochain/ARCHITECTURE-GAP.md` — Client vs Agent vs Community models
3. `holochain/COMMUNITY-COMPUTE.md` — The full vision document
4. `holochain/DEPLOYMENT-RUNTIMES.md` — Current deployment patterns
5. `holochain/doorway/` — The current web gateway
6. `holochain/elohim-storage/` — The current storage layer
7. `holochain/dna/` — Current Holochain DNA structure

Then help us design the **separation of concerns** for a P2P ecosystem that achieves:

### 1. Trust Layer (Holochain DHT — keep)
What belongs here:
- Agent registration
- Attestations and witnessed events
- Trust graph
- Content location index ("who has what")
- Economic events (hREA value flows)

What does NOT belong here:
- Actual content
- Queries
- Blobs
- High-frequency operations

### 2. Data Layer (Generalized P2P — design this)
Requirements:
- Content-addressed storage (hash-based)
- Works offline (local-first)
- Syncs when connected (CRDT or merge strategies)
- Community replication (not "everyone has everything")
- Scales with community investment
- Can run on: laptops, phones, home servers, doorways

Questions to answer:
- What existing P2P protocols/frameworks could serve this? (libp2p, IPFS, custom?)
- How does content discovery work without a global DHT?
- How do we handle sync/conflict resolution?
- How do we implement "replication follows relationship"?
- What's the minimum viable data plane for 5 learning paths?

### 3. Sync Engine (Design this)
Requirements:
- Offline changes queue locally
- Reconnection triggers sync
- Conflicts resolve (CRDT, last-write-wins, manual?)
- Partial sync (only what's relevant to you)
- Works across: native app, web app, family node

### 4. Native Experience (Design this)
The app experience we want:
- Open laptop → everything works
- Make changes offline → they persist
- Reconnect → sync happens automatically
- Your device IS your server (for you)
- Community backs up what you might lose

Questions:
- What does the local storage look like? (SQLite? IndexedDB? Custom?)
- How does the native app differ from web app?
- What's the sync boundary? (What syncs, what doesn't?)
- How do we handle "your relevant data" vs "everything"?

## Frameworks to Consider

Research these for the P2P data plane:

1. **libp2p** — Modular P2P networking (used by IPFS, Filecoin)
2. **Automerge** — CRDT library for local-first apps
3. **Yjs** — CRDT for real-time collaboration
4. **OrbitDB** — P2P database on IPFS
5. **GunDB** — Decentralized graph database
6. **Hypercore/Hyperswarm** — Append-only logs + P2P discovery
7. **Matrix protocol** — Federated sync (we cloned their repos in `/research/matrix/`)
8. **Electric SQL** — Local-first sync for SQLite

Don't just pick one. Understand what each does well and what we'd need to build ourselves.

## Output Expected

1. **Separation of Concerns Diagram** — What layer handles what
2. **Technology Mapping** — Which frameworks/protocols for each layer
3. **Gap Analysis** — What exists vs what we need to build
4. **Migration Path** — How to get from current architecture to target
5. **MVP Scope** — Minimum viable P2P data plane for 5 learning paths + offline-first

## Constraints

- Must preserve agent sovereignty (keys, signatures, attestations)
- Must work offline (not "offline-tolerant", truly offline-first)
- Must support community replication (not global DHT, not single server)
- Must be explainable (decisions traceable)
- Must scale with community investment, not infrastructure spend
- Web users still need doorways (browsers can't do full P2P)
- Native apps should be first-class citizens

## The North Star

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│   Fully decentralized.                                      │
│   Scalable with community.                                  │
│   Offline-first native experience.                          │
│   Ephemeral individual sovereignty...                       │
│   ...made tangible through embodied community.              │
│                                                             │
│   Not just technically possible.                            │
│   Actually usable by regular people.                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Getting Started

Start by reading:
1. `holochain/COMMUNITY-COMPUTE.md` (the vision)
2. `holochain/ARCHITECTURE-GAP.md` (the problem)
3. `/research/matrix/` repos (how Matrix solved federation/sync)

Then propose an architecture that separates:
- Trust (Holochain)
- Data (P2P layer TBD)
- Sync (engine TBD)
- Experience (native + web)

Be opinionated. We need a clear path forward, not a survey of options.
