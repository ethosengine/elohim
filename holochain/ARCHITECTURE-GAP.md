# Architecture Gap: Agentic vs Client Compute

## The Mental Model Shift

Understanding why we need a new model requires understanding what the existing models optimize for.

## Client-Server Compute

```
                    ┌─────────────────┐
                    │     SERVER      │
                    │                 │
                    │ • Owns data     │
                    │ • Runs compute  │
                    │ • Controls auth │
                    │ • Single source │
                    │   of truth      │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
         ┌────────┐    ┌────────┐    ┌────────┐
         │ Client │    │ Client │    │ Client │
         │  (dumb │    │  (dumb │    │  (dumb │
         │ viewer)│    │ viewer)│    │ viewer)│
         └────────┘    └────────┘    └────────┘
```

### What It Optimizes For
- **Simplicity**: One place for everything
- **Consistency**: Server is always right
- **Performance**: Optimized queries, caching
- **Control**: Operator has full power

### What It Sacrifices
- **Sovereignty**: User doesn't own their data
- **Resilience**: Server down = everything down
- **Trust**: Must trust the operator
- **Capture**: Operator can be coerced/compromised

### Identity Model
```
User: "I am alice@server.com"
Server: "I verify you are alice. Here's a session."

WHO YOU ARE = What the server's database says
```

## Agent-Centric Compute (Holochain Model)

```
    ┌────────┐        ┌────────┐        ┌────────┐
    │ Agent  │        │ Agent  │        │ Agent  │
    │   A    │        │   B    │        │   C    │
    │        │        │        │        │        │
    │• Keys  │        │• Keys  │        │• Keys  │
    │• Chain │        │• Chain │        │• Chain │
    │• Data  │        │• Data  │        │• Data  │
    └───┬────┘        └───┬────┘        └───┬────┘
        │                 │                 │
        └─────────────────┴─────────────────┘
                          │
                    ┌─────┴─────┐
                    │    DHT    │
                    │  (gossip) │
                    └───────────┘
```

### What It Optimizes For
- **Sovereignty**: Agent owns their keys and data
- **Capture Resistance**: No central point to compromise
- **Validation**: Peers validate each other
- **Portability**: Agent can move between networks

### What It Sacrifices
- **Performance**: DHT gossip is slow (200-2000ms)
- **Query Capability**: No indexes, no complex queries
- **Scale**: Chokes at thousands of entries
- **Usability**: Requires local conductor, can't run in browser

### Identity Model
```
Agent: *generates keypair*
Agent: "I AM this public key. I sign my own statements."

WHO YOU ARE = Your cryptographic keys
```

### The Practical Gap

**What Holochain promises:**
> "Holochain is an open source framework for building fully distributed, peer-to-peer applications... Each user runs the application on their own device, creating a peer-to-peer network."

**What we actually get:**
- DHT chokes at 3000 entries
- Web users need a "doorway" (server)
- Gossip latency makes real-time apps painful
- No practical query capability
- Cold start problem for new agents

## The Gap Illustrated

### Scenario: 5 Learning Paths, 3000 Content Nodes

**Client-Server Approach:**
```
Seed time: 2 seconds
Query time: 10-50ms
Works: Yes
Captured: Yes
```

**Pure Agent-Centric:**
```
Seed time: FOREVER (DHT chokes)
Query time: 200-2000ms (if it works)
Works: No
Captured: No
```

**What We Need:**
```
Seed time: Fast (to storage layer)
Query time: Fast (from storage layer)
Works: Yes
Captured: No (P2P base layer)
```

## The Missing Piece: Community Compute

Neither model accounts for **relational scaling** — the idea that:
1. Compute should scale with investment
2. Responsibility should be distributed by relationship
3. Cost should be visible and meaningful
4. The network should strengthen with community growth

```
CLIENT MODEL         AGENT MODEL          COMMUNITY MODEL
─────────────        ───────────          ───────────────

Scale with:          Scale with:          Scale with:
  $$$                  Agents (theory)      Investment
                       Nothing (practice)

Trust model:         Trust model:         Trust model:
  Operator             Math                 Relationships

Resilience:          Resilience:          Resilience:
  Backup servers       DHT replication      Community replication

Cost:                Cost:                Cost:
  Hidden ($$$)         Hidden (compute)     Visible (contribution)
```

## Why Doorway Exists (And Its Limits)

Doorway bridges the gap:
```
┌─────────────┐        ┌─────────────┐        ┌─────────────┐
│   Browser   │◄──────►│   Doorway   │◄──────►│  Holochain  │
│   (client)  │  HTTP  │   (server)  │  WS    │    (DHT)    │
└─────────────┘        └─────────────┘        └─────────────┘
```

**What Doorway provides:**
- Web access (clients can't run conductors)
- Caching (avoids DHT latency)
- Queries (that DHT can't do)
- Custodial keys (for users without devices)

**What Doorway reintroduces:**
- Single point of failure
- Capture potential
- Trust requirement
- The server model

**Doorway is a necessary compromise, not the solution.**

## The Path Forward

### Layer Separation

```
┌─────────────────────────────────────────────────────────────┐
│                    TRUST LAYER                              │
│                    (Holochain)                              │
│                                                             │
│  What belongs here:                                         │
│  • Agent identity (registration)                            │
│  • Attestations (signed claims)                             │
│  • Trust relationships (who trusts whom)                    │
│  • Content location (who has what)                          │
│                                                             │
│  Entry count: 100s-1000s (manageable)                       │
│  Entry size: <1KB each                                      │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    DATA LAYER                               │
│                 (elohim-storage / community nodes)          │
│                                                             │
│  What belongs here:                                         │
│  • Content (learning paths, media, blobs)                   │
│  • Queries (search, filter, aggregate)                      │
│  • Caching (performance optimization)                       │
│  • Replication (community-distributed)                      │
│                                                             │
│  Object count: 1000s-millions (scales with storage)         │
│  Object size: KB to GB                                      │
└─────────────────────────────────────────────────────────────┘
```

### Key Insight

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  The DHT should store COORDINATION data, not CONTENT.       │
│                                                             │
│  "Agent X exists" ✓                                         │
│  "Agent X trusts Agent Y" ✓                                 │
│  "Content Z is held by [A, B, C]" ✓                        │
│  "Agent X witnessed event W" ✓                              │
│                                                             │
│  Content itself: stored in data layer, verified by hash     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Community-Scaled Replication

Instead of:
- DHT automatically replicates everything (chokes)
- Server holds everything (captured)

We get:
- Community members allocate storage to what they value
- Replication scales with community investment
- Creators see network health, not individual nodes
- P2P base layer resists capture

See: [COMMUNITY-COMPUTE.md](./COMMUNITY-COMPUTE.md)

## Implementation Status

| Component | Status | Gap |
|-----------|--------|-----|
| Agent identity (Holochain) | ✅ Works | None |
| Attestations (Holochain) | ✅ Works | Need to use it right |
| Content storage (elohim-storage) | ✅ Works | Single operator |
| Discovery (DHT) | ⚠️ Partial | Need location-only entries |
| Community replication | ❌ Not built | Core gap |
| Family nodes | ❌ Not built | Core gap |
| Creator dashboard | ❌ Not built | Visualization |

## Questions This Document Should Answer

### "Why not just use Holochain as intended?"
Because it doesn't scale. 3000 entries chokes the DHT. The vision is beautiful; the implementation isn't ready for real workloads.

### "Why not just use a server?"
Because capture. A server can be seized, coerced, or shut down. The content dies with the operator.

### "What makes community compute different?"
Embodied investment. The people who value content bear the cost of preserving it. This creates resilience without centralization.

### "How do you bootstrap?"
Start with doorway as the first node. Add family nodes. Build community. Eventually, doorway becomes one node among many, not the central server.

### "What about web users?"
Doorways serve web users. Multiple doorways can exist. Competition and redundancy prevent capture. Native apps connect directly to community.

## Summary

| Question | Client | Agent | Community |
|----------|--------|-------|-----------|
| Who holds data? | Server | Each agent | Distributed by value |
| Who pays for hosting? | Operator | Each agent | Community (visible) |
| What if operator disappears? | Data lost | N/A | Community preserves |
| Who can be captured? | Operator | No one | No single point |
| Does it scale? | Yes ($$$) | No (DHT chokes) | Yes (with community) |
| Can browsers use it? | Yes | No | Yes (via doorways) |

The Community Compute Model isn't a rejection of agent-centric architecture. It's a pragmatic evolution that preserves sovereignty while actually working at scale.
