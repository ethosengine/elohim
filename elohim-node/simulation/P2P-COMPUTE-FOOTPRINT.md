# P2P Compute Footprint: 20 Humans × 3 Schema Versions

Analysis of what it takes to move from client/server seeding to true P2P dynamics
for the 27 humans modeled in `genesis/a2o`.

## Current State: Client/Server

The genesis pipeline is fundamentally centralized:

```
27 humans in JSON → seeder → POST /api/db/*/bulk → 1 doorway → 1 elohim-storage (SQLite)
```

All humans share one conductor, one storage database, one SQLite file scoped by `app_id`.
The simulation compose models **infrastructure nodes** (2 clusters × 2 nodes), not **human agents**.
No human in genesis actually runs their own conductor or storage instance.

## Per-Human Process Stack (Full P2P)

| Process | Memory (req/limit) | CPU (req/limit) | Disk |
|---------|-------------------|-----------------|------|
| Holochain Conductor | 1 Gi / 4 Gi | 500m / 2000m | 10 Gi |
| elohim-storage sidecar | 128 Mi / 512 Mi | 100m / 500m | 5 Gi |
| Socat proxy | 16 Mi / 64 Mi | 10m / 100m | — |
| **Per-human total** | **~1.2 Gi / ~4.6 Gi** | **610m / 2600m** | **15 Gi** |

## 20 Humans Total (Request Levels)

| Resource | Total | Notes |
|----------|-------|-------|
| Memory | ~24 Gi | Conductors dominate (CRDT merge + validation) |
| CPU | 12.2 cores | Burst during validation and sync storms |
| Disk | 300 Gi | 20 × 15 Gi (source chains + storage + WAL) |
| Containers | 60+ | 20 × 3 (conductor + storage + proxy) + infra |

Plus shared infrastructure (NATS ~64 Mi, MongoDB ~256 Mi, 1-3 Doorway instances ~128 Mi each).

**Machine spec**: 32 Gi / 16-core box runs ~12-14 humans. Need 64 Gi / 32 cores for 20.

## Per-Human Data Volume

| Data Type | Per Human | 20 Humans |
|-----------|-----------|-----------|
| Agent progress (100 paths) | ~1 MB | 20 MB |
| Content mastery (1000 items) | ~3 MB | 60 MB |
| Relationships + consent | ~750 KB | 15 MB |
| Source chain overhead | ~2 MB | 40 MB |
| **Hot data total** | **~6.75 MB** | **~135 MB** |

The cost is in processes, not bytes.

## 3 Schema Versions: Why It's Hard

Current versions:
- SQLite schema: **v3** (with v1→v2→v3 migrations)
- Holochain DNA: **v1**
- Human node `_schemaVersion`: **"1.0.0"**

Different DNA versions = different hashes = **separate DHT networks**.
Agents on v1 cannot directly gossip with agents on v3.

| Version | Humans | Conductors | Storage | Doorway |
|---------|--------|-----------|---------|---------|
| Schema v1 | 7 | 7 | 7 | 1 |
| Schema v2 | 7 | 7 | 7 | 1 |
| Schema v3 | 6 | 6 | 6 | 1 |
| **Total** | **20** | **20** | **20** | **3** |

= ~67 containers total.

Cross-version reads require bridge zomes (the `migration.rs` RNA pattern exists
but only handles same-agent cross-cell calls today).

## The Real Gap: Seeding ≠ Agency

The seeder creates data on behalf of humans through a single agent's source chain.
In Holochain P2P:

- Each human needs their own keypair (Lair keystore per conductor)
- Each human authors their own entries (signed by their agent key)
- Consent records require two independent signatures (not `0/1` flags)
- Relationships need bidirectional claims (party_a links, party_b reciprocates)

The seeder does `POST /api/db/content/bulk` as god-mode. For P2P you need a
**per-agent seeder** that generates 20 keypairs, installs DNA on 20 conductors,
and seeds each human's data through their own app WebSocket.

## Phased Path Forward

### Phase 1 — Lightweight (~5 Gi RAM, 10 cores)

Scale to 20 elohim-node instances without Holochain conductors.
Use the Automerge CRDT sync engine already in elohim-node.

**Tests**: sync protocol, conflict resolution, network partition behavior,
reach-based replication, cluster-to-cluster dynamics.

### Phase 2 — Full Holochain (~24-30 Gi RAM, 16+ cores)

Add conductors per human. Per-agent seeding script.

**Tests**: entry validation, DHT gossip, agent sovereignty, consent flows.

### Phase 3 — Mixed-version overlay (same compute, 3 DNA builds)

Deploy 3 DNA versions with migration bridge zomes.

**Tests**: forward/backward compatibility, cross-version reads, schema coexistence.

## What Exists vs. What's Missing

### Exists
- 2-cluster simulation compose with WAN latency injection
- Migration zome RNA pattern (transcribe between versions)
- Sync coordinator with stream positions
- 27 humans fully modeled with relationships, consent, and red-team personas
- Reed-Solomon blob sharding (4+3) in storage

### Missing
- Per-agent seeding script (keypair generation + per-conductor WebSocket calls)
- Multi-conductor docker-compose (20 humans, not 4 nodes)
- Schema-version-aware doorway routing (or multi-doorway orchestration)
- Bridge zome for cross-version DHT reads
- Consent ceremony orchestration (two-party signing flow)
