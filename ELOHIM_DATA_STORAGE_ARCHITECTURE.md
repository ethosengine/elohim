# Elohim Data Storage Architecture

## Overview

Complete guide to where data lives, how it flows, and how it's replicated across the Elohim network.

**Current Status**:
- ✅ Client-side caching (3-tier: memory, IndexedDB, browser cache)
- ✅ Performance optimization (WASM module, O(log n) operations)
- ⏳ **NOW**: CDN capabilities, replication, sharding, Shefa metrics
- ⏳ **NEXT**: Full custodian selection algorithm, automatic tier-based pricing

---

## Storage Architecture - Complete Picture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         USER DEVICE                                 │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ TIER 0: Browser Memory (JavaScript)                          │  │
│  │ - URL cache (10K items, unlimited TTL)                       │  │
│  │ - DHT-verified content metadata                              │  │
│  │ - No eviction needed, fast access                            │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                ↓                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ TIER 1: Client Blob Cache (Rust WASM, 1GB)                  │  │
│  │ - 128MB per reach level (8x reach-aware caches)             │  │
│  │ - O(log n) LRU eviction via BTreeMap                        │  │
│  │ - Full blobs, media files                                   │  │
│  │ - 24-hour TTL, mastery-based decay                          │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                ↓                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ TIER 2: Client Chunk Cache (Rust WASM, 10GB)                │  │
│  │ - 1.25GB per reach level (8x reach-aware caches)            │  │
│  │ - Download chunks, O(k) TTL cleanup                         │  │
│  │ - 7-day TTL, LRU fallback                                   │  │
│  │ - Streaming large files                                     │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                ↓                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ TIER 3: IndexedDB Persistence (50MB)                        │  │
│  │ - Cached reads for offline access                           │  │
│  │ - Survives page reloads                                     │  │
│  │ - L2 fallback when L1 misses                               │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                ↓                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ TIER 4: Operation Queue (IndexedDB)                         │  │
│  │ - Queued write operations (offline-first)                   │  │
│  │ - Auto-sync on reconnect, exponential backoff              │  │
│  │ - Survives page reloads                                     │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                │                                     │
│                    Network Request (HTTP/WebSocket)                 │
│                                │                                     │
└────────────────────────────────┼─────────────────────────────────────┘
                                 ↓
┌─────────────────────────────────────────────────────────────────────┐
│                     HOLOCHAIN DOORWAY (Proxy)                       │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ Cache Layer (embedded doorway-cache)                         │  │
│  │ - Response caching based on zome-defined rules              │  │
│  │ - Cache invalidation signals from DHT post_commit           │  │
│  │ - Separate caches per DNA/reach level                       │  │
│  │ - Hit rate: 95%+ for reads                                  │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                ↓                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ Worker Pool (4 workers, semaphore limit 1000)               │  │
│  │ - Protects conductor from thread starvation                 │  │
│  │ - MPSC channel for fair FIFO queuing                        │  │
│  │ - Non-blocking backpressure                                 │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                │                                     │
│                     WebSocket to Conductor                          │
│                                │                                     │
└────────────────────────────────┼─────────────────────────────────────┘
                                 ↓
┌─────────────────────────────────────────────────────────────────────┐
│                    HOLOCHAIN CONDUCTOR (Node)                       │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ TIER 5: Source of Truth - DHT (Distributed Hash Table)      │  │
│  │                                                              │  │
│  │  Content Entries:                                           │  │
│  │  - Full content hash → EntryHash                           │  │
│  │  - Content metadata (domain, epic, reach)                  │  │
│  │  - Creator signature, timestamp                            │  │
│  │  - Links to related entries (relationships)                │  │
│  │                                                              │  │
│  │  Blob Entries:                                             │  │
│  │  - Blob metadata (hash, size, mime type)                   │  │
│  │  - Content hash reference                                  │  │
│  │  - Sharding strategy (full_replica, threshold, erasure)   │  │
│  │                                                              │  │
│  │  Mastery/Attestation Entries:                              │  │
│  │  - Content mastery level (0-7)                             │  │
│  │  - Mastery decay state                                     │  │
│  │  - Attestation links to content                            │  │
│  │                                                              │  │
│  │  Custodian Commitment Entries:                             │  │
│  │  - Agent commits to replicate specific content             │  │
│  │  - Commitment expiration time                              │  │
│  │  - Bandwidth/compute capacity claims                       │  │
│  │  - Health status updates                                   │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                ↓                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ DHT Replication Rules (3-copy default)                      │  │
│  │ - Each entry replicated to 3 random neighbors              │  │
│  │ - Automatic gossip/neighbor propagation                    │  │
│  │ - Shard stored near original author (primary)              │  │
│  │ - 2 secondary replicas chosen by DHT ring                  │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                │                                     │
│         Gossip Protocol (P2P, eventual consistency)                 │
│                                │                                     │
└────────────────────────────────┼─────────────────────────────────────┘
                                 ↓
┌─────────────────────────────────────────────────────────────────────┐
│                    CUSTODIAN REPLICAS (Sharded)                     │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ Custodian A (Europe, 100Mbps, Expert Steward)              │  │
│  │ - Replicas of 500GB content                                │  │
│  │ - Committed to: elohim-protocol, governance                │  │
│  │ - Replication factor: threshold_split(3,5) - M-of-N        │  │
│  │ - Health: 95% uptime (tracked in Shefa)                   │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ Custodian B (Asia, 50Mbps, Curator)                        │  │
│  │ - Replicas of 250GB content                                │  │
│  │ - Committed to: fct, value_scanner                         │  │
│  │ - Replication factor: erasure_coded(5,8) - efficient       │  │
│  │ - Health: 99% uptime (tracked in Shefa)                   │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ Custodian C (Americas, 200Mbps, Pioneer)                   │  │
│  │ - Replicas of 1TB content                                  │  │
│  │ - Committed to: all domains, research                      │  │
│  │ - Replication factor: full_replica - most robust          │  │
│  │ - Health: 99.5% uptime (tracked in Shefa)                │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
└────────────────────────────────────────────────────────────────────┘
                                 ↓
┌─────────────────────────────────────────────────────────────────────┐
│                    PROJECTION ENGINE (MongoDB)                      │
│                                                                     │
│  Watches DHT post_commit signals and transforms to:                │
│  - Full-text searchable content index                              │
│  - Custodian replication status                                    │
│  - Mastery/attestation ledger                                      │
│  - Content graph (relationships, dependencies)                     │
│  - Custodian commitment tracking                                   │
│                                                                     │
│  Used for:                                                          │
│  - Content discovery (search, filters)                             │
│  - Shefa dashboard (custodian metrics)                             │
│  - Replication health monitoring                                   │
│  - Recommendation engine                                            │
│                                                                     │
└────────────────────────────────────────────────────────────────────┘
                                 ↓
┌─────────────────────────────────────────────────────────────────────┐
│                    SHEFA DASHBOARD (Analytics)                      │
│                                                                     │
│  Human-Readable Custodian Metrics:                                 │
│  - Storage used (GB)                                               │
│  - Bandwidth (Mbps in/out)                                         │
│  - Uptime % (from health tracking)                                 │
│  - Content domains served                                          │
│  - Replication commitments honored                                 │
│  - Computation resources available                                 │
│  - Cost/rewards tracking                                           │
│                                                                     │
│  Used to:                                                           │
│  - Route requests to healthy custodians                            │
│  - Calculate tier-based pricing                                    │
│  - Identify resource bottlenecks                                   │
│  - Incentivize node participation                                  │
│  - Plan network capacity                                           │
│                                                                     │
└────────────────────────────────────────────────────────────────────┘
```

---

## Data Flow - Three Scenarios

### Scenario 1: Read Cached Content

```
User requests learning path

    ↓

[Device] Check L0 (JavaScript cache) → FOUND → Return immediately
                                        (5-10ms, no network)

Result: User sees content instantly from browser cache
Cost: 0 network calls, 0 DHT queries
```

### Scenario 2: Read with L0 Miss (L1/L2 Hit)

```
User requests content not in L0

    ↓

[Device] Check L1 (WASM blob cache) → FOUND → Return
                                    (100ms, local memory)
    OR
[Device] Check L2 (IndexedDB) → FOUND → Return
                              (500ms, persistent storage)

Result: Content served from local device
Cost: 0 network calls, 0 DHT queries
```

### Scenario 3: Cold Start (Cache Miss)

```
User requests content not cached locally

    ↓

[Device] Miss all local caches → Network request → Doorway (HTTP)

    ↓

[Doorway] Check cache rules for this zome call
          - Is this a cacheable read? YES
          - Do we have response cached? NO

          Request → Conductor (WebSocket)

    ↓

[Conductor] Execute zome call
            - Query DHT for content entry
            - Verify signatures
            - Calculate freshness

            Return result → Doorway

    ↓

[Doorway] Cache response (TTL: 24 hours for reads)
          Send to Device

    ↓

[Device] Receive result
         Cache in L1 (WASM)
         Also save to L2 (IndexedDB)

         Return to user

Result: Content now cached locally for future access
Cost: 1 network call, 1 DHT query
Next access: 100ms instead of network latency
```

### Scenario 4: Custodian Serving from Replica

```
Content creator (C1) published learning path
Custodian A committed to replicate "governance" content

User in Asia requests this learning path

    ↓

[Device] Cache miss → Network request

    ↓

[Doorway] Not in cache → Check custodian index
          Custodian A (Asia) is nearest + has this content
          Route to Custodian A.doorway (local to their network)

    ↓

[Custodian A Doorway] Content in their replica cache
                      Return immediately (5-10ms)

[Doorway] Cache result (TTL: 24h)
          Send to Device

    ↓

[Device] Cache in L1/L2
         Return to user

Result: Content served from nearby custodian (CDN-like)
Cost: 1 network call (to nearest custodian, not origin)
Benefit: 10x faster than origin DHT query
```

---

## Sharding Strategies

Content creator can choose how their content is replicated across network:

### Strategy 1: Full Replica (Most Robust)

```
Content: "Governance 101 Video" (2GB)

Custodian A: Full copy (2GB)
Custodian B: Full copy (2GB)
Custodian C: Full copy (2GB)

Total: 6GB storage

Advantages:
- Highest availability (any custodian can serve)
- Fastest reads (no reconstruction)
- No computation required

Disadvantages:
- Expensive (3x storage)
- Best for small, critical content

Example: Core learning paths, critical documentation
```

### Strategy 2: Threshold Secret Sharing (M-of-N)

```
Content: "Research Dataset" (1GB)

Encoded into 5 shards, need any 3 to reconstruct:

Custodian A: Shard 1 (400MB)
Custodian B: Shard 2 (400MB)
Custodian C: Shard 3 (400MB)
Custodian D: Shard 4 (400MB)
Custodian E: Shard 5 (400MB)

Total: 2GB storage (2x vs full replica)

Read Process:
1. Request from any 3 custodians
2. Reconstruct original content
3. Return to user

Advantages:
- Balanced storage (2x vs 3x)
- High availability (need 3 of 5)
- Medium computation (reconstruction)

Disadvantages:
- Reconstruction latency (50-100ms)
- More complex coordination

Example: Medium-sized datasets, research data
```

### Strategy 3: Erasure Coding (5-of-8)

```
Content: "Course Videos" (5GB)

Encoded into 8 shards, need any 5 to reconstruct:

Custodian A: Shard 1 (800MB)
Custodian B: Shard 2 (800MB)
Custodian C: Shard 3 (800MB)
Custodian D: Shard 4 (800MB)
Custodian E: Shard 5 (800MB)
Custodian F: Shard 6 (800MB)
Custodian G: Shard 7 (800MB)
Custodian H: Shard 8 (800MB)

Total: 6.4GB storage (1.28x vs full replica!)

Read Process:
1. Request from any 5 custodians (parallel)
2. Reconstruct original content
3. Return to user

Advantages:
- Most storage efficient (1.28x)
- Very high availability (5 of 8 need to fail)
- Parallel reconstruction (fast)

Disadvantages:
- Computation intensive (polynomial interpolation)
- Coordination overhead (8 requests in parallel)

Example: Large content, media libraries, archives
```

---

## Custodian Selection Algorithm

When user requests content, how does system choose which custodian to fetch from?

```typescript
interface CustodianScore {
  custodianId: string;
  health: number;           // 0-100 (uptime %)
  latency: number;          // ms to reach
  bandwidth: number;        // Mbps available
  specialization: number;   // 0-1, relevance to domain
  commitment: number;       // Has content committed?
  pricing: number;          // Cost per GB
}

function selectCustodian(contentId: string, userId: string): CustodianScore {
  const custodians = dht.find_custodian_commitments(contentId);

  let bestScore = -Infinity;
  let selectedCustodian = null;

  for (const custodian of custodians) {
    const health = shefa.getHealth(custodian.id);        // 0-100
    const latency = network.estimate_latency(custodian);  // ms
    const bandwidth = shefa.getBandwidth(custodian.id);  // Mbps
    const specialization = evaluate_domain_specialization(
      custodian.domains,
      content.domain
    );
    const commitment = shefa.getCommitment(custodian.id, contentId);
    const pricing = shefa.getTierPrice(custodian.stewardTier);

    // Score: higher is better
    // Prioritizes: health > latency > bandwidth > specialization
    const score =
      (health * 0.4) +                           // 40% uptime
      ((100 - latency) * 0.3) +                  // 30% proximity
      (Math.min(bandwidth / 100, 1) * 0.15) +   // 15% bandwidth
      (specialization * 0.1) +                   // 10% specialization
      (commitment ? 0.05 : 0);                   // 5% bonus for commitment

    if (score > bestScore) {
      bestScore = score;
      selectedCustodian = custodian;
    }
  }

  return selectedCustodian;
}
```

**Factors in Priority Order**:

1. **Health (40%)**: No point using unhealthy custodian
   - Tracked in Shefa: uptime %, connection failures, response times
   - Historical: last 7 days rolling window

2. **Latency (30%)**: Geographic proximity matters
   - Tracked in Shefa: ping time to custodian
   - Used for: prefetch optimization, user experience

3. **Bandwidth (15%)**: Can they handle concurrent requests
   - Tracked in Shefa: declared bandwidth, current utilization
   - Used for: load balancing, surge protection

4. **Specialization (10%)**: Do they specialize in this domain
   - Domain affinity: track what domains custodian typically serves
   - Example: custodian A specializes in "governance", gets boost

5. **Commitment Bonus (5%)**: Did they explicitly commit to this content
   - Check CustodianCommitment entry for this content
   - If yes: +5% score bonus (strong signal they're prepared)

---

## Shefa Dashboard - What Node Operators See

The Shefa dashboard provides human-readable metrics for custodian operators:

### Storage Metrics

```
Custodian Dashboard
═══════════════════════════════════════════════════════════════

STORAGE UTILIZATION
├─ Total Capacity: 10TB
├─ Used: 7.2TB (72%)
├─ Available: 2.8TB (28%)
└─ Breakdown:
   ├─ elohim-protocol: 3.5TB (governance, autonomous_entity)
   ├─ fct: 1.8TB (governance, value_scanner)
   ├─ ethosengine: 1.2TB (research, value_scanner)
   └─ Other domains: 0.7TB

REPLICATION STRATEGY BREAKDOWN
├─ Full Replica: 2.0TB (minimum risk)
├─ Threshold (3-of-5): 3.5TB (balanced)
├─ Erasure (5-of-8): 1.7TB (efficient)

COMMITMENT STATUS
├─ Active Commitments: 145
├─ Expiring Soon (< 7 days): 12
├─ Expired (needs renewal): 3
├─ Total Content Committed: 7.2TB
```

### Bandwidth Metrics

```
BANDWIDTH UTILIZATION (Last 24 hours)
├─ Declared Capacity: 100Mbps
├─ Peak Usage: 85Mbps (85% utilization) at 14:00 UTC
├─ Average Usage: 45Mbps
├─ Idle Time: 15%
│
├─ By Direction:
│  ├─ Inbound (serving users): 35Mbps avg
│  └─ Outbound (replicating): 10Mbps avg
│
├─ By Domain:
│  ├─ elohim-protocol: 20Mbps (44%)
│  ├─ fct: 12Mbps (27%)
│  ├─ ethosengine: 10Mbps (22%)
│  └─ Other: 3Mbps (7%)

BANDWIDTH EFFICIENCY
├─ Requests Served: 45,123
├─ Cache Hit Rate: 87% (local cache, no bandwidth used)
├─ DHT Queries: 5,871 (13% miss rate)
├─ Avg Response Time: 125ms (healthy)
```

### Health Metrics

```
UPTIME & RELIABILITY
├─ Availability (last 30 days): 99.2%
├─ Outages: 1 incident (5 hours, 2 days ago)
├─ Mean Time Between Failures: 45 days
├─ Mean Time To Recover: 2 hours
│
├─ Response Time SLA:
│  ├─ p50 (median): 50ms
│  ├─ p95 (95th percentile): 200ms
│  ├─ p99 (99th percentile): 500ms
│  └─ SLA Target: < 300ms for 99% ✓ PASS

HEALTH SIGNALS
├─ Conductor Status: HEALTHY ✓
├─ Database Status: HEALTHY ✓
├─ Network Connectivity: HEALTHY ✓
├─ Disk Space: 28% free (HEALTHY ✓, warn at 10%)
├─ Memory Usage: 65% (HEALTHY ✓)
├─ Replication Lag: 2s (HEALTHY ✓, warn at >60s)
```

### Computation Metrics

```
COMPUTATION RESOURCES
├─ CPU Cores: 8 (all cores)
├─ CPU Usage: 45% avg (can handle spikes)
├─ Memory: 64GB
├─ Memory Usage: 42GB (66%, growing at 500MB/day)
├─ Disk I/O: 150MB/s (read), 50MB/s (write)
│
├─ Zome Execution Stats:
│  ├─ Queries (reads): 10,234 calls, 45ms avg
│  ├─ Mutations (writes): 234 calls, 120ms avg
│  ├─ Validations: 456 calls, 80ms avg
│
├─ Reconstruction Workload (erasure coding):
│  ├─ Reconstructions: 12 (last 24h)
│  ├─ Avg Time: 500ms
│  ├─ Peak CPU: 90% during reconstruction
└─ Computation Credits: 15,000 available / 50,000 max
```

### Economic Metrics

```
PRICING & REWARDS
├─ Steward Tier: EXPERT (3/4)
├─ Current Pricing:
│  ├─ Replication Cost: $0.05/GB/month
│  ├─ Bandwidth: $0.02/GB outbound
│  ├─ Computation: $0.001/operation
│
├─ Last 30 Days Earnings:
│  ├─ Replication Revenue: $360 (7.2TB)
│  ├─ Bandwidth Revenue: $5.40 (270GB served)
│  ├─ Computation Revenue: $10.23
│  ├─ Total Revenue: $375.63
│  ├─ Total Costs: $120 (operations, electricity)
│  └─ Net Profit: $255.63
│
├─ Reputation:
│  ├─ Health Score: 98/100
│  ├─ Reliability Rating: ⭐⭐⭐⭐⭐ (4.8)
│  ├─ Speed Rating: ⭐⭐⭐⭐ (4.5)
│  ├─ Specialization Bonus: +5% (governance expert)
│
└─ Tier Progression:
   ├─ Current: EXPERT (3/4)
   ├─ To PIONEER: +500GB committed + 99.5% uptime
   └─ Time to Promotion: ~3 months at current trajectory
```

### Alerts & Actions

```
ALERTS & RECOMMENDATIONS

⚠️  WARNINGS
├─ Disk Usage Growing: +500MB/day, 28% free
│  Action: Plan for 2.4TB expansion in 1 year
│
├─ Memory Trending Up: 66% utilization, +1GB/day
│  Action: Monitor for leaks, upgrade if >80%
│
└─ 3 Commitments Expiring: Renew in next 7 days
   Action: Review and renew to maintain reputation

✅ OPPORTUNITIES
├─ Available Bandwidth: 15Mbps unused capacity
│  Recommendation: Accept more replication commitments
│
├─ CPU Capacity: 55% available
│  Recommendation: Increase computation quota
│
└─ Specialization Bonus: +5% available for "ethical-innovation"
   Recommendation: Take on more ethics-related content
```

---

## Data Consistency & Freshness

### Consistency Model

**Eventual Consistency** (by design):
- Client reads from various sources (cache, custodian, DHT)
- Different sources may have different data temporarily
- All converge to same state within **TTL window** (24 hours)

**Strong Consistency** (when needed):
- Read from DHT (authoritative source)
- Verify all signatures
- Check against Holochain validation rules
- Used for: critical attestations, mastery updates

### Freshness Guarantees

```
Content Type              Cache TTL    Freshness Strategy
─────────────────────────────────────────────────────────
Learning Path             24 hours     Mastery-based decay
Mastery Level             1 hour       Real-time validation
Attestation               Never        DHT lookup (immutable)
User Profile              1 hour       Cache + occasional refresh
Search Index              5 minutes    Projection engine refresh
Custodian Health          5 minutes    Shefa polling
Replication Status        5 minutes    DHT monitoring
```

### Cache Invalidation

```
Trigger                           Action
──────────────────────────────────────────────────────────
Content updated by creator        - Invalidate L0 (JavaScript)
                                  - Invalidate Doorway cache
                                  - DHT gossip (5-30 seconds)
                                  - Keep client L1/L2 (will
                                    naturally expire in 24h)

Mastery level changed             - Invalidate L0 + Doorway
                                  - Immediate (reads depend
                                    on current mastery)

Custodian health event            - Update Shefa metrics
                                  - Adjust selection algorithm
                                  - Users auto-failover next
                                    request

Entry deleted/revoked             - DHT gossip propagation
                                  - Client L1/L2 keeps stale
                                    copy (won't be refreshed)
                                  - Soft delete best practice
                                    (mark deleted, keep entry)
```

---

## Storage Capacity Planning

### Typical Custodian Footprint

```
Small Custodian (Curator Tier - Tier 2)
├─ Storage Capacity: 1-5TB
├─ Bandwidth: 20-50Mbps
├─ Content: Specialized niche (1-2 domains)
├─ Replication Strategy: Full replica (fewer commitments)
├─ Expected Revenue: $50-200/month
└─ Use Case: Homelab, small organization

Medium Custodian (Expert Tier - Tier 3)
├─ Storage Capacity: 10-50TB
├─ Bandwidth: 100-500Mbps
├─ Content: Multiple domains, active participation
├─ Replication Strategy: Mix (full + threshold)
├─ Expected Revenue: $500-2000/month
└─ Use Case: Medium hosting provider, research institution

Large Custodian (Pioneer Tier - Tier 4)
├─ Storage Capacity: 100TB+
├─ Bandwidth: 1-10Gbps
├─ Content: Comprehensive coverage, primary replica
├─ Replication Strategy: Erasure coding (efficient)
├─ Expected Revenue: $5000-50000+/month
└─ Use Case: Major hosting, CDN provider, university
```

---

## Read Path - Detailed Flow

When user requests content, system tries in order:

```
1. Browser L0 Cache (JavaScript)
   ├─ Lookup: O(1) hash
   ├─ Speed: <1ms
   ├─ Storage: 10K items
   ├─ Hit Probability: 80%
   └─ Success? → Return immediately

2. Client L1 Cache (WASM Blob Cache)
   ├─ Lookup: O(1) HashMap + reach level check
   ├─ Speed: 5-10ms
   ├─ Storage: 1GB (128MB per reach level)
   ├─ Hit Probability: 70% (if L0 miss)
   └─ Success? → Load to L0, return

3. Client L2 Cache (IndexedDB Persistence)
   ├─ Lookup: O(1) IDB transaction
   ├─ Speed: 50-100ms
   ├─ Storage: 50MB
   ├─ Hit Probability: 60% (if L0/L1 miss)
   └─ Success? → Load to L1, return

4. Doorway Cache (HTTP Proxy)
   ├─ Lookup: Check HTTP cache headers/rules
   ├─ Speed: 100ms + network latency
   ├─ Storage: Configurable per DNA
   ├─ Hit Probability: 95% for cacheable reads
   └─ Success? → Cache at device, return

5. Custodian Replica (CDN-like)
   ├─ Lookup: Check custodian commitment index
   ├─ Selection: Algorithm picks best custodian
   ├─ Speed: Network latency to custodian (50-500ms)
   ├─ Replication Strategy: full_replica, threshold, or erasure
   ├─ Hit Probability: 99.9% (content stored there)
   └─ Success? → Cache through all tiers, return

6. DHT Origin (Source of Truth)
   ├─ Lookup: DHT query for entry hash
   ├─ Speed: DHT gossip (1-5 seconds typically)
   ├─ Replication: 3-copy default
   ├─ Hit Probability: 100% (if ever existed)
   └─ Success? → Custodians cache for replication

Total Latency:
- L0 hit: 1-5ms (instant to user)
- L1 hit: 10-20ms (imperceptible)
- L2 hit: 100-150ms (feels instant)
- Doorway hit: 100-200ms + network
- Custodian hit: 50-500ms (depends on distance)
- DHT origin: 1-5 seconds (cold start)

Cache Hit Rates (Typical):
- L0: 80%
- L1: 10% (of L0 misses)
- L2: 5% (of L1 misses)
- Doorway: 4% (of L2 misses)
- Custodian: 0.99% (of Doorway misses)
- DHT: 0.01% (cold start)

= 99.99% served from cache or nearby replica!
```

---

## Write Path - Complete Flow

When user creates/updates content:

```
1. Create on Device
   └─ Form filled out, user clicks "Save"

2. Queue Check
   ├─ Is device online?
   │  YES → Continue to 3
   │  NO → Queue operation in OfflineOperationQueueService
   │       (IndexedDB persistence)
   │       Return: "Saved locally, will sync when online"

3. Sign Content
   └─ Use user's signing key (from DHT)
      Signature proves authorship

4. Zome Call
   ├─ Device → Doorway (HTTP POST /api/v1/zome/...)
   │  (includes operation queue as payload)
   │
   └─ Doorway → Conductor (WebSocket)

5. Validation
   ├─ Conductor validates entry
   │  - Check signature valid
   │  - Check reach level permissions
   │  - Check referenced entries exist
   │  - Check mastery gating
   │
   ├─ If invalid → Return error to device
   │  Device: User sees error, operation stays queued
   │
   └─ If valid → Store in DHT

6. DHT Storage
   ├─ Entry stored in Conductor's local DHT shard
   │  EntryHash = hash(entry content + creator)
   │
   └─ DHT Replication (gossip protocol)
      ├─ 1st neighbor: propagates within 1-2 seconds
      ├─ 2nd neighbor: propagates within 2-5 seconds
      ├─ 3rd neighbor: propagates within 5-30 seconds
      └─ All: converge to same state

7. Post-Commit Hooks
   ├─ Creator's Conductor:
   │  ├─ Update replication index
   │  ├─ Emit DHT signal (post_commit)
   │  └─ Projection engine watches this signal
   │
   ├─ Projection Engine (MongoDB):
   │  ├─ Receive signal: new entry created
   │  ├─ Transform to searchable document
   │  ├─ Add to full-text index
   │  └─ Update content graph
   │
   ├─ Doorway Cache:
   │  ├─ Receive signal: content updated
   │  ├─ Invalidate cached responses that reference this
   │  └─ Cache clear for reach-level lookups
   │
   └─ Shefa Metrics:
      ├─ New content detected
      ├─ Emit "content_created" event
      ├─ Trigger custodian selection algorithm
      ├─ Create replication commitments (if enabled)
      └─ Track in creator's statistics

8. Client Response
   ├─ Device receives: { success: true, entryHash: "..." }
   │
   ├─ Update local state:
   │  ├─ Add to L0 cache (JavaScript)
   │  ├─ Add to L1 cache (WASM)
   │  ├─ Add to L2 cache (IndexedDB)
   │  └─ Clear operation queue
   │
   └─ User feedback:
      ├─ Loading indicator removed
      ├─ Show: "Content created successfully"
      ├─ Notification: "X liked your content" (when it arrives)
      └─ Redirect to view

Total Latency:
- Device → Doorway: ~100ms (network)
- Doorway → Conductor → DHT: ~50-200ms
- DHT Replication (1st): ~1-2 seconds
- DHT Replication (full): ~30 seconds
- Projection indexing: ~5-10 seconds (MongoDB)
- Custodian replication: ~1-5 minutes (if enabled)

Until full replication:
- Content visible to creator immediately
- Visible to peers who queried DHT neighbors after 5s
- Searchable in MongoDB after 10s
- Replicated across custodians after 5 minutes
```

---

## Metadata Tracking for Shefa

### What Shefa Tracks

```
For Each Custodian Entry:

struct CustodianMetrics {
  custodian_id: AgentId,

  // Storage
  total_storage_bytes: u64,
  used_storage_bytes: u64,
  free_storage_bytes: u64,

  // Breakdown by domain/epic
  storage_by_domain: Map<Domain, u64>,
  storage_by_epic: Map<Epic, u64>,

  // Replication strategy breakdown
  full_replica_bytes: u64,        // 3x storage
  threshold_bytes: u64,           // 2x storage
  erasure_coded_bytes: u64,       // 1.3x storage

  // Bandwidth
  declared_bandwidth_mbps: u32,
  current_bandwidth_usage: u32,
  peak_bandwidth: u32,
  average_bandwidth: u32,

  // Health
  uptime_percentage: f32,         // Last 30 days
  availability: f32,              // Current
  mean_response_time_ms: u32,
  p95_response_time_ms: u32,
  p99_response_time_ms: u32,
  last_outage: Timestamp,
  outage_duration_minutes: u32,

  // Commitments
  active_commitments: u32,
  total_committed_bytes: u64,
  commitment_fulfillment_rate: f32, // % of commitments honored

  // Computation
  available_cpu_cores: u32,
  cpu_utilization_percent: u32,
  available_memory_gb: u32,
  memory_utilization_percent: u32,
  zome_operations_per_second: f32,

  // Replication lag
  max_replication_lag_seconds: u32, // How far behind primary

  // Economic
  steward_tier: StewardTier,      // 1-4
  pricing_per_gb: f32,             // $/GB/month
  total_earned: u64,               // Lifetime earnings
  month_earned: u64,               // Current month

  // Reputation
  health_score: u32,               // 0-100
  reliability_rating: f32,         // 0-5 stars
  speed_rating: f32,               // 0-5 stars
  specialization_bonus: f32,       // 0-0.1 (10%)
}
```

### Prometheus Metrics Exposed

```
# For monitoring systems (Prometheus/Grafana)

elohim_custodian_storage_used_bytes{custodian_id="abc", domain="governance"}
elohim_custodian_storage_free_bytes{custodian_id="abc"}
elohim_custodian_bandwidth_used_mbps{custodian_id="abc"}
elohim_custodian_uptime_percentage{custodian_id="abc"}
elohim_custodian_response_time_ms{custodian_id="abc", percentile="p95"}
elohim_custodian_active_commitments{custodian_id="abc", domain="governance"}
elohim_custodian_cpu_utilization_percent{custodian_id="abc"}
elohim_custodian_zome_ops_per_second{custodian_id="abc"}
elohim_custodian_reputation_score{custodian_id="abc"}
elohim_custodian_monthly_earnings{custodian_id="abc"}
```

### Real-Time Alerts

```
Alert Rule: "CustodianHighMemoryUsage"
Condition: memory_utilization > 80% for 10 minutes
Action: Email custodian, suggest upgrade or pause commitments

Alert Rule: "CustodianHighLatency"
Condition: p95_response_time > 1000ms for 15 minutes
Action: Email custodian, reduce load or investigate

Alert Rule: "CustodianLowUptime"
Condition: uptime < 95% over 7 days
Action: Reduce reputation score, lower in selection algorithm

Alert Rule: "CommitmentUnfulfilled"
Condition: commitment_fulfillment_rate < 90% or lag > 60s
Action: Force renewal requirement, penalty points
```

---

## Integration with Pricing & Rewards

### Tier-Based Pricing (Dynamic)

```
Caretaker (Tier 1)
├─ Replication: $0.10/GB/month (higher cost, less verified)
├─ Bandwidth: $0.05/GB outbound
├─ Computation: $0.005/operation
├─ Requirements: Uptime >= 90%
└─ Max Capacity: 100GB commitment

Curator (Tier 2)
├─ Replication: $0.07/GB/month (proven, reliable)
├─ Bandwidth: $0.03/GB outbound
├─ Computation: $0.003/operation
├─ Requirements: Uptime >= 95%, 2+ successful commitments
└─ Max Capacity: 5TB commitment

Expert (Tier 3)
├─ Replication: $0.05/GB/month (trusted)
├─ Bandwidth: $0.02/GB outbound
├─ Computation: $0.001/operation
├─ Requirements: Uptime >= 97%, 10+ successful commitments
└─ Max Capacity: 100TB commitment

Pioneer (Tier 4)
├─ Replication: $0.03/GB/month (full trust)
├─ Bandwidth: $0.01/GB outbound
├─ Computation: $0.0005/operation
├─ Requirements: Uptime >= 98.5%, 50+ successful commitments
└─ Max Capacity: Unlimited
```

### Example Earnings Calculation

```
Custodian A (Expert Tier, committed 50GB)
├─ Replication Revenue:
│  └─ 50GB × $0.05/GB/month = $2.50/month
│
├─ Bandwidth Revenue:
│  ├─ Served 5TB last month
│  └─ 5000GB × $0.02/GB = $100/month
│
├─ Computation Revenue:
│  ├─ 50,000 zome operations
│  └─ 50,000 × $0.001 = $50/month
│
├─ Reputation Bonus:
│  ├─ Health score 97/100 → +2% bonus
│  ├─ Specialization "governance" → +5% bonus
│  └─ Total bonus: +7%
│
└─ Total Monthly:
   ├─ Base: $2.50 + $100 + $50 = $152.50
   ├─ With Bonuses: $152.50 × 1.07 = $163.18
   └─ Minus Operating Costs (~$30): **$133.18 net**

Annual Revenue: ~$1600 + bonuses
```

---

## Future Enhancements

### Phase 1 (Current)
- ✅ WASM blob cache with reach-level isolation
- ✅ Graceful degradation & offline support
- ⏳ **NOW**: Sharding strategies, custodian selection, Shefa metrics

### Phase 2 (Next)
- [ ] Automatic replication commitment creation (based on content popularity)
- [ ] Custodian selection algorithm optimization
- [ ] Tier-based pricing enforcement
- [ ] Reputation system for ranking custodians

### Phase 3
- [ ] Proof-of-Storage for fraud detection
- [ ] Economic coordination (prices, rewards, penalties)
- [ ] Predictive load balancing
- [ ] Automatic custodian failover

### Phase 4
- [ ] Cross-network replication (multiple Holochain networks)
- [ ] Blockchain settlement (if needed)
- [ ] Advanced compression & deduplication
- [ ] Confidential computing for sensitive data

---

## Summary

Data flows through a carefully orchestrated architecture:

```
User Device → Doorway → Conductor → DHT → Custodian Replicas
    ↓            ↓          ↓        ↓           ↓
  Cache        Cache      Authority  Peers   Distributed
  Tiers        Rules                         Copies

Every layer has:
├─ Cache (speed)
├─ Replication (redundancy)
├─ Health tracking (reliability)
├─ Metrics (observability via Shefa)
└─ Economic incentives (participation)
```

**Key Principles**:
1. **Locality**: Prefer nearest cache/custodian (latency)
2. **Redundancy**: Multiple copies via sharding strategies
3. **Efficiency**: LRU eviction, TTL expiration, compression
4. **Incentives**: Tier-based rewards for reliable operators
5. **Transparency**: Shefa dashboard for operator visibility

This enables:
- **99.99% cache hit rate** (most reads served locally/nearby)
- **Sub-second user experience** (99% of requests < 500ms)
- **Decentralized replication** (no single point of failure)
- **Economic coordination** (automatic pricing/rewards)
- **Scalability** (custodians can increase capacity independently)

