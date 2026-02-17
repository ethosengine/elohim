# Doorway Scaling: The Agency On-Ramp

> **See also**: [ARCHITECTURE.md](./ARCHITECTURE.md) for component details, [FEDERATION.md](./FEDERATION.md) for cross-doorway patterns, [RECOVERY-PROTOCOL.md](./RECOVERY-PROTOCOL.md) for identity recovery

## The Question

How does a node steward running doorway for their PTA, school, church, or co-op serve 150 families — and how does that scale to billions of humans worldwide?

There is no single answer because doorway has **two fundamentally different scaling concerns** that operate on different axes, with different characteristics, and different solutions.

---

## Two Scaling Axes

Doorway serves two audiences with completely different scaling profiles:

```
                        Axis 1: Projection (visitors reading content)
                        │
                        │  scales with: replicas, CDN, MongoDB read replicas
                        │  bounded by: content popularity (UNBOUNDED)
                        │  resolves: never — success means more readers
                        │
                        │
    ────────────────────┼──────────────────── Axis 2: Identity Hosting
                        │                     (humans in transition to P2P)
                        │
                        │  scales with: conductor pool
                        │  bounded by: hosted user count (BOUNDED)
                        │  resolves: graduation flywheel — users leave
                        │
                        doorway (today: both in one process)
```

**Axis 1** is classic web2. People browsing content, watching videos, reading posts, searching. No Holochain identity needed. Served from cache. Scales horizontally. Gets bigger as content gets popular. Does not self-resolve.

**Axis 2** is the agency transition. People creating accounts, getting custodial keys, writing to the DHT through doorway. Requires conductor cells. Scales via conductor pool. Gets smaller as people graduate to their own devices. Self-resolves.

These axes are **orthogonal**, not in tension. They share a process today for simplicity, but they have different scaling stories and different solutions.

---

## What Doorway Actually Is

Five services in one process, each belonging to one or both axes:

| Role | What It Does | Scaling Axis | Scales With | Stateful? |
|------|-------------|-------------|-------------|-----------|
| **DNS/TLS Gateway** | Terminates HTTPS, routes requests | Both | Request count | No |
| **Bootstrap/Signal** | Agent discovery + WebRTC relay | Axis 2 | Agent churn in DHT space | In-memory only |
| **Projection Cache** | Serves DHT content via HTTP/REST | **Axis 1** | Content read volume | MongoDB |
| **Identity Host** | Hosts agent keys + cells for web users | **Axis 2** | Hosted user count | Yes (conductor pool) |
| **Recovery Registrar** | Maintains recovery relationships | Axis 2 | Relationship count | Yes (lightweight) |

```
                                        +-- DNS/TLS ------ both axes, stateless
                                        |
Doorway is actually 5 things -----------+-- Bootstrap ---- axis 2, in-memory, bounded
                                        |
                                        +-- Projection --- AXIS 1: MongoDB-backed, horizontally scalable
                                        |
                                        +-- Identity ----- AXIS 2: conductor pool is the scaling axis
                                        |
                                        +-- Recovery ----- axis 2, lightweight metadata
```

---

## Axis 1: Projection Scaling

### How It Works Today

When a visitor reads content, the request never touches a Holochain conductor:

```
Browser → GET /api/v1/cache/Content/manifesto
    │
    ├─ Hot cache (DashMap): O(1) lookup, 10k entries, 5 min TTL     ~1-5ms
    │
    ├─ MongoDB projection: indexed query, millions of entries        ~10-50ms
    │
    └─ Conductor fallback (cache miss only, 1-5% of requests)       ~50-200ms
```

Blob streaming (`GET /store/{hash}`) goes to elohim-storage, not the conductor. HTTP 206 range requests for video seeking. Content-addressed, immutable, infinitely cacheable.

**No JWT required. No agent identity. No conductor cells. No custodial keys.**

This is a web server reading from a database. Every web2 scaling technique applies.

### How It Populates

Content flows one direction: DHT → projection → visitors.

```
DNA post_commit hook
    → ProjectionSignal over WebSocket
    → ProjectionEngine (one listener)
    → MongoDB + hot cache
    → available at /api/v1/cache/{type}/{id}
```

Only ONE doorway instance needs the signal subscriber connection. All other instances can read from the shared MongoDB projection.

### Why This Doesn't Self-Resolve

If the PTA publishes great curriculum content and it goes viral, read traffic grows without bound. More readers don't graduate — they aren't even users. They're just reading. This is the same scaling problem every content platform has, and the same solutions work:

- Multiple doorway read replicas behind a load balancer
- MongoDB read replicas for projection queries
- CDN for blob/media content (content-addressed = perfect cache keys)
- Edge caching (hot cache per replica is already built)

The graduation flywheel does nothing for this axis. A steward whose content gets popular needs more read capacity, period.

### What a Busy Projection Looks Like

```
PTA doorway serving popular curriculum content:
  Visitors per day:     10,000  (parents, teachers, curious people)
  Unique content items:  2,000  (lessons, videos, assessments)
  Blob storage:          50 GB  (video content, documents)
  Conductor load:        zero   (all served from projection)

  Scaling response:
    2 doorway replicas (load balanced, shared MongoDB)
    1 MongoDB replica set (3 nodes for HA + read scaling)
    CDN for /store/* blob routes (optional, content-addressed)
```

---

## Axis 2: Identity Hosting

### The Conductor Pool Model

A Holochain conductor can host cells for multiple agents simultaneously:

```
Conductor Process
  +-- Cell: elohim-user-alice (agent_key_1, source_chain_1, DHT participation)
  +-- Cell: elohim-user-bob   (agent_key_2, source_chain_2, DHT participation)
  +-- Cell: elohim-user-carol (agent_key_3, source_chain_3, DHT participation)
  +-- ...
  +-- Cell: elohim-user-N     (agent_key_N, source_chain_N, DHT participation)
```

Each cell:
- Has its own agent key (cryptographic identity)
- Maintains its own source chain (personal data journal)
- Participates independently in DHT gossip and validation
- Consumes memory (~20-50MB per active cell, varies with data volume)
- Consumes CPU (gossip, validation, zome execution)

**Practical capacity**: ~30-50 active hosted agents per conductor process, depending on:
- Available RAM (each cell caches DHT neighborhood data)
- Activity level (active users create more gossip/validation work)
- Zome complexity (compute-heavy zomes reduce per-conductor capacity)

### One Doorway, Many Conductors

A doorway steward with 150 hosted users doesn't need 150 conductors — they need ~3-5:

```
Doorway (1 instance)
  |
  +-- Routes requests based on agent identity
  |
  +---> Conductor-0: steward identity + users 1-40
  +---> Conductor-1: users 41-80
  +---> Conductor-2: users 81-120
  +---> Conductor-3: users 121-150
```

The routing is simple: doorway already has per-user JWT claims containing `agent_pub_key`. It just needs to map agent keys to conductor endpoints. (This routing is not yet built — see Future Work below.)

### The Custodial Key Vault

Each hosted user gets an Ed25519 keypair at registration. The private key is encrypted (Argon2id + ChaCha20-Poly1305) and stored in MongoDB. On login, the decrypted key is cached in doorway's process memory by `session_id` (up to 10k sessions, 1-hour TTL).

This is **state**. The signing key lives in one doorway process's RAM. This is why identity hosting doesn't scale with replicas — a user's session is pinned to the process that decrypted their key.

### The Graduation Flywheel

This is the key insight for axis 2: **identity hosting load decreases as doorway succeeds.**

```
Step 1: VISITOR
  Person discovers content via doorway.
  Doorway serves projection cache. Zero conductor load.
  Cost to steward: negligible (axis 1 concern, not axis 2).

Step 2: HOSTED HUMAN
  Person creates account. Doorway generates agent key, installs cells.
  They can CREATE content, build reputation, form relationships.
  Cost to steward: one cell in the conductor pool.
  THIS IS THE MOST EXPENSIVE PHASE.

Step 3: APP USER
  Person installs app on their device.
  Source chain migrates from hosted conductor to their device.
  Conductor cell freed up. Steward's load DECREASES.
  Doorway keeps: recovery registration (lightweight metadata).
  Cost to steward: near zero (DNS + recovery contract).

Step 4: NODE STEWARD
  Person runs their own always-on node. Fully self-directed.
  May run their own doorway for THEIR community.
  Original doorway's role: DNS/recovery only.
  Cost to steward: zero (the person is now a peer, not a dependent).
```

**Each graduation REDUCES the steward's identity hosting load while INCREASING the network's capacity.** But it does nothing for projection read traffic — that's axis 1.

### The Steady State

A mature doorway serving a community of 1,000 people:

```
Axis 2 (identity hosting):
  Active hosted users:    50  (new members still in Stage 2)
  Graduated to native:   600  (Stage 3-4, own devices)
  Recovery contracts:    900  (lightweight metadata)
  Conductor pool:         2   (steward + 50 hosted users)

Axis 1 (projection):
  Projection cache:     all   (serves visitors + native users + the world)
  Read traffic:         unbounded (depends on content popularity)
```

The conductor pool is small because most users graduated. But the projection layer may be busier than ever if the content is good.

---

## What Couples the Two Axes

Today both axes share one doorway process. The coupling points are thin:

| Coupling Point | What It Does | Who Needs It |
|---------------|-------------|-------------|
| **Signal subscriber** | Receives DHT signals, writes to MongoDB projection | One instance only (the "writer") |
| **Cache miss fallback** | On projection miss, falls back to conductor zome call | 1-5% of reads; could be optional |
| **Blob endpoint updates** | Infrastructure signals update P2P blob routing | One instance only |
| **Shared `AppState`** | Both axes share config, MongoDB handle, worker pools | Code convenience, not architectural requirement |

A future separation might look like:

```
Projection replicas (axis 1, stateless, N instances):
  +-- Read from shared MongoDB projection
  +-- Serve blobs from elohim-storage
  +-- No conductor connection needed
  +-- Hot cache per instance (DashMap)

Identity host (axis 2, stateful, 1 instance):
  +-- Signal subscriber (populates projection)
  +-- Custodial key vault (in-memory sessions)
  +-- Conductor pool routing
  +-- Admin API (user management)
  +-- Cache miss fallback (optional)
```

This separation is not needed today — a single doorway process handles both axes fine for small communities. But it's worth knowing where the seam is.

---

## The Human Topology

### A Family Running Doorway for Their PTA

```
Texas household (3 blades in the closet)
  +-- blade-1: conductor (steward identity + 50 hosted PTA families)
  +-- blade-2: conductor (50 hosted PTA families)
  +-- blade-3: conductor (50 hosted PTA families) + elohim-storage
  +-- doorway: routes to all 3 conductors
  |     DNS: pta-learning.familyname.community
  |     Recovery registrations: 1,500 (including graduated users)
  |     Projection cache: MongoDB on blade-1
  |
  6 months later (after graduation push):
  +-- blade-1: conductor (steward + 10 remaining hosted) + storage
  +-- blade-2: freed up --> repurposed for family media/storage
  +-- blade-3: freed up --> repurposed for family media/storage
  +-- doorway: still serves DNS/projection/recovery for 150 families
        Conductor load dropped 90%. Blades repurposed.
        Projection load may have INCREASED (content is popular).
```

### Extended Family Network (No Doorway Needed)

```
Arkansas siblings (pure P2P, 2 blades)
  +-- blade-1: conductor (dad) + storage
  +-- blade-2: conductor (mom) + storage
  +-- No doorway. Client devices connect directly.
  +-- Recovery: registered with Texas family's doorway (secondary)
  +-- P2P sync: replicate family data across households

Across-town family (3 blades)
  +-- blade-1: conductor (parents) + storage
  +-- blade-2: conductor (grandma) + storage
  +-- blade-3: doorway for their church small group
  +-- Recovery: cross-registered with Texas + Arkansas doorways
```

### The Doorway Steward's Own Identity

The steward is also a human. Their identity lives in the conductor pool alongside hosted users:

```
Conductor-0 on blade-1:
  +-- Cell: elohim-operator (the steward's own identity)
  +-- Cell: elohim-user-1 (first hosted user)
  +-- Cell: elohim-user-2
  +-- ...
```

The steward's cell is no different from a hosted user's cell — same DNA, same DHT participation. The only difference is that the steward controls the hardware.

---

## Global Scale

Scaling to billions happens differently on each axis:

```
Axis 1 (projection):
  Each doorway serves its community's content to the world.
  Popular content gets CDN/replica treatment — standard web2.
  Content is content-addressed — perfect for edge caching.
  Federation means doorways can cross-reference each other's projections.

Axis 2 (identity hosting):
  500,000 doorway stewards worldwide, each serving 50-500 hosted users at peak.
  Each has 1-10 conductors. Graduation reduces load over time.
  50,000,000 native P2P users (graduated from doorways).
  Each runs their own conductor. Some run their own doorways.
  DHT handles data at any scale — more nodes = smaller neighborhoods.
```

The network doesn't need one giant doorway. It needs many small doorways, each serving their community, each with content worth reading and people worth hosting.

---

## Current State vs Target

| Capability | Current | Target | Axis |
|-----------|---------|--------|------|
| Projection cache | Working (MongoDB + hot cache) | Working | 1 |
| Blob streaming | Working (elohim-storage, HTTP 206) | Working | 1 |
| Projection read replicas | Working (PROJECTION_WRITER flag, Ingress split) | Working | 1 |
| Multi-user auth | Working (JWT + agent_pub_key) | Working | 2 |
| Custodial key vault | Working (Argon2 + ChaCha20, session cache) | Working | 2 |
| Single conductor routing | Working (hardcoded `ws://conductor:4444`) | Working | 2 |
| Conductor registry | Working (skeleton — admin API, CONDUCTOR_URLS) | Per-request routing needed | 2 |
| Multi-conductor routing | NOT BUILT (registry exists, routing not wired) | Needed for hosted users | 2 |
| Dynamic agent provisioning | NOT BUILT | Needed for hosted users | 2 |
| Source chain migration | NOT BUILT | Needed for graduation | 2 |
| Recovery registration | Protocol designed, not implemented | Needed for steady state | 2 |
| Bootstrap/Signal | Working | Working | 2 |

### What's Blocking Multi-Conductor

Today, these are hardcoded to a single identity:

1. **`app_id: "elohim"`** everywhere (doorway config, elohim-storage config, happ-installer)
2. **Single `CONDUCTOR_URL`** in doorway config (no conductor pool routing)
3. **hApp installer** generates one agent key, installs one app instance
4. **elohim-storage** connects to one conductor with one app_id

### Future Work (Code Roadmap, Not This Sprint)

**Axis 1 (projection scaling):**

1. **Read replica support** — doorway instances that connect to MongoDB but not to any conductor. Serve projection reads only.
2. **CDN-friendly blob headers** — `Cache-Control: public, immutable` for content-addressed blobs
3. **Projection write/read split** — one doorway instance runs the signal subscriber; others read from shared MongoDB

**Axis 2 (identity hosting):**

1. **Dynamic app_id management** — doorway admin API to `POST /admin/hosted-users` which calls `generateAgentPubKey()` + `installApp()` on a conductor
2. **Conductor pool registry** — map agent_pub_key to conductor endpoint (which conductor hosts this user?)
3. **Per-request routing** — extract agent from JWT, look up conductor, route WebSocket/REST to correct conductor
4. **hApp installer generalization** — install N app instances per conductor (one per hosted user)
5. **Source chain migration** — export source chain from hosted conductor, import on user's device
6. **Recovery registration persistence** — store recovery contracts that survive conductor restarts
7. **Capacity management** — monitor cells per conductor, auto-assign new users to least-loaded conductor

---

## K8s Modeling (For Development)

The K8s manifests model this topology for development/CI:

```
Current (Sprint 9):
  Deployment: elohim-doorway-{env}          (1 replica, writer: PROJECTION_WRITER=true)
  Deployment: elohim-doorway-{env}-read     (N replicas, readers: PROJECTION_WRITER=false)
  StatefulSet: elohim-edgenode-{env}        (M replicas, conductor pool with volumeClaimTemplates)
  ConductorRegistry in doorway              (agent→conductor mapping in MongoDB)
  Ingress split: read-heavy → read replicas, write/admin → writer

  Alpha:   1 writer, 0 readers, 1 conductor
  Staging: 1 writer, 2 readers, 2 conductors

Future (per-request routing):
  Deployment: elohim-doorway-{env}          (1 replica, identity host + signal subscriber)
  Deployment: elohim-doorway-{env}-read     (N replicas, projection reads)
  StatefulSet: elohim-edgenode-{env}        (M replicas, conductor pool)
  Per-request routing: JWT agent_pub_key → ConductorRegistry → correct WorkerPool
```

But remember: K8s is a convenient abstraction for us as developers. Real stewards run bare metal. The architecture must make sense for 3 blades in a closet, not just for `kubectl apply`.

---

## Relationship to Other Documents

- **[ARCHITECTURE.md](./ARCHITECTURE.md)** — Component-level details: bootstrap, signal, gateway, cache, resolver
- **[FEDERATION.md](./FEDERATION.md)** — Cross-doorway patterns: projections not authorities, user portability
- **[RECOVERY-PROTOCOL.md](./RECOVERY-PROTOCOL.md)** — Identity recovery: social recovery, shard distribution, erasure coding
- **[../holochain/DEPLOYMENT-RUNTIMES.md](../holochain/DEPLOYMENT-RUNTIMES.md)** — The 4-stage agency journey
