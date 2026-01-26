# Community Compute Model

## Ephemeral Sovereignty

The real value of Holochain's native experience is local-first computing:

- **Your laptop is the server** — Full application, not a thin client
- **Offline capable** — Work on a plane, in a cabin, during an outage
- **Sync when reconnected** — Changes resolve, state converges
- **First-class participant** — Not dependent on someone else's uptime

But individual sovereignty is *ephemeral*:

- Hardware fails
- Devices get stolen
- Keys get lost
- Disks corrupt
- Life happens

**What makes sovereignty REAL is the community that supports it:**

| Community Function | What It Provides |
|-------------------|------------------|
| **Replication** | Your data survives your device |
| **Backup** | Redundancy across trusted relationships |
| **Recovery** | Social recovery when you lose keys |
| **Amplification** | Your voice reaches beyond your node |
| **Safe Harbor** | Political protection through distribution |

This mirrors embodied human experience: individual freedom is made tangible through relationship, not despite it. A person alone in the wilderness is "free" but vulnerable. A person in community has both sovereignty AND resilience.

The technical architecture must reflect this: **local-first for the experience, community-backed for the permanence**.

## The Native Experience

What we preserve from Holochain's vision:

```
┌─────────────────────────────────────────────────────────────┐
│                     YOUR DEVICE                             │
│                   (laptop, phone, tablet)                   │
│                                                             │
│   ┌───────────────────────────────────────────────────┐    │
│   │              LOCAL APPLICATION                     │    │
│   │                                                    │    │
│   │   Your keys (sovereign identity)                  │    │
│   │   Your data (what's relevant to you)              │    │
│   │   Full UI (not a thin client)                     │    │
│   │   Works offline (first-class, not degraded)       │    │
│   │                                                    │    │
│   └───────────────────────────────────────────────────┘    │
│                          │                                  │
│                          │ when online                      │
│                          ▼                                  │
│   ┌───────────────────────────────────────────────────┐    │
│   │              SYNC ENGINE                           │    │
│   │                                                    │    │
│   │   Changes you made → push to community            │    │
│   │   Changes others made → pull to your device       │    │
│   │   Conflicts → resolve (CRDT/merge strategies)     │    │
│   │   State → converges across all participants       │    │
│   │                                                    │    │
│   └───────────────────────────────────────────────────┘    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ fully distributed
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     COMMUNITY MESH                          │
│                                                             │
│    ┌─────┐     ┌─────┐     ┌─────┐     ┌─────┐            │
│    │Node │◄───►│Node │◄───►│Node │◄───►│Node │            │
│    │ A   │     │ B   │     │ C   │     │ D   │            │
│    └─────┘     └─────┘     └─────┘     └─────┘            │
│        ▲           ▲           ▲           ▲               │
│        │           │           │           │               │
│        └───────────┴───────────┴───────────┘               │
│                          │                                  │
│    No center. No master. No single point of failure.       │
│    Your device is one node among peers.                    │
│    When you're offline, others continue.                   │
│    When you return, you catch up.                          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Offline-First, Not Offline-Tolerant

The distinction matters:

| Offline-Tolerant | Offline-First |
|------------------|---------------|
| Degrades gracefully | Full experience offline |
| Queues actions for later | Completes actions locally |
| Shows stale data | Shows YOUR data (which is current to you) |
| "You're offline" warnings | No warnings needed |
| Server is source of truth | Your device is YOUR source of truth |

When you open your laptop on an airplane:
- Your learning paths are there
- Your progress is there
- Your notes are there
- You can complete lessons, take assessments, write reflections
- When you land and reconnect, it all syncs

This is the experience. Community provides the resilience behind it.

---

## The Problem We're Solving

Three compute models exist today, and none of them work:

| Model | Promise | Reality |
|-------|---------|---------|
| **Client-Server** | Simple, scalable | Captured, centralized, extractive |
| **Agent-Centric (DHT)** | Sovereign, distributed | Chokes at 3000 entries, impractical |
| **Blockchain** | Trustless, immutable | Expensive, slow, environmentally costly |

We need a fourth model: **Community-Scaled Compute**.

## The Insight

Compute and storage should scale with **embodied investment** — the people who care about content should bear the cost of keeping it alive.

This is how human communities already work:
- A church preserves its own records
- A family keeps its own photo albums
- A fan community archives their favorite creator's work

The internet broke this by:
1. Abstracting hosting costs (someone else's computer)
2. Centralizing responsibility (platforms own everything)
3. Severing the relationship between value and preservation

## The Community Compute Model

### Core Principles

1. **Hosting costs something** — and that cost is visible and meaningful
2. **Replication follows relationship** — you preserve what you value
3. **Scale follows community** — more invested people = more capacity
4. **Responsibility is distributed** — no single point of failure or control

### The Family Node

Every participant runs a "family node" — compute they control that serves:

```
┌─────────────────────────────────────────────────────────────┐
│                      MY FAMILY NODE                         │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Layer 1: SOVEREIGN (my data)                         │   │
│  │ • My identity (keys)                                 │   │
│  │ • My content (what I create)                         │   │
│  │ • My attestations (what I've witnessed/signed)       │   │
│  │                                                      │   │
│  │ Storage: ~1-10 GB                                    │   │
│  │ Priority: ALWAYS available                           │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Layer 2: RECIPROCAL (family/close relationships)     │   │
│  │ • Spouse's content (mutual backup)                   │   │
│  │ • Children's data (parental responsibility)          │   │
│  │ • Close collaborators (mutual aid)                   │   │
│  │                                                      │   │
│  │ Storage: ~10-50 GB                                   │   │
│  │ Priority: High availability                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Layer 3: INVESTED (communities I value)              │   │
│  │ • Learning paths I follow                            │   │
│  │ • Podcasters I support                               │   │
│  │ • Movements I believe in                             │   │
│  │                                                      │   │
│  │ Storage: ~50-200 GB (user-configured)                │   │
│  │ Priority: Best-effort, allocated by system           │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Layer 4: GIFT (network commons)                      │   │
│  │ • Small fraction for network health                  │   │
│  │ • Content I may never access                         │   │
│  │ • Supporting the commons                             │   │
│  │                                                      │   │
│  │ Storage: ~5-20 GB (configurable)                     │   │
│  │ Priority: Background, opportunistic                  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### The Listener's View

As a listener/learner/consumer, you see:

```
┌─────────────────────────────────────────────────────────────┐
│  MY COMMUNITY COMPUTE                                       │
│                                                             │
│  Total allocated: 100 GB                                    │
│  Currently used:  67 GB                                     │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ COMMUNITIES I SUPPORT                                │   │
│  │                                                      │   │
│  │ ● Elohim Protocol Learning    12 GB   ████████░░    │   │
│  │ ● Favorite Podcast            28 GB   ████████████░ │   │
│  │ ● Local Church Community       8 GB   ██████░░░░░░  │   │
│  │ ● Family Shared               15 GB   ████████░░░░  │   │
│  │ ● Network Commons              4 GB   ████░░░░░░░░  │   │
│  │                                                      │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  You don't choose WHAT to store.                           │
│  You choose WHO/WHAT to support.                           │
│  The network uses your capacity intelligently.             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

The system decides what shards you hold based on:
- What the communities you support need
- What you've recently accessed (caching)
- Network health optimization
- Your bandwidth/availability patterns

### The Creator's View

As a podcaster/teacher/creator, you see:

```
┌─────────────────────────────────────────────────────────────┐
│  MY CONTENT'S NETWORK HEALTH                                │
│                                                             │
│  "The Elohim Protocol Podcast"                              │
│                                                             │
│  Community Strength                                         │
│  ├── Supporters: 847 nodes                                  │
│  ├── Total allocated: 2.3 TB                               │
│  ├── Average per supporter: 2.7 GB                         │
│  └── Geographic distribution: 23 countries                  │
│                                                             │
│  Replication Health                                         │
│  ├── Episodes 1-50:    ████████████████████  98% (mature)  │
│  ├── Episodes 51-100:  █████████████████░░░  87% (growing) │
│  ├── Latest episode:   ████████░░░░░░░░░░░░  42% (new)     │
│  └── Overall:          █████████████████░░░  89%           │
│                                                             │
│  Resilience Score: STRONG                                   │
│  "Your content could survive loss of any 200 nodes"        │
│                                                             │
│  Distribution Reach                                         │
│  ├── Avg latency to listener: 45ms                         │
│  ├── Edge nodes in: NA, EU, APAC, LATAM, AF                │
│  └── Offline-capable listeners: 234                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

The creator doesn't manage individual replicas. They see:
- How strong is my community's commitment?
- How resilient is my content?
- How well distributed is my reach?

## How This Differs from Existing Systems

### vs. BitTorrent
- BitTorrent: Replication based on popularity (seeders)
- Community: Replication based on relationship (supporters)
- BitTorrent: Anonymous, no identity
- Community: Agent-centric, attestable

### vs. IPFS
- IPFS: You pin what you want to keep
- Community: You support communities, system pins intelligently
- IPFS: Filecoin for incentives (financialized)
- Community: Meaning as incentive (relational)

### vs. Holochain DHT
- DHT: Automatic replication of all entries
- Community: Selective replication by investment
- DHT: Chokes at scale (3000 entries)
- Community: Scales with community size
- DHT: Hidden costs
- Community: Visible, meaningful costs

### vs. Cloud Storage
- Cloud: Centralized, captured, extractive
- Community: Distributed, sovereign, reciprocal
- Cloud: Someone else's computer
- Community: Your community's computers

## The Economics

### Cost Visibility

Every participant sees:
```
This month I contributed:
├── Storage: 67 GB
├── Bandwidth: 12 GB transferred
├── Uptime: 94% (laptop sleeps at night)
└── Estimated value: ~$0.47 equivalent

This supported:
├── 3 creators I follow
├── 1 learning community
├── 12 family members
└── Network commons
```

### No Money Required

This is NOT a cryptocurrency or token system. The "cost" is:
- Real compute you provide
- Real storage you allocate
- Real bandwidth you share

The "value" is:
- Content you care about stays alive
- Community you're part of thrives
- Relationships are strengthened

### Natural Constraints

Dunbar's number applies:
- ~150 meaningful relationships
- Can't meaningfully support 10,000 creators
- Natural limit on allocation complexity
- Forces intentionality

## Technical Architecture

### Content Addressing

All content is addressed by hash:
```
content_id = hash(content)
```
- Content can live anywhere
- Verification is by hash
- Location is separate from identity

### Discovery Layer (Lightweight DHT)

The DHT stores only:
```rust
struct ContentLocation {
    content_hash: Hash,
    holders: Vec<NodeEndpoint>,
    replication_count: u32,
}

struct CommunityMembership {
    community_id: Hash,
    member: AgentPubKey,
    allocation: StorageAllocation,
}
```

NOT the content itself. Just where to find it.

### Storage Layer (elohim-storage)

Each node runs storage:
```
elohim-storage/
├── sovereign/      # My content (always keep)
├── reciprocal/     # Family content (high priority)
├── community/      # Allocated to communities
└── commons/        # Network gift
```

### Replication Protocol

```
When new content is created:

1. Creator publishes to their node
2. Creator's node announces to community
3. Community nodes that have capacity:
   - Check their allocation to this community
   - Request shards they should hold
   - Announce availability to DHT
4. DHT updates holder list
5. Content is now distributed

When content is requested:

1. Requester queries DHT: "Who has content X?"
2. DHT returns: [Node A, Node B, Node C, ...]
3. Requester fetches from nearest/fastest node
4. Optionally: Requester caches locally
```

### Rebalancing

Continuous background process:
```
For each community I support:
    current_allocation = what I'm holding
    target_allocation = my committed capacity
    community_needs = what's under-replicated

    if current < target:
        fetch(highest_priority_unmet_need)
    if current > target:
        release(lowest_priority_content)
```

## Relationship to Holochain

### What We Use Holochain For

1. **Agent Identity** — Keypairs, self-sovereign identity
2. **Attestations** — Signed claims, witnessed events
3. **Trust Graph** — Who trusts whom, relationships
4. **Coordination** — Lightweight DHT for discovery

### What We Don't Use Holochain For

1. **Content Storage** — Too slow, doesn't scale
2. **Heavy Queries** — No query capability
3. **Blob Storage** — Not designed for large data
4. **Real-time Sync** — Gossip is too slow

### The Hybrid

```
┌─────────────────────────────────────────────────────────────┐
│                    APPLICATION                              │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
┌──────────────────────────┐    ┌──────────────────────────┐
│      TRUST LAYER         │    │      DATA LAYER          │
│      (Holochain)         │    │    (elohim-storage)      │
│                          │    │                          │
│ • Agent registration     │    │ • Content storage        │
│ • Attestations           │    │ • Blob management        │
│ • Trust links            │    │ • Replication            │
│ • Content location DHT   │    │ • Queries                │
│                          │    │                          │
│ Entries: ~100s           │    │ Objects: ~1000s-millions │
│ Size: <1KB each          │    │ Size: KB to GB           │
└──────────────────────────┘    └──────────────────────────┘
```

## Migration Path

### Phase 1: Current (Doorway + Storage)
- Doorway as gateway
- elohim-storage as data layer
- Holochain for attestations only
- Works today, single operator

### Phase 2: Community Nodes
- Family nodes can join
- Replication within trusted network
- Creator sees community health
- Still needs bootstrap node

### Phase 3: Full P2P
- Discovery via lightweight DHT
- No required central node
- Community self-sustaining
- Creator can disappear, content survives

## Economic Layer: hREA for Value Flows

The economic layer uses hREA/ValueFlows — not for tracking content (that choked the DHT), but for tracking **value flows around compute**.

### What hREA Tracks

```
Economic Events (lightweight, DHT can handle):

• "Agent X provided 50GB storage for Community Y"
• "Wealthy user paid overage → Commons Fund"
• "Commons Fund subsidized edge node hardware"
• "Community contributed bandwidth to network"
```

### Constitutional Rates

Rates are not market-driven extraction. They are:

- **Base allocation**: FREE — everyone gets basic participation
- **Expanded capacity**: RENT — at constitutionally-set rates
- **Progressive**: Ability to pay considered
- **Localized**: Context of where/who compute serves
- **Negotiated**: Through governance process, not dictated

### Commons Fund Flows

"Rent" doesn't go to shareholders. It flows to:

- **Operating costs**: Actual network infrastructure needs
- **Edge subsidies**: Hardware for communities that can't afford it
- **Onboarding support**: Help new members join
- **Growth fund**: Expand the commons

### Solidarity Economics

```
Wealthy User                 Commons Fund               Edge User
(can afford more)            (held in trust)            (needs support)
     │                            │                          │
     │── pays overage ───────────►│                          │
     │   (constitutional rate)    │                          │
     │                            │── subsidizes ───────────►│
     │                            │   hardware, onboarding   │
     │                            │                          │
Still sovereign              Transparent              Now sovereign
Full participant             Democratically governed  Full participant
```

## Explainability as Justice

Outcomes may not make everyone "perfectly happy" — but they are **explainable**.

### Every Decision Has a Trace

```
"Your storage contribution increased by 15%"

WHY:
├── Community added 12 new edge nodes this quarter
├── Constitutional council approved rate adjustment
└── Proposal [hash], Vote: 7-2, Effective: 2025-02-01

VERIFY:
├── Proposal attestation on DHT
├── Council member signatures
└── Full deliberation record
```

### Education Over Compliance

If you don't understand, an Elohim agent generates a personalized learning path:

```
┌─────────────────────────────────────────────────────────────┐
│  📚 Personalized Path: Understanding Compute Economics      │
│                                                             │
│  Generated for you based on:                                │
│  • Your current understanding                               │
│  • The specific decision you're questioning                │
│  • Your learning style                                      │
│                                                             │
│  1. What are commons funds?                    [15 min]     │
│  2. Constitutional rate-setting                [20 min]     │
│  3. Edge subsidies and inclusion               [15 min]     │
│  4. How to participate in governance           [10 min]     │
│                                                             │
│  [Start Learning]                                           │
└─────────────────────────────────────────────────────────────┘
```

### Still Disagree?

The system provides pathways:
- Participate in next constitutional review
- Propose amendments
- Your voice is in the system

**Explainability + Education + Participation = Legitimacy**

## Values Aligned TO Layers (Negotiated)

Elohim values aren't imposed top-down. They're aligned **to** the layer of operation, in negotiation with lower and higher layers.

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│   DATA LAYER (Sovereignty)                                  │
│   "I can only give what I have"                            │
│        │                                                    │
│        │ constrains / enables                               │
│        ▼                                                    │
│   COMPUTE LAYER (Solidarity)                                │
│   "Can't demand more than sovereignty allows"              │
│   "But sovereignty needs community to persist"             │
│        │                                                    │
│        │ constrains / enables                               │
│        ▼                                                    │
│   ECONOMIC LAYER (Constitutional Justice)                   │
│   "Rates must reflect compute reality"                     │
│   "But compute needs resources to sustain"                 │
│        │                                                    │
│        │ constrains / enables                               │
│        ▼                                                    │
│   TRUST LAYER (Transparency)                                │
│   "Decisions must be verifiable"                           │
│   "But verification has costs"                             │
│        │                                                    │
│        │ constrains / enables                               │
│        ▼                                                    │
│   APPLICATION LAYER (Agency)                                │
│   "Education over compliance"                              │
│   "But agency requires understanding"                      │
│        │                                                    │
│        │ constrains / enables                               │
│        ▼                                                    │
│   GOVERNANCE LAYER (Participation)                          │
│   "Your voice in the system"                               │
│   "But participation requires all layers working"          │
│                                                             │
│   Each layer negotiates with adjacent layers.               │
│   Alignment emerges from negotiation, not imposition.       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

| Layer | Value | Negotiates With |
|-------|-------|-----------------|
| Data | Sovereignty | Compute (what can I share?) |
| Compute | Solidarity | Data (what's available?) + Economic (what's sustainable?) |
| Economic | Constitutional Justice | Compute (what's real?) + Trust (what's verifiable?) |
| Trust | Transparency | Economic (what's worth proving?) + Application (what's usable?) |
| Application | Agency | Trust (what's provable?) + Governance (what's changeable?) |
| Governance | Participation | All layers (what's actually possible?) |

When values are sufficiently aligned to the layer they operate in, mutually beneficial outcomes emerge. Not perfect happiness — but **explainable outcomes** that participants can understand, verify, and work to change.

## Technical Implementation Mapping

This section maps the vision above to concrete implementation in the codebase.

> **See also**: [P2P-DATAPLANE.md](./P2P-DATAPLANE.md) for the comprehensive P2P architecture.

### Family Node Layers → elohim-storage Sovereignty Modes

| Family Node Layer | Sovereignty Mode | Implementation |
|-------------------|-----------------|----------------|
| Layer 1: SOVEREIGN | `private` | Encrypted at rest, only my devices |
| Layer 2: RECIPROCAL | `invited` / `local` | Shared with named agents / family cluster |
| Layer 3: INVESTED | `neighborhood` | Community replication pools |
| Layer 4: GIFT | `commons` | Anyone willing to store |

```rust
// elohim-storage/src/reach.rs
pub enum Reach {
    Private,      // Layer 1: only creator
    Invited,      // Layer 2: named relationships
    Local,        // Layer 2: family cluster
    Neighborhood, // Layer 3: extended community
    Municipal,    // Layer 3+: regional
    Commons,      // Layer 4: public
}
```

### Replication Follows Relationship → Reach-Based Distribution

The vision "you preserve what you value" maps to reach-based gating:

```rust
// Who can receive this content?
fn can_replicate_to(&self, peer: &PeerId, content: &BlobMetadata) -> bool {
    match content.reach {
        Reach::Private => self.is_my_device(peer),
        Reach::Invited => self.has_relationship(peer, content.owner),
        Reach::Local => self.in_same_cluster(peer),
        Reach::Neighborhood => self.in_trust_network(peer),
        Reach::Commons => true,
    }
}
```

### Community Mesh → libp2p Peer Network

The community mesh diagram maps directly to libp2p:

| Vision | Implementation |
|--------|----------------|
| Node connections | libp2p streams |
| No center | Kademlia DHT |
| Geographic distribution | mDNS + relay nodes |
| Offline resilience | Local-first Automerge |

### Sync Engine → Automerge Integration

The vision's sync engine uses Automerge 3.0:

| Feature | Implementation |
|---------|----------------|
| "Changes you made" | Automerge local mutations |
| "Changes others made" | Automerge remote sync |
| "Conflicts resolve" | CRDT automatic merge |
| "State converges" | Eventual consistency |

See [SYNC-ENGINE.md](./SYNC-ENGINE.md) for detailed design.

### Discovery Layer → ContentLocation DHT

The lightweight DHT for discovery:

```rust
// dna/infrastructure/zomes/infrastructure/src/lib.rs
#[hdk_entry_helper]
pub struct ContentLocation {
    pub content_hash: String,
    pub holders: Vec<PeerId>,
    pub reach: Reach,
    pub replication_count: u32,
}
```

### Creator's View → Replication Health API

The creator dashboard maps to elohim-storage APIs:

```typescript
// sdk/src/replication-health.ts
interface ReplicationHealth {
  contentId: string;
  supporters: number;      // Community nodes storing this
  totalAllocated: number;  // Total GB committed
  replicationPercent: number;
  geographicDistribution: Map<string, number>;
  resilienceScore: 'weak' | 'moderate' | 'strong';
}
```

### Key Implementation Files

| Component | Primary Files |
|-----------|---------------|
| Reach enforcement | `elohim-storage/src/reach.rs` (planned) |
| P2P networking | `elohim-storage/src/p2p/mod.rs` |
| Sync protocol | `elohim-storage/src/sync/` (planned) |
| Content location | `dna/infrastructure/` |
| Bootstrap | `doorway/src/routes/signal.rs` |

---

## Open Questions

1. **Incentive Bootstrapping** — How do early supporters get rewarded?
2. **Free-rider Mitigation** — What if someone only consumes?
3. **Sybil Resistance** — How to prevent fake nodes?
4. **Offline Resilience** — What if most nodes are laptops?
5. **Content Moderation** — What if community hosts harmful content?
6. **Constitutional Process** — How are rates actually negotiated?
7. **Cross-Community Economics** — How do different communities interoperate?

## Conclusion

The Community Compute Model isn't just a technical architecture — it's a social architecture.

It recognizes that:
- Compute is not free
- Abstraction enables extraction
- Relationships create responsibility
- Communities can self-organize

The goal is not to build a better cloud. It's to make hosting as natural and meaningful as any other form of community participation.
