# The Elohim Protocol: Content Addressing for Human Flourishing
## A Specification for Value-Bearing, Governance-Aware, Knowledge-Coupled Content Distribution

### Version 0.1 | "A Dollar from Exploitation Equals a Dollar from Care" — The Protocol That Refuses to Forget

---

## Executive Summary

Existing federated protocols address content as bytes. AT Protocol addresses content as signed records in personal repositories. ActivityPub addresses content as URLs on servers. IPFS addresses content as hashes in a global namespace. None of them address content as knowledge that carries its economic meaning and governance context.

The Elohim Protocol introduces the **Elohim Protocol Reference (EPR)** — a content reference that always couples three dimensions:

- **Lamad** (knowledge): What is this content, how does it relate to other knowledge, what engagement exists?
- **Shefa** (value): Who stewards this, what recognition has accumulated, what economic flows attach?
- **Qahal** (governance): What authority ratified this, what reach level applies, what constitutional layer binds it?

These three dimensions are inseparable in the same way that information, value, and responsibility are inseparable in a flourishing society. The protocol structurally refuses to serve bytes without honoring the stewardship and governance that surrounds them — eliminating the "value-blind content" problem that plagues existing systems.

---

## Part I: The Protocol Gap

### Why Existing Protocols Fail

*"A dollar spent on weapons is identical to one spent on medicine — it's a system without values feedback."*

This observation from the manifesto about currency applies equally to content protocols. A CID on IPFS carries no values. An AT Protocol record carries one owner's signature. An ActivityPub URL carries one server's authority. None of them can express:

- **Multiple non-owning stewards** sharing fractional responsibility for a piece of knowledge
- **Governance-gated visibility** enforced at the protocol layer, not the application layer
- **Recognition flows** triggered by content delivery — the act of serving knowledge generating value for those who care for it
- **Constitutional verification** — proving that the agent serving content operates under a verified constitutional stack

| Protocol | Content Addressing | Ownership Model | Governance | Value Flow | P2P Native |
|----------|-------------------|-----------------|------------|------------|------------|
| AT Protocol | CIDs in signed repos | Single owner (one DID = one repo) | None (app-layer labeling) | None | No |
| ActivityPub | URLs on servers | Server owns URL | Advisory (`to`/`cc` fields) | None | No |
| IPFS | CIDv1 (content hash) | None (no ownership) | None (no access control) | None | Yes |
| Holochain | EntryHash + ActionHash | Agent provenance | DHT validation zomes | None native | Yes |
| **Elohim Protocol** | **EPR (three-pillar reference)** | **Multi-steward, non-ownership** | **Constitutional, protocol-level** | **Recognition flows on delivery** | **Yes** |

### What "Value-Blind Content" Costs

When content is addressed without its economic and governance context:

- **Stewards are invisible.** Someone curated, translated, or maintained this knowledge. The protocol forgets them.
- **Governance is optional.** Reach levels, constitutional constraints, and community ratification become application-layer conventions that any client can bypass.
- **Value extraction is frictionless.** Content can be served, cached, and redistributed without any recognition flowing to those who created and care for it.
- **Context is severed.** A concept torn from its knowledge graph, its learning paths, its prerequisite relationships is diminished — like a verse torn from its chapter.

The Elohim Protocol makes these costs structurally impossible. Not through policy. Through protocol.

---

## Part II: The Elohim Protocol Reference (EPR)

The EPR is the fundamental addressable unit. Every piece of content, every learning path, every assessment, every economic event is referenced through an EPR. An EPR always carries all three pillar dimensions.

### Three Tiers of Resolution

The EPR exists in three tiers, each serving a different availability and privacy requirement:

```
Tier 1: EPR Head        (~500 bytes, gossipped across DHT)
  ↓ resolve
Tier 2: EPR Document    (~5-50 KB, cached by interested peers)
  ↓ resolve
Tier 3: Content Bytes   (any size, steward-delivered, shard-encoded)
```

This tiered model solves the **steward offline problem**: even when every steward is asleep, Tier 1 EPR Heads remain available in the DHT, allowing peers to discover, preview, and queue content for later retrieval.

### Tier 1: EPR Head

The "magnet link" of the Elohim Protocol. Small enough to store in a Kademlia DHT record, embed in a feed message, or gossip to any peer. Contains just enough information to render a preview card, assess access rights, and locate delivery peers.

```
EPR Head {
  ── Identity ──────────────────────────────────────────────
  id:              String        // stable slug, survives updates
  cid:             CIDv1         // SHA256 of current content bytes (raw codec 0x55)
  version:         u64           // monotonic, increments on update

  ── Lamad (Knowledge) ────────────────────────────────────
  contentType:     ContentType   // concept | unit | path | assessment |
                                 // scenario | composite | simulation | ...
  title:           String        // human-readable, max 256 bytes
  previewCid:      CIDv1?        // thumbnail or summary blob

  ── Shefa (Value) ────────────────────────────────────────
  stewards: [{
    presenceId:    String        // ContributorPresence ID (resolves to DID)
    did:           DID           // steward identity (denormalized for resolution)
    ratio:         f32           // allocation (all ratios sum to 1.0)
    contribution:  Contribution  // original_creator | curator | translator |
                                 // editor | maintainer | inherited
  }]
  recognition:     f64           // accumulated recognition on this content

  ── Qahal (Governance) ───────────────────────────────────
  reach:           Reach         // private | self | intimate | trusted |
                                 // familiar | community | public | commons
  layer:           ConstitutionalLayer
                                 // individual(1)..global(7)
  governance:      Governance    // active | disputed | pending_review | superseded
  ratifier:        DID?          // Elohim agent that ratified this state
                                 // Ratifiers are selected via cryptographic sortition
                                 // per Constitutional Council Protocol — not appointed

  ── Provenance ───────────────────────────────────────────
  author:          DID           // original creator
  created:         u64           // Unix milliseconds
  updated:         u64           // Unix milliseconds
  signature:       [u8; 64]      // Ed25519 over canonical MessagePack of above fields
}
```

**Canonical encoding**: Fields are serialized in the order listed above using MessagePack (`rmp_serde`). The signature is computed over the MessagePack encoding of all fields *except* the signature itself. This is consistent with the existing shard and sync protocol codecs.

**DHT storage**: EPR Heads are stored in Kademlia using the content `id` as the key. Multiple versions may exist; peers accept the version with the highest `version` number whose `signature` verifies against a known steward DID.

**Size budget**: A typical EPR Head with 3 stewards fits in ~400-600 bytes — well within Kademlia's recommended record size limit.

### Tier 2: EPR Document

Full pillar context for content that a peer has affinity with. Cached locally by peers who are learning from, stewarding, or governing this content. Too large and too sensitive for DHT gossip; retrieved directly from known peers.

```
EPR Document {
  head:            EPR Head      // embedded, not referenced

  ── Lamad (Full Knowledge Context) ───────────────────────
  relationships: [{
    targetId:      String        // related content ID
    targetCid:     CIDv1         // integrity of related content
    type:          Relationship  // CONTAINS | REFERENCES | DEPENDS_ON |
                                 // CONTRASTS_WITH | CONCEPTUALLY_RELATED |
                                 // PRECEDES | PREREQUISITE | CREATED_BY | ...
  }]
  tags:            [String]
  contentFormat:   ContentFormat  // markdown | sophia-quiz-json | gherkin |
                                 // html5-app | composite-layout | ...
  pathMemberships: [{
    pathId:        String
    stepIndex:     u32
    chapterId:     String?
  }]
  attestationsRequired: [String] // gate conditions for access
  bloomLevel:      BloomLevel?   // not_started | remember | understand |
                                 // apply | analyze | evaluate | create

  ── Shefa (Full Economic Context) ────────────────────────
  allocations: [{                // full StewardshipAllocation records
    id:            String
    stewardDid:    DID
    presenceId:    String        // ContributorPresence ID
    ratio:         f32
    method:        AllocationMethod
                                 // manual | computed | negotiated
    contribution:  Contribution
    governance:    Governance
    ratifiedAt:    u64?
    ratifierId:    DID?
    recognition:   f64           // accumulated for this steward
    evidence:      Value?        // contribution evidence JSON
  }]
  economicEvents: [{             // REA events touching this content
    eventType:     String
    resourceType:  TokenType     // care | time | learning | steward |
                                 // culture | infrastructure
    agents: {
      primary:     DID
      supported:   DID?
      witnessed:   [DID]
      benefited:   [DID]
    }
    timestamp:     u64
  }]
  recognitionPolicy: {           // how recognition distributes on interaction
    onView:        f64           // micro-recognition on content view
    onComplete:    f64           // on path step completion
    onAttest:      f64           // on attestation earned
    onTeach:       f64           // on sharing/teaching forward
    distribution:  String        // "proportional" | "equal" | "primary_weighted"
  }

  ── Qahal (Full Governance Context) ──────────────────────
  constitution: {
    layer:         ConstitutionalLayer
    version:       String        // semver
    hash:          String        // SHA256, verifiable against blockchain anchor
    contextId:     String?       // community_id, family_id, etc.
  }
  policyChain: [{                // composable restriction stack
    source:        PolicySource  // org | guardian | elohim | subject
    restrictions:  [String]      // policy IDs applied
  }]
  reachNegotiation: {            // if reach was negotiated
    requestedReach: Reach
    grantedReach:   Reach
    reasoning:      String       // Elohim's negotiation explanation
    trustScore:     f32?         // at time of negotiation
  }?
  disputeHistory: [{             // governance state transitions
    from:          Governance
    to:            Governance
    reason:        String
    initiator:     DID
    timestamp:     u64
  }]

  ── Federation ───────────────────────────────────────────
  knownLocations: [{
    doorwayDid:    DID?          // doorway serving this content
    peerId:        String        // libp2p PeerId (base58)
    multiaddrs:    [String]      // libp2p multiaddrs
    lastSeen:      u64           // Unix milliseconds
    tier:          NodeTier      // network | home_node | home_cluster | laptop
    capabilities:  [String]      // ["shard", "sync", "relay"]
  }]
}
```

### Tier 3: Content Bytes

The actual content blob. Retrieved via the existing `/elohim/shard/1.0.0` protocol. Verified against the EPR Head's `cid` field.

```
Content Bytes:
  Retrieval:     ShardRequest::Get { hash: epr_head.cid }
  Verification:  sha256(received_bytes) == epr_head.cid
  Sharding:      Per ShardManifest encoding (none | chunked | rs-4-7)
  Access:        Governed by epr_head.reach + epr_document.policyChain
  Delivery:      Any peer holding the blob may serve it
  Caching:       Cache-Control: public, max-age=31536000, immutable
                 (content-addressed = immutable)
```

The existing `BlobStore`, `ShardManifest`, and doorway blob-serving infrastructure (`/blob/{address}`) serve Tier 3 without modification. The EPR layers above are additive.

---

## Part III: Pillar Coupling — The Protocol's Immune System

### The Four Coupling Rules

**Rule 1: No value-blind content.**

Every EPR Head carries a stewardship summary. Content with zero stewards is a protocol violation. At minimum, the original creator is assigned as bootstrap steward at ratio 1.0. This mirrors the manifesto's requirement that "attribution must be maintained for ALL content, even before creator joins."

```
VALID:    stewards: [{ did: "did:key:z6Mk...", ratio: 1.0, contribution: "original_creator" }]
INVALID:  stewards: []
```

**Rule 2: No governance-free content.**

Every EPR Head carries reach and constitutional layer. Content without explicit governance context defaults to `reach: private, layer: individual` — the most restrictive setting, not the most permissive. This prevents accidental exposure. You must explicitly grant reach; the protocol does not assume openness.

```
DEFAULT:  reach: "private", layer: "individual"
          (content is invisible to everyone except the author)
EXPLICIT: reach: "community", layer: "community"
          (content visible to community members, governed at community layer)
```

**Rule 3: Content delivery triggers recognition.**

When a peer serves Tier 3 Content Bytes in response to a valid EPR resolution, the serving peer logs a recognition event against the EPR's stewardship allocations. Recognition flows proportionally to steward ratios. This is not a financial transaction — it is a protocol-level acknowledgment that serving knowledge generates value for those who care for it.

The recognition event is recorded as an REA economic event:
```
EconomicEvent {
  eventType: "content_delivery"
  resourceType: "learning"        // or "care" for care-economy content
  agents: {
    primary:   serving_peer_did
    benefited: [steward_dids]     // per allocation ratio
  }
}
```

**Rule 4: Dimensions queryable independently, coupled structurally.**

While the EPR always carries all three dimensions, queries may filter by any single dimension:
- "All content I steward" (shefa query by steward DID)
- "Community-reach concepts" (qahal + lamad filter)
- "Recognition flows for this path" (shefa aggregate across path steps)

The coupling is structural — the data is always present — but the query interface respects that different contexts foreground different dimensions.

### Why Always-Coupled Matters

Consider a piece of content — say, the manifesto's chapter on economic architecture — addressed only by its CID:

`bafkreih7mdkh5gn3mkjbagj3n22w2eyrwya3eqzhdqr2t6lnk7iyn2blea`

This CID tells you nothing about:
- Who wrote it and who maintains it (Matthew Dowell as original creator, Susan as editor)
- Who can see it (public reach, community constitutional layer)
- What knowledge it connects to (builds on "poverty of currency" concept, contrasts with "engagement optimization")
- What happens when you learn from it (recognition flows to stewards, engagement tracked for affinity)

The EPR Head for the same content:
```
{
  id: "manifesto-economic-architecture",
  cid: "bafkreih7mdkh5gn3mkjbagj3n22w2eyrwya3eqzhdqr2t6lnk7iyn2blea",
  contentType: "unit",
  title: "Economic Architecture: The Poverty of Currency",
  stewards: [
    { did: "did:web:hosted.elohim.host:humans:matthew", ratio: 0.7, contribution: "original_creator" },
    { did: "did:web:hosted.elohim.host:humans:susan", ratio: 0.3, contribution: "editor" }
  ],
  reach: "public",
  layer: "global",
  governance: "active",
  ...
}
```

The content is the same bytes. But the EPR carries its meaning.

### Attestations: The Fourth Pillar Record

Attestations are the human-sourced signal that the manifesto places alongside content, stewardship, and governance as constitutionally protected records. An attestation is a signed statement about a human's relationship with knowledge — not a credential, not a grade, but a witnessed claim.

```
Attestation {
  id:              String        // unique identifier
  issuer:          DID           // who attests (may be self, peer, or Elohim agent)
  subject:         DID           // the human being attested
  contentCid:      CIDv1         // the content this attestation concerns
  claim:           String        // "engagement" | "reflection" | "application" | "teaching"
  evidence:        String?       // optional narrative or hash of assessment result
  timestamp:       u64           // Unix milliseconds
  signature:       [u8; 64]      // Ed25519 over canonical MessagePack of above fields
}
```

**Constitutional protection**: Attestations are append-only and irrevocable. Once issued, an attestation cannot be revoked, sold, transferred, or manipulated. This is enforced by storage on Holochain source chains (which are inherently append-only and tamper-evident) and by the EPR protocol refusing to process `Delete` or `Update` operations on attestation records.

Attestations enable access to gated content (`attestationsRequired` in EPR Documents) and contribute to the three meaning maps (knowledge, love, self). They are the protocol's alternative to engagement metrics — human witness replaces algorithmic score.

### Recognition Token Circulation

Recognition tokens generated by Rule 3 (content delivery triggers recognition) are subject to circulation rules defined in the Shefa economic layer:

- **Demurrage**: Recognition tokens decay over time to encourage circulation. Tokens not redistributed within constitutional time bounds lose value, preventing infinite accumulation. The specific decay rate and time bounds are governance-configurable at the community constitutional layer.
- **Accumulation thresholds**: Recognition beyond constitutional thresholds triggers redistribution to the next community or to infrastructure commons, per the manifesto's wealth transition architecture.
- **Token provenance**: Every recognition token carries its origin (which content delivery, which stewards benefited, which community context). Tokens are not value-blind — they remember where they came from.

The specific parameters (decay rates, thresholds, redistribution rules) are defined per-community in their constitutional documents and enforced by Elohim agents operating under those constitutions. This specification defines the recognition *event* format; the Shefa Economic Protocol (future specification) defines the circulation *rules*.

---

## Part IV: Resolution Protocol — `/elohim/epr/1.0.0`

### Protocol Definition

A new libp2p request-response protocol, following the same patterns as the existing shard and sync protocols.

```
Protocol ID:  "/elohim/epr/1.0.0"
Codec:        EprCodec (4-byte big-endian length + MessagePack payload)
Max Request:  1 MB
Max Response: 16 MB (for large EPR Documents with many relationships)
```

### Request Messages

```rust
pub enum EprRequest {
    /// Resolve the latest EPR Head for a content ID
    Resolve {
        id: String,
    },

    /// Resolve a specific version
    ResolveVersion {
        id: String,
        version: u64,
    },

    /// Get the full EPR Document
    GetDocument {
        id: String,
    },

    /// Search for content matching filters
    Search {
        query: Option<String>,          // text search
        content_type: Option<String>,   // filter by type
        reach: Option<String>,          // filter by max reach
        steward_did: Option<String>,    // filter by steward
        tags: Vec<String>,              // filter by tags
        offset: u64,
        limit: u64,
    },

    /// Announce a new or updated EPR Head to the network
    Announce {
        head: Vec<u8>,                  // MessagePack-encoded EPR Head
    },

    /// Batch resolve multiple EPR Heads (for composite content)
    ResolveBatch {
        ids: Vec<String>,
    },
}
```

### Response Messages

```rust
pub enum EprResponse {
    /// Single EPR Head
    Head(Vec<u8>),                      // MessagePack-encoded EPR Head

    /// Full EPR Document
    Document(Vec<u8>),                  // MessagePack-encoded EPR Document

    /// Search results (array of EPR Heads)
    SearchResults {
        heads: Vec<Vec<u8>>,
        total: u64,
        has_more: bool,
    },

    /// Announcement acknowledgment
    Announced {
        accepted: bool,
        reason: Option<String>,
    },

    /// Batch of EPR Heads
    HeadBatch(Vec<Vec<u8>>),

    /// Content not found
    NotFound,

    /// Access denied (governance-gated)
    AccessDenied {
        required_reach: String,
        current_reach: String,
        reason: String,
    },

    /// Error
    Error(String),
}
```

### Resolution Flow

A peer wants to access content with ID `manifesto-economic-architecture`:

```
0. CONSTITUTIONAL VERIFICATION (prerequisite)
   Before serving ANY EPR resolution, a peer verifies its own constitutional stack:
   - Load constitutional documents (Individual → Global)
   - Verify each layer's hash against blockchain anchor / DHT consensus
   - If any layer fails verification: refuse to serve, report to network
   - If all layers verify: proceed with resolution
   This ensures every serving peer operates under a verified constitutional stack.
   Peers that cannot verify their constitution are rejected by the network.

1. DISCOVERY
   Peer → Kademlia DHT: GET key="epr:manifesto-economic-architecture"
   DHT → Peer: EPR Head (Tier 1, ~500 bytes)

2. AUTHORIZATION CHECK
   Peer evaluates: Can I see this?
   - epr_head.reach == "public" → YES (no attestation required)
   - epr_head.reach == "trusted" → check relationship with steward DIDs
   - epr_head.reach == "intimate" → check mutual attestation

3. CONTEXT RESOLUTION (if authorized)
   Peer → any known peer: EprRequest::GetDocument { id }
   Peer receives: EPR Document (Tier 2, full pillar context)
   Peer now knows: knownLocations, relationships, policy chain

4. CONTENT RETRIEVAL (if context confirms access)
   Peer → steward/cache peer: ShardRequest::Get { hash: epr_head.cid }
   Peer receives: Content Bytes (Tier 3)
   Peer verifies: sha256(bytes) == epr_head.cid

5. RECOGNITION (on successful delivery)
   Serving peer logs: EconomicEvent { eventType: "content_delivery", ... }
   Recognition distributes proportionally to steward ratios
```

### Steward Offline Fallback

```
Tier 1 (EPR Heads):
  Stored in Kademlia DHT across all participating peers.
  Available as long as ANY peer in the network is online.
  TTL: republished every 24 hours by stewards.
  Stale detection: if updated > 30 days ago and no steward seen, mark as "dormant."

Tier 2 (EPR Documents):
  Cached by peers with affinity (learners on a path, community members, etc.).
  Survives individual steward downtime.
  Cache invalidation: version number in EPR Head > cached document's head version.

Tier 3 (Content Bytes):
  Reed-Solomon sharded (4 data + 3 parity) for blobs > 10MB.
  Recoverable from any 4 of 7 shard holders.
  Doorway servers maintain projection caches for hot content.
  Content-addressed = immutable. Any peer who ever cached the blob can serve it.
```

---

## Part V: Feed Protocol — `/elohim/feed/1.0.0`

*"Free speech does not mean free reach. There is no right to algorithmic amplification."*

Feeds in the Elohim Protocol are curated, not algorithmic. They are closer to a river than a recommendation engine — content flows through channels that communities and stewards maintain, not through engagement-optimized algorithms.

### Protocol Definition

```
Protocol ID:  "/elohim/feed/1.0.0"
Codec:        FeedCodec (4-byte big-endian length + MessagePack payload)
Max Request:  64 KB
Max Response: 1 MB (feed pages contain EPR Heads, not full content)
```

### Feed Types

| Feed Type | Curator | What It Contains | Example |
|-----------|---------|-----------------|---------|
| `path` | Path stewards | EPR Heads of steps in a learning path, in order | The `elohim-protocol` path is the "homepage feed" |
| `steward` | A steward's portfolio | EPR Heads of content they steward | Follow a creator to see their work |
| `community` | Qahal governance | EPR Heads curated by community consensus | A church community's learning feed |
| `layer` | Constitutional layer | EPR Heads at a governance layer | All community-level governance decisions |

### Request Messages

```rust
pub enum FeedRequest {
    /// Subscribe to a feed (push updates)
    Subscribe {
        feed_type: String,              // "path" | "steward" | "community" | "layer"
        feed_id: String,                // path ID, steward DID, community ID, layer name
        since: Option<u64>,             // Unix ms — only updates after this time
        filters: FeedFilters,
    },

    /// Unsubscribe from a feed
    Unsubscribe {
        subscription_id: String,
    },

    /// Pull a page of feed content (for catch-up or non-subscribers)
    GetPage {
        feed_type: String,
        feed_id: String,
        offset: u64,
        limit: u64,
        filters: FeedFilters,
    },
}

pub struct FeedFilters {
    pub content_types: Vec<String>,     // filter by content type
    pub min_reach: Option<String>,      // don't send content I can't see
    pub max_reach: Option<String>,      // don't send public when I want intimate
    pub tags: Vec<String>,              // topic filters
}
```

### Response Messages

```rust
pub enum FeedResponse {
    /// Subscription confirmed
    Subscribed {
        subscription_id: String,
        feed_type: String,
        feed_id: String,
    },

    /// Unsubscribed
    Unsubscribed,

    /// Feed update (pushed to subscribers)
    Update {
        subscription_id: String,
        entries: Vec<Vec<u8>>,          // MessagePack-encoded EPR Heads
        sequence: u64,                  // monotonic for ordering
    },

    /// Feed page (response to GetPage)
    Page {
        entries: Vec<Vec<u8>>,          // MessagePack-encoded EPR Heads
        total: u64,
        has_more: bool,
    },

    /// Error
    Error(String),
}
```

### Push and Pull

**Push**: Subscribers receive `FeedResponse::Update` messages whenever the feed changes. A path steward adds a new chapter → all subscribers to that path feed receive the new step's EPR Head. A community ratifies new content → all subscribers to that community feed receive the update.

**Pull**: Any peer can request `FeedRequest::GetPage` without subscribing. This enables catch-up after being offline, browsing feeds before committing to subscribe, and rendering feed previews in composite content.

### No Engagement Optimization

Feed ordering is determined by the feed type's natural order:
- **Path feeds**: step order (the learning sequence the steward designed)
- **Steward feeds**: chronological (most recent first)
- **Community feeds**: governance-determined order (community consensus on what surfaces)
- **Layer feeds**: chronological with severity weighting (disputed items surface)

There is no engagement score, no "trending" algorithm, no attention optimization. The protocol has no mechanism for it — this is not policy, it is architecture.

---

## Part VI: Composite Content — The Protocol Eating Its Own Landing Page

### The Problem

The landing page at `/` and the content system at `/lamad` are two disconnected applications sharing a domain. The landing page is static Angular components. The content system is dynamic EPR resolution. A visitor at the front door cannot see what's inside.

### The Solution: `composite` Content Type

A composite EPR is a ContentNode whose "body" is a layout of references to other EPR Heads. It is the protocol's equivalent of a WordPress page template — except it lives inside the content graph, carries stewardship, respects governance, and triggers recognition like any other content.

```
EPR Head {
  id:            "elohim-protocol-home"
  cid:           <CID of the layout descriptor>
  contentType:   "composite"
  title:         "Elohim Protocol"
  stewards:      [{ did: "did:web:elohim-protocol.org:system", ratio: 1.0,
                    contribution: "maintainer" }]
  reach:         "commons"           // maximally visible — the front door
  layer:         "global"
  governance:    "active"
}
```

### Layout Descriptor

The Tier 3 Content Bytes for a composite EPR is a JSON layout descriptor:

```json
{
  "layout": "sections",
  "sections": [
    {
      "type": "hero",
      "eprId": "manifesto",
      "display": "excerpt",
      "excerpt": "first_paragraph"
    },
    {
      "type": "featured_paths",
      "eprIds": [
        "elohim-protocol-path",
        "governance-policy-maker",
        "know-thyself-path"
      ],
      "display": "cards"
    },
    {
      "type": "content_grid",
      "query": {
        "contentType": "unit",
        "tags": ["crisis"],
        "reach": "public",
        "limit": 6
      },
      "display": "cards"
    },
    {
      "type": "steward_spotlight",
      "stewardDids": [
        "did:web:hosted.elohim.host:humans:matthew"
      ],
      "display": "portfolio_summary"
    },
    {
      "type": "community_feed",
      "feedType": "community",
      "feedId": "elohim-protocol-community",
      "limit": 5,
      "display": "feed_list"
    }
  ]
}
```

### Rendering a Composite EPR

1. **Resolve**: `EprRequest::Resolve { id: "elohim-protocol-home" }` → EPR Head
2. **Fetch layout**: `ShardRequest::Get { hash: epr_head.cid }` → layout descriptor JSON
3. **Batch resolve**: `EprRequest::ResolveBatch { ids: [...all eprIds from layout...] }` → array of EPR Heads
4. **Render sections**: Each section type maps to an Angular component:
   - `hero` → renders excerpt from referenced EPR's preview
   - `featured_paths` → renders path cards with thumbnail, title, steward info from EPR Heads
   - `content_grid` → renders content cards from EPR search results
   - `steward_spotlight` → renders steward portfolio from EPR search by steward DID
   - `community_feed` → renders feed entries from `FeedRequest::GetPage`
5. **Click-through**: Each card links to the full EPR resolution path — `/lamad/path/:pathId` or `/lamad/resource/:resourceId`

### What This Achieves

The landing page is no longer a static artifact maintained separately from the content it describes. It is a **live view** of the content graph — updated when paths are added, when stewards change, when communities curate new material. The same pillar coupling applies: the landing page itself has stewards, governance, and recognition flows.

A visitor sees preview cards rendered from EPR Heads — available even if specific stewards are offline. Clicking any card begins a full EPR resolution, descending from Tier 1 through Tier 2 to Tier 3. The front door is inside the system.

---

## Part VII: Relationship to Existing Implementation

The Elohim Protocol is not designed in a vacuum. It formalizes patterns that already exist in the codebase, fills gaps between them, and unifies scattered concerns into a coherent protocol layer.

### What Already Exists (No Changes Needed)

| Protocol Concept | Existing Implementation | Status |
|---|---|---|
| CIDv1 content addressing | `blob_store.rs`: `compute_addresses()` → CIDv1 (raw codec 0x55, SHA256) | Production |
| Shard delivery | `/elohim/shard/1.0.0`: `ShardRequest::Get/Have/Push` | Production |
| CRDT sync | `/elohim/sync/1.0.0`: Automerge document sync | Production |
| Wire format | 4-byte big-endian length + MessagePack (`rmp_serde`) | Production |
| Reed-Solomon sharding | `sharding.rs`: `ShardManifest` with `rs-4-7` encoding | Production |
| DID resolution | `did_resolver.rs`: `did:web` + `did:key` resolution with 5-min cache | Production |
| DID document serving | `identity.rs`: `/.well-known/did.json` with `elohim-protocol.org/ns/v1` context | Production |
| JWKS endpoint | `federation.rs`: `/.well-known/doorway-keys` (Ed25519 OKP) | Production |
| Constitutional types | `constitution/types.rs`: `ConstitutionalLayer`, `ImmutabilityLevel`, `ConstitutionalDocument` | Production |
| Stewardship model | `stewardship_allocations` table: ratios, governance state, Elohim ratification | Production |
| Reach levels | `content_store_integrity`: 8 reach levels from private to commons | Production |
| Blob serving | `doorway/blob.rs`: HTTP Range, ETag, immutable cache, shard fallback | Production |
| libp2p transport | `behaviour.rs`: Kademlia + mDNS + relay + DCUtR + AutoNAT | Production |
| W3C VC alignment | `verifiable-credential.model.ts`: `HolochainSignature2024` proof type | Model |
| ActivityPub type stubs | `ContentNode.activityPubType`, JSON-LD, OpenGraph fields | Stub |

### What Needs to Be Built

| Protocol Concept | Gap | Implementation Path |
|---|---|---|
| EPR Head type | Unify ContentNode + stewardship + reach into single signed struct | New Rust struct in `elohim-protocol` or `constitution` crate, with `ts-rs` export |
| EPR Document type | Formalize the aggregation that `DataLoaderService` does ad-hoc | New Rust struct, same pattern |
| `/elohim/epr/1.0.0` codec | No EPR resolution protocol exists | New `epr_protocol.rs` alongside `shard_protocol.rs`, same codec pattern |
| EPR Heads in DHT | Kademlia currently stores only shard hashes | Extend DHT records to include EPR Head bytes under `epr:` prefixed keys |
| `/elohim/feed/1.0.0` codec | No feed/subscription protocol exists | New `feed_protocol.rs`, same codec pattern |
| Feed subscription state | No subscription tracking | New table in elohim-storage: `feed_subscriptions` |
| `composite` content type | Landing page is static Angular | New content type + layout descriptor format + Angular renderer |
| Recognition on delivery | Economic events exist but delivery doesn't trigger them | Add recognition event logging in shard delivery handler |
| Pillar coupling validation | Enforcement is app-layer (Angular services) | Move validation to storage layer — reject EPR Heads with empty stewards |

### Migration Path

Existing content (3,526 nodes, 6 paths) can be incrementally wrapped in EPR Heads by:

1. For each content record in `content` table, generate an EPR Head from existing fields (`id`, `blob_cid`, `content_type`, `reach`) + stewardship data from `stewardship_allocations` + constitutional layer from governance state
2. Sign each EPR Head with the primary steward's key (or bootstrap key for system content)
3. Store EPR Heads in a new `epr_heads` table and announce to Kademlia DHT
4. The existing HTTP API (`/db/content/{id}`) continues to work — EPR resolution is an additional path, not a replacement
5. Composite landing page can be created as a new content record with `content_type: "composite"` once the renderer exists

This is additive, not disruptive. The existing infrastructure continues to function. The EPR layer adds protocol-level meaning to what was previously application-level convention.

---

## Appendix A: Enumeration Reference

### ContentType
```
concept | unit | epic | path | assessment | scenario | composite | simulation |
resource | reference | organization | community | human | instrument | quiz |
media | graph | gherkin | bible-verse | attestation | feature
```

### Reach (ordered from most restrictive to most open)
```
private | self | intimate | trusted | familiar | community | public | commons
```

### ConstitutionalLayer (ordered by precedence, highest first)
```
global(7) | bioregional(6) | nation_state(5) | provincial(4) |
community(3) | family(2) | individual(1)
```

### Contribution
```
original_creator | editor | translator | curator | maintainer | inherited
```

### Governance
```
active | disputed | pending_review | superseded
```

### AllocationMethod
```
manual | computed | negotiated
```

### TokenType (REA resource types)
```
care | time | learning | steward | culture | infrastructure | recognition
```

### NodeTier
```
network | home_node | home_cluster | laptop
```

Note: Doorway servers are a gateway service, not a stewardship tier. They proxy
to elohim-storage nodes which operate at one of the above tiers. A doorway's
backing storage node declares its own `NodeTier`.

### Relationship
```
CONTAINS | REFERENCES | DEPENDS_ON | IMPLEMENTS | DERIVED_FROM |
PREREQUISITE | FOLLOWUP | PRECEDES | SIBLING | PARENT | CHILD |
SIMILAR_TO | CONTRASTS_WITH | CONCEPTUALLY_RELATED | SHARED_CONCEPT |
ELABORATES | SUMMARIZES | EXAMPLE_OF | DEFINITION_OF | REQUIRES |
CREATED_BY | PUBLISHED_BY | RELATES_TO | DEMONSTRATES
```

---

## Appendix B: DID Methods

The Elohim Protocol uses three DID methods:

| Method | Usage | Example |
|--------|-------|---------|
| `did:web` | Doorway identity | `did:web:alpha.elohim.host` |
| `did:web` | Hosted human (custodial keys) | `did:web:hosted.elohim.host:humans:{humanId}` |
| `did:web` | Session identity (ephemeral) | `did:web:gateway.elohim.host:session:{sessionId}` |
| `did:web` | Content identity | `did:web:elohim.host:content:{contentId}` |
| `did:web` | Protocol system issuer | `did:web:elohim-protocol.org:system` |
| `did:web` | Steward issuer (for VCs) | `did:web:elohim-protocol.org:stewards:{id}` |
| `did:key` | Steward identity (local keypair) | `did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK` |

DID Documents are served at `/.well-known/did.json` with the context:
```json
[
  "https://www.w3.org/ns/did/v1",
  "https://w3id.org/security/suites/ed25519-2020/v1",
  "https://elohim-protocol.org/ns/v1"
]
```

The `elohim-protocol.org/ns/v1` context defines extensions:
- `elohim:capabilities` — node capabilities
- `elohim:region` — geographic region
- `elohim:holochainCellId` — Holochain cell identifier

---

## Appendix C: Wire Format Reference

All Elohim Protocol messages use the same framing convention:

```
┌────────────┬──────────────────────────────────────┐
│ Length (4B) │ MessagePack Payload                  │
│ big-endian  │ rmp_serde serialized enum variant    │
│ u32         │                                      │
└────────────┴──────────────────────────────────────┘
```

Protocol IDs registered in the Elohim network:
```
/elohim/shard/1.0.0    Blob shard transfer (existing)
/elohim/sync/1.0.0     Automerge CRDT sync (existing)
/elohim/epr/1.0.0      EPR resolution (this specification)
/elohim/feed/1.0.0     Feed subscription (this specification)
/elohim/id/1.0.0       Peer identification (existing)
/elohim/cluster/1.0.0  Cluster coordination (reserved)
```

---

## Closing Note

The Elohim Protocol does not compete with AT Protocol, ActivityPub, or IPFS. It composes their best insights — content addressing from IPFS, signed data repositories from AT Protocol, the vocabulary of ActivityPub — while adding what none of them provide: the structural coupling of knowledge, value, and governance in every content reference.

A CID tells you *what* the bytes are. An AT URI tells you *who* published them. An Elohim EPR tells you what they mean, who cares for them, and who said they could be here.

That is not a small difference. It is the difference between a currency that carries no values and one that remembers where it came from.

---

## Appendix D: Scope and Companion Specifications

This specification defines content addressing, resolution, and feed protocols. Several concerns referenced in the manifesto and constitution are governed by companion specifications at other layers:

| Concern | Governed By | Not This Spec Because |
|---------|------------|----------------------|
| Physical privacy controls | Observer Protocol + Hardware Specification | Hardware/firmware layer, not wire protocol |
| Harm response / victim sovereignty | Elohim Agent Constitution (TBD) | Agent behavior, not content addressing |
| Token circulation / demurrage rules | Shefa Economic Protocol (TBD) | Economic policy, not content format |
| Constitutional Council sortition | Constitutional Council Protocol (TBD) | Governance mechanism, not content delivery |
| Identity migration between tiers | Identity Portability Protocol (TBD) | Identity lifecycle, not content references |

The EPR protocol provides the *hooks* for these companion specs (the `ratifier` field connects to sortition, the `recognitionPolicy` connects to economic rules, the `reach` field connects to privacy), but does not define the rules themselves.

---

## Appendix E: EPR URI Scheme & Content Addressing Reference

*The `<a href="">` of the Elohim Protocol.*

Every protocol needs a single, canonical way to point at things. HTML has `href`. IPFS has `ipfs://cid`. AT Protocol has `at://did/collection/rkey`. The Elohim Protocol has the `epr:` URI — a reference that encodes not just *where* content lives, but *what it means* in the knowledge graph and *who cares for it*.

### E.1: The `epr:` URI Scheme

```
epr:{id}                                    Resolve EPR Head (Tier 1)
epr:{id}@{version}                          Resolve specific version
epr:{id}/doc                                Resolve EPR Document (Tier 2)
epr:{id}/blob                               Resolve Content Bytes (Tier 3)
epr:{id}#step/{index}                       Fragment: path step position
epr:{id}#chapter/{chapterId}                Fragment: path chapter
epr:{id}#rel/{relationship}/{targetId}      Fragment: graph edge
epr:{id}?via={did-or-peer-id}               Hint: prefer this resolver
epr:{id}?reach={reach-level}                Hint: my access level
```

**Syntax (informal ABNF)**:
```
epr-uri     = "epr:" epr-id [ "@" version ] [ "/" tier ] [ "?" query ] [ "#" fragment ]
epr-id      = 1*( ALPHA / DIGIT / "-" / "_" / "." )
version     = 1*DIGIT
tier        = "doc" / "blob"
query       = via-param / reach-param / ( via-param "&" reach-param )
via-param   = "via=" ( did / peer-id )
reach-param = "reach=" reach-level
fragment    = step-frag / chapter-frag / rel-frag
step-frag   = "step/" 1*DIGIT
chapter-frag = "chapter/" epr-id
rel-frag    = "rel/" relationship-type "/" epr-id
```

**Examples**:
```
epr:manifesto-foundations                    The manifesto's foundations unit
epr:manifesto-foundations@3                  Version 3 specifically
epr:manifesto-foundations/blob               The raw content bytes
epr:manifesto-foundations/doc                Full EPR Document with relationships
epr:elohim-protocol-path#step/2             Step 2 of the protocol learning path
epr:elohim-protocol-path#chapter/economic   The "economic" chapter of that path
epr:systems-thinking#rel/PREREQUISITE/feedback-loops
                                            The prerequisite edge from systems-thinking
                                            to feedback-loops
epr:manifesto-foundations?via=did:web:alpha.elohim.host
                                            Prefer resolving through alpha doorway
```

**Why `epr:` not `elohim://`**: The `elohim://` scheme is already registered for Tauri deep links (OAuth callbacks on P2P-native devices). `epr:` is short, unambiguous, and consistent with the EPR (Elohim Protocol Reference) naming used throughout this specification.

### E.2: The Four-Layer Hierarchy

Every content reference passes through four layers. The key design decision: **transport is NOT in the URI**. It is resolved at runtime by the connection strategy. This keeps references portable across all deployment modes.

```
┌─────────────────────────────────────────────────────────────────┐
│ Layer 4: GRAPH POSITION (fragment — how it connects)            │
│   #step/3    #chapter/economic    #rel/PREREQUISITE/feedback    │
├─────────────────────────────────────────────────────────────────┤
│ Layer 3: PROTOCOL CONTEXT (inherent in every EPR Head)          │
│   lamad: contentType, title, previewCid                         │
│   shefa: stewards[], recognition                                │
│   qahal: reach, layer, governance                               │
├─────────────────────────────────────────────────────────────────┤
│ Layer 2: RESOLUTION (query hint — where to find it)             │
│   ?via=did:web:alpha.elohim.host   (prefer this doorway)        │
│   ?via=12D3KooW...                 (prefer this peer)           │
│   (omitted)                        (use DHT / default strategy) │
├─────────────────────────────────────────────────────────────────┤
│ Layer 1: TRANSPORT (implicit — resolved at runtime)             │
│   web:     HTTPS via doorway (browser, hosted users)            │
│   native:  HTTP to local elohim-storage (P2P-native device)    │
│   p2p:     libp2p request-response (device-to-device)           │
│   dev:     Angular proxy to localhost (development)              │
└─────────────────────────────────────────────────────────────────┘

         epr:systems-thinking#rel/PREREQUISITE/feedback-loops
              ───────┬───────  ──────────┬──────────────────
                   id           graph position (Layer 4)

    Protocol context (Layer 3) is not in the URI — it's in the EPR Head
    that the URI resolves to. Every EPR Head always carries all three
    pillar dimensions. The URI is the key; the EPR is the value.
```

### E.3: Resolution Matrix — `epr:` to Transport-Specific URLs

Given an `epr:` reference, the connection strategy resolves it to a transport-specific operation. This table is the developer's lookup — the one place to find "how do I fetch this?"

#### Tier 1: EPR Head (metadata + stewardship + governance)

| EPR URI | Web (Doorway) | P2P-Native Device | P2P (libp2p) | App Route |
|---------|---------------|----------------------|--------------|-----------|
| `epr:{id}` | `GET {doorway}/db/content/{id}` | `GET {storage}/db/content/{id}` | `EprRequest::Resolve { id }` | `/lamad/resource/{id}` |
| `epr:{id}@{v}` | `GET {doorway}/db/content/{id}?version={v}` | `GET {storage}/db/content/{id}?version={v}` | `EprRequest::ResolveVersion { id, version }` | — |
| `epr:{id}` (path) | `GET {doorway}/db/paths/{id}` | `GET {storage}/db/paths/{id}` | `EprRequest::Resolve { id }` | `/lamad/path/{id}` |

Where:
- `{doorway}` = doorway base URL (e.g. `https://doorway-alpha.elohim.host`)
- `{storage}` = elohim-storage base URL (e.g. `http://localhost:8090`)

#### Tier 2: EPR Document (full pillar context, relationships, economic events)

| EPR URI | Web (Doorway) | P2P-Native Device | P2P (libp2p) |
|---------|---------------|----------------------|--------------|
| `epr:{id}/doc` | `GET {doorway}/db/content/{id}` + `GET {doorway}/db/relationships/graph/{id}` | Same pattern against `{storage}` | `EprRequest::GetDocument { id }` |

Note: The current HTTP API returns metadata and relationships separately. The P2P `EprRequest::GetDocument` returns them as a single EPR Document. HTTP implementations compose the equivalent from two calls until a unified endpoint exists.

#### Tier 3: Content Bytes (the blob)

| EPR URI | Web (Doorway) | P2P-Native Device | P2P (libp2p) |
|---------|---------------|----------------------|--------------|
| `epr:{id}/blob` | **Two-step**: resolve `epr:{id}` → get `blobHash` → `GET {doorway}/blob/{hash}` | **Two-step**: resolve → `GET {storage}/blob/{hash}` | `ShardRequest::Get { hash }` |
| (by hash directly) | `GET {doorway}/blob/{hash}` | `GET {storage}/blob/{hash}` | `ShardRequest::Get { hash }` |

**Path is unified**: Both doorway and the local steward (elohim-storage) serve blobs at `/blob/{hash}`. The difference is in what's wrapped around the request, not the path name. On doorway, requests pass through the projection/caching layer and reach the configured steward via registry-routed proxy (HTTP Range, ETag, CDN-friendly caching, shard-resolution fallback all happen there). On a P2P-native device, the local steward (elohim-storage) is hit directly — same path, no projection layer in front. The vocabulary cleanup (2026-04-30) retired the legacy `/store/{hash}` (gateway-only) and `/api/blob/{hash}` (admin proxy alias) paths in favor of this single canonical route. `POST /api/blob/verify` remains as a separate verification endpoint.

#### Path Steps (fragment resolution)

| EPR URI | Resolution |
|---------|-----------|
| `epr:{pathId}#step/{n}` | Resolve `epr:{pathId}` as path → extract step `n` from `steps[]` → resolve step's content EPR |
| App route | `/lamad/path/{pathId}/step/{n}` |

Fragments are resolved client-side after the parent EPR resolves. The fragment is not sent to the server.

#### Batch Resolution (for composite content)

| EPR URI | Web/Direct | P2P |
|---------|-----------|-----|
| Multiple `epr:{id}` refs in a composite layout | Parallel `GET /db/content/{id}` calls | `EprRequest::ResolveBatch { ids: [...] }` |

### E.4: DID ↔ EPR Bridge

Two identifier systems coexist. DIDs are for W3C interop (Verifiable Credentials, federation discovery, DID document resolution). EPR URIs are for protocol-native operations (content resolution, feed subscriptions, graph traversal, UI links).

**Conversion rules**:

| DID | EPR URI | When to use DID | When to use EPR |
|-----|---------|----------------|-----------------|
| `did:web:{host}:content:{id}` | `epr:{id}` | VCs, federation, external references | Resolution, feeds, UI |
| `did:web:{host}:paths:{id}` | `epr:{id}` | VCs about path completion | Resolution, UI |
| `did:web:{host}:humans:{id}` | `epr:human/{id}` | Identity, auth, VCs | Steward feed subscriptions |
| `did:web:{host}:agents:{id}` | `epr:agent/{id}` | Elohim agent identity | Agent-related queries |

**Examples**:
```
did:web:hosted.elohim.host:content:manifesto-foundations  ↔  epr:manifesto-foundations
did:web:hosted.elohim.host:paths:elohim-protocol          ↔  epr:elohim-protocol
did:web:hosted.elohim.host:humans:matthew                  ↔  epr:human/matthew
```

The `{host}` in the DID varies by deployment (doorway DIDs use their host, hosted users use `hosted.elohim.host`). The EPR URI is host-independent — it resolves through whatever transport is available. This is the key portability benefit: `epr:manifesto-foundations` works whether you're on a doorway, a P2P-native device, or a headless storage node.

### E.5: Hash Format Canonicalization

One canonical format for content-addressed blobs:

```
CANONICAL:     sha256-a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a
               ~~~~~~ ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
               prefix  64 lowercase hex characters (32 bytes)
```

All components MUST produce `sha256-{hex}` when generating blob references.
All components MUST accept these formats on input:

| Format | Example | Status | Where It Appears |
|--------|---------|--------|-----------------|
| `sha256-{hex}` | `sha256-a7ffc6f8...` | **Canonical** | EPR Heads, shard protocol, HTTP APIs, all new code |
| `bafkrei...` | `bafkreigdyrzt5sfp7...` | Valid input | CIDv1 (raw codec 0x55, SHA256 multihash, base32lower). Accepted everywhere. Stored internally as `sha256-{hex}` |
| `sha256:{hex}` | `sha256:a7ffc6f8...` | **Deprecated** | Legacy blob references in some content bodies. Accept on input, never produce |
| Raw hex | `a7ffc6f8bf1ed766...` | Valid input | 64-char hex string. Accepted by `parse_content_address()`. Never produce — always prefix with `sha256-` |

**Normalization** (pseudo-code for any component accepting a content address):
```
fn normalize(input: &str) -> String {
    if input.starts_with("sha256-") && input.len() == 71 { return input }
    if input.starts_with("sha256:") { return "sha256-" + &input[7..] }
    if input.len() == 64 && is_hex(input) { return "sha256-" + input }
    if input.starts_with("bafkrei") { return "sha256-" + cid_to_sha256_hex(input) }
    error("unrecognized content address format")
}
```

### E.6: Two-Step Content Resolution (Metadata → Blob)

Most content retrieval follows a two-step pattern: first resolve the EPR to get the blob hash, then fetch the blob by hash. This is the standard flow every developer will implement.

```
Step 1: Resolve EPR
  epr:manifesto-foundations
    → GET /db/content/manifesto-foundations
    → {
        id: "manifesto-foundations",
        blobHash: "sha256-a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a",
        contentType: "unit",
        contentFormat: "markdown",
        title: "Foundations of the Elohim Protocol",
        ...
      }

Step 2: Fetch blob (if content body is a blob reference)
  sha256-a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a
    → GET /blob/sha256-a7ffc6f8...   (canonical — through doorway with CDN/Range, or direct to storage)
    → ShardRequest::Get { hash: "sha256-a7ffc6f8..." }  (P2P)
    → raw bytes (markdown text, image data, video, etc.)

Step 3: Verify integrity
  sha256(received_bytes) == epr_head.cid
  If mismatch: discard, try alternate peer/doorway
```

**When is step 2 needed?** When the content's `content` field (or `contentBody`) starts with `sha256-` or `sha256:`, it's a blob reference. Otherwise, the content body is inline (small text content stored directly in the metadata record).

**Decision tree**:
```
content.contentBody
  ├── starts with "sha256-" or "sha256:" → BLOB: fetch from /blob/{hash}
  ├── starts with "{" or "[" → INLINE JSON: parse directly (sophia-quiz-json, html5-app config)
  └── otherwise → INLINE TEXT: render directly (short markdown, descriptions)
```

### E.7: Delivery Mode Decision Tree

For developers implementing content retrieval — which URL pattern to use:

```
Am I a P2P-native device? (own conductor, own storage, own keys)
  ├── YES: I talk to my local steward (elohim-storage) over HTTP
  │     │  AND to the network over libp2p — both paths are mine.
  │     │
  │     │  Local steward (HTTP to localhost):
  │     │     Metadata: GET http://localhost:8090/db/content/{id}
  │     │     Blobs:    GET http://localhost:8090/blob/{hash}
  │     │     Paths:    GET http://localhost:8090/db/paths/{id}
  │     │     WS:       ws://localhost:{port}
  │     │
  │     │  Network (libp2p to peers):
  │     │     Metadata: EprRequest::Resolve { id }
  │     │     Blobs:    ShardRequest::Get { hash }
  │     │     Feeds:    FeedRequest::Subscribe { ... }
  │     │     Sync:     Automerge sync protocol
  │     │
  │     └── My local steward may also use libp2p to fetch content
  │         I don't have yet — the two paths compose, not compete.
  │
  └── NO: I'm a browser. Someone else stewards for me.
        ├── Doorway configured (production/hosted):
        │     Metadata: GET {doorway}/db/content/{id}
        │     Blobs:    GET {doorway}/blob/{hash}      (CDN-cached, Range-capable, registry-routed to steward)
        │     Paths:    GET {doorway}/db/paths/{id}
        │     Graphs:   GET {doorway}/db/relationships/graph/{id}
        │     WS:       wss://{doorway}/hc/app/{port}?apiKey=...&token=...
        └── No doorway (dev server):
              Same paths, proxied through localhost:4200 → localhost:8888
              (proxy.conf.mjs maps /db/, /blob/, /api/ to doorway)
```

### E.8: Known Inconsistencies

Issues in the current codebase that this specification normalizes:

| Issue | Current State | Correct Per This Spec | Fix |
|-------|-------------|----------------------|-----|
| Legacy doorway blob paths | `/store/{hash}` and `/api/blob/{hash}` historically routed to storage | Single canonical `/blob/{hash}` via registry | RESOLVED 2026-04-30 (vocabulary cleanup sprint) |
| `sha256:` colon format | Accepted by `BlobManagerService` | Deprecated — accept, never produce | Add deprecation comment in `blob-manager.service.ts` |
| DID host inconsistency | Models use `elohim.host`, doorway uses `hosted.elohim.host` | `hosted.elohim.host` for hosted humans | Align model examples |

---

## Appendix F: IPLD Alignment — EPR as an IPLD Extension

The Elohim Protocol composes on IPFS primitives rather than building alongside them. EPR Heads are IPLD-compatible documents: any IPLD tool can traverse their links, but only EPR-aware tools understand the three-pillar semantics.

### F.1: Content Addressing — CIDv1 as Canonical

EPR uses IPFS Content Identifiers (CIDv1) as the canonical content address format:

- **Codec**: Raw (0x55) — content bytes are opaque, not structured
- **Hash**: SHA-256 (0x12) via multihash
- **Base**: base32lower — produces `bafkrei...` strings
- **Example**: `bafkreibm6jg3ux5qumhcn2b3flc3tyu6dmlb4xa7u5bf44mcplnzjhclme`

The Rust backend (`blob_store.rs`) already computes both CID and legacy `sha256-{hex}` on every store operation. The `parse_content_address()` function accepts CID, `sha256-{hex}`, and raw hex — enabling gradual migration.

Legacy `sha256-{hex}` format is accepted on input but should not be produced by new code.

### F.2: EPR Head as IPLD Document

An EPR Head serialized as DAG-CBOR (multicodec 0x71):

```cbor
{
  "version": 1,
  "id": "rea-foundations",
  "content": { "/": "bafkreibm6jg3..." },

  "lamad": {
    "title": "REA Foundations",
    "contentType": "concept",
    "contentFormat": "markdown",
    "description": "Resource-Event-Agent accounting model",
    "tags": ["economics", "rea", "accounting"]
  },

  "shefa": {
    "stewards": ["did:web:alpha.elohim.host:humans:contributor-1"],
    "allocations": [100]
  },

  "qahal": {
    "reach": "commons",
    "layer": "global"
  },

  "relationships": [
    {
      "type": "PREREQUISITE",
      "target": "systems-thinking",
      "targetCid": { "/": "bafkrei..." }
    },
    {
      "type": "TEACHES",
      "target": "hrea-agent-model"
    }
  ],

  "author": "did:web:alpha.elohim.host:humans:contributor-1",
  "updated": "2026-02-27T00:00:00Z"
}
```

**IPLD link format**: The `{ "/": "bafkrei..." }` syntax is IPLD's standard CID link representation in DAG-CBOR/DAG-JSON. IPLD tools follow these links during traversal.

**Three-pillar extension**: The `lamad`, `shefa`, and `qahal` fields are EPR's semantic extension to IPLD. They carry no meaning to generic IPLD tools but are the core value proposition for EPR-aware applications.

### F.3: What IPLD Provides vs What EPR Adds

| Capability | IPLD Primitive | EPR Extension |
|-----------|---------------|---------------|
| Content addressing | CIDv1 (multihash + multicodec) | — (uses as-is) |
| Immutable links | DAG-CBOR CID links | Typed relationships (PREREQUISITE, TEACHES, etc.) |
| Graph traversal | IPLD Selectors | Three-pillar-aware traversal (future) |
| Mutable naming | IPNS (name → CID pointer) | EPR Head (name → rich metadata → CID) |
| Block exchange | Bitswap | Shard protocol with stewardship economics |
| Data model | IPLD Data Model (maps, lists, links) | Three-pillar coupling (lamad/shefa/qahal) |
| Content verification | Hash verification | Hash + governance (reach) + steward attestation |

### F.4: Migration Path

**Current state** (this sprint):
- EPR Head defined as TypeScript interface (`epr-head.model.ts`)
- CIDv1 recognized as canonical in frontend (`multiformats` package)
- Backend produces CID on every blob store
- Wire format remains JSON (not yet DAG-CBOR)

**Next sprint**:
- Rust IPFS SDK (forked `rust-ipfs`) as submodule for Bitswap, IPLD DAG store, CID routing
- Helia `@helia/verified-fetch` for trustless blob retrieval in browser
- EPR Head serialization to DAG-CBOR alongside JSON

**Future**:
- EPR multicodec registration (private range, then IANA if adopted)
- IPNS integration for EPR mutable naming
- IPLD Selectors for knowledge graph traversal
- Push for IPLD spec extension with three-pillar patterns

### F.5: Interoperability Guarantee

Any IPFS node can:
1. **Store** EPR content bytes (they're just blobs with CIDs)
2. **Pin** EPR content (standard IPFS pinning)
3. **Exchange** EPR blobs via Bitswap (standard block exchange)
4. **Traverse** EPR Head links (standard IPLD DAG-CBOR)

Only EPR-aware nodes can:
1. **Interpret** three-pillar semantics (lamad/shefa/qahal)
2. **Enforce** governance (reach levels, constitutional layers)
3. **Route** stewardship economics (recognition flows)
4. **Resolve** context-aware navigation (path-aware link resolution)

This is by design: interoperable by default, EPR extensions prove their value through use. If EPR gains adoption, the three-pillar coupling becomes a candidate for IPLD spec extension.

### F.6: Multicodec Registration

EPR defines three application-specific multicodec entries in the private-use range `0x300000–0x3FFFFF`, alongside the standard DAG-CBOR codec used for EPR Head encoding:

| Code | Name | Description |
|------|------|-------------|
| `0x71` | dag-cbor | Standard IPLD DAG-CBOR codec (used for EPR Head encoding) |
| `0x300001` | epr-head | EPR Head metadata envelope (private-use range) |
| `0x300002` | epr-document | EPR Document body (private-use range) |
| `0x300003` | epr-relationship | EPR Relationship edge (private-use range) |

EPR uses the private-use range `0x300000–0x3FFFFF` per the [multicodec specification](https://github.com/multiformats/multicodec). These codes are not registered upstream and are only meaningful within the Elohim Protocol ecosystem. If EPR gains broader adoption, these codes will be submitted for formal IANA registration.

The `dag-cbor` codec (`0x71`) is the standard IPLD codec and is used directly for EPR Head serialization. The EPR-specific codes (`0x300001`–`0x300003`) identify the semantic type of the encoded document, enabling EPR-aware tools to distinguish an EPR Head from a generic DAG-CBOR document without inspecting the payload.

### F.7: Wire Format Migration (JSON to DAG-CBOR)

EPR Heads are migrating from raw JSON to DAG-CBOR as the canonical wire format. The migration proceeds in three phases:

1. **Phase 1 (current)**: EPR Heads stored as DAG-CBOR with codec `0x71`. Backward compatibility via first-byte detection: `0x7B` (ASCII `{`) indicates JSON, anything else indicates CBOR. All readers MUST support both formats during this phase.
2. **Phase 2 (planned)**: All new EPR Heads written exclusively as DAG-CBOR. JSON reading retained for migration of existing content. New implementations MAY omit JSON writing support.
3. **Phase 3 (future)**: JSON fallback deprecated. All content IPLD-native. Implementations MAY drop JSON reading support after a migration period (to be defined by constitutional governance).

**First-byte detection** (pseudo-code):
```
fn decode_epr_head(bytes: &[u8]) -> EprHead {
    if bytes[0] == 0x7B {   // '{' — JSON
        serde_json::from_slice(bytes)
    } else {                 // DAG-CBOR
        serde_cbor::from_slice(bytes)
    }
}
```

This approach avoids version negotiation or content-type headers at the storage layer — the format is self-describing from the first byte.

### F.8: CID Format Convention

CID prefixes encode both the hash function and the codec, providing a visual distinction between structured metadata and raw content bytes:

- **EPR Head CIDs** use codec `0x71` (dag-cbor), producing `bafyr...` prefixed strings (base32lower multibase + CIDv1 + dag-cbor + sha256)
- **Content blob CIDs** use codec `0x55` (raw), producing `bafkrei...` prefixed strings (base32lower multibase + CIDv1 + raw + sha256)

The CID prefix distinguishes structured metadata from raw bytes at a glance:

| Prefix | Codec | Meaning | Example |
|--------|-------|---------|---------|
| `bafyr...` | `0x71` (dag-cbor) | Structured IPLD document (EPR Head, EPR Document) | EPR Head for "rea-foundations" |
| `bafkrei...` | `0x55` (raw) | Opaque content bytes (markdown, images, video) | Blob for a unit's markdown body |

This convention means that any system processing EPR references can immediately determine whether a CID points to traversable structured data or opaque bytes, without resolving the reference first.

---

## License and Openness

This specification is published as open documentation under the same terms as the Elohim Protocol codebase. All protocol specifications, reference implementations, and constitutional documents are publicly auditable, modifiable, and community-maintained.

No entity — including the Elohim Protocol organization — holds exclusive rights to implement, extend, or restrict this specification. The protocol's anti-capture design applies to the specification itself: it belongs to the commons, stewarded by the community that uses it.

Implementations of this specification should be open source. Proprietary implementations that restrict auditability violate the constitutional requirement that "no single entity should control the infrastructure of human connection."
