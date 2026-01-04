# Architecture: Technical Lay of the Land

## Directory Structure

```
holochain/
├── dna/                    # Holochain DNAs (validation + data)
│   ├── elohim/            # Main DNA: content, paths, identity
│   ├── imagodei/          # Identity and attestations
│   ├── infrastructure/    # Doorway registry, node discovery
│   └── ...
│
├── doorway/               # Web2 gateway (Rust)
│
├── elohim-storage/        # Blob storage sidecar (Rust)
│
├── holochain-cache-core/  # Performance primitives (Rust/WASM)
│
├── sdk/                   # TypeScript client
│
├── rna/                   # Schema tooling (validation, codegen)
│
├── crates/                # Shared Rust libraries
│   └── doorway-client/   # Client for doorway APIs
│
├── edgenode/             # Kubernetes deployment
│
├── manifests/            # K8s manifests
│
└── local-dev/            # Local development scripts
```

## Separation of Concerns

### Control Plane vs Data Plane

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│   CONTROL PLANE (Holochain)       DATA PLANE (elohim-storage)  │
│   ┌─────────────────────┐         ┌─────────────────────┐      │
│   │ • Who owns what     │         │ • Actual bytes      │      │
│   │ • Permissions       │         │ • Fast R/W          │      │
│   │ • Manifests/hashes  │         │ • P2P transfer      │      │
│   │ • Agent identity    │         │ • RS shards         │      │
│   │                     │         │                     │      │
│   │ SLOW but TRUSTED    │         │ FAST but SIMPLE     │      │
│   └─────────────────────┘         └─────────────────────┘      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

- **Holochain DHT**: Identity, provenance, permissions, small metadata
- **elohim-storage**: Blob storage, streaming, Reed-Solomon encoding

### Web2 Path vs P2P Path

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│           WEB2 PATH                      P2P PATH               │
│           (Doorway)                      (Agent Device)         │
│                                                                 │
│   Browser ──► Doorway ──► DHT    Agent App ──► Conductor ──► DHT│
│                  │                    │                         │
│           ProjectionCache      holochain-cache-core             │
│           (MongoDB)            elohim-storage                   │
│                                                                 │
│   For visitors without         For users with their own        │
│   their own conductor          Family Node or app              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

### dna/ - Holochain DNAs

The source of truth. DNAs define:
- Entry types (Content, Path, Human, etc.)
- Validation rules (who can do what)
- Link types (relationships between entries)
- Zome functions (the API)

**Key DNA**: `dna/elohim/` - the main application DNA

### doorway/ - Web2 Gateway

Thin bridge between browsers and the P2P network.

**Does**:
- HTTP/WebSocket translation
- Connection pooling to conductor
- Caching for performance
- Rate limiting, DDoS protection
- Bootstrap and signal services

**Does NOT**:
- Own user data
- Make authorization decisions (DNA does)
- Store authoritative content

### elohim-storage/ - Blob Storage

Sidecar process for large files.

**Does**:
- SHA256-addressed blob storage
- Reed-Solomon erasure coding (4+3 shards)
- P2P replication to trusted peers
- Local-first, network-second reads

**Sovereignty modes**:
| Mode | Replicates To | Serves |
|------|---------------|--------|
| Laptop | Trusted nodes | No one |
| HomeNode | Family | Family |
| HomeCluster | Cluster + external | Cluster |
| Network | Anyone | Anyone |

### holochain-cache-core/ - Performance Primitives

Compiled to both native Rust and WASM (for browsers).

- **WriteBuffer**: Batches writes to protect conductor from storms
- **BlobCache**: LRU cache with reach-level isolation
- **ContentResolver**: Tiered source routing (local → projection → conductor)

### sdk/ - TypeScript Client

Browser and Node.js client for the Elohim Protocol.

### rna/ - Schema Tooling

Validation and code generation for seed data.

- `hc-rna-fixtures`: CLI for validating JSON content files
- Schema definitions for Content, Path, etc.

## Data Flow

### Read Path (Content Resolution)

```
1. Check local cache (memory)      → HIT: return instantly
2. Check projection (MongoDB)      → HIT: return fast
3. Query conductor (DHT)           → Authoritative, slower
4. Cache result locally            → Future reads are fast
```

### Write Path (Batched)

```
1. Client submits write
2. WriteBuffer queues by priority (High → Normal → Bulk)
3. Background flush batches to conductor
4. Conductor validates and commits
5. Signal triggers projection update
```

### Blob Path

```
1. Check local elohim-storage      → HIT: return
2. Query P2P peers for shards      → Reconstruct if needed
3. Cache locally for next time
```

## Deployment Topologies

### Development
```
Browser → Doorway (localhost:8888) → Conductor (sandbox)
```

### Production (Single Region)
```
Browser → Doorway (doorway.elohim.host) → Conductor (K8s)
                                              ↓
                                        elohim-storage
```

### Production (Multi-Region)
```
            ┌─────────────────┐
   ┌───────►│  us-west.door   │────┐
   │        └─────────────────┘    │
   │                               ▼
GeoDNS     ┌─────────────────┐   DHT
   │       │  eu-central.door│────►
   │       └─────────────────┘    ▲
   │        ┌─────────────────┐   │
   └───────►│  ap-south.door  │───┘
            └─────────────────┘
```

### Family Network (Future)
```
┌────────────────┐     ┌────────────────┐
│  Family Node A │◄───►│  Family Node B │
│  (your house)  │     │  (grandma's)   │
└───────┬────────┘     └───────┬────────┘
        │                      │
        └──────────┬───────────┘
                   │
              Community Hub
              (church/school)
```

## Key Files

| File | Purpose |
|------|---------|
| `dna/elohim/elohim.happ` | Compiled Holochain application |
| `doorway/src/main.rs` | Gateway entry point |
| `elohim-storage/src/main.rs` | Blob storage entry point |
| `holochain-cache-core/src/lib.rs` | Cache primitives |
| `local-dev/run-conductor.sh` | Local development startup |

## Related Documentation

- [DEVELOPMENT.md](./DEVELOPMENT.md) - Local dev setup
- [DEPLOYMENT-RUNTIMES.md](./DEPLOYMENT-RUNTIMES.md) - Deployment modes
- [doorway/ARCHITECTURE.md](./doorway/ARCHITECTURE.md) - Gateway details
- [doorway/FEDERATION.md](./doorway/FEDERATION.md) - Multi-doorway topology
- [elohim-storage/EDGE-ARCHITECTURE.md](./elohim-storage/EDGE-ARCHITECTURE.md) - Performance architecture
- [elohim-storage/P2P-ARCHITECTURE.md](./elohim-storage/P2P-ARCHITECTURE.md) - Blob replication
- [dna/NETWORK_UPGRADES.md](./dna/NETWORK_UPGRADES.md) - DNA migration strategy
