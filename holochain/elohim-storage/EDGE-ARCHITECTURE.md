# Elohim Edge Architecture

This document describes the performance architecture for Elohim Protocol's Holochain infrastructure. The key insight: **P2P performance happens at the agent level, not at the gateway.**

## Core Principle

```
Doorway is like Cloudflare - it doesn't define what domains you bring to it.
Agents configure doorway, not the other way around.
```

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    External Web Clients (browsers)                       │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    DOORWAY (thin web2 gateway)                          │
│                                                                         │
│  What it IS:                         What it is NOT:                    │
│  ✓ Web2 gateway (like Cloudflare)    ✗ Primary caching layer            │
│  ✓ DDoS protection                   ✗ Primary write buffer             │
│  ✓ Rate limiting                     ✗ Hard-coded DNA routes            │
│  ✓ Auth for external clients         ✗ Required for agent performance   │
│  ✓ Optional CDN extension                                               │
│  ✓ Configured BY agents                                                 │
│                                                                         │
│  Routes: Discovered from DNAs, not hard-coded                           │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │ (optional CDN/proxy)
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│              AGENT DEVICE (laptop, edgenode, phone)                     │
│              ═══════════════════════════════════════                    │
│              PRIMARY P2P PERFORMANCE LAYER                              │
│                                                                         │
│   ┌─────────────────────────────────────────────────────────────────┐  │
│   │  holochain-cache-core (Rust/WASM)                               │  │
│   │                                                                 │  │
│   │  WriteBuffer ────────────────────────────────────────────────   │  │
│   │  • Batches writes to protect conductor from write pressure      │  │
│   │  • Priority queues (High → Normal → Bulk)                       │  │
│   │  • Deduplication (last-write-wins)                              │  │
│   │  • Backpressure signaling                                       │  │
│   │  • Presets: for_seeding(), for_interactive(), for_recovery()    │  │
│   │                                                                 │  │
│   │  BlobCache ──────────────────────────────────────────────────   │  │
│   │  • LRU cache with O(log n) eviction                             │  │
│   │  • Reach-level isolation (private → commons)                    │  │
│   │  • Mastery-based freshness decay                                │  │
│   │                                                                 │  │
│   │  ContentResolver ────────────────────────────────────────────   │  │
│   │  • Tiered source resolution (Local → Projection → Authority)    │  │
│   │  • Learns from successful resolutions                           │  │
│   │                                                                 │  │
│   │  = Web 2.0 native performance in P2P deployment                 │  │
│   └─────────────────────────────────────────────────────────────────┘  │
│                                                                         │
│   ┌─────────────────────────────────────────────────────────────────┐  │
│   │  elohim-storage (sidecar process)                               │  │
│   │                                                                 │  │
│   │  • SHA256-addressed blob store                                  │  │
│   │  • Reed-Solomon erasure coding (4+3 shards)                     │  │
│   │  • Unified shard model (every blob has manifest)                │  │
│   │  • Local filesystem + sled metadata                             │  │
│   │                                                                 │  │
│   │  = Durable blob storage with network resilience                 │  │
│   └─────────────────────────────────────────────────────────────────┘  │
│                                                                         │
│   ┌─────────────────────────────────────────────────────────────────┐  │
│   │  Holochain Conductor                                            │  │
│   │                                                                 │  │
│   │  • Protected by WriteBuffer batching                            │  │
│   │  • Receives controlled flow of DHT writes                       │  │
│   │  • Metadata entries with blob_hash references                   │  │
│   └─────────────────────────────────────────────────────────────────┘  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

### holochain-cache-core

**Location**: `holochain/holochain-cache-core/`

The foundational performance library compiled to both native Rust and WASM. Provides web 2.0 native performance for P2P agents.

| Component | Purpose | Key Feature |
|-----------|---------|-------------|
| `WriteBuffer` | Batch writes to conductor | Protects conductor from write storms |
| `BlobCache` | LRU blob caching | Reach-aware isolation |
| `ChunkCache` | TTL-based streaming cache | For chunked media |
| `ReachAwareCache` | 8 isolated caches by reach | Prevents private evicting commons |
| `ContentResolver` | Tiered content resolution | Learns from success |

**Compilation targets**:
- Native Rust (for edgenode, doorway)
- WebAssembly (for browser clients)

### elohim-storage

**Location**: `holochain/elohim-storage/`

Blob storage sidecar running alongside Holochain conductor on agent devices.

| Component | Purpose |
|-----------|---------|
| `BlobStore` | SHA256-addressed content storage |
| `ShardEncoder` | Reed-Solomon erasure coding |
| `ShardManifest` | Unified blob metadata |

**Key design**: Every blob has a manifest, whether single-shard or distributed.

### doorway

**Location**: `holochain/doorway/`

Thin web2 gateway. Think Cloudflare for Holochain.

**Should do**:
- DDoS protection
- Rate limiting
- External client authentication
- Optional CDN caching (extends, doesn't replace agent cache)
- Route proxy configured by agents

**Should NOT do**:
- Primary caching (that's agent's job)
- Primary write buffering (that's agent's job)
- Hard-coded DNA-specific routes

### Route Registration Protocol

Doorway routes should be **declared by DNAs**, not hard-coded. This follows the existing `__doorway_import_config` pattern.

```rust
// DNA zome function (example)
#[hdk_extern]
fn __doorway_routes(_: ()) -> ExternResult<RouteConfig> {
    Ok(RouteConfig {
        routes: vec![
            Route {
                path: "/api/content/{id}",
                methods: vec!["GET"],
                handler: "get_content",
                cache_ttl: Some(3600),
            },
            Route {
                path: "/api/content",
                methods: vec!["POST"],
                handler: "create_content",
                auth_required: true,
            },
        ],
        blob_proxy: Some(BlobProxyConfig {
            enabled: true,
            base_path: "/store",
            // Doorway caches, but agent's elohim-storage is authoritative
        }),
    })
}
```

Doorway discovers routes from installed DNAs and configures itself accordingly.

## Seeder: Proof of P2P Performance

**Location**: `genesis/seeder/`

The seeder is an admin tool that proves an edgenode can handle heavy multimedia load without threatening the conductor.

### Purpose

1. Bootstrap a "seed node" agent with content
2. Performance test: proves holochain-cache-core + elohim-storage can handle write storms
3. Validates the P2P performance architecture

### Correct Flow

```
┌──────────────────────────────────────────────────────────────────────────┐
│                         SEEDER (admin tool)                              │
│                                                                          │
│  What it tests:                                                          │
│  • WriteBuffer can batch heavy write load                                │
│  • elohim-storage can absorb blob upload pressure                        │
│  • Conductor remains healthy under controlled write flow                 │
└──────────────────────────────┬───────────────────────────────────────────┘
                               │
                               │ Direct connection (NOT through doorway)
                               ▼
┌──────────────────────────────────────────────────────────────────────────┐
│              SEED NODE AGENT (edgenode)                                  │
│                                                                          │
│   Seeder → elohim-storage HTTP API ──────────────────→ Blob storage     │
│   Seeder → WriteBuffer → Conductor ──────────────────→ DHT entries      │
│                                                                          │
│   = PROVES: Edgenode handles multimedia + writes WITHOUT doorway        │
└──────────────────────────────────────────────────────────────────────────┘
```

### What Seeder Should NOT Do

- Go through doorway for blob uploads (tests wrong layer)
- Bypass WriteBuffer (defeats purpose of testing batch pattern)
- Hard-code doorway routes

## Migration Path

### Phase 1: Document & Clarify (Current)
- [x] Document corrected architecture (this file)
- [ ] Review what's in doorway that should move

### Phase 2: Remove WriteBuffer from Doorway
- Doorway's `DoorwayWriteBuffer` wraps `holochain-cache-core::WriteBuffer`
- This wrapper should be removed; agents use cache-core directly
- Doorway can proxy write requests to agent's WriteBuffer endpoint

### Phase 3: Dynamic Route Registration
- Extend `__doorway_import_config` pattern to all routes
- Remove hard-coded routes from doorway:
  - `/admin/seed/blob` → Agent's elohim-storage
  - `/store/{hash}` → Proxied to agent, cached optionally
  - `/api/v1/cache/` → Agent's cache-core
  - `/api/blob/verify` → Agent's verification
  - `/api/stream/` → Proxied to agent

### Phase 4: Update Seeder
- Seeder connects directly to agent's elohim-storage
- Seeder uses agent's WriteBuffer (via cache-core)
- Remove dependency on doorway routes

## Hash Encoding Convention

All SHA256 hashes use **hex encoding** with `sha256-` prefix:

```
sha256-6faf5b87cb250b09843cd58e9a2839ff3243dfe239e20a1bea96e8113fea9380
```

This applies to:
- Blob hashes in elohim-storage
- Content references in DNA
- Cache keys in holochain-cache-core
- Seeder blob management

Do NOT use base64 encoding for content hashes (base64 is fine for other things like public keys).
