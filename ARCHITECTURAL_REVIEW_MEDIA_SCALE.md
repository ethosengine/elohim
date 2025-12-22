# Architectural Review: Large Media & Real-Time at P2P Scale

**Date:** 2025-12-22
**Status:** Critical Review - Gaps Identified
**Concern Level:** ⚠️ HIGH for production deployment

## Executive Summary

The Elohim Protocol has a **well-designed foundational architecture** with proper separation of concerns, but **critical gaps exist** for large media delivery (video, podcasts, Zoom calls) at peer-to-peer scale. The current design assumes:

- Small payloads (~1-100 KB JSON documents)
- Complete payload delivery in single request
- Content hosted on external URLs (video-embed, external-link)
- Centralized or high-bandwidth infrastructure

**Problem:** Large media (podcasts 50-200 MB, videos 500 MB - 10 GB, real-time streams) cannot be efficiently delivered through the current architecture without fundamental changes.

## Current Architecture Assessment

### ✅ What's Done Right

#### 1. Excellent Separation of Concerns

The system properly isolates responsibilities across layers:

```
┌─────────────────────────────────────────────┐
│ APPLICATION LAYER (Steward/Lamad)           │
│  - ContentNode models                       │
│  - Lifecycle policies                       │
│  - Reach/trust definitions                  │
└──────────────────┬──────────────────────────┘
                   │
┌──────────────────▼──────────────────────────┐
│ HOLOCHAIN DHT LAYER (Source of Truth)       │
│  - Immutable content metadata               │
│  - Authority/authorship tracking            │
│  - Cryptographic integrity                  │
│  - P2P gossip protocol                      │
└──────────────────┬──────────────────────────┘
                   │
┌──────────────────▼──────────────────────────┐
│ PROJECTION LAYER (Hot Read Cache)           │
│  - MongoDB for complex queries              │
│  - DashMap for hot data (<10k entries)      │
│  - Signal-driven updates                    │
└──────────────────┬──────────────────────────┘
                   │
┌──────────────────▼──────────────────────────┐
│ DOORWAY GATEWAY (HTTP API)                  │
│  - Reach-aware access control               │
│  - ETag-based caching                       │
│  - Worker pool for async zome calls         │
│  - NATS for multi-node coordination         │
└──────────────────┬──────────────────────────┘
                   │
┌──────────────────▼──────────────────────────┐
│ HTTP RESPONSE LAYER (Client Interface)      │
│  - Cache headers (max-age, stale-while)     │
│  - Content-Type negotiation                 │
│  - Compression (gzip/brotli)                │
└─────────────────────────────────────────────┘
```

**Strengths:**
- ✅ Each layer has single responsibility
- ✅ Holochain DHT never stores large blobs (correct!)
- ✅ Projection handles read optimization (smart!)
- ✅ Reach-aware serving is layer-agnostic
- ✅ Cache architecture extensible (can add tiers)

#### 2. Smart Reach-Level Access Control

The reach model is sophisticated:

```
Reach Level      Authentication    Use Case                Scale
──────────────   ──────────────    ────────────────────    ────────
private          Owner only        Personal notes          1 person
invited          Whitelist         Shared workspace        10-50
local            Location/family   Household data          5-20
neighborhood     Authenticated     Community events        100-1000
municipal        Authenticated     Public announcements    10k-100k
bioregional      Authenticated     Environmental data      100k-1M
regional         Authenticated     State resources         1M-10M
commons          Everyone          Published knowledge     10M+
```

**Problem:** Reach level is **decoupled from delivery strategy**. You can serve 1 TB video with same code as 1 KB JSON doc.

#### 3. Solid Content Lifecycle

Content has versioning and deprecation:

```
draft → published → stale → deprecated → archived → forgotten
         │                    │
         visible to          refreshInterval
         reach level         (P6M, P1Y, P2Y)
```

**Problem:** Lifecycle tracks *logical* state, not *physical* resource state. A 4K video doesn't track:
- Codec obsolescence (H.264 → H.265 → VP9 → AV1)
- Bitrate requirements (4K needs 25-50 Mbps)
- Device compatibility (iPhone 8 can't decode certain codecs)

---

### ⚠️ Critical Gaps

#### Gap 1: No Blob Storage Strategy

**Current State:**
```typescript
contentFormat: "video-embed" | "video-file" | "external-link"
```

The system references external content but has **no built-in strategy** for:

1. **Where blobs live**
   - DHT? ❌ (Holochain not designed for >1 MB)
   - External S3? ⚠️ (Centralized, unreliable)
   - IPFS? ⚠️ (Not integrated)
   - Local network? ⚠️ (Peer might be offline)

2. **How they're referenced**
   ```typescript
   // Current
   metadata: { embedStrategy: 'iframe' }  // iframe src = external URL

   // Missing
   - Blob hash (for integrity verification)
   - Fallback URLs (for redundancy)
   - Bitrate variants (for adaptive delivery)
   - Expected size (for caching decisions)
   - Partial content support (for resumable downloads)
   ```

3. **Lifecycle of blobs**
   - No tracking when external URL goes dead
   - No automatic re-hosting if peer goes offline
   - No garbage collection if content is deprecated

**Impact for Podcasts/Videos:**
- Podcast (100 MB): Hosted where? If external URL dies, gone forever
- Video (2 GB): Who seeds it after creator goes offline?
- Zoom recording (500 MB): Stored in private reach level - how is it distributed to invited users?

---

#### Gap 2: No Streaming Support

**Current Delivery Model:**
```
Client Request
    ↓
Doorway checks cache
    ↓
If miss: Full payload loaded from Holochain
    ↓
Entire response cached in DashMap (max 10k entries)
    ↓
Entire response sent to client (Content-Length header)
    ↓
Client receives complete file or timeout
```

**Problems:**

1. **No Range Requests (HTTP 206)**
   - Can't resume interrupted downloads
   - Must re-download entire 1 GB video if connection drops
   - Can't seek in video timeline without full download

2. **Memory Ceiling**
   ```
   Max cache entries: 10,000
   Average entry: 1 KB to 1 GB

   If 1% of entries are 1 GB videos:
   100 entries × 1 GB = 100 GB RAM needed

   Holochain node with 8 GB RAM: 0.08% video capacity
   ```

3. **No Streaming Format Support**
   - HLS (HTTP Live Streaming): Requires .m3u8 manifest + TS chunks
   - DASH (Dynamic Adaptive Streaming): Requires MPD manifest + MP4 segments
   - Both require chunked delivery, not supported

4. **No Adaptive Bitrate**
   - Client on 5G getting same bitrate as client on 4G
   - No fallback quality if bandwidth drops

**Impact for Zoom Calls:**
- Live recording at 60 Mbps? Entire stream sits in cache
- 1-hour Zoom call = ~27 GB stored in one node's RAM
- Reaches scale: 2 concurrent calls on small deployment = ⚠️ OOM

---

#### Gap 3: Cache Architecture Not Media-Aware

**Current Cache:**
```rust
pub struct CacheConfig {
    pub max_entries: 10_000,           // By count
    pub content_ttl: Duration::from_secs(3600),  // 1 hour
    pub list_ttl: Duration::from_secs(300),      // 5 min
    pub user_ttl: Duration::from_secs(60),       // 1 min
}
```

**LRU Eviction Problem:**
```
Scenario: Node has 8 GB RAM cache

Time 1: Download 5 KB JSON doc → cached
Time 2: Download 2 GB podcast → cached, evicts JSON
Time 3: 8 more podcasts queued → each evicts older ones
Time 4: User requests original JSON → MISS (podcast eviction chain)

Cache hit rate: 20% instead of 80%
```

**Missing:**
- Size-aware limits (max 1 GB cache for large media)
- Frequency-based eviction (hot content stays, cold goes)
- Reach-level boundaries (private content never evicts commons content)
- Tier separation (metadata cache ≠ blob cache)

---

#### Gap 4: Bandwidth Awareness Defined but Unused

**Fields Exist:**
```rust
pub struct CustodianSource {
    pub bandwidth_class: String,    // "low" | "medium" | "high" | "ultra"
    pub cache_priority: u32,        // 0-100
    pub distance_km: Option<f64>,
}
```

**Usage:** ZERO
- Custodian Commitment model not yet implemented
- No code checks bandwidth_class when serving content
- Reach-aware serving doesn't consider bandwidth

**Missing:**
```
Reach Level + Bandwidth = Delivery Strategy

Example:
- commons + low_bandwidth = 480p max, 2 Mbps
- commons + high_bandwidth = 4K, 25 Mbps
- private + low_bandwidth = reject (too slow to serve securely)
```

---

#### Gap 5: P2P Network Assumptions Broken at Scale

**Current P2P Model:**
```
Agent A (has content)
         ↓ (gossip protocol)
      DHT
         ↓ (gossip protocol)
Agent B (finds content)
         ↓
    Conductor API call
         ↓
Agent A's storage (where?)
```

**Works for:** Small documents (1-100 KB)
**Breaks for:** Large media

**Why:**

1. **DHT Gossip Not Designed for Media**
   - Holochain shares metadata + small payloads
   - Large files would saturate P2P network
   - 100 nodes sharing 1 GB video = 100 GB gossip traffic

2. **Source Availability Assumption**
   - If peer has video and goes offline, video is lost
   - No built-in replication except manual custodian commitments
   - Podcast creator uploads video, then closes laptop = peer offline

3. **Latency Profile**
   - Small JSON: 50-200 ms round trip (acceptable)
   - 1 GB video: 500-5000 seconds on typical connections (unacceptable)
   - Holochain assumes request → response in seconds, not hours

4. **Network Topology Mismatch**
   - Holochain P2P is optimized for sparse, well-distributed queries
   - Media delivery needs **broadcast trees** (one source → many mirrors)
   - Requires **pull-based replication** (not Holochain's model)

---

#### Gap 6: Real-Time Communication Impossible

**Zoom/Live Streaming Requirements:**
```
Latency:     < 500 ms round trip
Bandwidth:   10-50 Mbps for HD
Presence:    Real-time connection tracking
Synchronization: Audio/video lip sync
Fallback:    Instant codec switch if bandwidth drops
```

**Current System Can Do:**
```
✅ Store recording after call ends
✅ Serve recording at user's reach level
❌ Live streaming to multiple users in real-time
❌ Real-time presence tracking
❌ Sub-second latency for interactive content
❌ Adaptive bitrate mid-stream
```

---

## Architecture by Media Type

### 1. Podcasts (50-200 MB)

**Current Approach:**
```
ContentNode.contentFormat = "external-link"
metadata.url = "https://cdn.example.com/podcast.mp3"
```

**Problem:** If external URL dies, content is gone forever

**What's Needed:**
```
┌─ Podcast Manifest (in DHT)
│  ├─ Title, description, duration
│  ├─ Blob hash (SHA256)
│  ├─ Fallback URLs (primary, secondary, tertiary)
│  └─ Expected size + bitrate
│
└─ Podcast Blob (NOT in DHT)
   ├─ Stored in Custodian network
   ├─ Replicated to 3+ peers based on reach level
   └─ Served with HTTP Range support
       (resume at 45:23 marker without re-downloading)
```

**At Scale (10,000 podcasts):**
- Metadata in DHT: ~500 KB per podcast = 5 GB (manageable)
- Blobs distributed: 3 replicas each = 600 GB across network
- Hot cache: Top 10 podcasts = ~1.5 GB RAM (manageable)

---

### 2. Videos (500 MB - 10 GB)

**Current Approach:**
```
ContentNode.contentFormat = "video-embed"
metadata.embedStrategy = "iframe"
// Embedded YouTube/Vimeo player
```

**Problem:** Dependent on YouTube/Vimeo uptime; not private

**What's Needed:**
```
┌─ Video Manifest (in DHT)
│  ├─ Title, description, duration, codec, resolution
│  ├─ Chunk list (hash of each 10 MB segment)
│  ├─ Bitrate variants (480p, 720p, 1080p, 4K)
│  ├─ Subtitle tracks + availability
│  └─ Reach level + expiration
│
└─ Video Chunks (distributed across custodians)
   ├─ 10 MB segments (standard chunk size)
   ├─ HLS manifest for adaptive streaming
   ├─ DASH manifest as alternative
   └─ Served with HTTP 206 Partial Content support
```

**At Scale (1,000 HD videos):**
- Metadata: ~200 KB each = 200 MB (DHT)
- Blobs: 3 replicas of 1.5 GB avg = 4.5 TB distributed
- Hot cache: Top 100 videos cached = 150 GB (requires external blob store)

---

### 3. Zoom Calls (Live + Recording)

**Current Approach:**
```
❌ Not supported for live streaming
✅ Can store recorded video
```

**What's Needed:**

**Live Streaming:**
```
WebRTC/SRTP connection
    ↓
Real-time presence tracker
    ├─ Agent B online? → can connect
    ├─ Agent B offline? → use relay/recording
    └─ Update every 100 ms

Participant A → Participant B (direct P2P)
             → Relay if direct fails
             → Record simultaneously
```

**Recording:**
```
Live stream chunks → temporary buffer (RAM)
                  → Media server (video transcode)
                  → Store in custodian network
                  → Replicate to 3+ nodes
                  → Remove from relay after store
```

**At Scale (100 concurrent calls):**
- Live relay traffic: 5 Mbps × 100 calls = 500 Mbps (manageable for datacenter, impossible for single node)
- Recording buffer: 10 sec × 10 Mbps = ~6 GB total (manageable)
- Concurrent users stored: ~10 recordings × 2 GB = 20 GB (requires distributed storage)

---

## Separation of Concerns Analysis

**Current Model: ✅ Good**

```
┌─────────────────────┐
│ Trust/Governance    │  Who says it's true?
│ (Reach + Attest)    │
├─────────────────────┤
│ Metadata Storage    │  What is it?
│ (DHT + Projection)  │
├─────────────────────┤
│ Blob Storage        │  ⚠️ MIXED (should be separate)
│ (??? + Cache)       │
├─────────────────────┤
│ Delivery Protocol   │  How to get it?
│ (HTTP + P2P)        │  ⚠️ MIXED (no clear separation)
└─────────────────────┘
```

**Problem: Blob Storage and Delivery are Tangled**

Current code doesn't distinguish:

```
1. METADATA LAYER (already separate)
   - ContentNode in DHT ✅
   - Reach level ✅
   - Trust score ✅

2. BLOB STORAGE LAYER (not separated)
   ❌ External URLs (hardcoded in HTTP request)
   ❌ Cache treated same as DHT metadata
   ❌ No separate blob replication strategy

3. DELIVERY LAYER (not separated)
   ❌ HTTP used for everything
   ❌ No range request support
   ❌ No streaming support
   ❌ No P2P fallback on HTTP failure
```

**Should Be:**
```
ContentNode (metadata)  ← What is it?
    ↓
Reach + Trust          ← Who can see it?
    ↓
Blob Hash Reference    ← Where is it?
    ├─→ Custodian 1
    ├─→ Custodian 2
    └─→ Custodian 3
            ↓
Delivery Strategy      ← How to get it?
    ├─ HTTP/2 (fast, centralized)
    ├─ HTTP Range (resumable)
    ├─ HLS/DASH (streaming)
    ├─ WebRTC (direct P2P)
    └─ BitTorrent fallback (distributed)
```

---

## Recommended Architecture

### Phase 1: Blob Pointer System (Minimal Change)

**Add to ContentNode:**
```typescript
interface ContentBlob {
    hash: string;           // SHA256 of blob
    sizeBytes: number;      // Expected size
    mimeType: string;       // "video/mp4", "audio/mpeg"
    fallbackUrls: string[]; // [primary, secondary, tertiary]
    bitrateMbps?: number;   // For codecs that need it
    durations?: {           // For variants (480p, 720p, 1080p)
        "480p": number;
        "720p": number;
        "1080p": number;
    };
}

interface ContentNode {
    // ... existing fields ...
    blobs?: ContentBlob[]; // NEW: for new media types
    embedStrategy?: 'iframe' | 'steward' | 'web-component' | 'http-range' | 'hls';
}
```

**Store in DHT:** Metadata only (5-50 KB per content)
**Store Separately:** Blob (external system)

**Benefits:**
- ✅ Minimal code change
- ✅ Backward compatible (embedStrategy default = 'iframe')
- ✅ Enables HTTP Range support
- ✅ Blob hash allows integrity checking

---

### Phase 2: Chunked Delivery (Content Strategy Layer)

**Add Doorway Route:**
```
GET /api/v1/{dna}/{zome}/{fn}?id=...
GET /api/blob/{content-id}/{chunk-index}?reach=...
GET /api/manifest/{content-id}/hls.m3u8
GET /api/manifest/{content-id}/dash.mpd
```

**Implement:**
```rust
pub enum DeliveryStrategy {
    Complete,           // Small files (<1 MB)
    Chunked,           // Medium files (1 MB - 1 GB)
    Streaming(StreamingFormat),  // HLS, DASH, WebRTC
    HTTPRange,         // Resume support
}

pub enum StreamingFormat {
    HLS {
        segments: Vec<HLSSegment>,
        target_duration: u32,
        max_bitrate_mbps: u32,
    },
    DASH {
        periods: Vec<DASHPeriod>,
        mpd_url: String,
    },
    WebRTC {
        offer: String,
        ice_candidates: Vec<String>,
    },
}
```

**Cache Tiers:**
```rust
pub struct CacheTierConfig {
    pub metadata_tier: {
        max_size: "unlimited",    // Manifests, .m3u8, .mpd
        ttl: 3600,                // 1 hour
    },
    pub chunk_tier: {
        max_size: "1GB",          // Individual chunks
        ttl: 86400,               // 24 hours
        reach_isolated: true,     // Don't mix reaches
    },
    pub complete_tier: {
        max_size: "100MB",        // Small complete files
        ttl: 3600,
    },
}
```

---

### Phase 3: Custodian Network (Distribution)

**Hook in Custodian Commitment:**
```rust
pub struct CustodianCommitment {
    // ... existing fields ...
    pub media_role: MediaCustodyRole,
}

pub enum MediaCustodyRole {
    Storage {
        max_storage_gb: u32,
        chunk_redundancy: u32,  // 2 = 2 copies, 3 = 3 copies
    },
    Cache {
        max_cache_gb: u32,
        serve_to_reach: Vec<String>,  // ["local", "neighborhood"]
    },
    Relay {
        max_concurrent_streams: u32,
        bandwidth_mbps: u32,
    },
}
```

**Replication Strategy:**
```
Content created at reach=private
    ↓
Holochain zome calculates needed copies:
    - private: 1 copy (owner)
    - local: 3 copies (family)
    - neighborhood: 5 copies
    - municipal: 10 copies
    - commons: available to all
    ↓
Query custodian network:
    - Find nodes with Media role
    - Match bandwidth_class to reach level
    - Select by geographic proximity
    ↓
Initiate Blob Replication:
    - Send to Custodian 1
    - Send to Custodian 2
    - etc.
    ↓
Track status in DHT:
    {"content_id": "...", "stored_at": ["custodian1", "custodian2"], ...}
```

---

### Phase 4: Real-Time Communication

**Add WebRTC Support:**
```
Zoom Call Recording
    ↓
WebRTC stream + metadata
    ↓
Holochain relay captures (temporary)
    ↓
Once call ends:
    - Transcode to standard codec (H.264)
    - Chunk into 10 MB segments
    - Calculate SHA256 hashes
    - Create blob manifest
    - Create ContentNode with blobs
    - Initiate custodian replication
    ↓
Participants access via:
- Direct blob fetch (if online)
- Cached chunk (if cached)
- P2P fallback (from other viewers)
```

---

## Implementation Roadmap

### Now (Already Done)
- ✅ ContentNode with reach levels
- ✅ Reach-aware access control
- ✅ Cache with reach-based keys
- ✅ Projection system for metadata

### Sprint 1 (2 weeks)
- Add ContentBlob type to models
- Implement blob hash verification
- Add HTTP Range request support in Doorway
- Add fallback URL cascading

### Sprint 2 (3 weeks)
- Implement HLS manifest generation
- Add chunk-based delivery route
- Create media-aware cache tiers
- Add size/frequency-based eviction

### Sprint 3 (2 weeks)
- Implement Custodian Commitment model (waiting in commits)
- Add media replication logic
- Implement custodian selection algorithm

### Sprint 4+ (Ongoing)
- WebRTC relay server
- Live stream ingestion
- Adaptive bitrate selection
- Fallback to BitTorrent for P2P distribution

---

## Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|-----------|
| **Large files OOM node** | Complete node crash | HIGH if video supported now | Implement Phase 1 + 2 before video support |
| **External URLs fail** | Content disappears | HIGH without blob storage | Phase 1: blob hashes + fallback URLs |
| **Cache thrashing** | Hit rate < 30% | HIGH | Phase 2: separate cache tiers |
| **P2P network saturated** | Timeouts for all users | HIGH at scale | Don't use DHT for blobs (use replication) |
| **Zoom call relay uses all bandwidth** | Other users starve | HIGH | Phase 4: separate relay network |
| **Codec obsolescence** | Video unwatchable | MEDIUM | Phase 1: track codec + allow versioning |

---

## Conclusion

**Current Architecture Score: 7/10**
- Excellent metadata handling and reach-based access control
- Good separation of concerns at high level
- **Critical gap:** No strategy for blobs, streaming, or real-time media

**Recommendation:**
1. **Do NOT store large media in DHT or HTTP cache yet** - it will break at scale
2. **Implement Phase 1 immediately** - add blob pointer system with hash verification
3. **Parallel track:** Build out Custodian Commitment model (commits exist, not integrated)
4. **Prototype Phase 2:** Implement HLS for video, HTTP Range for podcasts
5. **Test with realistic scale:** 100 concurrent users, 10 GB video, measure cache hit rates

The foundation is solid. The gaps are **solvable with the right abstraction layer** (separate blob storage from metadata storage from delivery strategy).

Would you like me to create:
1. Detailed API designs for blob delivery?
2. Reference implementation for HLS/DASH manifest generation?
3. Custodian replication algorithms?
4. Performance testing strategy for media at scale?
