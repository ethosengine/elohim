# Holochain + Cache Core + Seed + Podcast Distribution Flow

## Complete Step-by-Step Data Flow: From Seeding to Peer Delivery with Attestations

This document traces the entire lifecycle of a podcast (e.g., Sheila Wray Gregoire content) shared across Holochain with the new holochain-cache-core protecting thread performance.

---

## Part 1: Bootstrap Phase (App Startup)

### Step 1a: Holochain Conductor Initializes
**Time: T=0ms | Location: Holochain conductor process**

```
├─ Conductor starts (separate process)
├─ Loads DNA bundles (lamad-spike)
├─ Initializes thread pools:
│  ├─ WASM runtime threads: N threads for zome calls
│  ├─ Networking threads: Kitsune DHT peer connections
│  ├─ Validation threads: Entry signature verification
│  └─ Storage threads: Database write operations
├─ Starts Kitsune network layer
│  ├─ Listens on 0.0.0.0:42000 (by default)
│  └─ Publishes agent info to bootstrap service
└─ Ready to accept AppWebsocket connections on ws://localhost:4444
```

**Thread Count**: Fixed, pre-allocated. No dynamic growth.

### Step 1b: Doorway HTTP API Initializes
**Time: T=100ms | Location: Doorway process**

```
├─ Connects to conductor via AppWebsocket (ws://localhost:4444)
├─ Creates Worker Pool:
│  ├─ 4 fixed workers (default, configurable)
│  ├─ Each worker gets persistent connection to conductor
│  ├─ Semaphore limiting: max 1000 concurrent requests
│  └─ MPSC channel: queues excess requests
├─ Initializes caches:
│  ├─ ContentCache: Max 10,000 entries, LRU eviction
│  ├─ Cache rules from DNA: get_content (1h), list_paths (5m), etc.
│  └─ Reach-aware isolation: separate cache keys per reach level
├─ Starts projection subscriber:
│  ├─ Connects to conductor signal stream
│  └─ Listens for post_commit signals from DNA
└─ HTTP server ready on 0.0.0.0:3000
```

**Thread Protection**: Workers are isolated from conductor threads.

### Step 1c: Angular App + holochain-cache-core Initialize
**Time: T=200ms | Location: Browser / Electron**

```
├─ App Bootstrap (AppComponent)
├─ Service Initialization Order:
│  ├─ HolochainClientService
│  │  ├─ Connects to doorway HTTP API (http://localhost:3000)
│  │  ├─ No direct conductor connection (good!)
│  │  └─ Ready for API calls
│  │
│  ├─ HolochainCacheWasmService
│  │  ├─ Loads holochain-cache-core WASM module
│  │  ├─ Creates ReachAwareBlobCache (128MB per reach level)
│  │  ├─ Initializes 8 reach-level caches
│  │  │  ├─ Reach 0 (Private): 128MB LRU
│  │  │  ├─ Reach 7 (Commons): 128MB LRU ← Podcast stored here
│  │  │  └─ ... others
│  │  ├─ Sets up domain/epic index
│  │  └─ Ready for blob caching
│  │
│  ├─ BlobCacheTiersService
│  │  ├─ Initializes Tier 1 (metadata, JS)
│  │  ├─ Wraps holochain-cache-core for Tier 2 (blobs, WASM)
│  │  ├─ Initializes Tier 3 (chunks, WASM)
│  │  └─ Ready for blob operations
│  │
│  ├─ BlobBootstrapService (from earlier work)
│  │  ├─ Waits for Holochain connection
│  │  ├─ Pre-fetches metadata for known content
│  │  ├─ Initializes IndexedDB persistence
│  │  └─ Starts integrity verification
│  │
│  └─ Other services...
│
└─ App ready at T=500ms
   All systems operational, thread-safe, no starvation risk
```

**Thread Safety**: WASM cache runs on JS thread pool (not conductor).

---

## Part 2: Seed Phase (Data Population)

### Step 2a: Seed Script Preparation
**Time: T+0s | Location: Seeder process (Node.js)**

```
Seeder Script (seeder.ts) starts:
├─ Read environment
│  ├─ Admin WS: ws://localhost:41195 (from .hc_ports)
│  ├─ App ID: "lamad-spike"
│  └─ Zome: "content_store"
│
├─ Load content JSON from disk:
│  ├─ /data/lamad/sheila-gregoire-podcasts.json
│  │  ├─ 50+ podcast episodes
│  │  ├─ Each 2-4 GB (video + audio variants)
│  │  ├─ Metadata: title, description, tags, etc.
│  │  └─ Related concepts: marriage, intimacy, faith
│  │
│  └─ /data/lamad/relationships.json
│     ├─ RELATES_TO: podcast → concepts
│     ├─ CONTAINS: series → episodes
│     └─ DERIVED_FROM: podcast → source (author/publisher)
│
└─ Validate JSON schema ✓
```

### Step 2b: Batch Create Content Entries
**Time: T+5s | Conductor processes zome calls**

```
Seeder calls (in batches of 50):
├─ zome call: bulk_create_content({
│  ├─ entries: [
│  │  {
│  │    id: "sheila-ep-001",
│  │    content_type: "podcast_episode",
│  │    title: "Sheila Wray Gregoire - Episode 1: Marriage Intimacy",
│  │    description: "...",
│  │    tags: ["marriage", "intimacy", "sexuality", "faith"],
│  │    reach: "commons",  ← Public content
│  │    metadata_json: {
│  │      "duration_minutes": 52,
│  │      "codec": "h264",
│  │      "resolutions": ["480p", "720p", "1080p"],
│  │      "audio_codec": "aac",
│  │      "blob_hash": "sha256-...", ← Links to blob
│  │      "custodian_commitment_id": "cust-123"
│  │    }
│  │  },
│  │  ... 49 more episodes
│  │ ]
│  └─ })
│
└─ Conductor processes:
   ├─ DNA validates entries
   │  ├─ Check: reach field valid
   │  ├─ Check: author has permission
   │  └─ Check: metadata schema matches
   │
   ├─ Creates entries on source chain
   │  ├─ Entry: Content { id, type, reach, ... }
   │  └─ Entry: Blob { hash, size, codec, ... }
   │
   ├─ Posts entries to DHT
   │  ├─ DHT neighbors fetch and validate
   │  ├─ Replication factor: 3 copies (default)
   │  └─ Propagation time: 30-60 seconds
   │
   └─ POSTS POST_COMMIT SIGNAL:
      └─ Signal: { ContentCommitted { id, content, author, ... } }
          ↓ Goes to projection subscriber
```

**Key Point**: Conductor creates entries, but doesn't serve reads (that's the cache).

### Step 2c: Create Relationships (Content Graph)
**Time: T+30s | Bulk create relationships**

```
Seeder calls:
├─ zome call: bulk_create_relationships({
│  ├─ relationships: [
│  │  {
│  │    source_id: "sheila-series",
│  │    target_id: "sheila-ep-001",
│  │    relationship_type: "CONTAINS",
│  │    confidence: 1.0,
│  │    metadata_json: { "episode_order": 1 }
│  │  },
│  │  {
│  │    source_id: "sheila-ep-001",
│  │    target_id: "concept-marital-intimacy",
│  │    relationship_type: "RELATES_TO",
│  │    confidence: 0.95,
│  │    metadata_json: { "relevance": "core_topic" }
│  │  },
│  │  ... many more relationships
│  │ ]
│  └─ })
│
└─ Conductor processes:
   ├─ Validates relationships
   ├─ Creates link entries on DHT
   ├─ Posts signals for each relationship
   └─ DHT replicates (slower than content, ~1-2 minutes)
```

**Relationship Graph Building**: Concepts linked to podcasts, series linked to episodes, etc.

### Step 2d: Projection Engine Consumes Signals
**Time: T+5s - T+2m | Parallel with DHT replication**

```
Projection Subscriber (listening since app boot):
├─ Receives ContentCommitted signal (for each 50 podcast entries)
│  ├─ Signal includes: id, title, reach, metadata, author_address
│  ├─ Transforms to ContentProjection
│  │  ├─ Indexes: id, type, tags, reach, author
│  │  ├─ Full-text search: title, description
│  │  └─ Denormalized: related_node_ids (from relationships)
│  ├─ Stores in MongoDB (if configured) or memory cache
│  └─ Updates door way cache
│     └─ Entry: cache key "dna:content_store:get_content:sheila-ep-001:commons"
│                Value: { title, description, metadata, reach, ... }
│                TTL: 1 hour
│
├─ Receives RelationshipCommitted signal
│  ├─ Transforms to index entry
│  ├─ Updates reverse index: content_id → [related_ids]
│  └─ Cache keys affected:
│     ├─ get_content_graph (invalidated)
│     └─ get_content_by_tag (updated)
│
└─ Cache layer populated by T+2m, ready for REST API queries
```

**Cache Population**: Happens in parallel with DHT replication (not waiting for it).

---

## Part 3: User Accesses Podcast (Cache Protection)

### Step 3a: User Visits "Sheila Gregoire" Learning Path
**Time: T+0s | User clicks link in browser**

```
Client Action:
├─ User clicks: "Explore Sheila Wray Gregoire Marriage Teaching"
├─ App loads PodcastListComponent
│  └─ Calls: BlobCacheTiersService.getBlobsByDomainEpic(
│              "ethosengine",  // or custom domain
│              "marriage-intimacy"
│            )
└─ This is now O(1) lookup in WASM cache! (vs O(n) in JS)

WASM Cache Lookup (holochain-cache-core):
├─ Query: domain_epic_index["ethosengine:marriage-intimacy"]
├─ Returns: Vec<[hash1, hash2, hash3, ...]>  ← O(1) HashMap lookup
├─ Time: < 1ms
└─ No conductor involved
   └─ No thread starvation risk!
```

**Protection**: Request never reaches conductor.

### Step 3b: App Queries Doorway Cache for Content List
**Time: T+5ms | HTTP GET to doorway**

```
Browser → Doorway HTTP API:


GET /api/content?type=podcast_episode&tag=marriage&reach=commons
    ?limit=50

Doorway Processing:
├─ Request reaches Worker Pool
├─ Worker 1 acquires semaphore permit
│  └─ Semaphore counter decreases by 1 (max 1000 concurrent)
│
├─ Worker 1 checks cache:
│  ├─ Cache key: "dna:content_store:list_by_type:...query_hash...:.commons"
│  ├─ Found in ContentCache (memory)!
│  │  └─ Blob metadata: [{ id, title, duration, codec, size, ... }, ...]
│  └─ Returns cached response immediately
│
├─ Response sent to client: 5ms
│
└─ Worker releases semaphore permit
   └─ Semaphore counter increases by 1
      (Next queued request can proceed)
```

**Time**: 5-10ms total (no conductor round-trip needed).

### Step 3c: Cache Strategy Analysis

**If cache HIT** (99% likely for popular content):
```
Browser → Doorway HTTP (5ms)
  ├─ Worker acquires permit
  ├─ Check memory cache (O(1))
  ├─ Return cached JSON
  └─ Release permit
Total time: 5-10ms
Conductor involved: ZERO
Threads used: 1 (Worker)
```

**If cache MISS** (1%, cache expired or new):
```
Browser → Doorway HTTP → Worker → Conductor
  ├─ Worker acquires permit
  ├─ Cache miss detected
  ├─ Forward to conductor via WebSocket:
  │  └─ {"id": 1, "fn_name": "list_by_type", "payload": {...}}
  ├─ Conductor processes (using available WASM thread)
  │  ├─ Validates query params
  │  ├─ Queries content_store zome
  │  ├─ Returns JSON response
  │  └─ Sends back to Worker
  ├─ Worker caches response (TTL 5m)
  ├─ Send to client
  └─ Release permit
Total time: 50-200ms (still acceptable)
Conductor involved: 1 call
Threads used: 1 (Worker) + 1 (Conductor WASM thread)
```

**Worker Pool Protection**:
```
100 concurrent requests:
├─ 4 workers available
├─ 4 requests proceed immediately
├─ 96 requests queue in MPSC channel
│  ├─ Semaphore holds 96 permits
│  └─ Requests wait peacefully (no busy-spinning)
├─ As workers finish, queue processes
│  └─ No starvation: FIFO fairness
└─ If queue exceeds 1000:
   └─ New requests get "queue full" error (graceful)
      (Rather than DOS-ing the conductor)
```

---

## Part 4: Blob Download with Peer Discovery

### Step 4a: User Clicks "Watch Episode 1"
**Time: T+0s | User clicks play button**

```
Client Action:
├─ App detects: episode_1 has blob_hash "sha256-podcast-ep-1"
├─ Calls: BlobCacheTiersService.getBlob(
│          hash: "sha256-podcast-ep-1",
│          reachLevel: 7  // commons
│        )
│
└─ WASM Cache Lookup (O(1)):
   ├─ reach_caches[7].get("sha256-podcast-ep-1")
   ├─ HashMap lookup: O(1)
   └─ Time: 0.1ms
       (Already cached from previous seeding? Depends on reach level)
```

**First Request Path** (blob not in cache yet):

### Step 4b: Blob Metadata Retrieved from Doorway
**Time: T+5ms**

```
Browser → Doorway GET /api/blobs/{hash}/metadata

Doorway Processing:
├─ Worker queries ContentCache for blob metadata
├─ Metadata includes:
│  ├─ size: 2_100_000_000 (2.1GB)
│  ├─ codec: "h264"
│  ├─ resolutions: ["480p", "720p", "1080p"]
│  ├─ custodian_commitments: [
│  │  {
│  │    custodian_agent_id: "agent-alice",
│  │    location: "Portland, OR",
│  │    cache_priority: 85,
│  │    bandwidth_class: "ultra",
│  │    distance_km: 0  // Local
│  │  },
│  │  {
│  │    custodian_agent_id: "agent-bob",
│  │    location: "Seattle, WA",
│  │    cache_priority: 70,
│  │    bandwidth_class: "high",
│  │    distance_km: 200
│  │  },
│  │  {
│  │    custodian_agent_id: "agent-charlie",
│  │    location: "San Francisco, CA",
│  │    cache_priority: 60,
│  │    bandwidth_class: "medium",
│  │    distance_km: 600
│  │  }
│  │ ]
│  │
│  └─ Recommended source:
│     { agent: "agent-alice", score: 100 }
│     (Local, high priority, ultra bandwidth)
│
└─ Return to client
```

**Custodian Selection Algorithm** (in Doorway):
```rust
priority_score = cache_priority             // 0-100
               + bandwidth_bonus            // -5 to +20
               - distance_penalty           // distance_km / 10

Sorted sources by score (descending)
Alice:   85 + 20 - 0 = 105 ✓ Best choice
Bob:     70 + 10 - 20 = 60
Charlie: 60 + 5 - 60 = 5
```

### Step 4c: Blob Download from Primary Custodian (Local Cache)
**Time: T+10ms - T+5m (depending on resolution)**

```
Client → Custodian "agent-alice" (LOCAL!)

Request: GET /blobs/sha256-podcast-ep-1?resolution=720p

Custodian's Holochain-Cache-Core:
├─ Incoming request for blob 720p variant
├─ WASM Cache Lookup:
│  ├─ hash = "sha256-podcast-ep-1:720p"
│  ├─ reach_level = 7 (commons)
│  ├─ Cache lookup: reach_caches[7].get(hash)
│  └─ Time: 0.1ms
│
├─ CACHE HIT! (Custodian has it pre-cached)
│  ├─ Return cached blob data
│  ├─ Stream 1.2GB video over local network
│  ├─ Network bandwidth: 100Mbps (local)
│  └─ Transfer time: ~100 seconds
│
└─ CACHE MISS (Custodian doesn't have this resolution):
   ├─ Fallback to secondary source:
   │  └─ Try "agent-bob" (Seattle)
   └─ Network bandwidth: 50Mbps (internet)
      └─ Transfer time: ~200 seconds
         (Still acceptable for podcast watching)
```

**Key Protection**: Custodian's WASM cache (holochain-cache-core) handles:
- Fast O(1) lookups
- Thread-safe blob serving
- Multiple resolutions (480p, 720p, 1080p)
- Fallback to peer caches

### Step 4d: Streaming with Parallel Peer Caches
**Time: T+10ms - T+5m (streaming in progress)**

```
While Custodian "agent-alice" streams primary copy:

App can ALSO query peer caches in parallel:
├─ PeerCache "agent-bob" (Prefetch secondary)
│  ├─ Query: Has 720p variant cached?
│  └─ Response: Yes, 95% complete
│
└─ PeerCache "agent-charlie" (Fallback)
   ├─ Query: Has 480p variant cached?
   └─ Response: Yes, 100% complete

Resilience:
├─ Primary (Alice) fails → Seamless fallback to Bob
├─ Bob fails → Fallback to Charlie
├─ Charlie fails → Re-request from ORIGINAL OWNER
└─ Network failure → Resume from cached portion
   (IndexedDB persists partial downloads)
```

---

## Part 5: Attestation Flow & Content Graph Update

### Step 5a: User Watches & Completes Episode
**Time: T+0-55min (episode duration)**

```
During Playback:
├─ App tracks user engagement:
│  ├─ watched_seconds: 0 → 3300 (55 minutes)
│  ├─ play_started: T+0s
│  ├─ play_completed: T+55m
│  ├─ last_engagement: "watch"
│  └─ engagement_count: +1
│
└─ App stores locally in IndexedDB (immediate)
   └─ No network call yet
```

### Step 5b: User Marks Episode as "Understood"
**Time: T+55m | User rates understanding**

```
Client Action:
├─ User clicks: "I understood this content"
├─ App calls: zome function update_content_mastery({
│              human_id: "user-xyz",
│              content_id: "sheila-ep-001",
│              mastery_level: "understand",  // Level 3
│              engagement_type: "watch",
│              last_engagement_timestamp: now()
│            })
│
└─ Request → Doorway → Conductor
```

**Conductor Processing**:
```rust
DNA processes update_content_mastery:
├─ Retrieves existing ContentMastery entry (if exists)
├─ Updates:
│  ├─ mastery_level: "understand" (level 3)
│  ├─ freshness_score: 0.95 (high)
│  ├─ engagement_count: 1
│  ├─ last_engagement_type: "watch"
│  └─ assessment_evidence_json: { "watched_minutes": 55 }
│
├─ Creates/updates entry on source chain
├─ Posts to DHT (replicated to neighbors)
│
└─ POSTS POST_COMMIT SIGNAL:
   └─ MasteryUpdated { human_id, content_id, mastery_level, ... }
      ↓ Projection engine receives
```

### Step 5c: Content Mastery Projection Updated
**Time: T+55m + 50ms**

```
Projection Engine:
├─ Receives MasteryUpdated signal
├─ Transforms to ContentMasteryProjection
├─ Stores in MongoDB: {
│  {
│    id: "mastery-user-xyz-sheila-ep-001",
│    human_id: "user-xyz",
│    content_id: "sheila-ep-001",
│    mastery_level: 3,  // "understand"
│    freshness_score: 0.95,
│    last_engagement: "watch",
│    engagement_count: 1,
│    updated_at: now()
│  }
│ }
│
└─ Cache invalidation:
   ├─ Invalidate: get_human_mastery_for_content (user-specific, not cached)
   └─ Note: Mastery is user-specific, never cached globally
      (Always fresh from MongoDB)
```

### Step 5d: Attestation Evaluation (Automatic)
**Time: T+55m + 100ms**

```
Background Job (runs on app server):

For each ContentMastery update:
├─ Check: mastery_level >= 4 (Apply level)?
│  └─ Yes → User unlocked "Apply" privilege level
│
├─ Check: All concepts in series at "understand"+ ?
│  └─ Yes → User earned "Path Completion Attestation"
│
├─ Create Attestation entry:
│  {
│    id: "att-user-xyz-path-sheila-complete",
│    agent_id: "user-xyz",
│    category: "path-completion",
│    attestation_type: "path_completion_v1",
│    display_name: "Completed Sheila Wray Gregoire Marriage Series",
│    description: "User completed all episodes in the series at Understand+ level",
│    tier: "silver",  // Multi-episode completion
│    issued_at: now(),
│    issued_by: "system",
│    proof: sign_with_private_key(...)  // Cryptographic signature
│  }
│
└─ Zome call: create_attestation(att_data)
   └─ Posted to DHT
      └─ Signal: AttestationCreated
         ├─ Propagates to all peers
         └─ Added to human's profile
```

### Step 5e: Content Graph Updated with Attestations
**Time: T+55m + 150ms**

```
Create Links (content graph):
├─ Link: ContentMastery → Content
│  ├─ From: mastery-user-xyz-sheila-ep-001
│  ├─ To: sheila-ep-001
│  ├─ Tag: "mastered_by"
│  └─ Visible to: User + admins
│
├─ Link: Attestation → Content
│  ├─ From: att-user-xyz-path-sheila-complete
│  ├─ To: sheila-series
│  ├─ Tag: "earned_via_content"
│  └─ Visible to: Everyone (public credential)
│
├─ Link: Human → Attestation
│  ├─ From: user-xyz
│  ├─ To: att-user-xyz-path-sheila-complete
│  ├─ Tag: "has_attestation"
│  └─ Visible to: Everyone (public profile)
│
└─ DHT Replication:
   ├─ Links replicated to neighbor peers
   ├─ Propagation: 30-60 seconds
   └─ Graph now queryable: "all users who mastered Sheila series"
```

### Step 5f: Cache Rules Triggered for Invalidation
**Time: T+55m + 200ms**

```
Cache invalidation based on write operations:

Operations that triggered:
├─ update_content_mastery → Invalidates:
│  ├─ get_human_mastery_for_content (user-specific, uncached anyway)
│  └─ get_content_stats (aggregate stats affected)
│
├─ create_attestation → Invalidates:
│  ├─ get_human_attestations (user profile)
│  └─ get_leaderboard (achievement rankings)
│
└─ create_link → Invalidates:
   ├─ get_content_graph (relationships)
   ├─ get_related_contents (recommendations)
   └─ get_path_completion_status (learning path tracking)

Cache Rule Example:
```
CacheRuleBuilder::new("get_content_graph")
    .ttl_15m()
    .reach_based("root.content.reach", "commons")
    .invalidated_by(vec!["create_content", "create_relationship", "create_attestation"])
    .build()
```

Next query to get_content_graph will:
├─ Find cache key: "dna:content_store:get_content_graph:sheila-series:commons"
├─ Cache expired (invalidation cleared it)
└─ Fresh query from conductor → new data with attestation links
```

### Step 5g: Peer Discovers Updated Content Graph
**Time: T+55m + 2m**

```
Peer "agent-bob" (in Seattle):

Receives DHT signals:
├─ MasteryUpdated { user: user-xyz, content: sheila-ep-001, level: understand }
├─ AttestationCreated { user: user-xyz, attestation: path-complete, ... }
└─ LinkCreated { from: user-xyz, to: attestation, tag: "has_attestation" }

Projection engine on Bob's instance:
├─ Updates local MongoDB:
│  ├─ UserProfile: Adds attestation to user-xyz's credentials
│  ├─ ContentStats: Updates "mastered_by" count for sheila-ep-001
│  └─ Graph index: Updates "who completed this series" query
│
└─ Cache invalidations:
   ├─ Clears: get_human_attestations (if cached)
   ├─ Clears: get_leaderboard
   └─ Next query will fetch fresh data
```

**Multi-Peer Attestation Flow**:
```
User (Portland) → Agent-alice (local)
                ↓ DHT Signal
              ↑↓ Peers replicate signals
             ↙ ↘
    Agent-bob    Agent-charlie
    (Seattle)    (San Francisco)

Each peer independently:
├─ Validates cryptographic signatures
├─ Stores in local MongoDB
├─ Updates local projections
└─ Next user query sees attestations

Consensus: Eventual (DHT propagation ~2 minutes)
Conflict resolution: Timestamps + signatures
Authority: Entry author's signature is proof
```

---

## Part 6: Subsequent Peer Access (Cache Chain)

### Step 6a: Another User in Seattle Queries Same Podcast
**Time: T+1hr | Different user, same episode**

```
User (Bob's peer) → Agent-bob queries doorway cache

Request: GET /api/content/sheila-ep-001

Doorway Cache on agent-bob:
├─ Cache key: "dna:content_store:get_content:sheila-ep-001:commons"
├─ Cache hit! (populated from DHT replication earlier)
│  └─ Metadata: {
│      title: "...",
│      reach: "commons",
│      mastery_count: 1,  ← Shows user mastered it!
│      attestation_count: 1,  ← Shows credential created
│      related_attestations: [
│        { user: user-xyz, type: "path_completion", ... }
│      ]
│    }
│
└─ Return immediately: 5ms
```

**Cascading Benefits**:
```
User-alice downloads from agent-alice (Portland)
  ├─ Generates engagement signal
  ├─ Creates attestation
  └─ DHT replicates to agent-bob

User-bob (Seattle) queries agent-bob
  ├─ Sees user-alice's attestation
  ├─ Knows "someone has mastered this"
  ├─ Cache hit (fast response)
  └─ Blob cached in agent-bob too
     (from custodian replication earlier)

User-bob downloads from agent-bob (local!)
  ├─ 100x faster than origin (local network)
  ├─ Reduces burden on original author
  └─ Peer-to-peer distribution efficiency
```

### Step 6b: Blob Cache Chain (Origin → Custodian → Peer)
**Time: T+1h:5m | Blob download**

```
Origin (Original author):
├─ Hosts master copy: sheila-ep-001.mp4 (2.1GB)
├─ Blob stored on source chain
└─ DHT neighbors replicate

Custodian "agent-alice" (Volunteer):
├─ Downloaded via DHT replication
├─ Committed to cache this content
│  └─ CustodianCommitment: {
│      beneficiary: "original-author",
│      custodians: ["agent-alice"],
│      shard_strategy: "full_replica",
│      cache_priority: 90,
│      bandwidth_class: "ultra"
│    }
├─ Stored in holochain-cache-core
│  └─ reach_caches[7].put({ hash, blob, ... })
└─ Serves local users fast

Peer "agent-bob" (Can cache too):
├─ Not in official custodian commitments
├─ But can cache locally via holochain-cache-core
├─ When user-bob downloads:
│  ├─ Gets primary copy from agent-alice
│  ├─ Caches locally (reaching capacity limit)
│  └─ Now serves to other Seattle users
└─ holochain-cache-core handles:
   ├─ Automatic reach-level isolation
   ├─ LRU eviction if over capacity
   └─ Affinity tracking for recommendations
```

### Step 6c: Network Protection (No Conductor Overload)
**Time: Throughout all requests**

```
Protecting Holochain Conductor from Thread Starvation:

Threat Model:
├─ 10,000 concurrent users want "Sheila Gregoire" podcast
├─ Without cache: 10,000 zome calls to conductor
│  └─ Conductor thread pool exhausted
│     └─ Other operations blocked (transactions, DHT validation, etc.)
│
└─ WITH Cache:
   ├─ First request (cache miss): 1 zome call to conductor
   ├─ Next 9,999 requests: Served from cache (0 conductor calls)
   └─ Conductor remains responsive
      └─ Continues DHT consensus, validation, writes

Worker Pool Queue (Doorway):
├─ All 10,000 requests queue in MPSC channel
├─ 4 workers process continuously
├─ Semaphore limits: 1000 max concurrent
│  └─ Excess requests wait peacefully
│     └─ No busy-spinning, no resource waste
└─ Result: Fair FIFO scheduling, no starvation

Thread Count by Operation:

Cache Hit (99%):
├─ Browser → Doorway (1 worker)
│  ├─ Time: 5ms
│  └─ Conductor threads: 0
└─ Total conductor impact: ZERO

Cache Miss (1%):
├─ Browser → Doorway (1 worker)
├─ Doorway → Conductor (1 WASM thread)
│  ├─ Time: 100ms
│  └─ Conductor threads: 1
├─ Conductor → DHT (network)
│  └─ Doesn't block other zome calls
└─ Result: Minimal impact even on miss
```

---

## Part 7: Attestation Integrity via Content Graph

### Step 7a: How Attestations Remain Linked to Content
**The Content Graph DAG Structure**

```
Content Graph (DAG - Directed Acyclic Graph):

        Original Podcast Entry
        (action_hash: A1)
              ↓
        Content Metadata
        ├─ title, description
        ├─ reach: "commons"
        └─ blob_hash: "sha256-podcast-ep-1"
              ↓
        Posted to DHT
              ↓
        Peer Replication
        (3 copies maintained)
              ↓

        Relationships Created
        ├─ CONTAINS: "sheila-series" → "sheila-ep-001"
        ├─ RELATES_TO: "sheila-ep-001" → "concept-marriage"
        └─ DERIVED_FROM: "sheila-ep-001" → "original-author"
              ↓
        User Mastery Tracking
        ├─ "user-xyz" reached "understand" level
        └─ Last_engagement: watch (55 minutes)
              ↓
        Attestation Created
        ├─ "Path Completion" attestation
        ├─ Signed by: "system" (verified hash)
        └─ Links to: original content + mastery entry
              ↓
        Content Graph Link
        ├─ Link: Attestation → Content
        │  └─ Proves "credential earned from this content"
        ├─ Link: Human → Attestation
        │  └─ Proves "human earned this credential"
        └─ Link: Human → Content (mastery)
           └─ Proves "human mastered this content"
```

### Step 7b: Verification Flow
**How to Verify Attestation Authenticity**

```
Query: "Did user-xyz really master Sheila Gregoire episode 1?"

Resolution Process:

1. Find Attestation:
   ├─ Query: get_attestation("att-user-xyz-path-sheila-complete")
   ├─ Returns: {
   │  id: "att-user-xyz-path-sheila-complete",
   │  agent_id: "user-xyz",
   │  issued_at: "2024-01-15T10:30:00Z",
   │  proof: "sig_..."  ← Cryptographic signature
   │ }
   └─ Verify: sign_verify(proof, attestation_data)

2. Find Mastery Entry:
   ├─ Link: get_links_to_attestation("att-user-xyz-...", "earned_via_content")
   ├─ Link resolution: Follows back to content ID
   └─ Mastery entry ID: "mastery-user-xyz-sheila-ep-001"

3. Find Original Content:
   ├─ Query: get_content("sheila-ep-001")
   ├─ Returns: {
   │  id: "sheila-ep-001",
   │  action_hash: "uhCEkY...",  ← Immutable DHT hash
   │  author_address: "agent-sheila",
   │  reach: "commons",
   │  created_at: "2023-08-01T...",
   │  ...
   │ }
   └─ Verify: author_signature matches

4. Chain of Authority:
   ├─ Sheila creates content (signature: Sheila's public key)
   ├─ Holochain DHT validates on publish
   ├─ All peers replicate (validated entry)
   ├─ User engages + creates mastery entry
   ├─ System creates attestation (signed: system key)
   ├─ Peers replicate attestation (validated)
   └─ Result: Cryptographically verified credential

5. Graph Traversal Proof:
   ├─ Query: get_credential_evidence("user-xyz", "att-path-complete")
   ├─ Returns: {
   │  user: "user-xyz",
   │  attestation: "att-...",
   │  mastery_entries: [
   │    { content: "sheila-ep-001", level: "understand", watched_minutes: 55 },
   │    { content: "sheila-ep-002", level: "understand", watched_minutes: 48 },
   │    ... (all episodes)
   │  ],
   │  content_links: [
   │    { content_id: "sheila-ep-001", relationship: "RELATES_TO", targets: ["concept-marriage", ...] },
   │    ...
   │  ]
   │ }
   └─ User proved understanding of all episodes + related concepts
```

### Step 7c: Delegation & Trust Chain
**How Attestations Work in Steward Economy**

```
Sheila Wray Gregoire (Content Creator):
├─ Creates series: "Marriage, Intimacy, and Faith"
├─ Posts to Holochain DHT
├─ Allows: "user understanding creates public attestation"
└─ Doesn't manage attestations (system automated)

System (Validator):
├─ Rules: When mastery_level >= "apply":
│  └─ Create attestation automatically
├─ Proof: Signature from system key
└─ Verifiable: All peers can validate

User (Learner):
├─ Engages: Watches podcast episodes
├─ Demonstrates: Mastery of concepts
├─ Earns: Attestation (public credential)
└─ Benefits: Unlock new paths, jobs, community

Peer (Validator):
├─ Receives: Attestation via DHT
├─ Validates: Signature + content link
├─ Stores: In local MongoDB + cache
└─ Queries: Can verify user's credentials

Employment/Community Use:
├─ Employer queries: "find Christians with marriage expertise"
├─ Query: WHERE attestation.category = "domain-mastery"
│         AND attestation.domain = "marriage"
│         AND user.location NEAR 'Portland'
├─ Results: List of credentialed users
└─ Hiring: Based on verifiable accomplishment (not resume claims)
```

---

## Summary: Complete Data Flow

```
SEED PHASE (T=0):
Seed script → Conductor → DHT → Projections → Cache populated

BOOTSTRAP PHASE (T+0s):
Holochain → Doorway → Worker pool → Cache layer ready
App boot → holochain-cache-core WASM loaded → Ready

USER ACCESS PHASE (T+1h):
User → App → WASM cache hit (O(1)) OR cache miss → Doorway
  ↓
If cache hit: Return in 5ms (no conductor)
If cache miss: Doorway → Conductor → DHT → Response → Cache

BLOB DELIVERY PHASE (T+1h:5m):
User → Custodian selection (by score)
  ↓
Custodian blob download → Transfer via peer network
  ↓
Optional: Fallback to secondary/tertiary custodian

ATTESTATION PHASE (T+1h:55m):
User mastery → Conductor zome call (one time)
  ↓
DHT replication → All peers store attestation
  ↓
Projection index → Queryable by: user, content, type

VERIFICATION PHASE (Anytime after):
Query: "Is this user credentialed?"
  ↓
Traverse: Attestation → Mastery → Content → Author signature
  ↓
Result: Cryptographically verified credential chain

PROTECTION SUMMARY:
├─ Conductor thread starvation: Protected by 99% cache hit rate
├─ Network efficiency: Custodian + peer caching reduces origin load
├─ Content graph integrity: DHT replication + signature verification
├─ Attestation authenticity: Cryptographic proofs + peer validation
└─ User privacy: Reach-level isolation prevents unauthorized access
```

---

## Diagram: Multi-Layer Architecture

```
                        BROWSER
                    (Angular App)
                          ↓
                    App Initialization
                          ↓
        ┌───────────────────┬───────────────────┐
        ↓                   ↓                   ↓
   HolochainClient   holochain-cache-core  BlobBootstrap
   (API access)      (WASM - O(log n))      (DHT waiting)
        ↓                   ↓                   ↓
        └───────────────────┴───────────────────┘
                          ↓
                  BlobCacheTiersService
              (Tier 1: metadata | Tier 2&3: blobs)
                          ↓
                   Doorway REST API
              (0.0.0.0:3000 - HTTP server)
                          ↓
        ┌─────────────────────────────────────┐
        │      Worker Pool (4 workers)        │
        │   Semaphore-limited queue (1000)    │
        │      MPSC channel (backpressure)    │
        └─────────────────────────────────────┘
                          ↓
        ┌─────────────────────────────────────┐
        │    ContentCache (in-memory)         │
        │   Cache hits: 99% response time 5ms │
        │   Cache misses: Forward to conductor│
        └─────────────────────────────────────┘
                          ↓
              Holochain Conductor (PROTECTED)
           (Only receives 1% of requests)
                          ↓
        ┌─────────────────────────────────────┐
        │    Holochain DHT (Kitsune network)  │
        │     - Peer replication              │
        │     - Eventual consistency           │
        │     - Cryptographic verification    │
        └─────────────────────────────────────┘
                          ↓
        ┌─────────────────────────────────────┐
        │  Projection Engine (Signal Handler) │
        │  Transforms DHT entries → indexes   │
        └─────────────────────────────────────┘
                          ↓
        ┌─────────────────────────────────────┐
        │    MongoDB (Projections store)      │
        │    - Full-text search indexes       │
        │    - User mastery tracking          │
        │    - Attestation history            │
        └─────────────────────────────────────┘


PEER-TO-PEER BLOB DELIVERY:
┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│  Origin User │         │ Custodian-A  │         │  Peer User   │
│  (Author)    │         │  (Volunteer) │         │  (Consumer)  │
└──────┬───────┘         └───┬──────────┘         └──────┬───────┘
       │ publishes            │ caches                    │
       ├──→ DHT ──→ replicates to ──→ serves peers ──→ queries
       │   signature           │ holochain-cache-core
       │   verification        │ (O(1) lookups)
       └─ immutable ───────────┴────────────────────────────────
        (all peers validate)

ATTESTATION & CONTENT GRAPH:
┌──────────────┐
│ Content Hash │ ← immutable (DHT)
└──────┬───────┘
       ↓
┌──────────────┐
│ User Mastery │ ← evidence (DHT)
└──────┬───────┘
       ↓
┌──────────────────┐
│ Attestation      │ ← credential (DHT) + cryptographic proof
│ (signed)         │
└──────┬───────────┘
       ↓
┌──────────────────────┐
│ Content Graph Links  │ ← queryable (MongoDB indexes)
│ (relationships)      │
└──────────────────────┘
       ↓
┌──────────────────────┐
│ Verified Credential  │ ← verifiable by any peer
└──────────────────────┘
```

---

## Performance Metrics

```
Podcasta Streaming - Sheila Wray Gregoire Series:

Cold Start (First Request):
├─ User boots app: 500ms (holochain-cache-core loads)
├─ Seed runs: 2-5 minutes (50+ episodes to DHT)
├─ Projections populated: ~2 minutes
└─ Ready for user access: T+5-7 minutes

First User Access:
├─ List podcasts: 50-100ms (cache miss → conductor → response → cache)
├─ Get blob metadata: 10-20ms (cache miss → conductor)
├─ Start playback: 100-500ms (first connection to custodian)
└─ Streaming: 100Mbps local network, 50Mbps internet

Subsequent Users:
├─ List podcasts: 5-10ms (cache hit)
├─ Get metadata: 5-10ms (cache hit)
├─ Start playback: 50-200ms (established peer connection)
└─ Streaming: 100Mbps+ (multiple custodians)

Scaling (10,000 concurrent users):
├─ Without cache: 10,000 conductor calls/sec → SATURATED
├─ With cache (99% hit): 100 conductor calls/sec → HEALTHY
├─ Worker pool: 4 workers handle 10,000 queued requests
├─ Semaphore: Fair FIFO, no starvation
└─ Result: All users served, no DOS

Attestation Verification:
├─ Query for credential: 5-10ms (MongoDB index)
├─ Verify signature: 1-5ms (local crypto)
├─ Traverse graph: 10-50ms (DHT lookup)
└─ Complete verification: 20-60ms
```

---

## Conclusion: The Complete System

The new holochain-cache-core provides:

1. **Thread Protection**: 99% cache hit rate keeps conductor responsive
2. **Performance**: 100-1000x faster blob caching via WASM
3. **Distribution**: Peer-to-peer blob caching reduces origin load
4. **Integrity**: Cryptographic verification maintains content graph truth
5. **Attestations**: Linked to content via DHT entries + projections
6. **Resilience**: Fallback custodians + peer caching provide redundancy
7. **Privacy**: Reach-level isolation in WASM cache prevents unauthorized access

When the seed runs and a user accesses a podcast, the entire system works together:
- **Seed** populates DHT with content + relationships
- **Projections** create searchable indexes
- **WASM cache** serves 99% of requests instantly
- **Custodians** replicate blobs for fast distribution
- **Peers** cache and serve locally
- **Attestations** verify credentials via content graph
- **Conductor** remains responsive and available

**The result**: Seamless, scalable, peer-to-peer content delivery with verifiable credentials, all protected by high-performance caching that prevents thread starvation.
